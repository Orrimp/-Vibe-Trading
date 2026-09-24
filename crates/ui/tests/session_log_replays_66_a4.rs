//! Story 6-11 AC3 — **the story's real gate**: a replay of bug-log `#66` A.4's shape.
//!
//! ## What #66 A.4 was
//!
//! `report::sma`'s frontmatter template emitted its `strategy:` sub-keys UNINDENTED, so
//! `compare::cache::parse_frontmatter` filed them top-level (`id`, not `strategy.id`),
//! so `scan_one_root`'s `let Some(..) = fm.get("strategy.id") else { continue }` threw
//! away EVERY engine-written report. The Compare screen rendered correctly and showed
//! an empty matrix. An empty matrix is also what a fresh checkout shows. Nothing in the
//! product could tell those two apart, and the defect survived weeks until a code
//! review found it by reading.
//!
//! ## What this file asserts
//!
//! AC3, verbatim: "a surface that renders but populates from nothing must show up in
//! the log as `opened Compare → 0 cells from N discovered reports`, i.e. the log
//! records *emptiness with a denominator*. A log that cannot make that distinction has
//! not met this AC."
//!
//! So the gate is a DISCRIMINATION test, not a presence test: the defect corpus and an
//! empty directory must produce DIFFERENT log lines. A log that says "0 cells" for both
//! is exactly the log that would not have caught #66 A.4, and
//! [`the_defect_is_distinguishable_from_an_empty_corpus`] fails on it.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::path::{Path, PathBuf};

use ui::session_log::{Event, JsonlSink, Rejected, ScanTally, SessionSink};

/// A report that parses, is identified as a scenario, and carries NO `strategy.id`.
///
/// **This is not #66 A.4's literal trigger, and saying so matters.** That trigger was an
/// unindented `strategy:` block, and it no longer reproduces: A.4's fix was a
/// tolerant-reader change to `parse_frontmatter`, which now accepts the unindented shape
/// (its own regression test covers that, and this file deliberately does not re-assert
/// it). What this corpus reconstructs is A.4's OUTCOME — a directory full of valid,
/// readable reports that the scanner admits none of, for one attributable reason — which
/// is the thing AC3 asks the log to be able to say. Re-breaking the parser to get a more
/// literal replay would test the parser, not the log.
fn defect_shaped_report(scenario: &str) -> String {
    format!(
        "---\n\
         scenario: {scenario}\n\
         generated: 2026-05-20T10:00:00Z\n\
         data_source: synthetic\n\
         ---\n\
         \n\
         # Backtest Report\n\
         \n\
         | Metric        | Value      |\n\
         |---------------|------------|\n\
         | Sharpe ratio  | **1.42**   |\n\
         | Total return  | **12.3 %** |\n\
         | Max drawdown  | **-5.6 %** |\n\
         | Trade count   | **42**     |\n"
    )
}

/// The same report with the strategy block the scanner needs — the only difference.
fn healthy_report(scenario: &str) -> String {
    defect_shaped_report(scenario).replace(
        "data_source: synthetic\n",
        "data_source: synthetic\nstrategy:\n  id: v0.sma\n  kind: sma_crossover\n",
    )
}

fn write_corpus(root: &Path, bodies: &[String]) {
    let reports = root.join("v0.sma").join("reports");
    std::fs::create_dir_all(&reports).unwrap();
    for (i, body) in bodies.iter().enumerate() {
        std::fs::write(
            reports.join(format!("backtest-2026052{i}-btc_sma.md")),
            body,
        )
        .unwrap();
    }
}

fn scan(root: &Path) -> ScanTally {
    let roots: Vec<PathBuf> = vec![root.to_path_buf()];
    ui::compare::cache::scan_report_roots(&roots).1
}

/// **The gate.** The defect corpus and an empty corpus must not produce the same record.
#[test]
fn the_defect_is_distinguishable_from_an_empty_corpus() {
    const N: usize = 7;

    let broken_dir = tempfile::tempdir().unwrap();
    write_corpus(
        broken_dir.path(),
        &(0..N)
            .map(|_| defect_shaped_report("btc_sma"))
            .collect::<Vec<_>>(),
    );
    let broken = scan(broken_dir.path());

    let empty_dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(empty_dir.path().join("v0.sma").join("reports")).unwrap();
    let empty = scan(empty_dir.path());

    // Both surfaces render an empty Compare matrix. That is the premise, not the bug.
    assert_eq!(broken.admitted, 0, "the defect shape must admit nothing");
    assert_eq!(empty.admitted, 0, "an empty corpus admits nothing either");

    // AC3: the RECORD of the two must differ.
    assert_ne!(
        broken, empty,
        "the log cannot tell a silently-rejected corpus from an empty one — this is \
         exactly the state in which #66 A.4 survived for weeks"
    );
    assert_eq!(
        broken.discovered, N,
        "the denominator must be the number of reports the scan actually considered"
    );
    assert_eq!(empty.discovered, 0, "an empty corpus has no denominator");
    assert_eq!(
        broken.rejected.get(&Rejected::MissingStrategyId),
        Some(&N),
        "every rejection must be ATTRIBUTED, not merely counted — a generic \"skipped\" \
         bucket would leave the operator exactly as blind as #66 A.4 did: got {:?}",
        broken.rejected
    );
    assert!(
        broken.found_everything_and_used_nothing(),
        "the shape predicate must fire on the defect corpus"
    );
    assert!(
        !empty.found_everything_and_used_nothing(),
        "and must NOT fire on an empty one, or it is not a predicate about anything"
    );
}

/// The positive control. With the writer's indentation fixed, the SAME corpus admits
/// every report — so the gate above is measuring the defect and not the harness.
#[test]
fn the_fixed_writer_shape_admits_everything() {
    const N: usize = 7;
    let dir = tempfile::tempdir().unwrap();
    write_corpus(
        dir.path(),
        &(0..N)
            .map(|_| healthy_report("btc_sma"))
            .collect::<Vec<_>>(),
    );
    let t = scan(dir.path());

    assert_eq!(t.discovered, N);
    assert_eq!(
        t.admitted, N,
        "the only difference from the failing corpus is the strategy block; if this \
         does not admit everything, the test corpus is wrong and the gate above proves \
         nothing. tally: {t:?}"
    );
    assert!(
        t.rejected.is_empty(),
        "nothing should be rejected: {:?}",
        t.rejected
    );
}

/// AC1/AC2 — the record reaches disk as plain, inspectable JSONL, and carries the
/// denominator with it. A tally that is correct in memory and lost on the way to the
/// file would meet none of this story's point.
#[test]
fn the_denominator_survives_the_round_trip_to_disk() {
    const N: usize = 7;
    let corpus = tempfile::tempdir().unwrap();
    write_corpus(
        corpus.path(),
        &(0..N)
            .map(|_| defect_shaped_report("btc_sma"))
            .collect::<Vec<_>>(),
    );
    let tally = scan(corpus.path());

    let logs = tempfile::tempdir().unwrap();
    let sink = JsonlSink::create(logs.path(), 1_758_700_000).unwrap();
    sink.record(&Event::ScreenOpened {
        screen: "Compare".to_owned(),
    });
    sink.record(&Event::Scanned {
        what: "compare_reports",
        tally,
    });

    let written = std::fs::read_to_string(sink.path()).unwrap();
    let lines: Vec<&str> = written.lines().collect();
    assert_eq!(lines.len(), 2, "one line per event: {written}");

    let opened: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert_eq!(opened["event"], "screen_opened");
    assert_eq!(opened["screen"], "Compare");

    let scanned: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert_eq!(scanned["event"], "scanned");
    assert_eq!(scanned["what"], "compare_reports");
    assert_eq!(scanned["tally"]["discovered"], N);
    assert_eq!(scanned["tally"]["admitted"], 0);
    assert_eq!(scanned["tally"]["rejected"]["missing_strategy_id"], N);

    // Readable without tooling is an AC, so assert on the raw text a `cat` would show.
    assert!(
        written.contains("\"discovered\":7") && written.contains("\"missing_strategy_id\":7"),
        "the file must be legible as-is: {written}"
    );
}

/// AC4 — off costs nothing, and "off" is the default everywhere but the real binaries.
#[test]
fn the_log_is_off_unless_armed() {
    let c = ui::test_support::charts_screen_cockpit();
    assert!(
        c.session_log.is_none(),
        "a fixture cockpit must not be recording"
    );
    // The recorder on an unarmed cockpit is a no-op rather than a panic or a write.
    c.log_session(&Event::ScreenOpened {
        screen: "Compare".to_owned(),
    });
}

/// AC5 — retention is bounded, and the newest survive.
#[test]
fn retention_keeps_the_newest_and_drops_the_rest() {
    let dir = tempfile::tempdir().unwrap();
    let total = ui::session_log::KEEP_SESSIONS + 5;
    for i in 0..total {
        std::fs::write(dir.path().join(format!("session-{i}.jsonl")), "{}\n").unwrap();
    }
    // A stray file that is not a session must survive untouched.
    std::fs::write(dir.path().join("notes.txt"), "keep me").unwrap();

    ui::session_log::prune(dir.path());

    let mut kept: Vec<u64> = std::fs::read_dir(dir.path())
        .unwrap()
        .flatten()
        .filter_map(|e| {
            e.path()
                .file_stem()
                .and_then(|s| s.to_str())
                .and_then(|s| s.strip_prefix("session-"))
                .and_then(|s| s.parse::<u64>().ok())
        })
        .collect();
    kept.sort_unstable();

    assert_eq!(kept.len(), ui::session_log::KEEP_SESSIONS);
    assert_eq!(
        kept[0] as usize, 5,
        "the five OLDEST must be the ones dropped — a numeric sort, not a lexical one \
         (session-10 is older than session-9 and must not be mistaken for newer)"
    );
    assert!(
        dir.path().join("notes.txt").exists(),
        "pruning must not touch files it does not own"
    );
}
