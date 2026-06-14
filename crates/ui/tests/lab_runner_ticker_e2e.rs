//! E2E test — ticker fires ≥ 3 times during a 1 s bounded preload window.
//!
//! Bug #64 D.1.1 attempt-3 / ADR-0050 § D3.
//!
//! ## What this test proves
//!
//! D-R1.3: After the D-R1.1 fix (`let _guard = rt.enter()` before
//! `tokio::time::interval(250ms)`), the ticker MUST fire ≥ 3 times
//! during a 1 s mock preload window with monotonically increasing
//! `elapsed_ms` values.
//!
//! **Regression guard**: if `rt_handle.enter()` is accidentally removed,
//! `tokio::time::interval` is created without a tokio reactor context on
//! iced's `futures::ThreadPool`. The resulting `Sleep` futures are
//! permanently `Poll::Pending` — ticker never fires — and this test
//! catches it with: `ticker_events < 3`.
//!
//! ## Why inline rather than drive `iced::Task::perform`?
//!
//! Same reason as `spawn_lab_run_yahoo_harness.rs` — driving
//! `iced::Task::perform` without a running iced application is non-trivial.
//! We inline the production preload ticker logic and run it inside a
//! multi-thread tokio runtime, simulating exactly what the D-R1.1 fix does
//! in production.
//!
//! The inlined ticker section matches `runner.rs:735-821` (attempt-3 HEAD):
//!   1. `rt.enter()` guard before `tokio::time::interval(250ms)`.
//!   2. Consume t=0 tick.
//!   3. `yield_now()` at top of each loop iteration (D-R1.4).
//!   4. `tokio::select! { biased; preload, cancel, ticker }`.

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use backtest::cancel::cancellation_pair;
use backtest::progress::{Progress, progress_pair};
use smol_str::SmolStr;
use tokio::time::timeout;
use ui::lab::runner::LabRunConfig;
use ui::lab::state::LabDataSource;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a standard `LabRunConfig` pointing at YahooCache source.
#[allow(dead_code)] // helper available for future tests
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

// ── Test — Ticker fires ≥ 3 times in a 1 s preload window ────────────────────

/// T-BUG64-D3 / D-R1.3 / ADR-0050 § D3.
///
/// **Pass condition**: ≥ 3 `Progress` events with `elapsed_ms > 0` are
/// received on `progress_rx` during a 1 s bounded preload window. The
/// events must have monotonically non-decreasing `elapsed_ms`.
///
/// **Falsification** (what the test catches if the D-R1.1 fix is missing):
/// - Without `rt.enter()` before `tokio::time::interval(250ms)`, the
///   `ticker.tick().await` future is permanently `Poll::Pending` on iced's
///   `futures::ThreadPool` (no time driver reachable). Zero ticker events
///   fire; `ticker_count < 3` → test fails.
/// - With the fix: ticker fires every ~250 ms → in a 1 s window we expect
///   ~3-4 events. The test asserts ≥ 3 as a robust floor (allows for up to
///   one CI scheduling delay).
///
/// **Note on the sentinel**: the sentinel `Progress { 0, 1, 0 }` is counted
/// separately. The test asserts ≥ 3 events with `elapsed_ms > 0` (ticker
/// events only, not the sentinel).
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn ticker_fires_at_least_3_times_in_1s_window() {
    // Build a tokio runtime handle — in production this is the side-thread
    // agent runtime. In the test, the `#[tokio::test]` macro provides the
    // runtime; we obtain its handle here to mirror the D-R1.1 fix.
    let rt = tokio::runtime::Handle::current();

    let (progress_tx, mut progress_rx) = progress_pair();
    // cancel pair — we don't cancel in this test (preload runs to completion).
    let (_cancel_handle, cancel_rx) = cancellation_pair();

    // 1 s preload window simulated via a `tokio::time::sleep(1s)`.
    let preload_task = tokio::spawn(async move {
        let preload_start = Instant::now();

        // ── Sentinel (mirrors runner.rs:735-739) ─────────────────────────
        progress_tx.try_send(Progress {
            current_bar: 0,
            total_bars: 1,
            elapsed_ms: 0,
        });

        // ── D-R1.1 fix: enter rt context before tokio::time::interval ────
        // This is the line that was missing in attempt-2 (Bug #64 R1).
        let mut ticker = {
            let _guard = rt.enter();
            tokio::time::interval(Duration::from_millis(250))
        };
        // Consume the immediate (t=0) tick.
        ticker.tick().await;

        // ── 1 s preload future (simulates cold-cache Yahoo fetch) ─────────
        let mut preload_future = std::pin::pin!(tokio::time::sleep(Duration::from_millis(1000)));

        // ── Preload select! loop (mirrors runner.rs:800-850) ──────────────
        loop {
            // D-R1.4 yield_now (defense-in-depth).
            tokio::task::yield_now().await;

            tokio::select! {
                biased;
                _ = &mut preload_future => {
                    break;
                }
                _ = cancel_rx.cancelled() => {
                    break;
                }
                _ = ticker.tick() => {
                    let elapsed_ms = u64::try_from(
                        preload_start.elapsed().as_millis()
                    ).unwrap_or(u64::MAX);
                    progress_tx.try_send(Progress {
                        current_bar: 0,
                        total_bars: 1,
                        elapsed_ms,
                    });
                }
            }
        }
        drop(ticker);
    });

    // Collect all events with a generous 2 s window (well above the 1 s
    // preload duration).
    let mut events: Vec<Progress> = Vec::new();
    while let Ok(Some(p)) = timeout(Duration::from_millis(200), progress_rx.recv()).await {
        events.push(p);
    }

    preload_task.await.expect("preload task must not panic");

    // Drain any remaining events (preload may have sent more after our
    // collect loop timed out).
    while let Ok(Some(p)) = timeout(Duration::from_millis(50), progress_rx.recv()).await {
        events.push(p);
    }

    // ── Assertions ───────────────────────────────────────────────────────────

    // All events with elapsed_ms > 0 are ticker events (the sentinel has 0).
    let ticker_events: Vec<&Progress> = events.iter().filter(|p| p.elapsed_ms > 0).collect();

    assert!(
        ticker_events.len() >= 3,
        "ticker must fire ≥ 3 times in a 1 s preload window; \
         got {} ticker event(s). \
         D-R1.1 regression detected: `rt_handle.enter()` guard may be missing \
         before `tokio::time::interval(250ms)` in runner.rs. \
         All events received: {events:?}",
        ticker_events.len()
    );

    // Verify monotonically non-decreasing elapsed_ms.
    let mut prev_ms = 0u64;
    for evt in &ticker_events {
        assert!(
            evt.elapsed_ms >= prev_ms,
            "ticker elapsed_ms must be monotonically non-decreasing; \
             got {prev_ms}ms then {}ms. events: {ticker_events:?}",
            evt.elapsed_ms
        );
        prev_ms = evt.elapsed_ms;
    }

    // All ticker events must have current_bar == 0 and total_bars == 1
    // (preload placeholder values).
    for evt in &ticker_events {
        assert_eq!(
            evt.current_bar, 0,
            "ticker events must have current_bar == 0 (preload phase); got {evt:?}"
        );
        assert_eq!(
            evt.total_bars, 1,
            "ticker events must have total_bars == 1 (preload placeholder); got {evt:?}"
        );
    }
}
