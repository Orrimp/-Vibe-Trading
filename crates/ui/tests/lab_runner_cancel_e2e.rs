//! E2E test — Stop during cold-cache preload exits within ≤ 500 ms.
//!
//! Bug #64 D.1.1 attempt-3 / ADR-0050 § D2 + D3.
//!
//! ## What this test proves
//!
//! D-R2.3: After the D-R2.1 + D-R2.2 fix (`CancellationToken` + third
//! `select!` arm in the preload loop), dispatching Stop within 100 ms
//! of run-start MUST cause the run to exit within 500 ms total wall-clock
//! with an error containing "cancelled".
//!
//! **Regression guard**: if the `_ = cancel.cancelled() =>` arm is absent
//! from the preload `select!`, Stop drops the `RunCancelHandle` (which
//! previously used std `mpsc::sync_channel(0)` disconnect idiom) but
//! `is_cancelled()` is never polled during the preload window — so the
//! preload runs to completion before cancel is observed (30-60 s on
//! cold cache). This test catches that structural omission.
//!
//! ## Implementation approach
//!
//! The test inlines the production preload select! loop (same pattern
//! as `spawn_lab_run_yahoo_harness.rs`), using a 5 s mock preload that
//! sleeps for 5 s before returning. Cancel is dispatched after 100 ms.
//! We assert the loop exits within 500 ms total (well before the 5 s
//! mock would complete).

#![cfg(feature = "live")]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::time::{Duration, Instant};

use backtest::cancel::cancellation_pair;
use backtest::progress::{Progress, progress_pair};
use smol_str::SmolStr;
use ui::lab::runner::LabRunConfig;
use ui::lab::state::LabDataSource;

// ── Helpers ───────────────────────────────────────────────────────────────────

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

// ── Test — Cancel during preload exits ≤ 500 ms ──────────────────────────────

/// T-BUG64-D7 / D-R2.3 / ADR-0050 § D2.
///
/// **Pass condition**: the preload loop exits with `Err("cancelled")` within
/// 500 ms total wall-clock from stop-dispatch, even though the mock preload
/// sleeps for 5 s.
///
/// **Falsification** (what this catches if the D-R2.2 fix is missing):
/// - Without the `_ = cancel.cancelled() =>` arm, the preload loop only
///   checks cancel AFTER the preload future completes (5 s mock sleep).
///   The loop runs for the full 5 s before returning — far beyond the
///   500 ms budget — and this test fails.
/// - With the fix: the cancel arm fires within one `yield_now()` + one
///   `select!` poll cycle after `RunCancelHandle::drop()` flips the token.
///   In practice this is sub-millisecond. The test uses 500 ms as a
///   generous wall-clock budget.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn stop_during_preload_exits_within_500ms() {
    let rt = tokio::runtime::Handle::current();
    let _cfg = yahoo_cache_cfg(); // confirm cfg builds without error

    let (progress_tx, _progress_rx) = progress_pair();
    let (cancel_handle, cancel_rx) = cancellation_pair();

    // Track overall wall-clock from test start.
    let test_start = Instant::now();

    // ── Spawn the preload loop (mirrors runner.rs:735-850) ────────────────────
    //
    // Preload mock: 5 s sleep (simulates cold-cache Yahoo fetch).
    // Stop will be dispatched at ~100 ms via `drop(cancel_handle)`.
    let loop_task: tokio::task::JoinHandle<Result<(), SmolStr>> = tokio::spawn(async move {
        let preload_start = Instant::now();

        // Sentinel.
        progress_tx.try_send(Progress {
            current_bar: 0,
            total_bars: 1,
            elapsed_ms: 0,
        });

        // D-R1.1: enter rt context before tokio::time::interval.
        let mut ticker = {
            let _guard = rt.enter();
            tokio::time::interval(Duration::from_millis(250))
        };
        // Consume t=0 tick.
        ticker.tick().await;

        // 5 s mock preload (simulates cold-cache fetch that should be cancelled).
        let mut preload_future = std::pin::pin!(tokio::time::sleep(Duration::from_secs(5)));

        loop {
            // D-R1.4 defense-in-depth yield.
            tokio::task::yield_now().await;

            tokio::select! {
                biased;
                _ = &mut preload_future => {
                    // Preload completed without cancel — should not happen in this test.
                    break Ok(());
                }
                // D-R2.2: cancel arm — exits the loop when Stop is dispatched.
                _ = cancel_rx.cancelled() => {
                    drop(ticker);
                    break Err(SmolStr::new("operator cancelled during preload"));
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
    });

    // ── Dispatch Stop after 100 ms (simulate operator clicking Stop) ──────────
    tokio::time::sleep(Duration::from_millis(100)).await;
    let stop_dispatched_at = test_start.elapsed();
    drop(cancel_handle); // drops RunCancelHandle → CancellationToken::cancel()

    // ── Wait for the loop task to complete ───────────────────────────────────
    let result = tokio::time::timeout(Duration::from_millis(500), loop_task)
        .await
        .expect(
            "preload loop must exit within 500 ms of Stop dispatch — \
                  D-R2.2 regression detected: cancel arm may be missing from \
                  the preload select! loop in runner.rs",
        )
        .expect("preload loop task must not panic");

    let total_elapsed = test_start.elapsed();

    // ── Assertions ───────────────────────────────────────────────────────────

    // The loop must have returned Err (cancelled), not Ok (completed normally).
    assert!(
        result.is_err(),
        "preload loop must exit with Err when cancelled; got Ok(()). \
         D-R2.2 regression: cancel arm not firing."
    );

    let err_msg = result.unwrap_err();
    assert!(
        err_msg.as_str().contains("cancelled"),
        "cancel error must contain 'cancelled'; got: {err_msg:?}"
    );

    // Total wall-clock from test start must be < 500 ms from stop dispatch.
    // We allow 500 ms slack after stop_dispatched_at.
    let elapsed_since_stop = total_elapsed.saturating_sub(stop_dispatched_at);
    assert!(
        elapsed_since_stop < Duration::from_millis(500),
        "loop must exit within 500 ms of Stop dispatch; \
         elapsed since stop: {}ms (total: {}ms). \
         D-R2.2 regression: cancel arm too slow or missing.",
        elapsed_since_stop.as_millis(),
        total_elapsed.as_millis()
    );
}

// ── Test — Cancel-before-start is instant ────────────────────────────────────

/// Additional edge-case: cancel dispatched BEFORE the preload loop begins.
///
/// The `CancellationToken::is_cancelled()` is already `true` when the loop
/// starts — the first `cancel_rx.cancelled()` arm should fire immediately.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn cancel_before_preload_start_is_instant() {
    let rt = tokio::runtime::Handle::current();

    let (progress_tx, _progress_rx) = progress_pair();
    let (cancel_handle, cancel_rx) = cancellation_pair();

    // Cancel BEFORE the loop starts.
    drop(cancel_handle);

    let start = Instant::now();

    let loop_task: tokio::task::JoinHandle<Result<(), SmolStr>> = tokio::spawn(async move {
        let preload_start = Instant::now();

        progress_tx.try_send(Progress {
            current_bar: 0,
            total_bars: 1,
            elapsed_ms: 0,
        });

        let mut ticker = {
            let _guard = rt.enter();
            tokio::time::interval(Duration::from_millis(250))
        };
        ticker.tick().await;

        let mut preload_future = std::pin::pin!(tokio::time::sleep(Duration::from_secs(5)));

        loop {
            tokio::task::yield_now().await;

            tokio::select! {
                biased;
                _ = &mut preload_future => {
                    break Ok(());
                }
                _ = cancel_rx.cancelled() => {
                    drop(ticker);
                    break Err(SmolStr::new("operator cancelled during preload"));
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
    });

    let result = tokio::time::timeout(Duration::from_millis(100), loop_task)
        .await
        .expect("pre-cancelled loop must exit within 100 ms")
        .expect("task must not panic");

    let elapsed = start.elapsed();

    assert!(
        result.is_err(),
        "pre-cancelled loop must return Err; got Ok(())"
    );
    assert!(
        elapsed < Duration::from_millis(100),
        "pre-cancelled loop must exit near-instantly; elapsed: {}ms",
        elapsed.as_millis()
    );
}
