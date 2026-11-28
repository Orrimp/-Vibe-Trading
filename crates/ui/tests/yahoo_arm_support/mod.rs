//! Shared harness for the story 2-63 Wave F Yahoo-arm tests
//! (`spawn_lab_run_yahoo_harness.rs`, `lab_runner_ticker_e2e.rs`,
//! `lab_runner_cancel_e2e.rs`).
//!
//! NOT a cargo target: cargo auto-discovers `tests/*.rs` and `tests/*/main.rs`
//! only, so this `mod.rs` is compiled once into each test binary that includes
//! it with `#[path = "yahoo_arm_support/mod.rs"] mod support;` (precedent:
//! `tests/fixtures/mod.rs`).
//!
//! ## What it drives
//!
//! [`start_arm`] runs the PRODUCTION `ui::lab::runner::run_yahoo_preload_arm`
//! — the function `spawn_lab_run` calls once for every Yahoo source — inside
//! `futures::executor::block_on` on a bare `std::thread`, which is how iced's
//! `futures::ThreadPool` polls `spawn_lab_run`'s `iced::Task`: NO tokio
//! context on the polling thread. The first statement of the polled future
//! asserts that (`HARNESS: arm must be polled with no tokio context`). The
//! tokio runtime (multi-thread, 2 workers, `enable_all`) is built here and
//! NEVER entered; the arm receives its `Handle`, as production does.
//!
//! [`start_with`] is the general form: the yahoo harness's Task-driven tests
//! use it to drive the real `spawn_lab_run` `iced::Task` through
//! `iced_test::runtime::task::into_stream` on the same kind of thread.
//!
//! ## The mock
//!
//! [`MockYahoo`] implements `LabBarSource` + `LabYahooBarSource`. Its
//! `preload` runs on a runtime worker (the arm routes it through
//! `spawn_preload_on_rt`), sleeps `delay`, and returns per [`Behavior`].
//! [`Probe`] records whether `preload` was called, whether its future ran to
//! completion, and whether the source was DROPPED (its `Drop` impl): a source
//! is dropped when the spawned preload task ends, so `dropped && !completed`
//! shortly after a cancel is the observable of `JoinHandle::abort()`.
//!
//! ## Messages
//!
//! Every panic this module raises starts with `HARNESS:` — it reports a broken
//! fixture, never a verdict on the subject. The one exception is
//! [`Run::wait_done`]'s `arm thread panicked: <payload>`, which surfaces a
//! panic in the subject (for example the no-reactor panic of T-T4 F-P3).
//!
//! ## Drop order
//!
//! [`Run`]'s fields drop in declaration order: the cancel handle FIRST (so a
//! still-running arm is cancelled and returns), the runtime LAST.

#![allow(dead_code)]

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::mpsc as std_mpsc;
use std::time::{Duration, Instant};

use agent::EventBus;
use agent::activity::{ActivityEvent, ActivitySender};
use agent::config::BusConfig;
use backtest::ScenarioConfig;
use backtest::progress::{Progress, ProgressSender, progress_pair};
use backtest::scenarios::sma_composed_run::{default_start_price, synthetic_bars_minute};
use smol_str::SmolStr;
use tokio::sync::broadcast::error::TryRecvError as BroadcastTryRecvError;
use tokio::sync::mpsc::error::TryRecvError;
use trading_core::Symbol;
use ui::lab::runner::{
    LabBarSource, LabRunConfig, LabYahooBarSource, PreloadFuture, RunCancelHandle,
    RunCancelReceiver, cancellation_pair, lab_config_to_scenario, run_yahoo_preload_arm,
};
use ui::lab::state::LabDataSource;

/// The post-return marker the arm thread sends on the progress channel after
/// `run_yahoo_preload_arm` returns. No production emit has this shape (the
/// sentinel and the ticks are `(0, 1, ·)`; the engine never runs here).
pub const MARKER: Progress = Progress {
    current_bar: 7,
    total_bars: 7,
    elapsed_ms: 7,
};

/// The number of stale bars [`start_arm`] seeds into `scenario_cfg` before the
/// arm runs: only a real overwrite by the arm can replace them.
pub const STALE_BARS: usize = 3;

/// `true` for the `(0, 1, 0)` preload sentinel.
#[must_use]
pub fn is_sentinel(p: &Progress) -> bool {
    p.current_bar == 0 && p.total_bars == 1 && p.elapsed_ms == 0
}

/// `true` for [`MARKER`].
#[must_use]
pub fn is_marker(p: &Progress) -> bool {
    p.current_bar == MARKER.current_bar
        && p.total_bars == MARKER.total_bars
        && p.elapsed_ms == MARKER.elapsed_ms
}

// ── The mock ──────────────────────────────────────────────────────────────────

/// What [`MockYahoo`]'s preload returns after its delay.
#[derive(Clone, Copy, Debug)]
pub enum Behavior {
    /// `Ok((bars, sha))` with this many deterministic synthetic bars.
    Bars(usize),
    /// `Ok((vec![], sha))` — the empty-preload (no-data notice) trigger.
    Empty,
    /// `Err(msg)` — a hard, untagged preload failure.
    Fail(&'static str),
}

/// Shared observation flags of one [`MockYahoo`].
#[derive(Clone, Default, Debug)]
pub struct Probe {
    calls: Arc<AtomicUsize>,
    completed: Arc<AtomicBool>,
    dropped: Arc<AtomicBool>,
}

impl Probe {
    /// How many times `preload` was called (the spawned task was polled).
    #[must_use]
    pub fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
    /// Whether the preload future ran to completion (its sleep elapsed).
    #[must_use]
    pub fn completed(&self) -> bool {
        self.completed.load(Ordering::SeqCst)
    }
    /// Whether the source was dropped (the spawned preload task ended).
    #[must_use]
    pub fn dropped(&self) -> bool {
        self.dropped.load(Ordering::SeqCst)
    }
}

/// The deterministic synthetic bars every `Bars(n)` mock returns (the same
/// generator the Binance harness uses).
#[must_use]
pub fn mock_bars(symbol: &str, n: usize) -> Vec<trading_core::Bar> {
    let sym = Symbol::new(symbol);
    synthetic_bars_minute(&sym, n, 0xB1AB, default_start_price(&sym), 2023)
}

/// Test double for the Yahoo preload source.
pub struct MockYahoo {
    delay: Duration,
    behavior: Behavior,
    probe: Probe,
}

impl MockYahoo {
    /// A mock that sleeps `delay` and then returns per `behavior`.
    #[must_use]
    pub fn new(delay: Duration, behavior: Behavior, probe: Probe) -> Self {
        Self {
            delay,
            behavior,
            probe,
        }
    }
}

impl Drop for MockYahoo {
    fn drop(&mut self) {
        self.probe.dropped.store(true, Ordering::SeqCst);
    }
}

impl LabBarSource for MockYahoo {
    fn preload<'a>(
        &'a self,
        cfg: &'a LabRunConfig,
        _range: &'a backtest::engine::DateRange,
    ) -> PreloadFuture<'a> {
        self.probe.calls.fetch_add(1, Ordering::SeqCst);
        let delay = self.delay;
        Box::pin(async move {
            tokio::time::sleep(delay).await;
            let result = match self.behavior {
                Behavior::Bars(n) => Ok((
                    mock_bars(cfg.symbol.as_str(), n),
                    SmolStr::new("mock-yahoo-sha-0000000000000000"),
                )),
                Behavior::Empty => {
                    Ok((Vec::new(), SmolStr::new("mock-yahoo-sha-0000000000000000")))
                }
                Behavior::Fail(msg) => Err(SmolStr::new(msg)),
            };
            self.probe.completed.store(true, Ordering::SeqCst);
            result
        })
    }
}

impl LabYahooBarSource for MockYahoo {}

// ── The run spec ──────────────────────────────────────────────────────────────

/// One harness run.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// The mock preload's delay.
    pub delay: Duration,
    /// What the mock returns after the delay.
    pub behavior: Behavior,
    /// Drop the cancel handle BEFORE the arm starts.
    pub pre_cancelled: bool,
    /// Wire an `ActivitySender` (subscribed before the arm starts).
    pub with_activity: bool,
}

impl Spec {
    /// A spec with no activity and no pre-cancel.
    #[must_use]
    pub fn new(delay_ms: u64, behavior: Behavior) -> Self {
        Self {
            delay: Duration::from_millis(delay_ms),
            behavior,
            pre_cancelled: false,
            with_activity: false,
        }
    }
}

/// The Lab config every harness run uses: YahooCache, `v0.sma × BTCUSDT ×
/// Last30d`, no report.
#[must_use]
pub fn yahoo_cache_cfg() -> LabRunConfig {
    LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        range: ui::lab::state::DateRange::Preset(ui::lab::state::Preset::Last30d),
        seed: ui::lab::defaults::LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::YahooCache,
        sma_fast_len: None,
        sma_slow_len: None,
    }
}

// ── Starting a run ────────────────────────────────────────────────────────────

/// Everything the polled body receives on the bare thread.
pub struct ArmInputs {
    /// The (never entered) runtime's handle.
    pub handle: tokio::runtime::Handle,
    /// The boxed mock.
    pub source: Box<dyn LabYahooBarSource>,
    /// The Lab config ([`yahoo_cache_cfg`]).
    pub cfg: LabRunConfig,
    /// `lab_config_to_scenario(&cfg)` seeded with [`STALE_BARS`] stale bars.
    pub scenario_cfg: ScenarioConfig,
    /// The progress sender (the test holds the receiver).
    pub progress_tx: ProgressSender,
    /// The cancel receiver (the test holds the handle, unless pre-cancelled).
    pub cancel_rx: RunCancelReceiver,
    /// The activity sender, when the spec wires one.
    pub sender: Option<ActivitySender>,
}

/// What [`start_arm`]'s body reports after `run_yahoo_preload_arm` returns.
#[derive(Debug)]
pub struct Done {
    /// The arm's return value.
    pub result: Result<(), SmolStr>,
    /// `scenario_cfg` after the arm.
    pub scenario_cfg: ScenarioConfig,
    /// When the arm returned.
    pub returned_at: Instant,
}

/// One item read off the progress channel.
#[derive(Debug)]
pub enum Next {
    /// An item, and when it was read (since `t0`).
    Item(Progress, Duration),
    /// Nothing within the window; the channel is still open.
    Silent,
    /// The channel is closed and drained.
    Closed,
}

/// A running harness thread. Fields drop in declaration order (module doc).
pub struct Run<D> {
    cancel: Option<RunCancelHandle>,
    progress_rx: tokio::sync::mpsc::Receiver<Progress>,
    activity_rx: Option<tokio::sync::broadcast::Receiver<ActivityEvent>>,
    done_rx: std_mpsc::Receiver<D>,
    returned: Arc<AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
    /// The mock's observation flags.
    pub probe: Probe,
    /// Taken immediately before the thread is spawned.
    pub t0: Instant,
    bus: Option<Arc<EventBus>>,
    rt: tokio::runtime::Runtime,
}

/// Start a run of `body` on a bare thread with no tokio context.
///
/// `body` gets the [`ArmInputs`] and the `returned` flag it must set once the
/// subject has returned; its value is delivered by [`Run::wait_done`].
///
/// # Panics
///
/// `HARNESS:` panics when the fixture cannot be built.
pub fn start_with<D, F>(spec: Spec, body: F) -> Run<D>
where
    D: Send + 'static,
    F: FnOnce(ArmInputs, &AtomicBool) -> D + Send + 'static,
{
    let cfg = yahoo_cache_cfg();
    assert!(
        !cfg.write_report,
        "HARNESS: the Yahoo-arm harness never writes a report"
    );
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("HARNESS: build the 2-worker runtime");
    let handle = rt.handle().clone();

    let (cancel_handle, cancel_rx) = cancellation_pair();
    let cancel = if spec.pre_cancelled {
        drop(cancel_handle);
        None
    } else {
        Some(cancel_handle)
    };
    let (progress_tx, progress_rx) = progress_pair();

    let (bus, sender, activity_rx) = if spec.with_activity {
        let bus = Arc::new(EventBus::new(&BusConfig::default()));
        let sender = bus.activity();
        // Subscribe BEFORE the arm starts, so the Start event is seen.
        let rx = sender.subscribe();
        (Some(bus), Some(sender), Some(rx))
    } else {
        (None, None, None)
    };

    let mut scenario_cfg =
        lab_config_to_scenario(&cfg).expect("HARNESS: Last30d maps to a ScenarioConfig");
    scenario_cfg.bars_override = Some(mock_bars("BTCUSDT", STALE_BARS));

    let probe = Probe::default();
    let source: Box<dyn LabYahooBarSource> =
        Box::new(MockYahoo::new(spec.delay, spec.behavior, probe.clone()));

    assert!(
        tokio::runtime::Handle::try_current().is_err(),
        "HARNESS: the test thread must have no tokio context"
    );

    let returned = Arc::new(AtomicBool::new(false));
    let returned_in = Arc::clone(&returned);
    let (done_tx, done_rx) = std_mpsc::channel::<D>();
    let inputs = ArmInputs {
        handle,
        source,
        cfg,
        scenario_cfg,
        progress_tx,
        cancel_rx,
        sender,
    };

    let t0 = Instant::now();
    let thread = std::thread::Builder::new()
        .name("yahoo-arm-harness".into())
        .spawn(move || {
            let done = body(inputs, &returned_in);
            let _ = done_tx.send(done);
        })
        .expect("HARNESS: spawn the arm thread");

    Run {
        cancel,
        progress_rx,
        activity_rx,
        done_rx,
        returned,
        thread: Some(thread),
        probe,
        t0,
        bus,
        rt,
    }
}

/// Start the PRODUCTION `run_yahoo_preload_arm` per `spec` (module doc).
///
/// After the arm returns, the thread records `returned_at`, sets the returned
/// flag, sends [`MARKER`] on the progress channel and drops its sender, so a
/// channel that stays open past the marker has a sender leaked elsewhere.
#[must_use]
pub fn start_arm(spec: Spec) -> Run<Done> {
    start_with(spec, |inputs, returned| {
        let ArmInputs {
            handle,
            source,
            cfg,
            mut scenario_cfg,
            progress_tx,
            cancel_rx,
            sender,
        } = inputs;
        let result = futures::executor::block_on(async {
            assert!(
                tokio::runtime::Handle::try_current().is_err(),
                "HARNESS: arm must be polled with no tokio context"
            );
            run_yahoo_preload_arm(
                &handle,
                source,
                &cfg,
                &mut scenario_cfg,
                &progress_tx,
                &cancel_rx,
                sender.as_ref(),
            )
            .await
        });
        let returned_at = Instant::now();
        returned.store(true, Ordering::SeqCst);
        progress_tx.try_send(MARKER);
        drop(progress_tx);
        drop(sender);
        drop(cancel_rx);
        Done {
            result,
            scenario_cfg,
            returned_at,
        }
    })
}

impl<D> Run<D> {
    /// Read the next progress item, waiting at most `within`.
    pub fn next_item(&mut self, within: Duration) -> Next {
        let deadline = Instant::now() + within;
        loop {
            match self.progress_rx.try_recv() {
                Ok(p) => return Next::Item(p, self.t0.elapsed()),
                Err(TryRecvError::Disconnected) => return Next::Closed,
                Err(TryRecvError::Empty) => {
                    if Instant::now() >= deadline {
                        return Next::Silent;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
            }
        }
    }

    /// Collect every item up to (not including) [`MARKER`], each with its
    /// arrival time since `t0`.
    ///
    /// # Panics
    ///
    /// `HARNESS:` when the channel closes before the marker or the marker
    /// does not arrive within `within`.
    pub fn items_until_marker(&mut self, within: Duration) -> Vec<(Progress, Duration)> {
        let deadline = Instant::now() + within;
        let mut items = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match self.next_item(left) {
                Next::Item(p, _) if is_marker(&p) => return items,
                Next::Item(p, at) => items.push((p, at)),
                Next::Closed => panic!(
                    "HARNESS: the progress channel closed before the post-return marker \
                     (items so far: {items:?})"
                ),
                Next::Silent => panic!(
                    "HARNESS: no post-return marker within {within:?} (items so far: {items:?})"
                ),
            }
        }
    }

    /// Whether the subject has returned (the body set the flag).
    #[must_use]
    pub fn has_returned(&self) -> bool {
        self.returned.load(Ordering::SeqCst)
    }

    /// Wait at most `within` for the body's report.
    ///
    /// # Panics
    ///
    /// `arm thread panicked: <payload>` when the thread died before reporting
    /// (a panic in the subject or in a `HARNESS:` assert on that thread).
    pub fn wait_done(&mut self, within: Duration) -> Option<D> {
        match self.done_rx.recv_timeout(within) {
            Ok(done) => Some(done),
            Err(std_mpsc::RecvTimeoutError::Timeout) => None,
            Err(std_mpsc::RecvTimeoutError::Disconnected) => {
                let payload = self
                    .thread
                    .take()
                    .map(std::thread::JoinHandle::join)
                    .and_then(Result::err)
                    .map_or_else(
                        || "<no payload>".to_owned(),
                        |p| {
                            p.downcast_ref::<String>()
                                .cloned()
                                .or_else(|| p.downcast_ref::<&str>().map(|s| (*s).to_owned()))
                                .unwrap_or_else(|| "<non-string payload>".to_owned())
                        },
                    );
                panic!("arm thread panicked: {payload}");
            }
        }
    }

    /// Fire Stop: drop the cancel handle (the production cancel mechanism).
    ///
    /// # Panics
    ///
    /// `HARNESS:` on a pre-cancelled run (there is no handle left).
    pub fn cancel_now(&mut self) -> Instant {
        let handle = self
            .cancel
            .take()
            .expect("HARNESS: cancel_now on a run that has no cancel handle");
        let at = Instant::now();
        drop(handle);
        at
    }

    /// Every activity event received so far.
    ///
    /// # Panics
    ///
    /// `HARNESS:` when the run has no activity wiring or the receiver lagged.
    pub fn activity(&mut self) -> Vec<ActivityEvent> {
        let rx = self
            .activity_rx
            .as_mut()
            .expect("HARNESS: activity() on a run spawned without with_activity");
        let mut out = Vec::new();
        loop {
            match rx.try_recv() {
                Ok(ev) => out.push(ev),
                Err(BroadcastTryRecvError::Empty | BroadcastTryRecvError::Closed) => return out,
                Err(BroadcastTryRecvError::Lagged(n)) => {
                    panic!("HARNESS: the activity receiver lagged by {n} events")
                }
            }
        }
    }

    /// Spin until `cond` holds.
    ///
    /// # Panics
    ///
    /// `HARNESS: <what> not observed within <within>` on timeout.
    pub fn spin_until(&self, within: Duration, cond: impl FnMut() -> bool, what: &str) {
        assert!(
            becomes_true(within, cond),
            "HARNESS: {what} not observed within {within:?}"
        );
    }

    /// The runtime's handle (for tests that must spawn on it).
    #[must_use]
    pub fn rt_handle(&self) -> tokio::runtime::Handle {
        self.rt.handle().clone()
    }
}

/// Whether `cond` becomes true within `within` (polled every millisecond).
/// For SUBJECT bounds: the caller asserts on the result with its own message.
pub fn becomes_true(within: Duration, mut cond: impl FnMut() -> bool) -> bool {
    let deadline = Instant::now() + within;
    loop {
        if cond() {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}

/// Sleep until `t0 + at` (no-op if that is already past).
pub fn sleep_until(t0: Instant, at: Duration) {
    let target = t0 + at;
    let now = Instant::now();
    if target > now {
        std::thread::sleep(target - now);
    }
}
