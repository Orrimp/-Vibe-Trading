//! Backtest engine: `MatchingEngine` trait, `PaperEngine`, backtest loop.
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
#![warn(clippy::pedantic)]

pub mod bakeoff;
pub mod engine;
pub mod paper;

/// Cancellation primitives for the bar-loop poll boundary (lab-end-to-end-v2 T-AR-5 / Wave D-3).
pub mod cancel;

/// Progress primitives for the bar-loop poll boundary (lab-end-to-end-v2 T-AR-5 / Wave D-4).
pub mod progress;

/// Bug #56 — workspace-relative path resolver so cross-sectional scenarios
/// (`momentum`, `pairs`, `tcn_overlay`, etc.) can load their
/// `config/strategies/*.toml` from any CWD, not just workspace root.
pub mod paths;

/// Shared metric calculators and the Monte-Carlo distribution-summary reducer.
///
/// M-DEV-1: `compute_sharpe_hourly` / `compute_sortino_hourly` /
/// `compute_calmar` / `compute_max_drawdown_f64` / `compute_total_return`
/// lifted verbatim from `bin/threshold_sweep.rs` (R-NR.5 behaviour-preserving).
///
/// M-DEV-2: `DistributionSummary` reducer implementing ADR-0051 D2 frozen
/// reduction order (index-order mean / two-pass std / `total_cmp` sort /
/// type-7 linear percentile / NaN-absent assertion).
///
/// M-DEV-1 (horizon-retest-robustness): `compute_sharpe_periodic` /
/// `compute_sortino_periodic` / `compute_calmar_periodic` — horizon-aware
/// annualization siblings (pure additions; the 1h fns are byte-verbatim).
pub mod stats;

/// Monte-Carlo ensemble fan-out seam (review 1-14): `GeneratorKind`, the
/// ADR-0051 D1 seed derivation, `run_one_path` + `run_ensemble` — extracted
/// VERBATIM from `bin/monte_carlo.rs` so the R-NR.6(a)/(b) + FP-C2.1 e2e
/// gates exercise the REAL harness chain. Internal seam (like `scenarios`),
/// not a stable public API surface.
pub mod mc_harness;

/// OHLCV bar resampler and `Horizon` enum (horizon-retest-robustness M-DEV-2).
///
/// `resample_ohlcv(bars_1h, horizon)` folds 1h bars into coarser (4h/daily)
/// bars. `Horizon::OneHour` → identity pass-through (the existing 91 anchors
/// are byte-untouched by construction).
pub mod resample;

/// Shared CLI-scenario types used by `main.rs` and the extracted modules.
/// Made `pub` so that the backtest binary can access them.
pub mod cli_types;
/// Phase B extracted report writers (ADR-0035 / T-D-N1).
pub mod report;
/// Phase B extracted scenario execution bodies (ADR-0035 / T-D-N1).
/// Made `pub` so that the backtest binary (`main.rs`) can access these
/// modules. These are internal to the backtest crate — not part of the
/// stable public API surface.
pub mod scenarios;

#[cfg(feature = "realdata")]
pub mod realdata;

/// Pure, sync, deterministic single-coin short-execution helper (ADR-0068 D6).
///
/// Both `run_scenario` / `sma_composed_run` AND the agent forward loop call this
/// helper — consistent-by-construction parity (Q-SS-6 / F5b discipline).
pub mod short_exec;

/// Carry-funding parquet loader and as-of forward-fill (M-DEV-1 + M-DEV-2).
/// Compiled only when `--features realdata` (funding data requires real parquets).
#[cfg(feature = "realdata")]
pub mod funding_data;

/// Basis parquet loader and as-of join (perp-basis-signal-robustness M-DEV-1 + M-DEV-2).
/// Mirrors `funding_data` for `data/binance-basis/` (basis = `(markPrice − indexPrice)/indexPrice`).
/// Compiled only when `--features realdata` (basis data requires real parquets).
#[cfg(feature = "realdata")]
pub mod basis_data;

/// Deribit DVOL implied-vol parquet loader and as-of join (ADR-0072 / advisor-options-impliedvol-probe).
/// Mirrors `basis_data` for `data/deribit-dvol/` (DVOL = daily implied-vol index, BTC+ETH only).
/// Compiled only when `--features realdata` (DVOL data requires real parquets).
#[cfg(feature = "realdata")]
pub mod dvol_data;

/// Macro-regime exogenous-series loader (ADR-0073 / advisor-crossasset-macro-regime).
///
/// `load_macro_regime_series(yahoo_root, range)` reads the 3 pre-registered
/// macro daily series (`^GSPC` / `DX-Y.NYB` / `^TNX`) from the dedicated
/// `data/yahoo-macro/` corpus and reduces them to a `PitSeries<bool>` —
/// the daily risk-ON/risk-OFF regime flag for the `v0.macro_riskon` arm.
///
/// Compiled only when `--features yahoo` (which enables the Yahoo parquet reader).
#[cfg(feature = "yahoo")]
pub mod macro_regime;

pub use engine::run_scenario;
pub use engine::{
    BacktestKpis, DateRange, MatchingEngine, ParamSheet, RunError, RunReport, ScenarioConfig,
};
pub use paper::PaperEngine;

// Bake-off + ranking public surface (advisor feature F1+F2, ADR-0059).
pub use bakeoff::robustness::{ParamRobustnessVerdict, RobustnessFlag};
pub use bakeoff::{
    BakeoffConfig, BakeoffReport, BakeoffRequest, CandidateKpis, CandidateResult, Ranking,
    ReasonCode, Recommendation, RecommendationOutcome, RobustnessMode, TailSummary,
    compute_robustness_distribution, rank_candidates, run_bakeoff,
};
pub use stats::DistributionSummary;

// Candidate-level bake-off progress (Task 1 — ui mirrors this type directly).
pub use progress::{BakeoffProgress, BakeoffProgressSender, bakeoff_progress_pair};

// Gate-tied hyperparameter sweep (ADR-0069, T1–T5).
pub use bakeoff::sweep::{
    BollingerGrid, MAX_SWEEP_CONFIGS, MacdGrid, RsiGrid, SmaGrid, SweepAxis, SweepCellResult,
    SweepConfig, SweepFamily, SweepGrid, SweepProgressSender, SweepReport, SweepRequestEcho,
    SweptParams, run_param_sweep,
};

// ADR-0070 D2 — shared composed-TOML generators (promotion fidelity).
// The agent crate builds the same in-memory TOML the sweep used to score a
// cell, so what paper-trades == what the gate scored (one source of truth).
pub use bakeoff::sweep::{bbands_toml, macd_toml, rsi_toml};

/// Annualised Sharpe ratio from a minute-resolution equity curve.
///
/// Re-exported from `scenarios::sma_composed` per ADR-0035 § Decision 8 (T-D-N12).
/// Single source of truth — `main.rs` calls `backtest::compute_sharpe` so there
/// is no duplication. Signature locked: `pub fn compute_sharpe(equity_curve: &[Decimal]) -> f64`.
pub use scenarios::sma_composed::compute_sharpe;

/// Compute the deterministic-content SHA-256 of a backtest report.
///
/// # Determinism convention
///
/// The YAML front matter of every backtest report contains a `generated:` field
/// with a wall-clock timestamp.  That field is intentionally excluded from the
/// hash so that two runs of the same scenario at the same seed produce
/// byte-identical hashes even though they were run at different wall-clock times.
///
/// Everything from the first line **after** the closing `---` of the front
/// matter is included in the hash.  The front matter spans from the first `---`
/// line (inclusive) to the second `---` line (inclusive); only the body that
/// follows is hashed.
///
/// This function is the single source of truth for the hashing convention; both
/// the report writer and the T33 determinism test call it so that the comparison
/// is always apples-to-apples.
#[must_use]
pub fn report_body_hash(report: &str) -> Vec<u8> {
    use sha2::{Digest, Sha256};

    // Skip the YAML front matter (everything up to and including the closing
    // `---` delimiter).  The front matter starts at the first `---` line and
    // ends at the next `---` line.
    let body = extract_report_body(report);

    let mut hasher = Sha256::new();
    hasher.update(body.as_bytes());
    hasher.finalize().to_vec()
}

/// Extract the report body (everything after the YAML front-matter block).
///
/// The front matter is the region from the first `---` line up to and including
/// the second `---` line.  Everything that follows is the "body".  If no valid
/// front matter is found the entire string is returned as-is so the hash still
/// works on hand-crafted strings.
#[must_use]
pub fn extract_report_body(report: &str) -> &str {
    let mut dash_count = 0usize;
    let mut body_start = 0usize;

    for line in report.split_inclusive('\n') {
        body_start += line.len();
        if line.trim_end() == "---" {
            dash_count += 1;
            if dash_count == 2 {
                // body_start now points just past the closing `---` line
                break;
            }
        }
    }

    if dash_count < 2 {
        // No valid front-matter delimiter found — hash the whole thing
        return report;
    }

    &report[body_start..]
}
