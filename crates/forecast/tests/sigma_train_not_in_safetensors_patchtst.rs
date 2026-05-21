//! σ_train-not-in-safetensors invariant test for PatchTST (T-D-N13, ADR-0035 D4).
//!
//! Mirrors `sigma_train_not_in_safetensors.rs` but for the PatchTST anchored
//! checkpoint (`patchtst-bs1-<sha>.safetensors`).
//!
//! Pre-Wave-B: the checkpoint does not exist yet, so the outer test skips
//! gracefully and the inner `#[ignore]`d placeholder is reported as
//! "1 ignored".
//!
//! Post-Wave-B: remove `#[ignore]` from `check_no_sigma_in_patchtst_checkpoint`.
//!
//! # Cross-references
//!
//! - ADR-0035 D4 — σ_train-not-in-safetensors invariant.
//! - ADR-0036 § D3 — σ_train post-training contract for PatchTST.
//! - K2 risk register — safetensors contains σ_train (falsifies ADR-0035).

/// Tensor names that MUST NOT appear in the PatchTST anchored safetensors
/// (per ADR-0035 D4).
const FORBIDDEN_SUBSTRINGS: &[&str] = &["sigma", "output_scale", "sigma_train"];

/// Find all `patchtst-bs1-*.safetensors` files under `anchors_dir`.
fn find_patchtst_checkpoints(anchors_dir: &std::path::Path) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(anchors_dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("patchtst-bs1-") && n.ends_with(".safetensors"))
                .unwrap_or(false)
        })
        .collect()
}

/// Assert that the PatchTST anchored safetensors file does not contain any
/// tensor whose name includes a forbidden substring.
///
/// ## Graceful skip
///
/// When no checkpoint is present (pre-Wave-B), the test body is skipped
/// without panic. The inner assertion is `#[ignore]`d so the test is
/// reported as "1 ignored" rather than failing.
///
/// Run from the workspace root:
/// `cargo test -p forecast --features candle --test sigma_train_not_in_safetensors_patchtst`
///
/// Pre-Wave-B: `test result: ok. 1 passed (1 ignored)`.
/// Post-Wave-B: remove `#[ignore]` from `check_no_sigma_in_patchtst_checkpoint`.
#[test]
fn test_no_sigma_tensor_in_patchtst_anchors() {
    // Locate the anchors directory (workspace-root-relative or package-relative).
    let candidates = ["crates/forecast/checkpoints/anchors", "checkpoints/anchors"];

    let anchors_dir = candidates
        .iter()
        .map(std::path::Path::new)
        .find(|p| p.exists());

    let anchors_dir = match anchors_dir {
        Some(d) => d,
        None => {
            eprintln!(
                "SKIP sigma_train_not_in_safetensors_patchtst: anchors dir not found \
                 (run from the workspace root to locate checkpoints)"
            );
            return;
        }
    };

    let matching_files = find_patchtst_checkpoints(anchors_dir);

    if matching_files.is_empty() {
        eprintln!(
            "SKIP sigma_train_not_in_safetensors_patchtst: no patchtst-bs1-*.safetensors \
             found at {} (expected post-Wave-B)",
            anchors_dir.display()
        );
        return;
    }

    for path in &matching_files {
        let filename = path.file_name().unwrap_or_default().to_string_lossy();

        let bytes = std::fs::read(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));

        let st = safetensors::SafeTensors::deserialize(&bytes).unwrap_or_else(|e| {
            panic!(
                "failed to parse safetensors header for {}: {e}",
                path.display()
            )
        });

        let names: Vec<&String> = st.names();

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

        println!(
            "[sigma_train_not_in_safetensors_patchtst] {filename}: {} tensors, \
             none contain forbidden names",
            names.len()
        );
    }

    println!(
        "[sigma_train_not_in_safetensors_patchtst] PASS — {}/{} checkpoint(s) verified",
        matching_files.len(),
        matching_files.len()
    );
}

/// Pre-Wave-B placeholder: canonical post-Wave-B gate.
///
/// This test is `#[ignore]`d so the harness reports "1 ignored" (not failure)
/// before Wave B completes. After the checkpoint lands, remove `#[ignore]`
/// and run the full assertion.
#[test]
#[ignore = "Pre-Wave-B: patchtst-bs1-*.safetensors does not exist yet; remove #[ignore] after Wave B completes"]
fn check_no_sigma_in_patchtst_checkpoint() {
    let candidates = ["crates/forecast/checkpoints/anchors", "checkpoints/anchors"];
    let anchors_dir = candidates
        .iter()
        .map(std::path::Path::new)
        .find(|p| p.exists())
        .expect("anchors dir not found; run from workspace root");

    let matching_files = find_patchtst_checkpoints(anchors_dir);

    assert!(
        !matching_files.is_empty(),
        "No patchtst-bs1-*.safetensors found at {} — \
         run Wave B training first (T-D-N17)",
        anchors_dir.display()
    );

    for path in &matching_files {
        let bytes = std::fs::read(path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        let st = safetensors::SafeTensors::deserialize(&bytes)
            .unwrap_or_else(|e| panic!("safetensors parse error: {e}"));
        for name in st.names() {
            let name_lower = name.to_lowercase();
            for forbidden in FORBIDDEN_SUBSTRINGS {
                assert!(
                    !name_lower.contains(*forbidden),
                    "σ_train found in safetensors: tensor '{name}' violates ADR-0035 D4"
                );
            }
        }
    }
}
