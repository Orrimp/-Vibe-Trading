//! Binance WebSocket integration test (T08 acceptance).
//!
//! Connects to the Binance **public** WebSocket endpoint, subscribes to
//! `btcusdt@kline_1m` and `btcusdt@trade`, and verifies live message receipt
//! including a reconnect drill.
//!
//! # Running
//!
//! These tests are gated with `#[ignore]` because they require a live network
//! connection to the Binance production WebSocket and should not run in normal
//! CI without an explicit opt-in.
//!
//! ```bash
//! cargo test -p data --test binance_ws_integration -- --ignored
//! ```
//!
//! They are counted against T08's acceptance criterion when run in the
//! integration validation step.

use data::{BinanceFeed, source::MarketDataSource};
use futures::StreamExt;
use std::time::Duration;
use tokio::time::timeout;
use trading_core::{Symbol, Timeframe};

/// T08-A: receive at least one kline within 30 seconds.
#[tokio::test]
#[ignore = "requires live Binance WebSocket connection — run with --ignored"]
async fn t08_receives_kline_within_30s() {
    let feed = BinanceFeed::production();
    let symbol = Symbol::new("BTCUSDT");

    let mut stream = feed
        .subscribe_bars(symbol.clone(), Timeframe::OneMinute)
        .await
        .expect("subscribe_bars should succeed");

    let result = timeout(Duration::from_secs(30), stream.next()).await;
    match result {
        Ok(Some(Ok(bar))) => {
            assert_eq!(bar.symbol, symbol, "received bar for correct symbol");
        }
        Ok(Some(Err(e))) => panic!("stream error: {e}"),
        Ok(None) => panic!("stream closed before receiving a bar"),
        Err(_) => panic!("timed out (30s) waiting for first kline"),
    }
}

/// T08-B: receive at least one trade within 30 seconds.
#[tokio::test]
#[ignore = "requires live Binance WebSocket connection — run with --ignored"]
async fn t08_receives_trade_within_30s() {
    let feed = BinanceFeed::production();
    let symbol = Symbol::new("BTCUSDT");

    let mut stream = feed
        .subscribe_trades(symbol.clone())
        .await
        .expect("subscribe_trades should succeed");

    let result = timeout(Duration::from_secs(30), stream.next()).await;
    match result {
        Ok(Some(Ok(tick))) => {
            assert_eq!(tick.symbol, symbol, "received tick for correct symbol");
        }
        Ok(Some(Err(e))) => panic!("stream error: {e}"),
        Ok(None) => panic!("stream closed before receiving a tick"),
        Err(_) => panic!("timed out (30s) waiting for first trade"),
    }
}

/// T08-C: reconnect drill — drop the stream, re-subscribe, and receive another
/// message within 5 seconds.
///
/// Simulates a mid-stream disconnect by dropping the first subscription and
/// immediately creating a new one. The `BinanceFeed` reconnect logic must
/// allow opening a fresh connection quickly.
#[tokio::test]
#[ignore = "requires live Binance WebSocket connection — run with --ignored"]
async fn t08_reconnect_recovers_within_5s() {
    let feed = BinanceFeed::production();
    let symbol = Symbol::new("BTCUSDT");

    // Phase 1: open a trade stream and receive at least one item.
    {
        let mut stream = feed
            .subscribe_trades(symbol.clone())
            .await
            .expect("subscribe_trades (initial) should succeed");
        // Wait for at least one tick so we know the connection is live.
        let first = timeout(Duration::from_secs(30), stream.next()).await;
        match first {
            Ok(Some(Ok(_))) => {} // got a tick — connection confirmed live
            Ok(Some(Err(e))) => panic!("initial stream error: {e}"),
            Ok(None) => panic!("initial stream closed unexpectedly"),
            Err(_) => panic!("timed out waiting for initial connection"),
        }
        // Drop `stream` here — simulates a disconnection by closing the task.
    }

    // Phase 2: immediately re-subscribe and assert recovery within 5 seconds.
    let mut stream2 = feed
        .subscribe_trades(symbol.clone())
        .await
        .expect("subscribe_trades (reconnect) should succeed");

    let result = timeout(Duration::from_secs(5), stream2.next()).await;
    match result {
        Ok(Some(Ok(tick))) => {
            assert_eq!(
                tick.symbol, symbol,
                "recovered tick is for the correct symbol"
            );
        }
        Ok(Some(Err(e))) => panic!("reconnect stream error: {e}"),
        Ok(None) => panic!("reconnect stream closed without yielding a tick"),
        Err(_) => panic!("timed out (5s) after reconnect — feed did not recover"),
    }
}
