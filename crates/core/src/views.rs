//! Read-side view types used by `audit::query` and the UI.
//! These are pure data transfer objects — no back-edge from `core` to `audit`.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::asset::Usdt;
use crate::fill::FeeTier;
use crate::money::{Money, Price, Quantity};
use crate::symbol::{AccountId, Side, Symbol};
use crate::time::Timestamp;

/// Read-side representation of a fill, returned by `audit::query::recent_fills`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillView {
    pub symbol: Symbol,
    pub side: Side,
    pub price: Price,
    pub qty: Quantity,
    pub fee: Money<Usdt>,
    pub fee_tier: FeeTier,
    pub venue_ts: Timestamp,
}

/// Read-side representation of a journal entry row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalEntryView {
    pub account: AccountId,
    pub amount: Decimal,
    pub ts: Timestamp,
    pub memo: String,
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
