#![deny(clippy::float_arithmetic)]
//! LLM provider integration (v2 foundation surface).
//!
//! v2.0.0 ships the foundation in five sub-modules:
//!
//! - [`LlmProvider`] trait — `async fn complete(&self, ChatRequest) ->
//!   Result<ChatResponse, LlmError>`. Non-streaming. Tool-use mandatory.
//!   Implemented by three first-class providers (Anthropic, OpenAI-compatible,
//!   Ollama — land in T1902+).
//! - [`LlmError`] — 8-variant error enum (`Provider`, `RateLimited`,
//!   `Timeout`, `BudgetExceeded`, `InvalidResponse`, `ReplayMiss`, `Network`,
//!   `Auth`) wired to the consumer-side error-routing matrix.
//! - [`ToolSchema`] + [`validate_tool_use`] — JSON-Schema-validated
//!   structured outputs at the trait boundary.
//! - **Prompt-cache builder** (`prompt_cache` module — T1906) — layered
//!   `(project, role, dynamic)` system prompts with provider-aware
//!   `CacheBreakpoint::Ephemeral` markers (Anthropic) vs. silent
//!   flattening (OpenAI / Ollama).
//! - **Budget gate** (`budget` module — T1908+) — `BudgetedProvider<Inner>`
//!   decorator with atomic-cents `try_reserve` pre-call gate.
//! - **Record / replay** (`record_replay` module — T1922+) — SQLite-backed
//!   deterministic LLM I/O for research / regression workloads. Strict
//!   replay-only at v2.0.0 (cache miss panics; best-effort fallthrough
//!   deferred to v3).
//!
//! The `cost` crate's [`ProviderKind`] (renamed from `LlmProvider` at
//! T1901 to free the trait name) is re-exported here so trait methods
//! like `fn provider_kind(&self) -> ProviderKind` read naturally from a
//! single import.

pub mod error;
pub mod tools;
pub mod trait_def;

pub use cost::ProviderKind;
pub use error::LlmError;
pub use tools::{validate_tool_use, ToolSchema};
pub use trait_def::{
    CacheBreakpoint, ChatMessage, ChatRequest, ChatResponse, ContentBlock, LlmProvider,
    MessageRole, ModelId, StopReason, SystemBlock, TokenUsage,
};
