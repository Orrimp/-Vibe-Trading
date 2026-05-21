#![allow(clippy::cast_possible_truncation)]
//! Models screen — Phase F (ui-rethink-phase-f-memory-models-assistant R2.1-R2.4).
//!
//! Body composition (top to bottom):
//!
//! 1. **Toolbar row** — family filter chips (TCN active; `PatchTST` +
//!    `Transformer` disabled per R2.2 / K3) + status filter chips.
//! 2. **Checkpoint list** — one row per filtered `CheckpointMeta`, with
//!    columns: family | model_revision (truncated) | data span | status pill
//!    | sparkline placeholder (K3 deferred to v0.2.0) | file size.
//! 3. **Empty-state** — `MODELS_EMPTY_STATE` placeholder when `checkpoints`
//!    is empty post-hydrate (Q3=(a) / R2.3).
//!
//! **Q7=(c):** All checkpoints render status pill as `"staged"` at v0.1.0.
//! **K3:** Sparkline column shows `MODELS_SPARKLINE_PLACEHOLDER` ("—") with
//!    tooltip `MODELS_SPARKLINE_DEFERRED_TOOLTIP`; no live data at v0.1.0.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::{Button, Column, Container, Row, Scrollable, Text, button};
use iced::{Border, Element, Length};

use crate::models::state::{CheckpointMeta, ModelFamily, ModelStatus, ModelsScreenState};
use crate::state::{Cockpit, Message};
use crate::strings::{
    MODELS_EMPTY_STATE, MODELS_FAMILY_PATCHTST_DISABLED_TOOLTIP,
    MODELS_FAMILY_TRANSFORMER_DISABLED_TOOLTIP, MODELS_SPARKLINE_DEFERRED_TOOLTIP,
    MODELS_SPARKLINE_PLACEHOLDER, MODELS_STATUS_STAGED_TOOLTIP, MODELS_TOOLBAR_FAMILY_LABEL,
    MODELS_TOOLBAR_STATUS_LABEL,
};
use crate::theme::{ThemeMode, color, radius, space, text};

/// Render the Models screen body.
///
/// Called by `shell::screen_body` when `current_screen == Screen::Models`
/// (replacing the Phase A `placeholder::view` route per R2.3).
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_, Message> {
    let state = &model.models_screen_state;

    // ── Toolbar ─────────────────────────────────────────────────────────────
    let toolbar = build_toolbar(state, mode);

    // ── Body ─────────────────────────────────────────────────────────────────
    let filtered: Vec<&CheckpointMeta> = state
        .checkpoints
        .iter()
        .filter(|c| state.family_filter.contains(&c.family))
        .filter(|c| state.status_filter.contains(&c.status))
        .collect();

    let body: Element<'_, Message> = if filtered.is_empty() {
        Container::new(
            Text::new(MODELS_EMPTY_STATE)
                .size(text::BODY)
                .color(color::FG_3.current(mode)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    } else {
        let mut col = Column::new().spacing(space::XS);
        for cp in filtered {
            col = col.push(build_checkpoint_row(cp, mode));
        }
        Scrollable::new(col)
            .width(Length::Fill)
            .height(Length::Fill)
            .into()
    };

    Column::new()
        .spacing(space::S)
        .push(toolbar)
        .push(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(space::M as u16)
        .into()
}

/// Build the toolbar row: family chips + status chips.
fn build_toolbar(state: &ModelsScreenState, mode: ThemeMode) -> Element<'_, Message> {
    let family_label = Text::new(MODELS_TOOLBAR_FAMILY_LABEL)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    // TCN chip — active (only on-disk family at v0.1.0).
    let tcn_active = state.family_filter.contains(&ModelFamily::Tcn);
    let tcn_chip = chip_active(
        ModelFamily::Tcn.label(),
        tcn_active,
        Message::ModelsSetFamilyFilter(vec![ModelFamily::Tcn]),
        mode,
    );

    // PatchTST chip — disabled at v0.1.0.
    let patchtst_label = format!(
        "{} ({})",
        ModelFamily::PatchTst.label(),
        MODELS_FAMILY_PATCHTST_DISABLED_TOOLTIP
    );
    let patchtst_chip = chip_disabled(patchtst_label, mode);

    // Transformer chip — disabled at v0.1.0.
    let transformer_label = format!(
        "{} ({})",
        ModelFamily::Transformer.label(),
        MODELS_FAMILY_TRANSFORMER_DISABLED_TOOLTIP
    );
    let transformer_chip = chip_disabled(transformer_label, mode);

    let status_label = Text::new(MODELS_TOOLBAR_STATUS_LABEL)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    // Staged chip.
    let staged_active = state.status_filter.contains(&ModelStatus::Staged);
    let staged_chip = chip_active(
        ModelStatus::Staged.label(),
        staged_active,
        Message::ModelsSetStatusFilter(vec![ModelStatus::Staged]),
        mode,
    );

    Row::new()
        .spacing(space::S)
        .align_y(iced::alignment::Vertical::Center)
        .push(family_label)
        .push(tcn_chip)
        .push(patchtst_chip)
        .push(transformer_chip)
        .push(Text::new("  ").size(text::SMALL)) // gutter
        .push(status_label)
        .push(staged_chip)
        .into()
}

/// Active / inactive toggle chip button.
fn chip_active(label: &str, active: bool, msg: Message, mode: ThemeMode) -> Element<'_, Message> {
    Button::new(Text::new(label).size(text::SMALL).color(if active {
        color::FG_1.current(mode)
    } else {
        color::FG_2.current(mode)
    }))
    .on_press(msg)
    .padding([space::XS as u16, space::S as u16])
    .style(move |_t: &iced::Theme, _s: button::Status| button::Style {
        background: if active {
            Some(color::ACCENT.current(mode).into())
        } else {
            Some(color::PANEL_RAISED.current(mode).into())
        },
        text_color: if active {
            color::FG_1.current(mode)
        } else {
            color::FG_2.current(mode)
        },
        border: Border {
            radius: radius::R2.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

/// Visually disabled chip (no `on_press`). Takes an owned `String` so the
/// caller's local label is moved in and the iced widget owns the allocation.
fn chip_disabled(label: String, mode: ThemeMode) -> Element<'static, Message> {
    Button::new(
        Text::new(label)
            .size(text::SMALL)
            .color(color::FG_4.current(mode)),
    )
    .padding([space::XS as u16, space::S as u16])
    .style(move |_t: &iced::Theme, _s: button::Status| button::Style {
        background: Some(color::PANEL_RAISED.current(mode).into()),
        text_color: color::FG_4.current(mode),
        border: Border {
            radius: radius::R2.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

/// Build one checkpoint row.
///
/// Columns: family label | `model_revision` (first 8 chars) | data span |
/// status pill (`"staged"` per Q7=(c)) | sparkline placeholder (K3) |
/// file size (bytes).
fn build_checkpoint_row(cp: &CheckpointMeta, mode: ThemeMode) -> Element<'_, Message> {
    // Truncate `model_revision` to first 8 chars for display.
    let rev_display = if cp.model_revision.len() > 8 {
        format!("{}…", &cp.model_revision[..8])
    } else {
        cp.model_revision.to_string()
    };

    // Data span: "start … end"
    let span_display = format!(
        "{} \u{2026} {}",
        cp.data_span_start.as_str(),
        cp.data_span_end.as_str()
    );

    let family_cell = Text::new(cp.family.label())
        .size(text::SMALL)
        .color(color::ACCENT.current(mode));

    let rev_cell = Text::new(rev_display)
        .size(text::MICRO)
        .color(color::FG_1.current(mode));

    let span_cell = Text::new(span_display)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));

    // Status pill — always "staged" per Q7=(c); tooltip via label suffix.
    let status_label = format!("{} ({})", cp.status.label(), MODELS_STATUS_STAGED_TOOLTIP);
    let status_cell = Text::new(status_label)
        .size(text::MICRO)
        .color(color::FG_2.current(mode));

    // Sparkline — deferred per K3; shows "—" + tooltip in label.
    let sparkline_label =
        format!("{MODELS_SPARKLINE_PLACEHOLDER} {MODELS_SPARKLINE_DEFERRED_TOOLTIP}");
    let sparkline_cell = Text::new(sparkline_label)
        .size(text::MICRO)
        .color(color::FG_4.current(mode));

    let size_cell = Text::new(format!("{} B", cp.file_size_bytes))
        .size(text::MICRO)
        .color(color::FG_4.current(mode));

    let row = Row::new()
        .spacing(space::S)
        .align_y(iced::alignment::Vertical::Center)
        .push(family_cell)
        .push(rev_cell)
        .push(span_cell)
        .push(status_cell)
        .push(sparkline_cell)
        .push(iced::widget::Space::new().width(Length::Fill))
        .push(size_cell);

    Container::new(row)
        .width(Length::Fill)
        .padding([space::XS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| iced::widget::container::Style {
            background: Some(color::PANEL_RAISED.current(mode).into()),
            border: iced::Border {
                radius: radius::R2.into(),
                ..Default::default()
            },
            ..Default::default()
        })
        .into()
}
