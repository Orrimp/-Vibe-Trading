//! Integration test — backtest progress emission (T-D4 / T-D3.7).
//!
//! Verifies:
//! 1. A standard run with a live `ProgressSender` emits at least one
//!    `Progress` event.
//! 2. Each emitted `Progress` has `current_bar <= total_bars`.
//! 3. A run with a pre-dropped cancel handle returns `RunError::Cancelled`.

use backtest::DateRange;
use backtest::cancel::cancellation_pair;
use backtest::engine::{RunError, ScenarioConfig, ScenarioDataSource};
use backtest::progress::{ProgressSender, progress_pair};
use trading_core::{StrategyId, Symbol, Venue};

fn sma_config() -> ScenarioConfig {
    let mut seed = [0u8; 32];
    seed[0] = 0xAB;
    seed[1] = 0xCD;
    ScenarioConfig {
        strategy: StrategyId("sma_crossover".into()),
        pair: (Venue::Binance, Symbol::new("BTCUSDT")),
        range: DateRange::Last30d,
        params: None,
        seed,
        write_report: false,
        data_source: ScenarioDataSource::default(),
        bars_override: None,
    }
}

/// T-D4 — a live run emits at least one Progress event with valid fields.
#[tokio::test]
async fn progress_events_are_emitted() {
    let (_handle, cancel_rx) = cancellation_pair();
    let (progress_tx, mut progress_rx) = progress_pair();

    let handle = tokio::spawn(async move {
        backtest::engine::run_scenario(sma_config(), cancel_rx, progress_tx)
            .await
            .expect("run should succeed")
    });

    let mut received: Vec<backtest::progress::Progress> = Vec::new();
    // Drain all progress messages until the sender is dropped (channel closed).
    while let Some(p) = progress_rx.recv().await {
        received.push(p);
    }

    let _report = handle.await.expect("task should not panic");

    assert!(
        !received.is_empty(),
        "expected at least one Progress event but received none"
    );

    for p in &received {
        assert!(
            p.current_bar <= p.total_bars,
            "current_bar ({}) must not exceed total_bars ({})",
            p.current_bar,
            p.total_bars
        );
        assert!(
            p.total_bars > 0,
            "total_bars must be > 0 in emitted Progress events"
        );
    }
}

/// T-D3.7 — disabled ProgressSender does not block or panic.
#[tokio::test]
async fn disabled_progress_sender_is_noop() {
    let (_handle, cancel_rx) = cancellation_pair();
    let progress_tx = ProgressSender::disabled();

    let result = backtest::engine::run_scenario(sma_config(), cancel_rx, progress_tx).await;
    assert!(
        result.is_ok(),
        "disabled progress sender should not break run"
    );
}

/// T-D3.7 — cancel while run is in-flight returns `Cancelled`.
#[tokio::test]
async fn pre_dropped_handle_returns_cancelled() {
    let (handle, cancel_rx) = cancellation_pair();
    let progress_tx = ProgressSender::disabled();
    drop(handle);

    let result = backtest::engine::run_scenario(sma_config(), cancel_rx, progress_tx).await;
    assert!(
        matches!(result, Err(RunError::Cancelled)),
        "expected Cancelled, got: {result:?}"
    );
}
