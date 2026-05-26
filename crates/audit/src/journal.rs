//! Journal entry writers (R3.3).
//!
//! Every fill writes a balanced double-entry transaction atomically.
//! Debits == Credits is enforced per transaction.
//!
//! # Tee opt-in convention (K5 mitigation — audit-tick-consumer-envelope v0.1.0)
//!
//! Every writer that calls `db_txn.commit()` (or `execute(...)` on a
//! single-shot row) and represents an event a consumer might care about MUST
//! grow a `crate::tick::emit(ledger, AuditEvent::…)` call **after** the
//! commit. The in-scope writers at v0.1.0 are enumerated in
//! `spec/audit-tick-consumer-envelope/decomp.md §3`. Adding a new variant
//! requires an ADR amendment.
//!
//! ## chart-buy-sell-emphasis v1.9 — strategy-signal writers (T2014)
//!
//! [`post_strategy_signal`] and [`update_signal_clamp_status`] write to
//! the additive `strategy_signals` table created by migration 009. They
//! are the writer half of the ghost-marker layer (R5) — the cockpit
//! reads via [`crate::query::recent_signals`] and paints one ghost-
//! triangle per row.
//!
//! **Forward-compat note (R9.1, M3 task block):** This brief ships the
//! writer + reader + config gate + cockpit read path. The live agent-
//! runtime tap point that *calls* `post_strategy_signal` per emitted
//! `Signal` is **deferred to a follow-up brief** (the agent-runtime
//! track). With `agent.toml [signal_log] enabled = false` (the v1.9
//! default — see `crates/agent/src/config.rs::SignalLogConfig`), zero
//! rows land in production until an operator opts in. The reader
//! naturally returns `Ok(vec![])` on the empty table (V11c). When the
//! follow-up brief lands the live tap, it imports
//! [`post_strategy_signal`] and the ghost layer comes alive without
//! any further cockpit-side change.
use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use tracing::instrument;
use trading_core::{Fill, FundingObs, LedgerError, Price, Quantity, Side, Signal, Venue};
use uuid::Uuid;

use crate::Ledger;

/// Phase D extension of [`post_fill`] (R1.1 + R1.2 — ui-rethink-phase-d-trail).
///
/// Threads the upstream `signal_id` (from `strategy_signals.id` — the row id
/// returned by [`post_strategy_signal`]) into the
/// `journal_transactions.signal_id` column and persists the canonical
/// `fill.id` into `journal_transactions.fill_id` (separate from the
/// synthesised `journal_transactions.id` UUID the writer mints internally).
///
/// `post_fill(ledger, fill, venue, strategy_id)` is a thin
/// `post_fill_with_signal(ledger, fill, venue, strategy_id, None)`
/// wrapper — backwards-compatible per mig 004's `strategy_id` precedent (R1.2).
///
/// Returns the generated `journal_transactions.id` (a UUID v4 string wrapped
/// in [`SmolStr`]).
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL transaction fails.
#[allow(clippy::too_many_lines)] // double-entry for buy and sell requires this length
#[instrument(name = "ledger.post_fill_with_signal", skip(ledger, fill, signal_id),
    fields(fill_id = %fill.id, venue = %venue, strategy_id = ?strategy_id))]
pub async fn post_fill_with_signal(
    ledger: &Ledger,
    fill: &Fill,
    venue: Venue,
    strategy_id: Option<&str>,
    signal_id: Option<&str>,
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
    // Phase D: persist fill.id (the canonical Fill correlation id from mig 011).
    let fill_id_str = fill.id.to_string();

    let mut db_txn = ledger
        .pool
        .begin()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // Insert transaction header. The `strategy_id` column was added in
    // migration 004; `fill_id` and `signal_id` added in mig 011. All are
    // storage-only — they never surface in the rendered backtest report body.
    sqlx::query(
        "INSERT INTO journal_transactions \
         (id, ts, description, strategy_id, venue, fill_id, signal_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&txn_id)
    .bind(&ts)
    .bind(&description)
    .bind(strategy_id)
    .bind(venue.to_string())
    .bind(&fill_id_str)
    .bind(signal_id)
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

    // Post-commit tee — fires after the SQL transaction is durable (H2 / R2.3).
    crate::tick::emit(
        ledger,
        crate::tick::AuditEvent::Fill {
            fill: Box::new(fill.clone()),
            fees: fee,
        },
    );
    tracing::debug!(fill_id = %fill.id, side = %fill.side, notional = %notional, "fill journaled");
    Ok(SmolStr::new(&txn_id))
}

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
/// Thin wrapper over [`post_fill_with_signal`] with `signal_id = None`
/// for backwards-compatibility (mig 004 `strategy_id` precedent). All
/// callers that do not have a `signal_id` call this directly; callers
/// that thread signal lineage through use [`post_fill_with_signal`].
///
/// Returns the generated `journal_transactions.id` (a UUID v4 string wrapped
/// in [`SmolStr`]).
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL transaction fails.
#[instrument(name = "ledger.post_fill", skip(ledger, fill), fields(fill_id = %fill.id, venue = %venue, strategy_id = ?strategy_id))]
pub async fn post_fill(
    ledger: &Ledger,
    fill: &Fill,
    venue: Venue,
    strategy_id: Option<&str>,
) -> Result<SmolStr, LedgerError> {
    post_fill_with_signal(ledger, fill, venue, strategy_id, None).await
}

// ── chart-buy-sell-emphasis v1.9 (T2014) — strategy_signals writers ──────────

/// chart-buy-sell-emphasis v1.9 (T2014) — INSERT one row into
/// `strategy_signals` (migration 009) for the supplied `signal`.
///
/// Sibling of [`post_fill`] in shape (atomic `pool.begin / commit`,
/// UUID v4 row id, RFC-3339 microsecond ts). Pure additive — no money
/// columns, no chart-of-accounts impact, no reconciler-invariant
/// effect.
///
/// `intended_qty` carries the strategy-proposed quantity; `intended_price`
/// is `None` for market signals and `Some(price)` for limit-order
/// shapes (Q9 forward-compat — v1 strategies all emit market signals).
///
/// `was_clamped` is `false` on the initial INSERT (steady-state). The
/// risk engine's decision is captured by [`update_signal_clamp_status`]
/// after the risk engine returns. Callers that already know the
/// risk-decision at INSERT-time may pass `was_clamped = true` and a
/// `clamp_reason` directly; the typical agent-loop pattern is
/// `post_strategy_signal(..., false, None)` followed by
/// `update_signal_clamp_status(..., true, Some("per_symbol_cap"))`
/// once the risk engine has decided.
///
/// `Signal.kind` is projected onto the `side TEXT` column via
/// [`signal_kind_to_side_str`]: `Buy` / `OpenPairLong` → `"buy"`;
/// `Sell` / `ClosePair` / `PairShortObservation` → `"sell"`. `Hold`
/// signals are skipped (no row written; the writer returns `Ok(empty
/// SmolStr)`) — Hold carries no actionable intent and the ghost layer
/// has nothing to render.
///
/// Returns the generated `strategy_signals.id` UUID v4 wrapped in
/// [`SmolStr`] so the caller can pair the INSERT with a downstream
/// [`update_signal_clamp_status`] call keyed on the same id. For Hold
/// signals (no row written) returns an empty `SmolStr`.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL transaction fails.
#[instrument(
    name = "ledger.post_strategy_signal",
    skip(ledger, signal),
    fields(
        strategy_id = %signal.strategy_id,
        symbol = %signal.symbol,
        kind = ?signal.kind,
        venue = %venue,
        was_clamped,
    )
)]
#[allow(clippy::too_many_arguments)]
pub async fn post_strategy_signal(
    ledger: &Ledger,
    signal: &Signal,
    intended_qty: Quantity,
    intended_price: Option<Price>,
    venue: Venue,
    was_clamped: bool,
    clamp_reason: Option<&str>,
    forecast_correlation_id: Option<Uuid>, // Phase D R1.3 — NEW; existing callers pass None
) -> Result<SmolStr, LedgerError> {
    // Hold signals carry no actionable intent; the ghost layer has
    // nothing to render. Caller can rely on the empty SmolStr to mean
    // "no row written" without inspecting the row count.
    let Some(side_str) = signal_kind_to_side_str(signal) else {
        return Ok(SmolStr::default());
    };

    let row_id = Uuid::new_v4().to_string();

    // 6-digit microsecond ts — matches the strategy_events writers
    // (HF-3 / architect risk #4 determinism gate). Two consecutive
    // signals within the same wall-clock second still produce
    // monotonically-ordered `ts` values.
    let ts_fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    )
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let ts = signal
        .ts
        .inner()
        .format(&ts_fmt)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    let intended_qty_str = intended_qty.get().to_string();
    let intended_price_str = intended_price.map(|p| p.get().to_string());
    let venue_str = venue.to_string();
    let was_clamped_i = i64::from(was_clamped);
    // Phase D R1.3 — bind the upstream forecast correlation id (mig 011 new column).
    let forecast_corr_id_str = forecast_correlation_id.map(|u| u.to_string());

    // Atomic — sibling of post_fill. No new on-disk write path; reuses
    // the established `ledger.pool.begin() / commit()` shape (hard-
    // constraint 4 — atomic-write contract).
    let mut db_txn = ledger
        .pool
        .begin()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    sqlx::query(
        "INSERT INTO strategy_signals \
         (id, ts, strategy_id, venue, symbol, side, intended_qty_str, \
          intended_price_str, was_clamped, clamp_reason, forecast_correlation_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row_id)
    .bind(&ts)
    .bind(signal.strategy_id.0.as_str())
    .bind(&venue_str)
    .bind(signal.symbol.0.as_str())
    .bind(side_str)
    .bind(&intended_qty_str)
    .bind(&intended_price_str)
    .bind(was_clamped_i)
    .bind(clamp_reason)
    .bind(forecast_corr_id_str)
    .execute(&mut *db_txn)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    db_txn
        .commit()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // Post-commit tee — Hold branch returns early above (no SQL row → no tick).
    crate::tick::emit(
        ledger,
        crate::tick::AuditEvent::StrategySignal {
            strategy_id: signal.strategy_id.clone(),
            signal: Box::new(signal.clone()),
        },
    );
    tracing::debug!(
        row_id = %row_id,
        strategy_id = %signal.strategy_id,
        symbol = %signal.symbol,
        "strategy_signal persisted"
    );
    Ok(SmolStr::new(&row_id))
}

/// chart-buy-sell-emphasis v1.9 (T2014) — UPDATE the
/// `was_clamped` / `clamp_reason` columns on the row identified by
/// `signal_id`. Called by the agent's risk engine after it has decided
/// whether to clamp (or veto) the previously-posted signal.
///
/// One single-row UPDATE inside its own `pool.begin / commit` — same
/// atomic-write shape as [`post_strategy_signal`]. A missing row
/// (e.g. caller passed a stale id) is silently a no-op — sqlx returns
/// 0 rows affected without erroring, and the caller's tracing-span
/// captures the id-mismatch for debugging.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL UPDATE fails.
#[instrument(
    name = "ledger.update_signal_clamp_status",
    skip(ledger),
    fields(signal_id, was_clamped)
)]
pub async fn update_signal_clamp_status(
    ledger: &Ledger,
    signal_id: &str,
    was_clamped: bool,
    clamp_reason: Option<&str>,
) -> Result<(), LedgerError> {
    let was_clamped_i = i64::from(was_clamped);

    let mut db_txn = ledger
        .pool
        .begin()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    sqlx::query("UPDATE strategy_signals SET was_clamped = ?, clamp_reason = ? WHERE id = ?")
        .bind(was_clamped_i)
        .bind(clamp_reason)
        .bind(signal_id)
        .execute(&mut *db_txn)
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    db_txn
        .commit()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}

// ── ui-rethink-phase-d-trail Wave A — forecast_events writer (R1.4) ──────────

/// Phase D writer (R1.4 — ui-rethink-phase-d-trail). Persists a
/// [`trading_core::ForecastOverlay`] to the `forecast_events` table (mig 011)
/// and fires the **existing** `AuditEvent::ForecastEmitted` tick (no new
/// variant — the payload already carries `overlay.correlation_id`).
///
/// Call sites: the two existing `crates/forecast/src/tcn.rs` emit sites
/// (cache-hit at L822-831 and post-inference at L937-947) invoke this
/// **alongside** the existing tick-emit; the tick path stays the broadcast
/// contract for live consumers while the SQL row closes the durability gap
/// (K6 — restart-consumer backfill).
///
/// **Idempotent on `correlation_id` PK.** `INSERT OR IGNORE` — cache-hit and
/// post-inference branches may both fire for the same `correlation_id` on a
/// replay-warm cache; the second emit is a no-op at the SQL layer.
///
/// **Determinism gate.** This writer takes a `&Ledger`. In backtests the ledger
/// is constructed via `Ledger::open` (no tick bus → `tick.rs:104-107`
/// static-branch tee dormant); the SQL row still lands. Pre-existing 22
/// anchors do not read this table → anchor-safe.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL INSERT fails.
#[instrument(name = "ledger.post_forecast_event", skip(ledger, overlay),
    fields(correlation_id = %overlay.correlation_id, strategy_id, symbol, cache_hit))]
pub async fn post_forecast_event(
    ledger: &Ledger,
    overlay: &trading_core::ForecastOverlay,
    strategy_id: &str,
    symbol: &str,
    cache_hit: bool,
) -> Result<(), LedgerError> {
    // 6-digit microsecond ts — mirrors post_strategy_signal (HF-3 / ADR-0004
    // determinism gate).
    let ts_fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    )
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let ts = overlay
        .sampled_at
        .format(&ts_fmt)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    let direction_str = match overlay.direction {
        trading_core::Direction::Up => "up",
        trading_core::Direction::Down => "down",
        trading_core::Direction::Flat => "flat",
    };
    let confidence_str = overlay.confidence.to_string();
    let cache_hit_i = i64::from(cache_hit);

    sqlx::query(
        "INSERT OR IGNORE INTO forecast_events \
         (correlation_id, ts, strategy_id, symbol, direction, \
          confidence, model_revision, cache_hit) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(overlay.correlation_id.to_string())
    .bind(&ts)
    .bind(strategy_id)
    .bind(symbol)
    .bind(direction_str)
    .bind(&confidence_str)
    .bind(&overlay.model_revision)
    .bind(cache_hit_i)
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    tracing::debug!(
        correlation_id = %overlay.correlation_id,
        strategy_id,
        symbol,
        cache_hit,
        "forecast_event persisted"
    );
    Ok(())
}

// ── cockpit-training-control T-D-N8 ──────────────────────────────────────────

/// Microsecond-precision RFC3339 timestamp string — matches the format used by
/// `post_strategy_signal` and `uptime_ts_string` (HF-3 / ADR-0004 gate).
///
/// Two consecutive writes within the same wall-clock second still produce
/// monotonically-ordered `ts` values under sub-second scheduling.
fn training_ts_now() -> Result<String, LedgerError> {
    let fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    )
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    time::OffsetDateTime::now_utc()
        .format(&fmt)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))
}

/// Write a `kind='start'` row to `training_events` (R4.4, ADR-0034 § D5).
///
/// Called once per `train_tcn` invocation, immediately after config parsing.
/// Captures the OS `pid` of the subprocess so the orphan-detect reader
/// (`query::orphan_training_runs`) can do a liveness check without
/// round-tripping through external state.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL INSERT fails.
#[instrument(
    name = "ledger.post_training_start",
    skip(ledger),
    fields(run_id, scenario, seed)
)]
pub async fn post_training_start(
    ledger: &Ledger,
    run_id: &str,
    scenario: &str,
    seed: i64,
    pid: Option<i64>,
) -> Result<(), LedgerError> {
    let id = Uuid::new_v4().to_string();
    let ts = training_ts_now()?;

    sqlx::query(
        "INSERT INTO training_events \
         (id, ts, run_id, kind, epoch, total_epochs, train_loss, val_loss, \
          wall_clock_ms, model_revision, scenario, seed, pid, error_message) \
         VALUES (?, ?, ?, 'start', NULL, NULL, NULL, NULL, NULL, NULL, ?, ?, ?, NULL)",
    )
    .bind(&id)
    .bind(&ts)
    .bind(run_id)
    .bind(scenario)
    .bind(seed)
    .bind(pid)
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    tracing::debug!(run_id, scenario, seed, pid = ?pid, "training_start persisted");
    Ok(())
}

/// Write a `kind='epoch'` row to `training_events` (R4.4, ADR-0034 § D5).
///
/// Called once per completed epoch at the `info!(epoch = ..., train_loss = …)`
/// site in `train_tcn.rs`. `train_loss` / `val_loss` are stored as TEXT
/// (Decimal-as-TEXT contract per ADR-0003) — lossless for the plot surface.
/// `wall_clock_ms` is measured via `Instant::now() - epoch_start` at the call
/// site; it is wall-clock only and does not affect model determinism (R5.2).
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL INSERT fails.
#[instrument(
    name = "ledger.post_training_epoch",
    skip(ledger),
    fields(run_id, epoch, total_epochs)
)]
pub async fn post_training_epoch(
    ledger: &Ledger,
    run_id: &str,
    epoch: i64,
    total_epochs: i64,
    train_loss: f32,
    val_loss: f32,
    wall_clock_ms: i64,
) -> Result<(), LedgerError> {
    let id = Uuid::new_v4().to_string();
    let ts = training_ts_now()?;
    let train_loss_str = train_loss.to_string();
    let val_loss_str = val_loss.to_string();

    sqlx::query(
        "INSERT INTO training_events \
         (id, ts, run_id, kind, epoch, total_epochs, train_loss, val_loss, \
          wall_clock_ms, model_revision, scenario, seed, pid, error_message) \
         VALUES (?, ?, ?, 'epoch', ?, ?, ?, ?, ?, NULL, \
          (SELECT scenario FROM training_events WHERE run_id = ? AND kind = 'start' LIMIT 1), \
          (SELECT seed    FROM training_events WHERE run_id = ? AND kind = 'start' LIMIT 1), \
          NULL, NULL)",
    )
    .bind(&id)
    .bind(&ts)
    .bind(run_id)
    .bind(epoch)
    .bind(total_epochs)
    .bind(&train_loss_str)
    .bind(&val_loss_str)
    .bind(wall_clock_ms)
    .bind(run_id) // scenario subquery
    .bind(run_id) // seed subquery
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    tracing::debug!(
        run_id,
        epoch,
        total_epochs,
        train_loss,
        val_loss,
        wall_clock_ms,
        "training_epoch persisted"
    );
    Ok(())
}

/// Write a `kind='finish'` row to `training_events` (R4.4, ADR-0034 § D5).
///
/// Called once at the end of `write_checkpoint` with the canonical
/// `model_revision` SHA from `CheckpointMetadata`. `total_wall_clock_ms` is
/// the full run duration measured at the call site.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL INSERT fails.
#[instrument(
    name = "ledger.post_training_finish",
    skip(ledger),
    fields(run_id, model_revision)
)]
pub async fn post_training_finish(
    ledger: &Ledger,
    run_id: &str,
    model_revision: &str,
    final_train_loss: f32,
    final_val_loss: f32,
    total_wall_clock_ms: i64,
) -> Result<(), LedgerError> {
    let id = Uuid::new_v4().to_string();
    let ts = training_ts_now()?;
    let train_loss_str = final_train_loss.to_string();
    let val_loss_str = final_val_loss.to_string();

    sqlx::query(
        "INSERT INTO training_events \
         (id, ts, run_id, kind, epoch, total_epochs, train_loss, val_loss, \
          wall_clock_ms, model_revision, scenario, seed, pid, error_message) \
         VALUES (?, ?, ?, 'finish', \
          (SELECT MAX(epoch) FROM training_events WHERE run_id = ? AND kind = 'epoch'), \
          (SELECT MAX(total_epochs) FROM training_events WHERE run_id = ? AND kind = 'epoch'), \
          ?, ?, ?, ?, \
          (SELECT scenario FROM training_events WHERE run_id = ? AND kind = 'start' LIMIT 1), \
          (SELECT seed    FROM training_events WHERE run_id = ? AND kind = 'start' LIMIT 1), \
          NULL, NULL)",
    )
    .bind(&id)
    .bind(&ts)
    .bind(run_id)
    .bind(run_id) // epoch subquery
    .bind(run_id) // total_epochs subquery
    .bind(&train_loss_str)
    .bind(&val_loss_str)
    .bind(total_wall_clock_ms)
    .bind(model_revision)
    .bind(run_id) // scenario subquery
    .bind(run_id) // seed subquery
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    tracing::debug!(
        run_id,
        model_revision,
        final_train_loss,
        final_val_loss,
        total_wall_clock_ms,
        "training_finish persisted"
    );
    Ok(())
}

/// Write a `kind='failed'` row to `training_events` (R4.4, ADR-0034 § D5).
///
/// Called by the top-level error handler in `train_tcn::main()` when the
/// training loop exits with an `Err`. The `error_message` carries the last
/// fatal error string. This row is best-effort on `SIGKILL` — there may not
/// be enough time to write it before the process is killed.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] if the SQL INSERT fails.
#[instrument(name = "ledger.post_training_failed", skip(ledger), fields(run_id))]
pub async fn post_training_failed(
    ledger: &Ledger,
    run_id: &str,
    error_message: &str,
) -> Result<(), LedgerError> {
    let id = Uuid::new_v4().to_string();
    let ts = training_ts_now()?;

    sqlx::query(
        "INSERT INTO training_events \
         (id, ts, run_id, kind, epoch, total_epochs, train_loss, val_loss, \
          wall_clock_ms, model_revision, scenario, seed, pid, error_message) \
         VALUES (?, ?, ?, 'failed', NULL, NULL, NULL, NULL, NULL, NULL, \
          (SELECT scenario FROM training_events WHERE run_id = ? AND kind = 'start' LIMIT 1), \
          (SELECT seed    FROM training_events WHERE run_id = ? AND kind = 'start' LIMIT 1), \
          NULL, ?)",
    )
    .bind(&id)
    .bind(&ts)
    .bind(run_id)
    .bind(run_id) // scenario subquery
    .bind(run_id) // seed subquery
    .bind(error_message)
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    tracing::debug!(run_id, error_message, "training_failed persisted");
    Ok(())
}

/// Project `Signal.kind` onto the `strategy_signals.side` TEXT column.
///
/// Returns:
/// - `Some("buy")` for `Buy` and `OpenPairLong`.
/// - `Some("sell")` for `Sell`, `ClosePair`, and `PairShortObservation`
///   (the would-have-shorted observation is recorded as a `sell`-side
///   signal so the ghost layer paints the same down-triangle the
///   operator already associates with sell intent).
/// - `None` for `Hold` — Hold carries no actionable intent and the
///   ghost layer has nothing to render, so the writer skips the INSERT
///   entirely.
///
/// The mapping lives next to the writer (not on the `SignalKind` enum)
/// because the projection is audit-specific — other consumers of
/// `SignalKind` (risk engine, backtest binary) discriminate all six
/// variants explicitly.
fn signal_kind_to_side_str(signal: &Signal) -> Option<&'static str> {
    use trading_core::SignalKind::{
        Buy, ClosePair, Hold, OpenPairLong, PairShortObservation, Sell,
    };
    match signal.kind {
        Buy | OpenPairLong => Some("buy"),
        Sell | ClosePair | PairShortObservation => Some("sell"),
        Hold => None,
    }
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
    // Post-commit tee (R2.3 / T-D-6).
    crate::tick::emit(
        ledger,
        crate::tick::AuditEvent::KillSwitchTripped {
            reason: smol_str::SmolStr::new(reason),
        },
    );
    Ok(())
}

/// Phase 5 R5 — operator paused or resumed a strategy via the per-row
/// pause/resume button on the Strategies-detail screen. Atomic dual-
/// write — memo row in `journal_transactions` + `strategy_events` row
/// in one transaction (sibling of [`kill_switch_tripped`]).
///
/// - `paused == true`  → `error_summary = "paused"`,
///   memo description `"strategy:StrategyPaused:<id>:paused"`.
/// - `paused == false` → `error_summary = "resumed"`,
///   memo description `"strategy:StrategyPaused:<id>:resumed"`.
///
/// Memo row uses `Rfc3339` second precision (preserved from
/// [`kill_switch_tripped`]); the `strategy_events` row uses the 6-digit
/// fractional-second format used by every other v0.5+/v1+ writer
/// (HF-3 / architect risk #4 determinism gate).
///
/// `strategy_id` is bound to the affected strategy; `error_code =
/// "strategy_paused"`; `venue = NULL`.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error. On failure
/// the entire transaction rolls back; neither row is left orphaned.
#[instrument(name = "ledger.strategy_paused", skip(ledger))]
pub async fn strategy_paused(
    ledger: &Ledger,
    strategy_id: &str,
    paused: bool,
    operator: &str,
) -> Result<(), LedgerError> {
    let direction = if paused { "paused" } else { "resumed" };
    let metadata = serde_json::json!({
        "event": "StrategyPaused",
        "strategy_id": strategy_id,
        "paused": paused,
        "operator": operator,
    })
    .to_string();

    // Memo-row timestamp: RFC-3339 second precision — matches the v0
    // byte-for-byte registry_event format (preserved from
    // kill_switch_tripped).
    let memo_ts = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let memo_txn_id = Uuid::new_v4().to_string();
    let memo_description = format!("strategy:StrategyPaused:{strategy_id}:{direction}");

    // strategy_events row timestamp: 6-digit microsecond format.
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

    // (1) memo row — preserves the audit-ledger's "every operator
    // decision is reconstructible" rule.
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

    // (2) strategy_events row — operator-success-report and
    // recent_journal_filtered surface this row.
    sqlx::query(
        "INSERT INTO strategy_events \
         (id, ts, kind, strategy_id, old_hash, new_hash, source_path, operator, error_code, error_summary, venue) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&strategy_row_id)
    .bind(&strategy_ts)
    .bind("StrategyPaused")
    .bind(Some(strategy_id))
    .bind::<Option<&str>>(None) // old_hash
    .bind::<Option<&str>>(None) // new_hash
    .bind("") // source_path
    .bind(operator)
    .bind(Some("strategy_paused"))
    .bind(Some(direction))
    .bind::<Option<&str>>(None) // venue — operator action; venue-agnostic
    .execute(&mut *db_txn)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    db_txn
        .commit()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    Ok(())
}

/// Phase 5 R8 — operator overrode a risk-engine veto via the OVERRIDE
/// typed-confirm modal. Atomic dual-write per [`strategy_paused`].
///
/// Memo description: `"strategy:RiskVetoOverridden:<veto_id>:<reason>"`.
/// `error_code = "risk_veto_overridden"`; `error_summary` carries
/// `reason` verbatim.
///
/// `strategy_id` is bound to the strategy whose signal was vetoed;
/// `venue = NULL`. Forward-only per Q9 — the override is recorded;
/// the agent does NOT re-emit the blocked signal.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error. On failure
/// the entire transaction rolls back; neither row is left orphaned.
#[instrument(name = "ledger.risk_veto_overridden", skip(ledger))]
pub async fn risk_veto_overridden(
    ledger: &Ledger,
    veto_id: &str,
    strategy_id: &str,
    reason: &str,
    operator: &str,
) -> Result<(), LedgerError> {
    let metadata = serde_json::json!({
        "event": "RiskVetoOverridden",
        "veto_id": veto_id,
        "strategy_id": strategy_id,
        "reason": reason,
        "operator": operator,
    })
    .to_string();

    let memo_ts = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let memo_txn_id = Uuid::new_v4().to_string();
    let memo_description = format!("strategy:RiskVetoOverridden:{veto_id}:{reason}");

    let strategy_ts_fmt = time::format_description::parse(
        "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
    )
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let strategy_ts = time::OffsetDateTime::now_utc()
        .format(&strategy_ts_fmt)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let strategy_row_id = Uuid::new_v4().to_string();

    let mut db_txn = ledger
        .pool
        .begin()
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

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

    sqlx::query(
        "INSERT INTO strategy_events \
         (id, ts, kind, strategy_id, old_hash, new_hash, source_path, operator, error_code, error_summary, venue) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&strategy_row_id)
    .bind(&strategy_ts)
    .bind("RiskVetoOverridden")
    .bind(Some(strategy_id))
    .bind::<Option<&str>>(None) // old_hash
    .bind::<Option<&str>>(None) // new_hash
    .bind("") // source_path
    .bind(operator)
    .bind(Some("risk_veto_overridden"))
    .bind(Some(reason))
    .bind::<Option<&str>>(None) // venue — operator action; venue-agnostic
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
/// **T1917** — v2-llm-strategy M5: this is now a thin compatibility
/// wrapper around [`post_cost_llm`] that fills zero token counts +
/// `Uuid::nil()` for the correlation id. Existing non-LLM callers stay
/// green; the `LedgerCostSink` for `CostEvent::Llm` now goes through
/// [`post_cost_llm`] so the token-meta fields land on
/// `journal_transactions.metadata` (which feeds
/// [`crate::query::cache_hit_ratio_since`] / T1910).
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
    post_cost_llm(ledger, tier, usd, 0, 0, 0, Uuid::nil()).await
}

/// Post an LLM cost as a balanced journal entry with token metadata
/// (T1917 — v2-llm-strategy M5).
///
/// Same balanced double-entry shape as [`post_cost`] (Dr
/// `expense:llm:<tier>` / Cr `liabilities:llm_accrued`), but the
/// transaction header's `metadata` JSON column carries the four LLM-
/// event fields that [`crate::query::cache_hit_ratio_since`] reads:
///
/// ```text
/// { "tokens_in": <u64>,
///   "tokens_cached_in": <u64>,
///   "tokens_out": <u64>,
///   "correlation_id": "<uuid>" }
/// ```
///
/// Backwards-compat: the 3-arg [`post_cost`] continues to compile and
/// writes zeros into the new fields. The only production caller that
/// uses the extended signature is
/// `cost::LedgerCostSink::record(CostEvent::Llm { … })`, which pulls
/// the four fields off the event in T1917.
///
/// Zero-USD events return `Ok(())` without writing a row (matches the
/// pre-T1917 `post_cost` contract).
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(name = "ledger.post_cost_llm", skip(ledger))]
pub async fn post_cost_llm(
    ledger: &Ledger,
    tier: &str,
    usd: rust_decimal::Decimal,
    tokens_in: u64,
    tokens_out: u64,
    tokens_cached_in: u64,
    correlation_id: Uuid,
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
    let metadata = serde_json::json!({
        "tokens_in": tokens_in,
        "tokens_out": tokens_out,
        "tokens_cached_in": tokens_cached_in,
        "correlation_id": correlation_id.to_string(),
    })
    .to_string();

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
    .bind(&metadata)
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

// ── LLM budget-event memo (T1916 — v2-llm-strategy M5 / Design § Q6 + Q10) ────

/// Discriminator for an LLM budget-gate audit memo.
///
/// Emitted by `llm::BudgetedProvider` when the budget gate degrades a
/// `DeepThink` request to `QuickThink` (≥ 80 % of ceiling) or blocks an
/// LLM call outright (≥ 100 % of ceiling). The `Display` impl is the
/// canonical R11.1 tag string written to the journal-transaction
/// description so downstream readers (cockpit, reports) can grep for
/// the memo without re-parsing the metadata JSON.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetEventKind {
    /// Spend ≥ 80 % of ceiling → caller's `DeepThink` request was
    /// rewritten to `QuickThink` for this call.
    DegradeToQuickThink,
    /// Spend ≥ 100 % of ceiling → call was rejected with
    /// `LlmError::BudgetExceeded` and no HTTP request was made.
    Block,
}

impl std::fmt::Display for BudgetEventKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BudgetEventKind::DegradeToQuickThink => write!(f, "budget_degrade_to_quick_think"),
            BudgetEventKind::Block => write!(f, "budget_block"),
        }
    }
}

/// Post a $0.00 LLM budget-gate audit memo (T1916 — Design § Q6 + Q10 /
/// R11.1).
///
/// Writes ONE balanced journal transaction with a `metadata` payload
/// carrying `kind` (R11.1 tag), `spent_usd`, `ceiling_usd`, and
/// `tier`. The two-row entry pair is `Dr expense:llm:<tier> 0.00` +
/// `Cr liabilities:llm_accrued 0.00` so the reconciler invariant is
/// untouched — the row is a tagged audit breadcrumb, not a money
/// transfer.
///
/// `tier` is a `&str` rather than `cost::LlmTier` to avoid a
/// `cost → audit → cost` dependency cycle (the `cost` crate already
/// depends on `audit::journal::post_cost`). Callers stringify via
/// `LlmTier::to_string()` (`"deep_think"` / `"quick_think"` — same wire
/// form as `post_cost`'s tier parameter).
///
/// Schema note: **additive** — no migration. The memo uses the
/// existing `journal_transactions` + `journal_entries` tables; the
/// `kind` discriminator lives in the description (`"llm_budget:<tag>"`)
/// and in the metadata JSON's `"kind"` field so both grep-friendly and
/// JSON-query paths work.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(
    name = "ledger.post_llm_budget_event",
    skip(ledger),
    fields(kind = %kind, tier = %tier)
)]
pub async fn post_llm_budget_event(
    ledger: &Ledger,
    kind: BudgetEventKind,
    tier: &str,
    spent_usd: rust_decimal::Decimal,
    ceiling_usd: rust_decimal::Decimal,
) -> Result<(), LedgerError> {
    let txn_id = Uuid::new_v4().to_string();
    let ts = time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    let kind_str = kind.to_string();
    let description = format!("llm_budget:{kind_str}");
    let expense_account = format!("expense:llm:{tier}");
    let metadata = serde_json::json!({
        "kind": kind_str,
        "tier": tier,
        "spent_usd": spent_usd.to_string(),
        "ceiling_usd": ceiling_usd.to_string(),
    })
    .to_string();

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
    .bind(&metadata)
    .execute(&mut *db_txn)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    // Balanced zero-amount pair: Dr expense:llm:<tier> 0 / Cr liabilities:llm_accrued 0.
    // The reconciler invariant (Σdr == Σcr per txn) holds trivially.
    insert_entry(
        &mut db_txn,
        &txn_id,
        &ts,
        &expense_account,
        dec!(0),
        dec!(0),
    )
    .await?;
    insert_entry(
        &mut db_txn,
        &txn_id,
        &ts,
        "liabilities:llm_accrued",
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

// ── v3-llm-forecaster Wave E — llm_forecast_entries writer (T-D-N(E1)) ────────

/// Payload for [`post_llm_forecast`]. All fields come from `LlmForecast`
/// (strategy crate) after a successful `LlmForecaster::forecast()` call.
///
/// The struct uses `&str` / `rust_decimal::Decimal` to avoid pulling the
/// strategy crate into audit (circular dep). Callers stringify the relevant
/// `LlmForecast` fields before constructing this struct.
#[derive(Debug)]
pub struct LlmForecastWrite<'a> {
    /// `"llm_forecaster_v3"` (`StrategyId::0`).
    pub strategy_id: &'a str,
    /// Symbol string e.g. `"BTCUSDT"`.
    pub symbol: &'a str,
    /// UUID v4 echoed from `ForecastContext::correlation_id`.
    pub correlation_id: uuid::Uuid,
    /// 5-tier rating as `SCREAMING_SNAKE_CASE` e.g. `"BUY"`.
    pub rating: &'a str,
    /// Confidence value in `[0, 1]`.
    pub confidence: rust_decimal::Decimal,
    /// Horizon string e.g. `"one_hour"`.
    pub horizon: &'a str,
    /// Full reasoning trace text.
    pub reasoning_trace: &'a str,
    /// Lowercase 64-hex SHA-256 of `reasoning_trace`.
    pub trace_sha256: &'a str,
    /// JSON-serialized array of cited lesson card IDs e.g. `["lc_abc"]`.
    pub cited_lesson_ids_json: &'a str,
    /// Input tokens billed.
    pub tokens_in: i64,
    /// Output tokens billed.
    pub tokens_out: i64,
    /// Cache-read tokens (Anthropic `cache_read_input_tokens`).
    pub tokens_cached_in: i64,
    /// Actual cost for this call in USD.
    pub cost_usd: rust_decimal::Decimal,
    /// Name of the forecaster implementation e.g. `"llm_forecaster_impl"`.
    pub forecaster_name: &'a str,
    /// Model ID string e.g. `"claude-haiku-4-5-20251001"`.
    pub model_id: &'a str,
    /// Caller-supplied timestamp (RFC3339 microsecond precision). When `None`
    /// the writer stamps wall-clock time.
    pub ts: Option<&'a str>,
}

/// Persist one `LlmForecast` row to `llm_forecast_entries` (mig 012) and
/// fire `AuditEvent::LlmForecastEmitted` (R7.1.3 — v3-llm-forecaster Wave E).
///
/// The SQL INSERT is not wrapped in a double-entry transaction — the
/// `llm_forecast_entries` table is an event log (no balance sheet impact).
/// The corresponding billing cost was already posted via `post_cost_llm` by
/// the `BudgetedProvider` post-call reconcile step.
///
/// **Idempotent on `correlation_id`** — `INSERT OR IGNORE` guards against
/// duplicate emissions on replay-cache warm re-runs.
///
/// **Determinism note.** In backtest mode the ledger has no tick bus
/// (`tick.rs:104-107` static-branch dormant). The SQL row still lands.
/// Existing 34 anchors do not read this table → anchor-safe.
///
/// # Errors
///
/// Returns [`LedgerError::TransactionFailed`] on SQL error.
#[instrument(
    name = "ledger.post_llm_forecast",
    skip(ledger, write),
    fields(
        symbol = write.symbol,
        rating = write.rating,
        correlation_id = %write.correlation_id,
    )
)]
pub async fn post_llm_forecast(
    ledger: &Ledger,
    write: &LlmForecastWrite<'_>,
) -> Result<(), LedgerError> {
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

    let row_id = Uuid::new_v4().to_string();
    let confidence_str = write.confidence.to_string();
    let cost_usd_str = write.cost_usd.to_string();

    sqlx::query(
        "INSERT OR IGNORE INTO llm_forecast_entries \
         (id, ts, strategy_id, symbol, correlation_id, rating, confidence, \
          horizon, reasoning_trace, trace_sha256, cited_lesson_ids, \
          tokens_in, tokens_out, tokens_cached_in, cost_usd, \
          forecaster_name, model_id) \
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&row_id)
    .bind(&ts)
    .bind(write.strategy_id)
    .bind(write.symbol)
    .bind(write.correlation_id.to_string())
    .bind(write.rating)
    .bind(&confidence_str)
    .bind(write.horizon)
    .bind(write.reasoning_trace)
    .bind(write.trace_sha256)
    .bind(write.cited_lesson_ids_json)
    .bind(write.tokens_in)
    .bind(write.tokens_out)
    .bind(write.tokens_cached_in)
    .bind(&cost_usd_str)
    .bind(write.forecaster_name)
    .bind(write.model_id)
    .execute(&ledger.pool)
    .await
    .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;

    tracing::debug!(
        symbol = write.symbol,
        rating = write.rating,
        correlation_id = %write.correlation_id,
        cost_usd = %write.cost_usd,
        "llm_forecast_entry persisted"
    );

    // Fire AuditTick post-commit (R7.1.3).
    crate::tick::emit(
        ledger,
        crate::tick::AuditEvent::LlmForecastEmitted {
            symbol: write.symbol.into(),
            rating: write.rating.into(),
            confidence: confidence_str.into(),
            correlation_id: write.correlation_id,
            cost_usd: cost_usd_str.into(),
        },
    );

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

    // Post-execute tee — covers the 4 delegating writers automatically (T-D-7).
    crate::tick::emit(
        ledger,
        crate::tick::AuditEvent::StrategyEvent {
            kind: smol_str::SmolStr::new(write.kind),
            payload_json: write.error_summary.unwrap_or("").to_string(),
        },
    );
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
    // Post-execute tee (T-D-8).
    crate::tick::emit(
        ledger,
        crate::tick::AuditEvent::UptimeIntervalOpened {
            run_id: ledger
                .tick_bus
                .as_ref()
                .map_or(uuid::Uuid::nil(), |b| b.run_id),
        },
    );
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
    // SELECT started_at before UPDATE so we can compute duration_s for the tee
    // (decomp §3 row-10 note). Single-row read — no transaction needed.
    let started_at_row: Option<(String,)> =
        sqlx::query_as("SELECT started_at FROM agent_uptime WHERE boot_id = ?")
            .bind(boot_id)
            .fetch_optional(&ledger.pool)
            .await
            .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    sqlx::query("UPDATE agent_uptime SET stopped_at = ? WHERE boot_id = ?")
        .bind(&ts_str)
        .bind(boot_id)
        .execute(&ledger.pool)
        .await
        .map_err(|e| LedgerError::TransactionFailed(e.to_string()))?;
    // Post-execute tee (T-D-9). Compute duration_s from started_at, default 0.
    let duration_s = started_at_row
        .and_then(|(s,)| {
            let fmt = time::format_description::parse(
                "[year]-[month]-[day]T[hour]:[minute]:[second].[subsecond digits:6]Z",
            )
            .ok()?;
            let started = time::OffsetDateTime::parse(&s, &fmt).ok()?;
            let secs = (time::OffsetDateTime::now_utc() - started).whole_seconds();
            Some(u64::try_from(secs).unwrap_or(0))
        })
        .unwrap_or(0u64);
    crate::tick::emit(
        ledger,
        crate::tick::AuditEvent::UptimeIntervalClosed {
            run_id: ledger
                .tick_bus
                .as_ref()
                .map_or(uuid::Uuid::nil(), |b| b.run_id),
            duration_s,
        },
    );
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

// ── Phase 5 — operator-write audit writers (Q10 unit tests) ─────────────────
//
// Sibling-of-`kill_switch_tripped_test` shape (lives at
// `crates/audit/tests/kill_switch_dual_write_test.rs`) — each test asserts:
//  - balanced memo + strategy_events row landed,
//  - PascalCase `kind` value,
//  - `error_code` / `error_summary` projection per the Phase 5 Design's
//    column projection table,
//  - reconciler invariant Σ debits == Σ credits unchanged.
#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use crate::{Ledger, bootstrap};

    async fn open_ledger() -> Ledger {
        let ledger = Ledger::in_memory().await.expect("open in-memory ledger");
        bootstrap::chart_of_accounts(&ledger)
            .await
            .expect("bootstrap chart of accounts");
        ledger
    }

    // ── strategy_paused tests ───────────────────────────────────────────────

    #[tokio::test]
    async fn strategy_paused_emits_pascal_case_kind() {
        let ledger = open_ledger().await;
        strategy_paused(&ledger, "alpha", true, "operator")
            .await
            .expect("strategy_paused write");
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT kind FROM strategy_events WHERE kind = 'StrategyPaused'")
                .fetch_all(ledger.pool())
                .await
                .expect("select kind");
        assert_eq!(rows.len(), 1, "exactly one StrategyPaused row");
        assert_eq!(rows[0].0, "StrategyPaused");
    }

    #[tokio::test]
    async fn strategy_paused_balanced_memo_row() {
        let ledger = open_ledger().await;
        strategy_paused(&ledger, "alpha", true, "operator")
            .await
            .expect("strategy_paused write");
        // The memo row's single zero-amount journal entry is balanced.
        let entries: Vec<(String, String)> = sqlx::query_as(
            "SELECT je.debit_amount, je.credit_amount \
             FROM journal_entries je \
             JOIN journal_transactions jt ON je.transaction_id = jt.id \
             WHERE jt.description LIKE 'strategy:StrategyPaused:%'",
        )
        .fetch_all(ledger.pool())
        .await
        .expect("select memo entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "0");
        assert_eq!(entries[0].1, "0");
    }

    #[tokio::test]
    async fn strategy_paused_atomic_dual_write() {
        let ledger = open_ledger().await;
        strategy_paused(&ledger, "alpha", true, "operator")
            .await
            .expect("strategy_paused trip 1");
        strategy_paused(&ledger, "alpha", false, "operator")
            .await
            .expect("strategy_paused trip 2");

        let memo_count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM journal_transactions \
             WHERE description LIKE 'strategy:StrategyPaused:%'",
        )
        .fetch_one(ledger.pool())
        .await
        .expect("count memo");
        assert_eq!(memo_count.0, 2);

        let event_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM strategy_events WHERE kind = 'StrategyPaused'")
                .fetch_one(ledger.pool())
                .await
                .expect("count events");
        assert_eq!(event_count.0, 2);
    }

    #[tokio::test]
    async fn strategy_paused_resume_flips_error_summary() {
        let ledger = open_ledger().await;
        strategy_paused(&ledger, "alpha", true, "operator")
            .await
            .expect("pause");
        strategy_paused(&ledger, "alpha", false, "operator")
            .await
            .expect("resume");

        let rows: Vec<(String, String)> = sqlx::query_as(
            "SELECT error_summary, error_code FROM strategy_events \
             WHERE kind = 'StrategyPaused' ORDER BY ts",
        )
        .fetch_all(ledger.pool())
        .await
        .expect("select error_summary");
        assert_eq!(rows.len(), 2);
        // error_code is identical for both directions; error_summary
        // discriminates.
        assert_eq!(rows[0].1, "strategy_paused");
        assert_eq!(rows[1].1, "strategy_paused");
        assert_eq!(rows[0].0, "paused");
        assert_eq!(rows[1].0, "resumed");
    }

    // ── risk_veto_overridden tests ──────────────────────────────────────────

    #[tokio::test]
    async fn risk_veto_overridden_emits_pascal_case_kind() {
        let ledger = open_ledger().await;
        risk_veto_overridden(&ledger, "veto-1", "alpha", "daily_loss_cap", "operator")
            .await
            .expect("risk_veto_overridden write");
        let rows: Vec<(String,)> =
            sqlx::query_as("SELECT kind FROM strategy_events WHERE kind = 'RiskVetoOverridden'")
                .fetch_all(ledger.pool())
                .await
                .expect("select kind");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, "RiskVetoOverridden");
    }

    #[tokio::test]
    async fn risk_veto_overridden_balanced_memo_row() {
        let ledger = open_ledger().await;
        risk_veto_overridden(&ledger, "veto-1", "alpha", "daily_loss_cap", "operator")
            .await
            .expect("write");
        let entries: Vec<(String, String)> = sqlx::query_as(
            "SELECT je.debit_amount, je.credit_amount \
             FROM journal_entries je \
             JOIN journal_transactions jt ON je.transaction_id = jt.id \
             WHERE jt.description LIKE 'strategy:RiskVetoOverridden:%'",
        )
        .fetch_all(ledger.pool())
        .await
        .expect("select memo entries");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].0, "0");
        assert_eq!(entries[0].1, "0");
    }

    #[tokio::test]
    async fn risk_veto_overridden_reason_preserved_in_error_summary() {
        let ledger = open_ledger().await;
        let reason = "per_symbol_cap_BTCUSDT";
        risk_veto_overridden(&ledger, "veto-1", "alpha", reason, "operator")
            .await
            .expect("write");

        let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
            "SELECT error_summary, error_code, strategy_id FROM strategy_events \
             WHERE kind = 'RiskVetoOverridden'",
        )
        .fetch_all(ledger.pool())
        .await
        .expect("select error_summary");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].0, reason);
        assert_eq!(rows[0].1, "risk_veto_overridden");
        assert_eq!(rows[0].2.as_deref(), Some("alpha"));
    }

    // ── chart-buy-sell-emphasis v1.9 (T2014) — strategy_signals writers ────

    use rust_decimal_macros::dec as dec_lit;
    use time::OffsetDateTime;
    use trading_core::{
        Price as TPrice, Quantity as TQuantity, Signal as TSignal,
        SignalEvidence as TSignalEvidence, SignalKind as TSignalKind, StrategyId as TStrategyId,
        Symbol as TSymbol, Timestamp as TTimestamp, Venue as TVenue,
    };

    fn fixture_signal(kind: TSignalKind, symbol: &str, secs: i64) -> TSignal {
        TSignal {
            strategy_id: TStrategyId::new("sma_crossover"),
            symbol: TSymbol::new(symbol),
            ts: TTimestamp::new(OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs)),
            kind,
            evidence: TSignalEvidence::empty(),
            pair_data: None,
        }
    }

    /// T2014 V1 — `post_strategy_signal` writes one row into
    /// `strategy_signals` with all fields populated from the supplied
    /// `Signal` + writer parameters.
    #[tokio::test]
    async fn post_strategy_signal_writes_row() {
        let ledger = open_ledger().await;

        // Row count starts at zero (migration 009 ships no seed data).
        let count_before: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM strategy_signals")
            .fetch_one(ledger.pool())
            .await
            .expect("count before");
        assert_eq!(count_before.0, 0);

        let signal = fixture_signal(TSignalKind::Buy, "BTCUSDT", 1_700_000_000);
        let qty = TQuantity::new(dec_lit!(0.05)).expect("qty");
        let row_id = post_strategy_signal(
            &ledger,
            &signal,
            qty,
            None,
            TVenue::Binance,
            false,
            None,
            None,
        )
        .await
        .expect("post_strategy_signal");
        assert!(!row_id.is_empty(), "writer must return a non-empty row id");

        let count_after: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM strategy_signals")
            .fetch_one(ledger.pool())
            .await
            .expect("count after");
        assert_eq!(
            count_after.0, 1,
            "row count must go 0 → 1 after one post_strategy_signal call"
        );

        // Field-by-field validation against the writer parameters.
        #[allow(clippy::type_complexity)]
        let rows: Vec<(
            String,
            String,
            String,
            String,
            String,
            String,
            Option<String>,
            i64,
            Option<String>,
        )> = sqlx::query_as(
            "SELECT strategy_id, venue, symbol, side, intended_qty_str, ts, intended_price_str, \
                    was_clamped, clamp_reason \
             FROM strategy_signals WHERE id = ?",
        )
        .bind(row_id.as_str())
        .fetch_all(ledger.pool())
        .await
        .expect("select fields");
        assert_eq!(rows.len(), 1);
        let (
            strategy_id,
            venue,
            symbol,
            side,
            intended_qty_str,
            ts_str,
            intended_price_str,
            was_clamped,
            clamp_reason,
        ) = &rows[0];
        assert_eq!(strategy_id, "sma_crossover");
        assert_eq!(venue, "binance");
        assert_eq!(symbol, "BTCUSDT");
        assert_eq!(side, "buy");
        assert_eq!(intended_qty_str, "0.05");
        // 6-digit microsecond ts format — sub-second precision keeps
        // ORDER BY ts stable under rapid sequential writes (HF-3 gate).
        assert!(
            ts_str.contains('.') && ts_str.ends_with('Z'),
            "ts must be RFC-3339 with fractional-second component; got {ts_str}"
        );
        assert!(intended_price_str.is_none());
        assert_eq!(*was_clamped, 0);
        assert!(clamp_reason.is_none());
    }

    /// T2014 V2 — `update_signal_clamp_status` flips `was_clamped` from
    /// 0 → 1 and sets `clamp_reason` on the existing row.
    #[tokio::test]
    async fn update_signal_clamp_status_flips_field() {
        let ledger = open_ledger().await;

        let signal = fixture_signal(TSignalKind::Sell, "ETHUSDT", 1_700_000_100);
        let qty = TQuantity::new(dec_lit!(0.10)).expect("qty");
        let row_id = post_strategy_signal(
            &ledger,
            &signal,
            qty,
            None,
            TVenue::Binance,
            false,
            None,
            None,
        )
        .await
        .expect("INSERT");

        // Sanity: pre-UPDATE state is `was_clamped = 0, clamp_reason = NULL`.
        let pre: (i64, Option<String>) =
            sqlx::query_as("SELECT was_clamped, clamp_reason FROM strategy_signals WHERE id = ?")
                .bind(row_id.as_str())
                .fetch_one(ledger.pool())
                .await
                .expect("pre-UPDATE row");
        assert_eq!(pre.0, 0);
        assert!(pre.1.is_none());

        update_signal_clamp_status(&ledger, row_id.as_str(), true, Some("per_symbol_cap"))
            .await
            .expect("UPDATE");

        let post: (i64, Option<String>) =
            sqlx::query_as("SELECT was_clamped, clamp_reason FROM strategy_signals WHERE id = ?")
                .bind(row_id.as_str())
                .fetch_one(ledger.pool())
                .await
                .expect("post-UPDATE row");
        assert_eq!(post.0, 1, "was_clamped flipped 0 → 1");
        assert_eq!(post.1.as_deref(), Some("per_symbol_cap"));
    }

    /// T2014 V3 — `Hold` signals write no row (the writer returns an
    /// empty `SmolStr` and the table stays untouched). Defends against
    /// accidentally polluting the ghost layer with Hold no-ops.
    #[tokio::test]
    async fn post_strategy_signal_skips_hold_kind() {
        let ledger = open_ledger().await;

        let signal = fixture_signal(TSignalKind::Hold, "BTCUSDT", 1_700_000_000);
        let qty = TQuantity::new(dec_lit!(0)).expect("qty");
        let id = post_strategy_signal(
            &ledger,
            &signal,
            qty,
            None,
            TVenue::Binance,
            false,
            None,
            None,
        )
        .await
        .expect("post_strategy_signal Hold");
        assert!(id.is_empty(), "Hold must return an empty row id");

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM strategy_signals")
            .fetch_one(ledger.pool())
            .await
            .expect("count after Hold");
        assert_eq!(count.0, 0, "Hold must not write a row");
    }

    /// T2014 V4 — the writer accepts `intended_price = Some(price)` and
    /// stores the Decimal-as-TEXT representation (Q9 forward-compat).
    #[tokio::test]
    async fn post_strategy_signal_persists_intended_price() {
        let ledger = open_ledger().await;

        let signal = fixture_signal(TSignalKind::Buy, "BTCUSDT", 1_700_000_200);
        let qty = TQuantity::new(dec_lit!(0.01)).expect("qty");
        let price = TPrice::new(dec_lit!(45_000.5)).expect("price");
        let row_id = post_strategy_signal(
            &ledger,
            &signal,
            qty,
            Some(price),
            TVenue::Binance,
            false,
            None,
            None, // forecast_correlation_id (Phase D R1.3)
        )
        .await
        .expect("post w/ intended_price");

        let row: (Option<String>,) =
            sqlx::query_as("SELECT intended_price_str FROM strategy_signals WHERE id = ?")
                .bind(row_id.as_str())
                .fetch_one(ledger.pool())
                .await
                .expect("select intended_price_str");
        assert_eq!(row.0.as_deref(), Some("45000.5"));
    }

    // ── cockpit-training-control T-D-N8 training writer tests ─────────────────

    /// T-D-N8 V1 — `post_training_start` writes a `kind='start'` row with all
    /// expected fields and the supplied pid.
    #[tokio::test]
    async fn post_training_start_writes_row() {
        let ledger = open_ledger().await;
        let run_id = "run-0001-0000-0000-0000-000000000001";

        post_training_start(&ledger, run_id, "bs1", 42, Some(12345))
            .await
            .expect("post_training_start must succeed");

        let row: (String, String, String, Option<i64>, Option<i64>, i64) = sqlx::query_as(
            "SELECT run_id, kind, scenario, pid, epoch, seed FROM training_events WHERE run_id = ?",
        )
        .bind(run_id)
        .fetch_one(ledger.pool())
        .await
        .expect("must have one row");

        assert_eq!(row.0, run_id, "run_id matches");
        assert_eq!(row.1, "start", "kind = 'start'");
        assert_eq!(row.2, "bs1", "scenario = 'bs1'");
        assert_eq!(row.3, Some(12345), "pid captured");
        assert!(row.4.is_none(), "epoch is NULL for start rows");
        assert_eq!(row.5, 42, "seed matches");
    }

    /// T-D-N8 V2 — `post_training_epoch` writes a `kind='epoch'` row that
    /// inherits scenario/seed from the matching start row.
    #[tokio::test]
    async fn post_training_epoch_writes_row() {
        let ledger = open_ledger().await;
        let run_id = "run-0002-0000-0000-0000-000000000002";

        // Seed start row first so the subqueries resolve.
        post_training_start(&ledger, run_id, "bs2", 99, None)
            .await
            .expect("start");

        post_training_epoch(&ledger, run_id, 1, 50, 0.42_f32, 0.38_f32, 5000)
            .await
            .expect("post_training_epoch must succeed");

        let row: (
            String,
            i64,
            i64,
            Option<String>,
            Option<String>,
            i64,
            String,
        ) = sqlx::query_as(
            "SELECT kind, epoch, total_epochs, train_loss, val_loss, wall_clock_ms, scenario \
                 FROM training_events WHERE run_id = ? AND kind = 'epoch'",
        )
        .bind(run_id)
        .fetch_one(ledger.pool())
        .await
        .expect("must have epoch row");

        assert_eq!(row.0, "epoch");
        assert_eq!(row.1, 1, "epoch = 1");
        assert_eq!(row.2, 50, "total_epochs = 50");
        // TEXT fields: stored as the f32 Display representation.
        assert!(
            row.3.as_deref().unwrap_or("").starts_with("0.4"),
            "train_loss stored"
        );
        assert!(
            row.4.as_deref().unwrap_or("").starts_with("0.3"),
            "val_loss stored"
        );
        assert_eq!(row.5, 5000, "wall_clock_ms");
        assert_eq!(row.6, "bs2", "scenario inherited from start row");
    }

    /// T-D-N8 V3 — `post_training_finish` sets `model_revision` and populates
    /// `epoch` / `total_epochs` from the latest epoch row.
    #[tokio::test]
    async fn post_training_finish_sets_model_revision() {
        let ledger = open_ledger().await;
        let run_id = "run-0003-0000-0000-0000-000000000003";
        let model_rev = "abc123def456abc123def456abc123def456abc1";

        post_training_start(&ledger, run_id, "default", 7, None)
            .await
            .expect("start");
        post_training_epoch(&ledger, run_id, 5, 5, 0.10_f32, 0.11_f32, 1000)
            .await
            .expect("epoch");
        post_training_finish(&ledger, run_id, model_rev, 0.10_f32, 0.11_f32, 8000)
            .await
            .expect("post_training_finish must succeed");

        let row: (String, Option<String>, Option<i64>) = sqlx::query_as(
            "SELECT kind, model_revision, epoch FROM training_events WHERE run_id = ? AND kind = 'finish'",
        )
        .bind(run_id)
        .fetch_one(ledger.pool())
        .await
        .expect("must have finish row");

        assert_eq!(row.0, "finish");
        assert_eq!(
            row.1.as_deref(),
            Some(model_rev),
            "model_revision persisted"
        );
        assert_eq!(row.2, Some(5), "epoch from latest epoch row");
    }

    /// T-D-N8 V4 — `post_training_failed` writes `kind='failed'` with the
    /// `error_message` field populated and `epoch`/`model_revision` NULL.
    #[tokio::test]
    async fn post_training_failed_writes_error_message() {
        let ledger = open_ledger().await;
        let run_id = "run-0004-0000-0000-0000-000000000004";

        post_training_start(&ledger, run_id, "bs1", 1, Some(9999))
            .await
            .expect("start");
        post_training_failed(&ledger, run_id, "Metal device not found")
            .await
            .expect("post_training_failed must succeed");

        let row: (String, Option<String>, Option<i64>, Option<String>) = sqlx::query_as(
            "SELECT kind, error_message, epoch, model_revision \
             FROM training_events WHERE run_id = ? AND kind = 'failed'",
        )
        .bind(run_id)
        .fetch_one(ledger.pool())
        .await
        .expect("must have failed row");

        assert_eq!(row.0, "failed");
        assert_eq!(
            row.1.as_deref(),
            Some("Metal device not found"),
            "error_message"
        );
        assert!(row.2.is_none(), "epoch is NULL");
        assert!(row.3.is_none(), "model_revision is NULL");
    }
}
