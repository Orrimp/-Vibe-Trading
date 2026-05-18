//! M3 anchor-load smoke test (T-D-11 / T-D-12 acceptance criterion step 1).
//!
//! Verifies that both LFS-tracked checkpoints decode cleanly and that the
//! `model_revision` in the loaded `TcnForecaster` matches the expected prefix
//! hard-coded in `AnchorScenario`.
//!
//! This test is gated behind the `candle` feature because loading weights
//! requires `candle-nn::VarBuilder`.
//!
//! ## Checkpoints exercised
//!
//! - BS-1: `tcn-bs1-d1c3696d…` (Jan–Sep 2023 train, Oct–Dec 2023 val)
//! - BS-2: `tcn-bs2-3fabcabe…` (2023 full-year train, Q1 2024 val)
//!
//! ## What is verified
//!
//! 1. `TcnForecaster::load_anchor(Bs1)` returns `Ok(…)` — safetensors +
//!    metadata.json both decoded without error.
//! 2. The loaded forecaster's `model_revision` starts with the BS-1 SHA prefix
//!    (`d1c3696d`).
//! 3. `sigma_train` is finite and positive (calibrated confidence divisor).
//! 4. A forward pass on a synthetic `[1, 5, 256]` input returns `[1, 1]` shape.
//! 5. Same four checks for BS-2 (`3fabcabe`).

#[cfg(feature = "candle")]
mod anchor_tests {
    use candle_core::{DType, Device, Tensor};

    use forecast::tcn::{AnchorScenario, TcnForecaster, TcnForecasterError};

    /// BS-1 expected model_revision SHA prefix (from metadata.json).
    const BS1_SHA_PREFIX: &str = "d1c3696d";
    /// BS-2 expected model_revision SHA prefix (from metadata.json).
    const BS2_SHA_PREFIX: &str = "3fabcabe";

    // ── helper ────────────────────────────────────────────────────────────────

    /// Run the load + shape + sigma checks for a given `AnchorScenario`.
    fn verify_anchor(scenario: AnchorScenario, expected_prefix: &str) {
        let forecaster = match TcnForecaster::load_anchor(scenario) {
            Ok(f) => f,
            Err(TcnForecasterError::CheckpointNotFound { path }) => {
                // If the checkpoint is absent on this machine (e.g. LFS not
                // pulled) we skip gracefully rather than failing the build.
                eprintln!(
                    "SKIP anchor_load test: checkpoint not found at {path} \
                     (run `git lfs pull` to fetch checkpoints)"
                );
                return;
            }
            Err(e) => panic!("TcnForecaster::load_anchor failed: {e:?}"),
        };

        // 1. model_revision prefix matches the expected SHA prefix.
        assert!(
            forecaster.model_revision.starts_with(expected_prefix),
            "model_revision should start with {expected_prefix}, got: {}",
            forecaster.model_revision
        );

        // 2. sigma_train is finite and positive.
        let sigma = forecaster.sigma_train;
        assert!(
            sigma.is_finite() && sigma > 0.0,
            "sigma_train should be finite and positive, got: {sigma}"
        );

        // 3. Forward pass: [1, 5, 256] → [1, 1].
        let device = Device::Cpu;
        // Input tensor: channel-first, shape [batch=1, features=5, seq=256].
        let input =
            Tensor::zeros(&[1usize, 5, 256], DType::F32, &device).expect("create zero tensor");
        let output = forecaster.forward(&input, false).expect("forward failed");
        let shape = output.shape().dims().to_vec();
        assert_eq!(
            shape,
            vec![1usize, 1],
            "forward should return [1, 1], got shape: {shape:?}"
        );

        // 4. Output value is finite (not NaN/inf).
        let val: f32 = output
            .flatten_all()
            .expect("flatten")
            .get(0)
            .expect("get element 0")
            .to_scalar::<f32>()
            .expect("to_scalar");
        assert!(
            val.is_finite(),
            "forward output should be finite, got: {val}"
        );
    }

    // ── tests ─────────────────────────────────────────────────────────────────

    /// T-D-11 smoke — BS-1 checkpoint loads cleanly and forward pass is valid.
    #[test]
    fn td11_bs1_anchor_loads_and_forward_ok() {
        verify_anchor(AnchorScenario::Bs1, BS1_SHA_PREFIX);
    }

    /// T-D-12 smoke — BS-2 checkpoint loads cleanly and forward pass is valid.
    #[test]
    fn td12_bs2_anchor_loads_and_forward_ok() {
        verify_anchor(AnchorScenario::Bs2, BS2_SHA_PREFIX);
    }

    /// Determinism: two forward passes on the same input produce identical output.
    ///
    /// This is the CPU-path determinism guarantee — candle CPU inference is
    /// bit-identical across calls (no random sampling in TCN eval mode).
    #[test]
    fn td11_bs1_forward_deterministic() {
        let forecaster = match TcnForecaster::load_anchor(AnchorScenario::Bs1) {
            Ok(f) => f,
            Err(TcnForecasterError::CheckpointNotFound { path }) => {
                eprintln!(
                    "SKIP determinism test: checkpoint not found at {path} \
                     (run `git lfs pull` to fetch checkpoints)"
                );
                return;
            }
            Err(e) => panic!("TcnForecaster::load_anchor failed: {e:?}"),
        };

        let device = Device::Cpu;
        // Use a non-trivial input so we exercise the weight matmul paths.
        let vals: Vec<f32> = (0..5 * 256).map(|i| (i as f32 * 0.001).sin()).collect();
        let input =
            Tensor::from_vec(vals, &[1usize, 5, 256], &device).expect("create input tensor");

        let out1 = forecaster
            .forward(&input, false)
            .expect("forward 1")
            .flatten_all()
            .expect("flatten 1")
            .get(0)
            .expect("get 0")
            .to_scalar::<f32>()
            .expect("scalar 1");

        let out2 = forecaster
            .forward(&input, false)
            .expect("forward 2")
            .flatten_all()
            .expect("flatten 2")
            .get(0)
            .expect("get 0")
            .to_scalar::<f32>()
            .expect("scalar 2");

        assert_eq!(
            out1.to_bits(),
            out2.to_bits(),
            "CPU forward pass must be bit-identical across two calls: got {out1} vs {out2}"
        );
    }
}
