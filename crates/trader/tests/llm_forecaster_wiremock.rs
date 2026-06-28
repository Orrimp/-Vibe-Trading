//! Integration tests for `LlmForecasterImpl` with wiremock-backed Anthropic provider.
//!
//! ## Test coverage (T-D-N(B5))
//!
//! 1. **Happy-path**: well-formed `propose_forecast` tool-use response
//!    round-trips through `LlmForecasterImpl` → `LlmForecast` with correct fields.
//!
//! 2. **Error paths**:
//!    - Malformed JSON response (free-text, no tool-use block) → `InvalidResponse`.
//!    - Unknown rating in tool payload → `InvalidResponse`.
//!    - Missing required field (`reasoning_trace`) → `InvalidResponse`.
//!    - Bad confidence range (rejected by schema) → `InvalidResponse`.
//!    - HTTP 429 rate-limit (wiremock) → `LlmForecasterError::Provider` (after retry exhaustion).
//!    - HTTP 500 provider error → `LlmForecasterError::Provider`.
//!
//! 3. **Determinism**: same `ForecastContext` produces the same `request_hash`
//!    regardless of how many times it is called; the wiremock sees identical
//!    request bodies on repeated calls.
//!
//! 4. **Temperature pin**: every request sent to the wiremock server carries
//!    `"temperature": 0.0` in the JSON body.
//!
//! 5. **Cache breakpoints**: the request body contains exactly 2 `cache_control`
//!    Ephemeral markers.
//!
//! ## No real API calls
//!
//! All tests use `wiremock::MockServer` running on localhost. `ANTHROPIC_API_KEY`
//! is not required and is NOT read in these tests. Any test that accidentally
//! reaches `api.anthropic.com` will fail on network timeout, not pass silently.
//!
//! ## Cross-references
//!
//! - `spec/v1/v3-llm-forecaster/decomp.md § T-AR-5` — determinism contract.
//! - `spec/v1/v3-llm-forecaster/decomp.md § T-AR-2` — prompt + cache contract.
//! - `spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md § D1.b` — L4 gate.

use std::sync::Arc;

use cost::LlmTier;
use rust_decimal_macros::dec;
use serde_json::Value;
use time::OffsetDateTime;
use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use trader::llm_forecaster::{
    ForecastContext, LlmForecaster, LlmForecasterError, LlmForecasterImpl, Rating,
};

// ── Test helpers ──────────────────────────────────────────────────────────────

fn make_ts(epoch_s: i64) -> Timestamp {
    Timestamp::new(OffsetDateTime::from_unix_timestamp(epoch_s).expect("valid ts"))
}

fn make_bar(symbol: &str, open_ts_s: i64) -> Bar {
    let sym = Symbol::new(symbol);
    let ts = make_ts(open_ts_s);
    Bar {
        symbol: sym,
        tf: Timeframe::OneHour,
        open_ts: ts,
        close_ts: make_ts(open_ts_s + 3600),
        open: Price::new(dec!(45000)).expect("positive price"),
        high: Price::new(dec!(45100)).expect("positive price"),
        low: Price::new(dec!(44900)).expect("positive price"),
        close: Price::new(dec!(45050)).expect("positive price"),
        volume: Quantity::new(dec!(1000)).expect("positive qty"),
        trade_count: 100,
        local_recv_ts: ts,
        venue: Venue::Binance,
    }
}

fn minimal_ctx() -> ForecastContext {
    ForecastContext::test_fixture(
        Symbol::new("BTCUSDT"),
        make_ts(1_700_000_000),
        vec![make_bar("BTCUSDT", 1_700_000_000)],
    )
}

fn make_impl(base_url: &str) -> LlmForecasterImpl {
    let provider = Arc::new(llm::AnthropicProvider::with_base_url(
        base_url,
        "test-key",
        llm::ModelId::from("claude-haiku-4-5-20251001"),
    ));
    LlmForecasterImpl::new(provider, "claude-haiku-4-5-20251001", LlmTier::QuickThink)
}

/// Build the canned Anthropic tool-use response for `propose_forecast`.
fn canned_buy_response() -> Value {
    serde_json::json!({
        "id": "msg_01",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-4-5-20251001",
        "content": [{
            "type": "tool_use",
            "id": "toolu_01",
            "name": "propose_forecast",
            "input": {
                "rating": "BUY",
                "confidence": 0.72,
                "horizon": "short",
                "reasoning_trace": "RSI(14) = 62.5 trending above 60 for 3 consecutive bars. MACD histogram positive at 0.0023 and rising. BB upper band at 45,200 not yet breached — price has room. Lesson card lc_btc_bull_001 (BTC Bull regime, +0.8% outcome) provides directional confirmation. Recent decisions show 3/5 BUY forecasts with positive outcomes. Net assessment: moderate bullish.",
                "cited_lesson_ids": ["lc_btc_bull_001"]
            }
        }],
        "stop_reason": "tool_use",
        "usage": {
            "input_tokens": 5876,
            "output_tokens": 412,
            "cache_read_input_tokens": 2000
        }
    })
}

fn canned_hold_response() -> Value {
    serde_json::json!({
        "id": "msg_02",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-4-5-20251001",
        "content": [{
            "type": "tool_use",
            "id": "toolu_02",
            "name": "propose_forecast",
            "input": {
                "rating": "HOLD",
                "confidence": 0.45,
                "horizon": "short",
                "reasoning_trace": "Mixed signals: RSI at 50.1 in neutral territory. MACD near-zero histogram with no clear direction. BB midband price action. No strong lesson card match for current regime. Recent decisions show alternating BUY/SELL signals without sustained pattern. No directional edge — hold.",
                "cited_lesson_ids": []
            }
        }],
        "stop_reason": "tool_use",
        "usage": {
            "input_tokens": 5750,
            "output_tokens": 390,
            "cache_read_input_tokens": 2000
        }
    })
}

fn canned_free_text_response() -> Value {
    // LLM emits plain text instead of calling the tool — should trigger InvalidResponse.
    serde_json::json!({
        "id": "msg_03",
        "type": "message",
        "role": "assistant",
        "model": "claude-haiku-4-5-20251001",
        "content": [{"type": "text", "text": "I think you should BUY BTC."}],
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 5000,
            "output_tokens": 20,
            "cache_read_input_tokens": 0
        }
    })
}

// ── Happy-path tests ──────────────────────────────────────────────────────────

/// T-D-N(B5) — Happy-path: BUY response round-trips to LlmForecast::Buy.
#[tokio::test]
async fn b5_happy_path_buy_response_round_trips() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("x-api-key", "test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let forecast = impl_.forecast(ctx).await.expect("forecast ok");

    assert_eq!(forecast.rating, Rating::Buy);
    assert_eq!(forecast.cited_lessons.len(), 1);
    assert_eq!(forecast.cited_lessons[0].card_id, "lc_btc_bull_001");
    assert!(
        forecast.reasoning_trace.len() >= 50,
        "reasoning_trace must be >= 50 chars"
    );
    assert_eq!(forecast.symbol, Symbol::new("BTCUSDT"));
    assert_eq!(forecast.forecaster_name, "llm_forecaster_impl");
    // reasoning_trace_sha256 is precomputed from the trace
    assert_ne!(forecast.reasoning_trace_sha256, [0u8; 32]);
}

/// HOLD response with empty cited_lesson_ids round-trips correctly.
#[tokio::test]
async fn b5_happy_path_hold_response_empty_cited_lessons() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_hold_response()))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let forecast = impl_.forecast(ctx).await.expect("forecast ok");

    assert_eq!(forecast.rating, Rating::Hold);
    assert!(
        forecast.cited_lessons.is_empty(),
        "empty cited_lesson_ids must produce empty Vec"
    );
}

/// All 5 rating enum values round-trip correctly from the wire.
#[tokio::test]
async fn b5_all_rating_values_round_trip() {
    for (rating_str, expected_rating) in [
        ("STRONG_BUY", Rating::StrongBuy),
        ("BUY", Rating::Buy),
        ("HOLD", Rating::Hold),
        ("SELL", Rating::Sell),
        ("STRONG_SELL", Rating::StrongSell),
    ] {
        let server = MockServer::start().await;
        let mut response = canned_buy_response();
        response["content"][0]["input"]["rating"] = Value::String(rating_str.to_string());
        Mock::given(method("POST"))
            .and(path("/messages"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response))
            .mount(&server)
            .await;

        let impl_ = make_impl(&server.uri());
        let ctx = minimal_ctx();
        let forecast = impl_.forecast(ctx).await.expect("forecast ok");
        assert_eq!(
            forecast.rating, expected_rating,
            "rating '{rating_str}' must decode to {expected_rating:?}"
        );
    }
}

// ── Temperature pin test ──────────────────────────────────────────────────────

/// T-D-N(B5) — Temperature pin: request body always carries "temperature": 0.0.
#[tokio::test]
async fn b5_request_body_pins_temperature_zero() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let _ = impl_.forecast(ctx).await.expect("forecast ok");

    let received = server.received_requests().await.expect("received");
    assert_eq!(received.len(), 1, "exactly one request expected");
    let body: Value = serde_json::from_slice(&received[0].body).expect("body parses");
    assert_eq!(
        body["temperature"],
        Value::from(0.0_f64),
        "temperature must be 0.0 per T-D-N(B4)"
    );
}

// ── Cache breakpoints test ────────────────────────────────────────────────────

/// T-D-N(B5) — Cache breakpoints: request body carries exactly 2 ephemeral markers.
#[tokio::test]
async fn b5_request_body_emits_two_cache_breakpoints() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let _ = impl_.forecast(ctx).await.expect("forecast ok");

    let received = server.received_requests().await.expect("received");
    let body: Value = serde_json::from_slice(&received[0].body).expect("body parses");

    // Count Ephemeral cache_control markers in the system array.
    let system = body["system"].as_array().expect("system must be array");
    let ephemeral_count = system
        .iter()
        .filter(|block| {
            block
                .get("cache_control")
                .and_then(|cc| cc.get("type"))
                .and_then(|t| t.as_str())
                == Some("ephemeral")
        })
        .count();
    assert_eq!(
        ephemeral_count, 2,
        "request must carry exactly 2 ephemeral cache markers per T-D-N(B2)"
    );
}

// ── Determinism test ──────────────────────────────────────────────────────────

/// T-D-N(B5) — Determinism: two identical ForecastContexts produce identical request bodies.
#[tokio::test]
async fn b5_identical_contexts_produce_identical_request_bodies() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .expect(2) // expect exactly 2 calls
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());

    // Two calls with identical contexts (same symbol, same ts, same bars).
    let ctx1 = ForecastContext::test_fixture(
        Symbol::new("BTCUSDT"),
        make_ts(1_700_000_000),
        vec![make_bar("BTCUSDT", 1_700_000_000)],
    );
    let ctx2 = ForecastContext::test_fixture(
        Symbol::new("BTCUSDT"),
        make_ts(1_700_000_000),
        vec![make_bar("BTCUSDT", 1_700_000_000)],
    );

    let _ = impl_.forecast(ctx1).await.expect("first call ok");
    let _ = impl_.forecast(ctx2).await.expect("second call ok");

    let requests = server.received_requests().await.expect("received");
    assert_eq!(requests.len(), 2);

    // Parse both request bodies and compare the non-correlation-id fields.
    let body1: Value = serde_json::from_slice(&requests[0].body).expect("body1 parses");
    let body2: Value = serde_json::from_slice(&requests[1].body).expect("body2 parses");

    // The system prompt (project + role blocks) must be identical.
    assert_eq!(
        body1["system"], body2["system"],
        "identical contexts must produce identical system prompts"
    );

    // The tool definition must be identical.
    assert_eq!(
        body1["tools"], body2["tools"],
        "tool schema must be identical across calls"
    );

    // Temperature must be identical (both 0.0).
    assert_eq!(body1["temperature"], body2["temperature"]);
}

// ── Error path tests ──────────────────────────────────────────────────────────

/// T-D-N(B5) — Free-text response (no tool-use block) → InvalidResponse.
#[tokio::test]
async fn b5_free_text_response_produces_invalid_response_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_free_text_response()))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let err = impl_.forecast(ctx).await.expect_err("should fail");
    assert!(
        matches!(err, LlmForecasterError::InvalidResponse { .. }),
        "free-text response must produce InvalidResponse, got: {:?}",
        err
    );
}

/// T-D-N(B5) — Reasoning trace shorter than 50 chars in tool payload → InvalidResponse.
#[tokio::test]
async fn b5_short_reasoning_trace_produces_invalid_response_error() {
    let server = MockServer::start().await;
    let mut response = canned_buy_response();
    response["content"][0]["input"]["reasoning_trace"] = Value::String("Too short.".to_string()); // < 50 chars
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let err = impl_.forecast(ctx).await.expect_err("should fail");
    assert!(
        matches!(err, LlmForecasterError::InvalidResponse { .. }),
        "short reasoning_trace must produce InvalidResponse via schema validation"
    );
}

/// T-D-N(B5) — Unknown rating value → InvalidResponse.
#[tokio::test]
async fn b5_unknown_rating_produces_invalid_response_error() {
    let server = MockServer::start().await;
    let mut response = canned_buy_response();
    response["content"][0]["input"]["rating"] = Value::String("SUPER_BULLISH".to_string());
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let err = impl_.forecast(ctx).await.expect_err("should fail");
    assert!(
        matches!(err, LlmForecasterError::InvalidResponse { .. }),
        "unknown rating must produce InvalidResponse"
    );
}

/// T-D-N(B5) — Confidence out of range (> 1.0) → InvalidResponse.
#[tokio::test]
async fn b5_confidence_out_of_range_produces_invalid_response_error() {
    let server = MockServer::start().await;
    let mut response = canned_buy_response();
    response["content"][0]["input"]["confidence"] = Value::from(1.5_f64); // > 1.0
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let err = impl_.forecast(ctx).await.expect_err("should fail");
    assert!(
        matches!(err, LlmForecasterError::InvalidResponse { .. }),
        "confidence > 1.0 must produce InvalidResponse"
    );
}

/// T-D-N(B5) — Missing required field `reasoning_trace` → InvalidResponse.
#[tokio::test]
async fn b5_missing_reasoning_trace_produces_invalid_response_error() {
    let server = MockServer::start().await;
    let mut response = canned_buy_response();
    // Remove the reasoning_trace field from the tool input.
    response["content"][0]["input"]
        .as_object_mut()
        .unwrap()
        .remove("reasoning_trace");
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let err = impl_.forecast(ctx).await.expect_err("should fail");
    assert!(
        matches!(err, LlmForecasterError::InvalidResponse { .. }),
        "missing reasoning_trace must produce InvalidResponse"
    );
}

/// T-D-N(B5) — HTTP 500 server error surfaces as LlmForecasterError::Provider.
#[tokio::test]
async fn b5_http_500_surfaces_as_provider_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal server error"))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let err = impl_.forecast(ctx).await.expect_err("should fail on 500");
    // 500 surfaces as LlmError::Provider → LlmForecasterError::Provider
    assert!(
        matches!(err, LlmForecasterError::Provider(_)),
        "HTTP 500 must surface as Provider error, got: {:?}",
        err
    );
}

/// T-D-N(B5) — HTTP 401 auth failure surfaces as LlmForecasterError::Provider
/// (wrapping LlmError::Auth).
#[tokio::test]
async fn b5_http_401_surfaces_as_provider_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string("invalid api key"))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let err = impl_.forecast(ctx).await.expect_err("should fail on 401");
    // 401 surfaces as LlmError::Auth → LlmForecasterError::Provider(LlmError::Auth)
    assert!(
        matches!(err, LlmForecasterError::Provider(_)),
        "HTTP 401 must surface as Provider error wrapping Auth, got: {:?}",
        err
    );
}

/// T-D-N(B5) — Rate-limit (429) retries and then surfaces as Provider error.
///
/// wiremock is configured to always return 429, exhausting the retry policy.
#[tokio::test]
async fn b5_http_429_exhausts_retries_and_surfaces_as_provider_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limit exceeded"))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let err = impl_
        .forecast(ctx)
        .await
        .expect_err("should fail after retries");
    // After retry exhaustion the AnthropicProvider surfaces LlmError::RateLimited
    // which maps to LlmForecasterError::Provider.
    assert!(
        matches!(err, LlmForecasterError::Provider(_)),
        "429 after retry exhaustion must surface as Provider error, got: {:?}",
        err
    );
    // Verify that more than 1 request was made (retry happened).
    let received = server.received_requests().await.expect("received");
    assert!(
        received.len() > 1,
        "at least 1 retry should have occurred; got {} requests",
        received.len()
    );
}

// ── Proposed-forecast tool name test ─────────────────────────────────────────

/// The request body carries exactly 1 tool with name "propose_forecast".
#[tokio::test]
async fn b5_request_carries_propose_forecast_tool() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_buy_response()))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let _ = impl_.forecast(ctx).await.expect("forecast ok");

    let received = server.received_requests().await.expect("received");
    let body: Value = serde_json::from_slice(&received[0].body).expect("body parses");
    let tools = body["tools"].as_array().expect("tools must be array");
    assert_eq!(tools.len(), 1, "exactly 1 tool");
    assert_eq!(tools[0]["name"], "propose_forecast");
    // Tool must have input_schema with the 5-tier rating enum.
    let rating_enum = &tools[0]["input_schema"]["properties"]["rating"]["enum"];
    let rating_values: Vec<&str> = rating_enum
        .as_array()
        .expect("rating enum must be array")
        .iter()
        .filter_map(|v| v.as_str())
        .collect();
    assert_eq!(
        rating_values,
        &["STRONG_BUY", "BUY", "HOLD", "SELL", "STRONG_SELL"]
    );
}

// ── LlmForecaster::name() test ────────────────────────────────────────────────

/// LlmForecasterImpl::name() returns "llm_forecaster_impl".
#[test]
fn impl_name_returns_expected_string() {
    let provider = Arc::new(llm::AnthropicProvider::with_base_url(
        "http://localhost:1",
        "test-key",
        llm::ModelId::from("claude-haiku-4-5-20251001"),
    ));
    let impl_ = LlmForecasterImpl::new(provider, "claude-haiku-4-5-20251001", LlmTier::QuickThink);
    assert_eq!(impl_.name(), "llm_forecaster_impl");
}

// ── Schema validation unit tests (via the wiremock path) ──────────────────────

/// T-D-N(B5): schema validates a well-formed payload — via round-trip through wiremock.
#[tokio::test]
async fn b5_schema_validates_well_formed_strong_sell() {
    let server = MockServer::start().await;
    let mut response = canned_buy_response();
    response["content"][0]["input"]["rating"] = Value::String("STRONG_SELL".to_string());
    response["content"][0]["input"]["confidence"] = Value::from(0.90_f64);
    response["content"][0]["input"]["reasoning_trace"] = Value::String(
        "RSI(14) = 28.3, deeply oversold with downward momentum. MACD histogram strongly negative at -0.0089. BB lower band broken at 44,600. Lesson card lc_bear_001 (BTC Bear regime, -1.5% outcome) confirms bearish setup. High-conviction sell.".to_string(),
    );
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(response))
        .mount(&server)
        .await;

    let impl_ = make_impl(&server.uri());
    let ctx = minimal_ctx();
    let forecast = impl_.forecast(ctx).await.expect("forecast ok");
    assert_eq!(forecast.rating, Rating::StrongSell);
}
