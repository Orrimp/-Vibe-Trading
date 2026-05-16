//! LLM pricing — hard-coded base table + TOML override (T1911).
//!
//! Design § Q7 (`spec/v2-llm-strategy/feature.md:1567`):
//!
//! - Hard-coded base table at `base_rate(provider, model)` — exhaustive
//!   `match` over the v2 supported model set. Unknown combos return
//!   `None`; `BudgetedProvider`'s post-call reconcile treats that as a
//!   loud `LlmError::Provider` so model-id typos surface immediately.
//! - TOML override at `[llm.pricing.<provider>.<model>]` — an
//!   `OverrideMap` (passed by the factory from `LlmConfig.pricing`)
//!   shadows the base row for emergency price changes without
//!   recompile.
//! - Decimal-only arithmetic. No `f64`.
//! - Module location: this crate (`llm`), not `cost`, because the rate
//!   is LLM-domain-specific (provider × model cartesian) — the cost
//!   crate stays provider-agnostic.

use std::collections::HashMap;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use serde::{Deserialize, Serialize};

use crate::error::LlmError;
use crate::ProviderKind;

/// Per-million-token rate-card row.
///
/// Three Decimal fields — input, output, and cached-input — match the
/// provider-billable token classes. `cached_input_usd` is the
/// Anthropic-cached-read rate; for providers that don't surface cached
/// input separately, set this equal to `input_usd` (OpenAI) or to zero
/// (Ollama).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PricePerMillionTokens {
    /// USD per million uncached input tokens.
    pub input_usd: Decimal,
    /// USD per million output tokens.
    pub output_usd: Decimal,
    /// USD per million cached-read input tokens.
    pub cached_input_usd: Decimal,
}

impl PricePerMillionTokens {
    /// Construct a fully zero rate (Ollama or any cost-free provider).
    #[must_use]
    pub const fn zero() -> Self {
        Self {
            input_usd: Decimal::ZERO,
            output_usd: Decimal::ZERO,
            cached_input_usd: Decimal::ZERO,
        }
    }
}

/// Override map: `provider_name → model → PricePerMillionTokens`.
///
/// Stable as a typed `HashMap` (rather than re-parsing TOML at the
/// lookup site) so the factory does the TOML deserialize once at
/// startup and `resolve_rate` is a pure hash-key lookup.
pub type OverrideMap = HashMap<String, HashMap<String, PricePerMillionTokens>>;

/// Stable lowercase provider key for the override map.
///
/// Mirrors the wire-form of `ProviderKind`'s `serde(rename_all =
/// "snake_case")` representation, so TOML keys
/// (`[llm.pricing.anthropic."claude-opus-4-7"]`) line up.
fn provider_key(provider: &ProviderKind) -> String {
    match provider {
        ProviderKind::Anthropic => "anthropic".to_string(),
        ProviderKind::OpenAi => "open_ai".to_string(),
        ProviderKind::OpenRouter => "open_router".to_string(),
        ProviderKind::DeepSeek => "deep_seek".to_string(),
        ProviderKind::Other(name) => name.clone(),
    }
}

/// Base pricing table — the hard-coded v2.0.0 rate card.
///
/// Returns `None` for unsupported `(provider, model)` combos so a
/// model-id typo surfaces loudly at the budget-gate post-call
/// reconcile (`LlmError::Provider`). The override map can fill any
/// `None` slot at runtime, but unknown combos with no override are
/// a hard error.
///
/// Strawman v2 entries (USD per million tokens) per Design § Q7:
///
/// - **Anthropic Claude Opus 4.7**: input $15 / output $75 / cached $1.50.
/// - **Anthropic Claude Haiku 4.5**: input $1 / output $5 / cached $0.10.
/// - **OpenAI GPT-5**: input $10 / output $40 / cached $2.50.
/// - **OpenAI GPT-5 mini**: input $2 / output $8 / cached $0.50.
/// - **Ollama (any model)**: zeros across the board.
#[must_use]
pub fn base_rate(provider: &ProviderKind, model: &str) -> Option<PricePerMillionTokens> {
    match (provider, model) {
        (ProviderKind::Anthropic, "claude-opus-4-7") => Some(PricePerMillionTokens {
            input_usd: dec!(15.00),
            output_usd: dec!(75.00),
            cached_input_usd: dec!(1.50),
        }),
        (ProviderKind::Anthropic, "claude-haiku-4-5-20251001") => Some(PricePerMillionTokens {
            input_usd: dec!(1.00),
            output_usd: dec!(5.00),
            cached_input_usd: dec!(0.10),
        }),
        (ProviderKind::OpenAi, "gpt-5") => Some(PricePerMillionTokens {
            input_usd: dec!(10.00),
            output_usd: dec!(40.00),
            cached_input_usd: dec!(2.50),
        }),
        (ProviderKind::OpenAi, "gpt-5-mini") => Some(PricePerMillionTokens {
            input_usd: dec!(2.00),
            output_usd: dec!(8.00),
            cached_input_usd: dec!(0.50),
        }),
        (ProviderKind::Other(name), _) if name == "ollama" => Some(PricePerMillionTokens::zero()),
        // M6 / T1922 — research mode wraps `ReplayProvider` (whose
        // `provider_kind() = Other("replay")`) inside the factory's
        // `BudgetedProvider`. Replay calls are deterministic + free
        // by definition (product.md line 292 — "no LLM cost (cached
        // responses replay)"); reporting `usd: $0` for the cost
        // event keeps the audit ledger and the cockpit cost tile
        // consistent across modes.
        (ProviderKind::Other(name), _) if name == "replay" => Some(PricePerMillionTokens::zero()),
        _ => None,
    }
}

/// Resolve the effective rate for `(provider, model)` against the
/// override map + base table.
///
/// Lookup order:
/// 1. `overrides[provider_key][model]` — if present, return it.
/// 2. [`base_rate`] — if present, return it.
/// 3. None → [`LlmError::Provider`] with a clear message naming the
///    missing model id (operator gets actionable text without
///    re-reading source).
///
/// # Errors
///
/// Returns [`LlmError::Provider`] when the `(provider, model)` combo
/// is neither overridden nor in the base table.
pub fn resolve_rate(
    overrides: &OverrideMap,
    provider: &ProviderKind,
    model: &str,
) -> Result<PricePerMillionTokens, LlmError> {
    let key = provider_key(provider);
    if let Some(per_provider) = overrides.get(&key)
        && let Some(rate) = per_provider.get(model)
    {
        return Ok(rate.clone());
    }
    base_rate(provider, model).ok_or_else(|| LlmError::Provider {
        provider: provider.clone(),
        message: format!("no price for model {model}"),
    })
}

/// Compute the USD billed for a call given the token usage and rate.
///
/// Formula (Decimal — no float):
///
/// ```text
/// usd = ((tokens_in - tokens_cached_in) * input_usd
///        + tokens_cached_in           * cached_input_usd
///        + tokens_out                 * output_usd) / 1_000_000
/// ```
///
/// `tokens_cached_in` is treated as a subset of `tokens_in`; a
/// pathological provider that reports `cached > in` is clamped
/// (`tokens_in - tokens_cached_in` saturates at 0).
#[must_use]
pub fn cost_for_usage(
    rate: &PricePerMillionTokens,
    tokens_in: u64,
    tokens_out: u64,
    tokens_cached_in: u64,
) -> Decimal {
    let uncached_in = tokens_in.saturating_sub(tokens_cached_in);
    let per_million = Decimal::from(1_000_000u64);
    (Decimal::from(uncached_in) * rate.input_usd
        + Decimal::from(tokens_cached_in) * rate.cached_input_usd
        + Decimal::from(tokens_out) * rate.output_usd)
        / per_million
}

/// Estimate the worst-case USD before a call (fail-closed per Q6b).
///
/// Used by `BudgetedProvider`'s pre-call gate. Estimates `tokens_in`
/// from the request's serialized JSON byte-length divided by 4 (a
/// standard ~tokens-per-char heuristic — Ollama uses the same shape;
/// Anthropic's `count_tokens` endpoint is free but synchronous and
/// would add ~50ms per call, so we use the heuristic instead) and
/// uses the full `max_tokens` as the output estimate.
///
/// The estimate is intentionally an over-bound: pre-call gate would
/// rather fail-closed (refuse a call we could afford) than fail-open
/// (commit to a call that pushes over the ceiling).
#[must_use]
pub fn estimate_cost(
    rate: &PricePerMillionTokens,
    input_chars: u64,
    max_output_tokens: u64,
) -> Decimal {
    // ~4 chars per token is the AnthropicCookbook / OpenAI tiktoken
    // ballpark for English. Conservative over-estimate.
    let tokens_in_estimate = input_chars / 4 + 32; // padding for tool schemas etc.
    cost_for_usage(rate, tokens_in_estimate, max_output_tokens, 0)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// T1911 (a): every supported `(provider, model)` resolves.
    #[test]
    fn t1911_base_rate_covers_supported_set() {
        for (provider, model) in [
            (ProviderKind::Anthropic, "claude-opus-4-7"),
            (ProviderKind::Anthropic, "claude-haiku-4-5-20251001"),
            (ProviderKind::OpenAi, "gpt-5"),
            (ProviderKind::OpenAi, "gpt-5-mini"),
            (ProviderKind::Other("ollama".to_string()), "llama3"),
            (ProviderKind::Other("ollama".to_string()), "mistral"),
        ] {
            assert!(
                base_rate(&provider, model).is_some(),
                "({provider:?}, {model}) should resolve to a base rate"
            );
        }
    }

    /// T1911 (b): typo'd model id → `None` from base + `LlmError::Provider`
    /// from resolve.
    #[test]
    fn t1911_typo_model_id_returns_provider_error() {
        let bad = "claude-opus-4.7"; // dot instead of dash
        assert!(base_rate(&ProviderKind::Anthropic, bad).is_none());
        let err = resolve_rate(&OverrideMap::new(), &ProviderKind::Anthropic, bad)
            .expect_err("typo'd model id should error");
        let LlmError::Provider {
            provider, message, ..
        } = err
        else {
            panic!("wrong variant");
        };
        assert!(matches!(provider, ProviderKind::Anthropic));
        assert!(
            message.contains(bad),
            "message should name model: {message}"
        );
    }

    /// T1911 (c): TOML override shadows the base table for the same pair.
    #[test]
    fn t1911_override_shadows_base_rate() {
        let mut overrides: OverrideMap = HashMap::new();
        let mut per_provider: HashMap<String, PricePerMillionTokens> = HashMap::new();
        per_provider.insert(
            "claude-opus-4-7".to_string(),
            PricePerMillionTokens {
                input_usd: dec!(99.99),
                output_usd: dec!(199.99),
                cached_input_usd: dec!(9.99),
            },
        );
        overrides.insert("anthropic".to_string(), per_provider);

        let rate = resolve_rate(&overrides, &ProviderKind::Anthropic, "claude-opus-4-7")
            .expect("override must resolve");
        assert_eq!(rate.input_usd, dec!(99.99));
        assert_eq!(rate.output_usd, dec!(199.99));
        assert_eq!(rate.cached_input_usd, dec!(9.99));
    }

    /// T1911 (d): Ollama base rate is exact `Decimal::ZERO`.
    #[test]
    fn t1911_ollama_zero_rate_exact() {
        let r = base_rate(&ProviderKind::Other("ollama".to_string()), "llama3").unwrap();
        assert_eq!(r.input_usd, Decimal::ZERO);
        assert_eq!(r.output_usd, Decimal::ZERO);
        assert_eq!(r.cached_input_usd, Decimal::ZERO);
    }

    /// `cost_for_usage` round-trip: 1M uncached input + 1M output @ Opus rate
    /// = $15 + $75 = $90.
    #[test]
    fn t1911_cost_for_usage_round_trips() {
        let r = base_rate(&ProviderKind::Anthropic, "claude-opus-4-7").unwrap();
        let usd = cost_for_usage(&r, 1_000_000, 1_000_000, 0);
        assert_eq!(usd, dec!(90));
    }

    /// `cost_for_usage` cached-discount math: 1M cached input + 0 output =
    /// $1.50 (cached rate, not $15 input rate).
    #[test]
    fn t1911_cost_for_usage_cached_discount() {
        let r = base_rate(&ProviderKind::Anthropic, "claude-opus-4-7").unwrap();
        let usd = cost_for_usage(&r, 1_000_000, 0, 1_000_000);
        assert_eq!(usd, dec!(1.50));
    }

    /// Pathological `cached > in` clamps `uncached_in` to 0.
    #[test]
    fn t1911_cost_for_usage_clamps_pathological_cached() {
        let r = PricePerMillionTokens {
            input_usd: dec!(1),
            output_usd: dec!(1),
            cached_input_usd: dec!(0),
        };
        // cached(100) > tokens_in(50) — uncached = 0, so no input bill.
        let usd = cost_for_usage(&r, 50, 0, 100);
        assert_eq!(usd, dec!(0));
    }

    /// `estimate_cost` fail-closed over-estimates.
    #[test]
    fn t1911_estimate_cost_over_estimates() {
        let r = base_rate(&ProviderKind::Anthropic, "claude-haiku-4-5-20251001").unwrap();
        // 1KB input + 100 output tokens. Estimate uses input_chars/4 + 32
        // tokens. We assert it's strictly > a zero-input estimate.
        let estimate = estimate_cost(&r, 1024, 100);
        let zero_input = estimate_cost(&r, 0, 100);
        assert!(
            estimate > zero_input,
            "estimate should grow with input chars: {estimate} vs {zero_input}"
        );
    }
}
