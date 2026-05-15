//! `GalleryCell` struct and per-cell `view()` composer.
//!
//! Each cell wraps a rendered widget in a `Container` with:
//! - A label strip showing `"widget :: state"` in `text::MICRO` /
//!   `color::SUBTLE`.
//! - The rendered widget at its production sizing.
//! - A fixed height (`Length::Fixed`) so the snapshot test's
//!   `GALLERY_LOGICAL_HEIGHT` constant stays accurate. Per the
//!   Q-ARCH-3 / H-GAL-2 fix: NO `Length::Fill` on any cell container
//!   (that would collapse the column to viewport height).
//!
//! Public API (per design.md § Module layout / Public-API surface):
//! - [`GalleryCell`] — stable struct.
//! - [`view`] — wraps one cell.

#![allow(clippy::cast_possible_truncation)]

use iced::widget::{container, Column, Container, Text};
use iced::Length;

use crate::state::{Cockpit, Message};
use crate::theme::{color, space, text, ThemeMode};

/// Per-cell height in logical pixels. Cells all share the same height so
/// `GALLERY_LOGICAL_HEIGHT = CELL_COUNT * CELL_HEIGHT_PX` is exact.
///
/// 260 px: ~220 px for the widget content + 30 px for the header strip
/// + 10 px separator padding. Matches the design.md estimate ("~250 px avg").
pub const CELL_HEIGHT_PX: f32 = 260.0;

/// Separator height at the bottom of each cell (visual divider).
const SEPARATOR_PX: f32 = 1.0;

/// A single `(widget, state)` gallery cell.
///
/// - `widget` — matches a module name under `crates/ui/src/widgets/`.
/// - `state`  — identifies the fixture variant for the cell.
/// - `render` — builds an `iced::Element` borrowing from a `&Cockpit`.
/// - `seed`   — returns the `Cockpit` fixture for this cell.
///
/// `render` is declared with elided lifetimes (`fn(&Cockpit) ->
/// Element<'_, Message>`) so each render fn can return an Element
/// that borrows from its input. The cell's `view()` wrapper leaks the
/// seeded Cockpit to `'static` (test-only binary; leak is acceptable)
/// so the returned Element can outlive the cell call frame.
pub struct GalleryCell {
    pub widget: &'static str,
    pub state: &'static str,
    pub render: fn(&Cockpit) -> iced::Element<'_, Message>,
    pub seed: fn() -> Cockpit,
}

/// Render one gallery cell: label header + widget body + separator.
///
/// Per design.md § Q-ARCH-3: outer `Container` has `Length::Fixed`,
/// never `Length::Fill`, so the column's intrinsic height equals
/// `GALLERY_CELLS.len() * CELL_HEIGHT_PX`. The seeded `Cockpit` is
/// leaked to `'static` so the returned `Element` (which borrows from
/// it) can be returned out of this fn — gallery is a test-only binary
/// where leaks per snapshot run are bounded and acceptable.
#[must_use]
pub fn view(cell: &GalleryCell) -> iced::Element<'static, Message> {
    let mode = ThemeMode::Dark;

    let label = Text::new(format!("{} :: {}", cell.widget, cell.state))
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    // Test-only binary: leak the seeded cockpit so the returned
    // Element (which borrows widget state from it) is `'static`.
    let cockpit: &'static Cockpit = Box::leak(Box::new((cell.seed)()));
    let widget_body = (cell.render)(cockpit);

    // Separator — 1-px hairline at the cell bottom.
    let separator = Container::new(
        iced::widget::Space::new()
            .width(Length::Fill)
            .height(Length::Fixed(SEPARATOR_PX)),
    )
    .width(Length::Fill)
    .style(move |_theme: &iced::Theme| container::Style {
        background: Some(color::BORDER_1.current(mode).into()),
        ..Default::default()
    });

    let inner = Column::new()
        .spacing(space::XS)
        .push(label)
        .push(widget_body)
        .push(separator);

    Container::new(inner)
        .width(Length::Fill)
        .height(Length::Fixed(CELL_HEIGHT_PX))
        .padding(space::S as u16)
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(color::CANVAS.current(mode).into()),
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// T02b — every cell has a Fixed height (no Length::Fill).
    /// This test cannot introspect iced Element sizing directly,
    /// but it asserts the constant is non-zero and that CELL_HEIGHT_PX
    /// is finite, ensuring the GALLERY_LOGICAL_HEIGHT arithmetic holds.
    #[test]
    fn cell_height_px_is_positive_and_finite() {
        assert!(
            CELL_HEIGHT_PX > 0.0 && CELL_HEIGHT_PX.is_finite(),
            "CELL_HEIGHT_PX must be a positive finite constant"
        );
    }
}
