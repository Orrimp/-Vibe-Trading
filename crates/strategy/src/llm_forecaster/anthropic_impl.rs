//! `LlmForecasterImpl` — concrete `LlmForecaster` over `Arc<dyn llm::LlmProvider>` (T-D-N(B1)).
//!
//! ## Responsibilities
//!
//! 1. Build a `ChatRequest` from a `ForecastContext`:
//!    - System prompt via [`crate::llm_forecaster::prompt::build_system_prompt`]
//!      (2 cache breakpoints on Anthropic; flat on others).
//!    - Single user message with the dynamic context (already embedded in the
//!      system prompt's dynamic block — the user message is minimal).
//!    - `temperature = Some(0.0)` pinned (R3.4 / T-D-N(B4)).
//!    - `tools = vec![propose_forecast_schema()]` (T-D-N(B3)).
//!
//! 2. Call `llm::LlmProvider::complete(&request)` on the injected provider.
//!    In tests this is a wiremock-backed `AnthropicProvider`; in production
//!    it is a `BudgetedProvider<RecordingProvider<AnthropicProvider>>`.
//!
//! 3. Decode the `ContentBlock::ToolUse { name: "propose_forecast", input }` from
//!    the response: validate against the schema, extract typed fields, return
//!    `LlmForecast`.
//!
//! ## Temperature pin
//!
//! `temperature = Some(0.0)` is set in `build_request`. This pin is the
//! Layer-1 determinism guarantee in the 5-layer stack (T-AR-5). The backtest
//! replay-cache layer (Layer 2) is the stronger gate — temperature=0 alone
//! is not byte-deterministic across Anthropic server restarts (K4), but it
//! is the correct setting for maximum repeatability within a session.
//!
//! ## Error routing
//!
//! | LlmError variant              | Maps to                            |
//! |-------------------------------|------------------------------------|
//! | `ReplayMiss`                  | `LlmForecasterError::ReplayMiss`   |
//! | `BudgetExceeded`              | `LlmForecasterError::BudgetExceeded` |
//! | `Timeout`                     | `LlmForecasterError::Timeout`      |
//! | `InvalidResponse`             | `LlmForecasterError::InvalidResponse` |
//! | Any other `LlmError`          | `LlmForecasterError::Provider(e)`  |
//!
//! ## Cross-references
//!
//! - `spec/v3-llm-forecaster/decomp.md § T-AR-1` — signal pipeline.
//! - `spec/v3-llm-forecaster/decomp.md § T-AR-2` — prompt + replay-cache contract.
//! - `spec/v3-llm-forecaster/decomp.md § T-AR-5` — determinism contract.
//! - `crates/llm/src/trait_def.rs` — `LlmProvider` + `ChatRequest` / `ChatResponse`.
//! - `crates/llm/src/tools.rs` — `validate_tool_use`.

use std::sync::Arc;

use async_trait::async_trait;
use cost::{AgentRole, LlmTier};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::{debug, warn};

use llm::{
    ChatMessage, ChatRequest, ContentBlock, LlmError, LlmProvider, MessageRole, ModelId,
    validate_tool_use,
};

use super::{
    prompt::build_system_prompt,
    tool_schema::{PROPOSE_FORECAST_TOOL_NAME, propose_forecast_schema},
    trait_def::LlmForecaster,
    types::{
        Confidence, ForecastContext, Horizon, LessonCardRef, LlmForecast, LlmForecasterError,
        Rating,
    },
};

// ── LlmForecasterImpl ─────────────────────────────────────────────────────────

/// Concrete `LlmForecaster` implementation over `Arc<dyn LlmProvider>`.
///
/// Wraps the provider with:
/// - System prompt construction via `CachedSystemPromptBuilder` (2 cache breakpoints).
/// - `propose_forecast` tool-schema enforcement.
/// - `temperature = Some(0.0)` pin (R3.4).
/// - Response decoding into `LlmForecast`.
///
/// In tests, inject an `AnthropicProvider::with_base_url(wiremock_uri, ...)` to
/// avoid real API calls. In production, inject via `LlmProviderFactory::build`.
pub struct LlmForecasterImpl {
    provider: Arc<dyn LlmProvider>,
    /// Resolved model ID for every `ChatRequest`.
    model_id: ModelId,
    /// Cost tier for routing in `BudgetedProvider` + cost attribution.
    tier: LlmTier,
    /// `AgentRole` attribution for the cost-event row.
    role: AgentRole,
}

impl std::fmt::Debug for LlmForecasterImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LlmForecasterImpl")
            .field("model_id", &self.model_id)
            .field("tier", &self.tier)
            .finish()
    }
}

impl LlmForecasterImpl {
    /// Construct an `LlmForecasterImpl`.
    ///
    /// - `provider`: the LLM provider (wrapped in `BudgetedProvider` in
    ///   production; bare `AnthropicProvider::with_base_url(wiremock_uri)` in tests).
    /// - `model_id`: the model to use (e.g. `"claude-haiku-4-5-20251001"`).
    /// - `tier`: `LlmTier::QuickThink` for Haiku; `LlmTier::DeepThink` for Opus.
    #[must_use]
    pub fn new(provider: Arc<dyn LlmProvider>, model_id: impl Into<String>, tier: LlmTier) -> Self {
        Self {
            provider,
            model_id: ModelId::new(model_id),
            tier,
            role: AgentRole::Trader,
        }
    }

    /// Build the `ChatRequest` for this `ForecastContext`.
    ///
    /// - Composes the system prompt (project + role cached, dynamic plain).
    /// - Adds a single user message (the dynamic context is already in the system
    ///   prompt; the user message carries a minimal directive).
    /// - Pins `temperature = Some(0.0)` (T-D-N(B4) / R3.4).
    /// - Sets `tools = vec![propose_forecast_schema()]`.
    fn build_request(&self, ctx: &ForecastContext) -> ChatRequest {
        let provider_kind = self.provider.provider_kind();
        let system = build_system_prompt(ctx, &provider_kind);

        let mut req = ChatRequest::new(self.model_id.clone(), self.tier.clone(), self.role.clone());
        req.system = system;
        req.messages = vec![ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text(
                // Minimal user turn: the system prompt already contains all context.
                // This turn triggers the model to respond with a tool call.
                "Please analyze the above context and call `propose_forecast` with your forecast."
                    .to_string(),
            )],
        }];
        req.tools = vec![propose_forecast_schema()];
        // T-D-N(B4): temperature = Some(0.0) pinned per R3.4 + decomp § T-AR-5 Layer 1.
        req.temperature = Some(0.0);
        // Max tokens: 1024 generous ceiling for the reasoning trace (~400-600 tokens
        // expected per spike analysis) + tool-call overhead.
        req.max_tokens = 1024;
        // Echo the ForecastContext correlation_id so the response can be matched
        // in the replay-cache layer.
        req.correlation_id = ctx.correlation_id;

        debug!(
            target: "llm_forecaster::impl",
            symbol = %ctx.symbol,
            model = %self.model_id,
            temperature = 0.0,
            "built ChatRequest for forecast",
        );

        req
    }

    /// Decode a `ChatResponse` into a `LlmForecast`.
    ///
    /// Finds the first `ContentBlock::ToolUse { name: "propose_forecast", .. }`
    /// block, validates the payload against the schema, and extracts typed fields.
    ///
    /// Returns `LlmForecasterError::InvalidResponse` if:
    /// - No `ToolUse` block exists in the response.
    /// - The tool name is not `"propose_forecast"`.
    /// - The payload fails schema validation (malformed JSON, missing required
    ///   fields, bad confidence range, reasoning trace too short, etc.).
    fn decode_response(
        &self,
        response: llm::ChatResponse,
        ctx: &ForecastContext,
    ) -> Result<LlmForecast, LlmForecasterError> {
        // Find the propose_forecast tool-use block.
        let tool_input = response
            .content
            .iter()
            .find_map(|block| {
                if let ContentBlock::ToolUse { name, input, .. } = block
                    && name == PROPOSE_FORECAST_TOOL_NAME
                {
                    return Some(input.clone());
                }
                None
            })
            .ok_or_else(|| LlmForecasterError::InvalidResponse {
                reason: format!(
                    "response contained no '{}' tool-use block; \
                     content blocks: {:?}",
                    PROPOSE_FORECAST_TOOL_NAME, response.content
                ),
            })?;

        // Validate against the JSON schema.
        let schema = propose_forecast_schema();
        validate_tool_use(&schema, &tool_input).map_err(|e| {
            LlmForecasterError::InvalidResponse {
                reason: format!("tool-use payload failed schema validation: {e}"),
            }
        })?;

        // Extract typed fields.
        let rating_str =
            tool_input["rating"]
                .as_str()
                .ok_or_else(|| LlmForecasterError::InvalidResponse {
                    reason: "missing 'rating'".to_string(),
                })?;
        let rating =
            Rating::try_parse(rating_str).ok_or_else(|| LlmForecasterError::InvalidResponse {
                reason: format!("unknown rating value: {:?}", rating_str),
            })?;

        let confidence_raw = tool_input["confidence"].as_f64().ok_or_else(|| {
            LlmForecasterError::InvalidResponse {
                reason: "missing or non-numeric 'confidence'".to_string(),
            }
        })?;
        // Convert f64 → Decimal safely.  The schema already enforced [0,1] range.
        let confidence_decimal = Decimal::try_from(confidence_raw).unwrap_or(dec!(0.5));
        let confidence = Confidence::new(confidence_decimal.clamp(dec!(0), dec!(1)));

        let reasoning_trace = tool_input["reasoning_trace"]
            .as_str()
            .ok_or_else(|| LlmForecasterError::InvalidResponse {
                reason: "missing 'reasoning_trace'".to_string(),
            })?
            .to_string();

        // Parse cited_lesson_ids (optional field).
        let cited_lessons: Vec<LessonCardRef> = tool_input
            .get("cited_lesson_ids")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str())
                    .map(|id| LessonCardRef {
                        card_id: id.to_string(),
                    })
                    .collect()
            })
            .unwrap_or_default();

        debug!(
            target: "llm_forecaster::impl",
            symbol = %ctx.symbol,
            rating = %rating,
            confidence = %confidence,
            cited_count = cited_lessons.len(),
            "decoded propose_forecast tool-use response",
        );

        Ok(LlmForecast::new(
            ctx.symbol.clone(),
            ctx.now,
            rating,
            confidence,
            Horizon::OneHour,
            reasoning_trace,
            cited_lessons,
            None, // cost_ref: populated at Wave E audit-emission step
            self.name().to_string(),
            ctx.correlation_id,
        ))
    }

    /// Map an `LlmError` from the provider to an `LlmForecasterError`.
    fn map_provider_error(e: LlmError, timeout_ms: u64) -> LlmForecasterError {
        match e {
            LlmError::ReplayMiss { hash, .. } => LlmForecasterError::ReplayMiss { hash },
            LlmError::BudgetExceeded {
                spent_usd,
                ceiling_usd,
            } => LlmForecasterError::BudgetExceeded {
                cap_usd: ceiling_usd,
                actual_usd: spent_usd,
            },
            LlmError::Timeout { elapsed_ms } => {
                warn!(
                    target: "llm_forecaster::impl",
                    elapsed_ms,
                    configured_timeout_ms = timeout_ms,
                    "LLM call timed out"
                );
                LlmForecasterError::Timeout { timeout_ms }
            }
            LlmError::InvalidResponse(msg) => LlmForecasterError::InvalidResponse { reason: msg },
            other => LlmForecasterError::Provider(other),
        }
    }
}

#[async_trait]
impl LlmForecaster for LlmForecasterImpl {
    fn name(&self) -> &str {
        "llm_forecaster_impl"
    }

    async fn forecast(&self, ctx: ForecastContext) -> Result<LlmForecast, LlmForecasterError> {
        debug!(
            target: "llm_forecaster::impl",
            symbol = %ctx.symbol,
            model = %self.model_id,
            n_bars = ctx.recent_bars.len(),
            n_lessons = ctx.top_k_lessons.len(),
            n_decisions = ctx.recent_decisions.len(),
            "calling LlmProvider::complete",
        );

        let request = self.build_request(&ctx);
        let timeout_ms = 45_000u64; // Q5b default; TODO Wave C: from config

        let response = self
            .provider
            .complete(request)
            .await
            .map_err(|e| Self::map_provider_error(e, timeout_ms))?;

        self.decode_response(response, &ctx)
    }
}

// ── Unit tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use llm::{
        AnthropicProvider, ContentBlock as LlmContentBlock, ModelId as LlmModelId, StopReason,
        TokenUsage, trait_def::SystemBlock,
    };
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{Bar, Price, Quantity, Symbol, Timeframe, Timestamp, Venue};
    use uuid::Uuid;

    use crate::llm_forecaster::types::{DEFAULT_MODEL_ID, ForecastContext};

    fn make_ts(epoch_s: i64) -> Timestamp {
        Timestamp::new(OffsetDateTime::from_unix_timestamp(epoch_s).expect("valid ts"))
    }

    fn make_bar(symbol: &str, open_ts_s: i64) -> Bar {
        let sym = Symbol::new(symbol);
        let ts = make_ts(open_ts_s);
        Bar {
            symbol: sym,
            tf: Timeframe::OneHour,
            open_ts: ts,
            close_ts: make_ts(open_ts_s + 3600),
            open: Price::new(dec!(45000)).expect("positive price"),
            high: Price::new(dec!(45100)).expect("positive price"),
            low: Price::new(dec!(44900)).expect("positive price"),
            close: Price::new(dec!(45050)).expect("positive price"),
            volume: Quantity::new(dec!(1000)).expect("positive qty"),
            trade_count: 100,
            local_recv_ts: ts,
            venue: Venue::Binance,
        }
    }

    fn minimal_ctx() -> ForecastContext {
        ForecastContext::test_fixture(
            Symbol::new("BTCUSDT"),
            make_ts(1_700_000_000),
            vec![make_bar("BTCUSDT", 1_700_000_000)],
        )
    }

    fn make_impl_with_bare_provider(base_url: String) -> LlmForecasterImpl {
        let provider = Arc::new(AnthropicProvider::with_base_url(
            base_url,
            "test-key",
            LlmModelId::from(DEFAULT_MODEL_ID),
        ));
        LlmForecasterImpl::new(provider, DEFAULT_MODEL_ID, LlmTier::QuickThink)
    }

    /// build_request emits temperature = Some(0.0) (T-D-N(B4) pin).
    #[test]
    fn build_request_pins_temperature_zero() {
        // Build with a dummy URL — we only inspect the request shape, not make an HTTP call.
        let impl_ = make_impl_with_bare_provider("http://localhost:1".to_string());
        let ctx = minimal_ctx();
        let req = impl_.build_request(&ctx);
        assert_eq!(
            req.temperature,
            Some(0.0),
            "temperature must be pinned to 0.0 per R3.4"
        );
    }

    /// build_request carries exactly 1 tool (propose_forecast).
    #[test]
    fn build_request_carries_exactly_one_tool() {
        let impl_ = make_impl_with_bare_provider("http://localhost:1".to_string());
        let ctx = minimal_ctx();
        let req = impl_.build_request(&ctx);
        assert_eq!(req.tools.len(), 1);
        assert_eq!(req.tools[0].name, PROPOSE_FORECAST_TOOL_NAME);
    }

    /// build_request carries exactly 2 Cached system blocks for Anthropic provider.
    #[test]
    fn build_request_has_two_cached_system_blocks() {
        let impl_ = make_impl_with_bare_provider("http://localhost:1".to_string());
        let ctx = minimal_ctx();
        let req = impl_.build_request(&ctx);
        let cached_count = req
            .system
            .iter()
            .filter(|b| matches!(b, SystemBlock::Cached(_, _)))
            .count();
        assert_eq!(
            cached_count, 2,
            "Anthropic request must have exactly 2 cache breakpoints"
        );
    }

    /// build_request correlation_id matches the ForecastContext.
    #[test]
    fn build_request_echoes_correlation_id() {
        let impl_ = make_impl_with_bare_provider("http://localhost:1".to_string());
        let mut ctx = minimal_ctx();
        let expected_id = Uuid::new_v4();
        ctx.correlation_id = expected_id;
        let req = impl_.build_request(&ctx);
        assert_eq!(req.correlation_id, expected_id);
    }

    /// decode_response: well-formed BUY response → LlmForecast with Rating::Buy.
    #[test]
    fn decode_response_buy_rating() {
        let impl_ = make_impl_with_bare_provider("http://localhost:1".to_string());
        let ctx = minimal_ctx();
        let response = llm::ChatResponse {
            content: vec![LlmContentBlock::ToolUse {
                name: PROPOSE_FORECAST_TOOL_NAME.to_string(),
                input: serde_json::json!({
                    "rating": "BUY",
                    "confidence": 0.75,
                    "horizon": "short",
                    "reasoning_trace": "RSI(14) = 62.3 trending above 60 for 3 bars. MACD histogram positive. BB upper not yet breached. Strong bullish momentum confirmed.",
                    "cited_lesson_ids": ["lc_abc123"]
                }),
                id: "toolu_01".to_string(),
            }],
            stop_reason: StopReason::ToolUse,
            usage: TokenUsage {
                tokens_in: 5876,
                tokens_out: 412,
                tokens_cached_in: 2000,
            },
            model: LlmModelId::from(DEFAULT_MODEL_ID),
            correlation_id: ctx.correlation_id,
        };
        let forecast = impl_.decode_response(response, &ctx).expect("decode ok");
        assert_eq!(forecast.rating, Rating::Buy);
        assert_eq!(forecast.cited_lessons.len(), 1);
        assert_eq!(forecast.cited_lessons[0].card_id, "lc_abc123");
        assert!(!forecast.reasoning_trace.is_empty());
    }

    /// decode_response: malformed response missing tool-use block → InvalidResponse.
    #[test]
    fn decode_response_missing_tool_use_block() {
        let impl_ = make_impl_with_bare_provider("http://localhost:1".to_string());
        let ctx = minimal_ctx();
        let response = llm::ChatResponse {
            content: vec![LlmContentBlock::Text(
                "Here is my forecast: BUY".to_string(),
            )],
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage {
                tokens_in: 1000,
                tokens_out: 20,
                tokens_cached_in: 0,
            },
            model: LlmModelId::from(DEFAULT_MODEL_ID),
            correlation_id: ctx.correlation_id,
        };
        let err = impl_
            .decode_response(response, &ctx)
            .expect_err("should fail");
        assert!(
            matches!(err, LlmForecasterError::InvalidResponse { .. }),
            "missing tool-use block must produce InvalidResponse"
        );
    }

    /// decode_response: unknown rating in tool payload → InvalidResponse.
    #[test]
    fn decode_response_unknown_rating() {
        let impl_ = make_impl_with_bare_provider("http://localhost:1".to_string());
        let ctx = minimal_ctx();
        let response = llm::ChatResponse {
            content: vec![LlmContentBlock::ToolUse {
                name: PROPOSE_FORECAST_TOOL_NAME.to_string(),
                input: serde_json::json!({
                    "rating": "SUPER_BULLISH",
                    "confidence": 0.9,
                    "horizon": "short",
                    "reasoning_trace": "Extremely bullish momentum confirmed by all indicators above thresholds."
                }),
                id: "toolu_01".to_string(),
            }],
            stop_reason: StopReason::ToolUse,
            usage: TokenUsage {
                tokens_in: 5000,
                tokens_out: 200,
                tokens_cached_in: 0,
            },
            model: LlmModelId::from(DEFAULT_MODEL_ID),
            correlation_id: ctx.correlation_id,
        };
        let err = impl_
            .decode_response(response, &ctx)
            .expect_err("should fail");
        // Schema validation should catch the unknown enum value.
        assert!(
            matches!(err, LlmForecasterError::InvalidResponse { .. }),
            "unknown rating must produce InvalidResponse"
        );
    }

    /// map_provider_error: ReplayMiss maps to LlmForecasterError::ReplayMiss.
    #[test]
    fn map_provider_error_replay_miss() {
        let e = LlmError::ReplayMiss {
            hash: "abc123".to_string(),
            provider: cost::ProviderKind::Anthropic,
            model: "claude-haiku-4-5-20251001".to_string(),
        };
        let mapped = LlmForecasterImpl::map_provider_error(e, 45_000);
        assert!(matches!(&mapped, LlmForecasterError::ReplayMiss { hash, .. } if hash == "abc123"));
        assert!(
            mapped.is_backtest_fatal(),
            "ReplayMiss must be backtest-fatal"
        );
    }

    /// map_provider_error: BudgetExceeded maps correctly.
    #[test]
    fn map_provider_error_budget_exceeded() {
        let e = LlmError::BudgetExceeded {
            spent_usd: dec!(95.0),
            ceiling_usd: dec!(100.0),
        };
        let mapped = LlmForecasterImpl::map_provider_error(e, 45_000);
        assert!(
            matches!(
                mapped,
                LlmForecasterError::BudgetExceeded {
                    cap_usd,
                    actual_usd,
                } if cap_usd == dec!(100.0) && actual_usd == dec!(95.0)
            ),
            "BudgetExceeded must map fields correctly"
        );
        assert!(
            mapped.is_backtest_fatal(),
            "BudgetExceeded must be backtest-fatal"
        );
    }
}
