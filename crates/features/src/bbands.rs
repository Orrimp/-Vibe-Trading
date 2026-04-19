//! Bollinger Bands — streaming and batch adapters (T502).
//!
//! Classic Bollinger Bands:
//! - Mid  = SMA(period)
//! - Upper = mid + mult × std_dev
//! - Lower = mid − mult × std_dev
//!
//! `std_dev` is the population standard deviation of the `period`-bar window.

use rust_decimal::Decimal;

use crate::sma::SmaStream;

// ── Output ────────────────────────────────────────────────────────────────────

/// Bollinger Bands output at a single bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BbandsValue {
    pub upper: Decimal,
    pub mid: Decimal,
    pub lower: Decimal,
}

// ── Streaming ─────────────────────────────────────────────────────────────────

/// Streaming Bollinger Bands.
///
/// Returns `Some(BbandsValue)` once `period` bars have been seen.
pub struct BbandsStream {
    period: u32,
    mult: Decimal,
    sma: SmaStream,
    window: std::collections::VecDeque<Decimal>,
}

impl BbandsStream {
    /// Create a new streaming Bollinger Bands.
    ///
    /// # Panics
    ///
    /// Panics if `period == 0` or `mult <= 0`.
    #[must_use]
    pub fn new(period: u32, mult: Decimal) -> Self {
        assert!(period > 0, "Bollinger period must be > 0");
        assert!(mult > Decimal::ZERO, "Bollinger mult must be > 0");
        Self {
            period,
            mult,
            sma: SmaStream::new(period as usize),
            window: std::collections::VecDeque::with_capacity(period as usize),
        }
    }

    /// Push a new close price.
    pub fn push(&mut self, close: Decimal) -> Option<BbandsValue> {
        // Maintain rolling window for std-dev.
        if self.window.len() == self.period as usize {
            self.window.pop_front();
        }
        self.window.push_back(close);

        let mid = self.sma.push(close)?;

        // Population std dev over the window.
        let variance = self.window.iter()
            .map(|&x| {
                let diff = x - mid;
                diff * diff
            })
            .fold(Decimal::ZERO, |acc, v| acc + v)
            / Decimal::from(self.period);

        // sqrt via Newton-Raphson.
        let std_dev = decimal_sqrt(variance);

        let band = self.mult * std_dev;
        Some(BbandsValue {
            upper: mid + band,
            mid,
            lower: mid - band,
        })
    }
}

// ── Batch ─────────────────────────────────────────────────────────────────────

/// Batch Bollinger Bands.
pub struct BbandsBatch {
    period: u32,
    mult: Decimal,
}

impl BbandsBatch {
    /// Create a new batch Bollinger Bands.
    #[must_use]
    pub fn new(period: u32, mult: Decimal) -> Self {
        Self { period, mult }
    }

    /// Compute last Bollinger Bands value.
    ///
    /// Returns `None` if `prices.len() < period`.
    pub fn compute_last(&self, prices: &[Decimal]) -> Option<BbandsValue> {
        let mut stream = BbandsStream::new(self.period, self.mult);
        let mut last = None;
        for &p in prices {
            last = stream.push(p);
        }
        last
    }
}

/// Default Bollinger Bands type.
pub type Bbands = BbandsStream;

// ── Decimal square-root via Newton-Raphson ─────────────────────────────────────

/// Compute √x for `x ≥ 0` using Newton-Raphson iteration on `Decimal`.
///
/// Converges to within `Decimal::new(1, 28)` (full precision).
fn decimal_sqrt(x: Decimal) -> Decimal {
    if x == Decimal::ZERO {
        return Decimal::ZERO;
    }
    // Initial estimate: use f64 as seed to reduce iterations.
    let x_f64: f64 = x.to_string().parse().unwrap_or(1.0_f64);
    let seed = x_f64.sqrt();
    let mut est = Decimal::try_from(seed).unwrap_or(Decimal::ONE);

    // Newton-Raphson: est = (est + x/est) / 2
    for _ in 0..50 {
        if est == Decimal::ZERO {
            break;
        }
        let next = (est + x / est) / Decimal::from(2);
        if (next - est).abs() < Decimal::new(1, 28) {
            est = next;
            break;
        }
        est = next;
    }
    est
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn t502_bbands_upper_above_lower() {
        let mut bb = BbandsStream::new(3, dec!(2));
        let prices = [dec!(10), dec!(12), dec!(11), dec!(13), dec!(12)];
        for &p in &prices {
            if let Some(v) = bb.push(p) {
                assert!(v.upper >= v.mid, "upper < mid");
                assert!(v.mid >= v.lower, "mid < lower");
            }
        }
    }

    #[test]
    fn t502_bbands_constant_prices_give_zero_band() {
        let mut bb = BbandsStream::new(5, dec!(2));
        for _ in 0..10 {
            if let Some(v) = bb.push(dec!(100)) {
                // With all equal prices, std_dev = 0 so upper = lower = mid.
                assert_eq!(v.upper, v.mid, "upper should equal mid");
                assert_eq!(v.lower, v.mid, "lower should equal mid");
            }
        }
    }

    #[test]
    fn t502_bbands_batch_matches_stream() {
        let prices: Vec<Decimal> = (1..=25i32).map(Decimal::from).collect();
        let stream = {
            let mut bb = BbandsStream::new(20, dec!(2));
            let mut last = None;
            for &p in &prices {
                last = bb.push(p);
            }
            last
        };
        let batch = BbandsBatch::new(20, dec!(2)).compute_last(&prices);
        match (stream, batch) {
            (Some(s), Some(b)) => {
                let tol = Decimal::new(1, 6);
                assert!((s.upper - b.upper).abs() <= tol, "upper mismatch: s={} b={}", s.upper, b.upper);
                assert!((s.lower - b.lower).abs() <= tol, "lower mismatch");
            }
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
        fn t502_bbands_upper_gte_lower(
            prices_cents in proptest::collection::vec(1u64..10_000_000u64, 5..30),
            period in 2u32..6u32,
        ) {
            let prices: Vec<Decimal> = prices_cents.iter()
                .map(|c| Decimal::from(*c) / Decimal::from(100u64))
                .collect();

            if prices.len() < period as usize {
                return Ok(());
            }

            let mut bb = BbandsStream::new(period, Decimal::from(2));
            for &p in &prices {
                if let Some(v) = bb.push(p) {
                    prop_assert!(v.upper >= v.lower, "upper < lower: u={} l={}", v.upper, v.lower);
                }
            }
        }

        #[test]
        fn t502_bbands_stream_batch_agree(
            prices_cents in proptest::collection::vec(1u64..10_000_000u64, 10..30),
            period in 2u32..6u32,
        ) {
            let prices: Vec<Decimal> = prices_cents.iter()
                .map(|c| Decimal::from(*c) / Decimal::from(100u64))
                .collect();

            if prices.len() < period as usize {
                return Ok(());
            }

            let mut stream = BbandsStream::new(period, Decimal::from(2));
            let mut stream_val = None;
            for &p in &prices {
                stream_val = stream.push(p);
            }
            let batch_val = BbandsBatch::new(period, Decimal::from(2)).compute_last(&prices);

            match (stream_val, batch_val) {
                (Some(s), Some(b)) => {
                    let tol = Decimal::new(1, 6);
                    prop_assert!((s.upper - b.upper).abs() <= tol, "upper diff={}", (s.upper-b.upper).abs());
                    prop_assert!((s.lower - b.lower).abs() <= tol, "lower diff={}", (s.lower-b.lower).abs());
                }
                (None, None) => {}
                (sv, bv) => prop_assert!(false, "disagreement: stream={sv:?} batch={bv:?}"),
            }
        }
    }
}
