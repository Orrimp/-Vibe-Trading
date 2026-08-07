//! Monte-Carlo ensemble fan-out seam for `bin/monte_carlo.rs` (M-DEV-6).
//!
//! Behaviour-preserving extraction (review 1-14): `GeneratorKind`, the
//! ADR-0051 D1 per-path seed derivation, `run_one_path`, and the
//! fan-out → sort-by-`j` → reduce chain are moved VERBATIM from
//! `bin/monte_carlo.rs` so the R-NR.6(a)/(b) + FP-C2.1 e2e gates
//! (`tests/montecarlo_e2e.rs`) can exercise the REAL harness chain instead of
//! a synthetic stand-in. The bin now calls this module; the anchored
//! `block-bootstrap-real` lane's arithmetic is untouched.
//!
//! This is an internal test-seam module (like `scenarios`), NOT a stable
//! public API surface.
//!
//! ## Seed derivations owned here
//!
//! - **ADR-0051 D1 path seed**: [`derive_path_seed`] —
//!   `path_seed_j = master.wrapping_add(j · 0x9E37_79B9)`.
//! - **GBM-smoke per-symbol seed**: [`derive_gbm_sym_seed`] — SplitMix64
//!   mixing over `(path_seed, sym_i)`, matching the 1-13 `data::GbmPathGen`
//!   fix. The previous additive derivation
//!   (`path_seed + sym_i · 0x9E37_79B9`) collided on every anti-diagonal
//!   with the D1 rule (`seed(j, i) == seed(j', i')` whenever
//!   `i + j == i' + j'`), so e.g. symbol 1 of path 0 replayed symbol 0 of
//!   path 1 bit-for-bit. ANCHOR-SAFE change: the gbm-smoke lane is NOT
//!   anchor-grade (only `block-bootstrap-real` is).
//! - **GBM-smoke source-bar seed base**: [`GBM_SOURCE_SEED_BASE`] — a
//!   dedicated constant, domain-separated from the D1 path-seed family
//!   (the old base `0xC0FFEE + idx · 0x9E37_79B9` was EXACTLY the D1 rule
//!   at the default master seed, so source-bar streams replayed path
//!   streams).

use anyhow::{Context, Result};
use rayon::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

// ── Generator kind ────────────────────────────────────────────────────────────

/// Which path generator to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum GeneratorKind {
    /// Block-bootstrap generator on real Binance data (headline generator).
    /// Requires `--features realdata`. The anchored scenario uses this.
    BlockBootstrapReal,
    /// GBM smoke-test generator. Does NOT require real data. NOT anchored.
    GbmSmoke,
}

impl GeneratorKind {
    /// The label rendered into the hashed report body (FP-C2.4 / K4).
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::BlockBootstrapReal => "block-bootstrap-real",
            Self::GbmSmoke => "gbm-smoke",
        }
    }
}

// ── Seed derivation ───────────────────────────────────────────────────────────

/// ADR-0051 D1: derive per-path seed from master seed and path index.
/// `path_seed_j = master_seed.wrapping_add((j as u64).wrapping_mul(0x9E3779B9))`
#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation)] // usize → u64 is lossless on all supported targets
pub fn derive_path_seed(master: u64, j: usize) -> u64 {
    master.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))
}

/// `SplitMix64` finalizer (Steele–Lea–Flood 2014) — the same bijective
/// avalanche mixer used by the 1-13 `data::synth::gbm` seed-collision fix.
#[inline]
#[must_use]
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Derive the per-symbol GBM-smoke seed from `(path_seed, sym_i)`.
///
/// `splitmix64(splitmix64(path_seed) + sym_i)` — the 1-13 `GbmPathGen` idiom:
/// the outer mix breaks the linear relation between neighbouring `sym_i`, the
/// inner mix breaks the linear relation between neighbouring ADR-0051 D1 path
/// seeds, so no `(path, symbol)` anti-diagonal pair can collide structurally
/// (the previous additive scheme collided for EVERY master seed).
#[inline]
#[must_use]
#[allow(clippy::cast_possible_truncation)] // usize → u64 is lossless on all supported targets
pub fn derive_gbm_sym_seed(path_seed: u64, sym_i: usize) -> u64 {
    splitmix64(splitmix64(path_seed).wrapping_add(sym_i as u64))
}

/// Seed base for the GBM-smoke SOURCE bar series (`load_source_bars`),
/// domain-separated from the ADR-0051 D1 path-seed family (review 1-14).
///
/// The old inline base was `0xC0FFEE` — the DEFAULT master ensemble seed —
/// stepped by the D1 constant, so source-bar seed `idx` was bit-identical to
/// path seed `j = idx` at the default seed. Source bars are derived via
/// [`derive_gbm_sym_seed`]`(GBM_SOURCE_SEED_BASE, idx)`. NOT anchor-grade
/// (gbm-smoke lane only).
pub const GBM_SOURCE_SEED_BASE: u64 = 0x5EED_BA5E;

// ── CLI validation helpers ────────────────────────────────────────────────────

/// Upper bound for `--paths` (review 1-14): a sane cap so an absurd N fails
/// fast instead of attempting an unbounded fan-out/allocation.
pub const MAX_PATHS: usize = 100_000;

/// Validate the `--paths` argument (review 1-14).
///
/// # Errors
///
/// - `paths < 2`: a "distribution" over 0 or 1 paths is degenerate (0 used to
///   die late with a misleading reducer error; 1 has no spread).
/// - `paths > MAX_PATHS`: refuse an absurd N before any allocation.
pub fn validate_paths(paths: usize) -> Result<()> {
    if paths < 2 {
        anyhow::bail!(
            "--paths {paths}: a distribution summary needs at least 2 paths \
             (N=0 has nothing to reduce; N=1 has no spread). Pass --paths >= 2 \
             (anchored default: 500)."
        );
    }
    if paths > MAX_PATHS {
        anyhow::bail!(
            "--paths {paths}: exceeds the sanity cap of {MAX_PATHS} paths. \
             This harness is sized for N in the hundreds-to-thousands range."
        );
    }
    Ok(())
}

/// Map `--year` to its full-year hourly bar count (review 1-14).
///
/// # Errors
///
/// Bails on any year without an explicit mapping — the old code silently
/// mapped unmapped years to 8760 bars (leap-wrong for e.g. 2028, and the
/// report would be mislabeled with a span the data does not cover).
pub fn bar_count_for_year(year: i32) -> Result<usize> {
    match year {
        2023 => Ok(8760),
        2024 => Ok(8784),
        _ => anyhow::bail!(
            "--year {year}: unsupported (supported: 2023, 2024). Refusing to \
             guess a bar count — an unmapped year would silently produce a \
             mislabeled/leap-wrong report."
        ),
    }
}

/// Build the scenario name for a generator/year pair (review 1-14).
///
/// The `block-bootstrap-real` string is the ANCHORED scenario NAME
/// (`v1-momentum-2023-block-bootstrap-real-fy-mc` keys the locked anchor in
/// `evidence/anchors.toml`) — it must never drift. The gbm-smoke lane
/// previously reused the `block-bootstrap` token despite running no
/// bootstrap; it now names its generator honestly.
#[must_use]
pub fn scenario_name(generator: GeneratorKind, year: i32) -> String {
    match generator {
        GeneratorKind::BlockBootstrapReal => {
            format!("v1-momentum-{year}-block-bootstrap-real-fy-mc")
        }
        GeneratorKind::GbmSmoke => format!("v1-momentum-{year}-gbm-smoke-fy-mc"),
    }
}

// ── Per-path metric struct ────────────────────────────────────────────────────

/// One path's metrics tagged with its ensemble index `j` (ADR-0051 D1/D2).
#[derive(Debug, Clone)]
pub struct IndexedPathMetrics {
    /// Path index within the ensemble (seed-binding index, NOT completion order).
    pub j: usize,
    /// The per-path metric scalars.
    pub metrics: crate::stats::PathMetrics,
}

// ── Per-path runner (moved verbatim from bin/monte_carlo.rs) ─────────────────

/// Run one Monte-Carlo path (called from the rayon `par_iter` fan-out).
///
/// Returns `(j, PathMetrics)`. The index `j` is bound at the CALL SITE (before
/// rayon schedules the task) so completion order cannot affect which seed any
/// path receives (ADR-0051 D1).
///
/// # Errors
///
/// Propagates path-generation, config-load, and `run_path` engine errors,
/// each wrapped with the failing path index `j`.
#[allow(clippy::too_many_arguments)]
// Verbatim seam extraction from `bin/monte_carlo.rs` (1-14 review patch 1):
// splitting it into sub-fns would diverge the lifted code from its origin,
// defeating the byte-parity argument — mirrors the R-NR.5 verbatim-lift rule.
#[allow(clippy::too_many_lines)]
pub fn run_one_path(
    j: usize,
    path_seed_j: u64,
    fill_seed: u64,
    universe: &[(trading_core::Symbol, rust_decimal::Decimal)],
    real_bars_by_symbol: &[(trading_core::Symbol, Vec<trading_core::Bar>)],
    bar_count: usize,
    generator: GeneratorKind,
    year: i32,
) -> Result<IndexedPathMetrics> {
    // ── Generate the synthetic path ───────────────────────────────────────────
    let generated_path = match generator {
        GeneratorKind::BlockBootstrapReal => {
            use data::MonteCarloPathGen as _;
            let source: Vec<(trading_core::Symbol, Vec<trading_core::Bar>)> =
                real_bars_by_symbol.to_vec();
            let path_gen = data::BlockBootstrapPathGen::new(source, data::BlockLengthPolicy::Auto)
                .with_context(|| format!("build BlockBootstrapPathGen for path {j}"))?;
            path_gen
                .generate(universe, bar_count, path_seed_j)
                .with_context(|| format!("generate path {j}"))?
        }
        GeneratorKind::GbmSmoke => {
            // GBM: generate per-symbol synthetic bars and wrap in GeneratedPath.
            let bars_by_symbol: Vec<Vec<trading_core::Bar>> = universe
                .iter()
                .enumerate()
                .map(|(sym_i, (sym, start_price))| {
                    // Each symbol within path j gets a per-symbol seed derived
                    // from the path seed via SplitMix64 mixing (review 1-14 —
                    // the old additive derivation collided on anti-diagonals
                    // with the ADR-0051 D1 path-seed rule; see module doc).
                    let sym_seed = derive_gbm_sym_seed(path_seed_j, sym_i);
                    crate::scenarios::momentum::synthetic_bars_hourly(
                        sym,
                        bar_count,
                        sym_seed,
                        *start_price,
                        year,
                    )
                })
                .collect();
            data::GeneratedPath {
                bars_by_symbol,
                selected_block_length: None,
                funding_by_symbol: None,
                basis_by_symbol: None,
            }
        }
    };

    // ── Merge per-symbol bars into the flat replay feed ───────────────────────
    let merged_bars = data::ReplayFeed::merge_synthetic(generated_path.bars_by_symbol);

    // ── Build fresh strategy for this path ───────────────────────────────────
    let rel_path = std::path::PathBuf::from("config/strategies/top10_momentum_h1.toml");
    let toml_path = crate::paths::resolve_workspace_path(&rel_path);
    let cfg = strategy::CrossSectionalMomentumConfig::from_file(&toml_path)
        .with_context(|| format!("load momentum config for path {j}"))?;
    let strat = strategy::MomentumStrategy::from_config(
        cfg,
        smol_str::SmolStr::new(toml_path.to_string_lossy()),
    );

    // ── Run the backtest on this path ─────────────────────────────────────────
    let input = crate::cli_types::TcnScenarioInput {
        scenario_name: format!("mc-path-{j}"),
        start_year: year,
        bar_count: merged_bars.len(),
        initial_capital: rust_decimal_macros::dec!(100_000),
        slippage_bps: 2,
        taker_fee_bps: 4,
        config_id: "top10_momentum_h1".to_string(),
        forecaster_id: "montecarlo".to_string(),
        bars_override: Some(merged_bars),
        emit_equity_bin: None,
        latency_slippage_sim: crate::cli_types::LatencySlippageSimConfig::default(),
        funding_override: None,
        basis_override: None,
        bar_span_hours: 1,
    };

    let result = pollster::block_on(crate::scenarios::montecarlo::run_path(
        input, fill_seed, strat,
    ))
    .with_context(|| format!("run_path for MC path {j}"))?;

    // ── Compute per-path metric scalars ───────────────────────────────────────
    // Guard: clamp equity curve values to a small positive floor before computing
    // metrics. If equity goes negative (a "ruin" path — the strategy lost more than
    // the initial capital, which can happen in the GBM smoke test with high volatility),
    // the log-return computation in compute_sharpe_hourly would produce NaN.
    // We clamp negative equity to 1e-6 (representing near-zero remnant capital)
    // so the Sharpe on a ruin path is a finite large-negative number rather than NaN.
    // This is intentional: ADR-0051 D2 asserts NaN absent; we prevent NaN here.
    let equity_clamped: Vec<Decimal> = result
        .equity_curve
        .iter()
        .map(|&e| {
            if e <= Decimal::ZERO {
                dec!(0.000001)
            } else {
                e
            }
        })
        .collect();

    let sharpe = crate::stats::compute_sharpe_hourly(&equity_clamped);
    let sortino = crate::stats::compute_sortino_hourly(&equity_clamped);
    let calmar = crate::stats::compute_calmar(&equity_clamped);
    let max_dd = crate::stats::compute_max_drawdown_f64(&equity_clamped);
    let total_ret = crate::stats::compute_total_return(&equity_clamped);

    // Assert no NaN — if clamping didn't prevent NaN, something is wrong structurally.
    debug_assert!(sharpe.is_finite(), "Sharpe NaN after clamping at path {j}");
    debug_assert!(
        sortino.is_finite(),
        "Sortino NaN after clamping at path {j}"
    );
    debug_assert!(calmar.is_finite(), "Calmar NaN after clamping at path {j}");

    tracing::trace!(
        j,
        path_seed_j,
        sharpe,
        max_dd,
        trades = result.trades,
        "path complete"
    );

    Ok(IndexedPathMetrics {
        j,
        metrics: crate::stats::PathMetrics {
            sharpe,
            sortino,
            calmar,
            max_drawdown: max_dd,
            total_return: total_ret,
            final_equity: result.final_equity,
            initial_equity: result.initial_equity,
        },
    })
}

// ── Ensemble fan-out + reduce (moved verbatim from bin/monte_carlo.rs main) ──

/// Fan out over N paths in parallel, then reduce in path-index order.
///
/// This is THE production ensemble chain (extracted from `main` at review
/// 1-14 so the R-NR.6 gates exercise it directly):
///
/// - **ADR-0051 D1**: `path_seed_j = ensemble_seed.wrapping_add(j * 0x9E3779B9)`.
///   The seed is bound to index `j`; rayon completion order does NOT affect seeds.
/// - **ADR-0051 D2**: results are collected into a Vec, sorted by `j`
///   ascending (NOT completion order), and reduced sequentially.
///
/// Returns the per-path metrics (in ascending `j` order) plus the reduced
/// [`crate::stats::DistributionSummary`].
///
/// # Errors
///
/// Fails if any path fails ([`run_one_path`]) or the reduction fails
/// (`DistributionSummary::from_path_metrics` — empty input or non-finite
/// metric samples).
#[allow(clippy::too_many_arguments)]
pub fn run_ensemble(
    n_paths: usize,
    ensemble_seed: u64,
    fill_seed: u64,
    universe: &[(trading_core::Symbol, rust_decimal::Decimal)],
    real_bars_by_symbol: &[(trading_core::Symbol, Vec<trading_core::Bar>)],
    bar_count: usize,
    generator: GeneratorKind,
    year: i32,
) -> Result<(
    Vec<crate::stats::PathMetrics>,
    crate::stats::DistributionSummary,
)> {
    let path_indices: Vec<usize> = (0..n_paths).collect();

    let results: Vec<Result<IndexedPathMetrics>> = path_indices
        .into_par_iter()
        .map(|j| {
            let path_seed_j = derive_path_seed(ensemble_seed, j);
            run_one_path(
                j,
                path_seed_j,
                fill_seed,
                universe,
                real_bars_by_symbol,
                bar_count,
                generator,
                year,
            )
        })
        .collect();

    // ── Collect indexed results in path-index order ───────────────────────────
    // ADR-0051 D2: collect into a Vec indexed by j, sort by j so reduction
    // is in ascending index order (NOT completion order).
    let mut indexed: Vec<IndexedPathMetrics> = results
        .into_iter()
        .collect::<Result<Vec<_>>>()
        .context("one or more Monte-Carlo paths failed")?;

    // Sort by path index (ascending) — this is the load-bearing step for D2.
    indexed.sort_by_key(|r| r.j);

    let metrics: Vec<crate::stats::PathMetrics> = indexed.into_iter().map(|r| r.metrics).collect();

    // ── Reduce (ADR-0051 D2: sequential in index order) ───────────────────────
    let summary = crate::stats::DistributionSummary::from_path_metrics(&metrics)
        .context("build DistributionSummary")?;

    Ok((metrics, summary))
}
