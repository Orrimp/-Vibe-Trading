//! T1809 / R8.1 / Q4 — defensive negative test.
//!
//! Reads every `.rs` file under `crates/strategy/src/` and asserts
//! none contain `reflection::retrieve_top_k` or `reflection::store::`
//! references.  Fails CI if a future PR wires the trader without a
//! follow-up brief — same defensive-static-grep pattern as
//! `crates/reports/tests/body_no_volatile_metadata.rs`.

use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

#[test]
fn t1809_no_strategy_crate_consumes_reflection_retrieval() {
    // Repo-relative: walk up from `crates/reflection/` to the
    // workspace root, then into `crates/strategy/src/`.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("walk to workspace root");
    let strategy_src = workspace_root.join("crates").join("strategy").join("src");
    assert!(
        strategy_src.exists(),
        "strategy crate missing at {strategy_src:?}"
    );

    let forbidden_substrings = [
        "reflection::retrieve_top_k",
        "reflection::store::",
        "reflection::ReflectionStore",
        "reflection::store::sqlite",
    ];

    let mut offenses: Vec<String> = Vec::new();
    for entry in WalkDir::new(&strategy_src)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
    {
        let p = entry.path();
        let src = match fs::read_to_string(p) {
            Ok(s) => s,
            Err(_) => continue,
        };
        for needle in &forbidden_substrings {
            if src.contains(needle) {
                offenses.push(format!("{p:?} contains forbidden substring `{needle}`"));
            }
        }
    }
    assert!(
        offenses.is_empty(),
        "Q4 / R8.1 violation — strategy crate consumes reflection retrieval:\n{offenses:#?}\n\
         Trader-side wiring is a follow-up brief named `reflection-memory-trader-wiring`. \
         Route to analyst before landing this PR."
    );
}
