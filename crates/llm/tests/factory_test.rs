//! T1913 acceptance — `LlmProviderFactory::build`.
//!
//! Four acceptance criteria from `spec/v2-llm-strategy/tasks.md`:
//!
//! - (a) build with valid `agent.toml.local` succeeds in paper mode.
//! - (b) build with missing key returns `LlmError::Auth` whose
//!   `Display` names `config/agent.toml.local`.
//! - (c) build in research mode wraps in `ReplayProvider` — DEFERRED
//!   to M6 (T1922). Pass 3 ships a clearly-signed error from this
//!   arm so the test gates the contract until M6 lands.
//! - (d) build in paper mode wraps in `RecordingProvider` — partial:
//!   pass 3's paper-mode arm logs a `tracing::warn!` and falls
//!   through to `BudgetedProvider<Leaf>`. Once T1921 lands, the
//!   acceptance test below flips one assertion.

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

#[test]
fn t1913_a_paper_mode_with_valid_local_succeeds() {
    let td = tempfile::tempdir().unwrap();
    let agent_toml = write_overlay(
        &td,
        r#"
[llm.providers.anthropic]
api_key = "sk-ant-test-12345"
"#,
    );

    let cfg = Arc::new(LlmConfig::default());
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let provider = LlmProviderFactory::build(cfg, Mode::Paper, budget, sink, &agent_toml)
        .expect("paper build with valid keys");
    assert_eq!(provider.name(), "anthropic");
    assert!(matches!(provider.provider_kind(), ProviderKind::Anthropic));
}

#[test]
fn t1913_b_missing_key_returns_auth_naming_config_local() {
    let td = tempfile::tempdir().unwrap();
    // .local exists but anthropic key absent.
    let agent_toml = write_overlay(
        &td,
        r#"
[llm.providers.openai]
api_key = "sk-openai-stub"
"#,
    );
    let cfg = Arc::new(LlmConfig::default());
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let result = LlmProviderFactory::build(cfg, Mode::Paper, budget, sink, &agent_toml);
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

/// T1913 (c): pass-3 research-mode arm signals M6 dependency.
/// Flip this assertion to `provider.provider_kind() ==
/// ProviderKind::Other("replay")` once T1922 (ReplayProvider) lands.
#[test]
fn t1913_c_research_mode_pending_m6_signals_clearly() {
    let td = tempfile::tempdir().unwrap();
    let agent_toml = write_overlay(
        &td,
        r#"
[llm.providers.anthropic]
api_key = "sk-ant-stub"
"#,
    );
    let cfg = Arc::new(LlmConfig::default());
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let result = LlmProviderFactory::build(cfg, Mode::Research, budget, sink, &agent_toml);
    match result {
        Ok(_) => panic!("research mode should error pending M6"),
        Err(LlmError::Provider { message, .. }) => {
            assert!(
                message.contains("M6") || message.contains("ReplayProvider"),
                "operator must see M6 dependency: {message}"
            );
        }
        Err(other) => panic!("expected Provider, got {other:?}"),
    }
}

/// Ollama: no `.local` overlay required.
#[test]
fn t1913_ollama_builds_without_local_overlay() {
    let td = tempfile::tempdir().unwrap();
    let agent_toml = td.path().join("agent.toml");
    fs::write(&agent_toml, "").unwrap();

    let mut cfg = LlmConfig::default();
    cfg.default_provider = "ollama".to_string();
    let cfg = Arc::new(cfg);
    let budget = Arc::new(CostBudget::new(dec!(200.00)));
    let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

    let provider = LlmProviderFactory::build(cfg, Mode::Live, budget, sink, &agent_toml)
        .expect("ollama works without overlay");
    assert_eq!(provider.name(), "ollama");
}
