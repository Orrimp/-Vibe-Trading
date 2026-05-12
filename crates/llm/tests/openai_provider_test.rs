//! Integration tests for `OpenAiProvider` (T1904 acceptance).
//!
//! - (a) request body has **no** `cache_control` markers when the
//!   request carries `SystemBlock::Cached` items (silent flatten).
//! - (b) canned response parses correctly.
//! - (c) 429 → 200 retry round-trips.
//! - (d) `tokens_cached_in == 0` always.

use cost::{AgentRole, LlmTier};
use llm::{
    CacheBreakpoint, ChatMessage, ChatRequest, ContentBlock, LlmProvider, MessageRole, ModelId,
    OpenAiProvider, StopReason, SystemBlock,
};
use serde_json::Value;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn cached_request() -> ChatRequest {
    let mut req = ChatRequest::new(
        ModelId::from("gpt-4o-mini"),
        LlmTier::QuickThink,
        AgentRole::Trader,
    );
    req.system = vec![
        SystemBlock::Cached("project ctx".into(), CacheBreakpoint::Ephemeral),
        SystemBlock::Cached("role ctx".into(), CacheBreakpoint::Ephemeral),
        SystemBlock::Plain("dynamic ctx".into()),
    ];
    req.messages = vec![ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("hi".into())],
    }];
    req
}

fn canned_success() -> Value {
    serde_json::json!({
        "id": "chatcmpl-1",
        "object": "chat.completion",
        "model": "gpt-4o-mini-2024-07-18",
        "choices": [{
            "index": 0,
            "message": {"role": "assistant", "content": "hi back"},
            "finish_reason": "stop"
        }],
        "usage": {"prompt_tokens": 100, "completion_tokens": 20, "total_tokens": 120}
    })
}

#[tokio::test]
async fn t1904_request_body_has_no_cache_markers() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .and(header("authorization", "Bearer test-key"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_success()))
        .mount(&server)
        .await;

    let provider =
        OpenAiProvider::new_with_base_url(server.uri(), "test-key", ModelId::from("gpt-4o-mini"));
    let _ = provider.complete(cached_request()).await.expect("ok");

    let received = server.received_requests().await.expect("received");
    let raw = String::from_utf8(received[0].body.clone()).expect("utf8");
    assert!(
        !raw.contains("cache_control"),
        "openai body must NOT contain cache_control markers: {raw}"
    );
}

#[tokio::test]
async fn t1904_canned_response_parses_with_zero_cached() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_success()))
        .mount(&server)
        .await;

    let provider =
        OpenAiProvider::new_with_base_url(server.uri(), "test-key", ModelId::from("gpt-4o-mini"));
    let resp = provider.complete(cached_request()).await.expect("ok");
    assert_eq!(resp.usage.tokens_in, 100);
    assert_eq!(resp.usage.tokens_out, 20);
    assert_eq!(resp.usage.tokens_cached_in, 0);
    assert_eq!(resp.stop_reason, StopReason::EndTurn);
}

#[tokio::test]
async fn t1904_429_then_200_retries() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(429).set_body_string("rate"))
        .up_to_n_times(1)
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_success()))
        .mount(&server)
        .await;

    let provider =
        OpenAiProvider::new_with_base_url(server.uri(), "test-key", ModelId::from("gpt-4o-mini"));
    let resp = provider.complete(cached_request()).await.expect("ok");
    assert_eq!(resp.usage.tokens_cached_in, 0, "always 0 for openai-compat");
    let received = server.received_requests().await.expect("received");
    assert_eq!(received.len(), 2);
}
