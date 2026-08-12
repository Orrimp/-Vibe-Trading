//! Widget gallery visual snapshots — ui-gallery-bin v0.1 (T17).
//!
//! ## STATUS — V5 BLOCKED, deferred to follow-up `ui-gallery-table-cell`.
//!
//! The three slot tests below are `#[ignore]`d because a layout
//! interaction between iced 0.14's `widget::table::Table` (used by
//! `widgets::strategies::view`) and the fixed-height `cell::view`
//! container produces a degenerate quad at render time, panicking
//! `iced_tiny_skia::engine.rs:686` with "Build quad rectangle". The
//! diagnostic is in [`gallery_bisect.rs`](gallery_bisect.rs) — it
//! pinpoints `GALLERY_CELLS[7]` (`strategies :: ready_v1`) as the
//! first cell that triggers the panic.
//!
//! The cell wrapper's `Container::height(Length::Fixed(260))` forces
//! the inner `Table` to share that budget across its rows + header,
//! which interacts badly with the table's column-1
//! `Length::Fill`-height left-rule decoration. Bumping
//! `CELL_HEIGHT_PX` to 500 px does NOT resolve the panic, suggesting
//! the issue is in how iced computes child-quad bounds for `Table`
//! under a constrained-height parent — not a simple
//! "not-enough-space" symptom.
//!
//! Until a follow-up fixes the table cell render (either by using a
//! non-table replacement for strategies in the gallery, or by
//! deferring strategies to its own special-cased cell wrapper), V5+
//! cannot be made green. V1 (build), V2 (smoke), V3 (cell coverage),
//! V4 (mod-rs coverage) all pass and constitute the v0.1 partial
//! ship.
//!
//! ## H-GAL-2 fix (design.md § Q-ARCH-3 — H-GAL-2 FALSIFIED)
//!
//! `iced_test::screenshot` clips to the viewport rectangle; content
//! beyond the viewport is NOT captured. We therefore pass viewport
//! `(slot_w, GALLERY_LOGICAL_HEIGHT)` so the full column is rendered.
//!
//! ## R-DESIGN-2 — operator-slot PNG size
//!
//! The operator slot at 3360×9_960 @ 2.0× would yield a 6720×19920
//! physical PNG (~134 MB RGBA in memory) — the largest committed
//! baseline in the repo. The R-DESIGN-2 contingency (drop operator
//! scale to 1.5×) is documented in the design.md but not yet
//! triggered since V5 is blocked upstream.

// cockpit-cross-platform ADR-0057 D2: visual baselines are macOS-canonical.
// On Linux/Windows cosmic-text resolves body text via PlatformFallback against
// the per-OS system font DB, producing different glyph rasterization — these
// tests would not match the macOS-captured gallery PNGs. Gate the entire file
// to compile only on macOS; on Linux/Windows it compiles to nothing (skipped,
// never re-baselined). See ADR-0057 D2 and docs/runbooks/cockpit-cross-platform.md.
#![cfg(target_os = "macos")]
#![allow(clippy::expect_used, clippy::unwrap_used)]

use std::time::Duration;

#[path = "fixtures/mod.rs"]
mod fixtures;

use fixtures::visual_diff::matches_screenshot;
use ui::gallery::{GALLERY_LOGICAL_HEIGHT, GalleryApp, program_from_cockpit};

const SLOTS: &[(&str, u32, f32)] = &[
    ("floor", 1280, 1.0),
    ("typical", 1920, 1.0),
    ("operator", 3360, 2.0),
];

fn run_slot(slot_name: &str) {
    let (_, w, scale) = SLOTS
        .iter()
        .find(|(s, _, _)| *s == slot_name)
        .copied()
        .unwrap_or_else(|| panic!("unknown SLOTS row: {slot_name}"));

    let app = GalleryApp::default();
    let program = program_from_cockpit(app.cockpit.clone());
    let theme = iced::Theme::Dark;

    let screenshot = iced_test::screenshot(
        &program,
        &theme,
        (w, GALLERY_LOGICAL_HEIGHT),
        scale,
        Duration::ZERO,
    );

    let baseline = format!(
        "{}/tests/visual-baselines/gallery/{slot_name}.png",
        env!("CARGO_MANIFEST_DIR")
    );
    let test_name = format!("gallery_dark_{slot_name}");

    matches_screenshot(&screenshot, &baseline, &test_name).unwrap_or_else(|err| {
        panic!(
            "gallery visual snapshot mismatch for slot `{slot_name}`:\n{err}\n\n\
             Review the baseline / actual / diff triple."
        )
    });
}

#[test]
#[ignore = "BLOCKED on iced Table cell-bounds panic; see crate docs in gallery_snapshots.rs and gallery_bisect.rs. Re-enable when follow-up `ui-gallery-table-cell` fixes the strategies cell render."]
fn gallery_dark_floor() {
    run_slot("floor");
}

#[test]
#[ignore = "BLOCKED on iced Table cell-bounds panic; see gallery_snapshots.rs module docs."]
fn gallery_dark_typical() {
    run_slot("typical");
}

#[test]
#[ignore = "BLOCKED on iced Table cell-bounds panic; see gallery_snapshots.rs module docs."]
fn gallery_dark_operator() {
    run_slot("operator");
}
