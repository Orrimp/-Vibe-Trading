//! T1927 — Integration test for the M6 record/replay round-trip.
//!
//! Three phases per the brief:
//!
//! 1. **Record** — `RecordingProvider<MockProvider>` against wiremock
//!    writes one row into a tempfile cache.
//! 2. **Replay** — `ReplayProvider` reads the same hash and returns
//!    byte-identical `ChatResponse` content (V7 replay determinism).
//! 3. **Miss** — `ReplayProvider` against an un-cached request panics
//!    with the structured `LlmError::ReplayMiss { hash, provider,
//!    model }` (D2 strict-only).

use async_trait::async_trait;
use cost::{AgentRole, LlmTier};
use llm::{
    ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmError, LlmProvider, MessageRole,
    ModelId, ProviderKind, RecordingProvider, ReplayProvider, StopReason, TokenUsage,
};
use uuid::Uuid;

/// In-memory canned-response provider — the test's "mock leaf".
struct MockLeaf {
    response: ChatResponse,
}

#[async_trait]
impl LlmProvider for MockLeaf {
    fn name(&self) -> &str {
        "mock-anthropic"
    }
    fn provider_kind(&self) -> ProviderKind {
        ProviderKind::Anthropic
    }
    async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
        Ok(self.response.clone())
    }
}

fn sample_request(seed: &str) -> ChatRequest {
    let mut r = ChatRequest::new(
        ModelId::new("claude-opus-4-7"),
        LlmTier::DeepThink,
        AgentRole::Trader,
    );
    r.max_tokens = 256;
    r.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text(format!("smoke prompt {seed}"))],
    });
    r
}

fn canned_response() -> ChatResponse {
    ChatResponse {
        content: vec![ContentBlock::Text("OK".to_string())],
        stop_reason: StopReason::EndTurn,
        usage: TokenUsage {
            tokens_in: 7,
            tokens_out: 1,
            tokens_cached_in: 0,
        },
        model: ModelId::new("claude-opus-4-7"),
        correlation_id: Uuid::nil(),
    }
}

/// T1927 — phase 1 + phase 2: record then replay; bytes match.
#[tokio::test]
async fn t1927_record_then_replay_byte_identical() {
    let td = tempfile::tempdir().expect("tempdir");
    let db_path = td.path().join("replay.db");

    let canned = canned_response();
    let rec = RecordingProvider::open(
        MockLeaf {
            response: canned.clone(),
        },
        &db_path,
    )
    .await
    .expect("open recording");

    let req = sample_request("phase-1");
    let recorded = rec.complete(req.clone()).await.expect("record");
    assert_eq!(recorded, canned, "recording-side response equals canned");

    // Drop the recording handle so the WAL flushes — the
    // `RecordingProvider`'s pool stays alive as long as `rec`
    // is in scope, which is fine for SQLite WAL readers.
    drop(rec);

    let replay = ReplayProvider::open(&db_path).await.expect("open replay");
    let replayed = replay.complete(req.clone()).await.expect("replay hit");
    assert_eq!(replayed, canned, "V7: replay must be byte-identical");
}

/// T1927 — phase 3: miss → structured `LlmError::ReplayMiss`.
#[tokio::test]
async fn t1927_strict_miss_returns_structured_error() {
    let td = tempfile::tempdir().expect("tempdir");
    let db_path = td.path().join("replay.db");

    // Seed the schema via a one-shot record so ReplayProvider opens
    // cleanly (an empty file isn't a valid SQLite DB).
    let rec = RecordingProvider::open(
        MockLeaf {
            response: canned_response(),
        },
        &db_path,
    )
    .await
    .expect("open recording");
    rec.complete(sample_request("seeded"))
        .await
        .expect("seed row");
    drop(rec);

    let replay = ReplayProvider::open(&db_path).await.expect("open replay");
    let miss_req = sample_request("never-recorded");
    let err = replay.complete(miss_req.clone()).await.expect_err("miss");

    match err {
        LlmError::ReplayMiss {
            hash,
            provider,
            model,
        } => {
            assert_eq!(hash.len(), 64, "SHA-256 hex must be 64 chars");
            assert!(
                matches!(provider, ProviderKind::Other(ref s) if s == "replay"),
                "ReplayProvider advertises Other(\"replay\"); got {provider:?}"
            );
            assert_eq!(model, "claude-opus-4-7");
        }
        other => panic!("expected ReplayMiss, got {other:?}"),
    }
}

/// T1925 acceptance gate — open the committed fixture, assert
/// `SELECT COUNT(*) = 9` and that each row's `response_json` parses
/// as a `ChatResponse`.
#[tokio::test]
async fn t1925_fixture_cache_has_nine_rows() {
    // The fixture path is repo-relative; `CARGO_MANIFEST_DIR` is
    // the crate root.
    let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    let fixture_path = std::path::PathBuf::from(manifest).join("fixtures/replay-v1.db");
    assert!(
        fixture_path.exists(),
        "fixture not committed at {}",
        fixture_path.display()
    );

    let replay = ReplayProvider::open(&fixture_path)
        .await
        .expect("open committed fixture");
    drop(replay); // we just wanted the schema-version gate to pass

    // Direct SQL probe — bypass the LlmProvider surface so we can
    // SELECT COUNT(*) without crafting a hashed request.
    use sqlx::sqlite::SqliteConnectOptions;
    use sqlx::SqlitePool;
    let opts = SqliteConnectOptions::new()
        .filename(&fixture_path)
        .read_only(true);
    let pool = SqlitePool::connect_with(opts)
        .await
        .expect("open fixture pool");
    let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM llm_replay")
        .fetch_one(&pool)
        .await
        .expect("count rows");
    assert_eq!(count, 9, "fixture must hold 9 rows (3 providers × 3 roles)");

    // Each response_json parses cleanly as a ChatResponse.
    let rows: Vec<(String,)> = sqlx::query_as("SELECT response_json FROM llm_replay")
        .fetch_all(&pool)
        .await
        .expect("fetch response_json");
    for (json,) in &rows {
        let _: ChatResponse =
            serde_json::from_str(json).expect("each row response_json parses as ChatResponse");
    }
}
