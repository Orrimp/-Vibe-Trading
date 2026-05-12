//! Provider implementations of the [`crate::LlmProvider`] trait (M2).
//!
//! Three first-class providers ship at v2.0.0 — Anthropic (load-bearing,
//! includes prompt-cache headers + tool-use), OpenAI-compatible (covers
//! OpenAI / OpenRouter / DeepSeek / LM Studio via a common shape), and
//! Ollama (local, no auth, best-effort tool-use via prompt augmentation).
//!
//! Each impl carries its own per-provider 429 / `Retry-After` handling
//! via the shared [`crate::retry::run_with_backoff`] helper (Design Q9
//! resolution — leaf-provider impl, not a generic decorator).

pub mod anthropic;
pub mod ollama;
pub mod openai;

pub use anthropic::AnthropicProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAiProvider;
