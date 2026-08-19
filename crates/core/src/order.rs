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
    /// v1 — optional portfolio-level exposure cap: sum of all long notionals
    /// as a fraction of equity.  `None` = no portfolio cap (v0 backward-compat).
    /// When `Some(cap)`, `risk::size_portfolio_target` enforces the cap
    /// atomically across the entire rebalance vector.
    ///
    /// ⚠️ **BUT THAT ENFORCER HAS NO PRODUCTION CALLER (bug-log #69).** Census of
    /// `size_portfolio_target(` across the workspace: its definition, its own unit
    /// tests, and three sites in `agent/tests/v1_rebalance_reject.rs`. Zero in
    /// production — `montecarlo.rs` 0, `param_robustness_sweep.rs` 0. The sweep
    /// scenarios set `Some(dec!(0.50))`, that number is printed into hashed report
    /// bodies, and no shipped code path ever compares against it.
    ///
    /// Note the shape: a passing test proves the enforcer *works*, which is not the
    /// same as proving the limit *binds*. `Order::new` below caps PER-SYMBOL
    /// exposure (see the #71 block) and never reads this field.
    ///
    /// Enforce-or-delete is story 1-25 AC3. Until then, treat a `Some(..)` here as
    /// intent, not protection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub portfolio_exposure_cap: Option<Decimal>,
}

impl Default for RiskLimits {
    fn default() -> Self {
        Self {
            per_symbol_exposure_cap: Decimal::new(40, 2), // 0.40
            price_sanity_band: Decimal::new(10, 2),       // 0.10 = 10%
            portfolio_exposure_cap: None,                 // v0 compat — no portfolio cap
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

        // ── Exposure cap on the RESULTING exposure, side-aware (bug-log #71) ──
        //
        // This used to cap the ORDER'S OWN notional and consult neither `side` nor
        // `position_snapshot`:
        //     let notional = qty.get() * last_mark.get();
        //     if notional / equity > cap { reject }
        //
        // So a Sell that CLOSES a position — driving exposure toward zero — was
        // rejected exactly as if it opened that much, silently (no else arm, no
        // warn, no counter). The strategy recorded the close, the engine kept the
        // position, and every later decision ran off a false flat. The mirror case
        // bites shorts: a rising short's buy-to-cover is sized at the full short
        // notional, so it could exceed the cap and be refused, leaving forced
        // liquidation as the only exit (a mechanistic candidate for the 97.8-100%
        // p95 MaxDD and the 86-328 liquidation counts on the MN surfaces).
        //
        // The cap is a limit on HOW MUCH EXPOSURE YOU END UP WITH, so it is
        // evaluated on the resulting signed position:
        //     resulting = base_qty + (Buy ? +qty : -qty)
        //     |resulting| * mark / equity <= cap
        // which permits closes and covers (resulting -> 0) while still blocking
        // genuine over-exposure in EITHER direction, including opening a large
        // short via Sell — which the old check could not see at all.
        //
        // Anchor note: when `position_snapshot` is the empty placeholder
        // (`Position::empty(Symbol::new(""))`, as most unit tests and every
        // no-position caller pass), `base_qty` is 0 and this reduces to
        // `|±qty| * mark / equity` — byte-identical to the previous arithmetic.
        // Behaviour changes ONLY where a real position is supplied, which is
        // exactly the #71 case.
        if current_equity > Decimal::ZERO {
            let signed_delta = match side {
                Side::Buy => qty.get(),
                Side::Sell => -qty.get(),
            };
            let resulting_qty = position_snapshot.base_qty + signed_delta;
            let exposure_frac = resulting_qty.abs() * last_mark.get() / current_equity;
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
