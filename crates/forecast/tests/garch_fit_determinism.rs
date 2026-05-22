//! GARCH(1,1) fit determinism test — R11.4 / ADR-0038 § D3.
//!
//! Verifies that two sequential calls to `GarchModel::fit()` on identical
//! input data produce byte-identical (ω, α, β) output.
//!
//! This is the load-bearing determinism contract for the anchor lock:
//! if the GARCH fitter produces different results across runs, the
//! `checkpoint_revision` SHA-256 will differ and the anchor will be
//! unmatchable.
//!
//! ## What is checked
//!
//! 1. Two sequential `GarchModel::fit()` calls on a deterministic synthetic
//!    series produce identical (ω, α, β, unconditional_var).
//! 2. The comparison uses the bit-exact f64 representation (`to_bits()`),
//!    not a floating-point tolerance — byte identity is the contract.
//!
//! ## Cross-references
//!
//! - ADR-0038 § D3 — determinism contract (hand-rolled optimiser guarantees).
//! - R11.4 — byte-identity gate on per-symbol JSON outputs.
//! - `crates/forecast/src/garch.rs` — `GarchModel::fit()`.

use forecast::garch::GarchModel;

/// R11.4: two sequential fits on identical input produce byte-identical params.
#[test]
fn garch_fit_determinism_byte_identical() {
    // Deterministic synthetic series — alternating shock magnitudes.
    // 500 bars matches a realistic sub-year segment length.
    let returns: Vec<f64> = (0..500_usize)
        .map(|i| {
            // Pseudo-deterministic pattern: large shocks at multiples of 7.
            if i % 7 == 0 {
                0.025_f64
            } else if i % 13 == 0 {
                -0.018_f64
            } else {
                0.003_f64 * (1 + i as i64 % 3 - 1) as f64
            }
        })
        .collect();

    let model_a = GarchModel::fit(&returns).expect("first fit must succeed");
    let model_b = GarchModel::fit(&returns).expect("second fit must succeed");

    // Byte-identical comparison on all core parameters.
    assert_eq!(
        model_a.omega.to_bits(),
        model_b.omega.to_bits(),
        "omega differs between two runs: {:.9e} vs {:.9e}",
        model_a.omega,
        model_b.omega
    );
    assert_eq!(
        model_a.alpha.to_bits(),
        model_b.alpha.to_bits(),
        "alpha differs between two runs: {:.9e} vs {:.9e}",
        model_a.alpha,
        model_b.alpha
    );
    assert_eq!(
        model_a.beta.to_bits(),
        model_b.beta.to_bits(),
        "beta differs between two runs: {:.9e} vs {:.9e}",
        model_a.beta,
        model_b.beta
    );
    assert_eq!(
        model_a.unconditional_var.to_bits(),
        model_b.unconditional_var.to_bits(),
        "unconditional_var differs"
    );
    assert_eq!(
        model_a.n_iters, model_b.n_iters,
        "n_iters differs: {} vs {}",
        model_a.n_iters, model_b.n_iters
    );
    assert_eq!(
        model_a.converged, model_b.converged,
        "converged flag differs"
    );

    // Stationarity sanity check on both outputs.
    assert!(
        model_a.alpha + model_a.beta < 1.0,
        "model_a non-stationary: alpha+beta={}",
        model_a.alpha + model_a.beta
    );
    assert!(
        model_b.alpha + model_b.beta < 1.0,
        "model_b non-stationary: alpha+beta={}",
        model_b.alpha + model_b.beta
    );
}
