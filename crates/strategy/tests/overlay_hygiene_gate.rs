//! Hygiene gate: every `*_overlay.rs` in `crates/strategy/src/` MUST have a
//! matching `*_end_to_end.rs` integration test that contains at least ONE of
//! the recognised divergence-assertion patterns:
//!   - `baseline_equity`  (preferred future convention)
//!   - `quantity_scale`   (current convention in the reference test)
//!   - `diverges`         (prose marker)
//!   - `no-op signature`  (forensic-gate marker used by the reference test)
//!
//! AND the file must contain a numeric comparison literal indicating an actual
//! divergence assertion (`.abs() >=`, `.abs() <`, `>= 0.01`, etc.).
//!
//! Non-negotiable (CLAUDE.md § Non-negotiables):
//! > Every strategy overlay or sizing-modifier ships with a
//! > baseline-equity-divergence end-to-end test from day 1.  Unit tests on the
//! > math layer + anchored backtest reports are NOT sufficient to catch a no-op
//! > overlay where `scale` is computed but never applied.  The required gate is
//! > an e2e test that asserts the overlay's output equity diverges from the
//! > un-targeted baseline equity by ≥ 1 bp (or some testable epsilon) when the
//! > strategy decision variable is non-trivial.
//! > Pattern reference: `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`
//!
//! Cross-reference: `spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md`
//!
//! # Allowlist policy
//!
//! Overlays in `KNOWN_UNCOVERED` are treated as **warnings, not failures** —
//! their absence is printed to stderr but does not panic.  This keeps the gate
//! immediately green so it lands today; subsequent PRs can pick off allowlist
//! entries by writing the tests and removing the entry from `KNOWN_UNCOVERED`.
//!
//! CRITICAL: a NEW overlay (stem not in `KNOWN_UNCOVERED`) that lacks the
//! required test MUST panic — that is the whole point of this gate.

use std::fs;
use std::path::PathBuf;

// ── Allowlist ─────────────────────────────────────────────────────────────────

/// Overlay file stems (without the `.rs` extension) that are temporarily
/// exempt from the hard-fail.  They must still appear in the warning output.
/// Remove an entry only after writing the matching `*_end_to_end.rs`.
const KNOWN_UNCOVERED: &[&str] = &[
    // Killswitch has unit tests (#[cfg(test)] in the module) but no
    // end-to-end equity-divergence test.  Tracked: write
    // `vol_killswitch_overlay_end_to_end.rs` before the next overlay PR.
    "vol_killswitch_overlay",
];

// ── Divergence assertion substrings ──────────────────────────────────────────

/// At least one of these must appear in the test file body.
/// These are the recognised divergence-proof patterns across existing and
/// future end-to-end tests.
const DIVERGENCE_MARKERS: &[&str] = &[
    "baseline_equity", // preferred future convention
    "quantity_scale",  // current convention (vol_targeting_overlay_end_to_end.rs)
    "no-op signature", // forensic-gate marker used by the reference test
    "diverges",        // prose marker
    "equity_diverge",  // alternate naming
];

/// At least one of these numeric-comparison patterns must also appear,
/// confirming the file actually asserts a value (not just calls a fn).
const COMPARISON_MARKERS: &[&str] = &[
    ".abs() >=",
    ".abs() <",
    ">= 0.01",
    ">= 0.001",
    ">= 0.0001",
    ">= 1e-",
    "abs() > 0",
    "!= 1.0",
    "> 1.0",
    "< 1.0",
    "> 0.0",
    "assert_ne!",
];

// ── Gate helpers ──────────────────────────────────────────────────────────────

/// Returns `Ok(())` if the overlay with the given stem has a conforming
/// `*_end_to_end.rs` file, or `Err(reason)` otherwise.
///
/// Exposed at module scope so the self-test below can call it with synthetic paths.
fn check_overlay(stem: &str, tests_dir: &PathBuf) -> Result<(), String> {
    let expected = tests_dir.join(format!("{stem}_end_to_end.rs"));
    if !expected.exists() {
        return Err(format!(
            "missing end-to-end test file: {}",
            expected.display()
        ));
    }
    let body = fs::read_to_string(&expected)
        .map_err(|e| format!("cannot read {}: {e}", expected.display()))?;

    let has_divergence_marker = DIVERGENCE_MARKERS.iter().any(|m| body.contains(m));
    if !has_divergence_marker {
        return Err(format!(
            "{} does not contain any recognised divergence-proof marker \
             (expected one of: {})",
            expected.display(),
            DIVERGENCE_MARKERS.join(", ")
        ));
    }

    let has_numeric_comparison = COMPARISON_MARKERS.iter().any(|m| body.contains(m));
    if !has_numeric_comparison {
        return Err(format!(
            "{} contains a divergence marker but no recognisable numeric \
             comparison literal — the divergence assertion looks absent. \
             Expected one of: {}",
            expected.display(),
            COMPARISON_MARKERS.join(", ")
        ));
    }

    Ok(())
}

// ── Primary gate test ─────────────────────────────────────────────────────────

#[test]
fn every_overlay_has_end_to_end_divergence_test() {
    // Locate `crates/strategy/src/` relative to this test's manifest dir.
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let src_dir = manifest_dir.join("src");
    let tests_dir = manifest_dir.join("tests");

    assert!(
        src_dir.exists(),
        "strategy src dir missing at {} — misconfigured?",
        src_dir.display()
    );
    assert!(
        tests_dir.exists(),
        "strategy tests dir missing at {} — misconfigured?",
        tests_dir.display()
    );

    // Collect all `*_overlay.rs` stems (strip the `.rs` extension).
    let mut overlay_stems: Vec<String> = fs::read_dir(&src_dir)
        .expect("read_dir on strategy/src")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let name = entry.file_name();
            let name_str = name.to_string_lossy().to_string();
            if name_str.ends_with("_overlay.rs") {
                // stem = e.g. "vol_killswitch_overlay"
                Some(name_str.strip_suffix(".rs").unwrap().to_string())
            } else {
                None
            }
        })
        .collect();
    overlay_stems.sort(); // deterministic output order

    assert!(
        !overlay_stems.is_empty(),
        "no *_overlay.rs files found in {} — \
         update this test if the overlay pattern changed",
        src_dir.display()
    );

    let mut hard_failures: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();

    for stem in &overlay_stems {
        match check_overlay(stem, &tests_dir) {
            Ok(()) => {
                // Covered — great.
            }
            Err(reason) => {
                if KNOWN_UNCOVERED.contains(&stem.as_str()) {
                    warnings.push(format!("[ALLOWLIST-WARNING] {stem}.rs — {reason}"));
                } else {
                    hard_failures.push(format!(
                        "[HARD-FAIL] {stem}.rs\n\
                         Expected test file: {}/{stem}_end_to_end.rs\n\
                         Reason: {reason}",
                        tests_dir.display()
                    ));
                }
            }
        }
    }

    // Print allowlist warnings to stderr (visible in CI logs; does NOT fail).
    if !warnings.is_empty() {
        eprintln!();
        eprintln!("=== overlay_hygiene_gate ALLOWLIST WARNINGS ===");
        eprintln!("These overlays are in KNOWN_UNCOVERED and are warned, not failed.");
        eprintln!("Remove the entry from KNOWN_UNCOVERED once you write the test.");
        eprintln!();
        for w in &warnings {
            eprintln!("  {w}");
        }
        eprintln!();
        eprintln!("CLAUDE.md non-negotiable (verbatim):");
        eprintln!(
            "  \"Every strategy overlay or sizing-modifier ships with a \
             baseline-equity-divergence end-to-end test from day 1.  Per the \
             v3-volatility-forecaster-noop-fix 2026-05-22 precedent (see \
             spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md), unit \
             tests on the math layer + anchored backtest reports are NOT \
             sufficient to catch a no-op overlay where `scale` is computed but \
             never applied.  The required gate is an e2e test that asserts the \
             overlay's output equity diverges from the un-targeted baseline \
             equity by >= 1 bp (or some testable epsilon) when the strategy \
             decision variable is non-trivial.\""
        );
        eprintln!();
        eprintln!(
            "Cross-reference: \
             spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md"
        );
        eprintln!(
            "Pattern reference: \
             crates/strategy/tests/vol_targeting_overlay_end_to_end.rs"
        );
        eprintln!("=== end ALLOWLIST WARNINGS ===");
        eprintln!();
    }

    // Hard failures: new overlays without tests — panic loud.
    if !hard_failures.is_empty() {
        let mut msg = String::from(
            "\n\
             ============================================================\n\
             overlay_hygiene_gate HARD FAILURE — CI must not pass\n\
             ============================================================\n\
             A NEW *_overlay.rs file was found without a conforming\n\
             `*_end_to_end.rs` divergence test.\n\n\
             To fix, EITHER:\n\
               (a) Write `crates/strategy/tests/<stem>_end_to_end.rs`\n\
                   containing a divergence-proof marker (e.g. `quantity_scale`,\n\
                   `baseline_equity`, `no-op signature`) AND a numeric\n\
                   comparison (e.g. `.abs() >= 0.01`, `assert_ne!`).\n\
               (b) Add the stem to `KNOWN_UNCOVERED` in this file\n\
                   (allowlists should be temporary — schedule follow-up).\n\n\
             CLAUDE.md non-negotiable (verbatim):\n\
             \"Every strategy overlay or sizing-modifier ships with a \
             baseline-equity-divergence end-to-end test from day 1.  Per the \
             v3-volatility-forecaster-noop-fix 2026-05-22 precedent (see \
             spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md), unit \
             tests on the math layer + anchored backtest reports are NOT \
             sufficient to catch a no-op overlay where `scale` is computed but \
             never applied.\"\n\n\
             Cross-reference: \
             spec/dev-notes/v3-vol-overlay-noop-discovery-2026-05-22.md\n\n\
             Violations found:\n",
        );
        for f in &hard_failures {
            msg.push_str("  ");
            msg.push_str(f);
            msg.push('\n');
        }
        msg.push_str("\n============================================================\n");
        panic!("{msg}");
    }
}

// ── Self-test: gate function rejects a synthetic uncovered overlay ─────────────

#[test]
fn gate_function_rejects_synthetic_uncovered_overlay() {
    // Use a stem that will NEVER appear in KNOWN_UNCOVERED.
    // The matching file does not exist, so check_overlay must return Err.
    let fake_stem = "synthetic_test_overlay_zzz9999";

    // Any real tests_dir is fine — the file won't exist regardless.
    let tests_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests");

    let result = check_overlay(fake_stem, &tests_dir);
    assert!(
        result.is_err(),
        "check_overlay must return Err for a stem with no matching test file; \
         got Ok — the gate is broken"
    );

    let reason = result.unwrap_err();
    assert!(
        reason.contains("missing end-to-end test file")
            || reason.contains("does not contain")
            || reason.contains("cannot read"),
        "Err message should describe the missing test, got: {reason}"
    );
}
