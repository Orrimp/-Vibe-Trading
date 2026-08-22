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
    // ── GROSS, not long-only (ADR-0089 D7, operator ruling 2026-08-22) ──────
    // `exposure_cap` means Σ |notional| across ALL legs. Long-only was explicitly
    // rejected: it ignores half the book, so an MN arm could add unbounded short
    // exposure and never breach. Net was rejected as near-vacuous — ~0 by
    // construction on a market-neutral arm, so the cap could never bind on the
    // very lanes it was written for.
    let mut total_gross_notional = Decimal::ZERO;

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
        // Signed-aware (ADR-0089 D7). `target_weight` may now be negative, meaning
        // a short leg; 0 still means "flat this symbol".
        let needs_order = if leg.target_weight == Decimal::ZERO {
            // Close whatever is held, in either direction.
            current_qty != Decimal::ZERO
        } else if current_qty == Decimal::ZERO {
            // Flat -> open, long or short.
            true
        } else if current_qty.is_sign_negative() != target_notional.is_sign_negative() {
            // Sign crossing (long -> short or short -> long): always act. A drift
            // ratio is meaningless across a sign change — the position must be
            // reversed regardless of magnitude.
            true
        } else {
            // Same-side resize: drift measured on magnitudes.
            let relative_drift = if target_notional.is_zero() {
                Decimal::ZERO
            } else {
                (current_notional.abs() - target_notional.abs()).abs() / target_notional.abs()
            };
            relative_drift > drift_threshold
        };

        if !needs_order {
            // Count held exposure toward the cap as GROSS magnitude — a held short
            // consumes exposure exactly as a held long does.
            total_gross_notional += target_notional.abs();
            continue;
        }

        if leg.target_weight == Decimal::ZERO {
            // Close to flat in EITHER direction (ADR-0089 D7): a long closes with a
            // Sell, a short covers with a Buy. Before this the short case did not
            // exist, so a short leg could never be flattened by the sizer at all.
            if current_qty != Decimal::ZERO {
                let close_side = if current_qty > Decimal::ZERO {
                    Side::Sell
                } else {
                    Side::Buy
                };
                let qty =
                    Quantity::new(current_qty.abs()).map_err(|_| PortfolioSizeError::ZeroEquity)?;
                let position_snap = trading_core::Position::empty(symbol.clone());
                // Closing always OK — the per-symbol cap is resulting-exposure aware
                // (bug-log #71), so a close/cover reduces to ~0 and cannot breach.
                let ord = Order::new(
                    strategy_id.clone(),
                    symbol.clone(),
                    close_side,
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
            // Open/resize toward a SIGNED target (ADR-0089 D7): a negative
            // `target_weight` is a short leg and emits a Sell.
            //
            // ── bug-log #94: the order is the DELTA, not the target ───────────
            // This used to emit `|target_notional / mark|` — the WHOLE target
            // quantity — on every action, including a same-side resize of a leg
            // already held. That is only correct against a "set position to X"
            // venue API. Every execution path in this codebase fills orders
            // INCREMENTALLY (`PaperEngine::step` adds the fill to the book), so a
            // resize added a second full-size leg on top of the first: 10 % of
            // equity, then 20 %, then 30 %, until cash ran out.
            //
            // It survived because the function had no production caller (#69) —
            // and its own tests only ever exercised flat -> open and -> close,
            // where delta and target coincide. Wiring it is what made the
            // resize path reachable, and the first fixture through it lost 74 %
            // of equity with `min_cash` at 43.8 out of 100 000.
            let target_qty_signed = target_notional / mark;
            let delta_qty = target_qty_signed - current_qty;
            let open_side = if delta_qty > Decimal::ZERO {
                Side::Buy
            } else {
                Side::Sell
            };
            let target_qty = delta_qty.abs();

            // Per-symbol exposure check, on MAGNITUDE — a 40% short is exactly as
            // exposed as a 40% long.
            let per_sym_notional = (target_notional / equity_d).abs();
            if per_sym_notional > limits.per_symbol_exposure_cap {
                return Err(PortfolioSizeError::SymbolExposureBreach {
                    symbol: symbol.clone(),
                    proposed: per_sym_notional,
                    cap: limits.per_symbol_exposure_cap,
                });
            }

            // The cap accumulates the RESULTING leg (`target_notional`), never the
            // delta — the limit is on the book after the rebalance, not on its
            // turnover.
            total_gross_notional += target_notional.abs();

            let qty = Quantity::new(target_qty).map_err(|_| PortfolioSizeError::ZeroEquity)?;
            if qty.get() > Decimal::ZERO {
                let position_snap = trading_core::Position::empty(symbol.clone());
                // An EMPTY snapshot is sound here even though `Order::new`'s cap is
                // resulting-exposure aware (bug-log #71): the per-symbol limit has
                // already been applied above to `target_notional`, which IS the
                // resulting position — a strictly better measure than
                // `snapshot + delta` could give. `relaxed_limits` therefore turns
                // that check off rather than letting it re-run on the delta alone.
                // Use a relaxed limits for portfolio sizer (per-symbol cap already checked above).
                let relaxed_limits = RiskLimits {
                    per_symbol_exposure_cap: Decimal::ONE, // already checked above
                    price_sanity_band: limits.price_sanity_band,
                    portfolio_exposure_cap: None,
                };
                let ord = Order::new(
                    strategy_id.clone(),
                    symbol.clone(),
                    open_side,
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
        if total_gross_notional > portfolio_cap_notional + dec!(0.00000001) {
            return Err(PortfolioSizeError::PortfolioExposureBreach {
                proposed: total_gross_notional / equity_d,
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

    // ═══════════════════════════════════════════════════════════════════════
    // ADR-0089 D5 — BINDING TESTS for the GROSS cap and signed legs.
    //
    // These exist because bug-log #69 was a limit with a passing test and no
    // caller: the test proved the enforcer WORKED, which is not the same as
    // proving the limit BINDS. Each test below is written so it would go RED if
    // the control were removed — the cap test in particular constructs an
    // over-cap vector explicitly, because production lanes at 0.30 gross would
    // never supply one (ADR-0089 D6).
    // ═══════════════════════════════════════════════════════════════════════

    /// THE RULING, made binding: a market-neutral book that is INSIDE the
    /// long-only cap must still be REFUSED on gross.
    ///
    /// 3 long × 0.20 + 3 short × 0.20 = **1.20 gross**, but only **0.60 long**.
    /// Under the old `total_long_notional` accounting this passed a 1.00 cap
    /// outright. That is precisely the reading ADR-0089 D7 rejected, and it is
    /// why the anchored MN surfaces ran 0.60 gross against a hashed 0.50 claim
    /// while reporting compliance.
    #[test]
    fn gross_cap_refuses_a_book_that_the_long_only_measure_would_pass() {
        let targets = make_targets(&[
            ("AAA", 0.20, 100.0),
            ("BBB", 0.20, 100.0),
            ("CCC", 0.20, 100.0),
            ("DDD", -0.20, 100.0),
            ("EEE", -0.20, 100.0),
            ("FFF", -0.20, 100.0),
        ]);
        let result = size_portfolio_target(
            &targets,
            Money::<Usdt>::from_decimal(dec!(100_000)),
            &BTreeMap::new(),
            dec!(0.10),
            &limits_with_cap(0.50, Some(1.00)),
            &StrategyId::new("gross_cap_test"),
            ts(),
        );
        match result {
            Err(PortfolioSizeError::PortfolioExposureBreach { proposed, cap }) => {
                assert_eq!(cap, dec!(1.00));
                assert!(
                    proposed > dec!(1.0),
                    "gross must be ~1.20, got {proposed} — if this reads ~0.60 the cap is \
                     back on the LONG-ONLY measure that ADR-0089 D7 rejected"
                );
            }
            other => panic!(
                "GROSS CAP NOT BINDING: 3 long + 3 short at 0.20 each is 1.20 gross against a \
                 1.00 cap and must be refused. Got {other:?}. Under the long-only measure this \
                 book reads 0.60 and passes — which is exactly the defect (bug-log #69)."
            ),
        }
    }

    /// A negative target weight must emit a SELL. Before the signed extension
    /// there was no way to express a short at all, which is why the enforcer
    /// could not serve the long/short harness.
    #[test]
    fn negative_target_weight_opens_a_short() {
        let targets = make_targets(&[("AAA", -0.20, 100.0)]);
        let orders = size_portfolio_target(
            &targets,
            Money::<Usdt>::from_decimal(dec!(100_000)),
            &BTreeMap::new(),
            dec!(0.10),
            &limits_with_cap(0.50, Some(1.00)),
            &StrategyId::new("short_open_test"),
            ts(),
        )
        .expect("a 0.20 short is inside both caps");
        assert_eq!(orders.len(), 1, "expected exactly one leg, got {orders:?}");
        assert_eq!(
            orders[0].side(),
            Side::Sell,
            "a negative target_weight must open a SHORT (Sell), got {:?}",
            orders[0].side()
        );
        assert_eq!(
            orders[0].qty().get(),
            dec!(200),
            "0.20 x 100_000 / 100 = 200 units, on MAGNITUDE not sign"
        );
    }

    /// Target 0 while holding a SHORT must emit a BUY (cover). The old code
    /// only closed longs, so a short leg could never be flattened by the sizer.
    #[test]
    fn zero_target_covers_an_existing_short() {
        let targets = make_targets(&[("AAA", 0.0, 100.0)]);
        let mut book = BTreeMap::new();
        book.insert(sym("AAA"), dec!(-200)); // short 200 units
        let orders = size_portfolio_target(
            &targets,
            Money::<Usdt>::from_decimal(dec!(100_000)),
            &book,
            dec!(0.10),
            &limits_with_cap(0.50, Some(1.00)),
            &StrategyId::new("cover_test"),
            ts(),
        )
        .expect("covering a short is always allowed");
        assert_eq!(orders.len(), 1, "expected a cover leg, got {orders:?}");
        assert_eq!(
            orders[0].side(),
            Side::Buy,
            "flattening a SHORT must emit a Buy (cover), got {:?}",
            orders[0].side()
        );
        assert_eq!(orders[0].qty().get(), dec!(200), "cover the whole short");
    }

    /// REGRESSION GUARD: an all-long book must behave exactly as before. Gross
    /// and long-only coincide when nothing is short, so the extension must be
    /// invisible to every existing long-only caller.
    #[test]
    fn long_only_book_is_unchanged_by_the_signed_extension() {
        let targets = make_targets(&[("AAA", 0.20, 100.0), ("BBB", 0.20, 100.0)]);
        let orders = size_portfolio_target(
            &targets,
            Money::<Usdt>::from_decimal(dec!(100_000)),
            &BTreeMap::new(),
            dec!(0.10),
            &limits_with_cap(0.50, Some(1.00)),
            &StrategyId::new("long_only_regression"),
            ts(),
        )
        .expect("0.40 gross is inside a 1.00 cap");
        assert_eq!(orders.len(), 2);
        assert!(
            orders.iter().all(|o| o.side() == Side::Buy),
            "an all-positive target vector must still emit only Buys"
        );
    }
}
