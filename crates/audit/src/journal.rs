//! Journal entry writers (R3.3).
//!
//! Every fill writes a balanced double-entry transaction atomically.
//! Debits == Credits is enforced per transaction.
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tracing::instrument;
use trading_core::{Fill, FundingObs, LedgerError, Side, Venue};
use uuid::Uuid;

use crate::Ledger;

/// Post a fill as balanced double-entry journal entries (R3.3).
///
/// Buy fill of `q SYMBOL` @ `p` USDT with fee `f` USDT:
/// - Dr `assets:position:<SYMBOL>`  `q * p`  (T1102 — per-symbol-position-accounts)
/// - Cr `assets:cash:USDT`          `q * p`
/// - Dr `expense:fees:taker`        `f`
/// - Cr `assets:cash:USDT`          `f`
///
/// Sell fill is the mirror.
///
/// `<SYMBOL>` is the full Binance pair (e.g. `BTCUSDT`, `ETHUSDT`). The chart
/// of accounts row is seeded by migration `006_per_symbol_position_accounts.sql`
/// for the universe at `config/agent.toml [funding].universe`.
///
/// `strategy_id` (T802 — operator success reports R5.3 / Q2) tags the
/// fill with the strategy that emitted the signal so per-strategy
/// attribution can roll up over `[since, until]`.  `None` writes SQL
/// NULL — those rows surface in the report under the synthetic
/// `(unattributed)` bucket.  The column is storage-only; it must NOT
/// surface in the backtest report body bytes (V6 anchor gate).
///
/// Returns the generated `journal_transactions.id` (a UUID v4 string wrapped
/// in [`SmolStr`]) so the caller can stamp `Fill.transaction_id` on the
/// in-memory fill before announcing it to the engine
/// (tape-row-audit-modal Q5).
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL transaction fails.
#[allow(clippy::too_many_lines)] // double-entry for buy and sell requires this length
#[instrument(name = "ledger.post_fill", skip(ledger, fill), fields(fill_id = %fill.id, strategy_id = ?strategy_id))]
pub async fn post_fill(
    ledger: &Ledger,
    fill: &Fill,
    strategy_id: Option<&str>,
) -> Result<SmolStr, LedgerError> {
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
    // T1102 — per-symbol position account-id (per-symbol-position-accounts Q2):
    // chart-of-accounts row is seeded by migration `006`. Pre-T1102 fills wrote
    // to the literal `"assets:position:BTC"` regardless of `fill.symbol`; from
    // T1102 onward every fill targets `assets:position:<SYMBOL>` where
    // `<SYMBOL>` is the full Binance pair (e.g. `BTCUSDT`). Reader-side
    // description-parse stays the primary symbol source for backwards-compat
    // (Q4); the structural account-id is a defensive cross-check.
    let position_account_id = format!("assets:position:{}", fill.symbol);

    let mut db_txn = ledger
        .pool
        .begin()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // Insert transaction header. The `strategy_id` column was added in
    // migration 004; pre-migration rows are NULL.  The column is bound
    // verbatim and is storage-only — it never surfaces in the rendered
    // backtest report body.
    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, strategy_id) \
         VALUES (?, ?, ?, ?)",
    )
    .bind(&txn_id)
    .bind(&ts)
    .bind(&description)
    .bind(strategy_id)
    .execute(&mut *db_txn)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    match fill.side {
        Side::Buy => {
            // Dr assets:position:<SYMBOL>  notional (T1102 — per-pair account)
            insert_entry(
                &mut db_txn,
                &txn_id,
                &ts,
                &position_account_id,
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
            // Cr assets:position:<SYMBOL>  cost (T1102 — per-pair account)
            insert_entry(
                &mut db_txn,
                &txn_id,
                &ts,
                &position_account_id,
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
    Ok(SmolStr::new(&txn_id))
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

/// Post a kill-switch trip — **dual-write** under a single SQL transaction
/// (T809 — operator success reports Q8).
///
/// Writes BOTH:
///
/// 1. The v0 zero-amount memo journal row (preserved byte-for-byte for
///    backwards compatibility — same shape as [`registry_event`] would
///    produce: `journal_transactions` + a zero-amount
///    `equity:opening_balance` entry).
/// 2. A `strategy_events` row with `kind = "KillSwitchTripped"`,
///    `strategy_id = NULL`, `error_summary = <reason>`.  The
///    operator-success-report's R7 system-health row reads only this
///    second row — Q8's Migration policy: pre-existing v0 ledgers are
///    NOT retro-rewritten.
///
/// Both writes share one [`sqlx::Transaction`] so the pair is atomic —
/// either both land or neither does.  The memo row uses RFC-3339
/// second precision (matches v0 [`registry_event`] byte-for-byte).
/// The `strategy_events` row uses the 6-digit microsecond format used
/// by every other v0.5+/v1+ `strategy_event` writer (HF-3 / architect
/// risk #4 — sub-second precision keeps `ORDER BY ts` stable under
/// rapid sequential writes).
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.  On failure
/// the entire transaction rolls back; neither row is left orphaned.
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

    // Memo-row timestamp: RFC-3339 second precision — matches v0 byte-for-byte
    // (the original `registry_event` used `Rfc3339` here; do not change).
    let memo_ts = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let memo_txn_id = Uuid::new_v4().to_string();
    let memo_description = format!("registry:KillSwitchTripped:{reason}");

    // strategy_events-row timestamp: 6-digit microsecond format.  See
    // `strategy_event` / `uptime_ts_string` — sub-second precision is
    // mandatory so two consecutive writes within the same wall-clock
    // second produce monotonically-ordered `ts` values (HF-3 gate).
    let strategy_ts_fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    )
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let strategy_ts = time::OffsetDateTime::now_utc()
        .format(&strategy_ts_fmt)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let strategy_row_id = Uuid::new_v4().to_string();

    // Atomic dual-write: one transaction, both rows.
    let mut db_txn = ledger
        .pool
        .begin()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // (1) v0 memo row — backwards compat — preserved byte-for-byte.
    sqlx::query(
        "INSERT INTO journal_transactions (id, ts, description, metadata) VALUES (?, ?, ?, ?)",
    )
    .bind(&memo_txn_id)
    .bind(&memo_ts)
    .bind(&memo_description)
    .bind(&metadata)
    .execute(&mut *db_txn)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    insert_entry(
        &mut db_txn,
        &memo_txn_id,
        &memo_ts,
        "equity:opening_balance",
        dec!(0),
        dec!(0),
    )
    .await?;

    // (2) NEW v1+ strategy_events row — operator-success-report source of
    // truth for R7's "kill-switch trips" count (Q8).  No money columns
    // — reconciler invariant unaffected.
    sqlx::query(
        "INSERT INTO strategy_events \
         (id, ts, kind, strategy_id, old_hash, new_hash, source_path, operator, error_code, error_summary, venue) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&strategy_row_id)
    .bind(&strategy_ts)
    .bind("KillSwitchTripped")
    .bind::<Option<&str>>(None) // strategy_id — feed-level event
    .bind::<Option<&str>>(None) // old_hash
    .bind::<Option<&str>>(None) // new_hash
    .bind("") // source_path
    .bind(operator)
    .bind(Some("kill_switch_tripped"))
    .bind(Some(reason))
    .bind::<Option<&str>>(None) // venue — global trip; per-venue trips wired in a follow-up (R8.3)
    .execute(&mut *db_txn)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    db_txn
        .commit()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
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
    // Microsecond-precision format ensures that two sequential writes within
    // the same wall-clock second still produce distinct, monotonically-ordered
    // `ts` values.  Without sub-second precision, two rapid writes collide on
    // the same `ts` string and the sort falls back to `rowid ASC`, which is
    // fragile under concurrent inserts (architect risk #4).
    let ts_fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    )
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let ts = if let Some(t) = write.ts {
        t.to_owned()
    } else {
        time::OffsetDateTime::now_utc()
            .format(&ts_fmt)
            .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?
    };

    sqlx::query(
        "INSERT INTO strategy_events \
         (id, ts, kind, strategy_id, old_hash, new_hash, source_path, operator, error_code, error_summary, venue) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
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
    .bind(write.venue)
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
    /// Optional venue label written to `strategy_events.venue` (T1402 /
    /// Q11).  `Some(v.to_string().as_str())` for feed-level events
    /// (`FeedReconnect`); `None` for venue-agnostic events. Migration
    /// `007_strategy_events_venue.sql` adds the NULLABLE column.
    pub venue: Option<&'a str>,
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
            venue: None,
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
            venue: None,
        },
    )
    .await
}

/// Write a `feed_reconnect` event to the `strategy_events` table (T805 — v1+ R7.1; T1402 — v1.5b Q11).
///
/// Emitted when a venue WS handler re-establishes a connection.  The
/// reports binary counts these per-window for the R7 system-health row.
/// Extends `strategy_events.kind` with `"FeedReconnect"`.
///
/// `error_summary` carries the symbol identifier (e.g. `"BTCUSDT"`).  No
/// `strategy_id` (the event is feed-level, not strategy-level).
///
/// `venue` is required (T1402 / Q11): the v1.5b multi-venue feature
/// elevates `Venue` to a typed first-class field at the audit boundary
/// rather than encoding it inline in `error_summary`. The writer stamps
/// `venue.to_string()` (`"binance"` / `"coinbase"` / `"kraken"`) into
/// the `strategy_events.venue` column added by migration
/// `007_strategy_events_venue.sql`. Reconciler invariant preserved:
/// `strategy_events` carries no money.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.feed_reconnect", skip(ledger), fields(symbol = %symbol, venue = %venue))]
pub async fn feed_reconnect(
    ledger: &Ledger,
    symbol: &str,
    venue: Venue,
    ts: Option<&str>,
) -> Result<(), LedgerError> {
    let venue_str = venue.to_string();
    strategy_event(
        ledger,
        &StrategyEventWrite {
            kind: "FeedReconnect",
            strategy_id: None,
            old_hash: None,
            new_hash: None,
            source_path: "",
            operator: "system",
            error_code: Some("feed_reconnect"),
            error_summary: Some(symbol),
            ts,
            venue: Some(venue_str.as_str()),
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
            venue: None,
        },
    )
    .await
}

// ── T806 — agent_uptime writers (operator success reports R7.1) ─────────────

/// Format an RFC-3339 timestamp to the same 6-digit fractional-second format
/// the `strategy_events` writer uses (`HF-3`/architect risk #4 determinism
/// gate).  Returns the now-utc value when `ts` is `None`.
fn uptime_ts_string(ts: Option<&str>) -> Result<String, LedgerError> {
    if let Some(t) = ts {
        return Ok(t.to_owned());
    }
    let fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    )
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    time::OffsetDateTime::now_utc()
        .format(&fmt)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))
}

/// Open a new uptime interval — call exactly once per agent boot.
///
/// Inserts `(boot_id, started_at = ts, last_heartbeat_at = ts,
/// stopped_at = NULL)` into `agent_uptime`.  The caller supplies a
/// freshly-generated UUID v4 as `boot_id`.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.open_uptime_interval", skip(ledger))]
pub async fn open_uptime_interval(
    ledger: &Ledger,
    boot_id: &str,
    ts: Option<&str>,
) -> Result<(), LedgerError> {
    let ts_str = uptime_ts_string(ts)?;
    sqlx::query(
        "INSERT INTO agent_uptime (boot_id, started_at, last_heartbeat_at, stopped_at) \
         VALUES (?, ?, ?, NULL)",
    )
    .bind(boot_id)
    .bind(&ts_str)
    .bind(&ts_str)
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}

/// Update the `last_heartbeat_at` column for the given `boot_id`.
///
/// Idempotent — a heartbeat for a non-existent `boot_id` is a no-op (the
/// caller's spawned heartbeat task warn-logs and continues).
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.heartbeat_uptime", skip(ledger))]
pub async fn heartbeat_uptime(
    ledger: &Ledger,
    boot_id: &str,
    ts: Option<&str>,
) -> Result<(), LedgerError> {
    let ts_str = uptime_ts_string(ts)?;
    sqlx::query("UPDATE agent_uptime SET last_heartbeat_at = ? WHERE boot_id = ?")
        .bind(&ts_str)
        .bind(boot_id)
        .execute(&ledger.pool)
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}

/// Close the uptime interval for the given `boot_id` (graceful shutdown).
///
/// Sets `stopped_at = ts` for the row matching `boot_id`.  If the row does
/// not exist (caller never called `open_uptime_interval`), the UPDATE is a
/// no-op — the caller warn-logs.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.close_uptime_interval", skip(ledger))]
pub async fn close_uptime_interval(
    ledger: &Ledger,
    boot_id: &str,
    ts: Option<&str>,
) -> Result<(), LedgerError> {
    let ts_str = uptime_ts_string(ts)?;
    sqlx::query("UPDATE agent_uptime SET stopped_at = ? WHERE boot_id = ?")
        .bind(&ts_str)
        .bind(boot_id)
        .execute(&ledger.pool)
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
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
