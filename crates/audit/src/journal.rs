//! Journal entry writers (R3.3).
//!
//! Every fill writes a balanced double-entry transaction atomically.
//! Debits == Credits is enforced per transaction.
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use tracing::instrument;
use trading_core::{Fill, FundingObs, LedgerError, Side};
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

/// Write a strategy lifecycle event to the `strategy_events` table (T509, Q1).
///
/// This table is separate from `journal_entries` — it carries no debit/credit
/// amounts and does not affect the reconciliation invariant.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.strategy_event", skip(ledger, write), fields(kind = ?write.kind, id = write.strategy_id.unwrap_or("")))]
pub async fn strategy_event(
    ledger: &Ledger,
    write: &StrategyEventWrite<'_>,
) -> Result<(), LedgerError> {
    let row_id = Uuid::new_v4().to_string();
    // Use the caller-supplied timestamp when present (enables deterministic
    // tests with a synthetic replay clock — architect risk #4).  Fall back to
    // wall-clock time for production use where the watcher passes `None`.
    let ts = if let Some(t) = write.ts {
        t.to_owned()
    } else {
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?
    };

    sqlx::query(
        "INSERT INTO strategy_events \
         (id, ts, kind, strategy_id, old_hash, new_hash, source_path, operator, error_code, error_summary) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row_id)
    .bind(&ts)
    .bind(write.kind)
    .bind(write.strategy_id)
    .bind(write.old_hash)
    .bind(write.new_hash)
    .bind(write.source_path)
    .bind(write.operator)
    .bind(write.error_code)
    .bind(write.error_summary)
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    tracing::debug!(
        row_id = %row_id,
        kind = write.kind,
        strategy_id = ?write.strategy_id,
        "strategy_event written"
    );
    Ok(())
}

/// Kind discriminator — passed as a string to `SQLite`.
pub type StrategyEventKindStr = str;

/// Writer struct for `strategy_event` (T509).
///
/// Mirrors the `strategy_events` table schema without exposing `SQLite` types.
#[derive(Debug)]
pub struct StrategyEventWrite<'a> {
    /// `"Load"` | `"Swap"` | `"Unload"` | `"Reject"`.
    pub kind: &'a str,
    /// `None` for `Reject` when the filename stem is unparsable.
    pub strategy_id: Option<&'a str>,
    /// SHA-256 hex (64 chars) of the previous config. Present for Swap and Unload.
    pub old_hash: Option<&'a str>,
    /// SHA-256 hex (64 chars) of the new config. Present for Load and Swap.
    pub new_hash: Option<&'a str>,
    /// Repo-relative path to the source TOML.
    pub source_path: &'a str,
    /// `"system"` in v0.5.
    pub operator: &'a str,
    /// Machine-readable error code (Reject only).
    pub error_code: Option<&'a str>,
    /// One-line human error summary (Reject only).
    pub error_summary: Option<&'a str>,
    /// Optional RFC-3339 timestamp string.  When `Some`, the supplied value
    /// is written to the `ts` column instead of `OffsetDateTime::now_utc()`.
    /// Used by deterministic integration tests to inject the replay synthetic
    /// clock (architect risk #4).  Pass `None` in production code.
    pub ts: Option<&'a str>,
}

/// Write a `rebalance_rejected` event to the `strategy_events` table (T608 — v1 Q6).
///
/// Extends `strategy_events.kind` with `"rebalance_rejected"`.
/// No SQL migration — the `kind` column is TEXT.
/// Reconciler invariant preserved: `strategy_events` carries no money.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.rebalance_rejected", skip(ledger), fields(strategy_id = %strategy_id, error_code = %error_code))]
pub async fn rebalance_rejected(
    ledger: &Ledger,
    strategy_id: &str,
    error_code: &str,
    error_summary: &str,
    ts: Option<&str>,
) -> Result<(), LedgerError> {
    strategy_event(
        ledger,
        &StrategyEventWrite {
            kind: "RebalanceRejected",
            strategy_id: Some(strategy_id),
            old_hash: None,
            new_hash: None,
            source_path: "",
            operator: "system",
            error_code: Some(error_code),
            error_summary: Some(error_summary),
            ts,
        },
    )
    .await
}

/// Write a `mean_reversion_stop` event to the `strategy_events` table (T707 — v1.5a Q8).
///
/// Emitted when the hard-stop condition (`z >= z_stop`) closes a long position.
/// Extends `strategy_events.kind` with `"MeanReversionStop"`.
/// No SQL migration — the `kind` column is TEXT.
/// Reconciler invariant preserved: `strategy_events` carries no money.
///
/// `error_summary` JSON should contain `{"pair_key": "(a, b)", "z_at_stop": "4.23"}`.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.mean_reversion_stop", skip(ledger), fields(strategy_id = %strategy_id, pair_key = %pair_key))]
pub async fn mean_reversion_stop(
    ledger: &Ledger,
    strategy_id: &str,
    pair_key: &str,
    z_at_stop: &str,
    ts: Option<&str>,
) -> Result<(), LedgerError> {
    let error_summary = serde_json::json!({
        "pair_key": pair_key,
        "z_at_stop": z_at_stop,
    })
    .to_string();
    strategy_event(
        ledger,
        &StrategyEventWrite {
            kind: "MeanReversionStop",
            strategy_id: Some(strategy_id),
            old_hash: None,
            new_hash: None,
            source_path: "",
            operator: "system",
            error_code: Some("mean_reversion_stop"),
            error_summary: Some(&error_summary),
            ts,
        },
    )
    .await
}

/// Write a `pair_short_observation` event to the `strategy_events` table (T707 — v1.5a Q8).
///
/// Emitted alongside the executed long-leg buy on entry; records "would have
/// shorted `b`" in formulation C (R5.3 / Q3). No money moves.
/// Extends `strategy_events.kind` with `"PairShortObservation"`.
/// No SQL migration — the `kind` column is TEXT.
/// Reconciler invariant preserved: `strategy_events` carries no money.
///
/// `error_summary` JSON should contain `{"pair_key": "(a, b)", "z_at_entry": "-2.34"}`.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.pair_short_observation", skip(ledger), fields(strategy_id = %strategy_id, pair_key = %pair_key))]
pub async fn pair_short_observation(
    ledger: &Ledger,
    strategy_id: &str,
    pair_key: &str,
    z_at_entry: &str,
    ts: Option<&str>,
) -> Result<(), LedgerError> {
    let error_summary = serde_json::json!({
        "pair_key": pair_key,
        "z_at_entry": z_at_entry,
    })
    .to_string();
    strategy_event(
        ledger,
        &StrategyEventWrite {
            kind: "PairShortObservation",
            strategy_id: Some(strategy_id),
            old_hash: None,
            new_hash: None,
            source_path: "",
            operator: "system",
            error_code: Some("pair_short_observation"),
            error_summary: Some(&error_summary),
            ts,
        },
    )
    .await
}

/// Persist a `FundingObs` to the `funding_rates` table (T613 — v1 Q2).
///
/// This is NOT a double-entry ledger entry — it is an append-only log of
/// observation-only data. The reconciliation invariant is unaffected.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.insert_funding_obs", skip(ledger, obs), fields(symbol = %obs.symbol))]
pub async fn insert_funding_obs(ledger: &Ledger, obs: &FundingObs) -> Result<(), LedgerError> {
    let row_id = Uuid::new_v4().to_string();
    let funding_ts = obs
        .funding_ts
        .inner()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let next_funding_ts = obs
        .next_funding_ts
        .inner()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let poll_ts = obs
        .poll_ts
        .inner()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    sqlx::query(
        "INSERT INTO funding_rates \
         (id, symbol, funding_rate, funding_ts, next_funding_ts, poll_ts) \
         VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(&row_id)
    .bind(obs.symbol.0.as_str())
    .bind(obs.funding_rate.to_string())
    .bind(&funding_ts)
    .bind(&next_funding_ts)
    .bind(&poll_ts)
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    tracing::debug!(row_id = %row_id, symbol = %obs.symbol, "funding_obs persisted");
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
