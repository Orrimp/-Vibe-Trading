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
