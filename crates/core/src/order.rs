//! `Order`, `ProposedOrder`, `OrderKind`, `TimeInForce`, `RiskLimits`.
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{OrderError, RiskError};
use crate::money::{Price, Quantity};
use crate::position::Position;
use crate::symbol::{Side, StrategyId, Symbol};
use crate::time::Timestamp;

/// Unique order identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct OrderId(pub Uuid);

impl OrderId {
    #[must_use]
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for OrderId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for OrderId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Execution type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OrderKind {
    Market,
    Limit { price: Price },
}

/// Time-in-force policy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimeInForce {
    /// Good-till-cancel.
    Gtc,
    /// Immediate-or-cancel.
    Ioc,
    /// Fill-or-kill.
    Fok,
    /// Day order.
    Day,
}

/// Risk limits used to validate `Order::new`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskLimits {
    /// Maximum fraction of equity that can be in a single symbol (0..1).
    pub per_symbol_exposure_cap: Decimal,
    /// Price sanity band: reject if `|price - mark| / mark > band`.
    pub price_sanity_band: Decimal,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            per_symbol_exposure_cap: Decimal::new(40, 2), // 0.40
            price_sanity_band: Decimal::new(10, 2),       // 0.10 = 10%
        }
    }
}

/// A not-yet-validated order intent (pre-`Order::new`).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProposedOrder {
    pub symbol: Symbol,
    pub side: Side,
    pub qty: Quantity,
    pub kind: OrderKind,
    pub tif: TimeInForce,
}

/// A validated, immutable order.
/// Private fields — only constructable via `Order::new`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    id: OrderId,
    strategy_id: StrategyId,
    symbol: Symbol,
    side: Side,
    qty: Quantity,
    kind: OrderKind,
    tif: TimeInForce,
    created_at: Timestamp,
}

impl Order {
    /// Construct and validate an order.
    ///
    /// All arguments are required; the spec (R2.4) mandates this exact
    /// construction surface so position-snapshot and risk-limits are always
    /// supplied at the call site rather than carried on the builder.
    ///
    /// # Errors
    ///
    /// - [`OrderError::NonPositiveQty`] if `qty <= 0`
    /// - [`OrderError::AssetMismatch`] if `symbol != position_snapshot.symbol`
    /// - [`OrderError::PriceOutsideBand`] if a limit price is outside the
    ///   sanity band (`± risk_limits.price_sanity_band * last_mark`)
    /// - [`OrderError::Risk`] wrapping [`RiskError::ExposureCap`] if the
    ///   notional would exceed `risk_limits.per_symbol_exposure_cap * equity`
    #[allow(clippy::too_many_arguments)] // R2.4 mandates this exact surface
    pub fn new(
        strategy_id: StrategyId,
        symbol: Symbol,
        side: Side,
        qty: Quantity,
        kind: OrderKind,
        tif: TimeInForce,
        position_snapshot: &Position,
        last_mark: Price,
        risk_limits: &RiskLimits,
        current_equity: Decimal,
    ) -> Result<Self, OrderError> {
        // Reject non-positive quantity
        if qty.get() <= Decimal::ZERO {
            return Err(OrderError::NonPositiveQty(qty.get()));
        }

        // Symbol must match position snapshot if position exists
        if !position_snapshot.symbol.0.is_empty() && position_snapshot.symbol != symbol {
            return Err(OrderError::AssetMismatch {
                symbol: symbol.to_string(),
                position: position_snapshot.symbol.to_string(),
            });
        }

        // Price sanity band for limit orders
        if let OrderKind::Limit { price } = kind {
            let mark = last_mark.get();
            let band = risk_limits.price_sanity_band;
            let lo = mark * (Decimal::ONE - band);
            let hi = mark * (Decimal::ONE + band);
            let p = price.get();
            if p < lo || p > hi {
                return Err(OrderError::PriceOutsideBand { price: p, lo, hi });
            }
        }

        // Exposure cap: notional of this order / equity <= cap
        if current_equity > Decimal::ZERO {
            let notional = qty.get() * last_mark.get();
            let exposure_frac = notional / current_equity;
            if exposure_frac > risk_limits.per_symbol_exposure_cap {
                return Err(OrderError::Risk(RiskError::ExposureCap {
                    proposed: exposure_frac,
                    cap: risk_limits.per_symbol_exposure_cap,
                }));
            }
        }

        Ok(Self {
            id: OrderId::new(),
            strategy_id,
            symbol,
            side,
            qty,
            kind,
            tif,
            created_at: Timestamp::now(),
        })
    }

    #[must_use]
    pub fn id(&self) -> OrderId {
        self.id
    }

    #[must_use]
    pub fn strategy_id(&self) -> &StrategyId {
        &self.strategy_id
    }

    #[must_use]
    pub fn symbol(&self) -> &Symbol {
        &self.symbol
    }

    #[must_use]
    pub fn side(&self) -> Side {
        self.side
    }

    #[must_use]
    pub fn qty(&self) -> Quantity {
        self.qty
    }

    #[must_use]
    pub fn kind(&self) -> OrderKind {
        self.kind
    }

    #[must_use]
    pub fn tif(&self) -> TimeInForce {
        self.tif
    }

    #[must_use]
    pub fn created_at(&self) -> Timestamp {
        self.created_at
    }
}
