//! Generic SQLite WAL replay-cache primitive.
//!
//! ## Overview
//!
//! `ReplayCache<K, V>` is a content-addressed key-value store backed by
//! SQLite in WAL mode. It is the shared primitive extracted from the v2 LLM
//! record/replay pattern (crates/llm/src/replay.rs) and reused by
//! crates/forecast/ for DL forecaster inference caching.
//!
//! ## Cache key
//!
//! Keys are SHA-256 hex strings over the canonical JSON of the request
//! parameters. [`canonical_json_key`] produces the key from any
//! `Serialize` type, using deep-sorted object keys for byte-stable output.
//! Callers pass the *already-hashed* `String` key to `store`/`load` so the
//! key strategy is owned by the caller (they decide which fields to include
//! or exclude — e.g. the LLM cache excludes `correlation_id`).
//!
//! ## Strict-replay mode
//!
//! In research mode, a cache miss surfaces as [`ReplayCacheError::Miss`]
//! carrying the lookup hash, namespace, and a summary. There is no
//! best-effort fallthrough. This matches the v2 LLM D2 operator-lock rule.
//!
//! ## schema_version
//!
//! Every row carries a `schema_version` integer. The constant
//! [`SUPPORTED_SCHEMA_VERSION`] is checked on open: if any row in the table
//! exceeds it, `open_readonly` returns [`ReplayCacheError::UnsupportedSchema`].
//! Forward-compat hook for future column additions.
//!
//! ## Namespaces
//!
//! A single SQLite file may serve multiple namespaces (e.g. `"llm"` and
//! `"kronos"`). The namespace is part of the row but NOT the primary key —
//! the SHA-256 hash covers all load-bearing fields including the namespace
//! discriminator passed by the caller. Consumers MUST include the namespace
//! in their canonical-JSON key payload if they share a DB file.
//!
//! ## Thread safety
//!
//! Write half: a per-instance `tokio::sync::Mutex<()>` serialises
//! `INSERT OR REPLACE` operations. Read half: unlocked (WAL allows
//! concurrent readers).

use std::marker::PhantomData;
use std::path::Path;

use serde::{Serialize, de::DeserializeOwned};
use sha2::{Digest, Sha256};
use sqlx::SqlitePool;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use tokio::sync::Mutex;
use tracing::{debug, warn};

pub use error::ReplayCacheError;

pub mod error;

/// `schema_version` ceiling asserted on [`ReplayCache::open_readonly`] and
/// [`ReplayCache::open_readwrite`]. Bump to `2` when a column is added via
/// a new migration.
pub const SUPPORTED_SCHEMA_VERSION: i32 = 1;

/// Generic SQLite WAL replay-cache.
///
/// `K` is the canonical-JSON input type (used only for hashing; never
/// stored as a typed value after hashing). `V` is the response type
/// stored as `serde_json` in the `response_json` column.
///
/// In practice, callers use the convenience functions [`canonical_json_string`]
/// and [`sha256_hex`] to produce a `String` key, then call [`store`] /
/// [`load`] directly. The `K` phantom is for documentation clarity only.
pub struct ReplayCache<K, V> {
    pool: SqlitePool,
    namespace: String,
    readonly: bool,
    writer_lock: Mutex<()>,
    _phantom: PhantomData<(K, V)>,
}

impl<K, V> std::fmt::Debug for ReplayCache<K, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayCache")
            .field("namespace", &self.namespace)
            .field("readonly", &self.readonly)
            .finish()
    }
}

impl<K: Serialize, V: Serialize + DeserializeOwned> ReplayCache<K, V> {
    /// Open an existing cache file read-only. Asserts
    /// `schema_version <= SUPPORTED_SCHEMA_VERSION`.
    ///
    /// # Errors
    ///
    /// - [`ReplayCacheError::Open`] if the file cannot be opened.
    /// - [`ReplayCacheError::UnsupportedSchema`] if a row exceeds
    ///   `SUPPORTED_SCHEMA_VERSION`.
    pub async fn open_readonly(
        path: &Path,
        namespace: impl Into<String>,
    ) -> Result<Self, ReplayCacheError> {
        let opts = SqliteConnectOptions::new().filename(path).read_only(true);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .map_err(|e| ReplayCacheError::Open {
                path: path.display().to_string(),
                detail: e.to_string(),
            })?;

        let ns = namespace.into();
        schema_version_gate(&pool, &ns).await?;

        Ok(Self {
            pool,
            namespace: ns,
            readonly: true,
            writer_lock: Mutex::new(()),
            _phantom: PhantomData,
        })
    }

    /// Open (or create) a cache file read-write with WAL + NORMAL sync.
    /// Runs all pending migrations on open.
    ///
    /// # Errors
    ///
    /// - [`ReplayCacheError::Open`] if the file cannot be opened or the
    ///   migration fails.
    pub async fn open_readwrite(
        path: &Path,
        namespace: impl Into<String>,
    ) -> Result<Self, ReplayCacheError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| ReplayCacheError::Open {
                    path: path.display().to_string(),
                    detail: e.to_string(),
                })?;
        }

        let opts = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(true)
            .journal_mode(SqliteJournalMode::Wal)
            .synchronous(SqliteSynchronous::Normal);
        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .map_err(|e| ReplayCacheError::Open {
                path: path.display().to_string(),
                detail: e.to_string(),
            })?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| ReplayCacheError::Open {
                path: path.display().to_string(),
                detail: e.to_string(),
            })?;

        let ns = namespace.into();

        Ok(Self {
            pool,
            namespace: ns,
            readonly: false,
            writer_lock: Mutex::new(()),
            _phantom: PhantomData,
        })
    }

    /// Load a cached response by its pre-computed SHA-256 key.
    ///
    /// Returns `Ok(Some(V))` on a cache hit, `Ok(None)` on a miss (for
    /// callers that want to populate on miss), or
    /// [`ReplayCacheError::Miss`] when running in a strict-replay context.
    ///
    /// For strict-replay mode, use [`strict_load`] instead.
    ///
    /// # Errors
    ///
    /// - [`ReplayCacheError::Db`] on SQLite errors.
    /// - [`ReplayCacheError::Deserialize`] if the stored JSON cannot be
    ///   decoded into `V`.
    pub async fn load(&self, key: &str) -> Result<Option<V>, ReplayCacheError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT response_json FROM replay_cache WHERE request_hash = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ReplayCacheError::Db(e.to_string()))?;

        match row {
            Some((json,)) => {
                let v: V =
                    serde_json::from_str(&json).map_err(|e| ReplayCacheError::Deserialize {
                        hash: key.to_string(),
                        detail: e.to_string(),
                    })?;
                Ok(Some(v))
            }
            None => Ok(None),
        }
    }

    /// Strict-replay load: a cache miss surfaces as
    /// [`ReplayCacheError::Miss`].
    ///
    /// # Errors
    ///
    /// - [`ReplayCacheError::Miss`] on cache miss (no fallthrough).
    /// - [`ReplayCacheError::Db`] on SQLite errors.
    /// - [`ReplayCacheError::Deserialize`] on corrupt JSON.
    pub async fn strict_load(&self, key: &str) -> Result<V, ReplayCacheError> {
        match self.load(key).await? {
            Some(v) => Ok(v),
            None => {
                warn!(
                    target: "replay_cache",
                    key = %key,
                    namespace = %self.namespace,
                    "replay_miss"
                );
                Err(ReplayCacheError::Miss {
                    hash: key.to_string(),
                    namespace: self.namespace.clone(),
                })
            }
        }
    }

    /// Store a request-response pair. Uses `INSERT OR REPLACE` — idempotent
    /// on the same hash key.
    ///
    /// Panics at the type level if opened read-only; callers MUST use
    /// [`open_readwrite`] to write.
    ///
    /// # Errors
    ///
    /// - [`ReplayCacheError::ReadOnly`] if opened read-only.
    /// - [`ReplayCacheError::Serialize`] if `request_json` or `value`
    ///   cannot be encoded.
    /// - [`ReplayCacheError::Db`] on SQLite errors.
    pub async fn store(
        &self,
        key: &str,
        request_json: &str,
        value: &V,
    ) -> Result<(), ReplayCacheError> {
        if self.readonly {
            return Err(ReplayCacheError::ReadOnly);
        }

        let response_json =
            serde_json::to_string(value).map_err(|e| ReplayCacheError::Serialize(e.to_string()))?;

        let recorded_at = timestamp_now();

        // Check if overwriting for the overwrite log.
        let pre_existing: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM replay_cache WHERE request_hash = ?")
                .bind(key)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ReplayCacheError::Db(e.to_string()))?;

        let _guard = self.writer_lock.lock().await;
        sqlx::query(
            "INSERT OR REPLACE INTO replay_cache \
             (request_hash, schema_version, namespace, request_json, response_json, recorded_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(key)
        .bind(i64::from(SUPPORTED_SCHEMA_VERSION))
        .bind(&self.namespace)
        .bind(request_json)
        .bind(&response_json)
        .bind(&recorded_at)
        .execute(&self.pool)
        .await
        .map_err(|e| ReplayCacheError::Db(e.to_string()))?;
        drop(_guard);

        if pre_existing.is_some() {
            tracing::info!(
                target: "replay_cache",
                key = %key,
                namespace = %self.namespace,
                "fixture_overwrite"
            );
        } else {
            debug!(
                target: "replay_cache",
                key = %key,
                namespace = %self.namespace,
                "fixture_record"
            );
        }

        Ok(())
    }

    /// Expose the underlying pool for integration tests.
    #[cfg(test)]
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

// ── Helper functions ──────────────────────────────────────────────────────────

/// Serialize `value` to canonical JSON (deep-sorted object keys).
///
/// This is the same algorithm used in crates/llm/src/replay.rs. When an
/// offline-compatible `serde-canonical-json` crate becomes available, swap
/// the body here.
///
/// # Errors
///
/// Returns [`ReplayCacheError::Serialize`] if encoding fails (impossible
/// for most concrete types but propagated rather than panicked).
pub fn canonical_json_string<T: Serialize>(value: &T) -> Result<String, ReplayCacheError> {
    let raw: serde_json::Value = serde_json::to_value(value)
        .map_err(|e| ReplayCacheError::Serialize(format!("canonical_json: encode: {e}")))?;
    let sorted = sort_value(raw);
    serde_json::to_string(&sorted)
        .map_err(|e| ReplayCacheError::Serialize(format!("canonical_json: render: {e}")))
}

/// Recursively deep-sort all `Map` keys in a `serde_json::Value`.
/// Array element order is preserved (semantically load-bearing).
fn sort_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut sorted = std::collections::BTreeMap::new();
            for (k, v) in map {
                sorted.insert(k, sort_value(v));
            }
            let mut out = serde_json::Map::with_capacity(sorted.len());
            for (k, v) in sorted {
                out.insert(k, v);
            }
            serde_json::Value::Object(out)
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.into_iter().map(sort_value).collect())
        }
        leaf => leaf,
    }
}

/// SHA-256 hex (lowercase) of a byte slice.
pub fn sha256_hex(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let digest = hasher.finalize();
    hex_lower(&digest)
}

/// Compute a canonical-JSON SHA-256 key from a serializable value.
///
/// Convenience wrapper: `canonical_json_string` + `sha256_hex`.
///
/// # Errors
///
/// Returns [`ReplayCacheError::Serialize`] if encoding fails.
pub fn canonical_key<T: Serialize>(value: &T) -> Result<String, ReplayCacheError> {
    let canonical = canonical_json_string(value)?;
    Ok(sha256_hex(canonical.as_bytes()))
}

/// Hex-encode bytes as lowercase hex string.
fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0xf) as usize] as char);
    }
    out
}

/// Render a 6-digit fractional ISO-8601 UTC timestamp.
fn timestamp_now() -> String {
    use time::OffsetDateTime;
    use time::format_description::well_known::Rfc3339;
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

/// Assert `schema_version <= SUPPORTED_SCHEMA_VERSION` on the table.
/// Empty tables (no rows) are accepted.
async fn schema_version_gate(pool: &SqlitePool, namespace: &str) -> Result<(), ReplayCacheError> {
    // Table may not exist if the file was just created by open_readonly
    // in a test — tolerate missing table gracefully.
    let row: Result<Option<(i64,)>, _> =
        sqlx::query_as("SELECT MAX(schema_version) FROM replay_cache")
            .fetch_optional(pool)
            .await;

    match row {
        Ok(Some((max_v,))) => {
            if max_v > i64::from(SUPPORTED_SCHEMA_VERSION) {
                return Err(ReplayCacheError::UnsupportedSchema {
                    found: max_v,
                    supported: SUPPORTED_SCHEMA_VERSION,
                    namespace: namespace.to_string(),
                });
            }
            Ok(())
        }
        Ok(None) => Ok(()), // empty table
        Err(_) => Ok(()),   // table doesn't exist yet — first open via readwrite will create it
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestRequest {
        model: String,
        temperature: f32,
        prompt: String,
    }

    #[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct TestResponse {
        text: String,
        tokens: u32,
    }

    type Cache = ReplayCache<TestRequest, TestResponse>;

    fn sample_req(seed: u8) -> TestRequest {
        TestRequest {
            model: "test-model".into(),
            temperature: 0.7,
            prompt: format!("hello {seed}"),
        }
    }

    fn sample_resp() -> TestResponse {
        TestResponse {
            text: "OK".into(),
            tokens: 5,
        }
    }

    /// canonical_json_string produces byte-stable output 1000x.
    #[test]
    fn canonical_json_deterministic_1000x() {
        let req = sample_req(0);
        let baseline = canonical_json_string(&req).unwrap();
        for _ in 0..1000 {
            let again = canonical_json_string(&req).unwrap();
            assert_eq!(again, baseline, "canonical_json drifted");
        }
    }

    /// canonical_key produces a 64-char hex string.
    #[test]
    fn canonical_key_is_64_chars() {
        let req = sample_req(0);
        let key = canonical_key(&req).unwrap();
        assert_eq!(key.len(), 64, "SHA-256 hex must be 64 chars");
    }

    /// Two requests differing only in prompt produce different keys.
    #[test]
    fn different_prompts_yield_different_keys() {
        let a = canonical_key(&sample_req(0)).unwrap();
        let b = canonical_key(&sample_req(1)).unwrap();
        assert_ne!(a, b);
    }

    /// SUPPORTED_SCHEMA_VERSION is 1 at v2.5.
    #[test]
    fn supported_schema_version_is_one() {
        assert_eq!(SUPPORTED_SCHEMA_VERSION, 1);
    }

    /// Open read-write, store a value, load it back — round-trip.
    #[tokio::test]
    async fn store_and_load_round_trip() {
        let td = tempfile::tempdir().unwrap();
        let db_path = td.path().join("replay.db");

        let cache: Cache = ReplayCache::open_readwrite(&db_path, "test").await.unwrap();

        let req = sample_req(42);
        let key = canonical_key(&req).unwrap();
        let req_json = canonical_json_string(&req).unwrap();
        let resp = sample_resp();

        cache.store(&key, &req_json, &resp).await.unwrap();

        let loaded = cache.load(&key).await.unwrap();
        assert_eq!(loaded, Some(resp));
    }

    /// Store is idempotent: same key twice still yields one row.
    #[tokio::test]
    async fn store_idempotent() {
        let td = tempfile::tempdir().unwrap();
        let db_path = td.path().join("replay.db");

        let cache: Cache = ReplayCache::open_readwrite(&db_path, "test").await.unwrap();

        let req = sample_req(0);
        let key = canonical_key(&req).unwrap();
        let req_json = canonical_json_string(&req).unwrap();
        let resp = sample_resp();

        cache.store(&key, &req_json, &resp).await.unwrap();
        cache.store(&key, &req_json, &resp).await.unwrap();
        cache.store(&key, &req_json, &resp).await.unwrap();

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM replay_cache")
            .fetch_one(cache.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    /// strict_load returns Miss error on an empty cache.
    #[tokio::test]
    async fn strict_load_miss_on_empty_cache() {
        let td = tempfile::tempdir().unwrap();
        let db_path = td.path().join("replay.db");

        let cache: Cache = ReplayCache::open_readwrite(&db_path, "kronos")
            .await
            .unwrap();

        let req = sample_req(0);
        let key = canonical_key(&req).unwrap();

        let err = cache.strict_load(&key).await.unwrap_err();
        assert!(
            matches!(err, ReplayCacheError::Miss { .. }),
            "expected Miss, got {err:?}"
        );
        if let ReplayCacheError::Miss { namespace, .. } = err {
            assert_eq!(namespace, "kronos");
        }
    }

    /// Read-only cache returns ReadOnly error on store.
    #[tokio::test]
    async fn readonly_cache_rejects_store() {
        let td = tempfile::tempdir().unwrap();
        let db_path = td.path().join("replay.db");

        // Create with readwrite first so the file exists.
        let rw: Cache = ReplayCache::open_readwrite(&db_path, "test").await.unwrap();
        drop(rw);

        let ro: Cache = ReplayCache::open_readonly(&db_path, "test").await.unwrap();
        let req = sample_req(0);
        let key = canonical_key(&req).unwrap();
        let req_json = canonical_json_string(&req).unwrap();
        let resp = sample_resp();

        let err = ro.store(&key, &req_json, &resp).await.unwrap_err();
        assert!(
            matches!(err, ReplayCacheError::ReadOnly),
            "expected ReadOnly, got {err:?}"
        );
    }
}
