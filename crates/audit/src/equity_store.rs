//! Durable live-equity-series store — trait, DTO, and impls.
//!
//! # Design (ADR-0052 / live-equity-history-durable)
//!
//! The [`LiveEquityStore`] trait is the external-I/O-behind-a-trait
//! boundary (R3 / A1).  The production impl wraps [`Arc<Ledger>`];
//! tests use [`FakeLiveEquityStore`] (in-memory `Vec`).
//!
//! * Money is [`Money<Usdt>`] backed by [`Decimal`] — never `f64`
//!   (ADR-0003 / project law).
//! * Timestamps are [`Timestamp`] — the RFC3339-micros format is
//!   enforced in the writer/reader; the DTO carries the strong type.
//! * The trait is [`async_trait`]-erased so both the production impl
//!   (the production impl is async over `SQLite`; the fake is in-memory, non-blocking) satisfy it.

use std::sync::Arc;

use async_trait::async_trait;
use trading_core::{LedgerError, Money, Timestamp, Usdt};

use crate::Ledger;

// ── DTO ───────────────────────────────────────────────────────────────────────

/// A single persisted per-bar equity snapshot (A3 / live-equity-history-durable).
///
/// All money fields are [`Money<Usdt>`] — [`Decimal`]-backed, never `f64`.
/// Both timestamps are strong [`Timestamp`] values; the SQL round-trip
/// uses RFC3339 6-digit-fractional format (ADR-0004).
///
/// Field semantics:
/// - `bar_ts` — the bar's close time (the chart x-axis coordinate).
///   Stored as `bar_ts` per the **two-timestamp contract** (approach A,
///   2026-06-11): the UI curve plots `bar_ts ?? as_of`; keying the
///   delivery guard on `bar_ts` was tried and reverted (`40f5de9`).
/// - `as_of` — wallclock publish time (`Timestamp::now()`). The UI
///   delivery guard keys on `as_of` monotonicity (A4).
/// - `mode` — `"paper"` or `"live"`. Only paper/live rows are ever
///   written (A2 gate); stored for forensics.
#[derive(Debug, Clone, PartialEq)]
pub struct EquitySnapshotRow {
    /// UUID v4 row identifier (matches every other audit table).
    pub id: String,
    /// Row mint wallclock (RFC3339-micros). Used for retention purge.
    pub ts: Timestamp,
    /// Bar close timestamp (chart x-axis). RFC3339-micros.
    pub bar_ts: Timestamp,
    /// Wallclock delivery timestamp (delivery guard). RFC3339-micros.
    pub as_of: Timestamp,
    /// Total portfolio equity (cash + unrealized P&L).
    pub total_equity: Money<Usdt>,
    /// Cash balance.
    pub cash: Money<Usdt>,
    /// Cumulative realized P&L.
    pub realized: Money<Usdt>,
    /// Current unrealized P&L.
    pub unrealized: Money<Usdt>,
    /// `"paper"` or `"live"`.
    pub mode: String,
}

// ── Trait ─────────────────────────────────────────────────────────────────────

/// Durable live-equity-series store (R3 / A1 / ADR-0052).
///
/// External-I/O-behind-a-trait boundary: the production impl wraps
/// [`Arc<Ledger>`]; tests use [`FakeLiveEquityStore`].
///
/// Money is [`Money<Usdt>`] (Decimal), never `f64`.
#[async_trait]
pub trait LiveEquityStore: Send + Sync {
    /// Persist one per-bar snapshot. Fire-and-forget at the call site:
    /// the agent logs + continues on `Err`, never blocks/panics the
    /// trading loop (A6 — the `bus = None` backtest tolerance).
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError`] on SQL failure. The caller MUST log + continue
    /// on error; it MUST NOT propagate to the trading loop.
    async fn append_equity_snapshot(&self, row: &EquitySnapshotRow) -> Result<(), LedgerError>;

    /// Read the tail for boot hydration — newest-bounded, `LIMIT ≤ 2880`,
    /// returned in **monotone `bar_ts` order** (A4 / R4).
    ///
    /// The hydrate path slices at most `limit` rows (caller passes 2880
    /// to match `LIVE_EQUITY_BUFFER_CAP`).  The oldest rows are dropped
    /// so the UI buffer starts at the correct historical point.
    ///
    /// # Errors
    ///
    /// Returns [`LedgerError`] on SQL or parse failure. The caller
    /// SHOULD treat `Err` as an empty tail (fail-soft hydrate).
    async fn equity_snapshot_tail(
        &self,
        limit: usize,
    ) -> Result<Vec<EquitySnapshotRow>, LedgerError>;
}

// ── Production impl ───────────────────────────────────────────────────────────

/// Production [`LiveEquityStore`] impl wrapping an [`Arc<Ledger>`].
///
/// Delegates `append_equity_snapshot` to
/// [`crate::journal::post_equity_snapshot`] and
/// `equity_snapshot_tail` to
/// [`crate::query::equity_snapshot_tail`].
#[derive(Clone)]
pub struct LedgerEquityStore {
    ledger: Arc<Ledger>,
}

impl LedgerEquityStore {
    /// Construct from an existing [`Arc<Ledger>`].
    #[must_use]
    pub fn new(ledger: Arc<Ledger>) -> Self {
        Self { ledger }
    }
}

#[async_trait]
impl LiveEquityStore for LedgerEquityStore {
    async fn append_equity_snapshot(&self, row: &EquitySnapshotRow) -> Result<(), LedgerError> {
        crate::journal::post_equity_snapshot(&self.ledger, row).await
    }

    async fn equity_snapshot_tail(
        &self,
        limit: usize,
    ) -> Result<Vec<EquitySnapshotRow>, LedgerError> {
        crate::query::equity_snapshot_tail(&self.ledger, limit).await
    }
}

// ── Fake impl (tests) ─────────────────────────────────────────────────────────

/// In-memory fake [`LiveEquityStore`] for tests.
///
/// Rows are stored in insertion order; [`equity_snapshot_tail`] sorts
/// by `bar_ts` ascending and returns the last `limit` rows — mirroring
/// the production `ORDER BY bar_ts ASC … LIMIT` query.
///
/// Cheap to clone — the inner `Vec` is behind an `Arc<Mutex<…>>` so
/// multiple test tasks can share one fake instance.
#[derive(Default, Clone)]
pub struct FakeLiveEquityStore {
    rows: Arc<std::sync::Mutex<Vec<EquitySnapshotRow>>>,
}

impl FakeLiveEquityStore {
    /// Construct an empty fake store.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Snapshot of all stored rows (in insertion order).
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned — only possible in test teardowns.
    #[must_use]
    #[allow(clippy::unwrap_used)] // mutex poison = test teardown; safe for test helper
    pub fn rows(&self) -> Vec<EquitySnapshotRow> {
        self.rows.lock().unwrap().clone()
    }

    /// Number of stored rows.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    #[must_use]
    #[allow(clippy::unwrap_used)] // mutex poison = test teardown; safe for test helper
    pub fn len(&self) -> usize {
        self.rows.lock().unwrap().len()
    }

    /// Returns `true` when the store is empty.
    ///
    /// # Panics
    ///
    /// Panics if the mutex is poisoned.
    #[must_use]
    #[allow(clippy::unwrap_used)] // mutex poison = test teardown; safe for test helper
    pub fn is_empty(&self) -> bool {
        self.rows.lock().unwrap().is_empty()
    }
}

#[async_trait]
impl LiveEquityStore for FakeLiveEquityStore {
    #[allow(clippy::unwrap_used)] // mutex poison = test teardown; safe for test fake
    async fn append_equity_snapshot(&self, row: &EquitySnapshotRow) -> Result<(), LedgerError> {
        self.rows.lock().unwrap().push(row.clone());
        Ok(())
    }

    #[allow(clippy::unwrap_used)] // mutex poison = test teardown; safe for test fake
    async fn equity_snapshot_tail(
        &self,
        limit: usize,
    ) -> Result<Vec<EquitySnapshotRow>, LedgerError> {
        let guard = self.rows.lock().unwrap();
        // Sort by bar_ts ascending (mirroring the SQL ORDER BY bar_ts ASC).
        let mut sorted: Vec<EquitySnapshotRow> = guard.clone();
        sorted.sort_by_key(|r| r.bar_ts);

        // Return the last `limit` rows (the most-recent tail, in ascending order).
        let start = sorted.len().saturating_sub(limit);
        Ok(sorted[start..].to_vec())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use rust_decimal_macros::dec;
    use trading_core::{Money, Timestamp};

    use super::*;

    fn make_row(bar_offset_min: i64) -> EquitySnapshotRow {
        let base = time::OffsetDateTime::UNIX_EPOCH;
        let bar_ts = Timestamp::new(base + time::Duration::minutes(bar_offset_min));
        let as_of = Timestamp::new(base + time::Duration::minutes(bar_offset_min));
        let ts = Timestamp::now();
        EquitySnapshotRow {
            id: format!("id-{bar_offset_min}"),
            ts,
            bar_ts,
            as_of,
            total_equity: Money::from_decimal(
                dec!(100_000) + dec!(10) * rust_decimal::Decimal::from(bar_offset_min),
            ),
            cash: Money::from_decimal(dec!(50_000)),
            realized: Money::from_decimal(dec!(0)),
            unrealized: Money::from_decimal(dec!(50_000)),
            mode: "paper".to_string(),
        }
    }

    #[tokio::test]
    async fn fake_store_append_and_tail_monotone_order() {
        let store = FakeLiveEquityStore::new();

        // Insert in non-monotone order.
        store.append_equity_snapshot(&make_row(3)).await.unwrap();
        store.append_equity_snapshot(&make_row(1)).await.unwrap();
        store.append_equity_snapshot(&make_row(2)).await.unwrap();

        let tail = store.equity_snapshot_tail(10).await.unwrap();
        assert_eq!(tail.len(), 3);

        // tail must be bar_ts-ascending.
        let bar_ts_seq: Vec<_> = tail.iter().map(|r| r.bar_ts).collect();
        let mut sorted = bar_ts_seq.clone();
        sorted.sort();
        assert_eq!(bar_ts_seq, sorted, "tail must be monotone ascending bar_ts");
    }

    #[tokio::test]
    async fn fake_store_tail_respects_limit() {
        let store = FakeLiveEquityStore::new();
        for i in 0..10 {
            store.append_equity_snapshot(&make_row(i)).await.unwrap();
        }

        let tail = store.equity_snapshot_tail(5).await.unwrap();
        assert_eq!(tail.len(), 5, "tail should be limited to 5");

        // The tail should be the LAST 5 (by bar_ts).
        let last_bar_ts = tail.last().unwrap().bar_ts;
        let expected_last = time::OffsetDateTime::UNIX_EPOCH + time::Duration::minutes(9);
        assert_eq!(last_bar_ts.inner(), expected_last);
    }

    #[tokio::test]
    async fn fake_store_is_empty_and_len() {
        let store = FakeLiveEquityStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);

        store.append_equity_snapshot(&make_row(0)).await.unwrap();
        assert!(!store.is_empty());
        assert_eq!(store.len(), 1);
    }
}
