//! `LlmProviderFactory` — single construction site for every
//! `Arc<dyn LlmProvider>` consumed by the agent (T1913).
//!
//! Design § Q6: consumers never construct providers directly; the
//! factory always wraps the leaf in `BudgetedProvider` so the budget
//! gate is impossible to forget. Mode-aware wrapping (recording in
//! paper mode, replay in research mode) layers on top of the budget
//! gate — both M6 wrappers (T1921 / T1922) plug into the same factory
//! once they land.
//!
//! **Pass-3 scope note (developer, 2026-05-12).** Recording/Replay
//! providers are M6 — they land at T1921 / T1922 in pass 4+. This
//! pass 3 factory has the slot-shape ready (the `Mode` arm dispatches
//! to TODO holes that return `Provider`-flavoured errors with clear
//! "M6: ReplayProvider not yet wired" messages, so the surface is
//! testable today and the M6 PR is a localized swap of those two
//! arms). The factory's primary path — paper mode without recording
//! — is fully functional and exercised by T1913 acceptance (a).

use std::sync::Arc;

use crate::auth::{load_keys_from_path, KeyMap};
use crate::budgeted::BudgetedProvider;
use crate::config::{provider_kind_from_name, LlmConfig};
use crate::error::LlmError;
use crate::providers::{AnthropicProvider, OllamaProvider, OpenAiProvider};
use crate::trait_def::{ChatRequest, ChatResponse, LlmProvider};
use crate::ProviderKind;
use cost::{CostBudget, CostSink};
use std::path::Path;

/// Agent operating mode (mirrors `agent::config::Mode` once T1937
/// wires LlmConfig into agent::config; until then this enum is
/// llm-local).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Live trading. Real venues, real LLM calls.
    Live,
    /// Paper trading. Real LLM calls, mock venues. `RecordingProvider`
    /// captures every response into the replay cache so a subsequent
    /// research run replays the same fixtures (R10.2).
    Paper,
    /// Research / backtest. Deterministic replay only — every LLM call
    /// resolves against `data/llm-replay.db`; cache miss is a hard
    /// `LlmError::ReplayMiss`.
    Research,
}

/// Construction surface for `Arc<dyn LlmProvider>`.
pub struct LlmProviderFactory;

impl LlmProviderFactory {
    /// Build the configured provider stack for `mode` against the
    /// supplied budget + sink + agent-config path.
    ///
    /// Stack (outer-to-inner once M6 wrappers land):
    /// ```text
    /// Live    : BudgetedProvider<Leaf>
    /// Paper   : BudgetedProvider<RecordingProvider<Leaf>>   [M6]
    /// Research: BudgetedProvider<ReplayProvider>            [M6]
    /// ```
    ///
    /// Pass-3 reality: Live works end-to-end; Paper falls back to
    /// `BudgetedProvider<Leaf>` (no recording wrap) with a
    /// `tracing::warn!` advising the operator that fixtures will not
    /// be captured until M6 lands; Research returns
    /// `LlmError::Provider` because there's no leaf to call deterministically.
    ///
    /// # Errors
    ///
    /// - [`LlmError::Auth`] from [`load_keys_from_path`] (T1914) on
    ///   missing keys.
    /// - [`LlmError::Provider`] when `cfg.default_provider` is
    ///   unknown OR when `mode = Research` (M6 not wired).
    pub fn build(
        cfg: Arc<LlmConfig>,
        mode: Mode,
        budget: Arc<CostBudget>,
        sink: Arc<dyn CostSink>,
        agent_toml_path: &Path,
    ) -> Result<Arc<dyn LlmProvider>, LlmError> {
        // ── 1. Keys ──────────────────────────────────────────────────
        let keys = load_keys_from_path(cfg.as_ref(), agent_toml_path)?;

        // ── 2. Leaf provider ─────────────────────────────────────────
        let leaf: Box<dyn LlmProvider> = construct_leaf(&cfg, &keys)?;

        // ── 3. Mode-aware wrapping ───────────────────────────────────
        match mode {
            Mode::Live => Ok(Arc::new(BudgetedProvider::new(
                BoxedProvider(leaf),
                budget,
                sink,
                cfg,
            ))),
            Mode::Paper => {
                tracing::warn!(
                    target: "llm.factory",
                    "paper mode: RecordingProvider not yet wired (M6 / T1921). \
                     LLM responses will NOT be persisted into the replay cache \
                     for subsequent research-mode runs."
                );
                Ok(Arc::new(BudgetedProvider::new(
                    BoxedProvider(leaf),
                    budget,
                    sink,
                    cfg,
                )))
            }
            Mode::Research => Err(LlmError::Provider {
                provider: ProviderKind::Other("replay".to_string()),
                message: "research mode requires ReplayProvider; lands in M6 (T1922). \
                     Pass-3 factory cannot build a deterministic provider stack."
                    .to_string(),
            }),
        }
    }
}

/// `Box<dyn LlmProvider>` is not itself `LlmProvider` (auto-impl gap),
/// so we wrap with a thin newtype that forwards each method.
struct BoxedProvider(Box<dyn LlmProvider>);

#[async_trait::async_trait]
impl LlmProvider for BoxedProvider {
    fn name(&self) -> &str {
        self.0.name()
    }
    fn provider_kind(&self) -> ProviderKind {
        self.0.provider_kind()
    }
    async fn complete(&self, req: ChatRequest) -> Result<ChatResponse, LlmError> {
        self.0.complete(req).await
    }
}

/// Build the leaf provider for `cfg.default_provider`. Unknown names
/// return `LlmError::Auth` (operator-actionable: edit the TOML).
fn construct_leaf(cfg: &LlmConfig, keys: &KeyMap) -> Result<Box<dyn LlmProvider>, LlmError> {
    let provider_kind = provider_kind_from_name(&cfg.default_provider);
    let model = cfg.deep_think.model.clone(); // factory-default tier model.
    match provider_kind {
        ProviderKind::Anthropic => {
            let key = require_key(keys, "anthropic")?;
            Ok(Box::new(AnthropicProvider::new(key, model)))
        }
        ProviderKind::OpenAi => {
            let key = require_key(keys, "openai")?;
            Ok(Box::new(OpenAiProvider::new(key, model)))
        }
        ProviderKind::OpenRouter | ProviderKind::DeepSeek => {
            let name = cfg.default_provider.as_str();
            let key = require_key(keys, name)?;
            Ok(Box::new(OpenAiProvider::new(key, model)))
        }
        ProviderKind::Other(ref name) if name == "ollama" => {
            Ok(Box::new(OllamaProvider::new(model)))
        }
        ProviderKind::Other(name) => Err(LlmError::Provider {
            provider: ProviderKind::Other(name.clone()),
            message: format!("unknown provider '{name}' — no leaf constructor"),
        }),
    }
}

fn require_key<'a>(keys: &'a KeyMap, name: &str) -> Result<&'a str, LlmError> {
    keys.get(name)
        .ok_or_else(|| LlmError::Auth(format!("{name}.api_key not loaded")))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use cost::NoopCostSink;
    use rust_decimal_macros::dec;
    use std::fs;

    fn write_overlay(td: &tempfile::TempDir, content: &str) -> std::path::PathBuf {
        let agent_toml = td.path().join("agent.toml");
        fs::write(&agent_toml, "").unwrap();
        let overlay = td.path().join("agent.toml.local");
        fs::write(&overlay, content).unwrap();
        agent_toml
    }

    /// T1913 (a): build with valid `.local` overlay succeeds in
    /// Paper mode.
    #[test]
    fn t1913_a_build_with_valid_keys_paper_mode() {
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

        let provider =
            LlmProviderFactory::build(cfg, Mode::Paper, budget, sink, &agent_toml).expect("build");
        assert_eq!(provider.name(), "anthropic");
        assert!(matches!(provider.provider_kind(), ProviderKind::Anthropic));
    }

    /// T1913 (b): missing key → `LlmError::Auth` whose `Display`
    /// names `config/agent.toml.local`.
    #[test]
    fn t1913_b_missing_key_returns_auth_naming_the_file() {
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
                    "Display must name the local file: {msg}"
                );
            }
            Err(other) => panic!("expected Auth, got {other:?}"),
        }
    }

    /// T1913 (c) [PARTIAL]: Research mode currently errors loudly
    /// because the M6 ReplayProvider is not yet wired. Once T1922
    /// lands, this assertion flips to `provider.provider_kind() ==
    /// Other("replay")` (or similar).
    #[test]
    fn t1913_c_research_mode_clearly_signals_m6_missing() {
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
                    "message must direct operator to M6 dep: {message}"
                );
            }
            Err(other) => panic!("expected Provider error, got {other:?}"),
        }
    }

    /// Ollama mode needs no `.local` overlay.
    #[test]
    fn t1913_ollama_build_no_keys_needed() {
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
}
