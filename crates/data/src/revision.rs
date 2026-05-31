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

/// Advisory metadata for the `[revision.metadata]` section — NOT hashed.
///
/// Pass to `write_revision_manifest_with_tool` to override the defaults
/// written by `write_revision_manifest`.
pub struct RevisionMetadataInput<'a> {
    /// Tool name, e.g. `"fetch_binance_funding"`.
    pub fetch_tool: &'a str,
    /// Binance REST base URL used to fetch the data.
    pub binance_base: &'a str,
    /// Interval string (informational). Use `None` when not applicable.
    pub interval: Option<&'a str>,
}

/// Scan all `.parquet` files under `root`, compute per-file SHA-256, write
/// `REVISION.toml` to `root/REVISION.toml`.
///
/// Returns the aggregate SHA-256 written.
///
/// The `[revision.metadata]` section records the current UTC time as advisory
/// information. The aggregate hash is NOT influenced by metadata.
pub fn write_revision_manifest(root: &Path) -> Result<String, RevisionError> {
    write_revision_manifest_with_tool(
        root,
        RevisionMetadataInput {
            fetch_tool: "fetch_binance_klines",
            binance_base: "https://api.binance.com",
            interval: Some("1h"),
        },
    )
}

/// Like `write_revision_manifest` but allows the caller to specify advisory
/// metadata so the `REVISION.toml` correctly identifies which tool produced
/// the data.
///
/// The aggregate SHA is unchanged — only the `[revision.metadata]` block
/// differs between callers.
pub fn write_revision_manifest_with_tool(
    root: &Path,
    meta: RevisionMetadataInput<'_>,
) -> Result<String, RevisionError> {
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
                binance_base: Some(meta.binance_base.to_string()),
                fetch_tool: Some(meta.fetch_tool.to_string()),
                fetch_version: Some(env!("CARGO_PKG_VERSION").to_string()),
                interval: meta.interval.map(str::to_string),
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

    // ── Step 1: 250-file roundtrip regression test ───────────────────────────────
    //
    // Production has 240 parquets (10 symbols × 24 months).  We use 10 × 25 = 250
    // here for a small margin.
    //
    // Background: the bug reported in the revision-roundtrip issue was originally
    // suspected to be a TOML key-quoting roundtrip divergence.  Investigation
    // showed the TOML roundtrip is actually correct (both 2-file and 250-file
    // fixtures roundtrip cleanly).  The *real* root cause was that T-D-17 pinned
    // the aggregate SHA of the real `data/binance/` data while the determinism
    // tests (T-D-13/14/15) ran against a synthetic `tempdir` fixture that has a
    // *different* SHA — causing the revision-pin check in main.rs to fail.
    //
    // This test guards the TOML roundtrip invariant: `write_revision_manifest`
    // followed by `read_manifest_raw` + `compute_aggregate_sha` must produce the
    // same SHA regardless of the number of files.  If the TOML serializer ever
    // changes key quoting behaviour in a way that breaks deserialization, this
    // test will catch it before production breakage.

    const SYMBOLS_250: [&str; 10] = [
        "ADAUSDT", "AVAXUSDT", "BNBUSDT", "BTCUSDT", "DOGEUSDT", "DOTUSDT", "ETHUSDT", "LINKUSDT",
        "SOLUSDT", "XRPUSDT",
    ];

    /// Plant 10 × 25 fake parquets into `root` with deterministic content.
    fn plant_250_parquets(root: &Path) {
        for sym in &SYMBOLS_250 {
            for year in [2023_u32, 2024] {
                // 13 months for 2023 (to reach 25 total across 2 years), 12 for 2024
                let month_count = if year == 2023 { 13 } else { 12 };
                for month in 1_u8..=month_count {
                    let relpath = format!("{sym}/{year}/{month:02}.parquet");
                    let content = format!("sym={sym} y={year} m={month:02}");
                    make_fake_parquet(root, &relpath, content.as_bytes());
                }
            }
        }
    }

    /// The regression test.
    ///
    /// 1. Plant ~250 fake parquets.
    /// 2. Write the revision manifest (captures the disk-scan SHA).
    /// 3. Call `read_manifest_raw` (TOML parse) and recompute the aggregate.
    /// 4. Assert the two SHAs are identical.
    ///
    /// This test EXISTS to catch the 240-file TOML-key-quoting divergence
    /// described in the bug report.  It should FAIL before the fix and PASS
    /// after.
    #[test]
    fn test_roundtrip_250_files() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        plant_250_parquets(root);

        // Writer side: disk-scan → manifest write.
        let written_sha = write_revision_manifest(root)
            .unwrap_or_else(|e| panic!("write_revision_manifest failed: {e}"));

        // Verifier side: TOML parse → recompute aggregate.
        let (files_map, _claimed) =
            read_manifest_raw(root).unwrap_or_else(|e| panic!("read_manifest_raw failed: {e}"));
        let recomputed_sha = compute_aggregate_sha(&files_map);

        // Diagnostic: show first differing key if they mismatch.
        if written_sha != recomputed_sha {
            // Recompute writer's map for comparison.
            let writer_map = collect_parquet_files(root)
                .unwrap_or_else(|e| panic!("collect_parquet_files failed: {e}"));
            let writer_keys: Vec<_> = writer_map.keys().collect();
            let reader_keys: Vec<_> = files_map.keys().collect();
            if writer_keys.len() != reader_keys.len() {
                panic!(
                    "roundtrip_250_files: key count mismatch — writer {} vs reader {}\n\
                     first writer key: {:?}\n\
                     first reader key: {:?}",
                    writer_keys.len(),
                    reader_keys.len(),
                    writer_keys.first(),
                    reader_keys.first(),
                );
            }
            for (w, r) in writer_keys.iter().zip(reader_keys.iter()) {
                if w != r {
                    panic!(
                        "roundtrip_250_files: first differing key:\n\
                         writer:  {:?} (len {})\n\
                         reader:  {:?} (len {})",
                        w,
                        w.len(),
                        r,
                        r.len(),
                    );
                }
            }
        }

        assert_eq!(
            written_sha, recomputed_sha,
            "250-file roundtrip failed: writer SHA ({written_sha}) != reader SHA ({recomputed_sha})"
        );
    }

    /// Verify the production `data/binance/REVISION.toml` against on-disk files.
    ///
    /// This test is `#[ignore]` by default because it requires the real parquet
    /// files to be present on disk.  Run it with:
    ///   cargo test -p data --lib revision::tests::test_production_manifest_roundtrip -- --ignored
    #[test]
    #[ignore = "requires real data/binance/ parquet files on disk"]
    fn test_production_manifest_roundtrip() {
        // Find the workspace root relative to this source file.
        let manifest_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        // crates/data -> workspace root is 2 levels up
        let workspace_root = manifest_dir
            .parent()
            .and_then(|p| p.parent())
            .expect("could not find workspace root");
        let binance_root = workspace_root.join("data/binance");
        assert!(
            binance_root.exists(),
            "data/binance/ not found at {binance_root:?} — run with real data present"
        );

        // 1. Read + verify (checks every per-file SHA)
        let verified_sha = read_and_verify_revision_manifest(&binance_root)
            .unwrap_or_else(|e| panic!("read_and_verify_revision_manifest failed: {e}"));

        // 2. Re-scan from disk (what the writer did)
        let disk_map = collect_parquet_files(&binance_root)
            .unwrap_or_else(|e| panic!("collect_parquet_files failed: {e}"));
        let disk_sha = compute_aggregate_sha(&disk_map);

        // 3. Parse manifest raw (what the realdata verifier does)
        let (files_map, claimed_sha) = read_manifest_raw(&binance_root)
            .unwrap_or_else(|e| panic!("read_manifest_raw failed: {e}"));
        let raw_sha = compute_aggregate_sha(&files_map);

        // All three must agree.
        eprintln!("verified_sha  = {verified_sha}");
        eprintln!("disk_sha      = {disk_sha}");
        eprintln!("claimed_sha   = {claimed_sha}");
        eprintln!("raw_sha       = {raw_sha}");
        eprintln!("disk entries  = {}", disk_map.len());
        eprintln!("manifest entries = {}", files_map.len());

        assert_eq!(
            disk_sha, raw_sha,
            "disk-scan SHA differs from read_manifest_raw SHA:\n\
             disk:  {disk_sha}\n\
             raw:   {raw_sha}"
        );
        assert_eq!(
            disk_sha, claimed_sha,
            "disk SHA differs from REVISION.toml claimed SHA:\n\
             disk:    {disk_sha}\n\
             claimed: {claimed_sha}"
        );
        assert_eq!(disk_sha, verified_sha);
    }
}
