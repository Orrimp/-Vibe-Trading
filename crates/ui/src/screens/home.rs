//! Home screen — Phase 2 (T1604).
//!
//! Composes the existing four widgets into a 2×2 grid: `PnL` + Positions
//! on the top row, Strategies + Tape on the bottom row. Same widget
//! code as Phase 1, same panel chrome — Phase 2 only relocates the
//! rendering host. The tape-row → audit-modal trigger flow is preserved
//! (the modal is wrapped at the shell level, not the screen level).
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::{Column, Row};
use iced::Length;

use crate::state::Cockpit;
use crate::theme::{layout, space, ThemeMode};
use crate::widgets::{agent_feed, pnl, positions, strategies};

/// Render the Home screen body.
// `cast_possible_truncation`: `space::*` constants are `u32` with bounded
// values 0..64; cast to `u16` padding is safe.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(model: &Cockpit, _mode: ThemeMode) -> crate::Element<'_> {
    let _ = layout::PANEL_PADDING;
    let top = Row::new()
        .spacing(layout::PANEL_OUTER_GAP)
        .push(pnl::view(model))
        .push(positions::view(model));
    let bottom = Row::new()
        .spacing(layout::PANEL_OUTER_GAP)
        .push(strategies::view(model))
        .push(agent_feed::view(model));
    Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(top)
        .push(bottom)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}
