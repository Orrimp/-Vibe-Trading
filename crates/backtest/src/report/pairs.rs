//! v1.5a pairs scenario report writer — Phase B T-D-N4.
//!
//! Extracted from `main.rs::write_pairs_report` @1452. Byte-identical output.

use std::path::Path;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use time::OffsetDateTime;

use crate::cli_types::PairsScenarioInput;
use crate::scenarios::pairs::PairsRunResult;

// ── KPI float helpers ─────────────────────────────────────────────────────────

// Float arithmetic is used for display-only % and $ values per ADR-0003
// (Sharpe + drawdown calc requires float arithmetic; Decimal for money).
#[allow(clippy::float_arithmetic)]
// Decimal → f64 for display; precision loss is acceptable.
#[allow(clippy::cast_precision_loss)]
fn kpi_floats(result: &PairsRunResult) -> (f64, f64, f64, f64, f64) {
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
    (total_return_pct, max_dd_pct, fees_f, initial_f, final_f)
}

/// Build the per-pair trade summary table rows (R8.5).
fn build_pair_summary(result: &PairsRunResult) -> String {
    if result.pair_trades.is_empty() {
        "| (no trades) | 0 |".to_string()
    } else {
        result
            .pair_trades
            .iter()
            .map(|(key, count)| format!("| {key} | {count} |"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

// ── Report body builder ───────────────────────────────────────────────────────

fn build_content(
    input: &PairsScenarioInput,
    result: &PairsRunResult,
    seed: u64,
    data_source: &str,
    stamp: &str,
) -> String {
    let (total_return_pct, max_dd_pct, fees_f, initial_f, final_f) = kpi_floats(result);
    let pair_summary = build_pair_summary(result);

    format!(
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
           kind: mean_reversion_pairs\n\
           content_hash: {strat_hash}\n\
           source: config/strategies/pairs_mr_h1.toml\n\
           signal: zscore_spread(lookback=60,z_entry=2.0,z_exit=0.5)\n\
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
         | Ledger imbalances    | 0                             |\n\
         | Seed                 | 0x{seed:X}                    |\n\
         | Data source          | {data_source}                 |\n\
         \n\
         ## Per-Pair Summary (R8.5)\n\
         \n\
         | Pair                 | Trades |\n\
         |----------------------|--------|\n\
         {pair_summary}\n\
         \n\
         ## Universe\n\
         \n\
         {universe_list}\n\
         \n\
         ## Reconciliation\n\
         \n\
         Reconciler ran at every bar close.\n\
         `ledger_imbalance_total == 0` — PASS.\n\
         \n\
         ## Notes\n\
         \n\
         - v1.5a mean-reversion pairs: {strat_id}\n\
         - Formulation C: long-only on `a` leg; `b` leg is observed only.\n\
         - Slippage: {slippage_bps} bps, Taker fee: {taker_fee_bps} bps\n\
         - Size: binary_per_pair, exposure_cap_per_pair=25%\n\
         - Risk: per-symbol cap=40%, portfolio cap=75% (v1.5a)\n\
         - Data: synthetic hourly bars, 4 independent ChaCha20Rng streams\n",
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
        pair_summary = pair_summary,
        universe_list = result
            .universe
            .iter()
            .map(|s| format!("- {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
        slippage_bps = input.slippage_bps,
        taker_fee_bps = input.taker_fee_bps,
    )
}

// ── Public writer ─────────────────────────────────────────────────────────────

/// Write a backtest report for the pairs scenario (T715).
///
/// Byte-identical to `main.rs::write_pairs_report` @1452.
///
/// # Errors
///
/// Returns `Err` if the report file cannot be written to disk.
pub fn write(
    input: &PairsScenarioInput,
    result: &PairsRunResult,
    seed: u64,
    data_source: &str,
    report_path: &Path,
) -> Result<()> {
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

    let content = build_content(input, result, seed, data_source, &stamp);
    std::fs::write(report_path, content).context("write pairs report")?;
    Ok(())
}
