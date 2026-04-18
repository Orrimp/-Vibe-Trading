//! Journal entry writers (R3.3).
//!
//! Every fill writes a balanced double-entry transaction atomically.
//! Debits == Credits is enforced per transaction.
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::instrument;
use trading_core::{Fill, LedgerError, Side};
use uuid::Uuid;

use crate::Ledger;

/// Post a fill as balanced double-entry journal entries (R3.3).
///
/// Buy fill of `q` BTC @ `p` USDT with fee `f` USDT:
/// - Dr `assets:position:BTC`  `q * p`
/// - Cr `assets:cash:USDT`     `q * p`
/// - Dr `expense:fees:taker`   `f`
/// - Cr `assets:cash:USDT`     `f`
///
/// Sell fill is the mirror.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL transaction fails.
#[allow(clippy::too_many_lines)] // double-entry for buy and sell requires this length
#[instrument(name = "ledger.post_fill", skip(ledger, fill), fields(fill_id = %fill.id))]
pub async fn post_fill(ledger: &Ledger, fill: &Fill) -> Result<(), LedgerError> {
    let txn_id = Uuid::new_v4().to_string();
    let ts = fill
        .venue_ts
        .inner()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let notional = fill.qty.get() * fill.price.get();
    let fee = fill.fee.amount();
    let description = format!(
        "{} {} {} @ {}",
        fill.side, fill.qty, fill.symbol, fill.price
    );

    let mut db_txn = ledger
        .pool
        .begin()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // Insert transaction header
    sqlx::query("INSERT INTO journal_transactions (id, ts, description) VALUES (?, ?, ?)")
        .bind(&txn_id)
        .bind(&ts)
        .bind(&description)
        .execute(&mut *db_txn)
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    match fill.side {
        Side::Buy => {
            // Dr assets:position:BTC  notional
            insert_entry(
                &mut db_txn,
                &txn_id,
                &ts,
                "assets:position:BTC",
                notional,
                dec!(0),
            )
            .await?;
            // Cr assets:cash:USDT     notional
            insert_entry(
                &mut db_txn,
                &txn_id,
                &ts,
                "assets:cash:USDT",
                dec!(0),
                notional,
            )
            .await?;
            // Dr expense:fees:taker   fee
            insert_entry(
                &mut db_txn,
                &txn_id,
                &ts,
                "expense:fees:taker",
                fee,
                dec!(0),
            )
            .await?;
            // Cr assets:cash:USDT     fee
            insert_entry(&mut db_txn, &txn_id, &ts, "assets:cash:USDT", dec!(0), fee).await?;
        }
        Side::Sell => {
            let cost_basis_per_unit = if fill.qty.get() > dec!(0) {
                // For v0 we use fill price as cost basis (simplified; FIFO in v0.5)
                fill.price.get()
            } else {
                fill.price.get()
            };
            let cost = fill.qty.get() * cost_basis_per_unit;
            let realized = notional - cost;

            // Dr assets:cash:USDT     notional
            insert_entry(
                &mut db_txn,
                &txn_id,
                &ts,
                "assets:cash:USDT",
                notional,
                dec!(0),
            )
            .await?;
            // Cr assets:position:BTC  cost
            insert_entry(
                &mut db_txn,
                &txn_id,
                &ts,
                "assets:position:BTC",
                dec!(0),
                cost,
            )
            .await?;
            // Realized P&L (positive = profit, recorded as credit to income)
            if realized > dec!(0) {
                insert_entry(
                    &mut db_txn,
                    &txn_id,
                    &ts,
                    "income:realized_pnl",
                    dec!(0),
                    realized,
                )
                .await?;
            } else {
                // Loss: debit income:realized_pnl
                insert_entry(
                    &mut db_txn,
                    &txn_id,
                    &ts,
                    "income:realized_pnl",
                    realized.abs(),
                    dec!(0),
                )
                .await?;
            }
            // Dr expense:fees:taker   fee
            insert_entry(
                &mut db_txn,
                &txn_id,
                &ts,
                "expense:fees:taker",
                fee,
                dec!(0),
            )
            .await?;
            // Cr assets:cash:USDT     fee
            insert_entry(&mut db_txn, &txn_id, &ts, "assets:cash:USDT", dec!(0), fee).await?;
        }
    }

    db_txn
        .commit()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    tracing::debug!(fill_id = %fill.id, side = %fill.side, notional = %notional, "fill journaled");
    Ok(())
}

/// Insert a single journal entry line.
async fn insert_entry(
    txn: &mut sqlx::Transaction<'_, sqlx::Sqlite>,
    txn_id: &str,
    ts: &str,
    account_id: &str,
    debit: Decimal,
    credit: Decimal,
) -> Result<(), LedgerError> {
    let entry_id = Uuid::new_v4().to_string();
    sqlx::query(
        "INSERT INTO journal_entries \
         (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&entry_id)
    .bind(txn_id)
    .bind(account_id)
    .bind(debit.to_string())
    .bind(credit.to_string())
    .bind(ts)
    .execute(&mut **txn)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}

/// Write a registry-mutation memo entry (R3.4).
///
/// Zero-amount memo entries preserve the balance invariant.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.registry_event", skip(ledger))]
pub async fn registry_event(
    ledger: &Ledger,
    event_kind: &str,
    strategy_id: &str,
    metadata: &str,
) -> Result<(), LedgerError> {
    let txn_id = Uuid::new_v4().to_string();
    let ts = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let description = format!("registry:{event_kind}:{strategy_id}");

    let mut db_txn = ledger
        .pool
        .begin()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, metadata) VALUES (?, ?, ?, ?)",
    )
    .bind(&txn_id)
    .bind(&ts)
    .bind(&description)
    .bind(metadata)
    .execute(&mut *db_txn)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // Zero-amount memo entry on equity:opening_balance
    insert_entry(
        &mut db_txn,
        &txn_id,
        &ts,
        "equity:opening_balance",
        dec!(0),
        dec!(0),
    )
    .await?;

    db_txn
        .commit()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}

/// Post a kill-switch trip as a memo entry (R7.2).
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.kill_switch_trip", skip(ledger))]
pub async fn kill_switch_tripped(
    ledger: &Ledger,
    reason: &str,
    operator: &str,
) -> Result<(), LedgerError> {
    let metadata = serde_json::json!({
        "event": "KillSwitchTripped",
        "reason": reason,
        "operator": operator,
    })
    .to_string();
    registry_event(ledger, "KillSwitchTripped", reason, &metadata).await
}

/// Post an LLM / infra cost as a balanced journal entry (T30).
///
/// - Dr `expense:llm:<tier>` usd
/// - Cr `liabilities:llm_accrued` usd
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.post_cost", skip(ledger))]
pub async fn post_cost(
    ledger: &Ledger,
    tier: &str,
    usd: rust_decimal::Decimal,
) -> Result<(), LedgerError> {
    if usd == dec!(0) {
        return Ok(());
    }
    let txn_id = Uuid::new_v4().to_string();
    let ts = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let description = format!("llm_cost:{tier}");
    let expense_account = format!("expense:llm:{tier}");

    let mut db_txn = ledger
        .pool
        .begin()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    sqlx::query("INSERT INTO journal_transactions (id, ts, description) VALUES (?, ?, ?)")
        .bind(&txn_id)
        .bind(&ts)
        .bind(&description)
        .execute(&mut *db_txn)
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // Dr expense:llm:<tier>
    insert_entry(&mut db_txn, &txn_id, &ts, &expense_account, usd, dec!(0)).await?;
    // Cr liabilities:llm_accrued
    insert_entry(
        &mut db_txn,
        &txn_id,
        &ts,
        "liabilities:llm_accrued",
        dec!(0),
        usd,
    )
    .await?;

    db_txn
        .commit()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}

/// Verify that for a given transaction `Σ debits == Σ credits`.
///
/// # Errors
///
/// Returns [`LedgerError::Imbalance`] if the sums differ by more than 1e-8.
pub async fn verify_balance(ledger: &Ledger, transaction_id: &str) -> Result<(), LedgerError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT debit_amount, credit_amount FROM journal_entries WHERE transaction_id = ?",
    )
    .bind(transaction_id)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut total_debit = dec!(0);
    let mut total_credit = dec!(0);
    for (dr, cr) in rows {
        let dr: Decimal = dr
            .parse()
            .map_err(|_| LedgerError::Database("parse error".into()))?;
        let cr: Decimal = cr
            .parse()
            .map_err(|_| LedgerError::Database("parse error".into()))?;
        total_debit += dr;
        total_credit += cr;
    }

    let tolerance = dec!(0.00000001);
    if (total_debit - total_credit).abs() > tolerance {
        return Err(LedgerError::Imbalance {
            debits: total_debit,
            credits: total_credit,
        });
    }
    Ok(())
}
