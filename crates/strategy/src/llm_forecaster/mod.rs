//! LLM-based directional forecasting strategy (v3-llm-forecaster v0.1.0).
//!
//! ## Module layout (Wave A foundation)
//!
//! ```text
//! crates/strategy/src/llm_forecaster/
//! ├── mod.rs              ← this file (re-exports + module docs)
//! ├── trait_def.rs        ← `LlmForecaster: Send + Sync + 'static` async trait
//! ├── types.rs            ← `LlmForecast`, `ForecastContext`, `Rating`,
//! │                          `Confidence`, `Horizon`, `LlmForecasterError`,
//! │                          `LessonCardRef`, `CostEventRef`, `StubForecaster`
//! ├── canonicalize.rs     ← `request_hash` SHA-256 helpers
//! └── strategy.rs         ← `LlmForecasterStrategy: Strategy` (Wave A skeleton)
//! ```
//!
//! ## Wave plan summary
//!
//! - **Wave A** (this file): type-level foundation. `on_bar` is a stub
//!   returning `Vec::new()` when disabled (default) or delegating to
//!   `StubForecaster` in tests. No real LLM calls.
//! - **Wave B**: `anthropic_impl.rs` — real `LlmForecasterImpl` over
//!   `Arc<dyn llm::LlmProvider>` + prompt composition + tool-schema.
//! - **Wave C**: registry wiring + real `on_bar` loop.
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

pub mod canonicalize;
pub mod strategy;
pub mod trait_def;
pub mod types;

// Wave B / C files (stubs — created when those waves open):
// pub mod anthropic_impl;
// pub mod prompt;
// pub mod tool_schema;
// pub mod verdict;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use strategy::{LlmForecasterStrategy, STRATEGY_ID};
pub use trait_def::LlmForecaster;
pub use types::{
    CACHE_SCHEMA_VERSION, Confidence, CostEventRef, DEFAULT_FIRE_EVERY_N_BARS, DEFAULT_MODEL_ID,
    DEFAULT_TIMEOUT_MS, ForecastContext, Horizon, LessonCardRef, LlmForecast, LlmForecasterConfig,
    LlmForecasterError, PROMPT_TEMPLATE_VERSION, Rating, RecentDecision, StubForecaster,
    TOP_K_LESSONS, TechnicalIndicators, UnknownRating,
};
