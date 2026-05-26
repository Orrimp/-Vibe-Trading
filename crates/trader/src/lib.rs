//! Trader crate — runtime decision-synthesis layer.
//!
//! Per ADR-0041 (D1): the `trader` crate sits downstream of the analyst-layer
//! `strategy` crate and is the legitimate consumer of `reflection` retrieval.
//!
//! ## Layer topology (product.md § Trading-time agent roster)
//!
//! ```text
//! analysts (parallel) → researcher debate → trader → risk team → portfolio manager → exec
//!                                            ^^^^^^
//!                                            this crate
//! ```
//!
//! ## Module layout
//!
//! ```text
//! crates/trader/src/
//! ├── lib.rs              ← this file (re-exports + module docs)
//! ├── llm_forecaster/     ← moved from crates/strategy/src/llm_forecaster/
//! │   ├── mod.rs
//! │   ├── trait_def.rs
//! │   ├── types.rs
//! │   ├── canonicalize.rs
//! │   ├── strategy.rs
//! │   ├── anthropic_impl.rs
//! │   ├── prompt.rs
//! │   ├── tool_schema.rs
//! │   └── verdict.rs
//! └── registry_arm.rs     ← register_llm_forecaster_v3 free function
//! ```
//!
//! ## R8.1 layering contract (ADR-0041 § D1)
//!
//! `crates/strategy/` MUST NOT carry a `reflection` path-dep.
//! `crates/trader/` IS the reflection-consumer; both `reflection::retrieve_top_k`
//! and `reflection::ReflectionStore` are structurally legal here.
//!
//! The gate-test at `crates/reflection/tests/no_strategy_caller.rs` enforces
//! this layering invariant at every CI run:
//! - `t1809_no_strategy_crate_consumes_reflection_retrieval` — negative on strategy.
//! - `t1810_trader_crate_owns_reflection_retrieval` — positive on trader.

pub mod llm_forecaster;
pub mod registry_arm;

// ── Public re-exports ─────────────────────────────────────────────────────────

pub use llm_forecaster::{
    CACHE_SCHEMA_VERSION, Confidence, CostEventRef, DEFAULT_FIRE_EVERY_N_BARS, DEFAULT_MODEL_ID,
    DEFAULT_TIMEOUT_MS, ForecastContext, FromRuntimeError, Horizon, LessonCardRef, LlmForecast,
    LlmForecaster, LlmForecasterConfig, LlmForecasterError, LlmForecasterImpl,
    LlmForecasterStrategy, PROMPT_TEMPLATE_VERSION, Rating, RecentDecision, STRATEGY_ID,
    StubForecaster, TOP_K_LESSONS, TechnicalIndicators, UnknownRating,
};
pub use registry_arm::register_llm_forecaster_v3;
