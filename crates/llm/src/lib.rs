#![deny(clippy::float_arithmetic)]
//! # LLM provider integration — v2 foundation surface
//!
//! `llm` is the foundation crate every v2 LLM consumer plugs into. It
//! ships in v2.0.0 as a **foundation-only** surface (Q1 = Option A,
//! operator-decided 2026-05-10): the trait + three provider impls +
//! prompt-cache builder + budget gate + record/replay + a smoke binary,
//! with zero v2.0.0 consumers. Each consumer (reflection-memory LLM
//! enrichment, news-sentiment overlay, trader-debate, …) becomes its
//! own follow-up brief — Q12 in [`spec/v2-llm-strategy/feature.md`]
//! sketches the prior ordering.
//!
//! The crate is wired into the agent at boot via
//! [`crate::factory::LlmProviderFactory::build`], gated on
//! `cfg.llm.enabled` (default `false`). When `true`, the agent main
//! constructs the provider stack once and stores it as an
//! `Option<Arc<dyn LlmProvider>>` on the runtime context (no bus
//! channel — hard constraint #4).
//!
//! ## Provider trait + 3 implementations
//!
//! - [`LlmProvider`] trait (defined in [`trait_def`]) — `async fn
//!   complete(&self, ChatRequest) -> Result<ChatResponse, LlmError>`.
//!   Non-streaming at v2.0.0 (streaming is a v3 brief). Tool-use
//!   mandatory at the type level via [`ToolSchema`].
//! - [`providers::AnthropicProvider`] — Anthropic Messages API.
//!   Supports `cache_control: ephemeral` markers per the prompt-cache
//!   builder's `CacheBreakpoint::Ephemeral` boundary.
//! - [`providers::OpenAiProvider`] — OpenAI Chat Completions API.
//!   Used directly for OpenAI; also re-used for OpenRouter and DeepSeek
//!   via the configured `base_url`.
//! - [`providers::OllamaProvider`] — local Ollama HTTP at
//!   `http://localhost:11434`. Cost-free by definition; pricing emits
//!   zeros so token counts surface through cost telemetry per R2.3.
//!
//! See [Provider implementations][providers].
//!
//! ## Prompt-cache builder
//!
//! [`prompt_cache::CachedSystemPromptBuilder`] composes
//! `(project, role, dynamic)` system prompts with two cache breakpoints
//! per Q5b. The builder is **provider-aware** via `build_for(ProviderKind)`:
//!
//! - `ProviderKind::Anthropic` → emits `CacheBreakpoint::Ephemeral`
//!   markers at the project / role boundaries; the Anthropic provider
//!   translates these to `cache_control: {"type": "ephemeral"}` JSON.
//! - `ProviderKind::OpenAi | OpenRouter | DeepSeek | Other("ollama")`
//!   → silently flattens the breakpoints; the system prompt arrives as
//!   a single string.
//!
//! Cache-hit telemetry surfaces through
//! [`observability::emit_cache_event`] (per-role per-day moving gauge —
//! Q5d) and the operator success report's new `Cache hit ratio` row.
//!
//! ## Budget gate
//!
//! [`budgeted::BudgetedProvider`] is a decorator that wraps every leaf
//! provider:
//!
//! 1. **Mode check** — when [`cost::CostBudget::mode_override`] returns
//!    `None`, the call is blocked with `LlmError::BudgetExceeded`. A
//!    `Some(QuickThink)` override on a `DeepThink` request rewrites
//!    the request to the configured QuickThink model (degrade-to-quick).
//! 2. **Pre-call estimate** — [`pricing::resolve_rate`] +
//!    [`cost::CostBudget::try_reserve`] check the projected spend
//!    atomically before forwarding to the inner provider.
//! 3. **Post-call reconcile** — actual usage hits the budget via
//!    `add_spend(actual_usd)`, emits a `CostEvent::Llm` through the
//!    [`cost::CostSink`], and updates the cache-hit gauge.
//!
//! Audit memos via [`audit::journal::post_llm_budget_event`] post on
//! both `Block` and `DegradeToQuickThink` events (debounced once / 60s
//! per `BudgetedProvider` instance).
//!
//! See [Pricing tables][pricing] and [Budget decorator][budgeted].
//!
//! ## Record / replay — `recording.rs` + `replay.rs`
//!
//! [`recording::RecordingProvider`] and [`replay::ReplayProvider`]
//! ship SQLite-backed deterministic LLM I/O for research / regression
//! workloads.
//!
//! - **Paper mode** wraps the live stack as
//!   `BudgetedProvider<RecordingProvider<Leaf>>` — every successful
//!   `complete()` is canonicalised, SHA-256'd, and persisted into
//!   `cfg.replay_cache_path`.
//! - **Research mode** opens the cache read-only as
//!   `BudgetedProvider<ReplayProvider>` and resolves every call by
//!   request hash. **Strict replay only** at v2.0.0 (D2): a cache miss
//!   is an `LlmError::ReplayMiss { hash, provider, model }` — no
//!   best-effort fall-through to live calls (deferred to v3).
//!
//! See [`spec/runbooks/llm-replay.md`] for the operator's
//! record/refresh/reset/migration playbook.
//!
//! ## Smoke binary — `cargo run --bin llm-smoke`
//!
//! [`src/bin/llm-smoke.rs`][llm-smoke] exercises the full stack end-
//! to-end against either wiremock servers (CI / `cargo test`) or real
//! provider APIs (`--mode paper` against `agent.toml.local` keys). The
//! binary prints an aligned ASCII table:
//!
//! ```text
//! provider | model | tokens_in | tokens_out | usd | latency_ms | result
//! anthropic | claude-opus-4-7 | ... | ... | ... | ... | OK
//! openai    | gpt-5           | ... | ... | ... | ... | OK
//! ollama    | <local model>   | ... | ... | ... | ... | OK
//! ```
//!
//! Exit codes: `0` = every provider returned `OK`; `1` = any non-OK
//! response or research-mode cache miss; `2` = config / CLI parse
//! error. See [`spec/runbooks/llm-cost.md`] for the smoke procedure.
//!
//! ## Module index
//!
//! - [`auth`] — TOML-local API-key reader (`config/agent.toml.local`
//!   overlay) per Q3 = C.
//! - [`budgeted`] — `BudgetedProvider<Inner>` decorator (budget gate).
//! - [`config`] — `LlmConfig` (deserialised from `[llm]` in
//!   `agent.toml`), `TierConfig`, `ProviderConfig`.
//! - [`error`] — 8-variant `LlmError` enum.
//! - [`factory`] — `LlmProviderFactory::build(...)` — the single
//!   construction site for `Arc<dyn LlmProvider>`.
//! - [`observability`] — cache-event emission + counters.
//! - [`pricing`] — hard-coded base rate card + TOML override map.
//! - [`prompt_cache`] — layered `CachedSystemPrompt` builder.
//! - [`providers`] — Anthropic, OpenAI, Ollama leaf impls.
//! - [`recording`] — paper-mode `RecordingProvider`.
//! - [`redact`] — `tracing` field-redaction helper for secrets.
//! - [`replay`] — research-mode `ReplayProvider`.
//! - [`retry`] — full-jitter exponential backoff (Q9c).
//! - [`tools`] — `ToolSchema` + `validate_tool_use` (JSON-Schema).
//! - [`trait_def`] — `LlmProvider` trait + `ChatRequest` / `ChatResponse`.
//!
//! ## `ProviderKind` cross-crate boundary
//!
//! The [`cost`] crate's [`ProviderKind`] (renamed from `LlmProvider` at
//! T1901 to free the trait name) is re-exported here so trait methods
//! like `fn provider_kind(&self) -> ProviderKind` read naturally from a
//! single import. The `cost` crate stays provider-agnostic; the `llm`
//! crate depends on `cost` (additive — see architecture.md line ~2870
//! for the rationale).
//!
//! [`spec/v2-llm-strategy/feature.md`]: ../../../spec/v2-llm-strategy/feature.md
//! [`spec/runbooks/llm-cost.md`]: ../../../spec/runbooks/llm-cost.md
//! [`spec/runbooks/llm-replay.md`]: ../../../spec/runbooks/llm-replay.md
//! [llm-smoke]: ../../../crates/llm/src/bin/llm-smoke.rs

pub mod auth;
pub mod budgeted;
pub mod config;
pub mod error;
pub mod factory;
pub mod observability;
pub mod pricing;
pub mod prompt_cache;
pub mod providers;
pub mod recording;
pub mod redact;
pub mod replay;
pub mod retry;
pub mod tools;
pub mod trait_def;

pub use budgeted::BudgetedProvider;
pub use config::{LlmConfig, ProviderConfig, TierConfig};
pub use cost::ProviderKind;
pub use error::LlmError;
pub use prompt_cache::{CachedSystemPrompt, CachedSystemPromptBuilder};
pub use providers::{AnthropicProvider, OllamaProvider, OpenAiProvider};
pub use recording::RecordingProvider;
pub use replay::{request_hash, ReplayProvider, SUPPORTED_SCHEMA_VERSION};
pub use tools::{validate_tool_use, ToolSchema};
pub use trait_def::{
    CacheBreakpoint, ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmProvider,
    MessageRole, ModelId, StopReason, SystemBlock, TokenUsage,
};
