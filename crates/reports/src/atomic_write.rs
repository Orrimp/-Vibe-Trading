//! Atomic write helper (R12.2 / Q3).
//!
//! Writes via tempfile + fsync + `std::fs::rename` so an interrupted run
//! never publishes a half-written file at the canonical path.  The
//! tempfile name carries the writer's PID so concurrent runs from
//! different processes do not collide.

use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

/// Per-process counter appended to the tempfile name so concurrent
/// calls from multiple threads cannot race on the same path.  Defined
/// at module scope so `clippy::items_after_statements` is satisfied.
static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write `body` to `path` atomically.
///
/// Steps:
/// 1. Ensure the parent directory exists (`create_dir_all`).
/// 2. Open `<path>.tmp.<pid>` and write the body verbatim.
/// 3. `fsync` the tempfile.
/// 4. `std::fs::rename` the tempfile over `path`.
///
/// On any failure the canonical path is left untouched.  The tempfile
/// remains for forensic inspection so the operator can diagnose why the
/// rename failed.  Cross-filesystem renames will fail on macOS / Linux —
/// callers must pass a path under the same filesystem as the parent
/// directory.
///
/// # Errors
///
/// Returns the underlying [`std::io::Error`] for any of: parent-dir
/// creation, tempfile creation, write, fsync, or rename.
pub fn atomic_write(path: &Path, body: &str) -> std::io::Result<()> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(parent)?;

    let stem = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("report");
    // The .tmp.<pid> suffix prevents collision across processes (the
    // operator's documented run pattern — one report binary per cron
    // tick).  Within the same process we additionally append a
    // monotonic counter so concurrent calls from multiple threads do
    // not race on the same tempfile name.
    let n = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let tmp = parent.join(format!("{stem}.tmp.{}.{n}", std::process::id()));

    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(body.as_bytes())?;
        f.sync_all()?;
    }

    std::fs::rename(&tmp, path)?;
    Ok(())
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn t807_atomic_write_creates_file_with_body() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("out.md");
        atomic_write(&path, "hello world\n").unwrap();
        let body = std::fs::read_to_string(&path).unwrap();
        assert_eq!(body, "hello world\n");
    }

    #[test]
    fn t807_atomic_write_creates_parent_dir() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("nested/deep/out.md");
        atomic_write(&path, "x").unwrap();
        assert!(path.exists());
    }

    #[test]
    fn t807_atomic_write_overwrites_existing() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("out.md");
        std::fs::write(&path, "old").unwrap();
        atomic_write(&path, "new").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
    }

    #[test]
    fn t807_atomic_write_no_partial_file_at_canonical_path() {
        // Three concurrent renders to the same path; canonical path is
        // either absent or contains a complete file at every read.
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("out.md");
        let body_a = "AAAA".repeat(1024);
        let body_b = "BBBB".repeat(1024);
        let body_c = "CCCC".repeat(1024);

        let pa = path.clone();
        let pb = path.clone();
        let pc = path.clone();
        let ba = body_a.clone();
        let bb = body_b.clone();
        let bc = body_c.clone();

        let h1 = std::thread::spawn(move || atomic_write(&pa, &ba));
        let h2 = std::thread::spawn(move || atomic_write(&pb, &bb));
        let h3 = std::thread::spawn(move || atomic_write(&pc, &bc));

        h1.join().unwrap().unwrap();
        h2.join().unwrap().unwrap();
        h3.join().unwrap().unwrap();

        // After all three finish, the canonical path must contain one of
        // the three full bodies — never a partial.
        let final_body = std::fs::read_to_string(&path).unwrap();
        assert!(
            final_body == body_a || final_body == body_b || final_body == body_c,
            "canonical path contained a partial body"
        );
    }
}
