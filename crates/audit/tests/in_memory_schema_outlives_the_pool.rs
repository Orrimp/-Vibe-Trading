#![allow(clippy::unwrap_used, clippy::expect_used)]
//! bug-log #185 — an in-memory `Ledger`'s schema must not be able to evaporate.
//!
//! ## The property, and why it needs two tests
//!
//! An in-memory SQLite database exists only while at least one connection to it
//! is open. `Ledger::open(":memory:")` runs the migrations once at construction
//! and never again, so if the pool ever reaches ZERO open connections the
//! database is destroyed and every later query reports
//! `no such table: journal_transactions` — the intermittent, Windows-only CI
//! failure in `crates/ui/tests/cockpit_live_kill_button_writes_audit.rs`
//! (`t906_stitch_kill_confirmed_via_state_update_writes_both_audit_rows`).
//!
//! [`a_pool_that_reaches_zero_connections_loses_the_schema`] is the **why**: it
//! demonstrates the destruction on a plain sqlx pool, independent of `Ledger`.
//! Without it, the configuration assertion below is a magic incantation nobody
//! can evaluate — and if a future sqlx stops behaving this way, this test turns
//! red first and says the guard may be relaxed.
//!
//! [`the_in_memory_ledger_pool_cannot_reach_zero`] is the **guard**, two-sided:
//! `:memory:` pins a connection floor and disables both reapers, while a
//! file-backed ledger keeps sqlx's defaults, because its schema is on disk and
//! cannot evaporate.
//!
//! What this does NOT claim: it does not reproduce the CI failure. A runtime
//! drop also destroys connections (measured: `pool.size()` 3 → 2), and whether
//! Windows takes the last one is untested here. This closes the reaping path,
//! which is the one that is confirmed.

use std::time::Duration;

use audit::Ledger;
use sqlx::sqlite::SqlitePoolOptions;

/// **The why.** A pool allowed to reap to zero loses an in-memory schema.
#[tokio::test]
async fn a_pool_that_reaches_zero_connections_loses_the_schema() {
    let pool = SqlitePoolOptions::new()
        .min_connections(0)
        .idle_timeout(Duration::from_millis(50))
        .connect("sqlite::memory:")
        .await
        .expect("connect");

    sqlx::query("CREATE TABLE journal_transactions (id TEXT PRIMARY KEY)")
        .execute(&pool)
        .await
        .expect("create table");

    for _ in 0..40 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        if pool.size() == 0 {
            break;
        }
    }
    assert_eq!(
        pool.size(),
        0,
        "the idle reaper did not empty the pool, so this test cannot demonstrate \
         the destruction it exists to demonstrate — it would then pass while \
         measuring nothing"
    );

    let after = sqlx::query("SELECT COUNT(*) FROM journal_transactions")
        .fetch_one(&pool)
        .await;
    let err = after
        .err()
        .map(|e| e.to_string())
        .unwrap_or_else(|| "<query unexpectedly succeeded>".to_owned());
    assert!(
        err.contains("no such table"),
        "reaching zero connections must destroy the in-memory database. If this \
         now SUCCEEDS, sqlx no longer ties the database's lifetime to an open \
         connection and the guard below may be relaxed. Got: {err}"
    );
}

/// **The guard.** The in-memory ledger's pool cannot reach zero; the
/// file-backed one keeps sqlx's defaults.
#[tokio::test]
async fn the_in_memory_ledger_pool_cannot_reach_zero() {
    let mem = Ledger::in_memory().await.expect("open in-memory ledger");
    let opts = mem.pool().options();
    assert!(
        opts.get_min_connections() >= 1,
        "an in-memory ledger must keep at least one connection open or its schema \
         can be destroyed between queries (bug-log #185). min_connections = {}",
        opts.get_min_connections()
    );
    assert!(
        opts.get_idle_timeout().is_none(),
        "an idle timeout can reap the last connection and take the database with \
         it. idle_timeout = {:?}",
        opts.get_idle_timeout()
    );
    assert!(
        opts.get_max_lifetime().is_none(),
        "a max lifetime can retire the last connection and take the database with \
         it. max_lifetime = {:?}",
        opts.get_max_lifetime()
    );

    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("ledger.db");
    let file = Ledger::open(path.to_str().expect("utf-8 path"))
        .await
        .expect("open file-backed ledger");
    let fopts = file.pool().options();
    assert!(
        fopts.get_idle_timeout().is_some() || fopts.get_max_lifetime().is_some(),
        "a FILE-backed ledger must keep sqlx's defaults — its schema is on disk and \
         cannot evaporate, so pinning connections open there would be a production \
         change made to fix a test-only property"
    );
}
