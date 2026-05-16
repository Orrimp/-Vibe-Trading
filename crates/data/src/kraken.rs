//! Kraken WebSocket v2 feed (T1404).
//!
//! WS endpoint: `wss://ws.kraken.com/v2` (Q2 / Q8).
//!
//! Channels:
//!   - `trade` — raw individual trades
//!   - `ohlc`  — venue-closed OHLCV bars (interval in minutes; 1 = 1m)
//!
//! Auto-reconnects with exponential back-off (1 s base, cap 60 s).
//! Answers server pings with pongs per the WS protocol.
//!
//! See `spec/features/v1-5b-multi-venue.md` Q2, Q8, R2, R15.
//!
//! R15.4 hard rule: Kraken's WS v2 emits `price` / `qty` as JSON numbers.
//! The parser MUST cast each to its raw string representation via
//! `serde_json::Value::to_string()` then `Decimal::from_str` — never
//! through `f64`.
use std::str::FromStr;

use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::Value;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, warn};
use trading_core::{
    Bar, FeedError, Price, Quantity, Side, Symbol, Tick, Timeframe, Timestamp, Venue,
};

use crate::source::{MarketDataSource, SymbolInfo};

// ── Kraken REST AssetPairs response (minimal) ────────────────────────────────

#[derive(Debug, Deserialize)]
struct AssetPairsEnvelope {
    #[serde(default)]
    error: Vec<String>,
    result: Option<std::collections::BTreeMap<String, AssetPair>>,
}

#[derive(Debug, Deserialize)]
struct AssetPair {
    /// Web display name, e.g. `"XBT/USDC"`.
    #[serde(default)]
    wsname: Option<String>,
    base: String,
    quote: String,
    #[serde(default)]
    pair_decimals: Option<u32>,
    #[serde(default)]
    lot_decimals: Option<u32>,
    #[serde(default)]
    ordermin: Option<String>,
    #[serde(default)]
    costmin: Option<String>,
}

// ── Kraken WS v2 message shapes ──────────────────────────────────────────────

/// WS v2 envelope for any market-data event.
#[derive(Debug, Deserialize)]
struct WsV2Envelope {
    channel: String,
    #[serde(rename = "type")]
    msg_type: Option<String>,
    #[serde(default)]
    data: Vec<Value>,
}

#[derive(Debug, Deserialize)]
struct WsTrade {
    /// Captured for round-trip / debug; the per-stream caller pre-filters
    /// by symbol so we don't read it post-parse.
    #[allow(dead_code)]
    symbol: String,
    side: String,
    price: serde_json::Number,
    qty: serde_json::Number,
    trade_id: u64,
    timestamp: String,
}

#[derive(Debug, Deserialize)]
struct WsOhlc {
    #[allow(dead_code)]
    symbol: String,
    open: serde_json::Number,
    high: serde_json::Number,
    low: serde_json::Number,
    close: serde_json::Number,
    volume: serde_json::Number,
    /// Bucket start (RFC-3339).
    interval_begin: String,
    /// Interval in minutes — 1 / 5 / 15 / 60 / …
    interval: u32,
    #[serde(default)]
    trades: Option<u32>,
}

// ── KrakenFeed ───────────────────────────────────────────────────────────────

/// Kraken WS v2 adapter.
pub struct KrakenFeed {
    pub ws_url: String,
    pub rest_url: String,
    /// Optional audit ledger handle (T805 / Q11).
    pub ledger: Option<std::sync::Arc<audit::Ledger>>,
}

impl KrakenFeed {
    /// Create a new `KrakenFeed` with explicit URLs (tests).
    #[must_use]
    pub fn with_urls(ws_url: impl Into<String>, rest_url: impl Into<String>) -> Self {
        Self {
            ws_url: ws_url.into(),
            rest_url: rest_url.into(),
            ledger: None,
        }
    }

    /// Default Kraken production URLs.
    #[must_use]
    pub fn production() -> Self {
        Self::with_urls("wss://ws.kraken.com/v2", "https://api.kraken.com")
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

/// Map an agent-native slash-free symbol (`BTCUSDC`) to Kraken WS v2 form
/// (`XBT/USDC`).
///
/// Kraken uses the legacy ISO-4217 'X' prefix: `XBT` for Bitcoin. This
/// helper normalizes the base asset accordingly. Other assets (ETH, SOL,
/// …) keep their three-letter code.
#[must_use]
pub fn kraken_symbol_map(s: &Symbol) -> String {
    let raw = s.0.as_str().to_uppercase();
    let (base, quote) = split_base_quote(&raw);
    let base_kraken = match base.as_str() {
        "BTC" => "XBT".to_string(),
        other => other.to_string(),
    };
    format!("{base_kraken}/{quote}")
}

fn split_base_quote(raw: &str) -> (String, String) {
    for q in ["USDC", "USDT", "USD"] {
        if let Some(base) = raw.strip_suffix(q)
            && !base.is_empty()
        {
            return (base.to_string(), q.to_string());
        }
    }
    if raw.len() > 3 {
        let split = raw.len() - 3;
        let (b, q) = raw.split_at(split);
        (b.to_string(), q.to_string())
    } else {
        (raw.to_string(), String::new())
    }
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn parse_decimal(s: &str, field: &str) -> Result<Decimal, FeedError> {
    Decimal::from_str(s).map_err(|_| FeedError::Parse(format!("bad {field}: {s}")))
}

/// Convert a `serde_json::Number` (which may be a JSON number) to a `Decimal`
/// **without** going through `f64`. Per R15.4, money math never sees `f64`.
///
/// Strategy: take the canonical JSON string representation of the number
/// (`Number::to_string()` returns the raw textual form preserved by serde)
/// and parse that string with `Decimal::from_str`.
fn json_number_to_decimal(n: &serde_json::Number, field: &str) -> Result<Decimal, FeedError> {
    parse_decimal(&n.to_string(), field)
}

fn json_number_to_price(n: &serde_json::Number, field: &str) -> Result<Price, FeedError> {
    let d = json_number_to_decimal(n, field)?;
    Price::new(d).map_err(|e| FeedError::Parse(e.to_string()))
}

fn json_number_to_qty(n: &serde_json::Number, field: &str) -> Result<Quantity, FeedError> {
    let d = json_number_to_decimal(n, field)?;
    Quantity::new(d).map_err(|e| FeedError::Parse(e.to_string()))
}

fn parse_rfc3339(s: &str) -> Result<Timestamp, FeedError> {
    OffsetDateTime::parse(s, &Rfc3339)
        .map(Timestamp::new)
        .map_err(|e| FeedError::Parse(format!("bad timestamp {s}: {e}")))
}

/// Map an agent-native `Timeframe` to Kraken's `interval` minutes.
fn tf_to_kraken_minutes(tf: Timeframe) -> u32 {
    match tf {
        // Kraken doesn't expose 1s ohlc; closest is 1m. The agent's 1s
        // path is client-side aggregation (T1406) — this branch should
        // never be reached for `OneSecond` in live config.
        Timeframe::OneSecond | Timeframe::OneMinute => 1,
        Timeframe::FiveMinutes => 5,
        Timeframe::FifteenMinutes => 15,
        Timeframe::OneHour => 60,
        Timeframe::FourHours => 240,
        Timeframe::OneDay => 1440,
    }
}

/// Build a Kraken WS v2 `subscribe` message for a channel + symbol list.
///
/// Public channels (`trade`, `ohlc`) are unauthenticated (Q8) — no token.
fn build_subscribe_message(
    channel: &str,
    symbols: &[String],
    interval_minutes: Option<u32>,
) -> String {
    let mut params = serde_json::json!({
        "channel": channel,
        "symbol": symbols,
    });
    if let Some(m) = interval_minutes {
        params["interval"] = serde_json::json!(m);
    }
    let body = serde_json::json!({
        "method": "subscribe",
        "params": params,
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

// ── Parsers (boundary tested directly per Q10) ───────────────────────────────

/// Parse a single Kraken WS v2 `trade` channel data element to a `Tick`.
fn parse_trade_event(
    data_elem: &Value,
    symbol: &Symbol,
    local_recv_ts: Timestamp,
) -> Result<Tick, FeedError> {
    let t: WsTrade = serde_json::from_value(data_elem.clone())
        .map_err(|e| FeedError::Parse(format!("trade: {e}")))?;
    let venue_ts = parse_rfc3339(&t.timestamp)?;
    let side = match t.side.to_ascii_lowercase().as_str() {
        "buy" => Side::Buy,
        "sell" => Side::Sell,
        other => return Err(FeedError::Parse(format!("unknown side: {other}"))),
    };
    Ok(Tick {
        symbol: symbol.clone(),
        venue_ts,
        local_recv_ts,
        price: json_number_to_price(&t.price, "price")?,
        qty: json_number_to_qty(&t.qty, "qty")?,
        side,
        trade_id: t.trade_id,
        venue: Venue::Kraken,
    })
}

fn parse_ohlc_event(
    data_elem: &Value,
    symbol: &Symbol,
    tf: Timeframe,
    local_recv_ts: Timestamp,
) -> Result<Bar, FeedError> {
    let o: WsOhlc = serde_json::from_value(data_elem.clone())
        .map_err(|e| FeedError::Parse(format!("ohlc: {e}")))?;
    let open_ts = parse_rfc3339(&o.interval_begin)?;
    let interval_secs = i64::from(o.interval) * 60;
    let close_ts_dt =
        open_ts.inner() + time::Duration::seconds(interval_secs) - time::Duration::microseconds(1);
    let close_ts = Timestamp::new(close_ts_dt);
    Ok(Bar {
        symbol: symbol.clone(),
        tf,
        open_ts,
        close_ts,
        open: json_number_to_price(&o.open, "open")?,
        high: json_number_to_price(&o.high, "high")?,
        low: json_number_to_price(&o.low, "low")?,
        close: json_number_to_price(&o.close, "close")?,
        volume: json_number_to_qty(&o.volume, "volume")?,
        trade_count: o.trades.unwrap_or(0),
        local_recv_ts,
        venue: Venue::Kraken,
    })
}

// ── MarketDataSource impl ────────────────────────────────────────────────────

#[async_trait]
impl MarketDataSource for KrakenFeed {
    async fn exchange_info(&self, symbol: Symbol) -> Result<SymbolInfo, FeedError> {
        let kraken_pair = kraken_symbol_map(&symbol);
        // Kraken accepts the slash form via the `pair=` query param.
        let url = format!("{}/0/public/AssetPairs?pair={}", self.rest_url, kraken_pair);
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| FeedError::Connection(e.to_string()))?;
        let env: AssetPairsEnvelope = resp
            .json()
            .await
            .map_err(|e| FeedError::Parse(e.to_string()))?;
        if !env.error.is_empty() {
            return Err(FeedError::Parse(env.error.join(", ")));
        }
        let result = env.result.ok_or(FeedError::StreamClosed)?;
        // Kraken returns one or more entries; pick the one whose wsname
        // matches our slash form (or the first if none match).
        let pair = result
            .values()
            .find(|p| p.wsname.as_deref() == Some(kraken_pair.as_str()))
            .or_else(|| result.values().next())
            .ok_or(FeedError::StreamClosed)?;

        // Un-map base back to agent-native (XBT → BTC) for SymbolInfo.
        let base_asset = match pair.base.as_str() {
            "XBT" | "XXBT" => "BTC".to_string(),
            other => other
                .trim_start_matches('X')
                .trim_start_matches('Z')
                .to_string(),
        };
        let quote_asset = pair
            .quote
            .trim_start_matches('Z')
            .trim_start_matches('X')
            .to_string();

        let lot_size = pair
            .lot_decimals
            .map(|d| Decimal::new(1, d))
            .unwrap_or_else(|| Decimal::new(1, 5));
        let min_qty = pair
            .ordermin
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or(lot_size);
        let min_notional = pair
            .costmin
            .as_deref()
            .and_then(|s| Decimal::from_str(s).ok())
            .unwrap_or_else(|| Decimal::new(10, 0));
        // Captured but not surfaced in SymbolInfo today.
        let _ = pair.pair_decimals;

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
        let kraken_pair = kraken_symbol_map(&symbol);
        let interval = tf_to_kraken_minutes(tf);
        let ws_url = self.ws_url.clone();
        let symbol_clone = symbol.clone();
        let symbol_for_audit = symbol.clone();
        let ledger_for_stream = self.ledger.clone();

        let _ws = connect_ws(&ws_url).await?;

        let stream = async_stream::stream! {
            let mut backoff_secs: u64 = 1;
            let mut is_reconnect = false;
            loop {
                debug!(symbol = %kraken_pair, "connecting to kraken ohlc WS");
                match connect_ws(&ws_url).await {
                    Err(e) => {
                        error!(error = %e, "kraken ohlc WS connect failed, retrying in {backoff_secs}s");
                        sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(60);
                        continue;
                    }
                    Ok(mut ws) => {
                        backoff_secs = 1;
                        let sub = build_subscribe_message("ohlc", std::slice::from_ref(&kraken_pair), Some(interval));
                        if let Err(e) = futures::SinkExt::send(&mut ws, Message::Text(sub.into())).await {
                            warn!(error = %e, "kraken ohlc subscribe send failed");
                            continue;
                        }
                        if is_reconnect
                            && let Some(ledger) = ledger_for_stream.as_ref()
                            && let Err(e) = audit::journal::feed_reconnect(
                                ledger,
                                symbol_for_audit.0.as_str(),
                                Venue::Kraken,
                                None,
                            ).await
                        {
                            warn!(error = %e, "kraken feed_reconnect audit write failed (non-fatal)");
                        }
                        is_reconnect = true;
                        loop {
                            match ws.next().await {
                                None => { warn!("kraken ohlc WS closed, reconnecting"); break; }
                                Some(Err(e)) => { warn!(error = %e, "kraken ohlc WS error, reconnecting"); break; }
                                Some(Ok(Message::Ping(data))) => {
                                    if let Err(e) = futures::SinkExt::send(&mut ws, Message::Pong(data)).await {
                                        warn!(error = %e, "kraken pong send failed");
                                        break;
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    let local_ts = Timestamp::now();
                                    let env: WsV2Envelope = match serde_json::from_str(&text) {
                                        Ok(e) => e,
                                        Err(e) => {
                                            warn!(error = %e, text = %text, "kraken envelope parse error");
                                            continue;
                                        }
                                    };
                                    if env.channel != "ohlc" { continue; }
                                    // Skip snapshot replay; only emit `update`s for
                                    // closed bars. Kraken WS v2 sends a snapshot then
                                    // updates; both have channel=="ohlc".
                                    let is_update = env.msg_type.as_deref() == Some("update");
                                    if !is_update { continue; }
                                    for elem in &env.data {
                                        // Filter to our symbol — multiple symbols may share the channel.
                                        if elem.get("symbol").and_then(|v| v.as_str()) != Some(kraken_pair.as_str()) {
                                            continue;
                                        }
                                        yield parse_ohlc_event(elem, &symbol_clone, tf, local_ts);
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
        let kraken_pair = kraken_symbol_map(&symbol);
        let ws_url = self.ws_url.clone();
        let symbol_clone = symbol.clone();
        let symbol_for_audit = symbol.clone();
        let ledger_for_stream = self.ledger.clone();

        let _ws = connect_ws(&ws_url).await?;

        let stream = async_stream::stream! {
            let mut backoff_secs: u64 = 1;
            let mut is_reconnect = false;
            loop {
                debug!(symbol = %kraken_pair, "connecting to kraken trade WS");
                match connect_ws(&ws_url).await {
                    Err(e) => {
                        error!(error = %e, "kraken trade WS connect failed, retrying in {backoff_secs}s");
                        sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(60);
                        continue;
                    }
                    Ok(mut ws) => {
                        backoff_secs = 1;
                        let sub = build_subscribe_message("trade", std::slice::from_ref(&kraken_pair), None);
                        if let Err(e) = futures::SinkExt::send(&mut ws, Message::Text(sub.into())).await {
                            warn!(error = %e, "kraken trade subscribe send failed");
                            continue;
                        }
                        if is_reconnect
                            && let Some(ledger) = ledger_for_stream.as_ref()
                            && let Err(e) = audit::journal::feed_reconnect(
                                ledger,
                                symbol_for_audit.0.as_str(),
                                Venue::Kraken,
                                None,
                            ).await
                        {
                            warn!(error = %e, "kraken feed_reconnect audit write failed (non-fatal)");
                        }
                        is_reconnect = true;
                        loop {
                            match ws.next().await {
                                None => { warn!("kraken trade WS closed, reconnecting"); break; }
                                Some(Err(e)) => { warn!(error = %e, "kraken trade WS error, reconnecting"); break; }
                                Some(Ok(Message::Ping(data))) => {
                                    if let Err(e) = futures::SinkExt::send(&mut ws, Message::Pong(data)).await {
                                        warn!(error = %e, "kraken pong send failed");
                                        break;
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    let local_ts = Timestamp::now();
                                    let env: WsV2Envelope = match serde_json::from_str(&text) {
                                        Ok(e) => e,
                                        Err(e) => {
                                            warn!(error = %e, text = %text, "kraken envelope parse error");
                                            continue;
                                        }
                                    };
                                    if env.channel != "trade" { continue; }
                                    // Skip the initial snapshot; only emit live updates.
                                    let is_update = env.msg_type.as_deref() == Some("update");
                                    if !is_update { continue; }
                                    for elem in &env.data {
                                        if elem.get("symbol").and_then(|v| v.as_str()) != Some(kraken_pair.as_str()) {
                                            continue;
                                        }
                                        yield parse_trade_event(elem, &symbol_clone, local_ts);
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
    fn kraken_symbol_map_btc_uses_xbt() {
        assert_eq!(kraken_symbol_map(&Symbol::new("BTCUSDC")), "XBT/USDC");
        assert_eq!(kraken_symbol_map(&Symbol::new("BTCUSD")), "XBT/USD");
    }

    #[test]
    fn kraken_symbol_map_eth_keeps_eth() {
        assert_eq!(kraken_symbol_map(&Symbol::new("ETHUSDC")), "ETH/USDC");
    }

    #[test]
    fn kraken_symbol_map_xrp_usdc() {
        assert_eq!(kraken_symbol_map(&Symbol::new("XRPUSDC")), "XRP/USDC");
    }

    /// T1404 — verify subscription message matches the WS v2 protocol shape.
    #[test]
    fn t1404_kraken_subscription_message_shape() {
        let msg = build_subscribe_message("trade", &["XBT/USDC".to_string()], None);
        let v: serde_json::Value = serde_json::from_str(&msg).expect("valid json");
        assert_eq!(v["method"], "subscribe");
        assert_eq!(v["params"]["channel"], "trade");
        let symbols = v["params"]["symbol"].as_array().expect("symbol array");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0], "XBT/USDC");
    }

    #[test]
    fn t1404_kraken_ohlc_subscribe_includes_interval() {
        let msg = build_subscribe_message("ohlc", &["XBT/USDC".to_string()], Some(1));
        let v: serde_json::Value = serde_json::from_str(&msg).expect("valid json");
        assert_eq!(v["method"], "subscribe");
        assert_eq!(v["params"]["channel"], "ohlc");
        assert_eq!(v["params"]["interval"], 1);
    }

    #[test]
    fn t1404_parses_trade_event_to_tick() {
        // Per Design § sample: "data" element shape:
        //   {"symbol":"BTC/USD","side":"buy","price":60000.0,"qty":0.001,
        //    "trade_id":<u64>,"timestamp":"<RFC3339>"}
        let payload = serde_json::json!({
            "symbol": "XBT/USDC",
            "side": "buy",
            "price": 60000.0,
            "qty": 0.001,
            "trade_id": 12345_u64,
            "timestamp": "2026-05-01T12:00:00.123456Z"
        });
        let symbol = Symbol::new("BTCUSDC");
        let tick = parse_trade_event(&payload, &symbol, Timestamp::now()).expect("parse ok");
        assert_eq!(tick.venue, Venue::Kraken);
        assert_eq!(tick.symbol, symbol);
        assert_eq!(tick.side, Side::Buy);
        assert_eq!(tick.trade_id, 12345);
        // Critical: NO f64 — Decimal must be exact "60000".
        // serde_json renders 60000.0 as "60000" (no trailing zeros).
        assert_eq!(tick.price.get(), Decimal::from_str("60000").unwrap());
        assert_eq!(tick.qty.get(), Decimal::from_str("0.001").unwrap());
    }

    #[test]
    fn t1404_parses_trade_event_sell_side() {
        let payload = serde_json::json!({
            "symbol": "ETH/USDC",
            "side": "sell",
            "price": 3000.5,
            "qty": 0.5,
            "trade_id": 999_u64,
            "timestamp": "2026-05-01T12:00:01Z"
        });
        let tick = parse_trade_event(&payload, &Symbol::new("ETHUSDC"), Timestamp::now())
            .expect("parse ok");
        assert_eq!(tick.side, Side::Sell);
        assert_eq!(tick.trade_id, 999);
    }

    #[test]
    fn t1404_parses_ohlc_event_to_bar() {
        let payload = serde_json::json!({
            "symbol": "XBT/USDC",
            "open": 60000.0,
            "high": 60100.0,
            "low": 59900.0,
            "close": 60050.0,
            "volume": 1.5,
            "interval_begin": "2026-05-01T12:00:00Z",
            "interval": 1,
            "trades": 7
        });
        let bar = parse_ohlc_event(
            &payload,
            &Symbol::new("BTCUSDC"),
            Timeframe::OneMinute,
            Timestamp::now(),
        )
        .expect("parse ok");
        assert_eq!(bar.venue, Venue::Kraken);
        assert_eq!(bar.tf, Timeframe::OneMinute);
        assert_eq!(bar.trade_count, 7);
        assert_eq!(bar.open.get(), Decimal::from_str("60000").unwrap());
        assert_eq!(bar.close.get(), Decimal::from_str("60050").unwrap());
    }

    /// R15.4: prove `Decimal::from_str` is used on the canonical JSON
    /// string representation — never `f64::from_str` or `as f64`.
    /// This ensures Kraken's JSON-number price/qty cross the boundary
    /// through the safe path.
    #[test]
    fn t1404_json_number_path_is_decimal_safe() {
        // Typical venue precision (8 decimals).  We assert that the value
        // round-trips through serde_json::Number → string → Decimal exactly.
        let n: serde_json::Number = serde_json::from_str("60000.12345678").expect("parse number");
        let d = json_number_to_decimal(&n, "test").expect("decimal ok");
        assert_eq!(d.to_string(), "60000.12345678");
        // And confirm the conversion path does not silently floor / truncate
        // small fractional components like 0.001:
        let n2: serde_json::Number = serde_json::from_str("0.001").expect("parse number");
        let d2 = json_number_to_decimal(&n2, "test").expect("decimal ok");
        assert_eq!(d2, Decimal::from_str("0.001").unwrap());
    }
}
