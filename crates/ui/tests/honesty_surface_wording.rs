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
    ] {
        assert!(
            registry.contains(&key),
            "{key} is rendered but not in strings::all()"
        );
    }
}
