#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T2015 — `audit::query::recent_signals` acceptance.
//!
//! Per `spec/v1/chart-buy-sell-emphasis/feature.md` Design § Q1 + V11.
//!
//! Verifies:
//! - V11a — correct rows in correct order on a seeded ledger.
//! - V11b — empty window returns `Ok(vec![])`.
//! - V11c — gate-off ledger (zero rows in `strategy_signals`) returns
//!   `Ok(vec![])` without erroring.
//! - Round-trip — the reader's `SignalView` field values match what
//!   the writer was handed.
//! - `was_clamped` reflects the post-UPDATE state set by
//!   `update_signal_clamp_status`.
//! - Venue + symbol predicates isolate rows from other venues / symbols
//!   inside the same window.

use audit::journal::{post_strategy_signal, update_signal_clamp_status};
use audit::query::recent_signals;
use audit::{Ledger, bootstrap};
use rust_decimal_macros::dec;
use time::OffsetDateTime;
use trading_core::{
    Quantity, Side, Signal, SignalEvidence, SignalKind, StrategyId, Symbol, Timestamp, Venue,
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

fn make_signal(kind: SignalKind, symbol: &str, secs: i64, strategy: &str) -> Signal {
    Signal {
        strategy_id: StrategyId::new(strategy),
        symbol: Symbol::new(symbol),
        ts: ts_secs(secs),
        kind,
        evidence: SignalEvidence::empty(),
        pair_data: None,
    }
}

/// V11a — three rows seeded inside the window; reader returns them
/// newest-first with all `SignalView` fields populated correctly.
#[tokio::test]
async fn recent_signals_returns_window_subset() {
    let ledger = open_seeded_ledger().await;

    // Seed three BTCUSDT signals on Binance inside the window, plus
    // one outside (later than `until` — must be excluded).
    let s1 = make_signal(SignalKind::Buy, "BTCUSDT", 100, "sma_crossover");
    let s2 = make_signal(SignalKind::Sell, "BTCUSDT", 200, "sma_crossover");
    let s3 = make_signal(SignalKind::Buy, "BTCUSDT", 300, "sma_crossover");
    let s_outside = make_signal(SignalKind::Buy, "BTCUSDT", 5_000, "sma_crossover");
    let qty = Quantity::new(dec!(0.05)).expect("qty");

    let _id1 = post_strategy_signal(&ledger, &s1, qty, None, Venue::Binance, false, None, None)
        .await
        .expect("post s1");
    let _id2 = post_strategy_signal(&ledger, &s2, qty, None, Venue::Binance, false, None, None)
        .await
        .expect("post s2");
    let id3 = post_strategy_signal(
        &ledger,
        &s3,
        qty,
        None,
        Venue::Binance,
        true,
        Some("per_symbol_cap"),
        None, // forecast_correlation_id (Phase D R1.3)
    )
    .await
    .expect("post s3");
    let _id_outside = post_strategy_signal(
        &ledger,
        &s_outside,
        qty,
        None,
        Venue::Binance,
        false,
        None,
        None,
    )
    .await
    .expect("post outside");

    let since = ts_secs(0);
    let until = ts_secs(1_000);
    let rows = recent_signals(
        &ledger,
        Venue::Binance,
        Symbol::new("BTCUSDT"),
        since,
        until,
    )
    .await
    .expect("recent_signals");

    assert_eq!(
        rows.len(),
        3,
        "expected 3 in-window rows; outside-window row must be excluded"
    );

    // Newest first — id3 (ts=300) → s2 (ts=200) → s1 (ts=100).
    assert_eq!(rows[0].signal_id, id3);
    assert_eq!(rows[0].symbol, Symbol::new("BTCUSDT"));
    assert_eq!(rows[0].side, Side::Buy);
    assert_eq!(rows[0].intended_qty, qty);
    assert_eq!(rows[0].strategy_id, StrategyId::new("sma_crossover"));
    assert!(rows[0].was_clamped, "s3 was posted with was_clamped=true");
    assert_eq!(rows[0].clamp_reason.as_deref(), Some("per_symbol_cap"));

    assert_eq!(rows[1].side, Side::Sell, "s2 is a Sell");
    assert!(!rows[1].was_clamped);
    assert!(rows[1].clamp_reason.is_none());

    assert_eq!(rows[2].side, Side::Buy, "s1 is a Buy");
}

/// V11b — empty window (since == until OR no rows in range) returns
/// `Ok(vec![])`. Never `Err` for "no rows".
#[tokio::test]
async fn recent_signals_empty_window_returns_ok_empty() {
    let ledger = open_seeded_ledger().await;

    // Seed a row outside the queried window.
    let s = make_signal(SignalKind::Buy, "BTCUSDT", 100, "sma_crossover");
    let qty = Quantity::new(dec!(0.1)).expect("qty");
    let _ = post_strategy_signal(&ledger, &s, qty, None, Venue::Binance, false, None, None)
        .await
        .expect("post seed");

    // Query a window before the seeded row.
    let since = ts_secs(1_000_000);
    let until = ts_secs(2_000_000);
    let rows = recent_signals(
        &ledger,
        Venue::Binance,
        Symbol::new("BTCUSDT"),
        since,
        until,
    )
    .await
    .expect("query");
    assert!(rows.is_empty(), "out-of-window query returns Ok(vec![])");
}

/// V11c — gate-off ledger (no rows written ever) returns `Ok(vec![])`
/// even for a wide window. This is the production default state
/// (`[signal_log] enabled = false`).
#[tokio::test]
async fn recent_signals_gate_off_ledger_returns_ok_empty() {
    let ledger = open_seeded_ledger().await;

    // No `post_strategy_signal` calls — the `strategy_signals` table
    // is empty (V11c — operator has not opted in to the signal-log
    // gate).
    let since = ts_secs(0);
    let until = ts_secs(i64::from(u32::MAX));
    let rows = recent_signals(
        &ledger,
        Venue::Binance,
        Symbol::new("BTCUSDT"),
        since,
        until,
    )
    .await
    .expect("query gate-off");
    assert!(
        rows.is_empty(),
        "gate-off ledger must return Ok(vec![]) — never Err for the no-rows case"
    );
}

/// V11d — `update_signal_clamp_status` is reflected on the next reader
/// call. Defends against a stale-read regression where the reader would
/// return the pre-UPDATE `was_clamped`.
#[tokio::test]
async fn recent_signals_reflects_post_update_clamp_status() {
    let ledger = open_seeded_ledger().await;

    let s = make_signal(SignalKind::Sell, "ETHUSDT", 150, "alpha");
    let qty = Quantity::new(dec!(0.25)).expect("qty");
    let id = post_strategy_signal(&ledger, &s, qty, None, Venue::Binance, false, None, None)
        .await
        .expect("INSERT");

    // Pre-UPDATE — was_clamped = false.
    let pre = recent_signals(
        &ledger,
        Venue::Binance,
        Symbol::new("ETHUSDT"),
        ts_secs(0),
        ts_secs(1_000),
    )
    .await
    .expect("pre-UPDATE read");
    assert_eq!(pre.len(), 1);
    assert!(!pre[0].was_clamped);

    // UPDATE — flip the field.
    update_signal_clamp_status(&ledger, id.as_str(), true, Some("daily_loss_cap"))
        .await
        .expect("UPDATE");

    // Post-UPDATE — was_clamped = true; clamp_reason set.
    let post = recent_signals(
        &ledger,
        Venue::Binance,
        Symbol::new("ETHUSDT"),
        ts_secs(0),
        ts_secs(1_000),
    )
    .await
    .expect("post-UPDATE read");
    assert_eq!(post.len(), 1);
    assert!(post[0].was_clamped, "was_clamped must reflect the UPDATE");
    assert_eq!(post[0].clamp_reason.as_deref(), Some("daily_loss_cap"));
}

/// V11e — venue + symbol predicates isolate rows from other
/// (venue, symbol) tuples inside the same window. Defends against the
/// reader returning cross-venue / cross-symbol leakage.
#[tokio::test]
async fn recent_signals_isolates_by_venue_and_symbol() {
    let ledger = open_seeded_ledger().await;

    let qty = Quantity::new(dec!(0.05)).expect("qty");

    // Same time, same strategy, different (venue, symbol) tuples.
    let btc_binance = make_signal(SignalKind::Buy, "BTCUSDT", 100, "sma_crossover");
    let eth_binance = make_signal(SignalKind::Buy, "ETHUSDT", 100, "sma_crossover");
    let btc_coinbase = make_signal(SignalKind::Buy, "BTCUSDT", 100, "sma_crossover");

    let _ = post_strategy_signal(
        &ledger,
        &btc_binance,
        qty,
        None,
        Venue::Binance,
        false,
        None,
        None, // forecast_correlation_id (Phase D R1.3)
    )
    .await
    .expect("post btc/binance");
    let _ = post_strategy_signal(
        &ledger,
        &eth_binance,
        qty,
        None,
        Venue::Binance,
        false,
        None,
        None, // forecast_correlation_id (Phase D R1.3)
    )
    .await
    .expect("post eth/binance");
    let _ = post_strategy_signal(
        &ledger,
        &btc_coinbase,
        qty,
        None,
        Venue::Coinbase,
        false,
        None,
        None, // forecast_correlation_id (Phase D R1.3)
    )
    .await
    .expect("post btc/coinbase");

    let since = ts_secs(0);
    let until = ts_secs(1_000);

    // BTC/Binance — exactly 1 row.
    let btc_bn = recent_signals(
        &ledger,
        Venue::Binance,
        Symbol::new("BTCUSDT"),
        since,
        until,
    )
    .await
    .expect("read btc/binance");
    assert_eq!(btc_bn.len(), 1);
    assert_eq!(btc_bn[0].symbol, Symbol::new("BTCUSDT"));

    // ETH/Binance — exactly 1 row.
    let eth_bn = recent_signals(
        &ledger,
        Venue::Binance,
        Symbol::new("ETHUSDT"),
        since,
        until,
    )
    .await
    .expect("read eth/binance");
    assert_eq!(eth_bn.len(), 1);
    assert_eq!(eth_bn[0].symbol, Symbol::new("ETHUSDT"));

    // BTC/Coinbase — exactly 1 row (the cross-venue row, isolated by
    // the `venue = ?` predicate).
    let btc_cb = recent_signals(
        &ledger,
        Venue::Coinbase,
        Symbol::new("BTCUSDT"),
        since,
        until,
    )
    .await
    .expect("read btc/coinbase");
    assert_eq!(btc_cb.len(), 1);
}
