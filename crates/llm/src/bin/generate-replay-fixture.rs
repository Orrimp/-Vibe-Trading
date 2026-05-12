//! T1925 — One-shot generator for the synthetic 9-row replay fixture.
//!
//! Captures one canned response per (provider, role) into
//! `crates/llm/fixtures/replay-v1.db`. Runs against the wiremock-shaped
//! canned responses in `tests/smoke_harness.rs` (in-process — no
//! network) so the synthetic fixture is deterministic across CI runs.
//!
//! Re-running this binary is idempotent: each (provider, role) request
//! body hashes to the same SHA-256, so the `INSERT OR REPLACE` in
//! [`llm::RecordingProvider`] overwrites the same 9 rows in place. The
//! committed `replay-v1.db` is the only output; any sidecars (WAL,
//! SHM) are cleaned up after run.
//!
//! Replace this file's output with operator-environment real-API
//! captures once T1945 lands (the runbook at
//! `spec/runbooks/llm-replay.md` documents the procedure).
//!
//! Invocation: `cargo run --bin generate-replay-fixture`.

use std::path::PathBuf;

use async_trait::async_trait;
use cost::{AgentRole, LlmTier};
use llm::{
    ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmError, LlmProvider, MessageRole,
    ModelId, ProviderKind, RecordingProvider, StopReason, TokenUsage,
};
use uuid::Uuid;

/// Hand-authored canned response with provider-realistic token counts
/// (Q8d). Content is always `"OK"` — the test assertions and the
/// fixture both fail-closed on any drift from that literal.
fn canned_for(provider: &ProviderKind, model: &ModelId) -> ChatResponse {
    let (tokens_in, tokens_out, tokens_cached_in) = match provider {
        ProviderKind::Anthropic => (12, 1, 0),
        ProviderKind::OpenAi => (10, 1, 0),
        ProviderKind::Other(_) => (8, 1, 0),
        _ => (8, 1, 0),
    };
    ChatResponse {
        content: vec![ContentBlock::Text("OK".to_string())],
        stop_reason: StopReason::EndTurn,
        usage: TokenUsage {
            tokens_in,
            tokens_out,
            tokens_cached_in,
        },
        model: model.clone(),
        correlation_id: Uuid::nil(),
    }
}

struct CannedLeaf {
    response: ChatResponse,
    kind: ProviderKind,
    name: &'static str,
}
#[async_trait]
impl LlmProvider for CannedLeaf {
    fn name(&self) -> &str {
        self.name
    }
    fn provider_kind(&self) -> ProviderKind {
        self.kind.clone()
    }
    async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
        Ok(self.response.clone())
    }
}

fn smoke_prompt(role: &AgentRole) -> ChatRequest {
    // The fixture is keyed by canonical request JSON. Each role gets
    // a distinct user-message string so the 9 hashes are pairwise
    // unique. The model is held constant per provider — varying it
    // would inflate the fixture beyond the 9-row contract.
    let role_tag = match role {
        AgentRole::Trader => "trader",
        AgentRole::SentimentAnalyst => "sentiment_analyst",
        AgentRole::Other(s) => s.as_str(),
        AgentRole::RiskManager => "risk_manager",
        AgentRole::PortfolioManager => "portfolio_manager",
    };
    let mut r = ChatRequest::new(
        ModelId::new("model-placeholder"),
        LlmTier::DeepThink,
        role.clone(),
    );
    r.max_tokens = 256;
    r.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text(format!(
            "Reply with the literal string `OK` and nothing else. (role={role_tag})"
        ))],
    });
    r
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .try_init()
        .ok();

    let manifest = std::env::var("CARGO_MANIFEST_DIR")?;
    let fixture_path = PathBuf::from(manifest).join("fixtures/replay-v1.db");

    // Start clean so the SHA-256 hashes match the synthetic shape
    // committed under git. Any prior fixture is regenerated in place.
    for p in [
        fixture_path.clone(),
        PathBuf::from(format!("{}-wal", fixture_path.display())),
        PathBuf::from(format!("{}-shm", fixture_path.display())),
    ] {
        if p.exists() {
            std::fs::remove_file(&p)?;
        }
    }

    let providers = [
        (
            "anthropic",
            ProviderKind::Anthropic,
            ModelId::new("claude-opus-4-7"),
        ),
        (
            "openai",
            ProviderKind::OpenAi,
            ModelId::new("gpt-5"),
        ),
        (
            "ollama",
            ProviderKind::Other("ollama".to_string()),
            ModelId::new("llama3:8b"),
        ),
    ];
    let roles = [
        AgentRole::Trader,
        AgentRole::SentimentAnalyst,
        AgentRole::Other("smoke".to_string()),
    ];

    let mut total = 0usize;
    for (name, kind, model) in &providers {
        let canned = canned_for(kind, model);
        let leaf = CannedLeaf {
            response: canned,
            kind: kind.clone(),
            name,
        };
        let rec = RecordingProvider::open(leaf, &fixture_path).await?;

        for role in &roles {
            let mut req = smoke_prompt(role);
            req.model = model.clone();
            let _ = rec.complete(req).await?;
            total += 1;
        }
        // Drop the recording handle so the underlying pool closes
        // and the WAL frees its lock before the checkpoint pool opens.
        drop(rec);
        // sqlx closes pools async-eagerly when dropped; give the
        // runtime a tick to release file locks before the next loop
        // iteration re-opens.
        tokio::task::yield_now().await;
    }

    // Consolidate WAL → main DB so the committed fixture is a
    // single self-contained file. We open a one-shot pool with
    // max_connections=1 so the checkpoint runs against a single
    // unique connection.
    {
        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        let opts = SqliteConnectOptions::new()
            .filename(&fixture_path)
            .create_if_missing(false);
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(opts)
            .await?;
        // Checkpoint then flip to DELETE journal mode so the committed
        // fixture is a single-file artefact (no `-wal` / `-shm`
        // sidecars). RecordingProvider re-enables WAL when paper mode
        // re-opens the same path later.
        let _ = sqlx::query("PRAGMA wal_checkpoint(TRUNCATE);")
            .execute(&pool)
            .await;
        let _ = sqlx::query("PRAGMA journal_mode = DELETE;")
            .execute(&pool)
            .await;
        pool.close().await;
    }

    println!("wrote {total} rows to {}", fixture_path.display());

    // Best-effort: drop any remaining WAL sidecar.
    for suffix in ["-wal", "-shm"] {
        let mut sib = fixture_path.clone().into_os_string();
        sib.push(suffix);
        let sib = PathBuf::from(sib);
        let _ = std::fs::remove_file(&sib);
    }
    Ok(())
}
