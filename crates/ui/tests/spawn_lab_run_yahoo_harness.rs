//! Surface 1 — the Yahoo preload arm of `spawn_lab_run`, driven through the
//! PRODUCTION code (story 2-63 Wave F de-tautologization).
//!
//! lab-recipe-test-harness v0.1.0 (T-D2 / ADR-0048 D3); rewritten by story 2-63
//! (v0.3) Wave F.
//!
//! ## What this file tests
//!
//! Until Wave F this file re-implemented the arm's sentinel and `select!` loop
//! inline and asserted on its own copy (its old tests `sentinel_fires_before_preload_await`,
//! `channel_survives_after_preload` and `ticker_events_stop_after_preload_complete`):
//! a regression in `crates/ui/src/lab/runner.rs` left it green. Wave F
//! extracted the arm into the pub seam `ui::lab::runner::run_yahoo_preload_arm`
//! — the function `spawn_lab_run` calls ONCE for every Yahoo source, the
//! production default and a test override alike — and this file now calls it.
//!
//! Two drives:
//!
//! 1. **The arm** (`support::start_arm`, tests 1-6): `run_yahoo_preload_arm`
//!    with a mock source, polled by `futures::executor::block_on` on a bare
//!    thread with NO tokio context — how iced's `futures::ThreadPool` polls
//!    the Lab Task. Pins the sentinel, the 250 ms ticker (two-sided count,
//!    freshness), the ticker's stop at return, the three-way classification
//!    (bars → `Ok`, empty → tagged notice, hard error → verbatim) and the
//!    `YahooPreload` activity.
//! 2. **The Task** (`mod task_driven`, tests 7-8, `yahoo` feature): the REAL
//!    `spawn_lab_run` `iced::Task`, drained through
//!    `iced_test::runtime::task::into_stream` on the same kind of thread. Pins
//!    the call site: the override is routed through the arm (sentinel, cancel),
//!    the activity sender is passed, and the preloaded bars plus the progress
//!    channel reach the engine (the first test ever to run the preload → engine
//!    hand-off). The primary fall-through detector is the mock's probe (test
//!    8: `calls() == 1` and `completed()`; test 7: the sentinel, the cancel
//!    and the abort). That the cwd's `data/yahoo` stays absent before and
//!    after is SECONDARY: a fall-through to `DefaultLabYahooBarSource` runs
//!    `create_dir_all("data/yahoo")` and, on the cache miss, fetches from
//!    Yahoo over the network. The directory it leaves behind then fails the
//!    `HARNESS:` precondition of every later run until it is deleted
//!    (`rm -r crates/ui/data/yahoo`).
//!
//! The shared harness is `tests/yahoo_arm_support/mod.rs` (not a cargo target).
//!
//! ## `#[cfg(feature = "live")]` gate
//!
//! `LabYahooBarSource` and `run_yahoo_preload_arm` are compiled under `live`
//! (a default feature). The Task-driven tests also need `yahoo` (default):
//! without it `spawn_lab_run` returns early for a YahooCache config.
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE exact-once replace in `crates/ui/src/lab/runner.rs`,
//! located by symbol + quoted text. A one-line OLD has `grep -c` = 1; a
//! multi-line OLD (F-P1, F-P7, F-P15) is unique only as a block — count it
//! as a substring, its verbatim text is in the story's probe catalogue
//! (`PROBES.md`). Run `cargo test -p ui --test spawn_lab_run_yahoo_harness`,
//! restore from a `cp -p` snapshot, `cmp` it, `touch` it, re-run GREEN.
//!
//! | # | symbol | quoted line today | mutation | expected RED (here) |
//! |---|---|---|---|---|
//! | F-P1 (AC4) | `run_yahoo_preload_arm` | ONE block: from the sentinel `progress_tx.try_send(backtest::progress::Progress {` directly above `let preload_start = std::time::Instant::now();` through `drop(ticker);` (101 lines) | the same block with the sentinel moved after `drop(ticker);` | `yahoo_arm_emits_sentinel_before_preload_resolves` at "first progress item must be the sentinel (0,1,0) within 200 ms"; also test 2 ("the first item must be the sentinel (0,1,0)") and test 7 |
//! | F-P2 (AC4) | `run_yahoo_preload_arm` | `_ = ticker.tick() => {` | `_ = std::future::pending::<()>() => {` | `yahoo_arm_ticker_emits_during_preload_with_increasing_elapsed` at "expected 3..=5 ticker events during a 1000 ms preload; got 0"; test 3's precondition |
//! | F-P6 | `run_yahoo_preload_arm` | `ticker.tick().await;` (the t=0 consume) | append a detached `rt.spawn` ticker holding a `progress_tx` clone | `yahoo_arm_ticker_stops_after_preload_resolves` at "after the post-return marker the channel must close with no further item" |
//! | F-P7 | `run_yahoo_preload_arm` (NOT the Binance arm: its first line also occurs in `run_binance_preload_arm`) | the 3-line block `match classify_preload_result(preload_result, cfg) {` / `Ok((bars, _sha)) => {` / `scenario_cfg.data_source = backtest::engine::ScenarioDataSource::YahooCache;` | first line → `match preload_result {` | `yahoo_arm_empty_bars_returns_no_data_notice` at "empty preload must short-circuit with a NO_DATA_TAG notice" |
//! | F-P9 | `run_yahoo_preload_arm` | `preload_notice::classify(e.as_str()).msg().as_str()` | `e.as_str()` | `yahoo_arm_empty_bars_returns_no_data_notice` at "activity failure reason must be the tag-stripped notice" |
//! | F-P10 | `run_yahoo_preload_arm` | `s.start(ActivityKind::YahooPreload, yahoo_label)` | `ActivityKind::LabRun` | `yahoo_arm_reports_preload_activity_start_then_success_end` at "Start kind must be ActivityKind::YahooPreload" |
//! | F-P11 | `run_yahoo_preload_arm` | `tokio::time::interval(std::time::Duration::from_millis(250))` | `250` → `25` | test 2 at "expected 3..=5 ticker events during a 1000 ms preload; got …" (upper bound) |
//! | F-P12 | the same line | the same | `250` → `500` | test 2 at the same assert (lower bound) |
//! | F-P13 | `spawn_lab_run` | `if let Some(source) = yahoo_source {` | `yahoo_source.filter(\|_\| false)` | `task_driven::spawn_lab_run_routes_the_yahoo_override_through_the_arm` at "sentinel (0,1,0) must arrive within 200 ms"; test 8 at "first progress item must be the sentinel (0,1,0) within 200 ms" (its first item is the engine's `(0, 720, ·)`) |
//! | F-P14 | `spawn_lab_run` | `activity_sender_for_closure.as_ref(),` | `None,` | tests 7 and 8 at "the call site must pass the activity sender: YahooPreload Start expected" |
//! | F-P15 | `spawn_lab_run` | the 3-line block `&mut scenario_cfg,` / `&progress_tx,` / `&cancel,` (the arm call's arguments; the first two lines alone each occur twice) | first line → `&mut scenario_cfg.clone(),` | `task_driven::spawn_lab_run_hands_preloaded_bars_and_progress_channel_to_the_engine` at "engine progress total_bars must equal the mock's 120 bars" (the engine gets 720 synthetic bars); test 7 stays GREEN |
//!
//! F-P3, F-P4, F-P5 and F-P8 are the designated rows of `lab_runner_ticker_e2e.rs`
//! and `lab_runner_cancel_e2e.rs`; they also turn tests here red (reasoned, not
//! claimed).
//!
//! Bug-log `#215` (MEASURED 2026-11-24; the table is `lab_runner_cancel_e2e.rs`'s):
//! T215-P1, P2 (F-P8 re-designated), P3 and P4 turn test 7 red ("cancel must end
//! the YahooPreload activity Cancelled" for P1/P2, "… a CANCELLED_TAG-tagged
//! error" for P3, "… classified Cancelled" for P4); T215-P11 turns tests 4 and
//! 5 red.
//!
//! ## Coverage
//!
//! | # | test | what can fail it |
//! |---|---|---|
//! | 1 | `yahoo_arm_emits_sentinel_before_preload_resolves` | sentinel late / not first; arm returned first; stale bars not overwritten |
//! | 2 | `yahoo_arm_ticker_emits_during_preload_with_increasing_elapsed` | tick count outside 3..=5; first tick outside [200, 600) ms; stale stamps |
//! | 3 | `yahoo_arm_ticker_stops_after_preload_resolves` | any item after the arm returned |
//! | 4 | `yahoo_arm_empty_bars_returns_no_data_notice` | untagged / wrong notice; cfg written on failure; tagged activity reason |
//! | 5 | `yahoo_arm_hard_error_passes_through_verbatim` | a tagged / rewritten hard error; cfg written on failure |
//! | 6 | `yahoo_arm_reports_preload_activity_start_then_success_end` | wrong kind / label / outcome; missing or duplicate event |
//! | 7 | `task_driven::spawn_lab_run_routes_the_yahoo_override_through_the_arm` | override bypassed; no cancel; a Stop not tagged / not `Cancelled` (bug-log `#215`); activity not passed or not ended `Cancelled`; source not aborted |
//! | 8 | `task_driven::spawn_lab_run_hands_preloaded_bars_and_progress_channel_to_the_engine` | engine gets other bars or another channel; a report written |

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

#[path = "yahoo_arm_support/mod.rs"]
mod support;

use std::time::Duration;

use agent::activity::{ActivityKind, ActivityOutcome, ActivityPhase};
use backtest::engine::ScenarioDataSource;
use smol_str::SmolStr;
use support::{Behavior, Next, STALE_BARS, Spec, is_sentinel, start_arm};
use ui::lab::runner::preload_notice::{self, RunMessageKind};

/// Assert the run's first progress item is the `(0, 1, 0)` sentinel, read
/// within 200 ms of `t0`. The message is the one the F-P1 row quotes.
fn assert_sentinel_first<D>(run: &mut support::Run<D>) {
    match run.next_item(Duration::from_millis(200)) {
        Next::Item(p, at) if is_sentinel(&p) && at < Duration::from_millis(200) => {}
        other => {
            panic!("first progress item must be the sentinel (0,1,0) within 200 ms; got {other:?}")
        }
    }
}

// ── Test 1 — the sentinel precedes the preload's resolution ──────────────────

/// The arm emits `Progress { 0, 1, 0 }` BEFORE the 500 ms mock preload
/// resolves, then returns `Ok(())` with the mock's bars written over the
/// stale seed.
#[test]
fn yahoo_arm_emits_sentinel_before_preload_resolves() {
    let mut run = start_arm(Spec::new(500, Behavior::Bars(120)));

    assert_sentinel_first(&mut run);
    assert!(
        !run.has_returned(),
        "the sentinel must arrive while the arm is still in its 500 ms preload"
    );

    let done = run
        .wait_done(Duration::from_secs(3))
        .expect("the arm must return within 3 s of a 500 ms preload");
    assert_eq!(done.result, Ok(()), "a 120-bar preload must succeed");
    assert_eq!(
        done.scenario_cfg.data_source,
        ScenarioDataSource::YahooCache,
        "the arm must switch the engine source to YahooCache"
    );
    assert_eq!(
        done.scenario_cfg.bars_override.as_ref().map(Vec::len),
        Some(120),
        "bars_override must hold the mock's 120 bars (the {STALE_BARS} stale seed bars overwritten)"
    );
    assert_eq!(run.probe.calls(), 1, "the source must be preloaded once");
    assert!(
        run.probe.completed(),
        "the mock preload must have completed"
    );
}

// ── Test 2 — the ticker's cadence during a 1 s preload ───────────────────────

/// During a 1000 ms preload the 250 ms ticker emits 3..=5 `(0, 1, elapsed)`
/// events, the first in [200, 600) ms, fresh when read.
#[test]
fn yahoo_arm_ticker_emits_during_preload_with_increasing_elapsed() {
    let mut run = start_arm(Spec::new(1_000, Behavior::Bars(10)));
    let items = run.items_until_marker(Duration::from_secs(3));

    let (first, _) = items.first().expect("at least the sentinel must arrive");
    assert!(
        is_sentinel(first),
        "the first item must be the sentinel (0,1,0); got {first:?}"
    );
    let ticks = &items[1..];
    for (p, _) in ticks {
        assert!(
            p.current_bar == 0 && p.total_bars == 1 && p.elapsed_ms > 0,
            "every ticker event must be (0, 1, elapsed > 0); got {p:?}"
        );
    }
    let n = ticks.len();
    assert!(
        (3..=5).contains(&n),
        "expected 3..=5 ticker events during a 1000 ms preload; got {n} ({ticks:?})"
    );
    for w in ticks.windows(2) {
        assert!(
            w[1].0.elapsed_ms >= w[0].0.elapsed_ms,
            "ticker elapsed_ms must be non-decreasing; got {ticks:?}"
        );
    }
    let first_ms = ticks[0].0.elapsed_ms;
    assert!(
        (200..600).contains(&first_ms),
        "the first tick must come ~250 ms after the preload starts (in [200, 600) ms); got {first_ms} ms"
    );
    let span = ticks[n - 1].0.elapsed_ms - first_ms;
    assert!(
        span >= 400,
        "the ticks must span ≥ 400 ms of the preload; got {span} ms ({ticks:?})"
    );
    for (p, at) in ticks {
        let arrival_ms = u64::try_from(at.as_millis()).expect("arrival fits u64");
        let lag = arrival_ms.checked_sub(p.elapsed_ms);
        assert!(
            lag.is_some_and(|l| l < 250),
            "each tick must be read within 250 ms of its stamp (arrival {arrival_ms} ms since t0, stamp {} ms)",
            p.elapsed_ms
        );
    }
    match run.next_item(Duration::from_millis(600)) {
        Next::Closed => {}
        other => panic!("after the post-return marker the channel must close; got {other:?}"),
    }
}

// ── Test 3 — the ticker stops when the preload resolves ──────────────────────

/// After the arm returns, nothing more arrives: the marker the harness sends
/// after the return is the last item, and the channel then closes.
#[test]
fn yahoo_arm_ticker_stops_after_preload_resolves() {
    let mut run = start_arm(Spec::new(600, Behavior::Bars(10)));
    // HARNESS SANITY (not a subject assertion): the marker arrives. The arm
    // borrows `&ProgressSender`, so it cannot consume the harness's sender.
    let items = run.items_until_marker(Duration::from_secs(3));
    assert!(
        items.iter().any(|(p, _)| p.elapsed_ms > 0),
        "precondition: at least one tick during a 600 ms preload; got {items:?}"
    );
    match run.next_item(Duration::from_millis(600)) {
        Next::Closed => {}
        other => panic!(
            "after the post-return marker the channel must close with no further item; got {other:?}"
        ),
    }
}

// ── Test 4 — an empty preload is a tagged no-data notice ─────────────────────

/// Zero bars → `Err(NO_DATA_TAG + notice)`, `scenario_cfg` untouched, and the
/// activity ends `Failed(<the tag-stripped notice>)`.
#[test]
fn yahoo_arm_empty_bars_returns_no_data_notice() {
    let mut run = start_arm(Spec {
        with_activity: true,
        ..Spec::new(10, Behavior::Empty)
    });
    let done = run
        .wait_done(Duration::from_secs(3))
        .expect("the arm must return within 3 s");
    let e = done
        .result
        .expect_err("empty preload must short-circuit with a NO_DATA_TAG notice");
    assert!(
        e.starts_with(preload_notice::NO_DATA_TAG),
        "empty preload must short-circuit with a NO_DATA_TAG notice; got {e:?}"
    );
    let body = match preload_notice::classify(&e) {
        RunMessageKind::Notice(body) => body,
        RunMessageKind::Error(m) => {
            panic!("a tagged notice must classify as Notice; got Error({m:?})")
        }
        RunMessageKind::Cancelled(m) => {
            panic!("a tagged notice must classify as Notice; got Cancelled({m:?})")
        }
    };
    let cfg = support::yahoo_cache_cfg();
    let expected = ui::strings::LAB_YAHOO_NO_DATA_NOTICE
        .replace("{ticker}", cfg.symbol.as_str())
        .replace("{window}", cfg.range_label.as_str());
    assert_eq!(
        body.as_str(),
        expected,
        "the notice must be the Yahoo no-data copy for this ticker and window"
    );
    assert!(
        !body.contains("Binance"),
        "a Yahoo run must not get Binance copy: {body:?}"
    );
    assert_eq!(
        done.scenario_cfg.data_source,
        ScenarioDataSource::Synthetic,
        "a failed preload must leave data_source untouched"
    );
    assert_eq!(
        done.scenario_cfg.bars_override.as_ref().map(Vec::len),
        Some(STALE_BARS),
        "a failed preload must leave bars_override untouched (the stale seed)"
    );

    let events = run.activity();
    assert_eq!(
        events.len(),
        2,
        "exactly Start + End expected; got {events:?}"
    );
    assert_eq!(events[0].id, events[1].id, "one activity id: {events:?}");
    assert!(
        matches!(events[0].phase, ActivityPhase::Start { .. }),
        "the first event must be Start; got {events:?}"
    );
    match &events[1].phase {
        ActivityPhase::End(ActivityOutcome::Failed(reason)) => {
            assert_eq!(
                reason.as_str(),
                body.as_str(),
                "activity failure reason must be the tag-stripped notice"
            );
            assert!(
                !reason.contains('\u{1}'),
                "the activity reason must carry no tag byte: {reason:?}"
            );
        }
        other => panic!("the activity must end Failed(<notice>); got {other:?}"),
    }
}

// ── Test 5 — a hard error passes through verbatim ────────────────────────────

/// An untagged preload error is returned verbatim (classified `Error`), the
/// config is untouched, and the activity ends `Failed(<the same text>)`.
#[test]
fn yahoo_arm_hard_error_passes_through_verbatim() {
    const FAILURE: &str = "mock yahoo transport failure";
    let mut run = start_arm(Spec {
        with_activity: true,
        ..Spec::new(10, Behavior::Fail(FAILURE))
    });
    let done = run
        .wait_done(Duration::from_secs(3))
        .expect("the arm must return within 3 s");
    assert_eq!(
        done.result,
        Err(SmolStr::new(FAILURE)),
        "a hard preload error must pass through verbatim"
    );
    let e = done
        .result
        .as_ref()
        .expect_err("a hard preload error must pass through verbatim");
    assert!(
        matches!(preload_notice::classify(e), RunMessageKind::Error(_)),
        "the arm's verbatim error must classify as Error (no NO_DATA_TAG)"
    );
    assert_eq!(
        done.scenario_cfg.data_source,
        ScenarioDataSource::Synthetic,
        "a failed preload must leave data_source untouched"
    );
    assert_eq!(
        done.scenario_cfg.bars_override.as_ref().map(Vec::len),
        Some(STALE_BARS),
        "a failed preload must leave bars_override untouched (the stale seed)"
    );
    let events = run.activity();
    match events.last().map(|e| &e.phase) {
        Some(ActivityPhase::End(ActivityOutcome::Failed(reason))) => assert_eq!(
            reason.as_str(),
            FAILURE,
            "the activity must end Failed(<the verbatim error>)"
        ),
        other => panic!("the activity must end Failed(<the verbatim error>); got {other:?}"),
    }
}

// ── Test 6 — the YahooPreload activity on the success path ───────────────────

/// A successful preload reports exactly `Start { YahooPreload, "Yahoo BTCUSDT
/// · Last30d" }` then `End(Success)`; the same spec without an activity sender
/// still succeeds.
#[test]
fn yahoo_arm_reports_preload_activity_start_then_success_end() {
    let mut run = start_arm(Spec {
        with_activity: true,
        ..Spec::new(10, Behavior::Bars(5))
    });
    let done = run
        .wait_done(Duration::from_secs(3))
        .expect("the arm must return within 3 s");
    assert_eq!(done.result, Ok(()), "a 5-bar preload must succeed");

    let events = run.activity();
    assert_eq!(
        events.len(),
        2,
        "exactly Start + End expected; got {events:?}"
    );
    assert_eq!(events[0].id, events[1].id, "one activity id: {events:?}");
    assert_eq!(
        events[0].kind,
        ActivityKind::YahooPreload,
        "Start kind must be ActivityKind::YahooPreload"
    );
    assert_eq!(
        events[0].label, "Yahoo BTCUSDT · Last30d",
        "Start label must be `Yahoo <SYMBOL> · <RANGE>`"
    );
    assert!(
        matches!(events[0].phase, ActivityPhase::Start { total_units: None }),
        "the first event must be Start with no unit total; got {:?}",
        events[0].phase
    );
    assert!(
        matches!(
            events[1].phase,
            ActivityPhase::End(ActivityOutcome::Success)
        ),
        "a successful preload must end the activity Success; got {:?}",
        events[1].phase
    );

    // Control: no activity sender wired → the arm still succeeds.
    let mut control = start_arm(Spec::new(10, Behavior::Bars(5)));
    let done = control
        .wait_done(Duration::from_secs(3))
        .expect("the control arm must return within 3 s");
    assert_eq!(
        done.result,
        Ok(()),
        "the control (no activity) must succeed"
    );
    assert_eq!(
        done.scenario_cfg.bars_override.as_ref().map(Vec::len),
        Some(5),
        "the control must hold the mock's 5 bars"
    );
}

// ── Tests 7-8 — the real spawn_lab_run Task ──────────────────────────────────

#[cfg(feature = "yahoo")]
mod task_driven {
    use std::path::PathBuf;
    use std::sync::atomic::Ordering;
    use std::time::{Duration, Instant};

    use agent::activity::{ActivityKind, ActivityOutcome, ActivityPhase};
    use futures::StreamExt as _;
    use iced_test::runtime::Action;
    use ui::lab::runner::LabRunResult;
    use ui::lab::runner::preload_notice::{self, RunMessageKind};
    use ui::state::Message;

    use super::assert_sentinel_first;
    use super::support::{self, ArmInputs, Behavior, Next, Run, Spec};

    /// What the Task thread reports once the Task's stream yielded.
    struct TaskDone {
        result: LabRunResult,
        stream_ended: bool,
        returned_at: Instant,
    }

    /// The cwd-relative production Yahoo cache root (`data/yahoo`); cargo runs
    /// this binary with cwd = `crates/ui`, where it is absent. SECONDARY
    /// fall-through detector (the mock's probe is the primary one): a
    /// fall-through to `DefaultLabYahooBarSource` creates this directory and
    /// fetches from Yahoo over the network.
    fn cwd_yahoo_root() -> PathBuf {
        std::env::current_dir()
            .expect("HARNESS: read the cwd")
            .join("data/yahoo")
    }

    /// Start the REAL `spawn_lab_run` with the mock as `yahoo_source_override`
    /// and drain its `iced::Task` through `into_stream` on a thread with no
    /// tokio context.
    fn start_spawn_lab_run(spec: Spec) -> Run<TaskDone> {
        let root = cwd_yahoo_root();
        assert!(
            !root.exists(),
            "HARNESS: {} must be absent, so a fall-through to DefaultLabYahooBarSource is observable. If it exists, an earlier run fell through to the network source (or the cache was created by hand): delete it and re-run",
            root.display()
        );
        support::start_with(spec, |inputs, returned| {
            let ArmInputs {
                handle,
                source,
                cfg,
                scenario_cfg: _,
                progress_tx,
                cancel_rx,
                sender,
            } = inputs;
            let task = ui::lab::runner::spawn_lab_run(
                Some(&handle),
                cfg,
                cancel_rx,
                progress_tx,
                sender,
                Some(source),
            );
            let mut stream = iced_test::runtime::task::into_stream(task)
                .expect("HARNESS: spawn_lab_run must return a Task with a stream");
            let first = futures::executor::block_on(async {
                assert!(
                    tokio::runtime::Handle::try_current().is_err(),
                    "HARNESS: the Task must be polled with no tokio context"
                );
                stream.next().await
            });
            let returned_at = Instant::now();
            returned.store(true, Ordering::SeqCst);
            let stream_ended = futures::executor::block_on(stream.next()).is_none();
            let result = match first {
                Some(Action::Output(Message::LabRunCompleted(r))) => r,
                Some(Action::Output(other)) => {
                    panic!("HARNESS: the Task must output LabRunCompleted; got {other:?}")
                }
                Some(other) => panic!("HARNESS: the Task must output a message; got {other:?}"),
                None => panic!("HARNESS: the Task's stream ended without an output"),
            };
            TaskDone {
                result,
                stream_ended,
                returned_at,
            }
        })
    }

    /// Drain the progress channel until it closes (at most `within`).
    fn drain_to_closed(
        run: &mut Run<TaskDone>,
        within: Duration,
    ) -> Vec<backtest::progress::Progress> {
        let deadline = Instant::now() + within;
        let mut items = Vec::new();
        loop {
            let left = deadline.saturating_duration_since(Instant::now());
            match run.next_item(left) {
                Next::Item(p, _) => items.push(p),
                Next::Closed => return items,
                Next::Silent => panic!(
                    "the progress channel must close once the Task has returned; still open after {within:?} (items {items:?})"
                ),
            }
        }
    }

    /// The YahooPreload Start the call site's activity sender must produce.
    fn assert_yahoo_start(events: &[agent::activity::ActivityEvent]) {
        assert!(
            events.first().is_some_and(|e| {
                e.kind == ActivityKind::YahooPreload
                    && matches!(e.phase, ActivityPhase::Start { .. })
            }),
            "the call site must pass the activity sender: YahooPreload Start expected; got {events:?}"
        );
    }

    /// `spawn_lab_run` routes `yahoo_source_override` through
    /// `run_yahoo_preload_arm`: the sentinel arrives at once, Stop during the
    /// 5 s preload completes the Task with the production cancel text tagged
    /// as a Stop (`CANCELLED_TAG`, bug-log `#215`), no engine progress
    /// follows, the activity ends `Cancelled`, and the source is aborted.
    #[test]
    fn spawn_lab_run_routes_the_yahoo_override_through_the_arm() {
        let mut run = start_spawn_lab_run(Spec {
            with_activity: true,
            ..Spec::new(5_000, Behavior::Bars(120))
        });

        match run.next_item(Duration::from_millis(200)) {
            Next::Item(p, at) if support::is_sentinel(&p) && at < Duration::from_millis(200) => {}
            other => panic!(
                "the sentinel (0,1,0) must arrive within 200 ms (the override routed through the arm); got {other:?}"
            ),
        }
        support::sleep_until(run.t0, Duration::from_millis(100));
        assert!(
            !run.has_returned(),
            "precondition: the Task must still be in its 5 s preload at t0+100 ms"
        );
        assert!(
            !run.probe.completed(),
            "precondition: the 5 s mock preload must not have completed"
        );

        let at = run.cancel_now();
        let done = run
            .wait_done(Duration::from_millis(500))
            .expect("the Task must complete within 500 ms of Stop");
        assert!(
            done.returned_at.duration_since(at) < Duration::from_millis(500),
            "the Task must complete within 500 ms of Stop"
        );
        match &done.result {
            Err(e) => {
                assert!(
                    e.starts_with(preload_notice::CANCELLED_TAG),
                    "Stop during the preload must complete the Task with a CANCELLED_TAG-tagged error; got {e:?}"
                );
                assert!(
                    matches!(
                        preload_notice::classify(e),
                        RunMessageKind::Cancelled(body) if body == "operator cancelled during preload"
                    ),
                    "Stop during the preload must complete the Task with the production cancel text, classified Cancelled; got {e:?}"
                );
            }
            Ok(summary) => panic!("Stop during the preload must not complete Ok; got {summary:?}"),
        }
        assert!(
            done.stream_ended,
            "the Task's stream must end after its one output"
        );

        let rest = drain_to_closed(&mut run, Duration::from_secs(2));
        assert!(
            rest.iter().all(|p| p.total_bars == 1),
            "every progress item must be preload-phase (total_bars 1) — no engine run after Stop; got {rest:?}"
        );

        let events = run.activity();
        assert_yahoo_start(&events);
        assert_eq!(
            events.len(),
            2,
            "exactly Start + End expected; got {events:?}"
        );
        assert!(
            matches!(
                events[1].phase,
                ActivityPhase::End(ActivityOutcome::Cancelled)
            ),
            "cancel must end the YahooPreload activity Cancelled; got {:?}",
            events[1].phase
        );

        assert!(
            support::becomes_true(Duration::from_millis(200), || run.probe.dropped()),
            "the preload task must be aborted (source dropped) within 200 ms of the Task completing"
        );
        assert!(
            !run.probe.completed(),
            "an aborted preload must not complete"
        );
        assert!(
            !cwd_yahoo_root().exists(),
            "no fall-through to DefaultLabYahooBarSource: {} must still be absent",
            cwd_yahoo_root().display()
        );
    }

    /// `spawn_lab_run` hands the preloaded bars AND the progress channel to the
    /// engine: every engine emit counts the mock's 120 bars, the final-bar emit
    /// is bar 119, and the summary carries the 120 bars.
    #[test]
    fn spawn_lab_run_hands_preloaded_bars_and_progress_channel_to_the_engine() {
        const BARS: usize = 120;
        let mut run = start_spawn_lab_run(Spec {
            with_activity: true,
            ..Spec::new(10, Behavior::Bars(BARS))
        });
        // The run never writes a report: `support::start_with` asserts the
        // fixture's `write_report == false` (HARNESS).

        assert_sentinel_first(&mut run);
        let done = run
            .wait_done(Duration::from_secs(10))
            .expect("the Task must complete within 10 s");
        let summary = match done.result {
            Ok(summary) => summary,
            Err(e) => panic!("a 120-bar run must complete Ok; got Err({e:?})"),
        };
        assert!(
            done.stream_ended,
            "the Task's stream must end after its one output"
        );

        let items = drain_to_closed(&mut run, Duration::from_secs(2));
        let engine: Vec<_> = items.iter().filter(|p| p.total_bars != 1).collect();
        assert!(
            !engine.is_empty(),
            "the engine must emit progress on the channel the call site handed it; got {items:?}"
        );
        for p in &engine {
            assert_eq!(
                p.total_bars, BARS,
                "engine progress total_bars must equal the mock's 120 bars; got {p:?}"
            );
        }
        assert!(
            engine.iter().any(|p| p.current_bar > 0),
            "the engine must emit past bar 0; got {engine:?}"
        );
        assert_eq!(
            engine.last().map(|p| p.current_bar),
            Some(BARS - 1),
            "the last engine emit must be the final bar (index {})",
            BARS - 1
        );
        // The sentinel precedes every engine item (assert_sentinel_first read
        // it first). No preload-phase item (a ticker event) may follow the
        // engine's first emit either: unconditional, `engine` is non-empty.
        let first_engine = items
            .iter()
            .position(|p| p.total_bars != 1)
            .expect("engine items are present (asserted above)");
        assert!(
            items[first_engine..].iter().all(|p| p.total_bars != 1),
            "no preload-phase item may follow the engine's first emit; got {items:?}"
        );

        assert_eq!(
            summary.bars.len(),
            BARS,
            "the summary must carry the mock's 120 bars"
        );
        assert_eq!(summary.strategy_id.as_str(), "v0.sma", "strategy id");
        assert!(
            summary.report_path.is_none(),
            "write_report=false must write no report; got {:?}",
            summary.report_path
        );

        let events = run.activity();
        assert_yahoo_start(&events);
        assert_eq!(
            events.len(),
            2,
            "exactly Start + End expected; got {events:?}"
        );
        assert!(
            matches!(
                events[1].phase,
                ActivityPhase::End(ActivityOutcome::Success)
            ),
            "a successful preload must end the activity Success; got {:?}",
            events[1].phase
        );
        assert_eq!(run.probe.calls(), 1, "the source must be preloaded once");
        assert!(
            run.probe.completed(),
            "the mock preload must have completed"
        );
        assert!(
            !cwd_yahoo_root().exists(),
            "no fall-through to DefaultLabYahooBarSource: {} must still be absent",
            cwd_yahoo_root().display()
        );
    }
}
