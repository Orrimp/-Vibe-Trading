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
    AccountId, FillView, FundingObs, JournalEntryView, LedgerError, Money, OpenPosition, PairKey,
    PairMembership, Price, Quantity, Side, StrategyEventKind, StrategyEventView, StrategyId,
    Symbol, Timestamp, Usdt,
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
        "RebalanceRejected" | "rebalance_rejected" => StrategyEventKind::RebalanceRejected,
        // v1.5a Q8 variants (T707)
        "MeanReversionStop" | "mean_reversion_stop" => StrategyEventKind::MeanReversionStop,
        "PairShortObservation" | "pair_short_observation" => {
            StrategyEventKind::PairShortObservation
        }
        // v1+ Q8 / R7.1 variants (T801 / T805 / T809)
        "KillSwitchTripped" | "kill_switch_tripped" => StrategyEventKind::KillSwitchTripped,
        "FeedReconnect" | "feed_reconnect" => StrategyEventKind::FeedReconnect,
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

// ── Per-symbol P&L attribution (v1 T609) ─────────────────────────────────────

/// Return per-symbol realized P&L in `[since, until]`, sorted alphabetically.
///
/// Aggregates credits minus debits on `income:realized_pnl` split by symbol
/// (derived from the transaction description pattern `"<side> <qty> <symbol> @ <price>"`).
/// Zero-P&L symbols are omitted.
///
/// # P&L sum invariant (R8.5)
///
/// `Σ pnl_by_symbol(since, until) == realized_pnl_since(since)` when
/// `until` is the end of the window.  Asserted by integration tests.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn pnl_by_symbol(
    ledger: &Ledger,
    since: Timestamp,
    until: Timestamp,
) -> Result<Vec<(Symbol, Money<Usdt>)>, LedgerError> {
    let since_str = since
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let until_str = until
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    // Join journal_entries on income:realized_pnl with journal_transactions
    // to extract the symbol from the description.
    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT je.debit_amount, je.credit_amount, je.ts, jt.id, jt.description \
         FROM journal_entries je \
         JOIN journal_transactions jt ON je.transaction_id = jt.id \
         WHERE je.account_id = 'income:realized_pnl' \
           AND je.ts >= ? AND je.ts <= ?",
    )
    .bind(&since_str)
    .bind(&until_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    // Accumulate per-symbol P&L.
    let mut by_symbol: std::collections::BTreeMap<Symbol, Decimal> =
        std::collections::BTreeMap::new();

    for (dr_str, cr_str, _ts_str, _txn_id, description) in rows {
        let dr: Decimal = dr_str
            .parse()
            .map_err(|_| LedgerError::Database("pnl_by_symbol: parse debit".into()))?;
        let cr: Decimal = cr_str
            .parse()
            .map_err(|_| LedgerError::Database("pnl_by_symbol: parse credit".into()))?;
        let pnl_delta = cr - dr; // income: credit = profit, debit = loss

        // Extract symbol from transaction description.
        // Format: "<side> <qty> <symbol> @ <price>"
        let symbol = extract_symbol_from_description(&description);
        *by_symbol.entry(symbol).or_insert(Decimal::ZERO) += pnl_delta;
    }

    // Filter zero-P&L symbols, sort alphabetically (BTreeMap is already sorted).
    let result: Vec<(Symbol, Money<Usdt>)> = by_symbol
        .into_iter()
        .filter(|(_sym, pnl)| *pnl != Decimal::ZERO)
        .map(|(sym, pnl)| (sym, Money::<Usdt>::from_decimal(pnl)))
        .collect();

    Ok(result)
}

/// Extract the symbol from a journal transaction description.
///
/// Expected format: `"<side> <qty> <symbol> @ <price>"`.
/// Returns `Symbol::new("UNKNOWN")` if the format doesn't match.
fn extract_symbol_from_description(desc: &str) -> Symbol {
    let parts: Vec<&str> = desc.splitn(5, ' ').collect();
    if parts.len() >= 3 {
        Symbol::new(parts[2])
    } else {
        Symbol::new("UNKNOWN")
    }
}

// ── Per-strategy P&L attribution (v1+ T803, R5.3 / Q2) ───────────────────────

/// Per-strategy P&L + trade stats.
///
/// A struct (not tuple-of-vectors) so callers can grow new fields additively
/// without breaking call sites.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyPnl {
    /// Strategy identifier.  Pre-migration NULL rows surface as the synthetic
    /// `StrategyId::new("(unattributed)")`.
    pub strategy_id: StrategyId,
    /// Realized P&L in USDT (`Σ (credit − debit)` on `income:realized_pnl`).
    pub realized: Money<Usdt>,
    /// Closed-trade count = number of distinct journal transactions that
    /// produced at least one `income:realized_pnl` row in the window.
    pub closed_trade_count: u32,
    /// Subset of closed trades where the realized P&L was strictly positive.
    pub winning_trade_count: u32,
    /// `realized / closed_trade_count`, or `0` when `closed_trade_count == 0`.
    pub avg_trade_realized: Money<Usdt>,
}

/// Return per-strategy realized P&L + trade stats over `[since, until]`.
///
/// Pre-migration rows (`strategy_id IS NULL`) bucket into the synthetic
/// `StrategyId::new("(unattributed)")` row so historical fills surface in the
/// report under a clearly-named bucket rather than vanishing.
///
/// Returned rows are sorted by `realized` DESC, ties broken by `strategy_id`
/// ASC (R5.5).
///
/// # Sum invariant (R11.2)
///
/// `Σ rows.realized == realized_pnl_since(since)` when `until` is the end of
/// the window.  Asserted by an integration test in
/// `crates/audit/tests/pnl_by_strategy.rs`.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn pnl_by_strategy(
    ledger: &Ledger,
    since: Timestamp,
    until: Timestamp,
) -> Result<Vec<StrategyPnl>, LedgerError> {
    let since_str = since
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let until_str = until
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    // Pull every realized-pnl row in the window joined with its
    // transaction's strategy_id.  We aggregate in Rust (rather than via
    // GROUP BY) so the parse is uniform with `pnl_by_symbol` and the
    // closed-trade count uses `COUNT(DISTINCT transaction_id)` semantics
    // — every transaction id contributes at most one closed-trade tally.
    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT je.debit_amount, je.credit_amount, jt.id, jt.strategy_id \
         FROM journal_entries je \
         JOIN journal_transactions jt ON je.transaction_id = jt.id \
         WHERE je.account_id = 'income:realized_pnl' \
           AND je.ts >= ? AND je.ts <= ?",
    )
    .bind(&since_str)
    .bind(&until_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    // Per-strategy accumulator.  Keyed on strategy id (string) so NULL rows
    // bucket under the synthetic "(unattributed)" key alongside other rows
    // with the literal id.  Per-transaction stats use a side table keyed on
    // (strategy_id, transaction_id) so we count closed trades per-strategy.
    let mut realized: std::collections::BTreeMap<String, Decimal> =
        std::collections::BTreeMap::new();
    // (strategy_id, transaction_id) -> running per-txn realized delta.
    // We accumulate the per-transaction realized delta first, then derive
    // closed-trade and winning-trade counts at the end.  This keeps the
    // semantics aligned with "1 closed trade = 1 transaction that produced
    // a realized_pnl row" (a transaction may carry multiple realized_pnl
    // entries if a partial-fill ladder writes more than one row).
    let mut per_txn: std::collections::BTreeMap<(String, String), Decimal> =
        std::collections::BTreeMap::new();

    for (dr_str, cr_str, txn_id, strategy_id) in rows {
        let dr: Decimal = dr_str
            .parse()
            .map_err(|_| LedgerError::Database("pnl_by_strategy: parse debit".into()))?;
        let cr: Decimal = cr_str
            .parse()
            .map_err(|_| LedgerError::Database("pnl_by_strategy: parse credit".into()))?;
        let pnl_delta = cr - dr;

        let sid = strategy_id.unwrap_or_else(|| "(unattributed)".to_string());

        *realized.entry(sid.clone()).or_insert(Decimal::ZERO) += pnl_delta;
        *per_txn.entry((sid, txn_id)).or_insert(Decimal::ZERO) += pnl_delta;
    }

    // Per-strategy closed-trade and winning-trade counts.
    let mut closed_count: std::collections::BTreeMap<String, u32> =
        std::collections::BTreeMap::new();
    let mut winning_count: std::collections::BTreeMap<String, u32> =
        std::collections::BTreeMap::new();
    for ((sid, _txn_id), txn_realized) in &per_txn {
        *closed_count.entry(sid.clone()).or_insert(0) += 1;
        if *txn_realized > Decimal::ZERO {
            *winning_count.entry(sid.clone()).or_insert(0) += 1;
        }
    }

    // Build StrategyPnl rows.
    let mut result: Vec<StrategyPnl> = realized
        .into_iter()
        .map(|(sid, realized_sum)| {
            let closed = closed_count.get(&sid).copied().unwrap_or(0);
            let winning = winning_count.get(&sid).copied().unwrap_or(0);
            let avg = if closed == 0 {
                Decimal::ZERO
            } else {
                realized_sum / Decimal::from(closed)
            };
            StrategyPnl {
                strategy_id: StrategyId::new(sid.as_str()),
                realized: Money::<Usdt>::from_decimal(realized_sum),
                closed_trade_count: closed,
                winning_trade_count: winning,
                avg_trade_realized: Money::<Usdt>::from_decimal(avg),
            }
        })
        .collect();

    // Sort by realized DESC, ties broken by strategy_id ASC (R5.5).
    result.sort_by(|a, b| {
        b.realized
            .amount()
            .cmp(&a.realized.amount())
            .then_with(|| a.strategy_id.0.as_str().cmp(b.strategy_id.0.as_str()))
    });

    Ok(result)
}

// ── Per-pair P&L attribution (v1.5a T708) ────────────────────────────────────

/// Return per-pair realized P&L in `[since, until]`, lex-sorted by [`PairKey`].
///
/// Composes [`pnl_by_symbol`] (v1 T609) against the `&[PairMembership]` captured
/// at strategy-load time.  Only the `traded_a_symbol` leg contributes P&L in
/// formulation C (the `b` leg is never traded).  Zero-P&L pairs are omitted.
///
/// ## Multiplicity note (architect risk #3 / Q9)
///
/// When the same `a` symbol appears in more than one pair
/// (`k > 1` multiplicity — e.g. `BTCUSDT` in both `(BTCUSDT, ETHUSDT)` and
/// `(BTCUSDT, SOLUSDT)`), `pnl_by_pair[(a, b)] == pnl_by_symbol[a]` only holds
/// when `k == 1`.  For `k > 1`, `pnl_by_symbol[a]` is the **aggregate** P&L from
/// *all* pairs that traded `a`; `pnl_by_pair` reports the same aggregate value for
/// each pair that contains `a`, which means `Σ pnl_by_pair` can exceed
/// `Σ pnl_by_symbol` when `k > 1`.  Callers must be aware of this when summing
/// pair-level P&L across overlapping universes.
///
/// ## Sum invariant (R6.3) — k == 1 case only
///
/// `Σ pnl_by_pair(since, until) == Σ pnl_by_symbol(since, until)` when each
/// `a` symbol is unique across all pairs in `memberships`.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn pnl_by_pair(
    ledger: &Ledger,
    memberships: &[PairMembership],
    since: Timestamp,
    until: Timestamp,
) -> Result<Vec<(PairKey, Money<Usdt>)>, LedgerError> {
    // Build the per-symbol P&L lookup from the existing v1 query.
    let by_symbol = pnl_by_symbol(ledger, since, until).await?;
    let symbol_pnl: std::collections::HashMap<Symbol, Decimal> = by_symbol
        .into_iter()
        .map(|(sym, money)| (sym, money.amount()))
        .collect();

    // Project per-symbol P&L onto pairs via PairMembership.
    // Accumulate into BTreeMap for lex-sorted output (architect risk #1).
    let mut by_pair: std::collections::BTreeMap<PairKey, Decimal> =
        std::collections::BTreeMap::new();

    for membership in memberships {
        let pair_pnl = symbol_pnl
            .get(&membership.traded_a_symbol)
            .copied()
            .unwrap_or(Decimal::ZERO);
        // Accumulate in case the same pair appears more than once in memberships
        // (should not happen in practice but is defensive).
        *by_pair
            .entry(membership.key.clone())
            .or_insert(Decimal::ZERO) += pair_pnl;
    }

    // Filter zero-P&L pairs; BTreeMap is already lex-sorted.
    let result: Vec<(PairKey, Money<Usdt>)> = by_pair
        .into_iter()
        .filter(|(_key, pnl)| *pnl != Decimal::ZERO)
        .map(|(key, pnl)| (key, Money::<Usdt>::from_decimal(pnl)))
        .collect();

    Ok(result)
}

// ── Funding-rate history (v1 T613) ────────────────────────────────────────────

/// Return all funding-rate observations for `symbol` in `[since, until]`,
/// oldest first.
///
/// No `sqlx` types in the return type — all fields are from `trading_core`.
/// Returns an empty `Vec` if no rows match.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn funding_rate_history(
    ledger: &Ledger,
    symbol: Symbol,
    since: Timestamp,
    until: Timestamp,
) -> Result<Vec<FundingObs>, LedgerError> {
    let since_str = since
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let until_str = until
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    let rows: Vec<(String, String, String, String, String)> = sqlx::query_as(
        "SELECT symbol, funding_rate, funding_ts, next_funding_ts, poll_ts \
         FROM funding_rates \
         WHERE symbol = ? AND funding_ts >= ? AND funding_ts <= ? \
         ORDER BY funding_ts ASC",
    )
    .bind(symbol.0.as_str())
    .bind(&since_str)
    .bind(&until_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    rows.into_iter()
        .map(
            |(sym, rate_str, funding_ts_str, next_ts_str, poll_ts_str)| {
                let funding_rate: Decimal = rate_str.parse().map_err(|_| {
                    LedgerError::Database("funding_rate_history: parse rate".into())
                })?;
                let funding_ts = OffsetDateTime::parse(&funding_ts_str, &Rfc3339)
                    .map(Timestamp::new)
                    .map_err(|e| LedgerError::Database(format!("funding_ts parse: {e}")))?;
                let next_funding_ts = OffsetDateTime::parse(&next_ts_str, &Rfc3339)
                    .map(Timestamp::new)
                    .map_err(|e| LedgerError::Database(format!("next_funding_ts parse: {e}")))?;
                let poll_ts = OffsetDateTime::parse(&poll_ts_str, &Rfc3339)
                    .map(Timestamp::new)
                    .map_err(|e| LedgerError::Database(format!("poll_ts parse: {e}")))?;
                Ok(FundingObs {
                    symbol: Symbol::new(sym),
                    funding_rate,
                    funding_ts,
                    next_funding_ts,
                    poll_ts,
                })
            },
        )
        .collect()
}

// ── T806 — agent uptime intervals reader (operator success reports R7.1) ────

/// One row of the `agent_uptime` table.
///
/// `stopped_at` is `None` while the agent is running.  The reports binary
/// reads this slice over `[period_start, period_end]` and computes
/// effective uptime per the formula in
/// `spec/features/operator-success-reports.md` R7.1.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UptimeInterval {
    pub boot_id: SmolStr,
    pub started_at: Timestamp,
    pub last_heartbeat_at: Timestamp,
    pub stopped_at: Option<Timestamp>,
}

/// Return all `agent_uptime` rows whose `started_at >= since`, ordered
/// chronologically (`started_at ASC`).
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn uptime_intervals_since(
    ledger: &Ledger,
    since: Timestamp,
) -> Result<Vec<UptimeInterval>, LedgerError> {
    let since_str = since
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    let rows: Vec<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT boot_id, started_at, last_heartbeat_at, stopped_at \
         FROM agent_uptime \
         WHERE started_at >= ? \
         ORDER BY started_at ASC, boot_id ASC",
    )
    .bind(&since_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut out = Vec::with_capacity(rows.len());
    for (boot_id, started_at, last_heartbeat_at, stopped_at) in rows {
        let parse_ts = |s: &str| {
            OffsetDateTime::parse(s, &Rfc3339)
                .map(Timestamp::new)
                .map_err(|e| LedgerError::Database(format!("uptime ts parse: {e}")))
        };
        let stopped = match stopped_at {
            Some(s) => Some(parse_ts(&s)?),
            None => None,
        };
        out.push(UptimeInterval {
            boot_id: SmolStr::new(&boot_id),
            started_at: parse_ts(&started_at)?,
            last_heartbeat_at: parse_ts(&last_heartbeat_at)?,
            stopped_at: stopped,
        });
    }
    Ok(out)
}

// ── T804 — ledger snapshot SHA + inception timestamp helpers ────────────────

/// Stream-hash the `SQLite` database file at `db_path` with `sha2::Sha256`.
///
/// Used by the operator success report renderer to record the exact ledger
/// state a report was rendered from (front-matter `ledger_sha:` field —
/// R10.1).  The function reads the file in 64 KiB chunks so a multi-GiB
/// ledger does not load fully into memory.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on file IO failure (open/read).
pub fn ledger_snapshot_sha(db_path: &std::path::Path) -> Result<[u8; 32], LedgerError> {
    use sha2::{Digest, Sha256};
    use std::io::Read;

    let display = db_path.display();
    let mut file = std::fs::File::open(db_path)
        .map_err(|e| LedgerError::Database(format!("ledger_snapshot_sha: open {display}: {e}")))?;
    let mut hasher = Sha256::new();
    // Heap-allocated 64 KiB buffer — clippy::large_stack_arrays.
    let mut buf = vec![0u8; 64 * 1024];
    loop {
        let n = file
            .read(&mut buf)
            .map_err(|e| LedgerError::Database(format!("ledger_snapshot_sha: read: {e}")))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize().into())
}

/// Return the earliest `ts` across `journal_transactions` (the ledger's
/// inception timestamp).
///
/// Used by `ReportWindow::Inception` (`crates/reports/src/window.rs`) to
/// resolve to the (since, until) range covering the entire ledger.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.  Returns
/// [`LedgerError::Database`] with `"ledger_inception_ts: no transactions"`
/// when the table is empty.
pub async fn ledger_inception_ts(ledger: &Ledger) -> Result<Timestamp, LedgerError> {
    let rows: Vec<(Option<String>,)> = sqlx::query_as("SELECT MIN(ts) FROM journal_transactions")
        .fetch_all(&ledger.pool)
        .await
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    let ts_str = rows
        .into_iter()
        .next()
        .and_then(|(s,)| s)
        .ok_or_else(|| LedgerError::Database("ledger_inception_ts: no transactions".into()))?;

    OffsetDateTime::parse(&ts_str, &Rfc3339)
        .map(Timestamp::new)
        .map_err(|e| LedgerError::Database(format!("ledger_inception_ts: parse: {e}")))
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

// ── Open-positions reader (v1+ T1002 — real-mtm-unrealized-pnl) ──────────────

/// Project all fills in `journal_transactions` whose `ts <= ts` into open
/// positions per `(symbol, strategy_id)`.
///
/// Implements the architect's resolutions for the
/// `real-mtm-unrealized-pnl` feature (`spec/features/real-mtm-unrealized-pnl.md`):
/// Q1 (snapshot-vec signature parallel to `pnl_by_symbol`), Q3 (no new SQL
/// index — full-table scan over the description-prefixed rows; same pattern
/// `recent_fills` uses), Q7 (weighted-average cost basis with proportional
/// release on Sells), Q8 (long-only — net-negative qty raises
/// `LedgerError::Database`).
///
/// Symbol identification parses the **transaction description** via the
/// existing private [`extract_symbol_from_description`] helper. Per
/// `spec/features/per-symbol-position-accounts.md` Design § Q4,
/// description-parse stays the **primary** source pre- and post-T1102 so a
/// single code path covers both legacy rows (`account_id` =
/// `"assets:position:BTC"` regardless of underlying symbol) and post-T1102
/// rows (`account_id` = `format!("assets:position:{}", symbol)`). After
/// description-parse, a defensive cross-check compares the row's
/// position-side `account_id` against the parsed symbol; mismatches emit
/// `tracing::warn!` (observation-only — never raises an error) so a future
/// writer regression surfaces at read time rather than silently.
///
/// ## Algorithm
///
/// For each fill in `[inception, ts]`, ordered chronologically:
/// - Buy `qty_b @ price_b`:
///   `running_notional += qty_b * price_b; running_qty += qty_b`. The
///   first Buy (after the last `running_qty == 0` reset, or the very
///   first fill) records `opened_at` and `strategy_id` from that fill.
/// - Sell `qty_s @ price_s` (long-only, `qty_s <= running_qty`):
///   `released = (running_notional / running_qty) * qty_s;
///    running_notional -= released; running_qty -= qty_s`. If
///   `running_qty` returns to zero, the lot closes and the next Buy
///   re-opens with a fresh `opened_at` / `strategy_id`.
///
/// End-of-scan: groups with `running_qty > 0` emit `OpenPosition`;
/// `== 0` skip; `< 0` raises `LedgerError::Database`.
///
/// ## Determinism (R6)
///
/// - `BTreeMap` accumulator (no `HashMap` on the hot path), matching
///   the precedent at [`pnl_by_symbol`].
/// - `Decimal` arithmetic only; no `f64`.
/// - Output `Vec` sorted by `(symbol ASC, strategy_id ASC, None last)`
///   so two reads against the same DB return byte-identical slices.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error, and on
/// net-negative qty for any `(symbol, strategy_id)` group (Q8 — v1+ is
/// long-only; real shorts deferred to v2+).
#[allow(clippy::too_many_lines)] // double-pass fold + emit + sort requires this length
pub async fn open_positions_at(
    ledger: &Ledger,
    ts: Timestamp,
) -> Result<Vec<OpenPosition>, LedgerError> {
    /// One running open lot per `(symbol, strategy_id_string)` group.
    ///
    /// `strategy_id` is keyed as `Option<String>` (in the `BTreeMap` key)
    /// rather than `Option<StrategyId>` because `StrategyId` does not
    /// implement `Ord`; the inner `SmolStr`'s lex order is what we want
    /// for R6 determinism.
    struct Acc {
        running_qty: Decimal,
        running_notional: Decimal,
        opened_at: Timestamp,
        strategy_id: Option<StrategyId>,
    }

    let ts_str = ts
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    // Pull every Buy/Sell transaction up to `ts`, oldest first, so the fold
    // applies fills in chronological order (Q7 weighted-average semantics
    // depend on this). LEFT JOIN journal_entries on the position-side row
    // (account_id LIKE 'assets:position:%') so the Q4 cross-check can compare
    // the row's account_id against the description-parsed symbol. Pre-T1102
    // rows yield `assets:position:BTC` (regardless of underlying symbol);
    // post-T1102 rows yield `assets:position:<SYMBOL>` for the parsed symbol.
    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, String, String, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT jt.id, jt.ts, jt.description, jt.strategy_id, je.account_id \
         FROM journal_transactions jt \
         LEFT JOIN journal_entries je \
           ON je.transaction_id = jt.id \
          AND je.account_id LIKE 'assets:position:%' \
         WHERE (jt.description LIKE 'buy %' OR jt.description LIKE 'sell %') \
           AND jt.ts <= ? \
         GROUP BY jt.id \
         ORDER BY jt.ts ASC, jt.id ASC",
    )
    .bind(&ts_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut acc: std::collections::BTreeMap<(Symbol, Option<String>), Acc> =
        std::collections::BTreeMap::new();

    for (_txn_id, ts_str_row, desc, strategy_id_str, position_account_id) in rows {
        // Description format: "<side> <qty> <symbol> @ <price>"
        let parts: Vec<&str> = desc.splitn(5, ' ').collect();
        if parts.len() < 5 {
            // Same defensive skip as `parse_fill_view_from_description`.
            continue;
        }
        let side = match parts[0] {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            _ => continue,
        };
        let qty_d: Decimal = parts[1].parse().map_err(|_| {
            LedgerError::Database(format!("open_positions_at: bad qty in desc: {desc}"))
        })?;
        let price_d: Decimal = parts[4].parse().map_err(|_| {
            LedgerError::Database(format!("open_positions_at: bad price in desc: {desc}"))
        })?;

        let symbol = extract_symbol_from_description(&desc);

        // T1102 — Q4 defensive cross-check: account_id should be either the
        // legacy BTC bucket (`"assets:position:BTC"` — pre-migration row,
        // any underlying symbol) or the per-pair form
        // (`format!("assets:position:{}", symbol)` — post-T1102 row).
        // Any other value indicates a writer or renderer regression; emit
        // `tracing::warn!` and continue with the description-parsed symbol.
        // Never raises — description-parse is authoritative (Q4 primary).
        if let Some(ref account_id) = position_account_id {
            if account_id.starts_with("assets:position:") {
                let expected_per_pair = format!("assets:position:{symbol}");
                if account_id != "assets:position:BTC" && account_id.as_str() != expected_per_pair {
                    tracing::warn!(
                        target: "audit::query",
                        account_id = %account_id,
                        parsed_symbol = %symbol,
                        "open_positions_at: account_id / description-symbol mismatch; \
                         falling back to description-parsed symbol (Q4)"
                    );
                }
            }
        }

        let key = (symbol, strategy_id_str.clone());

        let row_ts = OffsetDateTime::parse(&ts_str_row, &Rfc3339)
            .map(Timestamp::new)
            .map_err(|e| LedgerError::Database(format!("open_positions_at: bad ts: {e}")))?;

        match side {
            Side::Buy => {
                let entry = acc.entry(key).or_insert_with(|| Acc {
                    running_qty: Decimal::ZERO,
                    running_notional: Decimal::ZERO,
                    opened_at: row_ts,
                    strategy_id: strategy_id_str.as_deref().map(StrategyId::new),
                });
                // If the lot was previously fully closed (`running_qty == 0`)
                // and is now re-opening, refresh `opened_at` and `strategy_id`
                // to reflect the FIRST fill of this new open lot (per Design
                // § "ts of first un-closed Buy" semantics).
                if entry.running_qty == Decimal::ZERO {
                    entry.opened_at = row_ts;
                    entry.strategy_id = strategy_id_str.as_deref().map(StrategyId::new);
                    entry.running_notional = Decimal::ZERO;
                }
                entry.running_qty += qty_d;
                entry.running_notional += qty_d * price_d;
            }
            Side::Sell => {
                let entry = acc.entry(key).or_insert_with(|| Acc {
                    running_qty: Decimal::ZERO,
                    running_notional: Decimal::ZERO,
                    opened_at: row_ts,
                    strategy_id: strategy_id_str.as_deref().map(StrategyId::new),
                });
                if entry.running_qty > Decimal::ZERO {
                    // Proportional release of the running cost basis (Q7).
                    let released = (entry.running_notional / entry.running_qty) * qty_d;
                    entry.running_notional -= released;
                }
                entry.running_qty -= qty_d;
                // Snap to zero if numerically equal, to keep the long-only
                // close detection clean (Decimal subtraction can leave
                // trailing zeros but is exact for integer-style quantities).
                if entry.running_qty == Decimal::ZERO {
                    entry.running_notional = Decimal::ZERO;
                }
            }
        }
    }

    // Materialize surviving open lots, raising on net-negative groups (Q8).
    let mut out: Vec<OpenPosition> = Vec::new();
    for ((symbol, _sid_str), entry) in acc {
        if entry.running_qty < Decimal::ZERO {
            return Err(LedgerError::Database(format!(
                "open_positions_at: net-negative qty for group ({symbol}, {sid:?}) — \
                 short positions out of scope at v1+; check ledger integrity",
                sid = entry.strategy_id.as_ref().map(|s| s.0.as_str()),
            )));
        }
        if entry.running_qty == Decimal::ZERO {
            continue;
        }
        let avg_cost_basis_d = entry.running_notional / entry.running_qty;
        out.push(OpenPosition {
            symbol,
            qty: entry.running_qty,
            avg_cost_basis: Money::<Usdt>::from_decimal(avg_cost_basis_d),
            opened_at: entry.opened_at,
            strategy_id: entry.strategy_id,
        });
    }

    // Final deterministic sort: (symbol ASC, strategy_id ASC, None last).
    // BTreeMap iteration over `(Symbol, Option<String>)` already yields
    // (symbol ASC, strategy_id ASC, None first) — re-sort here to honour
    // the architect's "None last" tiebreaker, which differs from
    // `Option<T>`'s natural ordering (None < Some).
    out.sort_by(|a, b| {
        a.symbol
            .cmp(&b.symbol)
            .then_with(|| match (&a.strategy_id, &b.strategy_id) {
                (Some(x), Some(y)) => x.0.as_str().cmp(y.0.as_str()),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            })
    });

    Ok(out)
}
