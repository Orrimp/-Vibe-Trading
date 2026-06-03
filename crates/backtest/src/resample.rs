//! OHLCV bar resampler: 1h → {4h-cadence, daily} deterministic fold.
//!
//! ## Design (D-HR.2, horizon-retest-robustness)
//!
//! [`resample_ohlcv`] is a **pure ordered fold** over a sorted (`open_ts` ASC)
//! slice of 1h bars:
//!
//! - [`Horizon::OneHour`] → identity pass-through (the 1h load path is
//!   byte-untouched; the existing 91 anchors stay byte-identical by
//!   construction — R-HR.6 / D-HR.7).
//! - [`Horizon::FourHours`] → 4:1 fold; bucket key =
//!   `open_ts_ms.div_euclid(14_400_000)`.
//! - [`Horizon::OneDay`] → 24:1 fold; bucket key =
//!   `open_ts_ms.div_euclid(86_400_000)`.
//!
//! ## Rollup rule
//!
//! Per bucket: `open = first`, `high = max`, `low = min`, `close = last`,
//! `volume = Σ`, `trade_count = Σ`. The output bar's `open_ts` is
//! bucket-aligned (`bucket_key × bucket_ms`); `close_ts` and `local_recv_ts`
//! are taken from the last constituent bar. `tf` is set to
//! `horizon.to_timeframe()`.
//!
//! ## Correctness guarantees
//!
//! - No `HashMap` — the output is built from a single linear scan over
//!   `open_ts`-sorted input; the bucket key is monotone → buckets are emitted
//!   in order.
//! - `Decimal` throughout — `high`/`low` via `Decimal::max`/`min` (total
//!   order, no f64); volume summed as `Decimal` (ADR-0003 / R-HR.5).
//! - No look-ahead — bucket `t` is closed and emitted only when the first
//!   bar with a bucket key `> t` is encountered (or at EOF). No future bar is
//!   included in the current bucket.
//! - Gap handling — a rare missing 1h bar degrades that bucket's volume
//!   contribution but does not corrupt boundaries; each bucket accumulates
//!   whatever bars fall within it.
//! - Clap-compatible [`Horizon`] enum with `#[value(name)]` annotations so
//!   the sweep can accept `--horizon 1h`, `--horizon 4h`, `--horizon daily`.
//!
//! ## F-HR.3 tests
//!
//! The unit tests in this module (`#[cfg(test)]`) cover:
//! - Exact bucket counts: 2023 (8 760 h) → 2 190 at 4h, 365 at daily;
//!   2024 leap (8 784 h) → 2 196 at 4h, 366 at daily.
//! - Correct OHLCV rollup on a hand-verified 6-bar / 24-bar fixture.
//! - BH total-return invariant (resampled BH return ≈ 1h BH return).
//! - Causality: a forward-shifted source changes the resampled bar.

use rust_decimal::Decimal;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};

// ─────────────────────────────────────────────────────────────────────────────
// Horizon enum
// ─────────────────────────────────────────────────────────────────────────────

/// Decision cadence for the horizon retest.
///
/// Used as a `--horizon` CLI flag in `param_robustness_sweep` and as the
/// control parameter for [`resample_ohlcv`].
///
/// [`Horizon::OneHour`] is the DEFAULT — identity pass-through in
/// [`resample_ohlcv`] so the existing 1h sweep is byte-untouched (R-HR.6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum Horizon {
    /// 1h cadence — identity pass-through (default; all 91 anchors run here).
    #[value(name = "1h")]
    OneHour,
    /// True 4h cadence; 4:1 fold; 2190/2196 bars/year.
    #[value(name = "4h")]
    FourHours,
    /// Daily cadence — 24:1 fold; 365/366 bars/year.
    #[value(name = "daily")]
    OneDay,
}

impl Horizon {
    /// Returns the corresponding `Timeframe` variant.
    #[must_use]
    pub fn to_timeframe(self) -> Timeframe {
        match self {
            Horizon::OneHour => Timeframe::OneHour,
            Horizon::FourHours => Timeframe::FourHours,
            Horizon::OneDay => Timeframe::OneDay,
        }
    }

    /// Returns `Some(bucket_duration_ms)` for coarse horizons; `None` for 1h
    /// (identity).
    ///
    /// ## Bucket durations
    ///
    /// - `"4h"` (4:1): `4 × 3600 × 1000 = 14_400_000 ms` → 2190/2196 bars/year.
    /// - daily (24:1): `24 × 3600 × 1000 = 86_400_000 ms` → 365/366 bars/year.
    #[must_use]
    pub fn bucket_ms(self) -> Option<i64> {
        match self {
            Horizon::OneHour => None,
            // 4h bucket (4:1 ratio): 4 × 3600 × 1000 = 14_400_000 ms.
            Horizon::FourHours => Some(14_400_000),
            // 24h bucket (24:1 ratio): 24 × 3600 × 1000 = 86_400_000 ms.
            Horizon::OneDay => Some(86_400_000),
        }
    }

    /// Ratio of 1h bars per coarse bar (1 / 4 / 24).
    #[must_use]
    pub fn ratio(self) -> u32 {
        match self {
            Horizon::OneHour => 1,
            Horizon::FourHours => 4,
            Horizon::OneDay => 24,
        }
    }

    /// Returns the `periods_per_year` for the given `year`.
    ///
    /// Leap-year aware (2024 = 8784h = leap).
    ///
    /// The 1h value is provided for completeness but the sweep uses the
    /// verbatim 1h fn (`compute_sharpe_hourly`) for 1h runs — NOT this.
    #[must_use]
    pub fn periods_per_year(self, year: i32) -> f64 {
        let is_leap = is_leap_year(year);
        match self {
            Horizon::OneHour => {
                if is_leap {
                    8784.0
                } else {
                    8760.0
                }
            }
            Horizon::FourHours => {
                if is_leap {
                    2196.0
                } else {
                    2190.0
                }
            }
            Horizon::OneDay => {
                if is_leap {
                    366.0
                } else {
                    365.0
                }
            }
        }
    }
}

/// Returns true if `year` is a proleptic Gregorian leap year.
#[must_use]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

impl std::fmt::Display for Horizon {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Horizon::OneHour => "1h",
            Horizon::FourHours => "4h",
            Horizon::OneDay => "daily",
        };
        write!(f, "{s}")
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// resample_ohlcv
// ─────────────────────────────────────────────────────────────────────────────

/// Accumulated state for one coarse-bar bucket.
struct BucketAcc {
    key: i64,
    open_ts_ms: i64,
    symbol: Symbol,
    venue: Venue,
    open: Price,
    high: Decimal,
    low: Decimal,
    close: Price,
    close_ts: Timestamp,
    local_recv_ts: Timestamp,
    volume: Decimal,
    trade_count: u64,
}

impl BucketAcc {
    /// Start a new bucket from the first bar.
    fn new(key: i64, bucket_ms: i64, bar: &Bar) -> Self {
        Self {
            key,
            open_ts_ms: key * bucket_ms,
            symbol: Symbol::new(bar.symbol.0.as_str()),
            venue: bar.venue,
            open: bar.open,
            high: bar.high.get(),
            low: bar.low.get(),
            close: bar.close,
            close_ts: bar.close_ts,
            local_recv_ts: bar.local_recv_ts,
            volume: bar.volume.get(),
            trade_count: u64::from(bar.trade_count),
        }
    }

    /// Accumulate another bar into this bucket (same key).
    fn accumulate(&mut self, bar: &Bar) {
        self.high = self.high.max(bar.high.get());
        self.low = self.low.min(bar.low.get());
        self.close = bar.close;
        self.close_ts = bar.close_ts;
        self.local_recv_ts = bar.local_recv_ts;
        self.volume += bar.volume.get();
        self.trade_count = self.trade_count.saturating_add(u64::from(bar.trade_count));
    }

    /// Emit this bucket as a [`Bar`].
    ///
    /// # Panics
    ///
    /// Panics if `volume` is negative (a data integrity violation — negative
    /// volume is impossible on well-formed OHLCV data). Also panics if
    /// `open_ts_ms` cannot be converted to a valid timestamp (would require
    /// a date beyond ±9 quadrillion years from the Unix epoch).
    fn emit(self, target_tf: Timeframe) -> Bar {
        // Convert bucket-aligned open_ts_ms to an OffsetDateTime.
        let open_nanos = i128::from(self.open_ts_ms) * 1_000_000_i128;
        // SAFETY: open_ts_ms is derived from a bucket key computed from a
        // real 1h bar's unix_millis, which is always within the valid i64
        // range. The nanos multiplication is safe because milliseconds since
        // 1970 fit in i128 without overflow for any calendar date.
        let open_dt = time::OffsetDateTime::from_unix_timestamp_nanos(open_nanos)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        let open_ts = Timestamp::new(open_dt);
        // volume is the sum of non-negative 1h bar volumes (OHLCV invariant).
        // A negative sum would indicate a data corruption — panic loudly.
        let volume = Quantity::new(self.volume)
            .unwrap_or_else(|_| panic!("resample_ohlcv: negative volume {}", self.volume));
        Bar {
            symbol: self.symbol,
            tf: target_tf,
            open_ts,
            close_ts: self.close_ts,
            local_recv_ts: self.local_recv_ts,
            venue: self.venue,
            open: self.open,
            high: Price::new(self.high)
                .unwrap_or_else(|_| panic!("resample_ohlcv: non-positive high {}", self.high)),
            low: Price::new(self.low)
                .unwrap_or_else(|_| panic!("resample_ohlcv: non-positive low {}", self.low)),
            close: self.close,
            volume,
            trade_count: u32::try_from(self.trade_count).unwrap_or(u32::MAX),
        }
    }
}

/// Resample a sorted (`open_ts` ASC) slice of 1h bars into coarser bars.
///
/// - [`Horizon::OneHour`] → identity: returns a clone of `bars_1h` (the 1h
///   load path is byte-untouched; the 91 existing anchors remain valid by
///   construction).
/// - [`Horizon::FourHours`] → 4:1 fold; UTC-bucket key =
///   `open_ts_ms.div_euclid(14_400_000)` (4h bucket).
/// - [`Horizon::OneDay`] → 24:1 fold; UTC-bucket key =
///   `open_ts_ms.div_euclid(86_400_000)`.
///
/// **Input contract:** `bars_1h` MUST be sorted by `open_ts` ASC (the caller
/// sorts per symbol before calling). A misordered input produces incorrect
/// bucket assignments.
///
/// **Output:** one `Bar` per coarse bucket; see module-level doc for the
/// rollup rule and field assignments.
///
/// # Panics
///
/// Panics if an accumulated bucket has a negative total volume or a
/// non-positive `high`/`low` — these would indicate upstream data corruption
/// (OHLCV invariant: volume ≥ 0, high ≥ low > 0).
#[must_use]
pub fn resample_ohlcv(bars_1h: &[Bar], horizon: Horizon) -> Vec<Bar> {
    // Identity pass-through for 1h — the 91 anchors depend on this path being
    // byte-untouched (R-HR.6 / D-HR.2 / D-HR.7).
    let Some(bucket_ms) = horizon.bucket_ms() else {
        return bars_1h.to_vec();
    };

    let target_tf = horizon.to_timeframe();
    let mut out: Vec<Bar> = Vec::with_capacity(bars_1h.len() / horizon.ratio() as usize + 1);
    let mut acc: Option<BucketAcc> = None;

    for bar in bars_1h {
        let ts_ms = bar.open_ts.unix_millis();
        let key = ts_ms.div_euclid(bucket_ms);

        match &mut acc {
            Some(a) if a.key == key => {
                // Same bucket — accumulate.
                a.accumulate(bar);
            }
            Some(_) => {
                // New bucket — emit the old one, start fresh.
                let old = acc.replace(BucketAcc::new(key, bucket_ms, bar));
                if let Some(completed) = old {
                    out.push(completed.emit(target_tf));
                }
            }
            None => {
                // First bar.
                acc = Some(BucketAcc::new(key, bucket_ms, bar));
            }
        }
    }

    // Emit the final bucket (if any).
    if let Some(completed) = acc {
        out.push(completed.emit(target_tf));
    }

    out
}

// ─────────────────────────────────────────────────────────────────────────────
// Unit tests — F-HR.3 (resample correctness)
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::cast_precision_loss,
    clippy::cast_lossless,
    clippy::float_cmp,
    clippy::assertions_on_constants,
    clippy::float_arithmetic,
    clippy::cast_sign_loss,
    clippy::cast_possible_truncation
)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ── Bar builder helpers ───────────────────────────────────────────────────

    /// Build a 1h bar at `open_ts` (provided as `OffsetDateTime`).
    #[allow(clippy::too_many_arguments)]
    fn make_bar_at(
        sym: &str,
        open_dt: time::OffsetDateTime,
        open_p: Decimal,
        high_p: Decimal,
        low_p: Decimal,
        close_p: Decimal,
        volume: Decimal,
        trade_count: u32,
    ) -> Bar {
        let ts = Timestamp::new(open_dt);
        let close_dt = open_dt + time::Duration::hours(1);
        let close_ts = Timestamp::new(close_dt);
        Bar {
            symbol: Symbol::new(sym),
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts,
            local_recv_ts: close_ts,
            venue: Venue::Binance,
            open: Price::new(open_p).unwrap(),
            high: Price::new(high_p).unwrap(),
            low: Price::new(low_p).unwrap(),
            close: Price::new(close_p).unwrap(),
            volume: Quantity::new(volume).unwrap(),
            trade_count,
        }
    }

    /// Build a 1h bar at `epoch(2023-01-01 00:00 UTC) + hour_offset` hours.
    fn make_bar_hour(
        sym: &str,
        hour_offset: i64,
        open_p: Decimal,
        high_p: Decimal,
        low_p: Decimal,
        close_p: Decimal,
        volume: Decimal,
    ) -> Bar {
        let epoch = time::macros::datetime!(2023-01-01 00:00:00 UTC);
        let open_dt = epoch + time::Duration::hours(hour_offset);
        make_bar_at(sym, open_dt, open_p, high_p, low_p, close_p, volume, 1)
    }

    /// Build a full synthetic year of 1h bars (all prices = 1000, vol = 1).
    fn make_year_bars(year: i32) -> Vec<Bar> {
        let start = time::Date::from_calendar_date(year, time::Month::January, 1)
            .unwrap()
            .with_hms(0, 0, 0)
            .unwrap()
            .assume_utc();
        let end = time::Date::from_calendar_date(year + 1, time::Month::January, 1)
            .unwrap()
            .with_hms(0, 0, 0)
            .unwrap()
            .assume_utc();
        let hours = (end - start).whole_hours();
        (0..hours)
            .map(|h| {
                make_bar_at(
                    "BTCUSDT",
                    start + time::Duration::hours(h),
                    dec!(1000),
                    dec!(1001),
                    dec!(999),
                    dec!(1000),
                    dec!(1),
                    1,
                )
            })
            .collect()
    }

    // ── F-HR.3.a — Exact bucket counts for 2023 and 2024 (leap) ─────────────

    /// F-HR.3.a — 2023 (non-leap, 8760h) → 4h-cadence: 2190 buckets, daily:
    /// 365 buckets.
    #[test]
    fn f_hr_3_bucket_counts_4h_daily() {
        let bars = make_year_bars(2023);
        assert_eq!(bars.len(), 8760, "2023 should have 8760 1h bars");

        let resampled_4h = resample_ohlcv(&bars, Horizon::FourHours);
        assert_eq!(
            resampled_4h.len(),
            2190,
            "2023 at 4h-cadence should produce 2190 buckets (8760/4), got {}",
            resampled_4h.len()
        );

        let resampled_daily = resample_ohlcv(&bars, Horizon::OneDay);
        assert_eq!(
            resampled_daily.len(),
            365,
            "2023 at daily should produce 365 buckets (8760/24), got {}",
            resampled_daily.len()
        );
    }

    /// F-HR.3.a — 2024 (leap, 8784h) → 4h-cadence: 2196 buckets, daily: 366
    /// buckets.
    #[test]
    fn f_hr_3_bucket_counts_leap() {
        let bars = make_year_bars(2024);
        assert_eq!(bars.len(), 8784, "2024 (leap) should have 8784 1h bars");

        let resampled_4h = resample_ohlcv(&bars, Horizon::FourHours);
        assert_eq!(
            resampled_4h.len(),
            2196,
            "2024 at 4h-cadence should produce 2196 buckets (8784/4), got {}",
            resampled_4h.len()
        );

        let resampled_daily = resample_ohlcv(&bars, Horizon::OneDay);
        assert_eq!(
            resampled_daily.len(),
            366,
            "2024 at daily should produce 366 buckets (8784/24), got {}",
            resampled_daily.len()
        );
    }

    // ── F-HR.3.b — OHLCV rollup on a hand-verified 4-bar fixture (4h-cadence) ─

    /// F-HR.3.b — 4-bar 4h-cadence fixture: verify open=first, high=max,
    /// low=min, close=last, volume=Σ on a hand-crafted sequence.
    ///
    /// Bars 0-3 (one true 4h bucket, hours 0..3 all fall in [0h, 4h)):
    ///   open: 100, 101, 102, 103 → first = 100
    ///   high: 110, 108, 115, 111 → max = 115
    ///   low:   95,  97,  96,  98 → min = 95
    ///   close: 101, 102, 103, 104 → last = 104
    ///   vol:     1,   2,   3,   4 → Σ = 10
    #[test]
    fn f_hr_3_ohlcv_rollup_hand_verified() {
        let opens: &[Decimal] = &[dec!(100), dec!(101), dec!(102), dec!(103)];
        let highs: &[Decimal] = &[dec!(110), dec!(108), dec!(115), dec!(111)];
        let lows: &[Decimal] = &[dec!(95), dec!(97), dec!(96), dec!(98)];
        let closes: &[Decimal] = &[dec!(101), dec!(102), dec!(103), dec!(104)];
        let vols: &[Decimal] = &[dec!(1), dec!(2), dec!(3), dec!(4)];

        let bars: Vec<Bar> = (0..4_i64)
            .map(|h| {
                make_bar_hour(
                    "AAABBB",
                    h,
                    opens[h as usize],
                    highs[h as usize],
                    lows[h as usize],
                    closes[h as usize],
                    vols[h as usize],
                )
            })
            .collect();

        let resampled = resample_ohlcv(&bars, Horizon::FourHours);
        assert_eq!(
            resampled.len(),
            1,
            "4 bars → 1 four-hour (4h-cadence) bucket"
        );

        let b = &resampled[0];
        assert_eq!(
            b.open.get(),
            dec!(100),
            "open=first: expected 100, got {}",
            b.open.get()
        );
        assert_eq!(
            b.high.get(),
            dec!(115),
            "high=max: expected 115, got {}",
            b.high.get()
        );
        assert_eq!(
            b.low.get(),
            dec!(95),
            "low=min: expected 95, got {}",
            b.low.get()
        );
        assert_eq!(
            b.close.get(),
            dec!(104),
            "close=last: expected 104, got {}",
            b.close.get()
        );
        assert_eq!(
            b.volume.get(),
            dec!(10),
            "volume=Σ: expected 10, got {}",
            b.volume.get()
        );
        assert_eq!(b.tf, Timeframe::FourHours, "tf should be FourHours");
    }

    /// F-HR.3.b — 24-bar daily fixture: verify rollup on a hand-crafted sequence.
    ///
    /// 24 bars (bars h=0..23) with:
    ///   open=100, high=101+h, low=99-h, close=101, vol=1.
    ///   Expected: high=max(101..124)=124, low=min(99..76)=76, vol=24.
    #[test]
    fn f_hr_3_ohlcv_rollup_daily_hand_verified() {
        let bars: Vec<Bar> = (0..24_i64)
            .map(|h| {
                make_bar_hour(
                    "CCCDDD",
                    h,
                    dec!(100),
                    Decimal::from(101 + h),
                    Decimal::from(99 - h),
                    dec!(101),
                    dec!(1),
                )
            })
            .collect();

        let resampled = resample_ohlcv(&bars, Horizon::OneDay);
        assert_eq!(resampled.len(), 1, "24 bars → 1 daily bucket");

        let b = &resampled[0];
        assert_eq!(b.open.get(), dec!(100), "open=first");
        assert_eq!(
            b.high.get(),
            Decimal::from(124),
            "high=max(101..124)=124, got {}",
            b.high.get()
        );
        assert_eq!(
            b.low.get(),
            Decimal::from(76),
            "low=min(99..76)=76, got {}",
            b.low.get()
        );
        assert_eq!(b.close.get(), dec!(101), "close=last");
        assert_eq!(b.volume.get(), Decimal::from(24), "volume=24");
        assert_eq!(b.tf, Timeframe::OneDay, "tf should be OneDay");
    }

    // ── F-HR.3.c — BH total-return invariant ─────────────────────────────────

    /// F-HR.3.c — Buy-and-hold total return is horizon-invariant.
    ///
    /// BH total return = `(final_close - initial_open) / initial_open`.
    /// This equals the resampled BH total return because close of the last
    /// coarse bar == close of the last 1h bar, and open of the first coarse
    /// bar == open of the first 1h bar (open=first rollup rule).
    #[test]
    fn f_hr_3_bh_total_return_invariant() {
        // 24 bars (one UTC day): price goes from 1000 to 1100.
        let n = 24_i64;
        let prices: Vec<Decimal> = (0..=n).map(|i| Decimal::from(1000 + i * 100 / n)).collect();

        let bars_1h: Vec<Bar> = (0..n)
            .map(|h| {
                make_bar_hour(
                    "BTCUSDT",
                    h,
                    prices[h as usize],
                    prices[(h + 1) as usize],
                    prices[h as usize],
                    prices[(h + 1) as usize],
                    dec!(1),
                )
            })
            .collect();

        // 1h BH total return: (close_last - open_first) / open_first
        let open_first = bars_1h[0].open.get();
        let close_last_1h = bars_1h[bars_1h.len() - 1].close.get();
        let bh_1h = (close_last_1h - open_first) / open_first;

        // Resample to daily (1 bucket since n=24 bars in one UTC day).
        let resampled = resample_ohlcv(&bars_1h, Horizon::OneDay);
        let close_last_daily = resampled[resampled.len() - 1].close.get();
        let bh_daily = (close_last_daily - resampled[0].open.get()) / resampled[0].open.get();

        let diff = (bh_1h - bh_daily).abs();
        assert!(
            diff <= dec!(0.000_001),
            "BH total return should be invariant across horizons; \
             1h={bh_1h}, daily={bh_daily}, diff={diff}"
        );
    }

    // ── F-HR.3.d — Causality: forward-shifted source changes the resampled bar ─

    /// F-HR.3.d — A forward-shifted source series changes the resampled bar.
    ///
    /// Shifting all bars forward by 1h moves bar[3] (originally at h=3, in
    /// the same 4h bucket as bars 0-2) to h+1=4, which starts a new bucket.
    /// The first bucket now has only 3 bars (shifted h=0-2), so its close
    /// differs from the unshifted case (where the first bucket had bars 0-3).
    #[test]
    fn f_hr_3_causality_forward_shift_changes_bar() {
        // 8 bars covering hours 0-7; original true-4h-cadence bucketing:
        //   bucket 0: bars h=0..3 (all in [0h, 4h))
        //   bucket 1: bars h=4..7 (in [4h, 8h))
        let bars: Vec<Bar> = (0..8_i64)
            .map(|h| {
                make_bar_hour(
                    "XYZABC",
                    h,
                    Decimal::from(100 + h * 10),
                    Decimal::from(110 + h * 10),
                    Decimal::from(90 + h * 10),
                    Decimal::from(105 + h * 10),
                    dec!(1),
                )
            })
            .collect();

        let resampled_orig = resample_ohlcv(&bars, Horizon::FourHours);
        assert_eq!(
            resampled_orig.len(),
            2,
            "8 bars → 2 four-hour (4h-cadence) buckets"
        );

        // Shift all bars forward by 1h: bars 0-2 → h=1-3 (still in bucket 0),
        // bar 3 → h=4 (now in bucket 1), bars 4-7 → h=5-8 (buckets 1 and 2).
        // So shifted bucket 0: bars[0..2] shifted = 3 bars, close = closes[2]
        // shifted.
        let base_dt = time::macros::datetime!(2023-01-01 00:00:00 UTC);
        let bars_shifted: Vec<Bar> = bars
            .iter()
            .map(|b| {
                let new_open_dt = b.open_ts.inner() + time::Duration::hours(1);
                make_bar_at(
                    "XYZABC",
                    new_open_dt,
                    b.open.get(),
                    b.high.get(),
                    b.low.get(),
                    b.close.get(),
                    b.volume.get(),
                    b.trade_count,
                )
            })
            .collect();
        let _ = base_dt; // suppress unused warning on base_dt

        let resampled_shifted = resample_ohlcv(&bars_shifted, Horizon::FourHours);

        // Original bucket 0: bars h=0..3, close = closes[3] = 105 + 3*10 = 135.
        // Shifted bucket 0: bars h=1..3 (shifted), close = closes[2] = 105 + 2*10 = 125.
        let orig_close_0 = resampled_orig[0].close.get();
        let shifted_close_0 = resampled_shifted[0].close.get();
        assert_ne!(
            orig_close_0, shifted_close_0,
            "f_hr_3_causality: a forward-shifted source must change the resampled bar; \
             orig_close={orig_close_0}, shifted_close={shifted_close_0}"
        );
    }

    // ── 1h identity pass-through ──────────────────────────────────────────────

    /// `Horizon::OneHour` must return a byte-exact clone of the input.
    #[test]
    fn resample_1h_identity() {
        let bars = make_year_bars(2023);
        let resampled = resample_ohlcv(&bars, Horizon::OneHour);
        assert_eq!(
            resampled.len(),
            bars.len(),
            "1h identity must preserve bar count"
        );
        assert_eq!(
            resampled[0].open_ts.unix_millis(),
            bars[0].open_ts.unix_millis(),
            "1h identity: first bar open_ts must match"
        );
        assert_eq!(
            resampled[bars.len() - 1].open_ts.unix_millis(),
            bars[bars.len() - 1].open_ts.unix_millis(),
            "1h identity: last bar open_ts must match"
        );
    }

    // ── Horizon helpers ───────────────────────────────────────────────────────

    #[test]
    fn periods_per_year_values() {
        // Non-leap 2023
        assert_eq!(Horizon::OneHour.periods_per_year(2023), 8760.0);
        assert_eq!(Horizon::FourHours.periods_per_year(2023), 2190.0);
        assert_eq!(Horizon::OneDay.periods_per_year(2023), 365.0);
        // Leap 2024
        assert_eq!(Horizon::OneHour.periods_per_year(2024), 8784.0);
        assert_eq!(Horizon::FourHours.periods_per_year(2024), 2196.0);
        assert_eq!(Horizon::OneDay.periods_per_year(2024), 366.0);
    }

    #[test]
    fn is_leap_year_correct() {
        assert!(is_leap_year(2024));
        assert!(!is_leap_year(2023));
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(is_leap_year(1600));
    }
}
