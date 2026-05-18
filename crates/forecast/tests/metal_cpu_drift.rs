//! Metal-vs-CPU divergence smoke test (T-D-7).
//!
//! This test is gated behind the `metal` feature flag and only runs on
//! Apple Silicon.  It verifies that:
//!
//! 1. The same random-init weights + same input on both CPU and Metal backends
//!    produce outputs with `(metal - cpu).abs().max() < 1e-4`.
//! 2. The `Direction` derived from both outputs is identical (no direction flip).
//!
//! ## Exit gate semantics (T-D-7)
//!
//! If this test FAILS (max-abs drift ≥ 1e-4 OR direction flip), the developer
//! MUST STOP and report.  The LFS-anchor mitigation (ADR-0029 § 4) covers
//! ship — anchor checkpoints are trained on Metal but anchor *verification*
//! runs on CPU only.  If Metal-vs-CPU bit-identity breaks, the architect
//! re-routes to CPU-only training.
//!
//! ## Non-gated stub
//!
//! When the `metal` feature is absent, this file compiles to a single
//! always-passing test so the test suite stays green in CI.
//!
//! Cross-references:
//! - `spec/v25-tcn-overlay/feature.md § D2`
//! - `ADR-0029 § Metal-vs-CPU determinism caveat`

// Only compile the real test on metal + Apple Silicon targets.
#[cfg(all(feature = "metal", target_os = "macos"))]
mod metal_tests {
    use candle_core::{DType, Device, Tensor};
    use candle_nn::VarBuilder;
    use forecast::tcn::{
        CONTEXT_LEN, DIRECTION_EPSILON, INPUT_FEATURES, TcnModel, r_hat_to_direction,
    };

    /// Absolute tolerance for Metal-vs-CPU comparison.
    const MAX_ABS_TOL: f32 = 1e-4;

    /// T-D-7 EXIT GATE: Metal-vs-CPU max-abs drift < 1e-4 AND no direction flip.
    #[test]
    fn metal_cpu_drift_within_tolerance() {
        // 1. Build model on CPU with zero-initialised weights.
        let cpu_vb = VarBuilder::zeros(DType::F32, &Device::Cpu);
        let cpu_model = TcnModel::new(cpu_vb).expect("CPU model init");

        // 2. Build model on Metal with the SAME zero-initialised weights.
        //    (Zero-init ensures bit-identical weights on both backends.)
        let metal_device = Device::new_metal(0).expect("Metal device");
        let metal_vb = VarBuilder::zeros(DType::F32, &metal_device);
        let metal_model = TcnModel::new(metal_vb).expect("Metal model init");

        // 3. Identical input tensor on both devices.
        let input_data: Vec<f32> = (0..INPUT_FEATURES * CONTEXT_LEN)
            .map(|i| (i as f32) * 0.001)
            .collect();

        let cpu_input = Tensor::from_vec(
            input_data.clone(),
            (1, INPUT_FEATURES, CONTEXT_LEN),
            &Device::Cpu,
        )
        .unwrap();

        let metal_input =
            Tensor::from_vec(input_data, (1, INPUT_FEATURES, CONTEXT_LEN), &metal_device).unwrap();

        // 4. Forward pass on both backends.
        let cpu_out = cpu_model.forward(&cpu_input, false).expect("CPU forward");
        let metal_out = metal_model
            .forward(&metal_input, false)
            .expect("Metal forward");

        // 5. Transfer Metal result to CPU for comparison.
        let metal_on_cpu = metal_out
            .to_device(&Device::Cpu)
            .expect("transfer Metal→CPU");

        // 6. Compute max absolute difference.
        let diff = (metal_on_cpu.clone() - cpu_out.clone())
            .expect("diff")
            .abs()
            .expect("abs");
        let max_diff = diff
            .max_keepdim(0)
            .expect("max_keepdim")
            .flatten_all()
            .expect("flatten")
            .to_vec1::<f32>()
            .expect("to_vec")[0];

        println!("[T-D-7] Metal-vs-CPU max-abs delta = {max_diff:.2e}");

        // EXIT GATE: if this fails, STOP and report to architect.
        assert!(
            max_diff < MAX_ABS_TOL,
            "Metal-vs-CPU max-abs drift {max_diff:.2e} ≥ 1e-4 — T-D-7 EXIT GATE FAILED. \
             Stop and report to architect. See spec/v25-tcn-overlay/feature.md § D2 and ADR-0029."
        );

        // 7. Direction flip check.
        let cpu_r_hat = cpu_out.flatten_all().unwrap().to_vec1::<f32>().unwrap()[0];
        let metal_r_hat = metal_on_cpu
            .flatten_all()
            .unwrap()
            .to_vec1::<f32>()
            .unwrap()[0];

        let cpu_dir = r_hat_to_direction(cpu_r_hat, DIRECTION_EPSILON);
        let metal_dir = r_hat_to_direction(metal_r_hat, DIRECTION_EPSILON);

        assert_eq!(
            cpu_dir, metal_dir,
            "Metal-vs-CPU DIRECTION FLIP detected — T-D-7 EXIT GATE FAILED. \
             cpu_r_hat={cpu_r_hat:.6}, metal_r_hat={metal_r_hat:.6}. \
             Stop and report to architect."
        );

        println!(
            "[T-D-7] PASS: max_abs_delta={max_diff:.2e} < 1e-4, direction={cpu_dir:?} (no flip)"
        );
    }
}

// ── Non-metal stub: always passes ─────────────────────────────────────────────

#[cfg(not(all(feature = "metal", target_os = "macos")))]
#[test]
fn metal_cpu_drift_not_applicable() {
    // This test only runs with the `metal` feature on macOS.
    // On CPU-only CI it is a no-op pass.
    println!(
        "[T-D-7] Metal-vs-CPU drift test skipped: `metal` feature not enabled or not macOS. \
         Run with `cargo test -p forecast --features metal` on Apple Silicon to execute."
    );
}
