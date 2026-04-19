//! Relative Strength Index — streaming and batch adapters (T502).
//!
//! Uses Wilder's smoothing (equivalent to EMA with α = 1/period).
//! RSI ∈ [0, 100] for all non-degenerate bar sequences.
//!
//! **Algorithm:**
//! 1. Seed: compute average gain and average loss over the first `period` bars.
//! 2. Rolling: smooth gain/loss with Wilder's multiplier (1/period).
//! 3. RSI = 100 − 100/(1 + avg_gain/avg_loss).

use rust_decimal::Decimal;

// ── Streaming ──────────────────────────────────────────────────────────────────

/// Streaming RSI (Wilder's smoothing).
///
/// Returns `Some(rsi)` once `period` bars have been processed.
pub struct RsiStream {
    period: u32,
    prev_close: Option<Decimal>,
    seed_gains: Vec<Decimal>,
    seed_losses: Vec<Decimal>,
    avg_gain: Option<Decimal>,
    avg_loss: Option<Decimal>,
}

impl RsiStream {
    /// Create a new streaming RSI.
    ///
    /// # Panics
    ///
    /// Panics if `period < 2`.
    #[must_use]
    pub fn new(period: u32) -> Self {
        assert!(period >= 2, "RSI period must be >= 2");
        Self {
            period,
            prev_close: None,
            seed_gains: Vec::with_capacity(period as usize),
            seed_losses: Vec::with_capacity(period as usize),
            avg_gain: None,
            avg_loss: None,
        }
    }

    /// Push a new close price; returns `Some(rsi)` once the seed window is full.
    pub fn push(&mut self, close: Decimal) -> Option<Decimal> {
        let Some(prev) = self.prev_close else {
            self.prev_close = Some(close);
            return None;
        };
        self.prev_close = Some(close);

        let diff = close - prev;
        let (gain, loss) = if diff > Decimal::ZERO {
            (diff, Decimal::ZERO)
        } else {
            (Decimal::ZERO, diff.abs())
        };

        if self.avg_gain.is_none() {
            // Still accumulating the seed window.
            self.seed_gains.push(gain);
            self.seed_losses.push(loss);

            if self.seed_gains.len() == self.period as usize {
                // Compute initial SMA of gains/losses.
                let sum_g: Decimal = self.seed_gains.iter().copied().sum();
                let sum_l: Decimal = self.seed_losses.iter().copied().sum();
                let p = Decimal::from(self.period);
                self.avg_gain = Some(sum_g / p);
                self.avg_loss = Some(sum_l / p);
                return Some(Self::rsi_value(
                    self.avg_gain.unwrap(),
                    self.avg_loss.unwrap(),
                ));
            }
            return None;
        }

        // Wilder's smoothing: (prev * (period-1) + current) / period
        let p = Decimal::from(self.period);
        let ag = (self.avg_gain.unwrap() * (p - Decimal::ONE) + gain) / p;
        let al = (self.avg_loss.unwrap() * (p - Decimal::ONE) + loss) / p;
        self.avg_gain = Some(ag);
        self.avg_loss = Some(al);
        Some(Self::rsi_value(ag, al))
    }

    fn rsi_value(avg_gain: Decimal, avg_loss: Decimal) -> Decimal {
        if avg_loss == Decimal::ZERO {
            return Decimal::ONE_HUNDRED;
        }
        let rs = avg_gain / avg_loss;
        Decimal::ONE_HUNDRED - Decimal::ONE_HUNDRED / (Decimal::ONE + rs)
    }

    /// Latest RSI value (None until seed window is complete).
    #[must_use]
    pub fn latest(&self) -> Option<Decimal> {
        match (self.avg_gain, self.avg_loss) {
            (Some(g), Some(l)) => Some(Self::rsi_value(g, l)),
            _ => None,
        }
    }
}

// ── Batch ─────────────────────────────────────────────────────────────────────

/// Batch RSI — computes the last RSI value from a `prices` slice.
pub struct RsiBatch {
    period: u32,
}

impl RsiBatch {
    /// Create a new batch RSI with the given `period`.
    #[must_use]
    pub fn new(period: u32) -> Self {
        Self { period }
    }

    /// Compute last RSI from `prices` slice.
    ///
    /// Returns `None` if fewer than `period + 1` prices are supplied.
    pub fn compute_last(&self, prices: &[Decimal]) -> Option<Decimal> {
        if prices.len() <= self.period as usize {
            return None;
        }
        let mut stream = RsiStream::new(self.period);
        let mut last = None;
        for &p in prices {
            last = stream.push(p);
        }
        last
    }
}

/// Default RSI type.
pub type Rsi = RsiStream;

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn t502_rsi_bounded_0_to_100() {
        let mut rsi = RsiStream::new(14);
        let prices = [
            dec!(100),
            dec!(102),
            dec!(101),
            dec!(103),
            dec!(105),
            dec!(104),
            dec!(106),
            dec!(103),
            dec!(101),
            dec!(99),
            dec!(98),
            dec!(100),
            dec!(102),
            dec!(104),
            dec!(103),
        ];
        for &p in &prices {
            if let Some(v) = rsi.push(p) {
                assert!(v >= dec!(0) && v <= dec!(100), "RSI out of range: {v}");
            }
        }
    }

    #[test]
    fn t502_rsi_all_gains_gives_100() {
        let mut rsi = RsiStream::new(3);
        // Feed enough strictly increasing prices
        for i in 1..20 {
            rsi.push(Decimal::from(i));
        }
        let v = rsi.latest().unwrap();
        assert_eq!(v, dec!(100), "All-gain RSI must be 100, got {v}");
    }

    #[test]
    fn t502_rsi_all_losses_gives_0() {
        let mut rsi = RsiStream::new(3);
        // Feed enough strictly decreasing prices
        for i in (1..20i64).rev() {
            rsi.push(Decimal::from(i));
        }
        let v = rsi.latest().unwrap();
        assert_eq!(v, dec!(0), "All-loss RSI must be 0, got {v}");
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
        fn t502_rsi_always_in_0_100(
            prices_cents in proptest::collection::vec(1u64..10_000_000u64, 5..40),
            period in 2u32..8u32,
        ) {
            let prices: Vec<Decimal> = prices_cents.iter()
                .map(|c| Decimal::from(*c) / Decimal::from(100u64))
                .collect();

            let mut rsi = RsiStream::new(period);
            for &p in &prices {
                if let Some(v) = rsi.push(p) {
                    prop_assert!(
                        v >= Decimal::ZERO && v <= Decimal::from(100),
                        "RSI={v} not in [0,100]"
                    );
                }
            }
        }

        #[test]
        fn t502_rsi_stream_batch_agree(
            prices_cents in proptest::collection::vec(1u64..10_000_000u64, 15..40),
            period in 2u32..8u32,
        ) {
            let prices: Vec<Decimal> = prices_cents.iter()
                .map(|c| Decimal::from(*c) / Decimal::from(100u64))
                .collect();

            if prices.len() <= period as usize {
                return Ok(());
            }

            let mut stream = RsiStream::new(period);
            let mut stream_val = None;
            for &p in &prices {
                stream_val = stream.push(p);
            }
            let batch_val = RsiBatch::new(period).compute_last(&prices);

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
