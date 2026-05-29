//! v5-latency-slippage-sim: shared fill-accounting helper.
//!
//! `sim_slippage_cost` was originally a private function inside
//! `crates/backtest/src/scenarios/momentum.rs:551`. This module lifts
//! it to a shared location so all 7 strategy execution paths can call it
//! without duplicating the implementation (ADR-0047 D2).
//!
//! ## v0.5.0 extension (ADR-0043 § Changelog 2026-05-29)
//!
//! `sim_slippage_cost` now dispatches on `SlippageModel`:
//! - `Linear { bps }`: byte-identical to the v0.4.0 body.
//! - `SquareRoot { alpha, volume_lookback_days }`: Almgren-Chriss form via
//!   `cost::apply_slippage_model`. `volume_usd` is passed in by the caller
//!   (retrieved at scenario load time via `daily_volume_usd_trailing`).
//!
//! ## Anchor-additive contract (ADR-0038 § D6.a)
//!
//! The `Linear` path body is byte-identical to the original private version in
//! `momentum.rs`. No behavioural change for the 71 existing anchors.
//!
//! ## Grep gate (ADR-0047 D2)
//!
//! `grep -r "fn sim_slippage_cost" crates/backtest/src` MUST return
//! exactly 1 line (this file). The tester enforces this at M-FINAL.

use rust_decimal::Decimal;
use trading_core::Side;

use cost::{SlippageModel, apply_slippage_model};

use crate::cli_types::LatencySlippageSimConfig;

/// Compute the extra cash cost due to simulated slippage on a fill.
///
/// At `slippage_model == Linear { bps: 0 }` (the default) returns
/// `Decimal::ZERO` — no change to the fill accounting, byte-identical to
/// the pre-feature code path.
///
/// For a Buy fill: simulated slippage costs EXTRA cash (we pay more).
/// For a Sell fill: simulated slippage costs LESS cash received (we get less).
///
/// The caller deducts the returned value from cash in both directions:
/// - Buy: `cash -= notional + fee + sim_slip_cost`
/// - Sell: `cash += notional - fee - sim_slip_cost`
///
/// `volume_usd` is the per-asset daily volume proxy in USD used by
/// `SlippageModel::SquareRoot`. Pass `Decimal::ZERO` for `Linear` configs
/// (the value is ignored).
#[allow(clippy::float_arithmetic)] // no float in this fn; Decimal throughout
#[must_use]
pub fn sim_slippage_cost(
    qty: Decimal,
    fill_price: Decimal,
    side: Side,
    cfg: &LatencySlippageSimConfig,
    volume_usd: Decimal,
) -> Decimal {
    match cfg.slippage_model {
        SlippageModel::Linear { bps: 0 } => Decimal::ZERO,
        SlippageModel::Linear { bps } => {
            // Byte-identical to the v0.4.0 body for the linear path.
            let bps_decimal = Decimal::from(bps) / Decimal::from(10_000_u32);
            let _ = side; // direction handled by caller's sign logic
            qty * fill_price * bps_decimal
        }
        SlippageModel::SquareRoot {
            alpha: _,
            volume_lookback_days: _,
        } => {
            // Square-root model: compute adjusted fill price, then derive cost delta.
            // notional = qty × fill_price (the Q term in α·√(Q/V)).
            let notional = qty * fill_price;
            let adjusted_fill =
                apply_slippage_model(fill_price, side, notional, cfg.slippage_model, volume_usd);
            // Cost = qty × |adjusted_fill - fill_price| (always non-negative).
            // For Buy: adjusted_fill > fill_price → extra cost.
            // For Sell: adjusted_fill < fill_price → cost = qty × (fill_price - adjusted_fill).
            match side {
                Side::Buy => qty * (adjusted_fill - fill_price),
                Side::Sell => qty * (fill_price - adjusted_fill),
            }
        }
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    /// At `slippage_model == Linear { bps: 0 }` the cost is always zero (noop / anchor-safe).
    #[test]
    fn zero_bps_is_noop() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 0,
            latency_ms_max: 0,
            slippage_model: SlippageModel::Linear { bps: 0 },
            volume_usd_per_symbol: None,
        };
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg, Decimal::ZERO);
        assert_eq!(cost, Decimal::ZERO, "zero bps must produce zero cost");
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Sell, &cfg, Decimal::ZERO);
        assert_eq!(cost, Decimal::ZERO, "zero bps sell must produce zero cost");
    }

    /// At 8 bps (canonical config), 1 BTC @ `$50_000` costs `$40` extra.
    #[test]
    fn canonical_8bps_buy() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::Linear { bps: 8 },
            volume_usd_per_symbol: None,
        };
        // 1.0 * 50_000 * 8 / 10_000 = 40
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg, Decimal::ZERO);
        assert_eq!(cost, dec!(40), "8bps on 50k should be 40");
    }

    /// Side does not change the cost value for linear (sign logic is in the caller).
    #[test]
    fn buy_and_sell_same_magnitude_linear() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::Linear { bps: 8 },
            volume_usd_per_symbol: None,
        };
        let buy_cost = sim_slippage_cost(dec!(2.5), dec!(40_000), Side::Buy, &cfg, Decimal::ZERO);
        let sell_cost = sim_slippage_cost(dec!(2.5), dec!(40_000), Side::Sell, &cfg, Decimal::ZERO);
        assert_eq!(
            buy_cost, sell_cost,
            "linear cost magnitude is identical for Buy and Sell"
        );
    }

    /// SquareRoot model: positive cost for buy, zero for zero volume.
    #[test]
    fn sqrt_cost_positive_for_buy() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::SquareRoot {
                alpha: dec!(1.0),
                volume_lookback_days: 90,
            },
            volume_usd_per_symbol: None,
        };
        // 1 BTC @ $50k, daily volume $10B
        let cost = sim_slippage_cost(
            dec!(1.0),
            dec!(50_000),
            Side::Buy,
            &cfg,
            dec!(10_000_000_000),
        );
        assert!(cost > Decimal::ZERO, "sqrt buy cost must be positive");
    }

    /// SquareRoot model: zero volume_usd → no impact (V=0 edge case).
    #[test]
    fn sqrt_zero_volume_zero_cost() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::SquareRoot {
                alpha: dec!(1.0),
                volume_lookback_days: 90,
            },
            volume_usd_per_symbol: None,
        };
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg, Decimal::ZERO);
        assert_eq!(
            cost,
            Decimal::ZERO,
            "zero volume must produce zero cost (V=0 edge case)"
        );
    }

    /// SquareRoot model: sell cost is positive (not negative).
    #[test]
    fn sqrt_cost_positive_for_sell() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::SquareRoot {
                alpha: dec!(1.0),
                volume_lookback_days: 90,
            },
            volume_usd_per_symbol: None,
        };
        let cost = sim_slippage_cost(
            dec!(1.0),
            dec!(50_000),
            Side::Sell,
            &cfg,
            dec!(10_000_000_000),
        );
        assert!(cost > Decimal::ZERO, "sqrt sell cost must also be positive");
    }
}
