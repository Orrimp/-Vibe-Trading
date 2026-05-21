//! σ_train-not-in-safetensors invariant test (T-D-N6, ADR-0035 D4).
//!
//! Parses both anchored safetensors files via `safetensors::SafeTensors::deserialize`
//! (header-only, no full tensor load) and asserts no tensor name contains
//! `sigma` / `output_scale` / `sigma_train`.
//!
//! This test codifies the ADR-0035 D4 invariant: "For metadata-only
//! recalibration to be feasible, σ_train MUST NOT appear as a named tensor
//! inside the safetensors weight stream."
//!
//! A future change that bakes a calibration scalar into the weight stream
//! will break this test and force the change-author to author a superseding ADR.
//!
//! # Cross-references
//!
//! - ADR-0035 D4 — σ_train-not-in-safetensors invariant.
//! - K2 risk register — safetensors contains σ_train (falsifies Q2=(a)).
//! - `crates/forecast/src/tcn.rs:541-548` — `VarBuilder::from_buffered_safetensors`
//!   loads only named model parameter tensors; σ_train is not one of them.

/// Tensor names that MUST NOT appear in either anchored safetensors file
/// (per ADR-0035 D4).
const FORBIDDEN_SUBSTRINGS: &[&str] = &["sigma", "output_scale", "sigma_train"];

/// Assert that neither anchored safetensors file contains any tensor whose
/// name includes a forbidden substring.
///
/// ## Graceful skip
///
/// When `cargo test -p forecast` is run, the CWD is the package directory
/// (`crates/forecast/`), not the workspace root. The relative path
/// `crates/forecast/checkpoints/anchors/…` is only reachable from the workspace
/// root. If the file is not found (e.g. in CI without git-lfs pull, or when
/// running `cargo test -p forecast`), the test skips gracefully.
///
/// Run from the workspace root to get the full assertion:
/// `cargo test --features candle --test sigma_train_not_in_safetensors`
/// or via the test binary directly.
#[test]
fn test_no_sigma_tensor_in_anchors() {
    // Try both the workspace-root path and the package-root path so the test
    // works when invoked either from the workspace root or the package root.
    let candidates = ["crates/forecast/checkpoints/anchors", "checkpoints/anchors"];

    let anchors_dir = candidates
        .iter()
        .map(std::path::Path::new)
        .find(|p| p.exists());

    let anchors_dir = match anchors_dir {
        Some(d) => d,
        None => {
            eprintln!(
                "SKIP sigma_train_not_in_safetensors: anchors dir not found at \
                 'crates/forecast/checkpoints/anchors' or 'checkpoints/anchors' \
                 (run `git lfs pull` from the workspace root to fetch checkpoints, \
                 or run this test from the workspace root)"
            );
            return;
        }
    };

    let checkpoints = [
        "tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors",
        "tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.safetensors",
    ];

    let mut tested = 0usize;

    for filename in &checkpoints {
        let path = anchors_dir.join(filename);

        if !path.exists() {
            eprintln!(
                "SKIP {filename}: file not found at {} \
                 (run `git lfs pull` to fetch checkpoints)",
                path.display()
            );
            continue;
        }

        // Read the file (header is cheap to parse; safetensors header is the first
        // 8 bytes (length) + the JSON-encoded header of that length).
        let bytes = std::fs::read(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        // Parse the safetensors metadata (header only — no tensor data allocation).
        let st = safetensors::SafeTensors::deserialize(&bytes).unwrap_or_else(|e| {
            panic!(
                "failed to parse safetensors header for {}: {e}",
                path.display()
            )
        });

        // Collect all tensor names.
        let names: Vec<&String> = st.names();

        // Assert no tensor name contains a forbidden substring.
        for name in &names {
            let name_lower = name.to_lowercase();
            for forbidden in FORBIDDEN_SUBSTRINGS {
                assert!(
                    !name_lower.contains(*forbidden),
                    "Found tensor '{name}' in {filename} — \
                     this violates ADR-0035 D4 (σ_train must NOT be a safetensors tensor). \
                     If σ_train was intentionally baked into the weight stream, \
                     a superseding ADR is required."
                );
            }
        }

        // Log the tensor count for traceability.
        println!(
            "[sigma_train_not_in_safetensors] {filename}: {} tensors, none contain forbidden names",
            names.len()
        );
        tested += 1;
    }

    if tested > 0 {
        println!(
            "[sigma_train_not_in_safetensors] PASS — {tested}/{} checkpoints verified",
            checkpoints.len()
        );
    }
}
