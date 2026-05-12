//! T1926 — V9 secrets-in-artifacts gate.
//!
//! Composes the smoke harness against fixture API keys that look
//! real (`sk-ant-V9-secretkey-12345678`,
//! `sk-V9-OpenAI-secretkey-87654321`), drives one round-trip per
//! provider, then invokes `scripts/check_no_secrets_in_llm_artifacts.sh`
//! against every artifact the smoke run could have written. The
//! gate asserts **zero substrings** of the two fixture keys land in
//! any scanned artifact.
//!
//! Why a shell script and not inline grep? The grep gate also fires
//! standalone (`bash scripts/check_no_secrets_in_llm_artifacts.sh`)
//! against CI artifacts that don't come from a Rust test run —
//! presenter screenshots, archived backtests, dev logs. Keeping the
//! pattern list in one shell script means the test and CI can never
//! drift.

use async_trait::async_trait;
use cost::{AgentRole, LlmTier};
use llm::{
    AnthropicProvider, ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmError,
    LlmProvider, MessageRole, ModelId, OpenAiProvider, ProviderKind, RecordingProvider,
};
use uuid::Uuid;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

const ANT_KEY: &str = "sk-ant-V9-secretkey-12345678";
const OAI_KEY: &str = "sk-V9-OpenAI-secretkey-87654321";

/// In-memory canned-response provider for the local-recording leg.
struct CannedLeaf(ChatResponse, ProviderKind, &'static str);
#[async_trait]
impl LlmProvider for CannedLeaf {
    fn name(&self) -> &str {
        self.2
    }
    fn provider_kind(&self) -> ProviderKind {
        self.1.clone()
    }
    async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
        Ok(self.0.clone())
    }
}

fn canned() -> ChatResponse {
    ChatResponse {
        content: vec![ContentBlock::Text("OK".to_string())],
        stop_reason: llm::StopReason::EndTurn,
        usage: llm::TokenUsage {
            tokens_in: 5,
            tokens_out: 1,
            tokens_cached_in: 0,
        },
        model: ModelId::new("claude-opus-4-7"),
        correlation_id: Uuid::nil(),
    }
}

/// Spawn an Anthropic-shaped mock that REQUIRES `x-api-key: <ANT_KEY>`
/// — proving the key crosses the wire — and returns canned `OK`.
async fn spawn_ant_mock() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/messages"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "msg_v9",
            "type": "message",
            "role": "assistant",
            "model": "claude-opus-4-7",
            "content": [{"type": "text", "text": "OK"}],
            "stop_reason": "end_turn",
            "usage": {
                "input_tokens": 5,
                "output_tokens": 1,
                "cache_read_input_tokens": 0
            }
        })))
        .mount(&s)
        .await;
    s
}

async fn spawn_oai_mock() -> MockServer {
    let s = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/chat/completions"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "id": "chatcmpl-v9",
            "object": "chat.completion",
            "model": "gpt-5",
            "choices": [{
                "index": 0,
                "message": {"role": "assistant", "content": "OK"},
                "finish_reason": "stop"
            }],
            "usage": {"prompt_tokens": 5, "completion_tokens": 1, "total_tokens": 6}
        })))
        .mount(&s)
        .await;
    s
}

#[tokio::test]
async fn t1926_no_secrets_in_artifacts() {
    let td = tempfile::tempdir().expect("tempdir");
    let db_path = td.path().join("replay-v9.db");

    // 1. Stand up two wiremock servers that the leaf providers talk to
    //    with the fixture keys. Even if the key crosses the wire,
    //    nothing inside this test ever writes the key to a persistent
    //    artifact.
    let ant_srv = spawn_ant_mock().await;
    let oai_srv = spawn_oai_mock().await;
    let ant = AnthropicProvider::with_base_url(
        ant_srv.uri(),
        ANT_KEY,
        ModelId::new("claude-opus-4-7"),
    );
    let oai = OpenAiProvider::new_with_base_url(
        oai_srv.uri(),
        OAI_KEY,
        ModelId::new("gpt-5"),
    );

    // 2. Wrap each in a RecordingProvider that writes to the same
    //    tempfile cache.
    let ant_rec = RecordingProvider::open(ant, &db_path).await.unwrap();
    let mut req = ChatRequest::new(
        ModelId::new("claude-opus-4-7"),
        LlmTier::DeepThink,
        AgentRole::Trader,
    );
    req.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("smoke-v9".into())],
    });
    ant_rec.complete(req).await.expect("ant record");
    drop(ant_rec);

    let oai_rec = RecordingProvider::open(oai, &db_path).await.unwrap();
    let mut req = ChatRequest::new(
        ModelId::new("gpt-5"),
        LlmTier::DeepThink,
        AgentRole::Trader,
    );
    req.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("smoke-v9-oai".into())],
    });
    oai_rec.complete(req).await.expect("oai record");
    drop(oai_rec);

    // Use the alternate canned leaf to seed a recording row whose
    // wire body never carried a key — covers the "in-process leaf"
    // path that bypasses HTTP entirely.
    let leaf = CannedLeaf(canned(), ProviderKind::Anthropic, "canned");
    let rec = RecordingProvider::open(leaf, &db_path).await.unwrap();
    let mut req = ChatRequest::new(
        ModelId::new("claude-opus-4-7"),
        LlmTier::QuickThink,
        AgentRole::SentimentAnalyst,
    );
    req.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("smoke-v9-canned".into())],
    });
    rec.complete(req).await.expect("canned record");
    drop(rec);

    // 3. Invoke the V9 grep script against the tempdir's artifacts.
    //    Point each artifact path at the per-test scratch so this
    //    test doesn't accidentally read the operator's live DBs.
    let script = repo_root().join("scripts/check_no_secrets_in_llm_artifacts.sh");
    assert!(
        script.exists(),
        "V9 grep script missing at {}",
        script.display()
    );
    let status = std::process::Command::new("bash")
        .arg(&script)
        .arg("--db")
        .arg(&db_path)
        .arg("--log-dir")
        .arg(td.path()) // tempdir itself — no log files inside
        .arg("--audit-db")
        .arg("/dev/null")
        .arg("--fixtures-dir")
        .arg(td.path())
        .current_dir(repo_root())
        .status()
        .expect("invoke V9 grep script");

    assert!(
        status.success(),
        "V9 grep script failed (exit code {:?}); see stderr for hits",
        status.code()
    );
}

/// Resolve the repo root from `CARGO_MANIFEST_DIR` (= `crates/llm`) by
/// going up two parents.
fn repo_root() -> std::path::PathBuf {
    let manifest =
        std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("MANIFEST_DIR"));
    manifest
        .parent()
        .and_then(std::path::Path::parent)
        .expect("repo root")
        .to_path_buf()
}
