//! `forecast_distribution` — M-R-HAT forecast-distribution inspector.
//!
//! Reads an anchored TCN checkpoint (BS-1 or BS-2), runs a forward pass over
//! all 10 USDT symbols for the checkpoint's evaluation span, and emits a
//! deterministic markdown report under `spec/v25-tcn-alpha-investigation/reports/`.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p forecast --bin forecast_distribution --features candle -- \
//!   --scenario bs1
//!
//! cargo run -p forecast --bin forecast_distribution --features candle -- \
//!   --scenario bs2
//! ```
//!
//! ## Read-only contract (K5)
//!
//! - NO writes to `crates/forecast/checkpoints/`.
//! - NO writes to `crates/forecast/replay-cache/`.
//! - Exactly one filesystem-write call: `std::fs::write(out_path, body)`
//!   where `out_path` is under `--out-dir`.
//! - No `--retrain`, `--update-sigma`, `--write-checkpoint` flags exist.
//!
//! ## Determinism (K3)
//!
//! - No `SystemTime::now()` on any hot path (wall-clock goes to frontmatter only).
//! - Sort uses `f32::total_cmp` (total order, NaN-safe).
//! - All floats serialised with fixed precision per ADR-0033 § D2.a.
//!
//! ## Cross-references
//!
//! - ADR-0033 § D1 — bin placement rationale.
//! - ADR-0033 § D2.a — report shape + float canonicalisation.
//! - ADR-0033 § D3 — F-verdict algorithm.
//! - `crates/forecast/src/tcn.rs:472` — `TcnForecaster::load_anchor`.
//! - `crates/forecast/src/features.rs:489` — `windows_for_symbol`.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
use tracing_subscriber::EnvFilter;

use forecast::{
    features::{FeatureConfig, TimeSpan, windows_for_symbol},
    tcn::{AnchorScenario, TcnForecaster},
};

// ── CLI ───────────────────────────────────────────────────────────────────────

/// Which anchored checkpoint to inspect.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ScenarioArg {
    /// BS-1: trained Jan–Sep 2023, evaluated 2023-01-01..2024-01-01.
    Bs1,
    /// BS-2: trained 2023 full year, evaluated 2024-01-01..2025-01-01.
    Bs2,
}

impl ScenarioArg {
    fn to_anchor(self) -> AnchorScenario {
        match self {
            ScenarioArg::Bs1 => AnchorScenario::Bs1,
            ScenarioArg::Bs2 => AnchorScenario::Bs2,
        }
    }

    /// Default evaluation span (UTC, half-open) for each checkpoint.
    fn default_span(self) -> (time::OffsetDateTime, time::OffsetDateTime) {
        match self {
            ScenarioArg::Bs1 => (
                time::macros::datetime!(2023-01-01 00:00:00 UTC),
                time::macros::datetime!(2024-01-01 00:00:00 UTC),
            ),
            ScenarioArg::Bs2 => (
                time::macros::datetime!(2024-01-01 00:00:00 UTC),
                time::macros::datetime!(2025-01-01 00:00:00 UTC),
            ),
        }
    }

    fn label(self) -> &'static str {
        match self {
            ScenarioArg::Bs1 => "bs1",
            ScenarioArg::Bs2 => "bs2",
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "forecast_distribution",
    about = "M-R-HAT: inspect forecast distribution of an anchored TCN checkpoint (read-only)",
    long_about = "Runs forward passes over all 10 USDT symbols for the checkpoint's evaluation span\n\
                  and emits a deterministic markdown report.\n\n\
                  Read-only contract: no writes to checkpoints/ or replay-cache/."
)]
struct Args {
    /// Which anchored checkpoint to inspect.
    #[arg(long, value_enum)]
    scenario: ScenarioArg,

    /// Parquet root for real OHLCV bars.
    #[arg(long, default_value = "data/binance/")]
    data_root: PathBuf,

    /// Output directory for the report.
    #[arg(long, default_value = "spec/v25-tcn-alpha-investigation/reports/")]
    out_dir: PathBuf,

    /// Evaluation span lower bound (UTC inclusive). Defaults to scenario default.
    #[arg(long)]
    span_start: Option<String>,

    /// Evaluation span upper bound (UTC exclusive). Defaults to scenario default.
    #[arg(long)]
    span_end: Option<String>,
}

// ── Statistics module ─────────────────────────────────────────────────────────

mod hist {
    /// Summary statistics over a `r_hat` sample.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Stats {
        pub count: usize,
        pub mean: f64,
        pub std: f64,
        pub min: f64,
        pub max: f64,
        pub p01: f64,
        pub p05: f64,
        pub p10: f64,
        pub p25: f64,
        pub p50: f64,
        pub p75: f64,
        pub p90: f64,
        pub p95: f64,
        pub p99: f64,
        pub abs_p50: f64,
        pub abs_p95: f64,
        pub abs_p99: f64,
    }

    /// Histogram over 100 fixed bins in `[-3·σ_train, +3·σ_train]`.
    #[derive(Debug, Clone)]
    pub struct Histogram {
        /// σ_train used to define the bin range.
        pub sigma_train: f32,
        /// 100 bin counts.
        pub count: Vec<u64>,
    }

    impl Histogram {
        /// Low edge of bin `i` (in raw r_hat units).
        pub fn bin_low(&self, i: usize) -> f64 {
            let sigma = self.sigma_train as f64;
            let range = 6.0 * sigma;
            let step = range / 100.0;
            -3.0 * sigma + i as f64 * step
        }

        /// High edge of bin `i` (in raw r_hat units).
        pub fn bin_high(&self, i: usize) -> f64 {
            let sigma = self.sigma_train as f64;
            let range = 6.0 * sigma;
            let step = range / 100.0;
            -3.0 * sigma + (i + 1) as f64 * step
        }
    }

    /// Type-7 quantile (linear interpolation between two nearest order-stats).
    ///
    /// `q` must be in `[0.0, 1.0]`. Returns `f64::NAN` on empty slice.
    fn quantile_type7(sorted: &[f32], q: f64) -> f64 {
        let n = sorted.len();
        if n == 0 {
            return f64::NAN;
        }
        if n == 1 {
            return sorted[0] as f64;
        }
        let h = (n as f64 - 1.0) * q;
        let lo = h.floor() as usize;
        let hi = h.ceil() as usize;
        let frac = h - lo as f64;
        let v_lo = sorted[lo] as f64;
        let v_hi = sorted[hi.min(n - 1)] as f64;
        v_lo + frac * (v_hi - v_lo)
    }

    /// Compute summary statistics over a `r_hat` sample.
    ///
    /// Returns a zeroed `Stats` with `count=0` on empty input.
    pub fn summary_stats(r_hat: &[f32]) -> Stats {
        let n = r_hat.len();
        if n == 0 {
            return Stats {
                count: 0,
                mean: f64::NAN,
                std: f64::NAN,
                min: f64::NAN,
                max: f64::NAN,
                p01: f64::NAN,
                p05: f64::NAN,
                p10: f64::NAN,
                p25: f64::NAN,
                p50: f64::NAN,
                p75: f64::NAN,
                p90: f64::NAN,
                p95: f64::NAN,
                p99: f64::NAN,
                abs_p50: f64::NAN,
                abs_p95: f64::NAN,
                abs_p99: f64::NAN,
            };
        }

        // Sort a copy using total_cmp (NaN-safe total order).
        let mut sorted: Vec<f32> = r_hat.to_vec();
        sorted.sort_unstable_by(f32::total_cmp);

        let sum: f64 = sorted.iter().map(|&x| x as f64).sum();
        let mean = sum / n as f64;
        let variance: f64 = sorted
            .iter()
            .map(|&x| (x as f64 - mean).powi(2))
            .sum::<f64>()
            / n as f64;
        let std = variance.sqrt();

        let min = sorted[0] as f64;
        let max = sorted[n - 1] as f64;

        let p01 = quantile_type7(&sorted, 0.01);
        let p05 = quantile_type7(&sorted, 0.05);
        let p10 = quantile_type7(&sorted, 0.10);
        let p25 = quantile_type7(&sorted, 0.25);
        let p50 = quantile_type7(&sorted, 0.50);
        let p75 = quantile_type7(&sorted, 0.75);
        let p90 = quantile_type7(&sorted, 0.90);
        let p95 = quantile_type7(&sorted, 0.95);
        let p99 = quantile_type7(&sorted, 0.99);

        // Absolute-value percentiles.
        let mut abs_sorted: Vec<f32> = r_hat.iter().map(|&x| x.abs()).collect();
        abs_sorted.sort_unstable_by(f32::total_cmp);
        let abs_p50 = quantile_type7(&abs_sorted, 0.50);
        let abs_p95 = quantile_type7(&abs_sorted, 0.95);
        let abs_p99 = quantile_type7(&abs_sorted, 0.99);

        Stats {
            count: n,
            mean,
            std,
            min,
            max,
            p01,
            p05,
            p10,
            p25,
            p50,
            p75,
            p90,
            p95,
            p99,
            abs_p50,
            abs_p95,
            abs_p99,
        }
    }

    /// Build a 100-bin histogram over `[-3·σ_train, +3·σ_train]`.
    ///
    /// Bins are half-open `[low, high)`. Out-of-range values are clamped to
    /// the first or last bin (saturating).
    pub fn histogram(r_hat: &[f32], sigma_train: f32) -> Histogram {
        let sigma = sigma_train as f64;
        let low = -3.0 * sigma;
        let high = 3.0 * sigma;
        let range = high - low;
        let bins = 100usize;
        let step = range / bins as f64;

        let mut count = vec![0u64; bins];
        for &v in r_hat {
            let v = v as f64;
            let idx = if v < low {
                0
            } else if v >= high {
                bins - 1
            } else {
                let raw = ((v - low) / step) as usize;
                raw.min(bins - 1)
            };
            count[idx] += 1;
        }

        Histogram { sigma_train, count }
    }

    /// Gate survival: fraction of bars with `|r_hat| / σ_train >= τ`
    /// for τ ∈ {0.1, 0.2, …, 0.9}.
    pub fn gate_survival(r_hat: &[f32], sigma_train: f32) -> [f32; 9] {
        let n = r_hat.len();
        if n == 0 {
            return [0.0f32; 9];
        }
        let mut result = [0.0f32; 9];
        for (i, &tau_tenth) in [1u32, 2, 3, 4, 5, 6, 7, 8, 9].iter().enumerate() {
            let tau = tau_tenth as f32 * 0.1;
            let passing = r_hat
                .iter()
                .filter(|&&v| v.abs() / sigma_train >= tau)
                .count();
            result[i] = passing as f32 / n as f32;
        }
        result
    }

    // ── Unit tests ────────────────────────────────────────────────────────────

    #[cfg(test)]
    mod tests {
        use super::*;

        const EPSILON: f64 = 1e-6;

        /// (a) summary_stats on a 9-element fixture with hand-computed percentiles.
        #[test]
        fn test_summary_stats_fixture() {
            // Sorted: [-4, -3, -2, -1, 0, 1, 2, 3, 4]
            let v: Vec<f32> = vec![-4.0, 2.0, -1.0, 3.0, 0.0, -2.0, 4.0, 1.0, -3.0];
            let s = summary_stats(&v);
            assert_eq!(s.count, 9);
            assert!((s.mean - 0.0).abs() < EPSILON, "mean={}", s.mean);
            // std = sqrt(mean of squares) = sqrt((16+9+4+1+0+1+4+9+16)/9) = sqrt(60/9) ≈ 2.5820
            let expected_std = (60.0f64 / 9.0).sqrt();
            assert!(
                (s.std - expected_std).abs() < 1e-4,
                "std={} expected={}",
                s.std,
                expected_std
            );
            assert!((s.min - (-4.0)).abs() < EPSILON);
            assert!((s.max - 4.0).abs() < EPSILON);
            // p50: sorted[4] = 0.0 (median of 9 elements, h=(9-1)*0.5=4.0)
            assert!((s.p50 - 0.0).abs() < EPSILON, "p50={}", s.p50);
            // p25: h=(9-1)*0.25=2.0 → sorted[2]=-2.0
            assert!((s.p25 - (-2.0)).abs() < EPSILON, "p25={}", s.p25);
            // p75: h=(9-1)*0.75=6.0 → sorted[6]=2.0
            assert!((s.p75 - 2.0).abs() < EPSILON, "p75={}", s.p75);
        }

        /// (b) percentile-of-empty returns NaN sentinels.
        #[test]
        fn test_summary_stats_empty() {
            let s = summary_stats(&[]);
            assert_eq!(s.count, 0);
            assert!(s.mean.is_nan());
            assert!(s.std.is_nan());
            assert!(s.p50.is_nan());
            assert!(s.abs_p95.is_nan());
        }

        /// (c) histogram bin-edge inclusiveness: value exactly at low edge goes
        /// to bin 0; value at high edge goes to last bin (clamped).
        #[test]
        fn test_histogram_bin_edges() {
            let sigma = 1.0f32;
            // Exactly at -3σ: should land in bin 0.
            let at_low = [-3.0f32];
            let h = histogram(&at_low, sigma);
            assert_eq!(h.count[0], 1, "value at low edge should be in bin 0");
            assert_eq!(h.count[99], 0);

            // Exactly at +3σ: should be clamped to last bin.
            let at_high = [3.0f32];
            let h = histogram(&at_high, sigma);
            assert_eq!(h.count[99], 1, "value at high edge should be in last bin");
            assert_eq!(h.count[0], 0);
        }

        /// (d) histogram clamping for out-of-range r_hat.
        #[test]
        fn test_histogram_clamping() {
            let sigma = 1.0f32;
            let out_low = [-100.0f32];
            let h = histogram(&out_low, sigma);
            assert_eq!(h.count[0], 1, "very negative value should go to bin 0");

            let out_high = [100.0f32];
            let h = histogram(&out_high, sigma);
            assert_eq!(h.count[99], 1, "very positive value should go to last bin");
        }

        /// (e) gate survival is monotone-decreasing in τ.
        #[test]
        fn test_gate_survival_monotone() {
            // Build a spread of values so not all survive every tau.
            let v: Vec<f32> = (0..100).map(|i| (i as f32 - 50.0) * 0.1).collect();
            let sigma = 1.0f32;
            let gs = gate_survival(&v, sigma);
            for i in 0..8 {
                assert!(
                    gs[i] >= gs[i + 1],
                    "gate_survival not monotone at i={}: {} < {}",
                    i,
                    gs[i],
                    gs[i + 1]
                );
            }
        }

        /// (f) determinism: summary_stats is pure — two calls return byte-identical
        /// results.
        #[test]
        fn test_summary_stats_deterministic() {
            let v: Vec<f32> = (0..1000).map(|i| (i as f32 * 0.001) - 0.5).collect();
            let s1 = summary_stats(&v);
            let s2 = summary_stats(&v);
            assert_eq!(s1.count, s2.count);
            // Compare bit patterns (NaN-safe: if both are NaN they compare equal via
            // the is_nan() path; if neither is NaN, they must be equal bits).
            assert_eq!(s1.mean.to_bits(), s2.mean.to_bits());
            assert_eq!(s1.std.to_bits(), s2.std.to_bits());
            assert_eq!(s1.p50.to_bits(), s2.p50.to_bits());
            assert_eq!(s1.abs_p95.to_bits(), s2.abs_p95.to_bits());
        }
    }
}

// ── Verdict module ────────────────────────────────────────────────────────────

pub mod verdict {
    /// Per-checkpoint inputs for the F-verdict classifier (ADR-0033 § D3.a).
    #[derive(Debug, Clone)]
    pub struct CheckpointStats {
        pub abs_p95: f32,
        pub abs_p99: f32,
        pub std: f32,
        pub sigma_train: f32,
        pub epsilon: f32,
        pub tau: f32,
        pub frac_inside_epsilon: f32,
        pub frac_passes_confidence_gate: f32,
        pub confidence_gate_survival: [f32; 9],
    }

    /// F-verdict per ADR-0033 § D3.b.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Verdict {
        F1 {
            evidence: String,
            follow_on: &'static str,
        },
        F2 {
            evidence: String,
            follow_on: &'static str,
        },
        F3 {
            evidence: String,
            follow_on: &'static str,
        },
        F4 {
            evidence: String,
            follow_on: &'static str,
        },
    }

    impl Verdict {
        pub fn label(&self) -> &'static str {
            match self {
                Verdict::F1 { .. } => "F1",
                Verdict::F2 { .. } => "F2",
                Verdict::F3 { .. } => "F3",
                Verdict::F4 { .. } => "F4",
            }
        }

        pub fn evidence(&self) -> &str {
            match self {
                Verdict::F1 { evidence, .. }
                | Verdict::F2 { evidence, .. }
                | Verdict::F3 { evidence, .. }
                | Verdict::F4 { evidence, .. } => evidence,
            }
        }

        pub fn follow_on(&self) -> &'static str {
            match self {
                Verdict::F1 { follow_on, .. }
                | Verdict::F2 { follow_on, .. }
                | Verdict::F3 { follow_on, .. }
                | Verdict::F4 { follow_on, .. } => follow_on,
            }
        }
    }

    /// Priority-ordered F-verdict classifier per ADR-0033 § D3.b.
    ///
    /// Returns exactly one of F1/F2/F3/F4.
    pub fn classify(s: &CheckpointStats) -> Verdict {
        // F1 — Training collapse: output numerically zero everywhere.
        if (s.abs_p95 as f64) < 1e-6 {
            return Verdict::F1 {
                evidence: format!("abs_p95 = {:.9} < 1e-6", s.abs_p95),
                follow_on: "v25-tcn-retrain",
            };
        }

        // F2 — sigma_train mis-calibration: meaningful spread but no bar passes
        // confidence gate.
        if s.std > 0.1 * s.sigma_train && (s.frac_passes_confidence_gate as f64) < 1e-6 {
            return Verdict::F2 {
                evidence: format!(
                    "std = {:.9}, sigma_train = {:.6}, std/sigma_train = {:.3}, \
                     frac_passes_confidence_gate = {:.9}",
                    s.std,
                    s.sigma_train,
                    s.std / s.sigma_train,
                    s.frac_passes_confidence_gate
                ),
                follow_on: "v25-tcn-recalibrate",
            };
        }

        // F3 — Gating too tight: real but small-magnitude signal exists, (ε, τ)
        // pair filters it. Operationalised as:
        //   - confidence_gate_survival at τ=0.6 (index 5) >= 1e-4
        //   - frac_inside_epsilon > 0.5
        if s.confidence_gate_survival[5] >= 1e-4 && s.frac_inside_epsilon > 0.5 {
            return Verdict::F3 {
                evidence: format!(
                    "frac_inside_epsilon = {:.6}, confidence_gate_survival[τ=0.6] = {:.6}",
                    s.frac_inside_epsilon, s.confidence_gate_survival[5]
                ),
                follow_on: "v25-tcn-threshold-tuning",
            };
        }

        // F4 — Fallback: model output not zero, not mis-calibrated, not gate-filtered.
        Verdict::F4 {
            evidence: format!(
                "abs_p95 = {:.9} >= 1e-6, std/sigma_train = {:.3} <= 0.1 OR \
                 frac_passes_confidence_gate = {:.9} >= 1e-6, \
                 frac_inside_epsilon = {:.6} <= 0.5",
                s.abs_p95,
                s.std / s.sigma_train,
                s.frac_passes_confidence_gate,
                s.frac_inside_epsilon
            ),
            follow_on: "v25-tcn-horizon-bump-or-retire",
        }
    }
}

// ── Report context ────────────────────────────────────────────────────────────

/// Run-varying fields stored in frontmatter (excluded from body hash).
struct ReportContext {
    generated: String,
    wall_clock_s: f64,
    host: String,
    git_commit: String,
    model_revision: String,
    sigma_train: f32,
    data_revision_sha: String,
    scenario_label: String,
    span_start: String,
    span_end: String,
    symbols: Vec<String>,
    total_inferences: usize,
    verdict_label: String,
}

// ── Report renderer ───────────────────────────────────────────────────────────

/// Render the YAML frontmatter (NOT included in body hash).
fn render_frontmatter(ctx: &ReportContext) -> String {
    format!(
        "---\n\
         slug: v25-tcn-alpha-investigation\n\
         scenario: forecast-distribution-{}-realdata\n\
         generated: {}\n\
         wall_clock_s: {:.1}\n\
         host: {}\n\
         git_commit: {}\n\
         model_revision: {}\n\
         sigma_train: {:.6}\n\
         data_revision_sha: {}\n\
         verdict: {}\n\
         ---\n",
        ctx.scenario_label,
        ctx.generated,
        ctx.wall_clock_s,
        ctx.host,
        ctx.git_commit,
        ctx.model_revision,
        ctx.sigma_train,
        ctx.data_revision_sha,
        ctx.verdict_label,
    )
}

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Parse an RFC-3339 string into `time::OffsetDateTime`.
fn parse_rfc3339(s: &str) -> Result<time::OffsetDateTime> {
    time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .with_context(|| format!("invalid RFC-3339 timestamp: {s}"))
}

/// Read the git HEAD commit hash (best-effort; returns "unknown" on failure).
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

/// Read the hostname (best-effort; returns "unknown" on failure).
fn read_hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown".to_string()))
}

/// Read `data/binance/REVISION.toml` revision SHA (best-effort).
fn read_data_revision_sha(data_root: &std::path::Path) -> String {
    let rev_path = data_root.join("REVISION.toml");
    std::fs::read_to_string(&rev_path)
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
                .add_directive("forecast_distribution=info".parse()?)
                .add_directive("forecast=info".parse()?),
        )
        .init();

    let args = Args::parse();

    // Resolve evaluation span.
    let (span_start, span_end) = match (&args.span_start, &args.span_end) {
        (Some(s), Some(e)) => (parse_rfc3339(s)?, parse_rfc3339(e)?),
        (None, None) => args.scenario.default_span(),
        _ => anyhow::bail!("--span-start and --span-end must both be provided or both omitted"),
    };
    let span = TimeSpan::new(span_start, span_end);

    // Load checkpoint.
    let anchor = args.scenario.to_anchor();
    let forecaster = TcnForecaster::load_anchor(anchor).context("loading anchor checkpoint")?;

    info!(
        model_revision = %forecaster.model_revision,
        sigma_train = forecaster.sigma_train,
        scenario = args.scenario.label(),
        "checkpoint loaded"
    );

    // The forward-pass collection loop and report write are in the
    // Wave 2/3 implementation (T-D-2, T-D-4). In this skeleton we
    // parse args and prove the CLI surface compiles.
    //
    // Ensure out_dir exists.
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("creating out_dir {:?}", args.out_dir))?;

    // ── Forward-pass collection loop (T-D-2) ──────────────────────────────────
    let symbols = [
        "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT", "DOTUSDT", "ETHUSDT", "LINKUSDT",
        "SOLUSDT", "XRPUSDT",
    ];

    let feat_cfg = FeatureConfig::default();
    let mut r_hat_all: Vec<f32> = Vec::with_capacity(90_000);
    let t_start = std::time::Instant::now();

    for &symbol in &symbols {
        let iter = windows_for_symbol(&args.data_root, symbol, span.clone(), &feat_cfg);
        let mut sym_count = 0usize;
        for window_result in iter {
            let window =
                window_result.with_context(|| format!("feature window for symbol {symbol}"))?;

            // Reshape [context_bars, 5] → [1, 5, context_bars] for the model.
            let x = window
                .features
                .transpose(0, 1)
                .context("transpose features")?
                .unsqueeze(0)
                .context("unsqueeze batch dim")?;

            let out = forecaster.forward(&x, false).context("TCN forward pass")?;
            let vals: Vec<f32> = out
                .flatten_all()
                .context("flatten output")?
                .to_vec1()
                .context("to_vec1")?;
            r_hat_all.push(vals[0]);
            sym_count += 1;
        }
        info!(symbol, windows = sym_count, "forward passes complete");
    }

    let wall_clock_s = t_start.elapsed().as_secs_f64();
    let total_inferences = r_hat_all.len();
    info!(
        total_inferences,
        wall_clock_s = format!("{:.1}", wall_clock_s),
        "forward-pass loop complete"
    );

    // ── Statistics ────────────────────────────────────────────────────────────
    let sigma_train = forecaster.sigma_train;
    let epsilon = 0.0005_f32;
    let tau = 0.60_f32;

    let stats = hist::summary_stats(&r_hat_all);
    let histogram = hist::histogram(&r_hat_all, sigma_train);
    let gate = hist::gate_survival(&r_hat_all, sigma_train);

    // Compute frac_inside_epsilon and frac_passes_confidence_gate.
    let n = r_hat_all.len();
    let frac_inside_epsilon = if n == 0 {
        0.0f32
    } else {
        r_hat_all.iter().filter(|&&v| v.abs() <= epsilon).count() as f32 / n as f32
    };
    // gate[5] = τ=0.6 survival
    let frac_passes_confidence_gate = gate[5];

    let checkpoint_stats = verdict::CheckpointStats {
        abs_p95: stats.abs_p95 as f32,
        abs_p99: stats.abs_p99 as f32,
        std: stats.std as f32,
        sigma_train,
        epsilon,
        tau,
        frac_inside_epsilon,
        frac_passes_confidence_gate,
        confidence_gate_survival: gate,
    };

    let v = verdict::classify(&checkpoint_stats);

    // ── Report context ────────────────────────────────────────────────────────
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let generated = {
        let secs = now.as_secs();
        // Format as ISO-8601 (second precision, UTC).
        let dt = time::OffsetDateTime::from_unix_timestamp(secs as i64)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        dt.format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "unknown".to_string())
    };
    let host = read_hostname();
    let git_commit = read_git_commit();
    let data_revision_sha = read_data_revision_sha(&args.data_root);

    let span_start_str = span_start
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());
    let span_end_str = span_end
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string());

    let ctx = ReportContext {
        generated,
        wall_clock_s,
        host,
        git_commit,
        model_revision: forecaster.model_revision.clone(),
        sigma_train,
        data_revision_sha,
        scenario_label: args.scenario.label().to_string(),
        span_start: span_start_str,
        span_end: span_end_str,
        symbols: symbols.iter().map(|s| s.to_string()).collect(),
        total_inferences,
        verdict_label: v.label().to_string(),
    };

    // Render report (frontmatter excluded from body hash).
    let body = render_report_full(
        &stats,
        &histogram,
        &gate,
        &v,
        &ctx,
        frac_inside_epsilon,
        frac_passes_confidence_gate,
    );
    let frontmatter = render_frontmatter(&ctx);
    let full_report = format!("{frontmatter}{body}");

    // Write report.
    let today = {
        let dt = time::OffsetDateTime::from_unix_timestamp(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
        )
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        format!("{}{:02}{:02}", dt.year(), dt.month() as u8, dt.day())
    };
    let filename = format!(
        "forecast-distribution-{}-realdata-{}.md",
        args.scenario.label(),
        today
    );
    let out_path = args.out_dir.join(&filename);
    std::fs::write(&out_path, full_report)
        .with_context(|| format!("writing report to {:?}", out_path))?;

    info!(
        path = %out_path.display(),
        verdict = v.label(),
        "report written"
    );

    Ok(())
}

/// Full report renderer with actual gate fractions wired in.
#[allow(clippy::too_many_arguments)]
fn render_report_full(
    stats: &hist::Stats,
    histogram: &hist::Histogram,
    gate: &[f32; 9],
    v: &verdict::Verdict,
    ctx: &ReportContext,
    frac_inside_epsilon: f32,
    frac_passes_confidence_gate: f32,
) -> String {
    use std::fmt::Write as FmtWrite;
    let mut body = String::with_capacity(8192);
    let sigma = histogram.sigma_train;
    let epsilon = 0.0005_f32;
    let tau = 0.60_f32;

    // ── Header ────────────────────────────────────────────────────────────────
    writeln!(
        &mut body,
        "# Forecast-distribution report — {} (real Binance hourly OHLCV)",
        ctx.scenario_label.to_uppercase()
    )
    .unwrap();

    // ── § Checkpoint ─────────────────────────────────────────────────────────
    writeln!(&mut body, "\n## Checkpoint\n").unwrap();
    writeln!(
        &mut body,
        "| Field            | Value                                          |"
    )
    .unwrap();
    writeln!(
        &mut body,
        "|------------------|------------------------------------------------|"
    )
    .unwrap();
    writeln!(&mut body, "| Anchor scenario  | {} |", ctx.scenario_label).unwrap();
    writeln!(&mut body, "| model_revision   | {} |", ctx.model_revision).unwrap();
    writeln!(&mut body, "| sigma_train      | {:.6} |", ctx.sigma_train).unwrap();
    writeln!(&mut body, "| ε (deadband)     | {:.6} |", epsilon).unwrap();
    writeln!(&mut body, "| τ (confidence)   | {:.6} |", tau).unwrap();

    // ── § Evaluation span ─────────────────────────────────────────────────────
    writeln!(&mut body, "\n## Evaluation span\n").unwrap();
    writeln!(
        &mut body,
        "| Field            | Value                                          |"
    )
    .unwrap();
    writeln!(
        &mut body,
        "|------------------|------------------------------------------------|"
    )
    .unwrap();
    writeln!(
        &mut body,
        "| Source           | Binance Vision via data/binance/ |"
    )
    .unwrap();
    writeln!(
        &mut body,
        "| Revision SHA     | {} |",
        ctx.data_revision_sha
    )
    .unwrap();
    writeln!(
        &mut body,
        "| Span (UTC, half-open) | {} .. {} |",
        ctx.span_start, ctx.span_end
    )
    .unwrap();
    writeln!(
        &mut body,
        "| Symbols (10)     | {} |",
        ctx.symbols.join(", ")
    )
    .unwrap();
    writeln!(&mut body, "| Inferences       | {} |", ctx.total_inferences).unwrap();

    // ── § Summary statistics ──────────────────────────────────────────────────
    writeln!(
        &mut body,
        "\n## Summary statistics — r_hat (raw, pre-direction-quantisation)\n"
    )
    .unwrap();
    writeln!(&mut body, "| Stat         | Value           |").unwrap();
    writeln!(&mut body, "|--------------|-----------------|").unwrap();
    writeln!(&mut body, "| count        | {} |", stats.count).unwrap();
    writeln!(&mut body, "| mean         | {:.9} |", stats.mean).unwrap();
    writeln!(&mut body, "| std          | {:.9} |", stats.std).unwrap();
    writeln!(&mut body, "| min          | {:.9} |", stats.min).unwrap();
    writeln!(&mut body, "| p01          | {:.9} |", stats.p01).unwrap();
    writeln!(&mut body, "| p05          | {:.9} |", stats.p05).unwrap();
    writeln!(&mut body, "| p10          | {:.9} |", stats.p10).unwrap();
    writeln!(&mut body, "| p25          | {:.9} |", stats.p25).unwrap();
    writeln!(&mut body, "| p50          | {:.9} |", stats.p50).unwrap();
    writeln!(&mut body, "| p75          | {:.9} |", stats.p75).unwrap();
    writeln!(&mut body, "| p90          | {:.9} |", stats.p90).unwrap();
    writeln!(&mut body, "| p95          | {:.9} |", stats.p95).unwrap();
    writeln!(&mut body, "| p99          | {:.9} |", stats.p99).unwrap();
    writeln!(&mut body, "| max          | {:.9} |", stats.max).unwrap();
    writeln!(&mut body, "| abs_p50      | {:.9} |", stats.abs_p50).unwrap();
    writeln!(&mut body, "| abs_p95      | {:.9} |", stats.abs_p95).unwrap();
    writeln!(&mut body, "| abs_p99      | {:.9} |", stats.abs_p99).unwrap();

    writeln!(&mut body).unwrap();
    writeln!(&mut body, "| Gate              | Fraction of bars |").unwrap();
    writeln!(&mut body, "|-------------------|------------------|").unwrap();
    writeln!(
        &mut body,
        "| \\|r_hat\\| ≤ ε     | {:.6} |",
        frac_inside_epsilon
    )
    .unwrap();
    writeln!(
        &mut body,
        "| \\|r_hat\\|/σ_train ≥ τ | {:.6} |",
        frac_passes_confidence_gate
    )
    .unwrap();

    // ── § Histogram ───────────────────────────────────────────────────────────
    writeln!(
        &mut body,
        "\n## Histogram — r_hat over [-3σ_train, +3σ_train]\n"
    )
    .unwrap();
    writeln!(
        &mut body,
        "100 fixed bins, half-open `[low, high)`. Bin counts as integers.\n"
    )
    .unwrap();
    writeln!(
        &mut body,
        "| bin_idx | bin_low (×10⁻⁶) | bin_high (×10⁻⁶) | count   |"
    )
    .unwrap();
    writeln!(
        &mut body,
        "|---------|-----------------|-------------------|---------|"
    )
    .unwrap();
    for i in 0..100usize {
        let low = histogram.bin_low(i);
        let high = histogram.bin_high(i);
        let low_micro = (low * 1e6) as i64;
        let high_micro = (high * 1e6) as i64;
        writeln!(
            &mut body,
            "| {:03}     | {:>15} | {:>17} | {:>7} |",
            i, low_micro, high_micro, histogram.count[i]
        )
        .unwrap();
    }

    // ── § Confidence-gate survival ────────────────────────────────────────────
    writeln!(
        &mut body,
        "\n## Confidence-gate survival — \\|r_hat\\|/σ_train per candidate τ\n"
    )
    .unwrap();
    writeln!(&mut body, "| τ    | bars surviving | fraction       |").unwrap();
    writeln!(&mut body, "|------|----------------|----------------|").unwrap();
    let taus = [0.10f32, 0.20, 0.30, 0.40, 0.50, 0.60, 0.70, 0.80, 0.90];
    for (i, &t) in taus.iter().enumerate() {
        let frac = gate[i];
        let bars = (frac as f64 * stats.count as f64).round() as u64;
        writeln!(&mut body, "| {:.2} | {:>14} | {:.6} |", t, bars, frac).unwrap();
    }

    // ── § Verdict ─────────────────────────────────────────────────────────────
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
    writeln!(&mut body, "| Case              | {} |", v.label()).unwrap();
    writeln!(&mut body, "| Trigger evidence  | {} |", v.evidence()).unwrap();
    writeln!(
        &mut body,
        "| Recommended follow-on | spawn feature `{}`. Operator-decide. |",
        v.follow_on()
    )
    .unwrap();

    // ── § Notes ───────────────────────────────────────────────────────────────
    writeln!(&mut body, "\n## Notes\n").unwrap();
    writeln!(&mut body, "- Read-only against checkpoint anchors.").unwrap();
    writeln!(
        &mut body,
        "- ε = {:.4} per v25-tcn-overlay/feature.md § R6.",
        epsilon
    )
    .unwrap();
    writeln!(
        &mut body,
        "- τ = {:.1} per v25-tcn-overlay/feature.md § D5.",
        tau
    )
    .unwrap();
    writeln!(
        &mut body,
        "- σ_train = {:.6} (from checkpoint metadata).",
        sigma
    )
    .unwrap();
    writeln!(
        &mut body,
        "- Histogram: 100 fixed bins over [-3·σ_train, +3·σ_train], ASCII-only, LF-only, integer counts."
    )
    .unwrap();
    writeln!(&mut body, "- F-verdict algorithm: see ADR-0033 § D3.").unwrap();

    body
}
