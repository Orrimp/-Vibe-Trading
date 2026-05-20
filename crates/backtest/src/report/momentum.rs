//! v1 momentum scenario report writer — Phase B T-D-N3.
//!
//! Extracted from `main.rs::write_momentum_report` @1026. Byte-identical output.

use std::path::Path;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use time::OffsetDateTime;

use crate::cli_types::MomentumScenarioInput;
use crate::scenarios::momentum::MomentumRunResult;

/// Write a backtest report for the momentum scenario.
///
/// Byte-identical to `main.rs::write_momentum_report` @1026.
///
/// # Errors
///
/// Returns `Err` if the report file cannot be written to disk.
// Float arithmetic is used for display-only % and $ values per ADR-0003.
#[allow(clippy::float_arithmetic)]
// Decimal → f64 cast for display; precision loss is acceptable here.
#[allow(clippy::cast_precision_loss)]
pub fn write(
    input: &MomentumScenarioInput,
    result: &MomentumRunResult,
    seed: u64,
    data_source: &str,
    report_path: &Path,
) -> Result<()> {
    let total_return_pct = if result.initial_equity > Decimal::ZERO {
        let r = (result.final_equity - result.initial_equity) / result.initial_equity;
        f64::try_from(r).unwrap_or(0.0) * 100.0
    } else {
        0.0
    };

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

    let max_dd_pct = f64::try_from(result.max_drawdown).unwrap_or(0.0) * 100.0;
    let fees_f = f64::try_from(result.total_fees).unwrap_or(0.0);
    let initial_f = f64::try_from(result.initial_equity).unwrap_or(0.0);
    let final_f = f64::try_from(result.final_equity).unwrap_or(0.0);

    let content = format!(
        "---\n\
         scenario: {scenario_name}\n\
         seed: 0x{seed:X}\n\
         generated: {stamp}\n\
         wall_clock_s: {elapsed:.1}\n\
         data_source: {data_source}\n\
         baseline_report: n/a\n\
         ledger_imbalance_total: 0\n\
         llm_spend_usd: 0.00\n\
         strategy:\n\
           id: {strat_id}\n\
           kind: cross_sectional_momentum\n\
           content_hash: {strat_hash}\n\
           source: config/strategies/top10_momentum_h1.toml\n\
           signal: vol_adjusted_log_return(lookback=60)\n\
         ---\n\
         \n\
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
         | Seed                 | 0x{seed:X}                    |\n\
         | Data source          | {data_source}                 |\n\
         \n\
         ## Universe\n\
         \n\
         {universe_list}\n\
         \n\
         ## Notes\n\
         \n\
         - v1 cross-sectional momentum: {strat_id}\n\
         - Slippage: {slippage_bps} bps, Taker fee: {taker_fee_bps} bps\n\
         - Size: equal_weight, exposure_cap=50%, k_long=3\n\
         - Risk: per-symbol cap=40%, portfolio cap=50%\n\
         - Data: synthetic hourly bars, 10 independent ChaCha20Rng streams\n",
        scenario_name = input.scenario_name,
        seed = seed,
        stamp = stamp,
        elapsed = result.elapsed_secs,
        data_source = data_source,
        strat_id = result.strategy_id,
        strat_hash = result.config_hash_hex,
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
        universe_list = result
            .universe
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
        slippage_bps = input.slippage_bps,
        taker_fee_bps = input.taker_fee_bps,
    );

    std::fs::write(report_path, content).context("write momentum report")?;
    Ok(())
}
