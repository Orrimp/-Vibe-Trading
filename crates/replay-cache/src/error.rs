//! Error types for [`crate::ReplayCache`].

use thiserror::Error;

/// Errors produced by [`crate::ReplayCache`] operations.
#[derive(Debug, Error)]
pub enum ReplayCacheError {
    /// Failed to open the SQLite file or run migrations.
    #[error("replay cache open failed at {path}: {detail}")]
    Open { path: String, detail: String },

    /// A row in the cache has `schema_version` higher than the crate
    /// supports. Upgrade the crate or reset the fixture.
    #[error(
        "replay cache {namespace}: schema_version {found} > supported {supported}; \
         upgrade the replay-cache crate or reset the fixture DB"
    )]
    UnsupportedSchema {
        found: i64,
        supported: i32,
        namespace: String,
    },

    /// Strict-replay mode: the requested key was not found in the cache.
    /// No fallthrough; the caller must populate the cache first.
    #[error("replay cache miss: hash={hash} namespace={namespace}")]
    Miss { hash: String, namespace: String },

    /// The cache was opened read-only and a write was attempted.
    #[error("replay cache is read-only")]
    ReadOnly,

    /// SQLite operation failed.
    #[error("replay cache db error: {0}")]
    Db(String),

    /// Failed to serialize a value to JSON for storage.
    #[error("replay cache serialize error: {0}")]
    Serialize(String),

    /// Failed to deserialize a stored JSON value.
    #[error("replay cache deserialize error for hash={hash}: {detail}")]
    Deserialize { hash: String, detail: String },
}
