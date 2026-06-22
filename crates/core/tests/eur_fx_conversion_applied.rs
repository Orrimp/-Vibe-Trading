//! Day-1 EUR→USDT conversion-applied gate (ADR-0065 § D4, CLAUDE.md non-negotiable).
//!
//! FAIL-before / PASS-after design:
//!   - Written against the un-converted (1:1) code so assertions (1) and (2) FAIL.
//!   - Once `FxRate::convert_eur_to_usdt` applies the real multiply, PASS.
//!
//! Three assertions, per the spec:
//!
//! **(1) Converted ≠ 1:1 (the no-op guard)**
//!   `FxRate::config(1.08).convert_eur_to_usdt(200)` must equal 216, not 200.
//!   Negative control: `rate = 1.0` → converted == 200 (1:1 by design).
//!
//! **(2) The converted value REACHES F4 (the load-bearing assertion)**
//!   Feed `conversion.usdt()` into `FixedFractionSizer::with_budget_cap` and assert
//!   the effective cap is 216 (not 200). This closes the "computed-then-dropped"
//!   hole — the exact v3-vol-overlay-noop failure mode the non-negotiable exists to catch.
//!
//! **(3) Display ↔ engine agreement**
//!   The value the display would format (via `conversion.usdt().amount()`) is
//!   byte-identical to the value F4 was given. Since both read the same
//!   `BudgetConversion`, this is structurally true; the test pins it against regression.

#[allow(clippy::unwrap_used, clippy::expect_used)]
mod eur_fx_gate {
    use risk::FixedFractionSizer;
    use rust_decimal::Decimal;
    use rust_decimal_macros::dec;
    use trading_core::{BudgetConversion, FxRate, Money, RiskLimits, Usdt};

    // ── (1) Converted ≠ 1:1 (no-op guard) ───────────────────────────────────

    #[test]
    fn converted_amount_equals_eur_times_rate() {
        let rate = FxRate::config(dec!(1.08));
        let usdt = rate.convert_eur_to_usdt(dec!(200));
        // €200 × 1.08 = $216
        assert_eq!(
            usdt.amount(),
            dec!(216),
            "conversion must apply the multiply: 200 * 1.08 = 216"
        );
    }

    #[test]
    fn converted_amount_is_not_equal_to_raw_eur_with_non_unit_rate() {
        // This is the no-op guard: a 1:1 stub would return 200, not 216.
        // The test FAILS if convert_eur_to_usdt ignores the rate.
        let rate = FxRate::config(dec!(1.08));
        let usdt = rate.convert_eur_to_usdt(dec!(200));
        assert_ne!(
            usdt.amount(),
            dec!(200),
            "a non-unit rate must produce a value ≠ the raw EUR input (the no-op guard)"
        );
    }

    /// Negative control: `rate = 1.0` → converted == raw EUR (1:1 by design).
    /// This must PASS both before and after the implementation.
    #[test]
    fn unit_rate_is_identity_negative_control() {
        let rate = FxRate::config(dec!(1.0));
        let usdt = rate.convert_eur_to_usdt(dec!(200));
        assert_eq!(
            usdt.amount(),
            dec!(200),
            "rate = 1.0 is the identity — converted must equal raw EUR"
        );
    }

    // ── (2) The converted value REACHES F4 (the load-bearing assertion) ──────

    #[test]
    fn converted_usdt_is_the_value_f4_caps_against() {
        // Build the carrier the same way the seam (cockpit_live.rs) does.
        let fx = FxRate::config(dec!(1.08));
        let conversion = BudgetConversion::new(dec!(200), fx);

        // Engine path: feed conversion.usdt() into F4.
        let budget_for_f4: Money<Usdt> = conversion.usdt();
        let sizer = FixedFractionSizer::with_budget_cap(dec!(1.0), budget_for_f4);

        // Assert the cap amount is 216 (not 200).
        // budget_cap is Some(216 USDT) — F4 will clamp at 216, not 200.
        let cap = sizer
            .budget_cap
            .expect("sizer must have a budget cap after with_budget_cap");
        assert_eq!(
            cap.amount(),
            dec!(216),
            "F4 budget cap must be the converted value (216), not the raw EUR (200)"
        );
        assert_ne!(
            cap.amount(),
            dec!(200),
            "F4 must NOT cap at the raw EUR amount — that is the 1:1 no-op bug"
        );

        // Also verify via a compute_qty call that the cap binds at 216, not 200.
        // Price = 1.0 USDT/unit, equity large enough that fraction is not the binding constraint.
        // With budget_cap = 216 and price = 1.0, max_qty_budget = 216 / 1.0 = 216.
        // With budget_cap = 200 and price = 1.0, max_qty_budget would be 200 — wrong.
        let equity = Money::<Usdt>::from_decimal(dec!(10000)); // large equity, cap binds
        let limits = RiskLimits::default(); // 40% per-symbol cap >> budget
        let qty = sizer
            .compute_qty(equity, dec!(1), &limits)
            .expect("compute_qty must succeed at price=1, large equity");
        // qty * price = qty * 1.0 = qty. Budget cap = 216 → qty ≤ 216.
        // If the bug existed (cap = 200), qty would be ≤ 200.
        assert!(
            qty.get() <= dec!(216),
            "qty must be at most 216 (the converted budget cap)"
        );
        assert!(
            qty.get() > dec!(200),
            "qty must exceed 200 — if it is ≤ 200 the cap is still at the 1:1 value (the bug)"
        );
    }

    // ── (3) Display ↔ engine agreement ───────────────────────────────────────

    #[test]
    fn display_and_engine_read_the_same_converted_value() {
        // Both engine and display should read the same BudgetConversion.
        let fx = FxRate::config(dec!(1.08));
        let conversion = BudgetConversion::new(dec!(200), fx);

        // Engine value (what ForwardRunConfig.budget is set to).
        let engine_budget: Money<Usdt> = conversion.usdt();

        // Display value (what the FX-note string formats from).
        // Since both call conversion.usdt() on the same struct, they are identical.
        let display_amount: Decimal = conversion.usdt().amount();

        assert_eq!(
            engine_budget.amount(),
            display_amount,
            "engine and display must read the same converted value from BudgetConversion"
        );
        // Pinned: both are 216.
        assert_eq!(display_amount, dec!(216));
    }

    // ── Grep guard meta-test: the multiply is in exactly one place ────────────

    /// This is a documentation test — the real grep guard is in T7 (CI / precheck).
    /// Here we assert structurally: `BudgetConversion::new` is the only call site
    /// for the arithmetic; the display doesn't re-multiply.
    #[test]
    fn budget_conversion_encapsulates_the_single_multiply() {
        // If a second conversion site existed, it would drift from this one.
        // The test is "structurally guaranteed" because `BudgetConversion::usdt()`
        // returns a pre-computed field, not a fresh multiply.
        let fx = FxRate::config(dec!(1.08));
        let c1 = BudgetConversion::new(dec!(200), fx.clone());
        let c2 = BudgetConversion::new(dec!(200), fx);
        // Two conversions with the same inputs agree.
        assert_eq!(c1.usdt().amount(), c2.usdt().amount());
        // And they equal the direct multiplication (structural check).
        let direct = dec!(200) * dec!(1.08);
        assert_eq!(c1.usdt().amount(), direct);
    }
}
