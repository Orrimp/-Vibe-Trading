//! Monte-Carlo synthetic path generators.
//!
//! Exposes the [`MonteCarloPathGen`] trait, the [`BlockBootstrapPathGen`]
//! (stationary block bootstrap — Politis–Romano 1994, headline generator),
//! the [`GbmPathGen`] (GBM smoke-test impl), and [`BlockLengthPolicy`].
//!
//! All generators are **pure functions** of their stated inputs:
//! identical `(universe, n_bars, path_seed)` ⇒ byte-identical output.
//! Randomness flows exclusively from [`rand_chacha::ChaCha20Rng`] (ADR-0002).
//! Money is `rust_decimal::Decimal` at `Bar` boundaries (ADR-0003).

pub mod block_length;
pub mod bootstrap;
pub mod gbm;

use rust_decimal::Decimal;
use trading_core::{Bar, Symbol};

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use bootstrap::BlockBootstrapPathGen;
pub use gbm::GbmPathGen;

// ── Core types ────────────────────────────────────────────────────────────────

/// One synthetic ensemble member: the per-symbol bar series for a single path.
///
/// Returned by [`MonteCarloPathGen::generate`]. The `selected_block_length`
/// field carries the auto-chosen (or fixed) `L` so C2 can print it in the
/// anchored report body (R3.2 / ADR-0051 D3). For generators with no
/// block-length concept (e.g. `GbmPathGen`) this field is `None`.
///
/// ## `funding_by_symbol` — ADR-0051 § D6.6 (carry-strategy Stage 2)
///
/// When the carry-strategy funding source is present in the generator,
/// `funding_by_symbol[sym_i][bar_i]` holds the resampled funding rate for
/// output bar `bar_i` of symbol `sym_i`. The resampling uses the **identical
/// `idx_seq`** that selected the bar's return — this is the co-resampling
/// invariant (FP-C1.5 extended to a second series): price and funding always
/// reflect the same underlying real source index.
///
/// When absent (`None`): momentum/MR/buy-and-hold paths are **byte-identical**
/// to the pre-carry code (the field is never written, the generator takes the
/// same code path). This is the anchor-neutrality guarantee.
///
/// `funding_by_symbol[sym_i][0]` corresponds to bar-0 (the "start" bar, no
/// return applied). Convention: bar-0 carries the same funding as the first
/// real source bar (index 0), i.e. the most-recent funding at `real_bar[0]`'s
/// open-ts. Bar-0 is a price-only sentinel; strategies should look at bar-1
/// onwards for the carry signal (the warm-up is the same as the price ring-
/// buffer warm-up for momentum).
#[derive(Debug, Clone)]
#[must_use]
pub struct GeneratedPath {
    /// Per-symbol bar series (universe order preserved).
    /// `bars_by_symbol[i]` is the bar series for `universe[i].0`.
    pub bars_by_symbol: Vec<Vec<Bar>>,
    /// Block length actually used. `Some(L)` for block-bootstrap generators;
    /// `None` for GBM and other non-block generators.
    pub selected_block_length: Option<usize>,
    /// Co-resampled funding rates (ADR-0051 § D6.6, carry-strategy Stage 2).
    ///
    /// `funding_by_symbol[sym_i][bar_i]` = the resampled funding rate for
    /// output bar `bar_i` of universe symbol `sym_i`. Indexed by the **same
    /// `ret_idx`** that selected the return for that bar — price and funding
    /// are always contemporaneous with their real source.
    ///
    /// `None` when no funding source was supplied to the generator (every
    /// momentum/MR/buy-and-hold run). Absent = byte-identical to pre-carry.
    pub funding_by_symbol: Option<Vec<Vec<Option<Decimal>>>>,
}

/// Block-length selection policy for [`BlockBootstrapPathGen`].
///
/// - `Fixed(L)` — use `L` as the expected block length for every draw.
///   `L = 1` degenerates to i.i.d. resampling (every step restarts a block).
///   Useful for tests and the GBM smoke-test where a trivially checkable `L`
///   is preferred.
/// - `Auto` — select `L` automatically via the Politis–White (2004) / Patton–
///   Politis–White (2009) spectral-density (PWSD) method (see
///   [`block_length::politis_white_block_length`] for the algorithm).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockLengthPolicy {
    /// Fixed expected block length (≥ 1). `Fixed(1)` = i.i.d. bootstrap.
    Fixed(usize),
    /// Automatic Politis–White / PPW-2009 selection from the source series.
    Auto,
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Pure synthetic path generator.
///
/// Implementors MUST satisfy the determinism contract:
/// - Identical `(self, universe, n_bars, path_seed)` ⇒ identical [`GeneratedPath`].
/// - No `thread_rng`, no `OsRng`, no wall-clock, no env reads.
/// - All randomness flows from `ChaCha20Rng::seed_from_u64(path_seed)` only.
///
/// C2 derives `path_seed` via ADR-0051 D1:
/// `path_seed_j = master.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))`.
pub trait MonteCarloPathGen {
    /// Generate one synthetic ensemble member.
    ///
    /// # Parameters
    /// - `universe` — `(symbol, start_price)` pairs; the generator rebuilds
    ///   the price path starting at `start_price` for each symbol.
    ///   All source series MUST have equal length (rectangular universe);
    ///   ragged input is an error.
    /// - `n_bars` — number of output bars (the synthetic path length).
    /// - `path_seed` — the single `u64` that seeds all randomness for this path.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the source series are ragged (unequal lengths) or
    /// if any source series is too short to resample (< 2 bars).
    fn generate(
        &self,
        universe: &[(Symbol, Decimal)],
        n_bars: usize,
        path_seed: u64,
    ) -> Result<GeneratedPath, SynthError>;
}

// ── Error ─────────────────────────────────────────────────────────────────────

/// Errors from the synthetic path generators.
#[derive(Debug, thiserror::Error)]
pub enum SynthError {
    /// Universe source series have unequal lengths (shared-index requires
    /// a rectangular universe — all symbols must have the same bar count).
    #[error("ragged universe: symbol '{symbol}' has {actual} bars; expected {expected}")]
    RaggedUniverse {
        /// Name of the offending symbol.
        symbol: String,
        /// Bar count of the offending symbol.
        actual: usize,
        /// Expected bar count (first symbol's bar count).
        expected: usize,
    },

    /// The source series is too short to produce a meaningful bootstrap
    /// (need ≥ 2 bars to compute at least one log-return).
    #[error("source series too short: {symbol} has {len} bars (need ≥ 2)")]
    SeriesTooShort {
        /// Name of the offending symbol.
        symbol: String,
        /// Actual bar count.
        len: usize,
    },

    /// `n_bars` must be ≥ 1.
    #[error("n_bars must be ≥ 1, got 0")]
    ZeroBars,

    /// Universe is empty.
    #[error("universe is empty — nothing to generate")]
    EmptyUniverse,
}
