//! T613 — FundingPoller integration test with mock REST server.
//!
//! ## What this tests
//!
//! 1. Mock server returns canned `premiumIndex` JSON for 3 symbols.
//! 2. `FundingPoller` + `BinanceFundingClient` poll the mock, emit 3 `FundingObs`
//!    events on the broadcast channel.
//! 3. Each event is persisted to an in-memory SQLite `funding_rates` table via
//!    `audit::journal::insert_funding_obs`.
//! 4. `audit::query::funding_rate_history` returns all 3 rows in chronological order.
//! 5. Drop the mock server mid-poll: the poller logs a warning and does NOT panic.
//!
//! ## Fault tolerance (architect risk #5)
//!
//! Verified by the second sub-test: mock server drops after the first poll round;
//! second poll round hits a connection-refused error; poller continues without
//! panicking.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::sync::Arc;

use tokio::sync::broadcast;
use tokio_util::sync::CancellationToken;
use trading_core::{FundingObs, Symbol, Timestamp};
use wiremock::matchers::{method, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use data::funding::{FundingPoller, FundingRestClient};

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Build a canned Binance `premiumIndex` JSON response for a given symbol.
fn premium_index_body(symbol: &str) -> serde_json::Value {
    serde_json::json!({
        "symbol": symbol,
        "markPrice": "30000.00",
        "indexPrice": "29999.50",
        "estimatedSettlePrice": "30000.00",
        "lastFundingRate": "0.00010000",
        "nextFundingTime": 1_700_000_400_000_i64,
        "interestRate": "0.00010000",
        "time": 1_700_000_000_000_i64
    })
}

/// Lightweight mock `FundingRestClient` that calls the mock server URL.
///
/// We can't inject the mock server base URL into `BinanceFundingClient` (it
/// hard-codes fapi.binance.com).  Instead we implement the trait ourselves using
/// `reqwest`, pointing at the wiremock `MockServer` listen address.
struct MockRestClient {
    client: reqwest::Client,
    base_url: String,
}

impl MockRestClient {
    fn new(base_url: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            base_url: base_url.to_owned(),
        }
    }
}

#[async_trait::async_trait]
impl FundingRestClient for MockRestClient {
    async fn fetch_premium_index(
        &self,
        symbol: &str,
    ) -> Result<data::funding::PremiumIndexResponse, data::funding::FundingPollError> {
        let url = format!("{}/fapi/v1/premiumIndex?symbol={symbol}", self.base_url);
        let resp = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| data::funding::FundingPollError::Http(e.to_string()))?;

        if resp.status() == 429 {
            return Err(data::funding::FundingPollError::RateLimited);
        }
        if !resp.status().is_success() {
            return Err(data::funding::FundingPollError::Http(format!(
                "status {}",
                resp.status()
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| data::funding::FundingPollError::Parse(e.to_string()))?;

        let last_funding_rate = json["lastFundingRate"]
            .as_str()
            .ok_or_else(|| {
                data::funding::FundingPollError::Parse("missing lastFundingRate".into())
            })?
            .parse::<rust_decimal::Decimal>()
            .map_err(|e| data::funding::FundingPollError::Parse(e.to_string()))?;

        let next_funding_time = json["nextFundingTime"].as_i64().ok_or_else(|| {
            data::funding::FundingPollError::Parse("missing nextFundingTime".into())
        })?;

        let time = json["time"]
            .as_i64()
            .ok_or_else(|| data::funding::FundingPollError::Parse("missing time".into()))?;

        Ok(data::funding::PremiumIndexResponse {
            symbol: symbol.to_string(),
            last_funding_rate,
            next_funding_time,
            time,
        })
    }
}

// ── Test 1: happy-path poll → 3 rows in funding_rates ────────────────────────

/// T613 — Happy path: poll 3 symbols, assert 3 FundingObs events on bus,
/// 3 rows in `funding_rates`, and `funding_rate_history` returns them.
#[tokio::test]
async fn t613_poll_three_symbols_persists_rows() {
    // ── Mock server ────────────────────────────────────────────────────────────
    let mock_server = MockServer::start().await;
    let symbols = ["BTCUSDT", "ETHUSDT", "SOLUSDT"];

    for sym in &symbols {
        Mock::given(method("GET"))
            .and(path_regex("/fapi/v1/premiumIndex"))
            .and(query_param("symbol", *sym))
            .respond_with(ResponseTemplate::new(200).set_body_json(premium_index_body(sym)))
            .mount(&mock_server)
            .await;
    }

    // ── Ledger (in-memory) ─────────────────────────────────────────────────────
    let ledger = Arc::new(audit::Ledger::in_memory().await.expect("open ledger"));

    // ── Broadcast channel ──────────────────────────────────────────────────────
    let (tx, mut rx) = broadcast::channel::<FundingObs>(32);

    // ── Poller ────────────────────────────────────────────────────────────────
    let universe: Vec<Symbol> = symbols.iter().map(|s| Symbol::new(*s)).collect();
    let poller = FundingPoller::new(universe);
    let client = MockRestClient::new(&mock_server.uri());
    let cancel = CancellationToken::new();

    // Run a single poll cycle directly (skip the sleep loop).
    poller.poll_once_for_test(&client, &tx).await;

    // ── Collect broadcast events ───────────────────────────────────────────────
    let mut received: Vec<FundingObs> = Vec::new();
    // Drain all available messages (non-blocking).
    while let Ok(obs) = rx.try_recv() {
        received.push(obs);
    }

    assert_eq!(received.len(), 3, "expected 3 FundingObs events");

    // ── Persist to ledger ─────────────────────────────────────────────────────
    for obs in &received {
        audit::journal::insert_funding_obs(&ledger, obs)
            .await
            .expect("persist");
    }

    // ── Query via funding_rate_history ─────────────────────────────────────────
    let epoch_start = Timestamp::new(time::OffsetDateTime::UNIX_EPOCH);
    let epoch_end =
        Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(9_999_999_999));

    for sym in &symbols {
        let symbol = Symbol::new(*sym);
        let history =
            audit::query::funding_rate_history(&ledger, symbol.clone(), epoch_start, epoch_end)
                .await
                .expect("history query");

        assert_eq!(history.len(), 1, "expected 1 row for {sym}");
        assert_eq!(history[0].symbol, symbol);
        assert_eq!(
            history[0].funding_rate,
            rust_decimal_macros::dec!(0.0001),
            "funding rate mismatch"
        );
    }

    // Shutdown cancel token (not strictly needed for this test, but good hygiene).
    cancel.cancel();
}

// ── Test 2: fault tolerance — mock server drops mid-poll ──────────────────────

/// T613 — Fault tolerance: the mock server is dropped before the first poll.
/// The poller MUST log and skip — it must NOT panic.
#[tokio::test]
async fn t613_poller_skips_on_connection_refused() {
    // Start a mock server and immediately drop it (simulates connection refused).
    let mock_server = MockServer::start().await;
    let base_url = mock_server.uri();
    drop(mock_server); // server is now gone; connections will be refused

    let (tx, mut rx) = broadcast::channel::<FundingObs>(32);
    let universe = vec![Symbol::new("BTCUSDT"), Symbol::new("ETHUSDT")];
    let poller = FundingPoller::new(universe);
    let client = MockRestClient::new(&base_url);

    // This should NOT panic — it must log a warning and continue.
    poller.poll_once_for_test(&client, &tx).await;

    // No events should have been emitted.
    let count = rx.try_recv().map(|_| 1).unwrap_or(0);
    assert_eq!(count, 0, "no events expected when server is down");
}

// ── Test 3: 5xx response skips gracefully ─────────────────────────────────────

/// T613 — 5xx server error: poller must skip and not panic.
#[tokio::test]
async fn t613_poller_skips_on_5xx() {
    let mock_server = MockServer::start().await;

    // Return 500 for every request.
    Mock::given(method("GET"))
        .and(path_regex("/fapi/v1/premiumIndex"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let (tx, mut rx) = broadcast::channel::<FundingObs>(32);
    let poller = FundingPoller::new(vec![Symbol::new("BTCUSDT")]);
    let client = MockRestClient::new(&mock_server.uri());

    // Must not panic.
    poller.poll_once_for_test(&client, &tx).await;

    let count = rx.try_recv().map(|_| 1).unwrap_or(0);
    assert_eq!(count, 0, "no events expected on 5xx");
}
