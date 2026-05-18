//! T805 — `audit::journal::feed_reconnect` writer + parser arm integration.
//!
//! Acceptance per the architect's task spec:
//!  - writes one FeedReconnect event;
//!  - `strategy_events_since(early_ts)` returns it with the correct `kind`
//!    and `error_summary`;
//!  - reconciler `Σ debits == Σ credits` unchanged (no money columns).

use audit::query::{global_debit_credit_sum, strategy_events_since};
use audit::{Ledger, bootstrap, journal};
use time::OffsetDateTime;
use trading_core::{StrategyEventKind, Timestamp, Venue};

async fn open_ledger() -> Ledger {
    let ledger = Ledger::in_memory().await.expect("open in-memory");
    bootstrap::chart_of_accounts(&ledger)
        .await
        .expect("bootstrap");
    ledger
}

fn ts_epoch() -> Timestamp {
    Timestamp::new(OffsetDateTime::UNIX_EPOCH)
}

#[tokio::test]
async fn t805_feed_reconnect_writes_and_reads() {
    let ledger = open_ledger().await;

    // Capture pre-write debit/credit sum to assert reconciler invariant.
    let (dr_before, cr_before) = global_debit_credit_sum(&ledger)
        .await
        .expect("global sum before");

    // Use a deterministic ts so we exercise the injected-clock path.
    let ts_str = "2030-06-01T00:00:00.123456Z";
    journal::feed_reconnect(&ledger, "BTCUSDT", Venue::Binance, Some(ts_str))
        .await
        .expect("feed_reconnect write");

    // Read it back via strategy_events_since.
    let events = strategy_events_since(&ledger, ts_epoch())
        .await
        .expect("strategy_events_since");

    assert_eq!(events.len(), 1, "expected exactly 1 event");
    let ev = &events[0];
    assert_eq!(
        ev.kind,
        StrategyEventKind::FeedReconnect,
        "kind must be FeedReconnect"
    );
    assert_eq!(
        ev.error_summary.as_deref(),
        Some("BTCUSDT"),
        "error_summary must carry the symbol"
    );
    assert!(ev.strategy_id.is_none(), "feed-level event has no strategy");
    assert_eq!(
        ev.error_code.as_deref(),
        Some("feed_reconnect"),
        "error_code is set"
    );

    // Reconciler invariant — strategy_events carry no money.
    let (dr_after, cr_after) = global_debit_credit_sum(&ledger)
        .await
        .expect("global sum after");
    assert_eq!(dr_before, dr_after, "feed_reconnect must not affect debits");
    assert_eq!(
        cr_before, cr_after,
        "feed_reconnect must not affect credits"
    );
}

#[tokio::test]
async fn t805_feed_reconnect_microsecond_timestamp_preserved() {
    // Determinism — when caller supplies a ts, it round-trips verbatim.
    let ledger = open_ledger().await;
    let ts_str = "2030-06-01T00:00:00.654321Z";
    journal::feed_reconnect(&ledger, "ETHUSDT", Venue::Binance, Some(ts_str))
        .await
        .expect("feed_reconnect");

    // Use a raw query because the TS in StrategyEventView is parsed back
    // into Timestamp — verify the microseconds survive round-trip.
    let rows: Vec<(String,)> =
        sqlx::query_as("SELECT ts FROM strategy_events WHERE kind = 'FeedReconnect'")
            .fetch_all(ledger.pool())
            .await
            .expect("select ts");

    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].0, ts_str, "microsecond ts must round-trip verbatim");
}
