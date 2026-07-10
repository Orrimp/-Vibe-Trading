//! Macro-regime exogenous series loader — ADR-0073 D3.
//!
//! `load_macro_regime_series` reads the three pre-registered macro daily
//! series (`^GSPC`, `DX-Y.NYB`, `^TNX`) from the dedicated
//! `data/yahoo-macro/` corpus (NOT `data/yahoo/`) via the UNCHANGED
//! `YahooBarSource::load_cached` read path and reduces them to a single
//! `PitSeries<bool>` — the daily regime flag for the `v0.macro_riskon` arm.
//!
//! # Look-ahead discipline (ADR-0058 / ADR-0073 D4)
//!
//! The LOCF daily→hourly join is `regime.as_of_value(bar.open_ts)` —
//! `PitSeries<bool>`'s only query method returns the record with
//! `ts ≤ query`. Look-ahead is structurally unrepresentable.
//!
//! # Feature gate
//!
//! Compiled only when `--features yahoo` is active (same gate as the
//! Yahoo corpus reader it depends on).

#![cfg(feature = "yahoo")]

use std::collections::BTreeMap;
use std::path::Path;

use rust_decimal::Decimal;
use trading_core::pit::{PitSeries, TimestampMs};

use crate::DateRange;

// ── Pre-registered macro tickers (LOCKED — D2/D4) ─────────────────────────────

/// S&P 500 index — SPX trend-up leg.
const TICKER_SPX: &str = "^GSPC";
/// ICE US Dollar index — dollar-not-bid leg.
const TICKER_DXY: &str = "DX-Y.NYB";
/// 10-year Treasury yield — rates-not-spiking leg.
const TICKER_TNX: &str = "^TNX";

/// SMA lookback for the SPX and DXY legs (daily bars).
const SMA_50: usize = 50;
/// SMA lookback for the TNX leg (daily bars).
const SMA_20: usize = 20;

/// How many extra calendar days BEFORE `start_ms` to extend the macro load.
///
/// SMA(50) requires 50 trading days of warm-up (~72 calendar days).
/// We extend by 100 calendar days to guarantee the SMA is warm from the
/// first coin bar (ADR-0073 D5; the macro corpus starts 2021-01-01 which
/// superset-covers any advisor bake-off window from 2024 onward).
const WARMUP_DAYS: i64 = 100;
const MS_PER_DAY: i64 = 86_400_000;

// ── Error type ─────────────────────────────────────────────────────────────────

#[derive(thiserror::Error, Debug)]
pub enum MacroRegimeError {
    #[error("macro ticker {ticker}: {source}")]
    YahooLoad {
        ticker: &'static str,
        source: data::yahoo::YahooError,
    },

    #[error("PitSeries sort error: {0}")]
    PitSort(trading_core::pit::PitError),
}

// ── Public API ─────────────────────────────────────────────────────────────────

/// Load the 3 pre-registered macro daily series and reduce them to a
/// `PitSeries<bool>` of daily regime flags.
///
/// **Algorithm (look-ahead-free, LOCKED — ADR-0073 D3/D4):**
/// 1. Load `^GSPC`, `DX-Y.NYB`, `^TNX` from `yahoo_root` (the dedicated
///    `data/yahoo-macro/` root) via `YahooBarSource::load_cached(Days1)`.
///    The load window is `[start_ms − WARMUP_DAYS, end_ms)` so the SMA(50)
///    is warm at the first coin bar.
/// 2. Build a trailing SMA for each series using only past closes
///    (no centering, no look-ahead).
/// 3. Evaluate the 3-AND risk-ON rule at the **union** of all three tickers'
///    close timestamps; absent legs carry their prior close forward (LOCF).
/// 4. Emit `(close_ts_ms, bool)` for every macro daily close and build
///    `PitSeries::from_sorted(...)`.
///
/// # Warm-up / None
///
/// If the regime timestamp is before the SMA(50) is warm (fewer than 50
/// bars of history), that timestamp is **excluded** from the series.
/// The arm treats `as_of_value → None` as risk-OFF / flat (D3 step 4).
///
/// # Errors
///
/// Returns `MacroRegimeError` if any ticker fails to load
/// (`YahooError::CacheMiss` → orchestrator must fetch the corpus first;
/// `YahooError::MissingData` → calendar fix not applied / corpus incomplete).
pub fn load_macro_regime_series(
    yahoo_root: &Path,
    range: &DateRange,
) -> Result<PitSeries<bool>, MacroRegimeError> {
    use data::yahoo::{Interval, YahooBarSource};

    // `DateRange` is an enum (not a struct) — convert to (start_ms, end_ms) via
    // the same helper the DVOL arm uses (anchor-safe, look-ahead-free bounds).
    let (start_ms, end_ms) = crate::bakeoff::date_range_to_ms_pair(range);
    // Extend back for SMA warm-up.
    let load_start_ms = start_ms - WARMUP_DAYS * MS_PER_DAY;

    let source = YahooBarSource::new(yahoo_root.to_path_buf());

    // ── Step 1: load closes per ticker ────────────────────────────────────────
    let spx_bars = source
        .load_cached(TICKER_SPX, Interval::Days1, load_start_ms, end_ms)
        .map_err(|source| MacroRegimeError::YahooLoad {
            ticker: TICKER_SPX,
            source,
        })?;

    let dxy_bars = source
        .load_cached(TICKER_DXY, Interval::Days1, load_start_ms, end_ms)
        .map_err(|source| MacroRegimeError::YahooLoad {
            ticker: TICKER_DXY,
            source,
        })?;

    let tnx_bars = source
        .load_cached(TICKER_TNX, Interval::Days1, load_start_ms, end_ms)
        .map_err(|source| MacroRegimeError::YahooLoad {
            ticker: TICKER_TNX,
            source,
        })?;

    // ── Step 2: build (close_ts_ms → close_decimal) maps ─────────────────────
    // `bar.close_ts` is the per-bar close timestamp.  For daily Yahoo bars
    // `close_ts` is approximately end-of-day UTC.  We key the regime record
    // by `close_ts` so the as-of join `regime.as_of_value(bar.open_ts)` only
    // sees a macro close AFTER it is fully observed — look-ahead-free.
    let spx_closes = bars_to_close_map(&spx_bars.bars);
    let dxy_closes = bars_to_close_map(&dxy_bars.bars);
    let tnx_closes = bars_to_close_map(&tnx_bars.bars);

    // ── Step 3: union of close timestamps → evaluate regime ───────────────────
    let mut all_ts: std::collections::BTreeSet<i64> = std::collections::BTreeSet::new();
    all_ts.extend(spx_closes.keys().copied());
    all_ts.extend(dxy_closes.keys().copied());
    all_ts.extend(tnx_closes.keys().copied());

    // Compute trailing SMAs per ticker.  We walk through the sorted union of
    // timestamps; for each timestamp we check if the ticker has a new close
    // (and append it to the window) or carry the prior close forward.
    let mut spx_window: Vec<Decimal> = Vec::new();
    let mut dxy_window: Vec<Decimal> = Vec::new();
    let mut tnx_window: Vec<Decimal> = Vec::new();

    // Last known closes (for LOCF across missing ticks).
    let mut spx_last: Option<Decimal> = None;
    let mut dxy_last: Option<Decimal> = None;
    let mut tnx_last: Option<Decimal> = None;

    let mut regime_records: Vec<(TimestampMs, bool)> = Vec::new();

    for &ts in &all_ts {
        // Update close + window if a new bar arrived at this timestamp.
        if let Some(&close) = spx_closes.get(&ts) {
            spx_last = Some(close);
            spx_window.push(close);
        }
        if let Some(&close) = dxy_closes.get(&ts) {
            dxy_last = Some(close);
            dxy_window.push(close);
        }
        if let Some(&close) = tnx_closes.get(&ts) {
            tnx_last = Some(close);
            tnx_window.push(close);
        }

        // Only emit regime record if all three legs have >= their required warm-up.
        let spx_ok = spx_window.len() >= SMA_50 && spx_last.is_some();
        let dxy_ok = dxy_window.len() >= SMA_50 && dxy_last.is_some();
        let tnx_ok = tnx_window.len() >= SMA_20 && tnx_last.is_some();

        if !spx_ok || !dxy_ok || !tnx_ok {
            // Warm-up — don't emit; arm will treat None as risk-OFF (flat).
            continue;
        }

        // Trailing SMAs (past-only; current bar IS included in the window).
        let spx_sma = trailing_sma(&spx_window, SMA_50);
        let dxy_sma = trailing_sma(&dxy_window, SMA_50);
        let tnx_sma = trailing_sma(&tnx_window, SMA_20);

        // Pre-registered 3-AND risk-ON rule (LOCKED — ADR-0073 D4):
        // 1. SPX close > SMA(SPX, 50)   — trend up
        // 2. DXY close < SMA(DXY, 50)   — dollar not bid
        // 3. TNX close < SMA(TNX, 20)   — rates not spiking
        let spx_close = spx_last.unwrap_or(Decimal::ZERO);
        let dxy_close = dxy_last.unwrap_or(Decimal::ZERO);
        let tnx_close = tnx_last.unwrap_or(Decimal::ZERO);

        let risk_on = spx_close > spx_sma && dxy_close < dxy_sma && tnx_close < tnx_sma;

        // Emit regime record keyed by this macro daily close timestamp.
        // Only emit records within [start_ms, end_ms) to keep the series tight;
        // earlier warm-up records are used for SMA computation but not emitted.
        if ts >= start_ms && ts < end_ms {
            regime_records.push((TimestampMs(ts), risk_on));
        }
    }

    // Records are produced in ascending timestamp order (BTreeSet).
    //
    // Explicit publication lag (ADR-0086 D2/D3, P3 M-DEV-6): macro's
    // publication_lag_ms = 0 per the feature's lag table
    // (spec/v3/advisor-pit-discipline/feature.md § D2) — the three legs
    // (^GSPC/DX-Y.NYB/^TNX) are market-observable prices/yields with no
    // release lag beyond end-of-day, and `close_ts` ≈ EOD UTC already
    // encodes that. `from_sorted_with_lag(_, 0)` is byte-identical to
    // `from_sorted(_)` (proven by
    // `macro_byte_identical_legacy_vs_with_lag_zero`).
    PitSeries::from_sorted_with_lag(regime_records, 0).map_err(MacroRegimeError::PitSort)
}

// ── Helpers ────────────────────────────────────────────────────────────────────

/// Convert bars to a `BTreeMap<close_ts_ms, close_decimal>`.
///
/// Duplicate close timestamps are resolved by keeping the LAST one
/// (in bar order) — consistent with the `BTreeMap` update semantics.
fn bars_to_close_map(bars: &[trading_core::Bar]) -> BTreeMap<i64, Decimal> {
    let mut map = BTreeMap::new();
    for bar in bars {
        let close_ts_ms = bar.close_ts.unix_millis();
        let close_dec = bar.close.get();
        map.insert(close_ts_ms, close_dec);
    }
    map
}

/// Compute the trailing mean of the last `window` values in `data`.
///
/// Returns `Decimal::ZERO` if `data` is empty (should not happen after the
/// warm-up guard, but defensive).
fn trailing_sma(data: &[Decimal], window: usize) -> Decimal {
    if data.is_empty() {
        return Decimal::ZERO;
    }
    let n = window.min(data.len());
    let slice = &data[data.len() - n..];
    let sum: Decimal = slice.iter().copied().sum();
    sum / Decimal::from(n)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::float_arithmetic,
    clippy::pedantic,
    clippy::cast_precision_loss
)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    // ── trailing_sma ──────────────────────────────────────────────────────────

    #[test]
    fn trailing_sma_full_window() {
        // 5-element series; SMA(3) of last 3 = (3+4+5)/3 = 4.
        let data = vec![dec!(1), dec!(2), dec!(3), dec!(4), dec!(5)];
        let result = trailing_sma(&data, 3);
        assert_eq!(result, dec!(4));
    }

    #[test]
    fn trailing_sma_window_larger_than_data() {
        // When window > data.len(), use all data.
        let data = vec![dec!(2), dec!(4)];
        let result = trailing_sma(&data, 10);
        assert_eq!(result, dec!(3));
    }

    #[test]
    fn trailing_sma_empty() {
        let result = trailing_sma(&[], 5);
        assert_eq!(result, Decimal::ZERO);
    }

    // ── bars_to_close_map ─────────────────────────────────────────────────────

    #[test]
    fn bars_to_close_map_empty() {
        let map = bars_to_close_map(&[]);
        assert!(map.is_empty());
    }

    // ── Risk-ON logic unit test (synthetic bars) ──────────────────────────────
    //
    // We test the 3-AND rule by constructing the windows manually and verifying
    // the boolean outcome. This is cheaper than loading real parquets.

    #[test]
    fn risk_on_when_all_three_conditions_met() {
        // SPX above SMA, DXY below SMA, TNX below SMA → risk-ON.
        let spx_window: Vec<Decimal> = (1..=50).map(Decimal::from).collect(); // SMA = 25.5; last = 50 > 25.5 ✓
        // DXY window where last < SMA (risk-ON requires the dollar NOT bid):
        let dxy_descending: Vec<Decimal> = (1..=50).map(|i| Decimal::from(100 - i)).collect(); // 99..50; last=50; SMA~74.5 → 50 < 74.5 ✓
        let tnx_window: Vec<Decimal> = (1..=20).map(|i| Decimal::from(30 - i)).collect(); // 29..10; last=10; SMA~19.5 → 10 < 19.5 ✓

        let spx_sma = trailing_sma(&spx_window, SMA_50);
        let dxy_sma = trailing_sma(&dxy_descending, SMA_50);
        let tnx_sma = trailing_sma(&tnx_window, SMA_20);

        let spx_close = *spx_window.last().unwrap();
        let dxy_close = *dxy_descending.last().unwrap();
        let tnx_close = *tnx_window.last().unwrap();

        let risk_on = spx_close > spx_sma && dxy_close < dxy_sma && tnx_close < tnx_sma;
        assert!(risk_on, "All three conditions met → should be risk-ON");
    }

    #[test]
    fn risk_off_when_spx_below_sma() {
        // SPX descending: close < SMA → risk-OFF.
        let spx_window: Vec<Decimal> = (1..=50).map(|i| Decimal::from(50 - i)).collect(); // last=0; SMA=24.5 → 0 < 24.5 → NOT above
        let dxy_descending: Vec<Decimal> = (1..=50).map(|i| Decimal::from(100 - i)).collect();
        let tnx_window: Vec<Decimal> = (1..=20).map(|i| Decimal::from(30 - i)).collect();

        let spx_sma = trailing_sma(&spx_window, SMA_50);
        let dxy_sma = trailing_sma(&dxy_descending, SMA_50);
        let tnx_sma = trailing_sma(&tnx_window, SMA_20);

        let spx_close = *spx_window.last().unwrap();
        let dxy_close = *dxy_descending.last().unwrap();
        let tnx_close = *tnx_window.last().unwrap();

        let risk_on = spx_close > spx_sma && dxy_close < dxy_sma && tnx_close < tnx_sma;
        assert!(!risk_on, "SPX below SMA → should be risk-OFF");
    }

    // ── Byte-identity test (ADR-0086 D3 / P3 M-TEST-3) ────────────────────────

    /// Proves the P3 retrofit (`PitSeries::from_sorted` →
    /// `from_sorted_with_lag(_, 0)` in `load_macro_regime_series`) moves NO
    /// value on a representative regime-record series + bar-open grid: the
    /// LEGACY raw `partition_point(|&(t,_)| t <= q)` predicate computed
    /// directly over the regime records must equal the RETROFITTED
    /// `PitSeries::from_sorted_with_lag(_, 0).as_of_value(_)` (the exact
    /// primitive the loader now calls) element-for-element.
    ///
    /// This tests the reduction at the `PitSeries` level rather than through
    /// the full `load_macro_regime_series` (which requires the Yahoo
    /// corpus I/O) — a synthetic `(TimestampMs, bool)` series is
    /// representative because the retrofit only changes HOW the series is
    /// constructed (`from_sorted` → `from_sorted_with_lag(_, 0)`), not what
    /// `regime_records` contains. Because macro also runs
    /// `write_report = false`, no anchored report body can move; this test
    /// is the load-bearing proof that the as-of VALUES are unchanged.
    #[test]
    fn macro_byte_identical_legacy_vs_with_lag_zero() {
        let one_day_ms: i64 = 86_400_000;
        let regime_records: Vec<(TimestampMs, bool)> = vec![
            (TimestampMs(one_day_ms - 1), true),
            (TimestampMs(2 * one_day_ms - 1), false),
            (TimestampMs(3 * one_day_ms - 1), true),
            (TimestampMs(3 * one_day_ms - 1), false), // tie: second wins
        ];

        // A representative bar-open grid: warm-up, exact boundary, between,
        // and past-last-record.
        let grid: Vec<i64> = vec![
            0,
            one_day_ms - 2,
            one_day_ms - 1,
            one_day_ms,
            2 * one_day_ms - 1,
            2 * one_day_ms,
            3 * one_day_ms - 1,
            3 * one_day_ms,
            10 * one_day_ms,
        ];

        // LEGACY: the exact raw predicate the primitive replaced (pinned by
        // `pit.rs`'s own docstring/tests as byte-for-byte identical at
        // lag=0; recomputed here directly over the tuples for the
        // independent oracle this test requires).
        let legacy_as_of = |query: i64| -> Option<bool> {
            let idx = regime_records.partition_point(|&(t, _)| t.0 <= query); // PIT-OK: legacy-predicate byte-identity oracle for the M-TEST-3 retrofit proof.
            if idx == 0 {
                None
            } else {
                Some(regime_records[idx - 1].1)
            }
        };
        let legacy_results: Vec<Option<bool>> = grid.iter().map(|&q| legacy_as_of(q)).collect();

        // RETROFITTED: the exact primitive load_macro_regime_series now calls.
        let retrofitted =
            PitSeries::from_sorted_with_lag(regime_records.clone(), 0).expect("sorted fixture");
        let retrofitted_results: Vec<Option<bool>> = grid
            .iter()
            .map(|&q| retrofitted.as_of_value(TimestampMs(q)))
            .collect();

        assert_eq!(
            legacy_results, retrofitted_results,
            "P3 retrofit must be byte-identical to the legacy raw partition_point predicate"
        );
    }
}
