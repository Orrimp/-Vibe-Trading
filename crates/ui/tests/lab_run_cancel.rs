//! Integration test — lab run cancellation round-trip (T-D3.6 / T-D4.6).
//!
//! Verifies:
//! 1. `state::update(LabRunStopRequested)` is a pure no-op on model state
//!    (the actual cancellation is a side-effect in `cockpit_live.rs`).
//! 2. `state::update(LabRunProgress(p))` stores the progress on `lab_state`.
//! 3. `state::update(LabRunProgressDone)` clears it.
//! 4. Dropping a `RunCancelHandle` causes `is_cancelled()` to return `true`.
//! 5. Engine returns `RunError::Cancelled` when handle is pre-dropped.

/// T-D3.6 — `LabRunStopRequested` is a pure state no-op.
#[test]
fn lab_run_stop_requested_is_noop_on_model() {
    use ui::state::{Cockpit, Message};

    let mut model = Cockpit::default();
    assert!(!model.lab_run_inflight);
    ui::state::update(&mut model, Message::LabRunStopRequested);
    assert!(
        !model.lab_run_inflight,
        "LabRunStopRequested must not change lab_run_inflight"
    );
}

/// T-D4.6 — `LabRunProgress` stores progress; `LabRunProgressDone` clears it.
#[test]
fn lab_run_progress_round_trip() {
    use backtest::progress::Progress;
    use ui::state::{Cockpit, Message};

    let progress = Progress {
        current_bar: 200,
        total_bars: 720,
        elapsed_ms: 500,
    };

    let mut model = Cockpit::default();
    ui::state::update(&mut model, Message::LabRunProgress(progress));
    assert_eq!(
        model.lab_state.run_progress.as_ref().map(|p| p.current_bar),
        Some(200),
        "run_progress must be set after LabRunProgress"
    );

    ui::state::update(&mut model, Message::LabRunProgressDone);
    assert!(
        model.lab_state.run_progress.is_none(),
        "run_progress must be cleared after LabRunProgressDone"
    );
}

/// T-D3.6 — dropping `RunCancelHandle` → `is_cancelled()` returns true.
#[test]
fn dropping_handle_signals_cancel() {
    let (handle, rx) = backtest::cancel::cancellation_pair();
    assert!(!rx.is_cancelled(), "not cancelled before drop");
    drop(handle);
    assert!(rx.is_cancelled(), "cancelled after drop");
}

/// T-D3.6 / T-D3.7 — engine returns `Cancelled` when handle is pre-dropped.
#[tokio::test]
async fn engine_returns_cancelled_when_handle_dropped() {
    use backtest::DateRange;
    use backtest::engine::{RunError, ScenarioConfig, ScenarioDataSource};
    use trading_core::{StrategyId, Symbol, Venue};

    let (handle, cancel_rx) = backtest::cancel::cancellation_pair();
    let progress_tx = backtest::progress::ProgressSender::disabled();

    // Drop the handle *before* running so is_cancelled() returns true
    // on the first poll site hit inside the bar loop.
    drop(handle);

    let mut seed = [0u8; 32];
    seed[0] = 0xC0;
    seed[1] = 0xFF;
    seed[2] = 0xEE;

    let cfg = ScenarioConfig {
        strategy: StrategyId("sma_crossover".into()),
        pair: (Venue::Binance, Symbol::new("BTCUSDT")),
        range: DateRange::Last30d,
        params: None,
        seed,
        write_report: false,
        data_source: ScenarioDataSource::default(),
        bars_override: None,
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
        initial_capital: None,
        composed_toml_override: None,
        dvol_override: None,
        macro_regime_series: None,
    };

    let result = backtest::engine::run_scenario(cfg, cancel_rx, progress_tx).await;
    assert!(
        matches!(result, Err(RunError::Cancelled)),
        "expected Cancelled, got: {result:?}"
    );
}
