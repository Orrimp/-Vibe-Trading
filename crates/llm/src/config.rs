//! `LlmConfig` — v2 configuration shape consumed by the pricing
//! module, the budget gate, the factory, and the agent's startup
//! loader (T1928, pass 6).
//!
//! **Pass-6 hoist (developer, 2026-05-12).** Per Design § "How it
//! shows up in code" item 10, the canonical `LlmConfig` lives at
//! `crates/agent/src/config.rs:300` — but the type's fields depend on
//! `cost::ProviderKind`, `OverrideMap`, and `ModelId`, all owned by
//! the `llm` crate. To honour the architect's intent without creating
//! a circular dep (`agent → llm → cost`, never the inverse), the
//! canonical struct *stays here* and `crate::agent::config` adds a
//! `pub llm: LlmConfig` field on the root `Config` via a `pub use
//! llm::LlmConfig` re-export. This file gains serde derives so the
//! `[llm]` TOML block deserialises directly into this struct.
//!
//! The shape covers everything T1911 (pricing), T1912 (budgeted),
//! T1913 (factory), T1914 (auth), and T1931 (agent main wire-up) need.

use std::collections::HashMap;
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

/// Per-provider committed-shape entry. `api_key` is `Option<String>`
/// so the committed `agent.toml` carries no key (the field defaults to
/// `None`); the operator-only `agent.toml.local` overlay fills it in
/// per Q3 = C. Path discovery lives in [`crate::auth::load_keys`].
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct ProviderConfig {
    /// Base URL for the provider's HTTP endpoint.
    pub base_url: String,
    /// Optional API key — populated only from the `.local` overlay.
    #[serde(default)]
    pub api_key: Option<String>,
}

/// LLM runtime configuration (v2.0.0 surface, T1928 pass-6 hoist).
///
/// Fields directly consumed by `pricing` / `BudgetedProvider` /
/// `LlmProviderFactory`. Deserialised from the `[llm]` block in
/// `config/agent.toml` (committed defaults) with the `.local` overlay
/// (Q3 = C) merging in the `api_key` fields at startup.
///
/// `enabled` defaults to `false` so a fresh checkout (no LLM consumers
/// in v2.0.0) does not boot the LLM subsystem (T1928 acceptance d).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// Master enable. `false` (default) keeps the agent from
    /// constructing any provider / reading any keys at boot. Flipping
    /// to `true` requires a valid `agent.toml.local` overlay unless
    /// `default_provider = "ollama"`.
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    /// Monthly USD ceiling (mirrors `[llm] budget_usd_month`).
    ///
    /// Deserialised via `deserialize_budget_usd_month` so the TOML
    /// form `budget_usd_month = 200.0` (float) parses naturally; we
    /// also accept string forms (`"200.00"`) and integers (`200`).
    /// Rounded to whole cents on the budget-gate side (T1908) —
    /// fractional sub-cents in the TOML are an operator typo that
    /// the gate's rounding silently fixes.
    #[serde(
        default = "default_budget_usd_month",
        deserialize_with = "deserialize_budget_usd_month",
        serialize_with = "serialize_budget_usd_month"
    )]
    pub budget_usd_month: Decimal,
    /// Default provider name (the leaf the factory constructs).
    #[serde(default = "default_provider_name")]
    pub default_provider: String,
    /// Replay-cache path (M6 — `data/llm-replay.db` by default).
    #[serde(default = "default_replay_cache_path")]
    pub replay_cache_path: PathBuf,
    /// DeepThink tier model selection.
    #[serde(default = "default_deep_think_tier")]
    pub deep_think: TierConfig,
    /// QuickThink tier model selection (used as the degrade target by
    /// `BudgetedProvider` when `mode_override == Some(QuickThink)`).
    #[serde(default = "default_quick_think_tier")]
    pub quick_think: TierConfig,
    /// Per-provider committed-shape entries (`base_url`, optional
    /// overlay-populated `api_key`).
    #[serde(default)]
    pub providers: HashMap<String, ProviderConfig>,
    /// Optional per-model pricing overrides (Q7 — TOML override path).
    #[serde(default)]
    pub pricing: OverrideMap,
}

fn default_enabled() -> bool {
    false
}

fn default_budget_usd_month() -> Decimal {
    dec!(200.00)
}

fn default_provider_name() -> String {
    "anthropic".to_string()
}

fn default_replay_cache_path() -> PathBuf {
    PathBuf::from("data/llm-replay.db")
}

fn default_deep_think_tier() -> TierConfig {
    TierConfig {
        provider: "anthropic".to_string(),
        model: ModelId::new("claude-opus-4-7"),
    }
}

fn default_quick_think_tier() -> TierConfig {
    TierConfig {
        provider: "anthropic".to_string(),
        model: ModelId::new("claude-haiku-4-5-20251001"),
    }
}

/// Accept TOML float `budget_usd_month = 200.0`, integer `200`, or
/// quoted-string `"200.00"`. The workspace-level `rust_decimal`
/// feature is `serde-with-str` which only handles the string form
/// natively; this helper bridges TOML's float surface without
/// requiring a workspace-wide feature flip.
#[allow(clippy::float_arithmetic)] // intentional: TOML deserializer hands us f64
fn deserialize_budget_usd_month<'de, D>(deserializer: D) -> Result<Decimal, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::de::Error;
    let value = toml::Value::deserialize(deserializer)?;
    match value {
        toml::Value::Float(f) => Decimal::try_from(f).map_err(D::Error::custom),
        toml::Value::Integer(i) => Ok(Decimal::from(i)),
        toml::Value::String(s) => s.parse::<Decimal>().map_err(D::Error::custom),
        other => Err(D::Error::custom(format!(
            "expected float/int/string for budget_usd_month, got {:?}",
            other.type_str()
        ))),
    }
}

/// Symmetric serializer — emits a string (matches the workspace's
/// `serde-with-str` Decimal default for forward-compat).
fn serialize_budget_usd_month<S>(value: &Decimal, serializer: S) -> Result<S::Ok, S::Error>
where
    S: serde::Serializer,
{
    serializer.serialize_str(&value.to_string())
}

impl Default for LlmConfig {
    /// v2.0.0 strawman defaults: foundation-off (T1928 acceptance d),
    /// Anthropic provider, Opus 4.7 for deep_think, Haiku 4.5 for
    /// quick_think, $200/mo ceiling.
    fn default() -> Self {
        Self {
            enabled: default_enabled(),
            budget_usd_month: default_budget_usd_month(),
            default_provider: default_provider_name(),
            replay_cache_path: default_replay_cache_path(),
            deep_think: default_deep_think_tier(),
            quick_think: default_quick_think_tier(),
            providers: HashMap::new(),
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

impl LlmConfig {
    /// Validate `LlmConfig` per T1928 acceptance (c): when `enabled =
    /// true` AND `default_provider != "ollama"`, the `providers` map
    /// must contain a `ProviderConfig` for `default_provider` with a
    /// non-empty `api_key`. The `.local` overlay (T1914) is expected
    /// to have already merged its keys into `self.providers` before
    /// this check runs.
    ///
    /// Returns `Err(LlmError::Auth(...))` with an operator-actionable
    /// message naming the missing field.
    ///
    /// # Errors
    ///
    /// - [`crate::LlmError::Auth`] when the gate above fails.
    pub fn validate_keys(&self) -> Result<(), crate::LlmError> {
        if !self.enabled {
            return Ok(());
        }
        if self.default_provider == "ollama" {
            return Ok(());
        }
        let Some(entry) = self.providers.get(&self.default_provider) else {
            return Err(crate::LlmError::Auth(format!(
                "[llm.providers.{}] section missing in agent.toml — copy \
                 config/agent.toml.local.example to config/agent.toml.local \
                 and edit in real keys",
                self.default_provider
            )));
        };
        match entry.api_key.as_deref() {
            Some(k) if !k.is_empty() => Ok(()),
            _ => Err(crate::LlmError::Auth(format!(
                "{}.api_key not set in config/agent.toml.local",
                self.default_provider
            ))),
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// T1928 (a) — the committed `agent.toml` `[llm]` block parses
    /// into `LlmConfig` with defaults preserved on missing fields.
    #[test]
    fn t1928_a_canonical_llm_block_parses() {
        let toml = r#"
enabled              = false
default_provider     = "anthropic"
budget_usd_month     = 200.0
replay_cache_path    = "./data/llm-replay.db"

[deep_think]
provider = "anthropic"
model    = "claude-opus-4-7"

[quick_think]
provider = "anthropic"
model    = "claude-haiku-4-5-20251001"

[providers.anthropic]
base_url = "https://api.anthropic.com/v1"

[providers.openai]
base_url = "https://api.openai.com/v1"

[providers.ollama]
base_url = "http://localhost:11434"
"#;
        let cfg: LlmConfig = toml::from_str(toml).expect("parse [llm] block");
        assert!(!cfg.enabled, "enabled defaults to false");
        assert_eq!(cfg.default_provider, "anthropic");
        assert_eq!(cfg.budget_usd_month, dec!(200.00));
        assert_eq!(cfg.deep_think.model.as_str(), "claude-opus-4-7");
        assert_eq!(cfg.quick_think.model.as_str(), "claude-haiku-4-5-20251001");
        assert!(cfg.providers.contains_key("anthropic"));
        assert!(cfg.providers.contains_key("openai"));
        assert!(cfg.providers.contains_key("ollama"));
        assert_eq!(
            cfg.providers["anthropic"].base_url,
            "https://api.anthropic.com/v1"
        );
        // No api_key in the committed shape.
        assert!(cfg.providers["anthropic"].api_key.is_none());
    }

    /// T1928 (b) — overlay-resolved `api_key` populates the provider
    /// entry. (Helper: parse a minimal overlay, merge by hand mimicking
    /// what the `.local` overlay loader does.)
    #[test]
    fn t1928_b_overlay_populates_api_key() {
        let toml = r#"
enabled              = true
default_provider     = "anthropic"

[deep_think]
provider = "anthropic"
model    = "claude-opus-4-7"

[quick_think]
provider = "anthropic"
model    = "claude-haiku-4-5-20251001"

[providers.anthropic]
base_url = "https://api.anthropic.com/v1"
api_key  = "sk-ant-test-stub-12345"
"#;
        let cfg: LlmConfig = toml::from_str(toml).expect("parse [llm] block");
        assert_eq!(
            cfg.providers["anthropic"].api_key.as_deref(),
            Some("sk-ant-test-stub-12345")
        );
        cfg.validate_keys()
            .expect("validate_keys ok when key is non-empty");
    }

    /// T1928 (c) — `enabled = true && default_provider = "anthropic"`
    /// with no api_key in the providers map rejects.
    #[test]
    fn t1928_c_enabled_without_key_rejects() {
        let mut cfg = LlmConfig {
            enabled: true,
            ..Default::default()
        };
        cfg.providers.insert(
            "anthropic".into(),
            ProviderConfig {
                base_url: "https://api.anthropic.com/v1".into(),
                api_key: None,
            },
        );
        let err = cfg
            .validate_keys()
            .expect_err("validate_keys must reject enabled-without-key");
        let msg = err.to_string();
        assert!(
            msg.contains("anthropic"),
            "error must name the missing provider: {msg}"
        );
    }

    /// T1928 (d) — default `enabled = false` boots without any key
    /// validation (the fresh-checkout case).
    #[test]
    fn t1928_d_default_disabled_no_key_required() {
        let cfg = LlmConfig::default();
        assert!(!cfg.enabled);
        cfg.validate_keys()
            .expect("validate_keys is a no-op when disabled");
    }

    /// Ollama provider with `enabled = true` and no key is fine
    /// (Ollama needs no auth — its `api_key` field is always `None`).
    #[test]
    fn t1928_ollama_enabled_without_key_passes() {
        let cfg = LlmConfig {
            enabled: true,
            default_provider: "ollama".into(),
            ..Default::default()
        };
        cfg.validate_keys()
            .expect("ollama needs no auth even when enabled");
    }
}
