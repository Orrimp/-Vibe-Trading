//! Exponential Moving Average — streaming and batch adapters (T502).
//!
//! Pure `Decimal` arithmetic, no TA-lib dependency.  Consistent with the
//! `features::sma` precedent from v0 T21.
//!
//! **EMA formula:**
//!   α = 2 / (period + 1)
//!   EMA_t = α × close_t + (1 − α) × EMA_{t-1}
//!
//! Seeded with the SMA of the first `period` bars (standard convention).

use rust_decimal::Decimal;

// ── Streaming ──────────────────────────────────────────────────────────────────

/// Streaming EMA.
///
/// Returns `Some(ema)` once the `period`-bar seed window is complete.
pub struct EmaStream {
    period: u32,
    alpha: Decimal,
    /// Accumulator for the seed-SMA window.
    seed_sum: Decimal,
    seed_count: u32,
    latest: Option<Decimal>,
}

impl EmaStream {
    /// Create a new streaming EMA with the given `period`.
    ///
    /// # Panics
    ///
    /// Panics if `period == 0`.
    #[must_use]
    pub fn new(period: u32) -> Self {
        assert!(period > 0, "EMA period must be > 0");
        let alpha = Decimal::from(2) / Decimal::from(period + 1);
        Self {
            period,
            alpha,
            seed_sum: Decimal::ZERO,
            seed_count: 0,
            latest: None,
        }
    }

    /// EMA smoothing factor α = 2/(period+1).
    #[must_use]
    pub fn alpha(&self) -> Decimal {
        self.alpha
    }

    /// Push a new value; returns `Some(ema)` once the seed window is complete.
    pub fn push(&mut self, value: Decimal) -> Option<Decimal> {
        if self.seed_count < self.period {
            self.seed_sum += value;
            self.seed_count += 1;
            if self.seed_count == self.period {
                // Seed with SMA of first `period` bars.
                let seed = self.seed_sum / Decimal::from(self.period);
                self.latest = Some(seed);
            }
            return self.latest;
        }
        // Recursive EMA.
        let prev = self.latest.unwrap_or(value);
        let ema = self.alpha * value + (Decimal::ONE - self.alpha) * prev;
        self.latest = Some(ema);
        self.latest
    }

    /// Latest computed EMA value (None until seed window is complete).
    #[must_use]
    pub fn latest(&self) -> Option<Decimal> {
        self.latest
    }
}

// ── Batch ─────────────────────────────────────────────────────────────────────

/// Batch EMA — computes the last EMA value over the full `prices` slice.
///
/// Seeded with the SMA of the first `period` bars.
pub struct EmaBatch {
    period: u32,
}

impl EmaBatch {
    /// Create a new batch EMA with the given `period`.
    #[must_use]
    pub fn new(period: u32) -> Self {
        Self { period }
    }

    /// Compute the last EMA value from `prices`.
    ///
    /// Returns `None` if `prices.len() < period`.
    pub fn compute_last(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() < self.period as usize {
            return None;
        }
        let alpha = Decimal::from(2) / Decimal::from(self.period + 1);
        let one_minus_alpha = Decimal::ONE - alpha;

        // Seed: SMA of first `period` bars.
        let seed_sum: Decimal = prices[..self.period as usize].iter().copied().sum();
        let mut ema = seed_sum / Decimal::from(self.period);

        for &p in &prices[self.period as usize..] {
            ema = alpha * p + one_minus_alpha * ema;
        }
        Some(ema)
    }
}

/// Default EMA type — the streaming variant.
pub type Ema = EmaStream;

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn t502_ema_seed_window_returns_none_until_full() {
        let mut ema = EmaStream::new(3);
        assert!(ema.push(dec!(10)).is_none());
        assert!(ema.push(dec!(20)).is_none());
        // On the 3rd bar, returns the SMA seed (10+20+30)/3 = 20
        let v = ema.push(dec!(30));
        assert_eq!(v, Some(dec!(20)));
    }

    #[test]
    fn t502_ema_recursive_update() {
        let mut ema = EmaStream::new(3);
        ema.push(dec!(10));
        ema.push(dec!(20));
        ema.push(dec!(30)); // seed = 20
        // α = 2/(3+1) = 0.5
        // EMA_4 = 0.5 * 40 + 0.5 * 20 = 30
        let v4 = ema.push(dec!(40));
        assert_eq!(v4, Some(dec!(30)));
    }

    #[test]
    fn t502_ema_batch_matches_stream() {
        let prices = vec![dec!(10), dec!(20), dec!(30), dec!(40), dec!(50)];
        let stream = {
            let mut s = EmaStream::new(3);
            let mut last = None;
            for &p in &prices {
                last = s.push(p);
            }
            last
        };
        let batch = EmaBatch::new(3).compute_last(&prices);
        let tolerance = Decimal::new(1, 8);
        match (stream, batch) {
            (Some(s), Some(b)) => assert!((s - b).abs() <= tolerance, "stream={s} batch={b}"),
            _ => panic!("expected Some from both"),
        }
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;
    use rust_decimal::Decimal;

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(500))]
        #[test]
        fn t502_ema_stream_batch_agree(
            prices_cents in proptest::collection::vec(1u64..10_000_000u64, 5..30),
            period in 2u32..6u32,
        ) {
            let prices: Vec<Decimal> = prices_cents.iter()
                .map(|c| Decimal::from(*c) / Decimal::from(100u64))
                .collect();

            if prices.len() < period as usize {
                return Ok(());
            }

            let mut stream = EmaStream::new(period);
            let mut stream_val = None;
            for &p in &prices {
                stream_val = stream.push(p);
            }
            let batch_val = EmaBatch::new(period).compute_last(&prices);

            match (stream_val, batch_val) {
                (Some(s), Some(b)) => {
                    let tolerance = Decimal::new(1, 8);
                    prop_assert!(
                        (s - b).abs() <= tolerance,
                        "stream={s} batch={b} diff={}",
                        (s - b).abs()
                    );
                }
                (None, None) => {}
                (s, b) => prop_assert!(false, "disagreement: stream={s:?} batch={b:?}"),
            }
        }
    }
}
