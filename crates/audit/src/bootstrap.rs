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
