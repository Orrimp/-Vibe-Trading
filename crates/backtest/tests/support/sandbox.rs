//! Shared test helpers for the report-path pins of the `backtest` crate's CLI bins
//! (bug-log #210, #227; ADR-0104). Not a test target: cargo autodiscovers only
//! `tests/*.rs` and `tests/*/main.rs`. A test file includes it with
//! `#[path = "support/sandbox.rs"] mod sandbox;`.
//!
//! **Key convention.** Every map and set key is RELATIVE TO THE ROOT PASSED IN.
//! Compare like with like: an `evidence/`-rooted snapshot only with another
//! `evidence/`-rooted snapshot, and a sandbox-rooted file set only with paths
//! relative to the sandbox.

#![allow(dead_code)] // each including test file uses a subset

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::Mutex;

/// The workspace root: `CARGO_MANIFEST_DIR/../..`, fixed at compile time, never a
/// walk from the working directory.
pub fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("locate workspace root")
        .to_path_buf()
}

/// (is a dir, length, mtime) of one entry.
pub type Meta = (bool, u64, std::time::SystemTime);

/// Every entry under `root` (root itself included, as `""`), relative to it, with
/// its metadata. Any read error panics: an unreadable tree never compares equal by
/// being empty. A dir's mtime moves when an entry is added or removed, so a
/// create-then-remove inside the tree is visible too.
pub fn snapshot(root: &Path) -> BTreeMap<PathBuf, Meta> {
    fn meta(p: &Path) -> Meta {
        let m = std::fs::symlink_metadata(p).unwrap_or_else(|e| panic!("stat {p:?}: {e}"));
        let mtime = m.modified().unwrap_or_else(|e| panic!("mtime {p:?}: {e}"));
        (m.is_dir(), m.len(), mtime)
    }
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Meta>) {
        let rd = std::fs::read_dir(dir).unwrap_or_else(|e| panic!("list {dir:?}: {e}"));
        for e in rd {
            let p = e.unwrap_or_else(|e| panic!("entry in {dir:?}: {e}")).path();
            let m = meta(&p);
            out.insert(p.strip_prefix(base).expect("under base").to_path_buf(), m);
            if m.0 {
                walk(base, &p, out);
            }
        }
    }
    let mut out = BTreeMap::new();
    out.insert(PathBuf::new(), meta(root));
    walk(root, root, &mut out);
    out
}

/// The non-dir keys of [`snapshot`]`(root)`, relative to `root`. Built on
/// `snapshot` so one blind walker reaches every positive control that uses either.
pub fn files_under(root: &Path) -> BTreeSet<PathBuf> {
    snapshot(root)
        .into_iter()
        .filter(|(_, (is_dir, _, _))| !is_dir)
        .map(|(k, _)| k)
        .collect()
}

/// A tempdir a research bin can run in with its working directory set to it:
/// `config/strategies/*` copied from the workspace (`monte_carlo` and
/// `param_robustness_sweep` load `top10_momentum_h1.toml` cwd-relative), and ONE
/// empty `evidence/` with nothing under it, so a write into `evidence/…` has to
/// create its way in and an `evidence/` snapshot sees it.
///
/// Panics (never skips) if an ancestor of the tempdir holds a `Cargo.lock`: then
/// `run_yahoo_sma`'s workspace walk and the config walk could resolve out of the
/// sandbox.
pub fn research_sandbox(ws: &Path) -> tempfile::TempDir {
    let t = tempfile::tempdir().expect("tempdir");
    for a in t.path().ancestors() {
        assert!(
            !a.join("Cargo.lock").exists(),
            "precondition: no ancestor of the sandbox {} may hold a Cargo.lock, and {} does",
            t.path().display(),
            a.display()
        );
    }
    let src = ws.join("config").join("strategies");
    let dst = t.path().join("config").join("strategies");
    std::fs::create_dir_all(&dst).expect("mkdir sandbox config/strategies");
    let mut copied = 0usize;
    for e in std::fs::read_dir(&src).unwrap_or_else(|e| panic!("list {src:?}: {e}")) {
        let p = e.unwrap_or_else(|e| panic!("entry in {src:?}: {e}")).path();
        if p.is_file() {
            std::fs::copy(&p, dst.join(p.file_name().expect("file name")))
                .unwrap_or_else(|e| panic!("copy {p:?}: {e}"));
            copied += 1;
        }
    }
    assert!(
        dst.join("top10_momentum_h1.toml").is_file(),
        "sandbox config lacks top10_momentum_h1.toml ({copied} files copied from {src:?})"
    );
    std::fs::create_dir(t.path().join("evidence")).expect("mkdir sandbox evidence/");
    t
}

/// The path a bin printed on the ONE stdout line whose `trim_start()` starts with
/// `label`: the text after that line's first `:`, trimmed. Panics on zero or
/// several such lines.
pub fn printed_path(stdout: &str, label: &str) -> PathBuf {
    let lines: Vec<&str> = stdout
        .lines()
        .filter(|l| l.trim_start().starts_with(label))
        .collect();
    assert_eq!(
        lines.len(),
        1,
        "expected exactly one stdout line starting with {label:?}, got {lines:?}\nstdout:\n{stdout}"
    );
    let (_, rest) = lines[0]
        .split_once(':')
        .unwrap_or_else(|| panic!("no `:` in {:?}", lines[0]));
    PathBuf::from(rest.trim())
}

/// Whether git ignores `rel` (relative to the workspace), whether or not it exists:
/// `git -C <ws> check-ignore -q --no-index <rel with / separators>`. Exit 0 is true,
/// 1 is false; any other exit panics.
pub fn ignored(ws: &Path, rel: &Path) -> bool {
    let rel = rel
        .components()
        .map(|c| c.as_os_str().to_str().expect("utf-8 path"))
        .collect::<Vec<_>>()
        .join("/");
    let out = Command::new("git")
        .arg("-C")
        .arg(ws)
        .args(["check-ignore", "-q", "--no-index", &rel])
        .output()
        .expect("spawn git");
    match out.status.code() {
        Some(0) => true,
        Some(1) => false,
        other => panic!(
            "git check-ignore {rel}: exit {other:?}: {}",
            String::from_utf8_lossy(&out.stderr)
        ),
    }
}

static BIN_RUN: Mutex<()> = Mutex::new(());

/// Run `cmd` to completion while holding one lock per test binary, so the bins
/// these targets spawn (`monte_carlo` and the sweep saturate rayon) never overlap.
/// The guard covers `output()` only and recovers from poison, so no assertion runs
/// while it is held and one red test cannot turn its siblings red.
pub fn run_serialised(cmd: &mut Command) -> Output {
    let guard = BIN_RUN
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    let out = cmd.output();
    drop(guard);
    out.unwrap_or_else(|e| panic!("spawn {cmd:?}: {e}"))
}
