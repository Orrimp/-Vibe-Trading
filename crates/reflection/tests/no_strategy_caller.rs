//! T1809 / R8.1 / Q4 — defensive negative test + T1810 positive assertion.
//!
//! T1809: Reads every `.rs` file under `crates/strategy/src/` and asserts
//! none contain `reflection::retrieve_top_k` or `reflection::store::`
//! references.  Fails CI if a future PR wires the trader without a
//! follow-up brief — same defensive-static-grep pattern as
//! `crates/reports/tests/body_no_volatile_metadata.rs`.
//!
//! T1810: Reads every `.rs` file under `crates/trader/src/` and asserts
//! AT LEAST ONE contains `reflection::retrieve_top_k`. Prevents accidental
//! deletion of the consumer logic during a future refactor (ADR-0041 § D5).

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

/// T1810 / ADR-0041 § D5 — positive-assertion sibling test.
///
/// Walks `crates/trader/src/` and asserts that at least one `.rs` file
/// contains `reflection::retrieve_top_k`. This guards against accidental
/// deletion of the reflection-consumer logic during a future refactor.
///
/// If memory retrieval is genuinely no longer needed, write a superseding ADR
/// removing both t1810 and the consumer.
#[test]
fn t1810_trader_crate_owns_reflection_retrieval() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .expect("walk to workspace root");
    let trader_src = workspace_root.join("crates").join("trader").join("src");
    assert!(trader_src.exists(), "trader crate missing at {trader_src:?}");

    let required_substring = "reflection::retrieve_top_k";
    let mut found = false;
    for entry in WalkDir::new(&trader_src)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("rs"))
    {
        let src = match fs::read_to_string(entry.path()) {
            Ok(s) => s,
            Err(_) => continue,
        };
        if src.contains(required_substring) {
            found = true;
            break;
        }
    }
    assert!(
        found,
        "R5.3 / ADR-0041 § D5 — trader crate must own reflection retrieval; \
         expected at least one .rs file under {trader_src:?} to contain \
         `{required_substring}`. If memory retrieval is genuinely no longer \
         needed, write a superseding ADR removing both t1810 and the consumer."
    );
}
