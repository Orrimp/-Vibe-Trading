//! LLM-based directional forecasting strategy (v3-llm-forecaster v0.1.0).
//!
//! ## Module layout (Wave B — `LlmForecasterImpl` + prompt + schema)
//!
//! ```text
//! crates/strategy/src/llm_forecaster/
//! ├── mod.rs              ← this file (re-exports + module docs)
//! ├── trait_def.rs        ← `LlmForecaster: Send + Sync + 'static` async trait
//! ├── types.rs            ← `LlmForecast`, `ForecastContext`, `Rating`,
//! │                          `Confidence`, `Horizon`, `LlmForecasterError`,
//! │                          `LessonCardRef`, `CostEventRef`, `StubForecaster`
//! ├── canonicalize.rs     ← `request_hash` SHA-256 helpers
//! ├── strategy.rs         ← `LlmForecasterStrategy: Strategy` (Wave A skeleton)
//! ├── anthropic_impl.rs   ← `LlmForecasterImpl` over `Arc<dyn LlmProvider>` (Wave B)
//! ├── prompt.rs           ← system-prompt composition via `CachedSystemPromptBuilder` (Wave B)
//! └── tool_schema.rs      ← `propose_forecast` ToolSchema definition (Wave B)
//! ```
//!
//! ## Wave plan summary
//!
//! - **Wave A** (foundation): type-level + `LlmForecasterStrategy` skeleton.
//! - **Wave B** (this update): `LlmForecasterImpl` over `Arc<dyn llm::LlmProvider>`
//!   + prompt composition (2 cache breakpoints) + `propose_forecast` tool schema
//!   + temperature pin (`temperature = Some(0.0)`) + wiremock integration tests.
//! - **Wave C**: reflection-memory top-K retrieval wiring + real `on_bar` loop.
//! - **Waves D-G**: backtest scenarios, audit wiring, Phase F UI, non-regression.
//!
//! ## Strategy ID
//!
//! `"llm_forecaster_v3"` — registered in `crates/strategy/src/registry.rs`
//! at Wave C. Opt-in via:
//!
//! ```toml
//! # config/agent.toml
//! [[strategies]]
//! kind = "llm_forecaster_v3"
//! enabled = false  # default per R9.3
//! ```
//!
//! ## Determinism
//!
//! - `temperature = Some(0.0)` pinned at every LLM call site (R3.4).
//! - `ForecastContext::request_hash()` is the replay-cache key (R6.6).
//! - `StubForecaster` is deterministic (fixed rating; no I/O).
//! - No `SystemTime::now()` / `Instant::now()` / `chrono::Utc::now()` in
//!   any hot path.
//!
//! ## Cross-references
//!
//! - `spec/v3-llm-forecaster/decomp.md` — architect decomposition.
//! - `spec/architecture/adr/0039-llm-forecaster-verdict-criteria.md` — L0-L4.
//! - `crates/llm/src/trait_def.rs` — `LlmProvider` trait (infra layer).
//! - `crates/reflection/src/lib.rs` — lesson-card retrieval (Wave C).

pub mod anthropic_impl;
pub mod canonicalize;
pub mod prompt;
pub mod strategy;
pub mod tool_schema;
pub mod trait_def;
pub mod types;

// Wave C / D / E / F files (deferred):
// pub mod verdict;  // Wave G — ADR-0039 L0-L4 classifier

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use anthropic_impl::LlmForecasterImpl;
pub use strategy::{LlmForecasterStrategy, STRATEGY_ID};
pub use tool_schema::{PROPOSE_FORECAST_TOOL_NAME, propose_forecast_schema};
pub use trait_def::LlmForecaster;
pub use types::{
    CACHE_SCHEMA_VERSION, Confidence, CostEventRef, DEFAULT_FIRE_EVERY_N_BARS, DEFAULT_MODEL_ID,
    DEFAULT_TIMEOUT_MS, ForecastContext, FromRuntimeError, Horizon, LessonCardRef, LlmForecast,
    LlmForecasterConfig, LlmForecasterError, PROMPT_TEMPLATE_VERSION, Rating, RecentDecision,
    StubForecaster, TOP_K_LESSONS, TechnicalIndicators, UnknownRating,
};
