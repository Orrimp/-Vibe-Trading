//! LLM provider integration.
//!
//! v0 stub — no LLM calls are made in v0.
//! Provider trait sketch ensures v0.5 has a home without a new crate.

use thiserror::Error;

/// Error type for LLM operations.
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("provider error: {0}")]
    Provider(String),
    #[error("rate limited")]
    RateLimited,
    #[error("timeout")]
    Timeout,
}

/// A single LLM provider — implemented in v0.5+.
#[allow(unused_variables)]
pub trait LlmProvider: Send + Sync {
    fn name(&self) -> &str;
}
