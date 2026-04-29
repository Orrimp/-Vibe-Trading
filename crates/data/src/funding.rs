//! Funding-rate REST poller (T613 — v1 Q2).
//!
//! Polls `https://fapi.binance.com/fapi/v1/premiumIndex` once per hour
//! for each universe symbol, persists rows to the `funding_rates` SQLite
//! table, and broadcasts on the `funding_obs` channel.
//!
//! ## Fault tolerance (architect risk #5)
//!
//! The poller MUST skip-and-log on transient errors (rate-limit, 5xx).
//! A venue blip must never take the agent down.  The polling loop uses
//! `continue` on every error variant, logging at `warn` level.
//!
//! ## Mode gating
//!
//! `FundingPoller` is active in `paper` and `research` modes only.
//! The caller is responsible for not starting the poller in `live` mode.

use std::time::Duration;

use rust_decimal::Decimal;
use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use tracing::{info, warn};
use trading_core::{FundingObs, Symbol, Timestamp};

/// Mock or real REST client trait — injected so tests can fake the endpoint.
#[cfg_attr(test, mockall::automock)]
#[async_trait::async_trait]
pub trait FundingRestClient: Send + Sync {
    async fn fetch_premium_index(
        &self,
        symbol: &str,
    ) -> Result<PremiumIndexResponse, FundingPollError>;
}

/// Response from Binance `GET /fapi/v1/premiumIndex`.
#[derive(Debug, Clone)]
pub struct PremiumIndexResponse {
    pub symbol: String,
    pub last_funding_rate: Decimal,
    pub next_funding_time: i64, // Unix ms
    pub time: i64,              // Unix ms
}

/// Polling error.
#[derive(Debug, thiserror::Error)]
pub enum FundingPollError {
    #[error("HTTP error: {0}")]
    Http(String),
    #[error("parse error: {0}")]
    Parse(String),
    #[error("rate limited")]
    RateLimited,
}

/// Funding rate poller task.
pub struct FundingPoller {
    pub universe: Vec<Symbol>,
    pub interval: Duration,
}

impl FundingPoller {
    /// Create a new poller with the default 1-hour interval.
    pub fn new(universe: Vec<Symbol>) -> Self {
        Self {
            universe,
            interval: Duration::from_secs(3600),
        }
    }

    /// Run the polling loop until the cancellation token is triggered.
    ///
    /// On each interval:
    /// 1. For each universe symbol, polls the Binance `premiumIndex` endpoint.
    /// 2. On success, broadcasts `FundingObs` on the channel.
    /// 3. On transient error, logs and continues (fault tolerance).
    pub async fn run(
        &self,
        client: &dyn FundingRestClient,
        tx: &broadcast::Sender<FundingObs>,
        cancel: CancellationToken,
    ) {
        info!(
            universe_size = self.universe.len(),
            "funding_poller started"
        );

        loop {
            tokio::select! {
                () = cancel.cancelled() => {
                    info!("funding_poller: cancellation received, stopping");
                    break;
                }
                () = tokio::time::sleep(self.interval) => {
                    self.poll_once(client, tx).await;
                }
            }
        }
    }

    async fn poll_once(&self, client: &dyn FundingRestClient, tx: &broadcast::Sender<FundingObs>) {
        let poll_ts = Timestamp::now();

        for symbol in &self.universe {
            match client.fetch_premium_index(symbol.0.as_str()).await {
                Ok(resp) => {
                    let funding_rate = resp.last_funding_rate;
                    let funding_ts = millis_to_ts(resp.time);
                    let next_funding_ts = millis_to_ts(resp.next_funding_time);

                    let obs = FundingObs {
                        symbol: symbol.clone(),
                        funding_rate,
                        funding_ts,
                        next_funding_ts,
                        poll_ts,
                    };

                    // Broadcast — ignore lag / closed errors per fault-tolerance policy.
                    let _ = tx.send(obs);
                }
                Err(e) => {
                    // Fault tolerance: skip-and-log, never panic (architect risk #5).
                    warn!(
                        symbol = %symbol,
                        error = %e,
                        "funding_poller: transient error fetching premium index, skipping"
                    );
                }
            }
        }
    }
}

fn millis_to_ts(ms: i64) -> Timestamp {
    use time::OffsetDateTime;
    OffsetDateTime::from_unix_timestamp_nanos(i128::from(ms) * 1_000_000)
        .map(Timestamp::new)
        .unwrap_or_else(|_| Timestamp::now())
}

/// Real reqwest-backed client.
pub struct BinanceFundingClient {
    client: reqwest::Client,
}

impl BinanceFundingClient {
    /// Create a new client.
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }
}

impl Default for BinanceFundingClient {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl FundingRestClient for BinanceFundingClient {
    async fn fetch_premium_index(
        &self,
        symbol: &str,
    ) -> Result<PremiumIndexResponse, FundingPollError> {
        let url = format!("https://fapi.binance.com/fapi/v1/premiumIndex?symbol={symbol}");
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| FundingPollError::Http(e.to_string()))?;

        if resp.status() == 429 {
            return Err(FundingPollError::RateLimited);
        }
        if !resp.status().is_success() {
            return Err(FundingPollError::Http(format!("status {}", resp.status())));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| FundingPollError::Parse(e.to_string()))?;

        let last_funding_rate = json["lastFundingRate"]
            .as_str()
            .ok_or_else(|| FundingPollError::Parse("missing lastFundingRate".into()))?
            .parse::<Decimal>()
            .map_err(|e| FundingPollError::Parse(e.to_string()))?;

        let next_funding_time = json["nextFundingTime"]
            .as_i64()
            .ok_or_else(|| FundingPollError::Parse("missing nextFundingTime".into()))?;

        let time = json["time"]
            .as_i64()
            .ok_or_else(|| FundingPollError::Parse("missing time".into()))?;

        Ok(PremiumIndexResponse {
            symbol: symbol.to_string(),
            last_funding_rate,
            next_funding_time,
            time,
        })
    }
}
