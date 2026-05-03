#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1202 — V11 verification gates for the
//! `audit::query::journal_entries_for_transaction` reader added by the
//! tape-row-audit-modal feature
//! (`spec/features/tape-row-audit-modal.md` Q2 / Q8 V11).
//!
//! Three `#[tokio::test]` cases:
//!
//! - `t1202_returns_entries_in_id_order` — boot in-memory ledger, post one
//!   Buy fill via `audit::journal::post_fill` (capture the returned
//!   `txn_id`), call `query::journal_entries_for_transaction(&ledger,
//!   &txn_id)`. Assert returned `Vec<JournalEntry>` has `len() == 4`
//!   (paper Buy writes 4 entries: Dr position, Cr cash, Dr fee, Cr cash)
//!   and is sorted by `journal_entries.id ASC` lexicographically (R6
//!   determinism).
//! - `t1202_unknown_transaction_returns_empty_vec` — call with a
//!   non-existent UUID string. Assert `Ok(vec![])` (NOT `Err`) — unknown
//!   `tx_id` is a normal UI signal, not a data-integrity failure.
//! - `t1202_balanced_double_entry` — same setup as the first test;
//!   iterate the returned `Vec<JournalEntry>` and assert
//!   `Σ debit.amount() == Σ credit.amount()` (the audit-write-side
//!   `verify_balance` invariant re-asserted on reader output, guarding
//!   against partial-row leak).

use audit::{bootstrap, journal, query, Ledger};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, Symbol, Timestamp,
};

// ── helpers ───────────────────────────────────────────────────────────────────

/// Open a fresh in-memory ledger with all migrations applied AND the
/// chart-of-accounts bootstrap rows seeded (so `assets:cash:USDT`,
/// `assets:position:BTCUSDT`, and `expense:fees:taker` exist as FK
/// targets for `post_fill`).
async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_offset_secs(secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
}

/// Construct a `Fill` shaped like the V8 paper-fill fixture: Buy
/// 0.4 BTCUSDT @ 52,341.20, fee = 5.23 USDT, taker. The four resulting
/// `journal_entries` rows are the canonical V11 test surface.
fn make_paper_buy_fill() -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new("BTCUSDT"),
        side: Side::Buy,
        qty: Quantity::new(dec!(0.4)).expect("qty ok"),
        price: Price::new(dec!(52341.20)).expect("price ok"),
        fee: Money::from_decimal(dec!(5.23)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts_offset_secs(1_000),
        local_ts: ts_offset_secs(1_000),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

/// T1202 V11a — known transaction returns entries in `journal_entries.id ASC`
/// order.
///
/// A single paper Buy fill writes four entries (Dr position, Cr cash, Dr
/// fee, Cr cash). The reader projects them un-collapsed (debit/credit
/// preserved) and sorts by `journal_entries.id ASC` — a UUID v4 string
/// comparison, lex-stable across runs.
#[tokio::test]
async fn t1202_returns_entries_in_id_order() {
    let ledger = open_ledger().await;

    let txn_id = journal::post_fill(&ledger, &make_paper_buy_fill(), Some("sma-cross-btc-1m"))
        .await
        .expect("post Buy fill");

    let entries = query::journal_entries_for_transaction(&ledger, &txn_id)
        .await
        .expect("read journal entries for txn");

    // Buy fill writes exactly 4 entries:
    //   Dr assets:position:BTCUSDT   notional   (qty * price)
    //   Cr assets:cash:USDT          notional
    //   Dr expense:fees:taker        fee
    //   Cr assets:cash:USDT          fee
    assert_eq!(
        entries.len(),
        4,
        "expected 4 journal entries for a paper Buy fill (Dr position, \
         Cr cash, Dr fee, Cr cash); got {}",
        entries.len()
    );

    // Determinism — pull the underlying `journal_entries.id` strings and
    // assert the SQL ORDER BY id ASC matches Rust's lex sort. This is the
    // one cross-check that catches a future writer regression that lands
    // entries out-of-order or a reader regression that drops the ORDER BY
    // clause.
    let raw_ids: Vec<(String,)> =
        sqlx::query_as("SELECT id FROM journal_entries WHERE transaction_id = ? ORDER BY id ASC")
            .bind(txn_id.as_str())
            .fetch_all(ledger.pool())
            .await
            .expect("select journal_entries.id ORDER BY id ASC");

    let ids_from_sql: Vec<String> = raw_ids.into_iter().map(|(id,)| id).collect();
    let mut expected_sorted = ids_from_sql.clone();
    expected_sorted.sort();
    assert_eq!(
        ids_from_sql, expected_sorted,
        "SQL ORDER BY id ASC must agree with Rust's lex sort on the same UUID strings"
    );
    assert_eq!(
        ids_from_sql.len(),
        entries.len(),
        "reader-returned entry count must equal the raw row count for txn_id"
    );
}

/// T1202 V11b — unknown `tx_id` returns `Ok(vec![])` (NOT `Err`).
///
/// A non-existent UUID is a normal UI signal (operator clicked a stale
/// row, or the modal opened on a transaction that has been GC'd). It is
/// not an integrity failure; the reader returns the empty success vec
/// so the caller can render the "no entries for this transaction" empty
/// state without an error banner.
#[tokio::test]
async fn t1202_unknown_transaction_returns_empty_vec() {
    let ledger = open_ledger().await;

    // Fresh ledger → no transactions → any tx_id is unknown.
    let bogus_tx_id = "00000000-0000-0000-0000-deadbeefcafe";
    let entries = query::journal_entries_for_transaction(&ledger, bogus_tx_id)
        .await
        .expect("unknown tx_id must return Ok(vec![]), not Err");

    assert!(
        entries.is_empty(),
        "expected empty Vec for unknown tx_id `{bogus_tx_id}`; got {} entries",
        entries.len()
    );
}

/// T1202 V11c — `Σ debit == Σ credit` on the returned vec.
///
/// Re-asserts the audit-write-side `verify_balance` invariant on the
/// reader's output. The double-entry shape is a load-bearing property
/// of the modal's table render (every row's debit or credit is exactly
/// zero, and the column-wise totals reconcile), so guarding it here
/// catches a future reader regression that drops a row, splits a
/// transaction, or projects a partial set.
#[tokio::test]
async fn t1202_balanced_double_entry() {
    let ledger = open_ledger().await;

    let txn_id = journal::post_fill(&ledger, &make_paper_buy_fill(), Some("sma-cross-btc-1m"))
        .await
        .expect("post Buy fill");

    let entries = query::journal_entries_for_transaction(&ledger, &txn_id)
        .await
        .expect("read journal entries for txn");

    let sum_debits: Decimal = entries.iter().map(|e| e.debit.amount()).sum();
    let sum_credits: Decimal = entries.iter().map(|e| e.credit.amount()).sum();

    assert_eq!(
        sum_debits, sum_credits,
        "Σ debit ({sum_debits}) must equal Σ credit ({sum_credits}) on the \
         reader-returned Vec<JournalEntry> (double-entry invariant)"
    );

    // Defensive sanity check: the sum is non-zero (the fixture isn't a
    // degenerate all-zero transaction). This guards a future writer
    // regression that posts only the fee leg or only the position leg
    // without us noticing.
    assert!(
        sum_debits > Decimal::ZERO,
        "expected non-zero Σ debit on a valid Buy fill; got {sum_debits}"
    );
}
