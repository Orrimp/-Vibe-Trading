//! Slippage simulation for backtest order fills (v5-latency-slippage-sim R3).
//!
//! # Models
//!
//! ## Linear-bps (v0.1.0–v0.4.0, preserved as default)
//!
//! - `Side::Buy`:  `fill_price = signal_price * (1 + bps / 10_000)`
//! - `Side::Sell`: `fill_price = signal_price * (1 - bps / 10_000)`
//! - `bps == 0`:   returns `signal_price` unchanged (noop — byte-identical to pre-feature).
//!
//! ## Square-root market-impact (v0.5.0, ADR-0043 § Changelog 2026-05-29)
//!
//! Implements the Almgren-Chriss (2001) volume-proxy form:
//! `slippage_bps = α · √(Q / V) · 10_000`
//! where Q = fill notional (USD), V = daily volume proxy (USD), α = impact coefficient.
//!
//! The `√` operation uses `f64` at an isolated conversion boundary (D-T1.3):
//! compute `Q/V` and `√` in f64, round-ties-even to `u32` bps, apply sign ×
//! multiplier in `Decimal` (preserves the existing fill-price `Decimal` contract).
//!
//! # Anchor safety
//!
//! At `bps == 0` (the noop default) the linear path is a pass-through.
//! `SlippageModel::default()` is `Linear { bps: 8 }` — the v0.4.0 canonical
//! pin from ADR-0045 D1. All 71 existing anchor SHAs are preserved by the
//! backward-compat serde adapter in `LatencySlippageSimConfig`.

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use trading_core::Side;

// ── Public types ──────────────────────────────────────────────────────────────

/// Slippage model variant. Linear preserves v0.1.0–v0.4.0 byte-identity
/// at `Linear { bps: 8 }`; SquareRoot adds the Almgren-Chriss volume-
/// proxy form `cost = α · √(Q/V)` per ADR-0043 § Changelog v0.5.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SlippageModel {
    /// Pre-v0.5.0 linear-bps model. Default `bps = 8` matches the
    /// `v5-realdata-medium-2026-05` canonical pin from ADR-0045 D1.
    Linear { bps: u32 },
    /// v0.5.0 square-root market-impact model (Almgren-Chriss 2001).
    /// Operator-locked defaults (M-OD 2026-05-29): `alpha = 1.0` (Q1=(a)
    /// Kissell 2014 midpoint), `volume_lookback_days = 90` (Q2=(a) Binance
    /// parquet trailing). `alpha` is `Decimal` at the public boundary; the
    /// f64 conversion happens INSIDE `apply_slippage_sqrt` per D-T1.3.
    SquareRoot {
        alpha: Decimal,
        volume_lookback_days: u16,
    },
}

impl Default for SlippageModel {
    /// Backward-compat default: `Linear { bps: 8 }` preserves the 71
    /// existing anchor SHAs byte-identically when `LatencySlippageSimConfig`
    /// is constructed without an explicit `slippage_model`.
    fn default() -> Self {
        SlippageModel::Linear { bps: 8 }
    }
}

/// Cap on `slippage_bps_effective` — fat-tail guard for thin-liquidity
/// hours (K3 falsifier route from feature.md).
/// Operator-locked 2026-05-29; revisitable at M-OD if dry runs surface
/// > 5% saturation.
pub const MAX_SLIPPAGE_BPS: u32 = 1_000; // 10%

// ── Public dispatcher ─────────────────────────────────────────────────────────

/// Apply slippage using the specified model.
///
/// ADR-0047 D2 SOLE-LOCATION contract: `sim_slippage_cost` in
/// `crates/backtest/src/scenarios/sim.rs` calls this dispatcher — NOT the
/// model-specific functions below. All model dispatch happens here.
///
/// # Parameters
///
/// - `signal_price`: raw bar-close price before slippage.
/// - `side`: trade direction.
/// - `notional`: fill notional Q = |qty| × fill_price (unused for Linear).
/// - `model`: which model to apply.
/// - `volume_usd`: per-asset daily volume V in USD (only used for SquareRoot;
///   pass `Decimal::ZERO` for Linear.
#[must_use]
pub fn apply_slippage_model(
    signal_price: Decimal,
    side: Side,
    notional: Decimal,
    model: SlippageModel,
    volume_usd: Decimal,
) -> Decimal {
    match model {
        SlippageModel::Linear { bps } => apply_slippage_linear(signal_price, side, bps),
        SlippageModel::SquareRoot {
            alpha,
            volume_lookback_days: _,
        } => {
            let (fill_price, _bps) = apply_slippage_sqrt(
                signal_price,
                side,
                notional,
                volume_usd,
                alpha,
                MAX_SLIPPAGE_BPS,
            );
            fill_price
        }
    }
}

/// Legacy entry-point preserved for call sites that already use `apply_slippage`.
/// Equivalent to `apply_slippage_model(price, side, notional, Linear { bps }, ZERO)`.
///
/// The `notional` parameter is unused (reserved for the original v0.2.0 promise
/// that is now fulfilled via the new `apply_slippage_model` dispatcher).
#[must_use]
pub fn apply_slippage(signal_price: Decimal, side: Side, _notional: Decimal, bps: u32) -> Decimal {
    apply_slippage_linear(signal_price, side, bps)
}

// ── Private model bodies ──────────────────────────────────────────────────────

/// Linear-bps slippage body (unchanged from v0.4.0 for byte-identity).
#[must_use]
pub(crate) fn apply_slippage_linear(signal_price: Decimal, side: Side, bps: u32) -> Decimal {
    if bps == 0 {
        return signal_price;
    }
    let bps_decimal = Decimal::from(bps) / Decimal::from(10_000_u32);
    match side {
        Side::Buy => signal_price * (Decimal::ONE + bps_decimal),
        Side::Sell => signal_price * (Decimal::ONE - bps_decimal),
    }
}

/// Square-root market-impact body (v0.5.0).
///
/// Implements D-T1.3 f64-boundary contract:
///
/// 1. Edge cases: V = 0 or Q = 0 → return `(signal_price, 0)` (no impact).
/// 2. Convert Q, V, α to f64 (bounded magnitudes make `.expect()` safe — see A2 in D-T1.10).
/// 3. `bps_raw = α × √(Q/V) × 10_000` in f64.
/// 4. Round-half-to-even (`f64::round_ties_even`, stable since Rust 1.77).
/// 5. Saturate at `[0, max_bps]` before lossless `as u32` cast.
/// 6. Apply sign × multiplier via `apply_slippage_linear` (reuses Decimal path).
///
/// Returns `(fill_price, slippage_bps_effective)`.
#[must_use]
pub(crate) fn apply_slippage_sqrt(
    signal_price: Decimal,
    side: Side,
    notional: Decimal,    // Q in USD = |qty| * fill_price
    v_daily_usd: Decimal, // V in USD = trailing-mean(volume × close) over N days
    alpha: Decimal,       // operator-locked α = 1.0 at v0.5.0
    max_bps: u32,         // MAX_SLIPPAGE_BPS = 1_000
) -> (Decimal, u32) {
    // Edge cases: zero notional or zero volume → no impact.
    if notional.is_zero() || v_daily_usd.is_zero() {
        return (signal_price, 0);
    }

    // ── f64 conversion boundary (K2 falsifier, D-T1.3) ───────────────────────
    // All inputs have well-bounded magnitudes:
    //   notional  ≤ ~$1e12 total wealth
    //   v_daily_usd ≤ ~$1e11 venue ADV
    //   alpha ∈ [0.0, ~2.0]
    // rust_decimal::Decimal::to_f64() is None only for values outside IEEE-754
    // double range (~±1.8e308), which cannot occur with these bounds.
    debug_assert!(
        !notional.is_sign_negative(),
        "notional must be non-negative"
    );
    debug_assert!(
        !v_daily_usd.is_sign_negative(),
        "v_daily_usd must be non-negative"
    );
    debug_assert!(!alpha.is_sign_negative(), "alpha must be non-negative");

    let q_f64: f64 = notional
        .to_f64()
        .expect("notional fits in f64 — bounded by total wealth");
    let v_f64: f64 = v_daily_usd
        .to_f64()
        .expect("v_daily_usd fits in f64 — bounded by venue ADV");
    let alpha_f64: f64 = alpha
        .to_f64()
        .expect("alpha fits in f64 — bounded [0.0, ~2.0]");

    // α · √(Q/V) · 10_000
    let ratio: f64 = q_f64 / v_f64; // dimensionless
    let sqrt_ratio: f64 = ratio.sqrt(); // f64::sqrt — IEEE-754 correctly rounded
    let bps_raw: f64 = alpha_f64 * sqrt_ratio * 10_000.0_f64;

    // Round-half-to-even (banker's rounding) → saturate → u32 cast.
    // Negative bps_raw impossible (all inputs non-negative) — saturating_cast.
    let bps_rounded: f64 = bps_raw.round_ties_even();
    let bps_u32: u32 = if bps_rounded >= f64::from(max_bps) {
        max_bps
    } else if bps_rounded <= 0.0_f64 {
        0
    } else {
        // SAFETY: bounded [0, max_bps] ≤ 1_000 ≪ u32::MAX; sign is non-negative;
        // value was rounded-ties-even so truncation is exact.
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        {
            bps_rounded as u32
        }
    };

    // ── Back to Decimal for sign × multiplier (R3 D3 preserved) ─────────────
    let fill_price = apply_slippage_linear(signal_price, side, bps_u32);
    (fill_price, bps_u32)
}

// ── Unit tests ────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use rust_decimal_macros::dec;

    use super::*;

    // ── Legacy linear tests (T-D-N5 preserved byte-identical) ─────────────────

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

        let buy_deviation = buy_result - price;
        let sell_deviation = price - sell_result;

        assert_eq!(
            buy_deviation, sell_deviation,
            "buy and sell slippage deviations must be equal in magnitude"
        );
    }

    /// T-D-N5 test 5: `Decimal` precision — no rounding artifacts on large prices.
    #[test]
    fn decimal_precision() {
        let price = dec!(2_345.67);
        let bps = 3_u32;
        let result = apply_slippage(price, Side::Buy, dec!(100_000), bps);
        assert!(
            result > price,
            "3 bps buy must produce a price above signal"
        );

        let expected_add = dec!(2_345.67) * dec!(3) / dec!(10_000);
        let expected = price + expected_add;
        assert_eq!(result, expected, "must match exact Decimal arithmetic");
    }

    // ── New v0.5.0 square-root tests ──────────────────────────────────────────

    /// Reference value: α=1.0, Q=$1M, V=$1B → √(1e6/1e9) = √(0.001) ≈ 0.031623
    /// bps_raw = 1.0 × 0.031623 × 10_000 = 316.23 → round_ties_even = 316
    #[test]
    fn sqrt_reference_alpha1_q1m_v1b() {
        let price = dec!(50_000.00);
        let notional = dec!(1_000_000); // $1M
        let volume = dec!(1_000_000_000); // $1B
        let alpha = dec!(1.0);

        let (_fill_price, bps) =
            apply_slippage_sqrt(price, Side::Buy, notional, volume, alpha, MAX_SLIPPAGE_BPS);

        // √(1e6/1e9) = √(0.001) = 0.031622776... × 10_000 = 316.227...
        // round_ties_even(316.227) = 316
        assert_eq!(bps, 316, "reference value: α=1 Q=$1M V=$1B → 316 bps");
    }

    /// sqrt_zero_notional_zero_slippage: Q=0 → return signal_price unchanged, bps=0.
    #[test]
    fn sqrt_zero_notional_zero_slippage() {
        let price = dec!(50_000.00);
        let (fill_price, bps) = apply_slippage_sqrt(
            price,
            Side::Buy,
            Decimal::ZERO,
            dec!(1_000_000_000),
            dec!(1.0),
            MAX_SLIPPAGE_BPS,
        );
        assert_eq!(fill_price, price, "zero notional must be noop");
        assert_eq!(bps, 0);
    }

    /// sqrt_zero_volume_zero_slippage: V=0 → return signal_price unchanged, bps=0.
    #[test]
    fn sqrt_zero_volume_zero_slippage() {
        let price = dec!(50_000.00);
        let (fill_price, bps) = apply_slippage_sqrt(
            price,
            Side::Buy,
            dec!(1_000_000),
            Decimal::ZERO,
            dec!(1.0),
            MAX_SLIPPAGE_BPS,
        );
        assert_eq!(fill_price, price, "zero volume must be noop");
        assert_eq!(bps, 0);
    }

    /// sqrt_huge_notional_caps_at_max_bps: Q=V → √(1)=1 → 10_000 bps > MAX → capped.
    #[test]
    fn sqrt_huge_notional_caps_at_max_bps() {
        let price = dec!(50_000.00);
        // Q = V → ratio = 1.0 → sqrt = 1.0 → bps_raw = 10_000 >> MAX_SLIPPAGE_BPS
        let (fill_price, bps) = apply_slippage_sqrt(
            price,
            Side::Buy,
            dec!(1_000_000_000), // Q = V
            dec!(1_000_000_000),
            dec!(1.0),
            MAX_SLIPPAGE_BPS,
        );
        assert_eq!(bps, MAX_SLIPPAGE_BPS, "Q=V must cap at MAX_SLIPPAGE_BPS");
        // fill_price for 1000 bps buy: 50_000 * (1 + 1000/10_000) = 55_000
        assert_eq!(
            fill_price,
            dec!(55_000.00),
            "capped fill price must be correct"
        );
    }

    /// sqrt_alpha_zero_zero_slippage: α=0 → bps_raw=0 → noop.
    #[test]
    fn sqrt_alpha_zero_zero_slippage() {
        let price = dec!(50_000.00);
        let (fill_price, bps) = apply_slippage_sqrt(
            price,
            Side::Buy,
            dec!(1_000_000),
            dec!(1_000_000_000),
            Decimal::ZERO, // α=0
            MAX_SLIPPAGE_BPS,
        );
        assert_eq!(fill_price, price, "alpha=0 must be noop");
        assert_eq!(bps, 0);
    }

    /// dispatcher_calls_linear_for_linear: model=Linear{8} on buy → same as legacy.
    #[test]
    fn dispatcher_calls_linear_for_linear() {
        let price = dec!(50_000.00);
        let legacy = apply_slippage(price, Side::Buy, dec!(1_000_000), 8);
        let dispatched = apply_slippage_model(
            price,
            Side::Buy,
            dec!(1_000_000),
            SlippageModel::Linear { bps: 8 },
            Decimal::ZERO,
        );
        assert_eq!(
            legacy, dispatched,
            "dispatcher must be byte-identical to legacy linear path"
        );
    }

    /// dispatcher_calls_sqrt_for_sqrt: model=SquareRoot dispatches sqrt body.
    #[test]
    fn dispatcher_calls_sqrt_for_sqrt() {
        let price = dec!(50_000.00);
        let notional = dec!(1_000_000);
        let volume = dec!(1_000_000_000);
        let alpha = dec!(1.0);

        let dispatched = apply_slippage_model(
            price,
            Side::Buy,
            notional,
            SlippageModel::SquareRoot {
                alpha,
                volume_lookback_days: 90,
            },
            volume,
        );
        // Verify it differs from the linear path (proves routing is correct).
        let linear = apply_slippage_model(
            price,
            Side::Buy,
            notional,
            SlippageModel::Linear { bps: 8 },
            Decimal::ZERO,
        );
        assert_ne!(
            dispatched, linear,
            "sqrt dispatcher must differ from linear for non-trivial Q/V"
        );
    }

    /// linear_dispatches_to_existing: Linear{8} via dispatcher = legacy apply_slippage.
    #[test]
    fn linear_dispatches_to_existing() {
        for bps in [0_u32, 1, 8, 10, 100, 1000] {
            let price = dec!(45_000.00);
            let legacy = apply_slippage(price, Side::Sell, dec!(500_000), bps);
            let via_dispatcher = apply_slippage_model(
                price,
                Side::Sell,
                dec!(500_000),
                SlippageModel::Linear { bps },
                Decimal::ZERO,
            );
            assert_eq!(
                legacy, via_dispatcher,
                "Linear{{{bps}}} dispatcher must match legacy for sell"
            );
        }
    }

    /// sqrt_sell_decreases_price: sqrt model decreases sell price.
    #[test]
    fn sqrt_sell_decreases_price() {
        let price = dec!(50_000.00);
        let (fill_price, bps) = apply_slippage_sqrt(
            price,
            Side::Sell,
            dec!(1_000_000),
            dec!(1_000_000_000),
            dec!(1.0),
            MAX_SLIPPAGE_BPS,
        );
        assert!(
            fill_price < price,
            "sell sqrt slippage must decrease fill price"
        );
        assert_eq!(bps, 316, "same bps as buy (sign applied separately)");
    }

    /// round_ties_even: verify a known tie-break case (bps_raw = 0.5 → 0).
    #[test]
    fn round_ties_even_tie_case() {
        // bps_raw = 0.5 → round_ties_even → 0 (ties to even)
        // α × √(Q/V) × 10_000 = 0.5
        // √(Q/V) = 0.5/10_000 = 5e-5 → Q/V = 2.5e-9
        // Use Q = 250, V = 1e11 (V = 100_000_000_000)
        let (_, bps) = apply_slippage_sqrt(
            dec!(10.00),
            Side::Buy,
            dec!(250),             // Q
            dec!(100_000_000_000), // V = 1e11
            dec!(1.0),
            MAX_SLIPPAGE_BPS,
        );
        // bps_raw = sqrt(250/1e11) * 10_000 = sqrt(2.5e-9) * 10_000
        //         = 5e-5 * 10_000 * 10_000 ≈ 0.5 → 0 (ties-to-even)
        // Actually: sqrt(2.5e-9) * 10_000 ≈ 0.0000500 * 10_000 = 0.50
        assert_eq!(bps, 0, "ties-to-even: 0.5 rounds to 0");
    }
}
