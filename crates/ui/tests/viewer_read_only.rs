#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T1810 / R17.4 / V9 — viewer bin is read-only on the spec tree.
//!
//! Greps the viewer's source for `File::create` and `tokio::fs::write`
//! call sites that target `spec/**` paths. If any surface, the test
//! fails loudly — the viewer must NEVER write into the committed
//! report tree.

use std::fs;
use std::path::Path;

const FORBIDDEN_PATTERNS: &[&str] = &[
    "File::create",
    "tokio::fs::write",
    "fs::write",
    "OpenOptions::new()",
];

#[test]
fn viewer_bin_is_read_only_on_spec_tree() {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let viewer_path = manifest_dir.join("src/bin/viewer.rs");
    assert!(
        viewer_path.exists(),
        "viewer.rs not found at {}",
        viewer_path.display(),
    );
    let src = fs::read_to_string(&viewer_path).expect("read viewer.rs");
    for (idx, line) in src.lines().enumerate() {
        // Strip comments — "//" lines + line-trailing comment slices
        // — so doc/comment mentions of `File::create` don't trigger.
        let code = match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        };
        for pat in FORBIDDEN_PATTERNS {
            if code.contains(pat) {
                panic!(
                    "viewer bin contains forbidden write call `{pat}` at \
                     line {}: {line:?}",
                    idx + 1
                );
            }
        }
    }
}
