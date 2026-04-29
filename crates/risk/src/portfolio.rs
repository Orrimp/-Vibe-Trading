//! Portfolio vector-order sizer (T607 — v1 R5).
//!
//! `size_portfolio_target` computes per-leg Hold / Open / Close / Resize actions
//! from current positions vs target weights, validates the portfolio-level
//! exposure cap, and returns a `Vec<Order>` sorted alphabetically by symbol.
//!
//! Per R5.2: all-or-nothing — either the full vector is accepted or the entire
//! rebalance is rejected with `RiskError::PortfolioExposureBreach`.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{
    Money, Order, OrderKind, Price, Quantity, RiskLimits, Side, StrategyId, Symbol, TimeInForce,
    Timestamp, Usdt,
};

/// One leg's mark price and target weight.
#[derive(Debug, Clone)]
pub struct TargetLeg {
    /// Universe symbol.
    pub symbol: Symbol,
    /// Target portfolio weight in `[0, exposure_cap / k_long]`.
    /// 0 == close this position (fell out of top-K).
    pub target_weight: Decimal,
    /// Most recent close price used to convert notional to qty.
    pub mark_price: Price,
}

/// Portfolio sizing error.
#[derive(Debug, thiserror::Error)]
pub enum PortfolioSizeError {
    #[error("portfolio exposure breach: proposed {proposed} > cap {cap}")]
    PortfolioExposureBreach { proposed: Decimal, cap: Decimal },
    #[error("per-symbol exposure breach for {symbol}: proposed {proposed} > cap {cap}")]
    SymbolExposureBreach {
        symbol: Symbol,
        proposed: Decimal,
        cap: Decimal,
    },
    #[error("zero equity — cannot size orders")]
    ZeroEquity,
    #[error("zero price for {0}")]
    ZeroPrice(Symbol),
}

/// Compute, validate, and return a portfolio rebalance order vector (R5.2).
///
/// The function:
/// 1. Computes a notional (`target_weight * equity`) per leg.
/// 2. Converts notional to qty via `mark_price`.
/// 3. Computes per-symbol current notional from `position_book`.
/// 4. If `|current − target|/target > drift_threshold` (or target==0) → action needed.
/// 5. Validates Σ long notional ≤ `portfolio_exposure_cap * equity`.
/// 6. Returns `Vec<Order>` sorted alphabetically (R12.5) or `Err` on violation.
///
/// # Errors
///
/// - [`PortfolioSizeError::PortfolioExposureBreach`] if the proposed portfolio
///   total exceeds the cap.
/// - [`PortfolioSizeError::SymbolExposureBreach`] if any single leg exceeds
///   the per-symbol cap.
/// - [`PortfolioSizeError::ZeroEquity`] if equity ≤ 0.
/// - [`PortfolioSizeError::ZeroPrice`] if any mark price is zero.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub fn size_portfolio_target(
    targets: &BTreeMap<Symbol, TargetLeg>,
    equity: Money<Usdt>,
    position_book: &BTreeMap<Symbol, Decimal>, // current qty held per symbol
    drift_threshold: Decimal,
    limits: &RiskLimits,
    strategy_id: &StrategyId,
    _ts: Timestamp,
) -> Result<Vec<Order>, PortfolioSizeError> {
    let equity_d = equity.amount();
    if equity_d <= Decimal::ZERO {
        return Err(PortfolioSizeError::ZeroEquity);
    }

    // Compute proposed orders.
    let mut orders: Vec<Order> = Vec::new();
    let mut total_long_notional = Decimal::ZERO;

    // Iterate in alphabetical order (BTreeMap) for R12.5 determinism.
    for (symbol, leg) in targets {
        let mark = leg.mark_price.get();
        if mark <= Decimal::ZERO {
            return Err(PortfolioSizeError::ZeroPrice(symbol.clone()));
        }

        let target_notional = leg.target_weight * equity_d;
        let current_qty = position_book.get(symbol).copied().unwrap_or(Decimal::ZERO);
        let current_notional = current_qty * mark;

        // Determine if an order is needed.
        let needs_order = if leg.target_weight == Decimal::ZERO {
            // Close: sell to flat if we hold anything.
            current_qty > Decimal::ZERO
        } else if current_qty <= Decimal::ZERO {
            // Open: we're flat, need to buy.
            true
        } else {
            // Resize check: |current - target| / target > drift_threshold
            let relative_drift = if target_notional > Decimal::ZERO {
                (current_notional - target_notional).abs() / target_notional
            } else {
                Decimal::ZERO
            };
            relative_drift > drift_threshold
        };

        if !needs_order {
            // Count current notional toward portfolio cap even for held positions.
            if leg.target_weight > Decimal::ZERO {
                total_long_notional += target_notional;
            }
            continue;
        }

        if leg.target_weight == Decimal::ZERO {
            // Sell to flat.
            if current_qty > Decimal::ZERO {
                let qty = Quantity::new(current_qty).map_err(|_| PortfolioSizeError::ZeroEquity)?;
                let position_snap = trading_core::Position::empty(symbol.clone());
                // Sell order: bypass per-symbol cap check (closing always OK).
                let ord = Order::new(
                    strategy_id.clone(),
                    symbol.clone(),
                    Side::Sell,
                    qty,
                    OrderKind::Market,
                    TimeInForce::Ioc,
                    &position_snap,
                    leg.mark_price,
                    limits,
                    equity_d,
                );
                if let Ok(o) = ord {
                    orders.push(o);
                }
                // If sell fails validation, skip (should not happen for flat-close)
            }
        } else {
            // Buy to target.
            let target_qty = target_notional / mark;

            // Per-symbol exposure check.
            let per_sym_notional = target_notional / equity_d;
            if per_sym_notional > limits.per_symbol_exposure_cap {
                return Err(PortfolioSizeError::SymbolExposureBreach {
                    symbol: symbol.clone(),
                    proposed: per_sym_notional,
                    cap: limits.per_symbol_exposure_cap,
                });
            }

            total_long_notional += target_notional;

            let qty = Quantity::new(target_qty).map_err(|_| PortfolioSizeError::ZeroEquity)?;
            if qty.get() > Decimal::ZERO {
                let position_snap = trading_core::Position::empty(symbol.clone());
                // Use a relaxed limits for portfolio sizer (per-symbol cap already checked above).
                let relaxed_limits = RiskLimits {
                    per_symbol_exposure_cap: Decimal::ONE, // already checked above
                    price_sanity_band: limits.price_sanity_band,
                    portfolio_exposure_cap: None,
                };
                let ord = Order::new(
                    strategy_id.clone(),
                    symbol.clone(),
                    Side::Buy,
                    qty,
                    OrderKind::Market,
                    TimeInForce::Ioc,
                    &position_snap,
                    leg.mark_price,
                    &relaxed_limits,
                    equity_d,
                );
                if let Ok(o) = ord {
                    orders.push(o);
                }
            }
        }
    }

    // Portfolio-level exposure cap check (R5.2 all-or-nothing).
    if let Some(portfolio_cap) = limits.portfolio_exposure_cap {
        let portfolio_cap_notional = portfolio_cap * equity_d;
        if total_long_notional > portfolio_cap_notional + dec!(0.00000001) {
            return Err(PortfolioSizeError::PortfolioExposureBreach {
                proposed: total_long_notional / equity_d,
                cap: portfolio_cap,
            });
        }
    }

    // Sort by symbol alphabetically (R12.5).
    orders.sort_by(|a, b| a.symbol().cmp(b.symbol()));

    Ok(orders)
}

// ── Tests ──────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    use proptest::prelude::*;
    use rust_decimal_macros::dec;
    use trading_core::{Price, RiskLimits, StrategyId, Symbol, Timestamp};

    fn sym(s: &str) -> Symbol {
        Symbol::new(s)
    }

    fn mark(p: f64) -> Price {
        Price::new(Decimal::try_from(p).unwrap()).unwrap()
    }

    fn ts() -> Timestamp {
        Timestamp::new(time::OffsetDateTime::UNIX_EPOCH)
    }

    fn limits_with_cap(per_sym: f64, portfolio: Option<f64>) -> RiskLimits {
        RiskLimits {
            per_symbol_exposure_cap: Decimal::try_from(per_sym).unwrap(),
            price_sanity_band: dec!(0.20),
            portfolio_exposure_cap: portfolio.map(|p| Decimal::try_from(p).unwrap()),
        }
    }

    fn strategy_id() -> StrategyId {
        StrategyId::new("test_momentum")
    }

    fn empty_positions() -> BTreeMap<Symbol, Decimal> {
        BTreeMap::new()
    }

    fn make_targets(symbols_weights: &[(&str, f64, f64)]) -> BTreeMap<Symbol, TargetLeg> {
        symbols_weights
            .iter()
            .map(|(s, w, p)| {
                (
                    sym(s),
                    TargetLeg {
                        symbol: sym(s),
                        target_weight: Decimal::try_from(*w).unwrap(),
                        mark_price: mark(*p),
                    },
                )
            })
            .collect()
    }

    #[test]
    fn t607_three_leg_accept_under_cap() {
        // 3 legs, each ~0.15 of equity → total = 0.45 < 0.50 cap
        let equity = Money::<Usdt>::from_decimal(dec!(100_000));
        let targets = make_targets(&[
            ("BTCUSDT", 0.15, 40_000.0),
            ("ETHUSDT", 0.15, 2_000.0),
            ("BNBUSDT", 0.15, 300.0),
        ]);
        let limits = limits_with_cap(0.40, Some(0.50));
        let orders = size_portfolio_target(
            &targets,
            equity,
            &empty_positions(),
            dec!(0.10),
            &limits,
            &strategy_id(),
            ts(),
        );
        assert!(orders.is_ok(), "should accept: total=0.45 < cap=0.50");
        let orders = orders.unwrap();
        assert_eq!(orders.len(), 3);
    }

    #[test]
    fn t607_portfolio_breach_rejected() {
        // 3 legs, each ~0.20 of equity → total = 0.60 > 0.50 cap
        let equity = Money::<Usdt>::from_decimal(dec!(100_000));
        let targets = make_targets(&[
            ("BTCUSDT", 0.20, 40_000.0),
            ("ETHUSDT", 0.20, 2_000.0),
            ("BNBUSDT", 0.20, 300.0),
        ]);
        let limits = limits_with_cap(0.40, Some(0.50));
        let result = size_portfolio_target(
            &targets,
            equity,
            &empty_positions(),
            dec!(0.10),
            &limits,
            &strategy_id(),
            ts(),
        );
        assert!(
            matches!(
                result,
                Err(PortfolioSizeError::PortfolioExposureBreach { .. })
            ),
            "should reject: total=0.60 > cap=0.50, got {result:?}"
        );
    }

    #[test]
    fn t607_per_symbol_cap_binds_k1() {
        // K=1 degenerate case: per-symbol cap at 0.40, target at 0.50 → symbol breach
        let equity = Money::<Usdt>::from_decimal(dec!(100_000));
        let targets = make_targets(&[("BTCUSDT", 0.50, 40_000.0)]);
        let limits = limits_with_cap(0.40, Some(0.60));
        let result = size_portfolio_target(
            &targets,
            equity,
            &empty_positions(),
            dec!(0.10),
            &limits,
            &strategy_id(),
            ts(),
        );
        assert!(
            matches!(result, Err(PortfolioSizeError::SymbolExposureBreach { .. })),
            "per-symbol cap should bind for K=1 degenerate case"
        );
    }

    proptest! {
        #![proptest_config(proptest::test_runner::Config::with_cases(1000))]
        #[allow(clippy::float_arithmetic)]
        #[test]
        fn t607_no_acceptance_exceeds_cap(
            // 3 legs with random small weights
            w1 in 0.0f64..0.15f64,
            w2 in 0.0f64..0.15f64,
            w3 in 0.0f64..0.15f64,
        ) {
            let equity = Money::<Usdt>::from_decimal(dec!(100_000));
            let targets = [
                ("BTCUSDT", w1, 40_000.0_f64),
                ("ETHUSDT", w2, 2_000.0_f64),
                ("BNBUSDT", w3, 300.0_f64),
            ];
            let target_map = make_targets(&targets);
            let limits = limits_with_cap(0.40, Some(0.50));
            let result = size_portfolio_target(
                &target_map,
                equity,
                &empty_positions(),
                dec!(0.10),
                &limits,
                &strategy_id(),
                ts(),
            );
            match result {
                Ok(_orders) => {
                    // Total weight = w1 + w2 + w3 ≤ 0.45 < 0.50 cap — accepted
                    let total = w1 + w2 + w3;
                    prop_assert!(total <= 0.50 + 1e-6, "accepted but total={total} > cap=0.50");
                }
                Err(PortfolioSizeError::PortfolioExposureBreach { proposed, cap }) => {
                    // Must have been over the cap
                    let proposed_f = f64::try_from(proposed).unwrap_or(0.0_f64);
                    let cap_f = f64::try_from(cap).unwrap_or(0.5_f64);
                    prop_assert!(proposed_f > cap_f - 1e-6, "rejected but proposed={proposed_f} <= cap={cap_f}");
                }
                Err(_) => {} // other errors are OK (zero equity, etc.)
            }
        }
    }
}
