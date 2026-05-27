//! v5-latency-slippage-sim: shared fill-accounting helper.
//!
//! `sim_slippage_cost` was originally a private function inside
//! `crates/backtest/src/scenarios/momentum.rs:551`. This module lifts
//! it to a shared location so all 7 strategy execution paths can call it
//! without duplicating the implementation (ADR-0047 D2).
//!
//! ## Anchor-additive contract (ADR-0038 § D6.a)
//!
//! The function body is byte-identical to the original private version in
//! `momentum.rs`. No behavioural change; only the import path changed.
//! The momentum backtest SHA-256 anchor is therefore unaffected by this
//! extraction.
//!
//! ## Grep gate (ADR-0047 D2)
//!
//! `grep -r "fn sim_slippage_cost" crates/backtest/src` MUST return
//! exactly 1 line (this file). The tester enforces this at M-FINAL.

use rust_decimal::Decimal;
use trading_core::Side;

use crate::cli_types::LatencySlippageSimConfig;

/// Compute the extra cash cost due to simulated slippage on a fill.
///
/// At `slippage_bps == 0` (the default) returns `Decimal::ZERO` — no change
/// to the fill accounting, byte-identical to the pre-feature code path.
///
/// For a Buy fill: simulated slippage costs EXTRA cash (we pay more).
/// For a Sell fill: simulated slippage costs LESS cash received (we get less).
///
/// The caller deducts the returned value from cash in both directions:
/// - Buy: `cash -= notional + fee + sim_slip_cost`
/// - Sell: `cash += notional - fee - sim_slip_cost`
#[allow(clippy::float_arithmetic)] // no float; uses Decimal throughout
#[must_use]
pub fn sim_slippage_cost(
    qty: Decimal,
    fill_price: Decimal,
    side: Side,
    cfg: &LatencySlippageSimConfig,
) -> Decimal {
    if cfg.slippage_bps == 0 {
        return Decimal::ZERO;
    }
    let bps_decimal = Decimal::from(cfg.slippage_bps) / Decimal::from(10_000_u32);
    // For both sides, the extra COST is qty * fill_price * bps/10_000.
    // For Buy: we pay bps% more than the fill_price → extra cost.
    // For Sell: we receive bps% less than the fill_price → extra cost.
    // The sign logic in the caller handles the direction.
    let _ = side; // direction is handled by the caller's sign logic
    qty * fill_price * bps_decimal
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// At `slippage_bps == 0` the cost is always zero (noop / anchor-safe).
    #[test]
    fn zero_bps_is_noop() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 0,
            latency_ms_max: 0,
            slippage_bps: 0,
        };
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg);
        assert_eq!(cost, Decimal::ZERO, "zero bps must produce zero cost");
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Sell, &cfg);
        assert_eq!(cost, Decimal::ZERO, "zero bps sell must produce zero cost");
    }

    /// At 8 bps (canonical config), 1 BTC @ `$50_000` costs `$40` extra.
    #[test]
    fn canonical_8bps_buy() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_bps: 8,
        };
        // 1.0 * 50_000 * 8 / 10_000 = 40
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg);
        assert_eq!(cost, dec!(40), "8bps on 50k should be 40");
    }

    /// Side does not change the cost value (sign logic is in the caller).
    #[test]
    fn buy_and_sell_same_magnitude() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_bps: 8,
        };
        let buy_cost = sim_slippage_cost(dec!(2.5), dec!(40_000), Side::Buy, &cfg);
        let sell_cost = sim_slippage_cost(dec!(2.5), dec!(40_000), Side::Sell, &cfg);
        assert_eq!(
            buy_cost, sell_cost,
            "cost magnitude is identical for Buy and Sell"
        );
    }
}
