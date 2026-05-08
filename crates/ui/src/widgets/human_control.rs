//! `HumanControl` panel — Phase 5 (T1904 / T1905 / T1906 / T1911).
//!
//! The `HumanControl` panel is the **first net-new operator-write
//! surface** in the Lumen rewrite. It bundles four sub-blocks under
//! one Tier-1 `frame::panel` chrome:
//!
//! 1. **Mode segmented control** (Observe / Supervised / Auto — T1911).
//! 2. **Three mirror rows** — Daily-loss limit / Max-position /
//!    Used-today (T1905).
//! 3. **Kill action** — bottom action via `widgets::kill::view_inner`
//!    (T1906 / R2.1).
//!
//! Per Phase 5 Design (Q1 ratification), the panel is rendered as the
//! 7th sidebar entry `Screen::Control` (NOT a Home-screen card).
//!
//! Per Phase 5 Q12 ratification, the kill button copy stays
//! `KILL_BUTTON_LABEL = "Stop trading"` — Lumen's `"Halt all agents"`
//! is **not** adopted (operator-locked Master Constraint 2).
//!
//! All net-new copy lives in `crate::strings::HUMAN_CONTROL_*` and
//! `crate::strings::EXECUTION_MODE_*` — no inline strings (consistency
//! gate).

// `cast_possible_truncation`: `space::*` constants are u32 with bounded
// values 0..64; the cast to u16 padding is safe by construction.
#![allow(clippy::cast_possible_truncation)]

use iced::widget::{button, Button, Column, Row, Text};
use iced::{Border, Element};
use rust_decimal::Decimal;
use rust_decimal_macros::dec;

use crate::state::{Cockpit, ExecutionMode, Message, PanelState};
use crate::strings::{
    EXECUTION_MODE_AUTO_HINT, EXECUTION_MODE_AUTO_LABEL, EXECUTION_MODE_OBSERVE_HINT,
    EXECUTION_MODE_OBSERVE_LABEL, EXECUTION_MODE_SUPERVISED_HINT, EXECUTION_MODE_SUPERVISED_LABEL,
    HUMAN_CONTROL_DAILY_LOSS_LABEL, HUMAN_CONTROL_LIMITS_UNAVAILABLE,
    HUMAN_CONTROL_MAX_POSITION_LABEL, HUMAN_CONTROL_USED_TODAY_LABEL, PANEL_HUMAN_CONTROL_TITLE,
    PLACEHOLDER_NONE,
};
use crate::theme::{color, color_for_delta, radius, space, text, ThemeMode};

use super::focus_ring;
use super::frame::{muted_body, panel};
use super::kill;
use super::num::{fmt_pct, fmt_usdt_signed};

/// Render the `HumanControl` panel. The panel is a Tier-1
/// `frame::panel(PANEL_HUMAN_CONTROL_TITLE, body, ThemeMode::Dark)`
/// wrapping the four sub-blocks top-to-bottom: mode segment + three
/// mirror rows + kill bottom action.
#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let body = Column::new()
        .spacing(space::M)
        .push(mode_segment(
            model.execution_mode,
            model.focused_widget.as_deref(),
        ))
        .push(mode_hint(model.execution_mode))
        .push(limit_rows(model))
        .push(kill::view_inner(model));
    panel(PANEL_HUMAN_CONTROL_TITLE, body.into(), ThemeMode::Dark)
}

// ── Mode segmented control (T1911) ──────────────────────────────────────────

/// Three-button segmented control for the execution mode (R9.2). Active
/// variant uses the Phase 1 active-row pattern (background
/// `PANEL_RAISED`, border `ACCENT @ 1px`); inactive variants render as
/// flat panel buttons.
#[must_use]
pub fn mode_segment<'a>(active: ExecutionMode, focused: Option<&str>) -> Element<'a, Message> {
    let observe = mode_button(
        EXECUTION_MODE_OBSERVE_LABEL,
        ExecutionMode::Observe,
        active,
        focus_ring::EXECUTION_MODE_OBSERVE,
        focused,
    );
    let supervised = mode_button(
        EXECUTION_MODE_SUPERVISED_LABEL,
        ExecutionMode::Supervised,
        active,
        focus_ring::EXECUTION_MODE_SUPERVISED,
        focused,
    );
    let auto = mode_button(
        EXECUTION_MODE_AUTO_LABEL,
        ExecutionMode::Auto,
        active,
        focus_ring::EXECUTION_MODE_AUTO,
        focused,
    );
    Row::new()
        .spacing(space::S)
        .push(observe)
        .push(supervised)
        .push(auto)
        .into()
}

fn mode_button<'a>(
    label: &'a str,
    target: ExecutionMode,
    active: ExecutionMode,
    focus_id: &'a str,
    focused: Option<&str>,
) -> Element<'a, Message> {
    let mode = ThemeMode::Dark;
    let is_active = target == active;
    let btn = Button::new(
        Text::new(label)
            .size(text::BODY)
            .color(color::FG_1.current(mode)),
    )
    .on_press(Message::ExecutionModeSelected(target))
    .padding([space::XS as u16, space::M as u16])
    .style(move |_theme: &iced::Theme, _status: button::Status| {
        // Phase 1 active-row pattern: PANEL_RAISED background +
        // ACCENT border @ 1 px when active; flat panel surface when
        // inactive.
        let (bg, border_color) = if is_active {
            (
                color::PANEL_RAISED.current(mode),
                color::ACCENT.current(mode),
            )
        } else {
            (color::PANEL.current(mode), color::BORDER_2.current(mode))
        };
        button::Style {
            background: Some(bg.into()),
            text_color: color::FG_1.current(mode),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: radius::R2.into(),
            },
            ..Default::default()
        }
    });
    focus_ring::wrap(focus_id, btn.into(), focused == Some(focus_id), mode)
}

fn mode_hint<'a>(mode: ExecutionMode) -> Element<'a, Message> {
    let hint = match mode {
        ExecutionMode::Observe => EXECUTION_MODE_OBSERVE_HINT,
        ExecutionMode::Supervised => EXECUTION_MODE_SUPERVISED_HINT,
        ExecutionMode::Auto => EXECUTION_MODE_AUTO_HINT,
    };
    muted_body(hint)
}

// ── Mirror rows (T1905) ─────────────────────────────────────────────────────

/// Render the three mirror rows (Daily-loss / Max-position /
/// Used-today). Reads `risk_state` for the first two rows and
/// `Cockpit::pnl` for the third (sign-coloured via
/// `theme::color_for_delta`). Loading + Error states render placeholder
/// dashes / `HUMAN_CONTROL_LIMITS_UNAVAILABLE` per R3.4.
fn limit_rows(model: &Cockpit) -> Element<'_, Message> {
    match &model.risk_state {
        PanelState::Loading | PanelState::Empty => Column::new()
            .spacing(space::XS)
            .push(limit_row(
                HUMAN_CONTROL_DAILY_LOSS_LABEL,
                PLACEHOLDER_NONE.to_string(),
                None,
            ))
            .push(limit_row(
                HUMAN_CONTROL_MAX_POSITION_LABEL,
                PLACEHOLDER_NONE.to_string(),
                None,
            ))
            .push(limit_row(
                HUMAN_CONTROL_USED_TODAY_LABEL,
                PLACEHOLDER_NONE.to_string(),
                None,
            ))
            .into(),
        PanelState::Error(_) => muted_body(HUMAN_CONTROL_LIMITS_UNAVAILABLE),
        PanelState::Ready(rs) => {
            let daily_value = fmt_pct(rs.daily_loss_cap_pct);
            let max_position_value = max_position_value(rs);
            let used_today_value = used_today_value(model);
            let used_today_color = used_today_sentiment(model);
            Column::new()
                .spacing(space::XS)
                .push(limit_row(HUMAN_CONTROL_DAILY_LOSS_LABEL, daily_value, None))
                .push(limit_row(
                    HUMAN_CONTROL_MAX_POSITION_LABEL,
                    max_position_value,
                    None,
                ))
                .push(limit_row(
                    HUMAN_CONTROL_USED_TODAY_LABEL,
                    used_today_value,
                    used_today_color,
                ))
                .into()
        }
    }
}

fn limit_row(label: &str, value: String, sentiment: Option<iced::Color>) -> Element<'_, Message> {
    let mode = ThemeMode::Dark;
    let value_color = sentiment.unwrap_or_else(|| color::FG_1.current(mode));
    Row::new()
        .spacing(space::M)
        .push(
            Text::new(label)
                .size(text::SMALL)
                .color(color::FG_3.current(mode)),
        )
        .push(Text::new(value).size(text::BODY).color(value_color))
        .into()
}

fn max_position_value(rs: &crate::state::RiskState) -> String {
    // Largest per-symbol cap — single-line summary suitable for the
    // mirror row. Empty caps map to PLACEHOLDER_NONE.
    let max_cap = rs
        .per_symbol_caps
        .values()
        .copied()
        .fold(Decimal::ZERO, Decimal::max);
    if max_cap == dec!(0) {
        PLACEHOLDER_NONE.to_string()
    } else {
        fmt_pct(max_cap)
    }
}

fn used_today_value(model: &Cockpit) -> String {
    match &model.pnl {
        PanelState::Ready(snap) => fmt_usdt_signed(snap.daily_return.amount()),
        _ => PLACEHOLDER_NONE.to_string(),
    }
}

fn used_today_sentiment(model: &Cockpit) -> Option<iced::Color> {
    match &model.pnl {
        PanelState::Ready(snap) => Some(color_for_delta(snap.daily_return.amount())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::Cockpit;

    #[test]
    fn human_control_renders_with_default_cockpit() {
        // Smoke test: the panel renders without panicking on a fresh
        // cockpit (all panels Loading; ExecutionMode::Observe default).
        let c = Cockpit::new();
        let _: Element<'_, Message> = view(&c);
    }

    #[test]
    fn human_control_limits_render_loading_state() {
        // Loading state renders three muted dashes (no risk state yet).
        let c = Cockpit::new();
        let _: Element<'_, Message> = limit_rows(&c);
    }

    #[test]
    fn mode_segment_renders_active_supervised() {
        let _: Element<'_, Message> = mode_segment(ExecutionMode::Supervised, None);
    }

    #[test]
    fn used_today_sentiment_neutral_when_pnl_loading() {
        let c = Cockpit::new();
        assert!(used_today_sentiment(&c).is_none());
    }
}
