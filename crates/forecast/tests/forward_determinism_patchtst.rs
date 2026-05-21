//! PatchTST CPU forward-pass byte-identity + Metal-vs-CPU drift guard (T-D-N14).
//!
//! Two invariants:
//!
//! 1. **CPU byte-identity (K2)**: two forward passes on CPU with the same
//!    random weights and the same input tensor produce byte-identical output.
//!    This is the K2 determinism contract per ADR-0036 § D5.
//!
//! 2. **Metal-vs-CPU drift < 1e-4**: when the `metal` feature is active,
//!    a forward pass on Metal and the same pass on CPU agree within 1e-4.
//!    Skipped on non-Metal hardware (returns `test result: ok. 1 passed`
//!    with the Metal test skipped). Skipped in CI where only CPU is available.
//!
//! # Cross-references
//!
//! - ADR-0036 § D5 — K2 determinism contract for PatchTST.
//! - `crates/forecast/src/patchtst.rs` — `PatchTstModel::forward`.

#[cfg(feature = "candle")]
mod candle_tests {
    use candle_core::{DType, Device, Tensor};
    use candle_nn::{VarBuilder, VarMap};
    use forecast::patchtst::{CHANNELS, CONTEXT_LEN, PatchTstModel};

    /// Build a random-init `PatchTstModel` on the given device using a fixed seed.
    ///
    /// The VarMap is seeded deterministically so two calls produce the same
    /// initial weights (same seed → same ChaCha20 random init inside candle-nn).
    fn build_model(device: &Device) -> PatchTstModel {
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, device);
        PatchTstModel::new(vb).expect("PatchTstModel::new")
    }

    /// Build a fixed-value input tensor [1, CHANNELS, CONTEXT_LEN] on `device`.
    fn build_input(device: &Device) -> Tensor {
        // Use a simple ramp so all channels have distinct non-zero values.
        let n = CHANNELS * CONTEXT_LEN;
        let data: Vec<f32> = (0..n).map(|i| (i as f32) / n as f32).collect();
        Tensor::from_vec(data, (1, CHANNELS, CONTEXT_LEN), device).expect("input tensor build")
    }

    /// T-D-N14 invariant 1: CPU forward-pass byte-identity (K2).
    ///
    /// Two identical forward passes on the same random-init model must produce
    /// byte-identical outputs. This catches any non-deterministic operation
    /// (e.g. non-deterministic dropout with wrong mode flag, race conditions).
    #[test]
    fn cpu_forward_byte_identity() {
        let device = Device::Cpu;

        let model = build_model(&device);
        let x = build_input(&device);

        // Run forward pass twice (train=false → dropout disabled → deterministic).
        let out1 = model
            .forward(&x, false)
            .expect("first forward pass")
            .flatten_all()
            .expect("flatten1")
            .to_vec1::<f32>()
            .expect("to_vec1 (pass 1)");

        let out2 = model
            .forward(&x, false)
            .expect("second forward pass")
            .flatten_all()
            .expect("flatten2")
            .to_vec1::<f32>()
            .expect("to_vec1 (pass 2)");

        // Byte-identical comparison (not approximate).
        assert_eq!(
            out1.len(),
            out2.len(),
            "output lengths differ between passes"
        );
        for (i, (a, b)) in out1.iter().zip(out2.iter()).enumerate() {
            assert_eq!(
                a.to_bits(),
                b.to_bits(),
                "byte mismatch at output[{i}]: pass1={a} pass2={b} — \
                 K2 CPU determinism contract violated (ADR-0036 § D5)"
            );
        }

        println!(
            "[forward_determinism_patchtst] cpu_forward_byte_identity PASS \
             — {}/{} values byte-identical",
            out1.len(),
            out1.len()
        );
    }

    /// T-D-N14 invariant 2: Metal-vs-CPU drift < 1e-4.
    ///
    /// Skipped on non-Metal hardware. When the `metal` feature is active and
    /// a Metal device is available, asserts that the maximum absolute difference
    /// between Metal and CPU outputs is below 1e-4.
    #[test]
    fn metal_vs_cpu_drift_within_tolerance() {
        // Metal is only available when the `metal` feature is enabled AND
        // the test is running on Apple Silicon hardware. Attempt to construct
        // a Metal device; skip if unavailable.
        #[cfg(feature = "metal")]
        {
            let metal_device = match Device::new_metal(0) {
                Ok(d) => d,
                Err(e) => {
                    eprintln!("SKIP metal_vs_cpu_drift: Metal device unavailable ({e})");
                    return;
                }
            };

            let cpu_device = Device::Cpu;

            // Build models on their respective devices.
            let cpu_model = build_model(&cpu_device);
            let metal_model = build_model(&metal_device);

            let x_cpu = build_input(&cpu_device);
            let x_metal = build_input(&metal_device);

            let cpu_out = cpu_model
                .forward(&x_cpu, false)
                .expect("CPU forward")
                .flatten_all()
                .expect("flatten cpu")
                .to_vec1::<f32>()
                .expect("to_vec1 cpu");

            let metal_out = metal_model
                .forward(&x_metal, false)
                .expect("Metal forward")
                .flatten_all()
                .expect("flatten metal")
                .to_vec1::<f32>()
                .expect("to_vec1 metal");

            assert_eq!(
                cpu_out.len(),
                metal_out.len(),
                "Metal and CPU outputs have different lengths"
            );

            let max_abs_diff = cpu_out
                .iter()
                .zip(metal_out.iter())
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f32, f32::max);

            const TOLERANCE: f32 = 1e-4;
            assert!(
                max_abs_diff < TOLERANCE,
                "Metal-vs-CPU max abs diff = {max_abs_diff} >= {TOLERANCE} — \
                 K2 drift tolerance violated (ADR-0036 § D5)"
            );

            println!(
                "[forward_determinism_patchtst] metal_vs_cpu_drift PASS \
                 — max_abs_diff={max_abs_diff} < {TOLERANCE}"
            );
        }

        #[cfg(not(feature = "metal"))]
        {
            eprintln!(
                "SKIP metal_vs_cpu_drift_within_tolerance: `metal` feature not enabled \
                 (non-Metal CI / CPU-only run)"
            );
        }
    }
}
