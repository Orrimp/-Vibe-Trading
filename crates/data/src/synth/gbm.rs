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
//! One `ChaCha20Rng::seed_from_u64(path_seed)` per call; draw order is:
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

// ── Generator ─────────────────────────────────────────────────────────────────

/// GBM smoke-test path generator.
///
/// Produces a GBM ensemble for use in the N-path harness smoke-test role.
/// The headline generator is [`crate::synth::bootstrap::BlockBootstrapPathGen`].
///
/// Per-symbol seeds are derived from `path_seed` via the same constant
/// `0x9E37_79B9` the project uses on the symbol axis (ADR-0051 D1 / momentum.rs:245):
/// `sym_seed_i = path_seed.wrapping_add((i as u64).wrapping_mul(0x9E37_79B9))`.
///
/// `selected_block_length` is always `None` — GBM has no block-length concept.
#[derive(Debug, Clone, Default)]
pub struct GbmPathGen {
    /// GBM parameters.
    pub params: GbmParams,
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
            // Per-symbol seed derived from path_seed (same idiom as momentum.rs:245).
            #[allow(clippy::cast_possible_truncation)]
            let sym_seed = path_seed.wrapping_add((sym_i as u64).wrapping_mul(0x9E37_79B9_u64));
            let mut rng = ChaCha20Rng::seed_from_u64(sym_seed);

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
}
