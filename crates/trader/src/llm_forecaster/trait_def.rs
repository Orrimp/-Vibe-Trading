//! `LlmForecaster` async trait — the single callable entry-point for
//! all LLM-forecast implementations (Anthropic, Ollama, stub).
//!
//! ## Design
//!
//! The trait is deliberately narrow: it takes a [`ForecastContext`] (the
//! full prompt payload assembled by `LlmForecasterStrategy::on_bar`) and
//! returns a [`LlmForecast`] (rating + confidence + reasoning trace +
//! cited lesson cards + cost ref). All infra concerns (budget gating,
//! replay-cache lookup, system-prompt composition, tool-schema validation)
//! live in `anthropic_impl.rs` (`LlmForecasterImpl`) — not here.
//!
//! ## Determinism
//!
//! All implementations MUST:
//! - Pin `temperature = Some(0.0)` at every call site (R3.4).
//! - Route through `crates/llm::ReplayProvider` in backtest / research mode
//!   (R6.2); a cache-miss is `LlmForecasterError::ReplayMiss` (fatal).
//! - Use the deterministic `ForecastContext::request_hash()` as the
//!   `(request_hash, response)` sqlite cache key (R6.6).
//!
//! ## Cross-references
//!
//! - `spec/v1/v3-llm-forecaster/decomp.md § T-AR-1` — signal-pipeline shape.
//! - `spec/v1/v3-llm-forecaster/decomp.md § T-AR-2` — prompt + replay-cache contract.
//! - `crates/llm/src/trait_def.rs` — `LlmProvider` trait (infrastructure layer).

use async_trait::async_trait;

use super::types::{ForecastContext, LlmForecast, LlmForecasterError};

/// Async trait for LLM-based forecasters.
///
/// Implementations wrap an `Arc<dyn llm::LlmProvider>` and handle
/// prompt composition, tool-schema validation, and response decoding.
/// The `Strategy` consumer calls `forecast()` synchronously via
/// `tokio::runtime::Handle::block_on` from within `on_bar`.
///
/// ## Send + Sync + 'static bounds
///
/// Required so `LlmForecasterStrategy` (which holds an
/// `Arc<dyn LlmForecaster>`) is itself `Send + Sync` and can be
/// placed in a `Box<dyn Strategy>` registry slot.
///
/// ## Wave A note
///
/// In Wave A this trait is type-only. The concrete `LlmForecasterImpl`
/// (Wave B) will implement it. Wave A ships a `StubForecaster` for tests.
#[async_trait]
pub trait LlmForecaster: Send + Sync + 'static {
    /// Human-readable name of this forecaster implementation.
    ///
    /// Used in log / metric labels and in the `LlmForecast::forecaster_name`
    /// field so the operator can tell Haiku vs Sonnet vs stub in the audit ledger.
    fn name(&self) -> &str;

    /// Produce a directional forecast for `ctx.symbol` at `ctx.now`.
    ///
    /// The implementation is responsible for:
    /// 1. Computing `ctx.request_hash()` → cache lookup.
    /// 2. Returning `Err(LlmForecasterError::ReplayMiss)` on cache-miss
    ///    in backtest / research mode.
    /// 3. Pinning `temperature = Some(0.0)` on every `ChatRequest`.
    /// 4. Validating the `propose_forecast` tool-use response against the
    ///    JSON schema before decoding into `LlmForecast`.
    ///
    /// # Errors
    ///
    /// - `LlmForecasterError::ReplayMiss` — cache-miss in replay mode.
    /// - `LlmForecasterError::BudgetExceeded` — per-call or per-backtest cap hit.
    /// - `LlmForecasterError::InvalidResponse` — tool-use payload fails schema validation.
    /// - `LlmForecasterError::Provider(e)` — upstream `LlmProvider::complete` error.
    async fn forecast(&self, ctx: ForecastContext) -> Result<LlmForecast, LlmForecasterError>;
}
