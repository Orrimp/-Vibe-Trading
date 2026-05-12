//! T1939 (v2-llm-strategy, pass 6) — V11 schema-migration forward-
//! compat test.
//!
//! Two arms:
//!
//! 1. **Accept v1 (current).** Open the committed `fixtures/replay-v1.db`
//!    fixture via `ReplayProvider::open`; the schema gate passes
//!    because `MAX(schema_version) = 1 <= SUPPORTED_SCHEMA_VERSION`.
//!
//! 2. **Reject v2 (synthesized).** Create a fresh tempdir DB with the
//!    same schema but one row carrying `schema_version = 2`; the gate
//!    surfaces `LlmError::Provider { message: "unknown schema version
//!    2 > supported 1; ..." }`.
//!
//! Mirrors the v1.8 reflection-memory T1816 forward-compat pattern.

#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::path::{Path, PathBuf};

use llm::{LlmError, ReplayProvider, SUPPORTED_SCHEMA_VERSION};
use sqlx::sqlite::SqliteConnectOptions;
use sqlx::SqlitePool;

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map_or_else(|| crate_dir.clone(), Path::to_path_buf)
}

/// Schema definition mirroring `migrations/001_llm_replay.sql`.
/// Inlined here so the test does not require sqlx-migrate at runtime;
/// the production crate's migration runs against `RecordingProvider::open`,
/// which we are intentionally bypassing here to synthesise a v2 row.
const V1_SCHEMA_SQL: &str = "
CREATE TABLE IF NOT EXISTS llm_replay (
    request_hash      TEXT PRIMARY KEY NOT NULL,
    schema_version    INTEGER NOT NULL,
    provider          TEXT NOT NULL,
    model             TEXT NOT NULL,
    request_json      TEXT NOT NULL,
    response_json     TEXT NOT NULL,
    recorded_at       TEXT NOT NULL
);
";

async fn open_writable(path: &Path) -> SqlitePool {
    let opts = SqliteConnectOptions::new()
        .filename(path)
        .create_if_missing(true);
    let pool = SqlitePool::connect_with(opts)
        .await
        .expect("open writable sqlite");
    sqlx::query(V1_SCHEMA_SQL)
        .execute(&pool)
        .await
        .expect("apply v1 schema");
    pool
}

/// T1939 (arm A) — `ReplayProvider::open` accepts a v1 schema (the
/// committed fixture).
#[tokio::test]
async fn t1939_a_accepts_v1_schema_fixture() {
    let fixture = workspace_root()
        .join("crates")
        .join("llm")
        .join("fixtures")
        .join("replay-v1.db");
    if !fixture.exists() {
        // Soft-skip when the fixture is absent (CI may strip it). The
        // synthetic-v2 arm below carries the load-bearing assertion.
        eprintln!(
            "T1939 (a) soft-skip: {} missing — synthetic-v2 arm still runs",
            fixture.display()
        );
        return;
    }

    let provider = ReplayProvider::open(&fixture)
        .await
        .expect("v1 fixture opens cleanly");
    // The `Debug` impl renders the advertised_kind as `Other("replay")`.
    let dbg = format!("{provider:?}");
    assert!(dbg.contains("replay"), "Debug names the replay surface");
}

/// T1939 (arm B) — `ReplayProvider::open` rejects a synthesized v2
/// row with a structured `LlmError::Provider`.
#[tokio::test]
async fn t1939_b_rejects_schema_v2_with_structured_error() {
    let td = tempfile::tempdir().expect("tempdir");
    let db_path = td.path().join("replay-v2.db");

    {
        let pool = open_writable(&db_path).await;
        // Insert one row at schema_version = 2 (hypothetical v3 cache).
        let next_version: i64 = i64::from(SUPPORTED_SCHEMA_VERSION) + 1;
        sqlx::query(
            "INSERT INTO llm_replay
                (request_hash, schema_version, provider, model,
                 request_json, response_json, recorded_at)
             VALUES
                ('deadbeef00000000000000000000000000000000000000000000000000000000',
                 ?, 'anthropic', 'claude-opus-4-7',
                 '{}', '{}',
                 '2026-05-12T00:00:00.000000Z')",
        )
        .bind(next_version)
        .execute(&pool)
        .await
        .expect("seed v2 row");
        pool.close().await;
    }

    let err = ReplayProvider::open(&db_path)
        .await
        .expect_err("v2 schema must reject");
    match err {
        LlmError::Provider { message, .. } => {
            assert!(
                message.contains("schema version"),
                "error message must name the schema-version mismatch: {message}"
            );
            assert!(
                message.contains(&format!("{}", i64::from(SUPPORTED_SCHEMA_VERSION) + 1)),
                "error must include the offending version: {message}"
            );
        }
        other => panic!("expected LlmError::Provider, got {other:?}"),
    }
}

/// T1939 (arm C) — empty cache (no rows) is permitted under any
/// SUPPORTED_SCHEMA_VERSION; the gate only kicks in once at least
/// one row exists.
#[tokio::test]
async fn t1939_c_empty_cache_permitted() {
    let td = tempfile::tempdir().expect("tempdir");
    let db_path = td.path().join("empty.db");
    {
        let pool = open_writable(&db_path).await;
        pool.close().await;
    }
    let _provider = ReplayProvider::open(&db_path)
        .await
        .expect("empty cache opens cleanly");
}
