//! Property tests for `Order::new` (T04 acceptance).
//! Lives inside the crate to avoid the `core`-name shadowing issue in
//! external integration tests.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::too_many_arguments)]

use proptest::prelude::*;
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::{
    OrderKind, Position, Price, Quantity, RiskLimits, Side, StrategyId, Symbol, TimeInForce,
};

fn btc_sym() -> Symbol {
    Symbol::new("BTCUSDT")
}

fn limits() -> RiskLimits {
    RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.10),
        portfolio_exposure_cap: None,
    }
}

fn flat_pos() -> Position {
    Position::empty(Symbol::new(""))
}

#[allow(clippy::too_many_arguments)]
fn try_order(
    symbol: Symbol,
    side: Side,
    qty: Quantity,
    kind: OrderKind,
    position: &Position,
    last_mark: Price,
    lims: &RiskLimits,
    equity: Decimal,
) -> bool {
    crate::Order::new(
        StrategyId::new("test"),
        symbol,
        side,
        qty,
        kind,
        TimeInForce::Gtc,
        position,
        last_mark,
        lims,
        equity,
    )
    .is_ok()
}

proptest! {
    #[test]
    fn prop_positive_qty_accepted(
        qty_micro in 1u64..=10_000u64,
        price_int in 1u64..=100u64,
    ) {
        let qty_dec = Decimal::from(qty_micro) / Decimal::from(100_000u64);
        let price_dec = Decimal::from(price_int);
        let qty = Quantity::new(qty_dec).unwrap();
        let last_mark = Price::new(price_dec).unwrap();
        let accepted = try_order(
            btc_sym(), Side::Buy, qty, OrderKind::Market,
            &flat_pos(), last_mark, &limits(), dec!(1_000_000),
        );
        prop_assert!(accepted, "positive qty should be accepted");
    }

    #[test]
    fn prop_zero_qty_rejected(
        price_int in 1u64..=100_000u64,
    ) {
        let qty = Quantity::new(dec!(0)).unwrap();
        let last_mark = Price::new(Decimal::from(price_int)).unwrap();
        let accepted = try_order(
            btc_sym(), Side::Buy, qty, OrderKind::Market,
            &flat_pos(), last_mark, &limits(), dec!(1_000_000),
        );
        prop_assert!(!accepted, "zero qty must be rejected");
    }

    #[test]
    fn prop_exposure_cap(equity_int in 1u64..=1_000u64) {
        let equity = Decimal::from(equity_int);
        let qty = Quantity::new(dec!(0.5)).unwrap();
        let price = dec!(1000);
        let last_mark = Price::new(price).unwrap();
        let notional = dec!(0.5) * price;
        let cap_notional = equity * dec!(0.40);
        let expected_breach = notional > cap_notional;
        let accepted = try_order(
            btc_sym(), Side::Buy, qty, OrderKind::Market,
            &flat_pos(), last_mark, &limits(), equity,
        );
        prop_assert_eq!(!accepted, expected_breach);
    }
}

#[test]
fn limit_price_within_band_accepted() {
    let last_mark = Price::new(dec!(40_000)).unwrap();
    let limit_price = Price::new(dec!(41_000)).unwrap();
    assert!(try_order(
        btc_sym(),
        Side::Buy,
        Quantity::new(dec!(0.001)).unwrap(),
        OrderKind::Limit { price: limit_price },
        &flat_pos(),
        last_mark,
        &limits(),
        dec!(1_000_000),
    ));
}

#[test]
fn limit_price_outside_band_rejected() {
    let last_mark = Price::new(dec!(40_000)).unwrap();
    let limit_price = Price::new(dec!(50_000)).unwrap();
    assert!(!try_order(
        btc_sym(),
        Side::Buy,
        Quantity::new(dec!(0.001)).unwrap(),
        OrderKind::Limit { price: limit_price },
        &flat_pos(),
        last_mark,
        &limits(),
        dec!(1_000_000),
    ));
}

#[test]
fn symbol_mismatch_rejected() {
    let position = Position::empty(Symbol::new("ETHUSDT"));
    let accepted = crate::Order::new(
        StrategyId::new("test"),
        Symbol::new("BTCUSDT"),
        Side::Buy,
        Quantity::new(dec!(0.001)).unwrap(),
        OrderKind::Market,
        TimeInForce::Gtc,
        &position,
        Price::new(dec!(40_000)).unwrap(),
        &limits(),
        dec!(1_000_000),
    )
    .is_ok();
    assert!(!accepted, "symbol mismatch should be rejected");
}
