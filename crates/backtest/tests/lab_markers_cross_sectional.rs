//! Integration test: cross-sectional fills + bars are surfaced via `engine::run_scenario`.
//!
//! ## Purpose
//!
//! Proves the F3 wiring for cross-sectional scenarios: when a Lab run completes
//! for a momentum strategy, `RunReport.fills` and `RunReport.bars` are both
//! non-empty, and every fill timestamp falls inside the run's bar window.
//!
//! ## Scope
//!
//! Exercises the full `engine::run_scenario` dispatch path for `v1.momentum`,
//! which is the simplest cross-sectional scenario (no candle/realdata features).
//! Does NOT spin up an iced runtime — validates the data contract the UI depends on.
//!
//! ## CWD note
//!
//! The momentum scenario loads `config/strategies/top10_momentum_h1.toml` relative
//! to the process CWD.  This test changes CWD to the workspace root before calling
//! `run_scenario` (resolved via `CARGO_MANIFEST_DIR`).
//!
//! ## Anchor safety
//!
//! This test does NOT write any report files and therefore cannot affect anchors.

#![allow(clippy::unwrap_used)]

use backtest::cancel::cancellation_pair;
use backtest::engine::{DateRange, ScenarioConfig, ScenarioDataSource};
use backtest::progress::ProgressSender;
use trading_core::{StrategyId, Symbol, Venue};

const TEST_SEED: [u8; 32] = {
    let mut s = [0u8; 32];
    s[0] = 0xC0;
    s[1] = 0xFF;
    s[2] = 0xEE;
    s
};

/// Workspace root path, resolved once from CARGO_MANIFEST_DIR.
fn workspace_root() -> std::path::PathBuf {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    std::path::Path::new(manifest_dir)
        .parent()
        .and_then(|p| p.parent())
        .expect("locate workspace root from CARGO_MANIFEST_DIR")
        .to_path_buf()
}

/// Build a minimal `ScenarioConfig` for the v1.momentum scenario.
fn momentum_config() -> ScenarioConfig {
    ScenarioConfig {
        strategy: StrategyId("v1.momentum".into()),
        pair: (Venue::Binance, Symbol::new("BTCUSDT")),
        // Last30d → 720 hourly bars — fast enough for CI.
        range: DateRange::Last30d,
        params: None,
        seed: TEST_SEED,
        write_report: false,
        data_source: ScenarioDataSource::Synthetic,
        bars_override: None,
        sma_fast_len: None,
        sma_slow_len: None,
        latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
        reports_dir: None,
        short_enabled: false,
    }
}

/// Run v1.momentum, with CWD set to the workspace root so the strategy TOML resolves.
async fn run_momentum() -> backtest::engine::RunReport {
    let root = workspace_root();
    std::env::set_current_dir(&root).unwrap_or_else(|e| panic!("set_current_dir({root:?}): {e}"));

    let (_handle, cancel_rx) = cancellation_pair();
    backtest::engine::run_scenario(momentum_config(), cancel_rx, ProgressSender::disabled())
        .await
        .expect("v1.momentum run must succeed")
}

/// F3 — momentum fills and bars are both non-empty after the wiring.
#[tokio::test]
async fn momentum_fills_and_bars_are_surfaced() {
    let report = run_momentum().await;

    // Bars must be surfaced (was `Arc::new(Vec::new())` before F3).
    assert!(
        !report.bars.is_empty(),
        "report.bars must be non-empty for v1.momentum after F3; got 0"
    );

    // At least one fill so the anchor test is meaningful.
    assert!(
        !report.fills.is_empty(),
        "report.fills must be non-empty for v1.momentum on 720 bars after F3; got 0"
    );

    let first_bar_open_ms = report.bars.first().unwrap().open_ts.unix_millis();
    let last_bar_close_ms = report.bars.last().unwrap().close_ts.unix_millis();

    // Every fill timestamp must fall inside the bar window.
    for (i, fill) in report.fills.iter().enumerate() {
        let fill_ts_ms = fill.venue_ts.unix_millis();
        assert!(
            fill_ts_ms >= first_bar_open_ms,
            "fill[{i}] ts {fill_ts_ms} is before first bar open {first_bar_open_ms}"
        );
        assert!(
            fill_ts_ms <= last_bar_close_ms,
            "fill[{i}] ts {fill_ts_ms} is after last bar close {last_bar_close_ms}"
        );
    }
}

/// Determinism: two identical v1.momentum runs produce identical fills + bars counts.
#[tokio::test]
async fn momentum_fills_bars_are_deterministic() {
    let r1 = run_momentum().await;
    let r2 = run_momentum().await;

    assert_eq!(
        r1.bars.len(),
        r2.bars.len(),
        "bars count must be deterministic across identical runs"
    );
    assert_eq!(
        r1.fills.len(),
        r2.fills.len(),
        "fills count must be deterministic across identical runs"
    );
}
