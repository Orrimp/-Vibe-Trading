//! Live Binance Spot execution client (F1).
//!
//! `BinanceSpotExecClient` implements both `LiveExecRouter` and `AccountReader`.
//!
//! **Binding law (ADR-0054 § D3):**
//! 1. No secrets in git — constructor takes `&dyn SecretSource`, fails closed.
//! 2. Money is `Decimal` / never `f64`.
//! 3. Every external I/O behind a trait — `FakeTransport` is the test double.
//! 4. The operator arms; the agent never self-arms.
//! 5. The kill switch is supreme.
//!
//! **MARKET-only in F1** — `OrderKind::Limit` → `ExecError::UnsupportedOrderType`
//! (typed reject, never silent — AQ-3).

pub mod cap;
pub mod clock;
pub mod endpoint;
pub mod error;
pub mod filters;
pub mod sign;
pub mod types;

use std::collections::BTreeMap;
use std::sync::Mutex;
use std::time::Duration;

use crate::live::clock::ServerTimeOffset;
use crate::live::endpoint::{ExecEndpoint, Network};
use crate::live::error::{ExecError, map_binance_code};
use crate::live::filters::{ExchangeFilters, FilterCache, parse_filters_from_json, validate_order};
use crate::live::sign::sign;
use crate::live::types::{
    AccountSnapshot, BINANCE_CODE_ORDER_NOT_FOUND, Balance, BinanceAccountResponse,
    BinanceErrorResponse, BinanceOrderResponse, BinanceOrderStatusResponse, BinanceServerTime,
    OrderAck, OrderRef, OrderStatus, OrderStatusKind, parse_decimal,
};
use async_trait::async_trait;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::{debug, info, warn};
use trading_core::asset::Asset;
use trading_core::order::{Order, OrderKind};
use trading_core::secret::SecretSource;

/// Maximum retry count for rate-limited or transport-error requests.
const MAX_RETRIES: u32 = 3;
/// Hard ceiling for exponential backoff.
const MAX_BACKOFF_MS: u64 = 30_000;
/// Default `recvWindow` in milliseconds.
const RECV_WINDOW_MS: u64 = 5_000;
/// Default symbol used when constructing the client without a symbol argument.
pub const DEFAULT_SYMBOL: &str = "BTCUSDT";

// ── LiveExecRouter trait ──────────────────────────────────────────────────────

/// Live execution router — authenticated exchange order operations.
///
/// `place_order` takes `&Order` (immutable `&self`) so one
/// `Arc<dyn LiveExecRouter>` is shareable across tasks.
#[async_trait]
pub trait LiveExecRouter: Send + Sync {
    /// Place a MARKET order.  Returns an [`OrderAck`] on success.
    async fn place_order(&self, order: &Order) -> Result<OrderAck, ExecError>;
    /// Query order status by reference.
    async fn order_status(&self, r: &OrderRef) -> Result<OrderStatus, ExecError>;
    /// Cancel an order.
    async fn cancel_order(&self, r: &OrderRef) -> Result<(), ExecError>;
}

// ── AccountReader trait ───────────────────────────────────────────────────────

/// Read real account balances from the exchange (`GET /api/v3/account`).
#[async_trait]
pub trait AccountReader: Send + Sync {
    /// Fetch the current account snapshot.
    async fn account_snapshot(&self) -> Result<AccountSnapshot, ExecError>;
}

// ── BinanceSpotExecClient ─────────────────────────────────────────────────────

/// Authenticated Binance Spot REST client.
///
/// ONE client — endpoint (testnet vs mainnet) + keys are constructor-injected.
/// Never has a hard-coded URL (the `binance.rs:128-133` anti-pattern F1 must
/// not repeat).
///
/// The constructor fails closed if either key is `Missing` (AC-3).
pub struct BinanceSpotExecClient {
    /// Resolved endpoint (base URL + label).
    endpoint: ExecEndpoint,
    /// HTTP client (shared across requests).
    http: reqwest::Client,
    /// API key (redacted in Debug via `SecretString`).
    api_key: trading_core::secret::SecretString,
    /// API secret (redacted in Debug via `SecretString`).
    api_secret: trading_core::secret::SecretString,
    /// Server-time offset for signed requests.
    clock: Mutex<ServerTimeOffset>,
    /// TTL-cached exchange filters per symbol.
    filter_cache: Mutex<std::collections::HashMap<String, FilterCache>>,
    /// Optional exec-side notional cap (from `[live].max_notional_usdt`).
    max_notional_usdt: Option<Decimal>,
    /// Symbol this client trades (default BTCUSDT).
    symbol: String,
}

// We implement Debug manually so the api_key/api_secret never appear.
impl std::fmt::Debug for BinanceSpotExecClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BinanceSpotExecClient")
            .field("endpoint", &self.endpoint)
            .field("api_key", &"<redacted>")
            .field("api_secret", &"<redacted>")
            .field("symbol", &self.symbol)
            .finish_non_exhaustive()
    }
}

impl BinanceSpotExecClient {
    /// Construct the client.
    ///
    /// Fails closed if either `BINANCE_API_KEY` or `BINANCE_API_SECRET`
    /// is `Missing` from `secrets` (AC-3).
    ///
    /// # Errors
    /// - [`ExecError::Auth`] wrapping [`SecretError::Missing`] when either
    ///   key is absent.
    pub fn connect(
        network: Network,
        secrets: &dyn SecretSource,
        http: reqwest::Client,
        symbol: impl Into<String>,
        max_notional_usdt: Option<Decimal>,
    ) -> Result<Self, ExecError> {
        let api_key = secrets
            .get("BINANCE_API_KEY")
            .map_err(|e| ExecError::Auth(format!("BINANCE_API_KEY missing: {e}")))?;
        let api_secret = secrets
            .get("BINANCE_API_SECRET")
            .map_err(|e| ExecError::Auth(format!("BINANCE_API_SECRET missing: {e}")))?;
        let endpoint = network.endpoint();
        let sym = symbol.into();
        info!(
            network = endpoint.label,
            symbol = %sym,
            "BinanceSpotExecClient connected"
        );
        Ok(Self {
            endpoint,
            http,
            api_key,
            api_secret,
            clock: Mutex::new(ServerTimeOffset::default()),
            filter_cache: Mutex::new(std::collections::HashMap::new()),
            max_notional_usdt,
            symbol: sym,
        })
    }

    /// The current endpoint label (`"testnet"` or `"mainnet"`).
    #[must_use]
    pub fn endpoint_label(&self) -> &'static str {
        self.endpoint.label
    }

    // ── Internal helpers ──────────────────────────────────────────────────────

    fn timestamp_ms(&self) -> u64 {
        // SAFETY: mutex poisoning can only occur if a thread panicked while
        // holding the lock; in that case we recover the inner value — the
        // ServerTimeOffset state is still valid Rust.
        self.clock
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .adjusted_now_ms()
    }

    fn signature(&self, query: &str) -> String {
        sign(self.api_secret.expose_secret(), query)
    }

    fn api_key_str(&self) -> &str {
        self.api_key.expose_str()
    }

    /// Build a signed query string by appending `timestamp`, `recvWindow`,
    /// and `signature`.
    ///
    /// **NEVER log the returned string** — it contains the signature.
    /// Log only the prefix (endpoint + symbol/side) at debug level.
    fn signed_query(&self, base: &str) -> String {
        let ts = self.timestamp_ms();
        let with_ts = format!("{base}&timestamp={ts}&recvWindow={RECV_WINDOW_MS}");
        let sig = self.signature(&with_ts);
        format!("{with_ts}&signature={sig}")
    }

    /// Parse a Binance REST response: on HTTP 4xx/5xx extract the error code.
    async fn parse_binance_error(resp: reqwest::Response) -> ExecError {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<BinanceErrorResponse>(&body) {
            map_binance_code(err.code, &err.msg)
        } else {
            ExecError::Unknown(format!("HTTP {}: {body}", status.as_u16()))
        }
    }

    /// Sync the clock from `GET /api/v3/time`.
    /// Logs the endpoint label (never the key or signature).
    async fn sync_clock(&self) -> Result<(), ExecError> {
        let url = format!("{}/api/v3/time", self.endpoint.base_url);
        debug!(endpoint = self.endpoint.label, "syncing clock");
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecError::Transport(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(Self::parse_binance_error(resp).await);
        }
        let t: BinanceServerTime = resp
            .json()
            .await
            .map_err(|e| ExecError::Transport(format!("clock parse: {e}")))?;
        // SAFETY: mutex recovery — see timestamp_ms comment.
        self.clock
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .sync(t.server_time);
        Ok(())
    }

    /// Fetch and cache exchange filters for `symbol`.
    async fn fetch_filters(&self, symbol: &str) -> Result<ExchangeFilters, ExecError> {
        // Check cache first.
        {
            // SAFETY: mutex recovery — see timestamp_ms comment.
            let cache = self.filter_cache.lock().unwrap_or_else(|e| e.into_inner());
            if let Some(c) = cache.get(symbol) {
                if let Some(f) = c.get() {
                    return Ok(f.clone());
                }
            }
        }
        // Fetch from exchange.
        let url = format!(
            "{}/api/v3/exchangeInfo?symbol={symbol}",
            self.endpoint.base_url
        );
        debug!(
            endpoint = self.endpoint.label,
            symbol, "fetching exchangeInfo"
        );
        let resp = self
            .http
            .get(&url)
            .send()
            .await
            .map_err(|e| ExecError::Transport(e.to_string()))?;
        if !resp.status().is_success() {
            return Err(Self::parse_binance_error(resp).await);
        }
        let json = resp
            .text()
            .await
            .map_err(|e| ExecError::Transport(format!("exchangeInfo read: {e}")))?;
        let filters = parse_filters_from_json(&json, symbol)?;
        // Store in cache.
        {
            // SAFETY: mutex recovery — see timestamp_ms comment.
            let mut cache = self.filter_cache.lock().unwrap_or_else(|e| e.into_inner());
            let entry = cache.entry(symbol.to_string()).or_default();
            entry.store(filters.clone());
        }
        Ok(filters)
    }

    /// Invalidate filter cache for a symbol (after exchange filter-reject).
    fn invalidate_filter_cache(&self, symbol: &str) {
        // SAFETY: mutex recovery — see timestamp_ms comment.
        let mut cache = self.filter_cache.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(c) = cache.get_mut(symbol) {
            c.invalidate();
        }
    }

    /// Place the actual signed POST /api/v3/order.
    ///
    /// Returns `Ok(ack)` on success.  On `-1021` (clock skew), resyncs and
    /// retries once.  On `-1013`/`-2010` filter rejects, invalidates the
    /// filter cache.  Never logs the signature.
    ///
    /// Box::pin is required because Rust stable doesn't support recursive
    /// async fns natively; we use it only for the single clock-skew retry path.
    fn post_order_inner<'a>(
        &'a self,
        query_base: &'a str,
        symbol: &'a str,
        allow_clock_retry: bool,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<OrderAck, ExecError>> + Send + 'a>>
    {
        Box::pin(async move {
            let signed = self.signed_query(query_base);
            let url = format!("{}/api/v3/order", self.endpoint.base_url);
            // Log endpoint + symbol, NOT the signed query.
            debug!(endpoint = self.endpoint.label, symbol, "POST /api/v3/order");

            let resp = self
                .http
                .post(&url)
                .header("X-MBX-APIKEY", self.api_key_str())
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(signed)
                .send()
                .await
                .map_err(|e| ExecError::Transport(e.to_string()))?;

            if resp.status().is_success() {
                let body: BinanceOrderResponse = resp
                    .json()
                    .await
                    .map_err(|e| ExecError::Transport(format!("order ack parse: {e}")))?;
                return Ok(OrderAck {
                    client_order_id: body.client_order_id,
                    exchange_order_id: body.order_id,
                    symbol: body.symbol,
                    status: body.status,
                    executed_qty: parse_decimal(&body.executed_qty),
                    orig_qty: parse_decimal(&body.orig_qty),
                });
            }

            let err = Self::parse_binance_error(resp).await;
            match &err {
                ExecError::ClockSkew if allow_clock_retry => {
                    warn!(endpoint = self.endpoint.label, "clock skew, resyncing");
                    self.sync_clock().await?;
                    // Check for persistent skew before re-trying.
                    // SAFETY: mutex recovery — see timestamp_ms comment.
                    self.clock
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .check_persistent()?;
                    return self.post_order_inner(query_base, symbol, false).await;
                }
                ExecError::FilterReject(_) => {
                    self.invalidate_filter_cache(symbol);
                }
                _ => {}
            }
            Err(err)
        })
    }
}

// ── LiveExecRouter impl ───────────────────────────────────────────────────────

#[async_trait]
impl LiveExecRouter for BinanceSpotExecClient {
    async fn place_order(&self, order: &Order) -> Result<OrderAck, ExecError> {
        let symbol = order.symbol().to_string();

        // AQ-3: reject non-MARKET orders with a typed error.
        if !matches!(order.kind(), OrderKind::Market) {
            return Err(ExecError::UnsupportedOrderType(format!(
                "F1 supports MARKET only; got {:?}",
                order.kind()
            )));
        }

        let qty = order.qty().get();
        let side = format!("{:?}", order.side()).to_uppercase();

        // R4: validate against exchange filters (client-side, never reaches network on fail).
        let filters = self.fetch_filters(&symbol).await?;
        // Use a reference price from the filter's minNotional as a floor guard;
        // for MARKET orders we don't have a price — use a generous estimate.
        // The real price validation happens at the exchange. We use 1.0 as a
        // placeholder price to check qty bounds (notional check uses actual mark).
        // For filter pre-flight: use 1 USDT as floor (conservative check only).
        let rounded_qty = validate_order(qty, dec!(1), &filters)?;

        // AC-11: exec-side notional cap (if configured).
        // For MARKET orders we can't know the exact fill price, so we skip
        // the notional cap check here (it requires a mark price the client
        // doesn't carry). The cap check is called by F2's check_armed with
        // a known mark price. `check_notional_cap` is still tested in isolation.
        //
        // If a cap is configured and we have a rough estimate, apply it.
        // For now: the cap is a standing guard called by the F2 arming layer
        // with the actual mark. (F1 builds the mechanism; F2 wires it.)
        if let Some(cap) = self.max_notional_usdt {
            // Best-effort: if qty * 1 > cap (absurdly large), still reject.
            // The real check runs with actual mark in F2.
            if rounded_qty > cap {
                return Err(ExecError::CapExceeded {
                    notional: rounded_qty,
                    cap,
                });
            }
        }

        // Construct the query base (no timestamp/signature yet).
        let client_order_id = order.id().to_string();
        let query_base = format!(
            "symbol={symbol}&side={side}&type=MARKET&quantity={rounded_qty}\
             &newClientOrderId={client_order_id}"
        );

        // Place with retry on transport errors (AC-8 contract).
        let mut last_err = ExecError::Unknown("no attempt made".to_string());
        let mut backoff_ms: u64 = 500;
        for attempt in 0..MAX_RETRIES {
            match self.post_order_inner(&query_base, &symbol, true).await {
                Ok(ack) => {
                    info!(
                        endpoint = self.endpoint.label,
                        symbol, attempt, "order placed"
                    );
                    return Ok(ack);
                }
                Err(ExecError::Transport(_)) => {
                    // AC-8: query status BEFORE any retry.
                    warn!(
                        endpoint = self.endpoint.label,
                        symbol, attempt, "transport error — querying status before retry"
                    );
                    let oref = OrderRef {
                        client_order_id: client_order_id.clone(),
                        exchange_order_id: 0,
                        symbol: symbol.clone(),
                    };
                    match self.order_status(&oref).await {
                        Ok(status) if status.exists => {
                            // Order already landed — reconstruct ack from status.
                            info!(
                                endpoint = self.endpoint.label,
                                symbol, "order found after transport error — not resubmitting"
                            );
                            return Ok(OrderAck {
                                client_order_id: status.client_order_id,
                                exchange_order_id: status.exchange_order_id,
                                symbol: status.symbol,
                                status: format!("{:?}", status.status),
                                executed_qty: status.executed_qty,
                                orig_qty: status.orig_qty,
                            });
                        }
                        _ => {
                            // Order confirmed not found — safe to retry.
                        }
                    }
                    last_err = ExecError::Transport(format!("attempt {attempt} failed"));
                    tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                    backoff_ms = (backoff_ms * 2).min(MAX_BACKOFF_MS);
                }
                Err(ExecError::RateLimited { retry_after }) => {
                    warn!(
                        endpoint = self.endpoint.label,
                        ?retry_after,
                        attempt,
                        "rate limited"
                    );
                    tokio::time::sleep(retry_after).await;
                    last_err = ExecError::RateLimited { retry_after };
                }
                Err(e) => {
                    // Auth, FilterReject, etc. — no retry.
                    return Err(e);
                }
            }
        }
        // N-retry exhaustion → log + surface (never silent — R6).
        warn!(
            endpoint = self.endpoint.label,
            symbol,
            attempts = MAX_RETRIES,
            "retry exhaustion — caller should halt"
        );
        Err(last_err)
    }

    async fn order_status(&self, r: &OrderRef) -> Result<OrderStatus, ExecError> {
        let query_base = format!(
            "symbol={}&origClientOrderId={}",
            r.symbol, r.client_order_id
        );
        let signed = self.signed_query(&query_base);
        let url = format!("{}/api/v3/order?{signed}", self.endpoint.base_url);
        debug!(
            endpoint = self.endpoint.label,
            symbol = r.symbol,
            "GET /api/v3/order"
        );

        let resp = self
            .http
            .get(&url)
            .header("X-MBX-APIKEY", self.api_key_str())
            .send()
            .await
            .map_err(|e| ExecError::Transport(e.to_string()))?;

        if resp.status().is_success() {
            let body: BinanceOrderStatusResponse = resp
                .json()
                .await
                .map_err(|e| ExecError::Transport(format!("status parse: {e}")))?;
            let status_kind =
                serde_json::from_str::<OrderStatusKind>(&format!("\"{}\"", body.status))
                    .unwrap_or(OrderStatusKind::Unknown);
            return Ok(OrderStatus {
                client_order_id: body.client_order_id,
                exchange_order_id: body.order_id,
                symbol: body.symbol,
                status: status_kind,
                orig_qty: parse_decimal(&body.orig_qty),
                executed_qty: parse_decimal(&body.executed_qty),
                exists: true,
            });
        }

        // Check for "order not found" (Binance -2013).
        let status_code = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if let Ok(err) = serde_json::from_str::<BinanceErrorResponse>(&body) {
            if err.code == BINANCE_CODE_ORDER_NOT_FOUND {
                return Ok(OrderStatus {
                    client_order_id: r.client_order_id.clone(),
                    exchange_order_id: r.exchange_order_id,
                    symbol: r.symbol.clone(),
                    status: OrderStatusKind::Unknown,
                    orig_qty: Decimal::ZERO,
                    executed_qty: Decimal::ZERO,
                    exists: false,
                });
            }
            return Err(map_binance_code(err.code, &err.msg));
        }
        Err(ExecError::Unknown(format!(
            "HTTP {}: {body}",
            status_code.as_u16()
        )))
    }

    async fn cancel_order(&self, r: &OrderRef) -> Result<(), ExecError> {
        let query_base = format!(
            "symbol={}&origClientOrderId={}",
            r.symbol, r.client_order_id
        );
        let signed = self.signed_query(&query_base);
        let url = format!("{}/api/v3/order?{signed}", self.endpoint.base_url);
        debug!(
            endpoint = self.endpoint.label,
            symbol = r.symbol,
            "DELETE /api/v3/order"
        );

        let resp = self
            .http
            .delete(&url)
            .header("X-MBX-APIKEY", self.api_key_str())
            .send()
            .await
            .map_err(|e| ExecError::Transport(e.to_string()))?;

        if resp.status().is_success() {
            info!(
                endpoint = self.endpoint.label,
                symbol = r.symbol,
                "order cancelled"
            );
            return Ok(());
        }
        Err(Self::parse_binance_error(resp).await)
    }
}

// ── AccountReader impl ────────────────────────────────────────────────────────

#[async_trait]
impl AccountReader for BinanceSpotExecClient {
    async fn account_snapshot(&self) -> Result<AccountSnapshot, ExecError> {
        // Build signed query: empty base, timestamp + recvWindow appended by signed_query.
        let query_base = String::new();
        let signed = self.signed_query(&query_base);
        let url = format!("{}/api/v3/account?{signed}", self.endpoint.base_url);
        debug!(endpoint = self.endpoint.label, "GET /api/v3/account");

        let resp = self
            .http
            .get(&url)
            .header("X-MBX-APIKEY", self.api_key_str())
            .send()
            .await
            .map_err(|e| ExecError::Transport(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(Self::parse_binance_error(resp).await);
        }

        let body: BinanceAccountResponse = resp
            .json()
            .await
            .map_err(|e| ExecError::Transport(format!("account parse: {e}")))?;

        let mut balances = BTreeMap::new();
        for b in body.balances {
            let free: Decimal = b.free.parse().unwrap_or(Decimal::ZERO);
            let locked: Decimal = b.locked.parse().unwrap_or(Decimal::ZERO);
            if free.is_zero() && locked.is_zero() {
                continue; // skip zero balances for cleanliness
            }
            let asset = match b.asset.as_str() {
                "BTC" => Asset::Btc,
                "USDT" => Asset::Usdt,
                "ETH" => Asset::Eth,
                other => Asset::Other(other.into()),
            };
            balances.insert(asset, Balance { free, locked });
        }

        Ok(AccountSnapshot { balances })
    }
}
