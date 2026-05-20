//! TCN overlay scenario report writer — Phase B T-D-N5/N6.
//!
//! Extracted from `main.rs::write_tcn_overlay_report` @2184. Byte-identical output.
//! Shared by both `tcn_overlay` and `tcn_overlay_weights` scenarios.

use std::path::Path;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use time::OffsetDateTime;

use crate::cli_types::TcnScenarioInput;
use crate::scenarios::tcn_overlay::TcnOverlayRunResult;

// ── KPI float helpers ─────────────────────────────────────────────────────────

// Float arithmetic is used for display-only % and $ values per ADR-0003
// (Sharpe + drawdown calc requires float arithmetic; Decimal for money).
#[allow(clippy::float_arithmetic)]
// Decimal → f64 for display; precision loss is acceptable.
#[allow(clippy::cast_precision_loss)]
fn kpi_floats(result: &TcnOverlayRunResult) -> (f64, f64, f64, f64, f64, f64) {
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
    // Cast u64 signal counts to f64 for dampen-rate display; precision loss
    // is acceptable (counts are statistical, not monetary values).
    #[allow(clippy::cast_precision_loss)]
    let eligible = (result.passed_through_signals + result.dampened_signals) as f64;
    let dampen_rate_pct = if eligible > 0.0 {
        result.dampened_signals as f64 / eligible * 100.0
    } else {
        0.0
    };
    (
        total_return_pct,
        max_dd_pct,
        fees_f,
        initial_f,
        final_f,
        dampen_rate_pct,
    )
}

// ── Data-source section builder ───────────────────────────────────────────────

fn build_data_source_section(
    input: &TcnScenarioInput,
    data_revision_sha: &str,
    loaded_bar_info: Option<(usize, usize)>,
) -> String {
    let Some((loaded, expected)) = loaded_bar_info else {
        return String::new();
    };

    // Float arithmetic only for display percentage; Decimal not applicable.
    #[allow(clippy::float_arithmetic, clippy::cast_precision_loss)]
    let pct = if expected > 0 {
        loaded as f64 / expected as f64 * 100.0
    } else {
        0.0
    };
    let span_start = format!("{:04}-01-01T00:00:00Z", input.start_year);
    let span_end = format!("{:04}-01-01T00:00:00Z", input.start_year + 1);
    format!(
        "\n## Data source\n\
         \n\
         | {field:<20} | {value:<36} |\n\
         |{dash_field:-<22}|{dash_value:-<38}|\n\
         | {source:<20} | {source_val:<36} |\n\
         | {rev_label:<20} | {rev_val:<36} |\n\
         | {univ_label:<20} | {univ_val:<36} |\n\
         | {interval_label:<20} | {interval_val:<36} |\n\
         | {span_label:<20} | {span_val:<36} |\n\
         | {exp_label:<20} | {exp_val:<36} |\n\
         | {loaded_label:<20} | {loaded_val:<36} |\n",
        field = "Field",
        value = "Value",
        dash_field = "",
        dash_value = "",
        source = "Source",
        source_val = "Binance Vision via data/binance/",
        rev_label = "Revision SHA",
        rev_val = data_revision_sha,
        univ_label = "Universe size",
        univ_val = "10 symbols",
        interval_label = "Bar interval",
        interval_val = "1h",
        span_label = "Span (UTC, half-open)",
        span_val = format!("{span_start} .. {span_end}"),
        exp_label = "Expected bars",
        exp_val = expected.to_string(),
        loaded_label = "Loaded bars",
        loaded_val = format!("{loaded} ({pct:.2}% present)"),
    )
}

// ── Universe list helper ──────────────────────────────────────────────────────

fn build_universe_list(result: &TcnOverlayRunResult) -> String {
    result
        .universe
        .iter()
        .map(|s| format!("- {s}"))
        .collect::<Vec<_>>()
        .join("\n")
}

// ── Report body builder ───────────────────────────────────────────────────────

fn build_content(
    input: &TcnScenarioInput,
    result: &TcnOverlayRunResult,
    seed: u64,
    data_source: &str,
    stamp: &str,
    data_revision_sha: &str,
    loaded_bar_info: Option<(usize, usize)>,
) -> String {
    let (total_return_pct, max_dd_pct, fees_f, initial_f, final_f, dampen_rate_pct) =
        kpi_floats(result);

    let data_source_section = build_data_source_section(input, data_revision_sha, loaded_bar_info);

    let data_notes_line = if loaded_bar_info.is_some() {
        "Data: real Binance hourly OHLCV, see ## Data source section above."
    } else {
        "Data: synthetic hourly bars, 10 independent ChaCha20Rng streams"
    };

    format!(
        "---\n\
         scenario: {scenario_name}\n\
         seed: 0x{seed:X}\n\
         generated: {stamp}\n\
         wall_clock_s: {elapsed:.1}\n\
         data_revision_sha: {data_revision_sha}\n\
         data_source: {data_source}\n\
         baseline_report: n/a\n\
         ledger_imbalance_total: 0\n\
         llm_spend_usd: 0.00\n\
         strategy:\n\
           id: {strat_id}\n\
           kind: tcn_overlay_momentum\n\
           content_hash: n/a\n\
           source: config/strategies/tcn_overlay_momentum.toml\n\
           signal: tcn_overlay(base=vol_adjusted_log_return,confidence_threshold=0.6)\n\
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
         ## TCN Overlay Modulation\n\
         \n\
         | Metric               | Value                         |\n\
         |----------------------|-------------------------------|\n\
         | Passed through       | {passed_through}              |\n\
         | Dampened to Hold     | {dampened}                    |\n\
         | Warming-up (no overlay) | {warmup}                  |\n\
         | Dampen rate          | {dampen_rate:.2}%             |\n\
         \n\
         ## Universe\n\
         \n\
         {universe_list}\n\
         {data_source_section}\n\
         ## Notes\n\
         \n\
         - v2.5 TCN overlay momentum: {strat_id}\n\
         - Forecaster: {forecaster_label}\n\
         - Slippage: {slippage_bps} bps, Taker fee: {taker_fee_bps} bps\n\
         - Size: equal_weight, exposure_cap=50%, k_long=3\n\
         - Risk: per-symbol cap=40%, portfolio cap=50%\n\
         - {data_notes_line}\n",
        scenario_name = input.scenario_name,
        seed = seed,
        stamp = stamp,
        elapsed = result.elapsed_secs,
        data_revision_sha = data_revision_sha,
        data_source = data_source,
        strat_id = result.strategy_id,
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
        passed_through = result.passed_through_signals,
        dampened = result.dampened_signals,
        warmup = result.warmup_signals,
        dampen_rate = dampen_rate_pct,
        universe_list = build_universe_list(result),
        data_source_section = data_source_section,
        slippage_bps = input.slippage_bps,
        taker_fee_bps = input.taker_fee_bps,
        forecaster_label = result.forecaster_label,
        data_notes_line = data_notes_line,
    )
}

// ── Public writer ─────────────────────────────────────────────────────────────

/// Write a backtest report for the TCN overlay momentum scenario.
///
/// Byte-identical to `main.rs::write_tcn_overlay_report` @2184.
/// Shared by `tcn_overlay` and `tcn_overlay_weights` scenarios.
///
/// # Errors
///
/// Returns `Err` if the report file cannot be written to disk.
#[allow(clippy::too_many_arguments)]
pub fn write(
    input: &TcnScenarioInput,
    result: &TcnOverlayRunResult,
    seed: u64,
    data_source: &str,
    report_path: &Path,
    // T-D-10: aggregate SHA for frontmatter. "n/a" for Synthetic.
    data_revision_sha: &str,
    // T-D-11: (loaded_count, expected_count) for the ## Data source body section.
    // None for Synthetic scenarios (section absent).
    loaded_bar_info: Option<(usize, usize)>,
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

    let content = build_content(
        input,
        result,
        seed,
        data_source,
        &stamp,
        data_revision_sha,
        loaded_bar_info,
    );
    std::fs::write(report_path, content).context("write tcn-overlay report")?;
    Ok(())
}
