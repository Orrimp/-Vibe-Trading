//! The advisor path's block-length direction gate — story 1-24 AC5, bug-log `#139`.
//!
//! ## Why this file exists
//!
//! `bakeoff/bootstrap.rs:139` feeds `politis_white_block_length` into
//! **`compute_robustness_flag`**, one of AD-1's four FROZEN gate functions. So the block-length
//! estimator reaches `classify_verdict` and crown eligibility in `rank_candidates`: story 1-24
//! altered ranking **not by editing a frozen function but by moving what it is fed.**
//!
//! `#139` records how close that came to being missed — the call is written through a crate name,
//! so CodeGraph cannot see it, and the one line of `scripts/callers.sh` output that did report it
//! was cut off by a `tail`.
//!
//! AD-1 asks an analytics change to prove it does not alter ranking. Here an identity test is not
//! available: the change *does* alter what the gate is fed, so such a test would either fail
//! honestly or be written to compare the new behaviour against itself. **This measures the
//! direction instead**, which is the property the thesis actually rests on.
//!
//! ## What the advisor path is, precisely
//!
//! A **single** equity-derived log-return series — `equity_to_log_returns_f64(equity)` — not the
//! multi-symbol panel the θ-surfaces resample. So story 1-24's AC2 aggregation change
//! (per-symbol estimate, `max` reduction) **cannot** affect the advisor: that code runs only in
//! `synth::bootstrap`'s `Auto` arm. Only the AC1 window/search convention reaches here. Stating
//! that narrows the blast radius to one cause and is why this file tests variants rather than
//! policies.
//!
//! ## What a RED here means
//!
//! Longer `L` ⇒ wider bootstrap null ⇒ harder for a candidate to clear the bar ⇒ **fewer** crowns
//! ⇒ the advisor says "just hold" more often. That is the thesis-safe direction. A shorter `L`
//! under the literature convention would mean the fidelity fix made the advisor **more** likely to
//! crown an active arm, which is a REGRESSION-class finding and belongs in front of the operator
//! before it ships — the same shape as story 1-24 AC4's guard, which tripped on 2 of 35 surfaces
//! precisely because its direction claim was an inference rather than a theorem.

#![allow(clippy::unwrap_used)]

use data::synth::block_length::{PwsdVariant, politis_white_block_length_variant};

/// Equity-derived log-returns, the way `bakeoff/bootstrap.rs` builds them: `ln(e[k+1]/e[k])`.
fn log_returns_from_equity(equity: &[f64]) -> Vec<f64> {
    equity.windows(2).map(|w| (w[1] / w[0]).ln()).collect()
}

/// A deterministic equity curve: geometric drift plus an alternating shock, so the returns carry
/// real variance. A constant-factor curve has zero return variance and would make every estimate
/// degenerate — the trap that made story 1-24's first scale-relative test pass for the wrong
/// reason.
fn equity_curve(n: usize, drift: f64, shock: f64, seed: u64) -> Vec<f64> {
    let mut e = vec![100_000.0_f64];
    let mut s = seed;
    for _ in 0..n {
        // xorshift — deterministic, no rand dependency, adequate for a fixture.
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let u = ((s >> 11) as f64) / ((1_u64 << 53) as f64); // [0,1)
        let step = 1.0 + drift + shock * (u - 0.5);
        e.push((e.last().unwrap() * step).max(1e-6));
    }
    e
}

/// An equity curve whose RETURNS are AR(1) — what a real strategy produces, and what makes this
/// test non-vacuous.
///
/// The first version of this file used only an i.i.d.-shock curve and every case came back
/// `L = 1` under both conventions: a pass that proved nothing, because a near-white series has no
/// dependence for the estimator to find. Caught by reading the printed numbers rather than the
/// green verdict. Real advisor equity carries volatility clustering and trend, so the fixture has
/// to as well or the direction gate is measuring the degenerate case.
fn equity_curve_ar1(n: usize, phi: f64, sigma: f64, seed: u64) -> Vec<f64> {
    let mut e = vec![100_000.0_f64];
    let mut r_prev = 0.0_f64;
    let mut s = seed;
    for _ in 0..n {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        let u = ((s >> 11) as f64) / ((1_u64 << 53) as f64);
        let eps = (u - 0.5) * 2.0;
        let r = phi * r_prev + (1.0 - phi * phi).sqrt() * eps * sigma;
        r_prev = r;
        e.push((e.last().unwrap() * (1.0 + r)).max(1e-6));
    }
    e
}

#[test]
fn literature_convention_never_shortens_l_on_the_advisor_path() {
    // Shapes an advisor bakeoff actually produces: a year of hourly bars down to a short
    // backtest, drifting up and down, calm and volatile.
    let cases: Vec<(&str, Vec<f64>)> = vec![
        (
            "hourly-year-grow",
            equity_curve(8759, 0.000_05, 0.004, 0xA1),
        ),
        (
            "hourly-year-decline",
            equity_curve(8759, -0.000_05, 0.004, 0xB2),
        ),
        ("hourly-year-volatile", equity_curve(8759, 0.0, 0.020, 0xC3)),
        ("hourly-quarter", equity_curve(2190, 0.000_05, 0.006, 0xD4)),
        ("daily-year", equity_curve(364, 0.000_5, 0.020, 0xE5)),
        ("short-500", equity_curve(500, 0.0, 0.010, 0xF6)),
        ("short-120", equity_curve(120, 0.001, 0.015, 0x17)),
        // AR(1) returns — the non-degenerate cases. Without these the gate measures only
        // near-white curves, where both conventions trivially agree at L = 1.
        (
            "ar1-phi.3-hourly-year",
            equity_curve_ar1(8759, 0.3, 0.004, 0x21),
        ),
        (
            "ar1-phi.6-hourly-year",
            equity_curve_ar1(8759, 0.6, 0.004, 0x32),
        ),
        (
            "ar1-phi.9-hourly-year",
            equity_curve_ar1(8759, 0.9, 0.004, 0x43),
        ),
        (
            "ar1-phi.6-quarter",
            equity_curve_ar1(2190, 0.6, 0.006, 0x54),
        ),
        (
            "ar1-phi.6-daily-year",
            equity_curve_ar1(364, 0.6, 0.020, 0x65),
        ),
        ("ar1-phi.8-500", equity_curve_ar1(500, 0.8, 0.010, 0x76)),
    ];

    let mut shortened = Vec::new();
    let mut nontrivial = 0_usize;
    for (name, equity) in &cases {
        let r = log_returns_from_equity(equity);
        let shipped = politis_white_block_length_variant(&r, PwsdVariant::Shipped).max(1);
        let lit = politis_white_block_length_variant(&r, PwsdVariant::Literature).max(1);
        eprintln!(
            "[1-24/AC5] {name:<22} N={:<5}  shipped L={shipped:<4}  literature L={lit:<4}  delta={:+}",
            r.len(),
            lit as i64 - shipped as i64
        );
        if shipped > 1 || lit > 1 {
            nontrivial += 1;
        }
        if lit < shipped {
            shortened.push(format!(
                "  {name}: shipped L={shipped} -> literature L={lit}"
            ));
        }
    }

    // Non-vacuity (bug-log #135's floor, applied to this file's own first version): if every case
    // collapses to L = 1 the gate is measuring the degenerate branch and a pass means nothing.
    assert!(
        nontrivial >= 3,
        "only {nontrivial} of {} cases produced L > 1 under either convention — the fixtures have \
         stopped exercising the estimator, so this gate would pass whatever the conventions did. \
         That is exactly how this file's first version passed: i.i.d.-shock curves only, every \
         case L=1.",
        cases.len()
    );

    assert!(
        shortened.is_empty(),
        "The literature convention SHORTENS the advisor's block length on {} case(s):\n{}\n\n\
         A shorter L narrows the bootstrap null, which makes a candidate EASIER to crown — so the \
         fidelity fix would have made the shipped advisor more likely to crown an active arm. \
         That is REGRESSION-class (AD-19) and belongs in front of the operator before it ships, \
         not in a passing test. Do not re-pin this expectation to the new direction; story 1-24 \
         AC4's guard tripped on 2 of 35 surfaces precisely because the direction claim was an \
         inference, and the same caution applies here.",
        shortened.len(),
        shortened.join("\n")
    );
}
