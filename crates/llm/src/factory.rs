//! `LlmProviderFactory` — single construction site for every
//! `Arc<dyn LlmProvider>` consumed by the agent (T1913).
//!
//! Design § Q6: consumers never construct providers directly; the
//! factory always wraps the leaf in `BudgetedProvider` so the budget
//! gate is impossible to forget. Mode-aware wrapping (recording in
//! paper mode, replay in research mode) layers on top of the budget
//! gate.
//!
//! **Pass-5 flip (developer, 2026-05-12).** T1921 + T1922 landed in
//! this pass (M6), shipping `RecordingProvider<Inner>` +
//! `ReplayProvider`. The factory's `Mode::Paper` arm now wraps the
//! leaf in `RecordingProvider` so subsequent research-mode runs replay
//! the captured fixtures; the `Mode::Research` arm builds
//! `ReplayProvider` (no leaf — strict-only at v2.0.0 per D2 operator
//! lock; cache miss surfaces as `LlmError::ReplayMiss { hash, provider,
//! model }`). Pass-3's "M6 not yet wired" holes are gone — T1913
//! flips from `[~]` to `[x]` in tasks.md.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::ProviderKind;
use crate::auth::{KeyMap, load_keys_from_path};
use crate::budgeted::BudgetedProvider;
use crate::config::{LlmConfig, provider_kind_from_name};
use crate::error::LlmError;
use crate::providers::{AnthropicProvider, OllamaProvider, OpenAiProvider};
use crate::recording::RecordingProvider;
use crate::replay::ReplayProvider;
use crate::trait_def::{ChatRequest, ChatResponse, LlmProvider};
use cost::{CostBudget, CostSink};

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
    /// Stack (outer-to-inner):
    /// ```text
    /// Live    : BudgetedProvider<Leaf>
    /// Paper   : BudgetedProvider<RecordingProvider<Leaf>>
    /// Research: BudgetedProvider<ReplayProvider>            (no Leaf — D2 strict-only)
    /// ```
    ///
    /// `Mode::Paper` opens / creates `cfg.replay_cache_path` and runs
    /// the schema migration; `Mode::Research` opens the same path
    /// read-only. The smoke binary (T1923) is the primary consumer
    /// of this surface.
    ///
    /// **Async.** Recording/Replay open SQLite asynchronously
    /// (sqlx's connect path is async-only). Pass-4 `build` was sync
    /// — the M6 flip turns this into an async fn. Every call site
    /// already runs inside a `tokio` runtime (the agent's main loop,
    /// the smoke binary, the integration tests) so the surface flip
    /// is mechanical.
    ///
    /// # Errors
    ///
    /// - [`LlmError::Auth`] from [`load_keys_from_path`] (T1914) on
    ///   missing keys.
    /// - [`LlmError::Provider`] when `cfg.default_provider` is
    ///   unknown OR when the replay-cache SQLite cannot be opened
    ///   (research mode) / created (paper mode).
    pub async fn build(
        cfg: Arc<LlmConfig>,
        mode: Mode,
        budget: Arc<CostBudget>,
        sink: Arc<dyn CostSink>,
        agent_toml_path: &Path,
    ) -> Result<Arc<dyn LlmProvider>, LlmError> {
        // ── 1. Mode-aware wrapping ───────────────────────────────────
        match mode {
            Mode::Live => {
                let keys = load_keys_from_path(cfg.as_ref(), agent_toml_path)?;
                let leaf = construct_leaf(&cfg, &keys)?;
                Ok(Arc::new(BudgetedProvider::new(
                    BoxedProvider(leaf),
                    budget,
                    sink,
                    cfg,
                )))
            }
            Mode::Paper => {
                let keys = load_keys_from_path(cfg.as_ref(), agent_toml_path)?;
                let leaf = construct_leaf(&cfg, &keys)?;
                let path = replay_cache_path(&cfg);
                let rec = RecordingProvider::open(BoxedProvider(leaf), &path).await?;
                tracing::info!(
                    target: "llm.factory",
                    path = %path.display(),
                    "paper mode: RecordingProvider wired"
                );
                Ok(Arc::new(BudgetedProvider::new(rec, budget, sink, cfg)))
            }
            Mode::Research => {
                // D2 strict-only: NO leaf, NO live API key required.
                // Research mode reads from the fixture cache and
                // panics-by-error on a miss. Skipping the auth load
                // also means a fresh dev box without `agent.toml.local`
                // can still run research replays.
                let path = replay_cache_path(&cfg);
                let replay = ReplayProvider::open(&path).await?;
                tracing::info!(
                    target: "llm.factory",
                    path = %path.display(),
                    "research mode: ReplayProvider wired (strict)"
                );
                Ok(Arc::new(BudgetedProvider::new(replay, budget, sink, cfg)))
            }
        }
    }
}

/// Resolve the replay-cache path from config, honouring an absolute
/// override or interpreting a relative path against the current
/// working directory. The default `LlmConfig::default()` ships with
/// `data/llm-replay.db` — relative.
fn replay_cache_path(cfg: &LlmConfig) -> PathBuf {
    cfg.replay_cache_path.clone()
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

    /// Override the cfg's replay-cache path into the per-test tempdir
    /// so paper/research opens land in scratch storage (not the
    /// crate's real `data/`).
    fn cfg_with_replay_in(td: &tempfile::TempDir, default_provider: &str) -> Arc<LlmConfig> {
        let cfg = LlmConfig {
            default_provider: default_provider.to_string(),
            replay_cache_path: td.path().join("replay.db"),
            ..Default::default()
        };
        Arc::new(cfg)
    }

    /// T1913 (a): build with valid `.local` overlay succeeds in
    /// Paper mode. Pass-5 update: `RecordingProvider` is now wired
    /// (M6 / T1921); the leaf provider name still surfaces through
    /// the recording wrapper.
    #[tokio::test]
    async fn t1913_a_build_with_valid_keys_paper_mode() {
        let td = tempfile::tempdir().unwrap();
        let agent_toml = write_overlay(
            &td,
            r#"
[llm.providers.anthropic]
api_key = "sk-ant-test-12345"
"#,
        );
        let cfg = cfg_with_replay_in(&td, "anthropic");
        let budget = Arc::new(CostBudget::new(dec!(200.00)));
        let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

        let provider = LlmProviderFactory::build(cfg, Mode::Paper, budget, sink, &agent_toml)
            .await
            .expect("build");
        assert_eq!(provider.name(), "anthropic");
        assert!(matches!(provider.provider_kind(), ProviderKind::Anthropic));
    }

    /// T1913 (b): missing key → `LlmError::Auth` whose `Display`
    /// names `config/agent.toml.local`.
    #[tokio::test]
    async fn t1913_b_missing_key_returns_auth_naming_the_file() {
        let td = tempfile::tempdir().unwrap();
        // .local exists but anthropic key absent.
        let agent_toml = write_overlay(
            &td,
            r#"
[llm.providers.openai]
api_key = "sk-openai-stub"
"#,
        );
        let cfg = cfg_with_replay_in(&td, "anthropic");
        let budget = Arc::new(CostBudget::new(dec!(200.00)));
        let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

        let result = LlmProviderFactory::build(cfg, Mode::Paper, budget, sink, &agent_toml).await;
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

    /// T1913 (c) [PASS-5 FLIP]: Research mode now builds a real
    /// `ReplayProvider` (D2 strict-only). The factory builds the
    /// stack against an empty fixture; the resulting provider's
    /// `name()` is "replay" (replay surface, not a real leaf).
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
        // Pre-create the fixture DB via RecordingProvider so the
        // schema exists when ReplayProvider opens it read-only.
        let cfg = cfg_with_replay_in(&td, "anthropic");
        {
            use crate::recording::RecordingProvider;
            use crate::trait_def::ChatRequest;
            #[derive(Clone)]
            struct Stub;
            #[async_trait::async_trait]
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
            .expect("research mode should build");
        assert_eq!(provider.name(), "replay");
        assert!(matches!(
            provider.provider_kind(),
            ProviderKind::Other(ref s) if s == "replay"
        ));
    }

    /// Ollama mode needs no `.local` overlay.
    #[tokio::test]
    async fn t1913_ollama_build_no_keys_needed() {
        let td = tempfile::tempdir().unwrap();
        let agent_toml = td.path().join("agent.toml");
        fs::write(&agent_toml, "").unwrap();

        let cfg = cfg_with_replay_in(&td, "ollama");
        let budget = Arc::new(CostBudget::new(dec!(200.00)));
        let sink: Arc<dyn CostSink> = Arc::new(NoopCostSink);

        let provider = LlmProviderFactory::build(cfg, Mode::Live, budget, sink, &agent_toml)
            .await
            .expect("ollama works without overlay");
        assert_eq!(provider.name(), "ollama");
    }
}
