//! Read-side view types used by `audit::query` and the UI.
//! These are pure data transfer objects — no back-edge from `core` to `audit`.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::asset::Usdt;
use crate::fill::FeeTier;
use crate::money::{Money, Price, Quantity};
use crate::symbol::{AccountId, Side, StrategyId, Symbol};
use crate::time::Timestamp;
use crate::venue::Venue;

/// Read-side representation of a fill, returned by `audit::query::recent_fills`.
///
/// `transaction_id` carries the `journal_transactions.id` UUID string, used by
/// the cockpit to drive the tape-row → audit-modal click-through
/// (tape-row-audit-modal Q5).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillView {
    pub symbol: Symbol,
    pub side: Side,
    pub price: Price,
    pub qty: Quantity,
    pub fee: Money<Usdt>,
    pub fee_tier: FeeTier,
    pub venue_ts: Timestamp,
    /// `journal_transactions.id` UUID string for click-through to the audit
    /// modal. Always populated when read from the audit DB; defaults to the
    /// empty `SmolStr` for fixture/synthetic fills.
    #[serde(default)]
    pub transaction_id: SmolStr,
}

/// Read-side representation of a journal entry row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntryView {
    pub account: AccountId,
    pub amount: Decimal,
    pub ts: Timestamp,
    pub memo: String,
}

/// Un-collapsed read-side representation of a journal entry row, used by the
/// tape-row → audit-modal feature (tape-row-audit-modal Q2). Where
/// [`JournalEntryView`] collapses the `(debit, credit)` pair into a signed
/// `amount`, this view preserves both columns so the modal can render a
/// 4-column `Account | Debit | Credit | Currency` table without losing the
/// "exact zero" cells that signed-amount rendering would erase.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JournalEntry {
    pub account: AccountId,
    /// Zero when this row is a credit.
    pub debit: Money<Usdt>,
    /// Zero when this row is a debit.
    pub credit: Money<Usdt>,
    /// Display ticker — `"USDT"`, `"BTC"`, etc.
    pub currency: SmolStr,
    pub ts: Timestamp,
    pub memo: SmolStr,
}

/// Read-side header for a journal-transaction row, returned by
/// `audit::query::journal_transaction_metadata`. Composed with
/// `Vec<JournalEntry>` at the `cockpit_live` `Task::perform` site to populate
/// `ui::state::JournalTransactionView`.
///
/// `description` is `SmolStr` — typical paper-fill descriptions
/// (`"buy 0.04 BTCUSDT @ 50000"`) fit in inline storage; LLM-cost and
/// registry-event descriptions spill to heap on the slow path at no extra cost
/// vs `String`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JournalTransactionMetadata {
    /// `journal_transactions.id` UUID string.
    pub transaction_id: SmolStr,
    /// Transaction timestamp (microsecond precision).
    pub ts: Timestamp,
    /// Free-form description (e.g. `"buy 0.04 BTCUSDT @ 50000"`).
    /// Empty `SmolStr` for legacy rows without a description.
    pub description: SmolStr,
    /// Attribution to the strategy that emitted the signal.
    /// `None` for pre-T802 rows or non-strategy transactions.
    pub strategy_id: Option<StrategyId>,
}

/// P&L snapshot as reported by `audit::query::*`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PnlSnapshot {
    pub cash: Money<Usdt>,
    pub unrealized: Money<Usdt>,
    pub realized: Money<Usdt>,
    pub total_equity: Money<Usdt>,
    pub daily_return: Money<Usdt>,
    /// **Wallclock** publish time (`Timestamp::now()` on the live path).
    /// Drives freshness / latency and the UI equity-buffer's out-of-order
    /// **delivery** guard — it is monotone by construction (a clock never goes
    /// back). Do NOT stamp this with bar/data time: a wallclock-anchored buffer
    /// relies on `as_of` being monotone, and stamping it with replay bar-time
    /// (2023) broke the live curve (reverted I1, 2026-06-11). The historical
    /// **data** time the chart plots on its x-axis lives in [`Self::bar_ts`].
    pub as_of: Timestamp,
    /// **Data / bar** time — the close timestamp of the bar this snapshot was
    /// computed at (`bar.close_ts` on the research-replay path). This is the
    /// x-axis coordinate the live equity curve plots, kept SEPARATE from
    /// [`Self::as_of`] so the chart shows meaningful historical dates during a
    /// fast replay (where every `as_of` is the same wallclock minute) while the
    /// delivery/freshness logic still rides `as_of`
    /// (cockpit-live-equity-render-guard, 2026-06-11, approach A).
    ///
    /// `None` for snapshots from a source that has no bar context (legacy
    /// rows, the reconciler's non-bar paths) — consumers fall back to `as_of`,
    /// so the curve degrades to the prior wallclock behavior rather than
    /// dropping points. `#[serde(default)]` keeps older serialized snapshots
    /// (without this field) deserializing cleanly.
    #[serde(default)]
    pub bar_ts: Option<Timestamp>,
}

/// Position as seen by the cockpit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PositionView {
    pub symbol: Symbol,
    pub base_qty: Decimal,
    pub cost_basis: Money<Usdt>,
    pub last_mark: Price,
    pub pnl: Money<Usdt>,
    pub pnl_pct: Decimal,
    /// `position_notional / equity`.
    pub exposure_pct: Decimal,
}

/// Phase 3 (Lumen detail screens) — discriminator for the audit-screen
/// table's `kind` column. Rendered as a label, not an icon
/// (operator-locked Constraint 3 — Lucide deferred).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditKindLabel {
    Fill,
    StrategyEvent,
    Reconciliation,
}

/// Phase 3 (Lumen detail screens) — single-select kind discriminator
/// for the Audit-screen filter row. `All` matches every row; the other
/// variants narrow the SQL `WHERE` predicate inside
/// `audit::query::recent_journal_filtered` (Q7 — sibling, not extension).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AuditKindFilter {
    #[default]
    All,
    Fill,
    StrategyEvent,
    Reconciliation,
}

/// Phase 3 (Lumen detail screens) — newest-first row projection for the
/// Audit / Journal screen table. Returned by
/// `audit::query::recent_journal_filtered` and consumed verbatim by the
/// cockpit's `screens::audit::view` body.
///
/// `tx_id` carries the `journal_transactions.id` UUID string for the
/// row-click → modal-open trigger (T1711); `kind` discriminates the
/// table-row label without surfacing icons (Constraint 3).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalRow {
    pub tx_id: SmolStr,
    pub ts: Timestamp,
    pub venue: Venue,
    pub symbol: Option<Symbol>,
    pub kind: AuditKindLabel,
    pub description: SmolStr,
    pub strategy_id: Option<StrategyId>,
}

/// chart-buy-sell-emphasis v1.9 (T2012, Q9) — read-side representation of a
/// strategy signal row written by `audit::journal::post_strategy_signal` and
/// returned by `audit::query::recent_signals`.
///
/// Sibling of [`FillView`]. The cockpit's chart canvas paints one ghost-
/// triangle marker per `SignalView` (with `was_clamped` toggling a visual
/// hint). The `signal_id` carries the `strategy_signals.id` UUID string for
/// the row-click → tooltip / modal trigger (M2 / M3 — UI track).
///
/// `intended_qty` carries the **strategy-proposed** quantity at signal-emit
/// time — distinct from the **executed** quantity surfaced by `FillView`.
/// When a signal is clamped, the executed fill (if any) will carry a
/// reduced `qty`; the ghost marker preserves the original intent so the
/// operator can see "what the strategy asked for vs what the risk engine
/// allowed."
///
/// `was_clamped == false` + `clamp_reason == None` is the steady state for
/// signals that pass the risk engine untouched. `was_clamped == true` is
/// set by `audit::journal::update_signal_clamp_status` after the risk
/// engine returns its decision; `clamp_reason` carries a short
/// human-readable tag (e.g. `"per_symbol_cap"`, `"daily_loss_cap"`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignalView {
    /// `strategy_signals.id` UUID string. Stable identifier for the
    /// click-through → tooltip / modal (M2 / M3).
    pub signal_id: SmolStr,
    pub symbol: Symbol,
    pub side: Side,
    pub intended_qty: Quantity,
    pub signal_ts: Timestamp,
    pub strategy_id: StrategyId,
    /// `true` once the risk engine has decided to clamp this signal.
    /// `false` for signals that passed through untouched OR for which
    /// the risk-decision row has not yet been `UPDATE`d.
    pub was_clamped: bool,
    /// Short human-readable reason set alongside `was_clamped = true`.
    /// `None` when `was_clamped = false`.
    pub clamp_reason: Option<SmolStr>,
}

// ── cockpit-training-control (T-D-N8 / T-D-N9) ───────────────────────────────

/// A single row from the `training_events` table (R4.2).
///
/// All optional fields map to SQL `NULL` when absent. `train_loss` /
/// `val_loss` are stored as TEXT in the DB (Decimal-as-TEXT contract per
/// ADR-0003) and parsed back to `f32` at read time — lossless for the
/// observability / plot surface they feed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrainingEventRow {
    /// UUID v4 primary-key of this row.
    pub id: SmolStr,
    /// RFC3339 with 6-digit microsecond precision (ADR-0004).
    pub ts: SmolStr,
    /// UUID v4 that groups all events from one `train_tcn` invocation.
    pub run_id: SmolStr,
    /// `'start' | 'epoch' | 'finish' | 'failed'`.
    pub kind: SmolStr,
    /// `None` for `kind = 'start' | 'failed'`.
    pub epoch: Option<i64>,
    /// `None` for `kind = 'start' | 'failed'`.
    pub total_epochs: Option<i64>,
    /// Training loss at this epoch (parsed from TEXT). `None` for start/failed.
    pub train_loss: Option<f32>,
    /// Validation loss at this epoch. `None` for start/failed.
    pub val_loss: Option<f32>,
    /// Wall-clock milliseconds for this epoch / full run. `None` for start/failed.
    pub wall_clock_ms: Option<i64>,
    /// Canonical SHA from `CheckpointMetadata.model_revision`. `None` except on `kind='finish'`.
    pub model_revision: Option<SmolStr>,
    /// Scenario label (`'bs1' | 'bs2' | 'default' | operator label`).
    pub scenario: SmolStr,
    /// Training seed from `train_tcn.toml`.
    pub seed: i64,
    /// OS PID captured at `kind='start'` for orphan-detect. `None` on non-start rows.
    pub pid: Option<i64>,
    /// Error message on `kind='failed'`. `None` on all other kinds.
    pub error_message: Option<SmolStr>,
}

/// Convenience summary for a single training run (the last row group), used by
/// the panel status strip (`query::latest_training_run`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TrainingRunSummary {
    /// UUID v4 run identifier.
    pub run_id: SmolStr,
    /// Scenario label.
    pub scenario: SmolStr,
    /// Training seed.
    pub seed: i64,
    /// Start timestamp (RFC3339 microsecond).
    pub started_at: SmolStr,
    /// Latest epoch seen so far (0 if only the start row has been written).
    pub latest_epoch: i64,
    /// Total epochs from the start row (0 if not yet known).
    pub total_epochs: i64,
    /// Most recent training loss (from the last epoch row). `None` before first epoch.
    pub latest_train_loss: Option<f32>,
    /// Most recent validation loss. `None` before first epoch.
    pub latest_val_loss: Option<f32>,
    /// `model_revision` from the finish row. `None` if not yet finished.
    pub model_revision: Option<SmolStr>,
    /// Terminal status: `'running' | 'done' | 'failed' | 'cancelled'`.
    /// Derived at read time — not stored directly.
    pub status: SmolStr,
    /// Error message if `status == 'failed'`.
    pub error_message: Option<SmolStr>,
    /// OS PID from the start row (for orphan-detect).
    pub pid: Option<i64>,
}

/// A training run whose `kind='start'` row has no matching `kind='finish'` or
/// `kind='failed'` row within `fresh_window`, and whose start row carries a PID
/// that the caller can check for liveness (`query::orphan_training_runs`).
///
/// Used by the cockpit boot-time orphan-detect path (ADR-0034 § D7).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OrphanTrainingRun {
    /// UUID v4 run identifier.
    pub run_id: SmolStr,
    /// Scenario label.
    pub scenario: SmolStr,
    /// Start timestamp (RFC3339 microsecond).
    pub started_at: SmolStr,
    /// OS PID from the start row (may be `None` for older rows without `pid`).
    pub pid: Option<i64>,
}

// ── Tests ─────────────────────────────────────────────────────────────────────
#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;

    /// T2012 — `SignalView` round-trips through JSON serde without losing any
    /// field (including the `Option<SmolStr>` `clamp_reason`).
    #[test]
    fn signal_view_serde_roundtrip() {
        let view = SignalView {
            signal_id: SmolStr::new("a1b2c3d4-0000-0000-0000-000000000001"),
            symbol: Symbol::new("BTCUSDT"),
            side: Side::Buy,
            intended_qty: Quantity::new(dec!(0.05)).unwrap(),
            signal_ts: Timestamp::new(
                OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(1_700_000_000),
            ),
            strategy_id: StrategyId::new("sma_crossover"),
            was_clamped: true,
            clamp_reason: Some(SmolStr::new("per_symbol_cap")),
        };

        let json = serde_json::to_string(&view).expect("serialize SignalView");
        let back: SignalView = serde_json::from_str(&json).expect("deserialize SignalView");
        assert_eq!(view, back, "SignalView must round-trip through JSON");

        // None-variant of clamp_reason round-trips correctly (was previously
        // a regression vector when the field was renamed).
        let unclamped = SignalView {
            was_clamped: false,
            clamp_reason: None,
            ..view
        };
        let json2 = serde_json::to_string(&unclamped).expect("serialize unclamped");
        let back2: SignalView = serde_json::from_str(&json2).expect("deserialize unclamped");
        assert_eq!(unclamped, back2);
        assert!(back2.clamp_reason.is_none());
    }
}
