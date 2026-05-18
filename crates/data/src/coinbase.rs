//! Coinbase Advanced Trade WebSocket feed (T1403).
//!
//! WS endpoint: `wss://advanced-trade-ws.coinbase.com` (Q2 / Q8).
//!
//! Channels:
//!   - `market_trades` — raw individual trades
//!   - `candles`       — venue-closed OHLCV bars
//!
//! Auto-reconnects with exponential back-off (1 s base, cap 60 s).
//! Answers server pings with pongs per the WS protocol.
//!
//! See `spec/features/v1-5b-multi-venue.md` Q2, Q8, R1, R15 — and
//! the sample on-wire payloads section in that brief.
use std::str::FromStr;

use async_trait::async_trait;
use futures::StreamExt;
use futures::stream::BoxStream;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use tokio::time::{Duration, sleep};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, warn};
use trading_core::{
    Bar, FeedError, Price, Quantity, Side, Symbol, Tick, Timeframe, Timestamp, Venue,
};

use crate::source::{MarketDataSource, SymbolInfo};

// ── Coinbase REST product response (minimal) ─────────────────────────────────

#[derive(Debug, Deserialize)]
#[serde(rename_all = "snake_case")]
struct CoinbaseProduct {
    product_id: String,
    base_currency_id: Option<String>,
    quote_currency_id: Option<String>,
    base_min_size: Option<String>,
    quote_increment: Option<String>,
    base_increment: Option<String>,
    /// Coinbase historically used `min_market_funds`; some endpoints expose
    /// `min_funds`. Accept either.
    #[serde(alias = "min_funds")]
    min_market_funds: Option<String>,
}

// ── Coinbase WS message shapes (Advanced Trade) ──────────────────────────────

/// Wrapper for any market-data event in the Advanced Trade WS protocol.
/// Messages of interest carry `channel` ∈ {`"market_trades"`, `"candles"`}
/// and an `events` array. Subscription acks (`channel == "subscriptions"`)
/// are also delivered here; the parser ignores them.
#[derive(Debug, Deserialize)]
struct AdvancedTradeEnvelope {
    channel: String,
    #[serde(default)]
    events: Vec<Value>,
}

/// `market_trades` event payload (per channel).
#[derive(Debug, Deserialize)]
struct MarketTradesEvent {
    #[serde(default)]
    trades: Vec<MarketTrade>,
}

/// Individual trade in a `market_trades` event.
#[derive(Debug, Deserialize)]
struct MarketTrade {
    trade_id: String,
    #[allow(dead_code)]
    product_id: String,
    price: String,
    size: String,
    /// `"BUY"` or `"SELL"` — aggressor side.
    side: String,
    time: String,
}

/// `candles` event payload.
#[derive(Debug, Deserialize)]
struct CandlesEvent {
    #[serde(default)]
    candles: Vec<Candle>,
}

#[derive(Debug, Deserialize)]
struct Candle {
    /// Bucket start, Unix seconds (string-encoded by the venue).
    start: String,
    open: String,
    high: String,
    low: String,
    close: String,
    volume: String,
    #[allow(dead_code)]
    product_id: String,
}

// ── CoinbaseFeed ─────────────────────────────────────────────────────────────

/// Coinbase Advanced Trade WebSocket adapter.
pub struct CoinbaseFeed {
    pub ws_url: String,
    pub rest_url: String,
    /// Optional audit ledger handle (T805 / Q11 — operator success reports R7.1).
    ///
    /// When `Some`, every WS reconnection writes a `FeedReconnect` strategy
    /// event (with `venue: Venue::Coinbase`) to the ledger.  When `None`, no
    /// audit write happens — kept `Option` so test/research callers can
    /// build a `CoinbaseFeed` without a Ledger.
    pub ledger: Option<std::sync::Arc<audit::Ledger>>,
}

impl CoinbaseFeed {
    /// Create a new `CoinbaseFeed` with explicit URLs.
    #[must_use]
    pub fn with_urls(ws_url: impl Into<String>, rest_url: impl Into<String>) -> Self {
        Self {
            ws_url: ws_url.into(),
            rest_url: rest_url.into(),
            ledger: None,
        }
    }

    /// Default Coinbase Advanced Trade production URLs.
    #[must_use]
    pub fn production() -> Self {
        Self::with_urls(
            "wss://advanced-trade-ws.coinbase.com",
            "https://api.coinbase.com",
        )
    }

    /// Builder-style helper that attaches an audit ledger so WS reconnects
    /// emit `FeedReconnect` strategy events (T805 / Q11).
    #[must_use]
    pub fn with_ledger(mut self, ledger: std::sync::Arc<audit::Ledger>) -> Self {
        self.ledger = Some(ledger);
        self
    }
}

// ── Symbol mapping ───────────────────────────────────────────────────────────

/// Map an agent-native slash-free symbol to Coinbase Advanced Trade
/// `product_id` shape.
///
/// Strategy: insert a `-` between the base and the quote.  v1.5b's
/// universe quotes USD / USDC / USDT; the function inspects the suffix
/// and falls back to inserting `-` before the last 3 characters when no
/// known quote suffix matches (defensive default).
///
/// Examples:
/// - `"BTCUSDC"` → `"BTC-USDC"`
/// - `"ETHUSDC"` → `"ETH-USDC"`
/// - `"BTCUSD"`  → `"BTC-USD"`
#[must_use]
pub fn coinbase_symbol_map(s: &Symbol) -> String {
    let raw = s.0.as_str().to_uppercase();
    for q in ["USDC", "USDT", "USD"] {
        if let Some(base) = raw.strip_suffix(q)
            && !base.is_empty()
        {
            return format!("{base}-{q}");
        }
    }
    // Defensive fallback: split at the third-from-last char.
    if raw.len() > 3 {
        let split = raw.len() - 3;
        let (base, quote) = raw.split_at(split);
        format!("{base}-{quote}")
    } else {
        raw
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_decimal(s: &str, field: &str) -> Result<Decimal, FeedError> {
    Decimal::from_str(s).map_err(|_| FeedError::Parse(format!("bad {field}: {s}")))
}

fn parse_price(s: &str, field: &str) -> Result<Price, FeedError> {
    let d = parse_decimal(s, field)?;
    Price::new(d).map_err(|e| FeedError::Parse(e.to_string()))
}

fn parse_qty(s: &str, field: &str) -> Result<Quantity, FeedError> {
    let d = parse_decimal(s, field)?;
    Quantity::new(d).map_err(|e| FeedError::Parse(e.to_string()))
}

fn parse_rfc3339(s: &str) -> Result<Timestamp, FeedError> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map(Timestamp::new)
        .map_err(|e| FeedError::Parse(format!("bad timestamp {s}: {e}")))
}

fn unix_secs_to_timestamp(secs: i64) -> Timestamp {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(secs) * 1_000_000_000)
        .map(Timestamp::new)
        .unwrap_or_else(|_| Timestamp::now())
}

/// Parse a single Coinbase `market_trades` event payload (the value of the
/// `events[i]` array element).  Returns an iterator of parsed `Tick`s for
/// the given symbol.
///
/// This function is the on-wire boundary unit-tested directly per Q10
/// (no WS server is stood up in tests).
fn parse_market_trades_event(
    event_payload: &Value,
    symbol: &Symbol,
    local_recv_ts: Timestamp,
) -> Vec<Result<Tick, FeedError>> {
    let evt: MarketTradesEvent = match serde_json::from_value(event_payload.clone()) {
        Ok(e) => e,
        Err(e) => {
            return vec![Err(FeedError::Parse(format!("market_trades event: {e}")))];
        }
    };
    evt.trades
        .into_iter()
        .map(|t| {
            let venue_ts = parse_rfc3339(&t.time)?;
            let trade_id_u64 = t
                .trade_id
                .parse::<u64>()
                .map_err(|e| FeedError::Parse(format!("bad trade_id {}: {e}", t.trade_id)))?;
            let side = match t.side.to_ascii_uppercase().as_str() {
                "BUY" => Side::Buy,
                "SELL" => Side::Sell,
                other => {
                    return Err(FeedError::Parse(format!("unknown side: {other}")));
                }
            };
            Ok(Tick {
                symbol: symbol.clone(),
                venue_ts,
                local_recv_ts,
                price: parse_price(&t.price, "price")?,
                qty: parse_qty(&t.size, "size")?,
                side,
                trade_id: trade_id_u64,
                venue: Venue::Coinbase,
            })
        })
        .collect()
}

/// Parse a single Coinbase `candles` event payload to a vector of `Bar`s.
fn parse_candles_event(
    event_payload: &Value,
    symbol: &Symbol,
    tf: Timeframe,
    local_recv_ts: Timestamp,
) -> Vec<Result<Bar, FeedError>> {
    let evt: CandlesEvent = match serde_json::from_value(event_payload.clone()) {
        Ok(e) => e,
        Err(e) => {
            return vec![Err(FeedError::Parse(format!("candles event: {e}")))];
        }
    };
    evt.candles
        .into_iter()
        .map(|c| {
            let start_secs: i64 = c
                .start
                .parse()
                .map_err(|e| FeedError::Parse(format!("bad candle start {}: {e}", c.start)))?;
            let open_ts = unix_secs_to_timestamp(start_secs);
            // Coinbase publishes 1-minute candles as the `candles` channel
            // default. Close is open + 60s - 1µs (matches the in-house 1m
            // convention used by Binance feed).
            let close_secs = match tf {
                Timeframe::OneSecond => start_secs + 1,
                Timeframe::OneMinute => start_secs + 60,
                Timeframe::FiveMinutes => start_secs + 300,
                Timeframe::FifteenMinutes => start_secs + 900,
                Timeframe::OneHour => start_secs + 3600,
                Timeframe::FourHours => start_secs + 14400,
                Timeframe::OneDay => start_secs + 86400,
            };
            let close_ts = unix_secs_to_timestamp(close_secs);
            Ok(Bar {
                symbol: symbol.clone(),
                tf,
                open_ts,
                close_ts,
                open: parse_price(&c.open, "open")?,
                high: parse_price(&c.high, "high")?,
                low: parse_price(&c.low, "low")?,
                close: parse_price(&c.close, "close")?,
                volume: parse_qty(&c.volume, "volume")?,
                trade_count: 0, // Coinbase candles do not carry trade_count
                local_recv_ts,
                venue: Venue::Coinbase,
            })
        })
        .collect()
}

/// Build a Coinbase Advanced Trade subscribe message for a given channel
/// and product_id list. Public channels are unauthenticated (Q8) — no JWT.
fn build_subscribe_message(channel: &str, product_ids: &[String]) -> String {
    let body = serde_json::json!({
        "type": "subscribe",
        "channel": channel,
        "product_ids": product_ids,
    });
    body.to_string()
}

async fn connect_ws(
    url: &str,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    FeedError,
> {
    let parsed = url
        .parse::<tokio_tungstenite::tungstenite::http::Uri>()
        .map_err(|e| FeedError::Connection(e.to_string()))?;
    tokio_tungstenite::connect_async(parsed)
        .await
        .map(|(ws, _resp)| ws)
        .map_err(|e| FeedError::Connection(e.to_string()))
}

// ── MarketDataSource impl ────────────────────────────────────────────────────

#[async_trait]
impl MarketDataSource for CoinbaseFeed {
    /// Fetch product metadata via the Advanced Trade brokerage REST endpoint:
    /// `GET /api/v3/brokerage/products/{product_id}`.
    async fn exchange_info(&self, symbol: Symbol) -> Result<SymbolInfo, FeedError> {
        let product_id = coinbase_symbol_map(&symbol);
        let url = format!("{}/api/v3/brokerage/products/{}", self.rest_url, product_id);
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| FeedError::Connection(e.to_string()))?;
        let product: CoinbaseProduct = resp
            .json()
            .await
            .map_err(|e| FeedError::Parse(e.to_string()))?;

        // Sanity: ensure the returned product matches the request.
        if !product.product_id.eq_ignore_ascii_case(&product_id) {
            return Err(FeedError::StreamClosed);
        }

        let base_asset = product.base_currency_id.unwrap_or_default();
        let quote_asset = product.quote_currency_id.unwrap_or_default();

        let min_qty = product
            .base_min_size
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_else(|| Decimal::new(1, 5));
        // Lot size approximates `base_increment`; fall back to `min_qty`.
        let lot_size = product
            .base_increment
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or(min_qty);
        // `min_notional` ≈ `min_market_funds` (Coinbase's rough equivalent).
        let min_notional = product
            .min_market_funds
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_else(|| Decimal::new(10, 0));
        // `quote_increment` is captured but not surfaced in `SymbolInfo` today.
        let _ = product.quote_increment;

        Ok(SymbolInfo {
            symbol: Symbol::new(symbol.0.as_str().to_uppercase()),
            base_asset,
            quote_asset,
            min_qty,
            lot_size,
            min_notional,
        })
    }

    async fn subscribe_bars(
        &self,
        symbol: Symbol,
        tf: Timeframe,
    ) -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError> {
        let product_id = coinbase_symbol_map(&symbol);
        let ws_url = self.ws_url.clone();
        let symbol_clone = symbol.clone();
        let symbol_for_audit = symbol.clone();
        let ledger_for_stream = self.ledger.clone();

        // Verify initial connection before returning the stream.
        let _ws = connect_ws(&ws_url).await?;

        let stream = async_stream::stream! {
            let mut backoff_secs: u64 = 1;
            let mut is_reconnect = false;
            loop {
                debug!(product = %product_id, "connecting to coinbase candles WS");
                match connect_ws(&ws_url).await {
                    Err(e) => {
                        error!(error = %e, "coinbase candles WS connect failed, retrying in {backoff_secs}s");
                        sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(60);
                        continue;
                    }
                    Ok(mut ws) => {
                        backoff_secs = 1;
                        // Send subscribe message.
                        let sub = build_subscribe_message("candles", std::slice::from_ref(&product_id));
                        if let Err(e) = futures::SinkExt::send(&mut ws, Message::Text(sub.into())).await {
                            warn!(error = %e, "coinbase candles subscribe send failed");
                            continue;
                        }
                        if is_reconnect
                            && let Some(ledger) = ledger_for_stream.as_ref()
                            && let Err(e) = audit::journal::feed_reconnect(
                                ledger,
                                symbol_for_audit.0.as_str(),
                                Venue::Coinbase,
                                None,
                            ).await
                        {
                            warn!(error = %e, "coinbase feed_reconnect audit write failed (non-fatal)");
                        }
                        is_reconnect = true;
                        loop {
                            match ws.next().await {
                                None => { warn!("coinbase candles WS closed, reconnecting"); break; }
                                Some(Err(e)) => { warn!(error = %e, "coinbase candles WS error, reconnecting"); break; }
                                Some(Ok(Message::Ping(data))) => {
                                    if let Err(e) = futures::SinkExt::send(&mut ws, Message::Pong(data)).await {
                                        warn!(error = %e, "coinbase pong send failed");
                                        break;
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    let local_ts = Timestamp::now();
                                    let env: AdvancedTradeEnvelope = match serde_json::from_str(&text) {
                                        Ok(e) => e,
                                        Err(e) => {
                                            warn!(error = %e, text = %text, "coinbase envelope parse error");
                                            continue;
                                        }
                                    };
                                    if env.channel != "candles" { continue; }
                                    for ev in &env.events {
                                        for bar_res in parse_candles_event(ev, &symbol_clone, tf, local_ts) {
                                            yield bar_res;
                                        }
                                    }
                                }
                                Some(Ok(_)) => {}
                            }
                        }
                    }
                }
                sleep(Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(60);
            }
        };
        Ok(Box::pin(stream))
    }

    async fn subscribe_trades(
        &self,
        symbol: Symbol,
    ) -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError> {
        let product_id = coinbase_symbol_map(&symbol);
        let ws_url = self.ws_url.clone();
        let symbol_clone = symbol.clone();
        let symbol_for_audit = symbol.clone();
        let ledger_for_stream = self.ledger.clone();

        let _ws = connect_ws(&ws_url).await?;

        let stream = async_stream::stream! {
            let mut backoff_secs: u64 = 1;
            let mut is_reconnect = false;
            loop {
                debug!(product = %product_id, "connecting to coinbase market_trades WS");
                match connect_ws(&ws_url).await {
                    Err(e) => {
                        error!(error = %e, "coinbase market_trades WS connect failed, retrying in {backoff_secs}s");
                        sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(60);
                        continue;
                    }
                    Ok(mut ws) => {
                        backoff_secs = 1;
                        let sub = build_subscribe_message("market_trades", std::slice::from_ref(&product_id));
                        if let Err(e) = futures::SinkExt::send(&mut ws, Message::Text(sub.into())).await {
                            warn!(error = %e, "coinbase market_trades subscribe send failed");
                            continue;
                        }
                        if is_reconnect
                            && let Some(ledger) = ledger_for_stream.as_ref()
                            && let Err(e) = audit::journal::feed_reconnect(
                                ledger,
                                symbol_for_audit.0.as_str(),
                                Venue::Coinbase,
                                None,
                            ).await
                        {
                            warn!(error = %e, "coinbase feed_reconnect audit write failed (non-fatal)");
                        }
                        is_reconnect = true;
                        loop {
                            match ws.next().await {
                                None => { warn!("coinbase market_trades WS closed, reconnecting"); break; }
                                Some(Err(e)) => { warn!(error = %e, "coinbase market_trades WS error, reconnecting"); break; }
                                Some(Ok(Message::Ping(data))) => {
                                    if let Err(e) = futures::SinkExt::send(&mut ws, Message::Pong(data)).await {
                                        warn!(error = %e, "coinbase pong send failed");
                                        break;
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    let local_ts = Timestamp::now();
                                    let env: AdvancedTradeEnvelope = match serde_json::from_str(&text) {
                                        Ok(e) => e,
                                        Err(e) => {
                                            warn!(error = %e, text = %text, "coinbase envelope parse error");
                                            continue;
                                        }
                                    };
                                    if env.channel != "market_trades" { continue; }
                                    for ev in &env.events {
                                        for tick_res in parse_market_trades_event(ev, &symbol_clone, local_ts) {
                                            yield tick_res;
                                        }
                                    }
                                }
                                Some(Ok(_)) => {}
                            }
                        }
                    }
                }
                sleep(Duration::from_secs(backoff_secs)).await;
                backoff_secs = (backoff_secs * 2).min(60);
            }
        };
        Ok(Box::pin(stream))
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::uninlined_format_args
)]
mod tests {
    use super::*;

    #[test]
    fn coinbase_symbol_map_usdc_quote() {
        let s = Symbol::new("BTCUSDC");
        assert_eq!(coinbase_symbol_map(&s), "BTC-USDC");
        let s = Symbol::new("ETHUSDC");
        assert_eq!(coinbase_symbol_map(&s), "ETH-USDC");
    }

    #[test]
    fn coinbase_symbol_map_usd_quote() {
        let s = Symbol::new("BTCUSD");
        assert_eq!(coinbase_symbol_map(&s), "BTC-USD");
    }

    #[test]
    fn coinbase_symbol_map_lowercase_input_normalizes() {
        let s = Symbol::new("btcusdc");
        assert_eq!(coinbase_symbol_map(&s), "BTC-USDC");
    }

    /// T1403 — verify that the subscribe message we put on the wire matches
    /// the Coinbase Advanced Trade WS protocol shape (Q2 / Q8).
    #[test]
    fn t1403_coinbase_subscription_message_shape() {
        let msg = build_subscribe_message("market_trades", &["BTC-USDC".to_string()]);
        let v: serde_json::Value = serde_json::from_str(&msg).expect("valid json");
        assert_eq!(v["type"], "subscribe");
        assert_eq!(v["channel"], "market_trades");
        let products = v["product_ids"].as_array().expect("product_ids array");
        assert_eq!(products.len(), 1);
        assert_eq!(products[0], "BTC-USDC");
    }

    #[test]
    fn t1403_parses_market_trades_event_to_tick() {
        // Per Design § Sample on-wire payloads: events array carries
        // {"trade_id":"...","product_id":"BTC-USD","price":"60000.00",
        //  "size":"0.001","side":"BUY","time":"<RFC3339>"}.
        let payload = serde_json::json!({
            "trades": [
                {
                    "trade_id": "12345",
                    "product_id": "BTC-USDC",
                    "price": "60000.00",
                    "size": "0.001",
                    "side": "BUY",
                    "time": "2026-05-01T12:00:00.123456Z"
                }
            ]
        });
        let symbol = Symbol::new("BTCUSDC");
        let local_ts = Timestamp::now();
        let parsed = parse_market_trades_event(&payload, &symbol, local_ts);
        assert_eq!(parsed.len(), 1);
        let tick = parsed.into_iter().next().unwrap().expect("parse ok");
        assert_eq!(tick.venue, Venue::Coinbase);
        assert_eq!(tick.symbol, symbol);
        assert_eq!(tick.side, Side::Buy);
        assert_eq!(tick.trade_id, 12345);
        assert_eq!(tick.price.get(), Decimal::from_str("60000.00").unwrap());
        assert_eq!(tick.qty.get(), Decimal::from_str("0.001").unwrap());
    }

    #[test]
    fn t1403_parses_market_trades_sell_side() {
        let payload = serde_json::json!({
            "trades": [
                {
                    "trade_id": "999",
                    "product_id": "ETH-USDC",
                    "price": "3000.50",
                    "size": "0.5",
                    "side": "SELL",
                    "time": "2026-05-01T12:00:01Z"
                }
            ]
        });
        let parsed = parse_market_trades_event(&payload, &Symbol::new("ETHUSDC"), Timestamp::now());
        let tick = parsed.into_iter().next().unwrap().expect("parse ok");
        assert_eq!(tick.side, Side::Sell);
        assert_eq!(tick.trade_id, 999);
    }

    #[test]
    fn t1403_parses_candles_event_to_bar() {
        let payload = serde_json::json!({
            "candles": [
                {
                    "start": "1714579200",
                    "open": "60000.00",
                    "high": "60100.00",
                    "low": "59900.00",
                    "close": "60050.00",
                    "volume": "1.5",
                    "product_id": "BTC-USDC"
                }
            ]
        });
        let symbol = Symbol::new("BTCUSDC");
        let parsed = parse_candles_event(&payload, &symbol, Timeframe::OneMinute, Timestamp::now());
        assert_eq!(parsed.len(), 1);
        let bar = parsed.into_iter().next().unwrap().expect("parse ok");
        assert_eq!(bar.venue, Venue::Coinbase);
        assert_eq!(bar.tf, Timeframe::OneMinute);
        assert_eq!(bar.symbol, symbol);
        assert_eq!(bar.open.get(), Decimal::from_str("60000.00").unwrap());
        assert_eq!(bar.close.get(), Decimal::from_str("60050.00").unwrap());
    }

    #[test]
    fn t1403_no_f64_in_money_paths() {
        // Sentinel: this whole module avoids `f64` for prices/qtys. The
        // actual gate is `grep -n "f64\|as_f64" crates/data/src/coinbase.rs`
        // — every occurrence must live in comments only.  This test just
        // anchors the intent in the test name; the gate is verified at
        // acceptance via grep.
        let _: Decimal = Decimal::from_str("60000.12345678").expect("decimal parse");
    }
}
