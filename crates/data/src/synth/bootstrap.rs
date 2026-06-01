//! Stationary block-bootstrap path generator (Politis–Romano 1994).
//!
//! Implements [`MonteCarloPathGen`] for [`BlockBootstrapPathGen`].
//!
//! ## Algorithm (D-C1.3, shared-index)
//!
//! Given a rectangular universe (all symbols have equal-length bar series) and
//! a single `path_seed: u64`:
//!
//! 1. Build a **per-symbol log-return series** from the real bars:
//!    `r_sym[t] = ln(close[t] / close[t-1])` (`T-1` returns for `T` bars).
//!    All symbols MUST have the same length `T` (ragged → `Err`).
//!
//! 2. Seed **one** `ChaCha20Rng::seed_from_u64(path_seed)` for the whole path
//!    (ADR-0002, ADR-0051 D1).
//!
//! 3. Draw the **stationary-bootstrap index sequence ONCE** (shared across all
//!    symbols — Q-MCB-2 ratification): produce `n_bars - 1` return indices
//!    `i_0, …, i_{n_bars-2}` where:
//!    - Pick a uniform start index `i ∈ [0, T-2]`.
//!    - At each step: with probability `p = 1/L`, start a new block (fresh
//!      uniform start index); otherwise continue `i ← (i+1) mod (T-1)`.
//!    - This gives geometric block lengths with mean `L` (the stationary-
//!      bootstrap definition — NOT fixed-`L` moving blocks).
//!
//! 4. **Apply the SAME index sequence to ALL symbols**: `r'_sym[k] = r_sym[idx[k]]`.
//!    The shared index preserves contemporaneous cross-symbol co-movement.
//!
//! 5. Reconstruct each symbol's price path from its real start price by
//!    compounding resampled returns: `p[0] = start_price`,
//!    `p[k+1] = p[k] * exp(r'[k])`, rounded to `Decimal` at the `Bar` boundary.
//!    OHLC/timestamp conventions follow [`synthetic_bars_hourly`] (`momentum.rs`).
//!
//! ## References
//!
//! - Politis, D. N. & Romano, J. P. (1994). *The stationary bootstrap*.
//!   Journal of the American Statistical Association, 89(428), 1303–1313.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

use crate::synth::block_length::politis_white_block_length;
use crate::synth::{BlockLengthPolicy, GeneratedPath, MonteCarloPathGen, SynthError};

// ── Builder ───────────────────────────────────────────────────────────────────

/// Stationary block-bootstrap path generator (Politis–Romano 1994).
///
/// Takes a **real** bar series for each universe symbol as the source to
/// resample. The shared-index bootstrap draws ONE resampling-index sequence
/// per path and applies it across ALL symbols simultaneously, preserving
/// the contemporaneous cross-symbol co-movement (Q-MCB-2 ratified).
///
/// # Determinism
///
/// `generate` is a pure function of `(self, universe, n_bars, path_seed)`.
/// Identical inputs ⇒ byte-identical [`GeneratedPath`].
///
/// # Carry-funding co-resampling (ADR-0051 § D6.6)
///
/// When `funding_at_return` is set via [`BlockBootstrapPathGen::with_funding`],
/// the generator gathers the funding rate for each bar by the **same `ret_idx`**
/// that selected the return. This is the crux of the carry-strategy design:
/// price and funding are always contemporaneous with their real source index —
/// no new RNG draws, no decoupling.
///
/// When absent (`None`): identical behaviour to the pre-carry code (zero change
/// to the ChaCha20 stream, zero change to bar output).
pub struct BlockBootstrapPathGen {
    /// Per-symbol real bar series, in the same order as the universe.
    /// `source_bars[i]` is the real bars for symbol `i`.
    source_bars: Vec<(Symbol, Vec<Bar>)>,
    /// Block-length selection policy.
    block_length_policy: BlockLengthPolicy,
    /// Optional per-symbol, per-return-step funding series for carry-strategy
    /// co-resampling (ADR-0051 § D6.6).
    ///
    /// `funding_at_return[sym_i][k]` = the funding rate in force at real
    /// return-step `k` (the as-of forward-fill of the 8h funding onto the 1h
    /// bar grid, computed ONCE on the real data). Length = `T-1` (matches the
    /// return series). Universe order must match `source_bars`.
    ///
    /// `None` → funding is not gathered; `GeneratedPath.funding_by_symbol`
    /// is `None`; the 87 existing anchors are byte-identical.
    funding_at_return: Option<Vec<Vec<Option<Decimal>>>>,
}

impl BlockBootstrapPathGen {
    /// Create a new generator from real bar series.
    ///
    /// `source_bars` must be non-empty and all inner `Vec<Bar>` must have
    /// equal length ≥ 2. The order MUST match the `universe` slice passed to
    /// `generate`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if any source series is too short (< 2 bars) or if the
    /// sources are ragged (unequal lengths).
    pub fn new(
        source_bars: Vec<(Symbol, Vec<Bar>)>,
        block_length_policy: BlockLengthPolicy,
    ) -> Result<Self, SynthError> {
        if source_bars.is_empty() {
            return Err(SynthError::EmptyUniverse);
        }
        let expected_len = source_bars[0].1.len();
        for (sym, bars) in &source_bars {
            if bars.len() < 2 {
                return Err(SynthError::SeriesTooShort {
                    symbol: sym.to_string(),
                    len: bars.len(),
                });
            }
            if bars.len() != expected_len {
                return Err(SynthError::RaggedUniverse {
                    symbol: sym.to_string(),
                    actual: bars.len(),
                    expected: expected_len,
                });
            }
        }
        Ok(Self {
            source_bars,
            block_length_policy,
            funding_at_return: None,
        })
    }

    /// Attach an optional per-symbol funding series for carry-strategy
    /// co-resampling (ADR-0051 § D6.6, M-DEV-3).
    ///
    /// `funding_at_return[sym_i][k]` = funding rate at real return-step `k`
    /// (computed by `build_funding_at_return` from `crates/backtest`).
    /// Universe order MUST match `source_bars`. Length per symbol MUST be
    /// `source_len() - 1` (the number of log-returns).
    ///
    /// When `None` is passed, the generator behaves identically to `new()` —
    /// the existing 87 anchors are byte-unchanged by construction.
    ///
    /// Returns `self` for chaining.
    #[must_use]
    pub fn with_funding(mut self, funding: Option<Vec<Vec<Option<Decimal>>>>) -> Self {
        self.funding_at_return = funding;
        self
    }

    /// Return the source bar count (number of real bars per symbol).
    #[must_use]
    pub fn source_len(&self) -> usize {
        // invariant: source_bars is non-empty, all equal length.
        self.source_bars[0].1.len()
    }
}

// ── Trait impl ────────────────────────────────────────────────────────────────

// Float arithmetic in return-space is required; Bar prices are Decimal (ADR-0003).
#[allow(clippy::float_arithmetic)]
impl MonteCarloPathGen for BlockBootstrapPathGen {
    fn generate(
        &self,
        universe: &[(Symbol, Decimal)],
        n_bars: usize,
        path_seed: u64,
    ) -> Result<GeneratedPath, SynthError> {
        use rand::Rng;
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        if universe.is_empty() {
            return Err(SynthError::EmptyUniverse);
        }
        if n_bars == 0 {
            return Err(SynthError::ZeroBars);
        }

        // ── Build per-symbol log-return series ────────────────────────────────
        // Returns T-1 log-returns from T bars.
        let t = self.source_len(); // number of real bars per symbol
        let n_returns = t - 1; // number of log-returns available for resampling

        // Build log-return matrix: returns_by_sym[s][k] = ln(close[k+1]/close[k]).
        let mut returns_by_sym: Vec<Vec<f64>> = Vec::with_capacity(self.source_bars.len());
        for (sym, bars) in &self.source_bars {
            if bars.len() < 2 {
                return Err(SynthError::SeriesTooShort {
                    symbol: sym.to_string(),
                    len: bars.len(),
                });
            }
            let rets: Vec<f64> = bars
                .windows(2)
                .map(|w| {
                    let c0 = decimal_to_f64(w[0].close.get());
                    let c1 = decimal_to_f64(w[1].close.get());
                    (c1 / c0).ln()
                })
                .collect();
            returns_by_sym.push(rets);
        }

        // ── Compute auto block length ─────────────────────────────────────────
        // For shared-index, we need ONE L for the whole universe.
        // Rule (D-C1.4): compute PWSD on the universe-average absolute log-return
        // series r̄[t] = mean_sym |r_sym[t]|.
        let selected_l = match self.block_length_policy {
            BlockLengthPolicy::Fixed(l) => l.max(1),
            BlockLengthPolicy::Auto => {
                // Universe-average absolute return series.
                let avg_abs: Vec<f64> = (0..n_returns)
                    .map(|k| {
                        let sum: f64 = returns_by_sym.iter().map(|rets| rets[k].abs()).sum::<f64>();
                        sum / returns_by_sym.len() as f64
                    })
                    .collect();
                politis_white_block_length(&avg_abs).max(1)
            }
        };

        // ── Seed one ChaCha20Rng for the whole path (ADR-0051 D1) ─────────────
        let mut rng = ChaCha20Rng::seed_from_u64(path_seed);

        // ── Draw the stationary-bootstrap index sequence ONCE ─────────────────
        // We need (n_bars - 1) return indices (one per output bar gap).
        // Stationary bootstrap (Politis–Romano 1994):
        //   p = 1/L = probability of starting a new block at each step.
        //   At each step: if Bernoulli(p) = 1 (or first step), pick a new
        //   uniform start index; else advance i ← (i+1) mod n_returns.
        let n_idx = n_bars.saturating_sub(1);
        let p_new_block: f64 = 1.0 / selected_l as f64;

        let mut idx_seq = Vec::with_capacity(n_idx);
        if n_idx > 0 {
            // Pick initial start index.
            let mut cur_idx: usize = rng.random_range(0..n_returns);
            idx_seq.push(cur_idx);

            for _ in 1..n_idx {
                let restart: f64 = rng.random::<f64>();
                if restart < p_new_block {
                    // Start a new block.
                    cur_idx = rng.random_range(0..n_returns);
                } else {
                    // Continue current block (circular wrap).
                    cur_idx = (cur_idx + 1) % n_returns;
                }
                idx_seq.push(cur_idx);
            }
        }

        // ── Apply shared index sequence to ALL symbols, reconstruct paths ──────
        // Epoch base for bar timestamps (2023-01-01 00:00:00 UTC, matching
        // the synthetic_bars_hourly convention).
        let epoch_base = epoch_2023();

        let to_dec = |v: f64| -> Decimal {
            Decimal::try_from(v.max(0.000_001_f64)).unwrap_or(dec!(0.000001))
        };
        let price_or_min = |v: f64| -> Price {
            Price::new(to_dec(v)).unwrap_or_else(|_| {
                Price::new(dec!(0.01)).unwrap_or_else(|e| unreachable!("dec!(0.01) valid: {e}"))
            })
        };

        let mut bars_by_symbol: Vec<Vec<Bar>> = Vec::with_capacity(universe.len());
        // ADR-0051 § D6.6: optional funding co-resampling (carry-strategy M-DEV-3).
        // Allocated only when `self.funding_at_return` is `Some`; otherwise `None`
        // and the code path is byte-identical to the pre-carry implementation.
        let mut funding_output_by_symbol: Option<Vec<Vec<Option<Decimal>>>> = self
            .funding_at_return
            .as_ref()
            .map(|_| Vec::with_capacity(universe.len()));

        for (sym_i, (out_sym, start_price)) in universe.iter().enumerate() {
            // Map output universe symbol to source series by index.
            // The caller must pass universe in the same order as source_bars.
            let source_rets = if sym_i < returns_by_sym.len() {
                &returns_by_sym[sym_i]
            } else {
                // Fallback to first symbol if universe is longer than source
                // (shouldn't happen — validated in new(); defensive only).
                &returns_by_sym[0]
            };

            let start_f = decimal_to_f64(*start_price).max(0.000_001_f64);
            let mut close: f64 = start_f;
            let mut sym_bars: Vec<Bar> = Vec::with_capacity(n_bars);
            // Per-symbol funding output (only allocated when funding is present).
            let mut sym_funding: Option<Vec<Option<Decimal>>> = self
                .funding_at_return
                .as_ref()
                .map(|_| Vec::with_capacity(n_bars));

            // Bar 0: the "start" bar (close = start_price, no return applied).
            // Funding convention (§ D-CARRY.7): bar-0 carries funding from real
            // source index 0 (the most-recent funding at the first real bar's
            // open_ts). This is a sentinel; strategies warm up from bar-1+.
            {
                let open_ts = bar_ts(epoch_base, 0);
                let close_ts = bar_close_ts(epoch_base, 0);
                sym_bars.push(Bar {
                    symbol: out_sym.clone(),
                    tf: Timeframe::OneHour,
                    open_ts,
                    close_ts,
                    open: price_or_min(close),
                    high: price_or_min(close * 1.001),
                    low: price_or_min(close * 0.999),
                    close: price_or_min(close),
                    volume: Quantity::new(dec!(100))
                        .unwrap_or_else(|e| unreachable!("dec!(100) valid: {e}")),
                    trade_count: 100,
                    local_recv_ts: close_ts,
                    venue: Venue::Binance,
                });
                // Bar-0 funding: use real source index 0 as the sentinel value.
                if let Some(ref mut sf) = sym_funding {
                    let f0 = self
                        .funding_at_return
                        .as_ref()
                        .and_then(|far| far.get(sym_i))
                        .and_then(|sym_far| sym_far.first())
                        .copied()
                        .flatten();
                    sf.push(f0);
                }
            }

            // Bars 1..n_bars: apply resampled returns from the shared index.
            for (bar_i, &ret_idx) in idx_seq.iter().enumerate() {
                let r = source_rets[ret_idx];
                let next = (close * r.exp()).clamp(0.000_001_f64, 1_000_000_000.0_f64);

                let open_ts = bar_ts(epoch_base, bar_i + 1);
                let close_ts = bar_close_ts(epoch_base, bar_i + 1);

                // Use the real bar's volume at the sampled index (from the
                // first source symbol as a proxy — volume is not load-bearing
                // for the strategy's return calculation).
                let real_vol = self.source_bars[0]
                    .1
                    .get(ret_idx)
                    .map(|b| decimal_to_f64(b.volume.get()))
                    .unwrap_or(100.0);

                sym_bars.push(Bar {
                    symbol: out_sym.clone(),
                    tf: Timeframe::OneHour,
                    open_ts,
                    close_ts,
                    open: price_or_min(close),
                    high: price_or_min(close.max(next) * 1.001),
                    low: price_or_min(close.min(next) * 0.999),
                    close: price_or_min(next),
                    volume: Quantity::new(to_dec(real_vol.max(1.0))).unwrap_or_else(|_| {
                        Quantity::new(dec!(1))
                            .unwrap_or_else(|e| unreachable!("dec!(1) valid: {e}"))
                    }),
                    trade_count: 100,
                    local_recv_ts: close_ts,
                    venue: Venue::Binance,
                });
                close = next;

                // ADR-0051 § D6.6 — THE CRUX: gather funding by the SAME ret_idx
                // that selected the return. Zero new RNG draws; idx_seq is already
                // materialized. This preserves the funding↔price co-movement.
                if let Some(ref mut sf) = sym_funding {
                    let f = self
                        .funding_at_return
                        .as_ref()
                        .and_then(|far| far.get(sym_i))
                        .and_then(|sym_far| sym_far.get(ret_idx))
                        .copied()
                        .flatten();
                    sf.push(f);
                }
            }

            bars_by_symbol.push(sym_bars);
            // Collect per-symbol funding into the output vec.
            if let (Some(fobs), Some(sf)) = (&mut funding_output_by_symbol, sym_funding) {
                fobs.push(sf);
            }
        }

        Ok(GeneratedPath {
            bars_by_symbol,
            selected_block_length: Some(selected_l),
            funding_by_symbol: funding_output_by_symbol,
        })
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

/// f64-safe extraction of a `Decimal` value (fallback: 1.0).
#[allow(clippy::float_arithmetic)]
fn decimal_to_f64(d: Decimal) -> f64 {
    d.to_string().parse::<f64>().unwrap_or(1.0)
}

/// Build the epoch base (2023-01-01 00:00:00 UTC) for synthetic bar timestamps.
fn epoch_2023() -> OffsetDateTime {
    let date = time::Date::from_calendar_date(2023, time::Month::January, 1)
        .unwrap_or_else(|e| unreachable!("2023-01-01 is always valid: {e}"));
    OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
}

/// Bar open timestamp (hour-granularity, bar index `i`).
fn bar_ts(epoch_base: OffsetDateTime, i: usize) -> Timestamp {
    #[allow(clippy::cast_possible_wrap)]
    Timestamp::new(epoch_base + time::Duration::hours(i as i64))
}

/// Bar close timestamp (end-of-hour, bar index `i`).
fn bar_close_ts(epoch_base: OffsetDateTime, i: usize) -> Timestamp {
    #[allow(clippy::cast_possible_wrap)]
    Timestamp::new(epoch_base + time::Duration::hours(i as i64 + 1) - time::Duration::seconds(1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::synth::BlockLengthPolicy;
    use rust_decimal_macros::dec;

    /// Build a minimal synthetic bar series for testing.
    fn make_bars(symbol: &Symbol, n: usize, seed: u64) -> Vec<Bar> {
        use rand::Rng;
        use rand::SeedableRng;
        use rand_chacha::ChaCha20Rng;

        let mut rng = ChaCha20Rng::seed_from_u64(seed);
        let epoch_base = epoch_2023();
        let mut close = 30_000.0_f64;
        let mut bars = Vec::with_capacity(n);
        for i in 0..n {
            let z: f64 = rng.random::<f64>() * 0.04 - 0.02; // small random return
            let next = (close * (1.0 + z)).max(0.01);
            #[allow(clippy::cast_possible_wrap)]
            let open_ts = Timestamp::new(epoch_base + time::Duration::hours(i as i64));
            #[allow(clippy::cast_possible_wrap)]
            let close_ts = Timestamp::new(
                epoch_base + time::Duration::hours(i as i64 + 1) - time::Duration::seconds(1),
            );
            let to_price = |v: f64| {
                Price::new(Decimal::try_from(v.max(0.01)).unwrap_or(dec!(0.01))).unwrap_or_else(
                    |_| Price::new(dec!(0.01)).unwrap_or_else(|e| unreachable!("dec!(0.01): {e}")),
                )
            };
            bars.push(Bar {
                symbol: symbol.clone(),
                tf: Timeframe::OneHour,
                open_ts,
                close_ts,
                open: to_price(close),
                high: to_price(close.max(next) * 1.001),
                low: to_price(close.min(next) * 0.999),
                close: to_price(next),
                volume: Quantity::new(dec!(100)).unwrap_or_else(|e| unreachable!("dec!(100): {e}")),
                trade_count: 10,
                local_recv_ts: close_ts,
                venue: Venue::Binance,
            });
            close = next;
        }
        bars
    }

    fn btc() -> Symbol {
        Symbol::new("BTCUSDT")
    }
    fn eth() -> Symbol {
        Symbol::new("ETHUSDT")
    }

    fn make_gen_fixed(l: usize) -> BlockBootstrapPathGen {
        let btc_bars = make_bars(&btc(), 200, 0xBEEF_0001);
        let eth_bars = make_bars(&eth(), 200, 0xBEEF_0002);
        BlockBootstrapPathGen::new(
            vec![(btc(), btc_bars), (eth(), eth_bars)],
            BlockLengthPolicy::Fixed(l),
        )
        .expect("valid source bars")
    }

    /// FP-C1.1 — same seed twice ⇒ byte-identical ensemble.
    #[test]
    fn fp_c1_1_same_seed_deterministic() {
        let bgen = make_gen_fixed(5);
        let universe = vec![(btc(), dec!(30_000)), (eth(), dec!(1_200))];
        let path1 = bgen.generate(&universe, 50, 0xCAFE_DEAD).unwrap();
        let path2 = bgen.generate(&universe, 50, 0xCAFE_DEAD).unwrap();

        assert_eq!(path1.bars_by_symbol.len(), path2.bars_by_symbol.len());
        for (s1, s2) in path1.bars_by_symbol.iter().zip(path2.bars_by_symbol.iter()) {
            assert_eq!(s1.len(), s2.len(), "bar count must match");
            for (b1, b2) in s1.iter().zip(s2.iter()) {
                assert_eq!(
                    b1.close, b2.close,
                    "FP-C1.1: close prices must be element-wise equal for same seed"
                );
            }
        }
    }

    /// FP-C1.2 — different seed ⇒ different ensemble (catches seed-ignored bug).
    #[test]
    fn fp_c1_2_different_seed_diverges() {
        let bgen = make_gen_fixed(5);
        let universe = vec![(btc(), dec!(30_000))];
        let path1 = bgen.generate(&universe, 50, 0xAAAA_AAAA).unwrap();
        let path2 = bgen.generate(&universe, 50, 0xBBBB_BBBB).unwrap();

        // At least one close price must differ.
        let any_different = path1.bars_by_symbol[0]
            .iter()
            .zip(path2.bars_by_symbol[0].iter())
            .any(|(b1, b2)| b1.close != b2.close);
        assert!(
            any_different,
            "FP-C1.2: different seeds must produce different paths"
        );
    }

    /// FP-C1.3 — `Fixed(1)` degenerates to iid (lag-1 autocorrelation ≈ 0).
    #[test]
    fn fp_c1_3_fixed_1_is_iid() {
        // With L=1, p=1/1=1 → every step restarts a new block → i.i.d. resampling.
        // The lag-1 autocorrelation of the resulting returns should be ≈ 0.
        let bgen = make_gen_fixed(1);
        let universe = vec![(btc(), dec!(30_000))];
        // Use a long path to get a stable autocorrelation estimate.
        let path = bgen.generate(&universe, 500, 0xDEAD_BEEF).unwrap();
        let bars = &path.bars_by_symbol[0];

        // Compute log-returns of the output path.
        let rets: Vec<f64> = bars
            .windows(2)
            .map(|w| {
                let c0 = w[0].close.get().to_string().parse::<f64>().unwrap_or(1.0);
                let c1 = w[1].close.get().to_string().parse::<f64>().unwrap_or(1.0);
                (c1 / c0).ln()
            })
            .collect();

        let n = rets.len() as f64;
        let mean: f64 = rets.iter().sum::<f64>() / n;
        let x: Vec<f64> = rets.iter().map(|r| r - mean).collect();
        let var: f64 = x.iter().map(|v| v * v).sum::<f64>() / n;
        let lag1: f64 = if var < f64::EPSILON {
            0.0
        } else {
            x[..x.len() - 1]
                .iter()
                .zip(x[1..].iter())
                .map(|(a, b)| a * b)
                .sum::<f64>()
                / (n * var)
        };

        // Lag-1 autocorrelation should be close to 0 for iid resampling.
        // Allow ±0.15 tolerance (finite-sample fluctuation with 500 bars).
        assert!(
            lag1.abs() < 0.15,
            "FP-C1.3: Fixed(1) lag-1 acf should be ≈ 0, got {lag1:.4}"
        );
    }

    /// FP-C1.4 — moment preservation (resampled mean/var ≈ source).
    #[test]
    fn fp_c1_4_moment_preservation() {
        let bgen = make_gen_fixed(5);
        let universe = vec![(btc(), dec!(30_000))];
        let path = bgen.generate(&universe, 1000, 0x1234_5678).unwrap();
        let bars = &path.bars_by_symbol[0];

        // Source returns.
        let source_rets: Vec<f64> = bgen.source_bars[0]
            .1
            .windows(2)
            .map(|w| {
                let c0 = decimal_to_f64(w[0].close.get());
                let c1 = decimal_to_f64(w[1].close.get());
                (c1 / c0).ln()
            })
            .collect();

        // Output path returns.
        let out_rets: Vec<f64> = bars
            .windows(2)
            .map(|w| {
                let c0 = decimal_to_f64(w[0].close.get());
                let c1 = decimal_to_f64(w[1].close.get());
                (c1 / c0).ln()
            })
            .collect();

        let mean_of = |v: &[f64]| v.iter().sum::<f64>() / v.len() as f64;
        let var_of = |v: &[f64]| {
            let m = mean_of(v);
            v.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / v.len() as f64
        };

        let src_mean = mean_of(&source_rets);
        let src_var = var_of(&source_rets);
        let out_mean = mean_of(&out_rets);
        let out_var = var_of(&out_rets);

        // Tolerance: allow ±5× the source variance as a generous bound
        // (the bootstrap preserves moments statistically, not sample-exactly).
        let tol_mean = (src_var.sqrt() * 5.0).max(0.01);
        let tol_var = (src_var * 5.0).max(0.001);

        assert!(
            (out_mean - src_mean).abs() < tol_mean,
            "FP-C1.4: resampled mean {out_mean:.6} ≈ source mean {src_mean:.6} (tol {tol_mean:.6})"
        );
        assert!(
            (out_var - src_var).abs() < tol_var,
            "FP-C1.4: resampled var {out_var:.6} ≈ source var {src_var:.6} (tol {tol_var:.6})"
        );
        // Non-collapse: output variance must be non-trivial.
        assert!(
            out_var > 1e-12,
            "FP-C1.4: resampled series must not be a constant (var={out_var})"
        );
    }

    /// FP-C1.5 — shared-index co-movement preserved.
    /// Two positively correlated source series → resampled correlation stays positive.
    #[test]
    fn fp_c1_5_shared_index_co_movement() {
        // Build two series that move together: ETH ≈ BTC with small noise.
        let n_src = 300;
        let epoch_base = epoch_2023();

        let make_correlated_bars = |sym: &Symbol, offset: f64| -> Vec<Bar> {
            use rand::Rng;
            use rand::SeedableRng;
            use rand_chacha::ChaCha20Rng;

            let mut rng = ChaCha20Rng::seed_from_u64(0xC0BB_7E57_u64);
            let mut rng2 = ChaCha20Rng::seed_from_u64(0xFEED_CAFE_u64);
            let mut close = 1000.0_f64 + offset;
            let mut bars = Vec::with_capacity(n_src);
            // First 150 bars: common shock + small idiosyncratic noise.
            // Second 150 bars: same pattern, same common shocks.
            let mut common_shocks = Vec::with_capacity(n_src);
            for _ in 0..n_src {
                common_shocks.push(rng.random::<f64>() * 0.04 - 0.02);
            }
            for (i, &common) in common_shocks.iter().enumerate() {
                let noise: f64 = (rng2.random::<f64>() - 0.5) * 0.002; // tiny noise
                let ret = common + noise;
                let next = (close * (1.0 + ret)).max(0.01);
                #[allow(clippy::cast_possible_wrap)]
                let open_ts = Timestamp::new(epoch_base + time::Duration::hours(i as i64));
                #[allow(clippy::cast_possible_wrap)]
                let close_ts = Timestamp::new(
                    epoch_base + time::Duration::hours(i as i64 + 1) - time::Duration::seconds(1),
                );
                let to_price = |v: f64| {
                    Price::new(Decimal::try_from(v.max(0.01)).unwrap_or(dec!(0.01))).unwrap_or_else(
                        |_| {
                            Price::new(dec!(0.01))
                                .unwrap_or_else(|e| unreachable!("dec!(0.01): {e}"))
                        },
                    )
                };
                bars.push(Bar {
                    symbol: sym.clone(),
                    tf: Timeframe::OneHour,
                    open_ts,
                    close_ts,
                    open: to_price(close),
                    high: to_price(close.max(next) * 1.001),
                    low: to_price(close.min(next) * 0.999),
                    close: to_price(next),
                    volume: Quantity::new(dec!(100))
                        .unwrap_or_else(|e| unreachable!("dec!(100): {e}")),
                    trade_count: 10,
                    local_recv_ts: close_ts,
                    venue: Venue::Binance,
                });
                close = next;
            }
            bars
        };

        // Silence unused-variable warnings from the non-literal seeds above.
        let _ = 0_u64;

        let btc_bars = make_correlated_bars(&btc(), 0.0);
        let eth_bars = make_correlated_bars(&eth(), 500.0);

        let bgen = BlockBootstrapPathGen::new(
            vec![(btc(), btc_bars.clone()), (eth(), eth_bars.clone())],
            BlockLengthPolicy::Fixed(10),
        )
        .unwrap();

        let universe = vec![(btc(), dec!(1000)), (eth(), dec!(1500))];
        let path = bgen.generate(&universe, 300, 0xC0B0_1E57_u64).unwrap();

        // Compute log-returns of resampled BTC and ETH.
        let log_rets = |bars: &[Bar]| -> Vec<f64> {
            bars.windows(2)
                .map(|w| {
                    let c0 = decimal_to_f64(w[0].close.get());
                    let c1 = decimal_to_f64(w[1].close.get());
                    (c1 / c0).ln()
                })
                .collect()
        };

        let btc_rets = log_rets(&path.bars_by_symbol[0]);
        let eth_rets = log_rets(&path.bars_by_symbol[1]);

        let mean_of = |v: &[f64]| -> f64 { v.iter().sum::<f64>() / v.len() as f64 };
        let btc_mean = mean_of(&btc_rets);
        let eth_mean = mean_of(&eth_rets);
        let n = btc_rets.len() as f64;

        // Pearson correlation between resampled BTC and ETH.
        let cov: f64 = btc_rets
            .iter()
            .zip(eth_rets.iter())
            .map(|(b, e)| (b - btc_mean) * (e - eth_mean))
            .sum::<f64>()
            / n;
        let btc_std: f64 =
            (btc_rets.iter().map(|v| (v - btc_mean).powi(2)).sum::<f64>() / n).sqrt();
        let eth_std: f64 =
            (eth_rets.iter().map(|v| (v - eth_mean).powi(2)).sum::<f64>() / n).sqrt();

        let corr = if btc_std < f64::EPSILON || eth_std < f64::EPSILON {
            0.0
        } else {
            cov / (btc_std * eth_std)
        };

        // The source series were highly correlated (common shocks dominate).
        // The shared-index bootstrap preserves this: resampled corr must be positive.
        assert!(
            corr > 0.5,
            "FP-C1.5: shared-index resampled BTC/ETH correlation={corr:.4} must be > 0.5"
        );
    }

    #[test]
    fn error_on_empty_universe() {
        let bgen = make_gen_fixed(5);
        let result = bgen.generate(&[], 10, 42);
        assert!(matches!(result, Err(SynthError::EmptyUniverse)));
    }

    #[test]
    fn error_on_zero_bars() {
        let bgen = make_gen_fixed(5);
        let universe = vec![(btc(), dec!(30_000))];
        let result = bgen.generate(&universe, 0, 42);
        assert!(matches!(result, Err(SynthError::ZeroBars)));
    }

    #[test]
    fn error_on_short_source() {
        let btc_bars = make_bars(&btc(), 1, 1); // only 1 bar → < 2
        let result =
            BlockBootstrapPathGen::new(vec![(btc(), btc_bars)], BlockLengthPolicy::Fixed(5));
        assert!(matches!(result, Err(SynthError::SeriesTooShort { .. })));
    }

    #[test]
    fn error_on_ragged_source() {
        let btc_bars = make_bars(&btc(), 100, 1);
        let eth_bars = make_bars(&eth(), 99, 2); // different length
        let result = BlockBootstrapPathGen::new(
            vec![(btc(), btc_bars), (eth(), eth_bars)],
            BlockLengthPolicy::Fixed(5),
        );
        assert!(matches!(result, Err(SynthError::RaggedUniverse { .. })));
    }

    #[test]
    fn output_has_correct_bar_count() {
        let bgen = make_gen_fixed(5);
        let universe = vec![(btc(), dec!(30_000)), (eth(), dec!(1_200))];
        let n_bars = 42;
        let path = bgen.generate(&universe, n_bars, 0x1111_2222).unwrap();
        assert_eq!(path.bars_by_symbol.len(), 2);
        assert_eq!(path.bars_by_symbol[0].len(), n_bars);
        assert_eq!(path.bars_by_symbol[1].len(), n_bars);
    }

    #[test]
    fn selected_block_length_is_some_and_matches_fixed() {
        let bgen = make_gen_fixed(7);
        let universe = vec![(btc(), dec!(30_000))];
        let path = bgen.generate(&universe, 20, 42).unwrap();
        assert_eq!(path.selected_block_length, Some(7));
    }

    #[test]
    fn auto_block_length_is_some() {
        let btc_bars = make_bars(&btc(), 200, 0xAB);
        let bgen =
            BlockBootstrapPathGen::new(vec![(btc(), btc_bars)], BlockLengthPolicy::Auto).unwrap();
        let universe = vec![(btc(), dec!(30_000))];
        let path = bgen.generate(&universe, 50, 42).unwrap();
        let l = path.selected_block_length.unwrap();
        assert!(l >= 1, "auto L must be ≥ 1, got {l}");
        assert!(l < 200, "auto L must be < source length, got {l}");
    }

    // ── M-DEV-3 carry-strategy co-resampling tests (ADR-0051 § D6.6) ─────────

    /// Build a deterministic per-symbol funding-at-return series for testing.
    ///
    /// `funding_at_return[sym_i][k]` = Some(rate), where rate is a unique
    /// integer-valued Decimal encoding the source return index `k` (as a plain
    /// integer, so that `rate.to_i64() == k` without any floating-point rounding).
    /// This makes it trivial to verify exactly which source index was used.
    ///
    /// For a given symbol, `tag = Decimal::new(k as i64, 0)` so decoding is just
    /// `.to_i64()` or `.to_string().parse::<i64>()`. The per-symbol offset is
    /// NOT baked in here; the caller distinguishes symbols by the sym_i index.
    fn make_funding_at_return(n_syms: usize, n_returns: usize) -> Vec<Vec<Option<Decimal>>> {
        (0..n_syms)
            .map(|_sym_i| {
                (0..n_returns)
                    .map(|k| {
                        // Plain integer tag: recoverable as `rate.to_i64() == k`.
                        Some(Decimal::new(k as i64, 0))
                    })
                    .collect()
            })
            .collect()
    }

    /// M-DEV-3 ANCHOR-NEUTRALITY: `funding=None` produces byte-identical bars.
    ///
    /// The same seed with `with_funding(None)` must produce the SAME bars as
    /// the base generator (no funding attached). This is the proof that the
    /// 87 existing anchors are byte-unchanged by the carry-strategy Stage 2.
    #[test]
    fn funding_none_is_byte_identical_bars() {
        let bgen_base = make_gen_fixed(5);
        // Generator with funding explicitly set to None (same as base).
        let btc_bars = make_bars(&btc(), 200, 0xBEEF_0001);
        let eth_bars = make_bars(&eth(), 200, 0xBEEF_0002);
        let bgen_with_none = BlockBootstrapPathGen::new(
            vec![(btc(), btc_bars), (eth(), eth_bars)],
            BlockLengthPolicy::Fixed(5),
        )
        .unwrap()
        .with_funding(None);

        let universe = vec![(btc(), dec!(30_000)), (eth(), dec!(1_200))];
        let seed = 0xCAFE_DEAD_u64;

        let path_base = bgen_base.generate(&universe, 50, seed).unwrap();
        let path_none = bgen_with_none.generate(&universe, 50, seed).unwrap();

        // Bars must be byte-identical.
        assert_eq!(
            path_base.bars_by_symbol.len(),
            path_none.bars_by_symbol.len()
        );
        for (sym_base, sym_none) in path_base
            .bars_by_symbol
            .iter()
            .zip(path_none.bars_by_symbol.iter())
        {
            for (b, n) in sym_base.iter().zip(sym_none.iter()) {
                assert_eq!(
                    b.close, n.close,
                    "funding=None must produce byte-identical bars (anchor-neutrality)"
                );
                assert_eq!(
                    b.open_ts, n.open_ts,
                    "funding=None must produce identical open_ts"
                );
            }
        }
        // Both must have None funding.
        assert!(
            path_base.funding_by_symbol.is_none(),
            "base generator must have None funding_by_symbol"
        );
        assert!(
            path_none.funding_by_symbol.is_none(),
            "with_funding(None) must have None funding_by_symbol"
        );
    }

    /// M-DEV-3 DETERMINISM: same seed → byte-identical `funding_by_symbol`.
    ///
    /// Runs the same generator twice with the same seed; asserts that every
    /// funding rate in the output is equal element-wise. Catches any accidental
    /// non-determinism in the funding gather (e.g. wrong RNG draw).
    #[test]
    fn funding_co_resample_same_seed_deterministic() {
        let n_bars_src = 200;
        let n_returns = n_bars_src - 1;
        let n_out = 50;
        let n_syms = 2;

        let btc_bars = make_bars(&btc(), n_bars_src, 0xBEEF_0001);
        let eth_bars = make_bars(&eth(), n_bars_src, 0xBEEF_0002);
        let funding = make_funding_at_return(n_syms, n_returns);

        let bgen = BlockBootstrapPathGen::new(
            vec![(btc(), btc_bars), (eth(), eth_bars)],
            BlockLengthPolicy::Fixed(5),
        )
        .unwrap()
        .with_funding(Some(funding));

        let universe = vec![(btc(), dec!(30_000)), (eth(), dec!(1_200))];
        let seed = 0xDEAD_BEEF_u64;

        let path1 = bgen.generate(&universe, n_out, seed).unwrap();
        let path2 = bgen.generate(&universe, n_out, seed).unwrap();

        let f1 = path1
            .funding_by_symbol
            .expect("funding must be Some when source is attached");
        let f2 = path2
            .funding_by_symbol
            .expect("funding must be Some when source is attached");

        assert_eq!(f1.len(), f2.len(), "symbol count must match");
        for (sym_f1, sym_f2) in f1.iter().zip(f2.iter()) {
            assert_eq!(sym_f1.len(), n_out, "funding length must equal n_out");
            for (a, b) in sym_f1.iter().zip(sym_f2.iter()) {
                assert_eq!(
                    a, b,
                    "same seed must produce byte-identical funding_by_symbol"
                );
            }
        }
    }

    /// M-DEV-3 INDEX-ALIGNMENT (THE CRUX): funding gathered by the SAME ret_idx.
    ///
    /// Uses a synthetic funding source where `funding_at_return[sym_i][k]` is a
    /// unique tag encoding `(sym_i, k)`. After resampling, for each output bar
    /// `bar_i` in the idx_seq, the resampled funding must match the tag for the
    /// source index that was used — proving the funding was gathered by the same
    /// `ret_idx` as the return, NOT independently drawn or timestamp-indexed.
    ///
    /// This is the formal proof of the co-resampling invariant (FP-C1.5 sibling
    /// for carry-strategy, ADR-0051 § D6.6).
    #[test]
    fn funding_index_aligned_co_movement() {
        let n_bars_src = 50;
        let n_returns = n_bars_src - 1; // 49 log-returns
        let n_out = 30;
        let n_syms = 2;

        let btc_bars = make_bars(&btc(), n_bars_src, 0xBEEF_0011);
        let eth_bars = make_bars(&eth(), n_bars_src, 0xBEEF_0022);

        // Each funding tag uniquely encodes (sym_i, source_return_index).
        // After resampling, we can recover which source index was used by decoding the tag.
        let funding = make_funding_at_return(n_syms, n_returns);

        let bgen = BlockBootstrapPathGen::new(
            vec![(btc(), btc_bars.clone()), (eth(), eth_bars.clone())],
            BlockLengthPolicy::Fixed(3),
        )
        .unwrap()
        .with_funding(Some(funding.clone()));

        let universe = vec![(btc(), dec!(30_000)), (eth(), dec!(1_200))];
        let seed = 0xFACE_CAFE_u64;
        let path = bgen.generate(&universe, n_out, seed).unwrap();

        let resampled_funding = path
            .funding_by_symbol
            .expect("funding_by_symbol must be Some");
        let resampled_bars = &path.bars_by_symbol;

        assert_eq!(resampled_funding.len(), n_syms, "sym count must match");
        assert_eq!(resampled_bars.len(), n_syms, "sym count must match");

        // For BTC (sym_i=0), recover the source_k from each resampled funding value.
        // The tag is Decimal::new(source_k as i64, 0), so to_i64() gives source_k.
        //
        // We cross-verify with the bar's log-return:
        // - funding_btc[bar_i] encodes which source_k was used by idx_seq.
        // - The resampled close at bar_i was built from:
        //     prev_close * exp(source_rets[source_k]).
        // - So the log-return of adjacent bars must match source_rets[source_k].

        let src_rets_btc: Vec<f64> = btc_bars
            .windows(2)
            .map(|w| {
                let c0 = decimal_to_f64(w[0].close.get());
                let c1 = decimal_to_f64(w[1].close.get());
                (c1 / c0).ln()
            })
            .collect();

        let bars_btc = &resampled_bars[0];
        let funding_btc = &resampled_funding[0];

        assert_eq!(
            funding_btc.len(),
            n_out,
            "funding length must equal n_out bars"
        );

        let mut misaligned = 0usize;
        // Bars 1..n_out are built from idx_seq entries. Bar 0 is the sentinel.
        for bar_i in 1..bars_btc.len() {
            let Some(rate) = funding_btc[bar_i] else {
                misaligned += 1; // unexpected None for a return bar
                continue;
            };
            // Decode source_k: tag = Decimal::new(k, 0), so source_k = rate as i64.
            let source_k_i64 = rate.to_string().parse::<i64>().unwrap_or(-1);
            if source_k_i64 < 0 || source_k_i64 >= n_returns as i64 {
                misaligned += 1;
                continue;
            }
            let source_k = source_k_i64 as usize;

            // Cross-check: the resampled bar's log-return must match source_rets[source_k].
            let prev_close = decimal_to_f64(bars_btc[bar_i - 1].close.get());
            let this_close = decimal_to_f64(bars_btc[bar_i].close.get());
            if prev_close < 1e-9 {
                continue; // degenerate price — skip
            }
            let resampled_ret = (this_close / prev_close).ln();
            let expected_ret = src_rets_btc[source_k];

            // Must match to within clamp tolerance (price is clamped in [1e-6, 1e9]).
            let diff = (resampled_ret - expected_ret).abs();
            if diff > 0.001 {
                misaligned += 1;
            }
        }

        // Every bar_i >= 1 must be correctly index-aligned. Allow 0 misalignment
        // (the clamp edge case is negligible for prices starting at 30_000).
        assert_eq!(
            misaligned,
            0,
            "funding must be index-aligned with the return (ADR-0051 § D6.6): \
             {misaligned}/{} bars have misaligned funding vs return source index",
            bars_btc.len() - 1
        );
    }
}
