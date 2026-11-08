//! Field-invariance test for `recalibrate_sigma_train` (T-D-N5, K5).
//!
//! Tests that the recalibrated metadata overlay leaves exactly 9 of 10 top-level
//! fields **value-identical** to the original — only `sigma_train` changes.
//!
//! "Value-identical under `provenance::canonicalise`", not byte-identical at the
//! file-text level: the committed BS-2 pair differs in the *text* of
//! `final_val_loss` (…069431 → …069433, one ULP, from serde_json's parse
//! rounding on a round-trip) while being the same `f64`. The `.5e` format in the
//! derivation report hides that. See
//! `docs/dev-notes/recalibrate-field-invariance-deviation-2026-11-08.md`.
//!
//! ## What changed 2026-11-08 (bug-log `#167`)
//!
//! Until then this file hand-copied the overlay logic into a five-line local
//! `apply_overlay` and tested the copy. Measured: `nm -a` on this test's binary
//! found `write_overlay` **0** times and `apply_overlay` twice, so perturbing the
//! copy reddened the gate and no edit to the shipped function could. The
//! substitution now lives at `forecast::provenance::overlay_with_sigma_train`
//! and every test below calls it.
//!
//! The in-memory fixture is kept — it is a cheap, readable statement of the
//! invariant — but it is no longer the ONLY input: the two committed
//! `{original, recalibrated}` metadata pairs under `checkpoints/anchors/` are
//! now asserted against directly, with a floor so a renamed or missing
//! checkpoint reads RED rather than as a silent skip.
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

/// The SHIPPED substitution — not a copy of it (bug-log `#167`).
///
/// Kept as a one-line shim so the four tests below read unchanged while calling
/// `forecast::provenance::overlay_with_sigma_train`, which
/// `bin/recalibrate_sigma_train.rs::write_overlay` also calls. Perturb the
/// shipped function and these tests redden; there is no longer a second
/// implementation to perturb instead.
fn apply_overlay(original: &Value, sigma_train_recal: f64) -> Value {
    forecast::provenance::overlay_with_sigma_train(original, sigma_train_recal)
        .expect("sigma_train_recal should be finite")
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

// ── The committed pairs, asserted directly (bug-log #167) ────────────────────
//
// The in-memory fixture above states the invariant; these tests MEASURE it on
// the artefacts that actually ship. Both `{original, recalibrated}` pairs are
// committed, git-tracked, non-LFS plain JSON under `checkpoints/anchors/`, so
// this needs no forward pass, no `candle` feature and no network — which is why
// it belongs in this target (the one `[[test]]` here with no
// `required-features`, and therefore the only one CI builds; see bug-log #163).

use std::path::{Path, PathBuf};

fn workspace_root() -> PathBuf {
    let crate_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    crate_dir
        .parent()
        .and_then(|p| p.parent())
        .map_or(crate_dir.clone(), Path::to_path_buf)
}

/// Every committed `(label, original, recalibrated)` metadata pair.
fn committed_overlay_pairs() -> Vec<(String, Value, Value)> {
    let dir = workspace_root().join("crates/forecast/checkpoints/anchors");
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return out;
    };
    let mut recal: Vec<PathBuf> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.to_string_lossy().ends_with(".metadata.recalibrated.json"))
        .collect();
    recal.sort();
    for r in recal {
        let raw = r.to_string_lossy().to_string();
        let orig = PathBuf::from(raw.replace(".metadata.recalibrated.json", ".metadata.json"));
        let (Ok(o), Ok(c)) = (std::fs::read(&orig), std::fs::read(&r)) else {
            continue;
        };
        let (Ok(ov), Ok(cv)) = (
            serde_json::from_slice::<Value>(&o),
            serde_json::from_slice::<Value>(&c),
        ) else {
            continue;
        };
        let label = r
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        out.push((label, ov, cv));
    }
    out
}

/// ADR-0035 D2 on the shipped artefacts: the committed overlay differs from its
/// original in exactly `sigma_train` **at the resolution the hasher works at**.
///
/// That qualifier is load-bearing and was measured, not assumed. At the FILE-TEXT
/// level the BS-2 pair differs in two places, and this comparison cannot see the
/// second one, because `serde_json`'s float parser is off by one ULP on the
/// original's `final_val_loss`:
///
/// ```text
/// str::parse::<f64>  "…069431" -> 3ee60a8b00000000   "…069433" -> 3ee60a8b00000001
/// serde_json         "…069431" -> 3ee60a8b00000001   "…069433" -> 3ee60a8b00000001
/// ```
///
/// So serde_json collapses the two decimals to one `f64` and re-serialises it as
/// `…069433` — which is why the committed overlay's text differs from the
/// original's on a field nothing substituted. It is a lossy round-trip in the
/// JSON library, NOT a D2 scope breach: `model_revision` is computed over
/// `canonicalise` output, so what this comparison sees IS the contract, and
/// `committed_overlays_replay_byte_identically_through_the_shipped_path` proves
/// the committed bytes are exactly what the shipped path produces.
///
/// `committed_overlay_text_divergence_is_exactly_the_declared_set` below pins the
/// file-text view, so this known deviation is DECLARED and a new one fails.
///
/// Compared per key through `provenance::canonicalise` — the repo's own hasher
/// input, never a re-implementation — so the comparison is the same one
/// `model_revision` is built on.
///
/// **This test does NOT gate the shipped code**, and the distinction matters.
/// It compares two committed FILES, so it catches a bad overlay being
/// committed; it cannot catch a change to `overlay_with_sigma_train`, and
/// measured, it stays green through all three perturbations of that function.
/// `committed_overlays_replay_byte_identically_through_the_shipped_path` below
/// is the one that gates the code — it caught all three. Two tests, two
/// different subjects, named so neither is mistaken for the other.
#[test]
fn committed_overlays_substitute_exactly_sigma_train() {
    let pairs = committed_overlay_pairs();

    // NON-VACUITY FLOOR. Without this, a renamed or deleted checkpoint makes
    // every assertion below iterate an empty list and the test passes having
    // measured nothing — the exact shape this whole pass is about.
    assert_eq!(
        pairs.len(),
        2,
        "expected the 2 committed overlay pairs (tcn-bs1-…, tcn-bs2-…) under \
         crates/forecast/checkpoints/anchors/, found {}: {:?}. A missing or \
         renamed checkpoint must read RED here, not as a silent skip.",
        pairs.len(),
        pairs.iter().map(|(l, _, _)| l).collect::<Vec<_>>(),
    );

    for (label, original, recalibrated) in pairs {
        let o = original
            .as_object()
            .unwrap_or_else(|| panic!("{label}: original is not a JSON object"));
        let c = recalibrated
            .as_object()
            .unwrap_or_else(|| panic!("{label}: recalibrated is not a JSON object"));

        // Same key set, same order — a dropped or added field is a D2 breach
        // regardless of whether any surviving value changed.
        let ok: Vec<&String> = o.keys().collect();
        let ck: Vec<&String> = c.keys().collect();
        assert_eq!(
            ok, ck,
            "{label}: top-level key vectors differ. ADR-0035 D2 allows a value \
             substitution, never a key change."
        );

        let differ: Vec<&str> = ok
            .iter()
            .filter(|k| {
                forecast::provenance::canonicalise(&o[k.as_str()])
                    != forecast::provenance::canonicalise(&c[k.as_str()])
            })
            .map(|k| k.as_str())
            .collect();
        assert_eq!(
            differ,
            vec!["sigma_train"],
            "{label}: ADR-0035 D2 says exactly one field is substituted, and it \
             is `sigma_train`. Differing under canonicalise: {differ:?}"
        );
    }
}

/// The committed overlay is reproducible from the committed original plus one
/// number, through the shipped code path, byte-for-byte.
///
/// This is the assertion the hand copy could never make: it rebuilds the overlay
/// with `provenance::overlay_with_sigma_train` + `canonicalise` — the same two
/// calls `write_overlay` makes — and compares against the committed file's raw
/// bytes. A change to either function breaks it.
///
/// PROBED 2026-11-08 against the shipped function, three ways, all RED:
/// touching a second field (3 tests red), substituting the wrong field (3 red),
/// and merely ROUNDING the value to 6 dp (2 red — caught here, and invisible to
/// the key-set comparison above). Before this pass, no perturbation of the
/// shipped function could redden this file at all.
#[test]
fn committed_overlays_replay_byte_identically_through_the_shipped_path() {
    let pairs = committed_overlay_pairs();
    assert_eq!(pairs.len(), 2, "floor: expected 2 committed pairs");

    let dir = workspace_root().join("crates/forecast/checkpoints/anchors");
    for (label, original, recalibrated) in pairs {
        let sigma = recalibrated["sigma_train"]
            .as_f64()
            .unwrap_or_else(|| panic!("{label}: recalibrated sigma_train is not a number"));
        let rebuilt = forecast::provenance::overlay_with_sigma_train(&original, sigma)
            .unwrap_or_else(|| panic!("{label}: sigma_train {sigma} is not a finite JSON number"));
        let rebuilt_bytes = forecast::provenance::canonicalise(&rebuilt);
        let committed_bytes =
            std::fs::read(dir.join(&label)).unwrap_or_else(|e| panic!("{label}: read: {e}"));
        assert_eq!(
            rebuilt_bytes,
            committed_bytes,
            "{label}: replaying the committed original through \
             overlay_with_sigma_train + canonicalise does NOT reproduce the \
             committed overlay. Either the shipped substitution changed, or the \
             canonical form did, or the committed overlay was not produced by \
             this path. rebuilt {} bytes, committed {} bytes.",
            rebuilt_bytes.len(),
            committed_bytes.len(),
        );
    }
}

/// Split a canonical metadata JSON (sorted keys, no whitespace) into its
/// top-level `key -> raw value text` pairs, WITHOUT parsing the numbers.
///
/// Deliberately text-level: every other comparison in this file goes through
/// `serde_json::Value` and therefore cannot see a difference serde_json's parser
/// collapses. This is the only view that can.
fn top_level_raw(text: &str) -> Vec<(String, String)> {
    let body = text.trim().trim_start_matches('{').trim_end_matches('}');
    let mut out = Vec::new();
    let b = body.as_bytes();
    let mut i = 0usize;
    while i < b.len() {
        // key
        while i < b.len() && b[i] != b'"' {
            i += 1;
        }
        if i >= b.len() {
            break;
        }
        let ks = i + 1;
        i += 1;
        while i < b.len() && b[i] != b'"' {
            i += 1;
        }
        let key = body[ks..i].to_string();
        i += 1; // closing quote
        while i < b.len() && b[i] != b':' {
            i += 1;
        }
        i += 1; // colon
        // value: to the next top-level comma
        let vs = i;
        let (mut depth, mut in_str, mut esc) = (0i32, false, false);
        while i < b.len() {
            let c = b[i];
            if in_str {
                if esc {
                    esc = false;
                } else if c == b'\\' {
                    esc = true;
                } else if c == b'"' {
                    in_str = false;
                }
            } else {
                match c {
                    b'"' => in_str = true,
                    b'{' | b'[' => depth += 1,
                    b'}' | b']' => depth -= 1,
                    b',' if depth == 0 => break,
                    _ => {}
                }
            }
            i += 1;
        }
        out.push((key, body[vs..i].to_string()));
        i += 1; // comma
    }
    out
}

/// The file-TEXT divergence between each committed pair is exactly the declared
/// set — so the one known serde_json artefact is recorded, and a NEW divergence
/// on any other field fails.
///
/// This is the assertion that `committed_overlays_substitute_exactly_sigma_train`
/// structurally cannot make: it compares raw value text, never a parsed `f64`.
#[test]
fn committed_overlay_text_divergence_is_exactly_the_declared_set() {
    // bs1: only the substituted field.
    // bs2: the substituted field PLUS `final_val_loss`, which nothing
    //      substituted — serde_json parses the original's "…069431" to the f64
    //      whose shortest form is "…069433" (1 ULP off correct rounding) and
    //      canonicalise writes that back. Declared here rather than tolerated
    //      silently; see
    //      docs/dev-notes/recalibrate-field-invariance-deviation-2026-11-08.md.
    let declared: &[(&str, &[&str])] = &[
        ("tcn-bs1-", &["sigma_train"]),
        ("tcn-bs2-", &["final_val_loss", "sigma_train"]),
    ];

    let dir = workspace_root().join("crates/forecast/checkpoints/anchors");
    let mut checked = 0usize;
    for (prefix, expected) in declared {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            panic!("cannot read {}", dir.display())
        };
        let mut recal: Vec<PathBuf> = entries
            .filter_map(Result::ok)
            .map(|e| e.path())
            .filter(|p| {
                let n = p
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
                n.starts_with(prefix) && n.ends_with(".metadata.recalibrated.json")
            })
            .collect();
        recal.sort();
        assert_eq!(
            recal.len(),
            1,
            "expected exactly one committed overlay for prefix {prefix:?}, found {}",
            recal.len()
        );
        let r = &recal[0];
        let o = PathBuf::from(
            r.to_string_lossy()
                .replace(".metadata.recalibrated.json", ".metadata.json"),
        );
        let ot = std::fs::read_to_string(&o).expect("read original");
        let ct = std::fs::read_to_string(r).expect("read overlay");

        let orig = top_level_raw(&ot);
        let recal_pairs = top_level_raw(&ct);
        assert!(
            orig.len() >= 9,
            "{prefix}: the raw splitter found only {} top-level keys, which cannot              be right for a metadata file — it has stopped parsing, and every              comparison below would be vacuous",
            orig.len()
        );
        assert_eq!(
            orig.iter().map(|(k, _)| k).collect::<Vec<_>>(),
            recal_pairs.iter().map(|(k, _)| k).collect::<Vec<_>>(),
            "{prefix}: top-level key order differs at the text level"
        );

        let differ: Vec<&str> = orig
            .iter()
            .zip(recal_pairs.iter())
            .filter(|((_, ov), (_, cv))| ov != cv)
            .map(|((k, _), _)| k.as_str())
            .collect();
        assert_eq!(
            differ, *expected,
            "{prefix}: the set of fields whose RAW TEXT differs between original              and overlay is not the declared one. Declared {expected:?}, found              {differ:?}. A new entry here is either a D2 scope breach or a second              float round-trip artefact — find out which before widening the              declaration."
        );
        checked += 1;
    }
    assert_eq!(
        checked, 2,
        "floor: both declared pairs must have been checked"
    );
}
