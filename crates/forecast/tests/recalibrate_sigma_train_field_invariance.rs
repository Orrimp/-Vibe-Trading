//! Field-invariance test for `recalibrate_sigma_train` (T-D-N5, K5).
//!
//! Tests that the recalibrated metadata overlay has exactly 9 of 10 top-level
//! fields byte-identical to the original — only `sigma_train` changes.
//!
//! Operates on an **in-memory fixture** (no full forward pass needed).
//! Tests the JSON-overlay logic in isolation.
//!
//! # Cross-references
//!
//! - ADR-0035 D2 — exactly one field substituted in the overlay.
//! - K5 risk register — no scope creep into other metadata fields.
//! - T-D-N5 (decomp.md Wave A) — field-invariance gate.

use serde_json::Value;

/// The 9 top-level keys that MUST stay byte-identical between original
/// and recalibrated overlay (per ADR-0035 D2).
const INVARIANT_KEYS: &[&str] = &[
    "architecture",
    "data_span",
    "epochs_trained",
    "final_train_loss",
    "final_val_loss",
    "model_revision",
    "tokenisation",
    "training",
    "weights_sha256",
];

/// Build a minimal fake metadata JSON that mirrors the real schema.
fn fake_metadata_json() -> Value {
    serde_json::json!({
        "architecture": {
            "blocks": 8,
            "channels": 96,
            "dilations": [1, 2, 4, 8, 16, 32, 64, 128],
            "dropout": "0.100000",
            "kernel": 3
        },
        "data_span": {
            "end": "2023-12-31T23:00:00Z",
            "interval": "1h",
            "source": "binance",
            "start": "2023-01-01T00:00:00Z",
            "symbols": ["ADA", "AVAX", "BNB", "BTC", "DOGE", "DOT", "ETH", "LINK", "SOL", "XRP"]
        },
        "epochs_trained": 30,
        "final_train_loss": 1.2167605746071786e-05_f64,
        "final_val_loss": 1.5389239706564695e-05_f64,
        "model_revision": "d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2",
        "sigma_train": 10.95425033569336_f64,
        "tokenisation": {
            "context_bars": 256,
            "features": ["logret", "logrange", "logvol_z", "hour_sin", "hour_cos"]
        },
        "training": {
            "batch": 128,
            "epochs": 30,
            "huber_delta": "0.001000",
            "loss": "huber",
            "lr_max": "0.001000",
            "optimiser": "adamw",
            "schedule": "onecycle",
            "seed": 12648430_u64
        },
        "weights_sha256": "4ed9064a3871d8bc911ad8b288dccfc597caa6a09cca3b2395a9e1717b8c7025"
    })
}

/// Apply the sigma_train substitution (mirrors the logic in the bin).
fn apply_overlay(original: &Value, sigma_train_recal: f64) -> Value {
    let mut overlay = original.clone();
    let sigma_json = serde_json::Number::from_f64(sigma_train_recal)
        .expect("sigma_train_recal should be finite");
    overlay["sigma_train"] = Value::Number(sigma_json);
    overlay
}

/// Core assertion: exactly the `sigma_train` field differs; all 9 invariant
/// keys have identical JSON representations.
///
/// This is the K5 guard codified in ADR-0035 D2.
#[test]
fn test_recalibrated_overlay_invariance() {
    let original = fake_metadata_json();
    let sigma_train_recal = 0.018015573_f64; // expected recalibrated value (BS-1 range)
    let overlay = apply_overlay(&original, sigma_train_recal);

    // All 9 invariant keys must match verbatim (JSON equality).
    for key in INVARIANT_KEYS {
        let orig_val = &original[key];
        let recal_val = &overlay[key];
        assert_eq!(
            orig_val, recal_val,
            "field '{key}' must be byte-identical between original and overlay, \
             but original={orig_val:?} recalibrated={recal_val:?}"
        );
    }

    // The sigma_train field MUST differ.
    let orig_sigma = original["sigma_train"].as_f64().unwrap();
    let recal_sigma = overlay["sigma_train"].as_f64().unwrap();
    assert!(
        (orig_sigma - recal_sigma).abs() > 1.0,
        "sigma_train should differ significantly: original={orig_sigma}, recalibrated={recal_sigma}"
    );
    assert!(
        recal_sigma < 1.0,
        "recalibrated sigma_train should be < 1.0 (in log-return units), got {recal_sigma}"
    );

    // There must be exactly 10 top-level keys.
    let total_keys = overlay.as_object().expect("overlay is an object").len();
    assert_eq!(
        total_keys, 10,
        "overlay should have exactly 10 top-level keys, got {total_keys}"
    );
}

/// The overlay must have exactly the same top-level keys as the original
/// (no keys added, no keys removed).
#[test]
fn test_overlay_no_key_count_change() {
    let original = fake_metadata_json();
    let overlay = apply_overlay(&original, 0.010_f64);

    let orig_keys: std::collections::BTreeSet<_> = original
        .as_object()
        .unwrap()
        .keys()
        .map(|s| s.as_str())
        .collect();
    let overlay_keys: std::collections::BTreeSet<_> = overlay
        .as_object()
        .unwrap()
        .keys()
        .map(|s| s.as_str())
        .collect();

    assert_eq!(
        orig_keys,
        overlay_keys,
        "overlay must have the same key set as original: \
         original_only={:?} overlay_only={:?}",
        orig_keys.difference(&overlay_keys).collect::<Vec<_>>(),
        overlay_keys.difference(&orig_keys).collect::<Vec<_>>()
    );
}

/// Overlay output via canonicalise is byte-stable across two calls
/// (determinism gate per ADR-0035 D1 + Q5 = (a)).
#[test]
fn test_overlay_canonical_deterministic() {
    let original = fake_metadata_json();
    let overlay = apply_overlay(&original, 0.018015573_f64);

    let bytes1 = forecast::provenance::canonicalise(&overlay);
    let bytes2 = forecast::provenance::canonicalise(&overlay);

    assert_eq!(
        bytes1, bytes2,
        "canonicalise on the same overlay Value must produce byte-identical output"
    );
    // Sanity: canonical JSON has no whitespace.
    let s = String::from_utf8(bytes1.clone()).expect("canonical JSON is valid UTF-8");
    assert!(
        !s.contains(' ') && !s.contains('\n'),
        "canonical JSON must have no whitespace"
    );
    // Sanity: sigma_train appears as a JSON number in the canonical bytes.
    assert!(
        bytes1
            .windows(b"sigma_train".len())
            .any(|w| w == b"sigma_train"),
        "canonical JSON must contain sigma_train key"
    );
}

/// On-disk JSON number convention: sigma_train must be a JSON number, NOT a string.
///
/// ADR-0035 D2 specifies this diverges from ADR-0029 § 2 rule 5 (which would
/// produce a string). This test asserts the overlay uses the correct format.
#[test]
fn test_sigma_train_is_json_number_not_string() {
    let original = fake_metadata_json();
    let recal = 0.018015573_f64;
    let overlay = apply_overlay(&original, recal);

    // sigma_train in the overlay must be a Number, not a String.
    let sigma_val = &overlay["sigma_train"];
    assert!(
        sigma_val.is_number(),
        "sigma_train must be a JSON number (not string) in the overlay; got {sigma_val:?}"
    );
    assert!(
        !sigma_val.is_string(),
        "sigma_train must NOT be a JSON string in the overlay; got {sigma_val:?}"
    );

    // The value must round-trip correctly via .as_f64().
    let roundtrip = sigma_val
        .as_f64()
        .expect("sigma_train must be parseable as f64");
    assert!(
        (roundtrip - recal).abs() < 1e-12,
        "sigma_train round-trip via .as_f64() must be lossless: expected {recal}, got {roundtrip}"
    );
}
