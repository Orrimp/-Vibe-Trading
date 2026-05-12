//! Integration tests for `OllamaProvider` (T1905 acceptance).
//!
//! - (a) mock-server canned response parses correctly.
//! - (b) `tokens_cached_in == 0`.
//! - (c) network failure → `LlmError::Network` immediately (no retry).
//! - (d) best-effort tool-use happy path.
//! - (e) tool-use schema-mismatch → `LlmError::InvalidResponse`.

use cost::{AgentRole, LlmTier};
use llm::{
    ChatMessage, ChatRequest, ContentBlock, LlmError, LlmProvider, MessageRole, ModelId,
    OllamaProvider, StopReason, ToolSchema,
};
use serde_json::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

fn buy_tool() -> ToolSchema {
    ToolSchema {
        name: "buy".to_string(),
        description: "Buy a given symbol".to_string(),
        input_schema: serde_json::json!({
            "type": "object",
            "properties": {
                "symbol": {"type": "string"},
                "qty": {"type": "number", "minimum": 0}
            },
            "required": ["symbol", "qty"]
        }),
    }
}

fn base_request() -> ChatRequest {
    let mut req = ChatRequest::new(
        ModelId::from("llama3:8b"),
        LlmTier::QuickThink,
        AgentRole::Trader,
    );
    req.messages = vec![ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("buy 0.5 BTC".into())],
    }];
    req
}

fn canned_text_response() -> Value {
    serde_json::json!({
        "model": "llama3:8b",
        "created_at": "2026-05-12T00:00:00Z",
        "message": {"role": "assistant", "content": "hello"},
        "done": true,
        "done_reason": "stop",
        "prompt_eval_count": 50,
        "eval_count": 12
    })
}

#[tokio::test]
async fn t1905_canned_response_parses_correctly() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(canned_text_response()))
        .mount(&server)
        .await;

    let provider = OllamaProvider::with_base_url(server.uri(), ModelId::from("llama3:8b"));
    let resp = provider.complete(base_request()).await.expect("ok");
    assert_eq!(resp.usage.tokens_in, 50);
    assert_eq!(resp.usage.tokens_out, 12);
    assert_eq!(resp.usage.tokens_cached_in, 0);
    assert_eq!(resp.stop_reason, StopReason::EndTurn);
    assert!(matches!(&resp.content[0], ContentBlock::Text(t) if t == "hello"));
}

#[tokio::test]
async fn t1905_network_failure_no_retry() {
    // Bind to a port that nothing is listening on so the connect fails
    // immediately at the transport layer.
    let provider = OllamaProvider::with_base_url(
        "http://127.0.0.1:1", // reserved port, refused
        ModelId::from("llama3:8b"),
    );
    let err = provider
        .complete(base_request())
        .await
        .expect_err("network");
    match err {
        LlmError::Network(_) => {}
        other => panic!("expected Network, got {other:?}"),
    }
}

#[tokio::test]
async fn t1905_best_effort_tool_use_happy_path() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "model": "llama3:8b",
            "message": {
                "role": "assistant",
                "content": "{\"name\": \"buy\", \"input\": {\"symbol\": \"BTC\", \"qty\": 0.5}}"
            },
            "done": true,
            "done_reason": "stop",
            "prompt_eval_count": 30,
            "eval_count": 18
        })))
        .mount(&server)
        .await;

    let mut req = base_request();
    req.tools = vec![buy_tool()];

    let provider = OllamaProvider::with_base_url(server.uri(), ModelId::from("llama3:8b"));
    let resp = provider.complete(req).await.expect("ok");
    match &resp.content[0] {
        ContentBlock::ToolUse { name, input, .. } => {
            assert_eq!(name, "buy");
            assert_eq!(input["qty"], 0.5);
        }
        other => panic!("expected ToolUse, got {other:?}"),
    }
    assert_eq!(resp.stop_reason, StopReason::ToolUse);
}

#[tokio::test]
async fn t1905_best_effort_tool_use_schema_mismatch() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "model": "llama3:8b",
            // Missing required `qty` field.
            "message": {
                "role": "assistant",
                "content": "{\"name\": \"buy\", \"input\": {\"symbol\": \"BTC\"}}"
            },
            "done": true,
            "prompt_eval_count": 1,
            "eval_count": 1
        })))
        .mount(&server)
        .await;

    let mut req = base_request();
    req.tools = vec![buy_tool()];

    let provider = OllamaProvider::with_base_url(server.uri(), ModelId::from("llama3:8b"));
    let err = provider.complete(req).await.expect_err("rejected");
    match err {
        LlmError::InvalidResponse(msg) => {
            assert!(msg.contains("schema-mismatch"));
        }
        other => panic!("expected InvalidResponse, got {other:?}"),
    }
}
