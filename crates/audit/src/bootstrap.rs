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
