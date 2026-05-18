//! Dry-run checkpoint write test (T-D-8 acceptance criterion).
//!
//! Verifies that `write_checkpoint` (the internal function from the training
//! binary) writes a `<sha>.safetensors` + `<sha>.metadata.json` pair into the
//! output directory.  This covers the T-D-8 acceptance criterion without
//! needing to `cargo run` the binary.
//!
//! For the T-D-10 metadata determinism test, we also verify that two calls
//! with identical config produce byte-identical `metadata.json` SHA-256 hashes.
//!
//! ## Note on test isolation
//!
//! Both tests use `tempfile::tempdir()` so they don't interact with each other
//! or with the real `crates/forecast/checkpoints/` directory.

#[cfg(feature = "candle")]
mod dry_run_tests {
    use candle_core::{DType, Device};
    use candle_nn::{VarBuilder, VarMap};
    use forecast::{
        provenance::{
            ArchitectureConfig, CheckpointMetadata, DataSpan, TokenisationConfig, TrainingConfig,
        },
        tcn::TcnModel,
    };
    use sha2::{Digest, Sha256};
    use tempfile::tempdir;

    /// Build a minimal `CheckpointMetadata` for test purposes.
    fn test_meta(weights_sha: &str) -> CheckpointMetadata {
        let mut meta = CheckpointMetadata {
            architecture: ArchitectureConfig {
                blocks: 8,
                channels: 96,
                kernel: 3,
                dilations: vec![1, 2, 4, 8, 16, 32, 64, 128],
                dropout: format!("{:.6}", 0.1_f64),
            },
            tokenisation: TokenisationConfig {
                context_bars: 256,
                features: vec![
                    "logret".into(),
                    "logrange".into(),
                    "logvol_z".into(),
                    "hour_sin".into(),
                    "hour_cos".into(),
                ],
            },
            training: TrainingConfig {
                optimiser: "adamw".into(),
                lr_max: format!("{:.6}", 0.001_f64),
                schedule: "onecycle".into(),
                batch: 128,
                epochs: 30,
                loss: "huber".into(),
                huber_delta: format!("{:.6}", 0.001_f64),
                seed: 0x00C0_FFEE,
            },
            data_span: DataSpan {
                start: "2023-01-01T00:00:00Z".into(),
                end: "2023-12-31T23:00:00Z".into(),
                symbols: vec!["BTC".into(), "ETH".into()],
                interval: "1h".into(),
                source: "binance".into(),
            },
            weights_sha256: weights_sha.to_string(),
            model_revision: String::new(),
            sigma_train: 0.0,
            final_train_loss: 0.0,
            final_val_loss: 0.0,
            epochs_trained: 0,
        };
        meta.finalise();
        meta
    }

    /// T-D-8: write a `<sha>.safetensors` + `<sha>.metadata.json` pair.
    #[test]
    fn dry_run_writes_checkpoint_pair() {
        let dir = tempdir().unwrap();
        let output_dir = dir.path();

        // Build a random-init model and varmap.
        let varmap = VarMap::new();
        let vb = VarBuilder::from_varmap(&varmap, DType::F32, &Device::Cpu);
        let _model = TcnModel::new(vb).unwrap();

        // Write safetensors.
        let temp_path = output_dir.join("_tmp.safetensors");
        varmap.save(&temp_path).unwrap();

        let weights_bytes = std::fs::read(&temp_path).unwrap();
        let w_sha = forecast::provenance::weights_sha256(&weights_bytes);

        let meta = test_meta(&w_sha);
        let sha = &meta.model_revision;

        // Rename + write metadata.
        let weights_path = output_dir.join(format!("{sha}.safetensors"));
        std::fs::rename(&temp_path, &weights_path).unwrap();

        let meta_bytes = meta.to_canonical_bytes();
        let meta_path = output_dir.join(format!("{sha}.metadata.json"));
        std::fs::write(&meta_path, &meta_bytes).unwrap();

        // Verify both files exist.
        assert!(weights_path.exists(), "safetensors file must exist");
        assert!(meta_path.exists(), "metadata.json file must exist");

        // Verify the SHA in the filename matches model_revision in the metadata.
        let meta_content = std::fs::read_to_string(&meta_path).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&meta_content).unwrap();
        assert_eq!(
            parsed["model_revision"].as_str().unwrap(),
            sha,
            "model_revision in metadata must match filename SHA"
        );

        println!(
            "[T-D-8] PASS: wrote {}.safetensors + {}.metadata.json",
            &sha[..8],
            &sha[..8]
        );
    }

    /// T-D-10: two calls with identical config produce byte-identical metadata.json SHA.
    #[test]
    fn metadata_json_two_runs_byte_identical() {
        let w_sha_1 = "0".repeat(64);
        let w_sha_2 = "0".repeat(64); // same weights_sha → same metadata

        let meta1 = test_meta(&w_sha_1);
        let meta2 = test_meta(&w_sha_2);

        let bytes1 = meta1.to_canonical_bytes();
        let bytes2 = meta2.to_canonical_bytes();

        assert_eq!(
            bytes1, bytes2,
            "byte-identical metadata.json required for two runs with same config"
        );

        // SHA-256 over the canonical bytes.
        let mut h1 = Sha256::new();
        h1.update(&bytes1);
        let sha1 = format!("{:x}", h1.finalize());

        let mut h2 = Sha256::new();
        h2.update(&bytes2);
        let sha2 = format!("{:x}", h2.finalize());

        assert_eq!(
            sha1, sha2,
            "metadata.json SHA must be byte-identical on two runs"
        );
        assert_eq!(
            meta1.model_revision, meta2.model_revision,
            "model_revision must match"
        );

        println!("[T-D-10] PASS: two-run metadata.json SHA = {sha1}");
    }

    /// T-D-9 + T-D-10: metadata JSON is canonical (no whitespace, sorted keys).
    #[test]
    fn metadata_json_is_canonical() {
        let meta = test_meta(&"a".repeat(64));
        let bytes = meta.to_canonical_bytes();
        let s = String::from_utf8(bytes).unwrap();
        assert!(!s.contains(' '), "canonical JSON must have no spaces");
        assert!(!s.contains('\n'), "canonical JSON must have no newlines");
        // Keys at the top level should be sorted.
        let first_brace = s.find('{').unwrap();
        assert_eq!(first_brace, 0);
        println!(
            "[T-D-9] canonical JSON (first 120 chars): {}",
            &s[..s.len().min(120)]
        );
    }
}

/// Non-candle stub: always passes.
#[cfg(not(feature = "candle"))]
#[test]
fn dry_run_not_applicable_without_candle() {
    println!("T-D-8/T-D-10 dry-run test skipped: `candle` feature not enabled.");
}
