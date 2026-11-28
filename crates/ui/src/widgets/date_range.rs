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
//! - Clicking "Custom…" opens the inline editor; typing dispatches
//!   `Message::LabCustomRangeEdited` with both fields' texts as shown; `update` commits it as
//!   the range only when it is a new pair whose two bounds parse
//!   (`lab::runner::custom_range_ms`: RFC 3339 or `YYYY-MM-DD`). Any other keystroke only
//!   updates the editor's text (bug-log `#230`).
//!
//! ## Parse-error highlight
//!
//! A bound that `lab::runner::custom_bound_ms` (the run's own parse, bug-log `#230`) refuses
//! is highlighted with a `color::DOWN_500` border + the `DATE_RANGE_INVALID_DATE` copy below the
//! field (R5.1). An empty field is not highlighted.
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

use crate::lab::runner::custom_bound_ms;
use crate::lab::state::{CustomRangeDraft, DateRange, Preset, custom_fields};
use crate::state::Message;
use crate::strings;
use crate::theme::{ThemeMode, color, radius, space, text};

/// Width of each Custom date field, so a whole RFC 3339 bound with its offset is visible
/// (bug-log `#230` review: at 120 px the field clipped a bound after about 15 characters, so the
/// offset, which decides whether the bound is valid, never showed). MEASURED by the
/// `lab_custom_range_render` `measure` test at `text::BODY`: the widest 25-character run
/// (`0000-00-00T00:00:00+00:00`) ends 190 px from the field's left border; + 5 px padding and
/// the 1 px border is 197, and the rest leaves room for the cursor.
const DATE_FIELD_WIDTH_PX: f32 = 210.0;

/// Render the date-range picker.
///
/// `range` — the current `DateRange` from `Cockpit::lab_state.range`.
/// `draft` — the Custom editor's uncommitted text (`Cockpit::lab_state.custom_draft`, bug-log
///   `#230`); the fields show it while it edits `range` (`lab::state::custom_fields`).
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
    draft: Option<&'a CustomRangeDraft>,
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

    // Inline editor when Custom is active: the fields show the draft while it edits `range`,
    // else the committed bounds (bug-log #230).
    if let Some((start_shown, end_shown)) = custom_fields(range, draft) {
        let start_ok = custom_bound_ms(start_shown).is_ok();
        let end_ok = custom_bound_ms(end_shown).is_ok();
        // bug-log #230: each keystroke carries the OTHER field's text as shown (the
        // draft's), not the committed range's.
        let (start_for_end, end_for_start) = (start_shown.clone(), end_shown.clone());

        let start_input = date_field(
            start_shown,
            strings::DATE_RANGE_START_PLACEHOLDER,
            start_ok || start_shown.is_empty(),
            move |s| Message::LabCustomRangeEdited {
                start_raw: SmolStr::new(&s),
                end_raw: end_for_start.clone(),
            },
            mode,
        );

        let end_input = date_field(
            end_shown,
            strings::DATE_RANGE_END_PLACEHOLDER,
            end_ok || end_shown.is_empty(),
            move |s| Message::LabCustomRangeEdited {
                start_raw: start_for_end.clone(),
                end_raw: SmolStr::new(&s),
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
        if !start_shown.is_empty() && !start_ok {
            col = col.push(
                Text::new(strings::DATE_RANGE_INVALID_DATE)
                    .size(text::MICRO)
                    .color(color::DOWN_500.current(mode)),
            );
        }
        if !end_shown.is_empty() && !end_ok {
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
        .width(Length::Fixed(DATE_FIELD_WIDTH_PX))
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
    use iced::Settings;
    use iced::keyboard::key::Named;
    use iced_test::Simulator;
    use smol_str::SmolStr;

    use crate::lab::runner::custom_bound_ms;
    use crate::lab::state::{CustomRangeDraft, DateRange, Preset};
    use crate::state::{Cockpit, Message};
    use crate::theme::ThemeMode;

    /// T-D-7 — `view` constructs for preset range (no narrowed badge).
    #[test]
    fn date_range_picker_presets_constructs() {
        let range = DateRange::Preset(Preset::Last90d);
        let _el = super::view(&range, None, None, ThemeMode::Dark);
    }

    /// T-D-7 — `view` constructs for Custom range with valid dates.
    #[test]
    fn date_range_picker_custom_valid_constructs() {
        let range = DateRange::Custom {
            start_raw: SmolStr::new("2024-01-01"),
            end_raw: SmolStr::new("2024-06-30"),
        };
        let _el = super::view(&range, None, None, ThemeMode::Dark);
    }

    /// T-D-7 — `view` constructs for Custom range with invalid start.
    /// This exercises the parse-error highlight path.
    #[test]
    fn date_range_picker_custom_invalid_constructs() {
        let range = DateRange::Custom {
            start_raw: SmolStr::new("not-a-date"),
            end_raw: SmolStr::new("2024-06-30"),
        };
        let _el = super::view(&range, None, None, ThemeMode::Dark);
    }

    /// T-D-7 — `view` constructs with a narrowed-from badge.
    #[test]
    fn date_range_picker_narrowed_badge_constructs() {
        let range = DateRange::Preset(Preset::Last30d);
        let report = SmolStr::new("backtest-20260429-195243-top10-2024-h1-momentum.md");
        let _el = super::view(&range, None, Some(&report), ThemeMode::Dark);
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
        let start_valid = custom_bound_ms(start).is_ok();
        let end_valid = custom_bound_ms(end).is_ok();
        let summary = format!(
            "range=Custom start={start} start_valid={start_valid} end={end} end_valid={end_valid} error={}",
            crate::strings::DATE_RANGE_INVALID_DATE
        );
        insta::assert_snapshot!("date_range_picker__custom_invalid", summary);
    }

    // ── bug-log #230: what a keystroke sends, and what `update` does with it ──
    //
    // The view is driven by `iced_test::Simulator` (headless, the Emulator's renderer). One
    // Simulator is ONE widget tree, as one iced event batch is many messages from one view
    // (`iced_test` `simulate`, `iced_winit` `lib.rs:1214-1231`), so these tests replay the
    // stale-view batches the classifier has to survive. Clicks target a field by its shown
    // value, or by its placeholder while it is empty, and are `expect`ed, so a missed selector
    // is a named failure; `End` then puts the cursor at the end whatever the click position.

    fn custom(start: &str, end: &str) -> DateRange {
        DateRange::Custom {
            start_raw: SmolStr::new(start),
            end_raw: SmolStr::new(end),
        }
    }

    fn draft_over(base: &DateRange, start: &str, end: &str) -> CustomRangeDraft {
        CustomRangeDraft {
            base: base.clone(),
            start_raw: SmolStr::new(start),
            end_raw: SmolStr::new(end),
        }
    }

    /// The picker over `range` and `draft`, in a Simulator on the embedded UI face. The field
    /// whose text is `field` is clicked, the cursor moved to its end, then `keys` run. Returns
    /// every message the one widget tree produced, in order.
    fn keystrokes(
        range: &DateRange,
        draft: Option<&CustomRangeDraft>,
        field: &str,
        keys: impl FnOnce(&mut Simulator<'_, Message>),
    ) -> Vec<Message> {
        let mut sim: Simulator<'_, Message> = Simulator::with_size(
            Settings {
                default_font: crate::theme::font::UI,
                fonts: vec![crate::theme::font::UI_REGULAR.into()],
                ..Settings::default()
            },
            (1920.0, 1080.0),
            super::view(range, draft, None, ThemeMode::Dark),
        );
        sim.click(field)
            .unwrap_or_else(|e| panic!("non-vacuity: a field showing {field:?}: {e:?}"));
        let _ = sim.tap_key(Named::End);
        keys(&mut sim);
        sim.into_messages().collect()
    }

    /// The start text a keystroke message carries, whichever message the view sent (K4's
    /// subject is the batch, not the message kind, which W2 pins); `None` for anything else.
    fn edited_start(m: &Message) -> Option<String> {
        match m {
            Message::LabCustomRangeEdited { start_raw, .. }
            | Message::LabSelectRange(DateRange::Custom { start_raw, .. }) => {
                Some(start_raw.to_string())
            }
            _ => None,
        }
    }

    /// A cockpit over the range and draft the view was built from.
    fn cockpit_over(range: &DateRange, draft: Option<&CustomRangeDraft>) -> Cockpit {
        let mut c = Cockpit::default();
        c.lab_state.range = range.clone();
        c.lab_state.custom_draft = draft.cloned();
        c
    }

    /// Apply `msgs` in order through `state::update`; the indices after which `range` moved.
    fn apply(c: &mut Cockpit, msgs: Vec<Message>) -> Vec<usize> {
        let mut changed = Vec::new();
        for (i, m) in msgs.into_iter().enumerate() {
            let before = c.lab_state.range.clone();
            crate::state::update(c, m);
            if c.lab_state.range != before {
                changed.push(i);
            }
        }
        changed
    }

    fn shown(c: &Cockpit) -> Option<(String, String)> {
        c.lab_state
            .custom_fields()
            .map(|(s, e)| (s.to_string(), e.to_string()))
    }

    /// W2 — each keystroke sends the accumulated start text and the end as shown, and never a
    /// `LabSelectRange` (HEAD sent 9 `LabSelectRange`, measured in the #230 fix session).
    #[test]
    fn each_keystroke_sends_the_accumulated_start_text_and_the_shown_end() {
        let typed = "2024-01-1";
        let range = custom("", "2024-02-01");
        let msgs = keystrokes(
            &range,
            None,
            crate::strings::DATE_RANGE_START_PLACEHOLDER,
            |sim| {
                let _ = sim.typewrite(typed);
            },
        );
        assert_eq!(msgs.len(), 9, "one message per keystroke: {msgs:?}");
        for (i, m) in msgs.iter().enumerate() {
            assert!(
                matches!(m, Message::LabCustomRangeEdited { start_raw, end_raw }
                    if start_raw.as_str() == &typed[..=i] && end_raw.as_str() == "2024-02-01"),
                "message {i} must be Edited{{{:?}, \"2024-02-01\"}}: {m:?}",
                &typed[..=i]
            );
        }
        assert!(
            !msgs.iter().any(|m| matches!(m, Message::LabSelectRange(_))),
            "no keystroke is a LabSelectRange"
        );
    }

    /// W5 — typing in the end field carries the start field AS SHOWN (the draft's text), not
    /// the committed range's.
    #[test]
    fn typing_in_the_end_field_carries_the_start_field_as_shown() {
        let range = custom("2024-01-15", "2024-02-01");
        let draft = draft_over(&range, "2024-01-1", "2024-02-01");
        let msgs = keystrokes(&range, Some(&draft), "2024-02-01", |sim| {
            let _ = sim.typewrite("2");
        });
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        assert!(
            matches!(&msgs[0], Message::LabCustomRangeEdited { start_raw, end_raw }
                if start_raw.as_str() == "2024-01-1" && end_raw.as_str() == "2024-02-012"),
            "{:?}",
            msgs[0]
        );
    }

    /// W5b — W5's twin, the other direction (bug-log `#230` review): typing in the START field
    /// carries the end field AS SHOWN (the draft's `2024-02-0`), not the committed `2024-02-01`.
    #[test]
    fn typing_in_the_start_field_carries_the_end_field_as_shown() {
        let range = custom("2024-01-15", "2024-02-01");
        let draft = draft_over(&range, "2024-01-15", "2024-02-0");
        let msgs = keystrokes(&range, Some(&draft), "2024-01-15", |sim| {
            let _ = sim.typewrite("6");
        });
        assert_eq!(msgs.len(), 1, "{msgs:?}");
        assert!(
            matches!(&msgs[0], Message::LabCustomRangeEdited { start_raw, end_raw }
                if start_raw.as_str() == "2024-01-156" && end_raw.as_str() == "2024-02-0"),
            "{:?}",
            msgs[0]
        );
    }

    /// K1 — typing a partial start bound commits nothing: the fields show it, and the last run
    /// and the saved session stay.
    #[test]
    fn typing_a_partial_start_bound_commits_nothing() {
        let range = custom("", "2024-02-01");
        let msgs = keystrokes(
            &range,
            None,
            crate::strings::DATE_RANGE_START_PLACEHOLDER,
            |sim| {
                let _ = sim.typewrite("2024-01-1");
            },
        );
        let mut c = cockpit_over(&range, None);
        let (last, prev) = crate::fixtures::fake_run_report_mirror_pair();
        c.lab_state.last_run_report = Some(last);
        c.lab_state.prev_run_report = Some(prev);
        assert!(
            !c.lab_persist.is_dirty(),
            "precondition: the session is clean"
        );
        let changed = apply(&mut c, msgs);
        assert_eq!(changed, Vec::<usize>::new(), "no keystroke moved the range");
        assert_eq!(shown(&c), Some(("2024-01-1".into(), "2024-02-01".into())));
        assert!(c.lab_state.last_run_report.is_some(), "last run kept");
        assert!(c.lab_state.prev_run_report.is_some(), "previous run kept");
        assert!(!c.lab_persist.is_dirty(), "nothing saved");
    }

    /// K2 — typing a whole start bound commits once, on its last keystroke.
    #[test]
    fn typing_a_whole_start_bound_commits_once_on_its_last_keystroke() {
        let range = custom("", "2024-02-01");
        let msgs = keystrokes(
            &range,
            None,
            crate::strings::DATE_RANGE_START_PLACEHOLDER,
            |sim| {
                let _ = sim.typewrite("2024-01-15");
            },
        );
        assert_eq!(msgs.len(), 10, "{msgs:?}");
        let mut c = cockpit_over(&range, None);
        let changed = apply(&mut c, msgs);
        assert_eq!(changed, vec![9], "one commit, after the 10th keystroke");
        assert_eq!(c.lab_state.range, custom("2024-01-15", "2024-02-01"));
        assert!(c.lab_state.custom_draft.is_none());
        assert!(c.lab_persist.is_dirty(), "the commit is saved");
    }

    /// K3 — typing a start bound with a time commits its date-only prefix, then the whole
    /// bound: two commits, the second the RFC 3339 pair.
    #[test]
    fn typing_a_start_bound_with_a_time_commits_it_whole() {
        let range = custom("", "2024-02-01");
        let msgs = keystrokes(
            &range,
            None,
            crate::strings::DATE_RANGE_START_PLACEHOLDER,
            |sim| {
                let _ = sim.typewrite("2024-01-01T00:00:00Z");
            },
        );
        assert_eq!(msgs.len(), 20, "{msgs:?}");
        let mut c = cockpit_over(&range, None);
        let changed = apply(&mut c, msgs);
        assert_eq!(
            changed,
            vec![9, 19],
            "commits after the date and after the Z"
        );
        assert_eq!(
            c.lab_state.range,
            custom("2024-01-01T00:00:00Z", "2024-02-01")
        );
        assert!(c.lab_state.custom_draft.is_none());
    }

    /// K4 — the stale-batch case (verifier C1/Q6): one widget tree, four keystrokes that commit
    /// 01-16 and then type back to 01-15. Decided in `update` against the live range, the batch
    /// ends where it started, with no parseable draft. Under a view-time decision the fourth
    /// keystroke would match the stale committed pair, and `range` would hold 01-16 under a
    /// shown and run 01-15.
    #[test]
    fn a_batched_edit_back_to_the_committed_pair_leaves_no_parseable_draft() {
        let x = custom("2024-01-15", "2024-02-01");
        let msgs = keystrokes(&x, None, "2024-01-15", |sim| {
            let _ = sim.tap_key(Named::Backspace);
            let _ = sim.typewrite("6");
            let _ = sim.tap_key(Named::Backspace);
            let _ = sim.typewrite("5");
        });
        let starts: Vec<Option<String>> = msgs.iter().map(edited_start).collect();
        let want: Vec<Option<String>> = ["2024-01-1", "2024-01-16", "2024-01-1", "2024-01-15"]
            .iter()
            .map(|s| Some((*s).to_owned()))
            .collect();
        assert_eq!(
            starts, want,
            "precondition: the four keystrokes of one tree"
        );
        let mut c = cockpit_over(&x, None);
        let _ = apply(&mut c, msgs);
        assert_eq!(c.lab_state.range, x, "the batch ends on the committed pair");
        assert!(
            c.lab_state.custom_draft.is_none(),
            "no parseable draft is kept"
        );
        assert_eq!(shown(&c), Some(("2024-01-15".into(), "2024-02-01".into())));
        assert_eq!(c.lab_state.run_range(), c.lab_state.range);
    }
}
