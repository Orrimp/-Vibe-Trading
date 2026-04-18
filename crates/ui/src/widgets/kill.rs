//! Kill-switch panel — the one destructive control in the cockpit.
//!
//! Flow (T19 + R6 + R7):
//! 1. Idle: big red button labelled "Stop trading".
//! 2. Confirming: dialog with body copy + typed phrase input. Confirm is
//!    disabled until the input exactly matches `strings::KILL_SAFETY_PHRASE`.
//! 3. Flattening: waiting for agent to ack. Button stays disabled.
//! 4. Halted: red banner, "remove .halt and re-arm" hint, link to runbook.

use iced::widget::{Button, Column, Row, Text, TextInput};
use iced::Element;

use crate::state::{Cockpit, KillState, Message};
use crate::strings::{
    KILL_BUTTON_HELP, KILL_BUTTON_LABEL, KILL_CANCEL_LABEL, KILL_CONFIRM_LABEL, KILL_DIALOG_BODY,
    KILL_DIALOG_TITLE, KILL_HALTED_BANNER, KILL_HALTED_HINT, KILL_PHRASE_LABEL,
    KILL_PHRASE_MISMATCH_HINT, KILL_RUNBOOK_LINK_LABEL, KILL_SAFETY_PHRASE, PANEL_KILL_TITLE,
};
use crate::theme::{color, space, text};

use super::frame::{muted_body, panel};

#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let body: Element<Message> = match &model.kill {
        KillState::Idle => idle_body(),
        KillState::Confirming { typed } => confirming_body(typed),
        KillState::Flattening => flattening_body(),
        KillState::Halted { reason } => halted_body(reason.as_str()),
    };
    panel(PANEL_KILL_TITLE, body)
}

fn idle_body<'a>() -> Element<'a, Message> {
    let button = Button::new(
        Text::new(KILL_BUTTON_LABEL)
            .size(text::DISPLAY)
            .color(color::FG),
    )
    .on_press(Message::KillPressed);

    Column::new()
        .spacing(space::S)
        .push(button)
        .push(muted_body(KILL_BUTTON_HELP))
        .into()
}

fn confirming_body(typed: &str) -> Element<'_, Message> {
    let matched = typed == KILL_SAFETY_PHRASE;

    let input = TextInput::new(KILL_PHRASE_LABEL, typed)
        .on_input(Message::KillConfirmPhraseChanged)
        .size(text::BODY);

    let confirm_text = Text::new(KILL_CONFIRM_LABEL)
        .size(text::BODY)
        .color(color::FG);
    let confirm_button = if matched {
        Button::new(confirm_text).on_press(Message::KillConfirmed)
    } else {
        // No `on_press` = disabled.
        Button::new(confirm_text)
    };

    let cancel_button = Button::new(
        Text::new(KILL_CANCEL_LABEL)
            .size(text::BODY)
            .color(color::FG),
    )
    .on_press(Message::KillCancelled);

    let mut col = Column::new()
        .spacing(space::M)
        .push(
            Text::new(KILL_DIALOG_TITLE)
                .size(text::TITLE)
                .color(color::FG),
        )
        .push(
            Text::new(KILL_DIALOG_BODY)
                .size(text::BODY)
                .color(color::FG_MUTED),
        )
        .push(input);

    if !matched && !typed.is_empty() {
        col = col.push(
            Text::new(KILL_PHRASE_MISMATCH_HINT)
                .size(text::CAPTION)
                .color(color::WARN),
        );
    }

    col.push(
        Row::new()
            .push(confirm_button)
            .push(cancel_button)
            .spacing(space::M),
    )
    .into()
}

fn flattening_body<'a>() -> Element<'a, Message> {
    muted_body(KILL_DIALOG_BODY)
}

fn halted_body(_reason: &str) -> Element<'_, Message> {
    Column::new()
        .spacing(space::M)
        .push(
            Text::new(KILL_HALTED_BANNER)
                .size(text::DISPLAY)
                .color(color::NEG),
        )
        .push(
            Text::new(KILL_HALTED_HINT)
                .size(text::BODY)
                .color(color::FG_MUTED),
        )
        .push(
            Text::new(KILL_RUNBOOK_LINK_LABEL)
                .size(text::CAPTION)
                .color(color::ACCENT),
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
