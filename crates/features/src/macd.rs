//! MACD — Moving Average Convergence Divergence (T502).
//!
//! Produces line, signal, and histogram from two EMAs.
//!
//! **Components:**
//! - MACD line   = EMA(fast) − EMA(slow)
//! - Signal line = EMA(macd_line, signal_period)
//! - Histogram   = MACD line − Signal line

use rust_decimal::Decimal;

use crate::ema::EmaStream;

// ── Output ────────────────────────────────────────────────────────────────────

/// MACD output at a single bar.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MacdValue {
    /// MACD line (fast EMA − slow EMA).
    pub line: Decimal,
    /// Signal line (EMA of MACD line).
    pub signal: Decimal,
    /// Histogram = line − signal.
    pub hist: Decimal,
}

// ── Streaming ─────────────────────────────────────────────────────────────────

/// Streaming MACD.
///
/// Returns `Some(MacdValue)` once the slow EMA + signal_period warmup is done.
pub struct MacdStream {
    fast_ema: EmaStream,
    slow_ema: EmaStream,
    signal_ema: EmaStream,
}

impl MacdStream {
    /// Create a new streaming MACD.
    ///
    /// # Panics
    ///
    /// Panics if `fast >= slow` or any period is 0.
    #[must_use]
    pub fn new(fast: u32, slow: u32, signal_period: u32) -> Self {
        assert!(fast < slow, "MACD fast({fast}) must be < slow({slow})");
        assert!(fast > 0 && slow > 0 && signal_period > 0, "MACD periods must be > 0");
        Self {
            fast_ema: EmaStream::new(fast),
            slow_ema: EmaStream::new(slow),
            signal_ema: EmaStream::new(signal_period),
        }
    }

    /// Push a new close price.
    ///
    /// Returns `Some(MacdValue)` once both EMAs and signal EMA are seeded.
    ///
    /// Both the fast and slow EMAs are pushed unconditionally so their internal
    /// ring buffers advance in lock-step. MACD line and signal are only
    /// computed once both EMAs have values.
    pub fn push(&mut self, close: Decimal) -> Option<MacdValue> {
        // Push both EMAs unconditionally so slow EMA warms up in parallel.
        let fast_val = self.fast_ema.push(close);
        let slow_val = self.slow_ema.push(close);
        let (fast, slow) = (fast_val?, slow_val?);
        let macd_line = fast - slow;
        let signal = self.signal_ema.push(macd_line)?;
        let hist = macd_line - signal;
        Some(MacdValue { line: macd_line, signal, hist })
    }
}

// ── Batch ─────────────────────────────────────────────────────────────────────

/// Batch MACD — computes last `MacdValue` from a `prices` slice.
pub struct MacdBatch {
    fast: u32,
    slow: u32,
    signal_period: u32,
}

impl MacdBatch {
    /// Create a new batch MACD.
    #[must_use]
    pub fn new(fast: u32, slow: u32, signal_period: u32) -> Self {
        Self { fast, slow, signal_period }
    }

    /// Compute last MACD value from `prices`.
    ///
    /// Returns `None` if there are not enough bars to seed both EMAs and the
    /// signal EMA.
    pub fn compute_last(&self, prices: &[Decimal]) -> Option<MacdValue> {
        let mut stream = MacdStream::new(self.fast, self.slow, self.signal_period);
        let mut last = None;
        for &p in prices {
            last = stream.push(p);
        }
        last
    }
}

/// Default MACD type.
pub type Macd = MacdStream;

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn t502_macd_returns_none_during_warmup() {
        // We just verify the invariant: once Some is returned, subsequent
        // pushes also return Some (no de-seeding), and at least the first
        // 25 bars return None (slow EMA not seeded yet).
        let mut macd = MacdStream::new(12, 26, 9);
        // First 25 bars: slow EMA (period=26) is still in seed window → None.
        for i in 0..25 {
            let v = macd.push(dec!(100));
            assert!(v.is_none(), "expected None at bar {}, got {v:?}", i + 1);
        }
        // Find the first bar with a value.
        let mut first_some_bar = None;
        for i in 25..100 {
            if let Some(_v) = macd.push(dec!(100)) {
                first_some_bar = Some(i + 1);
                break;
            }
        }
        assert!(first_some_bar.is_some(), "MACD never produced a value in 100 bars");
        // Slow EMA(26) and fast EMA(12) are pushed in parallel, so slow is ready at
        // bar 26, fast at bar 12. First MACD line = bar 26.
        // Signal EMA(9) needs 9 MACD values → first MacdValue = bar 26+9-1 = 34.
        let bar = first_some_bar.unwrap();
        assert_eq!(bar, 34, "expected first-Some bar at 34, got {bar}");
    }

    #[test]
    fn t502_macd_hist_is_line_minus_signal() {
        let mut macd = MacdStream::new(3, 6, 3);
        let prices = [
            dec!(10), dec!(11), dec!(12), dec!(13), dec!(12),
            dec!(11), dec!(12), dec!(13), dec!(14), dec!(15),
            dec!(14), dec!(13), dec!(14), dec!(15), dec!(16),
        ];
        for &p in &prices {
            if let Some(v) = macd.push(p) {
                let expected_hist = v.line - v.signal;
                assert_eq!(v.hist, expected_hist, "hist must be line - signal");
            }
        }
    }

    #[test]
    fn t502_macd_batch_matches_stream() {
        let prices: Vec<Decimal> = (1..=50i32).map(Decimal::from).collect();
        let stream = {
            let mut m = MacdStream::new(12, 26, 9);
            let mut last = None;
            for &p in &prices {
                last = m.push(p);
            }
            last
        };
        let batch = MacdBatch::new(12, 26, 9).compute_last(&prices);
        match (stream, batch) {
            (Some(s), Some(b)) => {
                let tol = Decimal::new(1, 8);
                assert!((s.line - b.line).abs() <= tol, "line mismatch");
                assert!((s.signal - b.signal).abs() <= tol, "signal mismatch");
                assert!((s.hist - b.hist).abs() <= tol, "hist mismatch");
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
        fn t502_macd_stream_batch_agree(
            prices_cents in proptest::collection::vec(1u64..10_000_000u64, 50..80),
        ) {
            // Use fixed params: fast=12, slow=26, signal=9 (canonical)
            let prices: Vec<Decimal> = prices_cents.iter()
                .map(|c| Decimal::from(*c) / Decimal::from(100u64))
                .collect();

            let mut stream = MacdStream::new(12, 26, 9);
            let mut stream_val = None;
            for &p in &prices {
                stream_val = stream.push(p);
            }
            let batch_val = MacdBatch::new(12, 26, 9).compute_last(&prices);

            match (stream_val, batch_val) {
                (Some(s), Some(b)) => {
                    let tol = Decimal::new(1, 8);
                    prop_assert!((s.line - b.line).abs() <= tol, "line diff={}", (s.line-b.line).abs());
                    prop_assert!((s.hist - b.hist).abs() <= tol, "hist diff={}", (s.hist-b.hist).abs());
                }
                (None, None) => {}
                (sv, bv) => prop_assert!(false, "disagreement: stream={sv:?} batch={bv:?}"),
            }
        }
    }
}
