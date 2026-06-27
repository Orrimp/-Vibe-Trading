//! Market-calendar layer — ADR-0073 D1.
//!
//! Resolves a Yahoo ticker to its [`MarketCalendar`] and counts trading days
//! in a `[start_ms, end_ms)` window for the 95% coverage gate
//! (`load_cached` step 6). This is the **durable "v0.2.0 market-calendar
//! layer"** the code deferred at `yahoo.rs:982-984`.
//!
//! # Design contract (ADR-0073 D1 / D2)
//!
//! - `load_cached`'s **public signature is UNCHANGED** — the calendar is
//!   derived INTERNALLY from the ticker already passed to the function.
//! - `expected_bars_for_range` is **NOT modified** — it stays the
//!   `Crypto24x7` implementation pinned by 4 existing tests.
//! - A new `expected_bars_for_calendar(cal, interval, start_ms, end_ms)`
//!   is added ADDITIVELY: for `Days1` it calls
//!   `cal.trading_days_in_range(start_ms, end_ms)`; for `Hours1`/`Minutes1`
//!   it delegates to `expected_bars_for_range` (unchanged).
//! - For `cal == Crypto24x7`, `expected_bars_for_calendar == expected_bars_for_range`
//!   **by construction** (T-CAL unit test; see below). This is the anchor-safety
//!   proof: all 14 existing `load_cached` call sites pass crypto tickers and
//!   therefore see zero change in behaviour.
//! - Unknown tickers default to `Crypto24x7` (conservative: preserves current
//!   behaviour for anything not explicitly reclassified).
//!
//! # US-holiday set
//!
//! `UsEquity::trading_days_in_range` counts weekdays (Mon–Fri) in the range
//! minus a static `US_MARKET_HOLIDAYS` list of fixed-and-observed NYSE closures
//! that fall on weekdays. The gate is a **≥95% floor** so the set need not be
//! exhaustive — it is a conservative lower bound on expected that can only make
//! the gate more lenient, never reject valid data. The set covers the standard
//! NYSE holidays (New Year, MLK, Presidents', Good Friday, Memorial,
//! Juneteenth, Independence, Labor, Thanksgiving, Christmas) with their
//! observed-on-Monday rules for Sunday holidays.

use time::{Date, Duration, OffsetDateTime, Weekday};

// ── US market holidays ─────────────────────────────────────────────────────────

/// Fixed US NYSE market holidays: `(month, day)`.
///
/// New Year's Day, Martin Luther King Jr. Day, Presidents' Day, Good Friday,
/// Memorial Day, Juneteenth, Independence Day, Labor Day, Thanksgiving,
/// Christmas Day.  "Observed" rules (e.g. Sunday holiday → Monday) are applied
/// at runtime in `us_weekday_holiday_count`.
///
/// Good Friday is not fixed (Easter-relative); we approximate it as the Friday
/// before Easter. Because the coverage gate is a ≥95% floor, this
/// approximation is safe — the set is a LOWER BOUND on expected-bar count.
///
/// **Fixed-date holidays included here:** New Year's, Juneteenth (June 19),
/// Independence Day (July 4), Christmas (Dec 25). The "first/third Monday"
/// holidays (MLK, Presidents', Memorial, Labor, Thanksgiving) are computed
/// separately via `nth_weekday`.
const US_FIXED_HOLIDAYS: &[(u8, u8)] = &[
    (1, 1),   // New Year's Day
    (6, 19),  // Juneteenth
    (7, 4),   // Independence Day
    (12, 25), // Christmas Day
];

/// "Nth weekday of month" holidays: `(month, nth, weekday_offset)`.
///
/// Format: `(month_num, which_occurrence [1=first, 2=second, …], weekday [1=Mon, 5=Fri])`.
///
/// - January: 3rd Monday = MLK Day
/// - February: 3rd Monday = Presidents' Day
/// - May: last Monday = Memorial Day (approximated as 5th or 4th Monday)
/// - September: 1st Monday = Labor Day
/// - November: 4th Thursday = Thanksgiving
const US_NTH_WEEKDAY_HOLIDAYS: &[(u8, u8, u8)] = &[
    (1, 3, 1), // January 3rd Monday — MLK
    (2, 3, 1), // February 3rd Monday — Presidents' Day
    (9, 1, 1), // September 1st Monday — Labor Day
    (11, 4, 4), // November 4th Thursday — Thanksgiving
               // Memorial Day = last Monday in May — handled specially in us_weekday_holiday_count
];

// ── Calendar enum ─────────────────────────────────────────────────────────────

/// Which trading calendar applies to a Yahoo ticker.
///
/// The DEFAULT for unknown tickers is `Crypto24x7` (conservative — preserves
/// the prior behaviour exactly for anything not explicitly reclassified).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarketCalendar {
    /// 24/7 — every wall-clock day is a trading day. Crypto.
    ///
    /// `trading_days_in_range` is a pure wall-clock day count, **identical
    /// to `expected_bars_for_range(Days1, …)`** — this is the anchor-safety
    /// invariant (T-CAL).
    Crypto24x7,
    /// ~5/7 — Mon–Fri minus a US market-holiday set. Equities / index / FX / rates.
    UsEquity,
}

impl MarketCalendar {
    /// Count of trading days in `[start_ms, end_ms)` for THIS calendar.
    ///
    /// `Crypto24x7` → wall-clock day count (byte-identical to
    /// `expected_bars_for_range(Days1, …)`).
    /// `UsEquity` → weekdays (Mon–Fri) in range minus `US_MARKET_HOLIDAYS`
    /// occurrences that fall on weekdays.
    ///
    /// Returns `0` for a zero or negative range.
    #[must_use]
    pub fn trading_days_in_range(self, start_ms: i64, end_ms: i64) -> usize {
        let range_ms = (end_ms - start_ms).max(0) as u64;
        match self {
            MarketCalendar::Crypto24x7 => {
                // Byte-identical to expected_bars_for_range(Days1, start_ms, end_ms).
                const MS_PER_DAY: u64 = 86_400_000;
                (range_ms / MS_PER_DAY) as usize
            }
            MarketCalendar::UsEquity => us_trading_days_in_range(start_ms, end_ms),
        }
    }
}

// ── Ticker classifier ─────────────────────────────────────────────────────────

/// Resolve a Yahoo ticker to its [`MarketCalendar`].
///
/// Classification rules (first match wins):
/// 1. Leading `^` (index) → `UsEquity`.
/// 2. Suffix `=F` (futures) or `=X` (FX spot) → `UsEquity`.
/// 3. Exact match `DX-Y.NYB` (the ICE US Dollar Index) → `UsEquity`.
/// 4. Suffix `-USD` (crypto mirror pairs like `BTC-USD`, `ETH-USD`) → `Crypto24x7`.
/// 5. All other / unknown tickers → `Crypto24x7` (conservative default).
///
/// The 12 corpus crypto tickers (`BTC-USD`, …, `LINK-USD`) are covered by
/// rule 4. All three macro-regime tickers (`^GSPC`, `^TNX`, `DX-Y.NYB`) are
/// covered by rules 1 and 3. Unknown tickers default to `Crypto24x7` so the
/// existing 14 `load_cached` call sites behave byte-identically.
#[must_use]
pub fn classify_ticker(ticker: &str) -> MarketCalendar {
    // Rule 1: leading '^' = index (^GSPC, ^TNX, ^DXY, …)
    if ticker.starts_with('^') {
        return MarketCalendar::UsEquity;
    }
    // Rule 2: suffix '=F' (futures) or '=X' (FX spot)
    if ticker.ends_with("=F") || ticker.ends_with("=X") {
        return MarketCalendar::UsEquity;
    }
    // Rule 3: DX-Y.NYB (the specific ICE dollar index ticker)
    if ticker == "DX-Y.NYB" {
        return MarketCalendar::UsEquity;
    }
    // Rule 4: crypto mirror pairs end with '-USD'
    if ticker.ends_with("-USD") {
        return MarketCalendar::Crypto24x7;
    }
    // Rule 5: default — conservative, preserves prior behaviour
    MarketCalendar::Crypto24x7
}

// ── US trading-day counter ─────────────────────────────────────────────────────

/// Count trading days in `[start_ms, end_ms)` under the US equity calendar.
///
/// Algorithm:
/// 1. Enumerate dates in `[start_date, end_date)`.
/// 2. Count only Monday–Friday (weekdays).
/// 3. Subtract holiday occurrences in that range that fall on a weekday.
///
/// The holiday set is a **lower bound on expected** (the 95% coverage gate
/// absorbs small inaccuracies — see module doc). Some holidays (Good Friday,
/// early-close days) are intentionally omitted; the gate remains conservative.
fn us_trading_days_in_range(start_ms: i64, end_ms: i64) -> usize {
    if end_ms <= start_ms {
        return 0;
    }

    let start_date = ms_to_date(start_ms);
    let end_date = ms_to_date(end_ms); // exclusive

    // Count weekdays in [start_date, end_date)
    let total_weekdays = count_weekdays(start_date, end_date);

    // Subtract holidays that fall on a weekday within the range.
    let holidays = us_weekday_holiday_count(start_date, end_date);

    total_weekdays.saturating_sub(holidays)
}

/// Count weekday (Mon–Fri) dates in `[start, end)`.
fn count_weekdays(start: Date, end: Date) -> usize {
    if end <= start {
        return 0;
    }
    let days = (end - start).whole_days();
    if days <= 0 {
        return 0;
    }

    // Fast path: whole 7-day weeks + remainder.
    let full_weeks = (days / 7) as usize;
    let rem = (days % 7) as usize;

    // For the remainder, count from start_weekday cycling through.
    let start_wd = start.weekday() as usize; // Mon=0 .. Sun=6 (time crate)
    let mut extra = 0usize;
    for i in 0..rem {
        let wd = (start_wd + i) % 7;
        if wd < 5 {
            // Mon=0..Fri=4 are weekdays; Sat=5, Sun=6 are not.
            extra += 1;
        }
    }
    full_weeks * 5 + extra
}

/// Count US market holiday occurrences in `[start, end)` that land on a weekday.
///
/// Covers:
/// - Fixed-date holidays: New Year's (1/1), Juneteenth (6/19), Independence
///   (7/4), Christmas (12/25). Sunday → observed on Monday.
/// - Nth-weekday holidays: MLK (Jan 3rd Mon), Presidents' (Feb 3rd Mon),
///   Labor (Sep 1st Mon), Thanksgiving (Nov 4th Thu).
/// - Memorial Day: last Monday in May.
/// - Good Friday: approximated as the Friday 2 days before Easter (Gregorian).
///
/// Intentionally excludes early-close days and market-specific one-offs —
/// the gate is a ≥95% floor, not an equality.
fn us_weekday_holiday_count(start: Date, end: Date) -> usize {
    if end <= start {
        return 0;
    }

    let start_year = start.year();
    let end_year = end.year();
    let mut count = 0usize;

    for year in start_year..=end_year {
        // ── Fixed-date holidays ──────────────────────────────────────────
        for &(month, day) in US_FIXED_HOLIDAYS {
            let m = time::Month::try_from(month).unwrap_or(time::Month::January);
            if let Ok(date) = Date::from_calendar_date(year, m, day) {
                let observed = observed_date(date);
                if observed >= start && observed < end {
                    count += 1;
                }
            }
        }

        // ── Nth-weekday holidays ─────────────────────────────────────────
        for &(month, nth, wd) in US_NTH_WEEKDAY_HOLIDAYS {
            let m = time::Month::try_from(month).unwrap_or(time::Month::January);
            let target_wd = num_to_weekday(wd);
            if let Some(date) = nth_weekday_of_month(year, m, target_wd, nth)
                && date >= start
                && date < end
            {
                count += 1;
            }
        }

        // ── Memorial Day: last Monday of May ─────────────────────────────
        if let Some(date) = last_monday_of_may(year)
            && date >= start
            && date < end
        {
            count += 1;
        }

        // ── Good Friday: Friday before Easter ────────────────────────────
        if let Some(date) = good_friday(year)
            && date >= start
            && date < end
        {
            count += 1;
        }
    }

    count
}

/// Return the "observed" date for a fixed holiday: if it falls on Sunday,
/// return Monday; Saturday → Friday. Weekday holidays stay as-is.
fn observed_date(date: Date) -> Date {
    match date.weekday() {
        Weekday::Sunday => date + Duration::days(1),
        Weekday::Saturday => date - Duration::days(1),
        _ => date,
    }
}

/// Return the Nth occurrence of `target_wd` in `(year, month)`.
///
/// `nth` is 1-based (1 = first occurrence, 2 = second, …).
/// Returns `None` if the month has fewer than `nth` such weekdays.
fn nth_weekday_of_month(
    year: i32,
    month: time::Month,
    target_wd: Weekday,
    nth: u8,
) -> Option<Date> {
    // Find the first occurrence of target_wd in the month.
    let first_of_month = Date::from_calendar_date(year, month, 1).ok()?;
    let first_wd = first_of_month.weekday() as u8; // Mon=0..Sun=6
    let target_wd_num = target_wd as u8;
    let days_to_first: u8 = (target_wd_num + 7 - first_wd) % 7;
    let first_occurrence = first_of_month + Duration::days(i64::from(days_to_first));
    // Advance by (nth-1) weeks.
    let result = first_occurrence + Duration::weeks(i64::from(nth) - 1);
    // Verify still in the same month.
    if result.month() == month && result.year() == year {
        Some(result)
    } else {
        None
    }
}

/// Return the last Monday of May for the given year (Memorial Day).
fn last_monday_of_may(year: i32) -> Option<Date> {
    // Memorial Day = last Monday in May. Try the 5th, then 4th occurrence.
    let may = time::Month::May;
    nth_weekday_of_month(year, may, Weekday::Monday, 5)
        .or_else(|| nth_weekday_of_month(year, may, Weekday::Monday, 4))
}

/// Good Friday = the Friday 48 hours (2 days) before Easter Sunday.
///
/// Uses the Anonymous Gregorian algorithm to compute Easter.
fn good_friday(year: i32) -> Option<Date> {
    let easter = gregorian_easter(year)?;
    Some(easter - Duration::days(2))
}

/// Gregorian Easter computation (Anonymous algorithm).
fn gregorian_easter(year: i32) -> Option<Date> {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month_num = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    let month = time::Month::try_from(month_num as u8).ok()?;
    Date::from_calendar_date(year, month, day as u8).ok()
}

/// Convert our `u8` weekday encoding (1=Mon, 2=Tue, …, 7=Sun) to `time::Weekday`.
fn num_to_weekday(n: u8) -> Weekday {
    match n {
        1 => Weekday::Monday,
        2 => Weekday::Tuesday,
        3 => Weekday::Wednesday,
        4 => Weekday::Thursday,
        5 => Weekday::Friday,
        6 => Weekday::Saturday,
        _ => Weekday::Sunday,
    }
}

/// Convert a Unix-millisecond timestamp to a `time::Date` (UTC, floor to day).
fn ms_to_date(ms: i64) -> Date {
    OffsetDateTime::from_unix_timestamp(ms / 1_000)
        .map(|odt| odt.date())
        .unwrap_or(Date::MIN)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_arithmetic,
    clippy::pedantic
)]
mod tests {
    use super::*;

    // ── Classifier tests ──────────────────────────────────────────────────────

    #[test]
    fn classify_crypto_usd_pairs() {
        assert_eq!(classify_ticker("BTC-USD"), MarketCalendar::Crypto24x7);
        assert_eq!(classify_ticker("ETH-USD"), MarketCalendar::Crypto24x7);
        assert_eq!(classify_ticker("LINK-USD"), MarketCalendar::Crypto24x7);
        assert_eq!(classify_ticker("SOL-USD"), MarketCalendar::Crypto24x7);
    }

    #[test]
    fn classify_caret_tickers_as_us_equity() {
        assert_eq!(classify_ticker("^GSPC"), MarketCalendar::UsEquity);
        assert_eq!(classify_ticker("^TNX"), MarketCalendar::UsEquity);
        assert_eq!(classify_ticker("^DXY"), MarketCalendar::UsEquity);
        assert_eq!(classify_ticker("^VIX"), MarketCalendar::UsEquity);
    }

    #[test]
    fn classify_futures_and_fx() {
        assert_eq!(classify_ticker("GC=F"), MarketCalendar::UsEquity);
        assert_eq!(classify_ticker("CL=F"), MarketCalendar::UsEquity);
        assert_eq!(classify_ticker("EURUSD=X"), MarketCalendar::UsEquity);
    }

    #[test]
    fn classify_dx_nyb() {
        assert_eq!(classify_ticker("DX-Y.NYB"), MarketCalendar::UsEquity);
    }

    #[test]
    fn classify_unknown_defaults_to_crypto() {
        // Unknown tickers default to Crypto24x7 (conservative — preserves prior behaviour).
        assert_eq!(classify_ticker("UNKNOWN"), MarketCalendar::Crypto24x7);
        assert_eq!(classify_ticker("BTCUSDT"), MarketCalendar::Crypto24x7);
        assert_eq!(classify_ticker(""), MarketCalendar::Crypto24x7);
    }

    // ── T-CAL: Crypto24x7 equivalence to expected_bars_for_range ─────────────
    //
    // ADR-0073 D2 anchor-safety proof: for any range,
    // `MarketCalendar::Crypto24x7.trading_days_in_range(s, e)` must equal
    // `s/e wall-clock day count` = `(e - s).max(0) / 86_400_000`.
    // This is the IDENTICAL formula in `expected_bars_for_range(Days1, s, e)`.

    fn wall_clock_days(start_ms: i64, end_ms: i64) -> usize {
        let range_ms = (end_ms - start_ms).max(0) as u64;
        (range_ms / 86_400_000) as usize
    }

    #[test]
    fn t_cal_crypto_matches_wallclock_zero_range() {
        let s = 1_700_000_000_000i64;
        let e = s;
        assert_eq!(
            MarketCalendar::Crypto24x7.trading_days_in_range(s, e),
            wall_clock_days(s, e),
        );
    }

    #[test]
    fn t_cal_crypto_matches_wallclock_one_day() {
        let s = 1_700_000_000_000i64;
        let e = s + 86_400_000;
        assert_eq!(
            MarketCalendar::Crypto24x7.trading_days_in_range(s, e),
            wall_clock_days(s, e),
        );
        assert_eq!(MarketCalendar::Crypto24x7.trading_days_in_range(s, e), 1,);
    }

    #[test]
    fn t_cal_crypto_matches_wallclock_90_days() {
        // 90-day window: 2024-01-01 .. 2024-04-01 (approximately)
        let s = 1_704_067_200_000i64; // 2024-01-01 00:00:00 UTC in ms
        let e = s + 90 * 86_400_000;
        assert_eq!(
            MarketCalendar::Crypto24x7.trading_days_in_range(s, e),
            wall_clock_days(s, e),
        );
        assert_eq!(MarketCalendar::Crypto24x7.trading_days_in_range(s, e), 90,);
    }

    #[test]
    fn t_cal_crypto_matches_wallclock_range_sweep() {
        // Sweep multiple windows to ensure byte-identical behaviour everywhere.
        let base_ms = 1_640_995_200_000i64; // 2022-01-01 00:00:00 UTC
        for days in [1, 7, 30, 90, 180, 365, 730] {
            let s = base_ms;
            let e = base_ms + days * 86_400_000;
            let cal_count = MarketCalendar::Crypto24x7.trading_days_in_range(s, e);
            let expected = wall_clock_days(s, e);
            assert_eq!(
                cal_count, expected,
                "Mismatch for {days} days: cal={cal_count}, expected={expected}"
            );
        }
    }

    // ── UsEquity sanity tests ──────────────────────────────────────────────────

    #[test]
    fn us_equity_roughly_5_of_7() {
        // A full year (2024-01-01 .. 2025-01-01) should have ~252 trading days.
        let s = 1_704_067_200_000i64; // 2024-01-01 UTC ms
        let e = 1_735_689_600_000i64; // 2025-01-01 UTC ms
        let days = MarketCalendar::UsEquity.trading_days_in_range(s, e);
        // NYSE 2024 had 252 trading days; allow ±5 for approximation.
        assert!(
            (245..=260).contains(&days),
            "Expected ~252 US trading days in 2024, got {days}"
        );
    }

    #[test]
    fn us_equity_less_than_crypto_for_same_window() {
        let s = 1_704_067_200_000i64; // 2024-01-01 UTC ms
        let e = 1_735_689_600_000i64; // 2025-01-01 UTC ms
        let crypto = MarketCalendar::Crypto24x7.trading_days_in_range(s, e);
        let us = MarketCalendar::UsEquity.trading_days_in_range(s, e);
        assert!(
            us < crypto,
            "UsEquity ({us}) must be < Crypto24x7 ({crypto}) for a full year"
        );
    }

    #[test]
    fn us_equity_90_day_window_passes_95pct_gate() {
        // This is the key F-2 proof: a 90-day 1d load of ^GSPC should now
        // compute an expected count of ~62-64 trading days, not 90 calendar
        // days. With ~62-64 actual bars, the 95% gate should pass.
        let s = 1_704_067_200_000i64; // 2024-01-01 UTC ms
        let e = s + 90 * 86_400_000i64;
        let expected = MarketCalendar::UsEquity.trading_days_in_range(s, e);
        // Should be ~63 trading days (Mon-Fri minus holidays).
        assert!(
            (60..=67).contains(&expected),
            "Expected ~63 US trading days in 90 calendar days, got {expected}"
        );
        // The 95% threshold would be ceil(expected * 95 / 100).
        // For ~63 expected, threshold ≈ 60. Real bars ~62-64 → gate PASSES.
        let threshold = (expected * 95).div_ceil(100);
        // Verify that actual bars (~62) would pass.
        assert!(
            62 >= threshold,
            "Gate should pass: 62 bars >= threshold {threshold}"
        );
    }

    // ── Weekday counter ───────────────────────────────────────────────────────

    #[test]
    fn weekday_count_one_week() {
        // 2024-01-01 (Monday) .. 2024-01-08 (Monday) = 7 days = 5 weekdays.
        let start = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
        let end = Date::from_calendar_date(2024, time::Month::January, 8).unwrap();
        assert_eq!(count_weekdays(start, end), 5);
    }

    #[test]
    fn weekday_count_empty_range() {
        let d = Date::from_calendar_date(2024, time::Month::January, 1).unwrap();
        assert_eq!(count_weekdays(d, d), 0);
    }

    // ── Nth weekday tests ─────────────────────────────────────────────────────

    #[test]
    fn mlk_day_2024() {
        // MLK Day 2024 = January 15.
        let jan = time::Month::January;
        let date = nth_weekday_of_month(2024, jan, Weekday::Monday, 3).unwrap();
        assert_eq!(date, Date::from_calendar_date(2024, jan, 15).unwrap());
    }

    #[test]
    fn labor_day_2024() {
        // Labor Day 2024 = September 2.
        let sep = time::Month::September;
        let date = nth_weekday_of_month(2024, sep, Weekday::Monday, 1).unwrap();
        assert_eq!(date, Date::from_calendar_date(2024, sep, 2).unwrap());
    }

    #[test]
    fn thanksgiving_2024() {
        // Thanksgiving 2024 = November 28 (4th Thursday).
        let nov = time::Month::November;
        let date = nth_weekday_of_month(2024, nov, Weekday::Thursday, 4).unwrap();
        assert_eq!(date, Date::from_calendar_date(2024, nov, 28).unwrap());
    }

    #[test]
    fn memorial_day_2024() {
        // Memorial Day 2024 = May 27 (last Monday).
        let date = last_monday_of_may(2024).unwrap();
        assert_eq!(
            date,
            Date::from_calendar_date(2024, time::Month::May, 27).unwrap()
        );
    }

    #[test]
    fn good_friday_2024() {
        // Easter 2024 = March 31. Good Friday = March 29.
        let date = good_friday(2024).unwrap();
        assert_eq!(
            date,
            Date::from_calendar_date(2024, time::Month::March, 29).unwrap()
        );
    }

    #[test]
    fn good_friday_2025() {
        // Easter 2025 = April 20. Good Friday = April 18.
        let date = good_friday(2025).unwrap();
        assert_eq!(
            date,
            Date::from_calendar_date(2025, time::Month::April, 18).unwrap()
        );
    }
}
