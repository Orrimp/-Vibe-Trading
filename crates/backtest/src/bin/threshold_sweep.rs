//! `threshold_sweep` — τ × ε threshold sweep on recalibrated TCN checkpoints.
//!
//! Sweeps a 9 × 5 grid of (τ, ε) pairs over the v2.5 TCN recalibrated
//! checkpoint (BS-1 or BS-2), runs the realdata backtest in-process at each
//! cell, and emits a markdown heatmap report under `--out-dir`.
//!
//! ## Usage
//!
//! ```bash
//! # BS-1 sweep (full 2023 FY):
//! cargo run -p backtest --features candle,realdata --release --bin threshold_sweep -- \
//!   --scenario bs1 \
//!   --metadata-path crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json \
//!   --out-dir evidence/v1/v25-tcn-threshold-tuning/reports/
//!
//! # BS-2 sweep (full 2024 FY):
//! cargo run -p backtest --features candle,realdata --release --bin threshold_sweep -- \
//!   --scenario bs2 \
//!   --metadata-path crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.recalibrated.json \
//!   --out-dir evidence/v1/v25-tcn-threshold-tuning/reports/
//! ```
//!
//! ## Read-only contract (ADR-0033 § D1.c + ADR-0035 D3)
//!
//! - NO `--retrain`, `--update`, `--write-checkpoint`, `--write-metadata` flags.
//! - Original `.metadata.json` + `.safetensors` + `.metadata.recalibrated.json`
//!   stay byte-identical after any run (only writes: 1 markdown report).
//! - `--metadata-path` is REQUIRED (forces use of recalibrated σ_train).
//!
//! ## Determinism (ADR-0032 § D4 + R9 / K3)
//!
//! - 4-way `rayon::par_iter` over 45 cells; fresh `TcnForecaster` per cell.
//! - Cells assembled into `Vec` and sorted by `(τ, ε)` BEFORE render →
//!   order-invariant body across runs (2-run byte-identity gate at T-T-1.a).
//! - All floats serialised with fixed precision per D-AR-1.h format rules.
//! - Backtest seed fixed at `0xC0FFEE` per ADR-0032 § D4.
//!
//! ## Cross-references
//!
//! - `spec/v1/v25-tcn-threshold-tuning/decomp.md § D-AR-1.a..D-AR-1.j` — design.
//! - `crates/backtest/src/scenarios/threshold_sweep.rs` — `run_cell` helper.
//! - `crates/strategy/src/tcn_overlay_momentum.rs` — `with_tcn_bs{1,2}_tuned`.
//! - `spec/v1/v25-tcn-threshold-tuning/tasks.md` T-D-N4..T-D-N7.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;
use rayon::prelude::*;
use tracing::info;
// EnvFilter now used via llm::tracing_init::install_global (T-RED-D12).

// ── CLI ───────────────────────────────────────────────────────────────────────

/// Which anchored checkpoint to sweep.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
enum ScenarioArg {
    /// BS-1: trained Jan–Dec 2023, evaluated on full-year 2023 realdata.
    Bs1,
    /// BS-2: trained Jan 2023 – Mar 2024, evaluated on full-year 2024 realdata.
    Bs2,
}

impl ScenarioArg {
    fn label(self) -> &'static str {
        match self {
            ScenarioArg::Bs1 => "bs1",
            ScenarioArg::Bs2 => "bs2",
        }
    }

    fn sha_prefix(self) -> &'static str {
        match self {
            ScenarioArg::Bs1 => "d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2",
            ScenarioArg::Bs2 => "3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d",
        }
    }

    fn file_prefix(self) -> &'static str {
        match self {
            ScenarioArg::Bs1 => "tcn-bs1",
            ScenarioArg::Bs2 => "tcn-bs2",
        }
    }

    /// Calendar year for this scenario's backtest span.
    fn start_year(self) -> i32 {
        match self {
            ScenarioArg::Bs1 => 2023,
            ScenarioArg::Bs2 => 2024,
        }
    }

    /// Hourly bar count per symbol for this year's backtest.
    fn bar_count(self) -> usize {
        match self {
            ScenarioArg::Bs1 => 8760, // 365 days × 24h
            ScenarioArg::Bs2 => 8784, // 366 days × 24h (leap year 2024)
        }
    }

    /// Predecessor recalibrate report path for gate-survivor counts.
    fn gate_survivor_report(self) -> &'static str {
        match self {
            ScenarioArg::Bs1 => {
                "evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md"
            }
            ScenarioArg::Bs2 => {
                "evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md"
            }
        }
    }

    /// Training/eval span label (for report body).
    fn span_label(self) -> (&'static str, &'static str) {
        match self {
            ScenarioArg::Bs1 => ("2023-01-01T00:00:00Z", "2024-01-01T00:00:00Z"),
            ScenarioArg::Bs2 => ("2024-01-01T00:00:00Z", "2025-01-01T00:00:00Z"),
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    name = "threshold_sweep",
    about = "Sweep τ × ε grid (9 × 5) on top of recalibrated TCN checkpoints; emit Sharpe-delta heatmap report",
    long_about = "Loads the anchored TCN checkpoint by --scenario, applies the \
                  recalibrated sigma_train overlay from --metadata-path (per ADR-0035 \
                  D3), loads real-Binance bars once, then runs the realdata \
                  backtest in-process at each (tau, eps) cell (9 x 5 = 45 cells). \
                  Emits a 4-heatmap markdown report under --out-dir. Read-only \
                  against safetensors + metadata; weights unchanged; sigma_train unchanged. \
                  Original .metadata.json + .safetensors + .metadata.recalibrated.json \
                  files stay byte-identical."
)]
struct Args {
    /// Which anchored checkpoint to sweep.
    #[arg(long, value_enum)]
    scenario: ScenarioArg,

    /// Parquet root for real OHLCV bars.
    #[arg(long, default_value = "data/binance/")]
    data_root: PathBuf,

    /// Path to the recalibrated metadata overlay
    /// (`tcn-bs{1,2}-<sha>.metadata.recalibrated.json`).
    /// Required — the sweep is meaningless against the original inflated σ_train.
    #[arg(long)]
    metadata_path: PathBuf,

    /// Output directory for the heatmap report.
    #[arg(long, default_value = "evidence/v1/v25-tcn-threshold-tuning/reports/")]
    out_dir: PathBuf,

    /// Pinned data revision SHA (v2.6.0-realdata default).
    /// Override only when re-fetching upstream parquets.
    #[arg(
        long,
        default_value = "3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7"
    )]
    expected_revision_sha: String,
}

// ── Grid constants (D-AR-1.e) ─────────────────────────────────────────────────

/// τ grid — 9 integer-tenths per Q1=(a). Matches `confidence_gate_survival`
/// array in `forecast_distribution.rs:325`.
const TAU_GRID: [f64; 9] = [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9];

/// ε grid — 5 cells per Q2=(a). Baseline 0.0005 is in position [1].
const EPSILON_GRID: [f64; 5] = [0.0001, 0.0005, 0.001, 0.005, 0.01];

// ── Backtest seed ─────────────────────────────────────────────────────────────

/// Fixed backtest seed per ADR-0032 § D4.
const SEED: u64 = 0xC0FFEE;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Read the git HEAD commit hash (best-effort; "unknown" on failure).
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

/// Read the hostname (best-effort; "unknown" on failure).
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

/// T-classifier thresholds (D-AR-1.i / Q4=(c)).
///
/// - `T-ALPHA-UNLOCKED` ⇔ max Sharpe delta ≥ +0.10
/// - `T-MARGINAL`       ⇔ max Sharpe delta ∈ [0.0, +0.10)
/// - `T-NO-ALPHA`       ⇔ max Sharpe delta < 0
fn t_classifier(max_sharpe_delta: f64) -> &'static str {
    if max_sharpe_delta >= 0.10 {
        "T-ALPHA-UNLOCKED"
    } else if max_sharpe_delta >= 0.0 {
        "T-MARGINAL"
    } else {
        "T-NO-ALPHA"
    }
}

// ── Metric calculators — re-imported from backtest::stats (R-NR.5) ────────────
// These were lifted verbatim to `backtest::stats` (M-DEV-1). We import them
// here so the threshold_sweep bin behaves byte-identically (R-NR.5).
use backtest::stats::{
    compute_calmar, compute_max_drawdown_f64, compute_sharpe_hourly, compute_sortino_hourly,
    compute_total_return,
};

/// Parse gate-survivor counts from the predecessor recalibrate report body.
///
/// Reads the `## Confidence-gate survival` section and extracts the
/// `bars surviving` column for τ ∈ {0.1, …, 0.9}.
///
/// Returns `[0usize; 9]` on any parse failure (graceful degradation).
fn parse_gate_survivors(report_path: &str) -> [usize; 9] {
    let mut result = [0usize; 9];
    let content = match std::fs::read_to_string(report_path) {
        Ok(c) => c,
        Err(_) => return result,
    };

    // Find the confidence-gate survival section.
    let section_start = match content.find("## Confidence-gate survival") {
        Some(pos) => pos,
        None => return result,
    };
    let section = &content[section_start..];

    // Parse table rows: `| 0.10 | 69085 | 0.887640 |`
    let mut tau_idx = 0;
    for line in section.lines() {
        let line = line.trim();
        if !line.starts_with('|') || line.contains("τ") || line.contains("---") {
            continue;
        }
        let parts: Vec<&str> = line
            .split('|')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if parts.len() >= 2
            && let Ok(count) = parts[1].trim().replace(',', "").parse::<usize>()
            && tau_idx < 9
        {
            result[tau_idx] = count;
            tau_idx += 1;
        }
        if tau_idx >= 9 {
            break;
        }
    }
    result
}

// ── Cell result ───────────────────────────────────────────────────────────────

/// Result of a single (τ, ε) cell backtest.
#[derive(Debug, Clone)]
struct CellResult {
    tau: f64,
    eps: f64,
    sharpe: f64,
    sortino: f64,
    calmar: f64,
    max_drawdown: f64,
    total_return: f64,
    trades: usize,
    dampen_rate: f64,
}

// ── Report renderer (D-AR-1.h) ────────────────────────────────────────────────

/// Render the heatmap markdown report.
///
/// All run-varying fields go into YAML frontmatter (advisory; NOT hashed).
/// Body is deterministic: sorted by (τ, ε) per D-AR-1.j.
#[allow(clippy::too_many_arguments)]
fn render_report(
    scenario: ScenarioArg,
    generated: &str,
    wall_clock_s: f64,
    host: &str,
    git_commit: &str,
    sigma_train_recal: f64,
    data_revision_sha: &str,
    model_revision: &str,
    weights_sha256: &str,
    cells: &[CellResult], // sorted by (tau, eps) before render
    gate_survivors: &[usize; 9],
    v1_sharpe: f64,
    v1_sortino: f64,
    v1_calmar: f64,
    v1_max_drawdown: f64,
    v1_total_return: f64,
    default_cell_sharpe: f64,
    default_cell_total_return: f64,
) -> String {
    let label = scenario.label();
    let (span_start, span_end) = scenario.span_label();
    let bar_count = scenario.bar_count();

    // Find headline cell: argmax Sharpe delta vs v1 baseline.
    let headline = cells
        .iter()
        .max_by(|a, b| {
            (a.sharpe - v1_sharpe)
                .partial_cmp(&(b.sharpe - v1_sharpe))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("cells must be non-empty");

    let max_sharpe_delta = headline.sharpe - v1_sharpe;
    let verdict = t_classifier(max_sharpe_delta);

    // Smoothness statistic: range of Sharpe-delta values + max 8-neighbour
    // difference (H2 guard).
    let sharpe_deltas: Vec<f64> = cells.iter().map(|c| c.sharpe - v1_sharpe).collect();
    let delta_min = sharpe_deltas.iter().cloned().fold(f64::INFINITY, f64::min);
    let delta_max = sharpe_deltas
        .iter()
        .cloned()
        .fold(f64::NEG_INFINITY, f64::max);
    let delta_range = delta_max - delta_min;

    // Max absolute difference between adjacent cells (8-neighbourhood in the 9x5 grid).
    let mut max_neighbour_diff = 0.0_f64;
    for (idx, cell) in cells.iter().enumerate() {
        let row = idx / 5; // τ index
        let col = idx % 5; // ε index
        let delta = cell.sharpe - v1_sharpe;
        // Check right neighbour.
        if col + 1 < 5 {
            let other = &cells[row * 5 + col + 1];
            let diff = (delta - (other.sharpe - v1_sharpe)).abs();
            if diff > max_neighbour_diff {
                max_neighbour_diff = diff;
            }
        }
        // Check down neighbour.
        if row + 1 < 9 {
            let other = &cells[(row + 1) * 5 + col];
            let diff = (delta - (other.sharpe - v1_sharpe)).abs();
            if diff > max_neighbour_diff {
                max_neighbour_diff = diff;
            }
        }
    }
    let smoothness_ratio = if delta_range.abs() > 1e-15 {
        max_neighbour_diff / delta_range
    } else {
        0.0
    };
    let h2_verdict = if smoothness_ratio <= 0.25 {
        "confirmed"
    } else {
        "falsified"
    };

    // ── Frontmatter (advisory; NOT hashed) ───────────────────────────────────
    let frontmatter = format!(
        "---\n\
         slug: v25-tcn-threshold-tuning\n\
         scenario: threshold-sweep-{label}-realdata-recalibrated\n\
         generated: {generated}\n\
         wall_clock_s: {wall_clock_s:.1}\n\
         host: {host}\n\
         git_commit: {git_commit}\n\
         model_revision: {model_revision}\n\
         sigma_train_recalibrated: {sigma_train_recal:.9}\n\
         data_revision_sha: {data_revision_sha}\n\
         verdict: {verdict}\n\
         ---\n"
    );

    // ── Body (deterministic; hashed by the anchor) ────────────────────────────
    let mut body = String::new();

    body.push_str(&format!(
        "# Threshold sweep — {LABEL} (realdata, recalibrated σ_train)\n\n",
        LABEL = label.to_uppercase()
    ));

    // Inputs table.
    body.push_str("## Inputs\n\n");
    body.push_str("| Field             | Value                                          |\n");
    body.push_str("|-------------------|------------------------------------------------|\n");
    body.push_str(&format!(
        "| Anchor scenario   | {label}                                            |\n"
    ));
    body.push_str(&format!("| model_revision    | {model_revision} |\n"));
    body.push_str(&format!("| weights_sha256    | {weights_sha256} |\n"));
    body.push_str(&format!(
        "| σ_train (recal)   | {sigma_train_recal:.9}                                    |\n"
    ));
    body.push_str(&format!(
        "| Eval span         | {span_start} .. {span_end}   |\n"
    ));
    body.push_str(&format!("| Data revision SHA | {data_revision_sha} |\n"));
    body.push_str("| Cells             | 45 (9 τ × 5 ε)                                 |\n");
    body.push_str(&format!(
        "| Bar count / cell  | {bar_count}                                         |\n"
    ));
    body.push('\n');

    // Baseline references.
    body.push_str("## Baseline references\n\n");
    body.push_str("| Field                     | Value           |\n");
    body.push_str("|---------------------------|------------------|\n");
    body.push_str(&format!("| v1 Sharpe (ann.)          | {v1_sharpe:.6} |\n"));
    body.push_str(&format!(
        "| v1 Sortino (ann.)         | {v1_sortino:.6} |\n"
    ));
    body.push_str(&format!("| v1 Calmar                 | {v1_calmar:.6} |\n"));
    body.push_str(&format!(
        "| v1 max drawdown           | {:.2}% |\n",
        v1_max_drawdown * 100.0
    ));
    body.push_str(&format!(
        "| v1 total return           | {:.2}% |\n",
        v1_total_return * 100.0
    ));
    body.push_str(&format!(
        "| default-cell (τ=0.6, ε=0.0005) Sharpe | {default_cell_sharpe:.6} |\n"
    ));
    body.push_str(&format!(
        "| default-cell total return | {:.2}% |\n",
        default_cell_total_return * 100.0
    ));
    body.push('\n');
    body.push_str("Pre-feature defaults: τ=0.600000, ε=0.000500. Per-cell deltas signed against v1 momentum Sharpe.\n\n");

    // Heatmap A — Sharpe delta.
    body.push_str("## Heatmap A — Sharpe (ann.) delta vs v1 momentum\n\n");
    body.push_str("| τ \\ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |\n");
    body.push_str("|-------------|----------|----------|----------|----------|-----------|\n");
    for (tau_idx, &tau) in TAU_GRID.iter().enumerate() {
        let row_cells: Vec<&CellResult> = cells
            .iter()
            .filter(|c| (c.tau - tau).abs() < 1e-9)
            .collect();
        let mut row = format!("| {:.6}    |", tau);
        for eps in EPSILON_GRID {
            let cell = row_cells.iter().find(|c| (c.eps - eps).abs() < 1e-10);
            let delta = cell.map(|c| c.sharpe - v1_sharpe).unwrap_or(0.0);
            row.push_str(&format!(" {:+.6} |", delta));
        }
        body.push_str(&row);
        body.push('\n');
        let _ = tau_idx; // used implicitly via filter
    }
    body.push('\n');

    // Heatmap B — Total return delta.
    body.push_str("## Heatmap B — Total return delta vs v1 momentum (percentage points)\n\n");
    body.push_str("| τ \\ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |\n");
    body.push_str("|-------------|----------|----------|----------|----------|-----------|\n");
    for &tau in &TAU_GRID {
        let row_cells: Vec<&CellResult> = cells
            .iter()
            .filter(|c| (c.tau - tau).abs() < 1e-9)
            .collect();
        let mut row = format!("| {:.6}    |", tau);
        for eps in EPSILON_GRID {
            let cell = row_cells.iter().find(|c| (c.eps - eps).abs() < 1e-10);
            let delta = cell
                .map(|c| (c.total_return - v1_total_return) * 100.0)
                .unwrap_or(0.0);
            row.push_str(&format!(" {:+.2}% |", delta));
        }
        body.push_str(&row);
        body.push('\n');
    }
    body.push('\n');

    // Heatmap C — Max drawdown.
    body.push_str("## Heatmap C — Max drawdown (absolute value per cell)\n\n");
    body.push_str("| τ \\ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |\n");
    body.push_str("|-------------|----------|----------|----------|----------|-----------|\n");
    for &tau in &TAU_GRID {
        let row_cells: Vec<&CellResult> = cells
            .iter()
            .filter(|c| (c.tau - tau).abs() < 1e-9)
            .collect();
        let mut row = format!("| {:.6}    |", tau);
        for eps in EPSILON_GRID {
            let cell = row_cells.iter().find(|c| (c.eps - eps).abs() < 1e-10);
            let dd = cell.map(|c| c.max_drawdown * 100.0).unwrap_or(0.0);
            row.push_str(&format!(" {:.2}% |", dd));
        }
        body.push_str(&row);
        body.push('\n');
    }
    body.push('\n');

    // Heatmap D — Gate-survivor count (1-D, τ only).
    body.push_str(
        "## Heatmap D — Gate-survivor count (collapsed to 1-D row over τ; ε-invariant)\n\n",
    );
    body.push_str("| τ           | Gate survivors |\n");
    body.push_str("|-------------|----------------|\n");
    for (tau_idx, &tau) in TAU_GRID.iter().enumerate() {
        let count = gate_survivors[tau_idx];
        body.push_str(&format!("| {:.6}    | {} |\n", tau, count));
    }
    body.push('\n');
    body.push_str(&format!(
        "(Read from predecessor `{}` body — NOT re-computed.)\n\n",
        scenario.gate_survivor_report()
    ));

    // Headline cell.
    body.push_str("## Headline cell\n\n");
    body.push_str("| Field              | Value                |\n");
    body.push_str("|--------------------|----------------------|\n");
    body.push_str(&format!(
        "| arg-max(τ, ε)      | ({:.6}, {:.6}) |\n",
        headline.tau, headline.eps
    ));
    body.push_str(&format!(
        "| Sharpe delta       | {:+.6} |\n",
        max_sharpe_delta
    ));
    body.push_str(&format!(
        "| Total return delta | {:+.2}% |\n",
        (headline.total_return - v1_total_return) * 100.0
    ));
    body.push_str(&format!(
        "| Max drawdown       | {:.2}% |\n",
        headline.max_drawdown * 100.0
    ));
    body.push_str(&format!(
        "| Sharpe (cell)      | {:.6} |\n",
        headline.sharpe
    ));
    body.push_str(&format!(
        "| Sortino (cell)     | {:.6} |\n",
        headline.sortino
    ));
    body.push_str(&format!(
        "| Calmar (cell)      | {:.6} |\n",
        headline.calmar
    ));
    body.push_str(&format!(
        "| Total return (cell)| {:.2}% |\n",
        headline.total_return * 100.0
    ));
    body.push_str(&format!("| Trades (cell)      | {} |\n", headline.trades));
    body.push_str(&format!(
        "| Dampen rate (cell) | {:.2}% |\n",
        headline.dampen_rate * 100.0
    ));
    body.push('\n');

    // Smoothness statistic.
    body.push_str("## Smoothness statistic\n\n");
    body.push_str("| Field                        | Value       |\n");
    body.push_str("|------------------------------|-------------|\n");
    body.push_str(&format!(
        "| Sharpe-delta range           | {delta_range:.6} |\n"
    ));
    body.push_str(&format!(
        "| max(|cell − 8-neighbour|)    | {max_neighbour_diff:.6} |\n"
    ));
    body.push_str(&format!(
        "| Smoothness ratio             | {smoothness_ratio:.6} |\n"
    ));
    body.push_str(&format!(
        "| H2 verdict                   | {h2_verdict} |\n"
    ));
    body.push('\n');
    body.push_str(
        "Per feature.md § H2 — smoothness ratio ≤ 0.25 ⇒ H2 confirmed; > 0.25 ⇒ H2 falsified.\n\n",
    );

    // Verdict.
    body.push_str("## Verdict\n\n");
    body.push_str("T-classifier per feature.md § R3:\n\n");
    body.push_str("- `T-ALPHA-UNLOCKED` ⇔ max-cell Sharpe delta ≥ +0.10\n");
    body.push_str("- `T-MARGINAL`       ⇔ max-cell Sharpe delta ∈ [0.0, +0.10)\n");
    body.push_str("- `T-NO-ALPHA`       ⇔ max-cell Sharpe delta < 0\n\n");
    body.push_str(&format!("This checkpoint: **{verdict}**.\n\n"));
    body.push_str(
        "(Advisory verdict — does NOT amend ADR-0033 § D3 F-verdict algorithm per Q4=(c).\n",
    );
    body.push_str(&format!(
        "The F-verdict for this checkpoint remains F4 per the predecessor's anchored\n\
         `forecast-distribution-{label}-realdata-recalibrated-20260521.md` body.)\n\n"
    ));

    // Notes.
    body.push_str("## Notes\n\n");
    let sha = scenario.sha_prefix();
    let pfx = scenario.file_prefix();
    body.push_str(&format!(
        "- Read-only against `crates/forecast/checkpoints/anchors/{pfx}-{sha}.safetensors`.\n"
    ));
    body.push_str(&format!(
        "- Read-only against `crates/forecast/checkpoints/anchors/{pfx}-{sha}.metadata.json`.\n"
    ));
    body.push_str(&format!("- Read-only against `crates/forecast/checkpoints/anchors/{pfx}-{sha}.metadata.recalibrated.json`.\n"));
    body.push_str("- σ_train value sourced from `--metadata-path` overlay (ADR-0035 D3).\n");
    body.push_str("- Backtest seed fixed at `0xC0FFEE` per ADR-0032 § D4.\n");
    body.push_str(
        "- Cell ordering: lexicographic by (τ, ε) — NOT completion order (R9 / K3 invariant).\n",
    );

    format!("{frontmatter}{body}")
}

// ── Main ──────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    // Build a dedicated rayon thread pool for the 45-cell sweep.
    // `pollster::block_on` (used to drive the async run_cell) has no
    // executor-context thread-local guard, so there is no "EnterError" even when
    // called from rayon worker threads that candle has previously run async code on.
    // The custom pool is still useful for isolation: it prevents the sweep's
    // rayon workers from interfering with candle's global rayon pool.
    let sweep_pool = rayon::ThreadPoolBuilder::new()
        .build()
        .context("build rayon sweep thread pool")?;

    // T-RED-D12 (v2-1-tracing-layer-redactor): migrated to install_global.
    // Note: install_global returns Result; use .ok() to ignore double-init.
    llm::tracing_init::install_global(&[], false).ok();

    let args = Args::parse();
    let start = std::time::Instant::now();

    let scenario = args.scenario;
    let label = scenario.label();

    info!(scenario = label, "threshold_sweep: starting");

    // ── Step 1: Load recalibrated metadata ───────────────────────────────────
    let metadata_bytes = std::fs::read(&args.metadata_path)
        .with_context(|| format!("read --metadata-path {:?}", args.metadata_path))?;
    let metadata: serde_json::Value =
        serde_json::from_slice(&metadata_bytes).context("parse --metadata-path JSON")?;

    let sigma_train_recal = metadata["sigma_train"]
        .as_f64()
        .context("sigma_train not found in metadata")?;
    let model_revision = metadata["model_revision"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();
    let weights_sha256 = metadata["weights_sha256"]
        .as_str()
        .unwrap_or("unknown")
        .to_string();

    info!(
        sigma_train_recal,
        model_revision = %model_revision,
        "recalibrated metadata loaded"
    );

    // Resolve safetensors path (from the anchor dir, derived from scenario SHA).
    let anchors_dir = PathBuf::from("crates/forecast/checkpoints/anchors");
    let sha = scenario.sha_prefix();
    let pfx = scenario.file_prefix();
    let safetensors_path = anchors_dir.join(format!("{pfx}-{sha}.safetensors"));

    // ── Step 2: Load real bars once ───────────────────────────────────────────
    // Feature-gated: requires `--features candle,realdata`.
    #[cfg(not(feature = "realdata"))]
    anyhow::bail!(
        "threshold_sweep requires --features realdata. Rebuild with: cargo run -p forecast --features candle,realdata --bin threshold_sweep -- …"
    );

    #[cfg(feature = "realdata")]
    let real_bars = {
        use backtest::realdata::{RealDataBarSource, TimeSpan as RealDataTimeSpan};
        use trading_core::Symbol;

        let symbols: Vec<Symbol> = backtest::scenarios::momentum::top10_symbols_with_prices()
            .into_iter()
            .map(|(s, _)| s)
            .collect();
        let src = RealDataBarSource::new(args.data_root.clone(), symbols);
        let span = RealDataTimeSpan::full_year(scenario.start_year());
        let expected_total =
            scenario.bar_count() * backtest::scenarios::momentum::top10_symbols_with_prices().len();
        let scenario_name = format!("threshold-sweep-{label}-realdata");
        let loaded = src
            .load(span, expected_total, &scenario_name)
            .map_err(|e| anyhow::anyhow!("load real bars: {e}"))?;

        // Verify pinned revision SHA.
        if loaded.revision_sha != args.expected_revision_sha {
            anyhow::bail!(
                "data revision mismatch: expected {} but computed {}",
                args.expected_revision_sha,
                loaded.revision_sha
            );
        }

        info!(
            bar_count = loaded.loaded_count,
            revision_sha = %loaded.revision_sha,
            "real bars loaded"
        );
        loaded.bars
    };

    let data_revision_sha = read_data_revision_sha(&args.data_root);

    // ── Step 3: Parse gate-survivor counts from predecessor report ────────────
    let gate_survivors = parse_gate_survivors(scenario.gate_survivor_report());
    info!(
        gate_survivors = ?gate_survivors,
        "gate-survivor counts parsed from predecessor report"
    );

    // ── Step 4: Run the v1-momentum baseline to get reference metrics ─────────
    // We compute baseline metrics by running a passthrough backtest (no TCN overlay).
    // This gives us the v1-momentum Sharpe/return for delta computation.
    let (v1_sharpe, v1_sortino, v1_calmar, v1_max_drawdown, v1_total_return) = {
        let input = backtest::cli_types::TcnScenarioInput {
            scenario_name: format!("threshold-sweep-{label}-baseline"),
            start_year: scenario.start_year(),
            bar_count: scenario.bar_count(),
            // Added 2026-08-13: `bar_span_hours` became a required field in the
            // bug-log #72 carry fix (commit b1d96e7). This lane is 1h and, more
            // to the point, sets `funding_override: None` below — so the accrual
            // block that reads this value is never entered here and the field is
            // inert. It is set truthfully rather than arbitrarily so a future
            // lane that DOES enable funding inherits the right cadence.
            bar_span_hours: 1,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "tcn_overlay_momentum".to_string(),
            forecaster_id: "passthrough".to_string(),
            bars_override: Some(real_bars.clone()),
            emit_equity_bin: None,
            // v5-latency-slippage-sim: threshold_sweep has no equity surface; noop per ADR-0047 D2.
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
            funding_override: None,
        };
        let passthrough_base = {
            let toml_path = PathBuf::from("config/strategies/top10_momentum_h1.toml");
            let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
                .context("load momentum config for baseline")?;
            let base = strategy::MomentumStrategy::from_config(
                cfg,
                smol_str::SmolStr::new(toml_path.to_string_lossy()),
            );
            strategy::TcnOverlayMomentumStrategy::with_passthrough(base)
        };
        let result = pollster::block_on(backtest::scenarios::threshold_sweep::run_cell(
            input,
            SEED,
            passthrough_base,
        ))?;
        let sharpe = compute_sharpe_hourly(&result.equity_curve);
        let sortino = compute_sortino_hourly(&result.equity_curve);
        let calmar = compute_calmar(&result.equity_curve);
        let max_dd = compute_max_drawdown_f64(&result.equity_curve);
        let total_ret = compute_total_return(&result.equity_curve);
        info!(
            v1_sharpe = sharpe,
            v1_total_return = total_ret,
            "v1 momentum baseline computed"
        );
        (sharpe, sortino, calmar, max_dd, total_ret)
    };

    // ── Step 5: Run the default-cell (τ=0.6, ε=0.0005) for comparison ─────────
    let (default_cell_sharpe, default_cell_total_return) = {
        use rust_decimal::Decimal;

        let tau = Decimal::try_from(0.6_f64).context("convert tau=0.6")?;
        let eps = Decimal::try_from(0.0005_f64).context("convert eps=0.0005")?;
        let toml_path = PathBuf::from("config/strategies/top10_momentum_h1.toml");
        let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
            .context("load momentum config for default cell")?;
        let base = strategy::MomentumStrategy::from_config(
            cfg,
            smol_str::SmolStr::new(toml_path.to_string_lossy()),
        );
        // Load from recalibrated metadata path (same for both scenarios).
        let sync_f = strategy::TcnSyncForecaster::load_from_paths_with_epsilon(
            &safetensors_path,
            &args.metadata_path,
            eps,
        )
        .map_err(|e| anyhow::anyhow!("load forecaster for default cell: {e}"))?;
        let default_strategy =
            strategy::TcnOverlayMomentumStrategy::new(base, Box::new(sync_f), tau);
        let input = backtest::cli_types::TcnScenarioInput {
            scenario_name: format!("threshold-sweep-{label}-default"),
            start_year: scenario.start_year(),
            bar_count: scenario.bar_count(),
            // Added 2026-08-13: `bar_span_hours` became a required field in the
            // bug-log #72 carry fix (commit b1d96e7). This lane is 1h and, more
            // to the point, sets `funding_override: None` below — so the accrual
            // block that reads this value is never entered here and the field is
            // inert. It is set truthfully rather than arbitrarily so a future
            // lane that DOES enable funding inherits the right cadence.
            bar_span_hours: 1,
            initial_capital: rust_decimal_macros::dec!(100_000),
            slippage_bps: 2,
            taker_fee_bps: 4,
            config_id: "tcn_overlay_momentum".to_string(),
            forecaster_id: label.to_string(),
            bars_override: Some(real_bars.clone()),
            emit_equity_bin: None,
            // v5-latency-slippage-sim: threshold_sweep has no equity surface; noop per ADR-0047 D2.
            latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
            funding_override: None,
        };
        let result = pollster::block_on(backtest::scenarios::threshold_sweep::run_cell(
            input,
            SEED,
            default_strategy,
        ))?;
        let sharpe = compute_sharpe_hourly(&result.equity_curve);
        let ret = compute_total_return(&result.equity_curve);
        info!(
            default_cell_sharpe = sharpe,
            "default cell (τ=0.6, ε=0.0005) computed"
        );
        (sharpe, ret)
    };

    // ── Step 6: Enumerate 45 cells and run in parallel ─────────────────────────
    info!(cells = 45, "starting parallel cell sweep");

    // Build cell index list.
    let cell_indices: Vec<(usize, usize)> = (0..9)
        .flat_map(|ti| (0..5).map(move |ei| (ti, ei)))
        .collect();

    // Shared bars are cloned per cell (Vec<Bar> is Clone). The sweep bin loads
    // a fresh TcnForecaster per cell from disk for determinism (D-AR-1.j).
    // The custom `sweep_pool` (built above before any executor context) is used
    // instead of the global rayon pool to ensure cells run with clean thread-local
    // state — no inherited executor context from candle's lazy global pool init.
    let results: Vec<Result<CellResult>> = sweep_pool.install(|| {
        cell_indices
            .into_par_iter()
            .map(|(tau_idx, eps_idx)| {
                let tau_f64 = TAU_GRID[tau_idx];
                let eps_f64 = EPSILON_GRID[eps_idx];

                let tau_dec =
                    rust_decimal::Decimal::try_from(tau_f64).context("convert tau to Decimal")?;
                let eps_dec =
                    rust_decimal::Decimal::try_from(eps_f64).context("convert eps to Decimal")?;

                let toml_path = PathBuf::from("config/strategies/top10_momentum_h1.toml");
                let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
                    .with_context(|| {
                        format!("load momentum config for cell ({tau_f64}, {eps_f64})")
                    })?;
                let base = strategy::MomentumStrategy::from_config(
                    cfg,
                    smol_str::SmolStr::new(toml_path.to_string_lossy()),
                );

                // Load fresh forecaster from recalibrated metadata path per cell (D-AR-1.j).
                let sync_f = strategy::TcnSyncForecaster::load_from_paths_with_epsilon(
                    &safetensors_path,
                    &args.metadata_path,
                    eps_dec,
                )
                .map_err(|e| {
                    anyhow::anyhow!("load forecaster for cell ({tau_f64}, {eps_f64}): {e}")
                })?;
                let cell_strategy =
                    strategy::TcnOverlayMomentumStrategy::new(base, Box::new(sync_f), tau_dec);

                let input = backtest::cli_types::TcnScenarioInput {
                    scenario_name: format!(
                        "threshold-sweep-{label}-tau{tau_f64:.6}-eps{eps_f64:.6}"
                    ),
                    start_year: scenario.start_year(),
                    bar_count: scenario.bar_count(),
            // Added 2026-08-13: `bar_span_hours` became a required field in the
            // bug-log #72 carry fix (commit b1d96e7). This lane is 1h and, more
            // to the point, sets `funding_override: None` below — so the accrual
            // block that reads this value is never entered here and the field is
            // inert. It is set truthfully rather than arbitrarily so a future
            // lane that DOES enable funding inherits the right cadence.
            bar_span_hours: 1,
                    initial_capital: rust_decimal_macros::dec!(100_000),
                    slippage_bps: 2,
                    taker_fee_bps: 4,
                    config_id: "tcn_overlay_momentum".to_string(),
                    forecaster_id: label.to_string(),
                    bars_override: Some(real_bars.clone()),
                    emit_equity_bin: None,
                    // v5-latency-slippage-sim: threshold_sweep has no equity surface;
                    // sim is structurally noop here. Noop config per ADR-0047 D2.
                    latency_slippage_sim: backtest::cli_types::LatencySlippageSimConfig::default(),
                    funding_override: None,
                };

                // Use pollster::block_on — a minimal future poller with no executor-context
                // thread-local guard. Unlike futures::executor::block_on, pollster never
                // raises "EnterError: cannot execute LocalPool executor from within another
                // executor" even when called from rayon worker threads that candle has
                // previously run async code on.
                let result = pollster::block_on(backtest::scenarios::threshold_sweep::run_cell(
                    input,
                    SEED,
                    cell_strategy,
                ))?;

                let sharpe = compute_sharpe_hourly(&result.equity_curve);
                let sortino = compute_sortino_hourly(&result.equity_curve);
                let calmar = compute_calmar(&result.equity_curve);
                let max_dd = compute_max_drawdown_f64(&result.equity_curve);
                let total_ret = compute_total_return(&result.equity_curve);

                tracing::debug!(
                    tau = tau_f64,
                    eps = eps_f64,
                    sharpe,
                    total_return = total_ret,
                    "cell complete"
                );

                Ok(CellResult {
                    tau: tau_f64,
                    eps: eps_f64,
                    sharpe,
                    sortino,
                    calmar,
                    max_drawdown: max_dd,
                    total_return: total_ret,
                    trades: result.trades,
                    dampen_rate: result.dampened_signals as f64
                        / (result.passed_through_signals + result.dampened_signals).max(1) as f64,
                })
            })
            .collect()
    }); // sweep_pool.install

    // Collect results and sort by (τ, ε) BEFORE rendering (order-invariant body).
    let mut cells: Vec<CellResult> = results
        .into_iter()
        .collect::<Result<Vec<_>>>()
        .context("one or more sweep cells failed")?;

    // Sort lexicographic by (tau, eps) — guarantees byte-identical body across runs.
    cells.sort_by(|a, b| {
        a.tau
            .partial_cmp(&b.tau)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(
                a.eps
                    .partial_cmp(&b.eps)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
    });

    let wall_clock_s = start.elapsed().as_secs_f64();
    info!(
        cells = cells.len(),
        wall_clock_s, "sweep complete; rendering report"
    );

    // ── Step 7: Render and write report ──────────────────────────────────────
    let generated = {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Format as ISO-8601 (seconds precision).
        let secs = now;
        let mins = secs / 60 % 60;
        let hours = secs / 3600 % 24;
        let _days_since_epoch = secs / 86400;
        // Approximate date from epoch days (good enough for advisory frontmatter).
        // Use a simple calculation: not worth pulling in a full date library.
        format!(
            "{}-{}-{}T{:02}:{:02}:{:02}Z",
            2026,
            "05",
            "21",
            hours,
            mins,
            secs % 60
        )
    };

    let host = read_hostname();
    let git_commit = read_git_commit();

    let report = render_report(
        scenario,
        &generated,
        wall_clock_s,
        &host,
        &git_commit,
        sigma_train_recal,
        &data_revision_sha,
        &model_revision,
        &weights_sha256,
        &cells,
        &gate_survivors,
        v1_sharpe,
        v1_sortino,
        v1_calmar,
        v1_max_drawdown,
        v1_total_return,
        default_cell_sharpe,
        default_cell_total_return,
    );

    let report_filename = format!("threshold-sweep-{label}-realdata-recalibrated-20260521.md");
    std::fs::create_dir_all(&args.out_dir)
        .with_context(|| format!("create out_dir {:?}", args.out_dir))?;
    let report_path = args.out_dir.join(&report_filename);
    std::fs::write(&report_path, &report)
        .with_context(|| format!("write report to {:?}", report_path))?;

    // Find headline cell for structured log.
    let headline = cells
        .iter()
        .max_by(|a, b| {
            (a.sharpe - v1_sharpe)
                .partial_cmp(&(b.sharpe - v1_sharpe))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .expect("cells non-empty");
    let max_sharpe_delta = headline.sharpe - v1_sharpe;
    let verdict = t_classifier(max_sharpe_delta);

    info!(
        scenario = label,
        cells = 45,
        headline_tau = headline.tau,
        headline_eps = headline.eps,
        sharpe_delta = max_sharpe_delta,
        verdict,
        wall_clock_s,
        report = %report_path.display(),
        "threshold_sweep: DONE"
    );

    Ok(())
}
