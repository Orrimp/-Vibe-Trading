#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1712 — `audit::query::recent_journal_filtered` integration test
//! (Phase 3 R12 / Q7).
//!
//! Seeds 250 + 5 fills across two venues × three kinds (fill / non-fill
//! mix), asserts the page-2 cursor returns the expected tail and
//! `total_count` is `255`.

use audit::query::recent_journal_filtered;
use audit::{bootstrap, journal, Ledger};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    AuditKindFilter, AuditKindLabel, FeeTier, Fill, FillId, Liquidity, Money, OrderId, Price,
    Quantity, Side, Symbol, Timestamp, Venue,
};

async fn open_seeded_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap chart of accounts");
    ledger
}

fn ts_secs(secs: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
}

fn make_fill(symbol: &str, side: Side, qty: Decimal, price: Decimal, secs: i64) -> Fill {
    Fill {
        id: FillId::new(),
        order_id: OrderId::new(),
        symbol: Symbol::new(symbol),
        side,
        qty: Quantity::new(qty).expect("qty ok"),
        price: Price::new(price).expect("price ok"),
        fee: Money::from_decimal(dec!(0.5)),
        fee_tier: FeeTier::Taker,
        venue_ts: ts_secs(secs),
        local_ts: ts_secs(secs),
        liquidity: Liquidity::Taker,
        transaction_id: None,
    }
}

/// 250 + 5 = 255 rows seeded across two venues × multiple symbols.
/// Page 2 (offset 250, size 250) returns the tail 5; `total_count == 255`.
#[tokio::test]
async fn recent_journal_filtered_paginates_255_rows() {
    let ledger = open_seeded_ledger().await;

    // 255 rows alternating Binance / Coinbase, BTCUSDT / ETHUSDT, Buy / Sell.
    for n in 0..255i64 {
        let venue = if n % 2 == 0 {
            Venue::Binance
        } else {
            Venue::Coinbase
        };
        let symbol = if n % 3 == 0 { "BTCUSDT" } else { "ETHUSDT" };
        let side = if n % 2 == 0 { Side::Buy } else { Side::Sell };
        // Ascending ts so newer rows have larger ts.
        let secs = 1_000 + n;
        let fill = make_fill(symbol, side, dec!(0.1), dec!(40_000), secs);
        journal::post_fill(&ledger, &fill, venue, None)
            .await
            .expect("post fill");
    }

    let since = ts_secs(0);
    let until = ts_secs(10_000);

    // Page 0 with size 250 returns 250 rows; total is 255.
    let (page0, total0) = recent_journal_filtered(
        &ledger,
        &[],
        None,
        AuditKindFilter::All,
        since,
        until,
        0,
        250,
    )
    .await
    .expect("page 0");
    assert_eq!(total0, 255, "total_count must be 255");
    assert_eq!(page0.len(), 250, "page 0 size 250 returns 250 rows");

    // Page 2 == offset 250 in the 250-row pagination — returns the tail
    // 5 rows; `total_count` stays 255.
    let (page2, total2) = recent_journal_filtered(
        &ledger,
        &[],
        None,
        AuditKindFilter::All,
        since,
        until,
        250,
        250,
    )
    .await
    .expect("page 2");
    assert_eq!(total2, 255, "total_count consistent across pages");
    assert_eq!(
        page2.len(),
        5,
        "page 2 (offset 250) returns the tail 5 rows"
    );

    // Newest-first ordering — page 2's last row is the oldest seeded ts.
    assert_eq!(
        page2[4].ts,
        ts_secs(1_000),
        "last row of last page is the oldest seeded ts"
    );
}

/// Phase 3 R12 — venue predicate isolates the requested venues.
#[tokio::test]
async fn recent_journal_filtered_filters_by_venue_set() {
    let ledger = open_seeded_ledger().await;
    journal::post_fill(
        &ledger,
        &make_fill("BTCUSDT", Side::Buy, dec!(0.1), dec!(40_000), 1_000),
        Venue::Binance,
        None,
    )
    .await
    .expect("seed Binance");
    journal::post_fill(
        &ledger,
        &make_fill("BTCUSDT", Side::Buy, dec!(0.1), dec!(40_000), 2_000),
        Venue::Coinbase,
        None,
    )
    .await
    .expect("seed Coinbase");
    journal::post_fill(
        &ledger,
        &make_fill("BTCUSDT", Side::Buy, dec!(0.1), dec!(40_000), 3_000),
        Venue::Kraken,
        None,
    )
    .await
    .expect("seed Kraken");

    let since = ts_secs(0);
    let until = ts_secs(10_000);

    let (rows, total) = recent_journal_filtered(
        &ledger,
        &[Venue::Coinbase, Venue::Kraken],
        None,
        AuditKindFilter::All,
        since,
        until,
        0,
        250,
    )
    .await
    .expect("multi-venue query");
    assert_eq!(total, 2, "two-venue predicate isolates 2 rows");
    assert_eq!(rows.len(), 2);
    assert!(rows.iter().all(|r| matches!(r.kind, AuditKindLabel::Fill)));
    assert!(rows
        .iter()
        .all(|r| r.venue == Venue::Coinbase || r.venue == Venue::Kraken));
}
