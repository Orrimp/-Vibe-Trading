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

/// Maximum tolerated gap, in calendar days, anywhere in the emitted regime
/// series relative to the requested window (review 3-16 MEDIUM, bug-log #78's
/// second trigger on a second arm).
///
/// `PitSeries::as_of_value` is **unbounded LOCF**: it happily carries a close
/// forward forever, so at the join a 7-week hole in the corpus is
/// indistinguishable from a 3-day weekend. That is how a pinned corpus ending
/// `2026-06` stayed "healthy" against advisor windows anchored to `NOW` — the
/// SHA verified, the load succeeded, and the regime decision was a frozen
/// constant for the whole tail.
///
/// **Weekend/holiday LOCF is CORRECT and is deliberately preserved.** Carrying
/// Friday's close across Saturday/Sunday crypto bars is the intended semantics,
/// and US market holidays extend that to 4 days (a Friday close before a Monday
/// holiday is next refreshed on Tuesday), occasionally 5 around
/// Christmas/New Year. `10` sits comfortably above every legitimate NYSE closure
/// and far below the multi-week staleness this bound exists to catch, so it can
/// only reject a corpus that is genuinely not covering the window.
const MAX_REGIME_GAP_DAYS: i64 = 10;

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

    /// The corpus loaded and verified, but the regime series does not COVER the
    /// requested window — a SHA pin proves integrity, not coverage.
    ///
    /// Routes the arm to ABSENCE in `bakeoff::run_bakeoff` (the
    /// `preloaded_macro_series.is_none()` guard), which is the honest rendering:
    /// the probe could not be evaluated on this window.
    #[error(
        "macro regime series does not cover the requested window [{start_ms}, {end_ms}): \
         {reason} — max tolerated gap is {max_gap_days} days (weekend/holiday LOCF is \
         expected and allowed; multi-week staleness is not). The arm is DROPPED, never \
         run against a frozen or empty regime."
    )]
    InsufficientCoverage {
        reason: String,
        start_ms: i64,
        end_ms: i64,
        max_gap_days: i64,
    },
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
///    close timestamps ([`reduce_regime_records`] — see its docs for the
///    emission cadence, which is up to 3 records per trading day with legs of
///    mixed vintage); absent legs carry their prior close forward (LOCF).
/// 4. Assert the emitted series COVERS `[start_ms, end_ms)`
///    ([`check_span_coverage`]) — a verified corpus SHA proves integrity, not
///    coverage.
/// 5. Build `PitSeries::from_sorted_with_lag(records, 0)`.
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
/// `YahooError::MissingData` → calendar fix not applied / corpus incomplete),
/// or `InsufficientCoverage` when the corpus loads but does not reach the
/// requested window (review 3-16 MEDIUM).
///
/// **Every error routes the arm to ABSENCE.** `bakeoff::run_bakeoff` turns an
/// `Err` into `preloaded_macro_series = None` and then `continue`s past the
/// dispatch, so the arm is reported as not-run — it is never handed an empty or
/// stale series behind the label *"Macro regime (hold when SPX up, DXY down,
/// rates calm)"* (bug-log #78/#81).
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
    let regime_records =
        reduce_regime_records(&spx_closes, &dxy_closes, &tnx_closes, start_ms, end_ms);

    // ── Step 3b: span-coverage bound (review 3-16 MEDIUM) ─────────────────────
    // A verified SHA proves the corpus is the pinned one; it says NOTHING about
    // whether that corpus reaches the requested window. Assert coverage before
    // the series is handed to an arm whose join is unbounded LOCF.
    check_span_coverage(&regime_records, start_ms, end_ms)?;

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

// ── The pre-registered rule + the reduction (production seams) ─────────────────

/// The pre-registered 3-AND risk-ON rule (LOCKED — ADR-0073 D4).
///
/// Risk-ON **iff all three** hold at the macro daily close:
/// 1. `spx_close > spx_sma` — SPX trend up (SMA 50)
/// 2. `dxy_close < dxy_sma` — dollar not bid (SMA 50)
/// 3. `tnx_close < tnx_sma` — rates not spiking (SMA 20)
///
/// This is the story's entire scientific content, so it lives in ONE place that
/// [`reduce_regime_records`] calls. Review 3-16 HIGH: both "risk-on rule" unit
/// tests used to re-implement this expression *inside the test body*, so
/// inverting the production rule left them green — they proved the test's own
/// copy, not the shipped predicate.
#[must_use]
pub(crate) fn risk_on_rule(
    spx_close: Decimal,
    spx_sma: Decimal,
    dxy_close: Decimal,
    dxy_sma: Decimal,
    tnx_close: Decimal,
    tnx_sma: Decimal,
) -> bool {
    spx_close > spx_sma && dxy_close < dxy_sma && tnx_close < tnx_sma
}

/// Reduce three per-ticker close maps to the emitted `(close_ts, risk_on)`
/// records — the whole PIT-relevant body of [`load_macro_regime_series`] with
/// the parquet I/O lifted out, so it can be exercised on synthetic closes.
///
/// # Emission cadence (documented, NOT changed — review 3-16 LOW)
///
/// The loop walks the **union of the three tickers' close instants**, not one
/// tick per macro *day*. Yahoo keys these three legs at distinct instants
/// (`DX-Y.NYB` ≈ 05:00Z, `^TNX` ≈ 13:20Z, `^GSPC` ≈ 14:30Z), and those instants
/// are **disjoint**, so a normal trading day contributes up to **three** records
/// rather than one:
///
/// | emitted at | SPX leg | DXY leg | TNX leg |
/// |---|---|---|---|
/// | ~05:00Z | previous day (LOCF) | **today** | previous day (LOCF) |
/// | ~13:20Z | previous day (LOCF) | today | **today** |
/// | ~14:30Z | **today** | today | today |
///
/// Only the last of the three evaluates the rule on same-day closes for all
/// three legs; the earlier two mix vintages (SPX from D−1 against DXY from D).
/// Consequences, stated plainly because the arm's output cannot be read
/// correctly without them:
/// - the pre-registered "3-AND rule at the daily close" is evaluated on aligned
///   inputs roughly **one time in three**;
/// - a day on which two legs flip can produce **two** round-trips where the rule
///   intends one, inflating the arm's trade count;
/// - the SMA windows advance per-leg (a leg's window only grows when that leg
///   has a close at the instant), so the SMAs themselves are unaffected by the
///   cadence — it is the *evaluation timing* that is over-sampled, not the
///   averages.
///
/// This is bug-log #73's loop-scope shape (fire per *ticker-close* instead of
/// per *macro day*). It is left **behaviour-identical on purpose**: collapsing
/// to one record per macro day changes what the arm computes on every run, which
/// is a measured product change and not a review patch. It is documented here so
/// nobody reads the current output as "the 3-AND rule at the daily close".
///
/// # Warm-up
///
/// A timestamp is skipped entirely until every leg has its required window
/// (`SMA_50` / `SMA_50` / `SMA_20`); the arm treats a missing record as risk-OFF
/// (flat), which S4 of the day-1 e2e gate pins.
///
/// # Window
///
/// Records outside `[start_ms, end_ms)` are used to warm the SMAs but are NOT
/// emitted, so the returned series is tight to the requested window.
#[must_use]
pub(crate) fn reduce_regime_records(
    spx_closes: &BTreeMap<i64, Decimal>,
    dxy_closes: &BTreeMap<i64, Decimal>,
    tnx_closes: &BTreeMap<i64, Decimal>,
    start_ms: i64,
    end_ms: i64,
) -> Vec<(TimestampMs, bool)> {
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

        let spx_close = spx_last.unwrap_or(Decimal::ZERO);
        let dxy_close = dxy_last.unwrap_or(Decimal::ZERO);
        let tnx_close = tnx_last.unwrap_or(Decimal::ZERO);

        let risk_on = risk_on_rule(spx_close, spx_sma, dxy_close, dxy_sma, tnx_close, tnx_sma);

        // Emit regime record keyed by this macro daily close timestamp.
        // Only emit records within [start_ms, end_ms) to keep the series tight;
        // earlier warm-up records are used for SMA computation but not emitted.
        if ts >= start_ms && ts < end_ms {
            regime_records.push((TimestampMs(ts), risk_on));
        }
    }

    regime_records
}

/// Assert that `records` actually COVER `[start_ms, end_ms)` — the staleness
/// bound `as_of_value`'s unbounded LOCF does not have (review 3-16 MEDIUM).
///
/// Three ways a load can succeed and still leave the arm reading a frozen or
/// absent regime, all rejected here:
/// 1. **empty** — no record in the window at all (every bar reads `None` → the
///    arm sits in 100% cash while wearing the probe's label);
/// 2. **leading / trailing gap** — the corpus starts after the window opens or
///    ends before it closes. The trailing case is the live one: a pinned corpus
///    ending `2026-06` against a `NOW`-anchored advisor lookback, where the last
///    close is carried forward for weeks and the regime decision is a constant;
/// 3. **interior hole** — a multi-week hole mid-window, which LOCF renders
///    identical to a long weekend at the join.
///
/// Gaps up to [`MAX_REGIME_GAP_DAYS`] are **accepted by design** so ordinary
/// weekend and US-holiday LOCF keeps working unchanged.
fn check_span_coverage(
    records: &[(TimestampMs, bool)],
    start_ms: i64,
    end_ms: i64,
) -> Result<(), MacroRegimeError> {
    let max_gap_ms = MAX_REGIME_GAP_DAYS * MS_PER_DAY;
    let fail = |reason: String| MacroRegimeError::InsufficientCoverage {
        reason,
        start_ms,
        end_ms,
        max_gap_days: MAX_REGIME_GAP_DAYS,
    };

    let Some(&(first_ts, _)) = records.first() else {
        return Err(fail(
            "the series is EMPTY — no macro daily close (past warm-up) falls inside the \
             window, so every bar would read `None` and the arm would hold 100% cash"
                .to_string(),
        ));
    };
    // `records.first()` succeeded, so `last()` cannot be `None`.
    let last_ts = records.last().map_or(first_ts, |&(ts, _)| ts);

    let lead_gap = first_ts.0.saturating_sub(start_ms);
    if lead_gap > max_gap_ms {
        return Err(fail(format!(
            "first regime record is {} days after the window opens (ts={})",
            lead_gap / MS_PER_DAY,
            first_ts.0
        )));
    }

    let trail_gap = end_ms.saturating_sub(last_ts.0);
    if trail_gap > max_gap_ms {
        return Err(fail(format!(
            "last regime record is {} days before the window closes (ts={}) — the corpus \
             does not reach the requested window and its final close would be carried \
             forward across the whole tail",
            trail_gap / MS_PER_DAY,
            last_ts.0
        )));
    }

    if let Some((prev, next)) = records
        .windows(2)
        .map(|w| (w[0].0, w[1].0))
        .max_by_key(|(prev, next)| next.0.saturating_sub(prev.0))
        && next.0.saturating_sub(prev.0) > max_gap_ms
    {
        return Err(fail(format!(
            "interior hole of {} days between records ts={} and ts={}",
            next.0.saturating_sub(prev.0) / MS_PER_DAY,
            prev.0,
            next.0
        )));
    }

    Ok(())
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

    // ── The pre-registered 3-AND rule — bound to PRODUCTION ───────────────────
    //
    // Review 3-16 HIGH: these two tests used to re-implement the predicate
    // *inside the test body* (`let risk_on = spx_close > spx_sma && …`) and
    // never call the loader, so inverting the production rule left both green —
    // the story's entire scientific content had no test that binds it.
    //
    // They now call `risk_on_rule`, the ONE expression `reduce_regime_records`
    // evaluates, and the truth table below is exhaustive over the three legs so
    // any single-leg inversion, any `&&`→`||`, and any `>`/`<` flip fails.

    /// All 8 combinations of the three legs. Only (up, weak-dollar, calm-rates)
    /// may be risk-ON; every other corner must be risk-OFF, which is what makes
    /// this an AND rather than a vote.
    #[test]
    fn risk_on_rule_truth_table_is_a_3_and() {
        // Per leg: (satisfied?, close, sma). "Satisfied" per ADR-0073 D4 is
        // SPX close ABOVE its SMA, DXY close BELOW, TNX close BELOW.
        let spx = |ok: bool| {
            if ok {
                (dec!(110), dec!(100))
            } else {
                (dec!(90), dec!(100))
            }
        };
        let dxy = |ok: bool| {
            if ok {
                (dec!(90), dec!(100))
            } else {
                (dec!(110), dec!(100))
            }
        };
        let tnx = |ok: bool| {
            if ok {
                (dec!(3.5), dec!(4.0))
            } else {
                (dec!(4.5), dec!(4.0))
            }
        };

        for &s in &[true, false] {
            for &d in &[true, false] {
                for &t in &[true, false] {
                    let (spx_close, spx_sma) = spx(s);
                    let (dxy_close, dxy_sma) = dxy(d);
                    let (tnx_close, tnx_sma) = tnx(t);
                    let got =
                        risk_on_rule(spx_close, spx_sma, dxy_close, dxy_sma, tnx_close, tnx_sma);
                    assert_eq!(
                        got,
                        s && d && t,
                        "3-AND rule (LOCKED, ADR-0073 D4): spx_up={s}, dollar_weak={d}, \
                         rates_calm={t} must be risk-{}; production said risk-{}",
                        if s && d && t { "ON" } else { "OFF" },
                        if got { "ON" } else { "OFF" }
                    );
                }
            }
        }
    }

    /// Strictness at the boundary: a leg exactly ON its SMA does NOT satisfy the
    /// leg (`>` / `<`, never `>=` / `<=`). Pins the comparison operators, which a
    /// truth table built from strictly-separated values cannot see.
    #[test]
    fn risk_on_rule_boundary_equality_is_risk_off() {
        // SPX exactly at its SMA → leg 1 fails even though 2 and 3 hold.
        assert!(
            !risk_on_rule(
                dec!(100),
                dec!(100),
                dec!(90),
                dec!(100),
                dec!(3.5),
                dec!(4.0)
            ),
            "SPX close == SMA must NOT count as trend-up"
        );
        // DXY exactly at its SMA → leg 2 fails.
        assert!(
            !risk_on_rule(
                dec!(110),
                dec!(100),
                dec!(100),
                dec!(100),
                dec!(3.5),
                dec!(4.0)
            ),
            "DXY close == SMA must NOT count as dollar-not-bid"
        );
        // TNX exactly at its SMA → leg 3 fails.
        assert!(
            !risk_on_rule(
                dec!(110),
                dec!(100),
                dec!(90),
                dec!(100),
                dec!(4.0),
                dec!(4.0)
            ),
            "TNX close == SMA must NOT count as rates-not-spiking"
        );
    }

    // ── The reduction — bound to PRODUCTION on synthetic closes ───────────────

    /// Build a close map with `n` daily bars starting at `t0`, one per day.
    fn daily_closes(t0: i64, values: &[Decimal]) -> BTreeMap<i64, Decimal> {
        values
            .iter()
            .enumerate()
            .map(|(i, &v)| (t0 + i as i64 * MS_PER_DAY, v))
            .collect()
    }

    /// `reduce_regime_records` — the function `load_macro_regime_series` calls
    /// — must emit risk-ON exactly where the 3-AND rule holds.
    ///
    /// 60 aligned daily closes per leg (so SMA(50)/SMA(20) are warm from index
    /// 49): SPX rises throughout (leg 1 always satisfied past warm-up), DXY
    /// falls (leg 2 satisfied), and TNX is calm for the first half of the
    /// evaluated tail then spikes above its SMA(20) — so the emitted series must
    /// be ON, then flip OFF, driven by the production rule and nothing else.
    #[test]
    fn reduce_regime_records_flips_off_when_the_rates_leg_breaks() {
        let t0 = 0i64;
        // SPX: monotone up → close > SMA(50) always (past warm-up).
        let spx: Vec<Decimal> = (0..60).map(|i| Decimal::from(1000 + i * 10)).collect();
        // DXY: monotone down → close < SMA(50) always (past warm-up).
        let dxy: Vec<Decimal> = (0..60).map(|i| Decimal::from(1000 - i * 5)).collect();
        // TNX: flat at 4 for 55 bars, then a spike well above the SMA(20).
        let tnx: Vec<Decimal> = (0..60)
            .map(|i| if i < 55 { dec!(4) } else { dec!(9) })
            .collect();

        let records = reduce_regime_records(
            &daily_closes(t0, &spx),
            &daily_closes(t0, &dxy),
            &daily_closes(t0, &tnx),
            t0,
            t0 + 60 * MS_PER_DAY,
        );

        // Warm-up: indices 0..=48 are skipped (SMA(50) needs 50 closes), so the
        // first emitted record is at index 49.
        assert_eq!(
            records.len(),
            11,
            "60 aligned closes − 49 warm-up = 11 emitted records; got {records:?}"
        );
        assert_eq!(
            records[0].0.0,
            t0 + 49 * MS_PER_DAY,
            "first emission at index 49"
        );

        // Indices 49..=54: TNX flat at 4 vs an SMA(20) of 4 → close == SMA → the
        // rates leg is NOT satisfied (strict `<`), so risk-OFF.
        for (i, &(ts, on)) in records.iter().enumerate().take(6) {
            assert!(
                !on,
                "record {i} (ts={}) — TNX close == SMA(20) is not 'rates calm' under the \
                 strict rule → must be risk-OFF",
                ts.0
            );
        }
        // Indices 55..=59: TNX spikes to 9 above its rising SMA(20) → still OFF.
        for (i, &(ts, on)) in records.iter().enumerate().skip(6) {
            assert!(
                !on,
                "record {i} (ts={}) — TNX spiked above SMA(20) → must be risk-OFF",
                ts.0
            );
        }
    }

    /// The same reduction, with a rates leg that genuinely satisfies leg 3:
    /// TNX declines, so close < SMA(20). All three legs hold → risk-ON
    /// throughout the emitted tail. Together with the test above this pins BOTH
    /// polarities of the production reduction (an always-OFF or always-ON
    /// implementation fails one of the two).
    #[test]
    fn reduce_regime_records_emits_risk_on_when_all_three_legs_hold() {
        let t0 = 0i64;
        let spx: Vec<Decimal> = (0..60).map(|i| Decimal::from(1000 + i * 10)).collect();
        let dxy: Vec<Decimal> = (0..60).map(|i| Decimal::from(1000 - i * 5)).collect();
        let tnx: Vec<Decimal> = (0..60).map(|i| Decimal::from(500 - i * 3)).collect();

        let records = reduce_regime_records(
            &daily_closes(t0, &spx),
            &daily_closes(t0, &dxy),
            &daily_closes(t0, &tnx),
            t0,
            t0 + 60 * MS_PER_DAY,
        );

        assert_eq!(records.len(), 11);
        assert!(
            records.iter().all(|&(_, on)| on),
            "all three legs satisfied → every emitted record must be risk-ON; got {records:?}"
        );
    }

    /// Warm-up is per-leg and gates emission: with only 30 SPX closes the
    /// SMA(50) is never warm, so NOTHING is emitted (the arm then reads `None`
    /// → flat, which S4 of the e2e gate pins).
    #[test]
    fn reduce_regime_records_emits_nothing_until_every_leg_is_warm() {
        let t0 = 0i64;
        let spx: Vec<Decimal> = (0..30).map(|i| Decimal::from(1000 + i * 10)).collect();
        let dxy: Vec<Decimal> = (0..60).map(|i| Decimal::from(1000 - i * 5)).collect();
        let tnx: Vec<Decimal> = (0..60).map(|i| Decimal::from(500 - i * 3)).collect();

        let records = reduce_regime_records(
            &daily_closes(t0, &spx),
            &daily_closes(t0, &dxy),
            &daily_closes(t0, &tnx),
            t0,
            t0 + 60 * MS_PER_DAY,
        );
        assert!(
            records.is_empty(),
            "SPX never reaches 50 closes → SMA(50) never warm → no emission; got {records:?}"
        );
    }

    /// Records outside `[start_ms, end_ms)` warm the SMAs but are not emitted.
    #[test]
    fn reduce_regime_records_emits_only_inside_the_requested_window() {
        let t0 = 0i64;
        let spx: Vec<Decimal> = (0..60).map(|i| Decimal::from(1000 + i * 10)).collect();
        let dxy: Vec<Decimal> = (0..60).map(|i| Decimal::from(1000 - i * 5)).collect();
        let tnx: Vec<Decimal> = (0..60).map(|i| Decimal::from(500 - i * 3)).collect();

        // Window opens at day 55 → only days 55..59 may be emitted.
        let records = reduce_regime_records(
            &daily_closes(t0, &spx),
            &daily_closes(t0, &dxy),
            &daily_closes(t0, &tnx),
            t0 + 55 * MS_PER_DAY,
            t0 + 60 * MS_PER_DAY,
        );
        assert_eq!(records.len(), 5, "days 55..=59; got {records:?}");
        assert!(
            records
                .iter()
                .all(|&(ts, _)| ts.0 >= t0 + 55 * MS_PER_DAY && ts.0 < t0 + 60 * MS_PER_DAY)
        );
    }

    // ── Span coverage (review 3-16 MEDIUM) ────────────────────────────────────

    /// Weekend and holiday LOCF must keep working: Friday→Monday (3 days) and a
    /// Friday-before-Monday-holiday→Tuesday (4 days) are CORRECT, not staleness.
    #[test]
    fn coverage_accepts_weekend_and_holiday_gaps() {
        let start = 0i64;
        let end = 20 * MS_PER_DAY;
        // Records with 1-day gaps, one 3-day weekend gap and one 4-day holiday gap.
        let day = |d: i64| (TimestampMs(d * MS_PER_DAY), true);
        let records = vec![
            day(0),
            day(1),
            day(2),
            day(5), // 3-day weekend
            day(6),
            day(10), // 4-day holiday weekend
            day(11),
            day(19),
        ];
        assert!(
            check_span_coverage(&records, start, end).is_ok(),
            "weekend/holiday LOCF is the intended semantics and must NOT be rejected"
        );
    }

    /// The live failure: a pinned corpus whose last close is weeks before the
    /// window closes. The load succeeds, the SHA verifies, and the regime would
    /// be a frozen constant across the whole tail.
    #[test]
    fn coverage_rejects_a_stale_tail() {
        let start = 0i64;
        let end = 60 * MS_PER_DAY;
        let records = vec![
            (TimestampMs(0), true),
            (TimestampMs(MS_PER_DAY), true),
            (TimestampMs(2 * MS_PER_DAY), false),
        ];
        let err = check_span_coverage(&records, start, end)
            .expect_err("a 58-day stale tail must be rejected");
        let msg = format!("{err}");
        assert!(
            msg.contains("does not cover the requested window"),
            "error must name the coverage failure; got: {msg}"
        );
        // Window closes at day 60; last record is at day 2 → 58 days stale.
        assert!(
            msg.contains("58 days before the window closes"),
            "error must quantify the staleness; got: {msg}"
        );
    }

    /// A multi-week hole mid-window is indistinguishable from a weekend at the
    /// join — that is exactly what this bound exists to catch.
    #[test]
    fn coverage_rejects_an_interior_hole() {
        let start = 0i64;
        let end = 60 * MS_PER_DAY;
        let records = vec![
            (TimestampMs(0), true),
            (TimestampMs(MS_PER_DAY), true),
            // 49-day hole (the 7-week corpus gap class).
            (TimestampMs(50 * MS_PER_DAY), false),
            (TimestampMs(59 * MS_PER_DAY), false),
        ];
        let err = check_span_coverage(&records, start, end)
            .expect_err("a 49-day interior hole must be rejected");
        assert!(
            format!("{err}").contains("interior hole of 49 days"),
            "error must locate and quantify the hole; got: {err}"
        );
    }

    /// A corpus that starts after the window opens leaves the head of the window
    /// with no regime at all.
    #[test]
    fn coverage_rejects_a_late_start() {
        let start = 0i64;
        let end = 60 * MS_PER_DAY;
        let records = vec![
            (TimestampMs(30 * MS_PER_DAY), true),
            (TimestampMs(59 * MS_PER_DAY), true),
        ];
        let err = check_span_coverage(&records, start, end)
            .expect_err("a 30-day leading gap must be rejected");
        assert!(
            format!("{err}").contains("30 days after the window opens"),
            "error must quantify the leading gap; got: {err}"
        );
    }

    /// An empty series is the corpus-absent / all-warm-up case: it must be an
    /// ERROR, never a series handed to the arm (that is the 100%-cash stub).
    #[test]
    fn coverage_rejects_an_empty_series() {
        let err = check_span_coverage(&[], 0, 60 * MS_PER_DAY)
            .expect_err("an empty regime series must be rejected");
        assert!(
            format!("{err}").contains("EMPTY"),
            "error must say the series is empty; got: {err}"
        );
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
