//! `vol_verdict` — GARCH(1,1) V-verdict report bin.
//!
//! Reads the anchored GARCH(1,1) checkpoint (BS-1 by default), runs
//! the GARCH recurrence over every bar in the evaluation span for all
//! 10 USDT-quote symbols, computes per-symbol QLIKE vs. the Parkinson
//! realized-vol target, and emits a deterministic markdown report under
//! `evidence/v1/v3-volatility-forecaster/reports/`.
//!
//! ## Usage
//!
//! ```bash
//! cargo run -p forecast --bin vol_verdict --features candle --release -- --scenario bs1
//! ```
//!
//! ## Read-only contract (K5 analog for vol)
//!
//! - NO writes to `crates/forecast/checkpoints/`.
//! - NO writes to replay-cache.
//! - Exactly one filesystem-write: `std::fs::write(out_path, body)` under `--out-dir`.
//!
//! ## Determinism (K3 / R11.9)
//!
//! - No `SystemTime::now()` on any hot path — wall-clock + generated timestamp
//!   go to YAML frontmatter only.
//! - All floats serialised with fixed precision per ADR-0038 § D2.a.
//! - Symbol-row order alphabetical USDT-quote (locked, ADR-0038 § D2.a).
//!
//! ## V-verdict priority tree (ADR-0038 § D1.b)
//!
//! V1 (CoV collapse) → V2 (QLIKE dispersion) → V3 (calibration drift) →
//! V4 (no improvement over constant-σ) → V5 (healthy fallback).
//!
//! ## Cross-references
//!
//! - ADR-0038 § D1 — V-verdict algorithm.
//! - ADR-0038 § D2.a — report body shape + float canonicalisation.
//! - ADR-0038 § D3 — GARCH(1,1) baseline contract.
//! - `crates/forecast/src/garch.rs` — `GarchModel::forecast_step`.
//! - `crates/forecast/src/features.rs` — `windows_for_symbol` + `VolTargetKind`.
//! - `crates/forecast/src/bin/forecast_distribution.rs` — sibling bin (F-verdict).

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::time::Instant;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::info;
// EnvFilter now used via llm::tracing_init::install_global (T-RED-D12).

use forecast::features::{FeatureConfig, TimeSpan, VolTargetKind, windows_for_symbol};
use forecast::garch::GarchModel;

// ── CLI ───────────────────────────────────────────────────────────────────────

/// Which anchored GARCH checkpoint to evaluate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ScenarioArg {
    /// BS-1: GARCH trained on full 2023 (2023-01-01..2024-01-01).
    Bs1,
}

impl ScenarioArg {
    fn eval_span(self) -> (time::OffsetDateTime, time::OffsetDateTime) {
        match self {
            ScenarioArg::Bs1 => (
                time::macros::datetime!(2023-01-01 00:00:00 UTC),
                time::macros::datetime!(2024-01-01 00:00:00 UTC),
            ),
        }
    }

    fn label(self) -> &'static str {
        match self {
            ScenarioArg::Bs1 => "bs1",
        }
    }

    /// Find the GARCH checkpoint JSON in the anchors directory.
    ///
    /// Looks for `garch-<label>-*.json` — picks the lexicographically
    /// largest (newest revision) if multiple exist.
    fn find_checkpoint(self, anchors_dir: &std::path::Path) -> Result<PathBuf> {
        let prefix = format!("garch-{}-", self.label());
        let mut matches: Vec<PathBuf> = std::fs::read_dir(anchors_dir)
            .with_context(|| format!("read anchors dir {}", anchors_dir.display()))?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.extension().map(|e| e == "json").unwrap_or(false)
                    && p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.starts_with(&prefix))
                        .unwrap_or(false)
            })
            .collect();
        matches.sort();
        matches.into_iter().next_back().with_context(|| {
            format!(
                "no garch-{}-*.json found in {}",
                self.label(),
                anchors_dir.display()
            )
        })
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "vol_verdict",
    about = "GARCH(1,1) V-verdict report bin — per ADR-0038 § D2.a",
    long_about = "Runs GARCH(1,1) forecasts over all 10 USDT symbols for the BS-1 eval span,\n\
                  computes per-symbol QLIKE vs Parkinson realized-vol target,\n\
                  and emits a deterministic V-verdict markdown report.\n\n\
                  Read-only contract: no writes to checkpoints/ or replay-cache/."
)]
struct Args {
    /// Which anchored GARCH checkpoint to inspect.
    #[arg(long, value_enum, default_value = "bs1")]
    scenario: ScenarioArg,

    /// Parquet root for real OHLCV bars.
    #[arg(long, default_value = "data/binance/")]
    data_root: PathBuf,

    /// Output directory for the V-verdict report.
    #[arg(long, default_value = "evidence/v1/v3-volatility-forecaster/reports/")]
    out_dir: PathBuf,

    /// Anchors directory for the GARCH JSON checkpoint.
    #[arg(long, default_value = "crates/forecast/checkpoints/anchors/")]
    anchors_dir: PathBuf,

    /// Evaluation span lower bound (UTC inclusive, e.g. `2023-01-01T00:00:00Z`).
    /// Auto-derived from scenario if omitted.
    #[arg(long)]
    span_start: Option<String>,

    /// Evaluation span upper bound (UTC exclusive).
    /// Auto-derived from scenario if omitted.
    #[arg(long)]
    span_end: Option<String>,
}

// ── Universe ──────────────────────────────────────────────────────────────────

/// Alphabetical USDT-quote universe (locked, ADR-0038 § D2.a).
const UNIVERSE: &[&str] = &[
    "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT", "DOTUSDT", "ETHUSDT", "LINKUSDT",
    "SOLUSDT", "XRPUSDT",
];

// ── Checkpoint types (mirrors train_garch.rs) ────────────────────────────────

#[derive(Debug, serde::Deserialize)]
struct SymbolParams {
    omega: f64,
    alpha: f64,
    beta: f64,
    unconditional_var: f64,
    log_likelihood: f64,
    n_iters: usize,
    converged: bool,
}

#[derive(Debug, serde::Deserialize)]
struct GarchCheckpoint {
    schema_version: u32,
    target_kind: String,
    target_horizon_bars: u32,
    train_span_start: String,
    train_span_end: String,
    data_revision_sha: String,
    params: BTreeMap<String, SymbolParams>,
}

// ── Per-symbol statistics (ADR-0038 § D1.a) ──────────────────────────────────

/// Per-symbol statistics computed over the evaluation span.
#[derive(Debug, Clone)]
struct PerSymbolStats {
    symbol: String,
    n_predictions: u64,
    qlike_garch: f64,
    qlike_constant: f64,
    mean_sigma_hat: f64,
    mean_sigma_realized: f64,
    std_sigma_hat: f64,
    std_sigma_realized: f64,
}

impl PerSymbolStats {
    /// Calibration ratio: mean_sigma_hat / mean_sigma_realized.
    fn calibration_ratio(&self) -> f64 {
        self.mean_sigma_hat / self.mean_sigma_realized.max(1e-12)
    }

    /// Relative QLIKE improvement over constant-σ baseline (in [0,1]):
    /// (qlike_constant - qlike_garch) / qlike_constant
    fn improvement_pct(&self) -> f64 {
        let improvement = (self.qlike_constant - self.qlike_garch) / self.qlike_constant.max(1e-12);
        improvement * 100.0
    }
}

// ── Aggregate statistics (ADR-0038 § D1.a) ───────────────────────────────────

struct AggregateStats {
    qlike_garch_mean: f64,
    qlike_constant_mean: f64,
    qlike_garch_max: f64,
    qlike_garch_min: f64,
    qlike_dispersion: f64,
    mean_calibration_ratio: f64,
    n_symbols_improving: usize,
}

fn compute_aggregate(per_symbol: &[PerSymbolStats]) -> AggregateStats {
    let n = per_symbol.len() as f64;
    let qlike_garch_mean = per_symbol.iter().map(|s| s.qlike_garch).sum::<f64>() / n;
    let qlike_constant_mean = per_symbol.iter().map(|s| s.qlike_constant).sum::<f64>() / n;
    let qlike_garch_max = per_symbol
        .iter()
        .map(|s| s.qlike_garch)
        .fold(f64::NEG_INFINITY, f64::max);
    let qlike_garch_min = per_symbol
        .iter()
        .map(|s| s.qlike_garch)
        .fold(f64::INFINITY, f64::min);
    let qlike_dispersion = if qlike_garch_min > 1e-12 {
        qlike_garch_max / qlike_garch_min
    } else {
        f64::INFINITY
    };
    let mean_calibration_ratio = per_symbol
        .iter()
        .map(|s| s.calibration_ratio())
        .sum::<f64>()
        / n;
    let n_symbols_improving = per_symbol
        .iter()
        .filter(|s| (s.qlike_constant - s.qlike_garch) / s.qlike_constant.max(1e-12) >= 0.10)
        .count();

    AggregateStats {
        qlike_garch_mean,
        qlike_constant_mean,
        qlike_garch_max,
        qlike_garch_min,
        qlike_dispersion,
        mean_calibration_ratio,
        n_symbols_improving,
    }
}

// ── Verdict ───────────────────────────────────────────────────────────────────

/// V-verdict variants per ADR-0038 § D1.b.
#[derive(Debug, Clone, PartialEq)]
enum Verdict {
    V1 {
        evidence: String,
        follow_on: &'static str,
    },
    V2 {
        evidence: String,
        follow_on: &'static str,
    },
    V3 {
        evidence: String,
        follow_on: &'static str,
    },
    V4 {
        evidence: String,
        follow_on: &'static str,
    },
    V5 {
        evidence: String,
        follow_on: &'static str,
    },
}

impl Verdict {
    fn label(&self) -> &'static str {
        match self {
            Verdict::V1 { .. } => "V1",
            Verdict::V2 { .. } => "V2",
            Verdict::V3 { .. } => "V3",
            Verdict::V4 { .. } => "V4",
            Verdict::V5 { .. } => "V5",
        }
    }

    fn evidence(&self) -> &str {
        match self {
            Verdict::V1 { evidence, .. }
            | Verdict::V2 { evidence, .. }
            | Verdict::V3 { evidence, .. }
            | Verdict::V4 { evidence, .. }
            | Verdict::V5 { evidence, .. } => evidence.as_str(),
        }
    }

    fn follow_on(&self) -> &str {
        match self {
            Verdict::V1 { follow_on, .. }
            | Verdict::V2 { follow_on, .. }
            | Verdict::V3 { follow_on, .. }
            | Verdict::V4 { follow_on, .. }
            | Verdict::V5 { follow_on, .. } => follow_on,
        }
    }

    fn routes_to(&self) -> &str {
        match self {
            Verdict::V5 { .. } => "V_ALPHA strategy-side gate (Sharpe-comparison bin)",
            Verdict::V1 { follow_on, .. }
            | Verdict::V2 { follow_on, .. }
            | Verdict::V3 { follow_on, .. }
            | Verdict::V4 { follow_on, .. } => follow_on,
        }
    }
}

/// V1-V5 priority tree per ADR-0038 § D1.b.
///
/// Locked thresholds (non-mutable per ADR-0038 § Alternatives):
/// - V1: CoV(σ̂) < 1e-3 on EVERY symbol (constant collapse).
/// - V2: qlike_dispersion > 3.0 (per-symbol mis-fit).
/// - V3: mean_calibration_ratio outside [0.7, 1.4] (calibration drift).
/// - V4: n_symbols_improving_≥10pct < 7/10 (no improvement over constant-σ).
/// - V5: fallback (all V1-V4 false).
fn classify_verdict(agg: &AggregateStats, per_symbol: &[PerSymbolStats]) -> Verdict {
    // V1 — Constant collapse.
    let max_cov = per_symbol
        .iter()
        .map(|s| s.std_sigma_hat / s.mean_sigma_hat.max(1e-12))
        .fold(0.0_f64, f64::max);
    if per_symbol
        .iter()
        .all(|s| s.std_sigma_hat / s.mean_sigma_hat.max(1e-12) < 1e-3)
    {
        let worst_symbol = per_symbol
            .iter()
            .max_by(|a, b| {
                (a.std_sigma_hat / a.mean_sigma_hat.max(1e-12))
                    .partial_cmp(&(b.std_sigma_hat / b.mean_sigma_hat.max(1e-12)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|s| s.symbol.as_str())
            .unwrap_or("?");
        return Verdict::V1 {
            evidence: format!(
                "max CoV(σ̂) = {max_cov:.6} < 1e-3 across all 10 symbols (worst-symbol = {worst_symbol})"
            ),
            follow_on: "v3-garch-refit-diagnose",
        };
    }

    // V2 — Per-symbol mis-fit.
    if agg.qlike_dispersion > 3.0 {
        return Verdict::V2 {
            evidence: format!(
                "qlike_dispersion = qlike_garch_max / qlike_garch_min = {:.6} > 3.0 \
                 (max = {:.6}, min = {:.6})",
                agg.qlike_dispersion, agg.qlike_garch_max, agg.qlike_garch_min,
            ),
            follow_on: "v3-garch-per-symbol-hyperparam-search",
        };
    }

    // V3 — Calibration drift.
    if agg.mean_calibration_ratio < 0.7 || agg.mean_calibration_ratio > 1.4 {
        return Verdict::V3 {
            evidence: format!(
                "mean_calibration_ratio = mean_over_symbols(mean(σ̂)/mean(σ_realized)) = {:.6} \
                 outside [0.7, 1.4]",
                agg.mean_calibration_ratio,
            ),
            follow_on: "v3-garch-calibration-tune",
        };
    }

    // V4 — No improvement over constant-σ baseline.
    if agg.n_symbols_improving < 7 {
        return Verdict::V4 {
            evidence: format!(
                "n_symbols_improving_≥10pct_over_constant_sigma = {} < 7 of 10",
                agg.n_symbols_improving,
            ),
            follow_on: "v3-data-vol-investigation",
        };
    }

    // V5 — Healthy fallback.
    Verdict::V5 {
        evidence: format!(
            "n_improving = {} ≥ 7; qlike_dispersion = {:.6} ≤ 3.0; \
             mean_calibration_ratio = {:.6} ∈ [0.7, 1.4]",
            agg.n_symbols_improving, agg.qlike_dispersion, agg.mean_calibration_ratio,
        ),
        follow_on: "v_alpha_strategy_gate",
    }
}

// ── QLIKE computation (Patton 2011) ──────────────────────────────────────────

/// QLIKE loss per Patton 2011 — invariant to noise in the Parkinson proxy.
///
/// `QLIKE(σ̂², σ_real²) = (σ_real² / σ̂²) - ln(σ_real² / σ̂²) - 1`
///
/// where `σ_real² = sigma_realized²` and `σ̂² = sigma_hat²`.
/// Non-negative; lower is better; zero iff σ̂ ≡ σ_real.
#[inline]
fn qlike_single(sigma_hat: f64, sigma_realized: f64) -> f64 {
    let sh2 = sigma_hat * sigma_hat;
    let sr2 = sigma_realized * sigma_realized;
    if sh2 <= 0.0 || sr2 <= 0.0 {
        return 0.0;
    }
    let ratio = sr2 / sh2;
    ratio - ratio.ln() - 1.0
}

// ── Report rendering (ADR-0038 § D2.a) ───────────────────────────────────────

#[allow(clippy::too_many_arguments)]
fn render_report(
    per_symbol: &[PerSymbolStats],
    agg: &AggregateStats,
    verdict: &Verdict,
    checkpoint: &GarchCheckpoint,
    checkpoint_revision: &str,
    generated: &str,
    wall_clock_s: f64,
    n_predictions_total: u64,
    data_revision_sha: &str,
) -> String {
    let host = hostname();
    let git_commit = git_head_sha();

    // ── Frontmatter (advisory, NOT hashed) ────────────────────────────────────
    let frontmatter = format!(
        "---\n\
         slug: v3-volatility-forecaster\n\
         scenario: vol-verdict-bs1-realdata\n\
         generated: {generated}\n\
         wall_clock_s: {:.1}\n\
         host: {host}\n\
         git_commit: {git_commit}\n\
         checkpoint_revision: {checkpoint_revision}\n\
         data_revision_sha: {data_revision_sha}\n\
         verdict: {}\n\
         ---\n",
        wall_clock_s,
        verdict.label(),
    );

    // ── Body (deterministic, hashed) ──────────────────────────────────────────
    let mut body = String::new();

    body.push_str(
        "# Vol-forecast V-verdict report — BS-1 (real Binance hourly OHLCV, GARCH(1,1))\n\n",
    );

    // Checkpoint section.
    body.push_str("## Checkpoint\n\n");
    body.push_str("| Field               | Value                                          |\n");
    body.push_str("|---------------------|------------------------------------------------|\n");
    body.push_str(&format!(
        "| Anchor scenario     | garch-{}                                       |\n",
        "bs1"
    ));
    body.push_str(&format!(
        "| checkpoint_revision | {checkpoint_revision} |\n"
    ));
    body.push_str(&format!(
        "| target_kind         | {}                                             |\n",
        checkpoint.target_kind
    ));
    body.push_str(&format!(
        "| target_horizon_bars | {}                                              |\n",
        checkpoint.target_horizon_bars
    ));
    body.push_str(&format!(
        "| evaluation_span     | {} .. {}   |\n",
        checkpoint.train_span_start, checkpoint.train_span_end
    ));
    body.push_str(&format!(
        "| n_symbols           | {}                                              |\n",
        UNIVERSE.len()
    ));
    body.push_str(&format!(
        "| n_predictions_total | {n_predictions_total}                                          |\n"
    ));
    body.push('\n');

    // Per-symbol QLIKE table.
    body.push_str("## Per-symbol QLIKE table\n\n");
    body.push_str(
        "| symbol   | n_pred | qlike_garch | qlike_const | improvement_pct | \
         mean_sigma_hat | mean_sigma_real | calib_ratio | std_sigma_hat | std_sigma_real |\n",
    );
    body.push_str(
        "|----------|--------|-------------|-------------|-----------------|----------------|-----------------|-------------|---------------|----------------|\n",
    );
    for s in per_symbol {
        body.push_str(&format!(
            "| {:<8} | {:<6} | {:.6}    | {:.6}    | {:.6}        | {:.6}       | {:.6}        | {:.6}    | {:.6}      | {:.6}       |\n",
            s.symbol,
            s.n_predictions,
            s.qlike_garch,
            s.qlike_constant,
            s.improvement_pct(),
            s.mean_sigma_hat,
            s.mean_sigma_realized,
            s.calibration_ratio(),
            s.std_sigma_hat,
            s.std_sigma_realized,
        ));
    }
    body.push('\n');

    // Aggregate statistics.
    body.push_str("## Aggregate statistics\n\n");
    body.push_str("| Field                       | Value      |\n");
    body.push_str("|-----------------------------|------------|\n");
    body.push_str(&format!(
        "| qlike_garch_mean            | {:.6}   |\n",
        agg.qlike_garch_mean
    ));
    body.push_str(&format!(
        "| qlike_constant_mean         | {:.6}   |\n",
        agg.qlike_constant_mean
    ));
    body.push_str(&format!(
        "| qlike_garch_max             | {:.6}   |\n",
        agg.qlike_garch_max
    ));
    body.push_str(&format!(
        "| qlike_garch_min             | {:.6}   |\n",
        agg.qlike_garch_min
    ));
    body.push_str(&format!(
        "| qlike_dispersion            | {:.6}   |\n",
        agg.qlike_dispersion
    ));
    body.push_str(&format!(
        "| mean_calibration_ratio      | {:.6}   |\n",
        agg.mean_calibration_ratio
    ));
    body.push_str(&format!(
        "| n_symbols_improving_≥10pct  | {}          |\n",
        agg.n_symbols_improving
    ));
    body.push('\n');

    // Verdict section.
    body.push_str("## Verdict\n\n");
    body.push_str("| Field             | Value                                          |\n");
    body.push_str("|-------------------|------------------------------------------------|\n");
    body.push_str(&format!(
        "| Case              | {}                                             |\n",
        verdict.label()
    ));
    body.push_str(&format!("| Trigger evidence  | {} |\n", verdict.evidence()));
    body.push_str(&format!(
        "| Routes to         | {} |\n",
        verdict.routes_to()
    ));
    body.push('\n');

    // Notes section.
    body.push_str("## Notes\n\n");
    body.push_str(&format!(
        "- Read-only against `crates/forecast/checkpoints/anchors/garch-bs1-{checkpoint_revision}.json`.\n"
    ));
    body.push_str(
        "- QLIKE per Patton 2011 *Volatility forecast comparison using\n  \
         imperfect volatility proxies* — robust to noise in the Parkinson\n  \
         σ_realized proxy; preferred over MSE for vol forecasts.\n",
    );
    body.push_str(
        "- Parkinson realized-vol target: \
         `σ̂_P² = (1/(4·ln 2)) · mean over k of (ln(high_k/low_k))²`.\n",
    );
    body.push_str(
        "- V-verdict algorithm: see \
         [ADR-0038 § D1](../architecture/adr/0038-vol-forecast-verdict-shape.md#d1-v-verdict-priority-tree-parallel-to-adr-0033--d3-not-extension).\n",
    );

    // Combine.
    format!("{frontmatter}{body}")
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn hostname() -> String {
    std::process::Command::new("hostname")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn git_head_sha() -> String {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

fn now_iso8601() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let odt = time::OffsetDateTime::from_unix_timestamp(now as i64)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        odt.year(),
        odt.month() as u8,
        odt.day(),
        odt.hour(),
        odt.minute(),
        odt.second(),
    )
}

fn today_yyyymmdd() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let odt = time::OffsetDateTime::from_unix_timestamp(now as i64)
        .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!("{:04}{:02}{:02}", odt.year(), odt.month() as u8, odt.day())
}

fn parse_ts(s: &str) -> Result<time::OffsetDateTime> {
    time::OffsetDateTime::parse(s, &time::format_description::well_known::Rfc3339)
        .with_context(|| format!("parse timestamp '{s}'"))
}

/// Derive checkpoint_revision from the JSON file name (last 64-hex characters before `.json`).
fn revision_from_path(path: &std::path::Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .and_then(|s| s.split('-').next_back())
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string())
}

/// Compute body-only SHA-256 of the report (for the stdout confirmation line).
fn body_sha256(report: &str) -> String {
    use sha2::{Digest, Sha256};
    // Strip frontmatter (lines between first --- and second ---).
    let mut in_frontmatter = false;
    let mut past_frontmatter = false;
    let mut body_lines: Vec<&str> = Vec::new();
    for (i, line) in report.lines().enumerate() {
        if i == 0 && line.trim() == "---" {
            in_frontmatter = true;
            continue;
        }
        if in_frontmatter && line.trim() == "---" {
            in_frontmatter = false;
            past_frontmatter = true;
            continue;
        }
        if past_frontmatter {
            body_lines.push(line);
        }
    }
    let body = body_lines.join("\n");
    let digest = Sha256::digest(body.as_bytes());
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(64);
    for b in &digest {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    // T-RED-D12 (v2-1-tracing-layer-redactor): migrated to install_global.
    llm::tracing_init::install_global(&[], false)?;

    let args = Args::parse();
    let t0 = Instant::now();

    // ── Derive evaluation span ────────────────────────────────────────────────
    let (span_start_dt, span_end_dt) = match (&args.span_start, &args.span_end) {
        (Some(s), Some(e)) => (parse_ts(s)?, parse_ts(e)?),
        (None, None) => args.scenario.eval_span(),
        _ => anyhow::bail!("must supply both --span-start and --span-end, or neither"),
    };
    let span = TimeSpan::new(span_start_dt, span_end_dt);

    // ── Load GARCH checkpoint ─────────────────────────────────────────────────
    let checkpoint_path = args.scenario.find_checkpoint(&args.anchors_dir)?;
    let checkpoint_revision = revision_from_path(&checkpoint_path);
    info!(
        checkpoint_path = %checkpoint_path.display(),
        checkpoint_revision = %checkpoint_revision,
        "loading GARCH checkpoint"
    );

    let json_str = std::fs::read_to_string(&checkpoint_path)
        .with_context(|| format!("read checkpoint {}", checkpoint_path.display()))?;
    let checkpoint: GarchCheckpoint =
        serde_json::from_str(&json_str).with_context(|| "parse GARCH checkpoint JSON")?;

    if checkpoint.schema_version != 1 {
        anyhow::bail!(
            "unsupported checkpoint schema_version: {} (expected 1)",
            checkpoint.schema_version
        );
    }

    let data_revision_sha = checkpoint.data_revision_sha.clone();

    // Build per-symbol GarchModel from checkpoint params.
    let models: std::collections::HashMap<String, GarchModel> = checkpoint
        .params
        .iter()
        .map(|(sym, p)| {
            (
                sym.clone(),
                GarchModel {
                    omega: p.omega,
                    alpha: p.alpha,
                    beta: p.beta,
                    unconditional_var: p.unconditional_var,
                    log_likelihood: p.log_likelihood,
                    n_iters: p.n_iters,
                    converged: p.converged,
                },
            )
        })
        .collect();

    info!(
        n_symbols = models.len(),
        scenario = args.scenario.label(),
        "GARCH checkpoint loaded"
    );

    // ── Feature config: Parkinson target + 24-bar horizon ─────────────────────
    let feat_cfg = FeatureConfig {
        context_bars: 336,       // PatchTST-style 336-bar context window
        target_horizon_bars: 24, // 24-bar Parkinson horizon (ADR-0038 § D3)
        vol_target_kind: Some(VolTargetKind::Parkinson),
        ..FeatureConfig::default()
    };

    // ── Per-symbol statistics accumulator ────────────────────────────────────
    let mut all_stats: Vec<PerSymbolStats> = Vec::with_capacity(10);
    let mut n_predictions_total: u64 = 0;

    for &symbol in UNIVERSE {
        let model = models
            .get(symbol)
            .with_context(|| format!("symbol {symbol} not found in checkpoint"))?;

        // Warmup: unconditional_var.sqrt() as initial sigma.
        let sigma_init = model.unconditional_var.sqrt().max(model.omega.sqrt());

        let iter = windows_for_symbol(&args.data_root, symbol, span.clone(), &feat_cfg);

        let mut sigma_hat_vec: Vec<f64> = Vec::with_capacity(9000);
        let mut sigma_real_vec: Vec<f64> = Vec::with_capacity(9000);

        // Running GARCH state: sigma from the previous bar.
        let mut sigma_prev = sigma_init;

        // We also need log-returns for the GARCH recurrence.  We extract these
        // from the feature vector (column 0 = logret, f32) and cast to f64.
        // The first bar's logret is r_{t-1} for the first forecast.
        let mut r_prev = 0.0_f64; // initial log-return (treat warm-up as zero)

        for window_result in iter {
            let window = window_result.with_context(|| format!("feature window for {symbol}"))?;

            // sigma_hat: GARCH forecast using r_prev and sigma_prev.
            let sigma_hat = model.forecast_step(r_prev, sigma_prev);

            // Extract target Parkinson sigma from the window.
            let sigma_realized = window
                .target_parkinson_vol
                .map(|v| v as f64)
                .unwrap_or(0.0_f64);

            if sigma_realized > 0.0 {
                sigma_hat_vec.push(sigma_hat);
                sigma_real_vec.push(sigma_realized);
            }

            // Advance state: r_prev = last bar's logret (column 0 of last row of features).
            // Feature matrix is [context, FEATURE_DIM], row-major. Last row = window.features
            // rows [context-1]. In the `Vec<f32>` path: index = (context-1)*FEATURE_DIM + 0.
            // We cast from the feature vector safely.
            #[cfg(not(feature = "candle"))]
            {
                let ctx = feat_cfg.context_bars;
                let feat = &window.features;
                if feat.len() >= ctx * 5 {
                    r_prev = feat[(ctx - 1) * 5] as f64;
                }
            }
            #[cfg(feature = "candle")]
            {
                // Extract last row, column 0 from the candle tensor via narrow.
                let ctx = feat_cfg.context_bars;
                // narrow(dim=0, start=ctx-1, len=1) selects row ctx-1 as shape [1, FEATURE_DIM].
                if let Ok(row_t) = window.features.narrow(0, ctx - 1, 1)
                    && let Ok(rows) = row_t.to_vec2::<f32>()
                    && let Some(row) = rows.first()
                    && let Some(&v) = row.first()
                {
                    r_prev = v as f64;
                }
            }

            sigma_prev = sigma_hat;
        }

        let n = sigma_hat_vec.len() as u64;
        n_predictions_total += n;

        if n == 0 {
            anyhow::bail!("no valid predictions for symbol {symbol}");
        }

        let nf = n as f64;

        // Compute QLIKE_GARCH.
        let qlike_garch = sigma_hat_vec
            .iter()
            .zip(sigma_real_vec.iter())
            .map(|(&sh, &sr)| qlike_single(sh, sr))
            .sum::<f64>()
            / nf;

        // Compute QLIKE_CONSTANT using unconditional_var.sqrt() as constant σ.
        let sigma_const = model.unconditional_var.sqrt().max(model.omega.sqrt());
        let qlike_constant = sigma_real_vec
            .iter()
            .map(|&sr| qlike_single(sigma_const, sr))
            .sum::<f64>()
            / nf;

        // Mean and std of sigma_hat.
        let mean_sigma_hat = sigma_hat_vec.iter().sum::<f64>() / nf;
        let var_sigma_hat = sigma_hat_vec
            .iter()
            .map(|&v| (v - mean_sigma_hat).powi(2))
            .sum::<f64>()
            / (nf - 1.0).max(1.0);
        let std_sigma_hat = var_sigma_hat.sqrt();

        // Mean and std of sigma_realized.
        let mean_sigma_realized = sigma_real_vec.iter().sum::<f64>() / nf;
        let var_sigma_realized = sigma_real_vec
            .iter()
            .map(|&v| (v - mean_sigma_realized).powi(2))
            .sum::<f64>()
            / (nf - 1.0).max(1.0);
        let std_sigma_realized = var_sigma_realized.sqrt();

        info!(
            symbol = symbol,
            n_predictions = n,
            qlike_garch = format!("{qlike_garch:.6}"),
            qlike_constant = format!("{qlike_constant:.6}"),
            mean_sigma_hat = format!("{mean_sigma_hat:.6}"),
            mean_sigma_realized = format!("{mean_sigma_realized:.6}"),
            "per_symbol_stats"
        );

        all_stats.push(PerSymbolStats {
            symbol: symbol.to_string(),
            n_predictions: n,
            qlike_garch,
            qlike_constant,
            mean_sigma_hat,
            mean_sigma_realized,
            std_sigma_hat,
            std_sigma_realized,
        });
    }

    // ── Compute aggregate statistics ─────────────────────────────────────────
    let agg = compute_aggregate(&all_stats);

    info!(
        qlike_garch_mean = format!("{:.6}", agg.qlike_garch_mean),
        qlike_constant_mean = format!("{:.6}", agg.qlike_constant_mean),
        qlike_dispersion = format!("{:.6}", agg.qlike_dispersion),
        mean_calibration_ratio = format!("{:.6}", agg.mean_calibration_ratio),
        n_symbols_improving = agg.n_symbols_improving,
        "aggregate_stats"
    );

    // ── Run V-verdict algorithm ──────────────────────────────────────────────
    let verdict = classify_verdict(&agg, &all_stats);
    info!(
        verdict = verdict.label(),
        evidence = verdict.evidence(),
        follow_on = verdict.follow_on(),
        "V-verdict"
    );

    // ── Render report ─────────────────────────────────────────────────────────
    let wall_clock_s = t0.elapsed().as_secs_f64();
    let generated = now_iso8601();

    let report = render_report(
        &all_stats,
        &agg,
        &verdict,
        &checkpoint,
        &checkpoint_revision,
        &generated,
        wall_clock_s,
        n_predictions_total,
        &data_revision_sha,
    );

    let body_sha = body_sha256(&report);

    // ── Write report ──────────────────────────────────────────────────────────
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("create out_dir {}", args.out_dir.display()))?;

    let date_tag = today_yyyymmdd();
    let out_filename = format!(
        "vol-verdict-{}-realdata-{}.md",
        args.scenario.label(),
        date_tag
    );
    let out_path = args.out_dir.join(&out_filename);

    std::fs::write(&out_path, &report)
        .with_context(|| format!("write report {}", out_path.display()))?;

    println!("wrote {} (body-SHA256 = {})", out_path.display(), body_sha);

    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_stats(
        symbol: &str,
        qlike_garch: f64,
        qlike_constant: f64,
        mean_sh: f64,
        mean_sr: f64,
        std_sh: f64,
        std_sr: f64,
    ) -> PerSymbolStats {
        PerSymbolStats {
            symbol: symbol.to_string(),
            n_predictions: 100,
            qlike_garch,
            qlike_constant,
            mean_sigma_hat: mean_sh,
            mean_sigma_realized: mean_sr,
            std_sigma_hat: std_sh,
            std_sigma_realized: std_sr,
        }
    }

    /// Build a set of 10 per-symbol stats all healthy.
    fn healthy_stats() -> Vec<PerSymbolStats> {
        (0..10)
            .map(|i| {
                let base = 0.001 * (i as f64 + 1.0);
                make_stats(
                    &format!("SYM{i}USDT"),
                    0.10 + base,               // qlike_garch
                    0.20 + base,               // qlike_constant (10% improvement)
                    0.01 + 0.001 * i as f64,   // mean_sigma_hat
                    0.011 + 0.001 * i as f64,  // mean_sigma_realized
                    0.002 + 0.0001 * i as f64, // std_sigma_hat (high CoV)
                    0.003,
                )
            })
            .collect()
    }

    #[test]
    fn qlike_single_non_negative() {
        // QLIKE is non-negative.
        for &(sh, sr) in &[(0.01, 0.01), (0.02, 0.01), (0.01, 0.02)] {
            let q = qlike_single(sh, sr);
            assert!(q >= 0.0, "qlike negative for sh={sh}, sr={sr}: {q}");
        }
    }

    #[test]
    fn qlike_single_zero_at_identity() {
        // QLIKE(σ, σ) = 0.
        let q = qlike_single(0.01, 0.01);
        assert!(q.abs() < 1e-10, "qlike(σ,σ) should be 0, got {q}");
    }

    #[test]
    fn verdict_v5_on_healthy_stats() {
        let per_symbol = healthy_stats();
        let agg = compute_aggregate(&per_symbol);
        let v = classify_verdict(&agg, &per_symbol);
        assert!(
            matches!(v, Verdict::V5 { .. }),
            "healthy stats should yield V5, got {:?}",
            v.label()
        );
    }

    #[test]
    fn verdict_v1_on_constant_sigma() {
        // All symbols have zero std_sigma_hat → CoV = 0 < 1e-3 → V1.
        let per_symbol: Vec<PerSymbolStats> = (0..10)
            .map(|i| {
                make_stats(
                    &format!("SYM{i}"),
                    0.10,  // qlike_garch
                    0.20,  // qlike_constant
                    0.01,  // mean_sigma_hat
                    0.011, // mean_sigma_realized
                    0.0,   // std_sigma_hat = 0 → CoV = 0
                    0.003,
                )
            })
            .collect();
        let agg = compute_aggregate(&per_symbol);
        let v = classify_verdict(&agg, &per_symbol);
        assert!(
            matches!(v, Verdict::V1 { .. }),
            "expected V1, got {}",
            v.label()
        );
    }

    #[test]
    fn verdict_v2_on_high_dispersion() {
        // Make QLIKE_max / QLIKE_min > 3 by setting one symbol's qlike very high.
        let mut per_symbol = healthy_stats();
        per_symbol[0].qlike_garch = 1.0;
        per_symbol[1].qlike_garch = 0.10;
        let agg = compute_aggregate(&per_symbol);
        assert!(agg.qlike_dispersion > 3.0, "dispersion should be > 3");
        let v = classify_verdict(&agg, &per_symbol);
        assert!(
            matches!(v, Verdict::V2 { .. }),
            "expected V2, got {}",
            v.label()
        );
    }

    #[test]
    fn verdict_v3_on_calibration_drift() {
        // mean_calibration_ratio = mean_sh / mean_sr; set mean_sh >> mean_sr.
        let per_symbol: Vec<PerSymbolStats> = (0..10)
            .map(|_| {
                make_stats(
                    "SYMXUSDT", 0.10, 0.20,
                    0.050, // mean_sigma_hat >> mean_sigma_realized → ratio > 1.4
                    0.010, 0.002, 0.003,
                )
            })
            .collect();
        let agg = compute_aggregate(&per_symbol);
        assert!(
            agg.mean_calibration_ratio > 1.4,
            "calibration_ratio should > 1.4"
        );
        let v = classify_verdict(&agg, &per_symbol);
        assert!(
            matches!(v, Verdict::V3 { .. }),
            "expected V3, got {}",
            v.label()
        );
    }

    #[test]
    fn verdict_v4_on_no_improvement() {
        // All symbols have qlike_garch == qlike_constant → 0% improvement → V4.
        let per_symbol: Vec<PerSymbolStats> = (0..10)
            .map(|i| {
                make_stats(
                    &format!("SYM{i}"),
                    0.20, // qlike_garch == qlike_constant → 0% improvement
                    0.20,
                    0.01,
                    0.010,
                    0.002,
                    0.003,
                )
            })
            .collect();
        let agg = compute_aggregate(&per_symbol);
        let v = classify_verdict(&agg, &per_symbol);
        assert!(
            matches!(v, Verdict::V4 { .. }),
            "expected V4, got {}",
            v.label()
        );
    }

    #[test]
    fn verdict_exactly_one_fires() {
        // Property: exactly one verdict fires for all test fixtures.
        let fixtures: &[&dyn Fn() -> Vec<PerSymbolStats>] = &[&healthy_stats];
        for f in fixtures {
            let per_symbol = f();
            let agg = compute_aggregate(&per_symbol);
            let v = classify_verdict(&agg, &per_symbol);
            let label = v.label();
            // label is exactly one of V1-V5.
            assert!(["V1", "V2", "V3", "V4", "V5"].contains(&label));
        }
    }
}
