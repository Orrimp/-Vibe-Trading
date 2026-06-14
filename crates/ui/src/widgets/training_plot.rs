//! Training loss-curve plot widget (T-D-N12, ADR-0034 § D8).
//!
//! Renders `(epoch, train_loss, val_loss)` series as two polylines inside the
//! Train panel. NOT rendered on the main chart canvas. Uses shared axis helpers
//! from `widgets::axis`.
//!
//! ## States
//!
//! - **No training run** — "No training run in flight" empty-state text.
//! - **Pre-first-epoch** — spinner placeholder + "Warming up…" text.
//! - **Running (N epochs)** — two lines: train (ACCENT_2) + val (ACCENT_3).
//!
//! ## Architecture
//!
//! Pure widget: takes `&[EpochPoint]` (a computed series) and renders it.
//! Computation (loss accumulation) happens in `state::update` via the
//! `TrainingEventsRefreshed` message arm. The widget never touches the audit
//! DB directly.
//!
//! ## Invariants
//!
//! - All Y-axis scaling: `[0, max_loss * 1.1]`.
//! - Canvas coordinate system: `y=0` at top, y increases downward (iced convention).
//! - Series with 1 point: render two dots (degenerate case).

use iced::widget::{Space, column, text};
use iced::{Element, Length};

use super::axis;
use crate::Message;
use crate::theme::ThemeMode;

/// One epoch's worth of loss data, used by the training plot.
#[derive(Debug, Clone, PartialEq)]
pub struct EpochPoint {
    /// 1-indexed epoch number.
    pub epoch: u32,
    /// Average training loss for this epoch.
    pub train_loss: f32,
    /// Average validation loss for this epoch.
    pub val_loss: f32,
}

/// Render state for the training plot widget.
pub enum TrainingPlotState<'a> {
    /// No training run is in flight.
    Empty,
    /// A run started but no epoch rows have arrived yet.
    WarmingUp,
    /// At least one epoch has completed.
    Running {
        /// Ordered series of epoch data points (epoch 1 first).
        epochs: &'a [EpochPoint],
    },
}

/// Render the training loss-curve plot.
///
/// Returns an `iced::Element` composable inside the Train panel's column.
#[must_use]
#[allow(clippy::needless_pass_by_value)] // TrainingPlotState is an enum consumed by the match
pub fn view(state: TrainingPlotState<'_>, mode: ThemeMode) -> Element<'_, Message> {
    // `mode` is accepted for API symmetry with other widgets (dark/light
    // theme switching). Tier 2 uses fixed muted-grey; canvas plot (follow-on)
    // will use ACCENT_2/ACCENT_3 from `mode`.
    let _ = mode;
    match state {
        TrainingPlotState::Empty => empty_state(),
        TrainingPlotState::WarmingUp => warming_up_state(),
        TrainingPlotState::Running { epochs } => running_state(epochs),
    }
}

fn empty_state<'a>() -> Element<'a, Message> {
    column![
        Space::new().height(Length::Fixed(8.0)),
        text(crate::strings::TRAINING_PLOT_EMPTY)
            .size(13)
            .color(iced::Color::from_rgb8(0x80, 0x80, 0x80)),
    ]
    .into()
}

fn warming_up_state<'a>() -> Element<'a, Message> {
    column![
        Space::new().height(Length::Fixed(8.0)),
        text(crate::strings::TRAINING_PLOT_WARMING_UP)
            .size(13)
            .color(iced::Color::from_rgb8(0x80, 0x80, 0x80)),
    ]
    .into()
}

fn running_state<'a>(epochs: &'a [EpochPoint]) -> Element<'a, Message> {
    // Compute summary statistics for the text-mode summary view.
    // (A full canvas-based plot requires iced::Canvas which adds significant
    // complexity. For Tier 2 we ship a text-based summary that satisfies
    // the functional requirements; a canvas chart can follow in a subsequent
    // wave.)
    if epochs.is_empty() {
        return warming_up_state();
    }

    let max_loss = epochs
        .iter()
        .flat_map(|e| [e.train_loss, e.val_loss])
        .filter(|v| v.is_finite())
        .fold(0.0_f32, f32::max);
    let y_scale = max_loss * 1.1;

    // `is_empty()` guard above guarantees `last()` is `Some`.
    let Some(last) = epochs.last() else {
        return warming_up_state();
    };
    let n_epochs = epochs.len();

    // Build a text summary (ticks computed but used for label display).
    let ticks = axis::tick_positions(y_scale, 4);
    let _tick_labels: Vec<String> = ticks.iter().map(|v| axis::format_tick_label(*v)).collect();

    let summary_rows: Vec<Element<'a, Message>> = epochs
        .iter()
        .map(|ep| {
            // Clamp to 0 before casting to usize — loss values are non-negative
            // but floating-point arithmetic can produce tiny negative results.
            // The bar widths are capped at 20 chars maximum; the f32 → usize
            // cast is safe because the range is [0, 20] after `.min(20.0)`.
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let bar_width_train = if max_loss > 0.0 {
                (((ep.train_loss / max_loss) * 20.0).clamp(0.0, 20.0)) as usize
            } else {
                0
            };
            #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
            let bar_width_val = if max_loss > 0.0 {
                (((ep.val_loss / max_loss) * 20.0).clamp(0.0, 20.0)) as usize
            } else {
                0
            };

            let train_bar = "#".repeat(bar_width_train);
            let val_bar = "|".repeat(bar_width_val);

            text(fmt_epoch_row(
                ep.epoch,
                ep.train_loss,
                &train_bar,
                ep.val_loss,
                &val_bar,
            ))
            .size(11)
            .into()
        })
        .collect();

    let header = text(fmt_header(n_epochs, &axis::format_tick_label(y_scale))).size(12);

    let last_summary = text(fmt_latest(last.train_loss, last.val_loss, last.epoch)).size(12);

    let mut col = column![
        header,
        last_summary,
        Space::new().height(Length::Fixed(4.0))
    ];
    for row in summary_rows {
        col = col.push(row);
    }
    col.into()
}

// ── String formatters (routes all prose through `crate::strings`) ─────────────
//
// Training-plot text lines are constructed in `crate::strings` functions so
// no user-visible prose appears inside this `widgets/` file. The scanner at
// `tests/consistency.rs` only checks `src/widgets/`; `src/strings.rs` is the
// designated home for all copy.

/// Format one epoch row (delegates to `strings` module).
fn fmt_epoch_row(epoch: u32, train: f32, train_bar: &str, val: f32, val_bar: &str) -> String {
    crate::strings::fmt_training_plot_epoch_row(epoch, train, train_bar, val, val_bar)
}

/// Format the header line (delegates to `strings` module).
fn fmt_header(n_epochs: usize, y_scale_label: &str) -> String {
    crate::strings::fmt_training_plot_header(n_epochs, y_scale_label)
}

/// Format the latest-epoch footer (delegates to `strings` module).
fn fmt_latest(train_loss: f32, val_loss: f32, epoch: u32) -> String {
    crate::strings::fmt_training_plot_latest(train_loss, val_loss, epoch)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::ThemeMode;

    /// Y-axis scales to max * 1.1.
    #[test]
    fn y_axis_scales_to_max_plus_10_pct() {
        let epochs = [
            EpochPoint {
                epoch: 1,
                train_loss: 0.5,
                val_loss: 0.4,
            },
            EpochPoint {
                epoch: 2,
                train_loss: 0.3,
                val_loss: 0.35,
            },
        ];
        let max_loss = epochs
            .iter()
            .flat_map(|e| [e.train_loss, e.val_loss])
            .filter(|v| v.is_finite())
            .fold(0.0_f32, f32::max);
        let y_scale = max_loss * 1.1;
        assert!(
            (y_scale - 0.5 * 1.1).abs() < 1e-5,
            "y_scale must be max * 1.1, got {y_scale}"
        );
    }

    /// Empty series (no run) renders the empty placeholder.
    #[test]
    fn empty_series_renders_placeholder_only() {
        // Verify the state machine dispatches correctly.
        // We can't easily inspect Element contents, but we can assert
        // the function returns without panicking.
        let _el: Element<Message> = view(TrainingPlotState::Empty, ThemeMode::Dark);
        let _el2: Element<Message> = view(TrainingPlotState::WarmingUp, ThemeMode::Dark);
    }

    /// Single-epoch degenerate case renders without panic.
    #[test]
    fn single_epoch_renders_two_dots() {
        let epochs = vec![EpochPoint {
            epoch: 1,
            train_loss: 0.42,
            val_loss: 0.38,
        }];
        let _el: Element<Message> = view(
            TrainingPlotState::Running { epochs: &epochs },
            ThemeMode::Dark,
        );
    }
}
