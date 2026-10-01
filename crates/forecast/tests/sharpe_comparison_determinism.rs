//! The ratified annualisation constant, pinned so a "correction" to √8760 goes RED.
//!
//! ## What this file used to be, and why it is not that any more (bug-log `#136`)
//!
//! It held a **test-local re-implementation** of `sharpe_comparison`'s renderer —
//! `fn render_report(results: &[RerunResult; 4], …)` plus mirrored `RerunResult` / `ReportContext`
//! types — and a test that rendered that copy twice and compared the two renders. Two consequences:
//!
//! - It could not fail in the way its name suggested. It proved a function the product never calls
//!   is deterministic. The file's own docstring said as much about its other half: *"Changing the
//!   constant changes both sides equally, so it stays green — a determinism tautology."*
//! - **The copy had drifted, measured not feared**: its notes line read *"the four -realdata
//!   reports"* against the bin's *"five"*, and its signature took `[RerunResult; 4]` against an arm
//!   that re-runs five scenarios. Nothing noticed, because the only thing comparing the two was the
//!   copy against itself.
//!
//! Retired 2026-10-01, under the one condition that makes deleting a test safe: **something real
//! replaced it.** The three `sharpe-comparison-*` rows now have reproduction gates in
//! `anchored_report_reproduction.rs` that re-run the **actual** binary and compare against the
//! **anchored** body — and on 2026-10-01 all three reported, measuring a real drift the
//! self-comparison was structurally incapable of seeing. A red gate is coverage; a green tautology
//! is not.
//!
//! `#133` fixed this file's other half the right way (the metrics moved to `forecast::metrics` so
//! bin and test share one definition). This is the same move finished.

use forecast::metrics::SQRT_HOURS_PER_YEAR;

/// Story 1-25 AC3.1, made enforceable — the ratification that was only ever a comment.
///
/// AC3.1 permits either moving to √8760 or *"a formal ratification of √8575 with the doc
/// corrected to match"*. The shipped constant has always been the latter; what was missing was
/// anything that could go RED if someone "corrected" it to the value the docs claimed.
///
/// This test compares against a PINNED number, which is why it can fail.
#[test]
fn sqrt_hours_per_year_is_the_ratified_8575_constant_and_not_8760() {
    let squared = SQRT_HOURS_PER_YEAR * SQRT_HOURS_PER_YEAR;

    assert!(
        (squared - 8_574.999_8).abs() < 1e-3,
        "SQRT_HOURS_PER_YEAR squared is {squared}, expected 8574.9998. This constant is \
         load-bearing for anchored sharpe-comparison bodies: changing it re-prices every \
         one of them. If the change is intended, re-lock the affected anchors under \
         ADR-0038 § D6.b and update this pin in the same commit."
    );

    let sqrt_8760 = (24.0_f64 * 365.0).sqrt();
    assert!(
        (SQRT_HOURS_PER_YEAR - sqrt_8760).abs() > 0.9,
        "SQRT_HOURS_PER_YEAR has been set to √(24·365) = {sqrt_8760}. That is the value the \
         doc comments asserted until 2026-09-29 while the code shipped √8574.9998 — a ~2.1% \
         difference. Adopting it is a deliberate re-pricing, not a typo fix."
    );
}
