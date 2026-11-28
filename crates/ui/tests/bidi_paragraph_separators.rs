//! bug-log `#239` / ADR-0106 — a bidi paragraph separator between text of
//! different direction, inside one `\r`/`\n`-free line, no longer panics
//! cosmic-text, on every path the cockpit shapes text through.
//!
//! ## The bug
//!
//! `Buffer::set_text` cuts lines only at `\r` and `\n` (`LineIter`), but
//! unicode-bidi starts a new paragraph after every Bidi_Class=B char, and five of
//! them can sit inside a line: U+001C, U+001D, U+001E, U+0085 and U+2029.
//! Unpatched cosmic-text 0.15 asserts that every paragraph of a line has
//! paragraph 0's direction (`assert_eq!(line_rtl, rtl)`, `shape.rs:960`), so a
//! line such as `"\u{2029}\u{5d0}"` panicked, inside iced's process-wide font
//! lock: the lock was poisoned and the cockpit died. The fix is ADR-0106's one
//! hunk in `vendor/cosmic-text/src/shape.rs`: when the paragraphs disagree, the
//! line is re-resolved with paragraph 0's level (UAX #9 HL1), so it takes
//! paragraph 0's direction and the kept assert holds.
//!
//! ## What this file pins
//!
//! - **W1** the wiring: `[patch.crates-io]` points cosmic-text at the vendored
//!   copy, and the lock resolves one source-less cosmic-text and one unicode-bidi
//!   (the oracle's build). **W2** the vendored tree is the reviewed one
//!   (`VENDORED.sha256`, the hunk's marker, the kept assert).
//! - **B1** class B is exactly seven chars, by two independent scans, and five of
//!   them are in-line. **B2** no line panics, whatever its paragraph directions,
//!   through `set_text` and `set_rich_text`. **B3** a mixed line takes its first
//!   paragraph's direction (the retirement and policy pin, ADR-0106 D4).
//! - **C1-C4** the cockpit's own widgets, through `iced_test::Simulator` on
//!   iced's real font system: the bake-off budget field takes a typed U+2029
//!   (C1), its U+2028 twin (C2), an IME-committed U+0085 (C3) and drops a typed
//!   U+0085 (C3t); the Lab chip lays out a strategy id holding U+2029 (C4).
//! - **LR-a / LR-b** (macOS only, ADR-0057 D1; AD-10) the rendered PIXELS: the
//!   budget field and a Lab chip draw both halves of a separator value.
//!
//! ## Child processes
//!
//! Every test that shapes through iced's global font system runs in a CHILD
//! process, so a regression fails one test cleanly instead of poisoning this
//! binary (the `#231` lesson); the parent binary itself never lays text out
//! through iced. The parent re-executes `current_exe()` with
//! `<child> --exact --ignored --nocapture --test-threads 1` and the env var
//! `BIDI_SEP_CHILD=<child>`; each child's first statement panics unless that
//! variable names it, so a by-hand `--ignored` or `--include-ignored` run cannot
//! run the children together in one process. The child's stdout and stderr go
//! to files, never pipes; the parent polls it against a 60 s deadline and
//! asserts its exit status AND the line `BIDI-SEP-CHILD-DONE <child>`, which the
//! child prints last (a misspelt filter runs 0 tests and exits 0: `#187`). On a
//! failure the parent quotes the child's stderr (before the fix it held
//! `shape.rs:960`). No test here calls `Snapshot::matches_image` or
//! `matches_hash` (a missing baseline mints itself: `#187`).
//!
//! The pixel children write their frames into a directory of their own run,
//! `temp_dir()/bidi_sep/<nonce>/` (the parent's nonce), as
//! `bidi_sep_budget_*.png` and `bidi_sep_lab_*.png`, and write the nonce to
//! `bidi_sep_{budget,lab}.nonce` there only after their assertions pass, so
//! stale frames never read as current and two runs on one box never touch each
//! other's files. The parent prints the directory (`-- --nocapture`); old run
//! directories are left in the temp dir.
//!
//! Every separator in this file is spelled as a `\u{...}` escape: no source
//! line carries a raw Bidi_Class B, Zl or Zp character.

#![allow(clippy::expect_used, clippy::unwrap_used)]

#[path = "fixtures/bidi.rs"]
mod bidi;

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use bidi::{class_b_chars, shape_on_a_private_font_system};
use iced::advanced::graphics::text::cosmic_text as ct;
use iced::advanced::input_method;
use iced::keyboard::key::Named;
use iced::{Event, Rectangle, Settings};
use iced_test::Simulator;
use iced_test::selector::Candidate;
use sha2::{Digest, Sha256};
use unicode_bidi::{BidiClass, BidiInfo, bidi_class};

use trading_core::{StrategyId, StrategyLoaded, Timestamp};
use ui::state::{Cockpit, Message};
use ui::theme::ThemeMode;

// ─── The oracle: unicode-bidi's own paragraph resolution ──────────────────

/// The directions of `row`'s bidi paragraphs (`BidiInfo::new(row, None)`), the
/// expected-value oracle: derived from unicode-bidi, not measured off the shaper.
fn reference_paragraph_rtl(row: &str) -> Vec<bool> {
    BidiInfo::new(row, None)
        .paragraphs
        .iter()
        .map(|p| p.level.is_rtl())
        .collect()
}

/// `row`'s paragraphs do not all share one direction: the line upstream
/// cosmic-text 0.15 panicked on.
fn disagrees(row: &str) -> bool {
    reference_paragraph_rtl(row)
        .windows(2)
        .any(|w| w[0] != w[1])
}

/// The direction cosmic-text gives a line: paragraph 0's, or LTR when the line
/// has no paragraph at all (`""`; `shape.rs`, `rtl` is `false` then).
fn reference_line_rtl(line: &str) -> bool {
    reference_paragraph_rtl(line)
        .first()
        .copied()
        .unwrap_or(false)
}

/// Every `char` of Bidi_Class B by `unicode_bidi::bidi_class`, the derivation
/// independent of `class_b_chars()` (which counts `BidiParagraphs`).
fn bidi_class_b_chars() -> &'static [char] {
    static SCAN: std::sync::OnceLock<Vec<char>> = std::sync::OnceLock::new();
    SCAN.get_or_init(|| {
        (0..=u32::from(char::MAX))
            .filter_map(char::from_u32)
            .filter(|&c| bidi_class(c) == BidiClass::B)
            .collect()
    })
}

/// The class-B chars a cosmic-text line can hold: class B minus the two line
/// endings `LineIter` splits on (`line_ending.rs:55`).
fn in_line_separators() -> Vec<char> {
    class_b_chars()
        .iter()
        .copied()
        .filter(|c| !['\n', '\r'].contains(c))
        .collect()
}

/// The lines `Buffer::set_text` builds from `text`: `LineIter`'s, plus the empty
/// line it appends when the last line has an ending (`buffer.rs:676-716`).
fn set_text_lines(text: &str) -> Vec<&str> {
    let mut lines = Vec::new();
    let mut last = ct::LineEnding::default();
    for (range, ending) in ct::LineIter::new(text) {
        lines.push(&text[range]);
        last = ending;
    }
    if last != ct::LineEnding::None {
        lines.push("");
    }
    lines
}

// ─── W1, W2: the wiring and the vendored tree ─────────────────────────────

const WORKSPACE_MANIFEST: &str = include_str!("../../../Cargo.toml");
const WORKSPACE_LOCK: &str = include_str!("../../../Cargo.lock");

/// The non-comment entries of the workspace's `[patch.crates-io]` table.
fn patch_crates_io_entries(manifest: &str) -> Vec<String> {
    let mut in_patch = false;
    let mut entries = Vec::new();
    for line in manifest.lines() {
        let t = line.trim();
        if t.starts_with('[') {
            in_patch = t == "[patch.crates-io]";
            continue;
        }
        if in_patch && !t.is_empty() && !t.starts_with('#') {
            entries.push(t.to_owned());
        }
    }
    entries
}

/// One `[[package]]` of `Cargo.lock`.
#[derive(Debug)]
struct LockPackage {
    name: String,
    version: String,
    source: Option<String>,
}

fn lock_packages(lock: &str) -> Vec<LockPackage> {
    lock.split("[[package]]")
        .skip(1)
        .map(|block| {
            let field = |key: &str| {
                block.lines().find_map(|l| {
                    l.strip_prefix(key)
                        .and_then(|v| v.strip_prefix(" = \""))
                        .and_then(|v| v.strip_suffix('"'))
                        .map(str::to_owned)
                })
            };
            LockPackage {
                name: field("name").expect("a lock package has a name"),
                version: field("version").expect("a lock package has a version"),
                source: field("source"),
            }
        })
        .collect()
}

fn packages_named<'a>(packages: &'a [LockPackage], name: &str) -> Vec<&'a LockPackage> {
    packages.iter().filter(|p| p.name == name).collect()
}

/// W1 (the `#190` wiring check). The workspace patches cosmic-text with the
/// vendored copy, the lock resolves exactly that one cosmic-text, and exactly one
/// unicode-bidi (the oracle above is the build cosmic-text shapes with).
/// Controls: the same parsers see `iced_tiny_skia`, the other fork, as patched
/// and source-less, and unicode-bidi with a `registry+` source.
#[test]
fn the_workspace_patches_cosmic_text_with_the_vendored_copy() {
    let entries = patch_crates_io_entries(WORKSPACE_MANIFEST);
    assert!(
        entries
            .iter()
            .any(|e| e == r#"iced_tiny_skia = { path = "vendor/iced_tiny_skia" }"#),
        "control: the parser must see the iced_tiny_skia patch, got {entries:?}"
    );
    assert!(
        entries
            .iter()
            .any(|e| e == r#"cosmic-text = { path = "vendor/cosmic-text" }"#),
        "[patch.crates-io] must point cosmic-text at vendor/cosmic-text (ADR-0106 D3), \
         got {entries:?}"
    );

    let packages = lock_packages(WORKSPACE_LOCK);
    assert!(
        packages.len() > 100,
        "control: the lock parser found only {} packages",
        packages.len()
    );
    let skia = packages_named(&packages, "iced_tiny_skia");
    assert!(
        skia.len() == 1 && skia[0].source.is_none(),
        "control: the parser must see the iced_tiny_skia fork as one source-less package, \
         got {skia:?}"
    );
    let cosmic = packages_named(&packages, "cosmic-text");
    assert!(
        cosmic.len() == 1 && cosmic[0].version == "0.15.0" && cosmic[0].source.is_none(),
        "Cargo.lock must hold exactly one cosmic-text, 0.15.0, with no source line (the \
         vendored path package), got {cosmic:?}"
    );
    let bidi = packages_named(&packages, "unicode-bidi");
    assert!(
        bidi.len() == 1
            && bidi[0].version == "0.3.18"
            && bidi[0]
                .source
                .as_deref()
                .is_some_and(|s| s.starts_with("registry+")),
        "Cargo.lock must hold exactly one unicode-bidi, 0.3.18 from the registry, so the \
         oracle is the build cosmic-text shapes with, got {bidi:?}"
    );
}

/// The vendored tree.
fn vendored_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../vendor/cosmic-text")
}

/// The repo `.gitignore`'s OS/editor patterns (`.DS_Store`, `*.swp`, `*.swo`, `*~`).
fn is_skipped_file(name: &str) -> bool {
    name == ".DS_Store" || name.ends_with(".swp") || name.ends_with(".swo") || name.ends_with('~')
}

/// Every file under `dir`, as a `/`-joined path relative to `root`.
fn walk(root: &Path, dir: &Path, out: &mut Vec<String>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
        .map(|e| e.expect("a directory entry"))
        .collect();
    entries.sort_by_key(std::fs::DirEntry::file_name);
    for entry in entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        let path = entry.path();
        if entry.file_type().expect("a file type").is_dir() {
            if name != "target" {
                walk(root, &path, out);
            }
        } else if !is_skipped_file(&name) {
            let rel = path.strip_prefix(root).expect("under the root");
            let joined: Vec<String> = rel
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect();
            out.push(joined.join("/"));
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// The 64-hex hash after `prefix` on a line of `VENDORED.md`.
fn recorded_hash(vendored_md: &str, prefix: &str) -> String {
    let line = vendored_md
        .lines()
        .find(|l| l.starts_with(prefix))
        .unwrap_or_else(|| panic!("VENDORED.md has no line starting `{prefix}`"));
    let hex: String = line[prefix.len()..].chars().take(64).collect();
    assert!(
        hex.len() == 64 && hex.chars().all(|c| c.is_ascii_hexdigit()),
        "VENDORED.md `{prefix}` is not followed by a SHA-256: {line:?}"
    );
    hex
}

/// W2. The vendored tree is the reviewed one.
///
/// - **Walk.** Every file under `vendor/cosmic-text`, as a `/`-joined relative
///   path (the Windows leg yields `src\shape.rs`), skipping exactly the repo
///   `.gitignore`'s OS/editor patterns (`.DS_Store`, `*.swp`, `*.swo`, `*~`) and
///   any `target/` directory.
/// - **Hash set.** Each file's SHA-256, except `VENDORED.sha256` itself, must
///   equal `VENDORED.sha256` as an exact set: a missing, extra or changed file fails.
/// - **Two-sided.** `src/shape.rs` is NOT the upstream hash `VENDORED.md`
///   records (the hunk is present), and IS the patched hash it records.
/// - **Markers, scoped to `src/**`.** `src/shape.rs` holds `TRADING-PATCH` once
///   and `assert_eq!(line_rtl, rtl);` once (the invariant is kept, not deleted);
///   no other file under `src/` holds `TRADING-PATCH`. `VENDORED.md` quotes the
///   hunk, so it is outside the scan by design.
///
/// It walks instead of running `git ls-files`: a test that spawns git inherits
/// a hook's `GIT_*` environment. It proves the tree is the reviewed one; it does
/// not re-derive upstream (CI has no registry copy): that `diff -r` was made once
/// at vendoring time and is recorded in bug-log `#239`.
#[test]
fn the_vendored_cosmic_text_is_the_reviewed_tree() {
    let root = vendored_dir();
    let mut files = Vec::new();
    walk(&root, &root, &mut files);
    assert!(
        files.iter().any(|f| f == "src/shape.rs") && files.len() > 25,
        "control: the walk must find the vendored sources, found {files:?}"
    );

    let actual: BTreeMap<String, String> = files
        .iter()
        .filter(|f| f.as_str() != "VENDORED.sha256")
        .map(|f| {
            let bytes = std::fs::read(root.join(f)).expect("read a vendored file");
            (f.clone(), sha256_hex(&bytes))
        })
        .collect();
    let declared_text =
        std::fs::read_to_string(root.join("VENDORED.sha256")).expect("read VENDORED.sha256");
    let declared: BTreeMap<String, String> = declared_text
        .lines()
        .map(|l| {
            let (hash, path) = l
                .split_once("  ")
                .unwrap_or_else(|| panic!("VENDORED.sha256 line is not `<hash>  <path>`: {l:?}"));
            (path.to_owned(), hash.to_owned())
        })
        .collect();
    let mut drift = Vec::new();
    for (path, hash) in &actual {
        match declared.get(path) {
            None => drift.push(format!("extra file {path}")),
            Some(d) if d != hash => drift.push(format!("changed {path}: {hash}, declared {d}")),
            Some(_) => {}
        }
    }
    for path in declared.keys().filter(|p| !actual.contains_key(*p)) {
        drift.push(format!("missing file {path}"));
    }
    assert!(
        drift.is_empty(),
        "vendor/cosmic-text is not the reviewed tree (ADR-0106 D1/D4: never edited \
         beyond the hunk):\n{}",
        drift.join("\n")
    );

    let vendored_md = std::fs::read_to_string(root.join("VENDORED.md")).expect("read VENDORED.md");
    let upstream = recorded_hash(&vendored_md, "- Upstream `src/shape.rs` SHA-256: `");
    let patched = recorded_hash(&vendored_md, "- Patched `src/shape.rs` SHA-256: `");
    let shape_rs = std::fs::read_to_string(root.join("src/shape.rs")).expect("read shape.rs");
    let shape_hash = sha256_hex(shape_rs.as_bytes());
    assert_ne!(
        shape_hash, upstream,
        "src/shape.rs is upstream's: the ADR-0106 hunk is missing"
    );
    assert_eq!(
        shape_hash, patched,
        "src/shape.rs is neither upstream's nor the patched file VENDORED.md records"
    );
    assert_eq!(
        shape_rs.matches("TRADING-PATCH").count(),
        1,
        "src/shape.rs must carry the TRADING-PATCH marker exactly once"
    );
    assert_eq!(
        shape_rs.matches("assert_eq!(line_rtl, rtl);").count(),
        1,
        "src/shape.rs must keep upstream's paragraph-direction assert (a live invariant)"
    );
    let marked: Vec<&String> = files
        .iter()
        .filter(|f| f.starts_with("src/") && f.as_str() != "src/shape.rs")
        .filter(|f| {
            std::fs::read_to_string(root.join(f.as_str()))
                .expect("read a vendored source")
                .contains("TRADING-PATCH")
        })
        .collect();
    assert!(
        marked.is_empty(),
        "only src/shape.rs may carry a TRADING-PATCH marker, found it in {marked:?}"
    );
}

// ─── B1-B3: shaping on a private font system ──────────────────────────────

/// The seven Bidi_Class B chars (unicode-bidi 0.3.18, UCD `B`).
const CLASS_B: &[char] = &[
    '\n', '\r', '\u{1C}', '\u{1D}', '\u{1E}', '\u{85}', '\u{2029}',
];
/// The five a cosmic-text line can hold.
const IN_LINE: &[char] = &['\u{1C}', '\u{1D}', '\u{1E}', '\u{85}', '\u{2029}'];

/// B1. Class B is exactly seven chars and five of them are in-line: two
/// independent derivations (cosmic-text's `BidiParagraphs` and unicode-bidi's
/// `bidi_class`) agree with each other and with the literal. Unconditional, so
/// B2's candidate set is pinned inside this binary.
#[test]
fn class_b_is_seven_chars_and_five_are_in_line() {
    let by_paragraphs: BTreeSet<char> = class_b_chars().iter().copied().collect();
    let by_class: BTreeSet<char> = bidi_class_b_chars().iter().copied().collect();
    let literal: BTreeSet<char> = CLASS_B.iter().copied().collect();
    assert_eq!(
        by_paragraphs, by_class,
        "the BidiParagraphs scan (left) and the bidi_class scan (right) disagree"
    );
    assert_eq!(by_class, literal, "class B moved (unicode-bidi bump?)");
    let in_line: BTreeSet<char> = in_line_separators().into_iter().collect();
    let declared: BTreeSet<char> = IN_LINE.iter().copied().collect();
    assert_eq!(in_line, declared, "the in-line separators moved");
}

/// B2's rows for candidate `c`: each pair differs in one dimension (which side
/// is RTL, a strong char after the separator or only neutrals, an invisible RLM
/// or ALM, an isolate, a third paragraph).
fn b2_rows(c: char) -> Vec<String> {
    vec![
        format!("{c}\u{5d0}"),
        format!("{c}a"),
        format!("a{c}\u{5d0}"),
        format!("\u{5d0}{c}a"),
        format!("\u{5d0}{c}1"),
        format!("\u{5d0}{c} "),
        format!("a{c}\u{200f}b"),
        format!("a{c}\u{61c}b"),
        format!("\u{5d0}{c}\u{5d1}"),
        format!("a{c}b"),
        format!("a{c}\u{2067}\u{5d0}\u{2069}"),
        format!("a{c}\u{5d0}{c}1"),
    ]
}

/// B2, the core pin. Candidates: the five in-line separators, `\n`, `\r`, and
/// U+2028 (the near-miss: class WS). Every row shapes (`Ok`) through `set_text`
/// and `set_rich_text`. For `\n` and `\r`, `set_text` gives one `LayoutRun` per
/// line with that line's reference direction. Non-vacuity, derived from the
/// oracle: each of the five has a row whose paragraphs disagree (the class that
/// panicked) and a row whose paragraphs agree; every U+2028 row is one paragraph.
#[test]
fn no_line_panics_whatever_its_paragraph_directions() {
    let mut candidates: Vec<char> = in_line_separators();
    candidates.extend(['\n', '\r', '\u{2028}']);
    let mut wrong = Vec::new();
    for &c in &candidates {
        let rows = b2_rows(c);
        for row in &rows {
            for rich in [false, true] {
                if let Err(panic) = shape_on_a_private_font_system(row, rich) {
                    wrong.push(format!("{row:?} (rich = {rich}) panics: {panic}"));
                }
            }
            if c == '\n' || c == '\r' {
                let expected: Vec<bool> = set_text_lines(row)
                    .into_iter()
                    .map(reference_line_rtl)
                    .collect();
                let got = shape_on_a_private_font_system(row, false);
                if got.as_ref().ok() != Some(&expected) {
                    wrong.push(format!(
                        "{row:?}: set_text runs {got:?}, expected one per line {expected:?}"
                    ));
                }
            }
        }
        if IN_LINE.contains(&c) {
            let disagreeing = rows.iter().filter(|r| disagrees(r)).count();
            assert!(
                disagreeing > 0 && disagreeing < rows.len(),
                "non-vacuity for {c:?}: {disagreeing} of {} rows disagree; B2 needs both \
                 kinds",
                rows.len()
            );
        }
        if c == '\u{2028}' {
            for row in &rows {
                assert_eq!(
                    reference_paragraph_rtl(row).len(),
                    1,
                    "U+2028 is WS, so {row:?} must be one paragraph"
                );
            }
        }
    }
    assert!(
        wrong.is_empty(),
        "cosmic-text panicked or mis-split (bug-log #239, ADR-0106):\n{}",
        wrong.join("\n")
    );
}

/// B3, the retirement and policy pin (ADR-0106 D4). A mixed line takes its
/// FIRST paragraph's direction: `"a\u{2029}\u{5d0}"` is one LTR line and
/// `"\u{5d0}\u{2029}a"` one RTL line. The two rows differ only in which side
/// holds the RTL text and expect different outcomes (`#161`), each derived from
/// the oracle. Controls: two agreeing lines on the path upstream already took.
/// On the vendored hunk B3 follows from B2 plus the kept assert; it fails first
/// on retirement, when an upstream fix splits lines at class B instead (which
/// would give `[false, true]`).
#[test]
fn a_mixed_line_takes_its_first_paragraphs_direction() {
    let mixed = ["a\u{2029}\u{5d0}", "\u{5d0}\u{2029}a"];
    let expected: Vec<bool> = mixed
        .iter()
        .map(|r| reference_paragraph_rtl(r)[0])
        .collect();
    assert_ne!(
        expected[0], expected[1],
        "the two mixed rows must expect different directions"
    );
    for (row, want) in mixed.iter().zip(&expected) {
        assert!(disagrees(row), "{row:?} must be a mixed line");
        assert_eq!(
            shape_on_a_private_font_system(row, false),
            Ok(vec![*want]),
            "{row:?} must shape as ONE line with its first paragraph's direction"
        );
    }
    for row in ["\u{5d0}\u{2029}\u{5d1}", "a\u{2029}b"] {
        assert!(!disagrees(row), "control {row:?} must agree");
        assert_eq!(
            shape_on_a_private_font_system(row, false),
            Ok(vec![reference_paragraph_rtl(row)[0]]),
            "control {row:?}"
        );
    }
}

// ─── The child-process harness ────────────────────────────────────────────

const CHILD_ENV: &str = "BIDI_SEP_CHILD";
/// The pixel children's nonce (macOS only, like the pixel pairs).
#[cfg(target_os = "macos")]
const NONCE_ENV: &str = "BIDI_SEP_NONCE";
const DONE_MARKER: &str = "BIDI-SEP-CHILD-DONE";
const CHILD_DEADLINE: Duration = Duration::from_secs(60);

/// Run the `#[ignore]`d test `child` in its own process and require it to pass
/// and to print its done marker. Output goes to files, so a chatty child cannot
/// block on a full pipe while this polls.
fn run_child(child: &str, extra_env: &[(&str, &str)]) {
    let out_dir = tempfile::tempdir().expect("a tempdir for the child's output");
    let out_path = out_dir.path().join("stdout.txt");
    let err_path = out_dir.path().join("stderr.txt");
    let exe = std::env::current_exe().expect("current_exe");
    let mut cmd = Command::new(&exe);
    cmd.args([
        child,
        "--exact",
        "--ignored",
        "--nocapture",
        "--test-threads",
        "1",
    ])
    .env(CHILD_ENV, child)
    .stdin(Stdio::null())
    .stdout(File::create(&out_path).expect("create the stdout file"))
    .stderr(File::create(&err_path).expect("create the stderr file"));
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let mut process = cmd
        .spawn()
        .unwrap_or_else(|e| panic!("spawn {} {child}: {e}", exe.display()));
    let deadline = Instant::now() + CHILD_DEADLINE;
    let status = loop {
        if let Some(status) = process.try_wait().expect("try_wait the child") {
            break status;
        }
        if Instant::now() >= deadline {
            let _ = process.kill();
            let _ = process.wait();
            panic!(
                "child {child} did not finish within {CHILD_DEADLINE:?}; stderr:\n{}",
                std::fs::read_to_string(&err_path).unwrap_or_default()
            );
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let stdout = std::fs::read_to_string(&out_path).unwrap_or_default();
    let stderr = std::fs::read_to_string(&err_path).unwrap_or_default();
    assert!(
        status.success(),
        "child {child} failed ({status}); its stderr:\n{stderr}\nits stdout:\n{stdout}"
    );
    let marker = format!("{DONE_MARKER} {child}");
    assert!(
        stdout.lines().any(|l| l.trim() == marker),
        "child {child} exited 0 without printing `{marker}`: it did not run (0 tests \
         matched?) or did not finish; its stdout:\n{stdout}"
    );
    // The pixel children's measurements, visible under `-- --nocapture`. The first
    // one shares its line with libtest's `test <name> ... `.
    for line in stdout
        .lines()
        .filter_map(|l| l.find("bidi-sep ").map(|i| &l[i..]))
    {
        println!("{child}: {line}");
    }
}

/// A child's first statement: refuse to run unless the parent spawned it.
fn enter_child(name: &str, parent: &str) {
    let spawned_as = std::env::var(CHILD_ENV).ok();
    assert!(
        spawned_as.as_deref() == Some(name),
        "run me through {parent}: {name} shapes through iced's process-wide font lock and \
         must run alone in its own process ({CHILD_ENV} = {spawned_as:?})"
    );
}

/// A child's last statement. The marker opens a fresh line: under `--nocapture`
/// libtest has already printed `test <name> ... ` without a newline.
fn leave_child(name: &str) {
    println!("\n{DONE_MARKER} {name}");
}

// ─── C1-C4: the cockpit's widgets on iced's real font system ──────────────

/// A Simulator at the 1920×1080 slot, on the embedded UI face (the simulator's
/// own default is "Fira Sans", which is not loaded).
fn simulator(element: iced::Element<'_, Message>) -> Simulator<'_, Message> {
    Simulator::with_size(
        Settings {
            default_font: ui::theme::font::UI,
            fonts: vec![ui::theme::font::UI_REGULAR.into()],
            ..Settings::default()
        },
        (1920.0, 1080.0),
        element,
    )
}

/// The layout bounds of every `Text` whose content is exactly `content`.
fn text_bounds(sim: &mut Simulator<'_, Message>, content: &str) -> Vec<Rectangle> {
    let hits = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&hits);
    let wanted = content.to_owned();
    // The selector never matches, so `find` visits every candidate.
    let _ = sim.find(move |candidate: Candidate<'_>| -> Option<()> {
        if let Candidate::Text {
            bounds, content, ..
        } = candidate
            && content == wanted
        {
            sink.lock().expect("the hit list").push(bounds);
        }
        None
    });
    hits.lock().expect("the hit list").clone()
}

/// The budget field's distinctive start value: the `&str` selector matches it
/// and nothing else on the screen.
const BUDGET_START: &str = "417";

/// Build the Leaderboard with the budget field at [`BUDGET_START`], click it,
/// move to its end, run `edit`, and return the last budget the field published.
fn budget_field_after(edit: impl FnOnce(&mut Simulator<'_, Message>)) -> Option<String> {
    let mut cockpit = Cockpit::new();
    cockpit.leaderboard_screen_state.budget_input = BUDGET_START.to_owned();
    let mut sim = simulator(ui::screens::leaderboard::view(&cockpit, ThemeMode::Dark));
    sim.click(BUDGET_START)
        .expect("non-vacuity: the Leaderboard holds a text input whose value is 417");
    let _ = sim.tap_key(Named::End);
    edit(&mut sim);
    sim.into_messages()
        .filter_map(|m| match m {
            Message::BakeoffSetBudget(s) => Some(s),
            _ => None,
        })
        .last()
}

/// C1's typed text: U+2029 (Zp, which iced's Cc filter keeps), an invisible RLM
/// and `9`. After `417` the line holds an LTR paragraph and an RTL one.
const C1_TYPED: &str = "\u{2029}\u{200f}9";

/// C1, parent (both legs): a typed paragraph separator reaches the budget field's
/// value, and the field shapes it. Before the fix the child panicked at the RLM
/// keystroke (`shape.rs:960`): the widget shapes its own value inside `update`,
/// before the app sees the message.
#[test]
fn budget_field_takes_a_typed_paragraph_separator() {
    run_child("child_budget_field_typed_separator", &[]);
}

#[test]
#[ignore = "child of budget_field_takes_a_typed_paragraph_separator"]
fn child_budget_field_typed_separator() {
    enter_child(
        "child_budget_field_typed_separator",
        "budget_field_takes_a_typed_paragraph_separator",
    );
    let expected = format!("{BUDGET_START}{C1_TYPED}");
    assert!(
        disagrees(&expected),
        "C1's value {expected:?} must mix directions (#161)"
    );
    let last = budget_field_after(|sim| {
        let _ = sim.typewrite(C1_TYPED);
    });
    assert_eq!(
        last.as_deref(),
        Some(expected.as_str()),
        "the field must publish the raw value: iced keeps U+2029 (Zp) when typed"
    );
    leave_child("child_budget_field_typed_separator");
}

/// C2's typed text: C1's with U+2028 (Zl, class WS), which opens no paragraph.
const C2_TYPED: &str = "\u{2028}\u{200f}9";

/// C2, parent (both legs): C1's twin control. Green before and after the fix, so
/// C1's red came from the separator class, not from typing, the RLM or the
/// Simulator. Also the Windows measurement of the Simulator path.
#[test]
fn budget_field_takes_a_typed_line_separator() {
    run_child("child_budget_field_typed_line_separator", &[]);
}

#[test]
#[ignore = "child of budget_field_takes_a_typed_line_separator"]
fn child_budget_field_typed_line_separator() {
    enter_child(
        "child_budget_field_typed_line_separator",
        "budget_field_takes_a_typed_line_separator",
    );
    let expected = format!("{BUDGET_START}{C2_TYPED}");
    assert!(
        !disagrees(&expected),
        "C2's value {expected:?} must be one paragraph"
    );
    let last = budget_field_after(|sim| {
        let _ = sim.typewrite(C2_TYPED);
    });
    assert_eq!(last.as_deref(), Some(expected.as_str()), "C2's typed value");
    leave_child("child_budget_field_typed_line_separator");
}

/// C3's IME commit: U+0085 (Cc, class B) and a Hebrew letter.
const C3_COMMIT: &str = "\u{85}\u{5d0}";

/// C3, parent (both legs): an IME commit of U+0085 reaches the value, because
/// iced's IME `Commit` arm filters nothing (`text_input.rs:1290-1309`).
#[test]
fn budget_field_takes_an_ime_committed_nel() {
    run_child("child_budget_field_ime_committed_nel", &[]);
}

#[test]
#[ignore = "child of budget_field_takes_an_ime_committed_nel"]
fn child_budget_field_ime_committed_nel() {
    enter_child(
        "child_budget_field_ime_committed_nel",
        "budget_field_takes_an_ime_committed_nel",
    );
    let expected = format!("{BUDGET_START}{C3_COMMIT}");
    assert!(
        disagrees(&expected),
        "C3's value {expected:?} must mix directions"
    );
    let last = budget_field_after(|sim| {
        let _ = sim.simulate([Event::InputMethod(input_method::Event::Commit(
            C3_COMMIT.to_owned(),
        ))]);
    });
    assert_eq!(
        last.as_deref(),
        Some(expected.as_str()),
        "the IME commit must keep U+0085 (the Commit arm filters nothing)"
    );
    leave_child("child_budget_field_ime_committed_nel");
}

/// C3t, parent (both legs): the same two chars TYPED lose the U+0085, because
/// typed text drops `char::is_control` (`text_input.rs:1023`). With C3, a
/// two-sided pin of the filter's scope. Green before and after the fix.
#[test]
fn budget_field_drops_a_typed_nel() {
    run_child("child_budget_field_typed_nel", &[]);
}

#[test]
#[ignore = "child of budget_field_drops_a_typed_nel"]
fn child_budget_field_typed_nel() {
    enter_child(
        "child_budget_field_typed_nel",
        "budget_field_drops_a_typed_nel",
    );
    let expected = format!("{BUDGET_START}\u{5d0}");
    assert!(
        !disagrees(&expected),
        "C3t's value {expected:?} must be one paragraph"
    );
    let last = budget_field_after(|sim| {
        let _ = sim.typewrite(C3_COMMIT);
    });
    assert_eq!(
        last.as_deref(),
        Some(expected.as_str()),
        "typed U+0085 must be dropped (Cc)"
    );
    leave_child("child_budget_field_typed_nel");
}

/// A cockpit whose strategies panel holds one strategy, `id`, fed through the
/// production reducer as the in-process watcher would (`state.rs:4119`).
fn lab_cockpit_with_strategy(id: &str) -> Cockpit {
    let mut cockpit = Cockpit::new();
    ui::state::update(
        &mut cockpit,
        Message::StrategyLoaded(StrategyLoaded {
            id: StrategyId::new(id),
            hash: [7; 32],
            source_path: "config/strategies/v0.zz.toml".into(),
            ts: Timestamp::new(time::OffsetDateTime::UNIX_EPOCH),
        }),
    );
    cockpit
}

/// C4's strategy id: U+2029 between `v0.zz` (LTR) and an RLM before `9`.
const C4_ID: &str = "v0.zz\u{2029}\u{200f}9";

/// C4, parent (both legs): the Lab (the boot screen) lays out a chip whose id
/// holds a separator, as a `config/strategies/*.toml` id or file stem can.
#[test]
fn lab_chip_lays_out_a_strategy_id_holding_a_separator() {
    run_child("child_lab_chip_separator_id", &[]);
}

#[test]
#[ignore = "child of lab_chip_lays_out_a_strategy_id_holding_a_separator"]
fn child_lab_chip_separator_id() {
    enter_child(
        "child_lab_chip_separator_id",
        "lab_chip_lays_out_a_strategy_id_holding_a_separator",
    );
    assert!(disagrees(C4_ID), "C4's id {C4_ID:?} must mix directions");
    let cockpit = lab_cockpit_with_strategy(C4_ID);
    let mut sim = simulator(ui::screens::lab::view(&cockpit, ThemeMode::Dark));
    let hits = text_bounds(&mut sim, C4_ID);
    assert_eq!(
        hits.len(),
        1,
        "the Lab must lay out exactly one Text holding the raw id (the chip, \
         strategy_chip.rs:109), found {hits:?}"
    );
    leave_child("child_lab_chip_separator_id");
}

// ─── LR-a, LR-b: the rendered pixels (macOS, ADR-0057 D1; AD-10) ──────────

/// The frames each pixel child renders. `nine` is the bare `9`, for `d3`.
#[cfg(target_os = "macos")]
const FRAMES: [&str; 6] = ["bare", "ascii", "prefix", "fixed", "tail", "nine"];

#[cfg(target_os = "macos")]
mod pixels {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use iced::Rectangle;

    pub const W: u32 = 1920;
    pub const H: u32 = 1080;

    /// A frame rectangle in whole pixels: columns `[x0, x1)`, rows `[y0, y1)`.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct PxRect {
        pub x0: u32,
        pub y0: u32,
        pub x1: u32,
        pub y1: u32,
    }

    /// `bounds` rounded outward to whole pixels at scale 1.0, clamped to the frame.
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn whole_pixels(bounds: Rectangle) -> PxRect {
        let clamp = |v: f32, max: u32| v.clamp(0.0, max as f32) as u32;
        PxRect {
            x0: clamp(bounds.x.floor(), W),
            y0: clamp(bounds.y.floor(), H),
            x1: clamp((bounds.x + bounds.width).ceil(), W),
            y1: clamp((bounds.y + bounds.height).ceil(), H),
        }
    }

    /// Foreground pixels inside `r`: luma `(2r+3g+b)/6 > 80`, the classifier of
    /// `forward_plan_confidence_render.rs`, above every dark surface tier.
    pub fn fg_in(rgba: &[u8], r: PxRect) -> i64 {
        let mut hits = 0i64;
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                let i = ((y * W + x) * 4) as usize;
                let (red, green, blue) = (
                    i32::from(rgba[i]),
                    i32::from(rgba[i + 1]),
                    i32::from(rgba[i + 2]),
                );
                if (red * 2 + green * 3 + blue) / 6 > 80 {
                    hits += 1;
                }
            }
        }
        hits
    }

    /// The bytes of `rgba` inside `r`, row by row.
    pub fn bytes_in(rgba: &[u8], r: PxRect) -> Vec<u8> {
        let mut out = Vec::new();
        for y in r.y0..r.y1 {
            let start = ((y * W + r.x0) * 4) as usize;
            let end = ((y * W + r.x1) * 4) as usize;
            out.extend_from_slice(&rgba[start..end]);
        }
        out
    }

    /// Screenshot `program` at 1920×1080, scale 1.0, Dark.
    pub fn screenshot<P: iced_test::program::Program<Theme = iced::Theme> + 'static>(
        program: &P,
    ) -> Vec<u8> {
        ui::force_chart_utc_for_tests();
        let shot = iced_test::screenshot(program, &iced::Theme::Dark, (W, H), 1.0, Duration::ZERO);
        assert!(
            shot.size.width == W && shot.size.height == H,
            "HARNESS: frame is {}×{}, expected {W}×{H} at scale 1.0",
            shot.size.width,
            shot.size.height
        );
        shot.rgba.to_vec()
    }

    /// One parent run's output directory, `temp_dir()/bidi_sep/<nonce>/`.
    pub fn out_dir(nonce: &str) -> PathBuf {
        std::env::temp_dir().join("bidi_sep").join(nonce)
    }

    /// The PNG of `frame` in `group` (`budget` or `lab`).
    pub fn png_path(dir: &Path, group: &str, frame: &str) -> PathBuf {
        dir.join(format!("bidi_sep_{group}_{frame}.png"))
    }

    /// The nonce sidecar of `group`.
    pub fn nonce_path(dir: &Path, group: &str) -> PathBuf {
        dir.join(format!("bidi_sep_{group}.nonce"))
    }

    /// Create `dir` and delete `group`'s PNGs and sidecar in it, so a child that
    /// dies early leaves none.
    pub fn clear_outputs(dir: &Path, group: &str) {
        std::fs::create_dir_all(dir).unwrap_or_else(|e| panic!("create {}: {e}", dir.display()));
        for frame in super::FRAMES {
            let _ = std::fs::remove_file(png_path(dir, group, frame));
        }
        let _ = std::fs::remove_file(nonce_path(dir, group));
    }

    pub fn dump_png(rgba: &[u8], path: &Path) {
        image::RgbaImage::from_raw(W, H, rgba.to_vec())
            .expect("a 1920×1080 RGBA frame")
            .save(path)
            .unwrap_or_else(|e| panic!("write {}: {e}", path.display()));
    }

    /// A fresh nonce for one parent run.
    pub fn fresh_nonce() -> String {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        format!("{}-{nanos}", std::process::id())
    }

    /// The parent's check after a pixel child passed: every frame exists in this
    /// run's directory and the sidecar holds this run's nonce; then it prints the
    /// directory.
    pub fn assert_fresh_outputs(group: &str, nonce: &str) {
        let dir = out_dir(nonce);
        for frame in super::FRAMES {
            let path = png_path(&dir, group, frame);
            assert!(path.exists(), "the child left no {}", path.display());
        }
        println!("bidi-sep {group} frames: {}", dir.display());
        let sidecar = std::fs::read_to_string(nonce_path(&dir, group)).unwrap_or_default();
        assert_eq!(
            sidecar.trim(),
            nonce,
            "the {group} frames are not this run's (sidecar nonce mismatch)"
        );
    }

    /// The six band counts, in [`super::FRAMES`] order, and the five assertions
    /// LR-a and LR-b share (the jitter control and `disagrees` are the callers').
    pub fn assert_both_halves_draw(what: &str, fg: &[i64; 6]) {
        let [bare, ascii, prefix, fixed, tail, nine] = *fg;
        println!(
            "bidi-sep {what}: fg bare {bare}, ascii {ascii}, prefix {prefix}, fixed {fixed}, \
             tail {tail}, nine {nine}"
        );
        assert!(
            bare > 0,
            "{what}: the band holds no foreground in the bare frame"
        );
        let d = ascii - bare;
        assert!(
            d > 0,
            "{what}: the classifier must see one `9` (ascii {ascii} vs bare {bare})"
        );
        assert!(
            2 * (fixed - prefix) >= d,
            "{what}: the post-separator `9` must draw: fixed {fixed} - prefix {prefix} < d/2 \
             = {d}/2"
        );
        let d3 = ascii - nine;
        assert!(
            d3 > 0,
            "{what}: the pre-separator text must draw ink (ascii {ascii} vs nine {nine})"
        );
        assert!(
            2 * (fixed - tail) >= d3,
            "{what}: the pre-separator text must draw: fixed {fixed} - tail {tail} < d3/2 = \
             {d3}/2"
        );
    }
}

/// LR-a's values, in [`FRAMES`] order. `str::trim` strips the White_Space
/// U+2029 but not U+200F, so neither `prefix` nor `fixed` parses as a budget:
/// they draw the same hint and context line.
#[cfg(target_os = "macos")]
const BUDGET_VALUES: [&str; 6] = [
    "417",
    "4179",
    "417\u{2029}\u{200f}",
    "417\u{2029}\u{200f}9",
    "\u{2029}\u{200f}9",
    "9",
];

/// The budget field's bounds with `value` in it, from a Simulator over the SAME
/// element the screenshot renders (`LeaderboardScreenApp::view` after
/// `BakeoffSetBudget(value)`), on the same embedded face.
#[cfg(target_os = "macos")]
fn budget_field_rect(value: &str) -> pixels::PxRect {
    let mut app = ui::test_support::LeaderboardScreenApp::default();
    let _ = app.update(Message::BakeoffSetBudget(value.to_owned()));
    let mut sim = simulator(app.view());
    let target = sim
        .find(value)
        .unwrap_or_else(|e| panic!("the budget field holding {value:?}: {e:?}"));
    pixels::whole_pixels(target.bounds())
}

#[cfg(target_os = "macos")]
fn render_budget(value: &str) -> Vec<u8> {
    let mut cockpit = Cockpit::new();
    ui::state::update(&mut cockpit, Message::BakeoffSetBudget(value.to_owned()));
    pixels::screenshot(&ui::test_support::leaderboard_screen_program(cockpit))
}

/// LR-a, parent (macOS): the budget field draws a separator value, both halves.
#[cfg(target_os = "macos")]
#[test]
fn budget_field_renders_a_separator_value() {
    let nonce = pixels::fresh_nonce();
    run_child(
        "child_budget_field_renders_a_separator_value",
        &[(NONCE_ENV, &nonce)],
    );
    pixels::assert_fresh_outputs("budget", &nonce);
}

/// LR-a, child. Band `R` = the budget field's bounds, derived at run time and
/// identical for all six values (the layout-stability assertion); the hint and
/// context line the value also drives lie outside it. Inside `R`:
/// 1. `ascii - bare > 0` (the classifier sees one `9`; `bare` is the negative
///    control);
/// 2. `fixed - prefix >= d/2`: the post-separator `9` draws (the two frames
///    differ only by it, and the separator's own glyph is in both);
/// 3. `fixed - tail >= d3/2`, `d3 = ascii - nine`: the pre-separator `417` draws;
/// 4. `bare > 0`; 5. two independent `fixed` renders are byte-identical in `R`;
/// 6. `prefix`, `fixed` and `tail` mix directions.
#[cfg(target_os = "macos")]
#[test]
#[ignore = "child of budget_field_renders_a_separator_value"]
fn child_budget_field_renders_a_separator_value() {
    enter_child(
        "child_budget_field_renders_a_separator_value",
        "budget_field_renders_a_separator_value",
    );
    let nonce = std::env::var(NONCE_ENV).expect("the parent passes a nonce");
    let dir = pixels::out_dir(&nonce);
    pixels::clear_outputs(&dir, "budget");
    for value in &BUDGET_VALUES[2..5] {
        assert!(disagrees(value), "{value:?} must mix directions");
    }
    let mut rects = Vec::new();
    let mut frames = Vec::new();
    for (frame, value) in FRAMES.iter().zip(BUDGET_VALUES) {
        rects.push(budget_field_rect(value));
        let rgba = render_budget(value);
        pixels::dump_png(&rgba, &pixels::png_path(&dir, "budget", frame));
        frames.push(rgba);
    }
    assert!(
        rects.iter().all(|r| *r == rects[0]),
        "the budget field must not move with its value: {rects:?}"
    );
    let band = rects[0];
    println!("bidi-sep budget: band {band:?}");
    let fg: [i64; 6] = std::array::from_fn(|i| pixels::fg_in(&frames[i], band));
    pixels::assert_both_halves_draw("budget", &fg);
    let again = render_budget(BUDGET_VALUES[3]);
    assert_eq!(
        pixels::bytes_in(&again, band),
        pixels::bytes_in(&frames[3], band),
        "jitter control: two independent `fixed` renders must be byte-identical in the band"
    );
    std::fs::write(pixels::nonce_path(&dir, "budget"), &nonce).expect("write the nonce sidecar");
    leave_child("child_budget_field_renders_a_separator_value");
}

/// LR-b's ids, in [`FRAMES`] order (`fixed` is C4's id).
#[cfg(target_os = "macos")]
const LAB_IDS: [&str; 6] = [
    "v0.zz",
    "v0.zz9",
    "v0.zz\u{2029}\u{200f}",
    C4_ID,
    "\u{2029}\u{200f}9",
    "9",
];

/// The Lab chip label's bounds for `id`, from a Simulator over the element the
/// screenshot renders (`shell::view`, which `program_from_cockpit`'s view calls).
#[cfg(target_os = "macos")]
fn lab_chip_rect(cockpit: &Cockpit, id: &str) -> pixels::PxRect {
    let mut sim = simulator(ui::shell::view(cockpit, ThemeMode::Dark));
    let hits = text_bounds(&mut sim, id);
    assert_eq!(
        hits.len(),
        1,
        "the Lab must draw exactly one Text holding {id:?} (the chip), found {hits:?}"
    );
    pixels::whole_pixels(hits[0])
}

/// LR-b, parent (macOS): a Lab chip draws a separator id, both halves.
#[cfg(target_os = "macos")]
#[test]
fn lab_chip_renders_a_separator_id() {
    let nonce = pixels::fresh_nonce();
    run_child(
        "child_lab_chip_renders_a_separator_id",
        &[(NONCE_ENV, &nonce)],
    );
    pixels::assert_fresh_outputs("lab", &nonce);
}

/// LR-b, child: LR-a's scheme on the Lab chip. Each frame's band is that
/// frame's own chip `Text` bounds, so it counts only its own id's ink, never the
/// chip's border, badge or swatch, nor another chip; the assertions mirror
/// LR-a's 1-6 (the bands differ with the id's width by design).
#[cfg(target_os = "macos")]
#[test]
#[ignore = "child of lab_chip_renders_a_separator_id"]
fn child_lab_chip_renders_a_separator_id() {
    enter_child(
        "child_lab_chip_renders_a_separator_id",
        "lab_chip_renders_a_separator_id",
    );
    let nonce = std::env::var(NONCE_ENV).expect("the parent passes a nonce");
    let dir = pixels::out_dir(&nonce);
    pixels::clear_outputs(&dir, "lab");
    for id in &LAB_IDS[2..5] {
        assert!(disagrees(id), "{id:?} must mix directions");
    }
    let mut bands = Vec::new();
    let mut frames = Vec::new();
    for (frame, id) in FRAMES.iter().zip(LAB_IDS) {
        let cockpit = lab_cockpit_with_strategy(id);
        bands.push(lab_chip_rect(&cockpit, id));
        let rgba = pixels::screenshot(&ui::test_support::program_from_cockpit(cockpit));
        pixels::dump_png(&rgba, &pixels::png_path(&dir, "lab", frame));
        frames.push(rgba);
    }
    println!("bidi-sep lab: bands {bands:?}");
    let fg: [i64; 6] = std::array::from_fn(|i| pixels::fg_in(&frames[i], bands[i]));
    pixels::assert_both_halves_draw("lab", &fg);
    let again = pixels::screenshot(&ui::test_support::program_from_cockpit(
        lab_cockpit_with_strategy(LAB_IDS[3]),
    ));
    assert_eq!(
        pixels::bytes_in(&again, bands[3]),
        pixels::bytes_in(&frames[3], bands[3]),
        "jitter control: two independent `fixed` renders must be byte-identical in its band"
    );
    std::fs::write(pixels::nonce_path(&dir, "lab"), &nonce).expect("write the nonce sidecar");
    leave_child("child_lab_chip_renders_a_separator_id");
}
