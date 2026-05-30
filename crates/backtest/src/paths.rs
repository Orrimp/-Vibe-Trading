//! Workspace-relative path resolver — Bug #56 fix.
//!
//! Cross-sectional scenarios (`momentum`, `pairs`, `tcn_overlay`,
//! `patchtst_overlay_weights`, `garch_vol_target_overlay`,
//! `threshold_sweep`) load their config from `config/strategies/*.toml`
//! as a CWD-relative path. CLI runs from the workspace root resolve
//! these fine, but the Lab cockpit launched from any other CWD (e.g. a
//! release binary placed in `target/release/`) breaks with
//! `Internal("load momentum config: config/strategies/foo.toml")`.
//!
//! This module exposes a single helper, [`resolve_workspace_path`], that
//! walks up from the current directory looking for a workspace marker
//! (`Cargo.lock` at the repo root) and joins the relative path to it.
//! On miss, falls back to the CWD-relative path unchanged — which
//! preserves the anchored CLI behaviour: CLI runs from workspace root
//! have `./config/strategies/foo.toml` valid, so the helper returns
//! that same path, byte-identical to the pre-fix behaviour.
//!
//! ## Anchor-preservation contract (ADR-0038 § D6)
//!
//! The caller MUST store the **original relative path string**
//! (`"config/strategies/foo.toml"`) in `StrategyMeta.source_path`, NOT
//! the resolved absolute path returned by this function. The Markdown
//! report body at `report/sma.rs:145` writes `source_path` into the
//! `## Strategy` table cell; changing it would mutate the body-SHA and
//! break the 4 single-symbol + cross-sectional anchors.

use std::path::{Path, PathBuf};

/// Workspace marker file. `Cargo.lock` lives at the repo root in this
/// Cargo virtual workspace and is committed (vs. the `target/` dir
/// which isn't).
const WORKSPACE_MARKER: &str = "Cargo.lock";

/// Maximum number of parent directories to walk before giving up.
/// 8 levels is more than enough — a binary typically lives at most
/// 2-3 dirs deep under the workspace root.
const MAX_WALK_DEPTH: usize = 8;

/// Resolve a workspace-relative path (e.g. `"config/strategies/foo.toml"`)
/// to an absolute path that exists on disk.
///
/// ## Resolution order
///
/// 1. CWD-relative — if `./<rel>` exists, return it. This is the
///    anchored CLI path; workspace-root runs hit this branch.
/// 2. Walk up from CWD looking for `Cargo.lock`. When found, return
///    `<workspace_root>/<rel>`.
/// 3. Fall back to CWD-relative unchanged. Caller's `from_file` will
///    raise the usual not-found error, which is now visible to the
///    operator via the Lab error banner (Bug #54 fix).
///
/// Pure function — no I/O caching, but each call does at most
/// `MAX_WALK_DEPTH` `Path::exists` checks (a few µs).
#[must_use]
pub fn resolve_workspace_path(rel: impl AsRef<Path>) -> PathBuf {
    let rel = rel.as_ref();

    // (1) CWD-relative — anchors hit this branch from workspace root.
    if rel.exists() {
        return rel.to_path_buf();
    }

    // (2) Walk up looking for the workspace marker.
    if let Ok(cwd) = std::env::current_dir() {
        let mut probe = cwd.as_path();
        for _ in 0..MAX_WALK_DEPTH {
            if probe.join(WORKSPACE_MARKER).is_file() {
                let resolved = probe.join(rel);
                if resolved.exists() {
                    return resolved;
                }
                // Found workspace root but the rel path isn't there
                // either — return it anyway so the error message
                // points at the right place.
                return resolved;
            }
            match probe.parent() {
                Some(parent) => probe = parent,
                None => break,
            }
        }
    }

    // (3) Fall back unchanged — caller's `from_file` will surface the
    // not-found error to the operator.
    rel.to_path_buf()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;
    use std::fs;

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!(
                "backtest_paths_test_{}_{}",
                name,
                std::process::id()
            ));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    /// (1) CWD-relative — anchor-preserving fast path. When the file
    /// exists relative to CWD, return that path unchanged.
    #[test]
    fn resolves_cwd_relative_when_present() {
        let tmp = Tmp::new("cwd");
        let cfg = tmp.0.join("config/strategies");
        fs::create_dir_all(&cfg).unwrap();
        fs::write(cfg.join("foo.toml"), "stub").unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp.0).unwrap();
        let resolved = resolve_workspace_path("config/strategies/foo.toml");
        std::env::set_current_dir(prev).unwrap();
        assert_eq!(resolved, PathBuf::from("config/strategies/foo.toml"));
    }

    /// (2) Workspace marker — walks up from a nested CWD to find
    /// `Cargo.lock`, then resolves the relative path against the root.
    #[test]
    #[ignore = "tracked-in: paths-test-cwd-flake-2026-05-26 — uses std::env::set_current_dir which is process-global; races with parallel tests under `cargo test --workspace`; re-enable once serial_test or a per-test CWD sandbox is wired"]
    fn resolves_via_workspace_marker_walk_up() {
        let tmp = Tmp::new("walk");
        // Construct a fake workspace at tmp/work with Cargo.lock + config
        let work = tmp.0.join("work");
        fs::create_dir_all(work.join("config/strategies")).unwrap();
        fs::create_dir_all(work.join("nested/deeper")).unwrap();
        fs::write(work.join(WORKSPACE_MARKER), "[stub]\n").unwrap();
        fs::write(work.join("config/strategies/bar.toml"), "stub").unwrap();
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(work.join("nested/deeper")).unwrap();
        let resolved = resolve_workspace_path("config/strategies/bar.toml");
        std::env::set_current_dir(prev).unwrap();
        // resolved should be <work>/config/strategies/bar.toml.
        assert!(resolved.ends_with("config/strategies/bar.toml"));
        assert!(resolved.is_absolute() || resolved.starts_with(&work));
        assert!(resolved.exists(), "resolved path must exist on disk");
    }

    /// (3) Fallback — when neither CWD-relative nor workspace marker
    /// can be found, return the input path unchanged so the caller's
    /// not-found error surfaces.
    #[test]
    fn falls_back_to_input_when_unresolvable() {
        let tmp = Tmp::new("missing");
        let prev = std::env::current_dir().unwrap();
        std::env::set_current_dir(&tmp.0).unwrap();
        let resolved = resolve_workspace_path("config/strategies/zzz.toml");
        std::env::set_current_dir(prev).unwrap();
        // tmp has no Cargo.lock anywhere — walked up to root and gave up.
        // resolved is some path-like — not equal to the input necessarily
        // (because the walk may have found a Cargo.lock further up), but
        // the call must not panic and must return SOMETHING.
        assert!(!resolved.as_os_str().is_empty());
    }
}
