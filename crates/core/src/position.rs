//! Position — current holdings for a symbol.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::asset::Usdt;
use crate::money::{Money, Price};
use crate::symbol::Symbol;

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
