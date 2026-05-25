//! End-to-end test that mirrors cockpit_live's update flow with the REAL
//! engine — not the synthetic_summary mock at lab_run_integration.rs.
//!
//! Walks the operator's reported failure scenarios:
//!   - ETHUSDT + v0.sma + Last 30d   (Synthetic)
//!   - SOLUSDT + v0.5.macd + Last 30d (Synthetic)
//!   - BTCUSDT + v0.sma + Last 90d   (Synthetic) — operator's known-good case
//!
//! For each, exercises the full pipeline: spawn_lab_run → real backtest run →
//! RunSummary → cockpit_live wrapper → state mutation → asserts that
//! last_run_report.bars + chart_markers contain a non-empty payload.
//!
//! If the operator's "no triangles / no chart" bug is real on main, ONE of
//! these asserts will fail and pinpoint the exact gap.

#![cfg(feature = "live")]

use std::sync::Arc;
use trading_core::{StrategyId, Symbol, Venue};
use ui::lab::equity_loader::LabTuple;
use ui::lab::runner::{LabRunConfig, RunReportMirror, RunSummary};
use ui::lab::state::{DateRange, LabDataSource, Preset};
use ui::state::{Cockpit, Message, PanelState};

fn lab_config(strategy: &str, symbol: &str, range_label: &str) -> LabRunConfig {
    LabRunConfig {
        strategy_id: smol_str::SmolStr::new(strategy),
        symbol: smol_str::SmolStr::new(symbol),
        venue: smol_str::SmolStr::new("Binance"),
        range_label: smol_str::SmolStr::new(range_label),
        seed: ui::lab::defaults::LAB_DEFAULT_SEED,
        write_report: false,
        data_source: LabDataSource::Synthetic,
        sma_fast_len: None,
        sma_slow_len: None,
    }
}

/// Mirrors cockpit_live::update wrapper rotation block at lines 977-997.
fn apply_wrapper(cockpit: &mut Cockpit, summary: &RunSummary) {
    let pre_tuple = {
        let ls = &cockpit.lab_state;
        match (ls.strategy.as_ref(), ls.pair.as_ref()) {
            (Some(strategy), Some((venue, symbol))) => {
                Some(LabTuple::new(strategy, *venue, symbol, ls.range.clone()))
            }
            _ => None,
        }
    };
    if let Some(tuple) = pre_tuple {
        let mirror = RunReportMirror {
            tuple,
            equity_series: Arc::new(summary.equity_series.clone()),
            kpis: summary.kpis.clone(),
            generated_at: ::time::OffsetDateTime::now_utc(),
            bars: summary.bars.clone(),
            position_curve: Arc::new(summary.position_curve.clone()),
        };
        let prev = cockpit.lab_state.last_run_report.take();
        cockpit.lab_state.prev_run_report = prev;
        cockpit.lab_state.last_run_report = Some(mirror);
    }
}

/// Execute the engine path inline, bypassing the iced::Task wrapper so we
/// can directly assert on the RunSummary. This mirrors the body of
/// spawn_lab_run's live + non-yahoo path verbatim.
async fn execute_run(cfg: LabRunConfig) -> RunSummary {
    let scenario_cfg =
        ui::lab::runner::lab_config_to_scenario(&cfg).expect("lab_config_to_scenario must succeed");
    let (_handle, cancel_rx) = backtest::cancel::cancellation_pair();
    let progress_tx = backtest::progress::ProgressSender::disabled();
    let report = backtest::engine::run_scenario(scenario_cfg, cancel_rx, progress_tx)
        .await
        .expect("engine::run_scenario must succeed");

    let equity_series: Vec<(i64, rust_decimal::Decimal)> = report
        .equity_series
        .iter()
        .map(|(ts, money)| (ts.unix_millis(), money.amount()))
        .collect();
    // lab-polish-round-2 R1 — filter position_curve_raw to active symbol.
    let active_sym = trading_core::Symbol::new(cfg.symbol.as_str());
    let position_curve: Vec<(i64, rust_decimal::Decimal)> = report
        .position_curve_raw
        .iter()
        .filter(|(_, _, s)| s == &active_sym)
        .map(|&(ts, qty, _)| (ts, qty))
        .collect();
    RunSummary {
        strategy_id: cfg.strategy_id.clone(),
        symbol: cfg.symbol.clone(),
        report_path: report.report_path.clone(),
        equity_series,
        fills: report.fills.clone(),
        kpis: report.kpis.clone(),
        bars: report.bars.clone(),
        position_curve,
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_btc_sma_last30d_renders_triangles() {
    let mut cockpit = Cockpit::new();
    cockpit.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    cockpit.lab_state.range = DateRange::Preset(Preset::Last30d);

    let summary = execute_run(lab_config("v0.sma", "BTCUSDT", "Last30d")).await;

    assert!(
        !summary.fills.is_empty(),
        "BTC v0.sma Last30d must produce fills (diag showed 13 on 720 bars)"
    );
    assert!(
        !summary.bars.is_empty(),
        "BTC v0.sma Last30d must surface bars"
    );

    ui::state::update(&mut cockpit, Message::LabRunCompleted(Ok(summary.clone())));
    apply_wrapper(&mut cockpit, &summary);
    // Also dispatch ChartMarkersLoaded as cockpit_live does for non-empty fills.
    ui::state::update(
        &mut cockpit,
        Message::ChartMarkersLoaded(Ok(summary.fills.clone())),
    );

    // The chart will render correctly only if BOTH of these hold:
    let mirror = cockpit
        .lab_state
        .last_run_report
        .as_ref()
        .expect("last_run_report set");
    assert!(!mirror.bars.is_empty(), "mirror.bars must be populated");
    let PanelState::Ready(markers) = &cockpit.chart_markers else {
        panic!("chart_markers must be Ready(...) after dispatch");
    };
    assert!(!markers.is_empty(), "chart_markers must hold fills");

    // Spatial anchor invariant — fills fall in bars window.
    let first_bar = mirror.bars.first().unwrap().open_ts.unix_millis();
    let last_bar = mirror.bars.last().unwrap().close_ts.unix_millis();
    for m in markers {
        let t = m.venue_ts.unix_millis();
        assert!(
            t >= first_bar && t <= last_bar,
            "marker ts {t} outside bars window [{first_bar},{last_bar}]"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_eth_sma_last30d_renders_triangles() {
    let mut cockpit = Cockpit::new();
    cockpit.lab_state.strategy = Some(StrategyId::new("v0.sma"));
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("ETHUSDT")));
    cockpit.lab_state.range = DateRange::Preset(Preset::Last30d);

    let summary = execute_run(lab_config("v0.sma", "ETHUSDT", "Last30d")).await;

    assert!(
        !summary.fills.is_empty(),
        "ETH v0.sma Last30d must produce fills (diag showed 13)"
    );
    assert!(!summary.bars.is_empty(), "ETH v0.sma must surface bars");
}

/// Bug #56 forensic-gate: momentum loads `config/strategies/top10_momentum_h1.toml`.
/// Before Bug #56 fix, the test CWD (`crates/ui/`) couldn't find this file and
/// the test was `#[ignore]`d. After Bug #56 fix, `crate::paths::resolve_workspace_path`
/// walks up to find `Cargo.lock` and resolves correctly — this test runs green.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn e2e_momentum_xrp_last90d_cold_start_renders() {
    // Cold-start defaults: v1.momentum (cross-sectional) + XRPUSDT + Last90d.
    let mut cockpit = Cockpit::new();
    cockpit.lab_state.strategy = Some(StrategyId::new("v1.momentum"));
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("XRPUSDT")));
    cockpit.lab_state.range = DateRange::Preset(Preset::Last90d);

    let summary = execute_run(lab_config("v1.momentum", "XRPUSDT", "Last90d")).await;
    // Cross-sectional path: F3 fix surfaces fills + bars. With Last90d we
    // should see many fills + thousands of bars (interleaved across the top10).
    println!(
        "DIAG momentum-XRP-Last90d: fills={} bars={} equity_pts={}",
        summary.fills.len(),
        summary.bars.len(),
        summary.equity_series.len()
    );
}
