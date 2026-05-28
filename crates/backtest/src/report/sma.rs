//! SMA/Composed scenario report writer — Phase B T-D-N2.
//!
//! Extracted from `main.rs::write_report` @2488. Byte-identical output.

use std::path::Path;

use anyhow::{Context, Result};
use rust_decimal::Decimal;
use time::OffsetDateTime;

use crate::cli_types::{BacktestState, SmaScenarioInput, StrategyMeta};
use crate::scenarios::sma_composed::compute_sharpe;

// ── KPI float helpers ─────────────────────────────────────────────────────────

// Float arithmetic is used for display-only % and $ values per ADR-0003
// (Sharpe + drawdown calc requires float arithmetic; Decimal for money).
#[allow(clippy::float_arithmetic)]
// Decimal → f64 for display; precision loss is acceptable.
#[allow(clippy::cast_precision_loss)]
fn kpi_floats(
    state: &BacktestState,
    initial_capital: Decimal,
    final_equity: Decimal,
) -> (f64, f64, f64, f64, f64, f64) {
    let total_return_pct = if initial_capital > Decimal::ZERO {
        let r = (final_equity - initial_capital) / initial_capital;
        f64::try_from(r).unwrap_or(0.0) * 100.0
    } else {
        0.0
    };
    let sharpe = compute_sharpe(&state.equity_curve);
    let max_dd_pct = f64::try_from(state.max_drawdown).unwrap_or(0.0) * 100.0;
    let fees_f = f64::try_from(state.total_fees).unwrap_or(0.0);
    let initial_f = f64::try_from(initial_capital).unwrap_or(0.0);
    let final_f = f64::try_from(final_equity).unwrap_or(0.0);
    (
        total_return_pct,
        sharpe,
        max_dd_pct,
        fees_f,
        initial_f,
        final_f,
    )
}

// ── Report body builder ───────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_lines)]
fn build_content(
    input: &SmaScenarioInput,
    state: &BacktestState,
    initial_capital: Decimal,
    final_equity: Decimal,
    seed: u64,
    data_source: &str,
    elapsed_secs: f64,
    stamp: &str,
    strategy_meta: &StrategyMeta,
    revision_sha: Option<&str>,
) -> String {
    let (total_return_pct, sharpe, max_dd_pct, fees_f, initial_f, final_f) =
        kpi_floats(state, initial_capital, final_equity);

    let baseline_line = input.baseline_report.as_deref().map_or_else(
        || "baseline_report: n/a".to_string(),
        |b| format!("baseline_report: {b}"),
    );

    let reconcile_result = if state.ledger_imbalance_events == 0 {
        "PASS"
    } else {
        "FAIL"
    };

    // body_name is the canonical scenario name written into the report body.
    let body_name = &input.body_name;

    // body_elapsed is the elapsed time written into the body's Wall-clock row.
    let body_elapsed = input.body_elapsed_override.unwrap_or(elapsed_secs);

    // Optional `revision_sha:` frontmatter line immediately after `data_source:`.
    // Populated by Yahoo emitters; `None` for Binance/synthetic paths (byte-identical).
    let revision_sha_line = revision_sha
        .map(|sha| format!("revision_sha: {sha}\n"))
        .unwrap_or_default();

    format!(
        "---\n\
         scenario: {scenario_name}\n\
         seed: 0x{seed:X}\n\
         generated: {stamp}\n\
         wall_clock_s: {elapsed:.1}\n\
         data_source: {data_source}\n\
         {revision_sha_line}\
         {baseline_line}\n\
         ledger_imbalance_total: {imbalance}\n\
         llm_spend_usd: 0.00\n\
         strategy:\n\
           id: {strat_id}\n\
           kind: {strat_kind}\n\
           content_hash: {strat_hash}\n\
           source: {strat_source}\n\
           signal: {strat_signal}\n\
         ---\n\
         \n\
         # Backtest Report — {body_name}\n\
         \n\
         ## Summary\n\
         \n\
         | Metric               | Value                      |\n\
         |----------------------|----------------------------|\n\
         | Scenario             | {body_name}            |\n\
         | Symbol               | {symbol}                   |\n\
         | Start year           | {start_year}               |\n\
         | Bars replayed        | {bars}                     |\n\
         | Initial capital      | ${initial:.2} USDT         |\n\
         | Final equity         | ${final_eq:.2} USDT        |\n\
         | Total return         | {ret:.2}%                  |\n\
         | Sharpe ratio (ann.)  | {sharpe:.4}                |\n\
         | Max drawdown         | {max_dd:.2}%               |\n\
         | Trades               | {trades}                   |\n\
         | Buys                 | {buys}                     |\n\
         | Sells                | {sells}                    |\n\
         | Total fees           | ${fees:.6} USDT            |\n\
         | Ledger imbalances    | {imbalance}                |\n\
         | LLM spend            | $0.00                      |\n\
         | Wall-clock time      | {body_elapsed:.1}s              |\n\
         | Seed                 | 0x{seed:X}                 |\n\
         | Data source          | {data_source}              |\n\
         \n\
         ## Reconciliation\n\
         \n\
         Minute-boundary reconciler ran at every bar close.\n\
         `ledger_imbalance_total == {imbalance}` — {reconcile_result}.\n\
         \n\
         ## Notes\n\
         \n\
         - {strategy_notes}\n\
         - Slippage: {slippage_bps} bps, Taker fee: {taker_fee_bps} bps\n\
         - Size: fixed_fraction = 10%\n\
         - Risk: per-symbol exposure cap = 40%\n",
        scenario_name = input.scenario_name,
        body_name = body_name,
        seed = seed,
        stamp = stamp,
        data_source = data_source,
        baseline_line = baseline_line,
        imbalance = state.ledger_imbalance_events,
        strat_id = strategy_meta.id,
        strat_kind = strategy_meta.kind,
        strat_hash = strategy_meta.hash_hex,
        strat_source = strategy_meta.source_path,
        strat_signal = strategy_meta.signal,
        symbol = input.symbol,
        start_year = input.start_year,
        bars = state.equity_curve.len(),
        initial = initial_f,
        final_eq = final_f,
        ret = total_return_pct,
        sharpe = sharpe,
        max_dd = max_dd_pct,
        trades = state.trades,
        buys = state.buys,
        sells = state.sells,
        fees = fees_f,
        elapsed = elapsed_secs,
        body_elapsed = body_elapsed,
        strategy_notes = strategy_meta.notes,
        slippage_bps = input.slippage_bps,
        taker_fee_bps = input.taker_fee_bps,
        reconcile_result = reconcile_result,
    )
}

// ── Public writer ─────────────────────────────────────────────────────────────

/// Write the SMA/Composed backtest report.
///
/// Byte-identical to `main.rs::write_report` @2488.
///
/// `revision_sha` — when `Some`, injects a `revision_sha: <64 hex>` line
/// immediately after `data_source:` in the YAML front-matter.  Pass `None`
/// for all Binance/synthetic callers to preserve the 33 existing anchors
/// byte-identically (D-V0.1.3-1 `None` arm contract).
///
/// # Errors
///
/// Returns `Err` if the report file cannot be written to disk.
#[allow(clippy::too_many_arguments)]
pub fn write(
    input: &SmaScenarioInput,
    state: &BacktestState,
    initial_capital: Decimal,
    final_equity: Decimal,
    seed: u64,
    data_source: &str,
    elapsed_secs: f64,
    report_path: &Path,
    strategy_meta: &StrategyMeta,
    revision_sha: Option<&str>,
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
        state,
        initial_capital,
        final_equity,
        seed,
        data_source,
        elapsed_secs,
        &stamp,
        strategy_meta,
        revision_sha,
    );
    std::fs::write(report_path, content).context("write report")?;
    Ok(())
}
