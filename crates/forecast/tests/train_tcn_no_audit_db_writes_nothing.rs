//! T-D-N10 non-regression: when `--audit-db` is omitted, no SQLite file is
//! created at any path.
//!
//! ADR-0034 § D4 — the default (no `--audit-db`) must open zero SQLite handles.
//! This test verifies the invariant by running a `--dry-run` without
//! `--audit-db` and asserting that no `.db` file exists in the tempdir.

#[cfg(feature = "candle")]
mod tests {
    use std::process::Command;

    use tempfile::tempdir;

    fn cargo() -> String {
        std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned())
    }

    fn workspace_root() -> std::path::PathBuf {
        let manifest_dir =
            std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR must be set by cargo");
        let crate_dir = std::path::PathBuf::from(manifest_dir);
        crate_dir
            .parent()
            .expect("crates/")
            .parent()
            .expect("workspace root")
            .to_path_buf()
    }

    /// When `--audit-db` is omitted, `train_tcn` must NOT create any SQLite
    /// database file anywhere in the output directory.
    #[test]
    fn no_audit_db_flag_creates_no_sqlite_file() {
        let dir = tempdir().expect("tempdir");
        let out_dir = dir.path().join("checkpoints");
        // Place a sentinel that would be at the default db path if one were
        // created — we assert its absence after the run.
        let sentinel_db = dir.path().join("audit.db");

        let root = workspace_root();
        let config = root.join("crates/forecast/train_tcn.toml");
        let cargo = cargo();
        let out = Command::new(&cargo)
            .current_dir(&root)
            .args([
                "run",
                "-p",
                "forecast",
                "--bin",
                "train_tcn",
                "--features",
                "candle",
                "--quiet",
                "--",
                "--config",
                config.to_str().unwrap(),
                "--dry-run",
                "--output-dir",
                out_dir.to_str().unwrap(),
                // Deliberately no --audit-db flag.
            ])
            .output()
            .expect("failed to spawn train_tcn");

        assert!(
            out.status.success(),
            "train_tcn --dry-run without --audit-db should succeed:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );

        // Assert: the sentinel path was NOT created.
        assert!(
            !sentinel_db.exists(),
            "audit.db must NOT exist when --audit-db is omitted"
        );

        // Also: no .db files anywhere in the tempdir.
        let mut db_files = Vec::new();
        for entry in walkdir::WalkDir::new(dir.path()) {
            let entry = entry.expect("walkdir entry");
            if entry.file_name().to_string_lossy().ends_with(".db") {
                db_files.push(entry.path().to_path_buf());
            }
        }
        assert!(
            db_files.is_empty(),
            "no .db files expected in tempdir without --audit-db, found: {db_files:?}"
        );
    }
}
