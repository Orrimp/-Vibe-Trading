//! Latency badge — venue ts vs local ts, color-coded per R6.2.
//!
//! Thresholds live in `theme::latency`; color logic in
//! `theme::color_for_latency_ms`. This module renders the label + value.

use iced::widget::{Column, Row, Text};
use iced::Element;

use crate::state::{AgentMode, Cockpit, Latency, Message};
use crate::strings::{
    LATENCY_HALTED_LABEL, LATENCY_HELP, LATENCY_HIGH_LABEL, LATENCY_OK_LABEL, LATENCY_UNIT_MS,
    LATENCY_UNKNOWN, LATENCY_WARN_LABEL, PANEL_LATENCY_TITLE,
};
use crate::theme::{color, color_for_latency_ms, latency as lat, space, text, ThemeMode};

use super::frame::panel;

/// Badge classification. Public so tests can assert the threshold->label
/// mapping directly without poking at rendered widgets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    Unknown,
    Ok,
    Warn,
    High,
    Halted,
}

impl Badge {
    /// Classify a latency reading against the R6.2 thresholds. Distinct
    /// from `theme::color_for_latency_ms` so widget tests can label-check
    /// without re-implementing the color table.
    #[must_use]
    pub fn classify(ms: i64) -> Self {
        if ms >= lat::HALTED_MS {
            Badge::Halted
        } else if ms >= lat::WARN_MS {
            Badge::High
        } else if ms >= lat::OK_MS {
            Badge::Warn
        } else {
            Badge::Ok
        }
    }

    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Badge::Unknown => LATENCY_UNKNOWN,
            Badge::Ok => LATENCY_OK_LABEL,
            Badge::Warn => LATENCY_WARN_LABEL,
            Badge::High => LATENCY_HIGH_LABEL,
            Badge::Halted => LATENCY_HALTED_LABEL,
        }
    }

    #[must_use]
    pub fn color(self) -> iced::Color {
        match self {
            Badge::Unknown => color::FG_3.current(ThemeMode::Dark),
            Badge::Ok => color::UP_500.current(ThemeMode::Dark),
            Badge::Warn => color::WARN_500.current(ThemeMode::Dark),
            // High and Halted share red by design — the label distinguishes.
            Badge::High | Badge::Halted => color::DOWN_500.current(ThemeMode::Dark),
        }
    }
}

/// Render the latency panel.
#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let (badge, value_text) = match (model.latency, model.mode) {
        (_, AgentMode::Halted) => (Badge::Halted, LATENCY_UNKNOWN.to_string()),
        (Latency::Unknown, _) => (Badge::Unknown, LATENCY_UNKNOWN.to_string()),
        (Latency::Known { ms }, _) => (Badge::classify(ms), format!("{ms} {LATENCY_UNIT_MS}")),
    };

    let label_color = color_for_latency_ms(match model.latency {
        Latency::Unknown => 0,
        Latency::Known { ms } => ms,
    });

    let label = Text::new(badge.label())
        .size(text::H2)
        .color(if matches!(badge, Badge::Unknown) {
            color::FG_3.current(ThemeMode::Dark)
        } else if matches!(badge, Badge::Halted) {
            color::DOWN_500.current(ThemeMode::Dark)
        } else {
            label_color
        });
    let value = Text::new(value_text)
        .size(text::BODY)
        .color(color::FG_1.current(ThemeMode::Dark));

    let body = Column::new()
        .spacing(space::S)
        .push(Row::new().push(label).push(value).spacing(space::M))
        .push(
            Text::new(LATENCY_HELP)
                .size(text::MICRO)
                .color(color::FG_3.current(ThemeMode::Dark)),
        );

    panel(PANEL_LATENCY_TITLE, body.into(), ThemeMode::Dark)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thresholds_match_r6_2() {
        assert_eq!(Badge::classify(0), Badge::Ok);
        assert_eq!(Badge::classify(499), Badge::Ok);
        assert_eq!(Badge::classify(500), Badge::Warn);
        assert_eq!(Badge::classify(1_999), Badge::Warn);
        assert_eq!(Badge::classify(2_000), Badge::High);
        assert_eq!(Badge::classify(9_999), Badge::High);
        assert_eq!(Badge::classify(10_000), Badge::Halted);
        assert_eq!(Badge::classify(30_000), Badge::Halted);
    }

    #[test]
    fn label_colors_never_coincide() {
        let cs = [Badge::Ok.color(), Badge::Warn.color(), Badge::High.color()];
        // High and Halted share red by design; ok / warn / high are distinct.
        assert_ne!(cs[0], cs[1]);
        assert_ne!(cs[0], cs[2]);
    }
}
