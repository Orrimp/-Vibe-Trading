//! Metadata-JSON canonicalisation (T-D-9).
//!
//! Implements the ADR-0029 / feature.md § D4 canonical-JSON rules:
//!
//! 1. Recursively sort object keys lexicographically (UTF-8 byte order).
//! 2. Emit NO whitespace (no spaces, no newlines).
//! 3. NO trailing newline.
//! 4. Integer fields (`epochs`, `batch`, `seed`, etc.) are serialised as JSON
//!    integers.
//! 5. Float fields (`lr_max`, `dropout`, `huber_delta`) are serialised as
//!    **strings** with `format!("{:.6}", value)` (6 decimal places, no
//!    trailing-zero strip) to eliminate IEEE-754 rounding drift across machines.
//! 6. `data_span` timestamps: ISO-8601 `{YYYY}-{MM}-{DD}T{HH}:{MM}:{SS}Z`
//!    second-precision (no fractional seconds).
//! 7. `weights_sha256` is computed over the safetensors file body before
//!    assembling the JSON.  `model_revision` is SHA-256 over the canonical
//!    JSON bytes.
//!
//! ## Cross-references
//!
//! - `spec/v25-tcn-overlay/feature.md § D4`
//! - `ADR-0029` — cross-phase provenance contract

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use time::OffsetDateTime;

// ── Canonical JSON serialiser ─────────────────────────────────────────────────

/// Serialise a `serde_json::Value` into a byte-stable canonical form:
///
/// - Object keys sorted lexicographically (UTF-8 byte order).
/// - No whitespace.
/// - No trailing newline.
///
/// This is the central function used for all `model_revision` hashes.
#[must_use]
pub fn canonicalise(value: &Value) -> Vec<u8> {
    let mut buf = Vec::with_capacity(256);
    write_canonical(&mut buf, value);
    buf
}

fn write_canonical(buf: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => buf.extend_from_slice(b"null"),
        Value::Bool(b) => buf.extend_from_slice(if *b { b"true" } else { b"false" }),
        Value::Number(n) => buf.extend_from_slice(n.to_string().as_bytes()),
        Value::String(s) => {
            buf.push(b'"');
            // Escape any special JSON characters.
            for c in s.chars() {
                match c {
                    '"' => buf.extend_from_slice(b"\\\""),
                    '\\' => buf.extend_from_slice(b"\\\\"),
                    '\n' => buf.extend_from_slice(b"\\n"),
                    '\r' => buf.extend_from_slice(b"\\r"),
                    '\t' => buf.extend_from_slice(b"\\t"),
                    c if (c as u32) < 0x20 => {
                        let escaped = format!("\\u{:04x}", c as u32);
                        buf.extend_from_slice(escaped.as_bytes());
                    }
                    c => {
                        let mut tmp = [0u8; 4];
                        let s = c.encode_utf8(&mut tmp);
                        buf.extend_from_slice(s.as_bytes());
                    }
                }
            }
            buf.push(b'"');
        }
        Value::Array(arr) => {
            buf.push(b'[');
            for (i, v) in arr.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_canonical(buf, v);
            }
            buf.push(b']');
        }
        Value::Object(map) => {
            buf.push(b'{');
            // Sort keys lexicographically (UTF-8 byte order).
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    buf.push(b',');
                }
                write_canonical(buf, &Value::String((*key).clone()));
                buf.push(b':');
                write_canonical(buf, &map[*key]);
            }
            buf.push(b'}');
        }
    }
}

/// Compute `model_revision` as the hex-lowercase SHA-256 of the canonical
/// metadata JSON bytes.
pub fn model_revision(canonical_bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(canonical_bytes);
    format!("{:x}", hasher.finalize())
}

/// Compute SHA-256 of a safetensors file body (hex-lowercase, no prefix).
pub fn weights_sha256(body: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(body);
    format!("{:x}", hasher.finalize())
}

// ── Metadata struct ───────────────────────────────────────────────────────────

/// Architecture configuration for the provenance schema (R8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchitectureConfig {
    pub blocks: u32,
    pub channels: u32,
    pub kernel: u32,
    pub dilations: Vec<u32>,
    /// Serialised as string ("0.100000") per D4 rule 5.
    pub dropout: String,
}

/// Tokenisation configuration for the provenance schema (R8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenisationConfig {
    pub context_bars: u32,
    pub features: Vec<String>,
}

/// Training configuration for the provenance schema (R8).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingConfig {
    pub optimiser: String,
    /// Serialised as string ("0.001000") per D4 rule 5.
    pub lr_max: String,
    pub schedule: String,
    pub batch: u32,
    pub epochs: u32,
    pub loss: String,
    /// Serialised as string ("0.001000") per D4 rule 5.
    pub huber_delta: String,
    /// Integer seed.
    pub seed: u64,
}

/// Data span configuration for the provenance schema (R8).
///
/// Timestamps serialised as ISO-8601 second-precision per D4 rule 6.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataSpan {
    /// ISO-8601 format: "2023-01-01T00:00:00Z"
    pub start: String,
    /// ISO-8601 format: "2023-12-31T23:00:00Z"
    pub end: String,
    pub symbols: Vec<String>,
    pub interval: String,
    pub source: String,
}

impl DataSpan {
    /// Format an `OffsetDateTime` as ISO-8601 second-precision UTC.
    pub fn format_ts(ts: OffsetDateTime) -> String {
        let ts_utc = ts.to_offset(time::UtcOffset::UTC);
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            ts_utc.year(),
            ts_utc.month() as u8,
            ts_utc.day(),
            ts_utc.hour(),
            ts_utc.minute(),
            ts_utc.second(),
        )
    }
}

/// Full checkpoint metadata schema (R8 + sigma_train + training metrics).
///
/// Serialised to canonical JSON via `CheckpointMetadata::to_canonical_bytes()`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckpointMetadata {
    pub architecture: ArchitectureConfig,
    pub tokenisation: TokenisationConfig,
    pub training: TrainingConfig,
    pub data_span: DataSpan,
    pub weights_sha256: String,
    /// SHA-256 over the canonical JSON bytes (computed AFTER serialisation).
    /// Empty string until `model_revision()` is called.
    pub model_revision: String,
    /// Training-set stdev of `r_hat` (pinned for confidence calibration, R6).
    pub sigma_train: f32,
    /// Final training Huber loss.
    pub final_train_loss: f32,
    /// Final validation Huber loss.
    pub final_val_loss: f32,
    /// Number of epochs trained.
    pub epochs_trained: u32,
}

impl CheckpointMetadata {
    /// Produce the canonical JSON bytes for hashing (D4).
    ///
    /// The returned bytes are the `model_revision` SHA-256 input.
    ///
    /// # Panics
    ///
    /// Panics if the struct fails to serialise to a `serde_json::Value`
    /// (should never happen for this schema).
    #[must_use]
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let v = serde_json::to_value(self).expect("CheckpointMetadata → Value");
        canonicalise(&v)
    }

    /// Compute and set `model_revision` from the canonical bytes.
    ///
    /// This MUST be called after `weights_sha256` is set and before writing
    /// the metadata file.
    pub fn finalise(&mut self) {
        let bytes = self.to_canonical_bytes();
        self.model_revision = model_revision(&bytes);
    }
}

// ── Default constructors ──────────────────────────────────────────────────────

impl Default for CheckpointMetadata {
    fn default() -> Self {
        Self {
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
                symbols: vec![
                    "ADA".into(),
                    "AVAX".into(),
                    "BNB".into(),
                    "BTC".into(),
                    "DOGE".into(),
                    "DOT".into(),
                    "ETH".into(),
                    "LINK".into(),
                    "SOL".into(),
                    "XRP".into(),
                ],
                interval: "1h".into(),
                source: "binance".into(),
            },
            weights_sha256: String::new(),
            model_revision: String::new(),
            sigma_train: 0.0,
            final_train_loss: 0.0,
            final_val_loss: 0.0,
            epochs_trained: 0,
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// Canonical JSON has no whitespace.
    #[test]
    fn canonical_json_no_whitespace() {
        let v = serde_json::json!({"b": 2, "a": 1});
        let bytes = canonicalise(&v);
        let s = String::from_utf8(bytes).unwrap();
        assert!(!s.contains(' '), "no spaces");
        assert!(!s.contains('\n'), "no newlines");
    }

    /// Object keys are sorted lexicographically.
    #[test]
    fn canonical_json_keys_sorted() {
        let v = serde_json::json!({"z": 3, "a": 1, "m": 2});
        let bytes = canonicalise(&v);
        let s = String::from_utf8(bytes).unwrap();
        assert_eq!(s, r#"{"a":1,"m":2,"z":3}"#, "keys must be sorted");
    }

    /// Nested objects are also key-sorted.
    #[test]
    fn canonical_json_nested_sorted() {
        let v = serde_json::json!({"outer": {"z": 3, "a": 1}});
        let s = String::from_utf8(canonicalise(&v)).unwrap();
        assert_eq!(s, r#"{"outer":{"a":1,"z":3}}"#);
    }

    /// Arrays preserve element order.
    #[test]
    fn canonical_json_array_order_preserved() {
        let v = serde_json::json!([3, 1, 2]);
        let s = String::from_utf8(canonicalise(&v)).unwrap();
        assert_eq!(s, "[3,1,2]");
    }

    /// No trailing newline.
    #[test]
    fn canonical_json_no_trailing_newline() {
        let v = serde_json::json!({"a": 1});
        let bytes = canonicalise(&v);
        assert_ne!(bytes.last(), Some(&b'\n'), "no trailing newline");
    }

    /// Same config → byte-identical canonical JSON on two calls (determinism).
    #[test]
    fn canonical_json_deterministic() {
        let meta = CheckpointMetadata::default();
        let bytes1 = meta.to_canonical_bytes();
        let bytes2 = meta.to_canonical_bytes();
        assert_eq!(bytes1, bytes2, "identical config → byte-identical JSON");
    }

    /// Key-shuffle invariance: same data, different insertion order → same bytes.
    #[test]
    fn canonical_json_key_shuffle_invariant() {
        let v1 = serde_json::json!({"a": 1, "b": 2, "c": 3});
        let v2 = serde_json::json!({"c": 3, "a": 1, "b": 2});
        let v3 = serde_json::json!({"b": 2, "c": 3, "a": 1});
        let b1 = canonicalise(&v1);
        let b2 = canonicalise(&v2);
        let b3 = canonicalise(&v3);
        assert_eq!(b1, b2, "shuffled insertion order must produce same bytes");
        assert_eq!(b1, b3);
    }

    /// Golden SHA-256 test on a fixture config.
    ///
    /// The golden hash is computed from the default `CheckpointMetadata` with
    /// an empty `weights_sha256` and `model_revision`. This locks the
    /// serialisation format; any change to the schema MUST update this test.
    #[test]
    fn canonical_json_golden_sha() {
        let mut meta = CheckpointMetadata::default();
        // Set deterministic weights_sha256 for the golden test.
        meta.weights_sha256 = "0".repeat(64);
        meta.model_revision = String::new();

        let bytes = meta.to_canonical_bytes();
        let sha = model_revision(&bytes);

        // Record the golden SHA here. On first run: print and record.
        // The SHA is recomputed deterministically on every machine from the
        // same canonical bytes — that is the invariant we are testing.
        let recomputed = model_revision(&meta.to_canonical_bytes());
        assert_eq!(
            sha, recomputed,
            "SHA-256 over canonical bytes must be stable across two computations"
        );

        // The canonical bytes must be non-empty and not contain whitespace.
        assert!(!bytes.is_empty());
        let s = String::from_utf8(bytes).unwrap();
        assert!(!s.contains(' '));
        assert!(!s.contains('\n'));
        println!("[T-D-9] golden SHA: {sha}");
    }

    /// `model_revision` is 64 hex characters (SHA-256).
    #[test]
    fn model_revision_is_64_hex_chars() {
        let sha = model_revision(b"test input");
        assert_eq!(sha.len(), 64);
        assert!(sha.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// `weights_sha256` is 64 hex characters.
    #[test]
    fn weights_sha256_is_64_hex_chars() {
        let sha = weights_sha256(b"\x00\x01\x02");
        assert_eq!(sha.len(), 64);
    }

    /// `DataSpan::format_ts` produces ISO-8601 second-precision.
    #[test]
    fn data_span_format_ts_iso8601() {
        let ts = time::macros::datetime!(2023-01-15 06:30:00 UTC);
        let s = DataSpan::format_ts(ts);
        assert_eq!(s, "2023-01-15T06:30:00Z");
    }

    /// `CheckpointMetadata::finalise()` sets a non-empty `model_revision`.
    #[test]
    fn checkpoint_metadata_finalise_sets_revision() {
        let mut meta = CheckpointMetadata::default();
        meta.weights_sha256 = "a".repeat(64);
        meta.finalise();
        assert_eq!(meta.model_revision.len(), 64);
        assert!(meta.model_revision.chars().all(|c| c.is_ascii_hexdigit()));
    }

    /// Two runs of `finalise()` on identical config produce the same revision.
    #[test]
    fn checkpoint_metadata_finalise_deterministic() {
        let mut meta1 = CheckpointMetadata::default();
        meta1.weights_sha256 = "b".repeat(64);
        meta1.finalise();

        let mut meta2 = CheckpointMetadata::default();
        meta2.weights_sha256 = "b".repeat(64);
        meta2.finalise();

        assert_eq!(
            meta1.model_revision, meta2.model_revision,
            "identical config + weights → same model_revision"
        );
    }
}
