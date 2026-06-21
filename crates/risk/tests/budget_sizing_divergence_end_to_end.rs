//! Day-1 baseline-equity-divergence e2e — F4 budget-aware sizing (M-DEV-F4.1).
//!
//! Asserts that a `FixedFractionSizer::with_budget_cap(fraction, 200 USDT)` run
//! produces a **different return path** (≥ 1 bp) from an un-capped
//! `FixedFractionSizer::new(fraction)` run — i.e. the budget cap genuinely
//! changes sizing behaviour and is not a no-op.
//!
//! # Why the cap binds in this fixture
//!
//! We use `fraction = 0.95` and `per_symbol_exposure_cap = 1.0` (no portfolio
//! limit) so the exposure cap never fires first.  Starting equity for the budget
//! arm is `200 USDT` (== budget).  On the first bar the deployed notional is
//! `200 × 0.95 = 190 USDT < 200 USDT` (budget slack) — but after the first
//! winning trade (1 % gain) equity becomes ≈ `201.9 USDT > 200`.  On bar 2
//! the fraction would deploy `201.9 × 0.95 ≈ 191.8 USDT` — still below budget.
//!
//! Key: the cap fires when `equity × fraction > budget`, i.e.
//! `equity > budget / fraction = 200 / 0.95 ≈ 210.5 USDT`.  So after ~5 winning
//! cycles of 10 % gain per cycle the budget arm's equity exceeds 210.5 USDT and
//! the cap starts to bind, limiting every subsequent BUY to 200/50_000 = 0.004 BTC
//! notional while the baseline arm (100_000 USDT, no cap) scales freely.
//!
//! We use 10 cycles with a 10 % gain per cycle to ensure the cap fires by cycle 2
//! on the baseline-normalised path (which starts at 100_000 × 0.95 notional on
//! cycle 1, unrestricted).  The result is that the budget arm's return path
//! diverges from the baseline's normalised path by well above 1 bp.
//!
//! # Forensic gate
//!
//! Run this test against the **stub** (field exists, `with_budget_cap` compiles,
//! but the clamp is NOT applied inside `compute_qty`).
//!
//! Expected FAIL-before:
//! ```text
//! assertion failed: budget cap changed the return path by >= 1 bp ...
//! divergence: 0.00000000 (need >= 0.0001)
//! ```
//! — because without the clamp both arms scale at identical fractions of their
//!   respective equity, yielding the same return percentage each cycle.
//!
//! After the real clamp lands (M-DEV-F4.2), the budget arm's BUY qty is capped
//! at `200 / price` once equity grows past `budget / fraction`, so it compounds
//! more slowly — the divergence asserted below PASSES.
//!
//! # Cross-references
//!
//! - `spec/advisor-forward-paper/feature.md` § 3 — the gate contract.
//! - `spec/architecture/adr/0060-budget-aware-sizing-and-forward-paper-run-seam.md` D2.
//! - `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` — the precedent.

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{Money, RiskLimits, Usdt};

use risk::FixedFractionSizer;

// ── Fixture parameters ────────────────────────────────────────────────────────

/// Large fraction (95 %) so the budget cap binds before the exposure-cap would
/// at the budget arm's equity level.  The exposure cap is set to 1.0 so it never
/// fires first (the budget cap is the operative limit).
const FRACTION: Decimal = dec!(0.95);

/// Budget (USDT).  The cap fires when `equity > budget / fraction ≈ 210.5 USDT`.
const BUDGET: Decimal = dec!(200);

/// Baseline arm starting cash: large enough that the budget cap is always slack
/// for the baseline (100_000 × 0.95 = 95_000 USDT notional — no cap → scales
/// freely with equity growth).
const BASELINE_CASH: Decimal = dec!(100_000);

/// Simulated fill price for all bars (constant, isolates sizing math).
const PRICE: Decimal = dec!(50_000);

/// Gain per sell cycle: 10 % above the buy price.  After 2 cycles the budget
/// arm's equity exceeds `budget / fraction`, so the cap starts binding by cycle 3.
const GAIN_PER_CYCLE: Decimal = dec!(0.10);

/// Number of buy-then-sell cycles.  10 cycles gives ≥ 7 cap-binding cycles for
/// the budget arm and a large cumulative divergence from the uncapped baseline.
const CYCLES: u32 = 10;

/// One basis point in return-space.
const ONE_BP: Decimal = dec!(0.0001);

// ── Helpers ───────────────────────────────────────────────────────────────────

fn risk_limits() -> RiskLimits {
    RiskLimits {
        // Set exposure cap to 1.0 (100 %) so the budget cap is the first to bind.
        per_symbol_exposure_cap: dec!(1.0),
        price_sanity_band: dec!(0.10),
        portfolio_exposure_cap: None,
    }
}

/// Simulate a series of BUY-then-SELL cycles over a constant price fixture.
///
/// Each cycle:
/// 1. BUY at `PRICE`: compute qty from the sizer, deduct notional from cash,
///    add qty to the position.
/// 2. SELL at `PRICE * (1 + GAIN_PER_CYCLE)`: realise the gain.
///
/// Returns `(final_equity, fill_count)`.
///
/// This deliberately does NOT use `spawn_trading_loop` — the gate is on
/// `compute_qty` and the equity arithmetic, matching the vol-targeting precedent.
fn simulate_sizing_path(
    sizer: &FixedFractionSizer,
    starting_cash: Decimal,
    limits: &RiskLimits,
) -> (Decimal, u32) {
    let mut cash = starting_cash;
    let mut position_qty = Decimal::ZERO;
    let mut fill_count: u32 = 0;

    for _cycle in 0..CYCLES {
        // Mark-to-market equity before the BUY.
        let equity_d = cash + position_qty * PRICE;
        let equity = Money::<Usdt>::from_decimal(equity_d);

        // BUY: compute qty with the sizer (respects budget cap when Some).
        if let Ok(qty_obj) = sizer.compute_qty(equity, PRICE, limits) {
            let qty = qty_obj.get();
            if qty > Decimal::ZERO {
                let notional = qty * PRICE;
                cash -= notional;
                position_qty += qty;
                fill_count += 1;
            }
        }

        // SELL at a 10 % gain: realise the position.
        let sell_price = PRICE * (Decimal::ONE + GAIN_PER_CYCLE);
        if position_qty > Decimal::ZERO {
            let proceeds = position_qty * sell_price;
            cash += proceeds;
            position_qty = Decimal::ZERO;
            fill_count += 1;
        }
    }

    // Final mark-to-market (position should be 0 after the last SELL).
    let final_equity = cash + position_qty * PRICE;
    (final_equity, fill_count)
}

// ── Forensic gate test ────────────────────────────────────────────────────────

#[test]
fn budget_cap_changes_return_path_vs_uncapped_baseline() {
    let limits = risk_limits();

    // Baseline arm: large capital, no budget cap.
    let baseline_sizer = FixedFractionSizer::new(FRACTION);
    let (baseline_final, baseline_fills) =
        simulate_sizing_path(&baseline_sizer, BASELINE_CASH, &limits);

    // Budget arm: 200 USDT starting capital + 200 USDT hard notional cap.
    let budget = Money::<Usdt>::from_decimal(BUDGET);
    let budget_sizer = FixedFractionSizer::with_budget_cap(FRACTION, budget);
    let (budget_final, budget_fills) = simulate_sizing_path(&budget_sizer, BUDGET, &limits);

    // Verify the decision variable is non-trivial: both arms produced fills.
    assert!(
        baseline_fills >= 2,
        "baseline arm produced < 2 fills ({baseline_fills}) — fixture is broken"
    );
    assert!(
        budget_fills >= 2,
        "budget arm produced < 2 fills ({budget_fills}) — fixture is broken"
    );

    // Compute normalised returns (removes the 500× starting-capital ratio).
    let baseline_return = (baseline_final - BASELINE_CASH) / BASELINE_CASH;
    let budget_return = (budget_final - BUDGET) / BUDGET;

    // Under a NO-OP stub: both arms deploy `equity × fraction / price` units per
    // BUY → the same return % each cycle → divergence = 0 → assertion FAILS.
    //
    // Under the REAL clamp: once the budget arm's equity exceeds `budget / fraction
    // ≈ 210.5 USDT` the BUY qty is capped at `200 / 50_000 = 0.004 BTC` instead
    // of `equity × 0.95 / 50_000` → budget arm earns less per cycle → cumulative
    // return diverges from baseline → divergence >> 1 bp → assertion PASSES.
    let divergence = (baseline_return - budget_return).abs();

    assert!(
        divergence >= ONE_BP,
        "budget cap changed the return path by >= 1 bp — FAIL means the cap is a no-op.\n\
         baseline return: {baseline_return:.8} (final equity {baseline_final:.6} from {BASELINE_CASH})\n\
         budget return:   {budget_return:.8} (final equity {budget_final:.6} from {BUDGET})\n\
         divergence:      {divergence:.8} (need >= {ONE_BP})\n\
         baseline fills: {baseline_fills}, budget fills: {budget_fills}\n\
         \n\
         This is the M-DEV-F4.1 forensic gate.  FAIL means `compute_qty` does not\n\
         apply the budget clamp.  Once M-DEV-F4.2 lands (the real clamp), the budget\n\
         arm is constrained to qty ≤ budget/price once equity > budget/fraction, and\n\
         this test PASSES."
    );
}
