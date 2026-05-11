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
    pub as_of: Timestamp,
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
    /// the risk-decision row has not yet been UPDATEed.
    pub was_clamped: bool,
    /// Short human-readable reason set alongside `was_clamped = true`.
    /// `None` when `was_clamped = false`.
    pub clamp_reason: Option<SmolStr>,
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
