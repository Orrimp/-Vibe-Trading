//! `LlmConfig` — minimal v2 configuration shape consumed by the
//! pricing module, the budget gate, and the factory.
//!
//! **Scope note (developer, pass 3, 2026-05-12).** Design § "How it
//! shows up in code" item 10 places `LlmConfig` at
//! `crates/agent/src/config.rs:300` (extending the agent's top-level
//! `Config` struct). That cross-crate wiring is T1937's job (M6 —
//! pass 4+). Pass 3 introduces the type locally in the `llm` crate so
//! the pricing module + budget decorator + factory have a typed
//! surface to consume; T1937 will either re-export this type into
//! `agent::config` or move it once the agent-config integration site
//! lands. **No `serde` derive macro on the agent-side TOML yet** —
//! that's part of T1937's brief.
//!
//! Until T1937 lands, callers construct `LlmConfig` via the builder /
//! defaults below. The shape covers everything T1911 (pricing),
//! T1912 (budgeted), T1913 (factory), and T1914 (auth) need.

use std::path::PathBuf;
use std::sync::Arc;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::pricing::OverrideMap;
use crate::trait_def::ModelId;
use crate::ProviderKind;

/// Per-tier model selection (deep_think vs quick_think — feature.md R12.1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TierConfig {
    /// Provider key for this tier (`"anthropic"`, `"openai"`, …).
    pub provider: String,
    /// Model identifier within that provider's namespace.
    pub model: ModelId,
}

/// LLM runtime configuration (v2.0.0 minimum surface).
///
/// Fields directly consumed by `pricing` / `BudgetedProvider` /
/// `LlmProviderFactory`. Constructed today via [`LlmConfig::new`] or
/// the builder pattern below; once T1937 wires it to the agent-config
/// crate, deserialised from `[llm]` in `agent.toml`.
#[derive(Debug, Clone)]
pub struct LlmConfig {
    /// Monthly USD ceiling (mirrors `[llm] budget_usd_month`).
    pub budget_usd_month: Decimal,
    /// Default provider name (the leaf the factory constructs).
    pub default_provider: String,
    /// Replay-cache path (M6 — `data/llm-replay.db` by default).
    pub replay_cache_path: PathBuf,
    /// DeepThink tier model selection.
    pub deep_think: TierConfig,
    /// QuickThink tier model selection (used as the degrade target by
    /// `BudgetedProvider` when `mode_override == Some(QuickThink)`).
    pub quick_think: TierConfig,
    /// Optional per-model pricing overrides (Q7 — TOML override path).
    pub pricing: OverrideMap,
}

impl Default for LlmConfig {
    /// v2.0.0 strawman defaults: Anthropic provider, Opus 4.7 for
    /// deep_think, Haiku 4.5 for quick_think, $200/mo ceiling.
    fn default() -> Self {
        Self {
            budget_usd_month: dec!(200.00),
            default_provider: "anthropic".to_string(),
            replay_cache_path: PathBuf::from("data/llm-replay.db"),
            deep_think: TierConfig {
                provider: "anthropic".to_string(),
                model: ModelId::new("claude-opus-4-7"),
            },
            quick_think: TierConfig {
                provider: "anthropic".to_string(),
                model: ModelId::new("claude-haiku-4-5-20251001"),
            },
            pricing: OverrideMap::new(),
        }
    }
}

impl LlmConfig {
    /// Construct with explicit budget + default provider; everything
    /// else defaults to the v2 strawman.
    #[must_use]
    pub fn new(budget_usd_month: Decimal, default_provider: impl Into<String>) -> Self {
        Self {
            budget_usd_month,
            default_provider: default_provider.into(),
            ..Self::default()
        }
    }

    /// The `ProviderKind` matching `default_provider`. Used by the
    /// budget decorator's pricing lookup when the request didn't pin a
    /// provider explicitly.
    #[must_use]
    pub fn default_provider_kind(&self) -> ProviderKind {
        provider_kind_from_name(&self.default_provider)
    }
}

/// Map a config-file provider name to its `ProviderKind`. Unknown
/// names route to `Other(name)` so a future provider plugged in via
/// TOML doesn't need a code change to flow through pricing — the
/// override map fills in the rate card.
#[must_use]
pub fn provider_kind_from_name(name: &str) -> ProviderKind {
    match name {
        "anthropic" => ProviderKind::Anthropic,
        "openai" => ProviderKind::OpenAi,
        "openrouter" => ProviderKind::OpenRouter,
        "deepseek" => ProviderKind::DeepSeek,
        other => ProviderKind::Other(other.to_string()),
    }
}

/// Shared-pointer alias for the factory's owned config.
pub type SharedConfig = Arc<LlmConfig>;
