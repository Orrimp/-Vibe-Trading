//! Position sizing + pre-trade validation (T23).
//!
//! `size_and_validate` is the single entry point: it computes a fixed-fraction
//! quantity, clamps to the per-symbol exposure cap, and builds an `Order` via
//! `Order::new` — so all type-level invariants are enforced atomically.
use rust_decimal::Decimal;
use trading_core::{
    Money, Order, OrderKind, Position, Price, Quantity, RiskError, RiskLimits, Side, StrategyId,
    Symbol, TimeInForce, Usdt,
};

/// Fixed-fraction position sizer.
pub struct FixedFractionSizer {
    /// Fraction of equity to size (e.g. 0.10 for 10%).
    pub fraction: Decimal,
}

impl FixedFractionSizer {
    /// Create a new sizer with the given fraction.
    #[must_use]
    pub fn new(fraction: Decimal) -> Self {
        Self { fraction }
    }

    /// Compute the quantity at the given price, clamped to the exposure cap.
    ///
    /// # Errors
    ///
    /// - [`SizingError::ZeroEquity`] if `equity <= 0`.
    /// - [`SizingError::ZeroPrice`] if `price <= 0`.
    /// - [`SizingError::NegativeQty`] if the computed quantity is negative.
    pub fn compute_qty(
        &self,
        equity: Money<Usdt>,
        price: Decimal,
        risk_limits: &RiskLimits,
    ) -> Result<Quantity, SizingError> {
        if equity.amount() <= Decimal::ZERO {
            return Err(SizingError::ZeroEquity);
        }
        if price <= Decimal::ZERO {
            return Err(SizingError::ZeroPrice);
        }

        let notional = equity.amount() * self.fraction;
        let mut qty = notional / price;

        // Clamp to exposure cap: qty * price <= cap * equity
        let max_notional = equity.amount() * risk_limits.per_symbol_exposure_cap;
        let max_qty = max_notional / price;
        if qty > max_qty {
            qty = max_qty;
        }

        Quantity::new(qty).map_err(|_| SizingError::NegativeQty)
    }
}

/// Error from the sizing engine.
#[derive(Debug, thiserror::Error)]
pub enum SizingError {
    #[error("equity is zero")]
    ZeroEquity,
    #[error("price is zero or negative")]
    ZeroPrice,
    #[error("computed negative quantity")]
    NegativeQty,
    #[error("exposure cap breach: proposed {proposed}, cap {cap}")]
    ExposureCap { proposed: Decimal, cap: Decimal },
}

/// Compute size and build a validated `Order` (R4.5 / T23 acceptance criterion).
///
/// Steps:
/// 1. Compute qty via fixed-fraction sizing, clamped to exposure cap.
/// 2. Call `Order::new` with full `RiskLimits` + position snapshot.
///
/// # Errors
///
/// - [`SizingError`] variants on sizing failure.
/// - `trading_core::OrderError` (wrapped as `SizingError::ExposureCap`) on
///   `Order::new` exposure-cap rejection.
#[allow(clippy::too_many_arguments)]
pub fn size_and_validate(
    sizer: &FixedFractionSizer,
    strategy_id: StrategyId,
    symbol: Symbol,
    side: Side,
    equity: Money<Usdt>,
    mark_price: Price,
    position_snapshot: &Position,
    risk_limits: &RiskLimits,
) -> Result<Order, SizingError> {
    let price_d = mark_price.get();
    let qty = sizer.compute_qty(equity, price_d, risk_limits)?;

    Order::new(
        strategy_id,
        symbol,
        side,
        qty,
        OrderKind::Market,
        TimeInForce::Ioc,
        position_snapshot,
        mark_price,
        risk_limits,
        equity.amount(),
    )
    .map_err(|e| match e {
        trading_core::OrderError::Risk(RiskError::ExposureCap { proposed, cap }) => {
            SizingError::ExposureCap { proposed, cap }
        }
        _other => SizingError::NegativeQty, // rare fallthrough — shouldn't happen
    })
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use trading_core::{Money, RiskLimits, Symbol};

    fn default_limits() -> RiskLimits {
        RiskLimits {
            per_symbol_exposure_cap: dec!(0.40), // 40%
            price_sanity_band: dec!(0.10),
            portfolio_exposure_cap: None,
        }
    }

    fn empty_position() -> Position {
        Position::empty(Symbol::new(""))
    }

    #[test]
    fn t23_basic_sizing() {
        // equity=100_000, fraction=0.1, price=40_000 → qty=0.25 BTC
        let sizer = FixedFractionSizer::new(dec!(0.1));
        let equity: Money<Usdt> = Money::from_decimal(dec!(100_000));
        let limits = default_limits();
        let qty = sizer.compute_qty(equity, dec!(40_000), &limits).unwrap();
        assert_eq!(qty.get(), dec!(0.25), "expected 0.25 BTC");
    }

    #[test]
    fn t23_exposure_cap_clamps_qty() {
        // equity=100_000, fraction=0.5, cap=0.4, price=40_000
        // fraction would give 1.25 BTC (notional=50_000), cap gives max 1.0 BTC (notional=40_000)
        let sizer = FixedFractionSizer::new(dec!(0.5));
        let equity: Money<Usdt> = Money::from_decimal(dec!(100_000));
        let limits = default_limits();
        let qty = sizer.compute_qty(equity, dec!(40_000), &limits).unwrap();
        // 100_000 * 0.40 / 40_000 = 1.0 BTC
        assert_eq!(qty.get(), dec!(1.0));
    }

    #[test]
    fn t23_zero_equity_error() {
        let sizer = FixedFractionSizer::new(dec!(0.1));
        let equity: Money<Usdt> = Money::from_decimal(dec!(0));
        let err = sizer
            .compute_qty(equity, dec!(40_000), &default_limits())
            .unwrap_err();
        assert!(matches!(err, SizingError::ZeroEquity));
    }

    #[test]
    fn t23_zero_price_error() {
        let sizer = FixedFractionSizer::new(dec!(0.1));
        let equity: Money<Usdt> = Money::from_decimal(dec!(100_000));
        let err = sizer
            .compute_qty(equity, dec!(0), &default_limits())
            .unwrap_err();
        assert!(matches!(err, SizingError::ZeroPrice));
    }

    #[test]
    fn t23_size_and_validate_happy_path() {
        let sizer = FixedFractionSizer::new(dec!(0.1));
        let equity: Money<Usdt> = Money::from_decimal(dec!(100_000));
        let mark = Price::new(dec!(40_000)).unwrap();
        let result = size_and_validate(
            &sizer,
            trading_core::StrategyId::new("test"),
            Symbol::new("BTCUSDT"),
            Side::Buy,
            equity,
            mark,
            &empty_position(),
            &default_limits(),
        );
        let order = result.unwrap();
        assert_eq!(order.qty().get(), dec!(0.25));
    }

    #[test]
    fn t23_exposure_cap_error_from_order_new() {
        // qty = 2.0 BTC, notional = 80_000 > 40% of 100_000 = 40_000
        let equity: Money<Usdt> = Money::from_decimal(dec!(100_000));
        let mark = Price::new(dec!(40_000)).unwrap();
        let qty = Quantity::new(dec!(2.0)).unwrap();
        let limits = default_limits();
        let result = Order::new(
            trading_core::StrategyId::new("test"),
            Symbol::new("BTCUSDT"),
            Side::Buy,
            qty,
            OrderKind::Market,
            TimeInForce::Ioc,
            &empty_position(),
            mark,
            &limits,
            equity.amount(),
        );
        assert!(
            matches!(
                result,
                Err(trading_core::OrderError::Risk(
                    RiskError::ExposureCap { .. }
                ))
            ),
            "expected ExposureCap error"
        );
    }
}
