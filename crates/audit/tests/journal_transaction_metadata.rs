#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1302 — V1 / V2 verification gates for the
//! `audit::query::journal_transaction_metadata` reader added by the
//! journal-transactions-metadata feature
//! (`spec/features/journal-transactions-metadata.md` Design § Q2 / SQL shape).
//!
//! Three `#[tokio::test]` cases:
//!
//! - `t1302_v1_returns_metadata_for_existing_transaction` — boot in-memory
//!   ledger, post one paper Buy fill via `audit::journal::post_fill` (capture
//!   the returned `txn_id`), call
//!   `query::journal_transaction_metadata(&ledger, &txn_id)`. Assert the
//!   four-field header round-trips faithfully (`transaction_id`, `ts`,
//!   `description`, `strategy_id`).
//! - `t1302_v2_returns_none_for_unknown_tx_id` — call with a non-existent
//!   UUID string. Assert `Ok(None)` (NOT `Err`, NOT `Ok(Some(default))`).
//!   Mirrors T1202's `Ok(vec![])` contract.
//! - `t1302_strategy_id_optional` — post a fill with `strategy_id: None`
//!   (legacy / pre-T802 row shape). Assert the reader returns
//!   `Ok(Some(meta))` with `strategy_id: None`.

use audit::{Ledger, bootstrap, journal, query};
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price, Quantity, Side, StrategyId, Symbol,
    Timestamp, Venue,
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

/// Construct a `Fill` shaped like the T1202 paper-fill fixture: Buy
/// 0.4 BTCUSDT @ 52,341.20, fee = 5.23 USDT, taker. The `journal::post_fill`
/// description site renders this as `"buy 0.4 BTCUSDT @ 52341.20"`
/// (lowercase `Side`, no thousands separator on `Price`).
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

/// T1302 V1 — known transaction returns the four-field header.
///
/// The reader projects `journal_transactions(id, ts, description,
/// strategy_id)` exactly. Assert each field round-trips with no loss:
/// `transaction_id` matches the `post_fill` return value, `description`
/// matches the `format!("{} {} {} @ {}", side, qty, sym, px)` shape at
/// `crates/audit/src/journal.rs:58`, `strategy_id` is `Some(...)` when one
/// was supplied, and `ts` parses back to the fill's `venue_ts`.
#[tokio::test]
async fn t1302_v1_returns_metadata_for_existing_transaction() {
    let ledger = open_ledger().await;

    let fill = make_paper_buy_fill();
    let expected_ts = fill.venue_ts;
    let txn_id = journal::post_fill(&ledger, &fill, Venue::Binance, Some("sma-cross-btc-1m"))
        .await
        .expect("post Buy fill");

    let meta = query::journal_transaction_metadata(&ledger, &txn_id)
        .await
        .expect("read journal transaction metadata")
        .expect("expected Some(metadata) for a freshly-posted fill");

    assert_eq!(
        meta.transaction_id.as_str(),
        txn_id.as_str(),
        "metadata.transaction_id must match the post_fill return value"
    );
    assert_eq!(
        meta.description.as_str(),
        "buy 0.4 BTCUSDT @ 52341.20",
        "metadata.description must match the post_fill format!() shape \
         (\"{{side}} {{qty}} {{symbol}} @ {{price}}\", lowercase Side)"
    );
    assert_eq!(
        meta.strategy_id,
        Some(StrategyId::new("sma-cross-btc-1m")),
        "metadata.strategy_id must round-trip the post_fill argument"
    );
    assert_eq!(
        meta.ts, expected_ts,
        "metadata.ts must round-trip the fill's venue_ts via Rfc3339"
    );
}

/// T1302 V2 — unknown `tx_id` returns `Ok(None)` (NOT `Err`, NOT
/// `Ok(Some(default))`).
///
/// A non-existent UUID is a normal UI signal (operator clicked a stale
/// row, or the modal opened on a transaction that has been GC'd). It is
/// not an integrity failure; the reader returns `None` so the caller
/// can render an "unknown transaction" error state without an exception
/// path. Mirrors T1202's `Ok(vec![])` empty-result contract.
#[tokio::test]
async fn t1302_v2_returns_none_for_unknown_tx_id() {
    let ledger = open_ledger().await;

    // Fresh ledger → no transactions → any tx_id is unknown.
    let bogus_tx_id = "00000000-0000-0000-0000-deadbeefcafe";
    let result = query::journal_transaction_metadata(&ledger, bogus_tx_id)
        .await
        .expect("unknown tx_id must return Ok(None), not Err");

    assert!(
        result.is_none(),
        "expected None for unknown tx_id `{bogus_tx_id}`; got {result:?}"
    );
}

/// T1302 — `strategy_id` column is nullable; legacy / pre-T802 rows surface
/// as `strategy_id: None` (mirrors the `(unattributed)` bucket convention
/// in `pnl_by_strategy`). Verify the reader does not collapse `NULL` into
/// an `Err` or a fabricated default.
#[tokio::test]
async fn t1302_strategy_id_optional() {
    let ledger = open_ledger().await;

    // post_fill with strategy_id = None writes a NULL into the column.
    let txn_id = journal::post_fill(&ledger, &make_paper_buy_fill(), Venue::Binance, None)
        .await
        .expect("post Buy fill (no strategy)");

    let meta = query::journal_transaction_metadata(&ledger, &txn_id)
        .await
        .expect("read journal transaction metadata")
        .expect("expected Some(metadata) for the legacy NULL-strategy row");

    assert_eq!(
        meta.strategy_id, None,
        "strategy_id NULL in the column must surface as None in the metadata"
    );
    assert_eq!(
        meta.transaction_id.as_str(),
        txn_id.as_str(),
        "transaction_id must still round-trip when strategy_id is NULL"
    );
    assert!(
        !meta.description.is_empty(),
        "description should be the populated paper-fill string, not empty"
    );
}
