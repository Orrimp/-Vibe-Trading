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

use agent::{ActivityKind, ActivitySender};
use async_trait::async_trait;
use cost::{AgentRole, LlmTier};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::{debug, warn};

use llm::{
    ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmError, LlmProvider, MessageRole,
    ModelId, validate_tool_use,
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

// ── Label constant (R2.4 / T-AR-3) ────────────────────────────────────────────

/// Activity-tape label prefix for LLM calls (K6 / PII-redaction contract).
///
/// The full label is `ACTIVITY_LABEL_PREFIX + model_id`. Only `self.model_id`
/// (a construction-time constant) is concatenated — no field of
/// `ForecastContext`, `LlmRequest`, `Bar`, or `LessonCard` flows into the
/// label. PII / prompt-content leakage is structurally impossible at v0.1.1.
/// Maximum label length: 10 + 32 (longest Anthropic model ID) = 42 chars,
/// well within the parent's 64-char budget (ADR-0042 § R1.2).
const ACTIVITY_LABEL_PREFIX: &str = "LLM call: ";

// ── LlmForecasterImpl ─────────────────────────────────────────────────────────

/// Concrete `LlmForecaster` implementation over `Arc<dyn LlmProvider>`.
///
/// Wraps the provider with:
/// - System prompt construction via `CachedSystemPromptBuilder` (2 cache breakpoints).
/// - `propose_forecast` tool-schema enforcement.
/// - `temperature = Some(0.0)` pin (R3.4).
/// - Response decoding into `LlmForecast`.
/// - Audit-row emission via `audit::journal::post_llm_forecast` (Wave E, R7.1).
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
    /// Optional audit ledger handle. When `Some`, every successful `forecast()`
    /// call fires a `post_llm_forecast` audit row (R7.1.2) + `LlmForecastEmitted`
    /// tick (R7.1.3) via `tokio::spawn` (fire-and-forget). When `None`, no audit
    /// row is written (backtest replay or test mode without a ledger).
    audit_ledger: Option<Arc<audit::Ledger>>,
    /// Optional activity-tape producer (cockpit-activity-llm-producer v0.1.1 R1.2).
    ///
    /// When `Some`, each `forecast()` call emits `ActivityKind::LlmCall` Start
    /// and End events on the bus so the status bar tape shows in-flight LLM calls.
    /// When `None` (all existing tests + backtest bin paths + `llm_verdict` CLI),
    /// the activity path is a no-op — zero events emitted, zero perf impact.
    /// Injected via `.with_activity_sender(sender)` builder (R5.2 / K6).
    activity_sender: Option<ActivitySender>,
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
    ///
    /// No audit ledger wired — `post_llm_forecast` is not called on forecast
    /// calls. Use [`Self::with_audit_ledger`] to add audit wiring.
    #[must_use]
    pub fn new(provider: Arc<dyn LlmProvider>, model_id: impl Into<String>, tier: LlmTier) -> Self {
        Self {
            provider,
            model_id: ModelId::new(model_id),
            tier,
            role: AgentRole::Trader,
            audit_ledger: None,
            activity_sender: None,
        }
    }

    /// Construct an `LlmForecasterImpl` with an audit ledger wired.
    ///
    /// Every successful `forecast()` call fires `post_llm_forecast` (R7.1.2)
    /// via `tokio::spawn` (fire-and-forget). The tick (R7.1.3) fires from
    /// inside `post_llm_forecast`. See [`audit::journal::post_llm_forecast`].
    #[must_use]
    pub fn with_audit_ledger(
        provider: Arc<dyn LlmProvider>,
        model_id: impl Into<String>,
        tier: LlmTier,
        audit_ledger: Arc<audit::Ledger>,
    ) -> Self {
        Self {
            provider,
            model_id: ModelId::new(model_id),
            tier,
            role: AgentRole::Trader,
            audit_ledger: Some(audit_ledger),
            activity_sender: None,
        }
    }

    /// Wire the cockpit activity-tape producer for LLM calls (R1.2 / T-D-N1).
    ///
    /// Returns `self` for chaining after `new()` or `with_audit_ledger()`:
    ///
    /// ```rust,ignore
    /// let forecaster = LlmForecasterImpl::new(provider, model_id, tier)
    ///     .with_activity_sender(bus.activity());
    /// ```
    ///
    /// When wired, each `forecast()` call emits `ActivityKind::LlmCall` Start
    /// and End events on the bus broadcast channel. When not wired (the default),
    /// `forecast()` is byte-identical to v0.1.0 — zero events, zero overhead.
    #[must_use]
    pub fn with_activity_sender(mut self, sender: ActivitySender) -> Self {
        self.activity_sender = Some(sender);
        self
    }

    /// Fire-and-forget audit row emission after a successful forecast.
    ///
    /// Spawns a tokio task to call `post_llm_forecast`. No-op when
    /// `audit_ledger` is `None`.
    fn spawn_audit_row(&self, forecast: &LlmForecast, response: &ChatResponse) {
        let Some(ledger) = self.audit_ledger.as_ref() else {
            return;
        };
        let ledger = Arc::clone(ledger);

        // Extract fields needed for post_llm_forecast.
        let strategy_id = "llm_forecaster_v3";
        let symbol = forecast.symbol.0.to_string();
        let correlation_id = forecast.correlation_id;
        let rating = forecast.rating.as_str().to_string();
        let confidence = forecast.confidence.value();
        let horizon = "one_hour";
        let reasoning_trace = forecast.reasoning_trace.clone();
        let trace_sha256 = forecast.reasoning_trace_sha256_hex();
        let cited_json = serde_json::to_string(
            &forecast
                .cited_lessons
                .iter()
                .map(|l| &l.card_id)
                .collect::<Vec<_>>(),
        )
        .unwrap_or_else(|_| "[]".to_string());
        let tokens_in = response.usage.tokens_in as i64;
        let tokens_out = response.usage.tokens_out as i64;
        let tokens_cached_in = response.usage.tokens_cached_in as i64;
        // Cost = 0 here — the actual cost is tracked by BudgetedProvider +
        // LedgerCostSink. The `cost_usd` field in the audit row is advisory
        // (the double-entry ledger is authoritative). Populate with 0 for now.
        let cost_usd = dec!(0);
        let forecaster_name = self.name().to_string();
        let model_id = response.model.as_str().to_string();

        tokio::spawn(async move {
            let write = audit::journal::LlmForecastWrite {
                strategy_id,
                symbol: &symbol,
                correlation_id,
                rating: &rating,
                confidence,
                horizon,
                reasoning_trace: &reasoning_trace,
                trace_sha256: &trace_sha256,
                cited_lesson_ids_json: &cited_json,
                tokens_in,
                tokens_out,
                tokens_cached_in,
                cost_usd,
                forecaster_name: &forecaster_name,
                model_id: &model_id,
                ts: None,
            };
            if let Err(e) = audit::journal::post_llm_forecast(&ledger, &write).await {
                tracing::error!(
                    target: "llm_forecaster::impl",
                    error = %e,
                    "post_llm_forecast failed; forecast was recorded but audit row missing"
                );
            }
        });
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
        let timeout_ms = 45_000u64; // Q5b architect-locked (45s; Anthropic Sonnet p99 + safety)

        // ── Activity-tape wire-up (cockpit-activity-llm-producer v0.1.1 R1.2) ──
        //
        // The handle is created BEFORE the `.await` and explicitly dropped
        // BEFORE any subsequent call so the `!Send` `ActivityHandle` (which
        // uses `Cell<_>` internally — see `crates/agent/src/activity.rs:177-179`)
        // never crosses an `.await` boundary. The resulting `forecast` future
        // stays `Send` for `async-trait` (T-AR-2 / H1 falsification probe).
        //
        // INVARIANT: `drop(activity)` MUST remain before `decode_response` and
        // `spawn_audit_row` with no intervening `.await`. If a future refactor
        // introduces an `.await` between handle creation and `drop(activity)`,
        // `cargo build -p trader` will fail with "future is not `Send`" (K7).
        let activity = self.activity_sender.as_ref().map(|s| {
            s.start(
                ActivityKind::LlmCall,
                format!("{ACTIVITY_LABEL_PREFIX}{}", self.model_id),
            )
        });

        let response_result = self
            .provider
            .complete(request)
            .await
            .map_err(|e| Self::map_provider_error(e, timeout_ms));

        // Map each error variant to a human-readable fail-reason string for the
        // activity tape's red 3-second hold (R4.1 / Q3=(a) default).
        // The mapped reason is captured in `ActivityEvent::End(Failed(reason))`
        // and available for structured logging; it is NOT rendered in the tape
        // UI widget (parent R4.2 — the tape shows the label, not the reason).
        if let (Some(handle), Err(err)) = (&activity, &response_result) {
            let reason = match err {
                LlmForecasterError::Provider(LlmError::Network(_)) => "network error".to_string(),
                LlmForecasterError::Provider(LlmError::Auth(_)) => "auth error".to_string(),
                LlmForecasterError::Provider(LlmError::RateLimited { .. }) => {
                    "rate limited".to_string()
                }
                LlmForecasterError::Provider(LlmError::Provider { .. }) => {
                    "server error".to_string()
                }
                LlmForecasterError::Provider(LlmError::BudgetExceeded { .. }) => {
                    "budget cap".to_string()
                }
                LlmForecasterError::Timeout { timeout_ms: ms } => {
                    format!("timeout {ms}ms")
                }
                LlmForecasterError::InvalidResponse { reason } => {
                    format!("invalid response: {reason}")
                }
                LlmForecasterError::BudgetExceeded { .. } => "budget cap".to_string(),
                _ => "provider error".to_string(),
            };
            handle.fail(reason);
        }
        // Explicit drop here emits `End { Success }` or `End { Failed(reason) }`.
        // MUST precede `decode_response` and `spawn_audit_row` (no `.await` between).
        drop(activity);

        let response = response_result?;

        // Decode the response. Keep a reference to the raw response for audit
        // emission (token counts, model id) — `decode_response` clones what it
        // needs but does not take ownership of the whole response.
        let forecast = self.decode_response(response.clone(), &ctx)?;

        // Wave E: emit audit row + AuditTick (R7.1.2 + R7.1.3). Fire-and-forget.
        self.spawn_audit_row(&forecast, &response);

        Ok(forecast)
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
