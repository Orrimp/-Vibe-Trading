//! bug-log `#151`, class-level: a writer that prints a return must print a benchmark.
//!
//! `#151` was not one body's omission. Of the seven report writers under
//! `crates/backtest/src/report/`, **not one** emitted a buy-and-hold control, so every lane
//! body stated a total return with nothing beside it to judge it against. The live case was
//! `top10-2023-fy-patchtst-overlay-realdata` at +41.81 %: good news or bad? The corpus could
//! not say.
//!
//! Fixing six writers leaves the class open — the next writer is copied from one of these
//! and inherits the table, not the standard. So this test does not list writers. It finds
//! every writer whose table renders a `Total return` row and requires the same table to
//! render the control and the delta.
//!
//! ## What this pins, and what it does not
//!
//! It pins that the cells are IN the template. It does not pin that the number is right —
//! that is `report::benchmark`'s own unit tests, which derive every expected value by hand
//! (equal weight across legs, first and last observed close, entry fee only, and the delta's
//! sign from both directions). Nor does it pin that a reader finds the comparison useful.
//! Stating the split is the point: a source-level scan is the half `#151` was missing and it
//! is not the whole thing.

use std::path::{Path, PathBuf};

fn report_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/report")
}

/// Measured on 2026-11-08: six writers render a `Total return` row — `momentum`, `pairs`,
/// `regime_dispatcher`, `sma`, `tcn_overlay`, and `yahoo` by delegating to `sma`. The floor
/// is a non-vacuity guard: a scan that silently finds nothing must not read as compliance,
/// which is the `#164` failure mode.
const WRITERS_WITH_A_RETURN_ROW_FLOOR: usize = 5;

#[test]
fn every_writer_that_prints_a_return_also_prints_a_buy_and_hold_control() {
    let dir = report_dir();
    let mut with_return: Vec<String> = Vec::new();
    let mut missing: Vec<String> = Vec::new();

    for entry in std::fs::read_dir(&dir).expect("report dir must be readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("writer must be readable");
        if !src.contains("| Total return") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("utf-8 filename")
            .to_string();
        with_return.push(name.clone());
        if !(src.contains("{bh_control}") && src.contains("{bh_delta}")) {
            missing.push(name);
        }
    }

    assert!(
        with_return.len() >= WRITERS_WITH_A_RETURN_ROW_FLOOR,
        "found only {} writer(s) rendering a `Total return` row under {} — expected at \
         least {}. The scan is in the wrong place or the row changed shape; an empty scan \
         must never read as compliance (bug-log #151, #164).",
        with_return.len(),
        dir.display(),
        WRITERS_WITH_A_RETURN_ROW_FLOOR
    );

    assert!(
        missing.is_empty(),
        "these report writers print a `Total return` row with NO buy-and-hold control \
         beside it: {missing:?}\n\n\
         A number with no comparator is not a weak result, it is not a result (bug-log \
         #151). Add both cells to the table:\n  \
         | Buy and hold (equal-wt) | {{bh_control}} |\n  \
         | vs buy and hold         | {{bh_delta}}   |\n\n\
         and fill them from `report::benchmark::bh_cells(..)`, whose input is a pure \
         function of the `bars` the run result already carries — no second backtest."
    );

    println!(
        "[benchmark] {} writer(s) print a return [floor {}] · all {} carry the control",
        with_return.len(),
        WRITERS_WITH_A_RETURN_ROW_FLOOR,
        with_return.len()
    );
}

/// The control must be fed real inputs, not a placeholder.
///
/// A writer could satisfy the test above by printing the two cells while passing an empty
/// slice, which renders "not computed" forever and reads in a diff exactly like the real
/// thing. So every writer carrying the cells must also CALL the computation.
#[test]
fn a_writer_that_prints_the_cells_also_computes_them() {
    let dir = report_dir();
    let mut checked = 0usize;

    for entry in std::fs::read_dir(&dir).expect("report dir must be readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let src = std::fs::read_to_string(&path).expect("readable");
        if !src.contains("{bh_control}") {
            continue;
        }
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("?");
        assert!(
            src.contains("equal_weight_buy_and_hold(") && src.contains("bh_cells("),
            "{name}: renders the buy-and-hold cells but does not call \
             `equal_weight_buy_and_hold` / `bh_cells` — the cells would be a constant."
        );
        checked += 1;
    }

    assert!(
        checked >= WRITERS_WITH_A_RETURN_ROW_FLOOR,
        "only {checked} writer(s) carried the cells — expected at least {}. A scan that \
         checks nothing passes trivially.",
        WRITERS_WITH_A_RETURN_ROW_FLOOR
    );
    println!("[benchmark] computation verified present in {checked} writer(s)");
}
