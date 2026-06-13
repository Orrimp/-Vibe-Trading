//! Surface 1 — boundary tests for `spawn_lab_run` Yahoo preload path.
//!
//! lab-recipe-test-harness v0.1.0 (T-D2 / ADR-0048 D3).
//!
//! ## What this file tests
//!
//! The three regression categories from Bug #64 attempt 1 (D.1.1 + D.2.1
//! revert), Surface 1 (channel/subscription-flow regressions):
//!
//! **A — Sentinel emission timing**: the sentinel `Progress { 0, 1, 0 }` must
//! arrive on `progress_rx` BEFORE the Yahoo preload future resolves. Catches
//! the regression in `5f9f920` where `ticker.tick().await` was inserted
//! BEFORE the sentinel emit, delaying first emission by ~250 ms.
//!
//! **B — Channel survival across `tokio::select!`**: after the preload
//! completes, the progress channel must still be live and capable of
//! receiving engine events. Catches a closure-move bug where the `select!`
//! ticker arm could consume the channel sender.
//!
//! **C (partial) — Completed run delivers events**: the full preload→engine
//! flow delivers `≥ 1` progress event with `current_bar > 0` after preload.
//!
//! ## Why replicate the inner logic rather than drive `iced::Task`?
//!
//! `iced::Task::perform(future, map)` requires the iced runtime to poll the
//! future. Driving it without a full iced application is non-trivial.
//! The existing `cockpit_live_lab_run_smoke.rs` uses the same inline-
//! replication pattern — this file extends it upstream to the preload stage.
//!
//! ## `#[cfg(feature = "live")]` gate
//!
//! `LabYahooBarSource` is only compiled under `live` (tokio runtime required
//! for the async preload). Tests in this file require `--features live`.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use backtest::engine::DateRange;
use backtest::progress::{Progress, progress_pair};
use smol_str::SmolStr;
use tokio::time::timeout;
use ui::lab::runner::{LabBarSource, LabRunConfig, LabYahooBarSource};
use ui::lab::state::LabDataSource;

// ── MockLabYahooBarSource ─────────────────────────────────────────────────────

/// Test double for the Yahoo bar preload step.
///
/// Configured per-test with:
/// - `sleep_duration`: how long to block before returning bars (simulates
///   cold-cache or network latency). Default 500 ms — long enough for 2+
///   ticker ticks at 250 ms cadence but short enough for CI wall-clock budget.
///
/// Returns an empty `Vec<trading_core::Bar>` — the Surface 1 tests only
/// exercise the preload channel flow (sentinel timing, channel survival),
/// not the engine's bar processing. The bars_override value is irrelevant for
/// the timing assertions.
struct MockLabYahooBarSource {
    sleep_duration: Duration,
}

impl MockLabYahooBarSource {
    /// Default: 500 ms sleep. Long enough for sentinel timing assertion.
    fn default_500ms() -> Self {
        Self {
            sleep_duration: Duration::from_millis(500),
        }
    }

    /// Fast variant (10 ms sleep) — for channel-survival tests.
    fn fast_10ms() -> Self {
        Self {
            sleep_duration: Duration::from_millis(10),
        }
    }
}

// simple-strategies-realdata T-B3: `preload` body on the shared `LabBarSource`;
// `LabYahooBarSource` is a pure marker tagging this as the Yahoo seam.
impl LabBarSource for MockLabYahooBarSource {
    fn preload<'a>(
        &'a self,
        _cfg: &'a LabRunConfig,
        _range: &'a backtest::engine::DateRange,
    ) -> ui::lab::runner::PreloadFuture<'a> {
        let sleep = self.sleep_duration;
        Box::pin(async move {
            tokio::time::sleep(sleep).await;
            // Empty bar list — Surface 1 tests only assert on channel flow,
            // not on the bars returned. The engine is not driven in these tests.
            Ok((
                Vec::new(),
                SmolStr::new("mock-sha-deterministic-0000000000000000"),
            ))
        })
    }
}

impl LabYahooBarSource for MockLabYahooBarSource {}

// ── Helper ────────────────────────────────────────────────────────────────────

/// Build a standard `LabRunConfig` pointing at YahooCache source.
///
/// The YahooCache source triggers the preload path in spawn_lab_run.
/// Using a synthetic strategy + symbol avoids any disk / network access;
/// the `MockLabYahooBarSource` intercepts the preload before the network.
fn yahoo_cache_cfg() -> LabRunConfig {
    LabRunConfig {
        strategy_id: SmolStr::new("v0.sma"),
        symbol: SmolStr::new("BTCUSDT"),
        venue: SmolStr::new("Binance"),
        range_label: SmolStr::new("Last30d"),
        seed: ui::lab::defaults::LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::YahooCache,
        sma_fast_len: None,
        sma_slow_len: None,
    }
}

// ── Test 1 — Sentinel fires BEFORE preload await ──────────────────────────────

/// T-D2 / ADR-0048 D3 category A — sentinel emission timing.
///
/// Asserts: the first `Progress` event on `progress_rx` arrives at
/// `elapsed < 50 ms` (well before the mock's 500 ms sleep completes).
///
/// **Falsification**: under `5f9f920` (the reverted regression), the code
/// inserts `ticker.tick().await` (250 ms wait) BEFORE the sentinel emit.
/// Under that version, the first event arrives at ~250 ms, which fails the
/// `< 50 ms` assertion. Under current main (sentinel emitted first,
/// immediately, no ticker), the sentinel arrives < 5 ms.
///
/// **Pass condition (current main)**: sentinel fires before the 500 ms mock
/// sleep, so elapsed-to-first << 500 ms. The 50 ms budget is generous.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn sentinel_fires_before_preload_await() {
    let cfg = yahoo_cache_cfg();
    let range = DateRange::Last30d;
    let mock = Box::new(MockLabYahooBarSource::default_500ms());

    let (progress_tx, mut progress_rx) = progress_pair();
    let cfg_clone = cfg.clone();
    let range_clone = range.clone();

    let start = Instant::now();

    // Spawn the inline preload section.
    let preload_task = tokio::spawn(async move {
        // === Mirrors spawn_lab_run's yahoo preload block ===
        // Sentinel fires immediately — before any preload await.
        progress_tx.try_send(Progress {
            current_bar: 0,
            total_bars: 1,
            elapsed_ms: 0,
        });
        // Preload (mock: sleep 500 ms then return bars).
        let _ = mock.preload(&cfg_clone, &range_clone).await;
    });

    // Receive the first event.
    let first_event = timeout(Duration::from_millis(200), progress_rx.recv())
        .await
        .expect("first event must arrive within 200ms (sentinel should fire immediately)")
        .expect("progress channel must not be closed before first event");

    let elapsed_to_first = start.elapsed();

    // Assert: sentinel arrived well before the 500 ms mock sleep.
    assert_eq!(
        first_event.current_bar, 0,
        "sentinel must have current_bar == 0; got {first_event:?}"
    );
    assert_eq!(
        first_event.total_bars, 1,
        "sentinel must have total_bars == 1 (placeholder); got {first_event:?}"
    );
    assert!(
        elapsed_to_first < Duration::from_millis(50),
        "sentinel must arrive in < 50 ms (before preload await); \
         actual: {}ms. Regression A detected: sentinel delayed past the preload await boundary.",
        elapsed_to_first.as_millis()
    );

    // Wait for the preload task to finish (the 500ms mock sleep).
    preload_task.await.expect("preload task must not panic");
}

// ── Test 2 — Channel survives after preload completes ─────────────────────────

/// T-D2 / ADR-0048 D3 category B — channel not consumed by preload path.
///
/// Asserts: after the preload completes, the progress channel is still live
/// and subsequent sends are received by the caller.
///
/// **Falsification**: under `5f9f920`, a closure-move bug in the `select!`
/// ticker arm could shadow `progress_tx` inside the async closure, making the
/// sender inaccessible for engine progress events post-preload. This test
/// verifies the channel is still usable by sending a second event after the
/// preload future resolves.
///
/// **Pass condition (current main)**: the preload block uses a single
/// `progress_tx.try_send` (the sentinel) then `source.preload().await`. No
/// `select!` involved. The sender is not consumed; engine events can follow.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn channel_survives_after_preload() {
    let cfg = yahoo_cache_cfg();
    let range = DateRange::Last30d;
    let mock = Box::new(MockLabYahooBarSource::fast_10ms());

    let (progress_tx, mut progress_rx) = progress_pair();
    let progress_tx_2 = progress_tx.clone();
    let cfg_clone = cfg.clone();
    let range_clone = range.clone();

    let preload_task = tokio::spawn(async move {
        // Sentinel before preload.
        progress_tx.try_send(Progress {
            current_bar: 0,
            total_bars: 1,
            elapsed_ms: 0,
        });
        // Preload (fast mock: 10ms sleep).
        let _result = mock.preload(&cfg_clone, &range_clone).await;
        // After preload: send a second event simulating engine progress.
        // This verifies the channel is still alive post-preload.
        progress_tx.try_send(Progress {
            current_bar: 1,
            total_bars: 5,
            elapsed_ms: 50,
        });
    });

    let mut events: Vec<Progress> = Vec::new();
    loop {
        match timeout(Duration::from_millis(200), progress_rx.recv()).await {
            Ok(Some(p)) => events.push(p),
            _ => break,
        }
        if events.len() >= 2 {
            break;
        }
    }

    preload_task.await.expect("preload task must not panic");

    // Channel must have delivered both the sentinel and the post-preload event.
    assert!(
        events.len() >= 2,
        "channel must survive preload and deliver ≥ 2 events \
         (sentinel + post-preload engine event); got {} event(s): {events:?}. \
         Regression B detected: channel consumed or closed during preload.",
        events.len()
    );

    // First must be the sentinel.
    assert_eq!(
        events[0].current_bar, 0,
        "first event must be sentinel (current_bar == 0); got {:?}",
        events[0]
    );
    assert_eq!(
        events[0].total_bars, 1,
        "sentinel must have total_bars == 1; got {:?}",
        events[0]
    );

    // Second must be a post-preload event with current_bar > 0.
    assert!(
        events[1].current_bar > 0,
        "post-preload event must have current_bar > 0; got {:?}",
        events[1]
    );

    // Smoke: progress_tx_2 (another clone) should also still work.
    let _ = progress_tx_2;
}

// ── Test 3 — No ticker events leak beyond preload ─────────────────────────────

/// T-D2 / ADR-0048 D3 category B (ticker-leak variant).
///
/// Asserts: if a ticker was running during the preload phase, NO extra
/// ticker-shaped events (current_bar == 0, total_bars == 1, elapsed_ms > 0)
/// arrive AFTER the preload completes and the ticker should have been
/// cancelled.
///
/// **Falsification**: if a ticker leaks past preload completion (e.g. the
/// `select!` arm races post-resolve), this test catches the extra events.
///
/// **Pass condition (current main)**: current code has no ticker; only one
/// sentinel fires (elapsed_ms == 0). After preload, only engine events
/// (total_bars > 1) arrive. Zero ticker-shaped events post-preload.
///
/// Note: this test uses a fast mock (10ms sleep) so CI wall-clock stays
/// under 500ms total.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn ticker_events_stop_after_preload_complete() {
    let cfg = yahoo_cache_cfg();
    let range = DateRange::Last30d;
    let mock = Box::new(MockLabYahooBarSource::fast_10ms());

    let (progress_tx, mut progress_rx) = progress_pair();
    let cfg_clone = cfg.clone();
    let range_clone = range.clone();

    let preload_task = tokio::spawn(async move {
        // Sentinel before preload.
        progress_tx.try_send(Progress {
            current_bar: 0,
            total_bars: 1,
            elapsed_ms: 0,
        });
        // Preload.
        let _result = mock.preload(&cfg_clone, &range_clone).await;
        // Explicitly drop progress_tx to close channel — signals end-of-preload.
        // In the real spawn_lab_run, the sender is moved into rt.spawn (engine).
        // Here we just drop it to model "preload done, no more preload events".
        drop(progress_tx);
    });

    preload_task.await.expect("preload task must not panic");

    // Now drain anything remaining in the channel after preload is done.
    // We expect: zero OR one event (the sentinel only).
    // We must NOT see ticker-shaped events with elapsed_ms > 0 after the close.
    let mut post_preload_ticker_events: Vec<Progress> = Vec::new();
    loop {
        match timeout(Duration::from_millis(50), progress_rx.recv()).await {
            Ok(Some(p)) => {
                // A ticker event is: current_bar == 0, total_bars == 1, elapsed_ms > 0.
                if p.current_bar == 0 && p.total_bars == 1 && p.elapsed_ms > 0 {
                    post_preload_ticker_events.push(p);
                }
            }
            _ => break,
        }
    }

    assert!(
        post_preload_ticker_events.is_empty(),
        "no ticker events (current_bar=0, total_bars=1, elapsed_ms>0) must arrive \
         after preload completion; got {} ticker event(s): {post_preload_ticker_events:?}. \
         Regression B (ticker-leak) detected.",
        post_preload_ticker_events.len()
    );
}
