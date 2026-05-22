//! `ReflectionStore` trait + sqlite default impl.
//!
//! Stub — T1805 lands the real trait + impl.

use async_trait::async_trait;
use thiserror::Error;

use crate::types::{LessonCard, RetrievalQuery};

pub mod sqlite;

/// Errors emitted by any `ReflectionStore` impl.
#[derive(Debug, Error)]
pub enum ReflectionStoreError {
    #[error("database error: {0}")]
    Database(String),
    #[error("encoding error: {0}")]
    Encoding(String),
}

/// Trait surface that any reflection store must implement.
///
/// `upsert` returns `Ok(true)` for a new row, `Ok(false)` for an
/// idempotent skip (R2.4).  `top_k` is ordered by `(score DESC,
/// closed_at ASC)` for byte-stable retrieval.
#[async_trait]
pub trait ReflectionStore: Send + Sync {
    async fn upsert(&self, card: &LessonCard) -> Result<bool, ReflectionStoreError>;
    async fn top_k(
        &self,
        query: &RetrievalQuery,
        k: usize,
    ) -> Result<Vec<LessonCard>, ReflectionStoreError>;
    async fn count(&self) -> Result<u64, ReflectionStoreError>;
}

// ── NullReflectionStore ───────────────────────────────────────────────────────

/// A no-op `ReflectionStore` implementation that always returns empty results.
///
/// Used in unit tests and analytical Wave C builds where no real lesson cards
/// are available. The strategy falls through cleanly — top-K returns `[]`,
/// the `ForecastContext::top_k_lessons` is empty, and the LLM prompt skips
/// the lesson-cards section.
///
/// ## Thread safety
///
/// Trivially `Send + Sync` — zero mutable state.
#[derive(Debug, Default)]
pub struct NullReflectionStore;

#[async_trait]
impl ReflectionStore for NullReflectionStore {
    async fn upsert(&self, _card: &LessonCard) -> Result<bool, ReflectionStoreError> {
        Ok(false)
    }

    async fn top_k(
        &self,
        _query: &RetrievalQuery,
        _k: usize,
    ) -> Result<Vec<LessonCard>, ReflectionStoreError> {
        Ok(Vec::new())
    }

    async fn count(&self) -> Result<u64, ReflectionStoreError> {
        Ok(0)
    }
}
