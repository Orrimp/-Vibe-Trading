//! Story 3-20 AC3/AC4 — the two claims a pixel gate cannot make.
//!
//! A render test proves that something painted. It cannot read the words. These two
//! ACs are about WHAT THE WORDS SAY, so they are asserted here, on the constants the
//! screen renders, and the pixel proofs that those constants reach the screen live in
//! `leaderboard_scorecard_render.rs` (the block is on screen, above the fold) and
//! `leaderboard_search_completeness_render.rs` (the search statement paints).
//!
//! Splitting it this way is the honest division: neither half is sufficient, and
//! claiming the pixel gate covers the wording would be the kind of proxy AD-10 exists
//! to refuse.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use ui::strings;

/// **AC3 — the scorecard must read as report-only.**
///
/// ADR-0075 E-1: the DSR / N_eff / MinBTL block does not gate the crown. A user must
/// not be able to read it as a filter that was applied. The caption is what says so,
/// and this fails if a future reword drops the disclaiming clause — which is the only
/// way this AC can regress, since the block itself is already gated at the pixels.
#[test]
fn the_scorecard_caption_still_disclaims_gating() {
    let c = strings::LEADERBOARD_SCORECARD_CAPTION;
    assert!(
        c.contains("never changes the result"),
        "the scorecard caption must still say the check does not gate the crown \
         (ADR-0075 E-1). It now reads: {c:?}"
    );
}

/// **AC4 — the standing qualifier is defined ONCE.**
///
/// The requirement is not "show a caveat"; it is that the screen and the record
/// cannot drift, which holds only while exactly one place in the repo spells it out.
/// This scans `crates/ui/src` for a second copy of the qualifier's distinctive
/// opening and fails if one appears — a copy-paste into a second screen is exactly
/// how the drift this AC forbids would start.
#[test]
fn the_standing_qualifier_is_defined_once() {
    const NEEDLE: &str = "Standing qualifier: the research evidence behind";
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut hits = Vec::new();
    let mut stack = vec![root];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).expect("readable src dir") {
            let path = entry.expect("dir entry").path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let text = std::fs::read_to_string(&path).expect("readable source");
                if text.contains(NEEDLE) {
                    hits.push(path);
                }
            }
        }
    }
    assert_eq!(
        hits.len(),
        1,
        "the standing qualifier must be spelled out in exactly one file, so the screen \
         cannot drift from the record. Found it in: {hits:?}"
    );
}

/// **AC4 — the qualifier states BOTH halves, and the part that keeps it honest.**
///
/// A caveat that only says "the evidence is contaminated" would overstate: bug-log
/// #67 verified that the advisor gate resamples candidate equity curves and never
/// re-executes fills, so the ranking does not rest on the contaminated lane. A caveat
/// that only says "direction preserved" would understate. The screen says all three.
#[test]
fn the_standing_qualifier_carries_direction_limit_and_scope() {
    let q = strings::LEADERBOARD_STANDING_QUALIFIER;
    for fragment in [
        "direction is preserved",
        "magnitudes are not final",
        "does not rest on it",
        "#67",
    ] {
        assert!(
            q.contains(fragment),
            "the standing qualifier must state {fragment:?}; it reads: {q:?}"
        );
    }
}

/// Every string the screens render is registered, so the wording is inspectable in
/// one place. A new honesty string that skips the registry is invisible to review.
#[test]
fn the_new_honesty_strings_are_registered() {
    let registry: Vec<&str> = strings::all().iter().map(|(k, _)| *k).collect();
    for key in [
        "LEADERBOARD_SEARCH_COMPLETE_FMT",
        "LEADERBOARD_SEARCH_INCOMPLETE_FMT",
        "LEADERBOARD_SEARCH_ARMS_FMT",
        "LEADERBOARD_STANDING_QUALIFIER",
        // story 4-13 / gap-analysis B7 — the cross-run check
        "LEADERBOARD_CROSS_RUN_WITHIN_CHANCE_FMT",
        "LEADERBOARD_CROSS_RUN_ABOVE_CHANCE_FMT",
        "LEADERBOARD_CROSS_RUN_INSUFFICIENT_NO_HISTORY",
        "LEADERBOARD_CROSS_RUN_INSUFFICIENT_TOO_FEW_FMT",
        "LEADERBOARD_CROSS_RUN_INSUFFICIENT_DAMAGED_FMT",
        "LEADERBOARD_CROSS_RUN_CAPTION",
    ] {
        assert!(
            registry.contains(&key),
            "{key} is rendered but not in strings::all()"
        );
    }
}

// ── Story 4-13 (gap-analysis B7) — the cross-run check's WORDING ──────────────
//
// Same division as above, for the same reason: `leaderboard_cross_run_annex_render`
// proves the line paints and that its four states paint differently. It cannot read
// the words, and two of this story's acceptance criteria are entirely about what the
// words say. Asserting them here on the constants is the honest split; claiming the
// pixel gate covered them would be the proxy AD-10 exists to refuse.

/// **AC2 — the caption states the limitation, in BOTH directions.**
///
/// What shipped is the static family-wise bar, not LORD alpha-investing with decaying
/// memory. The AC permits the cheap version provided the choice is stated, and the
/// statement is only honest if it names both errors: the bar is CONSERVATIVE about old
/// runs (it never forgets) and OPTIMISTIC about correlated ones (it cannot tell a
/// re-run of the same coin over an overlapping window from a fresh test). A caption
/// that named only the first would flatter the number; only the second would look like
/// a disclaimer with no content.
///
/// Asserted on meaning-bearing fragments rather than the whole sentence, so a reword
/// that keeps the content passes and one that drops half of it fails.
#[test]
fn the_cross_run_caption_names_both_directions_of_its_limitation() {
    let c = strings::LEADERBOARD_CROSS_RUN_CAPTION;
    for fragment in [
        // independence + recency assumption, in plain words
        "independent",
        "equally recent",
        // the two errors it causes
        "hard on old runs",
        "easy on repeats",
        // the concrete instance of the optimistic one
        "overlapping window is not a fresh test",
        // and what is NOT built
        "not built yet",
    ] {
        assert!(
            c.contains(fragment),
            "the cross-run caption must state {fragment:?}; it reads: {c:?}"
        );
    }
}

/// **AC5 — the damaged-ledger line names the damage AND which way it biases.**
///
/// Unreadable rows are the one insufficiency where something is WRONG rather than
/// merely absent: they make the counted sequence shorter than the real one, which
/// shrinks the chance-alone expectation, which makes the whole check read better than
/// the truth. An operator who is told only "insufficient" cannot know that. The line
/// therefore carries the damaged-row count as a placeholder AND says the direction of
/// the error in words — which is also what keeps the `WARN` tint from being the only
/// signal.
#[test]
fn the_damaged_ledger_line_names_the_rows_and_the_bias_direction() {
    let d = strings::LEADERBOARD_CROSS_RUN_INSUFFICIENT_DAMAGED_FMT;
    assert!(
        d.contains("{runs}") && d.contains("{bad}"),
        "the damaged-ledger line must carry BOTH counts — readable and damaged — or \
         the operator cannot tell '3 of 40 rows are broken' from '3 runs happened'. \
         It reads: {d:?}"
    );
    assert!(
        d.contains("damaged"),
        "the damaged-ledger line must say 'damaged' in words, so the WARN tint is \
         never the only signal. It reads: {d:?}"
    );
    assert!(
        d.contains("BETTER than the truth"),
        "the damaged-ledger line must state WHICH WAY the under-count biases the \
         check — optimistic is the one direction an honesty surface must not fail in \
         quietly. It reads: {d:?}"
    );
}

/// **AC5 — the three insufficiency reasons are three different sentences.**
///
/// The render gate proves they paint differently; this proves they are not three
/// renderings of one shrug. Each must state the N it is talking about (so "no history"
/// and "one run" cannot be confused) and no two may be the same text.
#[test]
fn the_three_insufficiency_reasons_say_different_things() {
    let reasons = [
        strings::LEADERBOARD_CROSS_RUN_INSUFFICIENT_NO_HISTORY,
        strings::LEADERBOARD_CROSS_RUN_INSUFFICIENT_TOO_FEW_FMT,
        strings::LEADERBOARD_CROSS_RUN_INSUFFICIENT_DAMAGED_FMT,
    ];
    for r in reasons {
        assert!(
            r.contains("insufficient") && r.contains("N="),
            "every insufficiency line must name itself insufficient and state its N \
             (AC5's literal wording); this one reads: {r:?}"
        );
    }
    for (i, a) in reasons.iter().enumerate() {
        for b in &reasons[i + 1..] {
            assert_ne!(
                a, b,
                "two insufficiency reasons share the same text — an operator cannot \
                 act on a difference the copy does not make"
            );
        }
    }
}

/// **The no-jargon rule, on the story most tempted to break it.**
///
/// The engine's module doc says "Šidák", "family-wise", "online FDR" and
/// "alpha-investing", and it should — that is where the literature reference belongs.
/// None of it may reach the screen: the reader of this line is someone deciding what
/// to do with €200, and a term they have to look up is a term that stops them reading.
/// The rendered copy says "chance alone would produce about N" instead.
#[test]
fn the_cross_run_copy_carries_no_jargon() {
    let rendered = [
        strings::LEADERBOARD_CROSS_RUN_WITHIN_CHANCE_FMT,
        strings::LEADERBOARD_CROSS_RUN_ABOVE_CHANCE_FMT,
        strings::LEADERBOARD_CROSS_RUN_INSUFFICIENT_NO_HISTORY,
        strings::LEADERBOARD_CROSS_RUN_INSUFFICIENT_TOO_FEW_FMT,
        strings::LEADERBOARD_CROSS_RUN_INSUFFICIENT_DAMAGED_FMT,
        strings::LEADERBOARD_CROSS_RUN_CAPTION,
    ];
    for term in [
        "Šidák",
        "Sidak",
        "family-wise",
        "familywise",
        "FDR",
        "false discovery",
        "alpha-investing",
        "alpha investing",
        "LORD",
        "Bonferroni",
        "p-value",
    ] {
        for line in rendered {
            assert!(
                !line.to_lowercase().contains(&term.to_lowercase()),
                "the rendered cross-run copy must not use the term {term:?} — the \
                 method's name belongs in the engine's doc comment, not on a screen \
                 someone reads to decide what to do. Offending line: {line:?}"
            );
        }
    }
}
