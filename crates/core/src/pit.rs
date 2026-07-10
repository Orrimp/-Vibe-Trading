//! Point-in-time (PIT) as-of join primitive.
//!
//! A [`PitSeries<T>`] is a sorted, timestamped series whose ONLY query method
//! is [`PitSeries::as_of`]`(query) -> Option<AsOf<T>>`, returning the
//! most-recent record at-or-before `query` (`None` during warm-up). There is
//! no public method that returns a record with `ts > query`, so joining future
//! data onto a bar is **UNREPRESENTABLE** — look-ahead is a compile error, not
//! a runtime bug.
//!
//! # Usage
//!
//! This is the single guarded as-of join every sidecar feature (funding,
//! basis, on-chain) routes through. A hand-rolled `partition_point` is the
//! anti-pattern this replaces. See
//! `spec/v1/point-in-time-data-discipline/feature.md` § Design and ADR-0058.
//!
//! ```rust
//! use trading_core::pit::{PitSeries, TimestampMs};
//! use rust_decimal_macros::dec;
//!
//! let records = vec![
//!     (TimestampMs(1_000), dec!(0.001)),
//!     (TimestampMs(2_000), dec!(0.002)),
//! ];
//! let series = PitSeries::from_sorted(records).unwrap();
//!
//! // Returns the most-recent record at-or-before query ts.
//! assert_eq!(series.as_of_value(TimestampMs(1_500)), Some(dec!(0.001)));
//! // Warm-up: no record precedes the query.
//! assert_eq!(series.as_of_value(TimestampMs(500)), None);
//! ```

use serde::{Deserialize, Serialize};

/// Milliseconds since the Unix epoch — the as-of join key.
///
/// Transparent `i64` newtype. We key on raw ms (NOT `Timestamp`) because the
/// production loaders join on `i64` ms and a `Timestamp` round-trip would
/// truncate (`i128 → i64`) and risk an anchor delta. `Ord` is the plain `i64`
/// ordering, so the `partition_point(|r| r.ts <= q)` predicate is preserved
/// byte-for-byte against the legacy `|&(t, _)| t <= bar_ts`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TimestampMs(pub i64);

/// The result of an as-of query: a value whose timestamp is PROVEN `<=` the
/// query timestamp. Constructed ONLY by [`PitSeries::as_of`]; there is no
/// public constructor that lets a caller fabricate an `AsOf` whose `ts >
/// query`.
///
/// `as_of_ts` is the timestamp of the record that was in force at the query
/// (the proof-carrying field); `value` is its payload. Callers that only need
/// the payload use [`.into_value()`](AsOf::into_value) /
/// [`.value()`](AsOf::value).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AsOf<T> {
    as_of_ts: TimestampMs,
    value: T,
}

impl<T> AsOf<T> {
    /// The timestamp of the in-force record.
    ///
    /// Invariant: `as_of_ts <= query` for the `query` that produced this
    /// `AsOf`.
    #[must_use]
    pub fn as_of_ts(&self) -> TimestampMs {
        self.as_of_ts
    }

    /// Borrow the payload.
    #[must_use]
    pub fn value(&self) -> &T {
        &self.value
    }

    /// Consume into the payload (the hot path for `build_*_at_return`).
    #[must_use]
    pub fn into_value(self) -> T {
        self.value
    }
}

/// Error from the checked constructor.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum PitError {
    /// The series was not sorted ascending by timestamp (ties allowed).
    #[error("PitSeries records not sorted ascending by ts (violation at index {0})")]
    NotSorted(usize),
}

/// A sorted, timestamped series supporting causal (no-look-ahead) as-of
/// queries.
///
/// `T` is the payload (`Decimal` in production; generic so research/tests can
/// use any `Clone` type). Stores `Vec<(TimestampMs, T)>` sorted ascending,
/// plus a `publication_lag_ms` (ADR-0086 D2 / P3 M-DEV-4) declaring how long
/// after a record's own `ts` it becomes queryable.
///
/// # PIT discipline
///
/// Reach for this type — never hand-roll `partition_point`. The only way to
/// get a `T` out of a `PitSeries<T>` keyed to a query is
/// [`as_of`](Self::as_of) / [`as_of_value`](Self::as_of_value), and both
/// return only the record at `idx-1` where `idx =
/// partition_point(t <= query)` — a record with `ts <= query`. There is no
/// `get(i)`, no `Index`, no `iter()` returning future records, no
/// `records()` accessor.
///
/// # Publication lag (ADR-0086)
///
/// `publication_lag_ms` is a **declared, per-series** interval: a record at
/// `ts` is not visible until `query >= ts + publication_lag_ms`. It defaults
/// to `0` via [`from_sorted`](Self::from_sorted) / [`from_unsorted`](Self::from_unsorted)
/// / [`from_sorted_slice`](Self::from_sorted_slice), which are all defined
/// as their `*_with_lag(records, 0)` sibling — **byte-identical to the
/// pre-P3 primitive**. Use [`from_sorted_with_lag`](Self::from_sorted_with_lag)
/// / [`from_unsorted_with_lag`](Self::from_unsorted_with_lag) to declare a
/// non-zero lag for a channel with a genuine release delay (none exist in
/// production today — every current series' join key already encodes its
/// availability instant; see `spec/v3/advisor-pit-discipline/feature.md` §
/// D2 lag table).
#[derive(Debug, Clone)]
pub struct PitSeries<T> {
    records: Vec<(TimestampMs, T)>,
    publication_lag_ms: i64,
}

impl<T: Clone> PitSeries<T> {
    /// Build from an already-sorted owned vec, CHECKING the sort invariant,
    /// with `publication_lag_ms = 0` (byte-identical to the pre-P3 primitive).
    ///
    /// Returns [`PitError::NotSorted`] at the first index where a record's
    /// timestamp is strictly less than the previous record's timestamp (ties —
    /// equal adjacent timestamps — are allowed and preserved).
    ///
    /// # Errors
    ///
    /// Returns `PitError::NotSorted(i)` where `i` is the first out-of-order
    /// index.
    pub fn from_sorted(records: Vec<(TimestampMs, T)>) -> Result<Self, PitError> {
        Self::from_sorted_with_lag(records, 0)
    }

    /// Build from an already-sorted owned vec, CHECKING the sort invariant,
    /// with an explicit, declared `publication_lag_ms` (ADR-0086 D2).
    ///
    /// A record at `ts` becomes queryable only once
    /// `query >= ts + publication_lag_ms`. With `publication_lag_ms == 0`
    /// this is character-for-character [`from_sorted`](Self::from_sorted).
    ///
    /// # Errors
    ///
    /// Returns `PitError::NotSorted(i)` where `i` is the first out-of-order
    /// index. Sort-order is checked on the RAW record `ts`, independent of
    /// `publication_lag_ms` (the lag shifts availability, not the series'
    /// own ordering).
    pub fn from_sorted_with_lag(
        records: Vec<(TimestampMs, T)>,
        publication_lag_ms: i64,
    ) -> Result<Self, PitError> {
        for i in 1..records.len() {
            if records[i].0 < records[i - 1].0 {
                return Err(PitError::NotSorted(i));
            }
        }
        Ok(Self {
            records,
            publication_lag_ms,
        })
    }

    /// Build from an unsorted owned vec, sorting by `ts` with a STABLE sort
    /// (`sort_by_key`) so equal-timestamp records keep input order, with
    /// `publication_lag_ms = 0` (byte-identical to the pre-P3 primitive).
    ///
    /// Matching the loaders' `sort_unstable_by_key`-then-dedup discipline is
    /// the caller's job; this primitive preserves whatever order it is given
    /// for ties.
    #[must_use]
    pub fn from_unsorted(records: Vec<(TimestampMs, T)>) -> Self {
        Self::from_unsorted_with_lag(records, 0)
    }

    /// Build from an unsorted owned vec, sorting by `ts` with a STABLE sort
    /// (`sort_by_key`), with an explicit, declared `publication_lag_ms`
    /// (ADR-0086 D2). See [`from_sorted_with_lag`](Self::from_sorted_with_lag)
    /// for the lag semantics.
    #[must_use]
    pub fn from_unsorted_with_lag(
        mut records: Vec<(TimestampMs, T)>,
        publication_lag_ms: i64,
    ) -> Self {
        records.sort_by_key(|&(t, _)| t);
        Self {
            records,
            publication_lag_ms,
        }
    }

    /// Borrowing constructor over a sorted slice (zero-copy view + clone);
    /// CHECKED. `publication_lag_ms = 0` (byte-identical to the pre-P3
    /// primitive).
    ///
    /// For callers that hold a `&[(TimestampMs, T)]` and want to avoid
    /// constructing an owned `Vec` themselves. The production loaders use the
    /// owned constructors above.
    ///
    /// # Errors
    ///
    /// Returns `PitError::NotSorted(i)` if the slice is not sorted ascending.
    pub fn from_sorted_slice(records: &[(TimestampMs, T)]) -> Result<Self, PitError> {
        for i in 1..records.len() {
            if records[i].0 < records[i - 1].0 {
                return Err(PitError::NotSorted(i));
            }
        }
        Ok(Self {
            records: records.to_vec(),
            publication_lag_ms: 0,
        })
    }

    /// THE query. Returns the most-recent record at-or-before the record's
    /// EFFECTIVE availability instant `ts + publication_lag_ms <= query`
    /// (equivalently: the most-recent record with
    /// `ts <= query - publication_lag_ms`), or `None` if no record's
    /// availability instant precedes `query` (warm-up).
    ///
    /// Implemented by querying the RAW record timestamps against
    /// `query.saturating_sub(publication_lag_ms)`:
    /// `self.records.partition_point(|&(t, _)| t <= adjusted_query)` — the
    /// EXACT legacy predicate at `publication_lag_ms == 0` (`adjusted_query
    /// == query` character-for-character) — taking `idx-1` (or `None` when
    /// `idx == 0`). This is the single line that guarantees byte-identical
    /// migration (R3 / ADR-0058 D4, preserved verbatim by ADR-0086 D2 at
    /// lag=0).
    #[must_use]
    pub fn as_of(&self, query: TimestampMs) -> Option<AsOf<T>> {
        let adjusted_query = TimestampMs(query.0.saturating_sub(self.publication_lag_ms));
        let idx = self.records.partition_point(|&(t, _)| t <= adjusted_query); // PIT-OK: the sanctioned core::pit implementation itself.
        if idx == 0 {
            None
        } else {
            let (ts, ref val) = self.records[idx - 1];
            Some(AsOf {
                as_of_ts: ts,
                value: val.clone(),
            })
        }
    }

    /// Convenience: as-of, projecting straight to the owned payload.
    ///
    /// This is the EXACT shape `funding_as_of` / `basis_as_of` need —
    /// `Option<T>` per query — so the migrated wrappers are
    /// `series.as_of_value(q)` and nothing else.
    #[must_use]
    pub fn as_of_value(&self, query: TimestampMs) -> Option<T> {
        self.as_of(query).map(AsOf::into_value)
    }

    /// Number of records (for warm-up/diagnostic assertions in tests).
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns `true` if the series contains no records.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::float_arithmetic,
    clippy::pedantic
)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn ms(t: i64) -> TimestampMs {
        TimestampMs(t)
    }

    // ── (a) Look-ahead falsifier — M-TEST-1 ──────────────────────────────────

    /// Self-proving look-ahead falsifier (AC2 — the feature's day-1 gate).
    ///
    /// Mirrors `funding_data.rs::no_look_ahead_falsifier` exactly. Build a
    /// series, query causally, forward-shift EVERY record's `ts` by `+Δ`,
    /// query the same `q` again, and assert the two results DIFFER.
    ///
    /// Wired so that BREAKING the guard (exposing future records) makes this
    /// fail: if `as_of` ever returned a record with `ts > query`, the
    /// shifted result would equal the causal result and `assert_ne!` would
    /// panic.
    #[test]
    fn as_of_no_look_ahead_falsifier() {
        let delta: i64 = 1_000; // shift all records forward by 1 000 ms

        let records = vec![
            (ms(1_000), dec!(0.001)),
            (ms(2_000), dec!(0.002)),
            (ms(3_000), dec!(0.003)),
        ];
        let series = PitSeries::from_sorted(records.clone()).unwrap();

        // Shift every record's ts by +delta.
        let shifted: Vec<(TimestampMs, _)> = records
            .into_iter()
            .map(|(t, v)| (TimestampMs(t.0 + delta), v))
            .collect();
        let shifted_series = PitSeries::from_sorted(shifted).unwrap();

        // Query at ts=2000: causal series has a record at 2000 → Some(0.002).
        let query = ms(2_000);
        let causal = series.as_of_value(query);
        // Shifted series: record formerly at 2000 is now at 3000 > query; so
        // at query=2000 only the record at 2000 (was 1000) is visible → Some(0.001).
        let shifted_result = shifted_series.as_of_value(query);

        assert_eq!(causal, Some(dec!(0.002)));
        assert_eq!(shifted_result, Some(dec!(0.001)));
        // They MUST differ — proves no look-ahead (the guard is real).
        assert_ne!(
            causal, shifted_result,
            "look-ahead falsifier: causal ≠ future-shifted"
        );
    }

    // ── (b) Warm-up ──────────────────────────────────────────────────────────

    #[test]
    fn warm_up_before_first_record_is_none() {
        let series =
            PitSeries::from_sorted(vec![(ms(1_000), dec!(0.1)), (ms(2_000), dec!(0.2))]).unwrap();
        assert_eq!(series.as_of_value(ms(500)), None, "warm-up must be None");
        assert_eq!(series.as_of(ms(500)), None);
    }

    // ── (c) At-boundary: ts == query is INCLUDED (the ≤ convention) ──────────

    #[test]
    fn at_boundary_ts_eq_query_is_included() {
        let series = PitSeries::from_sorted(vec![
            (ms(1_000), dec!(0.1)),
            (ms(2_000), dec!(0.2)),
            (ms(3_000), dec!(0.3)),
        ])
        .unwrap();
        // Exact match: ts == query.
        let got = series.as_of(ms(2_000)).unwrap();
        assert_eq!(*got.value(), dec!(0.2));
        assert_eq!(got.as_of_ts(), ms(2_000));
    }

    // ── (d) Between records: forward-fill picks the earlier ──────────────────

    #[test]
    fn between_records_picks_earlier() {
        let series =
            PitSeries::from_sorted(vec![(ms(1_000), dec!(0.1)), (ms(3_000), dec!(0.3))]).unwrap();
        // query=2000 is between 1000 and 3000 → picks the 1000 record.
        let got = series.as_of_value(ms(2_000));
        assert_eq!(got, Some(dec!(0.1)));
    }

    // ── (e) Empty series → None ───────────────────────────────────────────────

    #[test]
    fn empty_series_is_none() {
        let series: PitSeries<rust_decimal::Decimal> = PitSeries::from_sorted(vec![]).unwrap();
        assert_eq!(series.as_of_value(ms(0)), None);
        assert!(series.is_empty());
        assert_eq!(series.len(), 0);
    }

    // ── (f) from_sorted rejects a descending pair ─────────────────────────────

    #[test]
    fn from_sorted_rejects_descending_pair() {
        let bad = vec![(ms(2_000), dec!(0.2)), (ms(1_000), dec!(0.1))];
        let err = PitSeries::from_sorted(bad).unwrap_err();
        assert_eq!(err, PitError::NotSorted(1));
    }

    #[test]
    fn from_sorted_rejects_violation_later() {
        let bad = vec![
            (ms(1_000), dec!(0.1)),
            (ms(2_000), dec!(0.2)),
            (ms(1_500), dec!(0.15)), // violation at index 2
        ];
        let err = PitSeries::from_sorted(bad).unwrap_err();
        assert_eq!(err, PitError::NotSorted(2));
    }

    // ── (g) Ties (equal adjacent ts) are preserved ───────────────────────────

    #[test]
    fn equal_adjacent_timestamps_are_allowed() {
        // Ties are allowed; from_sorted should NOT reject them.
        let records = vec![
            (ms(1_000), dec!(0.1)),
            (ms(1_000), dec!(0.2)), // same ts, different value — second wins
            (ms(2_000), dec!(0.3)),
        ];
        let series = PitSeries::from_sorted(records).unwrap();
        assert_eq!(series.len(), 3);
        // At query=1000 the rightmost record at ts<=1000 is index 1 (dec!(0.2)).
        let got = series.as_of_value(ms(1_000));
        assert_eq!(got, Some(dec!(0.2)));
    }

    // ── from_unsorted sorts correctly ─────────────────────────────────────────

    #[test]
    fn from_unsorted_sorts_and_queries() {
        let records = vec![
            (ms(3_000), dec!(0.3)),
            (ms(1_000), dec!(0.1)),
            (ms(2_000), dec!(0.2)),
        ];
        let series = PitSeries::from_unsorted(records);
        assert_eq!(series.as_of_value(ms(2_500)), Some(dec!(0.2)));
    }

    // ── from_sorted_slice clones correctly ────────────────────────────────────

    #[test]
    fn from_sorted_slice_works() {
        let records = vec![(ms(1_000), dec!(0.1)), (ms(2_000), dec!(0.2))];
        let series = PitSeries::from_sorted_slice(&records).unwrap();
        assert_eq!(series.as_of_value(ms(1_000)), Some(dec!(0.1)));
    }

    #[test]
    fn from_sorted_slice_rejects_unsorted() {
        let records = vec![(ms(2_000), dec!(0.2)), (ms(1_000), dec!(0.1))];
        assert!(PitSeries::from_sorted_slice(&records).is_err());
    }

    // ── AsOf accessors ────────────────────────────────────────────────────────

    #[test]
    fn as_of_accessors_are_consistent() {
        let series = PitSeries::from_sorted(vec![(ms(1_000), dec!(0.42))]).unwrap();
        let result = series.as_of(ms(5_000)).unwrap();
        assert_eq!(result.as_of_ts(), ms(1_000));
        assert_eq!(*result.value(), dec!(0.42));
        assert_eq!(result.into_value(), dec!(0.42));
    }

    // ── Publication lag (ADR-0086 D2 / P3 M-TEST-2) ──────────────────────────

    /// (a) Lag-0 reduction: `from_sorted_with_lag(r, 0)` is byte-identical to
    /// `from_sorted(r)` for a sweep of queries, including warm-up, exact
    /// boundary, and between-record cases. This is the AC2 / feature.md §
    /// D4.3 "byte-identical default" proof.
    #[test]
    fn lag_zero_reduction_matches_legacy_from_sorted() {
        let records = vec![
            (ms(1_000), dec!(0.1)),
            (ms(2_000), dec!(0.2)),
            (ms(3_000), dec!(0.3)),
        ];
        let legacy = PitSeries::from_sorted(records.clone()).unwrap();
        let with_lag_zero = PitSeries::from_sorted_with_lag(records, 0).unwrap();

        // Sweep across warm-up, exact-boundary, between-record, and
        // past-last-record queries.
        for q in [0, 500, 1_000, 1_500, 2_000, 2_500, 3_000, 3_500, 10_000] {
            let legacy_result = legacy.as_of(ms(q));
            let lagged_result = with_lag_zero.as_of(ms(q));
            assert_eq!(
                legacy_result, lagged_result,
                "lag=0 must equal legacy from_sorted at query={q}"
            );
            // Also confirm the value-projection wrapper agrees.
            assert_eq!(
                legacy.as_of_value(ms(q)),
                with_lag_zero.as_of_value(ms(q)),
                "as_of_value must also agree at query={q}"
            );
        }
    }

    /// (a-2) `from_unsorted_with_lag(r, 0)` is byte-identical to
    /// `from_unsorted(r)`.
    #[test]
    fn lag_zero_reduction_matches_legacy_from_unsorted() {
        let records = vec![
            (ms(3_000), dec!(0.3)),
            (ms(1_000), dec!(0.1)),
            (ms(2_000), dec!(0.2)),
        ];
        let legacy = PitSeries::from_unsorted(records.clone());
        let with_lag_zero = PitSeries::from_unsorted_with_lag(records, 0);

        for q in [0, 500, 1_000, 1_500, 2_000, 2_500, 3_000, 3_500] {
            assert_eq!(
                legacy.as_of_value(ms(q)),
                with_lag_zero.as_of_value(ms(q)),
                "lag=0 (from_unsorted_with_lag) must equal legacy from_unsorted at query={q}"
            );
        }
    }

    /// (b) Positive lag delays availability — the explicit-lag analogue of
    /// `as_of_no_look_ahead_falsifier`. A record at `ts=1000` with
    /// `lag=500` is invisible at `query=1200` (< 1500) and visible at
    /// `query=1500` (== 1000+500), with `as_of_ts()` still returning the
    /// RECORD's own ts (1000), not the query or the availability instant.
    #[test]
    fn positive_lag_delays_availability() {
        let series = PitSeries::from_sorted_with_lag(vec![(ms(1_000), dec!(0.42))], 500).unwrap();

        // Before ts+lag (1000+500=1500): not yet available.
        assert_eq!(
            series.as_of(ms(1_200)),
            None,
            "record must be invisible before ts+lag"
        );
        assert_eq!(series.as_of_value(ms(1_499)), None);

        // At exactly ts+lag: available (the lag's <= boundary).
        let at_boundary = series.as_of(ms(1_500)).unwrap();
        assert_eq!(
            at_boundary.as_of_ts(),
            ms(1_000),
            "as_of_ts must be the RECORD's own ts, not the query or availability instant"
        );
        assert_eq!(*at_boundary.value(), dec!(0.42));

        // After ts+lag: still available (forward-filled).
        let after = series.as_of(ms(2_000)).unwrap();
        assert_eq!(after.as_of_ts(), ms(1_000));
        assert_eq!(*after.value(), dec!(0.42));
    }

    /// (b-2) Positive lag across multiple records: forward-fill still
    /// respects each record's OWN effective availability instant
    /// independently.
    #[test]
    fn positive_lag_multi_record_forward_fill() {
        // Two records, lag=1000: record@2000 available from query=3000;
        // record@5000 available from query=6000.
        let series = PitSeries::from_sorted_with_lag(
            vec![(ms(2_000), dec!(0.1)), (ms(5_000), dec!(0.2))],
            1_000,
        )
        .unwrap();

        assert_eq!(
            series.as_of_value(ms(2_500)),
            None,
            "before first availability"
        );
        assert_eq!(
            series.as_of_value(ms(3_000)),
            Some(dec!(0.1)),
            "first record available at ts+lag"
        );
        assert_eq!(
            series.as_of_value(ms(5_999)),
            Some(dec!(0.1)),
            "second record not yet available — forward-fill on first"
        );
        assert_eq!(
            series.as_of_value(ms(6_000)),
            Some(dec!(0.2)),
            "second record available at its own ts+lag"
        );
    }

    /// (b-3) Negative-lag guard: `saturating_sub` must not panic/overflow
    /// when `publication_lag_ms` exceeds `query` in magnitude (an
    /// pathological but representable config) — the query clamps to
    /// `TimestampMs(0)` rather than wrapping.
    #[test]
    fn lag_saturating_sub_does_not_underflow() {
        let series = PitSeries::from_sorted_with_lag(vec![(ms(0), dec!(0.1))], i64::MAX).unwrap();
        // query.saturating_sub(i64::MAX) saturates to a very negative number
        // (i64::MIN-ish), well below the record's ts=0, so warm-up (None).
        assert_eq!(series.as_of(ms(100)), None);
    }
}
