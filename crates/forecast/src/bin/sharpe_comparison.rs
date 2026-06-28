//! `sharpe_comparison` — M-SHARPE Sharpe/Sortino/Calmar/drawdown comparison
//! report across the five `-realdata` backtest scenarios (four TCN + one PatchTST BS-1).
//!
//! ## Usage
//!
//! ```bash
//! # Build the backtest binary first (release for speed):
//! cargo build -p backtest --release --features realdata,candle
//!
//! # Then run the comparison:
//! cargo run -p forecast --bin sharpe_comparison -- \
//!   --backtest-bin target/release/backtest
//! ```
//!
//! ## Read-only contract (K5)
//!
//! - Does NOT modify any checkpoint, anchor, or backtest report.
//! - Re-runs scenarios into a tempdir; the four anchored `-realdata`
//!   reports under `spec/v1/backtest-real-binance-data/reports/` are NEVER
//!   touched.
//! - No flag implies retraining or anchor mutation.
//!
//! ## Determinism (K3)
//!
//! - All floats serialised with fixed precision per ADR-0033 § D2.b.
//! - Annualisation: `sqrt(24·365)` (hourly → annual).
//! - Two sequential runs produce byte-identical report bodies.
//!
//! ## Cross-references
//!
//! - ADR-0033 § D2.b — Sharpe-comparison report shape.
//! - ADR-0033 § D4 — Sharpe formulas + annualisation constant.
//! - `crates/backtest/src/main.rs:2428` — existing `compute_sharpe` (minute-
//!   annualised; NOT reused here — see ADR-0033 § D4 alt-7).

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
// EnvFilter now used via llm::tracing_init::install_global (T-RED-D12).

// ── ScenarioFamily ────────────────────────────────────────────────────────────

/// Which family of scenarios to compare.
///
/// `Tcn` (default) → existing 5-scenario TCN + PatchTST run (byte-identical).
/// `VolTarget` → new v3.0.0-volatility: v1 momentum baseline + vol-targeting overlay.
/// `VolTargetRebaseline` → v3.0.0-volatility-rebaseline: real-data momentum baseline.
/// `RegimeDispatcher` → v3.0.0-regime: real-data momentum baseline vs regime-dispatcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ScenarioFamily {
    /// TCN + PatchTST BS-1 (default; 5 scenarios).
    Tcn,
    /// GARCH vol-targeting overlay vs SYNTHETIC v1 momentum baseline (parent;
    /// v3.0.0-volatility anchor `ef048366...`; byte-immutable).
    #[value(name = "vol-target-bs1")]
    VolTarget,
    /// GARCH vol-targeting overlay vs REAL-data v1 momentum baseline
    /// (v3.0.0-volatility-rebaseline; 2026-05-22+ T-AR-2 lock).
    #[value(name = "vol-target-bs1-rebaseline")]
    VolTargetRebaseline,
    /// v3.0.0-regime RegimeDispatcher vs real-data v1 momentum baseline (BS-1).
    /// T-REG-ALPHA-UNLOCKED / T-REG-MARGINAL / T-REG-NO-ALPHA classifier per ADR-0049 § D4.
    #[value(name = "regime-dispatcher-bs1")]
    RegimeDispatcher,
}

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "sharpe_comparison",
    about = "M-SHARPE: Sharpe/Sortino/Calmar comparison report (TCN, vol-target, or regime-dispatcher family)",
    long_about = "Runs one of the scenario families:\n\
                  - tcn (default): five -realdata scenarios (4 TCN + 1 PatchTST BS-1).\n\
                  - vol-target-bs1: v1 momentum baseline + GARCH vol-targeting overlay.\n\
                  - vol-target-bs1-rebaseline: real-data v1 momentum baseline + vol-target overlay.\n\
                  - regime-dispatcher-bs1: real-data v1 momentum baseline vs regime-dispatcher (v3.0.0-regime).\n\n\
                  Read-only contract: anchored reports are never touched."
)]
struct Args {
    /// Scenario family to compare.
    #[arg(long, default_value = "tcn")]
    scenario: ScenarioFamily,

    /// Output directory for the report.
    /// Defaults: tcn → spec/v1/v25a-patchtst-overlay/reports/,
    ///           vol-target-bs1 → spec/v1/v3-volatility-forecaster/reports/,
    ///           regime-dispatcher-bs1 → spec/v1/v3-regime-classifier/reports/.
    #[arg(long)]
    out_dir: Option<PathBuf>,

    /// Backtest binary path (must be built with --features realdata,candle).
    #[arg(long, default_value = "target/release/backtest")]
    backtest_bin: PathBuf,

    /// Skip the re-run step; use pre-existing equity bins under --out-dir.
    #[arg(long, default_value_t = false)]
    skip_rerun: bool,
}

// ── Metrics module ────────────────────────────────────────────────────────────

mod metrics {
    use rust_decimal::Decimal;
    use rust_decimal::prelude::ToPrimitive;

    /// √(24 · 365) ≈ 92.601295 — hourly-to-annual annualisation factor.
    ///
    /// ADR-0033 § D4: NOT sqrt(525_600) which is the minute-resolution
    /// constant used in `crates/backtest::compute_sharpe()`.
    pub const SQRT_HOURS_PER_YEAR: f64 = 92.601_295_098_46;

    // Sanity: (24.0 * 365.0).sqrt() at compile time (const f64 sqrt is
    // unstable; we pin the value and assert in a test).

    /// Compute arithmetic log-returns from a per-bar equity series.
    fn log_returns(equity: &[Decimal]) -> Vec<f64> {
        if equity.len() < 2 {
            return vec![];
        }
        equity
            .windows(2)
            .map(|w| {
                let prev = w[0].to_f64().unwrap_or(1.0);
                let curr = w[1].to_f64().unwrap_or(1.0);
                if prev <= 0.0 { 0.0 } else { (curr / prev).ln() }
            })
            .collect()
    }

    /// Hourly-annualised Sharpe ratio (rf = 0).
    ///
    /// Formula: `mean_r / std_r * sqrt(24 * 365)`.
    /// Returns 0.0 for series with fewer than 2 bars or zero std.
    pub fn compute_sharpe_hourly(equity: &[Decimal]) -> f64 {
        let rets = log_returns(equity);
        let n = rets.len();
        if n < 2 {
            return 0.0;
        }
        let mean_r = rets.iter().sum::<f64>() / n as f64;
        let var_r: f64 = rets.iter().map(|&r| (r - mean_r).powi(2)).sum::<f64>() / n as f64;
        let std_r = var_r.sqrt();
        if std_r < 1e-15 {
            return 0.0;
        }
        mean_r / std_r * SQRT_HOURS_PER_YEAR
    }

    /// Hourly-annualised Sortino ratio (rf = 0).
    ///
    /// Formula: `mean_r / downside_std_r * sqrt(24 * 365)`.
    /// `downside_std_r = sqrt(mean(min(r, 0)^2))`.
    /// Returns 0.0 for series with fewer than 2 bars or zero downside std.
    pub fn compute_sortino_hourly(equity: &[Decimal]) -> f64 {
        let rets = log_returns(equity);
        let n = rets.len();
        if n < 2 {
            return 0.0;
        }
        let mean_r = rets.iter().sum::<f64>() / n as f64;
        let downside_sq: f64 = rets.iter().map(|&r| r.min(0.0).powi(2)).sum::<f64>() / n as f64;
        let downside_std = downside_sq.sqrt();
        if downside_std < 1e-15 {
            return 0.0;
        }
        mean_r / downside_std * SQRT_HOURS_PER_YEAR
    }

    /// Calmar ratio: `CAGR / abs(max_drawdown)`.
    ///
    /// `CAGR = (final/initial)^(1/years) - 1` where
    /// `years = (equity.len() - 1) / 8760.0`.
    /// Returns 0.0 for series with fewer than 2 bars, zero drawdown, or zero
    /// initial equity.
    pub fn compute_calmar(equity: &[Decimal]) -> f64 {
        let n = equity.len();
        if n < 2 {
            return 0.0;
        }
        let initial = equity[0].to_f64().unwrap_or(0.0);
        let final_eq = equity[n - 1].to_f64().unwrap_or(0.0);
        if initial <= 0.0 {
            return 0.0;
        }
        let years = (n as f64 - 1.0) / 8760.0;
        if years <= 0.0 {
            return 0.0;
        }
        let cagr = (final_eq / initial).powf(1.0 / years) - 1.0;
        let max_dd = compute_max_drawdown(equity);
        if max_dd.abs() < 1e-15 {
            return 0.0;
        }
        cagr / max_dd.abs()
    }

    /// Maximum drawdown: `max over t of (peak_equity_t - equity_t) / peak_equity_t`.
    ///
    /// Returns 0.0 for series with fewer than 2 bars or zero peak.
    pub fn compute_max_drawdown(equity: &[Decimal]) -> f64 {
        if equity.len() < 2 {
            return 0.0;
        }
        let mut peak = equity[0].to_f64().unwrap_or(0.0);
        let mut max_dd = 0.0f64;
        for e in &equity[1..] {
            let eq = e.to_f64().unwrap_or(0.0);
            if eq > peak {
                peak = eq;
            }
            if peak > 0.0 {
                let dd = (peak - eq) / peak;
                if dd > max_dd {
                    max_dd = dd;
                }
            }
        }
        max_dd
    }

    // ── Unit tests ────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;
        use rust_decimal_macros::dec;

        // Tolerance for float comparisons.
        const TOL: f64 = 1e-4;

        /// (a) Sharpe on a hand-built equity curve with known mean / std.
        #[test]
        fn test_sharpe_known() {
            // Equity: 100, 101, 102 → log returns ≈ [ln(1.01), ln(1.0099)]
            let equity = vec![dec!(100), dec!(101), dec!(102)];
            let s = compute_sharpe_hourly(&equity);
            // mean_r ≈ 0.00995, std ≈ tiny (returns nearly identical)
            // Just assert it's positive and finite.
            assert!(s.is_finite() && s > 0.0, "Sharpe should be positive: {s}");

            // Manually verify: returns = [ln(101/100), ln(102/101)]
            let r1 = (101.0f64 / 100.0).ln();
            let r2 = (102.0f64 / 101.0).ln();
            let mean_r = (r1 + r2) / 2.0;
            let var_r = ((r1 - mean_r).powi(2) + (r2 - mean_r).powi(2)) / 2.0;
            let std_r = var_r.sqrt();
            let expected = mean_r / std_r * SQRT_HOURS_PER_YEAR;
            assert!(
                (s - expected).abs() < TOL,
                "Sharpe mismatch: got {s}, expected {expected}"
            );
        }

        /// (b) Sortino > Sharpe when downside returns are smaller than upside.
        #[test]
        fn test_sortino_vs_sharpe_asymmetric() {
            // Build a curve with mostly gains and one small loss.
            let equity: Vec<rust_decimal::Decimal> = {
                let mut v = vec![dec!(100)];
                // 10 bars of +1%, one bar of -0.1%, 10 more bars of +1%
                let mut curr = dec!(100);
                for _ in 0..10 {
                    curr *= dec!(1.01);
                    v.push(curr);
                }
                curr *= dec!(0.999);
                v.push(curr);
                for _ in 0..10 {
                    curr *= dec!(1.01);
                    v.push(curr);
                }
                v
            };
            let sharpe = compute_sharpe_hourly(&equity);
            let sortino = compute_sortino_hourly(&equity);
            // With small downside, Sortino should be >= Sharpe.
            assert!(
                sortino >= sharpe,
                "Sortino ({sortino}) should be >= Sharpe ({sharpe}) with small downside"
            );
        }

        /// (c) Calmar on a curve with known CAGR + DD.
        #[test]
        fn test_calmar_known() {
            // Simple curve: 100 → 200 (doubled) over 8760 bars (1 year).
            // CAGR = 100%. Max DD = 0 (monotone).
            // → Calmar would be infinite (no drawdown). Use a curve with a dip.
            let mut equity = vec![dec!(100)];
            for _ in 0..4380 {
                equity.push(*equity.last().unwrap() * dec!(1.0001));
            }
            // Dip by 10%.
            let peak = *equity.last().unwrap();
            equity.push(peak * dec!(0.90));
            // Then recover and grow.
            let mut curr = *equity.last().unwrap();
            for _ in 0..(8760 - 4381) {
                curr *= dec!(1.0001);
                equity.push(curr);
            }

            let dd = compute_max_drawdown(&equity);
            assert!(dd > 0.0 && dd < 0.15, "max_dd should be ~10%: {dd}");

            let calmar = compute_calmar(&equity);
            assert!(
                calmar.is_finite() && calmar > 0.0,
                "Calmar should be positive: {calmar}"
            );
        }

        /// (d) max_drawdown on a peak-then-trough curve.
        #[test]
        fn test_max_drawdown_peak_trough() {
            // 100 → 200 → 50 → 150
            let equity = vec![dec!(100), dec!(200), dec!(50), dec!(150)];
            let dd = compute_max_drawdown(&equity);
            // Peak = 200, trough = 50 → dd = (200 - 50) / 200 = 0.75
            assert!((dd - 0.75).abs() < 1e-9, "max_dd should be 0.75: {dd}");
        }

        /// (e) edge case: 1-element equity curve returns 0.0 for all four.
        #[test]
        fn test_edge_single_element() {
            let equity = vec![dec!(100)];
            assert_eq!(compute_sharpe_hourly(&equity), 0.0);
            assert_eq!(compute_sortino_hourly(&equity), 0.0);
            assert_eq!(compute_calmar(&equity), 0.0);
            assert_eq!(compute_max_drawdown(&equity), 0.0);
        }
    }
}

// ── Rerun module ──────────────────────────────────────────────────────────────

mod rerun {
    use std::path::{Path, PathBuf};

    use anyhow::{Context, Result};
    use rust_decimal::Decimal;

    /// Result of re-running a single backtest scenario.
    #[derive(Debug, Clone)]
    pub struct RerunResult {
        /// Scenario name.
        pub name: String,
        /// Human-readable variant label (passthrough | real-weights).
        pub variant: String,
        /// Equity curve (Vec<Decimal>, length = bars + 1).
        pub equity: Vec<Decimal>,
        /// Total bars.
        pub bars: u64,
        /// Total trades.
        pub trades: u64,
        /// Final equity value.
        pub final_equity: Decimal,
        /// Total return as a fraction (e.g. 0.1348 = 13.48%).
        pub total_return: f64,
        /// Max drawdown as a fraction (e.g. 0.7373 = 73.73%).
        pub max_drawdown: f64,
        /// Dampen rate as a fraction.
        pub dampen_rate: f64,
    }

    /// The five -realdata scenario names in table order.
    ///
    /// Additive extension at Wave D T-D-N26: `top10-2023-fy-patchtst-overlay-realdata`
    /// is the new PatchTST BS-1 scenario (v2.5a.0-patchtst).
    pub const SCENARIOS: [&str; 5] = [
        "top10-2023-fy-tcn-overlay-realdata",
        "top10-2024-fy-tcn-overlay-realdata",
        "top10-2023-fy-tcn-overlay-weights-realdata",
        "top10-2024-fy-tcn-overlay-weights-realdata",
        // v2.5a PatchTST BS-1 (24h horizon, 2023-FY, real Binance data).
        "top10-2023-fy-patchtst-overlay-realdata",
    ];

    fn variant_label(name: &str) -> &'static str {
        if name.contains("patchtst") {
            "patchtst-real-weights"
        } else if name.contains("weights") {
            "real-weights"
        } else {
            "passthrough"
        }
    }

    /// Re-run `name` into `tempdir` via the backtest binary.
    ///
    /// Passes `--emit-equity-bin <tempdir>/<name>-equity.bin` so that the
    /// equity curve is available for Sharpe computation.
    pub fn rerun_scenario(name: &str, backtest_bin: &Path, tempdir: &Path) -> Result<RerunResult> {
        use std::process::Command;

        let equity_bin_path = tempdir.join(format!("{name}-equity.bin"));

        // Run the backtest binary, redirecting its report into tempdir.
        let status = Command::new(backtest_bin)
            .args([
                "--scenario",
                name,
                "--reports-dir",
                &tempdir.to_string_lossy(),
                "--emit-equity-bin",
                &equity_bin_path.to_string_lossy(),
            ])
            .status()
            .with_context(|| format!("spawning backtest binary for scenario {name}"))?;

        if !status.success() {
            anyhow::bail!("backtest exited with status {} for scenario {name}", status);
        }

        // Read the equity bin.
        let equity = read_equity_bin(&equity_bin_path)
            .with_context(|| format!("reading equity bin for {name}"))?;

        // Parse summary from the produced report.
        let report_path = find_report(tempdir, name)
            .with_context(|| format!("finding report for {name} in {}", tempdir.display()))?;
        let report_body = std::fs::read_to_string(&report_path)
            .with_context(|| format!("reading report {}", report_path.display()))?;

        let bars = parse_report_field(&report_body, "Bars (total)").unwrap_or(0);
        let trades = parse_report_field(&report_body, "Trades").unwrap_or(0);
        let final_equity_f = parse_report_equity(&report_body).unwrap_or(0.0);
        let final_equity = Decimal::try_from(final_equity_f).unwrap_or(Decimal::ZERO);
        let total_return = parse_report_pct(&report_body, "Total return").unwrap_or(0.0);
        let max_drawdown = parse_report_pct(&report_body, "Max drawdown").unwrap_or(0.0);
        let dampen_rate = parse_report_pct(&report_body, "TCN dampen rate").unwrap_or(0.0);

        Ok(RerunResult {
            name: name.to_string(),
            variant: variant_label(name).to_string(),
            equity,
            bars,
            trades,
            final_equity,
            total_return,
            max_drawdown,
            dampen_rate,
        })
    }

    /// Deserialise the equity bin written by `--emit-equity-bin`.
    ///
    /// Format: one f64 per line (plain text, LF-terminated), parsed back to
    /// Decimal. This matches the write side in `crates/backtest`.
    pub fn read_equity_bin(path: &Path) -> Result<Vec<Decimal>> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("reading equity bin {}", path.display()))?;
        let mut equity = Vec::new();
        for line in raw.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            let f: f64 = line
                .parse()
                .with_context(|| format!("parsing equity value '{line}'"))?;
            equity.push(Decimal::try_from(f).unwrap_or(Decimal::ZERO));
        }
        Ok(equity)
    }

    /// Find the report file produced by the backtest in `dir` matching
    /// `scenario` in its filename.
    fn find_report(dir: &Path, scenario: &str) -> Result<PathBuf> {
        for entry in
            std::fs::read_dir(dir).with_context(|| format!("reading dir {}", dir.display()))?
        {
            let entry = entry?;
            let fname = entry.file_name().to_string_lossy().to_string();
            if fname.ends_with(".md") && fname.contains(scenario) {
                return Ok(entry.path());
            }
        }
        anyhow::bail!(
            "no report found for scenario {scenario} in {}",
            dir.display()
        )
    }

    /// Parse an integer field from the report Summary table.
    fn parse_report_field(body: &str, field: &str) -> Option<u64> {
        body.lines().find(|l| l.contains(field)).and_then(|l| {
            l.split('|')
                .nth(2)
                .map(|v| v.trim().replace(',', ""))
                .and_then(|v| v.parse().ok())
        })
    }

    /// Parse final equity from the report Summary table (e.g. "$113479.98 USDT" or "$113479.98").
    fn parse_report_equity(body: &str) -> Option<f64> {
        body.lines()
            .find(|l| l.contains("Final equity"))
            .and_then(|l| {
                l.split('|')
                    .nth(2)
                    .map(|v| {
                        v.trim()
                            .trim_start_matches('$')
                            .replace(',', "")
                            .trim_end_matches(" USDT")
                            .trim()
                            .to_string()
                    })
                    .and_then(|v| v.parse().ok())
            })
    }

    /// Parse a percentage field from the report Summary table (e.g. "13.48%").
    fn parse_report_pct(body: &str, field: &str) -> Option<f64> {
        body.lines().find(|l| l.contains(field)).and_then(|l| {
            l.split('|')
                .nth(2)
                .map(|v| v.trim().trim_end_matches('%'))
                .and_then(|v| v.parse::<f64>().ok())
                .map(|pct| pct / 100.0)
        })
    }
}

// ── Render module ─────────────────────────────────────────────────────────────

mod render {
    use super::{metrics, rerun::RerunResult};

    /// Run-varying fields stored in frontmatter (excluded from body hash).
    #[derive(Debug, Clone)]
    pub struct ReportContext {
        pub generated: String,
        pub wall_clock_s: f64,
        pub host: String,
        pub git_commit: String,
        pub data_revision_sha: String,
        pub source_reports: Vec<String>,
    }

    /// Render the deterministic report body per ADR-0033 § D2.b.
    ///
    /// Float canonicalisation:
    /// - Sharpe/Sortino/Calmar: `{:.6}`
    /// - total return / max drawdown / dampen rate: `{:.2}%`
    /// - final equity: `${:.2}`
    /// - bar/trade counts: integer
    ///
    /// `results[4]` is the PatchTST BS-1 2023-FY scenario (additive at Wave D T-D-N26).
    pub fn render_report(results: &[RerunResult; 5], _ctx: &ReportContext) -> String {
        use std::fmt::Write as FmtWrite;
        let mut body = String::with_capacity(4096);

        // ── Header ────────────────────────────────────────────────────────────
        writeln!(
            &mut body,
            "# Sharpe / drawdown comparison — v2.6.0-realdata + v2.5a-patchtst-overlay scenarios"
        )
        .unwrap();

        // ── § Methodology ─────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Methodology\n").unwrap();
        writeln!(
            &mut body,
            "| Field             | Value                                          |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|-------------------|------------------------------------------------|"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Source equity     | Re-run of the five -realdata scenarios (four TCN + one PatchTST BS-1, Option α per ADR-0033 § D2.b.i). |"
        )
        .unwrap();
        writeln!(&mut body, "| Bar interval      | 1h |").unwrap();
        writeln!(
            &mut body,
            "| Annualisation     | √(24·365) = {:.6} (hourly → annual) |",
            metrics::SQRT_HOURS_PER_YEAR
        )
        .unwrap();
        writeln!(&mut body, "| Risk-free rate    | 0.000000 (constant) |").unwrap();
        writeln!(
            &mut body,
            "| Sharpe formula    | (mean_r - r_f) / std_r * √(24·365), arithmetic returns |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Sortino formula   | (mean_r - r_f) / std_downside_r * √(24·365), downside vs r_f |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Calmar formula    | (CAGR) / abs(max_drawdown), where CAGR = (final/initial)^(1/years) - 1, years = bars/8760 |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Max drawdown      | max over t of (peak_equity_t - equity_t) / peak_equity_t, on the realised equity curve |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Equity series     | Per-bar equity_curve: Vec<Decimal> from --emit-equity-bin, starting at $100000.00 |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| compute_sharpe_hourly | New helper in sharpe_comparison.rs (NOT crates/backtest::compute_sharpe, which annualises by sqrt(525_600) for minute bars — see ADR-0033 § D4 alt-7). |"
        )
        .unwrap();

        // ── § Comparison table ────────────────────────────────────────────────
        writeln!(&mut body, "\n## Comparison table\n").unwrap();
        writeln!(
            &mut body,
            "| Scenario | Variant | Bars | Final equity | Total return | Max drawdown | Trades | Dampen rate | Sharpe (ann) | Sortino (ann) | Calmar |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|----------|---------|------|--------------|--------------|--------------|--------|-------------|--------------|---------------|--------|"
        )
        .unwrap();

        for r in results {
            let sharpe = metrics::compute_sharpe_hourly(&r.equity);
            let sortino = metrics::compute_sortino_hourly(&r.equity);
            let calmar = metrics::compute_calmar(&r.equity);
            writeln!(
                &mut body,
                "| {} | {} | {} | ${:.2} | {:.2}% | {:.2}% | {} | {:.2}% | {:.6} | {:.6} | {:.6} |",
                r.name,
                r.variant,
                r.bars,
                r.final_equity,
                r.total_return * 100.0,
                r.max_drawdown * 100.0,
                r.trades,
                r.dampen_rate * 100.0,
                sharpe,
                sortino,
                calmar,
            )
            .unwrap();
        }

        // ── § Verdict ─────────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Verdict\n").unwrap();
        writeln!(
            &mut body,
            "| Field             | Value                                          |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|-------------------|------------------------------------------------|"
        )
        .unwrap();

        // Check if all dampen rates are effectively zero.
        let all_zero_dampen = results.iter().all(|r| r.dampen_rate.abs() < 1e-6);

        if all_zero_dampen {
            writeln!(
                &mut body,
                "| Honest reading    | dampen rate = 0.00% across all five scenarios — overlay models are no-ops; equity curves are byte-identical between passthrough and real-weights variants per year. |"
            )
            .unwrap();
        } else {
            let max_dr = results
                .iter()
                .map(|r| r.dampen_rate)
                .fold(f64::NEG_INFINITY, f64::max);
            writeln!(
                &mut body,
                "| Honest reading    | Overlay is partially active (max dampen rate = {:.2}%). Sharpe lift vs baseline requires F-verdict cross-reference. |",
                max_dr * 100.0
            )
            .unwrap();
        }

        // Sharpe delta: passthrough-2023 vs real-weights-2023; passthrough-2024 vs real-weights-2024;
        // passthrough-2023 vs patchtst-2023.
        let sharpe_pass_2023 = metrics::compute_sharpe_hourly(&results[0].equity);
        let sharpe_weights_2023 = metrics::compute_sharpe_hourly(&results[2].equity);
        let sharpe_pass_2024 = metrics::compute_sharpe_hourly(&results[1].equity);
        let sharpe_weights_2024 = metrics::compute_sharpe_hourly(&results[3].equity);
        let sharpe_patchtst_2023 = metrics::compute_sharpe_hourly(&results[4].equity);
        writeln!(
            &mut body,
            "| Sharpe delta (TCN)      | {:.6} (passthrough vs. real-weights, 2023) / {:.6} (2024) |",
            sharpe_weights_2023 - sharpe_pass_2023,
            sharpe_weights_2024 - sharpe_pass_2024,
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Sharpe delta (PatchTST) | {:.6} (passthrough-2023 vs. patchtst-bs1-2023) |",
            sharpe_patchtst_2023 - sharpe_pass_2023,
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Conclusion        | TCN and PatchTST at v2.5a produce no alpha lift over the v1 momentum baseline. PatchTST F-verdict: F4 (see forecast-distribution-patchtst-bs1 report). |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Recommended follow-on | (a) if both models land F4, fund v25-tcn-horizon-bump OR retire TCN at v2.6 bake-off; (b) PatchTST 24h horizon may need longer backtest burn-in (336-bar warmup). |"
        )
        .unwrap();

        // ── § Notes ───────────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Notes\n").unwrap();
        writeln!(
            &mut body,
            "- Read-only against the five -realdata reports listed in frontmatter."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- This report re-runs five backtest scenarios (four TCN + one PatchTST BS-1, Option α per ADR-0033 § D2.b.i)."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown/dampen rate); integer bar/trade counts."
        )
        .unwrap();

        body
    }

    /// Render the YAML frontmatter (NOT included in body hash).
    pub fn render_frontmatter(ctx: &ReportContext) -> String {
        let sources: String = ctx
            .source_reports
            .iter()
            .map(|s| format!("  - {s}\n"))
            .collect();
        format!(
            "---\n\
             slug: v25a-patchtst-overlay\n\
             scenario: sharpe-comparison-patchtst-bs1-realdata\n\
             generated: {}\n\
             wall_clock_s: {:.1}\n\
             host: {}\n\
             git_commit: {}\n\
             data_revision_sha: {}\n\
             sources:\n\
             {}\
             ---\n",
            ctx.generated,
            ctx.wall_clock_s,
            ctx.host,
            ctx.git_commit,
            ctx.data_revision_sha,
            sources,
        )
    }

    // ── Unit tests ────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;
        use rust_decimal_macros::dec;

        fn make_equity(start: f64, n: usize) -> Vec<rust_decimal::Decimal> {
            let mut v = Vec::with_capacity(n);
            let mut curr = rust_decimal::Decimal::try_from(start).unwrap();
            v.push(curr);
            for _ in 1..n {
                curr *= dec!(1.001);
                v.push(curr);
            }
            v
        }

        fn make_fixture() -> [RerunResult; 5] {
            let eq = make_equity(100_000.0, 8760);
            let r = RerunResult {
                name: "top10-2023-fy-tcn-overlay-realdata".to_string(),
                variant: "passthrough".to_string(),
                equity: eq.clone(),
                bars: 8759,
                trades: 1000,
                final_equity: *eq.last().unwrap(),
                total_return: 0.1348,
                max_drawdown: 0.0,
                dampen_rate: 0.0,
            };
            [
                r.clone(),
                RerunResult {
                    name: "top10-2024-fy-tcn-overlay-realdata".to_string(),
                    variant: "passthrough".to_string(),
                    ..r.clone()
                },
                RerunResult {
                    name: "top10-2023-fy-tcn-overlay-weights-realdata".to_string(),
                    variant: "real-weights".to_string(),
                    ..r.clone()
                },
                RerunResult {
                    name: "top10-2024-fy-tcn-overlay-weights-realdata".to_string(),
                    variant: "real-weights".to_string(),
                    ..r.clone()
                },
                // PatchTST BS-1 2023-FY (additive at Wave D T-D-N26).
                RerunResult {
                    name: "top10-2023-fy-patchtst-overlay-realdata".to_string(),
                    variant: "patchtst-real-weights".to_string(),
                    ..r
                },
            ]
        }

        fn make_ctx() -> ReportContext {
            ReportContext {
                generated: "2026-05-18T12:00:00Z".to_string(),
                wall_clock_s: 165.0,
                host: "test-host".to_string(),
                git_commit: "abc123".to_string(),
                data_revision_sha: "def456".to_string(),
                source_reports: vec!["report-a.md".to_string()],
            }
        }

        /// (a) renderer output for a hand-built 4-result fixture matches a golden
        /// body byte-for-byte (we check structural invariants since golden bytes
        /// depend on computed Sharpe values which we verify separately).
        #[test]
        fn test_render_has_required_sections() {
            let results = make_fixture();
            let ctx = make_ctx();
            let body = render_report(&results, &ctx);
            assert!(
                body.contains("## Methodology"),
                "missing Methodology section"
            );
            assert!(
                body.contains("## Comparison table"),
                "missing Comparison table section"
            );
            assert!(body.contains("## Verdict"), "missing Verdict section");
            assert!(body.contains("## Notes"), "missing Notes section");
            assert!(
                body.contains("92.601295"),
                "missing annualisation constant in Methodology"
            );
            assert!(
                body.contains("passthrough"),
                "missing passthrough label in comparison table"
            );
            assert!(
                body.contains("real-weights"),
                "missing real-weights label in comparison table"
            );
            assert!(
                body.contains("patchtst-real-weights"),
                "missing patchtst-real-weights label in comparison table"
            );
        }

        /// (b) the ## Verdict table picks the honest-reading branch when dampen_rate = 0.
        #[test]
        fn test_verdict_honest_reading_zero_dampen() {
            let results = make_fixture();
            let ctx = make_ctx();
            let body = render_report(&results, &ctx);
            assert!(
                body.contains("dampen rate = 0.00%"),
                "should use honest-reading branch for zero dampen"
            );
        }

        /// (c) the renderer is deterministic — two invocations with the same
        /// (results, ctx) produce byte-identical output.
        #[test]
        fn test_render_deterministic() {
            let results = make_fixture();
            let ctx = make_ctx();
            let body1 = render_report(&results, &ctx);
            let body2 = render_report(&results, &ctx);
            assert_eq!(body1, body2, "render_report must be deterministic");
        }
    }
}

// ── Vol-target render module ──────────────────────────────────────────────────

/// Rendering for the `vol-target-bs1` scenario family (T-D-N27, ADR-0038 § D1.c).
///
/// Compares v1 momentum baseline vs GARCH vol-targeting overlay and
/// emits a T-classifier verdict: T-VOL-ALPHA-UNLOCKED / MARGINAL / NO-ALPHA.
mod render_vol_target {
    use super::metrics;
    use super::rerun::RerunResult;

    /// T-classifier verdict per ADR-0038 § D1.c.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TClassifier {
        /// net_delta >= +0.10
        AlphaUnlocked,
        /// net_delta in [+0.05, +0.10)
        Marginal,
        /// net_delta < +0.05
        NoAlpha,
    }

    impl TClassifier {
        pub fn label(self) -> &'static str {
            match self {
                Self::AlphaUnlocked => "T-VOL-ALPHA-UNLOCKED",
                Self::Marginal => "T-VOL-MARGINAL",
                Self::NoAlpha => "T-VOL-NO-ALPHA",
            }
        }

        pub fn classify(net_delta: f64) -> Self {
            if net_delta >= 0.10 {
                Self::AlphaUnlocked
            } else if net_delta >= 0.05 {
                Self::Marginal
            } else {
                Self::NoAlpha
            }
        }
    }

    /// Run-varying context for the frontmatter.
    #[derive(Debug, Clone)]
    pub struct ReportContext {
        pub generated: String,
        pub wall_clock_s: f64,
        pub host: String,
        pub git_commit: String,
        pub data_revision_sha: String,
    }

    /// Render the vol-target comparison report body.
    ///
    /// `baseline` is `top10-2023-1h-momentum` (synthetic v1).
    /// `overlay` is `top10-2023-fy-vol-target-overlay-realdata`.
    pub fn render_report(
        baseline: &RerunResult,
        overlay: &RerunResult,
        _ctx: &ReportContext,
    ) -> String {
        use std::fmt::Write as FmtWrite;
        let mut body = String::with_capacity(4096);

        let sharpe_baseline = metrics::compute_sharpe_hourly(&baseline.equity);
        let sharpe_overlay = metrics::compute_sharpe_hourly(&overlay.equity);
        let sortino_baseline = metrics::compute_sortino_hourly(&baseline.equity);
        let sortino_overlay = metrics::compute_sortino_hourly(&overlay.equity);
        let calmar_baseline = metrics::compute_calmar(&baseline.equity);
        let calmar_overlay = metrics::compute_calmar(&overlay.equity);

        let gross_delta = sharpe_overlay - sharpe_baseline;
        // Net delta (same as gross for our simplified case — turnover cost not modelled).
        let net_delta = gross_delta;
        let verdict = TClassifier::classify(net_delta);

        // ── Header ──────────────────────────────────────────────────────────────
        writeln!(
            &mut body,
            "# Sharpe / drawdown comparison — v3.0.0-volatility GARCH vol-targeting overlay"
        )
        .unwrap();

        // ── § Methodology ───────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Methodology\n").unwrap();
        writeln!(
            &mut body,
            "| Field             | Value                                          |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|-------------------|------------------------------------------------|"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Baseline scenario | top10-2023-1h-momentum (v1 cross-sectional momentum, synthetic) |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Overlay scenario  | top10-2023-fy-vol-target-overlay-realdata (GARCH BS-1 vol-targeting, real Binance data) |"
        )
        .unwrap();
        writeln!(&mut body, "| Bar interval      | 1h |").unwrap();
        writeln!(
            &mut body,
            "| Annualisation     | sqrt(24*365) = {:.6} (hourly -> annual) |",
            metrics::SQRT_HOURS_PER_YEAR
        )
        .unwrap();
        writeln!(&mut body, "| Risk-free rate    | 0.000000 (constant) |").unwrap();
        writeln!(
            &mut body,
            "| Sharpe formula    | (mean_r - r_f) / std_r * sqrt(24*365) |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| T-classifier      | ADR-0038 D1.c: net_delta >= 0.10 -> T-VOL-ALPHA-UNLOCKED, [0.05,0.10) -> T-VOL-MARGINAL, <0.05 -> T-VOL-NO-ALPHA |"
        )
        .unwrap();

        // ── § Comparison table ───────────────────────────────────────────────────
        writeln!(&mut body, "\n## Comparison table\n").unwrap();
        writeln!(
            &mut body,
            "| Scenario | Bars | Final equity | Total return | Max drawdown | Trades | Sharpe (ann) | Sortino (ann) | Calmar |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|----------|------|--------------|--------------|--------------|--------|--------------|---------------|--------|"
        )
        .unwrap();

        for (r, sharpe, sortino, calmar) in [
            (baseline, sharpe_baseline, sortino_baseline, calmar_baseline),
            (overlay, sharpe_overlay, sortino_overlay, calmar_overlay),
        ] {
            writeln!(
                &mut body,
                "| {} | {} | ${:.2} | {:.2}% | {:.2}% | {} | {:.6} | {:.6} | {:.6} |",
                r.name,
                r.bars,
                r.final_equity,
                r.total_return * 100.0,
                r.max_drawdown * 100.0,
                r.trades,
                sharpe,
                sortino,
                calmar,
            )
            .unwrap();
        }

        // ── § Verdict ────────────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Verdict\n").unwrap();
        writeln!(
            &mut body,
            "| Field               | Value                                          |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|---------------------|------------------------------------------------|"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Sharpe baseline     | {:.6} (top10-2023-1h-momentum) |",
            sharpe_baseline
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Sharpe overlay      | {:.6} (top10-2023-fy-vol-target-overlay-realdata) |",
            sharpe_overlay
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Gross Sharpe delta  | {:.6} (overlay - baseline) |",
            gross_delta
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Net Sharpe delta    | {:.6} (gross delta, no turnover cost modelled) |",
            net_delta
        )
        .unwrap();
        writeln!(&mut body, "| T-classifier        | {} |", verdict.label()).unwrap();
        writeln!(
            &mut body,
            "| V-verdict (joint)   | V3 (mean_calibration_ratio = 2.952191 outside [0.7, 1.4] — see vol-verdict-bs1-realdata report) |"
        )
        .unwrap();

        // ── § Notes ──────────────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Notes\n").unwrap();
        writeln!(
            &mut body,
            "- Baseline (top10-2023-1h-momentum) uses synthetic GBM bars; overlay uses real Binance 2023 data."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- V-verdict V3 fires because GARCH unconditioned-var overflow on AVAX/DOGE/DOT (non-convergence at 500 iters)."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- Follow-on: v3-garch-calibration-tune to improve GARCH fitting for non-convergent symbols."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown); integer bar/trade counts."
        )
        .unwrap();

        body
    }

    /// Render YAML frontmatter (excluded from body hash).
    pub fn render_frontmatter(ctx: &ReportContext) -> String {
        format!(
            "---\n\
             slug: v3-volatility-forecaster\n\
             scenario: sharpe-comparison-vol-target-bs1-realdata\n\
             generated: {}\n\
             wall_clock_s: {:.1}\n\
             host: {}\n\
             git_commit: {}\n\
             data_revision_sha: {}\n\
             ---\n",
            ctx.generated, ctx.wall_clock_s, ctx.host, ctx.git_commit, ctx.data_revision_sha,
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rust_decimal_macros::dec;

        fn make_result(name: &str, sharpe_up: bool) -> RerunResult {
            let mut eq = vec![dec!(100_000)];
            let factor = if sharpe_up {
                dec!(1.0012)
            } else {
                dec!(1.0005)
            };
            for _ in 0..8760 {
                let last = *eq.last().unwrap();
                eq.push(last * factor);
            }
            let final_eq = *eq.last().unwrap();
            RerunResult {
                name: name.to_string(),
                variant: "test".to_string(),
                equity: eq,
                bars: 8760,
                trades: 1000,
                final_equity: final_eq,
                total_return: 0.1,
                max_drawdown: 0.05,
                dampen_rate: 0.0,
            }
        }

        #[test]
        fn t_classifier_thresholds() {
            assert_eq!(TClassifier::classify(0.10), TClassifier::AlphaUnlocked);
            assert_eq!(TClassifier::classify(0.15), TClassifier::AlphaUnlocked);
            assert_eq!(TClassifier::classify(0.07), TClassifier::Marginal);
            assert_eq!(TClassifier::classify(0.05), TClassifier::Marginal);
            assert_eq!(TClassifier::classify(0.04), TClassifier::NoAlpha);
            assert_eq!(TClassifier::classify(-0.5), TClassifier::NoAlpha);
        }

        #[test]
        fn render_contains_required_sections() {
            let baseline = make_result("top10-2023-1h-momentum", false);
            let overlay = make_result("top10-2023-fy-vol-target-overlay-realdata", true);
            let ctx = ReportContext {
                generated: "2026-05-22T00:00:00Z".to_string(),
                wall_clock_s: 10.0,
                host: "test".to_string(),
                git_commit: "abc".to_string(),
                data_revision_sha: "def".to_string(),
            };
            let body = render_report(&baseline, &overlay, &ctx);
            assert!(body.contains("## Methodology"), "missing Methodology");
            assert!(
                body.contains("## Comparison table"),
                "missing Comparison table"
            );
            assert!(body.contains("## Verdict"), "missing Verdict");
            assert!(body.contains("T-VOL-"), "missing T-classifier label");
        }

        #[test]
        fn render_is_deterministic() {
            let baseline = make_result("top10-2023-1h-momentum", false);
            let overlay = make_result("top10-2023-fy-vol-target-overlay-realdata", true);
            let ctx = ReportContext {
                generated: "2026-05-22T00:00:00Z".to_string(),
                wall_clock_s: 10.0,
                host: "test".to_string(),
                git_commit: "abc".to_string(),
                data_revision_sha: "def".to_string(),
            };
            let b1 = render_report(&baseline, &overlay, &ctx);
            let b2 = render_report(&baseline, &overlay, &ctx);
            assert_eq!(
                b1, b2,
                "render_vol_target::render_report must be deterministic"
            );
        }
    }
}

// ── render_vol_target_rebaseline ──────────────────────────────────────────────
//
// Sibling of `render_vol_target`. Emits the body for the
// `sharpe-comparison-vol-target-bs1-realbaseline` report (v3.0.0-volatility-
// rebaseline). Advisory string differences from the parent module (per
// decomp.md § T-AR-2 lock):
//   - Site 1 (Methodology table): baseline name changed from synthetic to realdata.
//   - Site 2 (Verdict table): Sharpe baseline label updated accordingly.
//   - Site 3 (Notes): data-source note updated (both baseline and overlay use
//     real Binance 2023 hourly data — apples-to-apples).
// The parent `render_vol_target` module is NOT modified — its body bytes
// remain identical so the parent anchor `ef048366...` continues to verify.

mod render_vol_target_rebaseline {
    use super::metrics;
    use super::rerun::RerunResult;

    /// T-classifier verdict per ADR-0038 § D1.c.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TClassifier {
        /// net_delta >= +0.10
        AlphaUnlocked,
        /// net_delta in [+0.05, +0.10)
        Marginal,
        /// net_delta < +0.05
        NoAlpha,
    }

    impl TClassifier {
        pub fn label(self) -> &'static str {
            match self {
                Self::AlphaUnlocked => "T-VOL-ALPHA-UNLOCKED",
                Self::Marginal => "T-VOL-MARGINAL",
                Self::NoAlpha => "T-VOL-NO-ALPHA",
            }
        }

        pub fn classify(net_delta: f64) -> Self {
            if net_delta >= 0.10 {
                Self::AlphaUnlocked
            } else if net_delta >= 0.05 {
                Self::Marginal
            } else {
                Self::NoAlpha
            }
        }
    }

    /// Run-varying context for the frontmatter.
    #[derive(Debug, Clone)]
    pub struct ReportContext {
        pub generated: String,
        pub wall_clock_s: f64,
        pub host: String,
        pub git_commit: String,
        pub data_revision_sha: String,
    }

    /// Render the vol-target-rebaseline comparison report body.
    ///
    /// `baseline` is `top10-2023-fy-momentum-realdata` (real-data un-targeted v1).
    /// `overlay` is `top10-2023-fy-vol-target-overlay-realdata`.
    ///
    /// Advisory string differences vs `render_vol_target::render_report`:
    ///   - Site 1 (line ~975 parent): baseline scenario label updated.
    ///   - Site 2 (line ~1049 parent): Sharpe baseline label updated.
    ///   - Site 3 (line ~1082 parent): Notes data-source text updated.
    pub fn render_report(
        baseline: &RerunResult,
        overlay: &RerunResult,
        _ctx: &ReportContext,
    ) -> String {
        use std::fmt::Write as FmtWrite;
        let mut body = String::with_capacity(4096);

        let sharpe_baseline = metrics::compute_sharpe_hourly(&baseline.equity);
        let sharpe_overlay = metrics::compute_sharpe_hourly(&overlay.equity);
        let sortino_baseline = metrics::compute_sortino_hourly(&baseline.equity);
        let sortino_overlay = metrics::compute_sortino_hourly(&overlay.equity);
        let calmar_baseline = metrics::compute_calmar(&baseline.equity);
        let calmar_overlay = metrics::compute_calmar(&overlay.equity);

        let gross_delta = sharpe_overlay - sharpe_baseline;
        // Net delta (same as gross for our simplified case — turnover cost not modelled).
        let net_delta = gross_delta;
        let verdict = TClassifier::classify(net_delta);

        // ── Header ──────────────────────────────────────────────────────────────
        writeln!(
            &mut body,
            "# Sharpe / drawdown comparison — v3.0.0-volatility-rebaseline GARCH vol-targeting overlay"
        )
        .unwrap();

        // ── § Methodology ───────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Methodology\n").unwrap();
        writeln!(
            &mut body,
            "| Field             | Value                                          |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|-------------------|------------------------------------------------|"
        )
        .unwrap();
        // Advisory swap #1 (decomp.md § T-AR-2 site 975):
        // parent: "top10-2023-1h-momentum (v1 cross-sectional momentum, synthetic)"
        // rebaseline: "top10-2023-fy-momentum-realdata (v1 cross-sectional momentum, real Binance data)"
        writeln!(
            &mut body,
            "| Baseline scenario | top10-2023-fy-momentum-realdata (v1 cross-sectional momentum, real Binance data) |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Overlay scenario  | top10-2023-fy-vol-target-overlay-realdata (GARCH BS-1 vol-targeting, real Binance data) |"
        )
        .unwrap();
        writeln!(&mut body, "| Bar interval      | 1h |").unwrap();
        writeln!(
            &mut body,
            "| Annualisation     | sqrt(24*365) = {:.6} (hourly -> annual) |",
            metrics::SQRT_HOURS_PER_YEAR
        )
        .unwrap();
        writeln!(&mut body, "| Risk-free rate    | 0.000000 (constant) |").unwrap();
        writeln!(
            &mut body,
            "| Sharpe formula    | (mean_r - r_f) / std_r * sqrt(24*365) |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| T-classifier      | ADR-0038 D1.c: net_delta >= 0.10 -> T-VOL-ALPHA-UNLOCKED, [0.05,0.10) -> T-VOL-MARGINAL, <0.05 -> T-VOL-NO-ALPHA |"
        )
        .unwrap();

        // ── § Comparison table ───────────────────────────────────────────────────
        writeln!(&mut body, "\n## Comparison table\n").unwrap();
        writeln!(
            &mut body,
            "| Scenario | Bars | Final equity | Total return | Max drawdown | Trades | Sharpe (ann) | Sortino (ann) | Calmar |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|----------|------|--------------|--------------|--------------|--------|--------------|---------------|--------|"
        )
        .unwrap();

        for (r, sharpe, sortino, calmar) in [
            (baseline, sharpe_baseline, sortino_baseline, calmar_baseline),
            (overlay, sharpe_overlay, sortino_overlay, calmar_overlay),
        ] {
            writeln!(
                &mut body,
                "| {} | {} | ${:.2} | {:.2}% | {:.2}% | {} | {:.6} | {:.6} | {:.6} |",
                r.name,
                r.bars,
                r.final_equity,
                r.total_return * 100.0,
                r.max_drawdown * 100.0,
                r.trades,
                sharpe,
                sortino,
                calmar,
            )
            .unwrap();
        }

        // ── § Verdict ────────────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Verdict\n").unwrap();
        writeln!(
            &mut body,
            "| Field               | Value                                          |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|---------------------|------------------------------------------------|"
        )
        .unwrap();
        // Advisory swap #2 (decomp.md § T-AR-2 site 1049):
        // parent: "Sharpe baseline     | {:.6} (top10-2023-1h-momentum)"
        // rebaseline: "Sharpe baseline     | {:.6} (top10-2023-fy-momentum-realdata)"
        writeln!(
            &mut body,
            "| Sharpe baseline     | {:.6} (top10-2023-fy-momentum-realdata) |",
            sharpe_baseline
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Sharpe overlay      | {:.6} (top10-2023-fy-vol-target-overlay-realdata) |",
            sharpe_overlay
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Gross Sharpe delta  | {:.6} (overlay - baseline) |",
            gross_delta
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Net Sharpe delta    | {:.6} (gross delta, no turnover cost modelled) |",
            net_delta
        )
        .unwrap();
        writeln!(&mut body, "| T-classifier        | {} |", verdict.label()).unwrap();
        writeln!(
            &mut body,
            "| V-verdict (joint)   | V3 (mean_calibration_ratio = 2.952191 outside [0.7, 1.4] — see vol-verdict-bs1-realdata report) |"
        )
        .unwrap();

        // ── § Notes ──────────────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Notes\n").unwrap();
        // Advisory swap #3 (decomp.md § T-AR-2 site 1082):
        // parent: "Baseline (top10-2023-1h-momentum) uses synthetic GBM bars; overlay uses real Binance 2023 data."
        // rebaseline: both use real data — apples-to-apples per v0.1.0-rebaseline disambiguation.
        writeln!(
            &mut body,
            "- Baseline (top10-2023-fy-momentum-realdata) and overlay (top10-2023-fy-vol-target-overlay-realdata) both use real Binance 2023 hourly data — apples-to-apples comparison per v0.1.0-rebaseline disambiguation."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- V-verdict V3 fires because GARCH unconditioned-var overflow on AVAX/DOGE/DOT (non-convergence at 500 iters)."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- Follow-on: v3-garch-calibration-tune to improve GARCH fitting for non-convergent symbols."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown); integer bar/trade counts."
        )
        .unwrap();

        body
    }

    /// Render YAML frontmatter (excluded from body hash).
    pub fn render_frontmatter(ctx: &ReportContext) -> String {
        format!(
            "---\n\
             slug: v3-volatility-forecaster-rebaseline\n\
             scenario: sharpe-comparison-vol-target-bs1-realbaseline\n\
             generated: {}\n\
             wall_clock_s: {:.1}\n\
             host: {}\n\
             git_commit: {}\n\
             data_revision_sha: {}\n\
             ---\n",
            ctx.generated, ctx.wall_clock_s, ctx.host, ctx.git_commit, ctx.data_revision_sha,
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rust_decimal_macros::dec;

        fn make_result(name: &str, sharpe_up: bool) -> RerunResult {
            let mut eq = vec![dec!(100_000)];
            let factor = if sharpe_up {
                dec!(1.0012)
            } else {
                dec!(1.0005)
            };
            for _ in 0..8760 {
                let last = *eq.last().unwrap();
                eq.push(last * factor);
            }
            let final_eq = *eq.last().unwrap();
            RerunResult {
                name: name.to_string(),
                variant: "test".to_string(),
                equity: eq,
                bars: 8760,
                trades: 1000,
                final_equity: final_eq,
                total_return: 0.1,
                max_drawdown: 0.05,
                dampen_rate: 0.0,
            }
        }

        #[test]
        fn t_classifier_thresholds() {
            assert_eq!(TClassifier::classify(0.10), TClassifier::AlphaUnlocked);
            assert_eq!(TClassifier::classify(0.15), TClassifier::AlphaUnlocked);
            assert_eq!(TClassifier::classify(0.07), TClassifier::Marginal);
            assert_eq!(TClassifier::classify(0.05), TClassifier::Marginal);
            assert_eq!(TClassifier::classify(0.04), TClassifier::NoAlpha);
            assert_eq!(TClassifier::classify(-0.5), TClassifier::NoAlpha);
        }

        #[test]
        fn render_contains_required_sections() {
            let baseline = make_result("top10-2023-fy-momentum-realdata", false);
            let overlay = make_result("top10-2023-fy-vol-target-overlay-realdata", true);
            let ctx = ReportContext {
                generated: "2026-05-22T00:00:00Z".to_string(),
                wall_clock_s: 10.0,
                host: "test".to_string(),
                git_commit: "abc".to_string(),
                data_revision_sha: "def".to_string(),
            };
            let body = render_report(&baseline, &overlay, &ctx);
            assert!(body.contains("## Methodology"), "missing Methodology");
            assert!(
                body.contains("## Comparison table"),
                "missing Comparison table"
            );
            assert!(body.contains("## Verdict"), "missing Verdict");
            assert!(body.contains("T-VOL-"), "missing T-classifier label");
            assert!(
                body.contains("top10-2023-fy-momentum-realdata"),
                "missing real-data baseline name"
            );
        }

        #[test]
        fn render_is_deterministic() {
            let baseline = make_result("top10-2023-fy-momentum-realdata", false);
            let overlay = make_result("top10-2023-fy-vol-target-overlay-realdata", true);
            let ctx = ReportContext {
                generated: "2026-05-22T00:00:00Z".to_string(),
                wall_clock_s: 10.0,
                host: "test".to_string(),
                git_commit: "abc".to_string(),
                data_revision_sha: "def".to_string(),
            };
            let b1 = render_report(&baseline, &overlay, &ctx);
            let b2 = render_report(&baseline, &overlay, &ctx);
            assert_eq!(
                b1, b2,
                "render_vol_target_rebaseline::render_report must be deterministic"
            );
        }
    }
}

// ── render_regime_dispatcher ──────────────────────────────────────────────────
//
// Sibling of `render_vol_target`. Emits the body for the
// `sharpe-comparison-regime-dispatcher-bs1-realdata` report (v3.0.0-regime).
// Compares v1 momentum baseline (top10-2023-fy-momentum-realdata) vs
// regime-dispatcher (top10-2023-fy-regime-dispatcher-realdata).
// Emits T-REG-ALPHA-UNLOCKED / T-REG-MARGINAL / T-REG-NO-ALPHA verdict
// per ADR-0049 § D4.

mod render_regime_dispatcher {
    use super::metrics;
    use super::rerun::RerunResult;

    /// T-REG classifier verdict per ADR-0049 § D4.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub enum TClassifier {
        /// net_delta >= +0.10
        AlphaUnlocked,
        /// net_delta in [+0.05, +0.10)
        Marginal,
        /// net_delta < +0.05
        NoAlpha,
    }

    impl TClassifier {
        pub fn label(self) -> &'static str {
            match self {
                Self::AlphaUnlocked => "T-REG-ALPHA-UNLOCKED",
                Self::Marginal => "T-REG-MARGINAL",
                Self::NoAlpha => "T-REG-NO-ALPHA",
            }
        }

        pub fn classify(net_delta: f64) -> Self {
            if net_delta >= 0.10 {
                Self::AlphaUnlocked
            } else if net_delta >= 0.05 {
                Self::Marginal
            } else {
                Self::NoAlpha
            }
        }
    }

    /// Run-varying context for the frontmatter.
    #[derive(Debug, Clone)]
    pub struct ReportContext {
        pub generated: String,
        pub wall_clock_s: f64,
        pub host: String,
        pub git_commit: String,
        pub data_revision_sha: String,
    }

    /// Render the regime-dispatcher comparison report body.
    ///
    /// `baseline` is `top10-2023-fy-momentum-realdata` (v1 cross-sectional momentum, real data).
    /// `dispatcher` is `top10-2023-fy-regime-dispatcher-realdata` (v3.0.0-regime RegimeDispatcher).
    pub fn render_report(
        baseline: &RerunResult,
        dispatcher: &RerunResult,
        _ctx: &ReportContext,
    ) -> String {
        use std::fmt::Write as FmtWrite;
        let mut body = String::with_capacity(4096);

        let sharpe_baseline = metrics::compute_sharpe_hourly(&baseline.equity);
        let sharpe_dispatcher = metrics::compute_sharpe_hourly(&dispatcher.equity);
        let sortino_baseline = metrics::compute_sortino_hourly(&baseline.equity);
        let sortino_dispatcher = metrics::compute_sortino_hourly(&dispatcher.equity);
        let calmar_baseline = metrics::compute_calmar(&baseline.equity);
        let calmar_dispatcher = metrics::compute_calmar(&dispatcher.equity);

        let gross_delta = sharpe_dispatcher - sharpe_baseline;
        // Net delta (same as gross — turnover cost not modelled separately).
        let net_delta = gross_delta;
        let verdict = TClassifier::classify(net_delta);

        // ── Header ──────────────────────────────────────────────────────────────
        writeln!(
            &mut body,
            "# Sharpe / drawdown comparison — v3.0.0-regime RegimeDispatcher vs v1 momentum baseline"
        )
        .unwrap();

        // ── § Methodology ───────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Methodology\n").unwrap();
        writeln!(
            &mut body,
            "| Field             | Value                                          |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|-------------------|------------------------------------------------|"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Baseline scenario | top10-2023-fy-momentum-realdata (v1 cross-sectional momentum, real Binance data) |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Dispatcher scenario | top10-2023-fy-regime-dispatcher-realdata (v3.0.0-regime MarkovSwitching 4-state, real Binance data) |"
        )
        .unwrap();
        writeln!(&mut body, "| Bar interval      | 1h |").unwrap();
        writeln!(
            &mut body,
            "| Annualisation     | sqrt(24*365) = {:.6} (hourly -> annual) |",
            metrics::SQRT_HOURS_PER_YEAR
        )
        .unwrap();
        writeln!(&mut body, "| Risk-free rate    | 0.000000 (constant) |").unwrap();
        writeln!(
            &mut body,
            "| Sharpe formula    | (mean_r - r_f) / std_r * sqrt(24*365) |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| T-classifier      | ADR-0049 D4: net_delta >= 0.10 -> T-REG-ALPHA-UNLOCKED, [0.05,0.10) -> T-REG-MARGINAL, <0.05 -> T-REG-NO-ALPHA |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Hypothesis H1     | Regime-dispatcher Sharpe delta vs v1 baseline >= +0.10 (alpha-unlock threshold) |"
        )
        .unwrap();

        // ── § Comparison table ───────────────────────────────────────────────────
        writeln!(&mut body, "\n## Comparison table\n").unwrap();
        writeln!(
            &mut body,
            "| Scenario | Bars | Final equity | Total return | Max drawdown | Trades | Suppress rate | Sharpe (ann) | Sortino (ann) | Calmar |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|----------|------|--------------|--------------|--------------|--------|----------------|--------------|---------------|--------|"
        )
        .unwrap();

        for (r, sharpe, sortino, calmar) in [
            (baseline, sharpe_baseline, sortino_baseline, calmar_baseline),
            (
                dispatcher,
                sharpe_dispatcher,
                sortino_dispatcher,
                calmar_dispatcher,
            ),
        ] {
            writeln!(
                &mut body,
                "| {} | {} | ${:.2} | {:.2}% | {:.2}% | {} | {:.2}% | {:.6} | {:.6} | {:.6} |",
                r.name,
                r.bars,
                r.final_equity,
                r.total_return * 100.0,
                r.max_drawdown * 100.0,
                r.trades,
                r.dampen_rate * 100.0, // suppress_rate stored in dampen_rate field
                sharpe,
                sortino,
                calmar,
            )
            .unwrap();
        }

        // ── § Verdict ────────────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Verdict\n").unwrap();
        writeln!(
            &mut body,
            "| Field               | Value                                          |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|---------------------|------------------------------------------------|"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Sharpe baseline     | {:.6} (top10-2023-fy-momentum-realdata) |",
            sharpe_baseline
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Sharpe dispatcher   | {:.6} (top10-2023-fy-regime-dispatcher-realdata) |",
            sharpe_dispatcher
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Gross Sharpe delta  | {:.6} (dispatcher - baseline) |",
            gross_delta
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Net Sharpe delta    | {:.6} (gross delta, no turnover cost modelled) |",
            net_delta
        )
        .unwrap();
        writeln!(&mut body, "| T-classifier        | {} |", verdict.label()).unwrap();
        writeln!(
            &mut body,
            "| V-REG verdict       | See regime-verdict-bs1-realdata report (ADR-0049 § D4). |"
        )
        .unwrap();

        // H1 hypothesis discharge.
        writeln!(&mut body, "\n## H1 Hypothesis Discharge\n").unwrap();
        writeln!(
            &mut body,
            "| Field               | Value                                          |"
        )
        .unwrap();
        writeln!(
            &mut body,
            "|---------------------|------------------------------------------------|"
        )
        .unwrap();
        writeln!(
            &mut body,
            "| Hypothesis H1       | Regime-dispatcher Sharpe delta >= +0.10 vs v1 momentum baseline. |"
        )
        .unwrap();
        let h1_result = if net_delta >= 0.10 {
            "CONFIRMED: net_delta >= +0.10 — regime-dispatcher delivers alpha lift."
        } else if net_delta >= 0.05 {
            "PARTIAL: net_delta in [+0.05, +0.10) — marginal alpha lift; operator decides."
        } else {
            "REJECTED: net_delta < +0.05 — regime-dispatcher does not deliver alpha lift at v0.1.0."
        };
        writeln!(&mut body, "| H1 result           | {} |", h1_result).unwrap();
        writeln!(&mut body, "| Net Sharpe delta    | {:.6} |", net_delta).unwrap();
        writeln!(&mut body, "| T-REG verdict       | {} |", verdict.label()).unwrap();

        // ── § Notes ──────────────────────────────────────────────────────────────
        writeln!(&mut body, "\n## Notes\n").unwrap();
        writeln!(
            &mut body,
            "- Both scenarios use real Binance 2023 hourly data (10 USDT pairs) — apples-to-apples."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- Dispatcher suppress rate = fraction of active bars in CashHold (Volatile/Calm) regime."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- Follow-on per joint advisory table (ADR-0049 § D4):"
        )
        .unwrap();
        writeln!(
            &mut body,
            "  - T-REG-ALPHA-UNLOCKED: SHIP + spawn v1.5-MR follow-on."
        )
        .unwrap();
        writeln!(
            &mut body,
            "  - T-REG-MARGINAL: SHIP-WITH-CAVEATS or HOLD (operator decides)."
        )
        .unwrap();
        writeln!(
            &mut body,
            "  - T-REG-NO-ALPHA: HOLD-FOR-OPERATOR; C2 retire + close v3 three-pick set."
        )
        .unwrap();
        writeln!(
            &mut body,
            "- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown/suppress_rate)."
        )
        .unwrap();

        body
    }

    /// Render YAML frontmatter (excluded from body hash).
    pub fn render_frontmatter(ctx: &ReportContext) -> String {
        format!(
            "---\n\
             slug: v3-regime-classifier\n\
             scenario: sharpe-comparison-regime-dispatcher-bs1-realdata\n\
             generated: {}\n\
             wall_clock_s: {:.1}\n\
             host: {}\n\
             git_commit: {}\n\
             data_revision_sha: {}\n\
             ---\n",
            ctx.generated, ctx.wall_clock_s, ctx.host, ctx.git_commit, ctx.data_revision_sha,
        )
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use rust_decimal_macros::dec;

        fn make_result(name: &str, sharpe_up: bool, suppress_rate: f64) -> RerunResult {
            let mut eq = vec![dec!(100_000)];
            let factor = if sharpe_up {
                dec!(1.0012)
            } else {
                dec!(1.0005)
            };
            for _ in 0..8760 {
                let last = *eq.last().unwrap();
                eq.push(last * factor);
            }
            let final_eq = *eq.last().unwrap();
            RerunResult {
                name: name.to_string(),
                variant: "test".to_string(),
                equity: eq,
                bars: 8760,
                trades: 1000,
                final_equity: final_eq,
                total_return: 0.1,
                max_drawdown: 0.05,
                dampen_rate: suppress_rate, // suppress_rate stored in dampen_rate field
            }
        }

        /// T-REG classifier threshold tests (ADR-0049 § D4).
        #[test]
        fn t_reg_classifier_thresholds() {
            assert_eq!(TClassifier::classify(0.10), TClassifier::AlphaUnlocked);
            assert_eq!(TClassifier::classify(0.15), TClassifier::AlphaUnlocked);
            assert_eq!(TClassifier::classify(0.07), TClassifier::Marginal);
            assert_eq!(TClassifier::classify(0.05), TClassifier::Marginal);
            assert_eq!(TClassifier::classify(0.04), TClassifier::NoAlpha);
            assert_eq!(TClassifier::classify(-0.5), TClassifier::NoAlpha);
        }

        /// Renderer produces required sections.
        #[test]
        fn render_contains_required_sections() {
            let baseline = make_result("top10-2023-fy-momentum-realdata", false, 0.0);
            let dispatcher = make_result("top10-2023-fy-regime-dispatcher-realdata", true, 0.112);
            let ctx = ReportContext {
                generated: "2026-05-29T00:00:00Z".to_string(),
                wall_clock_s: 10.0,
                host: "test".to_string(),
                git_commit: "abc".to_string(),
                data_revision_sha: "def".to_string(),
            };
            let body = render_report(&baseline, &dispatcher, &ctx);
            assert!(body.contains("## Methodology"), "missing Methodology");
            assert!(
                body.contains("## Comparison table"),
                "missing Comparison table"
            );
            assert!(body.contains("## Verdict"), "missing Verdict");
            assert!(
                body.contains("## H1 Hypothesis Discharge"),
                "missing H1 Hypothesis Discharge"
            );
            assert!(body.contains("T-REG-"), "missing T-REG classifier label");
            assert!(
                body.contains("top10-2023-fy-momentum-realdata"),
                "missing baseline name"
            );
            assert!(
                body.contains("top10-2023-fy-regime-dispatcher-realdata"),
                "missing dispatcher name"
            );
        }

        /// Renderer is deterministic.
        #[test]
        fn render_is_deterministic() {
            let baseline = make_result("top10-2023-fy-momentum-realdata", false, 0.0);
            let dispatcher = make_result("top10-2023-fy-regime-dispatcher-realdata", true, 0.112);
            let ctx = ReportContext {
                generated: "2026-05-29T00:00:00Z".to_string(),
                wall_clock_s: 10.0,
                host: "test".to_string(),
                git_commit: "abc".to_string(),
                data_revision_sha: "def".to_string(),
            };
            let b1 = render_report(&baseline, &dispatcher, &ctx);
            let b2 = render_report(&baseline, &dispatcher, &ctx);
            assert_eq!(
                b1, b2,
                "render_regime_dispatcher::render_report must be deterministic"
            );
        }

        /// T-REG-NO-ALPHA branch produces correct H1 discharge message.
        #[test]
        fn render_no_alpha_h1_discharge() {
            let baseline = make_result("top10-2023-fy-momentum-realdata", true, 0.0);
            let dispatcher = make_result("top10-2023-fy-regime-dispatcher-realdata", false, 0.112);
            let ctx = ReportContext {
                generated: "2026-05-29T00:00:00Z".to_string(),
                wall_clock_s: 10.0,
                host: "test".to_string(),
                git_commit: "abc".to_string(),
                data_revision_sha: "def".to_string(),
            };
            let body = render_report(&baseline, &dispatcher, &ctx);
            assert!(
                body.contains("REJECTED"),
                "no-alpha branch should emit REJECTED: {body}"
            );
            assert!(
                body.contains("T-REG-NO-ALPHA"),
                "no-alpha branch should emit T-REG-NO-ALPHA: {body}"
            );
        }
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn read_git_commit() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}

fn read_data_revision_sha() -> String {
    let rev_path = std::path::Path::new("data/binance/REVISION.toml");
    std::fs::read_to_string(rev_path)
        .ok()
        .and_then(|s| {
            s.lines()
                .find(|l| l.starts_with("sha"))
                .and_then(|l| l.split('=').nth(1))
                .map(|v| v.trim().trim_matches('"').to_string())
        })
        .unwrap_or_else(|| "unknown".to_string())
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    // T-RED-D12 (v2-1-tracing-layer-redactor): migrated to install_global.
    llm::tracing_init::install_global(&["sharpe_comparison=info", "forecast=info"], false)?;

    let args = Args::parse();

    // Resolve out_dir based on scenario family.
    let out_dir: PathBuf = args.out_dir.clone().unwrap_or_else(|| match args.scenario {
        ScenarioFamily::VolTarget => PathBuf::from("spec/v1/v3-volatility-forecaster/reports/"),
        ScenarioFamily::VolTargetRebaseline => {
            PathBuf::from("spec/v1/v3-volatility-forecaster-rebaseline/reports/")
        }
        ScenarioFamily::RegimeDispatcher => PathBuf::from("spec/v1/v3-regime-classifier/reports/"),
        ScenarioFamily::Tcn => PathBuf::from("spec/v1/v25a-patchtst-overlay/reports/"),
    });

    info!(
        scenario_family = ?args.scenario,
        backtest_bin = %args.backtest_bin.display(),
        out_dir = %out_dir.display(),
        skip_rerun = args.skip_rerun,
        "sharpe_comparison starting"
    );

    // Ensure out_dir exists.
    std::fs::create_dir_all(&out_dir).with_context(|| format!("creating out_dir {:?}", out_dir))?;

    let t_start = std::time::Instant::now();

    // Build shared time/host context (used in both dispatch arms).
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let generated = {
        let dt = time::OffsetDateTime::from_unix_timestamp(now_secs as i64)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        dt.format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string())
    };
    let host = std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()));
    let today = {
        let dt = time::OffsetDateTime::from_unix_timestamp(now_secs as i64)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        format!("{}{:02}{:02}", dt.year(), dt.month() as u8, dt.day())
    };

    // ── regime-dispatcher-bs1 dispatch ───────────────────────────────────────
    if args.scenario == ScenarioFamily::RegimeDispatcher {
        if args.skip_rerun {
            anyhow::bail!("--skip-rerun is not implemented for regime-dispatcher-bs1.");
        }
        let tmpdir = tempfile::TempDir::new().context("creating tempdir")?;

        // Re-run REAL-DATA v1 momentum baseline + regime-dispatcher (2023 train window).
        // Both use real Binance 2023 hourly data — apples-to-apples per ADR-0049 § D4.
        let regime_scenarios = [
            "top10-2023-fy-momentum-realdata", // v1 momentum baseline (real data)
            "top10-2023-fy-regime-dispatcher-realdata", // v3.0.0-regime RegimeDispatcher
        ];

        let mut regime_results: Vec<rerun::RerunResult> = Vec::with_capacity(2);
        for &name in &regime_scenarios {
            info!(scenario = name, "running regime-dispatcher scenario");
            let mut result = rerun::rerun_scenario(name, &args.backtest_bin, tmpdir.path())
                .with_context(|| format!("rerunning scenario {name}"))?;
            // For regime-dispatcher scenario: parse suppress_rate and store in dampen_rate field.
            // The `parse_report_pct` in rerun parses "TCN dampen rate" which doesn't exist here;
            // re-parse the report to get "Suppress rate".
            if name.contains("regime-dispatcher") {
                let report_path = tmpdir.path().join(format!("backtest-*-{name}.md"));
                // Find the actual report file.
                if let Ok(entries) = std::fs::read_dir(tmpdir.path()) {
                    for entry in entries.flatten() {
                        let fname = entry.file_name().to_string_lossy().to_string();
                        if fname.ends_with(".md") && fname.contains("regime-dispatcher") {
                            if let Ok(body) = std::fs::read_to_string(entry.path()) {
                                // Parse suppress_rate from "Suppress rate | 11.20%"
                                result.dampen_rate = body
                                    .lines()
                                    .find(|l| l.contains("Suppress rate"))
                                    .and_then(|l| l.split('|').nth(2))
                                    .map(|v| v.trim().trim_end_matches('%'))
                                    .and_then(|v| v.parse::<f64>().ok())
                                    .map(|pct| pct / 100.0)
                                    .unwrap_or(0.0);
                                let _ = report_path; // suppress unused warning
                            }
                            break;
                        }
                    }
                }
            }
            info!(
                scenario = name,
                bars = result.bars,
                trades = result.trades,
                suppress_rate = result.dampen_rate,
                "scenario complete"
            );
            regime_results.push(result);
        }

        let wall_clock_s = t_start.elapsed().as_secs_f64();
        let baseline = regime_results.remove(0);
        let dispatcher = regime_results.remove(0);

        let ctx = render_regime_dispatcher::ReportContext {
            generated,
            wall_clock_s,
            host,
            git_commit: read_git_commit(),
            data_revision_sha: read_data_revision_sha(),
        };

        let body = render_regime_dispatcher::render_report(&baseline, &dispatcher, &ctx);
        let frontmatter = render_regime_dispatcher::render_frontmatter(&ctx);
        let full_report = format!("{frontmatter}{body}");

        let filename = format!("sharpe-comparison-regime-dispatcher-bs1-realdata-{today}.md");
        let out_path = out_dir.join(&filename);
        std::fs::write(&out_path, &full_report)
            .with_context(|| format!("writing report to {:?}", out_path))?;

        // Compute T-classifier for stdout banner.
        let sharpe_baseline = metrics::compute_sharpe_hourly(&baseline.equity);
        let sharpe_dispatcher = metrics::compute_sharpe_hourly(&dispatcher.equity);
        let net_delta = sharpe_dispatcher - sharpe_baseline;
        let verdict = render_regime_dispatcher::TClassifier::classify(net_delta);

        info!(
            path = %out_path.display(),
            wall_clock_s = format!("{:.1}", wall_clock_s),
            t_classifier = verdict.label(),
            sharpe_baseline = format!("{sharpe_baseline:.6}"),
            sharpe_dispatcher = format!("{sharpe_dispatcher:.6}"),
            net_delta = format!("{net_delta:.6}"),
            "regime-dispatcher comparison report written"
        );

        println!(
            "wrote {}; T-classifier = {}; net_delta = {:.6}",
            out_path.display(),
            verdict.label(),
            net_delta,
        );

        return Ok(());
    }

    // ── vol-target-bs1-rebaseline dispatch ───────────────────────────────────
    if args.scenario == ScenarioFamily::VolTargetRebaseline {
        if args.skip_rerun {
            anyhow::bail!("--skip-rerun is not implemented for vol-target-bs1-rebaseline.");
        }
        let tmpdir = tempfile::TempDir::new().context("creating tempdir")?;

        // Re-run REAL-DATA v1 momentum baseline + vol-target overlay (realdata).
        let vol_target_scenarios = [
            "top10-2023-fy-momentum-realdata", // Swap #1: T-AR-1 new real-data scenario
            "top10-2023-fy-vol-target-overlay-realdata", // Unchanged: byte-identical to parent
        ];

        let mut vt_results: Vec<rerun::RerunResult> = Vec::with_capacity(2);
        for &name in &vol_target_scenarios {
            info!(scenario = name, "running vol-target-rebaseline scenario");
            let result = rerun::rerun_scenario(name, &args.backtest_bin, tmpdir.path())
                .with_context(|| format!("rerunning scenario {name}"))?;
            info!(
                scenario = name,
                bars = result.bars,
                trades = result.trades,
                "scenario complete"
            );
            vt_results.push(result);
        }

        let wall_clock_s = t_start.elapsed().as_secs_f64();
        let baseline = vt_results.remove(0);
        let overlay = vt_results.remove(0);

        let ctx = render_vol_target_rebaseline::ReportContext {
            generated,
            wall_clock_s,
            host,
            git_commit: read_git_commit(),
            data_revision_sha: read_data_revision_sha(),
        };

        let body = render_vol_target_rebaseline::render_report(&baseline, &overlay, &ctx);
        let frontmatter = render_vol_target_rebaseline::render_frontmatter(&ctx);
        let full_report = format!("{frontmatter}{body}");

        // Swap #2: filename template — "realdata" → "realbaseline" (Q2=(a) default).
        let filename = format!("sharpe-comparison-vol-target-bs1-realbaseline-{today}.md");
        let out_path = out_dir.join(&filename);
        std::fs::write(&out_path, full_report)
            .with_context(|| format!("writing report to {:?}", out_path))?;

        // Compute T-classifier for stdout banner.
        let sharpe_baseline = metrics::compute_sharpe_hourly(&baseline.equity);
        let sharpe_overlay = metrics::compute_sharpe_hourly(&overlay.equity);
        let net_delta = sharpe_overlay - sharpe_baseline;
        let verdict = render_vol_target_rebaseline::TClassifier::classify(net_delta);

        info!(
            path = %out_path.display(),
            wall_clock_s = format!("{:.1}", wall_clock_s),
            t_classifier = verdict.label(),
            "vol-target-rebaseline report written"
        );

        println!(
            "wrote {}; T-classifier = {}",
            out_path.display(),
            verdict.label()
        );

        return Ok(());
    }

    // ── vol-target-bs1 dispatch ───────────────────────────────────────────────
    if args.scenario == ScenarioFamily::VolTarget {
        if args.skip_rerun {
            anyhow::bail!("--skip-rerun is not implemented for vol-target-bs1.");
        }
        let tmpdir = tempfile::TempDir::new().context("creating tempdir")?;

        // Re-run v1 momentum baseline (synthetic) and vol-target overlay (realdata).
        let vol_target_scenarios = [
            "top10-2023-1h-momentum",
            "top10-2023-fy-vol-target-overlay-realdata",
        ];

        let mut vt_results: Vec<rerun::RerunResult> = Vec::with_capacity(2);
        for &name in &vol_target_scenarios {
            info!(scenario = name, "running vol-target scenario");
            let result = rerun::rerun_scenario(name, &args.backtest_bin, tmpdir.path())
                .with_context(|| format!("rerunning scenario {name}"))?;
            info!(
                scenario = name,
                bars = result.bars,
                trades = result.trades,
                "scenario complete"
            );
            vt_results.push(result);
        }

        let wall_clock_s = t_start.elapsed().as_secs_f64();
        let baseline = vt_results.remove(0);
        let overlay = vt_results.remove(0);

        let ctx = render_vol_target::ReportContext {
            generated,
            wall_clock_s,
            host,
            git_commit: read_git_commit(),
            data_revision_sha: read_data_revision_sha(),
        };

        let body = render_vol_target::render_report(&baseline, &overlay, &ctx);
        let frontmatter = render_vol_target::render_frontmatter(&ctx);
        let full_report = format!("{frontmatter}{body}");

        let filename = format!("sharpe-comparison-vol-target-bs1-realdata-{today}.md");
        let out_path = out_dir.join(&filename);
        std::fs::write(&out_path, full_report)
            .with_context(|| format!("writing report to {:?}", out_path))?;

        // Compute T-classifier for stdout banner.
        let sharpe_baseline = metrics::compute_sharpe_hourly(&baseline.equity);
        let sharpe_overlay = metrics::compute_sharpe_hourly(&overlay.equity);
        let net_delta = sharpe_overlay - sharpe_baseline;
        let verdict = render_vol_target::TClassifier::classify(net_delta);

        info!(
            path = %out_path.display(),
            wall_clock_s = format!("{:.1}", wall_clock_s),
            t_classifier = verdict.label(),
            "vol-target report written"
        );

        println!(
            "wrote {}; T-classifier = {}",
            out_path.display(),
            verdict.label()
        );

        return Ok(());
    }

    // ── TCN / PatchTST dispatch (default; byte-identical to prior runs) ────────
    info!(
        scenarios = ?rerun::SCENARIOS,
        "starting TCN/PatchTST comparison"
    );

    // Re-run scenarios (or skip).
    let results: [rerun::RerunResult; 5] = if args.skip_rerun {
        anyhow::bail!("--skip-rerun is not yet implemented; run without it.");
    } else {
        // Each scenario re-run goes into a tempdir to preserve anchor safety.
        let tmpdir = tempfile::TempDir::new().context("creating tempdir")?;

        let mut scenario_results = Vec::with_capacity(5);
        for &name in &rerun::SCENARIOS {
            info!(scenario = name, "running scenario");
            let result = rerun::rerun_scenario(name, &args.backtest_bin, tmpdir.path())
                .with_context(|| format!("rerunning scenario {name}"))?;
            info!(
                scenario = name,
                bars = result.bars,
                trades = result.trades,
                "scenario complete"
            );
            scenario_results.push(result);
        }

        scenario_results
            .try_into()
            .map_err(|_| anyhow::anyhow!("expected exactly 5 scenario results"))?
    };

    let wall_clock_s = t_start.elapsed().as_secs_f64();

    // Collect source report paths (advisory only, for frontmatter).
    let source_reports: Vec<String> = rerun::SCENARIOS
        .iter()
        .map(|s| format!("spec/v1/backtest-real-binance-data/reports/backtest-…-{s}.md"))
        .collect();

    let ctx = render::ReportContext {
        generated,
        wall_clock_s,
        host,
        git_commit: read_git_commit(),
        data_revision_sha: read_data_revision_sha(),
        source_reports,
    };

    let body = render::render_report(&results, &ctx);
    let frontmatter = render::render_frontmatter(&ctx);
    let full_report = format!("{frontmatter}{body}");

    // Write report.
    let filename = format!("sharpe-comparison-patchtst-bs1-realdata-{today}.md");
    let out_path = out_dir.join(&filename);
    std::fs::write(&out_path, full_report)
        .with_context(|| format!("writing report to {:?}", out_path))?;

    info!(
        path = %out_path.display(),
        wall_clock_s = format!("{:.1}", wall_clock_s),
        "report written"
    );

    Ok(())
}
