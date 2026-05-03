#![allow(clippy::unwrap_used, clippy::expect_used)]
//! T813 — R6 memory-highlights integration test.
//!
//! Asserts the placeholder body is byte-stable across runs and that
//! the decay-candidates footer appears when the heuristic fires.

use reports::render::memory_highlights::{render, render_with_decay, PLACEHOLDER};

#[test]
fn t813_r6_placeholder_byte_stable() {
    let a = render();
    let b = render();
    assert_eq!(a, b);
    assert_eq!(a, PLACEHOLDER);
}

#[test]
fn t813_r6_render_with_decay_no_decay_does_not_emit_footer() {
    let body = render_with_decay(&[]);
    assert!(!body.contains("decay candidates:"));
    assert!(body.contains(PLACEHOLDER));
}

#[test]
fn t813_r6_render_with_decay_emits_footer_for_decayed_strategies() {
    let decayed = vec!["alpha".to_string(), "beta".to_string()];
    let body = render_with_decay(&decayed);
    assert!(body.contains("decay candidates: alpha, beta"));
}
