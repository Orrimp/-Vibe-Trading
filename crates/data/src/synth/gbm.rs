//! GBM (Geometric Brownian Motion) smoke-test path generator.
//!
//! A **new, independent** GBM ensemble generator for the `MonteCarloPathGen`
//! trait. This is the "demoted smoke-test" role described in the feature brief
//! (D-C1.5 / Q-MCB-3 = thin-wrap + defer):
//!
//! - It is **anchor-free** — it does NOT feed any of the 84 existing anchors
//!   and is never the headline robustness verdict source.
//! - It does NOT touch / move / re-route `momentum.rs::synthetic_bars_hourly`,
//!   `main.rs::synthetic_bars`, or `tests/determinism.rs::synthetic_bars_det`.
//!   The 3-copy dedup is a v0.2.0 carve-out (D-C1.5).
//! - The shape (Box-Muller + intrabar + volume + trade_count) mirrors
//!   `synthetic_bars_hourly` as a starting point but is a fresh, independent
//!   implementation parameterised for the harness's needs via the trait.
//!
//! ## Determinism
//!
//! `generate` is a pure function of `(self, universe, n_bars, path_seed)`.
//! One `ChaCha20Rng` per SYMBOL, seeded via `derive_sym_seed(path_seed, sym_i)`
//! (SplitMix64 mixing — see the fn doc for the anti-diagonal collision the
//! old additive derivation had); draw order is:
//! per bar: u1, u2 (Box-Muller) → noise1, noise2 (intrabar) → vol → trade_count.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

use crate::synth::{GeneratedPath, MonteCarloPathGen, SynthError};

// ── Parameters ────────────────────────────────────────────────────────────────

/// Per-hour GBM parameters for the smoke-test generator.
/// These mirror the `synthetic_bars_hourly` parameter regime so the GBM
/// baseline is a faithful smoke-test representative.
#[derive(Debug, Clone)]
pub struct GbmParams {
    /// Per-bar volatility (annualised vol / sqrt(bars_per_year)).
    /// Default: 0.012 (≈ hourly vol for typical crypto).
    pub per_bar_vol: f64,
    /// Per-bar drift.
    /// Default: 0.000_03.
    pub per_bar_drift: f64,
    /// Intrabar noise scale relative to close price.
    /// Default: 0.002.
    pub intrabar_scale: f64,
    /// Volume distribution: `rng * vol_range + vol_min`.
    /// Defaults: range 500, min 10.
    pub vol_range: f64,
    pub vol_min: f64,
    /// Trade-count range: `rng.random_range(tc_lo..tc_hi)`.
    /// Defaults: 100..5000.
    pub tc_lo: u32,
    pub tc_hi: u32,
    /// Price clamp bounds.
    pub price_lo: f64,
    pub price_hi: f64,
}

impl Default for GbmParams {
    fn default() -> Self {
        Self {
            per_bar_vol: 0.012,
            per_bar_drift: 0.000_03,
            intrabar_scale: 0.002,
            vol_range: 500.0,
            vol_min: 10.0,
            tc_lo: 100,
            tc_hi: 5000,
            price_lo: 0.01,
            price_hi: 10_000_000.0,
        }
    }
}

impl GbmParams {
    /// Validate the parameter set.
    ///
    /// Called by [`GbmPathGen`]'s `generate` before any path work (the struct
    /// has public fields + `Default`, so construction itself cannot gate).
    /// Rejects what previously failed silently or loudly-wrong:
    /// - non-finite `f64` fields — NaN vol/drift used to flatline every bar
    ///   to `price_lo` silently, and NaN clamp bounds PANIC in `f64::clamp`;
    /// - inverted clamp bounds (`price_lo > price_hi`) — also a
    ///   `f64::clamp` PANIC;
    /// - non-positive `price_lo` — prices must stay strictly positive
    ///   (`Price::new` rejects `d <= 0`).
    ///
    /// # Errors
    ///
    /// Returns [`SynthError::InvalidGbmParams`] naming the offending field.
    pub fn validate(&self) -> Result<(), SynthError> {
        let finite_fields = [
            ("per_bar_vol", self.per_bar_vol),
            ("per_bar_drift", self.per_bar_drift),
            ("intrabar_scale", self.intrabar_scale),
            ("vol_range", self.vol_range),
            ("vol_min", self.vol_min),
            ("price_lo", self.price_lo),
            ("price_hi", self.price_hi),
        ];
        for (name, value) in finite_fields {
            if !value.is_finite() {
                return Err(SynthError::InvalidGbmParams {
                    reason: format!("{name} must be finite, got {value}"),
                });
            }
        }
        if self.price_lo <= 0.0 {
            return Err(SynthError::InvalidGbmParams {
                reason: format!("price_lo must be > 0, got {}", self.price_lo),
            });
        }
        if self.price_lo > self.price_hi {
            return Err(SynthError::InvalidGbmParams {
                reason: format!(
                    "inverted price clamp bounds: price_lo {} > price_hi {}",
                    self.price_lo, self.price_hi
                ),
            });
        }
        Ok(())
    }
}

// ── Generator ─────────────────────────────────────────────────────────────────

/// GBM smoke-test path generator.
///
/// Produces a GBM ensemble for use in the N-path harness smoke-test role.
/// The headline generator is [`crate::synth::bootstrap::BlockBootstrapPathGen`].
///
/// Per-symbol seeds are derived from `(path_seed, sym_i)` via SplitMix64
/// mixing (see [`derive_sym_seed`]). The previous additive derivation
/// (`path_seed + sym_i · 0x9E37_79B9`) collided on every anti-diagonal with
/// the ADR-0051 D1 consumer path-seed rule
/// (`path_seed_j = master + j · 0x9E37_79B9`): `seed(j, i) == seed(j', i')`
/// whenever `i + j == i' + j'`, so e.g. ETH-on-path-0 replayed BTC-on-path-1
/// bit-for-bit. ANCHOR-SAFE change: no anchored report body derives from
/// `GbmPathGen` (smoke-test role only — verified at review 1-13).
///
/// `selected_block_length` is always `None` — GBM has no block-length concept.
#[derive(Debug, Clone, Default)]
pub struct GbmPathGen {
    /// GBM parameters.
    pub params: GbmParams,
}

/// SplitMix64 finalizer (Steele–Lea–Flood 2014; the `splitmix64` reference
/// generator's output function). A bijective avalanche mixer on `u64`.
#[inline]
#[must_use]
fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Derive the per-symbol ChaCha20 seed from `(path_seed, sym_i)`.
///
/// `splitmix64(splitmix64(path_seed) + sym_i)`: the outer mix breaks the
/// linear relation between neighbouring `sym_i`, and the inner mix breaks
/// the linear relation between neighbouring ADR-0051 D1 path seeds — so no
/// `(path, symbol)` anti-diagonal pair can collide structurally (a collision
/// would require `splitmix64(ps') - splitmix64(ps) == Δi`, which is not an
/// identity for any seed family, unlike the previous additive scheme where
/// it held for EVERY master seed).
#[inline]
#[must_use]
fn derive_sym_seed(path_seed: u64, sym_i: usize) -> u64 {
    splitmix64(splitmix64(path_seed).wrapping_add(sym_i as u64))
}

impl GbmPathGen {
    /// Create a new `GbmPathGen` with default parameters.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a new `GbmPathGen` with custom parameters.
    #[must_use]
    pub fn with_params(params: GbmParams) -> Self {
        Self { params }
    }
}

// ── Trait impl ────────────────────────────────────────────────────────────────

// Float arithmetic for GBM price simulation (ADR-0003: f64 in return space,
// Decimal at Bar boundary).
#[allow(clippy::float_arithmetic)]
impl MonteCarloPathGen for GbmPathGen {
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
        // Bound n_bars (time-crate year-9999 overflow / alloc abort guard —
        // see `MAX_N_BARS`).
        if n_bars > crate::synth::MAX_N_BARS {
            return Err(SynthError::TooManyBars {
                n_bars,
                max: crate::synth::MAX_N_BARS,
            });
        }
        // Validate params before any path work (NaN/inverted clamp bounds
        // would PANIC in f64::clamp; NaN vol/drift silently flatlined).
        self.params.validate()?;
        // Start prices must be strictly positive — previously silently
        // clamped to `price_lo`, producing plausible-shaped garbage.
        for (symbol, start_price) in universe {
            if *start_price <= Decimal::ZERO {
                return Err(SynthError::NonPositiveStartPrice {
                    symbol: symbol.to_string(),
                    start_price: *start_price,
                });
            }
        }

        let epoch_base = epoch_2023();

        let p = &self.params;
        let to_dec =
            |v: f64| -> Decimal { Decimal::try_from(v.max(p.price_lo)).unwrap_or(dec!(0.01)) };
        let price_or_min = |v: f64| -> Price {
            Price::new(to_dec(v)).unwrap_or_else(|_| {
                Price::new(dec!(0.01)).unwrap_or_else(|e| unreachable!("dec!(0.01) valid: {e}"))
            })
        };

        let mut bars_by_symbol: Vec<Vec<Bar>> = Vec::with_capacity(universe.len());

        for (sym_i, (symbol, start_price)) in universe.iter().enumerate() {
            // Per-symbol seed: SplitMix64 mix over (path_seed, sym_i) — see
            // `derive_sym_seed` for why the old additive idiom collided on
            // anti-diagonals with the ADR-0051 D1 path-seed rule.
            let sym_seed = derive_sym_seed(path_seed, sym_i);
            let mut rng = ChaCha20Rng::seed_from_u64(sym_seed);

            // start_price > 0 was validated above; a positive Decimal's
            // string form always parses to a positive finite f64 (the
            // fallback is an unreachable non-panicking last resort). The
            // `.max(price_lo)` only lifts sub-price_lo positive starts into
            // the clamp band (documented GbmParams behaviour).
            let mut close: f64 = start_price
                .to_string()
                .parse::<f64>()
                .unwrap_or(30_000.0_f64)
                .max(p.price_lo);

            let mut sym_bars: Vec<Bar> = Vec::with_capacity(n_bars);

            for i in 0..n_bars {
                // Box-Muller transform (same as synthetic_bars_hourly draw order).
                let u1: f64 = rng.random::<f64>().max(1e-10_f64);
                let u2: f64 = rng.random::<f64>();
                let z = (-2.0_f64 * u1.ln()).sqrt() * (2.0_f64 * std::f64::consts::PI * u2).cos();
                let ret = p.per_bar_drift + p.per_bar_vol * z;
                let next = (close * (1.0 + ret)).clamp(p.price_lo, p.price_hi);

                // Intrabar noise.
                let intra_vol = close * p.intrabar_scale;
                let noise1: f64 = rng.random::<f64>() * intra_vol;
                let noise2: f64 = rng.random::<f64>() * intra_vol;

                let open = close;
                let high = (open.max(next) + noise1).clamp(p.price_lo, p.price_hi);
                let low = (open.min(next) - noise2).max(p.price_lo);

                // Volume draw.
                let vol: f64 = rng.random::<f64>() * p.vol_range + p.vol_min;

                // Trade count.
                let tc: u32 = if p.tc_lo < p.tc_hi {
                    rng.random_range(p.tc_lo..p.tc_hi)
                } else {
                    p.tc_lo
                };

                #[allow(clippy::cast_possible_wrap)]
                let open_ts = Timestamp::new(epoch_base + time::Duration::hours(i as i64));
                #[allow(clippy::cast_possible_wrap)]
                let close_ts = Timestamp::new(
                    epoch_base + time::Duration::hours(i as i64 + 1) - time::Duration::seconds(1),
                );

                sym_bars.push(Bar {
                    symbol: symbol.clone(),
                    tf: Timeframe::OneHour,
                    open_ts,
                    close_ts,
                    open: price_or_min(open),
                    high: price_or_min(high.max(open).max(next)),
                    low: price_or_min(low.min(open).min(next).max(p.price_lo)),
                    close: price_or_min(next),
                    volume: Quantity::new(to_dec(vol.max(1.0))).unwrap_or_else(|_| {
                        Quantity::new(dec!(1))
                            .unwrap_or_else(|e| unreachable!("dec!(1) valid: {e}"))
                    }),
                    trade_count: tc,
                    local_recv_ts: close_ts,
                    venue: Venue::Binance,
                });

                close = next;
            }

            bars_by_symbol.push(sym_bars);
        }

        Ok(GeneratedPath {
            bars_by_symbol,
            // GBM has no block-length concept.
            selected_block_length: None,
            // GBM does not support carry-funding co-resampling.
            funding_by_symbol: None,
            // GBM does not support basis co-resampling (MN-spread M-DEV-1).
            basis_by_symbol: None,
        })
    }
}

// ── Private helpers ───────────────────────────────────────────────────────────

fn epoch_2023() -> OffsetDateTime {
    let date = time::Date::from_calendar_date(2023, time::Month::January, 1)
        .unwrap_or_else(|e| unreachable!("2023-01-01 is always valid: {e}"));
    OffsetDateTime::new_utc(date, time::Time::MIDNIGHT)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use trading_core::Symbol;

    fn btc() -> Symbol {
        Symbol::new("BTCUSDT")
    }

    /// GBM determinism: same seed → same bars.
    #[test]
    fn gbm_same_seed_deterministic() {
        let ggen = GbmPathGen::new();
        let universe = vec![(btc(), dec!(30_000))];
        let p1 = ggen.generate(&universe, 30, 0xDEAD_CAFE_u64).unwrap();
        let p2 = ggen.generate(&universe, 30, 0xDEAD_CAFE_u64).unwrap();
        assert_eq!(p1.bars_by_symbol[0].len(), p2.bars_by_symbol[0].len());
        for (b1, b2) in p1.bars_by_symbol[0].iter().zip(p2.bars_by_symbol[0].iter()) {
            assert_eq!(
                b1.close, b2.close,
                "GBM same-seed must produce identical close prices"
            );
        }
    }

    /// GBM: different seed → different bars.
    #[test]
    fn gbm_different_seed_diverges() {
        let ggen = GbmPathGen::new();
        let universe = vec![(btc(), dec!(30_000))];
        let p1 = ggen.generate(&universe, 30, 0x1111_1111_u64).unwrap();
        let p2 = ggen.generate(&universe, 30, 0x2222_2222_u64).unwrap();
        let any_diff = p1.bars_by_symbol[0]
            .iter()
            .zip(p2.bars_by_symbol[0].iter())
            .any(|(b1, b2)| b1.close != b2.close);
        assert!(any_diff, "GBM different seeds must produce different paths");
    }

    /// GBM: selected_block_length is None.
    #[test]
    fn gbm_selected_block_length_none() {
        let ggen = GbmPathGen::new();
        let universe = vec![(btc(), dec!(30_000))];
        let path = ggen.generate(&universe, 10, 42).unwrap();
        assert!(
            path.selected_block_length.is_none(),
            "GbmPathGen must return None for selected_block_length"
        );
    }

    #[test]
    fn gbm_correct_bar_count() {
        let ggen = GbmPathGen::new();
        let universe = vec![(btc(), dec!(30_000)), (Symbol::new("ETHUSDT"), dec!(1_200))];
        let path = ggen.generate(&universe, 100, 99).unwrap();
        assert_eq!(path.bars_by_symbol.len(), 2);
        assert_eq!(path.bars_by_symbol[0].len(), 100);
        assert_eq!(path.bars_by_symbol[1].len(), 100);
    }

    #[test]
    fn gbm_error_on_empty_universe() {
        let ggen = GbmPathGen::new();
        assert!(matches!(
            ggen.generate(&[], 10, 0),
            Err(SynthError::EmptyUniverse)
        ));
    }

    #[test]
    fn gbm_error_on_zero_bars() {
        let ggen = GbmPathGen::new();
        let universe = vec![(btc(), dec!(30_000))];
        assert!(matches!(
            ggen.generate(&universe, 0, 0),
            Err(SynthError::ZeroBars)
        ));
    }

    // ── Review 1-13 patches ──────────────────────────────────────────────────

    /// Anti-diagonal seed-collision regression proof (review 1-13).
    ///
    /// Under the ADR-0051 D1 consumer rule
    /// `path_seed_j = master + j·0x9E37_79B9`, the OLD additive per-symbol
    /// derivation `sym_seed = path_seed + sym_i·0x9E37_79B9` made
    /// `seed(path 0, sym 1) == seed(path 1, sym 0)` for EVERY master seed —
    /// symbol 1 of path 0 replayed symbol 0 of path 1 bit-for-bit (both
    /// symbols get the same start price here so the collision would surface
    /// as identical closes). The SplitMix64 derivation must make the two
    /// streams differ.
    #[test]
    fn gbm_no_anti_diagonal_seed_collision_across_paths() {
        const GOLDEN_GAMMA: u64 = 0x9E37_79B9; // ADR-0051 D1 path-seed step
        let ggen = GbmPathGen::new();
        // SAME start price for both symbols: any seed collision ⇒ identical bars.
        let universe = vec![
            (btc(), dec!(30_000)),
            (Symbol::new("ETHUSDT"), dec!(30_000)),
        ];

        let master = 0xA11C_E5EE_u64;
        let path0 = ggen.generate(&universe, 50, master).unwrap(); // j = 0
        let path1 = ggen
            .generate(&universe, 50, master.wrapping_add(GOLDEN_GAMMA)) // j = 1
            .unwrap();

        // seed(j=0, i=1) vs seed(j=1, i=0): the generated return streams must differ.
        let any_diff = path0.bars_by_symbol[1]
            .iter()
            .zip(path1.bars_by_symbol[0].iter())
            .any(|(a, b)| a.close != b.close);
        assert!(
            any_diff,
            "anti-diagonal collision: (path 0, sym 1) and (path 1, sym 0) \
             produced identical close streams — per-symbol seed derivation \
             must mix (path_seed, sym_i) non-additively"
        );
    }

    /// Inverted clamp bounds previously PANICKED inside `f64::clamp`;
    /// now a typed error.
    #[test]
    fn gbm_error_on_inverted_price_bounds() {
        let params = GbmParams {
            price_lo: 100.0,
            price_hi: 1.0, // inverted
            ..GbmParams::default()
        };
        let ggen = GbmPathGen::with_params(params);
        let universe = vec![(btc(), dec!(30_000))];
        assert!(matches!(
            ggen.generate(&universe, 10, 42),
            Err(SynthError::InvalidGbmParams { .. })
        ));
    }

    /// NaN vol/drift previously flatlined every bar to `price_lo` silently;
    /// NaN clamp bounds previously PANICKED. Both are typed errors now.
    #[test]
    fn gbm_error_on_non_finite_params() {
        for params in [
            GbmParams {
                per_bar_vol: f64::NAN,
                ..GbmParams::default()
            },
            GbmParams {
                per_bar_drift: f64::INFINITY,
                ..GbmParams::default()
            },
            GbmParams {
                price_lo: f64::NAN,
                ..GbmParams::default()
            },
        ] {
            let ggen = GbmPathGen::with_params(params);
            let universe = vec![(btc(), dec!(30_000))];
            assert!(matches!(
                ggen.generate(&universe, 10, 42),
                Err(SynthError::InvalidGbmParams { .. })
            ));
        }
    }

    #[test]
    fn gbm_error_on_non_positive_price_lo() {
        let params = GbmParams {
            price_lo: 0.0,
            ..GbmParams::default()
        };
        let ggen = GbmPathGen::with_params(params);
        let universe = vec![(btc(), dec!(30_000))];
        assert!(matches!(
            ggen.generate(&universe, 10, 42),
            Err(SynthError::InvalidGbmParams { .. })
        ));
    }

    /// Non-positive start price previously silently clamped to `price_lo`.
    #[test]
    fn gbm_error_on_non_positive_start_price() {
        let ggen = GbmPathGen::new();
        for bad_start in [dec!(0), dec!(-42)] {
            let universe = vec![(btc(), bad_start)];
            assert!(matches!(
                ggen.generate(&universe, 10, 42),
                Err(SynthError::NonPositiveStartPrice { .. })
            ));
        }
    }

    #[test]
    fn gbm_error_on_too_many_bars() {
        let ggen = GbmPathGen::new();
        let universe = vec![(btc(), dec!(30_000))];
        // Fires before any allocation.
        let result = ggen.generate(&universe, crate::synth::MAX_N_BARS + 1, 42);
        assert!(matches!(result, Err(SynthError::TooManyBars { .. })));
    }
}
