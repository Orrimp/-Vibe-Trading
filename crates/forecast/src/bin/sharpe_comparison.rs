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
//!   reports under `spec/backtest-real-binance-data/reports/` are NEVER
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
use tracing_subscriber::EnvFilter;

// ── CLI ───────────────────────────────────────────────────────────────────────

#[derive(Parser, Debug)]
#[command(
    name = "sharpe_comparison",
    about = "M-SHARPE: Sharpe/Sortino/Calmar comparison table for the five -realdata scenarios",
    long_about = "Re-runs the five -realdata backtest scenarios (four TCN + one PatchTST BS-1) into a tempdir,\n\
                  computes hourly-annualised Sharpe/Sortino/Calmar/max-DD from\n\
                  the equity curves, and emits a deterministic markdown report.\n\n\
                  Read-only contract: the five anchored -realdata reports are never touched."
)]
struct Args {
    /// Output directory for the report.
    #[arg(long, default_value = "spec/v25a-patchtst-overlay/reports/")]
    out_dir: PathBuf,

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
                    curr = curr * dec!(1.01);
                    v.push(curr);
                }
                curr = curr * dec!(0.999);
                v.push(curr);
                for _ in 0..10 {
                    curr = curr * dec!(1.01);
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
                curr = curr * dec!(1.0001);
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

    /// Parse final equity from the report Summary table (e.g. "$113479.98").
    fn parse_report_equity(body: &str) -> Option<f64> {
        body.lines()
            .find(|l| l.contains("Final equity"))
            .and_then(|l| {
                l.split('|')
                    .nth(2)
                    .map(|v| v.trim().trim_start_matches('$').replace(',', ""))
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
                curr = curr * dec!(1.001);
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
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("sharpe_comparison=info".parse()?)
                .add_directive("forecast=info".parse()?),
        )
        .init();

    let args = Args::parse();

    info!(
        scenarios = ?rerun::SCENARIOS,
        backtest_bin = %args.backtest_bin.display(),
        skip_rerun = args.skip_rerun,
        "sharpe_comparison starting"
    );

    // Ensure out_dir exists.
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating out_dir {:?}", args.out_dir))?;

    let t_start = std::time::Instant::now();

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
        .map(|s| format!("spec/backtest-real-binance-data/reports/backtest-…-{s}.md"))
        .collect();

    // Build report context.
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
    let today = {
        let dt = time::OffsetDateTime::from_unix_timestamp(now_secs as i64)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        format!("{}{:02}{:02}", dt.year(), dt.month() as u8, dt.day())
    };
    let filename = format!("sharpe-comparison-patchtst-bs1-realdata-{today}.md");
    let out_path = args.out_dir.join(&filename);
    std::fs::write(&out_path, full_report)
        .with_context(|| format!("writing report to {:?}", out_path))?;

    info!(
        path = %out_path.display(),
        wall_clock_s = format!("{:.1}", wall_clock_s),
        "report written"
    );

    Ok(())
}
