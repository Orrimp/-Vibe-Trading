//! Binance spot WebSocket feed (T08).
//!
//! Streams:
//!   - `<symbol>@kline_<tf>` — venue-closed OHLCV bars
//!   - `<symbol>@trade`      — raw individual trades
//!
//! Auto-reconnects with exponential back-off (1 s base, cap 60 s).
//! Answers server pings with pongs per the Binance WS protocol.

use async_trait::async_trait;
use futures::stream::BoxStream;
use futures::StreamExt;
use rust_decimal::Decimal;
use serde::Deserialize;
use time::OffsetDateTime;
use tokio::time::{sleep, Duration};
use tokio_tungstenite::tungstenite::Message;
use tracing::{debug, error, warn};
use trading_core::{
    Bar, FeedError, Price, Quantity, Side, Symbol, Tick, Timeframe, Timestamp, Venue,
};

use crate::source::{MarketDataSource, SymbolInfo};

// ── Binance REST exchange-info response (minimal) ────────────────────────────

#[derive(Debug, Deserialize)]
struct BinanceExchangeInfo {
    symbols: Vec<BinanceSymbolInfo>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BinanceSymbolInfo {
    symbol: String,
    base_asset: String,
    quote_asset: String,
    filters: Vec<serde_json::Value>,
}

// ── Binance WS message shapes ─────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct KlineEvent {
    // event_time and symbol are present in the WS message but not used
    // in parsing — the symbol comes from the stream subscription context.
    #[serde(rename = "k")]
    kline: KlineData,
}

#[derive(Debug, Deserialize)]
struct KlineData {
    #[serde(rename = "t")]
    open_time: i64,
    #[serde(rename = "T")]
    close_time: i64,
    #[serde(rename = "o")]
    open: String,
    #[serde(rename = "h")]
    high: String,
    #[serde(rename = "l")]
    low: String,
    #[serde(rename = "c")]
    close: String,
    #[serde(rename = "v")]
    volume: String,
    #[serde(rename = "n")]
    trade_count: u32,
    #[serde(rename = "x")]
    is_closed: bool,
}

#[derive(Debug, Deserialize)]
struct TradeEvent {
    #[serde(rename = "T")]
    trade_time: i64,
    // symbol is in the WS message but we use the subscription symbol
    #[serde(rename = "t")]
    trade_id: u64,
    #[serde(rename = "p")]
    price: String,
    #[serde(rename = "q")]
    qty: String,
    #[serde(rename = "m")]
    is_buyer_maker: bool,
}

// ── BinanceFeed ───────────────────────────────────────────────────────────────

/// Binance spot WebSocket adapter.
pub struct BinanceFeed {
    pub ws_url: String,
    pub rest_url: String,
    /// Optional audit ledger handle (T805 — operator success reports R7.1).
    ///
    /// When `Some`, every WS reconnection writes a `FeedReconnect` strategy
    /// event to the ledger so the report's R7 system-health row can count
    /// reconnects per window.  When `None`, no audit write happens — kept
    /// `Option` so existing test/research callers (which build their own
    /// `BinanceFeed` without a Ledger) compile unchanged.
    pub ledger: Option<std::sync::Arc<audit::Ledger>>,
}

impl BinanceFeed {
    /// Create a new `BinanceFeed`.
    ///
    /// - `ws_url`   — WebSocket base URL (e.g. `wss://stream.binance.com:9443/ws`)
    /// - `rest_url` — REST base URL (e.g. `https://api.binance.com`)
    #[must_use]
    pub fn new(ws_url: impl Into<String>, rest_url: impl Into<String>) -> Self {
        Self {
            ws_url: ws_url.into(),
            rest_url: rest_url.into(),
            ledger: None,
        }
    }

    /// Builder-style helper that attaches an audit ledger so WS reconnects
    /// emit `FeedReconnect` strategy events (T805 — R7.1).
    #[must_use]
    pub fn with_ledger(mut self, ledger: std::sync::Arc<audit::Ledger>) -> Self {
        self.ledger = Some(ledger);
        self
    }

    /// Default Binance production URLs.
    #[must_use]
    pub fn production() -> Self {
        Self::new(
            "wss://stream.binance.com:9443/ws",
            "https://api.binance.com",
        )
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn millis_to_timestamp(ms: i64) -> Timestamp {
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .map(Timestamp::new)
        .unwrap_or_else(|_| Timestamp::now())
}

fn parse_decimal(s: &str, field: &str) -> Result<Decimal, FeedError> {
    s.parse()
        .map_err(|_| FeedError::Parse(format!("bad {field}: {s}")))
}

fn parse_price(s: &str, field: &str) -> Result<Price, FeedError> {
    let d = parse_decimal(s, field)?;
    Price::new(d).map_err(|e| FeedError::Parse(e.to_string()))
}

fn parse_qty(s: &str, field: &str) -> Result<Quantity, FeedError> {
    let d = parse_decimal(s, field)?;
    Quantity::new(d).map_err(|e| FeedError::Parse(e.to_string()))
}

fn tf_to_binance_str(tf: Timeframe) -> &'static str {
    match tf {
        // Binance does expose `kline_1s` but v1.5b aggregates 1s bars
        // client-side from the Tick stream (Q5) — never via this server-side
        // kline subscription path. Emit "1s" defensively for completeness.
        Timeframe::OneSecond => "1s",
        Timeframe::OneMinute => "1m",
        Timeframe::FiveMinutes => "5m",
        Timeframe::FifteenMinutes => "15m",
        Timeframe::OneHour => "1h",
        Timeframe::FourHours => "4h",
        Timeframe::OneDay => "1d",
    }
}

/// Connect with exponential back-off, return the first successful stream.
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

// ── MarketDataSource impl ─────────────────────────────────────────────────────

#[async_trait]
impl MarketDataSource for BinanceFeed {
    /// Fetch symbol metadata from the REST `GET /api/v3/exchangeInfo` endpoint.
    ///
    /// # Errors
    ///
    /// Returns [`FeedError::Connection`] on HTTP failure, [`FeedError::Parse`]
    /// on JSON parse failure, or [`FeedError::StreamClosed`] if the symbol
    /// is not found.
    async fn exchange_info(&self, symbol: Symbol) -> Result<SymbolInfo, FeedError> {
        let url = format!(
            "{}/api/v3/exchangeInfo?symbol={}",
            self.rest_url,
            symbol.0.as_str().to_uppercase()
        );
        let resp = reqwest::get(&url)
            .await
            .map_err(|e| FeedError::Connection(e.to_string()))?;
        let info: BinanceExchangeInfo = resp
            .json()
            .await
            .map_err(|e| FeedError::Parse(e.to_string()))?;

        let sym_info = info
            .symbols
            .into_iter()
            .find(|s| s.symbol.eq_ignore_ascii_case(symbol.0.as_str()))
            .ok_or(FeedError::StreamClosed)?;

        // Extract min_qty, lot_size, min_notional from filters
        let mut min_qty = Decimal::new(1, 5);
        let mut lot_size = Decimal::new(1, 5);
        let mut min_notional = Decimal::new(10, 0);

        for filter in &sym_info.filters {
            let kind = filter.get("filterType").and_then(|v| v.as_str());
            match kind {
                Some("LOT_SIZE") => {
                    if let Some(v) = filter.get("minQty").and_then(|v| v.as_str()) {
                        min_qty = v.parse().unwrap_or(min_qty);
                        lot_size = min_qty;
                    }
                }
                Some("NOTIONAL") | Some("MIN_NOTIONAL") => {
                    if let Some(v) = filter.get("minNotional").and_then(|v| v.as_str()) {
                        min_notional = v.parse().unwrap_or(min_notional);
                    }
                }
                _ => {}
            }
        }

        Ok(SymbolInfo {
            symbol: Symbol::new(sym_info.symbol),
            base_asset: sym_info.base_asset,
            quote_asset: sym_info.quote_asset,
            min_qty,
            lot_size,
            min_notional,
        })
    }

    /// Subscribe to closed kline bars with exponential-backoff reconnect.
    ///
    /// Only emits bars where `k.x == true` (bar is closed on the venue).
    ///
    /// # Errors
    ///
    /// Returns [`FeedError::Connection`] if the initial connection fails.
    async fn subscribe_bars(
        &self,
        symbol: Symbol,
        tf: Timeframe,
    ) -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError> {
        let stream_name = format!(
            "{}@kline_{}",
            symbol.0.as_str().to_lowercase(),
            tf_to_binance_str(tf)
        );
        let ws_url = format!("{}/{}", self.ws_url, stream_name);
        let symbol_clone = symbol.clone();
        // Capture optional ledger for T805 — write a `FeedReconnect`
        // event each time the WS re-establishes (after the initial connect).
        let ledger_for_stream = self.ledger.clone();
        let symbol_for_audit = symbol.clone();

        // Verify initial connection before returning the stream.
        let _ws = connect_ws(&ws_url).await?;

        let stream = async_stream::stream! {
            let mut backoff_secs: u64 = 1;
            // Track whether this is the first iteration of the outer loop
            // — the very first `Ok(mut ws)` is the initial connect, not a
            // reconnect, so we suppress the audit write on it.
            let mut is_reconnect = false;
            loop {
                debug!(stream = %stream_name, "connecting to kline WS");
                match connect_ws(&ws_url).await {
                    Err(e) => {
                        error!(error = %e, "kline WS connect failed, retrying in {backoff_secs}s");
                        sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(60);
                        continue;
                    }
                    Ok(mut ws) => {
                        backoff_secs = 1;
                        // T805 — emit a `FeedReconnect` strategy event on
                        // re-establishment (skip the first connect).  Failure
                        // to write is warn-logged, never breaks the stream.
                        if is_reconnect
                            && let Some(ledger) = ledger_for_stream.as_ref()
                            && let Err(e) = audit::journal::feed_reconnect(
                                ledger,
                                symbol_for_audit.0.as_str(),
                                Venue::Binance,
                                None,
                            ).await
                        {
                            warn!(error = %e, "feed_reconnect audit write failed (non-fatal)");
                        }
                        is_reconnect = true;
                        loop {
                            match ws.next().await {
                                None => {
                                    warn!("kline WS stream closed, reconnecting");
                                    break;
                                }
                                Some(Err(e)) => {
                                    warn!(error = %e, "kline WS error, reconnecting");
                                    break;
                                }
                                Some(Ok(Message::Ping(data))) => {
                                    if let Err(e) = futures::SinkExt::send(
                                        &mut ws,
                                        Message::Pong(data)
                                    ).await {
                                        warn!(error = %e, "pong send failed");
                                        break;
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    let local_ts = Timestamp::now();
                                    match serde_json::from_str::<KlineEvent>(&text) {
                                        Err(e) => {
                                            warn!(error = %e, text = %text, "kline parse error");
                                        }
                                        Ok(evt) => {
                                            if !evt.kline.is_closed {
                                                continue;
                                            }
                                            let k = &evt.kline;
                                            let result = (|| -> Result<Bar, FeedError> {
                                                Ok(Bar {
                                                    symbol: symbol_clone.clone(),
                                                    tf,
                                                    open_ts: millis_to_timestamp(k.open_time),
                                                    close_ts: millis_to_timestamp(k.close_time),
                                                    open: parse_price(&k.open, "open")?,
                                                    high: parse_price(&k.high, "high")?,
                                                    low: parse_price(&k.low, "low")?,
                                                    close: parse_price(&k.close, "close")?,
                                                    volume: parse_qty(&k.volume, "volume")?,
                                                    trade_count: k.trade_count,
                                                    local_recv_ts: local_ts,
                                                    venue: Venue::Binance,
                                                })
                                            })();
                                            yield result;
                                        }
                                    }
                                }
                                Some(Ok(_)) => { /* binary / close frames — ignore */ }
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

    /// Subscribe to individual trades with exponential-backoff reconnect.
    ///
    /// # Errors
    ///
    /// Returns [`FeedError::Connection`] if the initial connection fails.
    async fn subscribe_trades(
        &self,
        symbol: Symbol,
    ) -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError> {
        let stream_name = format!("{}@trade", symbol.0.as_str().to_lowercase());
        let ws_url = format!("{}/{}", self.ws_url, stream_name);
        let symbol_clone = symbol.clone();
        // Capture optional ledger for T805 — emit a `FeedReconnect` event
        // each time the trade WS re-establishes (after the initial connect).
        let ledger_for_stream = self.ledger.clone();
        let symbol_for_audit = symbol.clone();

        // Verify initial connection before returning the stream.
        let _ws = connect_ws(&ws_url).await?;

        let stream = async_stream::stream! {
            let mut backoff_secs: u64 = 1;
            // Same is_reconnect flag as in subscribe_bars — first connect
            // is not a reconnect.
            let mut is_reconnect = false;
            loop {
                debug!(stream = %stream_name, "connecting to trade WS");
                match connect_ws(&ws_url).await {
                    Err(e) => {
                        error!(error = %e, "trade WS connect failed, retrying in {backoff_secs}s");
                        sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(60);
                        continue;
                    }
                    Ok(mut ws) => {
                        backoff_secs = 1;
                        if is_reconnect
                            && let Some(ledger) = ledger_for_stream.as_ref()
                            && let Err(e) = audit::journal::feed_reconnect(
                                ledger,
                                symbol_for_audit.0.as_str(),
                                Venue::Binance,
                                None,
                            ).await
                        {
                            warn!(error = %e, "feed_reconnect audit write failed (non-fatal)");
                        }
                        is_reconnect = true;
                        loop {
                            match ws.next().await {
                                None => {
                                    warn!("trade WS stream closed, reconnecting");
                                    break;
                                }
                                Some(Err(e)) => {
                                    warn!(error = %e, "trade WS error, reconnecting");
                                    break;
                                }
                                Some(Ok(Message::Ping(data))) => {
                                    if let Err(e) = futures::SinkExt::send(
                                        &mut ws,
                                        Message::Pong(data)
                                    ).await {
                                        warn!(error = %e, "pong send failed");
                                        break;
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    let local_ts = Timestamp::now();
                                    match serde_json::from_str::<TradeEvent>(&text) {
                                        Err(e) => {
                                            warn!(error = %e, text = %text, "trade parse error");
                                        }
                                        Ok(evt) => {
                                            let result = (|| -> Result<Tick, FeedError> {
                                                Ok(Tick {
                                                    symbol: symbol_clone.clone(),
                                                    venue_ts: millis_to_timestamp(evt.trade_time),
                                                    local_recv_ts: local_ts,
                                                    price: parse_price(&evt.price, "price")?,
                                                    qty: parse_qty(&evt.qty, "qty")?,
                                                    side: if evt.is_buyer_maker {
                                                        Side::Sell
                                                    } else {
                                                        Side::Buy
                                                    },
                                                    trade_id: evt.trade_id,
                                                    venue: Venue::Binance,
                                                })
                                            })();
                                            yield result;
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

// ── T1405 — multi-symbol combined-stream impl ────────────────────────────────
//
// Binance combined-stream URL form:
//   `wss://stream.binance.com:9443/stream?streams=<a>/<b>/...`
// The combined-stream wrapper is:
//   `{"stream":"btcusdt@kline_1m","data":{...}}`
//
// Single-symbol API (`subscribe_bars` / `subscribe_trades`) is unchanged
// (R10.3); these `_multi` methods are additive and use the combined-stream
// host path even when `self.ws_url` was constructed for the single-symbol
// `/ws` path. Q9 / R4 — one WS connection per venue covers up to 200
// streams; v1.5b worst case is 40.

/// Combined-stream wrapper that Binance emits when subscribing via
/// `/stream?streams=<list>`. The `data` field carries the same JSON shape
/// as the single-symbol stream events.
#[derive(Debug, Deserialize)]
struct CombinedStreamEnvelope {
    stream: String,
    data: serde_json::Value,
}

/// Convert a `/ws` (single-stream) base URL to the corresponding combined-
/// stream base URL. Tolerates inputs that already point at `/stream`.
fn combined_stream_base(ws_base: &str) -> String {
    // Strip trailing `/ws` if present.
    let trimmed = ws_base
        .strip_suffix("/ws")
        .unwrap_or_else(|| ws_base.strip_suffix("/stream").unwrap_or(ws_base));
    trimmed.trim_end_matches('/').to_string()
}

/// Build the combined-stream URL for a list of stream names.
///
/// `wss://host[:port]/stream?streams=a/b/c`
pub(crate) fn build_combined_stream_url(ws_base: &str, streams: &[String]) -> String {
    let base = combined_stream_base(ws_base);
    format!("{base}/stream?streams={}", streams.join("/"))
}

/// Parse a combined-stream envelope's `stream` field back to the agent's
/// `Symbol`. The Binance `stream` is the lowercased stream name, e.g.
/// `"btcusdt@kline_1m"`. We derive the symbol by uppercasing the prefix.
fn symbol_from_stream(stream: &str) -> Symbol {
    let prefix = stream.split('@').next().unwrap_or(stream);
    Symbol::new(prefix.to_uppercase())
}

impl BinanceFeed {
    /// Subscribe to closed kline bars across multiple symbols on a single
    /// WebSocket connection (T612 — finally lands as T1405).
    ///
    /// Uses the Binance combined-stream URL:
    /// `wss://stream.binance.com:9443/stream?streams=<list>` with each
    /// stream of the form `<symbol>@kline_<tf>`. The merged stream is
    /// parsed via the combined-stream envelope and dispatched to the
    /// existing per-stream parser shape.
    ///
    /// Single-symbol callers MUST keep using `subscribe_bars` (R10.3).
    ///
    /// # Errors
    ///
    /// Returns [`FeedError::Connection`] if the initial connection fails
    /// or if `symbols` is empty.
    pub async fn subscribe_bars_multi(
        &self,
        symbols: &[Symbol],
        tf: Timeframe,
    ) -> Result<BoxStream<'static, Result<Bar, FeedError>>, FeedError> {
        if symbols.is_empty() {
            return Err(FeedError::Connection(
                "subscribe_bars_multi requires at least one symbol".into(),
            ));
        }
        let stream_names: Vec<String> = symbols
            .iter()
            .map(|s| {
                format!(
                    "{}@kline_{}",
                    s.0.as_str().to_lowercase(),
                    tf_to_binance_str(tf)
                )
            })
            .collect();
        let ws_url = build_combined_stream_url(&self.ws_url, &stream_names);
        let symbols_clone: Vec<Symbol> = symbols.to_vec();
        let ledger_for_stream = self.ledger.clone();

        // Verify initial connection.
        let _ws = connect_ws(&ws_url).await?;

        let stream = async_stream::stream! {
            let mut backoff_secs: u64 = 1;
            let mut is_reconnect = false;
            loop {
                debug!(streams = ?stream_names, "connecting to binance combined kline WS");
                match connect_ws(&ws_url).await {
                    Err(e) => {
                        error!(error = %e, "binance combined kline WS connect failed, retrying in {backoff_secs}s");
                        sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(60);
                        continue;
                    }
                    Ok(mut ws) => {
                        backoff_secs = 1;
                        if is_reconnect
                            && let Some(ledger) = ledger_for_stream.as_ref()
                        {
                            // Emit one feed_reconnect per subscribed symbol.
                            for sym in &symbols_clone {
                                if let Err(e) = audit::journal::feed_reconnect(
                                    ledger,
                                    sym.0.as_str(),
                                    Venue::Binance,
                                    None,
                                ).await {
                                    warn!(error = %e, "feed_reconnect audit write failed (non-fatal)");
                                }
                            }
                        }
                        is_reconnect = true;
                        loop {
                            match ws.next().await {
                                None => { warn!("binance combined kline WS closed, reconnecting"); break; }
                                Some(Err(e)) => { warn!(error = %e, "binance combined kline WS error, reconnecting"); break; }
                                Some(Ok(Message::Ping(data))) => {
                                    if let Err(e) = futures::SinkExt::send(&mut ws, Message::Pong(data)).await {
                                        warn!(error = %e, "pong send failed");
                                        break;
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    let local_ts = Timestamp::now();
                                    let env: CombinedStreamEnvelope = match serde_json::from_str(&text) {
                                        Ok(e) => e,
                                        Err(e) => {
                                            warn!(error = %e, text = %text, "binance combined-stream envelope parse error");
                                            continue;
                                        }
                                    };
                                    // Dispatch to per-stream kline parser.
                                    let kline_evt: KlineEvent = match serde_json::from_value(env.data) {
                                        Ok(k) => k,
                                        Err(e) => {
                                            warn!(error = %e, "kline data parse error");
                                            continue;
                                        }
                                    };
                                    if !kline_evt.kline.is_closed { continue; }
                                    let sym = symbol_from_stream(&env.stream);
                                    let k = &kline_evt.kline;
                                    let result = (|| -> Result<Bar, FeedError> {
                                        Ok(Bar {
                                            symbol: sym,
                                            tf,
                                            open_ts: millis_to_timestamp(k.open_time),
                                            close_ts: millis_to_timestamp(k.close_time),
                                            open: parse_price(&k.open, "open")?,
                                            high: parse_price(&k.high, "high")?,
                                            low: parse_price(&k.low, "low")?,
                                            close: parse_price(&k.close, "close")?,
                                            volume: parse_qty(&k.volume, "volume")?,
                                            trade_count: k.trade_count,
                                            local_recv_ts: local_ts,
                                            venue: Venue::Binance,
                                        })
                                    })();
                                    yield result;
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

    /// Subscribe to raw trades across multiple symbols on a single WS
    /// connection (T612 — T1405). See [`Self::subscribe_bars_multi`] for
    /// rationale and topology.
    ///
    /// # Errors
    ///
    /// Returns [`FeedError::Connection`] if the initial connection fails
    /// or if `symbols` is empty.
    pub async fn subscribe_trades_multi(
        &self,
        symbols: &[Symbol],
    ) -> Result<BoxStream<'static, Result<Tick, FeedError>>, FeedError> {
        if symbols.is_empty() {
            return Err(FeedError::Connection(
                "subscribe_trades_multi requires at least one symbol".into(),
            ));
        }
        let stream_names: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@trade", s.0.as_str().to_lowercase()))
            .collect();
        let ws_url = build_combined_stream_url(&self.ws_url, &stream_names);
        let symbols_clone: Vec<Symbol> = symbols.to_vec();
        let ledger_for_stream = self.ledger.clone();

        let _ws = connect_ws(&ws_url).await?;

        let stream = async_stream::stream! {
            let mut backoff_secs: u64 = 1;
            let mut is_reconnect = false;
            loop {
                debug!(streams = ?stream_names, "connecting to binance combined trade WS");
                match connect_ws(&ws_url).await {
                    Err(e) => {
                        error!(error = %e, "binance combined trade WS connect failed, retrying in {backoff_secs}s");
                        sleep(Duration::from_secs(backoff_secs)).await;
                        backoff_secs = (backoff_secs * 2).min(60);
                        continue;
                    }
                    Ok(mut ws) => {
                        backoff_secs = 1;
                        if is_reconnect
                            && let Some(ledger) = ledger_for_stream.as_ref()
                        {
                            for sym in &symbols_clone {
                                if let Err(e) = audit::journal::feed_reconnect(
                                    ledger,
                                    sym.0.as_str(),
                                    Venue::Binance,
                                    None,
                                ).await {
                                    warn!(error = %e, "feed_reconnect audit write failed (non-fatal)");
                                }
                            }
                        }
                        is_reconnect = true;
                        loop {
                            match ws.next().await {
                                None => { warn!("binance combined trade WS closed, reconnecting"); break; }
                                Some(Err(e)) => { warn!(error = %e, "binance combined trade WS error, reconnecting"); break; }
                                Some(Ok(Message::Ping(data))) => {
                                    if let Err(e) = futures::SinkExt::send(&mut ws, Message::Pong(data)).await {
                                        warn!(error = %e, "pong send failed");
                                        break;
                                    }
                                }
                                Some(Ok(Message::Text(text))) => {
                                    let local_ts = Timestamp::now();
                                    let env: CombinedStreamEnvelope = match serde_json::from_str(&text) {
                                        Ok(e) => e,
                                        Err(e) => {
                                            warn!(error = %e, text = %text, "binance combined-stream envelope parse error");
                                            continue;
                                        }
                                    };
                                    let trade_evt: TradeEvent = match serde_json::from_value(env.data) {
                                        Ok(t) => t,
                                        Err(e) => {
                                            warn!(error = %e, "trade data parse error");
                                            continue;
                                        }
                                    };
                                    let sym = symbol_from_stream(&env.stream);
                                    let result = (|| -> Result<Tick, FeedError> {
                                        Ok(Tick {
                                            symbol: sym,
                                            venue_ts: millis_to_timestamp(trade_evt.trade_time),
                                            local_recv_ts: local_ts,
                                            price: parse_price(&trade_evt.price, "price")?,
                                            qty: parse_qty(&trade_evt.qty, "qty")?,
                                            side: if trade_evt.is_buyer_maker { Side::Sell } else { Side::Buy },
                                            trade_id: trade_evt.trade_id,
                                            venue: Venue::Binance,
                                        })
                                    })();
                                    yield result;
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
mod multi_tests {
    use super::*;

    /// T1405 — verify the combined-stream URL builder produces the canonical
    /// Binance URL for a 10-symbol fan-out subscription.
    #[test]
    fn t1405_binance_multi_symbol_fan_out() {
        let symbols: Vec<Symbol> = [
            "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT",
            "AVAXUSDT", "DOTUSDT", "LINKUSDT",
        ]
        .iter()
        .map(|s| Symbol::new(*s))
        .collect();
        let stream_names: Vec<String> = symbols
            .iter()
            .map(|s| format!("{}@kline_1m", s.0.as_str().to_lowercase()))
            .collect();
        let url = build_combined_stream_url("wss://stream.binance.com:9443/ws", &stream_names);
        // Q9: combined-stream URL form `/stream?streams=a/b/...`.
        assert_eq!(
            url,
            "wss://stream.binance.com:9443/stream?streams=\
btcusdt@kline_1m/ethusdt@kline_1m/bnbusdt@kline_1m/solusdt@kline_1m/xrpusdt@kline_1m/\
adausdt@kline_1m/dogeusdt@kline_1m/avaxusdt@kline_1m/dotusdt@kline_1m/linkusdt@kline_1m"
        );
    }

    #[test]
    fn t1405_combined_stream_strips_ws_suffix() {
        let url = build_combined_stream_url(
            "wss://stream.binance.com:9443/ws",
            &["btcusdt@trade".to_string()],
        );
        assert_eq!(
            url,
            "wss://stream.binance.com:9443/stream?streams=btcusdt@trade"
        );
    }

    #[test]
    fn t1405_combined_stream_handles_already_stream_base() {
        let url = build_combined_stream_url(
            "wss://stream.binance.com:9443/stream",
            &["btcusdt@trade".to_string()],
        );
        assert_eq!(
            url,
            "wss://stream.binance.com:9443/stream?streams=btcusdt@trade"
        );
    }

    #[test]
    fn t1405_combined_envelope_parses_kline() {
        let raw = r#"{"stream":"btcusdt@kline_1m","data":{"e":"kline","E":1714579260123,"s":"BTCUSDT","k":{"t":1714579200000,"T":1714579259999,"s":"BTCUSDT","i":"1m","f":1,"L":2,"o":"60000.0","c":"60050.0","h":"60100.0","l":"59950.0","v":"1.5","n":7,"x":true,"q":"90000.0","V":"0.7","Q":"42000.0","B":"0"}}}"#;
        let env: CombinedStreamEnvelope = serde_json::from_str(raw).expect("envelope parses");
        assert_eq!(env.stream, "btcusdt@kline_1m");
        let kline: KlineEvent = serde_json::from_value(env.data).expect("kline parses");
        assert!(kline.kline.is_closed);
        assert_eq!(kline.kline.trade_count, 7);
        assert_eq!(symbol_from_stream(&env.stream), Symbol::new("BTCUSDT"));
    }

    #[test]
    fn t1405_symbol_from_stream_handles_trade_channel() {
        assert_eq!(symbol_from_stream("ethusdt@trade"), Symbol::new("ETHUSDT"));
        assert_eq!(
            symbol_from_stream("dogeusdt@kline_1m"),
            Symbol::new("DOGEUSDT")
        );
    }
}
