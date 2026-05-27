//! Toast-tray overlay widget — cockpit-toast-queue v0.1.0 (ADR-0046).
//!
//! Renders a stacked vertical list of ≤ `MAX_TOAST_QUEUE_LEN` toast entries
//! in the bottom-right corner of the cockpit shell. Each entry is a Lumen
//! card with a severity-tinted 4 px left border, the message text, and a
//! manual-dismiss `×` button.
//!
//! ## Public entry point
//!
//! ```text
//! pub fn view(queue: &VecDeque<ToastEntry>, mode: ThemeMode) -> Element<'_, Message>
//! ```
//!
//! Returns a 0×0 [`Container`] wrapping [`Space`] when the queue is empty
//! so the [`iced::widget::Stack`] layer in `shell.rs` is structurally present
//! but pixel-silent (mirrors `journal_transaction_modal` empty-layer pattern).
//!
//! ## Severity → Lumen token mapping (zero new tokens, per ADR-0046 § Decision)
//!
//! | `ToastSeverity` | Border color token   |
//! |-----------------|----------------------|
//! | `Info`          | `color::FG_2`        |
//! | `Success`       | `color::UP_500`      |
//! | `Warning`       | `color::INFO_400`    |
//! | `Danger`        | `color::DOWN_500`    |
//!
//! ## Placement
//!
//! The outer `Container` is aligned to `Bottom + Right` of its parent cell.
//! `padding.bottom` = 28 px clears the 24 px activity tape plus a 4 px gap,
//! satisfying Q4=(a) "above the activity tape".
//!
//! ## Zero string literals discipline
//!
//! All user-visible text via `crate::strings::*`. The dismiss button label
//! uses `strings::TOAST_DISMISS_BUTTON` ("×").

use std::collections::VecDeque;

use iced::widget::{Column, Container, Row, Space, button, text};
use iced::{Alignment, Length, Padding};

use crate::state::{
    MAX_TOAST_QUEUE_LEN, Message, TOAST_CARD_WIDTH_PX, ToastEntry, ToastId, ToastSeverity,
};
use crate::strings;
use crate::theme::{ThemeMode, color, radius};

/// Bottom offset for the tray: clears the 24 px activity tape + 4 px gap.
const TOAST_TRAY_BOTTOM_OFFSET_PX: f32 = 28.0;

/// Right offset — a small visual breathing room from the shell edge.
const TOAST_TRAY_RIGHT_OFFSET_PX: f32 = 12.0;

/// Width of the left severity-tinted border stripe on each card (px).
const TOAST_BORDER_STRIPE_PX: f32 = 4.0;

/// Padding applied uniformly inside the message area of each card (= `space::S` = 8 px).
const CARD_MSG_PADDING: f32 = 8.0;

/// Horizontal padding on the dismiss button (= `space::XXS` = 4 px).
const CARD_BTN_PAD_H: f32 = 4.0;

/// Vertical gap between stacked toast cards (= `space::XXS` = 4 px).
const CARD_COLUMN_SPACING: f32 = 4.0;

/// Map `ToastSeverity` to the corresponding Lumen `ModeColor` token.
///
/// Per ADR-0046 § Decision: zero new tokens. Uses existing palette.
fn severity_color(severity: ToastSeverity) -> color::ModeColor {
    match severity {
        ToastSeverity::Info => color::FG_2,
        ToastSeverity::Success => color::UP_500,
        ToastSeverity::Warning => color::INFO_400,
        ToastSeverity::Danger => color::DOWN_500,
    }
}

/// Render a single toast card.
///
/// Layout: `Row [ 4 px severity stripe | message text | × button ]`
/// inside a `Container` with `PANEL_RAISED` background, `radius::R4`
/// corners, and `space::S` (8 px) internal padding.
fn toast_card(entry: &ToastEntry, mode: ThemeMode) -> crate::Element<'_> {
    let stripe_color = severity_color(entry.severity).current(mode);
    let id: ToastId = entry.id;

    let stripe = Container::new(Space::new().width(Length::Fixed(TOAST_BORDER_STRIPE_PX)))
        .width(Length::Fixed(TOAST_BORDER_STRIPE_PX))
        .height(Length::Fill)
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(stripe_color.into()),
            ..Default::default()
        });

    let msg = text(entry.message.as_str())
        .size(crate::theme::text::BODY)
        .color(color::FG_1.current(mode));

    let dismiss_btn = button(
        text(strings::TOAST_DISMISS_BUTTON)
            .size(crate::theme::text::BODY)
            .color(color::FG_3.current(mode)),
    )
    .on_press(Message::DismissToastById(id))
    .style(|theme: &iced::Theme, status| {
        // Use a transparent / borderless button style.
        let mut base = iced::widget::button::text(theme, status);
        base.background = None;
        base
    })
    .padding(Padding {
        top: 0.0,
        bottom: 0.0,
        left: CARD_BTN_PAD_H,
        right: CARD_BTN_PAD_H,
    });

    let card_row = Row::new()
        .push(stripe)
        .push(
            Container::new(msg)
                .padding(Padding {
                    top: CARD_MSG_PADDING,
                    bottom: CARD_MSG_PADDING,
                    left: CARD_MSG_PADDING,
                    right: CARD_MSG_PADDING,
                })
                .width(Length::Fill)
                .align_y(Alignment::Center),
        )
        .push(Container::new(dismiss_btn).align_y(Alignment::Center))
        .align_y(Alignment::Center)
        .width(Length::Fixed(TOAST_CARD_WIDTH_PX));

    Container::new(card_row)
        .width(Length::Fixed(TOAST_CARD_WIDTH_PX))
        .style(move |_theme: &iced::Theme| {
            let bg = color::PANEL_RAISED.current(mode);
            iced::widget::container::Style {
                background: Some(bg.into()),
                border: iced::Border {
                    radius: radius::R4.into(),
                    width: 0.0,
                    color: iced::Color::TRANSPARENT,
                },
                text_color: Some(color::FG_1.current(mode)),
                ..Default::default()
            }
        })
        .into()
}

/// Render the full toast tray as a Stack overlay layer.
///
/// - Empty queue → 0×0 `Container<Space>` (pixel-silent, structurally present).
/// - Non-empty queue → bottom-right aligned `Column` of toast cards,
///   newest entry at the BOTTOM (matches macOS Notifications direction),
///   with `TOAST_TRAY_BOTTOM_OFFSET_PX` clearance above the activity tape.
///
/// The outer `Container` fills its parent (shell-cell sized), so `align_x` /
/// `align_y` push the inner column to the bottom-right corner without an
/// absolute position.
///
/// # Panics
///
/// Never panics: `queue` length is capped at `MAX_TOAST_QUEUE_LEN` by the
/// `enqueue_toast` helper in `state.rs::update`.
#[must_use]
pub fn view(queue: &VecDeque<ToastEntry>, mode: ThemeMode) -> crate::Element<'_> {
    // Structural but pixel-silent empty path.
    if queue.is_empty() {
        return Container::new(Space::new())
            .width(Length::Fill)
            .height(Length::Fill)
            .into();
    }

    // Build column of cards. Queue is FIFO — front is oldest.
    // Newest at the BOTTOM → iterate front-to-back so oldest is top, newest is bottom.
    // Capacity is bounded by MAX_TOAST_QUEUE_LEN; no runtime panic risk.
    let _ = MAX_TOAST_QUEUE_LEN; // reference the const so it appears in docs
    let mut col = Column::new().spacing(CARD_COLUMN_SPACING);
    for entry in queue {
        col = col.push(toast_card(entry, mode));
    }

    // Outer container: fill the parent, align to bottom-right.
    Container::new(col)
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Right)
        .align_y(iced::alignment::Vertical::Bottom)
        .padding(Padding {
            bottom: TOAST_TRAY_BOTTOM_OFFSET_PX,
            right: TOAST_TRAY_RIGHT_OFFSET_PX,
            top: 0.0,
            left: 0.0,
        })
        .into()
}
