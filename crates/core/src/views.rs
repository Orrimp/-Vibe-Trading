//! Read-side view types used by `audit::query` and the UI.
//! These are pure data transfer objects — no back-edge from `core` to `audit`.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;

use crate::asset::Usdt;
use crate::fill::FeeTier;
use crate::money::{Money, Price, Quantity};
use crate::symbol::{AccountId, Side, Symbol};
use crate::time::Timestamp;

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
