//! Binding test for the per-symbol exposure cap (bug-log #71, story 1-25 AC3).
//!
//! AC3 requires the cap be "evaluated against RESULTING exposure with the side
//! considered, plus a binding test". This is that test.
//!
//! The cap used to be computed on the ORDER'S OWN notional, consulting neither
//! `side` nor `position_snapshot`. Two consequences, and the second was not in
//! the original #71 write-up:
//!
//!   1. It REJECTED position-closing orders — a Sell that drives exposure to zero
//!      was refused exactly as if it opened that much. Silently: no else arm, no
//!      warn, no counter. The strategy recorded the close, the engine kept the
//!      position, later decisions ran off a false flat.
//!
//!   2. It was EVADABLE BY SPLITTING. Each small order passed on its own notional
//!      while the position accumulated straight past the cap. A limit that binds
//!      per-order but not per-position is not a limit on exposure at all.
//!
//! Fixture throughout: equity 100_000, mark 1000, cap 0.40 => the cap permits at
//! most 40 units (40_000 notional).

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{
    Order, OrderError, OrderKind, Position, Price, Quantity, RiskError, RiskLimits, Side,
    StrategyId, Symbol, TimeInForce,
};

const EQUITY: Decimal = dec!(100_000);

fn mark() -> Price {
    Price::new(dec!(1000)).unwrap()
}

fn limits() -> RiskLimits {
    RiskLimits {
        per_symbol_exposure_cap: dec!(0.40),
        price_sanity_band: dec!(0.50),
        portfolio_exposure_cap: None,
    }
}

/// A position snapshot carrying a real signed quantity.
fn pos_with(base_qty: Decimal) -> Position {
    let mut p = Position::empty(Symbol::new("BTCUSDT"));
    p.base_qty = base_qty;
    p
}

fn try_order(side: Side, qty: Decimal, pos: &Position) -> Result<Order, OrderError> {
    Order::new(
        StrategyId::new("cap_binding_test"),
        Symbol::new("BTCUSDT"),
        side,
        Quantity::new(qty).unwrap(),
        OrderKind::Market,
        TimeInForce::Ioc,
        pos,
        mark(),
        &limits(),
        EQUITY,
    )
}

fn is_cap_rejection(r: &Result<Order, OrderError>) -> bool {
    matches!(r, Err(OrderError::Risk(RiskError::ExposureCap { .. })))
}

/// #71 PRIMARY: a large CLOSING sell must be accepted — it drives exposure to 0.
///
/// Old behaviour: 50 units x 1000 = 50_000 / 100_000 = 0.50 > 0.40 => REJECTED,
/// leaving the engine holding a position the strategy believed it had closed.
#[test]
fn closing_sell_is_accepted_even_when_its_own_notional_exceeds_the_cap() {
    let held = pos_with(dec!(50)); // already 0.50 of equity — legitimately, e.g. after a rally
    let r = try_order(Side::Sell, dec!(50), &held);
    assert!(
        r.is_ok(),
        "a sell that closes the position (resulting exposure 0) must never be capped — \
         this is bug-log #71: the strategy's book diverges from the engine's when it is"
    );
}

/// #71 MIRROR (1-21 review): a short's buy-to-cover is sized at the full short
/// notional, so the old cap refused it — leaving forced liquidation as the only
/// exit. A mechanistic candidate for the 97.8-100% p95 MaxDD on the MN surfaces.
#[test]
fn buy_to_cover_a_short_is_accepted() {
    let short = pos_with(dec!(-50));
    let r = try_order(Side::Buy, dec!(50), &short);
    assert!(
        r.is_ok(),
        "a buy-to-cover (resulting exposure 0) must never be capped — if it is, \
         forced liquidation becomes the only exit from a losing short"
    );
}

/// THE CAP STILL BINDS: a single order that would open past the cap is refused.
#[test]
fn oversized_opening_buy_is_still_rejected() {
    let flat = pos_with(Decimal::ZERO);
    let r = try_order(Side::Buy, dec!(50), &flat); // resulting 50 => 0.50 > 0.40
    assert!(
        is_cap_rejection(&r),
        "the cap must still refuse an order that ENDS above it; got {r:?}"
    );
}

/// Same, in the short direction — resulting exposure is signed-magnitude.
#[test]
fn oversized_opening_short_is_still_rejected() {
    let flat = pos_with(Decimal::ZERO);
    let r = try_order(Side::Sell, dec!(50), &flat); // resulting -50 => |0.50| > 0.40
    assert!(
        is_cap_rejection(&r),
        "opening a large SHORT must be refused too; got {r:?}"
    );
}

/// THE EVASION THE OLD CHECK ALLOWED — and the reason this test exists.
///
/// Position already at 39 units (0.39, just under the cap). A 5-unit top-up ends
/// at 44 units = 0.44, ABOVE the cap. The old per-order check saw only
/// 5 x 1000 / 100_000 = 0.05 and waved it through, so exposure could be walked
/// past the cap in arbitrarily small steps. The cap now refuses it.
#[test]
fn cap_cannot_be_evaded_by_splitting_into_small_increments() {
    let near_cap = pos_with(dec!(39));
    let r = try_order(Side::Buy, dec!(5), &near_cap);
    assert!(
        is_cap_rejection(&r),
        "a small order that pushes RESULTING exposure past the cap must be refused — \
         the old per-order check let the cap be walked past in increments; got {r:?}"
    );
}

/// A reduction that stays within the cap is fine (the ordinary case).
#[test]
fn partial_reduction_is_accepted() {
    let held = pos_with(dec!(50));
    let r = try_order(Side::Sell, dec!(20), &held); // resulting 30 => 0.30 <= 0.40
    assert!(
        r.is_ok(),
        "reducing toward the cap must be allowed; got {r:?}"
    );
}

/// ANCHOR-SAFETY PROPERTY: with the empty placeholder snapshot that most callers
/// and unit tests pass, `base_qty` is 0 and the arithmetic reduces to the previous
/// `|qty| * mark / equity`. Behaviour is unchanged for every no-position caller,
/// which is why this fix is surgical rather than sweeping.
#[test]
fn empty_placeholder_snapshot_reproduces_the_previous_arithmetic() {
    let placeholder = Position::empty(Symbol::new(""));
    assert!(
        try_order(Side::Buy, dec!(40), &placeholder).is_ok(),
        "40 units = exactly the cap must pass"
    );
    assert!(
        is_cap_rejection(&try_order(Side::Buy, dec!(41), &placeholder)),
        "41 units = just over the cap must fail"
    );
}
