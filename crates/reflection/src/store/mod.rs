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
