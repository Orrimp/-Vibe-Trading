//! Chart-of-accounts bootstrap (R3.2).
//!
//! Creates all required accounts if they do not already exist.
use tracing::instrument;
use trading_core::LedgerError;

use crate::Ledger;

/// All v0 account IDs in the chart of accounts (canonical count: 13).
///
/// The three LLM cost accounts (`expense:llm:deep_think`,
/// `expense:llm:quick_think`, `liabilities:llm_accrued`) and the two
/// infra/data cost accounts (`expense:infra`, `expense:data`) are
/// pre-created in v0 even though v0 posts zero entries to them. This means
/// the v0.5 cost-telemetry work (T30) can start posting without a schema
/// migration.
pub const ACCOUNTS: &[(&str, &str, &str)] = &[
    ("assets:cash:USDT", "asset", "USDT"),
    ("assets:position:BTC", "asset", "BTC"),
    ("assets:position_mark:BTC", "asset", "BTC"),
    ("income:realized_pnl", "income", "USDT"),
    ("income:unrealized_pnl", "income", "USDT"),
    ("expense:fees:taker", "expense", "USDT"),
    ("expense:fees:maker", "expense", "USDT"),
    // LLM cost accounts — zero entries in v0; pre-seeded for v0.5 (R10)
    ("expense:llm:deep_think", "expense", "USD"),
    ("expense:llm:quick_think", "expense", "USD"),
    ("liabilities:llm_accrued", "liability", "USD"),
    // Infra and data cost accounts — zero entries in v0; pre-seeded for v1+
    ("expense:infra", "expense", "USD"),
    ("expense:data", "expense", "USD"),
    ("equity:opening_balance", "equity", "USDT"),
];

/// Create all v0 ledger accounts if they do not exist.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on any SQL error.
#[instrument(name = "ledger.bootstrap", skip_all)]
pub async fn chart_of_accounts(ledger: &Ledger) -> Result<(), LedgerError> {
    for (id, kind, currency) in ACCOUNTS {
        sqlx::query("INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES (?, ?, ?)")
            .bind(id)
            .bind(kind)
            .bind(currency)
            .execute(&ledger.pool)
            .await
            .map_err(|e| LedgerError::Database(e.to_string()))?;
    }
    Ok(())
}

/// Idempotently seed v1 universe asset accounts for the given base assets (T610).
///
/// Creates `assets:position:<ASSET>` and `assets:position_mark:<ASSET>` for
/// each base asset that is not already in the chart of accounts.
///
/// Uses `INSERT OR IGNORE` so restarting the agent is a no-op.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on any SQL error.
#[deprecated(
    since = "1.6.0",
    note = "shape mismatch — takes base assets (e.g. \"BTC\") but \
            migration 006_per_symbol_position_accounts.sql seeds \
            pair symbols (e.g. \"BTCUSDT\"). The migration is the \
            canonical seed; this function has zero callers and \
            will be removed in a follow-up wave."
)]
#[instrument(name = "ledger.bootstrap_v1_universe", skip_all)]
pub async fn seed_universe_accounts(
    ledger: &Ledger,
    base_assets: &[&str],
) -> Result<(), LedgerError> {
    for asset in base_assets {
        let position_id = format!("assets:position:{asset}");
        let position_mark_id = format!("assets:position_mark:{asset}");

        sqlx::query("INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES (?, ?, ?)")
            .bind(&position_id)
            .bind("asset")
            .bind(*asset)
            .execute(&ledger.pool)
            .await
            .map_err(|e| LedgerError::Database(e.to_string()))?;

        sqlx::query("INSERT OR IGNORE INTO accounts (id, kind, currency) VALUES (?, ?, ?)")
            .bind(&position_mark_id)
            .bind("asset")
            .bind(*asset)
            .execute(&ledger.pool)
            .await
            .map_err(|e| LedgerError::Database(e.to_string()))?;
    }
    Ok(())
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use crate::Ledger;

    /// T-D-N7 — migration 010 (`training_events` table) applies cleanly on a
    /// fresh in-memory DB and is idempotent on re-apply.
    ///
    /// `Ledger::in_memory()` calls `sqlx::migrate!("./migrations").run(...)`,
    /// which applies 001..010 in order. We assert the `training_events` table
    /// exists and has the expected columns by running a SELECT.
    ///
    /// Idempotency: `Ledger::in_memory()` is called twice (two separate pools).
    /// `SQLite`'s `IF NOT EXISTS` guarantees the second apply is a no-op and does
    /// not raise an error. We verify no error is returned on the second open.
    #[tokio::test]
    async fn migration_010() {
        // First apply — fresh DB.
        let ledger = Ledger::in_memory().await.expect("first open must succeed");

        // Assert table exists + all expected columns present.
        let cols: Vec<(String,)> =
            sqlx::query_as("SELECT name FROM pragma_table_info('training_events') ORDER BY name")
                .fetch_all(ledger.pool())
                .await
                .expect("pragma_table_info must work");

        let col_names: Vec<&str> = cols.iter().map(|(n,)| n.as_str()).collect();
        let expected = [
            "epoch",
            "error_message",
            "id",
            "kind",
            "model_revision",
            "pid",
            "run_id",
            "scenario",
            "seed",
            "total_epochs",
            "train_loss",
            "ts",
            "val_loss",
            "wall_clock_ms",
        ];
        for col in &expected {
            assert!(
                col_names.contains(col),
                "column '{col}' missing from training_events; found: {col_names:?}"
            );
        }

        // Second apply — same URL pattern (in-memory gives a fresh pool each time,
        // but the migration state is tracked per-pool). We just verify no error.
        let ledger2 = Ledger::in_memory()
            .await
            .expect("second open must also succeed");
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM training_events")
            .fetch_one(ledger2.pool())
            .await
            .expect("count must work on fresh table");
        assert_eq!(count.0, 0, "training_events must start empty");
    }
}
