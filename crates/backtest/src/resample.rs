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
//!   included in the current bucket. Enforced by the prefix-invariance test
//!   (`f_hr_3_prefix_invariance_no_look_ahead`): a COMPLETE bucket is
//!   field-for-field unchanged when later source bars arrive.
//! - Gap handling — a rare missing 1h bar degrades that bucket's volume
//!   contribution but does not corrupt boundaries; each bucket accumulates
//!   whatever bars fall within it. Such a bucket is still EMITTED (dropping it
//!   would change the coarse series and move the locked anchors) and, since
//!   review 1-18, is no longer invisible: it is counted, reported on
//!   [`ResampledBars::partial_buckets`], and logged at `WARN`.
//! - Fallible, never panicking — a corrupt bucket returns [`ResampleError`]
//!   (review 1-18; the previous code panicked on two paths and SILENTLY fell
//!   back to `UNIX_EPOCH` on the third). One `(symbol, venue)` per call is a
//!   hard guard; sorted input is a `debug_assert`.
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
//! - Prefix-invariance (the real no-look-ahead property) — plus the weaker
//!   forward-shift responsiveness check it supersedes.
//! - Leap-awareness of [`bars_per_year_1h`] for every year, and its agreement
//!   with [`Horizon::periods_per_year`].
//! - The mixed-instrument guard and the partial-bucket census.

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
    ///
    /// Derived from [`bars_per_year_1h`] so the annualization scalar and the
    /// sweep's expected bar count can never disagree (review 1-18: they used to
    /// be two independent tables — the bar-count one had a non-leap-aware
    /// `_ => 8760` catch-all, so a `--year 2028` run would have counted 365
    /// daily bars against a `periods_per_year` of 366).
    #[must_use]
    pub fn periods_per_year(self, year: i32) -> f64 {
        // Exact for every value in range: bars_per_year_1h ∈ {8760, 8784} and
        // ratio ∈ {1, 4, 24}, and 8760/8784 divide exactly by both 4 and 24.
        #[allow(clippy::cast_precision_loss)]
        {
            (bars_per_year_1h(year) / self.ratio() as usize) as f64
        }
    }
}

/// The number of 1h bars in `year` — 8784 for a leap year, 8760 otherwise.
///
/// THE single source for both the sweep's expected-bar-count arithmetic and
/// [`Horizon::periods_per_year`] (review 1-18 leap-table consistency). Divides
/// exactly by every [`Horizon::ratio`] (1 / 4 / 24) for both values.
#[must_use]
pub fn bars_per_year_1h(year: i32) -> usize {
    if is_leap_year(year) { 8784 } else { 8760 }
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

/// Why a resample could not produce a well-formed coarse bar.
///
/// Every variant is an upstream **data-integrity** violation, not a user error:
/// the OHLCV newtypes ([`Price`] > 0, [`Quantity`] ≥ 0) make all three
/// unreachable for well-formed input. Review 1-18 turned them from `panic!`s
/// into a `Result` because `resample_ohlcv` is library code (CLAUDE.md: no
/// panics outside tests) and because the timestamp case used to fail SILENTLY
/// — it fell back to `UNIX_EPOCH`, which would have re-dated the whole surface
/// to 1970 instead of stopping the run.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ResampleError {
    /// The accumulated bucket volume was negative (impossible on well-formed
    /// OHLCV: every constituent [`Quantity`] is ≥ 0).
    #[error(
        "resample_ohlcv: negative accumulated volume {volume} in bucket at open_ts_ms {open_ts_ms}"
    )]
    NegativeVolume {
        /// The offending sum.
        volume: Decimal,
        /// Bucket-aligned open timestamp (ms since epoch).
        open_ts_ms: i64,
    },
    /// The accumulated `high`/`low` was non-positive (impossible on well-formed
    /// OHLCV: every constituent [`Price`] is > 0).
    #[error("resample_ohlcv: non-positive {field} {value} in bucket at open_ts_ms {open_ts_ms}")]
    NonPositivePrice {
        /// `"high"` or `"low"`.
        field: &'static str,
        /// The offending value.
        value: Decimal,
        /// Bucket-aligned open timestamp (ms since epoch).
        open_ts_ms: i64,
    },
    /// The bucket-aligned `open_ts_ms` is not a representable timestamp.
    ///
    /// Pre-1-18 this silently became `UNIX_EPOCH` — a corrupt bar that looked
    /// perfectly valid downstream.
    #[error("resample_ohlcv: bucket open_ts_ms {open_ts_ms} is not a representable timestamp")]
    InvalidTimestamp {
        /// The un-representable value.
        open_ts_ms: i64,
    },
    /// Two bars with different `(symbol, venue)` landed in the same bucket.
    ///
    /// The caller contract is one symbol+venue per call (the sweep splits by
    /// symbol before resampling). A violation would silently merge two
    /// instruments' OHLCV into one bar.
    #[error(
        "resample_ohlcv: bucket at open_ts_ms {open_ts_ms} mixes instruments — \
         bucket is {bucket_symbol}@{bucket_venue:?} but got {bar_symbol}@{bar_venue:?}; \
         resample one (symbol, venue) series per call"
    )]
    MixedInstrument {
        /// Bucket-aligned open timestamp (ms since epoch).
        open_ts_ms: i64,
        /// The symbol the bucket was opened with.
        bucket_symbol: String,
        /// The venue the bucket was opened with.
        bucket_venue: Venue,
        /// The offending bar's symbol.
        bar_symbol: String,
        /// The offending bar's venue.
        bar_venue: Venue,
    },
}

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
    /// How many source (1h) bars were folded into this bucket.
    ///
    /// Review 1-18 partial-bucket visibility: a bucket built from fewer than
    /// [`Horizon::ratio`] source bars (a gap in the corpus, or a truncated
    /// first/last bucket) used to be byte-indistinguishable from a complete
    /// one. Partials are still EMITTED — dropping them would change the coarse
    /// source series and move the locked anchors — but they are now counted,
    /// exposed on [`ResampledBars::partial_buckets`], and logged loudly.
    source_bar_count: u32,
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
            source_bar_count: 1,
        }
    }

    /// Accumulate another bar into this bucket (same key).
    ///
    /// # Errors
    ///
    /// [`ResampleError::MixedInstrument`] if `bar` carries a different
    /// `(symbol, venue)` than the bar that opened the bucket.
    fn accumulate(&mut self, bar: &Bar) -> Result<(), ResampleError> {
        // Bucket homogeneity guard (review 1-18): silently folding two
        // instruments into one bar would corrupt the surface with no signal.
        if bar.symbol.0.as_str() != self.symbol.0.as_str() || bar.venue != self.venue {
            return Err(ResampleError::MixedInstrument {
                open_ts_ms: self.open_ts_ms,
                bucket_symbol: self.symbol.to_string(),
                bucket_venue: self.venue,
                bar_symbol: bar.symbol.to_string(),
                bar_venue: bar.venue,
            });
        }
        self.high = self.high.max(bar.high.get());
        self.low = self.low.min(bar.low.get());
        self.close = bar.close;
        self.close_ts = bar.close_ts;
        self.local_recv_ts = bar.local_recv_ts;
        self.volume += bar.volume.get();
        self.trade_count = self.trade_count.saturating_add(u64::from(bar.trade_count));
        self.source_bar_count = self.source_bar_count.saturating_add(1);
        Ok(())
    }

    /// Emit this bucket as a [`Bar`].
    ///
    /// # Errors
    ///
    /// - [`ResampleError::NegativeVolume`] if the accumulated volume is < 0.
    /// - [`ResampleError::NonPositivePrice`] if `high`/`low` is ≤ 0.
    /// - [`ResampleError::InvalidTimestamp`] if the bucket-aligned `open_ts_ms`
    ///   is not representable (previously a SILENT `UNIX_EPOCH` fallback — the
    ///   contradiction review 1-18 closed: the doc claimed a panic while the
    ///   code produced a 1970-dated bar).
    ///
    /// All three are unreachable for well-formed OHLCV input (`Price` > 0,
    /// `Quantity` ≥ 0, real-corpus timestamps).
    fn emit(self, target_tf: Timeframe) -> Result<Bar, ResampleError> {
        // Convert bucket-aligned open_ts_ms to an OffsetDateTime.
        // open_ts_ms is derived from a bucket key computed from a real 1h bar's
        // unix_millis, so the i128 nanos multiplication cannot overflow for any
        // calendar date; an out-of-range value is a corrupt-input signal and is
        // now reported instead of silently collapsing to UNIX_EPOCH.
        let open_nanos = i128::from(self.open_ts_ms) * 1_000_000_i128;
        let open_dt =
            time::OffsetDateTime::from_unix_timestamp_nanos(open_nanos).map_err(|_| {
                ResampleError::InvalidTimestamp {
                    open_ts_ms: self.open_ts_ms,
                }
            })?;
        let open_ts = Timestamp::new(open_dt);
        // volume is the sum of non-negative 1h bar volumes (OHLCV invariant).
        let volume = Quantity::new(self.volume).map_err(|_| ResampleError::NegativeVolume {
            volume: self.volume,
            open_ts_ms: self.open_ts_ms,
        })?;
        let high = Price::new(self.high).map_err(|_| ResampleError::NonPositivePrice {
            field: "high",
            value: self.high,
            open_ts_ms: self.open_ts_ms,
        })?;
        let low = Price::new(self.low).map_err(|_| ResampleError::NonPositivePrice {
            field: "low",
            value: self.low,
            open_ts_ms: self.open_ts_ms,
        })?;
        Ok(Bar {
            symbol: self.symbol,
            tf: target_tf,
            open_ts,
            close_ts: self.close_ts,
            local_recv_ts: self.local_recv_ts,
            venue: self.venue,
            open: self.open,
            high,
            low,
            close: self.close,
            volume,
            trade_count: u32::try_from(self.trade_count).unwrap_or(u32::MAX),
        })
    }
}

/// The coarse bars plus the partial-bucket census (review 1-18).
///
/// `bars` is exactly what [`resample_ohlcv`] returns; `partial_buckets` names
/// the buckets that were folded from FEWER than [`Horizon::ratio`] source
/// bars. Partials are **never dropped** — dropping them would change the
/// coarse source series and move every locked horizon anchor — so this is
/// visibility only.
///
/// (No `PartialEq`: `trading_core::Bar` does not implement it. Compare the
/// fields you care about, as the prefix-invariance tests do.)
#[derive(Debug, Clone)]
pub struct ResampledBars {
    /// One `Bar` per coarse bucket, in ascending `open_ts` order.
    pub bars: Vec<Bar>,
    /// `(output_index, open_ts_ms, source_bar_count)` for every incomplete
    /// bucket, in output order. Empty on a gap-free full-year corpus whose span
    /// is bucket-aligned.
    pub partial_buckets: Vec<PartialBucket>,
}

impl ResampledBars {
    /// True when every emitted bucket was folded from a full `ratio` bars.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.partial_buckets.is_empty()
    }
}

/// One incomplete coarse bucket (see [`ResampledBars::partial_buckets`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartialBucket {
    /// Index into [`ResampledBars::bars`].
    pub index: usize,
    /// Bucket-aligned open timestamp (ms since epoch) — names the bucket.
    pub open_ts_ms: i64,
    /// How many source (1h) bars actually landed in it.
    pub source_bar_count: u32,
    /// How many the horizon's ratio requires for a complete bucket.
    pub expected_bar_count: u32,
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
/// **Input contract:** `bars_1h` MUST be sorted by `open_ts` ASC and carry ONE
/// `(symbol, venue)` (the caller splits per symbol and sorts before calling).
/// The sort is `debug_assert`ed; a mixed instrument is a hard
/// [`ResampleError::MixedInstrument`].
///
/// **Output:** one `Bar` per coarse bucket; see module-level doc for the
/// rollup rule and field assignments. Use [`resample_ohlcv_detailed`] when the
/// caller needs the partial-bucket census.
///
/// # Errors
///
/// See [`ResampleError`] — all variants signal upstream data corruption and
/// are unreachable for well-formed OHLCV input.
pub fn resample_ohlcv(bars_1h: &[Bar], horizon: Horizon) -> Result<Vec<Bar>, ResampleError> {
    resample_ohlcv_detailed(bars_1h, horizon).map(|r| r.bars)
}

/// [`resample_ohlcv`] plus the partial-bucket census (review 1-18).
///
/// Identical fold, identical output bars — the only difference is that the
/// incomplete buckets are reported to the caller instead of being
/// byte-indistinguishable from complete ones. Each partial is also logged at
/// `WARN` (target `backtest.resample`) naming the bucket.
///
/// # Errors
///
/// See [`ResampleError`].
pub fn resample_ohlcv_detailed(
    bars_1h: &[Bar],
    horizon: Horizon,
) -> Result<ResampledBars, ResampleError> {
    // Input contract: sorted by open_ts ASC. A misordered input silently
    // produces wrong bucket assignments (the fold is single-pass and monotone),
    // so assert it in debug builds rather than let it corrupt a surface.
    debug_assert!(
        bars_1h.windows(2).all(|w| w[0].open_ts <= w[1].open_ts),
        "resample_ohlcv input contract violated: bars_1h must be sorted by open_ts ASC"
    );

    // Identity pass-through for 1h — the 91 anchors depend on this path being
    // byte-untouched (R-HR.6 / D-HR.2 / D-HR.7). Every 1h bar is its own
    // complete bucket (ratio 1), so there is nothing to report.
    let Some(bucket_ms) = horizon.bucket_ms() else {
        return Ok(ResampledBars {
            bars: bars_1h.to_vec(),
            partial_buckets: Vec::new(),
        });
    };

    let target_tf = horizon.to_timeframe();
    let expected_bar_count = horizon.ratio();
    let mut out: Vec<Bar> = Vec::with_capacity(bars_1h.len() / horizon.ratio() as usize + 1);
    let mut partials: Vec<PartialBucket> = Vec::new();
    let mut acc: Option<BucketAcc> = None;

    // Emit one completed bucket, recording it if it was partial.
    // Partials are EMITTED, never dropped (dropping would change the coarse
    // source series and move the locked anchors) — visibility only.
    let finish = |completed: BucketAcc,
                  out: &mut Vec<Bar>,
                  partials: &mut Vec<PartialBucket>|
     -> Result<(), ResampleError> {
        let source_bar_count = completed.source_bar_count;
        let open_ts_ms = completed.open_ts_ms;
        let bar = completed.emit(target_tf)?;
        let index = out.len();
        out.push(bar);
        if source_bar_count < expected_bar_count {
            tracing::warn!(
                target: "backtest.resample",
                horizon = %horizon,
                bucket_open_ts_ms = open_ts_ms,
                source_bar_count,
                expected_bar_count,
                "partial coarse bucket: folded from fewer than `ratio` source bars \
                 (emitted anyway — a dropped bucket would change the coarse series)"
            );
            partials.push(PartialBucket {
                index,
                open_ts_ms,
                source_bar_count,
                expected_bar_count,
            });
        }
        Ok(())
    };

    for bar in bars_1h {
        let ts_ms = bar.open_ts.unix_millis();
        let key = ts_ms.div_euclid(bucket_ms);

        match &mut acc {
            Some(a) if a.key == key => {
                // Same bucket — accumulate.
                a.accumulate(bar)?;
            }
            Some(_) => {
                // New bucket — emit the old one, start fresh.
                let old = acc.replace(BucketAcc::new(key, bucket_ms, bar));
                if let Some(completed) = old {
                    finish(completed, &mut out, &mut partials)?;
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
        finish(completed, &mut out, &mut partials)?;
    }

    Ok(ResampledBars {
        bars: out,
        partial_buckets: partials,
    })
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

        let resampled_4h = resample_ohlcv(&bars, Horizon::FourHours).expect("well-formed");
        assert_eq!(
            resampled_4h.len(),
            2190,
            "2023 at 4h-cadence should produce 2190 buckets (8760/4), got {}",
            resampled_4h.len()
        );

        let resampled_daily = resample_ohlcv(&bars, Horizon::OneDay).expect("well-formed");
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

        let resampled_4h = resample_ohlcv(&bars, Horizon::FourHours).expect("well-formed");
        assert_eq!(
            resampled_4h.len(),
            2196,
            "2024 at 4h-cadence should produce 2196 buckets (8784/4), got {}",
            resampled_4h.len()
        );

        let resampled_daily = resample_ohlcv(&bars, Horizon::OneDay).expect("well-formed");
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

        let resampled = resample_ohlcv(&bars, Horizon::FourHours).expect("well-formed");
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

        let resampled = resample_ohlcv(&bars, Horizon::OneDay).expect("well-formed");
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
        let resampled = resample_ohlcv(&bars_1h, Horizon::OneDay).expect("well-formed");
        let close_last_daily = resampled[resampled.len() - 1].close.get();
        let bh_daily = (close_last_daily - resampled[0].open.get()) / resampled[0].open.get();

        let diff = (bh_1h - bh_daily).abs();
        assert!(
            diff <= dec!(0.000_001),
            "BH total return should be invariant across horizons; \
             1h={bh_1h}, daily={bh_daily}, diff={diff}"
        );
    }

    // ── F-HR.3.d — bucket boundaries are UTC-aligned, not input-relative ──────

    /// F-HR.3.d — A forward-shifted source series changes the resampled bar.
    ///
    /// Shifting all bars forward by 1h moves bar[3] (originally at h=3, in
    /// the same 4h bucket as bars 0-2) to h+1=4, which starts a new bucket.
    /// The first bucket now has only 3 bars (shifted h=0-2), so its close
    /// differs from the unshifted case (where the first bucket had bars 0-3).
    ///
    /// ## What this does and does NOT prove (review 1-18 doc-truth pass)
    ///
    /// It proves the bucket key is derived from the bar's absolute UTC
    /// timestamp rather than from its position in the input slice — a
    /// position-keyed fold would produce the SAME first bucket after the shift.
    /// It does NOT prove the absence of look-ahead: "output changes when input
    /// changes" holds for any function that reads its input, including one that
    /// reaches forward. `f_hr_3_prefix_invariance_no_look_ahead` is the test
    /// that carries the causality claim.
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

        let resampled_orig = resample_ohlcv(&bars, Horizon::FourHours).expect("well-formed");
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

        let resampled_shifted =
            resample_ohlcv(&bars_shifted, Horizon::FourHours).expect("well-formed");

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
        let resampled = resample_ohlcv(&bars, Horizon::OneHour).expect("identity path");
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

    /// Review 1-18: the bar-count table is leap-aware for EVERY year, and
    /// `periods_per_year` is derived from it — one source, no drift.
    ///
    /// RED-on-revert: restoring a non-leap-aware catch-all (the sweep bin used
    /// to carry `_ => 8760`) makes 2028 report 8760 here, and the derived daily
    /// ppy 365 while the fold produces 366 buckets.
    #[test]
    fn bars_per_year_1h_is_leap_aware_for_every_year() {
        assert_eq!(bars_per_year_1h(2023), 8760, "2023 is not a leap year");
        assert_eq!(bars_per_year_1h(2024), 8784, "2024 is a leap year");
        // The years the old `_ => 8760` catch-all got wrong.
        assert_eq!(bars_per_year_1h(2028), 8784, "2028 is a leap year");
        assert_eq!(bars_per_year_1h(2032), 8784, "2032 is a leap year");
        assert_eq!(bars_per_year_1h(2100), 8760, "2100 is NOT a leap year");
        assert_eq!(bars_per_year_1h(2400), 8784, "2400 IS a leap year");
        // Every horizon divides both values exactly, and ppy agrees with the
        // bucket count the fold would produce.
        for year in [2023, 2024, 2028, 2100, 2400] {
            for horizon in [Horizon::OneHour, Horizon::FourHours, Horizon::OneDay] {
                let ratio = horizon.ratio() as usize;
                assert_eq!(
                    bars_per_year_1h(year) % ratio,
                    0,
                    "{horizon} must divide {year}'s 1h bar count exactly"
                );
                assert_eq!(
                    horizon.periods_per_year(year) as usize,
                    bars_per_year_1h(year) / ratio,
                    "periods_per_year must equal the bucket count at {horizon} in {year}"
                );
            }
        }
    }

    // ── Review 1-18 — resampler hardening ────────────────────────────────────

    /// Two instruments in one bucket are REJECTED, not silently merged.
    ///
    /// RED-on-revert: dropping the homogeneity guard makes this return
    /// `Ok(_)` with one bar whose OHLCV mixes both symbols' prices — a
    /// corruption that no downstream consumer could detect.
    #[test]
    fn mixed_instrument_in_one_bucket_is_rejected() {
        let mut bars = vec![
            make_bar_hour(
                "AAABBB",
                0,
                dec!(100),
                dec!(110),
                dec!(95),
                dec!(101),
                dec!(1),
            ),
            make_bar_hour(
                "AAABBB",
                1,
                dec!(101),
                dec!(111),
                dec!(96),
                dec!(102),
                dec!(1),
            ),
        ];
        // Third bar in the SAME 4h bucket but a different symbol.
        bars.push(make_bar_hour(
            "ZZZYYY",
            2,
            dec!(500),
            dec!(510),
            dec!(490),
            dec!(505),
            dec!(1),
        ));
        let err = resample_ohlcv(&bars, Horizon::FourHours)
            .expect_err("a bucket mixing two symbols must be rejected");
        match &err {
            ResampleError::MixedInstrument {
                bucket_symbol,
                bar_symbol,
                ..
            } => {
                assert_eq!(bucket_symbol, "AAABBB");
                assert_eq!(bar_symbol, "ZZZYYY");
            }
            other => panic!("expected MixedInstrument, got {other:?}"),
        }
        // The message must name both instruments so the corpus bug is findable.
        let msg = err.to_string();
        assert!(
            msg.contains("AAABBB") && msg.contains("ZZZYYY"),
            "error must name both instruments: {msg}"
        );
        // Same symbol, different VENUE is equally rejected.
        let mut cross_venue = vec![make_bar_hour(
            "AAABBB",
            0,
            dec!(100),
            dec!(110),
            dec!(95),
            dec!(101),
            dec!(1),
        )];
        let mut other_venue = make_bar_hour(
            "AAABBB",
            1,
            dec!(101),
            dec!(111),
            dec!(96),
            dec!(102),
            dec!(1),
        );
        other_venue.venue = Venue::Kraken;
        cross_venue.push(other_venue);
        assert!(
            resample_ohlcv(&cross_venue, Horizon::FourHours).is_err(),
            "the same symbol on two venues must not be folded into one bar"
        );
    }

    /// Partial buckets are EMITTED but reported (review 1-18 visibility).
    ///
    /// RED-on-revert (both directions): dropping the partial changes
    /// `bars.len()` — that would move the coarse source series and the locked
    /// horizon anchors — and silencing the census empties `partial_buckets`,
    /// restoring the state where a gap-shortened bucket was
    /// byte-indistinguishable from a complete one.
    #[test]
    fn partial_buckets_are_emitted_and_reported() {
        // 30 hourly bars at daily → one 24-bar bucket + one 6-bar partial.
        let bars: Vec<Bar> = (0..30_i64)
            .map(|h| {
                make_bar_hour(
                    "AAABBB",
                    h,
                    dec!(100),
                    dec!(110),
                    dec!(95),
                    dec!(101),
                    dec!(1),
                )
            })
            .collect();
        let out = resample_ohlcv_detailed(&bars, Horizon::OneDay).expect("well-formed input");
        assert_eq!(
            out.bars.len(),
            2,
            "the partial bucket must still be emitted"
        );
        assert!(!out.is_complete());
        assert_eq!(out.partial_buckets.len(), 1);
        assert_eq!(out.partial_buckets[0].index, 1);
        assert_eq!(out.partial_buckets[0].source_bar_count, 6);
        assert_eq!(out.partial_buckets[0].expected_bar_count, 24);
        assert_eq!(
            out.partial_buckets[0].open_ts_ms,
            out.bars[1].open_ts.unix_millis(),
            "the census names the bucket by its open timestamp"
        );
        // The partial's volume reflects only its 6 constituent bars.
        assert_eq!(out.bars[1].volume.get(), dec!(6));

        // A gap in the MIDDLE of a bucket is caught too (23 of 24 bars).
        let with_gap: Vec<Bar> = (0..24_i64)
            .filter(|h| *h != 5)
            .map(|h| {
                make_bar_hour(
                    "AAABBB",
                    h,
                    dec!(100),
                    dec!(110),
                    dec!(95),
                    dec!(101),
                    dec!(1),
                )
            })
            .collect();
        let gapped = resample_ohlcv_detailed(&with_gap, Horizon::OneDay).expect("well-formed");
        assert_eq!(gapped.bars.len(), 1);
        assert_eq!(
            gapped.partial_buckets.len(),
            1,
            "a single missing 1h bar must be visible, not silently absorbed"
        );
        assert_eq!(gapped.partial_buckets[0].source_bar_count, 23);

        // A whole number of complete buckets reports nothing.
        let full = resample_ohlcv_detailed(&make_year_bars(2023), Horizon::OneDay)
            .expect("well-formed year");
        assert!(
            full.is_complete(),
            "a gap-free full year has no partial buckets: {:?}",
            full.partial_buckets
        );
        // 1h identity has no buckets to be partial.
        let identity = resample_ohlcv_detailed(&make_year_bars(2023), Horizon::OneHour)
            .expect("identity path");
        assert!(identity.is_complete());
        assert_eq!(identity.bars.len(), 8760);
    }

    /// Prefix-invariance: a COMPLETE bucket never changes when later 1h bars
    /// arrive. This is the causality property the fold actually has to satisfy.
    ///
    /// RED-on-revert: any fold that reads forward — `open = last`, a centred
    /// high/low window, a borrowed next-bucket close — changes an
    /// already-complete bucket when the input grows, and the field comparison
    /// fails at that bucket's index.
    ///
    /// (`f_hr_3_causality_forward_shift_changes_bar` below is a WEAKER,
    /// complementary check: it only shows the output responds to a shifted
    /// input, which is true of any function that reads its input.)
    #[test]
    fn f_hr_3_prefix_invariance_no_look_ahead() {
        let bars: Vec<Bar> = (0..48_i64)
            .map(|h| {
                make_bar_hour(
                    "AAABBB",
                    h,
                    Decimal::from(100 + h),
                    Decimal::from(120 + h * 2),
                    Decimal::from(90 + h),
                    Decimal::from(105 + h),
                    Decimal::from(h + 1),
                )
            })
            .collect();

        for horizon in [Horizon::FourHours, Horizon::OneDay] {
            let ratio = horizon.ratio() as usize;
            let full = resample_ohlcv(&bars, horizon).expect("well-formed");
            for k in 1..=bars.len() {
                let prefix = resample_ohlcv(&bars[..k], horizon).expect("well-formed");
                let complete = k / ratio;
                for i in 0..complete.min(prefix.len()).min(full.len()) {
                    let (p, f) = (&prefix[i], &full[i]);
                    assert_eq!(
                        (
                            p.open_ts.unix_millis(),
                            p.open.get(),
                            p.high.get(),
                            p.low.get(),
                            p.close.get(),
                            p.volume.get()
                        ),
                        (
                            f.open_ts.unix_millis(),
                            f.open.get(),
                            f.high.get(),
                            f.low.get(),
                            f.close.get(),
                            f.volume.get()
                        ),
                        "prefix-invariance violated at {horizon} bucket {i}: it changed when \
                         the input grew from {k} to {n} source bars — the fold reads forward",
                        n = bars.len()
                    );
                }
            }
        }
    }
}
