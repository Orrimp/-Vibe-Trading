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
//!   `cost::apply_slippage_model`. The per-symbol volume is looked up from
//!   `cfg.volume_usd_per_symbol` using the provided `symbol` key; if the map
//!   is absent or the symbol is missing, `Decimal::ZERO` is used (no impact).
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
use trading_core::{Side, Symbol};

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
/// `symbol` is used to look up the per-asset daily volume proxy from
/// `cfg.volume_usd_per_symbol` for the `SlippageModel::SquareRoot` path.
/// For `Linear` configs the symbol is ignored.
#[allow(clippy::float_arithmetic)] // no float in this fn; Decimal throughout
#[must_use]
pub fn sim_slippage_cost(
    qty: Decimal,
    fill_price: Decimal,
    side: Side,
    cfg: &LatencySlippageSimConfig,
    symbol: &Symbol,
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
            // Square-root model: look up per-symbol volume from the cfg map.
            // Falls back to Decimal::ZERO if map absent or symbol not found
            // (triggers the V=0 no-impact edge case in apply_slippage_sqrt).
            let volume_usd = cfg
                .volume_usd_per_symbol
                .as_deref()
                .and_then(|m| m.get(symbol).copied())
                .unwrap_or(Decimal::ZERO);

            // Compute adjusted fill price, then derive cost delta.
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
    use std::collections::HashMap;
    use std::sync::Arc;

    use super::*;
    use rust_decimal_macros::dec;

    fn btc_symbol() -> Symbol {
        Symbol::new("BTCUSDT")
    }

    /// At `slippage_model == Linear { bps: 0 }` the cost is always zero (noop / anchor-safe).
    #[test]
    fn zero_bps_is_noop() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 0,
            latency_ms_max: 0,
            slippage_model: SlippageModel::Linear { bps: 0 },
            volume_usd_per_symbol: None,
        };
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg, &btc_symbol());
        assert_eq!(cost, Decimal::ZERO, "zero bps must produce zero cost");
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Sell, &cfg, &btc_symbol());
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
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg, &btc_symbol());
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
        let buy_cost = sim_slippage_cost(dec!(2.5), dec!(40_000), Side::Buy, &cfg, &btc_symbol());
        let sell_cost = sim_slippage_cost(dec!(2.5), dec!(40_000), Side::Sell, &cfg, &btc_symbol());
        assert_eq!(
            buy_cost, sell_cost,
            "linear cost magnitude is identical for Buy and Sell"
        );
    }

    /// SquareRoot model: positive cost for buy when volume map contains the symbol.
    #[test]
    fn sqrt_cost_positive_for_buy() {
        let sym = btc_symbol();
        let mut map = HashMap::new();
        map.insert(sym.clone(), dec!(10_000_000_000)); // $10B daily volume
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::SquareRoot {
                alpha: dec!(1.0),
                volume_lookback_days: 90,
            },
            volume_usd_per_symbol: Some(Arc::new(map)),
        };
        // 1 BTC @ $50k, daily volume $10B → positive cost
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg, &sym);
        assert!(cost > Decimal::ZERO, "sqrt buy cost must be positive");
    }

    /// SquareRoot model: no volume map → no impact (V=0 edge case via missing map).
    #[test]
    fn sqrt_zero_volume_zero_cost() {
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::SquareRoot {
                alpha: dec!(1.0),
                volume_lookback_days: 90,
            },
            volume_usd_per_symbol: None, // no map → V=0 fallback
        };
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg, &btc_symbol());
        assert_eq!(
            cost,
            Decimal::ZERO,
            "absent volume map must produce zero cost (V=0 edge case)"
        );
    }

    /// SquareRoot model: sell cost is positive (not negative) when volume available.
    #[test]
    fn sqrt_cost_positive_for_sell() {
        let sym = btc_symbol();
        let mut map = HashMap::new();
        map.insert(sym.clone(), dec!(10_000_000_000));
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::SquareRoot {
                alpha: dec!(1.0),
                volume_lookback_days: 90,
            },
            volume_usd_per_symbol: Some(Arc::new(map)),
        };
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Sell, &cfg, &sym);
        assert!(cost > Decimal::ZERO, "sqrt sell cost must also be positive");
    }

    /// SquareRoot model: symbol not in volume map → falls back to V=0 (no impact).
    #[test]
    fn sqrt_missing_symbol_fallback_zero() {
        let sym = btc_symbol();
        let other_sym = Symbol::new("ETHUSDT");
        let mut map = HashMap::new();
        map.insert(other_sym, dec!(5_000_000_000)); // only ETH in map
        let cfg = LatencySlippageSimConfig {
            latency_ms_min: 30,
            latency_ms_max: 80,
            slippage_model: SlippageModel::SquareRoot {
                alpha: dec!(1.0),
                volume_lookback_days: 90,
            },
            volume_usd_per_symbol: Some(Arc::new(map)),
        };
        // BTC not in map → V=0 fallback → zero cost
        let cost = sim_slippage_cost(dec!(1.0), dec!(50_000), Side::Buy, &cfg, &sym);
        assert_eq!(
            cost,
            Decimal::ZERO,
            "symbol missing from volume map must produce zero cost"
        );
    }
}
