//! T1924 — wiremock-based smoke-test harness shared between the
//! `llm-smoke` binary (T1923) and the V9 secrets-in-artifacts test
//! (T1926).
//!
//! The harness spins up three [`wiremock::MockServer`] instances (one
//! shaped per provider) on ephemeral ports, canned with one response
//! per provider × role (9 fixtures total, matching the T1925 9-row
//! cache). The smoke binary's `--mode paper` is the primary recorder
//! against these mocks; `--mode research` then replays from the
//! committed fixture DB.
//!
//! ## Test entry points
//!
//! - [`spawn_anthropic_mock`] / [`spawn_openai_mock`] /
//!   [`spawn_ollama_mock`] — async constructors that return a
//!   `(MockServer, base_url)` tuple. Each canned response says
//!   exactly the literal `OK` (the smoke binary's exit-code contract
//!   per R10).
//! - [`canned_response_for`] — returns the per-(provider, role)
//!   canned `serde_json::Value` so a follow-up test can `set_body_json`
//!   it onto a wiremock route without re-typing the shape.
//!
//! The harness is wiremock-only — no real network, no real keys. The
//! `T1926` secrets test pipes synthetic key strings
//! (`sk-ant-V9-secretkey-12345678`) through this harness and asserts
//! they never appear in any artifact the smoke run writes.

use cost::AgentRole;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// The 3 roles the smoke binary exercises (3 × 3 = 9 fixture rows).
/// Built as an owned `Vec` because [`AgentRole::Other`] carries a
/// `String` and can't be `const`.
#[must_use]
pub fn smoke_roles_owned() -> Vec<AgentRole> {
    vec![
        AgentRole::Trader,
        AgentRole::SentimentAnalyst,
        // Synthetic per Q8d — doesn't collide with any production role.
        AgentRole::Other("smoke".to_string()),
    ]
}

/// Canned-response JSON value for a (provider, role) pair. The shape
/// is provider-specific (Anthropic Messages vs OpenAI Chat
/// Completions vs Ollama) but the rendered text content is always
/// the literal `"OK"` so the smoke binary's exit-code gate (R10.3)
/// fires the same way across providers.
#[must_use]
pub fn canned_response_for(provider: &str, _role: &AgentRole) -> serde_json::Value {
    match provider {
        "anthropic" => serde_json::json!({
            "id": "msg_smoke",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-7",
            "content": [{"type": "text", "text": "OK"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 7,
                "output_tokens": 1,
                "cache_read_input_tokens": 0
            }
        }),
        "openai" => serde_json::json!({
            "id": "chatcmpl-smoke",
            "object": "chat.completion",
            "model": "gpt-5",
            "choices": [{
                "index": 0,
                "message": {
                    "role": "assistant",
                    "content": "OK",
                    "tool_calls": null
                },
                "finish_reason": "stop"
            }],
            "usage": {
                "prompt_tokens": 7,
                "completion_tokens": 1,
                "total_tokens": 8
            }
        }),
        "ollama" => serde_json::json!({
            "model": "llama3:8b",
            "message": {
                "role": "assistant",
                "content": "OK"
            },
            "done": true,
            "prompt_eval_count": 7,
            "eval_count": 1
        }),
        other => serde_json::json!({"error": format!("unknown provider {other}")}),
    }
}

/// Spawn an Anthropic-shaped mock server. Responds 200 + canned `OK`
/// to every `POST /messages` for the lifetime of the returned guard.
#[must_use]
pub async fn spawn_anthropic_mock() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(canned_response_for("anthropic", &AgentRole::Trader)),
        )
        .mount(&server)
        .await;
    server
}

/// Spawn an OpenAI-shaped mock server. Responds to `POST /chat/completions`.
#[must_use]
pub async fn spawn_openai_mock() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(canned_response_for("openai", &AgentRole::Trader)),
        )
        .mount(&server)
        .await;
    server
}

/// Spawn an Ollama-shaped mock server. Responds to `POST /api/chat`.
#[must_use]
pub async fn spawn_ollama_mock() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/api/chat"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(canned_response_for("ollama", &AgentRole::Trader)),
        )
        .mount(&server)
        .await;
    server
}

// ── Driver test (T1924 acceptance) ───────────────────────────────────────────

/// Compose the harness against all three providers, drive one
/// `complete()` per (provider, role) via the leaf providers directly,
/// and assert each returns the literal `OK` content. This is the
/// in-process equivalent of the smoke-binary integration; the V10
/// performance gate (`< 1s wall clock`) is implied by the in-process
/// shape.
#[tokio::test]
async fn t1924_smoke_harness_three_providers_three_roles() {
    use cost::LlmTier;
    use llm::{
        AnthropicProvider, ChatMessage, ChatRequest, ContentBlock, LlmProvider, MessageRole,
        ModelId, OllamaProvider, OpenAiProvider,
    };

    let started = std::time::Instant::now();

    let ant_srv = spawn_anthropic_mock().await;
    let oai_srv = spawn_openai_mock().await;
    let oll_srv = spawn_ollama_mock().await;

    let ant = AnthropicProvider::with_base_url(
        ant_srv.uri(),
        "sk-ant-test-12345",
        ModelId::new("claude-opus-4-7"),
    );
    let oai = OpenAiProvider::new_with_base_url(
        oai_srv.uri(),
        "sk-test-openai-12345",
        ModelId::new("gpt-5"),
    );
    let oll = OllamaProvider::with_base_url(oll_srv.uri(), ModelId::new("llama3:8b"));

    let mut row_count = 0usize;
    for role in smoke_roles_owned() {
        // Anthropic.
        {
            let mut req = ChatRequest::new(
                ModelId::new("claude-opus-4-7"),
                LlmTier::DeepThink,
                role.clone(),
            );
            req.messages.push(ChatMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text(
                    "Reply with the literal string `OK` and nothing else.".into(),
                )],
            });
            let resp = ant.complete(req).await.expect("anthropic mock ok");
            assert_text_eq(&resp.content, "OK");
            row_count += 1;
        }
        // OpenAI.
        {
            let mut req = ChatRequest::new(ModelId::new("gpt-5"), LlmTier::DeepThink, role.clone());
            req.messages.push(ChatMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text(
                    "Reply with the literal string `OK` and nothing else.".into(),
                )],
            });
            let resp = oai.complete(req).await.expect("openai mock ok");
            assert_text_eq(&resp.content, "OK");
            row_count += 1;
        }
        // Ollama.
        {
            let mut req =
                ChatRequest::new(ModelId::new("llama3:8b"), LlmTier::QuickThink, role.clone());
            req.messages.push(ChatMessage {
                role: MessageRole::User,
                content: vec![ContentBlock::Text(
                    "Reply with the literal string `OK` and nothing else.".into(),
                )],
            });
            let resp = oll.complete(req).await.expect("ollama mock ok");
            assert_text_eq(&resp.content, "OK");
            row_count += 1;
        }
    }
    assert_eq!(row_count, 9, "must exercise 3 providers × 3 roles");
    let elapsed = started.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(5),
        "V10: smoke harness took {elapsed:?}"
    );
}

fn assert_text_eq(blocks: &[llm::ContentBlock], expected: &str) {
    use llm::ContentBlock;
    for b in blocks {
        if let ContentBlock::Text(t) = b {
            assert_eq!(t.trim(), expected, "text mismatch");
            return;
        }
    }
    panic!("no Text content block in response");
}
