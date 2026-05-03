//! T1101 — Migration 006_per_symbol_position_accounts smoke test.
//!
//! Asserts:
//!   1. A fresh in-memory ledger (which auto-applies all migrations via
//!      `sqlx::migrate!("./migrations")` at `Ledger::open` time) carries the
//!      10 per-symbol `assets:position:<SYMBOL>` rows in the `accounts`
//!      table after migrations apply.
//!   2. Re-opening / re-running migrations is idempotent (no error, no
//!      duplicate rows) — the migration uses `INSERT OR IGNORE` so a second
//!      apply against the same DB is a no-op.

use audit::Ledger;

/// The 10 universe symbols enumerated in `006_per_symbol_position_accounts.sql`,
/// matching `config/agent.toml [funding].universe` (lines 62-65).
const UNIVERSE: &[&str] = &[
    "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "XRPUSDT", "ADAUSDT", "DOGEUSDT", "AVAXUSDT",
    "DOTUSDT", "LINKUSDT",
];

#[tokio::test]
async fn t1101_migration_006_seeds_per_symbol_accounts() {
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");

    for symbol in UNIVERSE {
        let id = format!("assets:position:{symbol}");
        let rows: Vec<(String, String, String)> =
            sqlx::query_as("SELECT id, kind, currency FROM accounts WHERE id = ?")
                .bind(&id)
                .fetch_all(ledger.pool())
                .await
                .expect("select account row");
        assert_eq!(
            rows.len(),
            1,
            "expected exactly one chart-of-accounts row for {id}"
        );
        let (got_id, kind, currency) = &rows[0];
        assert_eq!(got_id, &id);
        assert_eq!(kind, "asset", "kind must be 'asset' for {id}");
        assert_eq!(currency, "USDT", "currency must be 'USDT' for {id}");
    }

    // Coverage: assert all 10 rows present in a single query (defence in
    // depth against typos in the per-symbol assertions above).
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM accounts WHERE id LIKE 'assets:position:%USDT'")
            .fetch_one(ledger.pool())
            .await
            .expect("count per-symbol accounts");
    assert_eq!(
        count.0, 10,
        "expected 10 per-symbol position accounts; got {}",
        count.0
    );
}

#[tokio::test]
async fn t1101_migration_006_is_idempotent() {
    // First open applies migrations 001..006.
    let ledger = Ledger::in_memory().await.expect("open in-memory ledger");

    // Re-running the migration's INSERT statements directly against the same
    // pool exercises the `INSERT OR IGNORE` idempotency clause: no error,
    // no duplicate rows.
    for symbol in UNIVERSE {
        let id = format!("assets:position:{symbol}");
        sqlx::query(
            "INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES (?, 'asset', 'USDT')",
        )
        .bind(&id)
        .execute(ledger.pool())
        .await
        .expect("re-apply INSERT OR IGNORE");
    }

    // Row count for per-symbol position accounts is still exactly 10.
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM accounts WHERE id LIKE 'assets:position:%USDT'")
            .fetch_one(ledger.pool())
            .await
            .expect("count per-symbol accounts after re-apply");
    assert_eq!(
        count.0, 10,
        "re-applying migration 006 must not duplicate rows; got {}",
        count.0
    );
}
