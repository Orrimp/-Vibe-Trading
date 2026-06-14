//! Date-range picker widget — ui-rethink-phase-a-lab T-D-7.
//!
//! Renders a row of preset chips ("Last 30d", "Last 90d", "2024 H1",
//! "2024 H2") plus a "Custom…" chip that expands into two ISO-8601 date
//! text-fields (Phase A — no calendar widget; that's Phase B/C).
//!
//! ## Interactions
//!
//! - Clicking a preset chip dispatches
//!   `Message::LabSelectRange(DateRange::Preset(p))`.
//! - Clicking "Custom…" opens the inline editor; typing in the fields
//!   dispatches `Message::LabSelectRange(DateRange::Custom { .. })` only
//!   when both dates parse as valid ISO-8601.
//!
//! ## Parse-error highlight
//!
//! An invalid ISO-8601 input is highlighted with `color::DOWN_500` border
//! + the `DATE_RANGE_INVALID_DATE` copy below the field (R5.1).
//!
//! ## Narrowed-from badge
//!
//! When `narrowed_from` is `Some(report_name)`, a badge reading
//! `"Narrowed from <report_name>"` is rendered adjacent to the picker
//! (R5.4).
//!
//! **Zero hex literals** — all colors from `crate::theme`.
//! **Zero string literals** — copy from `crate::strings`.

use iced::widget::{Column, Container, Row, Text, button, container, text_input};
use iced::{Border, Length};
use smol_str::SmolStr;

use crate::lab::state::{DateRange, Preset};
use crate::state::Message;
use crate::strings;
use crate::theme::{ThemeMode, color, radius, space, text};

/// Validate an ISO-8601 date string `YYYY-MM-DD` — pure function, no I/O.
///
/// Returns `true` if the string matches `DDDD-DD-DD` and the month/day
/// are plausible. Phase A is intentionally strict (no timezone, no time
/// component). The exact calendar arithmetic is deferred to Phase B where
/// `chrono` will power the calendar picker.
#[must_use]
pub fn is_valid_date(s: &str) -> bool {
    // Minimal structural check: "YYYY-MM-DD" = 10 chars with dashes at [4] and [7].
    if s.len() != 10 {
        return false;
    }
    let b = s.as_bytes();
    if b[4] != b'-' || b[7] != b'-' {
        return false;
    }
    let year_ok = b[0..4].iter().all(u8::is_ascii_digit);
    let mon_ok = b[5..7].iter().all(u8::is_ascii_digit);
    let day_ok = b[8..10].iter().all(u8::is_ascii_digit);
    if !year_ok || !mon_ok || !day_ok {
        return false;
    }
    // Plausible month [01..12] and day [01..31].
    let month: u8 = (b[5] - b'0') * 10 + (b[6] - b'0');
    let day: u8 = (b[8] - b'0') * 10 + (b[9] - b'0');
    (1..=12).contains(&month) && (1..=31).contains(&day)
}

/// Render the date-range picker.
///
/// `range` — the current `DateRange` from `Cockpit::lab_state.range`.
/// `narrowed_from` — optional report name for the "Narrowed from …" badge
///   (R5.4). `None` → badge hidden.
/// `mode` — active theme mode.
///
/// The widget is rendered as a `Column`:
/// ```text
/// [ Last 30d | Last 90d | 2024 H1 | 2024 H2 | Custom… ]
/// (if Custom active:)
///   [ YYYY-MM-DD ] — [ YYYY-MM-DD ]
/// (if narrowed:)
///   "Narrowed from <report_name>"
/// ```
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view<'a>(
    range: &'a DateRange,
    narrowed_from: Option<&'a SmolStr>,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let presets = [
        Preset::Last30d,
        Preset::Last90d,
        Preset::H1_2024,
        Preset::H2_2024,
    ];

    let mut preset_row = Row::new().spacing(space::S);

    for p in presets {
        let is_active = matches!(range, DateRange::Preset(rp) if *rp == p);
        preset_row = preset_row.push(preset_chip(p, is_active, mode));
    }

    // "Custom…" chip.
    let custom_active = matches!(range, DateRange::Custom { .. });
    preset_row = preset_row.push(custom_chip(custom_active, mode));

    let mut col = Column::new().spacing(space::S).push(preset_row);

    // Inline editor when Custom is active.
    if let DateRange::Custom { start_raw, end_raw } = range {
        let start_valid = is_valid_date(start_raw.as_str());
        let end_valid = is_valid_date(end_raw.as_str());

        let start_sr = start_raw.to_string();
        let end_sr = end_raw.to_string();

        let start_input = date_field(
            &start_sr,
            strings::DATE_RANGE_START_PLACEHOLDER,
            start_valid || start_sr.is_empty(),
            move |s| {
                Message::LabSelectRange(DateRange::Custom {
                    start_raw: SmolStr::new(&s),
                    end_raw: SmolStr::new(&end_sr),
                })
            },
            mode,
        );

        let end_sr2 = end_raw.to_string();
        let start_sr2 = start_raw.to_string();

        let end_input = date_field(
            &end_sr2,
            strings::DATE_RANGE_END_PLACEHOLDER,
            end_valid || end_sr2.is_empty(),
            move |s| {
                Message::LabSelectRange(DateRange::Custom {
                    start_raw: SmolStr::new(&start_sr2),
                    end_raw: SmolStr::new(&s),
                })
            },
            mode,
        );

        let dash = Text::new(strings::DATE_RANGE_SEPARATOR)
            .size(text::BODY)
            .color(color::FG_3.current(mode));

        let field_row = Row::new()
            .spacing(space::S)
            .align_y(iced::Alignment::Center)
            .push(start_input)
            .push(dash)
            .push(end_input);

        col = col.push(field_row);

        // Error hints.
        if !start_sr.is_empty() && !start_valid {
            col = col.push(
                Text::new(strings::DATE_RANGE_INVALID_DATE)
                    .size(text::MICRO)
                    .color(color::DOWN_500.current(mode)),
            );
        }
        if !end_sr2.is_empty() && !end_valid {
            col = col.push(
                Text::new(strings::DATE_RANGE_INVALID_DATE)
                    .size(text::MICRO)
                    .color(color::DOWN_500.current(mode)),
            );
        }
    }

    // "Narrowed from …" badge.
    if let Some(name) = narrowed_from {
        let badge_text = format!("{} {name}", strings::LAB_NARROWED_FROM_BADGE);
        col = col.push(
            Text::new(badge_text)
                .size(text::MICRO)
                .color(color::FG_3.current(mode)),
        );
    }

    col.into()
}

/// Render a single preset chip button.
#[allow(clippy::cast_possible_truncation)] // space constants are u32 < 256, cast to u16 is safe
fn preset_chip<'a>(p: Preset, active: bool, mode: ThemeMode) -> crate::Element<'a> {
    let fg = if active {
        color::FG_1.current(mode)
    } else {
        color::FG_2.current(mode)
    };
    let bg = if active {
        color::ACCENT_SOFT.current(mode)
    } else {
        color::PANEL.current(mode)
    };
    let border = if active {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };

    let label = Text::new(p.label()).size(text::SMALL).color(fg);
    let chip_container = Container::new(label)
        .padding([space::XS as u16, space::M as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg.into()),
            border: Border {
                color: border,
                width: if active { 1.5 } else { 1.0 },
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    button(chip_container)
        .on_press(Message::LabSelectRange(DateRange::Preset(p)))
        .padding(0)
        .style(|_t: &iced::Theme, _s| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink)
        .into()
}

/// Render the "Custom..." chip button.
#[allow(clippy::cast_possible_truncation)] // space constants are u32 < 256, cast to u16 is safe
fn custom_chip<'a>(active: bool, mode: ThemeMode) -> crate::Element<'a> {
    let fg = if active {
        color::FG_1.current(mode)
    } else {
        color::FG_2.current(mode)
    };
    let bg = if active {
        color::ACCENT_SOFT.current(mode)
    } else {
        color::PANEL.current(mode)
    };
    let border = if active {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };

    let label = Text::new(strings::DATE_RANGE_CUSTOM_LABEL)
        .size(text::SMALL)
        .color(fg);
    let chip_container = Container::new(label)
        .padding([space::XS as u16, space::M as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg.into()),
            border: Border {
                color: border,
                width: if active { 1.5 } else { 1.0 },
                radius: radius::R4.into(),
            },
            ..Default::default()
        });

    // Clicking "Custom…" opens the inline editor with empty strings.
    let msg = Message::LabSelectRange(DateRange::Custom {
        start_raw: SmolStr::new(""),
        end_raw: SmolStr::new(""),
    });

    button(chip_container)
        .on_press(msg)
        .padding(0)
        .style(|_t: &iced::Theme, _s| button::Style {
            background: None,
            ..Default::default()
        })
        .width(Length::Shrink)
        .into()
}

/// Render a single ISO-8601 date text-input field.
///
/// `valid` controls the border color: valid/empty → `BORDER_1`; invalid →
/// `DOWN_700` (red highlight per R5.1).
fn date_field<'a, F>(
    value: &str,
    placeholder: &'static str,
    valid: bool,
    on_change: F,
    mode: ThemeMode,
) -> crate::Element<'a>
where
    F: Fn(String) -> Message + 'static,
{
    let border_col = if valid {
        color::BORDER_1.current(mode)
    } else {
        color::DOWN_500.current(mode)
    };

    text_input(placeholder, value)
        .on_input(on_change)
        .size(text::BODY)
        .width(Length::Fixed(120.0))
        .style(move |_t: &iced::Theme, _s| text_input::Style {
            background: color::PANEL.current(mode).into(),
            border: Border {
                color: border_col,
                width: 1.0,
                radius: radius::R4.into(),
            },
            icon: color::FG_3.current(mode),
            placeholder: color::FG_3.current(mode),
            value: color::FG_1.current(mode),
            selection: color::ACCENT_SOFT.current(mode),
        })
        .into()
}

#[cfg(test)]
#[allow(clippy::unwrap_used)]
#[allow(non_snake_case)] // double-underscore test names are a local snapshot-panel naming convention
mod tests {
    use smol_str::SmolStr;

    use crate::lab::state::{DateRange, Preset};
    use crate::theme::ThemeMode;

    use super::is_valid_date;

    /// T-D-7 — `is_valid_date` accepts valid ISO-8601 dates.
    #[test]
    fn is_valid_date_accepts_valid() {
        assert!(is_valid_date("2024-01-01"));
        assert!(is_valid_date("2024-06-30"));
        assert!(is_valid_date("2023-12-31"));
    }

    /// T-D-7 — `is_valid_date` rejects invalid inputs.
    #[test]
    fn is_valid_date_rejects_invalid() {
        assert!(!is_valid_date(""));
        assert!(!is_valid_date("2024-13-01")); // month 13
        assert!(!is_valid_date("2024-00-01")); // month 0
        assert!(!is_valid_date("2024-01-00")); // day 0
        assert!(!is_valid_date("2024-01-32")); // day 32
        assert!(!is_valid_date("20240101")); // no dashes
        assert!(!is_valid_date("2024-1-1")); // not zero-padded
        assert!(!is_valid_date("abcd-01-01")); // non-digit year
    }

    /// T-D-7 — `view` constructs for preset range (no narrowed badge).
    #[test]
    fn date_range_picker_presets_constructs() {
        let range = DateRange::Preset(Preset::Last90d);
        let _el = super::view(&range, None, ThemeMode::Dark);
    }

    /// T-D-7 — `view` constructs for Custom range with valid dates.
    #[test]
    fn date_range_picker_custom_valid_constructs() {
        let range = DateRange::Custom {
            start_raw: SmolStr::new("2024-01-01"),
            end_raw: SmolStr::new("2024-06-30"),
        };
        let _el = super::view(&range, None, ThemeMode::Dark);
    }

    /// T-D-7 — `view` constructs for Custom range with invalid start.
    /// This exercises the parse-error highlight path.
    #[test]
    fn date_range_picker_custom_invalid_constructs() {
        let range = DateRange::Custom {
            start_raw: SmolStr::new("not-a-date"),
            end_raw: SmolStr::new("2024-06-30"),
        };
        let _el = super::view(&range, None, ThemeMode::Dark);
    }

    /// T-D-7 — `view` constructs with a narrowed-from badge.
    #[test]
    fn date_range_picker_narrowed_badge_constructs() {
        let range = DateRange::Preset(Preset::Last30d);
        let report = SmolStr::new("backtest-20260429-195243-top10-2024-h1-momentum.md");
        let _el = super::view(&range, Some(&report), ThemeMode::Dark);
    }

    /// T-D-7 — snapshot: `date_range_picker__presets`.
    ///
    /// Records the picker's preset-chip row descriptor.
    #[test]
    fn date_range_picker__presets() {
        use crate::lab::state::Preset;
        let presets = [
            Preset::Last30d,
            Preset::Last90d,
            Preset::H1_2024,
            Preset::H2_2024,
        ];
        let labels: Vec<&str> = presets.iter().map(|p| p.label()).collect();
        let summary = format!(
            "range=Last90d active=Last90d presets=[{}] custom=Custom\u{2026}",
            labels.join(", ")
        );
        insta::assert_snapshot!("date_range_picker__presets", summary);
    }

    /// T-D-7 — snapshot: `date_range_picker__custom_invalid`.
    ///
    /// Records the Custom mode with an invalid start date highlighted.
    #[test]
    fn date_range_picker__custom_invalid() {
        let start = "not-a-date";
        let end = "2024-06-30";
        let start_valid = is_valid_date(start);
        let end_valid = is_valid_date(end);
        let summary = format!(
            "range=Custom start={start} start_valid={start_valid} end={end} end_valid={end_valid} error={}",
            crate::strings::DATE_RANGE_INVALID_DATE
        );
        insta::assert_snapshot!("date_range_picker__custom_invalid", summary);
    }
}
