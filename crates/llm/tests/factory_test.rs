//! T1913 acceptance — `LlmProviderFactory::build`.
//!
//! Four acceptance criteria from `spec/v1/v2-llm-strategy/tasks.md`:
//!
//! - (a) build with valid `agent.toml.local` succeeds in paper mode.
//! - (b) build with missing key returns `LlmError::Auth` whose
//!   `Display` names `config/agent.toml.local`.
//! - (c) build in research mode wraps in `ReplayProvider` — **PASS-5
//!   FLIP**: T1922 landed, factory now returns a real
//!   `BudgetedProvider<ReplayProvider>` against the configured
//!   replay-cache path.
//! - (d) build in paper mode wraps in `RecordingProvider` — **PASS-5
//!   FLIP**: T1921 landed, factory now wraps the leaf in
//!   `RecordingProvider`. The leaf's `provider_kind()` still
//!   surfaces through the recording wrapper.

use std::fs;
use std::sync::Arc;

use cost::{CostBudget, CostSink, NoopCostSink};
use llm::factory::{LlmProviderFactory, Mode};
use llm::{LlmConfig, LlmError, ProviderKind};
use rust_decimal_macros::dec;

fn write_overlay(td: &tempfile::TempDir, content: &str) -> std::path::PathBuf {
    let agent_toml = td.path().join("agent.toml");
    fs::write(&agent_toml, "").unwrap();
    let overlay = td.path().join("agent.toml.local");
    fs::write(&overlay, content).unwrap();
    agent_toml
}

fn cfg_in(td: &tempfile::TempDir, default_provider: &str) -> Arc<LlmConfig> {
    let cfg = LlmConfig {
        default_provider: default_provider.to_string(),
        replay_cache_path: td.path().join("replay.db"),
        ..Default::default()
    };
    Arc::new(cfg)
}

#[tokio::test]
async fn t1913_a_paper_mode_with_valid_local_succeeds() {
    let td = tempfile::tempdir().unwrap();
    let agent_toml = write_overlay(
        &td,
        r#"
[llm.providers.anthropic]
api_key = "sk-ant-test-12345"
"#,
    );

    let cfg = cfg_in(&td, "anthropic");
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let provider = LlmProviderFactory::build(cfg, Mode::Paper, budget, sink, &agent_toml)
        .await
        .expect("paper build with valid keys");
    assert_eq!(provider.name(), "anthropic");
    assert!(matches!(provider.provider_kind(), ProviderKind::Anthropic));
}

#[tokio::test]
async fn t1913_b_missing_key_returns_auth_naming_config_local() {
    let td = tempfile::tempdir().unwrap();
    // .local exists but anthropic key absent.
    let agent_toml = write_overlay(
        &td,
        r#"
[llm.providers.openai]
api_key = "sk-openai-stub"
"#,
    );
    let cfg = cfg_in(&td, "anthropic");
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let result = LlmProviderFactory::build(cfg, Mode::Paper, budget, sink, &agent_toml).await;
    match result {
        Ok(_) => panic!("expected Auth error"),
        Err(LlmError::Auth(msg)) => {
            assert!(
                msg.contains("agent.toml.local"),
                "Display must name the .local path: {msg}"
            );
        }
        Err(other) => panic!("expected Auth, got {other:?}"),
    }
}

/// T1913 (c) [PASS-5 FLIP]: research-mode builds a real
/// `ReplayProvider`. Strict-only at v2.0.0 per D2 operator lock.
#[tokio::test]
async fn t1913_c_research_mode_builds_replay_provider() {
    let td = tempfile::tempdir().unwrap();
    let agent_toml = write_overlay(
        &td,
        r#"
[llm.providers.anthropic]
api_key = "sk-ant-stub"
"#,
    );
    let cfg = cfg_in(&td, "anthropic");
    // Pre-create the fixture DB so ReplayProvider can open read-only.
    {
        use async_trait::async_trait;
        use llm::{ChatRequest, ChatResponse, LlmProvider, RecordingProvider};
        struct Stub;
        #[async_trait]
        impl LlmProvider for Stub {
            fn name(&self) -> &str {
                "stub"
            }
            fn provider_kind(&self) -> ProviderKind {
                ProviderKind::Anthropic
            }
            async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
                unreachable!()
            }
        }
        let _ = RecordingProvider::open(Stub, &cfg.replay_cache_path)
            .await
            .expect("seed schema");
    }
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let provider = LlmProviderFactory::build(cfg, Mode::Research, budget, sink, &agent_toml)
        .await
        .expect("research mode builds ReplayProvider");
    assert_eq!(provider.name(), "replay");
    assert!(matches!(
        provider.provider_kind(),
        ProviderKind::Other(ref s) if s == "replay"
    ));
}

/// T1913 + T1922 stack — research mode end-to-end: factory builds
/// `BudgetedProvider<ReplayProvider>`, a recorded request resolves to
/// the canned response, an un-recorded request errors with
/// `LlmError::ReplayMiss { hash, provider, model }`.
#[tokio::test]
async fn t1913_research_mode_end_to_end() {
    use async_trait::async_trait;
    use cost::{AgentRole, LlmTier};
    use llm::{
        ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmProvider, MessageRole, ModelId,
        RecordingProvider, StopReason, TokenUsage,
    };

    let td = tempfile::tempdir().unwrap();
    let agent_toml = write_overlay(&td, "");
    let cfg = cfg_in(&td, "anthropic");

    // Seed one row via RecordingProvider.
    struct CannedLeaf;
    #[async_trait]
    impl LlmProvider for CannedLeaf {
        fn name(&self) -> &str {
            "canned"
        }
        fn provider_kind(&self) -> ProviderKind {
            ProviderKind::Anthropic
        }
        async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
            Ok(ChatResponse {
                content: vec![ContentBlock::Text("OK".into())],
                stop_reason: StopReason::EndTurn,
                usage: TokenUsage {
                    tokens_in: 5,
                    tokens_out: 1,
                    tokens_cached_in: 0,
                },
                model: ModelId::new("claude-opus-4-7"),
                correlation_id: uuid::Uuid::nil(),
            })
        }
    }

    fn make_req() -> ChatRequest {
        let mut r = ChatRequest::new(
            ModelId::new("claude-opus-4-7"),
            LlmTier::DeepThink,
            AgentRole::Trader,
        );
        r.messages.push(ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text("stable-prompt".into())],
        });
        r
    }

    {
        let rec = RecordingProvider::open(CannedLeaf, &cfg.replay_cache_path)
            .await
            .expect("seed");
        rec.complete(make_req()).await.expect("seed row");
    }

    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let provider = LlmProviderFactory::build(cfg, Mode::Research, budget, sink, &agent_toml)
        .await
        .expect("research mode build");

    // (a) recorded request hits.
    let resp = provider.complete(make_req()).await.expect("replay hit");
    let got_text = match resp.content.first() {
        Some(ContentBlock::Text(t)) => t.clone(),
        other => panic!("unexpected content {other:?}"),
    };
    assert_eq!(got_text, "OK");

    // (b) un-recorded request misses with structured ReplayMiss.
    let mut miss_req = make_req();
    miss_req.messages.push(ChatMessage {
        role: MessageRole::User,
        content: vec![ContentBlock::Text("never-recorded".into())],
    });
    let err = provider.complete(miss_req).await.expect_err("miss");
    assert!(
        matches!(err, LlmError::ReplayMiss { ref model, .. } if model == "claude-opus-4-7"),
        "expected ReplayMiss with model={{claude-opus-4-7}}, got {err:?}"
    );
}

/// Ollama: no `.local` overlay required.
#[tokio::test]
async fn t1913_ollama_builds_without_local_overlay() {
    let td = tempfile::tempdir().unwrap();
    let agent_toml = td.path().join("agent.toml");
    fs::write(&agent_toml, "").unwrap();

    let cfg = cfg_in(&td, "ollama");
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let provider = LlmProviderFactory::build(cfg, Mode::Live, budget, sink, &agent_toml)
        .await
        .expect("ollama works without overlay");
    assert_eq!(provider.name(), "ollama");
}
