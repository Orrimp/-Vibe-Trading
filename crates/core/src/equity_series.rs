//! Cross-phase equity-history primitive — Phase 4 (R10).
//!
//! Two consumers build this shape from different sources:
//! * `viewer` (offline) — reads the report's `<stem>__equity.csv`
//!   companion file via `reports::csv_artifacts::EquitySample` rows
//!   (R11.2).
//! * `cockpit` (online) — `audit::query::equity_curve_for_strategy`
//!   walks the realized-pnl journal rows (R12, Q7).
//!
//! Consumers never recompute drawdown / peak / trough / max-DD;
//! [`EquitySeries::from_points`] does it once in an O(N) `Decimal`
//! walk at build time (Q1).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{Money, Timestamp, Usdt};

/// One point on the equity curve. Drawdown is precomputed against the
/// running peak so render-time consumers branchless-render straight from
/// the struct (Q1 — no parallel vectors, no off-by-one risk).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquityPoint {
    pub ts: Timestamp,
    pub equity: Money<Usdt>,
    /// `(running_peak - equity) / running_peak`, in fractional units
    /// (0.0 = at peak; 0.10 = 10 % below peak). Always non-negative;
    /// monotone-up runs leave this at `Decimal::ZERO`.
    pub drawdown_pct: Decimal,
}

/// Equity history with precomputed peak / trough / max-DD metadata.
/// Pure-data, `serde`-friendly. No clock reads, no `f64`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquitySeries {
    /// Oldest-first; `points[0].ts == inception_ts`,
    /// `points[N-1].ts == as_of_ts`.
    pub points: Vec<EquityPoint>,
    pub inception_ts: Timestamp,
    pub as_of_ts: Timestamp,
    pub peak: Money<Usdt>,
    pub trough: Money<Usdt>,
    /// Max of `points[i].drawdown_pct`; `Decimal::ZERO` for monotone-up
    /// inputs. Stored separately from a per-point lookup so KPI
    /// consumers (the viewer's strip) can render `Max DD` without
    /// re-walking the vector.
    pub max_drawdown_pct: Decimal,
}

/// Errors constructing an [`EquitySeries`].
#[derive(Debug, thiserror::Error)]
pub enum EquitySeriesError {
    #[error("equity series cannot be empty")]
    Empty,
    #[error("timestamps must be monotone non-decreasing")]
    NonMonotoneTimestamps,
}

impl EquitySeries {
    /// Single O(N) `Decimal` walk: `running_peak` / `running_trough` /
    /// drawdown vector / max-DD all computed in one left-to-right pass.
    ///
    /// # Errors
    ///
    /// - [`EquitySeriesError::Empty`] when `points.is_empty()`.
    /// - [`EquitySeriesError::NonMonotoneTimestamps`] on the first
    ///   non-monotone-non-decreasing timestamp pair.
    pub fn from_points(points: Vec<(Timestamp, Money<Usdt>)>) -> Result<Self, EquitySeriesError> {
        if points.is_empty() {
            return Err(EquitySeriesError::Empty);
        }

        let inception_ts = points[0].0;
        let as_of_ts = points[points.len() - 1].0;

        let mut running_peak = points[0].1.amount();
        let mut running_trough = points[0].1.amount();
        let mut max_dd = Decimal::ZERO;
        let mut last_ts = inception_ts;
        let mut out: Vec<EquityPoint> = Vec::with_capacity(points.len());

        for (idx, (ts, equity)) in points.into_iter().enumerate() {
            if idx > 0 && ts.unix_millis() < last_ts.unix_millis() {
                return Err(EquitySeriesError::NonMonotoneTimestamps);
            }
            last_ts = ts;
            let amt = equity.amount();
            if amt > running_peak {
                running_peak = amt;
            }
            if amt < running_trough {
                running_trough = amt;
            }
            // Drawdown = (peak − value) / |peak|.
            //
            // The denominator is the ABSOLUTE peak (2-15 review L11): with a
            // NEGATIVE `running_peak` (an all-underwater series — reachable
            // now that LIVE equity is routed through here, e.g. a short gone
            // wrong past zero) the signed denominator flips the ratio's sign,
            // `.max(ZERO)` clamps it, and a series 50 % further underwater
            // reports "Max DD 0.00 %" — a silent lie on the honesty surface.
            // Taking `|peak|` keeps the fraction positive and monotone in the
            // loss. INERT for every non-negative series: `running_peak` is a
            // running max, so it is > 0 for the whole walk whenever
            // `points[0].equity > 0` (every backtest, every funded account)
            // and `|peak| == peak` there — byte-identical output, anchors
            // included.
            let peak_denom = running_peak.abs();
            let dd = if peak_denom.is_zero() {
                Decimal::ZERO
            } else {
                ((running_peak - amt) / peak_denom).max(Decimal::ZERO)
            };
            if dd > max_dd {
                max_dd = dd;
            }
            out.push(EquityPoint {
                ts,
                equity,
                drawdown_pct: dd,
            });
        }

        Ok(Self {
            points: out,
            inception_ts,
            as_of_ts,
            peak: Money::<Usdt>::from_decimal(running_peak),
            trough: Money::<Usdt>::from_decimal(running_trough),
            max_drawdown_pct: max_dd,
        })
    }

    /// Equal-stride bucketing, last-value-wins per bucket; preserves
    /// `points[0]` and `points[N-1]` exactly so peak / trough / inception /
    /// as-of survive the downsample. Short-circuits when
    /// `self.points.len() <= max_points`.
    ///
    /// # Panics
    ///
    /// Panics if `max_points == 0` — caller bug; the cockpit consumer
    /// passes `SPARKLINE_POINT_CAP = 120` and the viewer passes
    /// `2000` per Q5.
    #[must_use]
    pub fn downsample(self, max_points: usize) -> Self {
        assert!(max_points > 0, "downsample requires max_points > 0");
        let n = self.points.len();
        if n <= max_points {
            return self;
        }
        // Equal-stride: stride = ceil(n / max_points).
        let stride = n.div_ceil(max_points);
        let mut bucketed: Vec<EquityPoint> = Vec::with_capacity(max_points + 1);
        // Always preserve `points[0]`.
        bucketed.push(self.points[0].clone());
        // Walk the middle: emit the last value in each `stride`-sized
        // bucket starting after `points[0]`.
        let mut cursor = 1usize;
        while cursor + stride < n - 1 {
            let bucket_end = cursor + stride - 1;
            bucketed.push(self.points[bucket_end].clone());
            cursor += stride;
        }
        // Always preserve `points[N-1]`.
        bucketed.push(self.points[n - 1].clone());

        Self {
            points: bucketed,
            inception_ts: self.inception_ts,
            as_of_ts: self.as_of_ts,
            peak: self.peak,
            trough: self.trough,
            max_drawdown_pct: self.max_drawdown_pct,
        }
    }
}

/// Six-card KPI strip primitive (Phase 4 R2 / Q3) — pure data, no
/// behaviour beyond the [`Self::all_absent`] sentinel for the
/// graceful-fallback path.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BacktestMetrics {
    pub total_return_pct: Decimal,
    pub cagr_pct: Decimal,
    pub cagr_present: bool,
    pub sharpe: Decimal,
    pub sharpe_present: bool,
    pub max_drawdown_pct: Decimal,
    pub win_rate_pct: Decimal,
    pub win_rate_present: bool,
    pub trades: u64,
}

impl BacktestMetrics {
    /// All-absent sentinel for the R2.6 / Q3 graceful-fallback path.
    /// The strip renders six `—` dashes + the muted-body line.
    #[must_use]
    pub fn all_absent() -> Self {
        Self {
            total_return_pct: Decimal::ZERO,
            cagr_pct: Decimal::ZERO,
            cagr_present: false,
            sharpe: Decimal::ZERO,
            sharpe_present: false,
            max_drawdown_pct: Decimal::ZERO,
            win_rate_pct: Decimal::ZERO,
            win_rate_present: false,
            trades: 0,
        }
    }

    /// `true` when this value is indistinguishable from the
    /// [`Self::all_absent`] sentinel — every present-flag false, both
    /// percentages zero, zero trades.
    ///
    /// **This is a statement about the DATA, not about the screen.** A
    /// producer that genuinely parsed no metrics (`reports::parse` on a
    /// `## Summary` with no recognised rows) uses it to route to
    /// `PanelState::Empty`, which is what renders the honest "Backtest
    /// metrics unavailable" strip. A producer holding real, healthy,
    /// flat-at-zero numbers (a live session with no fills yet) must NOT:
    /// `Ready(zeros)` renders `0.00 % / 0.00 % / 0`, because "no data" and
    /// "data is fine and flat" must not look identical (2-15 review H2).
    #[must_use]
    pub fn is_all_absent(&self) -> bool {
        !self.cagr_present
            && !self.sharpe_present
            && !self.win_rate_present
            && self.total_return_pct.is_zero()
            && self.max_drawdown_pct.is_zero()
            && self.trades == 0
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::float_arithmetic)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;

    fn ts(secs: i64) -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
    }

    fn m(d: Decimal) -> Money<Usdt> {
        Money::<Usdt>::from_decimal(d)
    }

    #[test]
    fn from_points_computes_drawdown_correctly() {
        // Peak 100 at idx 0; trough 60 at idx 2; back to 90 at idx 4.
        let pts = vec![
            (ts(0), m(dec!(100))),
            (ts(60), m(dec!(80))),
            (ts(120), m(dec!(60))),
            (ts(180), m(dec!(75))),
            (ts(240), m(dec!(90))),
        ];
        let s = EquitySeries::from_points(pts).expect("ok");
        assert_eq!(s.points.len(), 5);
        assert_eq!(s.peak.amount(), dec!(100));
        assert_eq!(s.trough.amount(), dec!(60));
        assert_eq!(s.max_drawdown_pct, dec!(0.40));
        // Drawdown vector: [0, 0.20, 0.40, 0.25, 0.10].
        assert_eq!(s.points[0].drawdown_pct, Decimal::ZERO);
        assert_eq!(s.points[1].drawdown_pct, dec!(0.20));
        assert_eq!(s.points[2].drawdown_pct, dec!(0.40));
        assert_eq!(s.points[3].drawdown_pct, dec!(0.25));
        assert_eq!(s.points[4].drawdown_pct, dec!(0.10));
    }

    #[test]
    fn from_points_monotone_up_returns_all_zero_drawdown() {
        let pts = vec![
            (ts(0), m(dec!(100))),
            (ts(60), m(dec!(110))),
            (ts(120), m(dec!(120))),
            (ts(180), m(dec!(130))),
            (ts(240), m(dec!(140))),
        ];
        let s = EquitySeries::from_points(pts).expect("ok");
        assert_eq!(s.max_drawdown_pct, Decimal::ZERO);
        assert_eq!(s.trough.amount(), dec!(100));
        for p in &s.points {
            assert_eq!(p.drawdown_pct, Decimal::ZERO);
        }
    }

    #[test]
    fn from_points_50_percent_drawdown_then_recovery() {
        let pts = vec![
            (ts(0), m(dec!(100))),
            (ts(60), m(dec!(200))),
            (ts(120), m(dec!(150))),
            (ts(180), m(dec!(100))),
            (ts(240), m(dec!(180))),
            (ts(300), m(dec!(220))),
        ];
        let s = EquitySeries::from_points(pts).expect("ok");
        assert_eq!(s.max_drawdown_pct, dec!(0.50));
        assert_eq!(s.trough.amount(), dec!(100));
    }

    #[test]
    fn from_points_empty_returns_err() {
        let res = EquitySeries::from_points(vec![]);
        assert!(matches!(res, Err(EquitySeriesError::Empty)));
    }

    #[test]
    fn from_points_non_monotone_returns_err() {
        let pts = vec![
            (ts(0), m(dec!(100))),
            (ts(120), m(dec!(110))),
            (ts(60), m(dec!(120))),
        ];
        let res = EquitySeries::from_points(pts);
        assert!(matches!(res, Err(EquitySeriesError::NonMonotoneTimestamps)));
    }

    #[test]
    fn downsample_to_2000_preserves_peak_and_trough() {
        // Synthetic 5000-point series with a known peak (idx 1500) +
        // trough (idx 3500). Outside the bucket-boundary so the
        // last-value-wins might miss them — verify peak/trough
        // metadata still survives.
        let mut pts: Vec<(Timestamp, Money<Usdt>)> = Vec::with_capacity(5000);
        for i in 0..5000i64 {
            // Base curve: 100 + 0.01*i; at idx 1500 spike to 250; at
            // idx 3500 dip to 30.
            let mut v = dec!(100) + Decimal::new(i, 2);
            if i == 1500 {
                v = dec!(250);
            } else if i == 3500 {
                v = dec!(30);
            }
            pts.push((ts(i * 60), m(v)));
        }
        let s = EquitySeries::from_points(pts).expect("ok");
        let peak_before = s.peak.amount();
        let trough_before = s.trough.amount();
        let max_dd_before = s.max_drawdown_pct;
        let down = s.downsample(2000);
        assert!(down.points.len() <= 2000);
        // Peak / trough metadata survives.
        assert_eq!(down.peak.amount(), peak_before);
        assert_eq!(down.trough.amount(), trough_before);
        assert_eq!(down.max_drawdown_pct, max_dd_before);
    }

    #[test]
    fn downsample_below_target_is_noop() {
        let pts: Vec<(Timestamp, Money<Usdt>)> = (0..100i64)
            .map(|i| (ts(i * 60), m(Decimal::from(100 + i))))
            .collect();
        let s = EquitySeries::from_points(pts).expect("ok");
        let down = s.downsample(2000);
        assert_eq!(down.points.len(), 100);
    }

    #[test]
    fn downsample_preserves_first_and_last_point() {
        let pts: Vec<(Timestamp, Money<Usdt>)> = (0..1000i64)
            .map(|i| (ts(i * 60), m(Decimal::from(100 + i))))
            .collect();
        let first_amt = pts[0].1.amount();
        let last_amt = pts[999].1.amount();
        let s = EquitySeries::from_points(pts).expect("ok");
        let down = s.downsample(120);
        assert_eq!(down.points[0].equity.amount(), first_amt);
        assert_eq!(down.points[down.points.len() - 1].equity.amount(), last_amt);
    }

    // ── 2-15 review L11 — the negative-equity drawdown sign trap ────────────

    /// An ALL-UNDERWATER series (equity never above zero) must report a REAL
    /// drawdown, not `0.00 %`.
    ///
    /// Before the `|peak|` denominator fix: `running_peak = −100` (the
    /// least-negative value), `(−100 − −200)/−100 = −1`, `.max(ZERO)` clamps
    /// to `0` — an account that halved *again* below zero rendered "Max DD
    /// 0.00 %" on the KPI strip. This story is the first to route LIVE equity
    /// into this walk, so the trap became reachable from the product surface.
    #[test]
    fn negative_equity_reports_a_real_drawdown_not_zero() {
        let pts = vec![
            (ts(0), m(dec!(-100))),
            (ts(60), m(dec!(-150))),
            (ts(120), m(dec!(-200))),
        ];
        let s = EquitySeries::from_points(pts).expect("ok");
        assert_eq!(s.peak.amount(), dec!(-100));
        assert_eq!(s.trough.amount(), dec!(-200));
        // (−100 − −200) / |−100| = 1.00 → 100 % below the (negative) peak.
        assert_eq!(s.max_drawdown_pct, dec!(1.00));
        assert_eq!(s.points[0].drawdown_pct, Decimal::ZERO);
        assert_eq!(s.points[1].drawdown_pct, dec!(0.50));
        assert_eq!(s.points[2].drawdown_pct, dec!(1.00));
    }

    /// The fix is INERT for a positive-peak series — the anchored corpus and
    /// every funded account live here. A series that crosses INTO negative
    /// territory keeps its positive `running_peak`, so the denominator is
    /// unchanged and the numbers are byte-identical to the pre-fix walk.
    #[test]
    fn positive_peak_drawdown_unchanged_even_when_equity_goes_negative() {
        let pts = vec![
            (ts(0), m(dec!(100))),
            (ts(60), m(dec!(50))),
            (ts(120), m(dec!(-50))),
        ];
        let s = EquitySeries::from_points(pts).expect("ok");
        assert_eq!(s.peak.amount(), dec!(100));
        // (100 − −50)/100 = 1.50 — the same value the signed denominator gave.
        assert_eq!(s.max_drawdown_pct, dec!(1.50));
        assert_eq!(s.points[1].drawdown_pct, dec!(0.50));
    }

    // ── 2-15 review H2 — the all-absent sentinel is a DATA predicate ────────

    #[test]
    fn all_absent_sentinel_is_recognised_and_real_zeros_are_not() {
        assert!(BacktestMetrics::all_absent().is_all_absent());

        // A live session that is genuinely flat with one fill is NOT absent.
        let mut flat_with_a_fill = BacktestMetrics::all_absent();
        flat_with_a_fill.trades = 1;
        assert!(!flat_with_a_fill.is_all_absent());

        // …nor is a session with a real (non-zero) return.
        let mut moved = BacktestMetrics::all_absent();
        moved.total_return_pct = dec!(0.01);
        assert!(!moved.is_all_absent());
    }
}
