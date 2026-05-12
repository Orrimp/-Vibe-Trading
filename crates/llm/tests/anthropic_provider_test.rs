//! Integration tests for `AnthropicProvider` (T1903 acceptance).
//!
//! These tests exercise the HTTP boundary via `wiremock` — no real
//! Anthropic API calls. They cover the four T1903 acceptance items:
//!
//! - (a) the request body carries **exactly two** `cache_control:
//!   {"type": "ephemeral"}` markers when the request carries two
//!   `SystemBlock::Cached` items.
//! - (b) a canned 200 response parses into a `ChatResponse` with the
//!   expected `usage` fields.
//! - (c) a 429 → 200 retry round-trips.
//! - (d) a 401 surfaces as `LlmError::Auth`.

use cost::{AgentRole, LlmTier};
use llm::{
    AnthropicProvider, CacheBreakpoint, ChatMessage, ChatRequest, ContentBlock, LlmError,
    LlmProvider, MessageRole, ModelId, StopReason, SystemBlock,
};
use serde_json::Value;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn cached_request() -> ChatRequest {
    let mut req = ChatRequest::new(
        ModelId::from("claude-3-5-sonnet-20241022"),
        LlmTier::DeepThink,
        AgentRole::Trader,
    );
    req.system = vec![
        SystemBlock::Cached("project context".into(), CacheBreakpoint::Ephemeral),
        SystemBlock::Cached("role context".into(), CacheBreakpoint::Ephemeral),
        SystemBlock::Plain("dynamic context".into()),
    ];
    req.messages = vec![ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("hello".into())],
    }];
    req
}

fn canned_success() -> Value {
    serde_json::json!({
        "id": "msg_01",
        "type": "message",
        "role": "assistant",
        "model": "claude-3-5-sonnet-20241022",
        "content": [{"type": "text", "text": "hi"}],
        "stop_reason": "end_turn",
        "usage": {
            "input_tokens": 1000,
            "output_tokens": 200,
            "cache_read_input_tokens": 500
        }
    })
}

/// T1903 (a): request body carries exactly two `cache_control:
/// {"type": "ephemeral"}` markers.
#[tokio::test]
async fn t1903_request_body_emits_two_cache_breakpoints() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .and(header("x-api-key", "test-key"))
        .and(header("anthropic-version", "2023-06-01"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_success()))
        .mount(&server)
        .await;

    let provider =
        AnthropicProvider::with_base_url(server.uri(), "test-key", ModelId::from("default"));
    let _ = provider
        .complete(cached_request())
        .await
        .expect("call succeeds");

    let received = server.received_requests().await.expect("received");
    assert_eq!(received.len(), 1);
    let body: Value = serde_json::from_slice(&received[0].body).expect("body parses");
    let system = body["system"].as_array().expect("system array");
    assert_eq!(system.len(), 3);
    let ephemeral_count = system
        .iter()
        .filter(|b| {
            b.get("cache_control")
                .and_then(|cc| cc.get("type"))
                .and_then(|t| t.as_str())
                == Some("ephemeral")
        })
        .count();
    assert_eq!(ephemeral_count, 2);
    assert_eq!(body["stream"], Value::Bool(false));
}

/// T1903 (b): canned 200 parses into `ChatResponse` with usage mapped.
#[tokio::test]
async fn t1903_canned_200_parses_into_chat_response() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_success()))
        .mount(&server)
        .await;

    let provider =
        AnthropicProvider::with_base_url(server.uri(), "test-key", ModelId::from("default"));
    let req = cached_request();
    let correlation_id = req.correlation_id;
    let resp = provider.complete(req).await.expect("ok");
    assert_eq!(resp.usage.tokens_in, 1000);
    assert_eq!(resp.usage.tokens_out, 200);
    assert_eq!(resp.usage.tokens_cached_in, 500);
    assert_eq!(resp.stop_reason, StopReason::EndTurn);
    assert_eq!(resp.correlation_id, correlation_id);
}

/// T1903 (c): 429 → 200 retry round-trips and ultimately returns Ok.
#[tokio::test]
async fn t1903_429_then_200_retries_to_success() {
    let server = MockServer::start().await;
    // First call → 429.
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate limit"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    // Second call → 200.
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_success()))
        .mount(&server)
        .await;

    let provider =
        AnthropicProvider::with_base_url(server.uri(), "test-key", ModelId::from("default"));
    let resp = provider.complete(cached_request()).await.expect("ok");
    assert_eq!(resp.usage.tokens_in, 1000);
    let received = server.received_requests().await.expect("received");
    assert_eq!(received.len(), 2, "exactly one retry");
}

/// T1903 (d): 401 surfaces as `LlmError::Auth` (fatal, no retry).
#[tokio::test]
async fn t1903_401_surfaces_as_auth_error() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(401).set_body_string("invalid api key"))
        .mount(&server)
        .await;

    let provider =
        AnthropicProvider::with_base_url(server.uri(), "bogus-key", ModelId::from("default"));
    let err = provider
        .complete(cached_request())
        .await
        .expect_err("rejected");
    match err {
        LlmError::Auth(msg) => assert!(msg.contains("401")),
        other => panic!("expected Auth, got {other:?}"),
    }
    let received = server.received_requests().await.expect("received");
    assert_eq!(received.len(), 1, "no retry on 401");
}

/// T1903 (a, bonus): when the request carries no `SystemBlock::Cached`,
/// the body contains zero `cache_control` markers.
#[tokio::test]
async fn t1903_request_body_no_markers_when_uncached() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_success()))
        .mount(&server)
        .await;

    let mut req = cached_request();
    req.system = vec![SystemBlock::Plain("just plain".into())];
    let provider =
        AnthropicProvider::with_base_url(server.uri(), "test-key", ModelId::from("default"));
    let _ = provider.complete(req).await.expect("ok");

    let received = server.received_requests().await.expect("received");
    let raw = String::from_utf8(received[0].body.clone()).expect("utf8");
    assert!(!raw.contains("cache_control"), "no markers when not cached");
}

/// Ignore a non-cached request to keep `Request` referenced (avoid an
/// "unused import" warning when wiremock evolves).
#[allow(dead_code)]
fn _keep_request_in_scope(_r: &Request) {}
