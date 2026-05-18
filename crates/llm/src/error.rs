//! `LlmError` — 8-variant error enum per Design Q4f.
//!
//! Variants carry structured data (not opaque strings) so the cockpit
//! alert renderer (R11) and audit memo (R11.1) can surface specifics
//! without re-parsing. `Display` impls are hand-checked for R8.3
//! (no API-key substring leaks via `Debug` / `Display`).

use rust_decimal::Decimal;
use thiserror::Error;

use crate::ProviderKind;

/// All failure modes the [`crate::LlmProvider::complete`] surface can return.
///
/// One variant per consumer-side error route (see Design § Q4f). Each variant
/// is structured (not stringly-typed) so consumers and the audit memo can
/// switch on the cause without re-parsing.
#[derive(Debug, Error)]
pub enum LlmError {
    /// A provider returned an error response. `message` is a sanitized
    /// summary — never the raw HTTP body — so accidental key echoes from the
    /// provider don't leak into logs.
    #[error("provider error ({provider:?}): {message}")]
    Provider {
        /// Which provider produced the error.
        provider: ProviderKind,
        /// Sanitized error message.
        message: String,
    },

    /// Rate-limit was hit and the retry policy in `retry.rs` (T1902) gave up
    /// after `retries` attempts.
    #[error("rate limited after {retries} retries")]
    RateLimited {
        /// Number of retries attempted before surfacing.
        retries: u8,
    },

    /// The provider call exceeded the configured per-call timeout.
    #[error("timeout after {elapsed_ms}ms")]
    Timeout {
        /// How long the call ran before the timeout fired, in milliseconds.
        elapsed_ms: u64,
    },

    /// Pre-call budget gate refused the request — running spend plus the
    /// reservation estimate would exceed the monthly ceiling. The
    /// `BudgetedProvider` decorator (T1908) is the only emitter.
    #[error("budget exceeded: spent {spent_usd} of {ceiling_usd}")]
    BudgetExceeded {
        /// Already-reserved spend, in USD.
        spent_usd: Decimal,
        /// Monthly ceiling, in USD.
        ceiling_usd: Decimal,
    },

    /// The provider's response didn't match the expected shape (missing
    /// fields, malformed tool-use payload, schema validation failure).
    /// Consumer-side: route to the prompt author for fix-forward.
    #[error("invalid response: {0}")]
    InvalidResponse(String),

    /// Research-mode `ReplayProvider` (T1922) was asked for a request hash
    /// that isn't in the replay cache. **Strict-only at v2.0.0** per
    /// operator-locked decision D2 — surfaces as a loud test failure,
    /// never silently falls through to a live API call.
    ///
    /// Carries `provider` + `model` alongside the request hash so the
    /// research-mode operator knows which fixture row the run was looking
    /// for (the smallest possible repro: "open `crates/llm/fixtures/replay-v1.db`,
    /// `SELECT * FROM llm_replay WHERE provider = ? AND model = ?`"). The
    /// fields are the values the `ReplayProvider` resolved from the
    /// `ChatRequest` at miss-time — provider from `provider_kind()`, model
    /// from the request's `model: ModelId`. M6, T1919 (pass 5) refined the
    /// pass-1 stub `ReplayMiss(String)` to this struct shape.
    #[error("replay miss: provider={provider:?} model={model} hash={hash}")]
    ReplayMiss {
        /// SHA-256 hex of the canonical-JSON request body (the lookup key).
        hash: String,
        /// Provider the failed lookup targeted (i.e. the wrapper's
        /// `provider_kind()`).
        provider: ProviderKind,
        /// `ChatRequest::model.as_str()` at miss-time.
        model: String,
    },

    /// Transport-level HTTP error from `reqwest`.
    #[error(transparent)]
    Network(#[from] reqwest::Error),

    /// Startup misconfig — credential missing, malformed, or refused by
    /// the provider's auth endpoint. Surfaces at factory time (T1913),
    /// not at first call.
    #[error("auth: {0}")]
    Auth(String),
}

/// Lift a `cost::BudgetError` from the pre-call gate into the
/// LLM-domain error surface so the caller's `?` chain is uniform.
impl From<cost::BudgetError> for LlmError {
    fn from(err: cost::BudgetError) -> Self {
        match err {
            cost::BudgetError::BudgetExceeded {
                spent_usd,
                ceiling_usd,
            } => LlmError::BudgetExceeded {
                spent_usd,
                ceiling_usd,
            },
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// T1901 acceptance (b): every `LlmError` variant has a non-empty
    /// `Display` impl.
    #[test]
    fn t1901_every_llmerror_variant_has_nonempty_display() {
        // Manually construct one of each variant — if a new variant is added
        // and this test isn't updated, the `match` below fails to compile.
        let variants: Vec<LlmError> = vec![
            LlmError::Provider {
                provider: ProviderKind::Anthropic,
                message: "context length exceeded".to_string(),
            },
            LlmError::RateLimited { retries: 3 },
            LlmError::Timeout { elapsed_ms: 30_000 },
            LlmError::BudgetExceeded {
                spent_usd: dec!(180.00),
                ceiling_usd: dec!(200.00),
            },
            LlmError::InvalidResponse("missing tool_use block".to_string()),
            LlmError::ReplayMiss {
                hash: "abc123def456".to_string(),
                provider: ProviderKind::Anthropic,
                model: "claude-opus-4-7".to_string(),
            },
            // `Network` variant is constructed via the From<reqwest::Error>
            // impl in real code; we synthesize one here by forcing a
            // request build error.
            LlmError::Network(
                reqwest::Client::new()
                    .request(reqwest::Method::GET, "not a valid url")
                    .build()
                    .unwrap_err(),
            ),
            LlmError::Auth("ANTHROPIC_API_KEY missing".to_string()),
        ];

        // Exhaustiveness check via `match` — compile fails if a variant is
        // added without updating this test.
        for v in &variants {
            match v {
                LlmError::Provider { .. }
                | LlmError::RateLimited { .. }
                | LlmError::Timeout { .. }
                | LlmError::BudgetExceeded { .. }
                | LlmError::InvalidResponse(_)
                | LlmError::ReplayMiss { .. }
                | LlmError::Network(_)
                | LlmError::Auth(_) => {}
            }
            let rendered = format!("{v}");
            assert!(
                !rendered.is_empty(),
                "Display impl produced empty string for variant: {v:?}"
            );
        }

        // Sanity-check structured field rendering.
        assert!(
            format!(
                "{}",
                LlmError::Provider {
                    provider: ProviderKind::OpenAi,
                    message: "rate".to_string()
                }
            )
            .contains("OpenAi")
        );
        assert!(format!("{}", LlmError::RateLimited { retries: 5 }).contains('5'));
    }
}
