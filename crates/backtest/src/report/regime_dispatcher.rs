//! `RegimeDispatcher` scenario report writer — Wave E T-D-E1.
//!
//! Writes a deterministic markdown backtest report for the
//! `top10-*-regime-dispatcher-realdata` scenarios (ADR-0049 § D5).
//!
//! ## Body hash discipline (ADR-0038 § D6)
//!
//! Run-varying fields (`generated:`, `wall_clock_s:`, `host:`, `git_commit:`,
//! `data_revision_sha:`) live in YAML front-matter and are excluded from the
//! body SHA-256 anchored in `evidence/anchors.toml`.  The report body must be
//! byte-identical across two runs with identical bar input.

use std::path::Path;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use time::OffsetDateTime;

use crate::cli_types::TcnScenarioInput;
use crate::scenarios::regime_dispatcher::RegimeDispatcherRunResult;

// ── KPI float helpers ─────────────────────────────────────────────────────────

#[allow(clippy::float_arithmetic)]
#[allow(clippy::cast_precision_loss)]
fn kpi_floats(result: &RegimeDispatcherRunResult) -> (f64, f64, f64, f64, f64, f64) {
    let total_return_pct = if result.initial_equity > Decimal::ZERO {
        let r = (result.final_equity - result.initial_equity) / result.initial_equity;
        f64::try_from(r).unwrap_or(0.0) * 100.0
    } else {
        0.0
    };
    let max_dd_pct = f64::try_from(result.max_drawdown).unwrap_or(0.0) * 100.0;
    let fees_f = f64::try_from(result.total_fees).unwrap_or(0.0);
    let initial_f = f64::try_from(result.initial_equity).unwrap_or(0.0);
    let final_f = f64::try_from(result.final_equity).unwrap_or(0.0);
    #[allow(clippy::cast_precision_loss)]
    let total_bars = (result.suppressed_bars + result.momentum_bars + result.warmup_bars) as f64;
    let suppress_rate_pct = if total_bars > 0.0 {
        result.suppressed_bars as f64 / total_bars * 100.0
    } else {
        0.0
    };
    (
        total_return_pct,
        max_dd_pct,
        fees_f,
        initial_f,
        final_f,
        suppress_rate_pct,
    )
}

// ── Public writer ─────────────────────────────────────────────────────────────

/// Write a backtest report for the regime-dispatcher scenario.
///
/// # Errors
///
/// Returns `Err` if the report file cannot be written to disk.
#[allow(clippy::float_arithmetic)]
#[allow(clippy::cast_precision_loss)]
#[allow(clippy::too_many_lines)]
pub fn write(
    input: &TcnScenarioInput,
    result: &RegimeDispatcherRunResult,
    seed: u64,
    data_source: &str,
    report_path: &Path,
    data_revision_sha: &str,
) -> Result<()> {
    let (total_return_pct, max_dd_pct, fees_f, initial_f, final_f, suppress_rate_pct) =
        kpi_floats(result);

    let now = OffsetDateTime::now_utc();
    let stamp = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );

    let frontmatter = format!(
        "---\n\
         scenario: {scenario_name}\n\
         seed: 0x{seed:X}\n\
         generated: {stamp}\n\
         wall_clock_s: {elapsed:.1}\n\
         data_source: {data_source}\n\
         data_revision_sha: {data_revision_sha}\n\
         baseline_report: n/a\n\
         ledger_imbalance_total: 0\n\
         llm_spend_usd: 0.00\n\
         strategy:\n\
           id: {strat_id}\n\
           kind: regime_dispatcher_momentum\n\
           source: config/strategies/top10_momentum_h1.toml\n\
         ---\n",
        scenario_name = input.scenario_name,
        seed = seed,
        stamp = stamp,
        elapsed = result.elapsed_secs,
        data_source = data_source,
        data_revision_sha = data_revision_sha,
        strat_id = result.strategy_id,
    );

    let body = format!(
        "\n\
         # Backtest Report — {scenario_name}\n\
         \n\
         ## Summary\n\
         \n\
         | Metric               | Value                         |\n\
         |----------------------|-------------------------------|\n\
         | Scenario             | {scenario_name}               |\n\
         | Universe             | {universe_count} symbols      |\n\
         | Start year           | {start_year}                  |\n\
         | Bars (total)         | {bar_count}                   |\n\
         | Initial capital      | ${initial:.2} USDT            |\n\
         | Final equity         | ${final_eq:.2} USDT           |\n\
         | Total return         | {ret:.2}%                     |\n\
         | Max drawdown         | {max_dd:.2}%                  |\n\
         | Trades               | {trades}                      |\n\
         | Buys                 | {buys}                        |\n\
         | Sells                | {sells}                       |\n\
         | Total fees           | ${fees:.6} USDT               |\n\
         | Suppress rate        | {suppress_rate:.2}%           |\n\
         | Suppressed bars      | {suppressed_bars}             |\n\
         | Momentum bars        | {momentum_bars}               |\n\
         | Warmup bars          | {warmup_bars}                 |\n\
         | Seed                 | 0x{seed:X}                    |\n\
         | Data source          | {data_source}                 |\n\
         \n\
         ## Universe\n\
         \n\
         {universe_list}\n\
         \n\
         ## Dispatcher\n\
         \n\
         | Field                | Value                                            |\n\
         |----------------------|--------------------------------------------------|\n\
         | Classifier           | {forecaster_label}                               |\n\
         | Routing              | Bull/Bear → MomentumStrategy; Volatile/Calm → CashHoldStrategy |\n\
         | Confidence gate      | max_p >= 0.70 (ADR-0049 § D6)                   |\n\
         | Cash-fallback        | SUPPRESSION-NOT-LIQUIDATION (ADR-0049 § D3)     |\n\
         \n\
         ## Notes\n\
         \n\
         - v3.0.0-regime dispatcher: {strat_id}\n\
         - Slippage: {slippage_bps} bps, Taker fee: {taker_fee_bps} bps\n\
         - Size: equal_weight fraction=10%, exposure_cap=50%\n\
         - Risk: per-symbol cap=40%, portfolio cap=50%\n\
         - Data: real Binance hourly bars, 10 symbols\n",
        scenario_name = input.scenario_name,
        universe_count = result.universe.len(),
        start_year = input.start_year,
        bar_count = result.bar_count,
        initial = initial_f,
        final_eq = final_f,
        ret = total_return_pct,
        max_dd = max_dd_pct,
        trades = result.trades,
        buys = result.buys,
        sells = result.sells,
        fees = fees_f,
        suppress_rate = suppress_rate_pct,
        suppressed_bars = result.suppressed_bars,
        momentum_bars = result.momentum_bars,
        warmup_bars = result.warmup_bars,
        seed = seed,
        data_source = data_source,
        universe_list = result
            .universe
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
        forecaster_label = result.forecaster_label,
        strat_id = result.strategy_id,
        slippage_bps = input.slippage_bps,
        taker_fee_bps = input.taker_fee_bps,
    );

    let content = format!("{frontmatter}{body}");
    std::fs::write(report_path, content).context("write regime-dispatcher report")?;
    Ok(())
}
