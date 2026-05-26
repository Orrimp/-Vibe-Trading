//! Linear-bps slippage simulation for backtest order fills (v5-latency-slippage-sim R3).
//!
//! # Design (ADR-0043 § D3)
//!
//! v0.1.0 implements the **linear bps** model:
//!
//! - `Side::Buy`:  `fill_price = signal_price * (1 + bps / 10_000)`
//! - `Side::Sell`: `fill_price = signal_price * (1 - bps / 10_000)`
//! - `bps == 0`:   returns `signal_price` unchanged (noop — byte-identical to pre-feature).
//!
//! The `notional` parameter is included in the signature for the v0.2.0
//! square-root market-impact extension (Q2; deferred) but is unused at v0.1.0.
//!
//! # Anchor safety
//!
//! At `bps == 0` (the default) this function is a pass-through. All 34
//! anchored backtest reports constructed with `LatencySlippageSimConfig::default()`
//! call `apply_slippage` with `bps = 0`, so the fill prices remain byte-identical
//! to the pre-feature values (R-NR.1 / ADR-0043 § D1).

use rust_decimal::Decimal;
use trading_core::Side;

/// Apply linear-bps slippage to a fill price.
///
/// # Parameters
///
/// - `signal_price`: the raw bar-close price (or signal price) before slippage.
/// - `side`: trade side (`Buy` or `Sell`).
/// - `_notional`: reserved for v0.2.0 square-root market-impact model; unused at v0.1.0.
/// - `bps`: slippage in basis points (e.g. `10` = 10 bps = 0.1 %).
///
/// # Returns
///
/// Adjusted fill price:
/// - `bps == 0`: `signal_price` unchanged.
/// - `Side::Buy`:  `signal_price * (1 + bps / 10_000)`.
/// - `Side::Sell`: `signal_price * (1 - bps / 10_000)`.
#[must_use]
pub fn apply_slippage(
    signal_price: Decimal,
    side: Side,
    _notional: Decimal, // unused at v0.1.0; reserved for v0.2.0 square-root market impact
    bps: u32,
) -> Decimal {
    // Fast noop path — branch prediction makes this effectively free (ADR-0043 § D1).
    if bps == 0 {
        return signal_price;
    }
    let bps_decimal = Decimal::from(bps) / Decimal::from(10_000_u32);
    match side {
        Side::Buy => signal_price * (Decimal::ONE + bps_decimal),
        Side::Sell => signal_price * (Decimal::ONE - bps_decimal),
    }
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    /// T-D-N5 test 1: `bps == 0` → signal_price unchanged (noop).
    #[test]
    fn noop_at_zero_bps() {
        let price = dec!(50_000.00);
        let result = apply_slippage(price, Side::Buy, dec!(1_000_000), 0);
        assert_eq!(result, price, "zero bps must be a noop");

        let result_sell = apply_slippage(price, Side::Sell, dec!(1_000_000), 0);
        assert_eq!(result_sell, price, "zero bps must be a noop (sell)");
    }

    /// T-D-N5 test 2: Buy side → price increases.
    #[test]
    fn buy_increases_price() {
        let price = dec!(50_000.00);
        // 10 bps = 0.10 % → expected = 50_050.00
        let result = apply_slippage(price, Side::Buy, dec!(1_000_000), 10);
        assert!(
            result > price,
            "buy slippage must increase price; got {result}"
        );
        assert_eq!(result, dec!(50_050.00), "10 bps on 50_000 = 50_050 for buy");
    }

    /// T-D-N5 test 3: Sell side → price decreases.
    #[test]
    fn sell_decreases_price() {
        let price = dec!(50_000.00);
        // 10 bps = 0.10 % → expected = 49_950.00
        let result = apply_slippage(price, Side::Sell, dec!(1_000_000), 10);
        assert!(
            result < price,
            "sell slippage must decrease price; got {result}"
        );
        assert_eq!(
            result,
            dec!(49_950.00),
            "10 bps on 50_000 = 49_950 for sell"
        );
    }

    /// T-D-N5 test 4: buy and sell slippage are symmetric around the signal price.
    #[test]
    fn sign_symmetry() {
        let price = dec!(100.00);
        let bps = 5_u32; // 5 bps = 0.05 %
        let buy_result = apply_slippage(price, Side::Buy, dec!(10_000), bps);
        let sell_result = apply_slippage(price, Side::Sell, dec!(10_000), bps);

        // buy deviation = buy_result - price = price * bps/10_000
        let buy_deviation = buy_result - price;
        // sell deviation = price - sell_result = price * bps/10_000
        let sell_deviation = price - sell_result;

        assert_eq!(
            buy_deviation, sell_deviation,
            "buy and sell slippage deviations must be equal in magnitude"
        );
    }

    /// T-D-N5 test 5: `Decimal` precision — no rounding artifacts on large prices.
    #[test]
    fn decimal_precision() {
        // A mid-size price typical in crypto trading.
        let price = dec!(2_345.67);
        let bps = 3_u32; // 3 bps = 0.03 %

        // Expected: 2_345.67 * 1.0003 = 2_346.3734... → Decimal retains full precision.
        let result = apply_slippage(price, Side::Buy, dec!(100_000), bps);
        // Verify it's larger than the signal price.
        assert!(
            result > price,
            "3 bps buy must produce a price above signal"
        );

        // Verify precision: 2345.67 * 3 / 10000 = 0.703701 added.
        let expected_add = dec!(2_345.67) * dec!(3) / dec!(10_000);
        let expected = price + expected_add;
        assert_eq!(result, expected, "must match exact Decimal arithmetic");
    }
}
