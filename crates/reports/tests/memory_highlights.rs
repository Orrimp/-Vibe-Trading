#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T813 / T1810 — R6 memory-highlights integration test.
//!
//! Asserts the empty-state body is byte-stable across runs and that
//! the decay-candidates footer appears when the heuristic fires.
//!
//! Originally written against the v1+ `PLACEHOLDER` constant; updated
//! at T1810 to reference the post-reflection-memory empty-state
//! constant.  The test intent (byte-stability + decay footer) is
//! preserved; the strings shifted because reflection-memory replaces
//! the v1+ placeholder body with the operator-locked Q7 empty-state.

use reports::render::memory_highlights::{
    REFLECTION_MEMORY_EMPTY_STATE, render_with_decay, render_with_lessons,
};

#[test]
fn t1810_r6_empty_state_byte_stable() {
    let a = render_with_decay(&[]);
    let b = render_with_decay(&[]);
    assert_eq!(a, b, "empty-state body must be byte-stable");
    assert!(a.contains(REFLECTION_MEMORY_EMPTY_STATE));
}

#[test]
fn t1810_r6_render_with_decay_no_decay_does_not_emit_footer() {
    let body = render_with_decay(&[]);
    assert!(!body.contains("decay candidates:"));
    assert!(body.contains(REFLECTION_MEMORY_EMPTY_STATE));
}

#[test]
fn t1810_r6_render_with_decay_emits_footer_for_decayed_strategies() {
    let decayed = vec!["alpha".to_string(), "beta".to_string()];
    let body = render_with_decay(&decayed);
    assert!(body.contains("decay candidates: alpha, beta"));
}

#[test]
fn t1810_r6_empty_lessons_collapses_to_empty_state() {
    let body = render_with_lessons(&[], &[]);
    assert!(body.starts_with("## Memory highlights\n\n"));
    assert!(body.contains(REFLECTION_MEMORY_EMPTY_STATE));
}
