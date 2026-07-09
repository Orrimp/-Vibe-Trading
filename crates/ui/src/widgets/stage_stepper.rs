//! Spine stepper — the DATA → CALIBRATE → ANALYZE → SUGGEST orientation band
//! (advisor-calibrate-stage / R3-3a, ADR-0083).
//!
//! ## What this is (and is NOT)
//!
//! A shell-chrome **orientation band** that shows the four verbs of the advisor
//! journey with a "you are here" highlight on the current stage. It is pushed at
//! the top of the shell's `centre` Column (`crate::shell::view`, above `body`),
//! so it spans every advisor-journey screen consistently (the halted-banner
//! placement precedent).
//!
//! It is **NOT a router.** The decisive IA finding (ADR-0083 § Context): the four
//! verbs do not map 1:1 to four screens — DATA (the F3 guided input + the
//! data-quality panel) and ANALYZE (the ranked bake-off table + scorecard) BOTH
//! live inside `Screen::Leaderboard`. So the highlighted stage is resolved by the
//! pure [`stage_for`] over `current_screen` + the EXISTING leaderboard panel
//! substate (`PanelState::Empty` → DATA, a `Ready` result → ANALYZE) — no new
//! state field, no navigation. Off the advisor journey the band is elided
//! (pixel-silent).
//!
//! **Zero string literals** — verb labels resolve via `crate::strings`.
//! **Zero hex colours** — tokens come from `crate::theme`.

use iced::widget::{Container, Row, Space, Text, container};
use iced::{Border, Length};

use crate::state::{PanelState, Screen};
use crate::strings::{
    SPINE_STAGE_ANALYZE, SPINE_STAGE_CALIBRATE, SPINE_STAGE_DATA, SPINE_STAGE_SUGGEST,
};
use crate::theme::{ThemeMode, color, radius, space, text};

/// The four verbs of the advisor spine. A plain UI-owned orientation type — it
/// carries no journey state (the deferred `agent::AdvisorStage` carrier, D7/D3,
/// is explicitly NOT this).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpineStage {
    /// F3 guided input (coin + budget + lookback) + the data-quality panel —
    /// the `Leaderboard` screen BEFORE a bake-off has run.
    Data,
    /// The gate-tied hyperparameter sweep editor (`Screen::Tune`).
    Calibrate,
    /// The ranked bake-off table + scorecard — the `Leaderboard` screen AFTER a
    /// bake-off has produced a `Ready` result.
    Analyze,
    /// The forward buy/sell plan (`Screen::ForwardPlan`).
    Suggest,
}

impl SpineStage {
    /// The four stages in spine (left-to-right) order.
    pub const ORDER: [SpineStage; 4] = [
        SpineStage::Data,
        SpineStage::Calibrate,
        SpineStage::Analyze,
        SpineStage::Suggest,
    ];

    /// The stage's display verb (from `crate::strings` — no inline literal).
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            SpineStage::Data => SPINE_STAGE_DATA,
            SpineStage::Calibrate => SPINE_STAGE_CALIBRATE,
            SpineStage::Analyze => SPINE_STAGE_ANALYZE,
            SpineStage::Suggest => SPINE_STAGE_SUGGEST,
        }
    }
}

/// Resolve the highlighted spine stage from the current screen + the leaderboard
/// substate (ADR-0083 D2). Pure — unit-tested exhaustively below; the render
/// test is the load-bearing proof.
///
/// | `screen`                         | result             |
/// |----------------------------------|--------------------|
/// | `Leaderboard` + `result = Empty` | `Some(Data)`       |
/// | `Leaderboard` + `result = Ready` | `Some(Analyze)`    |
/// | `Tune`                           | `Some(Calibrate)`  |
/// | `ForwardPlan`                    | `Some(Suggest)`    |
/// | anything else                    | `None` (band hidden) |
///
/// The DATA/ANALYZE discriminator reads the EXISTING
/// [`crate::leaderboard::LeaderboardScreenState::result`] `PanelState` — no new
/// state field. `Loading` / `Error` on the leaderboard resolve to `Data` (still
/// pre-analysis: the operator is on the input surface, a run has not yet
/// produced a ranked table to analyse).
#[must_use]
pub fn stage_for<T>(screen: Screen, leaderboard_result: &PanelState<T>) -> Option<SpineStage> {
    match screen {
        Screen::Leaderboard => match leaderboard_result {
            // A finished bake-off → the ranked table + scorecard IS the analysis.
            PanelState::Ready(_) => Some(SpineStage::Analyze),
            // Cold / in-flight / errored → still the DATA surface (guided input +
            // data-quality panel); no ranked result to analyse yet.
            PanelState::Empty | PanelState::Loading | PanelState::Error(_) => {
                Some(SpineStage::Data)
            }
        },
        Screen::Tune => Some(SpineStage::Calibrate),
        Screen::ForwardPlan => Some(SpineStage::Suggest),
        // Every other screen (Lab, Live, Compare, Baseline, Strategies, Memory,
        // Models, Reports, Trail, Settings + deprecated aliases) is off the
        // advisor journey → no band.
        _ => None,
    }
}

/// Render the spine-stepper band.
///
/// `current` is the highlighted stage (from [`stage_for`]); `None` elides the
/// band to a pixel-silent zero-height `Space` (the operator is off the advisor
/// journey). The active segment paints a SOLID `ACCENT` fill with
/// `FG_ON_ACCENT` text (the leaderboard active-chip idiom — a strong, legible
/// "you are here"); the rest paint a `PANEL_RAISED` chip with `FG_2` text. A
/// `›` chevron between segments reads the left-to-right flow.
///
/// Colour is never the ONLY signal (accessibility): the active segment ALSO
/// carries a leading `●` marker, so the current stage is legible without hue.
// `cast_possible_truncation`: space::* constants are bounded u32; the u16
// padding cast is safe.
#[allow(clippy::cast_possible_truncation)]
#[must_use]
pub fn view<'a>(current: Option<SpineStage>, mode: ThemeMode) -> crate::Element<'a> {
    let Some(current) = current else {
        // Off-journey — elide. A zero-sized Space keeps the band pixel-silent
        // (the shell Column simply has nothing to lay out here).
        return Space::new()
            .width(Length::Fixed(0.0))
            .height(Length::Fixed(0.0))
            .into();
    };

    let mut row = Row::new()
        .spacing(space::S)
        .align_y(iced::alignment::Vertical::Center);

    for (idx, stage) in SpineStage::ORDER.iter().enumerate() {
        if idx > 0 {
            // Flow chevron between segments — FG_3 muted, non-interactive.
            row = row.push(
                Text::new(SPINE_STEPPER_SEPARATOR)
                    .size(text::SMALL)
                    .color(color::FG_3.current(mode)),
            );
        }
        row = row.push(segment(*stage, *stage == current, mode));
    }

    // The band: the segment row on a PANEL surface with a 1px BORDER_1 bottom
    // hairline feel (via the container border), padded to compact operator
    // density. Full width so it spans the centre column.
    Container::new(row)
        .width(Length::Fill)
        .padding([space::S as u16, space::L as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R2.into(),
            },
            ..Default::default()
        })
        .into()
}

/// One stepper segment. Active → solid `ACCENT` fill + `FG_ON_ACCENT` text +
/// a leading `●` marker (shape signal, not colour-only). Inactive →
/// `PANEL_RAISED` chip + `FG_2` text.
// `cast_possible_truncation`: bounded space constants → safe u16 padding.
#[allow(clippy::cast_possible_truncation)]
fn segment<'a>(stage: SpineStage, active: bool, mode: ThemeMode) -> crate::Element<'a> {
    let (bg, fg, label) = if active {
        (
            color::ACCENT.current(mode),
            color::FG_ON_ACCENT.current(mode),
            // Leading marker makes the active stage legible without relying on
            // hue (accessibility: colour is never the only signal).
            format!("{SPINE_STEPPER_ACTIVE_MARKER}{}", stage.label()),
        )
    } else {
        (
            color::PANEL_RAISED.current(mode),
            color::FG_2.current(mode),
            stage.label().to_string(),
        )
    };

    Container::new(Text::new(label).size(text::SMALL).color(fg))
        .padding([space::XS as u16, space::M as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg.into()),
            border: Border {
                color: bg,
                width: 1.0,
                radius: radius::R2.into(),
            },
            text_color: Some(fg),
            ..Default::default()
        })
        .into()
}

/// Between-segment flow chevron (`›`). A pure glyph token — the ONE display
/// literal that is a structural separator, not user copy; kept module-local
/// beside the widget (matching the sidebar hairline-divider idiom which also
/// builds its separator inline).
const SPINE_STEPPER_SEPARATOR: &str = "›";

/// Leading marker on the active segment — a shape signal so the current stage
/// is legible without colour (accessibility minimum).
const SPINE_STEPPER_ACTIVE_MARKER: &str = "● ";

#[cfg(test)]
mod tests {
    use super::*;

    // ── stage_for — every row of the ADR-0083 D2 table (T12) ─────────────────

    #[test]
    fn leaderboard_empty_maps_to_data() {
        assert_eq!(
            stage_for(Screen::Leaderboard, &PanelState::<()>::Empty),
            Some(SpineStage::Data),
        );
    }

    #[test]
    fn leaderboard_ready_maps_to_analyze() {
        assert_eq!(
            stage_for(Screen::Leaderboard, &PanelState::Ready(())),
            Some(SpineStage::Analyze),
        );
    }

    #[test]
    fn leaderboard_loading_maps_to_data() {
        // In-flight bake-off — still the DATA surface (no ranked result yet).
        assert_eq!(
            stage_for(Screen::Leaderboard, &PanelState::<()>::Loading),
            Some(SpineStage::Data),
        );
    }

    #[test]
    fn leaderboard_error_maps_to_data() {
        // A failed run leaves the operator on the input surface → DATA.
        assert_eq!(
            stage_for(
                Screen::Leaderboard,
                &PanelState::<()>::Error(smol_str::SmolStr::new("boom")),
            ),
            Some(SpineStage::Data),
        );
    }

    #[test]
    fn tune_maps_to_calibrate() {
        // The leaderboard substate is irrelevant off the leaderboard.
        assert_eq!(
            stage_for(Screen::Tune, &PanelState::Ready(())),
            Some(SpineStage::Calibrate),
        );
        assert_eq!(
            stage_for(Screen::Tune, &PanelState::<()>::Empty),
            Some(SpineStage::Calibrate),
        );
    }

    #[test]
    fn forward_plan_maps_to_suggest() {
        assert_eq!(
            stage_for(Screen::ForwardPlan, &PanelState::<()>::Empty),
            Some(SpineStage::Suggest),
        );
    }

    #[test]
    #[allow(deprecated)]
    fn non_journey_screens_map_to_none() {
        // Every non-journey screen elides the band, regardless of the
        // leaderboard substate.
        for screen in [
            Screen::Lab,
            Screen::Live,
            Screen::Compare,
            Screen::Baseline,
            Screen::Strategies,
            Screen::Memory,
            Screen::Models,
            Screen::Reports,
            Screen::Trail,
            Screen::Settings,
            // deprecated aliases
            Screen::Home,
            Screen::Charts,
            Screen::Audit,
            Screen::Risk,
            Screen::Debug,
            Screen::Control,
        ] {
            assert_eq!(
                stage_for(screen, &PanelState::Ready(())),
                None,
                "screen {screen:?} must elide the spine band",
            );
        }
    }

    // ── SpineStage helpers ───────────────────────────────────────────────────

    #[test]
    fn order_is_the_spine_left_to_right() {
        assert_eq!(
            SpineStage::ORDER,
            [
                SpineStage::Data,
                SpineStage::Calibrate,
                SpineStage::Analyze,
                SpineStage::Suggest,
            ],
        );
    }

    #[test]
    fn every_stage_has_a_non_empty_label() {
        for stage in SpineStage::ORDER {
            assert!(!stage.label().is_empty(), "empty label for stage {stage:?}",);
        }
    }
}
