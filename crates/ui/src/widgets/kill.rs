//! Kill-switch panel — the one destructive control in the cockpit.
//!
//! Flow (T19 + R6 + R7):
//! 1. Idle: big red button labelled "Stop trading".
//! 2. Confirming: dialog with body copy + typed phrase input. Confirm is
//!    disabled until the input exactly matches `strings::KILL_SAFETY_PHRASE`.
//! 3. Flattening: waiting for agent to ack. Button stays disabled.
//! 4. Halted: red banner, "remove .halt and re-arm" hint, link to runbook.
//!
//! ## T1504 — Focus ring (Phase 5 closure)
//!
//! **API limitation (iced 0.14):** `button::Status` does NOT expose a
//! `Focused` variant in this version. Available variants are
//! `Active / Hovered / Pressed / Disabled`. The Phase 1–4 implementation
//! routed `focus::ring` to the `Hovered` state as a best-effort visual
//! approximation.
//!
//! **Phase 5 (T1912 / TD-1 path b)** closes the four-phase deferral via
//! the custom-widget escape hatch at
//! [`widgets::focus_ring`](super::focus_ring). The kill button +
//! kill-confirm input + confirm/cancel buttons now wrap in
//! `focus_ring::wrap(...)` so the halo overlay lands on the
//! parent-side-tracked focus state (`Cockpit::focused_widget`) rather
//! than the iced-side `Hovered` proxy.
//!
//! ## T1506 — Sunken input styling
//!
//! The confirm input uses sunken styling:
//!   - Background: `PANEL_SUNKEN`
//!   - Border unfocused: `BORDER_2` (1 px, R2)
//!   - Border focused:   `ACCENT` (1 px, R2)
//!
//! A 1 px hairline `Container` row above the input renders the
//! `shadow::shadow_inset(mode)` colour — CSS `inset 0 1px 0` analogue
//! (iced's `Shadow` is outer-only; the hairline is the workaround).
//!
//! **API limitation (iced 0.14):** `text_input::Style` does NOT have a
//! `shadow` field, so the `focus::ring` shadow cannot be applied
//! directly to the confirm input via the iced-native style system.
//! Phase 5 (T1912) routes the halo through the parent-side
//! `widgets::focus_ring::wrap(...)` overlay, which is iced-version-
//! independent. The border-colour shift from `BORDER_2` → `ACCENT` on
//! iced-side focus is preserved (via `text_input::Status::Focused
//! { is_hovered: _ }`) for redundancy.

use iced::widget::{
    Button, Column, Container, Row, Space, Text, TextInput, button, container, text_input,
};
use iced::{Border, Element, Length};

use crate::state::{Cockpit, KillState, Message};
use crate::strings::{
    KILL_BUTTON_HELP, KILL_BUTTON_LABEL, KILL_CANCEL_LABEL, KILL_CONFIRM_LABEL, KILL_DIALOG_BODY,
    KILL_DIALOG_TITLE, KILL_HALTED_BANNER, KILL_HALTED_HINT, KILL_PHRASE_LABEL,
    KILL_PHRASE_MISMATCH_HINT, KILL_RUNBOOK_LINK_LABEL, KILL_SAFETY_PHRASE, PANEL_KILL_TITLE,
};
use crate::theme::{ThemeMode, color, focus, radius, shadow, space, text};

use super::focus_ring;
use super::frame::{muted_body, panel};

#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    panel(PANEL_KILL_TITLE, view_inner(model), ThemeMode::Dark)
}

/// Body-only renderer used by the Phase 5 `HumanControl` panel
/// (T1906 / R2.3) — returns the kill body without the outer
/// `frame::panel` chrome so the `HumanControl` panel can host it as the
/// bottom-action sub-block. Public `view` retains its current shape
/// (R2.3) so the `Debug`-screen invariant doesn't shift.
#[must_use]
pub fn view_inner(model: &Cockpit) -> Element<'_, Message> {
    let focused = model.focused_widget.as_deref();
    match &model.kill {
        KillState::Idle => idle_body(focused),
        KillState::Confirming { typed } => confirming_body(typed, focused),
        KillState::Flattening => flattening_body(),
        KillState::Halted { reason } => halted_body(reason.as_str()),
    }
}

fn idle_body<'a>(focused: Option<&str>) -> Element<'a, Message> {
    let mode = ThemeMode::Dark;
    // T1504 — kill trigger button: focus ring wired on Hovered as best-effort
    // (iced 0.14 button::Status has no Focused variant — see module doc).
    let button = Button::new(
        Text::new(KILL_BUTTON_LABEL)
            .size(text::DISPLAY)
            .color(color::FG_1.current(mode)),
    )
    .on_press(Message::KillPressed)
    .style(move |_theme: &iced::Theme, status| {
        let shadow = match status {
            button::Status::Hovered => focus::ring(mode),
            button::Status::Active | button::Status::Pressed | button::Status::Disabled => {
                iced::Shadow::default()
            }
        };
        button::Style {
            shadow,
            ..button::Style::default()
        }
    });

    let button_focused = focused == Some(focus_ring::KILL_BUTTON);
    let wrapped = focus_ring::wrap(focus_ring::KILL_BUTTON, button.into(), button_focused, mode);
    Column::new()
        .spacing(space::S)
        .push(wrapped)
        .push(muted_body(KILL_BUTTON_HELP))
        .into()
}

// `too_many_lines`: the kill-confirm body is one render pass —
// splitting into helpers obscures the input + button + label flow.
#[allow(clippy::too_many_lines)]
fn confirming_body<'a>(typed: &'a str, focused: Option<&str>) -> Element<'a, Message> {
    let mode = ThemeMode::Dark;
    let matched = typed == KILL_SAFETY_PHRASE;

    // T1506 — 1 px hairline Container above the confirm input.
    // Renders `shadow::shadow_inset(mode)` colour as a top-edge visual cue,
    // approximating CSS `inset 0 1px 0 <color>` (iced Shadow is outer-only).
    let hairline_color = shadow::shadow_inset(mode);
    let hairline = Container::new(Space::new())
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(hairline_color.into()),
            ..Default::default()
        });

    // T1504 + T1506 — confirm input style:
    //   Unfocused: BORDER_2 border, PANEL_SUNKEN background.
    //   Focused:   ACCENT border,   PANEL_SUNKEN background.
    //   NOTE: focus::ring shadow NOT applied — text_input::Style has no
    //   `shadow` field in iced 0.14 (see module doc for the limitation).
    let input = TextInput::new(KILL_PHRASE_LABEL, typed)
        .on_input(Message::KillConfirmPhraseChanged)
        .size(text::BODY)
        .style(move |_theme: &iced::Theme, status| {
            let border_color = match status {
                text_input::Status::Focused { .. } => color::ACCENT.current(mode),
                _ => color::BORDER_2.current(mode),
            };
            text_input::Style {
                background: color::PANEL_SUNKEN.current(mode).into(),
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: radius::R2.into(),
                },
                icon: color::FG_4.current(mode),
                placeholder: color::FG_4.current(mode),
                value: color::FG_1.current(mode),
                selection: color::ACCENT_SOFT.current(mode),
            }
        });

    // T1504 — confirm button: focus ring wired on Hovered as best-effort
    // (iced 0.14 button::Status has no Focused variant — see module doc).
    let confirm_text = Text::new(KILL_CONFIRM_LABEL)
        .size(text::BODY)
        .color(color::FG_1.current(mode));
    let confirm_button = if matched {
        Button::new(confirm_text)
            .on_press(Message::KillConfirmed)
            .style(move |_theme: &iced::Theme, status| button::Style {
                shadow: match status {
                    button::Status::Hovered => focus::ring(mode),
                    button::Status::Active | button::Status::Pressed | button::Status::Disabled => {
                        iced::Shadow::default()
                    }
                },
                ..button::Style::default()
            })
    } else {
        // No `on_press` = disabled.
        Button::new(confirm_text)
            .style(move |_theme: &iced::Theme, _status| button::Style::default())
    };

    let cancel_button = Button::new(
        Text::new(KILL_CANCEL_LABEL)
            .size(text::BODY)
            .color(color::FG_1.current(mode)),
    )
    .on_press(Message::KillCancelled);

    // Phase 5 (T1912 / TD-1 path b) — wrap the input + both buttons in
    // the focus-ring overlay so the halo lands on the parent-side
    // `Cockpit::focused_widget` state.
    let input_wrapped = focus_ring::wrap(
        focus_ring::KILL_CONFIRM_INPUT,
        input.into(),
        focused == Some(focus_ring::KILL_CONFIRM_INPUT),
        mode,
    );
    let confirm_wrapped = focus_ring::wrap(
        focus_ring::KILL_CONFIRM_BUTTON,
        confirm_button.into(),
        focused == Some(focus_ring::KILL_CONFIRM_BUTTON),
        mode,
    );
    let cancel_wrapped = focus_ring::wrap(
        focus_ring::KILL_CANCEL_BUTTON,
        cancel_button.into(),
        focused == Some(focus_ring::KILL_CANCEL_BUTTON),
        mode,
    );

    let mut col = Column::new()
        .spacing(space::M)
        .push(
            Text::new(KILL_DIALOG_TITLE)
                .size(text::H2)
                .color(color::FG_1.current(mode)),
        )
        .push(
            Text::new(KILL_DIALOG_BODY)
                .size(text::BODY)
                .color(color::FG_3.current(mode)),
        )
        // T1506 — hairline inset shadow workaround above the confirm input.
        .push(hairline)
        .push(input_wrapped);

    if !matched && !typed.is_empty() {
        col = col.push(
            Text::new(KILL_PHRASE_MISMATCH_HINT)
                .size(text::MICRO)
                .color(color::WARN_500.current(mode)),
        );
    }

    col.push(
        Row::new()
            .push(confirm_wrapped)
            .push(cancel_wrapped)
            .spacing(space::M),
    )
    .into()
}

fn flattening_body<'a>() -> Element<'a, Message> {
    muted_body(KILL_DIALOG_BODY)
}

fn halted_body(_reason: &str) -> Element<'_, Message> {
    let mode = ThemeMode::Dark;
    Column::new()
        .spacing(space::M)
        .push(
            Text::new(KILL_HALTED_BANNER)
                .size(text::DISPLAY)
                .color(color::DOWN_500.current(mode)),
        )
        .push(
            Text::new(KILL_HALTED_HINT)
                .size(text::BODY)
                .color(color::FG_3.current(mode)),
        )
        .push(
            Text::new(KILL_RUNBOOK_LINK_LABEL)
                .size(text::MICRO)
                .color(color::ACCENT.current(mode)),
        )
        .into()
}

/// Dialog open predicate — used by tests and the binary to decide whether
/// to intercept keyboard input.
#[must_use]
pub fn dialog_open(model: &Cockpit) -> bool {
    matches!(model.kill, KillState::Confirming { .. })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_open_only_while_confirming() {
        let mut m = Cockpit::new();
        assert!(!dialog_open(&m));
        crate::state::update(&mut m, Message::KillPressed);
        assert!(dialog_open(&m));
        crate::state::update(&mut m, Message::KillCancelled);
        assert!(!dialog_open(&m));
    }
}
