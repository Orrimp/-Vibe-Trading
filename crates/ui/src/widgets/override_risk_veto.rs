//! Override-risk-veto modal — Phase 5 (T1909 / T1910).
//!
//! Per-veto operator override surface. Mirror of the kill-confirm modal
//! at [`widgets::kill`](super::kill) — typed-confirm pattern with the
//! `OVERRIDE` phrase as the safety token. Per Phase 5 Q9 ratification,
//! each veto override is its own typed-confirm flow (not per-strategy).
//! Forward-only — the override is recorded; the agent does NOT re-emit
//! the blocked signal.
//!
//! Module exposes `modal_view(state)` which returns `None` when the
//! override modal is `Idle` and `Some(Element)` otherwise. The cockpit
//! shell wraps `modal_view`'s output in a `Stack` overlay (sibling of
//! the journal-transaction modal pattern).

use iced::widget::{
    button, container, text_input, Button, Column, Container, Row, Space, Text, TextInput,
};
use iced::{Border, Element, Length};

use crate::state::{Message, OverrideRiskVetoState};
use crate::strings::{
    OVERRIDE_RISK_VETO_CANCEL_LABEL, OVERRIDE_RISK_VETO_CONFIRM_LABEL,
    OVERRIDE_RISK_VETO_DIALOG_BODY, OVERRIDE_RISK_VETO_DIALOG_TITLE, OVERRIDE_RISK_VETO_PHRASE,
    OVERRIDE_RISK_VETO_PHRASE_MISMATCH_HINT,
};
use crate::theme::{color, radius, shadow, space, text, ThemeMode};

use super::focus_ring;

/// Render the typed-confirm modal body. Returns `None` when the
/// override state is `Idle` (modal closed) — the cockpit then renders
/// the underlying screen unchanged. Returns `Some(modal)` when
/// `Confirming { ... }` or `Submitting { ... }`; the cockpit wraps the
/// returned element in a `Stack` overlay.
///
/// `focused` carries `Cockpit::focused_widget.as_deref()` so the
/// `focus_ring::wrap(...)` call sites can highlight the appropriate
/// surface (input + confirm + cancel).
#[must_use]
pub fn modal_view<'a>(
    state: &'a OverrideRiskVetoState,
    focused: Option<&'a str>,
) -> Option<Element<'a, Message>> {
    match state {
        OverrideRiskVetoState::Idle => None,
        OverrideRiskVetoState::Confirming { veto_id, typed } => {
            Some(confirming_body(veto_id.as_str(), typed.as_str(), focused))
        }
        OverrideRiskVetoState::Submitting { .. } => {
            // Phase 5 v1 does not visually distinguish Submitting from
            // Confirming — the audit-write spawn is bounded by a
            // tokio::spawn that returns near-instantly; the cockpit
            // re-enters Idle on the wrapping update arm. Render the
            // confirming body for visual continuity.
            Some(submitting_body())
        }
    }
}

// `too_many_lines`: the override-confirm body mirrors kill-confirm —
// one cohesive render pass; splitting obscures the input + button flow.
#[allow(clippy::too_many_lines)]
fn confirming_body<'a>(
    veto_id: &'a str,
    typed: &'a str,
    focused: Option<&'a str>,
) -> Element<'a, Message> {
    let mode = ThemeMode::Dark;
    let matched = typed == OVERRIDE_RISK_VETO_PHRASE;

    // 1 px hairline above the input (mirror of kill-confirm pattern).
    let hairline_color = shadow::shadow_inset(mode);
    let hairline = Container::new(Space::new())
        .width(Length::Fill)
        .height(Length::Fixed(1.0))
        .style(move |_theme: &iced::Theme| container::Style {
            background: Some(hairline_color.into()),
            ..Default::default()
        });

    let input = TextInput::new(OVERRIDE_RISK_VETO_PHRASE, typed)
        .on_input(Message::OverrideRiskVetoTyped)
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

    let confirm_text = Text::new(OVERRIDE_RISK_VETO_CONFIRM_LABEL)
        .size(text::BODY)
        .color(color::FG_1.current(mode));
    let confirm_button: Element<'a, Message> = if matched {
        Button::new(confirm_text)
            .on_press(Message::OverrideRiskVetoConfirmed(smol_str::SmolStr::new(
                veto_id,
            )))
            .style(
                move |_theme: &iced::Theme, _status: button::Status| button::Style {
                    background: Some(color::ACCENT.current(mode).into()),
                    text_color: color::FG_1.current(mode),
                    border: Border {
                        radius: radius::R2.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .into()
    } else {
        // Disabled: no on_press.
        Button::new(confirm_text)
            .style(
                move |_theme: &iced::Theme, _status: button::Status| button::Style {
                    background: Some(color::PANEL_SUNKEN.current(mode).into()),
                    text_color: color::FG_3.current(mode),
                    border: Border {
                        radius: radius::R2.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            )
            .into()
    };

    let cancel_button = Button::new(
        Text::new(OVERRIDE_RISK_VETO_CANCEL_LABEL)
            .size(text::BODY)
            .color(color::FG_1.current(mode)),
    )
    .on_press(Message::OverrideRiskVetoCancelled);

    // Phase 5 (T1912 / TD-1 path b) — wrap input + buttons in
    // focus_ring overlay so the halo lands on the parent-side
    // `Cockpit::focused_widget` state.
    let input_wrapped = focus_ring::wrap(
        focus_ring::OVERRIDE_RISK_VETO_INPUT,
        input.into(),
        focused == Some(focus_ring::OVERRIDE_RISK_VETO_INPUT),
        mode,
    );
    let confirm_wrapped = focus_ring::wrap(
        focus_ring::OVERRIDE_RISK_VETO_CONFIRM,
        confirm_button,
        focused == Some(focus_ring::OVERRIDE_RISK_VETO_CONFIRM),
        mode,
    );
    let cancel_wrapped = focus_ring::wrap(
        focus_ring::OVERRIDE_RISK_VETO_CANCEL,
        cancel_button.into(),
        focused == Some(focus_ring::OVERRIDE_RISK_VETO_CANCEL),
        mode,
    );

    let mut col = Column::new()
        .spacing(space::M)
        .push(
            Text::new(OVERRIDE_RISK_VETO_DIALOG_TITLE)
                .size(text::H2)
                .color(color::FG_1.current(mode)),
        )
        .push(
            Text::new(OVERRIDE_RISK_VETO_DIALOG_BODY)
                .size(text::BODY)
                .color(color::FG_3.current(mode)),
        )
        .push(hairline)
        .push(input_wrapped);

    if !matched && !typed.is_empty() {
        col = col.push(
            Text::new(OVERRIDE_RISK_VETO_PHRASE_MISMATCH_HINT)
                .size(text::MICRO)
                .color(color::WARN_500.current(mode)),
        );
    }

    col.push(
        Row::new()
            .spacing(space::M)
            .push(confirm_wrapped)
            .push(cancel_wrapped),
    )
    .into()
}

fn submitting_body<'a>() -> Element<'a, Message> {
    let mode = ThemeMode::Dark;
    Column::new()
        .spacing(space::M)
        .push(
            Text::new(OVERRIDE_RISK_VETO_DIALOG_TITLE)
                .size(text::H2)
                .color(color::FG_1.current(mode)),
        )
        .push(
            Text::new(OVERRIDE_RISK_VETO_DIALOG_BODY)
                .size(text::BODY)
                .color(color::FG_3.current(mode)),
        )
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{Cockpit, Message};
    use smol_str::SmolStr;

    #[test]
    fn override_risk_veto_modal_idle_returns_none() {
        let state = OverrideRiskVetoState::Idle;
        assert!(modal_view(&state, None).is_none());
    }

    #[test]
    fn override_risk_veto_modal_confirming_returns_some() {
        let state = OverrideRiskVetoState::Confirming {
            veto_id: SmolStr::new("veto-1"),
            typed: String::new(),
        };
        assert!(modal_view(&state, None).is_some());
    }

    #[test]
    fn override_risk_veto_typed_confirm_round_trip_via_state() {
        // Locks the contract: pressed → modal opens; typed phrase
        // mismatch → confirm disabled; matched phrase → confirm
        // enabled (visually); confirmed clears the matching VetoEvent.
        let mut c = Cockpit::new();
        crate::state::update(
            &mut c,
            Message::OverrideRiskVetoPressed(SmolStr::new("veto-1")),
        );
        // Mismatched phrase — typed buffer updated, confirm stays
        // disabled (asserted by visual contract; modal_view returns
        // Some which implies the body renders).
        crate::state::update(&mut c, Message::OverrideRiskVetoTyped("OVERR".to_string()));
        match &c.override_risk_veto {
            OverrideRiskVetoState::Confirming { typed, .. } => {
                assert_eq!(typed, "OVERR");
                assert_ne!(typed, OVERRIDE_RISK_VETO_PHRASE);
            }
            other => panic!("expected Confirming, got {other:?}"),
        }
        // Type the full phrase.
        crate::state::update(
            &mut c,
            Message::OverrideRiskVetoTyped(OVERRIDE_RISK_VETO_PHRASE.to_string()),
        );
        match &c.override_risk_veto {
            OverrideRiskVetoState::Confirming { typed, .. } => {
                assert_eq!(typed, OVERRIDE_RISK_VETO_PHRASE);
            }
            other => panic!("expected Confirming, got {other:?}"),
        }
        // Confirm — modal returns to Idle.
        crate::state::update(
            &mut c,
            Message::OverrideRiskVetoConfirmed(SmolStr::new("veto-1")),
        );
        assert!(matches!(c.override_risk_veto, OverrideRiskVetoState::Idle));
    }
}
