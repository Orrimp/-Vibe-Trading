//! REVISION.toml manifest write + verify for `data/binance/`.
//!
//! This module is the single source of truth for the aggregate-SHA algorithm
//! described in ADR-0032 § 2. Both `fetch_binance_klines --emit-revision-manifest`
//! (writer) and `backtest::realdata::RealDataBarSource::load()` (verifier) call
//! into this module so the two implementations are always byte-for-byte identical.
//!
//! # Aggregate-SHA algorithm
//!
//! ```text
//! entries = sorted_lexicographically(files.entries())   // by relative path
//! buf     = b""
//! for (relpath, sha256) in entries:
//!     buf += relpath.as_bytes() + b"\t" + sha256.as_bytes() + b"\n"
//! revision_sha = hex(sha256(buf))
//! ```
//!
//! The `[revision.metadata]` section is advisory only and NOT part of the
//! aggregate SHA — only the `[files]` sorted-map entries are hashed.

use std::collections::BTreeMap;
use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};
use thiserror::Error;

// ── Error type ────────────────────────────────────────────────────────────────

#[derive(Error, Debug)]
pub enum RevisionError {
    #[error("REVISION.toml not found at {path}")]
    Missing { path: String },
    #[error("REVISION.toml parse error: {0}")]
    Parse(String),
    #[error("data revision mismatch for {file}: manifest={manifest_sha}, on-disk={actual_sha}")]
    FileMismatch {
        file: String,
        manifest_sha: String,
        actual_sha: String,
    },
    #[error("aggregate SHA mismatch: manifest claimed {claimed}, recomputed {recomputed}")]
    AggregateMismatch { claimed: String, recomputed: String },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("toml serialization error: {0}")]
    TomlSer(String),
}

// ── TOML schema structs ───────────────────────────────────────────────────────

/// The `[revision]` table.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RevisionSection {
    sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    metadata: Option<RevisionMetadata>,
}

/// The `[revision.metadata]` table — advisory, NOT hashed.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RevisionMetadata {
    generated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    binance_base: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fetch_tool: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fetch_version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    interval: Option<String>,
}

/// The full `REVISION.toml` document.
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct RevisionManifest {
    revision: RevisionSection,
    files: BTreeMap<String, String>,
}

// ── Core algorithm ────────────────────────────────────────────────────────────

/// Compute the SHA-256 hex digest of a single file.
///
/// Reads the entire file into memory (files are ~25 KB each).
pub fn file_sha256(path: &Path) -> Result<String, RevisionError> {
    let mut f = std::fs::File::open(path)?;
    let mut buf = Vec::new();
    f.read_to_end(&mut buf)?;
    let digest = Sha256::digest(&buf);
    Ok(format!("{digest:x}"))
}

/// Compute the aggregate SHA-256 over a sorted `(relpath, file_sha)` map.
///
/// Algorithm (from ADR-0032 § 2, identical in writer and verifier):
/// ```text
/// for (relpath, sha256) in sorted_entries:
///     buf += relpath + "\t" + sha256 + "\n"
/// aggregate = hex(sha256(buf))
/// ```
///
/// The entries must already be lexicographically sorted (BTreeMap guarantees this).
#[must_use]
pub fn compute_aggregate_sha(files: &BTreeMap<String, String>) -> String {
    let mut hasher = Sha256::new();
    for (relpath, sha256) in files.iter() {
        hasher.update(relpath.as_bytes());
        hasher.update(b"\t");
        hasher.update(sha256.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Scan all `.parquet` files under `root`, compute per-file SHA-256, write
/// `REVISION.toml` to `root/REVISION.toml`.
///
/// Returns the aggregate SHA-256 written.
///
/// The `[revision.metadata]` section records the current UTC time as advisory
/// information. The aggregate hash is NOT influenced by metadata.
pub fn write_revision_manifest(root: &Path) -> Result<String, RevisionError> {
    // Collect all parquet files relative to root.
    let files = collect_parquet_files(root)?;
    let aggregate = compute_aggregate_sha(&files);

    let now = time::OffsetDateTime::now_utc();
    let generated_at = format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        now.year(),
        now.month() as u8,
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    );

    let manifest = RevisionManifest {
        revision: RevisionSection {
            sha256: aggregate.clone(),
            metadata: Some(RevisionMetadata {
                generated_at,
                binance_base: Some("https://api.binance.com".to_string()),
                fetch_tool: Some("fetch_binance_klines".to_string()),
                fetch_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                interval: Some("1h".to_string()),
            }),
        },
        files,
    };

    let toml_str =
        toml::to_string_pretty(&manifest).map_err(|e| RevisionError::TomlSer(e.to_string()))?;

    let manifest_path = root.join("REVISION.toml");
    std::fs::write(&manifest_path, toml_str)?;

    Ok(aggregate)
}

/// Read and verify `root/REVISION.toml`.
///
/// Verification steps (from ADR-0032 § 2):
/// 1. Manifest file exists.
/// 2. Every file listed in `[files]` has an on-disk SHA that matches.
/// 3. Recompute the aggregate from `[files]` and verify it matches `[revision].sha256`.
///
/// Returns the **recomputed** aggregate SHA (never the manifest's claimed value),
/// so a hand-edit cannot fool the anchor lock.
pub fn read_and_verify_revision_manifest(root: &Path) -> Result<String, RevisionError> {
    let manifest_path = root.join("REVISION.toml");
    if !manifest_path.exists() {
        return Err(RevisionError::Missing {
            path: manifest_path.to_string_lossy().into_owned(),
        });
    }

    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest: RevisionManifest =
        toml::from_str(&content).map_err(|e| RevisionError::Parse(e.to_string()))?;

    // Step 2: verify each file's on-disk SHA matches the manifest.
    for (relpath, manifest_sha) in &manifest.files {
        let abs_path = root.join(relpath);
        let actual_sha = file_sha256(&abs_path)?;
        if &actual_sha != manifest_sha {
            return Err(RevisionError::FileMismatch {
                file: relpath.clone(),
                manifest_sha: manifest_sha.clone(),
                actual_sha,
            });
        }
    }

    // Step 3: recompute aggregate and verify.
    let recomputed = compute_aggregate_sha(&manifest.files);
    if recomputed != manifest.revision.sha256 {
        return Err(RevisionError::AggregateMismatch {
            claimed: manifest.revision.sha256.clone(),
            recomputed,
        });
    }

    Ok(recomputed)
}

/// Read ONLY the manifest file (no on-disk SHA verification).
///
/// Returns `(files_map, claimed_aggregate_sha)`. Used by `RealDataBarSource`
/// for selective-file verification (only files in the scenario's span).
pub fn read_manifest_raw(root: &Path) -> Result<(BTreeMap<String, String>, String), RevisionError> {
    let manifest_path = root.join("REVISION.toml");
    if !manifest_path.exists() {
        return Err(RevisionError::Missing {
            path: manifest_path.to_string_lossy().into_owned(),
        });
    }

    let content = std::fs::read_to_string(&manifest_path)?;
    let manifest: RevisionManifest =
        toml::from_str(&content).map_err(|e| RevisionError::Parse(e.to_string()))?;

    Ok((manifest.files, manifest.revision.sha256))
}

// ── Internal helpers ──────────────────────────────────────────────────────────

/// Recursively collect all `.parquet` files under `root`, returning a
/// `BTreeMap<relative_path_string, sha256_hex>`.
///
/// Relative paths use `/` as separator on all platforms (for determinism).
fn collect_parquet_files(root: &Path) -> Result<BTreeMap<String, String>, RevisionError> {
    let mut files = BTreeMap::new();
    collect_recursive(root, root, &mut files)?;
    Ok(files)
}

fn collect_recursive(
    root: &Path,
    dir: &Path,
    files: &mut BTreeMap<String, String>,
) -> Result<(), RevisionError> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_recursive(root, &path, files)?;
        } else if path.extension().is_some_and(|e| e == "parquet") {
            // Compute relative path with / separators.
            let rel = path.strip_prefix(root).map_err(|_| {
                RevisionError::Io(std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    format!("{} is not under {}", path.display(), root.display()),
                ))
            })?;
            let rel_str = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            let sha = file_sha256(&path)?;
            files.insert(rel_str, sha);
        }
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    /// Create a tiny fake parquet file with known bytes.
    fn make_fake_parquet(dir: &Path, relpath: &str, content: &[u8]) {
        let full = dir.join(relpath);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(&full, content).unwrap();
    }

    /// Hand-compute the expected aggregate SHA for 2 files.
    ///
    /// Files (sorted by relpath):
    ///   "ADAUSDT/2023/01.parquet" -> sha256(b"aaa")
    ///   "BTCUSDT/2023/01.parquet" -> sha256(b"bbb")
    ///
    /// sha256(b"aaa") = 9834876dcfb05cb167a5c24953eba58c4ac89b1adf57f28f2f9d09af107ee8f0
    /// sha256(b"bbb") = 3b9c358f36f0a31b6ad3e14f309c7cf198ac9246e8316f9111558d9c5da19ae3 (note: verify below)
    ///
    /// buf = "ADAUSDT/2023/01.parquet\t{sha_ada}\nBTCUSDT/2023/01.parquet\t{sha_btc}\n"
    /// aggregate = hex(sha256(buf))
    fn expected_aggregate(ada_sha: &str, btc_sha: &str) -> String {
        let buf =
            format!("ADAUSDT/2023/01.parquet\t{ada_sha}\nBTCUSDT/2023/01.parquet\t{btc_sha}\n");
        let digest = Sha256::digest(buf.as_bytes());
        format!("{digest:x}")
    }

    #[test]
    fn test_compute_aggregate_sha_two_files() {
        // Known content → known SHA-256.
        let ada_content = b"aaa";
        let btc_content = b"bbb";
        let ada_sha = format!("{:x}", Sha256::digest(ada_content));
        let btc_sha = format!("{:x}", Sha256::digest(btc_content));

        let mut files = BTreeMap::new();
        files.insert("ADAUSDT/2023/01.parquet".to_string(), ada_sha.clone());
        files.insert("BTCUSDT/2023/01.parquet".to_string(), btc_sha.clone());

        let got = compute_aggregate_sha(&files);
        let want = expected_aggregate(&ada_sha, &btc_sha);
        assert_eq!(got, want, "aggregate SHA mismatch");
        assert_eq!(got.len(), 64, "SHA-256 hex should be 64 chars");
    }

    #[test]
    fn test_aggregate_sha_is_order_independent_of_insertion_but_sorted() {
        // BTreeMap sorts by key, so insertion order must not matter.
        let ada_sha = format!("{:x}", Sha256::digest(b"aaa"));
        let btc_sha = format!("{:x}", Sha256::digest(b"bbb"));

        let mut files1 = BTreeMap::new();
        files1.insert("ADAUSDT/2023/01.parquet".to_string(), ada_sha.clone());
        files1.insert("BTCUSDT/2023/01.parquet".to_string(), btc_sha.clone());

        let mut files2 = BTreeMap::new();
        files2.insert("BTCUSDT/2023/01.parquet".to_string(), btc_sha.clone());
        files2.insert("ADAUSDT/2023/01.parquet".to_string(), ada_sha.clone());

        assert_eq!(
            compute_aggregate_sha(&files1),
            compute_aggregate_sha(&files2),
            "aggregate SHA must be same regardless of BTreeMap insertion order"
        );
    }

    #[test]
    fn test_write_and_verify_roundtrip() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        make_fake_parquet(root, "ADAUSDT/2023/01.parquet", b"fake_ada_data");
        make_fake_parquet(root, "BTCUSDT/2023/01.parquet", b"fake_btc_data");

        let written_sha = write_revision_manifest(root).unwrap();
        assert_eq!(written_sha.len(), 64);

        let verified_sha = read_and_verify_revision_manifest(root).unwrap();
        assert_eq!(written_sha, verified_sha, "roundtrip SHA must match");
    }

    #[test]
    fn test_verify_detects_file_tamper() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        make_fake_parquet(root, "ADAUSDT/2023/01.parquet", b"original_data");
        write_revision_manifest(root).unwrap();

        // Tamper with the parquet file AFTER writing the manifest.
        fs::write(root.join("ADAUSDT/2023/01.parquet"), b"tampered_data").unwrap();

        let err = read_and_verify_revision_manifest(root).unwrap_err();
        assert!(
            matches!(err, RevisionError::FileMismatch { .. }),
            "expected FileMismatch, got: {err}"
        );
    }

    #[test]
    fn test_verify_missing_manifest() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        let err = read_and_verify_revision_manifest(root).unwrap_err();
        assert!(
            matches!(err, RevisionError::Missing { .. }),
            "expected Missing, got: {err}"
        );
    }
}
