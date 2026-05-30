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
#[derive(Debug, Clone)]
#[must_use]
pub struct GeneratedPath {
    /// Per-symbol bar series (universe order preserved).
    /// `bars_by_symbol[i]` is the bar series for `universe[i].0`.
    pub bars_by_symbol: Vec<Vec<Bar>>,
    /// Block length actually used. `Some(L)` for block-bootstrap generators;
    /// `None` for GBM and other non-block generators.
    pub selected_block_length: Option<usize>,
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
