//! Read-only query surface (T07).
//!
//! No `sqlx` types in the public API. All amounts are returned as `Decimal` or
//! `Money<Usdt>` — never raw `String` or `sqlx` row types.
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;
use trading_core::{
    AccountId, FillView, JournalEntryView, LedgerError, Money, Price, Quantity, Side,
    StrategyEventKind, StrategyEventView, StrategyId, Symbol, Timestamp, Usdt,
};

use crate::Ledger;

// ── Cash & equity ─────────────────────────────────────────────────────────────

/// Cash balance for `assets:cash:USDT` account (credits − debits).
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error, [`LedgerError::Database`]
/// on decimal parse failure.
pub async fn cash_balance(ledger: &Ledger) -> Result<Money<Usdt>, LedgerError> {
    let bal = account_balance(ledger, "assets:cash:USDT").await?;
    Ok(Money::from_decimal(bal))
}

/// Total realized P&L since `ts` (credits − debits on `income:realized_pnl`).
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error or parse failure.
pub async fn realized_pnl_since(
    ledger: &Ledger,
    ts: Timestamp,
) -> Result<Money<Usdt>, LedgerError> {
    let ts_str = ts
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT debit_amount, credit_amount \
         FROM journal_entries \
         WHERE account_id = 'income:realized_pnl' AND ts >= ?",
    )
    .bind(&ts_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut total = dec!(0);
    for (dr, cr) in rows {
        let dr: Decimal = dr
            .parse()
            .map_err(|_| LedgerError::Database("parse error".into()))?;
        let cr: Decimal = cr
            .parse()
            .map_err(|_| LedgerError::Database("parse error".into()))?;
        // income: credit increases balance, debit decreases
        total += cr - dr;
    }
    Ok(Money::from_decimal(total))
}

/// Total fee spend (debits on `expense:fees:taker` and `expense:fees:maker`).
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error or parse failure.
pub async fn total_fees(ledger: &Ledger) -> Result<Money<Usdt>, LedgerError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT debit_amount, credit_amount \
         FROM journal_entries \
         WHERE account_id IN ('expense:fees:taker', 'expense:fees:maker')",
    )
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut total = dec!(0);
    for (dr, _cr) in rows {
        let dr: Decimal = dr
            .parse()
            .map_err(|_| LedgerError::Database("parse error".into()))?;
        total += dr;
    }
    Ok(Money::from_decimal(total))
}

// ── Account list ───────────────────────────────────────────────────────────────

/// List all account IDs in the chart of accounts.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error.
pub async fn account_list(ledger: &Ledger) -> Result<Vec<String>, LedgerError> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM accounts ORDER BY id")
        .fetch_all(&ledger.pool)
        .await
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

// ── Transaction-level balance check ───────────────────────────────────────────

/// Verify that for a given transaction `Σ debits == Σ credits`.
/// Calls through to [`crate::journal::verify_balance`].
///
/// # Errors
///
/// Returns [`LedgerError::Imbalance`] if sums differ by more than 1e-8, or
/// [`LedgerError::Database`] on SQL/parse error.
pub async fn verify_transaction_balance(
    ledger: &Ledger,
    transaction_id: &str,
) -> Result<(), LedgerError> {
    crate::journal::verify_balance(ledger, transaction_id).await
}

// ── Recent fills ──────────────────────────────────────────────────────────────

/// Return the last `limit` fills, newest first.
///
/// Reconstructs `FillView` from the journal entries for buy/sell transactions.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn recent_fills(ledger: &Ledger, limit: usize) -> Result<Vec<FillView>, LedgerError> {
    // We reconstruct fills from journal_transactions tagged with a fill
    // description: "<side> <qty> <symbol> @ <price>".
    let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, ts, description \
         FROM journal_transactions \
         WHERE description LIKE 'buy %' OR description LIKE 'sell %' \
         ORDER BY ts DESC \
         LIMIT ?",
    )
    .bind(limit_i64)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut fills = Vec::with_capacity(rows.len());
    for (txn_id, ts_str, desc) in rows {
        let Some(fill) = parse_fill_view_from_description(&txn_id, &ts_str, &desc, ledger).await?
        else {
            continue;
        };
        fills.push(fill);
    }
    Ok(fills)
}

/// Parse a `FillView` from the transaction description + fee entry.
async fn parse_fill_view_from_description(
    txn_id: &str,
    ts_str: &str,
    desc: &str,
    ledger: &Ledger,
) -> Result<Option<FillView>, LedgerError> {
    // desc format: "<side> <qty> <symbol> @ <price>"
    let parts: Vec<&str> = desc.splitn(5, ' ').collect();
    if parts.len() < 4 {
        return Ok(None);
    }
    let side = match parts[0] {
        "buy" => Side::Buy,
        "sell" => Side::Sell,
        _ => return Ok(None),
    };
    let qty_d: Decimal = parts[1]
        .parse()
        .map_err(|_| LedgerError::Database(format!("bad qty in desc: {desc}")))?;
    let symbol = Symbol::new(parts[2]);
    // parts[3] is "@", parts[4] is price
    if parts.len() < 5 {
        return Ok(None);
    }
    let price_d: Decimal = parts[4]
        .parse()
        .map_err(|_| LedgerError::Database(format!("bad price in desc: {desc}")))?;

    let qty = Quantity::new(qty_d).map_err(|e| LedgerError::Database(e.to_string()))?;
    let price = Price::new(price_d).map_err(|e| LedgerError::Database(e.to_string()))?;

    // Look up fee from expense:fees:taker entry in this transaction
    let fee_rows: Vec<(String,)> = sqlx::query_as(
        "SELECT debit_amount FROM journal_entries \
         WHERE transaction_id = ? AND account_id = 'expense:fees:taker'",
    )
    .bind(txn_id)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let fee_amount = fee_rows
        .first()
        .map(|(dr,)| dr.parse::<Decimal>().unwrap_or(dec!(0)))
        .unwrap_or(dec!(0));
    let fee = Money::<Usdt>::from_decimal(fee_amount);

    let venue_ts = OffsetDateTime::parse(ts_str, &Rfc3339)
        .map(Timestamp::new)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    Ok(Some(FillView {
        symbol,
        side,
        price,
        qty,
        fee,
        fee_tier: trading_core::FeeTier::Taker,
        venue_ts,
    }))
}

// ── Recent journal entries ─────────────────────────────────────────────────────

/// Return the last `limit` journal entry lines, newest first.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn recent_journal(
    ledger: &Ledger,
    limit: usize,
) -> Result<Vec<JournalEntryView>, LedgerError> {
    let limit_i64 = i64::try_from(limit).unwrap_or(i64::MAX);
    let rows: Vec<(String, String, String, String)> = sqlx::query_as(
        "SELECT account_id, debit_amount, credit_amount, ts \
         FROM journal_entries \
         ORDER BY ts DESC, rowid DESC \
         LIMIT ?",
    )
    .bind(limit_i64)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut entries = Vec::with_capacity(rows.len());
    for (account, dr_str, cr_str, ts_str) in rows {
        let dr: Decimal = dr_str
            .parse()
            .map_err(|_| LedgerError::Database("parse error".into()))?;
        let cr: Decimal = cr_str
            .parse()
            .map_err(|_| LedgerError::Database("parse error".into()))?;
        let amount = cr - dr; // positive = credit dominates
        let ts = OffsetDateTime::parse(&ts_str, &Rfc3339)
            .map(Timestamp::new)
            .map_err(|e| LedgerError::Database(e.to_string()))?;
        entries.push(JournalEntryView {
            account: AccountId::new(account),
            amount,
            ts,
            memo: String::new(),
        });
    }
    Ok(entries)
}

// ── Transaction verification helpers (used by integration tests) ───────────────

/// List all transaction IDs in the journal, ordered by ts.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error.
pub async fn all_transaction_ids(ledger: &Ledger) -> Result<Vec<String>, LedgerError> {
    let rows: Vec<(String,)> = sqlx::query_as("SELECT id FROM journal_transactions ORDER BY ts")
        .fetch_all(&ledger.pool)
        .await
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    Ok(rows.into_iter().map(|(id,)| id).collect())
}

/// Sum of all debit and credit amounts across every journal entry.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error or parse failure.
pub async fn global_debit_credit_sum(ledger: &Ledger) -> Result<(Decimal, Decimal), LedgerError> {
    let rows: Vec<(String, String)> =
        sqlx::query_as("SELECT debit_amount, credit_amount FROM journal_entries")
            .fetch_all(&ledger.pool)
            .await
            .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut sum_debits = dec!(0);
    let mut sum_credits = dec!(0);
    for (debit_str, credit_str) in &rows {
        sum_debits += debit_str
            .parse::<Decimal>()
            .map_err(|_| LedgerError::Database("parse debit".into()))?;
        sum_credits += credit_str
            .parse::<Decimal>()
            .map_err(|_| LedgerError::Database("parse credit".into()))?;
    }
    Ok((sum_debits, sum_credits))
}

// ── Strategy events ───────────────────────────────────────────────────────────

/// Return all strategy lifecycle events since `ts`, oldest first.
///
/// No `sqlx` types in the return type — all fields are from `trading_core`.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn strategy_events_since(
    ledger: &Ledger,
    ts: Timestamp,
) -> Result<Vec<StrategyEventView>, LedgerError> {
    let ts_str = ts
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, ts, kind, strategy_id, old_hash, new_hash, source_path, operator, error_code, error_summary \
         FROM strategy_events \
         WHERE ts >= ? \
         ORDER BY ts ASC, rowid ASC",
    )
    .bind(&ts_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    rows.into_iter().map(parse_strategy_event_view).collect()
}

/// Return all strategy lifecycle events for a given strategy id, oldest first.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn strategy_history(
    ledger: &Ledger,
    id: StrategyId,
) -> Result<Vec<StrategyEventView>, LedgerError> {
    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, String, String, Option<String>, Option<String>, Option<String>, Option<String>, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT id, ts, kind, strategy_id, old_hash, new_hash, source_path, operator, error_code, error_summary \
         FROM strategy_events \
         WHERE strategy_id = ? \
         ORDER BY ts ASC, rowid ASC",
    )
    .bind(id.0.as_str())
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    rows.into_iter().map(parse_strategy_event_view).collect()
}

/// Parse a strategy event row from a `SQLite` query result.
#[allow(clippy::type_complexity)]
fn parse_strategy_event_view(
    row: (
        String,
        String,
        String,
        Option<String>,
        Option<String>,
        Option<String>,
        Option<String>,
        String,
        Option<String>,
        Option<String>,
    ),
) -> Result<StrategyEventView, LedgerError> {
    let (
        id,
        ts_str,
        kind_str,
        strategy_id,
        old_hash,
        new_hash,
        source_path,
        operator,
        error_code,
        error_summary,
    ) = row;

    let ts = OffsetDateTime::parse(&ts_str, &Rfc3339)
        .map(Timestamp::new)
        .map_err(|e| LedgerError::Database(format!("bad ts in strategy_events: {e}")))?;

    let kind = match kind_str.as_str() {
        "Load" => StrategyEventKind::Load,
        "Swap" => StrategyEventKind::Swap,
        "Unload" => StrategyEventKind::Unload,
        "Reject" => StrategyEventKind::Reject,
        other => {
            return Err(LedgerError::Database(format!(
                "unknown strategy event kind: {other}"
            )))
        }
    };

    Ok(StrategyEventView {
        id: SmolStr::new(&id),
        ts,
        kind,
        strategy_id: strategy_id.map(|s| StrategyId::new(s.as_str())),
        old_hash: old_hash.map(SmolStr::new),
        new_hash: new_hash.map(SmolStr::new),
        source_path: source_path.map(SmolStr::new),
        operator: SmolStr::new(&operator),
        error_code: error_code.map(SmolStr::new),
        error_summary: error_summary.map(SmolStr::new),
    })
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Net balance of an account: `Σ credits − Σ debits` for asset/income accounts,
/// `Σ debits − Σ credits` for expense accounts.
///
/// For simplicity we return `credits − debits` for all accounts so callers
/// interpret the sign themselves.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error or parse failure.
async fn account_balance(ledger: &Ledger, account_id: &str) -> Result<Decimal, LedgerError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT debit_amount, credit_amount FROM journal_entries WHERE account_id = ?",
    )
    .bind(account_id)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut total = dec!(0);
    for (dr, cr) in rows {
        let dr: Decimal = dr
            .parse()
            .map_err(|_| LedgerError::Database("parse error".into()))?;
        let cr: Decimal = cr
            .parse()
            .map_err(|_| LedgerError::Database("parse error".into()))?;
        total += cr - dr;
    }
    Ok(total)
}
