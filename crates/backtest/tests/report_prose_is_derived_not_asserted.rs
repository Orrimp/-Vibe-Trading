//! bug-log `#178`: a sentence in a report body that asserts a configuration value is a COPY
//! of that configuration, and copies drift.
//!
//! Four writers printed a risk line as a literal. `#95`'s gate wiring, committed the same
//! day, made `NO portfolio gross cap on this lane` false for four of the five lanes rendered
//! by `report/tcn_overlay.rs`; `report/regime_dispatcher.rs` asserted `portfolio cap=50%`
//! while that cap was declared-but-unenforced, false in the other direction. The bodies
//! denied a control three lines above a row counting that control's refusals.
//!
//! Nothing caught it. `#95`'s gate asks "does every lane that declares a cap enforce it?" and
//! passed; `#151`'s asks "does every writer that prints a return print a benchmark?" and
//! passed. Neither asks whether the PROSE agrees with the value. This test is that question,
//! in the one form that can be checked mechanically: a cap percentage in a writer's output
//! template must come from a value, not from a keystroke.

use std::path::{Path, PathBuf};

fn report_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("src/report")
}

/// Literals that predate this test and are, as measured on 2026-11-08, TRUE.
///
/// `report/sma.rs` prints `per-symbol exposure cap = 40%`. Deriving it means threading the
/// cap through ten `sma::write` call sites; until that happens the literal is a known debt
/// rather than a hidden one, which is the entire point of listing it here. Anything NOT on
/// this list fails.
///
/// Do not add to this list to make a new literal pass. Thread the value.
const GRANDFATHERED: &[(&str, &str)] = &[("sma.rs", "per-symbol exposure cap = 40%")];

/// Lines inside `///` doc comments, `//` comments and `assert!` bodies are not report output
/// — `report/mod.rs` documents the old literals verbatim in `risk_line`'s own doc comment,
/// and its tests assert on rendered percentages. Matching those would make this test fire on
/// its own explanation.
fn output_template_lines(src: &str) -> Vec<(usize, String)> {
    src.lines()
        .enumerate()
        .filter(|(_, l)| {
            let t = l.trim_start();
            !t.starts_with("//") && !t.starts_with("assert")
        })
        .map(|(i, l)| (i + 1, l.to_string()))
        .collect()
}

#[test]
fn no_writer_asserts_a_cap_percentage_as_a_literal() {
    let dir = report_dir();
    // `cap` followed by `=` or `= `, then digits, then `%` — the shape all four defects had.
    let re_like = |l: &str| {
        let lower = l.to_lowercase();
        lower.match_indices("cap").any(|(i, _)| {
            let rest: String = lower[i + 3..]
                .chars()
                .take_while(|c| *c == '=' || *c == ' ' || c.is_ascii_digit() || *c == '%')
                .collect();
            rest.contains('%') && rest.chars().any(|c| c.is_ascii_digit())
        })
    };

    let mut offenders: Vec<String> = Vec::new();
    let mut scanned = 0usize;

    for entry in std::fs::read_dir(&dir).expect("report dir must be readable") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|e| e.to_str()) != Some("rs") {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("utf-8 filename")
            .to_string();
        let src = std::fs::read_to_string(&path).expect("readable");
        scanned += 1;
        for (lineno, line) in output_template_lines(&src) {
            if !re_like(&line) {
                continue;
            }
            let trimmed = line.trim();
            if GRANDFATHERED
                .iter()
                .any(|(f, frag)| *f == name && trimmed.contains(frag))
            {
                continue;
            }
            offenders.push(format!("{name}:{lineno}  {trimmed}"));
        }
    }

    assert!(
        scanned >= 6,
        "scanned only {scanned} writer(s) — expected at least 6. An empty scan must not read \
         as compliance (bug-log #164)."
    );

    assert!(
        offenders.is_empty(),
        "these report writers assert a cap percentage as a LITERAL:\n  {}\n\n\
         A sentence asserting a configuration is a copy of that configuration, and copies \
         drift (bug-log #178, #175). Render it from the value instead — see \
         `report::risk_line(per_symbol, portfolio)`, fed from the `per_symbol_exposure_cap` / \
         `portfolio_exposure_cap` fields the run results carry.\n\n\
         Do NOT add the line to GRANDFATHERED to make this pass; that list is for literals \
         measured TRUE before this test existed.",
        offenders.join("\n  ")
    );

    println!(
        "[prose] {scanned} writer(s) scanned · 0 undeclared cap literals · {} grandfathered",
        GRANDFATHERED.len()
    );
}
