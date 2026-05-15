//! Widget gallery module — ui-gallery-bin v0.1.
//!
//! ## Origin
//!
//! Implements Cycle-1, item C from
//! [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md § 5.1`][1]
//! and [`§ 3.3`][2].
//!
//! [1]: ../../../../spec/dev-notes/ui-testability-deep-dive-2026-05-15.md
//! [2]: ../../../../spec/dev-notes/ui-testability-deep-dive-2026-05-15.md
//!
//! ## Q-GALLERY-SCOPE (LOCKED 2026-05-15)
//!
//! The gallery imports `crate::fixtures` directly and does **not** fork
//! the state builders. Any `fake_*` helper the gallery needs is added to
//! `crates/ui/src/fixtures.rs`, not authored locally. This contract is
//! enforced by the evaluator's drift-gate:
//!
//! ```text
//! grep -n 'fn fake_\|fn synth' crates/ui/src/gallery/
//! ```
//!
//! The above MUST produce no output.
//!
//! ## H-GAL-2 / Q-ARCH-3 (design.md — H-GAL-2 FALSIFIED)
//!
//! `iced_test::screenshot` clips to the viewport rectangle; scrollable
//! content beyond the viewport is NOT captured. The snapshot test
//! therefore passes viewport `(slot_w, GALLERY_LOGICAL_HEIGHT)` so the
//! full column is rendered. The bin (operator path) wraps `view_all` in
//! `scrollable(...)` for normal window UX.
//!
//! ## Public-API surface (design.md § Module layout)
//!
//! | # | Item                                        | Module          |
//! |---|---------------------------------------------|-----------------|
//! | 1 | `pub struct GalleryCell`                    | `gallery::cell` |
//! | 2 | `pub const GALLERY_CELLS: &[GalleryCell]`   | `gallery::routes`|
//! | 3 | `pub const EXPECTED_WIDGETS: &[&str]`       | `gallery::routes`|
//! | 4 | `pub const GALLERY_LOGICAL_HEIGHT: u32`     | `gallery`       |
//! | 5 | `pub fn view(cell) -> Element`              | `gallery::cell` |
//!
//! `view_all` is `pub(crate)` — not part of the stable contract.

pub mod cell;
pub mod routes;

pub use cell::{view, GalleryCell, CELL_HEIGHT_PX};
pub(crate) use routes::seed_for_all_cells;
pub use routes::{view_slice, EXPECTED_WIDGETS, GALLERY_CELLS, GALLERY_CELL_COUNT};

use crate::state::{Cockpit, Message};

/// Logical height of the gallery canvas for snapshot tests.
///
/// Analytically computed: `GALLERY_CELLS.len() * CELL_HEIGHT_PX`.
/// The snapshot test passes `(slot_w, GALLERY_LOGICAL_HEIGHT)` as the
/// viewport so `iced_test::screenshot` renders the full column height.
///
/// If cells overflow (R-DESIGN-5), bump this constant by the measured
/// overflow and regenerate baselines.
pub const GALLERY_LOGICAL_HEIGHT: u32 = {
    // GALLERY_CELL_COUNT * CELL_HEIGHT_PX + padding headroom.
    // CELL_HEIGHT_PX = 260.0, GALLERY_CELL_COUNT = 36 cells.
    // 36 * 260 = 9_360. Adding 600 px headroom for the outer container
    // padding (space::S per cell * 2 sides = ~16 px * 36 = 576 px).
    9_960
};

/// Compose all gallery cells into a bare `column!` (no scrollable).
///
/// The interactive bin calls `scrollable(view_all(...))` for operator UX.
/// The snapshot test drives `view_all` directly at `GALLERY_LOGICAL_HEIGHT`.
#[must_use]
pub fn view_all(model: &Cockpit) -> iced::Element<'_, Message> {
    routes::view_all(model)
}

/// Build a gallery `iced::Application` for the snapshot test.
///
/// Returns an `impl iced::Program` seeded with the canonical gallery
/// `Cockpit` fixture. The snapshot test passes `&program` to
/// `iced_test::screenshot(...)`.
#[must_use]
pub fn program_from_cockpit(
    cockpit: Cockpit,
) -> iced::Application<impl iced::Program<State = GalleryApp, Message = Message, Theme = iced::Theme>>
{
    let boot = move || {
        (
            GalleryApp {
                cockpit: cockpit.clone(),
            },
            iced::Task::none(),
        )
    };
    iced::application(boot, GalleryApp::update, GalleryApp::view)
        .title(GalleryApp::title)
        .theme(GalleryApp::theme)
}

/// Test wrapper carrying the seeded `Cockpit` for the gallery snapshot.
pub struct GalleryApp {
    pub cockpit: Cockpit,
}

impl Default for GalleryApp {
    fn default() -> Self {
        Self {
            cockpit: seed_for_all_cells(),
        }
    }
}

impl GalleryApp {
    /// Title — minimal; window title is not visible in snapshot tests.
    #[must_use]
    pub fn title(&self) -> String {
        "Widget Gallery".to_string()
    }

    /// Theme locked to Dark — baselines are dark-mode only.
    #[must_use]
    pub fn theme(&self) -> iced::Theme {
        iced::Theme::Dark
    }

    /// Update — gallery is static; no messages are expected.
    pub fn update(&mut self, msg: Message) -> iced::Task<Message> {
        crate::state::update(&mut self.cockpit, msg);
        iced::Task::none()
    }

    /// View — bare `view_all` column (no scrollable).
    ///
    /// The snapshot test uses this to capture all cells at
    /// `GALLERY_LOGICAL_HEIGHT`. The scrollable is the bin's concern.
    #[must_use]
    pub fn view(&self) -> iced::Element<'_, Message> {
        view_all(&self.cockpit)
    }

    /// View wrapped in a `scrollable` — used by the interactive bin so
    /// the operator can scroll the 9 000+ px tall gallery in a normal
    /// window. NOT used by the snapshot test.
    #[must_use]
    pub fn view_scrollable(&self) -> iced::Element<'_, Message> {
        iced::widget::scrollable(view_all(&self.cockpit))
            .height(iced::Length::Fill)
            .into()
    }
}

// ── Tests (V3 + V4 — exhaustiveness) ─────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::unwrap_used)]
mod tests {
    use super::*;

    /// V3 — every widget in `EXPECTED_WIDGETS` has at least one cell in
    /// `GALLERY_CELLS`. Adding a widget to `EXPECTED_WIDGETS` without
    /// authoring a cell fails this test with the missing widget set.
    #[test]
    fn every_expected_widget_has_at_least_one_gallery_cell() {
        let covered: std::collections::HashSet<&str> =
            GALLERY_CELLS.iter().map(|c| c.widget).collect();
        let missing: Vec<&&str> = EXPECTED_WIDGETS
            .iter()
            .filter(|w| !covered.contains(*w))
            .collect();
        assert!(
            missing.is_empty(),
            "gallery is missing cells for widgets: {missing:?}",
        );
    }

    /// V4 — every `pub mod` in `widgets/mod.rs` is listed in
    /// `EXPECTED_WIDGETS`. Adding a module without updating this list
    /// fails loudly.
    ///
    /// **Q-ARCH-2:** uses `include_str!` + pure-stdlib `strip_prefix`
    /// parse (no `regex` crate). `pub(crate) mod canvas_chart;` is
    /// intentionally excluded because it starts with `pub(crate)`, not
    /// `pub mod`.
    #[test]
    fn every_widget_mod_is_listed_in_expected_widgets() {
        let mod_rs = include_str!("../widgets/mod.rs");
        let expected_set: std::collections::HashSet<&str> =
            EXPECTED_WIDGETS.iter().copied().collect();

        let mut unlisted: Vec<&str> = Vec::new();
        for line in mod_rs.lines() {
            let trimmed = line.trim_start();
            // Match only `pub mod NAME;` — NOT `pub(crate) mod NAME;`
            let Some(after_pub_mod) = trimmed.strip_prefix("pub mod ") else {
                continue;
            };
            let Some(name) = after_pub_mod.strip_suffix(';') else {
                continue;
            };
            let name = name.trim();
            if !expected_set.contains(name) {
                unlisted.push(name);
            }
        }
        assert!(
            unlisted.is_empty(),
            "widgets/mod.rs has `pub mod` entries not listed in EXPECTED_WIDGETS: {unlisted:?}\n\
             Add them to `gallery::routes::EXPECTED_WIDGETS` and author a GalleryCell.",
        );
    }

    /// Sanity: `GALLERY_LOGICAL_HEIGHT` is large enough to hold all cells.
    #[test]
    fn gallery_logical_height_covers_all_cells() {
        let min_required = GALLERY_CELL_COUNT as f32 * CELL_HEIGHT_PX;
        assert!(
            GALLERY_LOGICAL_HEIGHT as f32 >= min_required,
            "GALLERY_LOGICAL_HEIGHT ({}) is too small for {} cells × {} px = {} px",
            GALLERY_LOGICAL_HEIGHT,
            GALLERY_CELL_COUNT,
            CELL_HEIGHT_PX,
            min_required,
        );
    }
}
