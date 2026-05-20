//! Read-only query surface (T07).
//!
//! No `sqlx` types in the public API. All amounts are returned as `Decimal` or
//! `Money<Usdt>` — never raw `String` or `sqlx` row types.
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use trading_core::{
    AccountId, AuditKindFilter, AuditKindLabel, EquitySeries, FillView, FundingObs, JournalEntry,
    JournalEntryView, JournalRow, JournalTransactionMetadata, LedgerError, Money, OpenPosition,
    OrphanTrainingRun, PairKey, PairMembership, Price, Quantity, Side, SignalView,
    StrategyEventKind, StrategyEventView, StrategyId, Symbol, Timestamp, TrainingEventRow,
    TrainingRunSummary, Usdt, Venue,
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

/// Realized P&L for a single closed-trade transaction (T1801, R2.2).
///
/// Sums `(credit_amount - debit_amount)` over `journal_entries` rows
/// where `account_id = 'income:realized_pnl' AND transaction_id = ?`.
/// Forward-compat: returns `Money::from_decimal(dec!(0))` for a
/// `trade_id` that has no `realized_pnl` entry (e.g. a buy-only
/// transaction).
///
/// Same TEXT-amount Decimal-only contract as
/// [`realized_pnl_since`].  Sibling reader; no new account, no new
/// migration.  Reflection-memory's `post_mortem_analyst::generate_card`
/// is the v1 caller.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error or parse failure.
pub async fn realized_pnl_for_trade(
    ledger: &Ledger,
    trade_id: &str,
) -> Result<Money<Usdt>, LedgerError> {
    let rows: Vec<(String, String)> = sqlx::query_as(
        "SELECT debit_amount, credit_amount \
         FROM journal_entries \
         WHERE account_id = 'income:realized_pnl' AND transaction_id = ?",
    )
    .bind(trade_id)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut total = dec!(0);
    for (dr, cr) in rows {
        let dr: Decimal = dr
            .parse()
            .map_err(|_| LedgerError::Database("realized_pnl_for_trade: parse debit".into()))?;
        let cr: Decimal = cr
            .parse()
            .map_err(|_| LedgerError::Database("realized_pnl_for_trade: parse credit".into()))?;
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

// ── LLM cache observability (T1910 / Q5d) ────────────────────────────────────

/// 24-hour aggregate cache-hit ratio across LLM cost events.
///
/// Reads `journal_transactions.metadata` JSON for every transaction
/// whose entries hit any `expense:llm:%` account at or after `since`,
/// summing the `tokens_in` and `tokens_cached_in` fields, then returns
/// `Σ tokens_cached_in / Σ tokens_in` as a [`Decimal`].
///
/// Design § Q5d: the operator-success-report's System Health table
/// reads this Decimal (no Prometheus dep at report-render time —
/// reports binary stays Prometheus-free per the existing invariant).
///
/// Token fields land on `journal_transactions.metadata` via the
/// extended `post_cost` signature in T1917 (M5). Until then — or for
/// any window with no LLM events — the result is
/// `Decimal::ZERO` (defensive default; mirrors the research-mode 0.00
/// ratio invariant). Forward-compat: a row whose metadata JSON omits
/// the token fields is treated as 0/0 and skipped.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error or timestamp
/// formatting failure. Malformed metadata JSON does NOT error — the
/// row is logged via `tracing::debug!` and skipped (read-only query
/// must not block on a stray bad row).
pub async fn cache_hit_ratio_since(
    ledger: &Ledger,
    since: Timestamp,
) -> Result<Decimal, LedgerError> {
    let ts_str = since
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    // One row per LLM transaction in the window. DISTINCT so a
    // multi-leg LLM transaction (Dr expense:llm:<tier> + Cr
    // liabilities:llm_accrued) doesn't double-count.
    let rows: Vec<(String,)> = sqlx::query_as(
        "SELECT DISTINCT t.metadata \
         FROM journal_transactions t \
         JOIN journal_entries e ON e.transaction_id = t.id \
         WHERE e.account_id LIKE 'expense:llm:%' AND t.ts >= ?",
    )
    .bind(&ts_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut sum_in: u128 = 0;
    let mut sum_cached: u128 = 0;
    for (meta_json,) in rows {
        let parsed: serde_json::Value = match serde_json::from_str(&meta_json) {
            Ok(v) => v,
            Err(e) => {
                tracing::debug!(
                    target: "audit.query.cache_hit_ratio",
                    error = %e,
                    "skipping row with malformed metadata"
                );
                continue;
            }
        };
        let tokens_in = parsed
            .get("tokens_in")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        let tokens_cached = parsed
            .get("tokens_cached_in")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0);
        sum_in = sum_in.saturating_add(u128::from(tokens_in));
        sum_cached = sum_cached.saturating_add(u128::from(tokens_cached));
    }

    if sum_in == 0 {
        return Ok(Decimal::ZERO);
    }
    // u128 → Decimal: `Decimal` impls `From<u64>` (infallible). Saturate at
    // u64::MAX for the (operator-unreachable) 18-quintillion-token case, then
    // convert via the infallible `u64::try_from` (succeeds because the min
    // already guarantees the value fits).
    let sum_in_u64 = u64::try_from(sum_in.min(u128::from(u64::MAX))).unwrap_or(u64::MAX);
    let sum_in_dec = Decimal::from(sum_in_u64);
    let sum_cached_u64 = u64::try_from(sum_cached.min(u128::from(u64::MAX))).unwrap_or(u64::MAX);
    let sum_cached_dec = Decimal::from(sum_cached_u64);
    Ok(sum_cached_dec / sum_in_dec)
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

/// Phase 2 addition (R12 / Q4). Return all fills for `(venue, symbol)`
/// inside the half-open interval `[since, until)`, newest-first.
///
/// Same description-prefixed-rows scan as [`recent_fills`]; narrower
/// predicate (venue + symbol + time-range vs `recent_fills`'s limit-only).
/// Read-only over committed audit data; does not alter any committed
/// report body. Additive — `recent_fills` unchanged.
///
/// **Phase 3 R13.4 — venue predicate.** Migration 008 adds the
/// `journal_transactions.venue` column with `'binance'` backfill on
/// existing rows; the writer at `journal::post_fill` now binds
/// `venue.to_string()` on insert. The Phase 2 venue gate
/// (`if venue != Venue::Binance { return Ok(Vec::new()) }`) is
/// removed; the SQL gains a `WHERE venue = ?` predicate.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn recent_fills_filtered(
    ledger: &Ledger,
    venue: Venue,
    symbol: Symbol,
    since: Timestamp,
    until: Timestamp,
) -> Result<Vec<FillView>, LedgerError> {
    let since_str = since
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let until_str = until
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let venue_str = venue.to_string();

    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT id, ts, description \
         FROM journal_transactions \
         WHERE (description LIKE 'buy %' OR description LIKE 'sell %') \
           AND ts >= ? AND ts < ? \
           AND venue = ? \
         ORDER BY ts DESC, rowid DESC",
    )
    .bind(&since_str)
    .bind(&until_str)
    .bind(&venue_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut fills = Vec::with_capacity(rows.len());
    for (txn_id, ts_str, desc) in rows {
        // Symbol filter — pre-parse, so we don't pay the description-parse
        // cost for rows we'll discard anyway.
        if extract_symbol_from_description(&desc) != symbol {
            continue;
        }
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
        transaction_id: SmolStr::new(txn_id),
    }))
}

// ── chart-buy-sell-emphasis v1.9 (T2015) — recent_signals reader ──
//
// Sibling of [`recent_fills_filtered`] in shape (same RFC-3339 binding,
// same `venue.to_string()` predicate, same `[since, until)` half-open
// window, same `ORDER BY ts DESC, rowid DESC` stability rule). The
// cockpit polls this on `SelectSymbol` and after `BarClose` for the
// active symbol — analogous to how `chart_markers` is populated from
// `recent_fills_filtered` (R9.1: no new bus channel).
//
// With `agent.toml [signal_log] enabled = false` (the v1.9 default),
// the writer is never called and this reader naturally returns
// `Ok(vec![])` against the empty `strategy_signals` table (V11c).

// Tuple shape for `recent_signals`: (id, ts, strategy_id, side,
// intended_qty_str, was_clamped, clamp_reason). Module-level to avoid
// `clippy::items_after_statements` inside the async fn body.
type SignalRow = (String, String, String, String, String, i64, Option<String>);

/// chart-buy-sell-emphasis v1.9 (T2015) — return every `SignalView` for
/// the supplied `(venue, symbol)` inside the half-open interval
/// `[since, until)`, newest-first.
///
/// Sibling of [`recent_fills_filtered`] — same RFC3339 binding,
/// `venue.to_string()` predicate, half-open time window, stable
/// `ORDER BY ts DESC, rowid DESC` (defends against ties on the same
/// microsecond ts).
///
/// `intended_qty_str` is parsed back to [`Quantity`] via the
/// `Decimal::parse` ↔ `Quantity::new` round-trip already used by
/// `parse_fill_view_from_description`. Bad rows surface as
/// [`LedgerError::Database`] (defensive — should be unreachable
/// because the writer always binds well-formed Decimal strings).
///
/// `clamp_reason = NULL` (column absent or unset) maps to `None`.
///
/// **V11 acceptance:**
/// - V11a — correct rows in correct order on a seeded ledger
///   (`recent_signals_returns_window_subset`).
/// - V11b — empty window returns `Ok(vec![])`
///   (`recent_signals_empty_window_returns_ok_empty`).
/// - V11c — gate-off ledger with no rows in `strategy_signals` returns
///   `Ok(vec![])` (`recent_signals_gate_off_ledger_returns_ok_empty`).
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or Decimal parse error.
pub async fn recent_signals(
    ledger: &Ledger,
    venue: Venue,
    symbol: Symbol,
    since: Timestamp,
    until: Timestamp,
) -> Result<Vec<SignalView>, LedgerError> {
    let since_str = since
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let until_str = until
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let venue_str = venue.to_string();
    let symbol_str = symbol.0.as_str();

    // Tuple shape: (id, ts, strategy_id, side, intended_qty_str,
    // was_clamped, clamp_reason). Symbol and venue are filtered in the
    // WHERE; we don't need them back in the projection.
    let rows: Vec<SignalRow> = sqlx::query_as(
        "SELECT id, ts, strategy_id, side, intended_qty_str, was_clamped, clamp_reason \
         FROM strategy_signals \
         WHERE ts >= ? AND ts < ? \
           AND venue = ? \
           AND symbol = ? \
         ORDER BY ts DESC, rowid DESC",
    )
    .bind(&since_str)
    .bind(&until_str)
    .bind(&venue_str)
    .bind(symbol_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut out: Vec<SignalView> = Vec::with_capacity(rows.len());
    for (id, ts_str, strategy_id, side_str, qty_str, was_clamped_i, clamp_reason) in rows {
        let side = match side_str.as_str() {
            "buy" => Side::Buy,
            "sell" => Side::Sell,
            other => {
                // Defensive — writer never produces other values today,
                // but future SignalKind variants may extend the column.
                return Err(LedgerError::Database(format!(
                    "recent_signals: unknown side '{other}'"
                )));
            }
        };
        let qty_d: Decimal = qty_str
            .parse()
            .map_err(|_| LedgerError::Database(format!("recent_signals: bad qty '{qty_str}'")))?;
        let intended_qty =
            Quantity::new(qty_d).map_err(|e| LedgerError::Database(e.to_string()))?;
        let signal_ts = OffsetDateTime::parse(&ts_str, &Rfc3339)
            .map(Timestamp::new)
            .map_err(|e| LedgerError::Database(e.to_string()))?;
        let was_clamped = was_clamped_i != 0;

        out.push(SignalView {
            signal_id: SmolStr::new(&id),
            symbol: symbol.clone(),
            side,
            intended_qty,
            signal_ts,
            strategy_id: StrategyId::new(strategy_id),
            was_clamped,
            clamp_reason: clamp_reason.map(SmolStr::new),
        });
    }
    Ok(out)
}

// ── Phase 3 R12 / Q7 — recent_journal_filtered (sibling of recent_fills_filtered) ──

/// Phase 3 addition (R12 / Q7). Return the page of journal rows
/// matching the filter, newest-first. Read-only over committed audit
/// data; additive sibling of [`recent_fills_filtered`].
///
/// `venues.is_empty()` ↔ all venues; `symbol.is_none()` ↔ all symbols;
/// `kind == AuditKindFilter::All` ↔ all kinds. The half-open window
/// `[since, until)` matches the `recent_fills_filtered` shape. Returns
/// `(rows, total_count)` so the screen header can render
/// "Showing N–M of T" without a separate `COUNT(*)` round-trip.
///
/// **Determinism / money math.** No `f64`. Description-amount parsing
/// reuses the existing `Price` / `Quantity` newtypes for any computed
/// amount fields. Empty result returns `Ok((vec![], 0))`; never `Err`
/// for "no rows".
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
#[allow(clippy::too_many_arguments)]
pub async fn recent_journal_filtered(
    ledger: &Ledger,
    venues: &[Venue],
    symbol: Option<&Symbol>,
    kind: AuditKindFilter,
    since: Timestamp,
    until: Timestamp,
    page_offset: u32,
    page_size: u32,
) -> Result<(Vec<JournalRow>, u64), LedgerError> {
    let since_str = since
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let until_str = until
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    // Build the venue predicate. Empty venue list ↔ all venues (no
    // additional `IN` constraint). Otherwise inline the snake_case
    // venue strings (closed enum — bounded set, no escaping).
    let venue_clause = if venues.is_empty() {
        String::new()
    } else {
        let placeholders = venues
            .iter()
            .map(|v| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(",");
        format!(" AND venue IN ({placeholders})")
    };

    // Kind discriminator — translates to a description-prefix scan +
    // an EXISTS sub-query for strategy-event rows (Phase 3 Design).
    let kind_clause = match kind {
        AuditKindFilter::All => String::new(),
        AuditKindFilter::Fill => {
            " AND (description LIKE 'buy %' OR description LIKE 'sell %')".to_string()
        }
        AuditKindFilter::StrategyEvent => " AND EXISTS (SELECT 1 FROM strategy_events se \
              WHERE se.transaction_id = journal_transactions.id)"
            .to_string(),
        AuditKindFilter::Reconciliation => " AND description LIKE 'reconcile %'".to_string(),
    };

    let where_predicate = format!("WHERE ts >= ? AND ts < ?{venue_clause}{kind_clause}");

    // Total count under the same WHERE — one extra round-trip; well
    // under the user-perceptible threshold for a 250-row page.
    let count_sql = format!("SELECT COUNT(*) FROM journal_transactions {where_predicate}");
    let count_row: (i64,) = sqlx::query_as(&count_sql)
        .bind(&since_str)
        .bind(&until_str)
        .fetch_one(&ledger.pool)
        .await
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let total_count = u64::try_from(count_row.0).unwrap_or(0);

    // Page query — ORDER BY ts DESC, rowid DESC (Phase 2 R12.5
    // determinism); LIMIT ? OFFSET ?.
    let select_sql = format!(
        "SELECT id, ts, description, strategy_id, venue \
         FROM journal_transactions {where_predicate} \
         ORDER BY ts DESC, rowid DESC LIMIT ? OFFSET ?"
    );
    // Tuple shape mirrors the 5-column projection above; clippy
    // `type_complexity` warns on five-element tuples but extracting
    // a `type` alias for a one-call-site shape adds noise without
    // semantic value here.
    #[allow(clippy::type_complexity)]
    let rows: Vec<(String, String, String, Option<String>, Option<String>)> =
        sqlx::query_as(&select_sql)
            .bind(&since_str)
            .bind(&until_str)
            .bind(i64::from(page_size))
            .bind(i64::from(page_offset))
            .fetch_all(&ledger.pool)
            .await
            .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut out: Vec<JournalRow> = Vec::with_capacity(rows.len());
    for (txn_id, ts_str, desc, strategy_id, venue_str) in rows {
        let row_kind = classify_kind(&desc);
        let row_symbol = match row_kind {
            AuditKindLabel::Fill => Some(extract_symbol_from_description(&desc)),
            // Non-fill rows do not encode symbol in description; the
            // operator filter, when set, narrows to fill-only by virtue
            // of `extract_symbol_from_description` always returning
            // `Symbol::new("UNKNOWN")` for those rows. Symbol filtering
            // is applied below as a post-filter on the parsed value.
            _ => None,
        };
        // Symbol post-filter (skip rows that don't match the operator's
        // selected symbol — only relevant for fills, since non-fill
        // rows surface with `symbol = None`).
        if let Some(target) = symbol {
            match row_kind {
                AuditKindLabel::Fill => {
                    if row_symbol.as_ref() != Some(target) {
                        continue;
                    }
                }
                _ => {
                    // Filtering by symbol on non-fill rows yields no
                    // matches by construction — drop the row.
                    continue;
                }
            }
        }
        let venue = venue_str
            .as_deref()
            .and_then(|s| s.parse::<Venue>().ok())
            // Fallback: pre-008 NULL rows backfilled to 'binance' by
            // the migration. Defensive default keeps the row visible.
            .unwrap_or(Venue::Binance);
        let ts = OffsetDateTime::parse(&ts_str, &Rfc3339)
            .map(Timestamp::new)
            .map_err(|e| LedgerError::Database(e.to_string()))?;
        out.push(JournalRow {
            tx_id: SmolStr::new(txn_id),
            ts,
            venue,
            symbol: row_symbol,
            kind: row_kind,
            description: SmolStr::new(desc),
            strategy_id: strategy_id.map(StrategyId::new),
        });
    }

    Ok((out, total_count))
}

/// Phase 3 R12 — classify a `journal_transactions.description` into
/// the audit-screen `AuditKindLabel`. Mirrors the SQL discriminator
/// shape in [`recent_journal_filtered`].
fn classify_kind(desc: &str) -> AuditKindLabel {
    if desc.starts_with("buy ") || desc.starts_with("sell ") {
        AuditKindLabel::Fill
    } else if desc.starts_with("reconcile ") {
        AuditKindLabel::Reconciliation
    } else {
        // Default to StrategyEvent for any non-fill, non-reconcile
        // description (e.g. registry events, kill-switch memos).
        AuditKindLabel::StrategyEvent
    }
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

// ── Journal entries for a single transaction (T1202) ──────────────────────────

/// Return every `journal_entries` row attached to `tx_id`, un-collapsed
/// (debit + credit kept as separate columns) and joined with `accounts` so
/// each row carries its display currency ticker.
///
/// Used by the tape-row → audit-modal feature
/// (`spec/features/tape-row-audit-modal.md` Q2 / Q8 V11) to render the
/// 4-column `Account | Debit | Credit | Currency` table without losing the
/// "exact zero" debit or credit cells that signed-amount rendering would
/// erase.
///
/// ## Determinism
///
/// Rows are ordered by `journal_entries.id ASC` — the column is a UUID v4
/// string, lex-sorted; stable across runs. No `f64` math.
///
/// ## Empty result
///
/// When `tx_id` does not match any row, the function returns `Ok(vec![])`
/// — never `Err`. (Unknown / typo'd transaction ids are a normal UI signal,
/// not a data corruption.)
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or `Decimal` / timestamp parse
/// error.
pub async fn journal_entries_for_transaction(
    ledger: &Ledger,
    tx_id: &str,
) -> Result<Vec<JournalEntry>, LedgerError> {
    // Join with `accounts` so each row carries its display currency ticker
    // (USDT / BTC / …) without baking chart-of-accounts naming into the
    // reader. ORDER BY journal_entries.id ASC for deterministic output
    // (UUID v4 strings sort lex-stably).
    let rows: Vec<(String, String, String, String, String, String)> = sqlx::query_as(
        "SELECT je.account_id, je.debit_amount, je.credit_amount, \
                a.currency, je.ts, je.memo \
         FROM journal_entries je \
         JOIN accounts a ON a.id = je.account_id \
         WHERE je.transaction_id = ? \
         ORDER BY je.id ASC",
    )
    .bind(tx_id)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let mut entries = Vec::with_capacity(rows.len());
    for (account, dr_str, cr_str, currency, ts_str, memo) in rows {
        let dr: Decimal = dr_str.parse().map_err(|_| {
            LedgerError::Database(format!(
                "journal_entries_for_transaction: parse debit `{dr_str}`"
            ))
        })?;
        let cr: Decimal = cr_str.parse().map_err(|_| {
            LedgerError::Database(format!(
                "journal_entries_for_transaction: parse credit `{cr_str}`"
            ))
        })?;
        let ts = OffsetDateTime::parse(&ts_str, &Rfc3339)
            .map(Timestamp::new)
            .map_err(|e| {
                LedgerError::Database(format!("journal_entries_for_transaction: parse ts: {e}"))
            })?;
        entries.push(JournalEntry {
            account: AccountId::new(account),
            debit: Money::<Usdt>::from_decimal(dr),
            credit: Money::<Usdt>::from_decimal(cr),
            currency: SmolStr::new(currency),
            ts,
            memo: SmolStr::new(memo),
        });
    }
    Ok(entries)
}

// ── Journal-transaction header metadata (T1302) ───────────────────────────────

/// Header-only read for the `journal_transactions` row identified by `tx_id`.
///
/// Returns the four-field header (`transaction_id`, `ts`, `description`,
/// `strategy_id`) the live cockpit's tape-row → audit-modal feature uses to
/// populate `JournalTransactionView` next to the entries returned by
/// [`journal_entries_for_transaction`]. The two readers compose at the
/// `cockpit_live` `Task::perform` site
/// (`spec/features/journal-transactions-metadata.md` Design § Q2 / Q4).
///
/// ## Determinism
///
/// Single-row `SELECT ... WHERE id = ?` against the `journal_transactions`
/// PRIMARY KEY column; deterministic by construction. No `f64` math.
///
/// ## Empty result
///
/// Returns `Ok(None)` when no row matches `tx_id` (stale row, fixture-mode
/// click, unknown UUID); never `Err` for missing rows. Mirrors the
/// empty-result contract of [`journal_entries_for_transaction`] (which
/// returns `Ok(vec![])` for the same condition).
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or timestamp parse error.
pub async fn journal_transaction_metadata(
    ledger: &Ledger,
    tx_id: &str,
) -> Result<Option<JournalTransactionMetadata>, LedgerError> {
    let row: Option<(String, String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, ts, description, strategy_id \
         FROM journal_transactions \
         WHERE id = ?",
    )
    .bind(tx_id)
    .fetch_optional(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let Some((id, ts_str, description, strategy_id)) = row else {
        return Ok(None);
    };

    let ts = OffsetDateTime::parse(&ts_str, &Rfc3339)
        .map(Timestamp::new)
        .map_err(|e| {
            LedgerError::Database(format!("journal_transaction_metadata: parse ts: {e}"))
        })?;

    Ok(Some(JournalTransactionMetadata {
        transaction_id: SmolStr::new(id),
        ts,
        description: SmolStr::new(description),
        strategy_id: strategy_id.map(StrategyId::new),
    }))
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
        // Phase 5 (T1902) — operator-write variants. Only the PascalCase
        // form is written by the new audit writers; the snake_case
        // alternative is accepted for forward-compat with future migration
        // (mirrors the pattern used by the v1+ variants above).
        "StrategyPaused" | "strategy_paused" => StrategyEventKind::StrategyPaused,
        "RiskVetoOverridden" | "risk_veto_overridden" => StrategyEventKind::RiskVetoOverridden,
        other => {
            return Err(LedgerError::Database(format!(
                "unknown strategy event kind: {other}"
            )));
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

// ── Phase 4 R12 / Q7 — equity_curve_for_strategy (sibling of pnl_by_strategy) ──

/// Phase 4 addition (R12 / Q7). Walk the `journal_entries` rows on the
/// `income:realized_pnl` account joined to their parent
/// `journal_transactions` row's `strategy_id`, emitting an
/// [`EquitySeries`] whose points are the running-sum of realized P&L
/// samples in the half-open window `[since, until_or_now)`.
///
/// Read-only over committed audit data; additive sibling of
/// [`pnl_by_strategy`]. The inception-equity baseline is read from the
/// same journal: the first sample carries the running cash balance at
/// `since` (computed via the [`cash_balance`] query at the same
/// instant), and each subsequent sample increments by the row's
/// `(credit - debit)` delta.
///
/// `until = None` ↔ "to now" (the function reads
/// [`Timestamp::now()`] once at the call boundary). The cockpit
/// consumer (R13.4) uses `until: None` so the call-site doesn't read
/// the clock.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error. Returns
/// `Err(LedgerError::EmptyWindow)` when the window contains zero rows
/// (so the cockpit consumer can render the R13.8 empty state without
/// inspecting an `Ok(EquitySeries)` for `points.is_empty()` — keeps
/// the `from_points` `Empty` invariant load-bearing).
pub async fn equity_curve_for_strategy(
    ledger: &Ledger,
    strategy_id: StrategyId,
    since: Timestamp,
    until: Option<Timestamp>,
) -> Result<EquitySeries, LedgerError> {
    let until = until.unwrap_or_else(Timestamp::now);

    let since_str = since
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;
    let until_str = until
        .inner()
        .format(&Rfc3339)
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    let rows: Vec<(String, String, String)> = sqlx::query_as(
        "SELECT je.ts, je.debit_amount, je.credit_amount \
         FROM journal_entries je \
         JOIN journal_transactions jt ON je.transaction_id = jt.id \
         WHERE je.account_id = 'income:realized_pnl' \
           AND jt.strategy_id = ? \
           AND je.ts >= ? \
           AND je.ts <  ? \
         ORDER BY je.ts ASC, je.id ASC",
    )
    .bind(strategy_id.0.as_str())
    .bind(&since_str)
    .bind(&until_str)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    if rows.is_empty() {
        return Err(LedgerError::EmptyWindow);
    }

    let baseline = cash_balance(ledger).await?;
    let mut running = baseline.amount();
    let mut points: Vec<(Timestamp, Money<Usdt>)> = Vec::with_capacity(rows.len());
    for (ts_str, dr_str, cr_str) in rows {
        let ts = OffsetDateTime::parse(&ts_str, &Rfc3339)
            .map(Timestamp::new)
            .map_err(|e| LedgerError::Database(e.to_string()))?;
        let dr: Decimal = dr_str
            .parse()
            .map_err(|_| LedgerError::Database("equity_curve_for_strategy: parse debit".into()))?;
        let cr: Decimal = cr_str
            .parse()
            .map_err(|_| LedgerError::Database("equity_curve_for_strategy: parse credit".into()))?;
        running += cr - dr;
        points.push((ts, Money::<Usdt>::from_decimal(running)));
    }

    EquitySeries::from_points(points)
        .map_err(|e| LedgerError::Database(format!("equity_curve_for_strategy: {e}")))
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
/// existing private `extract_symbol_from_description` helper. Per
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
        if let Some(ref account_id) = position_account_id
            && account_id.starts_with("assets:position:")
        {
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

// ── cockpit-training-control T-D-N9 — training_events readers ────────────────

/// Internal row type for `recent_training_events` and `latest_training_run`.
///
/// All fields map directly to the `training_events` columns. `train_loss` /
/// `val_loss` are stored as TEXT in the DB; we parse them back to `f32` here.
#[allow(clippy::type_complexity)]
type TrainingEventSqlRow = (
    String,         // id
    String,         // ts
    String,         // run_id
    String,         // kind
    Option<i64>,    // epoch
    Option<i64>,    // total_epochs
    Option<String>, // train_loss TEXT
    Option<String>, // val_loss TEXT
    Option<i64>,    // wall_clock_ms
    Option<String>, // model_revision
    String,         // scenario
    i64,            // seed
    Option<i64>,    // pid
    Option<String>, // error_message
);

fn row_to_training_event(r: TrainingEventSqlRow) -> Result<TrainingEventRow, LedgerError> {
    let (
        id,
        ts,
        run_id,
        kind,
        epoch,
        total_epochs,
        train_loss_s,
        val_loss_s,
        wall_clock_ms,
        model_revision,
        scenario,
        seed,
        pid,
        error_message,
    ) = r;

    let train_loss = train_loss_s
        .as_deref()
        .map(|s| {
            s.parse::<f32>()
                .map_err(|_| LedgerError::Database(format!("bad train_loss '{s}'")))
        })
        .transpose()?;
    let val_loss = val_loss_s
        .as_deref()
        .map(|s| {
            s.parse::<f32>()
                .map_err(|_| LedgerError::Database(format!("bad val_loss '{s}'")))
        })
        .transpose()?;

    Ok(TrainingEventRow {
        id: SmolStr::new(&id),
        ts: SmolStr::new(&ts),
        run_id: SmolStr::new(&run_id),
        kind: SmolStr::new(&kind),
        epoch,
        total_epochs,
        train_loss,
        val_loss,
        wall_clock_ms,
        model_revision: model_revision.map(|s| SmolStr::new(&s)),
        scenario: SmolStr::new(&scenario),
        seed,
        pid,
        error_message: error_message.map(|s| SmolStr::new(&s)),
    })
}

/// Return all `training_events` rows in the half-open window `[since, until)`,
/// newest-first (R4.5, ADR-0034 § D2).
///
/// `since` and `until` are RFC3339 strings. The half-open convention matches
/// [`recent_signals`] and [`recent_fills_filtered`].
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn recent_training_events(
    ledger: &Ledger,
    since: &str,
    until: &str,
) -> Result<Vec<TrainingEventRow>, LedgerError> {
    let rows: Vec<TrainingEventSqlRow> = sqlx::query_as(
        "SELECT id, ts, run_id, kind, epoch, total_epochs, train_loss, val_loss, \
                wall_clock_ms, model_revision, scenario, seed, pid, error_message \
         FROM training_events \
         WHERE ts >= ? AND ts < ? \
         ORDER BY ts DESC, rowid DESC",
    )
    .bind(since)
    .bind(until)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    rows.into_iter().map(row_to_training_event).collect()
}

/// Return a `TrainingRunSummary` for the most recent training run, or `None`
/// if the `training_events` table is empty (R4.5 / panel status strip R3.5).
///
/// "Most recent" is determined by the latest `kind='start'` row's `ts`.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error.
pub async fn latest_training_run(
    ledger: &Ledger,
) -> Result<Option<TrainingRunSummary>, LedgerError> {
    // Find the most recent run_id (from the latest start row).
    let start_row: Option<(String, String, String, i64, Option<i64>)> = sqlx::query_as(
        "SELECT run_id, ts, scenario, seed, pid \
         FROM training_events \
         WHERE kind = 'start' \
         ORDER BY ts DESC, rowid DESC \
         LIMIT 1",
    )
    .fetch_optional(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let Some((run_id, started_at, scenario, seed, pid)) = start_row else {
        return Ok(None);
    };

    // Fetch the latest epoch row for this run.
    let epoch_row: Option<(i64, i64, Option<String>, Option<String>)> = sqlx::query_as(
        "SELECT epoch, total_epochs, train_loss, val_loss \
         FROM training_events \
         WHERE run_id = ? AND kind = 'epoch' \
         ORDER BY epoch DESC LIMIT 1",
    )
    .bind(&run_id)
    .fetch_optional(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let (latest_epoch, total_epochs, latest_train_loss_s, latest_val_loss_s) =
        epoch_row.unwrap_or((0, 0, None, None));

    let latest_train_loss = latest_train_loss_s
        .as_deref()
        .map(|s| {
            s.parse::<f32>()
                .map_err(|_| LedgerError::Database(format!("bad train_loss '{s}'")))
        })
        .transpose()?;
    let latest_val_loss = latest_val_loss_s
        .as_deref()
        .map(|s| {
            s.parse::<f32>()
                .map_err(|_| LedgerError::Database(format!("bad val_loss '{s}'")))
        })
        .transpose()?;

    // Check for finish row.
    let finish_row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT model_revision FROM training_events WHERE run_id = ? AND kind = 'finish' LIMIT 1",
    )
    .bind(&run_id)
    .fetch_optional(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    // Check for failed row.
    let failed_row: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT error_message FROM training_events WHERE run_id = ? AND kind = 'failed' LIMIT 1",
    )
    .bind(&run_id)
    .fetch_optional(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    let (model_revision, status, error_message) = match (finish_row, failed_row) {
        (Some((rev,)), _) => (rev.map(SmolStr::new), SmolStr::new("done"), None),
        (_, Some((err,))) => (None, SmolStr::new("failed"), err.map(SmolStr::new)),
        _ => (None, SmolStr::new("running"), None),
    };

    Ok(Some(TrainingRunSummary {
        run_id: SmolStr::new(&run_id),
        scenario: SmolStr::new(&scenario),
        seed,
        started_at: SmolStr::new(&started_at),
        latest_epoch,
        total_epochs,
        latest_train_loss,
        latest_val_loss,
        model_revision,
        status,
        error_message,
        pid,
    }))
}

/// Return all training runs that have a `kind='start'` row but no
/// `kind='finish'` or `kind='failed'` row within the given `fresh_window_secs`
/// seconds (ADR-0034 § D7 orphan-detect).
///
/// A run is considered "orphaned" when:
/// 1. Its `started_at` is more than `fresh_window_secs` seconds in the past, AND
/// 2. There is no `finish` or `failed` row for the same `run_id`.
///
/// Callers can then do `libc::kill(pid, 0)` on the returned `pid` to determine
/// whether the process is still alive (false-positive risk: PID reuse within
/// the 24h orphan window — bounded per ADR-0034 § D7).
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error.
pub async fn orphan_training_runs(
    ledger: &Ledger,
    fresh_window_secs: i64,
) -> Result<Vec<OrphanTrainingRun>, LedgerError> {
    // SQLite datetime arithmetic: use `strftime` + `unixepoch`.
    // The cutoff is `now - fresh_window_secs` seconds.
    // We compare against the `ts` column which is RFC3339 TEXT —
    // lexicographic ordering works for ISO8601 strings at the same
    // timezone (all our timestamps end in `Z`).
    let rows: Vec<(String, String, String, Option<i64>)> = sqlx::query_as(
        "SELECT run_id, ts, scenario, pid \
         FROM training_events \
         WHERE kind = 'start' \
           AND ts < datetime('now', printf('-%d seconds', ?)) \
           AND run_id NOT IN ( \
               SELECT run_id FROM training_events WHERE kind IN ('finish', 'failed') \
           ) \
         ORDER BY ts DESC",
    )
    .bind(fresh_window_secs)
    .fetch_all(&ledger.pool)
    .await
    .map_err(|e| LedgerError::Database(e.to_string()))?;

    Ok(rows
        .into_iter()
        .map(|(run_id, started_at, scenario, pid)| OrphanTrainingRun {
            run_id: SmolStr::new(&run_id),
            scenario: SmolStr::new(&scenario),
            started_at: SmolStr::new(&started_at),
            pid,
        })
        .collect())
}

// ── Phase D (T-D-N25) — Trail reconstruction ──────────────────────────────────

/// One row from `journal_transactions` as seen by the trail reconstructor.
///
/// Only the columns relevant to the trail chain are captured; other columns
/// live in [`JournalRow`] which is the general-purpose read model.
#[derive(Debug, Clone, Default)]
pub struct TrailFillRow {
    /// `journal_transactions.id`
    pub id: String,
    /// `journal_transactions.fill_id` (mig 011 — NULL on pre-mig rows).
    pub fill_id: Option<String>,
    /// `journal_transactions.signal_id` (mig 011 — NULL on pre-mig rows).
    pub signal_id: Option<String>,
    /// `journal_transactions.ts` formatted as RFC3339.
    pub ts: String,
    /// `journal_transactions.description`
    pub description: String,
}

/// One row from `strategy_signals` as seen by the trail reconstructor.
#[derive(Debug, Clone)]
pub struct TrailSignalRow {
    /// `strategy_signals.id`
    pub id: String,
    /// `strategy_signals.side` ("Buy" or "Sell")
    pub side: String,
    /// `strategy_signals.intended_qty_str`
    pub intended_qty: String,
    /// `strategy_signals.intended_price_str`
    pub intended_price: Option<String>,
    /// `strategy_signals.was_clamped`
    pub was_clamped: bool,
    /// `strategy_signals.clamp_reason`
    pub clamp_reason: Option<String>,
    /// `strategy_signals.forecast_correlation_id` (mig 011).
    pub forecast_correlation_id: Option<String>,
    /// `strategy_signals.ts`
    pub ts: String,
}

/// One row from `forecast_events` as seen by the trail reconstructor.
#[derive(Debug, Clone)]
pub struct TrailForecastRow {
    /// `forecast_events.correlation_id` (primary key)
    pub correlation_id: String,
    /// `forecast_events.ts`
    pub ts: String,
    /// `forecast_events.direction` ("up", "down", "flat")
    pub direction: String,
    /// `forecast_events.confidence`
    pub confidence: String,
    /// `forecast_events.model_revision`
    pub model_revision: String,
    /// `forecast_events.cache_hit`
    pub cache_hit: bool,
}

/// Four-stage trail reconstruction result.
///
/// All stages are `Option` — absent when the relevant row does not exist (R3.4).
/// `debate` is always `None` at v0.1.0 (R1.5 — `debate_events` not yet wired).
#[derive(Debug, Clone, Default)]
pub struct TrailReconstruction {
    /// Fill stage — the `journal_transactions` row for this fill's audit entry.
    pub fill: Option<TrailFillRow>,
    /// Signal stage — the `strategy_signals` row linked via `signal_id`.
    pub signal: Option<TrailSignalRow>,
    /// Forecast stage — the `forecast_events` row linked via `forecast_correlation_id`.
    pub forecast: Option<TrailForecastRow>,
    /// LLM debate — always `None` at v0.1.0.
    pub debate: Option<()>,
}

/// Phase D (T-D-N25) — Reconstruct the four-stage trail for a given
/// fill audit id.
///
/// Performs 4 indexed point-lookups:
/// 1. `journal_transactions WHERE id = fill_audit_id` → fill row + `signal_id`.
/// 2. `strategy_signals WHERE id = signal_id` → signal row + `forecast_correlation_id`.
/// 3. `forecast_events WHERE correlation_id = forecast_correlation_id` → forecast row.
/// 4. `debate_events WHERE ...` → always `None` at v0.1.0 (R1.5).
///
/// All four indexes are from mig 011. Each lookup is O(log n) — H5 invariant
/// (p99 < 50 ms at ≥10⁵ rows). Pre-mig rows return `None` for stages 2–4.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL error.
pub async fn trail_for_fill_id(
    ledger: &Ledger,
    fill_audit_id: &str,
) -> Result<TrailReconstruction, LedgerError> {
    // ── Stage 1: journal_transactions ────────────────────────────────────────
    #[allow(clippy::type_complexity)]
    let fill_row: Option<(String, Option<String>, Option<String>, String, String)> =
        sqlx::query_as(
            "SELECT id, fill_id, signal_id, ts, description \
             FROM journal_transactions \
             WHERE id = ?",
        )
        .bind(fill_audit_id)
        .fetch_optional(&ledger.pool)
        .await
        .map_err(|e| LedgerError::Database(e.to_string()))?;

    let Some((id, fill_id, signal_id, ts, description)) = fill_row else {
        return Ok(TrailReconstruction::default());
    };

    let fill = TrailFillRow {
        id,
        fill_id,
        signal_id: signal_id.clone(),
        ts,
        description,
    };

    // ── Stage 2: strategy_signals ────────────────────────────────────────────
    let signal = if let Some(ref sid) = signal_id {
        #[allow(clippy::type_complexity)]
        let row: Option<(
            String,
            String,
            String,
            Option<String>,
            bool,
            Option<String>,
            Option<String>,
            String,
        )> = sqlx::query_as(
            "SELECT id, side, intended_qty_str, intended_price_str, \
                    was_clamped, clamp_reason, forecast_correlation_id, ts \
             FROM strategy_signals \
             WHERE id = ?",
        )
        .bind(sid)
        .fetch_optional(&ledger.pool)
        .await
        .map_err(|e| LedgerError::Database(e.to_string()))?;

        row.map(
            |(id, side, qty, price, clamped, clamp_reason, fcast_id, ts)| TrailSignalRow {
                id,
                side,
                intended_qty: qty,
                intended_price: price,
                was_clamped: clamped,
                clamp_reason,
                forecast_correlation_id: fcast_id,
                ts,
            },
        )
    } else {
        None
    };

    // ── Stage 3: forecast_events ─────────────────────────────────────────────
    let forecast_correlation_id = signal
        .as_ref()
        .and_then(|s| s.forecast_correlation_id.clone());
    let forecast = if let Some(ref fid) = forecast_correlation_id {
        let row: Option<(String, String, String, String, String, bool)> = sqlx::query_as(
            "SELECT correlation_id, ts, direction, confidence, model_revision, cache_hit \
             FROM forecast_events \
             WHERE correlation_id = ?",
        )
        .bind(fid)
        .fetch_optional(&ledger.pool)
        .await
        .map_err(|e| LedgerError::Database(e.to_string()))?;

        row.map(
            |(corr_id, ts, direction, confidence, model_rev, cache_hit)| TrailForecastRow {
                correlation_id: corr_id,
                ts,
                direction,
                confidence,
                model_revision: model_rev,
                cache_hit,
            },
        )
    } else {
        None
    };

    // ── Stage 4: LLM debate — always None at v0.1.0 (R1.5) ─────────────────
    let debate = None;

    Ok(TrailReconstruction {
        fill: Some(fill),
        signal,
        forecast,
        debate,
    })
}

// ── T1606 — recent_fills_filtered unit tests (Phase 2 R12 / Q10) ─────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use crate::journal::post_fill;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;
    use trading_core::{FeeTier, Fill, FillId, Liquidity, OrderId, Side};
    // `Venue` is already in scope via `use super::*;` (re-exports the
    // outer `query.rs` imports including `trading_core::Venue`).

    /// Build an in-memory ledger with the chart-of-accounts pre-seeded so
    /// `post_fill` lands its FK targets cleanly.
    async fn open_seeded_ledger() -> Ledger {
        let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
        bootstrap::chart_of_accounts(&ledger)
            .await
            .expect("bootstrap chart of accounts");
        ledger
    }

    fn ts_secs(secs: i64) -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs))
    }

    fn make_fill(symbol: &str, side: Side, secs: i64) -> Fill {
        Fill {
            id: FillId::new(),
            order_id: OrderId::new(),
            symbol: Symbol::new(symbol),
            side,
            qty: Quantity::new(dec!(0.1)).expect("qty"),
            price: Price::new(dec!(40_000)).expect("price"),
            fee: Money::from_decimal(dec!(0.5)),
            fee_tier: FeeTier::Taker,
            venue_ts: ts_secs(secs),
            local_ts: ts_secs(secs),
            liquidity: Liquidity::Taker,
            transaction_id: None,
        }
    }

    /// T1606 — seed 6 fills across two `(venue, symbol)` pairs (3 BTCUSDT,
    /// 3 ETHUSDT; two of each inside the window, one outside). Assert
    /// `recent_fills_filtered(&ledger, Binance, BTCUSDT, since, until)`
    /// returns exactly the two BTCUSDT fills inside `[since, until)` in
    /// newest-first order.
    #[tokio::test]
    async fn recent_fills_filtered_returns_window_subset() {
        let ledger = open_seeded_ledger().await;
        for secs in [10, 100, 200] {
            post_fill(
                &ledger,
                &make_fill("BTCUSDT", Side::Buy, secs),
                Venue::Binance,
                None,
            )
            .await
            .expect("post BTCUSDT fill");
        }
        for secs in [120, 180, 900] {
            post_fill(
                &ledger,
                &make_fill("ETHUSDT", Side::Sell, secs),
                Venue::Binance,
                None,
            )
            .await
            .expect("post ETHUSDT fill");
        }
        let since = ts_secs(50);
        let until = ts_secs(500);
        let fills = recent_fills_filtered(
            &ledger,
            Venue::Binance,
            Symbol::new("BTCUSDT"),
            since,
            until,
        )
        .await
        .expect("query ok");
        assert_eq!(
            fills.len(),
            2,
            "expected two BTCUSDT fills in window, got {}",
            fills.len()
        );
        // Newest-first — ts=200 before ts=100.
        assert_eq!(fills[0].venue_ts, ts_secs(200));
        assert_eq!(fills[1].venue_ts, ts_secs(100));
        for f in &fills {
            assert_eq!(f.symbol, Symbol::new("BTCUSDT"));
        }
    }

    /// T1606 — far-future window returns `Ok(vec![])` (never `Err`).
    #[tokio::test]
    async fn recent_fills_filtered_empty_window_returns_ok_empty() {
        let ledger = open_seeded_ledger().await;
        post_fill(
            &ledger,
            &make_fill("BTCUSDT", Side::Buy, 100),
            Venue::Binance,
            None,
        )
        .await
        .expect("seed fill");
        let since = ts_secs(10_000_000);
        let until = ts_secs(20_000_000);
        let fills = recent_fills_filtered(
            &ledger,
            Venue::Binance,
            Symbol::new("BTCUSDT"),
            since,
            until,
        )
        .await
        .expect("query ok");
        assert!(fills.is_empty(), "far-future window must return empty vec");
    }

    /// Phase 3 R13.4 — multi-venue case. After migration `008`,
    /// `recent_fills_filtered(.., Venue::Coinbase, ..)` returns the
    /// matching subset (previously `Ok(vec![])` under the Phase 2 gate).
    #[tokio::test]
    async fn recent_fills_filtered_multi_venue_returns_matching_subset() {
        let ledger = open_seeded_ledger().await;
        post_fill(
            &ledger,
            &make_fill("BTCUSDT", Side::Buy, 100),
            Venue::Binance,
            None,
        )
        .await
        .expect("post Binance fill");
        post_fill(
            &ledger,
            &make_fill("BTCUSDT", Side::Buy, 200),
            Venue::Coinbase,
            None,
        )
        .await
        .expect("post Coinbase fill");

        let since = ts_secs(0);
        let until = ts_secs(1_000);

        // Phase 2 behaviour was `Ok(vec![])`; post-008 behaviour returns
        // the Coinbase row.
        let coinbase = recent_fills_filtered(
            &ledger,
            Venue::Coinbase,
            Symbol::new("BTCUSDT"),
            since,
            until,
        )
        .await
        .expect("Coinbase query ok");
        assert_eq!(
            coinbase.len(),
            1,
            "Coinbase must return 1 fill post-008; got {}",
            coinbase.len()
        );
    }

    // ── T1712 — recent_journal_filtered unit tests (Phase 3 R12 / Q7) ─────────

    /// T1712 — multi-venue / multi-kind seed; page-0 cursor returns the
    /// expected slice newest-first. `kind = All` matches every row.
    #[tokio::test]
    async fn recent_journal_filtered_returns_window_subset() {
        let ledger = open_seeded_ledger().await;
        // 3 fills at secs 100 / 200 / 300, all Binance.
        for secs in [100, 200, 300] {
            post_fill(
                &ledger,
                &make_fill("BTCUSDT", Side::Buy, secs),
                Venue::Binance,
                None,
            )
            .await
            .expect("seed Binance fill");
        }
        let since = ts_secs(0);
        let until = ts_secs(1_000);
        let (rows, total) = recent_journal_filtered(
            &ledger,
            &[],
            None,
            AuditKindFilter::All,
            since,
            until,
            0,
            250,
        )
        .await
        .expect("query ok");
        assert_eq!(total, 3, "expected 3 total rows; got {total}");
        assert_eq!(rows.len(), 3, "page 0 returns all 3 rows");
        // Newest-first: ts=300 before 200 before 100.
        assert_eq!(rows[0].ts, ts_secs(300));
        assert_eq!(rows[1].ts, ts_secs(200));
        assert_eq!(rows[2].ts, ts_secs(100));
    }

    /// T1712 — `kind = Fill` isolates fill rows.
    #[tokio::test]
    async fn recent_journal_filtered_kind_fill_isolates_fills() {
        let ledger = open_seeded_ledger().await;
        // 2 fills + 1 registry memo (which lands as a non-fill description
        // via the legacy `registry_event` writer — synthesised here via
        // raw SQL to keep this test self-contained).
        post_fill(
            &ledger,
            &make_fill("BTCUSDT", Side::Buy, 100),
            Venue::Binance,
            None,
        )
        .await
        .expect("seed Buy");
        post_fill(
            &ledger,
            &make_fill("ETHUSDT", Side::Sell, 200),
            Venue::Binance,
            None,
        )
        .await
        .expect("seed Sell");
        sqlx::query(
            "INSERT INTO journal_transactions (id, ts, description, venue) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(uuid::Uuid::new_v4().to_string())
        .bind("2026-04-27T20:00:50Z")
        .bind("registry:Bootstrap:initial seed")
        .bind("binance")
        .execute(ledger.pool())
        .await
        .expect("seed registry row");

        let since = ts_secs(0);
        let until = ts_secs(1_000_000_000);
        let (rows, total) = recent_journal_filtered(
            &ledger,
            &[],
            None,
            AuditKindFilter::Fill,
            since,
            until,
            0,
            250,
        )
        .await
        .expect("query ok");
        assert_eq!(
            total, 2,
            "kind=Fill must isolate the 2 fill rows; got {total}"
        );
        assert_eq!(rows.len(), 2);
        for r in &rows {
            assert!(matches!(r.kind, AuditKindLabel::Fill));
        }
    }

    /// T1712 — empty window returns `Ok((vec![], 0))` (never `Err`).
    #[tokio::test]
    async fn recent_journal_filtered_empty_window_returns_ok_zero() {
        let ledger = open_seeded_ledger().await;
        post_fill(
            &ledger,
            &make_fill("BTCUSDT", Side::Buy, 100),
            Venue::Binance,
            None,
        )
        .await
        .expect("seed");
        let since = ts_secs(10_000_000);
        let until = ts_secs(20_000_000);
        let (rows, total) = recent_journal_filtered(
            &ledger,
            &[],
            None,
            AuditKindFilter::All,
            since,
            until,
            0,
            250,
        )
        .await
        .expect("query ok");
        assert_eq!(total, 0, "far-future window total must be 0");
        assert!(rows.is_empty());
    }

    /// T1712 — pagination: 12 rows seeded, page 0 size 5 returns 5;
    /// page 1 returns 5; page 2 returns 2.
    #[tokio::test]
    async fn recent_journal_filtered_pagination_returns_correct_total() {
        let ledger = open_seeded_ledger().await;
        for secs in 100..112 {
            post_fill(
                &ledger,
                &make_fill("BTCUSDT", Side::Buy, secs),
                Venue::Binance,
                None,
            )
            .await
            .expect("seed");
        }
        let since = ts_secs(0);
        let until = ts_secs(1_000);
        let (rows0, total0) =
            recent_journal_filtered(&ledger, &[], None, AuditKindFilter::All, since, until, 0, 5)
                .await
                .expect("page 0");
        assert_eq!(total0, 12, "total_count is window-wide, not page-bound");
        assert_eq!(rows0.len(), 5, "page 0 returns 5 rows");

        let (rows2, total2) = recent_journal_filtered(
            &ledger,
            &[],
            None,
            AuditKindFilter::All,
            since,
            until,
            10,
            5,
        )
        .await
        .expect("page 2");
        assert_eq!(total2, 12);
        assert_eq!(
            rows2.len(),
            2,
            "page 2 (offset 10, size 5) returns the tail 2"
        );
    }

    /// T1712 — venue predicate isolates the requested venue.
    #[tokio::test]
    async fn recent_journal_filtered_venue_predicate_isolates() {
        let ledger = open_seeded_ledger().await;
        post_fill(
            &ledger,
            &make_fill("BTCUSDT", Side::Buy, 100),
            Venue::Binance,
            None,
        )
        .await
        .expect("seed Binance");
        post_fill(
            &ledger,
            &make_fill("BTCUSDT", Side::Buy, 200),
            Venue::Coinbase,
            None,
        )
        .await
        .expect("seed Coinbase");

        let since = ts_secs(0);
        let until = ts_secs(1_000);

        let (rows, total) = recent_journal_filtered(
            &ledger,
            &[Venue::Coinbase],
            None,
            AuditKindFilter::All,
            since,
            until,
            0,
            250,
        )
        .await
        .expect("Coinbase query ok");
        assert_eq!(total, 1, "single-venue predicate isolates 1 row");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].venue, Venue::Coinbase);
    }

    // ── Phase 4 (T1802) — equity_curve_for_strategy unit tests ───────────────

    /// Inject a closed-trade `income:realized_pnl` row tagged with the
    /// given strategy id and timestamp. Bypasses `post_fill`'s
    /// simplified zero-realized cost basis so tests can verify the
    /// running-sum walk against realistic deltas.
    async fn inject_realized_pnl(
        ledger: &Ledger,
        strategy_id: Option<&str>,
        pnl: Decimal,
        ts: Timestamp,
    ) {
        let ts_str = ts.inner().format(&Rfc3339).expect("rfc3339 fmt");
        let txn_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO journal_transactions (id, ts, description, strategy_id) \
             VALUES (?, ?, ?, ?)",
        )
        .bind(&txn_id)
        .bind(&ts_str)
        .bind("sell 1 BTCUSDT @ 1000")
        .bind(strategy_id)
        .execute(&ledger.pool)
        .await
        .expect("insert transaction");

        let (debit, credit) = if pnl >= Decimal::ZERO {
            (Decimal::ZERO, pnl)
        } else {
            (pnl.abs(), Decimal::ZERO)
        };
        let entry_id = uuid::Uuid::new_v4().to_string();
        sqlx::query(
            "INSERT INTO journal_entries \
             (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&entry_id)
        .bind(&txn_id)
        .bind("income:realized_pnl")
        .bind(debit.to_string())
        .bind(credit.to_string())
        .bind(&ts_str)
        .execute(&ledger.pool)
        .await
        .expect("insert journal entry");

        // Balancing entry against assets:cash:USDT.
        let bal_id = uuid::Uuid::new_v4().to_string();
        let (bal_debit, bal_credit) = if pnl >= Decimal::ZERO {
            (pnl, Decimal::ZERO)
        } else {
            (Decimal::ZERO, pnl.abs())
        };
        sqlx::query(
            "INSERT INTO journal_entries \
             (id, transaction_id, account_id, debit_amount, credit_amount, ts) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&bal_id)
        .bind(&txn_id)
        .bind("assets:cash:USDT")
        .bind(bal_debit.to_string())
        .bind(bal_credit.to_string())
        .bind(&ts_str)
        .execute(&ledger.pool)
        .await
        .expect("insert balancing entry");
    }

    #[tokio::test]
    async fn equity_curve_for_strategy_returns_window_samples() {
        let ledger = open_seeded_ledger().await;
        // 5 known realized-pnl rows: +5, -3, +8, +2, -1.
        let deltas = [dec!(5), dec!(-3), dec!(8), dec!(2), dec!(-1)];
        for (i, d) in deltas.iter().enumerate() {
            let secs = 10 + i64::try_from(i).unwrap_or(0) * 10;
            inject_realized_pnl(&ledger, Some("alpha"), *d, ts_secs(secs)).await;
        }

        let since = ts_secs(0);
        let until = ts_secs(10_000);
        let series =
            equity_curve_for_strategy(&ledger, StrategyId::new("alpha"), since, Some(until))
                .await
                .expect("query ok");
        assert_eq!(series.points.len(), 5);

        // Running-sum walk against the baseline cash balance.
        let baseline = cash_balance(&ledger).await.expect("cash ok").amount();
        let mut running = baseline;
        let expected: Vec<Decimal> = deltas
            .iter()
            .map(|d| {
                running += d;
                running
            })
            .collect();
        for (idx, p) in series.points.iter().enumerate() {
            assert_eq!(
                p.equity.amount(),
                expected[idx],
                "point {idx} running equity mismatch",
            );
        }
    }

    #[tokio::test]
    async fn equity_curve_for_strategy_empty_window_returns_empty_window_err() {
        let ledger = open_seeded_ledger().await;
        let since = ts_secs(0);
        let until = ts_secs(10);
        let res =
            equity_curve_for_strategy(&ledger, StrategyId::new("alpha"), since, Some(until)).await;
        assert!(matches!(res, Err(LedgerError::EmptyWindow)));
    }

    #[tokio::test]
    async fn equity_curve_for_strategy_until_none_includes_to_now() {
        let ledger = open_seeded_ledger().await;
        // Seed a row at "5 seconds ago"; until=None reads
        // Timestamp::now() so the row must surface.
        let now = OffsetDateTime::now_utc();
        let ts5 = Timestamp::new(now - time::Duration::seconds(5));
        inject_realized_pnl(&ledger, Some("alpha"), dec!(7), ts5).await;

        let since = Timestamp::new(OffsetDateTime::UNIX_EPOCH);
        let series = equity_curve_for_strategy(&ledger, StrategyId::new("alpha"), since, None)
            .await
            .expect("until=None must succeed");
        assert_eq!(series.points.len(), 1);
    }

    #[tokio::test]
    async fn equity_curve_for_strategy_filters_by_strategy_id() {
        let ledger = open_seeded_ledger().await;
        // alpha: 2 rows; beta: 1 row.
        inject_realized_pnl(&ledger, Some("alpha"), dec!(10), ts_secs(10)).await;
        inject_realized_pnl(&ledger, Some("alpha"), dec!(5), ts_secs(20)).await;
        inject_realized_pnl(&ledger, Some("beta"), dec!(20), ts_secs(30)).await;

        let since = ts_secs(0);
        let until = ts_secs(10_000);
        let alpha_series =
            equity_curve_for_strategy(&ledger, StrategyId::new("alpha"), since, Some(until))
                .await
                .expect("alpha ok");
        assert_eq!(alpha_series.points.len(), 2);

        let beta_series =
            equity_curve_for_strategy(&ledger, StrategyId::new("beta"), since, Some(until))
                .await
                .expect("beta ok");
        assert_eq!(beta_series.points.len(), 1);
    }

    /// T1606 — calling for ETHUSDT returns the ETHUSDT subset only.
    #[tokio::test]
    async fn recent_fills_filtered_distinct_symbols_isolated() {
        let ledger = open_seeded_ledger().await;
        for secs in [100, 200] {
            post_fill(
                &ledger,
                &make_fill("BTCUSDT", Side::Buy, secs),
                Venue::Binance,
                None,
            )
            .await
            .expect("seed BTC");
        }
        for secs in [150, 250] {
            post_fill(
                &ledger,
                &make_fill("ETHUSDT", Side::Sell, secs),
                Venue::Binance,
                None,
            )
            .await
            .expect("seed ETH");
        }
        let since = ts_secs(0);
        let until = ts_secs(1_000);
        let eth = recent_fills_filtered(
            &ledger,
            Venue::Binance,
            Symbol::new("ETHUSDT"),
            since,
            until,
        )
        .await
        .expect("eth ok");
        assert_eq!(eth.len(), 2);
        for f in &eth {
            assert_eq!(f.symbol, Symbol::new("ETHUSDT"));
        }
    }

    // ── cockpit-training-control T-D-N9 — training reader tests ───────────────

    use crate::journal::{
        post_training_epoch, post_training_failed, post_training_finish, post_training_start,
    };

    async fn seed_run(ledger: &Ledger, run_id: &str, scenario: &str, seed: i64, pid: Option<i64>) {
        post_training_start(ledger, run_id, scenario, seed, pid)
            .await
            .expect("start");
    }

    /// T-D-N9 V1 — `recent_training_events` returns rows in the half-open
    /// `[since, until)` window, newest-first.
    #[tokio::test]
    async fn recent_training_events_filters_by_window() {
        let ledger = open_seeded_ledger().await;
        let run_id = "qrec-0001-0000-0000-0000-000000000001";

        seed_run(&ledger, run_id, "bs1", 1, None).await;
        post_training_epoch(&ledger, run_id, 1, 10, 0.5_f32, 0.4_f32, 1000)
            .await
            .expect("epoch 1");
        post_training_epoch(&ledger, run_id, 2, 10, 0.3_f32, 0.25_f32, 2000)
            .await
            .expect("epoch 2");

        // Use a wide window that captures the inserts.
        let since = "2000-01-01T00:00:00.000000Z";
        let until = "2099-12-31T23:59:59.999999Z";
        let rows = recent_training_events(&ledger, since, until)
            .await
            .expect("recent_training_events");

        // start + 2 epochs = 3 rows total.
        assert_eq!(rows.len(), 3, "must return all 3 rows in the window");
        // Newest first: epoch rows have higher ts than start row (inserted after).
        // The kind column tells us the order:
        let kinds: Vec<&str> = rows.iter().map(|r| r.kind.as_str()).collect();
        // epoch-2 or epoch-1 first (both after start), then start.
        assert_eq!(
            kinds.last(),
            Some(&"start"),
            "start is oldest → appears last"
        );
    }

    /// T-D-N9 V2 — `recent_training_events` returns empty when the window
    /// is before all rows (no rows in [past, past+1s)).
    #[tokio::test]
    async fn recent_training_events_empty_outside_window() {
        let ledger = open_seeded_ledger().await;
        let run_id = "qrec-0002-0000-0000-0000-000000000002";
        seed_run(&ledger, run_id, "bs2", 7, None).await;

        // Window entirely before Unix epoch (before any ts we'd insert).
        let rows = recent_training_events(
            &ledger,
            "1970-01-01T00:00:00.000000Z",
            "1970-01-01T00:00:01.000000Z",
        )
        .await
        .expect("recent_training_events");
        assert!(rows.is_empty(), "no rows before epoch window");
    }

    /// T-D-N9 V3 — `latest_training_run` returns `None` on an empty table.
    #[tokio::test]
    async fn latest_training_run_none_when_empty() {
        let ledger = open_seeded_ledger().await;
        let result = latest_training_run(&ledger)
            .await
            .expect("latest_training_run");
        assert!(result.is_none(), "must be None on empty table");
    }

    /// T-D-N9 V4 — `latest_training_run` returns status='running' when
    /// only a start row exists.
    #[tokio::test]
    async fn latest_training_run_running_status() {
        let ledger = open_seeded_ledger().await;
        let run_id = "qrun-0001-0000-0000-0000-000000000001";
        seed_run(&ledger, run_id, "bs1", 42, Some(12345)).await;

        let summary = latest_training_run(&ledger)
            .await
            .expect("latest_training_run")
            .expect("must have Some");

        assert_eq!(summary.run_id.as_str(), run_id);
        assert_eq!(summary.status.as_str(), "running");
        assert_eq!(summary.scenario.as_str(), "bs1");
        assert_eq!(summary.seed, 42);
        assert_eq!(summary.pid, Some(12345));
        assert!(summary.model_revision.is_none());
        assert_eq!(summary.latest_epoch, 0, "no epochs yet");
    }

    /// T-D-N9 V5 — `latest_training_run` returns status='done' after a
    /// finish row is inserted, and `model_revision` is populated.
    #[tokio::test]
    async fn latest_training_run_done_status() {
        let ledger = open_seeded_ledger().await;
        let run_id = "qrun-0002-0000-0000-0000-000000000002";
        let model_rev = "deadbeef1234deadbeef1234deadbeef12340001";

        seed_run(&ledger, run_id, "bs2", 99, None).await;
        post_training_epoch(&ledger, run_id, 1, 5, 0.4_f32, 0.35_f32, 3000)
            .await
            .expect("epoch");
        post_training_finish(&ledger, run_id, model_rev, 0.4_f32, 0.35_f32, 10_000)
            .await
            .expect("finish");

        let summary = latest_training_run(&ledger)
            .await
            .expect("latest_training_run")
            .expect("must have Some");

        assert_eq!(summary.status.as_str(), "done");
        assert_eq!(summary.model_revision.as_deref(), Some(model_rev));
        assert_eq!(summary.latest_epoch, 1);
    }

    /// T-D-N9 V6 — `latest_training_run` returns status='failed' after a
    /// failed row is inserted, and `error_message` is populated.
    #[tokio::test]
    async fn latest_training_run_failed_status() {
        let ledger = open_seeded_ledger().await;
        let run_id = "qrun-0003-0000-0000-0000-000000000003";

        seed_run(&ledger, run_id, "default", 1, None).await;
        post_training_failed(&ledger, run_id, "CUDA out of memory")
            .await
            .expect("failed");

        let summary = latest_training_run(&ledger)
            .await
            .expect("latest_training_run")
            .expect("must have Some");

        assert_eq!(summary.status.as_str(), "failed");
        assert_eq!(summary.error_message.as_deref(), Some("CUDA out of memory"));
        assert!(summary.model_revision.is_none());
    }

    /// T-D-N9 V7 — `orphan_training_runs` returns runs that started more
    /// than `fresh_window_secs` ago with no finish/failed row. Completed
    /// runs are excluded.
    ///
    /// We directly INSERT rows with a known past timestamp (2020-01-01) so
    /// the window check (`ts < now - fresh_window_secs`) is deterministic
    /// regardless of wall-clock timing. The completed run is also inserted
    /// with a past timestamp but has a matching `finish` row.
    #[tokio::test]
    async fn orphan_training_runs_excludes_completed() {
        let ledger = open_seeded_ledger().await;

        let orphan_id = "qorp-0001-0000-0000-0000-000000000001";
        let done_id = "qorp-0002-0000-0000-0000-000000000002";
        // A fixed past timestamp well outside any reasonable fresh_window.
        let past_ts = "2020-01-01T00:00:00.000000Z";

        // Insert orphan start row directly (no finish/failed).
        sqlx::query(
            "INSERT INTO training_events \
             (id, ts, run_id, kind, epoch, total_epochs, train_loss, val_loss, \
              wall_clock_ms, model_revision, scenario, seed, pid, error_message) \
             VALUES ('orphan-row-id-001', ?, ?, 'start', NULL, NULL, NULL, NULL, NULL, NULL, 'bs1', 1, 777, NULL)",
        )
        .bind(past_ts)
        .bind(orphan_id)
        .execute(ledger.pool())
        .await
        .expect("insert orphan start");

        // Insert done start row + finish row.
        sqlx::query(
            "INSERT INTO training_events \
             (id, ts, run_id, kind, epoch, total_epochs, train_loss, val_loss, \
              wall_clock_ms, model_revision, scenario, seed, pid, error_message) \
             VALUES ('done-row-id-start', ?, ?, 'start', NULL, NULL, NULL, NULL, NULL, NULL, 'bs2', 2, NULL, NULL)",
        )
        .bind(past_ts)
        .bind(done_id)
        .execute(ledger.pool())
        .await
        .expect("insert done start");

        sqlx::query(
            "INSERT INTO training_events \
             (id, ts, run_id, kind, epoch, total_epochs, train_loss, val_loss, \
              wall_clock_ms, model_revision, scenario, seed, pid, error_message) \
             VALUES ('done-row-id-finish', ?, ?, 'finish', 5, 5, '0.1', '0.1', 1000, 'rev001', 'bs2', 2, NULL, NULL)",
        )
        .bind(past_ts)
        .bind(done_id)
        .execute(ledger.pool())
        .await
        .expect("insert done finish");

        // fresh_window = 60 seconds: cutoff = now - 60s.
        // Our rows have ts = 2020-01-01 which is far before the cutoff.
        let orphans = orphan_training_runs(&ledger, 60)
            .await
            .expect("orphan_training_runs");

        let orphan_ids: Vec<&str> = orphans.iter().map(|o| o.run_id.as_str()).collect();
        assert!(
            orphan_ids.contains(&orphan_id),
            "orphan run must appear; got {orphan_ids:?}"
        );
        assert!(
            !orphan_ids.contains(&done_id),
            "completed run must not appear; got {orphan_ids:?}"
        );
        // Verify pid is captured.
        let orphan_entry = orphans
            .iter()
            .find(|o| o.run_id.as_str() == orphan_id)
            .unwrap();
        assert_eq!(orphan_entry.pid, Some(777), "pid must be captured");
    }
}
