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
//! `spec/point-in-time-data-discipline/feature.md` § Design and ADR-0058.
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
/// use any `Clone` type). Stores `Vec<(TimestampMs, T)>` sorted ascending.
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
#[derive(Debug, Clone)]
pub struct PitSeries<T> {
    records: Vec<(TimestampMs, T)>,
}

impl<T: Clone> PitSeries<T> {
    /// Build from an already-sorted owned vec, CHECKING the sort invariant.
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
        for i in 1..records.len() {
            if records[i].0 < records[i - 1].0 {
                return Err(PitError::NotSorted(i));
            }
        }
        Ok(Self { records })
    }

    /// Build from an unsorted owned vec, sorting by `ts` with a STABLE sort
    /// (`sort_by_key`) so equal-timestamp records keep input order.
    ///
    /// Matching the loaders' `sort_unstable_by_key`-then-dedup discipline is
    /// the caller's job; this primitive preserves whatever order it is given
    /// for ties.
    #[must_use]
    pub fn from_unsorted(mut records: Vec<(TimestampMs, T)>) -> Self {
        records.sort_by_key(|&(t, _)| t);
        Self { records }
    }

    /// Borrowing constructor over a sorted slice (zero-copy view + clone);
    /// CHECKED.
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
        })
    }

    /// THE query. Returns the most-recent record at-or-before `query`
    /// (`ts <= query`), or `None` if no record precedes `query` (warm-up).
    ///
    /// Implemented as `self.records.partition_point(|&(t, _)| t <= query)` —
    /// the EXACT legacy predicate — taking `idx-1` (or `None` when `idx ==
    /// 0`). This is the single line that guarantees byte-identical migration
    /// (R3 / ADR-0058 D4).
    #[must_use]
    pub fn as_of(&self, query: TimestampMs) -> Option<AsOf<T>> {
        let idx = self.records.partition_point(|&(t, _)| t <= query);
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
}
