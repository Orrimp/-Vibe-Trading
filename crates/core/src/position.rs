//! Position — current holdings for a symbol.
//!
//! Two struct shapes coexist here:
//!
//! - [`Position`] — the v0 mark-to-market view of a single symbol, carrying
//!   `last_mark` / `realized_pnl` / `unrealized_pnl` snapshots used by the
//!   live agent's in-memory state.
//! - [`OpenPosition`] — the v1+ typed open-position projection emitted by
//!   `audit::query::open_positions_at` (T1002) and consumed by the
//!   `crates/reports` orchestrator (T1003) and the cockpit positions widget.
//!   Mark-source-agnostic by design: no `unrealized_pnl` field — that is a
//!   function of `MarkSource` and belongs in the orchestrator (per
//!   `spec/features/real-mtm-unrealized-pnl.md` Design § Q2).
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::asset::Usdt;
use crate::money::{Money, Price};
use crate::symbol::{StrategyId, Symbol};
use crate::time::Timestamp;

/// Current position in a single symbol.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Position {
    pub symbol: Symbol,
    /// Net quantity, signed: positive = long, negative = short.
    /// v0 is spot-only so this is always ≥ 0.
    pub base_qty: Decimal,
    /// Weighted-average cost basis.
    pub cost_basis: Money<Usdt>,
    pub last_mark: Price,
    pub realized_pnl: Money<Usdt>,
    pub unrealized_pnl: Money<Usdt>,
}

impl Position {
    /// Create an empty position for the given symbol.
    #[must_use]
    pub fn empty(symbol: Symbol) -> Self {
        Self {
            symbol,
            base_qty: Decimal::ZERO,
            cost_basis: Money::zero(),
            last_mark: Price::new(Decimal::ONE).unwrap_or_else(|_| {
                // SAFETY: 1 is strictly positive — this branch is unreachable.
                unreachable!("Price::new(1) is always valid")
            }),
            realized_pnl: Money::zero(),
            unrealized_pnl: Money::zero(),
        }
    }

    /// True if there is no open position.
    #[must_use]
    pub fn is_flat(&self) -> bool {
        self.base_qty == Decimal::ZERO
    }

    /// Mark-to-market equity of this position in USDT.
    #[must_use]
    pub fn mark_to_market(&self) -> Money<Usdt> {
        Money::from_decimal(self.base_qty * self.last_mark.get())
    }
}

/// Typed open-position row produced by `audit::query::open_positions_at`
/// (T1002) and consumed by the `crates/reports` orchestrator (T1003) and the
/// `crates/ui` positions widget.
///
/// Long-only at v1+ (Q8): `qty > 0` is invariant; net-negative qty raises
/// `LedgerError::Database` at the reader (per
/// `spec/features/real-mtm-unrealized-pnl.md` Design § Q8) and never
/// materializes as an `OpenPosition`.
///
/// `avg_cost_basis` is **per-unit** (USDT per unit of `symbol`), NOT
/// notional. The orchestrator computes notional contribution as
/// `qty * avg_cost_basis` at mark time. Cost basis is weighted-average
/// across all open Buy fills with proportional release on Sells (Q7).
///
/// No `unrealized_pnl` field — by design (Q2): that is a function of
/// `MarkSource` and belongs in the orchestrator, not the reader (the
/// reader has no business reaching into parquet).
///
/// Derives `Debug, Clone, PartialEq, Eq` only — no `serde::Serialize` at
/// v1+ (not on the wire / not in front-matter). Add additively if a future
/// cockpit-bus consumer needs it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenPosition {
    /// Trading symbol, e.g. `BTCUSDT`.
    pub symbol: Symbol,
    /// Open quantity. Long-only invariant: `qty > 0`.
    pub qty: Decimal,
    /// Per-unit cost basis (USDT per unit of `symbol`), weighted across all
    /// un-closed Buy fills in the open lot. NOT notional — multiply by `qty`
    /// to get the notional cost.
    pub avg_cost_basis: Money<Usdt>,
    /// Timestamp of the first un-closed Buy fill in the open lot.
    pub opened_at: Timestamp,
    /// Strategy id (T802 column on `journal_transactions`). `None` for rows
    /// written before the T802 migration.
    pub strategy_id: Option<StrategyId>,
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use time::OffsetDateTime;

    fn dummy_ts() -> Timestamp {
        Timestamp::new(OffsetDateTime::UNIX_EPOCH)
    }

    fn sample_open_position() -> OpenPosition {
        OpenPosition {
            symbol: Symbol::new("BTCUSDT"),
            qty: dec!(0.01),
            avg_cost_basis: Money::from_decimal(dec!(60_000)),
            opened_at: dummy_ts(),
            strategy_id: Some(StrategyId::new("strat_alpha")),
        }
    }

    #[test]
    fn t1001_open_position_partialeq_round_trip() {
        // Construct → clone → assert structural equality. Mirrors the
        // round-trip pattern in `strategy_events::tests` but uses
        // `Clone + PartialEq` instead of serde because OpenPosition is
        // intentionally NOT on the wire at v1+ (per task T1001 acceptance).
        let pos = sample_open_position();
        let copy = pos.clone();
        assert_eq!(pos, copy);
        assert_eq!(pos.symbol, Symbol::new("BTCUSDT"));
        assert_eq!(pos.qty, dec!(0.01));
        assert_eq!(pos.avg_cost_basis, Money::from_decimal(dec!(60_000)));
        assert_eq!(pos.opened_at, dummy_ts());
        assert_eq!(pos.strategy_id, Some(StrategyId::new("strat_alpha")));
    }

    #[test]
    fn t1001_open_position_partialeq_distinguishes_strategy_id() {
        let a = sample_open_position();
        let mut b = sample_open_position();
        b.strategy_id = Some(StrategyId::new("strat_beta"));
        assert_ne!(a, b);

        let mut c = sample_open_position();
        c.strategy_id = None;
        assert_ne!(a, c);
    }

    #[test]
    fn t1001_open_position_partialeq_distinguishes_qty_and_cost_basis() {
        let a = sample_open_position();

        let mut b = sample_open_position();
        b.qty = dec!(0.02);
        assert_ne!(a, b);

        let mut c = sample_open_position();
        c.avg_cost_basis = Money::from_decimal(dec!(60_001));
        assert_ne!(a, c);
    }
}
