//! Monte-Carlo synthetic path generators.
//!
//! Exposes the [`MonteCarloPathGen`] trait, the [`BlockBootstrapPathGen`]
//! (stationary block bootstrap — Politis–Romano 1994, headline generator),
//! the [`GbmPathGen`] (GBM smoke-test impl), and [`BlockLengthPolicy`].
//!
//! All generators are **pure functions** of their stated inputs:
//! identical `(universe, n_bars, path_seed)` ⇒ byte-identical output **on a
//! given platform/toolchain**. The return-space `f64` math (`ln`/`exp` via
//! libm) may differ by 1 ulp across platforms; the byte-identical determinism
//! scope is the Apple-Silicon canonical box (ADR-0051 D5).
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

// ── Global bounds ─────────────────────────────────────────────────────────────

/// Upper bound on `n_bars` accepted by [`MonteCarloPathGen::generate`].
///
/// Why 2,000,000: hourly bars overflow the `time` crate's year-9999 ceiling at
/// ≈ 69.9M bars from the 2023 epoch (a panic), and unbounded `n_bars` can abort
/// on allocation long before that. 2M hourly bars ≈ 228 years (epoch 2023 →
/// year ≈ 2251) — comfortably inside the `time` range while > 200× the largest
/// legitimate shipped request (one year of hourly bars: 8,784 in the
/// `monte_carlo` / `param_robustness_sweep` binaries; the integration-test
/// ensembles use the real-series length, ≤ a few×10⁴). Requests above this
/// bound are rejected with [`SynthError::TooManyBars`] instead of panicking.
pub const MAX_N_BARS: usize = 2_000_000;

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
///
/// ## `basis_by_symbol` — ADR-0051 § D6.10 (perp-basis-mn-spread M-DEV-1)
///
/// The exact twin of `funding_by_symbol` for the perp-spot basis series.
/// When the MN-spread basis source is present in the generator,
/// `basis_by_symbol[sym_i][bar_i]` holds the resampled basis value for
/// output bar `bar_i` of symbol `sym_i`. Co-resampled at the **identical
/// `idx_seq`** that selected the return AND the funding — ZERO new RNG draws
/// (the three-series shared-index extension, D6.6.5).
///
/// When absent (`None`): all non-MN-spread runs (momentum/MR/carry/basis-reversal)
/// are **byte-identical** to pre-M-DEV-1 code. Default `None` at every
/// non-MN construction site (~6 `GeneratedPath` literals).
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
    /// Co-resampled perp-spot basis values (ADR-0051 § D6.10, M-DEV-1).
    ///
    /// Exact twin of `funding_by_symbol` for the basis series. Gathered from
    /// `basis_at_return[sym_i][idx_seq[k]]` at the SAME `idx_seq` — ZERO new
    /// RNG draws. `None` for every non-MN-spread run (anchor-neutral).
    pub basis_by_symbol: Option<Vec<Vec<Option<Decimal>>>>,
}

/// Block-length selection policy for [`BlockBootstrapPathGen`].
///
/// - `Fixed(L)` — use `L` as the expected block length for every draw.
///   `L = 1` degenerates to i.i.d. resampling (every step restarts a block).
///   Useful for tests and the GBM smoke-test where a trivially checkable `L`
///   is preferred. Valid domain: `1 ≤ L ≤ n_returns` (the source's log-return
///   count); `Fixed(0)` and `Fixed(L) > n_returns` are rejected by `generate`
///   with [`SynthError::InvalidFixedBlockLength`] — an `L ≫ n_returns` makes
///   the restart probability `p = 1/L ≈ 0`, degenerating the "ensemble" into
///   a circular replay of the source with zero dispersion.
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
    ///   the price path starting at `start_price` for each symbol. For source-
    ///   backed generators the universe must correspond 1:1 (same arity, same
    ///   symbol at each index) to the source series the generator was built
    ///   from. Start prices must be strictly positive.
    /// - `n_bars` — number of output bars (the synthetic path length),
    ///   `1 ..= MAX_N_BARS`.
    /// - `path_seed` — the single `u64` that seeds all randomness for this path.
    ///
    /// # Errors
    ///
    /// Returns `Err` on request-validation failure. Which conditions apply is
    /// implementation-specific; the shipped implementations reject:
    /// - an empty universe ([`SynthError::EmptyUniverse`]), `n_bars == 0`
    ///   ([`SynthError::ZeroBars`]), or `n_bars > MAX_N_BARS`
    ///   ([`SynthError::TooManyBars`]) — both generators;
    /// - a non-positive `start_price`
    ///   ([`SynthError::NonPositiveStartPrice`]) — both generators;
    /// - a universe that does not match the generator's source series 1:1
    ///   ([`SynthError::UniverseSourceArityMismatch`] /
    ///   [`SynthError::UniverseSymbolMismatch`]) or an out-of-range
    ///   `BlockLengthPolicy::Fixed` ([`SynthError::InvalidFixedBlockLength`])
    ///   — [`BlockBootstrapPathGen`];
    /// - invalid [`gbm::GbmParams`] ([`SynthError::InvalidGbmParams`]) —
    ///   [`GbmPathGen`].
    ///
    /// Ragged (unequal-length) or too-short source series are rejected at
    /// **construction** ([`BlockBootstrapPathGen::new`]:
    /// [`SynthError::RaggedUniverse`] / [`SynthError::SeriesTooShort`]);
    /// `generate` cannot raise them through the public API (it re-checks
    /// series length defensively).
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

    /// The source series is too short to produce a meaningful bootstrap.
    ///
    /// ≥ 3 bars (≥ 2 log-returns) are required: a 2-bar source has a single
    /// return, so every resampling index is forced to 0 and the "ensemble" is
    /// seed-independent with zero dispersion — divergence must be structurally
    /// possible.
    #[error("source series too short: {symbol} has {len} bars (need ≥ 3 bars / ≥ 2 log-returns)")]
    SeriesTooShort {
        /// Name of the offending symbol.
        symbol: String,
        /// Actual bar count.
        len: usize,
    },

    /// `n_bars` must be ≥ 1.
    #[error("n_bars must be ≥ 1, got 0")]
    ZeroBars,

    /// `n_bars` exceeds [`MAX_N_BARS`] (guards `time`-crate year-9999
    /// overflow panics and allocation aborts — see the constant's doc).
    #[error("n_bars {n_bars} exceeds the maximum of {max} (MAX_N_BARS)")]
    TooManyBars {
        /// Requested output bar count.
        n_bars: usize,
        /// The enforced upper bound ([`MAX_N_BARS`]).
        max: usize,
    },

    /// Universe is empty.
    #[error("universe is empty — nothing to generate")]
    EmptyUniverse,

    /// The universe passed to `generate` has a different symbol count than
    /// the source series the generator was constructed from. The pairing is
    /// positional — a longer universe would previously fall back to symbol
    /// 0's returns silently; now it is a loud error.
    #[error(
        "universe/source arity mismatch: universe has {universe_len} symbols, \
         source has {source_len}"
    )]
    UniverseSourceArityMismatch {
        /// Symbol count of the universe passed to `generate`.
        universe_len: usize,
        /// Symbol count of the source series the generator holds.
        /// (Named `source_len`, not `source` — a `source` field would be
        /// picked up by thiserror as the error-source chain.)
        source_len: usize,
    },

    /// The universe symbol at `index` does not match the source symbol at the
    /// same index. The shared-index bootstrap pairs universe and source
    /// positionally; a name mismatch means the caller would resample the
    /// wrong series under the requested symbol's name.
    #[error(
        "universe symbol mismatch at index {index}: universe has '{universe_symbol}', \
         source has '{source_symbol}' (pairing is positional)"
    )]
    UniverseSymbolMismatch {
        /// Position in the universe / source ordering.
        index: usize,
        /// Symbol name supplied in the universe.
        universe_symbol: String,
        /// Symbol name held in the source series at that index.
        source_symbol: String,
    },

    /// A source bar's close price is not strictly positive. (Unreachable
    /// through `trading_core::Price` today — `Price::new` rejects `d <= 0` —
    /// but enforced here so the log-return math can never see a zero/negative
    /// close even if `Bar` construction changes.)
    #[error("non-positive source close for {symbol} at bar {bar_index}: {close}")]
    NonPositiveSourcePrice {
        /// Name of the offending symbol.
        symbol: String,
        /// Index of the offending bar within the source series.
        bar_index: usize,
        /// The offending close value.
        close: Decimal,
    },

    /// A universe `start_price` is not strictly positive. Previously clamped
    /// silently (bootstrap → 1e-6, GBM → `price_lo`), producing plausible-
    /// shaped garbage paths; now a loud error.
    #[error("non-positive start_price for {symbol}: {start_price}")]
    NonPositiveStartPrice {
        /// Symbol whose start price is invalid.
        symbol: String,
        /// The offending start price.
        start_price: Decimal,
    },

    /// `BlockLengthPolicy::Fixed(L)` is out of range for the source series.
    /// `L = 0` was previously promoted to 1 silently; `L > n_returns` drives
    /// the restart probability `p = 1/L` toward 0 and degenerates the
    /// ensemble into a zero-dispersion circular replay of the source.
    #[error(
        "invalid fixed block length L={l}: must be in 1..={n_returns} \
         (the source's log-return count)"
    )]
    InvalidFixedBlockLength {
        /// The requested fixed block length.
        l: usize,
        /// Number of log-returns available in the source.
        n_returns: usize,
    },

    /// The [`gbm::GbmParams`] are invalid (non-finite fields or inverted /
    /// non-positive price clamp bounds). Previously `f64::clamp` would panic
    /// on inverted or NaN bounds and non-finite vol/drift would silently
    /// flatline the path to `price_lo`.
    #[error("invalid GbmParams: {reason}")]
    InvalidGbmParams {
        /// Human-readable description of the failed validation.
        reason: String,
    },
}
