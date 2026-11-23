//! Integration test — lab run cancellation round-trip (T-D3.6 / T-D4.6).
//!
//! Verifies:
//! 1. `state::update(LabRunStopRequested)` is a pure no-op on model state
//!    (the actual cancellation is a side-effect in `cockpit_live.rs`).
//! 2. `state::update(LabRunProgress(p))` stores the progress on `lab_state`.
//! 3. `state::update(LabRunProgressDone)` clears it.
//! 4. Dropping a `RunCancelHandle` causes `is_cancelled()` to return `true`.
//! 5. Engine returns `RunError::Cancelled` when handle is pre-dropped.
//!
//! ## Test 1 is two-sided (story 2-63 Wave E3)
//!
//! Until Wave E3, `lab_run_stop_requested_is_noop_on_model` dispatched
//! `LabRunStopRequested` on a `Cockpit::default()`, whose `lab_run_inflight`
//! is already `false`, and asserted it stayed `false`: an arm that CLEARED the
//! flag passed too. Since Wave E3 the test first arms a run through the
//! production `update` (`LabRunRequested` sets `lab_run_inflight = true`,
//! `LabRunProgress` stores bar 200), asserts both preconditions, and only then
//! dispatches Stop: the flag must stay `true` and the progress must stay
//! `Some(200)` (the Stop arm is documented as "pure state is unchanged
//! (inflight stays true until LabRunCompleted arrives)").
//!
//! ## T-T4 falsification probe table (story 2-63 AC4)
//!
//! Each probe is ONE exact-once replace in PRODUCTION code (`grep -c` of the
//! OLD text is 1), located by symbol + quoted line. Run
//! `cargo test -p ui --test lab_run_cancel`, then restore `crates/ui/src/state.rs`
//! from a `cp -p` snapshot taken immediately before the probe (NEVER from git:
//! the working tree differs from `HEAD`), `cmp` it, `touch` it, re-run GREEN.
//!
//! | # | symbol (file) | quoted line today | mutation | expected RED |
//! |---|---|---|---|---|
//! | E3-RC-P1 | `ui::state::update`, arm `Message::LabRunStopRequested` (`crates/ui/src/state.rs`) | `Message::LabRunStopRequested => {` | append `model.lab_run_inflight = false;` inside the arm | `lab_run_stop_requested_is_noop_on_model` at "LabRunStopRequested must not change lab_run_inflight" |
//! | E3-RC-P2 (optional) | the same arm | the same line | append `model.lab_state.run_progress = None;` inside the arm | the same test at "LabRunStopRequested must not clear lab_state.run_progress" |
//!
//! Vacuity of the pre-E3 test (measured by the E3 base run): it stays GREEN
//! under E3-RC-P1, because it starts from `false`.

/// T-D3.6 — `LabRunStopRequested` is a pure state no-op, observed on an
/// IN-FLIGHT run (both preconditions are produced by the production `update`).
#[test]
fn lab_run_stop_requested_is_noop_on_model() {
    use backtest::progress::Progress;
    use ui::state::{Cockpit, Message};

    let mut model = Cockpit::default();
    ui::state::update(&mut model, Message::LabRunRequested);
    ui::state::update(
        &mut model,
        Message::LabRunProgress(Progress {
            current_bar: 200,
            total_bars: 720,
            elapsed_ms: 500,
        }),
    );
    assert!(
        model.lab_run_inflight,
        "precondition: LabRunRequested arms lab_run_inflight"
    );
    assert_eq!(
        model.lab_state.run_progress.as_ref().map(|p| p.current_bar),
        Some(200),
        "precondition: LabRunProgress stored bar 200"
    );

    ui::state::update(&mut model, Message::LabRunStopRequested);
    assert!(
        model.lab_run_inflight,
        "LabRunStopRequested must not change lab_run_inflight"
    );
    assert_eq!(
        model.lab_state.run_progress.as_ref().map(|p| p.current_bar),
        Some(200),
        "LabRunStopRequested must not clear lab_state.run_progress"
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
