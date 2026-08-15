//! Bug-log #90 — lock the forced-liquidation carve-out so it cannot widen silently.
//!
//! `short_exec::check_and_liquidate` force-covers at the raw mark with the taker
//! fee only and emits **no `Fill`**. Both short/long friction-parity gates
//! measure the *fill tape*, so this path is invisible to them **by
//! construction** — not by oversight, and not fixably by improving those gates.
//!
//! That makes the usual protection unavailable: if someone routes another exit
//! through this function, no parity gate will notice, because there is nothing
//! on the tape to notice. The only thing that can notice is a census.
//!
//! So this test asserts the *caller set* rather than any numeric property: the
//! two production files that are allowed to force-liquidate, and no others. A
//! third caller turns this RED and forces the decision to be made consciously —
//! which is exactly what #90 asks for ("document the carve-out and make sure it
//! can't silently widen").
//!
//! **If this goes RED:** do not simply add the new file to `ALLOWED`. First
//! decide whether that call site should be engine-routed instead (see #90's
//! three options) — and record the decision in the bug-log. Widening the
//! allow-list without that is precisely the silent widening this guards.

use std::fs;
use std::path::{Path, PathBuf};

/// Production files permitted to call `check_and_liquidate`.
///
/// - `short_exec.rs` — the definition itself (plus its own `#[cfg(test)]` module).
/// - `runtime.rs` — the agent forward loop (executes the operator's actual plan).
/// - `sma_composed_run.rs` — the ranking bake-off path.
///
/// Both non-definition entries call it **identically**, which is why #90 is
/// symmetric rather than a repeat of #80's asymmetry.
const ALLOWED: &[&str] = &[
    "crates/backtest/src/short_exec.rs",
    "crates/agent/src/runtime.rs",
    "crates/backtest/src/scenarios/sma_composed_run.rs",
];

fn workspace_root() -> PathBuf {
    // CARGO_MANIFEST_DIR = <root>/crates/backtest
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .expect("workspace root is two levels above crates/backtest")
        .to_path_buf()
}

/// Recursively collect every `.rs` file under `<root>/crates/*/src/`.
fn production_rs_files(root: &Path) -> Vec<PathBuf> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }

    let mut out = Vec::new();
    let crates_dir = root.join("crates");
    let Ok(crate_entries) = fs::read_dir(&crates_dir) else {
        panic!("cannot read {}", crates_dir.display());
    };
    for entry in crate_entries.flatten() {
        let src = entry.path().join("src");
        if src.is_dir() {
            walk(&src, &mut out);
        }
    }
    out
}

/// Strip `//`-style line comments so doc-comment mentions (this file added
/// several to `short_exec.rs`) are not counted as call sites.
fn strip_line_comments(text: &str) -> String {
    text.lines()
        .map(|line| match line.find("//") {
            Some(idx) => &line[..idx],
            None => line,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn forced_liquidation_carve_out_has_not_widened() {
    let root = workspace_root();
    let files = production_rs_files(&root);

    assert!(
        files.len() > 100,
        "sanity: expected to scan the whole workspace, found only {} files — \
         the walker is broken and this gate would pass vacuously",
        files.len()
    );

    let mut offenders: Vec<String> = Vec::new();

    for path in &files {
        let Ok(raw) = fs::read_to_string(path) else {
            continue;
        };
        let code = strip_line_comments(&raw);
        if !code.contains("check_and_liquidate(") {
            continue;
        }

        let rel = path
            .strip_prefix(&root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace('\\', "/");

        if !ALLOWED.contains(&rel.as_str()) {
            offenders.push(rel);
        }
    }

    assert!(
        offenders.is_empty(),
        "BUG-LOG #90 — the forced-liquidation carve-out has WIDENED.\n\n\
         New file(s) now call `short_exec::check_and_liquidate`:\n  {}\n\n\
         That function force-covers at the raw mark, pays no slippage, and emits \
         NO `Fill` — so BOTH friction-parity gates are blind to it by construction \
         and will stay green no matter how much friction escapes here.\n\n\
         Do NOT just add the file to ALLOWED. Decide first whether this call site \
         should be engine-routed instead (bug-log #90 lists three costed options), \
         then record the decision.",
        offenders.join("\n  ")
    );

    // Non-vacuity: the allow-list must actually describe reality. If a listed
    // file stops calling it, the entry is stale and the gate is guarding less
    // than it claims.
    for allowed in ALLOWED {
        let path = root.join(allowed);
        let raw = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("ALLOWED entry {allowed} is unreadable: {e}"));
        assert!(
            strip_line_comments(&raw).contains("check_and_liquidate("),
            "ALLOWED lists {allowed}, but it no longer calls `check_and_liquidate` — \
             remove the stale entry so this gate keeps describing reality"
        );
    }
}
