//! Phase F — Models-screen checkpoint registry reader (R5.2 / T-D-N9).
//!
//! Walks `crates/forecast/checkpoints/anchors/*.metadata.json`, parses each
//! JSON file with `serde_json`, and builds a `Vec<CheckpointMeta>` view-model.
//!
//! **K2 robustness contract:** every non-load-bearing field carries
//! `#[serde(default)]`. Malformed JSON returns `None` from `parse_metadata`
//! and the file is skipped with a `tracing::warn!` breadcrumb. Phase F
//! never panics on schema drift (H5).
//!
//! **K3 sparkline deferred:** no residual fetch at v0.1.0. Row layout ships
//! with `MODELS_SPARKLINE_PLACEHOLDER` (`—`) + `MODELS_SPARKLINE_DEFERRED_TOOLTIP`.

use std::path::{Path, PathBuf};

use crate::models::state::{CheckpointMeta, ModelFamily, ModelStatus};

// ── Serde shapes (wire format from metadata.json) ────────────────────────────

/// Phase F — serde shape for `*.metadata.json` architecture block (§ 1.2).
///
/// `#[serde(default)]` on every non-load-bearing field — `PatchTST` / `Transformer`
/// families may omit `blocks` / `dilations` / `kernel` (H5).
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct CheckpointArchitecture {
    pub blocks: u32,
    pub channels: u32,
    pub dilations: Vec<u32>,
    /// String form (e.g. `"0.100000"`) — v0.1.0 renders as-is.
    pub dropout: String,
    pub kernel: u32,
}

/// Phase F — serde shape for `*.metadata.json` `data_span` block (§ 1.2).
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct CheckpointDataSpan {
    pub start: String,
    pub end: String,
    pub interval: String,
    pub source: String,
    pub symbols: Vec<String>,
}

/// Phase F — serde shape for the full `*.metadata.json` file (§ 1.2).
///
/// `#[serde(default)]` at the struct level means every field falls back to its
/// `Default::default()` when absent — full K2 robustness contract honored.
///
/// **Opaque blobs:** `tokenisation` + `training` are stored as
/// `serde_json::Value` so future schema drift in those subtrees doesn't cause
/// parse failures. v0.1.0 never renders their contents.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(default)]
pub struct CheckpointMetadata {
    pub model_revision: String,
    pub epochs_trained: u32,
    pub final_train_loss: f64,
    pub final_val_loss: f64,
    pub sigma_train: f64,
    pub weights_sha256: String,
    pub architecture: CheckpointArchitecture,
    pub data_span: CheckpointDataSpan,
    /// Opaque blob — v0.1.0 does not render tokenisation details.
    pub tokenisation: serde_json::Value,
    /// Opaque blob — v0.1.0 does not render training hyperparams.
    pub training: serde_json::Value,
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Discover all checkpoints under `checkpoint_dir` by globbing
/// `<checkpoint_dir>/*.metadata.json`.
///
/// For each file:
/// 1. Read it as UTF-8 text.
/// 2. Stat the sibling `.safetensors` file for `file_size_bytes`.
/// 3. Parse it with `serde_json::from_str::<CheckpointMetadata>`.
/// 4. Discriminate the `ModelFamily` from the filename prefix.
/// 5. Build a `CheckpointMeta` view-model.
///
/// Files that fail to parse (malformed JSON, non-UTF-8) are skipped with a
/// `tracing::warn!` breadcrumb — H5 "never panic on schema drift" contract.
/// Unknown-family prefixes are also skipped with a `tracing::warn!`.
///
/// Returns `Vec<CheckpointMeta>` sorted by `model_revision` for deterministic
/// row ordering in the Models screen. Empty vec if `checkpoint_dir` does not
/// exist or contains no `*.metadata.json` files.
#[must_use]
pub fn discover_checkpoints(checkpoint_dir: &Path) -> Vec<CheckpointMeta> {
    let Ok(entries) = std::fs::read_dir(checkpoint_dir) else {
        tracing::debug!(
            path = %checkpoint_dir.display(),
            "discover_checkpoints: directory not found or not readable — returning empty list"
        );
        return Vec::new();
    };

    let mut results: Vec<CheckpointMeta> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.extension()?.to_str()? != "json" {
                return None;
            }
            // Only process files whose stem ends with `.metadata`
            // (FileName shape: `tcn-bs1-<sha>.metadata.json`).
            let stem = path.file_stem()?.to_str()?;
            if !stem.ends_with(".metadata") {
                return None;
            }
            parse_checkpoint_file(&path)
        })
        .collect();

    // Sort by model_revision for deterministic display order.
    results.sort_by(|a, b| a.model_revision.cmp(&b.model_revision));
    results
}

/// Parse a single `*.metadata.json` file into `CheckpointMeta`.
///
/// Returns `None` on any error (file read failure, JSON parse failure,
/// unknown family prefix). All error paths emit `tracing::warn!`.
fn parse_checkpoint_file(meta_path: &Path) -> Option<CheckpointMeta> {
    // Discriminate family from the filename stem prefix BEFORE reading
    // (if the prefix is unknown we skip the read entirely).
    let file_name = meta_path.file_name()?.to_str()?;
    let family = discriminate_family(file_name)?;

    // Read the metadata JSON.
    let json_text = std::fs::read_to_string(meta_path)
        .map_err(|e| {
            tracing::warn!(
                path = %meta_path.display(),
                error = %e,
                "discover_checkpoints: failed to read metadata file — skipping"
            );
        })
        .ok()?;

    parse_metadata(&json_text, meta_path, family)
}

/// Parse a JSON string into `CheckpointMeta`.
///
/// Exported for unit-testing (H5 falsification). Returns `None` on parse
/// failure; emits `tracing::warn!` with the path and error.
///
/// # Arguments
///
/// * `json_text` — raw JSON content.
/// * `source_path` — path of the source file (for the warn log + view-model).
/// * `family` — pre-discriminated `ModelFamily`.
#[must_use]
pub fn parse_metadata(
    json_text: &str,
    source_path: &Path,
    family: ModelFamily,
) -> Option<CheckpointMeta> {
    let raw: CheckpointMetadata = serde_json::from_str(json_text)
        .map_err(|e| {
            tracing::warn!(
                path = %source_path.display(),
                error = %e,
                "discover_checkpoints: failed to parse metadata JSON — skipping"
            );
        })
        .ok()?;

    // Stat the sibling safetensors file for file_size_bytes.
    let safetensors_path = safetensors_path_from_metadata_path(source_path);
    let file_size_bytes = safetensors_path
        .as_ref()
        .and_then(|p| std::fs::metadata(p).ok())
        .map_or(0, |m| m.len());

    Some(CheckpointMeta {
        model_revision: smol_str::SmolStr::new(&raw.model_revision),
        family,
        data_span_start: smol_str::SmolStr::new(&raw.data_span.start),
        data_span_end: smol_str::SmolStr::new(&raw.data_span.end),
        interval: smol_str::SmolStr::new(&raw.data_span.interval),
        symbols_count: raw.data_span.symbols.len(),
        final_val_loss: raw.final_val_loss,
        final_train_loss: raw.final_train_loss,
        sigma_train: raw.sigma_train,
        weights_sha256: smol_str::SmolStr::new(&raw.weights_sha256),
        file_size_bytes,
        status: ModelStatus::Staged, // Q7=(c) — all checkpoints are Staged at v0.1.0
        source_path: source_path.to_path_buf(),
    })
}

/// Derive the sibling safetensors path from the metadata path.
///
/// `<stem>.metadata.json` → `<stem>.safetensors`.
///
/// Returns `None` if the stem cannot be determined.
fn safetensors_path_from_metadata_path(meta_path: &Path) -> Option<PathBuf> {
    // Stem of `tcn-bs1-<sha>.metadata.json` is `tcn-bs1-<sha>.metadata`.
    // We strip the `.metadata` suffix to get `tcn-bs1-<sha>`.
    let stem_with_suffix = meta_path.file_stem()?.to_str()?;
    let base_stem = stem_with_suffix.strip_suffix(".metadata")?;
    let parent = meta_path.parent()?;
    Some(parent.join(format!("{base_stem}.safetensors")))
}

/// Discriminate `ModelFamily` from the filename prefix.
///
/// Supported prefixes: `tcn-`, `patchtst-`, `transformer-`.
/// Unknown prefixes emit `tracing::warn!` and return `None`.
fn discriminate_family(file_name: &str) -> Option<ModelFamily> {
    if file_name.starts_with("tcn-") {
        Some(ModelFamily::Tcn)
    } else if file_name.starts_with("patchtst-") {
        Some(ModelFamily::PatchTst)
    } else if file_name.starts_with("transformer-") {
        Some(ModelFamily::Transformer)
    } else {
        tracing::warn!(
            file_name,
            "discover_checkpoints: unknown family prefix — skipping file"
        );
        None
    }
}

// ── Unit tests (H5 falsification) ────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used)] // test module: panicking on test-setup failure is appropriate
mod tests {
    use super::*;

    /// Live tcn-bs1 JSON (verbatim from 2026-05-20 on-disk file).
    const FULL_SCHEMA_JSON: &str = r#"{
  "architecture": {
    "blocks": 8,
    "channels": 96,
    "dilations": [1,2,4,8,16,32,64,128],
    "dropout": "0.100000",
    "kernel": 3
  },
  "data_span": {
    "end": "2023-12-31T23:00:00Z",
    "interval": "1h",
    "source": "binance",
    "start": "2023-01-01T00:00:00Z",
    "symbols": ["ADA","AVAX","BNB","BTC","DOGE","DOT","ETH","LINK","SOL","XRP"]
  },
  "epochs_trained": 30,
  "final_train_loss": 0.000012167605746071786,
  "final_val_loss": 0.000015389239706564695,
  "model_revision": "d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2",
  "sigma_train": 10.95425033569336,
  "tokenisation": {"context_bars":256,"features":["logret","logrange"]},
  "training": {"batch":128,"epochs":30,"lr_max":"0.001000"},
  "weights_sha256": "4ed9064a3871d8bc911ad8b288dccfc597caa6a09cca3b2395a9e1717b8c7025"
}"#;

    /// H5 test 1: full schema round-trips correctly.
    #[test]
    fn parse_full_schema_round_trips() {
        let path = Path::new("tcn-bs1-d1c3696d.metadata.json");
        let meta = parse_metadata(FULL_SCHEMA_JSON, path, ModelFamily::Tcn);
        let meta = meta.expect("full schema should parse");
        assert_eq!(
            meta.model_revision.as_str(),
            "d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2"
        );
        assert_eq!(meta.symbols_count, 10);
        assert_eq!(meta.interval.as_str(), "1h");
        assert!((meta.sigma_train - 10.954_250_336).abs() < 1e-6);
        assert_eq!(meta.status, ModelStatus::Staged);
        assert_eq!(meta.family, ModelFamily::Tcn);
    }

    /// H5 test 2: `architecture.dropout` absent → parses with `dropout == ""`.
    #[test]
    fn parse_missing_dropout_uses_default() {
        let json = r#"{
  "architecture": {"blocks": 4, "channels": 64, "dilations": [1,2], "kernel": 3},
  "data_span": {"end": "2023-12-31T23:00:00Z", "interval": "1h", "source": "binance",
                 "start": "2023-01-01T00:00:00Z", "symbols": ["BTC"]},
  "epochs_trained": 10,
  "final_train_loss": 0.001,
  "final_val_loss": 0.002,
  "model_revision": "abcd1234",
  "sigma_train": 5.0,
  "weights_sha256": "deadbeef"
}"#;
        let path = Path::new("tcn-test.metadata.json");
        let meta = parse_metadata(json, path, ModelFamily::Tcn).expect("should parse");
        // `dropout` absent → default empty string — screen renders "—" for this column.
        // The raw struct dropout is String default = ""; it's in architecture.
        // We just verify no panic and the meta is constructed.
        assert_eq!(meta.model_revision.as_str(), "abcd1234");
        assert_eq!(meta.symbols_count, 1);
    }

    /// H5 test 3: `sigma_train` absent → parses with `sigma_train == 0.0`.
    #[test]
    fn parse_missing_sigma_train_uses_default() {
        let json = r#"{
  "architecture": {},
  "data_span": {"end": "2023-12-31T23:00:00Z", "interval": "1h", "source": "binance",
                 "start": "2023-01-01T00:00:00Z", "symbols": ["BTC", "ETH"]},
  "epochs_trained": 5,
  "final_train_loss": 0.003,
  "final_val_loss": 0.004,
  "model_revision": "missing_sigma",
  "weights_sha256": "deadbeef2"
}"#;
        let path = Path::new("tcn-missing-sigma.metadata.json");
        let meta = parse_metadata(json, path, ModelFamily::Tcn).expect("should parse");
        // sigma_train absent → 0.0 default; screen renders "—" for the sigma column.
        assert!((meta.sigma_train - 0.0).abs() < f64::EPSILON);
        assert_eq!(meta.model_revision.as_str(), "missing_sigma");
    }

    /// H5 test 4: malformed (truncated) JSON returns `None`.
    #[test]
    fn parse_malformed_truncated_returns_none() {
        let json = r#"{"model_revision": "broken", "data_span":"#; // truncated
        let path = Path::new("tcn-malformed.metadata.json");
        let meta = parse_metadata(json, path, ModelFamily::Tcn);
        assert!(meta.is_none(), "malformed JSON should return None");
    }

    /// H5 test 5: unknown family prefix is skipped.
    #[test]
    fn discover_checkpoints_skips_unknown_family() {
        // `discriminate_family` returns `None` for unknown prefixes.
        assert!(discriminate_family("unknown-model.metadata.json").is_none());
        assert!(discriminate_family("tcn-bs1.metadata.json").is_some());
        assert!(discriminate_family("patchtst-v1.metadata.json").is_some());
        assert!(discriminate_family("transformer-v1.metadata.json").is_some());
    }
}
