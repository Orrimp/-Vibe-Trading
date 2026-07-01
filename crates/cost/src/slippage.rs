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
//! ## Vol-scaled spread (v2.0.0, ADR-0081, opt-in-forever — D6)
//!
//! State-aware spread that widens in high-volatility regimes. From the research:
//! spreads on liquid majors widen 2–3× in volatility/stress `research/backtesting[47]`.
//!
//! Formula:
//! ```text
//! effective_bps = base_bps + vol_multiplier · σ̂_ewma(bar_returns)
//! ```
//! where σ̂_ewma is an EWMA of the trailing `sigma_window` log-returns (in bps units:
//! 1 unit return = 10_000 bps). The EWMA λ defaults to 0.94 (RiskMetrics).
//!
//! **σ̂ computation (inlined):** the `cost` crate does NOT depend on `crates/strategy`.
//! That dep direction would create a cycle (`strategy` dev-depends on `cost`).
//! An equivalent 5-line EWMA closed form is inlined here. This is recorded in
//! ADR-0081 §D1 (dep-cycle avoidance). The formula is identical to
//! `strategy::vol_estimator::ewma_realized_vol` with `λ = sigma_lambda`.
//!
//! **Decimal/f64 boundary:** log-returns and EWMA variance are computed in `f64`
//! (statistical / dimensionless). The final `effective_bps` is converted back to
//! `Decimal` before the fill-price multiply (preserving the ADR-0003 money invariant).
//!
//! **Default unchanged (D6 contract).** `SlippageModel::default() = Linear { bps: 8 }`.
//! `VolScaledSpread` is opt-in ONLY. Anchors 119/119 by construction.
//!
//! # Anchor safety
//!
//! At `bps == 0` (the noop default) the linear path is a pass-through.
//! `SlippageModel::default()` is `Linear { bps: 8 }` — the v0.4.0 canonical
//! pin from ADR-0045 D1. All 119 existing anchor SHAs are preserved because
//! `VolScaledSpread` is a NEW variant, opt-in-forever (ADR-0081 §D6 contract).

use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde::{Deserialize, Serialize};
use trading_core::Side;

// ── Public types ──────────────────────────────────────────────────────────────

/// Slippage model variant. Linear preserves v0.1.0–v0.4.0 byte-identity
/// at `Linear { bps: 8 }`; SquareRoot adds the Almgren-Chriss volume-
/// proxy form `cost = α · √(Q/V)` per ADR-0043 § Changelog v0.5.0;
/// VolScaledSpread adds a state-aware vol-regime spread per ADR-0081 (opt-in-forever).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
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
    /// v2.0.0 vol-scaled spread (ADR-0081, **opt-in-forever — D6**).
    ///
    /// Widens the effective spread in high-volatility regimes per
    /// `research/backtesting/application-cost-and-impact-modeling.md §6 E`
    /// and `research/crypto-market-structure/application-data-integrity.md §6 A`:
    ///
    /// ```text
    /// effective_bps = base_bps + vol_multiplier · σ̂_ewma(bar_returns) · 10_000
    /// ```
    ///
    /// `σ̂_ewma` is computed from the last `sigma_window` log-returns using
    /// an EWMA with decay parameter `sigma_lambda`. The resulting effective bps
    /// is capped at `MAX_SLIPPAGE_BPS` and applied via the linear path.
    ///
    /// ## Chosen defaults (operator-ratified ADR-0081 § D2)
    ///
    /// | Parameter | Default | Rationale |
    /// |-----------|---------|-----------|
    /// | `base_bps` | `8` | Matches the Linear default (ADR-0045 D1 pin) |
    /// | `vol_multiplier` | `2.0` | Research: spreads widen 2–3× in high-vol; midpoint = 2.0 (`backtesting[47]`) |
    /// | `sigma_window` | `20` | ~1 trading month of hourly bars; responsive without overfitting transient spikes |
    /// | `sigma_lambda` | `0.94` | RiskMetrics λ (≈11.3-day half-life); faster-reacting than the 126-day sizing default to respond to short-term liquidity stress |
    ///
    /// ## σ̂ unit note
    ///
    /// σ̂ is a dimensionless per-bar return (log-return std). Multiplied by 10_000
    /// it converts to bps-scale, matching the `base_bps` units.
    ///
    /// ## Dep-cycle note (ADR-0081 § D1)
    ///
    /// The EWMA is inlined here rather than consuming `strategy::vol_estimator`
    /// because `strategy` dev-depends on `cost` → adding `cost → strategy` creates
    /// a cycle. The inlined recurrence is identical to `ewma_realized_vol` in
    /// `crates/strategy/src/vol_estimator.rs`.
    VolScaledSpread {
        /// Base spread in basis points (floor when vol = 0).
        base_bps: u32,
        /// Multiplier applied to σ̂_ewma (dimensionless, in bps per unit σ̂).
        vol_multiplier: f64,
        /// Number of trailing log-returns to use for the EWMA initialisation.
        /// Also the minimum history needed before the EWMA settles; shorter histories
        /// fall back to `base_bps` only.
        sigma_window: usize,
        /// EWMA decay parameter λ ∈ (0, 1). Default = 0.94 (RiskMetrics).
        sigma_lambda: f64,
    },
}

impl Default for SlippageModel {
    /// Backward-compat default: `Linear { bps: 8 }` preserves the 119
    /// existing anchor SHAs byte-identically when `LatencySlippageSimConfig`
    /// is constructed without an explicit `slippage_model`.
    ///
    /// **D6 (ADR-0081) — NEVER CHANGE THIS DEFAULT.** `VolScaledSpread`
    /// is opt-in-forever; bumping the default would break all 119 anchors.
    fn default() -> Self {
        SlippageModel::Linear { bps: 8 }
    }
}

/// Default parameters for `SlippageModel::VolScaledSpread` (ADR-0081 § D2).
///
/// These are the operator-ratified defaults; callers may override any field.
///
/// ```rust
/// use cost::slippage::{SlippageModel, DEFAULT_VOL_SCALED_SPREAD};
/// let model = DEFAULT_VOL_SCALED_SPREAD;
/// assert!(matches!(model, SlippageModel::VolScaledSpread { base_bps: 8, .. }));
/// ```
pub const DEFAULT_VOL_SCALED_SPREAD: SlippageModel = SlippageModel::VolScaledSpread {
    base_bps: 8,
    vol_multiplier: 2.0,
    sigma_window: 20,
    sigma_lambda: 0.94,
};

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
///   pass `Decimal::ZERO` for Linear).
/// - `bar_log_returns`: recent per-bar log-returns (only used for VolScaledSpread;
///   pass `&[]` for Linear / SquareRoot).
#[must_use]
pub fn apply_slippage_model(
    signal_price: Decimal,
    side: Side,
    notional: Decimal,
    model: SlippageModel,
    volume_usd: Decimal,
) -> Decimal {
    apply_slippage_model_with_returns(signal_price, side, notional, model, volume_usd, &[])
}

/// Apply slippage using the specified model, with optional bar log-returns
/// for the `VolScaledSpread` variant.
///
/// This is the full dispatcher. `apply_slippage_model` is a convenience
/// wrapper for backward-compat call sites that don't have a returns slice.
///
/// # Parameters
///
/// - `signal_price`: raw bar-close price before slippage.
/// - `side`: trade direction.
/// - `notional`: fill notional Q = |qty| × fill_price (unused for Linear).
/// - `model`: which model to apply.
/// - `volume_usd`: per-asset daily volume V in USD (only used for SquareRoot).
/// - `bar_log_returns`: trailing log-returns ln(close_t/close_{t-1}) from
///   recent bars (only consumed by VolScaledSpread).
#[must_use]
pub fn apply_slippage_model_with_returns(
    signal_price: Decimal,
    side: Side,
    notional: Decimal,
    model: SlippageModel,
    volume_usd: Decimal,
    bar_log_returns: &[f64],
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
        SlippageModel::VolScaledSpread {
            base_bps,
            vol_multiplier,
            sigma_window,
            sigma_lambda,
        } => {
            let effective_bps = apply_slippage_vol_scaled_bps(
                bar_log_returns,
                base_bps,
                vol_multiplier,
                sigma_window,
                sigma_lambda,
            );
            apply_slippage_linear(signal_price, side, effective_bps)
        }
    }
}

// ── Fee-sensitivity report ────────────────────────────────────────────────────

/// Fee-sensitivity sweep: returns a `Vec<(multiplier, effective_base_bps)>` for
/// a range of vol-scale multipliers at a given observed volatility.
///
/// This is the "spec-curve for costs" from `backtesting[24][47]`: re-rank across
/// a small grid of cost assumptions; a crown whose verdict flips at plausible fees
/// is not robust. The output is report-only — it does NOT change any gate band.
///
/// # Parameters
///
/// - `base_bps`: the floor spread in basis points.
/// - `sigma_hat`: a single σ̂ value (e.g. the current-regime EWMA vol).
/// - `vol_scale_factors`: the multiplier grid to sweep (e.g. `[1.0, 2.0, 3.0]`).
///
/// # Returns
///
/// A vector of `(multiplier, effective_bps_f64)` pairs, one per factor.
/// `effective_bps_f64` = `base_bps + factor · sigma_hat · 10_000`, capped at
/// `MAX_SLIPPAGE_BPS as f64`. The caller converts to display units as needed.
///
/// # Example
///
/// ```rust
/// use cost::slippage::fee_sensitivity_report;
/// let results = fee_sensitivity_report(8, 0.01, &[1.0, 2.0, 3.0]);
/// // 0.01 daily σ = 1 % per bar → 100 bps; with multiplier 2.0 → 8 + 200 = 208 bps
/// assert_eq!(results.len(), 3);
/// let (mult, eff_bps) = results[1]; // multiplier 2.0
/// assert!((mult - 2.0).abs() < 1e-10);
/// // 8 + 2.0 * 0.01 * 10_000 = 8 + 200 = 208.0
/// assert!((eff_bps - 208.0).abs() < 1e-6);
/// ```
#[must_use]
pub fn fee_sensitivity_report(
    base_bps: u32,
    sigma_hat: f64,
    vol_scale_factors: &[f64],
) -> Vec<(f64, f64)> {
    let base_f64 = f64::from(base_bps);
    let max_f64 = f64::from(MAX_SLIPPAGE_BPS);

    vol_scale_factors
        .iter()
        .map(|&factor| {
            // effective_bps = base + factor · σ̂ · 10_000
            let raw = base_f64 + factor * sigma_hat * 10_000.0_f64;
            let capped = raw.min(max_f64).max(base_f64);
            (factor, capped)
        })
        .collect()
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

/// Vol-scaled spread body (ADR-0081 § D1).
///
/// Computes `effective_bps = base_bps + vol_multiplier · σ̂_ewma · 10_000`,
/// capped at `MAX_SLIPPAGE_BPS`.
///
/// # EWMA inlined (dep-cycle avoidance)
///
/// Rather than calling `strategy::vol_estimator::ewma_realized_vol`, we inline
/// the identical 3-line recurrence to avoid a `cost → strategy` dep cycle.
/// (`strategy` dev-depends on `cost`; adding the reverse dep creates a cycle.)
/// ADR-0081 § D1 records this divergence explicitly.
///
/// The recurrence is:
/// ```text
/// σ²_t = (1 − λ) · r_t² + λ · σ²_{t-1}
/// σ̂   = √(σ²_T)   (last element of the EWMA series)
/// ```
/// Initialised with the population variance of the `sigma_window`-length suffix
/// of `returns` (or all available returns if fewer). Falls back to `base_bps`
/// when the returns slice is empty (warm-up / not enough data).
///
/// # Units
///
/// - `returns`: dimensionless log-returns (e.g. 0.01 = 1 % per bar).
/// - `σ̂`: dimensionless per-bar vol (same units as returns).
/// - `vol_multiplier · σ̂ · 10_000`: converts σ̂ to bps-scale.
/// - `effective_bps`: u32 basis-points, capped at `MAX_SLIPPAGE_BPS`.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
#[allow(clippy::float_arithmetic)] // statistical computation — intentional
pub(crate) fn apply_slippage_vol_scaled_bps(
    returns: &[f64],
    base_bps: u32,
    vol_multiplier: f64,
    sigma_window: usize,
    sigma_lambda: f64,
) -> u32 {
    // Clamp λ to [0, 1].
    let lam = sigma_lambda.clamp(0.0, 1.0);

    if returns.is_empty() || sigma_window == 0 {
        return base_bps;
    }

    // Take the last `sigma_window` returns (or all if fewer).
    let n = sigma_window.min(returns.len());
    let slice = &returns[returns.len() - n..];

    // Seed with population variance of the slice.
    let mean: f64 = slice.iter().sum::<f64>() / (slice.len() as f64);
    let var0: f64 =
        slice.iter().map(|&r| (r - mean) * (r - mean)).sum::<f64>() / (slice.len() as f64);
    let mut sigma2 = if var0 > 0.0 {
        var0
    } else {
        // Constant/zero returns — use first squared return or tiny epsilon.
        let r0sq = slice[0] * slice[0];
        if r0sq > 0.0 { r0sq } else { 1e-16_f64 }
    };

    // EWMA recurrence (identical to strategy::vol_estimator::ewma_realized_vol).
    for &r in slice {
        sigma2 = (1.0 - lam) * r * r + lam * sigma2;
    }

    let sigma_hat: f64 = sigma2.sqrt(); // dimensionless per-bar vol

    // effective_bps = base + vol_multiplier · σ̂ · 10_000
    let bps_add: f64 = vol_multiplier * sigma_hat * 10_000.0_f64;
    let effective_f64: f64 = f64::from(base_bps) + bps_add;

    // Clamp to [base_bps, MAX_SLIPPAGE_BPS].
    if effective_f64 >= f64::from(MAX_SLIPPAGE_BPS) {
        MAX_SLIPPAGE_BPS
    } else if effective_f64 <= f64::from(base_bps) {
        base_bps
    } else {
        effective_f64.round() as u32
    }
}

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

    // ── VolScaledSpread unit tests (ADR-0081) ─────────────────────────────────

    /// default_is_linear: SlippageModel::default() == Linear { bps: 8 }.
    /// LOAD-BEARING backward-compat proof (D6 contract — NEVER regress).
    #[test]
    fn default_is_linear_bps_8() {
        let default = SlippageModel::default();
        assert!(
            matches!(default, SlippageModel::Linear { bps: 8 }),
            "SlippageModel::default() MUST be Linear{{bps:8}} — D6 contract; got {default:?}"
        );
    }

    /// vol_scaled_zero_vol_gives_base_bps: zero-vol input → spread = base_bps only.
    #[test]
    fn vol_scaled_zero_vol_gives_base_bps() {
        // A constant price series: zero log-returns → σ̂ = 0 → effective = base only.
        let returns_constant = vec![0.0_f64; 30];
        let effective_bps = apply_slippage_vol_scaled_bps(&returns_constant, 8, 2.0, 20, 0.94);
        assert_eq!(
            effective_bps, 8,
            "constant returns → σ̂=0 → effective_bps must equal base_bps=8; got {effective_bps}"
        );
    }

    /// vol_scaled_empty_returns_gives_base_bps: empty returns → warm-up fallback → base_bps.
    #[test]
    fn vol_scaled_empty_returns_gives_base_bps() {
        let effective_bps = apply_slippage_vol_scaled_bps(&[], 8, 2.0, 20, 0.94);
        assert_eq!(
            effective_bps, 8,
            "empty returns → warm-up fallback → base_bps=8; got {effective_bps}"
        );
    }

    /// vol_scaled_constant_vol_closed_form: constant-vol input → spread = base + vol_mult · σ̂.
    ///
    /// Given returns all equal to `r`, the EWMA settles to σ̂ = |r|.
    /// effective_bps = base + vol_multiplier · |r| · 10_000.
    #[test]
    fn vol_scaled_constant_vol_closed_form() {
        // Constant return = 0.01 (1 % per bar).
        // Population variance = 0 (all equal), so seed falls back to r^2 = 0.0001.
        // EWMA with λ=0.0 (instant reactivity) settles immediately: σ²_t = r² = 0.0001, σ̂ = 0.01.
        // effective_bps = 8 + 2.0 · 0.01 · 10_000 = 8 + 200 = 208.
        let returns = vec![0.01_f64; 30];
        let effective_bps = apply_slippage_vol_scaled_bps(&returns, 8, 2.0, 30, 0.0);
        // With λ=0: EWMA = current return squared → σ̂ = 0.01 exactly.
        assert_eq!(
            effective_bps, 208,
            "constant 1% returns, λ=0: expected 8 + 2.0*0.01*10000 = 208; got {effective_bps}"
        );
    }

    /// vol_scaled_high_vol_widens_vs_low_vol: high-vol regime → wider spread than low-vol.
    #[test]
    fn vol_scaled_high_vol_widens_vs_low_vol() {
        // Low vol: 0.1% per bar returns.
        let low_vol_returns = vec![0.001_f64; 30];
        let bps_low = apply_slippage_vol_scaled_bps(&low_vol_returns, 8, 2.0, 20, 0.94);

        // High vol: 5% per bar returns (simulating a stress regime).
        let high_vol_returns = vec![0.05_f64; 30];
        let bps_high = apply_slippage_vol_scaled_bps(&high_vol_returns, 8, 2.0, 20, 0.94);

        assert!(
            bps_high > bps_low,
            "high-vol regime must widen spread: bps_high={bps_high} vs bps_low={bps_low}"
        );
        // Spread must widen meaningfully (at least 2x the base_bps for 5% vol)
        assert!(
            bps_high > bps_low + 8,
            "high-vol spread must exceed low-vol by more than the base: {bps_high} vs {bps_low}"
        );
    }

    /// vol_scaled_capped_at_max: very-high vol input → capped at MAX_SLIPPAGE_BPS.
    #[test]
    fn vol_scaled_capped_at_max_slippage_bps() {
        // Extreme volatility: 50% per bar — would give huge bps, must cap.
        let extreme_vol_returns = vec![0.50_f64; 30];
        let bps = apply_slippage_vol_scaled_bps(&extreme_vol_returns, 8, 100.0, 20, 0.0);
        assert_eq!(
            bps, MAX_SLIPPAGE_BPS,
            "extreme vol must cap at MAX_SLIPPAGE_BPS={MAX_SLIPPAGE_BPS}; got {bps}"
        );
    }

    /// vol_scaled_fill_price_buy_increases: VolScaledSpread on buy → fill_price > signal.
    #[test]
    fn vol_scaled_fill_price_buy_increases() {
        let price = dec!(50_000.00);
        let returns = vec![0.02_f64; 30]; // 2% per bar — meaningful vol
        let fill = apply_slippage_model_with_returns(
            price,
            Side::Buy,
            Decimal::ZERO,
            SlippageModel::VolScaledSpread {
                base_bps: 8,
                vol_multiplier: 2.0,
                sigma_window: 20,
                sigma_lambda: 0.94,
            },
            Decimal::ZERO,
            &returns,
        );
        assert!(
            fill > price,
            "VolScaledSpread buy must increase fill price; fill={fill}, signal={price}"
        );
    }

    /// vol_scaled_fill_price_sell_decreases: VolScaledSpread on sell → fill_price < signal.
    #[test]
    fn vol_scaled_fill_price_sell_decreases() {
        let price = dec!(50_000.00);
        let returns = vec![0.02_f64; 30];
        let fill = apply_slippage_model_with_returns(
            price,
            Side::Sell,
            Decimal::ZERO,
            SlippageModel::VolScaledSpread {
                base_bps: 8,
                vol_multiplier: 2.0,
                sigma_window: 20,
                sigma_lambda: 0.94,
            },
            Decimal::ZERO,
            &returns,
        );
        assert!(
            fill < price,
            "VolScaledSpread sell must decrease fill price; fill={fill}, signal={price}"
        );
    }

    /// vol_scaled_widens_vs_linear_on_volatile_returns.
    /// High vol → vol-scaled fill differs from linear at same base_bps.
    #[test]
    fn vol_scaled_widens_vs_linear_on_volatile_returns() {
        let price = dec!(50_000.00);
        let high_vol_returns = vec![0.05_f64; 30]; // 5% per bar

        let fill_vol_scaled = apply_slippage_model_with_returns(
            price,
            Side::Buy,
            Decimal::ZERO,
            SlippageModel::VolScaledSpread {
                base_bps: 8,
                vol_multiplier: 2.0,
                sigma_window: 20,
                sigma_lambda: 0.94,
            },
            Decimal::ZERO,
            &high_vol_returns,
        );
        let fill_linear = apply_slippage_model(
            price,
            Side::Buy,
            Decimal::ZERO,
            SlippageModel::Linear { bps: 8 },
            Decimal::ZERO,
        );
        assert!(
            fill_vol_scaled > fill_linear,
            "VolScaledSpread must give a worse fill than Linear on high-vol returns; \
             vol_scaled={fill_vol_scaled}, linear={fill_linear}"
        );
    }

    /// anchor_safety_proof: Linear{bps:8} produces the SAME fill before and after
    /// the VolScaledSpread variant was added. This proves opt-in-forever holds.
    #[test]
    fn anchor_safety_linear_unchanged_by_vol_scaled_variant() {
        let price = dec!(45_321.75);
        // The pre-existing Linear path must be byte-identical regardless of the
        // VolScaledSpread variant existing in the same enum.
        let result_default = apply_slippage_model(
            price,
            Side::Buy,
            dec!(1_000_000),
            SlippageModel::default(), // == Linear { bps: 8 }
            Decimal::ZERO,
        );
        let result_explicit_linear = apply_slippage_model(
            price,
            Side::Buy,
            dec!(1_000_000),
            SlippageModel::Linear { bps: 8 },
            Decimal::ZERO,
        );
        assert_eq!(
            result_default, result_explicit_linear,
            "default() and Linear{{bps:8}} must be byte-identical (anchor-safety proof)"
        );
        // Sanity-check the value is what we expect: 8 bps on 45_321.75 buy.
        // 45_321.75 * (1 + 8/10_000) = 45_321.75 * 1.0008 = 45_357.9666
        // Use Decimal::from_str for precision.
        let expected_fill = dec!(45_321.75) * (rust_decimal::Decimal::ONE + dec!(8) / dec!(10_000));
        assert_eq!(
            result_default, expected_fill,
            "Linear{{bps:8}} fill must equal exact Decimal arithmetic; got {result_default}"
        );
    }

    // ── Fee-sensitivity report tests ──────────────────────────────────────────

    /// fee_sensitivity_report_zero_vol: σ̂=0 → all entries = base_bps.
    #[test]
    fn fee_sensitivity_report_zero_vol() {
        let results = fee_sensitivity_report(8, 0.0, &[1.0, 2.0, 3.0]);
        assert_eq!(results.len(), 3);
        for (factor, eff_bps) in &results {
            assert!(
                (eff_bps - 8.0).abs() < 1e-10,
                "σ̂=0: factor={factor} → effective_bps must be 8.0 (base), got {eff_bps}"
            );
        }
    }

    /// fee_sensitivity_report_known_value: σ̂=0.01, mult=2.0 → 8 + 200 = 208.
    #[test]
    fn fee_sensitivity_report_known_value() {
        let results = fee_sensitivity_report(8, 0.01, &[1.0, 2.0, 3.0]);
        let (mult1, eff1) = results[0]; // factor 1.0 → 8 + 100 = 108
        let (mult2, eff2) = results[1]; // factor 2.0 → 8 + 200 = 208
        let (mult3, eff3) = results[2]; // factor 3.0 → 8 + 300 = 308

        assert!((mult1 - 1.0).abs() < 1e-10);
        assert!((eff1 - 108.0).abs() < 1e-6, "1× σ̂: got {eff1}");
        assert!((mult2 - 2.0).abs() < 1e-10);
        assert!((eff2 - 208.0).abs() < 1e-6, "2× σ̂: got {eff2}");
        assert!((mult3 - 3.0).abs() < 1e-10);
        assert!((eff3 - 308.0).abs() < 1e-6, "3× σ̂: got {eff3}");
    }

    /// fee_sensitivity_report_capped: huge σ̂ → capped at MAX_SLIPPAGE_BPS.
    #[test]
    fn fee_sensitivity_report_capped_at_max() {
        // σ̂ = 1.0 (100% per bar), multiplier=100 → 8 + 100*1.0*10_000 >> MAX_SLIPPAGE_BPS
        let results = fee_sensitivity_report(8, 1.0, &[100.0]);
        let (_, eff_bps) = results[0];
        assert_eq!(
            eff_bps,
            f64::from(MAX_SLIPPAGE_BPS),
            "extreme σ̂ must be capped at MAX_SLIPPAGE_BPS; got {eff_bps}"
        );
    }

    /// fee_sensitivity_report_empty_factors: empty input → empty output.
    #[test]
    fn fee_sensitivity_report_empty_factors() {
        let results = fee_sensitivity_report(8, 0.01, &[]);
        assert!(results.is_empty(), "empty factors → empty output");
    }

    /// default_vol_scaled_spread_constant: DEFAULT_VOL_SCALED_SPREAD has expected fields.
    #[test]
    fn default_vol_scaled_spread_constant_fields() {
        assert!(matches!(
            DEFAULT_VOL_SCALED_SPREAD,
            SlippageModel::VolScaledSpread {
                base_bps: 8,
                vol_multiplier: _,
                sigma_window: 20,
                sigma_lambda: _,
            }
        ));
        if let SlippageModel::VolScaledSpread {
            base_bps,
            vol_multiplier,
            sigma_window,
            sigma_lambda,
        } = DEFAULT_VOL_SCALED_SPREAD
        {
            assert_eq!(base_bps, 8);
            assert!((vol_multiplier - 2.0).abs() < 1e-10);
            assert_eq!(sigma_window, 20);
            assert!((sigma_lambda - 0.94).abs() < 1e-10);
        }
    }
}
