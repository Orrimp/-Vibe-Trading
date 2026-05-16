//! `RecordingProvider<Inner>` — write half of the M6 record/replay
//! pair (T1921).
//!
//! Wraps any [`LlmProvider`], forwards the call to `inner`, then on
//! success `INSERT OR REPLACE`s the request + response into the
//! `llm_replay` table at the configured SQLite path. Per Design § Q8e:
//!
//! - **SQLite WAL mode** — set via `journal_mode = WAL` on connect.
//!   WAL handles SQLite-side serialisation; a crash mid-write leaves
//!   the WAL with the failed transaction unapplied, and the next open
//!   replays cleanly. No `atomic_write`-style tempfile-rename is
//!   needed (Q8-bonus § Atomic-write contract).
//! - **Per-process tokio `Mutex<()>` for the writer half** — two
//!   parallel `complete()` calls serialise their `INSERT OR REPLACE`
//!   through this lock. Reads (the `ReplayProvider` side) are
//!   unblocked.
//! - **`INSERT OR REPLACE`** — idempotent on the hash key. Re-recording
//!   the same request body overwrites the row in place and emits
//!   `tracing::info!(target: "llm.replay", "fixture_overwrite", hash)`
//!   so a paper-mode operator sees when a fixture changed (R6.5).
//!
//! The migration at `crates/llm/migrations/001_llm_replay.sql` is run
//! on every [`RecordingProvider::open`] call (sqlx tracks applied
//! versions, so re-opens are no-ops).

use std::path::Path;

use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions, SqliteSynchronous};
use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::error::LlmError;
use crate::replay::{canonical_json_string, request_hash, SUPPORTED_SCHEMA_VERSION};
use crate::trait_def::{ChatRequest, ChatResponse, LlmProvider};
use crate::ProviderKind;

/// Decorator that wraps `inner`, records every successful
/// `complete()` into the SQLite replay cache, and is otherwise
/// transparent.
///
/// **Paper mode wrap.** The factory's `Mode::Paper` arm
/// (`LlmProviderFactory::build` at T1913) wraps the leaf provider in
/// this decorator so subsequent research-mode runs replay the
/// captured fixtures. Live mode never wraps with this — recording
/// during live trading is a v3 forensic concern.
pub struct RecordingProvider<Inner: LlmProvider> {
    inner: Inner,
    pool: SqlitePool,
    /// Per-process single-writer lock (Q8e). The `Mutex<()>` carries
    /// no state — the body of `complete()` acquires it before the
    /// `INSERT OR REPLACE` and releases on drop.
    writer_lock: Mutex<()>,
}

impl<Inner: LlmProvider> std::fmt::Debug for RecordingProvider<Inner> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RecordingProvider")
            .field("inner.name", &self.inner.name())
            .field("inner.kind", &self.inner.provider_kind())
            .finish()
    }
}

impl<Inner: LlmProvider> RecordingProvider<Inner> {
    /// Open the replay cache at `path`, create it if missing, run the
    /// migration, and wrap `inner`.
    ///
    /// SQLite is opened with `journal_mode = WAL` + `synchronous =
    /// NORMAL` (Q8e). The parent directory of `path` is created via
    /// `tokio::fs::create_dir_all` so callers don't need to pre-stage
    /// the layout (the smoke binary in particular runs against a
    /// fresh `data/` on every CI run).
    ///
    /// # Errors
    ///
    /// - [`LlmError::Provider`] when the SQLite file cannot be opened
    ///   OR the migration fails.
    pub async fn open(inner: Inner, path: &Path) -> Result<Self, LlmError> {
        if let Some(parent) = path.parent()
            && !parent.as_os_str().is_empty()
        {
            tokio::fs::create_dir_all(parent)
                .await
                .map_err(|e| LlmError::Provider {
                    provider: ProviderKind::Other("replay".to_string()),
                    message: format!("create parent dir {}: {e}", parent.display()),
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
            .map_err(|e| LlmError::Provider {
                provider: ProviderKind::Other("replay".to_string()),
                message: format!("open replay cache at {}: {e}", path.display()),
            })?;

        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| LlmError::Provider {
                provider: ProviderKind::Other("replay".to_string()),
                message: format!("run llm_replay migration: {e}"),
            })?;

        Ok(Self {
            inner,
            pool,
            writer_lock: Mutex::new(()),
        })
    }

    /// Expose the underlying pool — used by integration tests that
    /// want to `SELECT count(*)` the recorded rows without re-opening
    /// the file (which under WAL can race the writer briefly on
    /// macOS). Not part of the public consumer surface.
    #[cfg(test)]
    pub(crate) fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

#[async_trait]
impl<Inner: LlmProvider> LlmProvider for RecordingProvider<Inner> {
    fn name(&self) -> &str {
        self.inner.name()
    }

    fn provider_kind(&self) -> ProviderKind {
        self.inner.provider_kind()
    }

    async fn complete(&self, request: ChatRequest) -> Result<ChatResponse, LlmError> {
        // 1. Forward to the inner provider. Recording is post-success
        //    only — a failed call records nothing (R9.3 budget rule
        //    transplanted: no cost event on failure → no replay row
        //    either; fixture content remains authoritative).
        let response = self.inner.complete(request.clone()).await?;

        // 2. Compute hash + canonical request JSON.
        let hash = request_hash(&request)?;
        let request_json = canonical_json_string(&request)?;
        let response_json = serde_json::to_string(&response)
            .map_err(|e| LlmError::InvalidResponse(format!("encode response: {e}")))?;
        let provider_name = self.inner.name().to_string();
        let model_str = response.model.as_str().to_string();
        let recorded_at = chrono_like_timestamp_or_default();

        // 3. Detect overwrite for the R6.5 info log. SELECT first so
        //    we know whether this is a fresh row vs a re-record.
        //    The SELECT runs OUTSIDE the writer_lock since reads are
        //    unblocked under WAL — only the INSERT serialises.
        let pre_existing: Option<(i64,)> =
            sqlx::query_as("SELECT 1 FROM llm_replay WHERE request_hash = ?")
                .bind(&hash)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| LlmError::Provider {
                    provider: ProviderKind::Other("replay".to_string()),
                    message: format!("pre-check hash {hash}: {e}"),
                })?;

        // 4. Acquire the writer lock + INSERT OR REPLACE.
        let _guard = self.writer_lock.lock().await;
        sqlx::query(
            "INSERT OR REPLACE INTO llm_replay \
             (request_hash, schema_version, provider, model, \
              request_json, response_json, recorded_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(&hash)
        .bind(i64::from(SUPPORTED_SCHEMA_VERSION))
        .bind(&provider_name)
        .bind(&model_str)
        .bind(&request_json)
        .bind(&response_json)
        .bind(&recorded_at)
        .execute(&self.pool)
        .await
        .map_err(|e| LlmError::Provider {
            provider: ProviderKind::Other("replay".to_string()),
            message: format!("insert replay row hash={hash}: {e}"),
        })?;
        drop(_guard);

        if pre_existing.is_some() {
            tracing::info!(
                target: "llm.replay",
                hash = %hash,
                provider = %provider_name,
                model = %model_str,
                "fixture_overwrite"
            );
        } else {
            tracing::debug!(
                target: "llm.replay",
                hash = %hash,
                provider = %provider_name,
                model = %model_str,
                "fixture_record"
            );
        }

        Ok(response)
    }
}

/// Render a 6-digit fractional ISO-8601 UTC timestamp. The audit ledger
/// uses the same shape (microsecond precision avoids `ORDER BY` ties).
///
/// We avoid pulling in `chrono` here — `time::OffsetDateTime` is
/// already in the workspace and matches the audit-side rendering.
fn chrono_like_timestamp_or_default() -> String {
    use time::format_description::well_known::Rfc3339;
    use time::OffsetDateTime;
    let now = OffsetDateTime::now_utc();
    now.format(&Rfc3339)
        .unwrap_or_else(|_| String::from("1970-01-01T00:00:00Z"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::trait_def::{
        ChatMessage, ContentBlock, MessageRole, ModelId, StopReason, TokenUsage,
    };
    use cost::{AgentRole, LlmTier};
    use uuid::Uuid;

    struct ProgrammedInner {
        response: ChatResponse,
        kind: ProviderKind,
    }
    #[async_trait]
    impl LlmProvider for ProgrammedInner {
        fn name(&self) -> &str {
            match &self.kind {
                ProviderKind::Anthropic => "anthropic",
                ProviderKind::OpenAi => "openai",
                ProviderKind::Other(s) => s,
                _ => "other",
            }
        }
        fn provider_kind(&self) -> ProviderKind {
            self.kind.clone()
        }
        async fn complete(&self, _req: ChatRequest) -> Result<ChatResponse, LlmError> {
            Ok(self.response.clone())
        }
    }

    fn sample_request() -> ChatRequest {
        let mut r = ChatRequest::new(
            ModelId::new("claude-opus-4-7"),
            LlmTier::DeepThink,
            AgentRole::Trader,
        );
        r.max_tokens = 256;
        r.messages.push(ChatMessage {
            role: MessageRole::User,
            content: vec![ContentBlock::Text("hi".into())],
        });
        r
    }

    fn sample_response() -> ChatResponse {
        ChatResponse {
            content: vec![ContentBlock::Text("OK".into())],
            stop_reason: StopReason::EndTurn,
            usage: TokenUsage {
                tokens_in: 5,
                tokens_out: 1,
                tokens_cached_in: 0,
            },
            model: ModelId::new("claude-opus-4-7"),
            correlation_id: Uuid::nil(),
        }
    }

    /// T1921 (a) — one call lands one row in the SQLite.
    #[tokio::test]
    async fn t1921_a_single_call_lands_one_row() {
        let td = tempfile::tempdir().unwrap();
        let db_path = td.path().join("replay.db");
        let rec = RecordingProvider::open(
            ProgrammedInner {
                response: sample_response(),
                kind: ProviderKind::Anthropic,
            },
            &db_path,
        )
        .await
        .unwrap();

        rec.complete(sample_request()).await.unwrap();

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM llm_replay")
            .fetch_one(rec.pool())
            .await
            .unwrap();
        assert_eq!(count, 1);
    }

    /// T1921 (b) — same hash re-records idempotently.
    #[tokio::test]
    async fn t1921_b_idempotent_overwrite() {
        let td = tempfile::tempdir().unwrap();
        let db_path = td.path().join("replay.db");
        let rec = RecordingProvider::open(
            ProgrammedInner {
                response: sample_response(),
                kind: ProviderKind::Anthropic,
            },
            &db_path,
        )
        .await
        .unwrap();

        rec.complete(sample_request()).await.unwrap();
        rec.complete(sample_request()).await.unwrap();
        rec.complete(sample_request()).await.unwrap();

        let (count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM llm_replay")
            .fetch_one(rec.pool())
            .await
            .unwrap();
        assert_eq!(count, 1, "INSERT OR REPLACE must keep row count at 1");
    }

    /// T1921 (c) — hash is byte-stable across two recordings of the
    /// same request. The row's `request_hash` column matches the
    /// pure-fn `request_hash(&req)`.
    #[tokio::test]
    async fn t1921_c_hash_byte_stable_across_recordings() {
        let td = tempfile::tempdir().unwrap();
        let db_path = td.path().join("replay.db");
        let rec = RecordingProvider::open(
            ProgrammedInner {
                response: sample_response(),
                kind: ProviderKind::Anthropic,
            },
            &db_path,
        )
        .await
        .unwrap();

        let req = sample_request();
        rec.complete(req.clone()).await.unwrap();

        let expected = request_hash(&req).unwrap();
        let (got,): (String,) = sqlx::query_as("SELECT request_hash FROM llm_replay LIMIT 1")
            .fetch_one(rec.pool())
            .await
            .unwrap();
        assert_eq!(got, expected, "row hash diverged from pure-fn hash");
    }
}
