//! Top-K retrieval — public entry-point.  Stub; T1809 lands.

use thiserror::Error;

use crate::store::{ReflectionStore, ReflectionStoreError};
use crate::types::{LessonCard, RetrievalQuery};

/// Errors from retrieval.
#[derive(Debug, Error)]
pub enum RetrievalError {
    #[error("store error: {0}")]
    Store(#[from] ReflectionStoreError),
}

/// Retrieve the top `k` cards matching `query` from `store`.
///
/// Stub — T1809 wires the real call through.
///
/// # Errors
///
/// Returns [`RetrievalError::Store`] on store failure.
pub async fn retrieve_top_k(
    store: &dyn ReflectionStore,
    query: &RetrievalQuery,
    k: usize,
) -> Result<Vec<LessonCard>, RetrievalError> {
    Ok(store.top_k(query, k).await?)
}
