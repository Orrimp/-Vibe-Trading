//! LLM-cache observability helper (T1909).
//!
//! Per Design § Q5d: each successful `complete()` post-call hook fires
//! [`emit_cache_event`], which
//!
//! - increments two `metrics` counters labeled by role:
//!   - `llm_cache_input_tokens_total{role}` — every billed input token.
//!   - `llm_cache_hit_tokens_total{role}` — the subset served from cache.
//!
//!   The agent's `metrics-exporter-prometheus` exporter (T27) surfaces
//!   them at `/metrics` as a counter pair the cockpit's Prometheus scrape
//!   reads directly — no additional renderer-side ratio math needed.
//! - emits one `tracing::info!(target: "llm.cache")` event carrying
//!   `tokens_in`, `tokens_cached_in`, and the derived `hit_ratio` for
//!   structured-log forensics.
//!
//! **Divergence from feature.md** (flagged 2026-05-12 by developer): the
//! design note literally says
//! `once_cell::sync::Lazy<prometheus::CounterVec>`. The workspace
//! standardised on the `metrics` façade crate (see
//! `crates/agent/src/observability.rs:10`) which routes to the same
//! Prometheus exporter — using `metrics::counter!` matches that
//! convention and avoids splitting the metrics surface across two
//! Prometheus client crates. Same `/metrics` output, same operator
//! experience, one less dep.
//!
//! Float arithmetic is allowed in this helper because the `hit_ratio`
//! field on the `tracing` event is `f64` by convention (operator-visible
//! forensic display); the audit ledger's authoritative ratio (R9.5) is
//! computed Decimal-only via `audit::query::cache_hit_ratio_since`
//! (T1910), so this `f64` value never feeds Decimal-typed math.

use cost::AgentRole;

const TARGET_CACHE: &str = "llm.cache";
const METRIC_INPUT_TOKENS: &str = "llm_cache_input_tokens_total";
const METRIC_HIT_TOKENS: &str = "llm_cache_hit_tokens_total";

/// Stable lowercase label for an [`AgentRole`].
///
/// Mirrors `AgentRole`'s serde `rename_all = "snake_case"` shape so the
/// Prometheus label values align with the on-disk serialization. Custom
/// `Other(name)` roles surface as `other:<name>` to keep them
/// distinguishable from the four canonical roles.
fn role_label(role: &AgentRole) -> String {
    match role {
        AgentRole::Trader => "trader".to_string(),
        AgentRole::SentimentAnalyst => "sentiment_analyst".to_string(),
        AgentRole::RiskManager => "risk_manager".to_string(),
        AgentRole::PortfolioManager => "portfolio_manager".to_string(),
        AgentRole::Other(name) => format!("other:{name}"),
    }
}

/// Emit a cache-observability event for one completed LLM call.
///
/// - `tokens_in` — total billed input tokens for the call.
/// - `tokens_cached_in` — subset served from the provider's prompt cache
///   (Anthropic's `cache_read_input_tokens`; 0 for providers that don't
///   surface cached input).
///
/// Fired from each successful provider impl's post-response handler in
/// `BudgetedProvider` (T1912). Failed calls do NOT fire (R9.3 — no
/// cost events on failed calls; cache observability rides the same
/// hook).
#[allow(clippy::cast_precision_loss, clippy::float_arithmetic)] // forensic ratio only — see module doc
pub fn emit_cache_event(role: &AgentRole, tokens_in: u64, tokens_cached_in: u64) {
    let role_label = role_label(role);

    // Prometheus counter pair (counter-by-name with label set, per the
    // `metrics` façade's `counter!` macro idiom).
    metrics::counter!(METRIC_INPUT_TOKENS, "role" => role_label.clone()).increment(tokens_in);
    metrics::counter!(METRIC_HIT_TOKENS, "role" => role_label.clone()).increment(tokens_cached_in);

    let hit_ratio: f64 = if tokens_in > 0 {
        // float arithmetic intentional — see module doc.
        (tokens_cached_in as f64) / (tokens_in as f64)
    } else {
        0.0
    };

    tracing::info!(
        target: TARGET_CACHE,
        role = %role_label,
        tokens_in,
        tokens_cached_in,
        hit_ratio,
        "llm.cache.event"
    );
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Role-labelling matches the serde wire form of `AgentRole`.
    #[test]
    fn t1909_role_label_matches_snake_case() {
        assert_eq!(role_label(&AgentRole::Trader), "trader");
        assert_eq!(
            role_label(&AgentRole::SentimentAnalyst),
            "sentiment_analyst"
        );
        assert_eq!(role_label(&AgentRole::RiskManager), "risk_manager");
        assert_eq!(
            role_label(&AgentRole::PortfolioManager),
            "portfolio_manager"
        );
        assert_eq!(
            role_label(&AgentRole::Other("post_mortem".into())),
            "other:post_mortem"
        );
    }

    /// `emit_cache_event` is safe to call with no `metrics` recorder
    /// installed (the global recorder is a no-op until an exporter is
    /// installed). Smoke-tests the panic-free contract.
    #[test]
    fn t1909_emit_cache_event_panic_free_without_recorder() {
        // No global recorder is installed in this test process; the
        // `metrics` macros become no-ops. We just verify the call returns
        // cleanly with no panics.
        emit_cache_event(&AgentRole::Trader, 1_000, 750);
        emit_cache_event(&AgentRole::Trader, 0, 0);
    }
}
