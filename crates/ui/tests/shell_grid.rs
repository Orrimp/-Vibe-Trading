//! T1603 / T1611 — shell grid invariants.
//!
//! Verifies that the right-rail track in the Phase 2 shell is reserved
//! at zero pixels and the sidebar nav is fixed at the documented
//! width. The test reads the constants directly so a refactor that
//! drops the right-rail or shifts the sidebar surface in this file.

use ui::theme::layout::{RIGHT_RAIL_WIDTH_PX, SIDEBAR_ENTRIES_PHASE_3, SIDEBAR_WIDTH_PX};

#[test]
fn shell_grid_reserves_right_rail() {
    // R13 — Phase 6 Assistant slot ships as a `Length::Fixed(0.0)`
    // column today. Phase 6 swaps to a real width.
    assert!(
        (RIGHT_RAIL_WIDTH_PX - 0.0).abs() < f32::EPSILON,
        "right-rail track must reserve a 0.0 px column in Phase 2; got {RIGHT_RAIL_WIDTH_PX}"
    );
}

#[test]
fn shell_grid_sidebar_width_pinned() {
    assert!(
        (SIDEBAR_WIDTH_PX - 180.0).abs() < f32::EPSILON,
        "sidebar width must be 180 px; got {SIDEBAR_WIDTH_PX}"
    );
}

#[test]
fn shell_grid_phase_3_entries_are_six() {
    // Sidebar entries in Phase 3: Home / Debug / Strategies / Risk / Audit
    // / Charts. Phase 2's 3-entry constant was removed atomically on
    // Phase 3 ship (no forward-compat need).
    assert_eq!(SIDEBAR_ENTRIES_PHASE_3.len(), 6);
}
