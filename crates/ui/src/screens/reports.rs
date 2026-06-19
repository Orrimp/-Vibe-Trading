//! cockpit-reports-viewer v0.1.0 — Reports screen body (M-DEV-7 / R2 / R3).
//!
//! A list-detail screen that browses the committed `backtest-*.md` corpus
//! and renders the selected report, reusing the existing widgets
//! **verbatim**. Layout (left ‖ right split, the Memory/Models precedent):
//!
//! ```text
//! ┌─ Backtest reports ─────┬─ <detail pane> ─────────────────────────────┐
//! │ slug · backtest-…-a   ◀│ kpi_strip::view(&loaded.metrics)            │
//! │ slug · backtest-…-b    │ equity_curve::view(&loaded.equity)  ← Empty │
//! │ otherslug · backtest…  │ drawdown_band::view(&loaded.equity) ← Empty │
//! │ …                      │ body_render::view(&loaded.body_markdown)    │
//! └────────────────────────┴─────────────────────────────────────────────┘
//! ```
//!
//! - **Left picker** — a scrollable column of selectable rows, one per
//!   discovered `ReportEntry`, labelled `"<slug> · <file_stem>"`. The active
//!   row reuses the Baseline chip-token discipline (active = `PANEL_RAISED`
//!   bg + `ACCENT` text + `ACCENT` border; inactive = `FG_3` + `BORDER_1`).
//!   Empty corpus → `REPORTS_EMPTY_LIST` copy (never a blank list).
//! - **Right detail** — `selected == None` → `REPORTS_SELECT_PROMPT`. A
//!   `Ready(ReportLoadResult)` → the verbatim `bin/viewer.rs` stack (KPI
//!   strip → equity curve → drawdown band → markdown body). The curve/band
//!   render their built-in **Empty** body for companion-less reports
//!   (expected — § Data contract, NOT a failure); the KPI strip renders its
//!   muted body on a malformed `## Summary`. `loaded == Error` (vanished
//!   file) → `REPORTS_LOAD_ERROR` copy.
//!
//! The render widgets + `body_render` return `Element<'_, ViewerMessage>`;
//! they are bridged to the screen's `Message` with `.map(|_| Message::
//! ChartMarkerHoverEnded)` — the harmless never-fired no-op arm the Baseline
//! screen uses (these panels emit no interactions for Reports).
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new theme token, no new widget** (AC7).

// Per-module clippy allow-pattern (mirrors `screens/baseline.rs:32`): the
// `space::* as u16`/`as f32` layout casts are bounded + safe; `view`/helpers
// take `mode` by value (the `Copy` `ThemeMode`). These match the crate's
// pre-existing pedantic baseline — no NEW warning is introduced.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::needless_pass_by_value
)]

use iced::widget::{Button, Column, Container, Row, Scrollable, Space, Text, button};
use iced::{Border, Length};

use crate::reports::ReportEntry;
use crate::reports::state::companion_count;
use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    REPORTS_EMPTY_LIST, REPORTS_FILTER_ALL, REPORTS_FILTER_CURVE_ONLY,
    REPORTS_FILTER_NO_CURVE_HINT, REPORTS_HAS_CURVE_MARKER, REPORTS_LOAD_ERROR,
    REPORTS_PICKER_TITLE, REPORTS_SELECT_PROMPT,
};
use crate::theme::{ThemeMode, color, radius, space, text};
use crate::widgets::{drawdown_band, equity_curve, kpi_strip};

/// Fixed width of the left report picker (Lumen list-detail proportion,
/// matching the Models/Memory list rails — a `layout`-token-free local
/// constant is avoided; reuse the existing sidebar-adjacent list width).
const PICKER_WIDTH: f32 = 320.0;

/// Render the Reports screen body (R1–R3).
///
/// Called by `shell::screen_body` when `current_screen == Screen::Reports`.
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let st = &model.reports_screen_state;

    let picker = picker_pane(st, mode);
    let detail = detail_pane(st, mode);

    Row::new()
        .padding(space::L as u16)
        .spacing(space::L)
        .push(picker)
        .push(detail)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// Build the left report picker (R1 / R3).
///
/// **reports-picker-curve-filter.** Below the title sits a compact two-chip
/// filter toggle ("Curve only (N)" / "All (M)"). It defaults to "Curve only"
/// — the operator kept landing on companion-less "no equity data" reports, so
/// the rail shows only `has_companion` rows by default, with "All" to reveal
/// the full corpus. When the corpus carries reports but none have a curve and
/// the curve-only filter is active, a graceful hint replaces the empty list
/// (never a blank rail).
fn picker_pane(st: &crate::reports::ReportsScreenState, mode: ThemeMode) -> crate::Element<'_> {
    let title = Text::new(REPORTS_PICKER_TITLE)
        .size(text::H3)
        .color(color::FG_1.current(mode));

    let mut col = Column::new().spacing(space::S).push(title);

    match &st.discovered {
        PanelState::Ready(entries) if !entries.is_empty() => {
            // The two filter counts: N = companion-bearing rows ("Curve only"),
            // M = the full discovered corpus ("All"). Both derived from the
            // FULL list so the chips report the true scope regardless of which
            // rows are currently displayed.
            let curve_n = companion_count(entries);
            let all_m = entries.len();
            col = col.push(filter_toggle(st.show_all_reports, curve_n, all_m, mode));

            // ── INDEX-SAFETY CONTRACT (reports-picker-curve-filter) ──────────
            // `selected` and `Message::ReportsSelect(idx)` are indices into the
            // FULL discovered list (`entries`), consumed verbatim by
            // `load_selection(idx)`. We MUST emit each visible row's TRUE
            // full-list index. So we iterate the FULL list with `.enumerate()`
            // (preserving the real index) and SKIP `!has_companion` rows when
            // the curve-only filter is active — we NEVER re-index against a
            // filtered subset. A wrong index would load the wrong report; this
            // enumerate-and-skip keeps `idx` authoritative.
            let mut rows = Column::new().spacing(space::XS);
            let mut visible = 0usize;
            for (idx, entry) in entries.iter().enumerate() {
                if !st.show_all_reports && !entry.has_companion {
                    continue; // curve-only filter hides companion-less rows
                }
                let is_active = st.selected == Some(idx);
                rows = rows.push(picker_row(idx, entry, is_active, mode));
                visible += 1;
            }

            if visible == 0 {
                // Curve-only is active but the corpus has zero companion rows
                // (belt-and-braces — live corpus has 14). Graceful hint, not a
                // blank rail; points the operator at the "All" toggle above.
                col = col.push(
                    Text::new(REPORTS_FILTER_NO_CURVE_HINT)
                        .size(text::BODY)
                        .color(color::FG_3.current(mode)),
                );
            } else {
                col = col.push(
                    Scrollable::new(rows)
                        .width(Length::Fixed(PICKER_WIDTH))
                        .height(Length::Fill),
                );
            }
        }
        // Empty corpus OR pre-boot Loading both surface the same "nothing to
        // pick" copy — honest, never a blank list. (The boot scan resolves
        // `Loading` → `Ready`/`Empty` synchronously, so `Loading` is only
        // ever seen in a not-yet-booted unit fixture.)
        _ => {
            col = col.push(
                Text::new(REPORTS_EMPTY_LIST)
                    .size(text::BODY)
                    .color(color::FG_3.current(mode)),
            );
        }
    }

    Container::new(col)
        .width(Length::Fixed(PICKER_WIDTH))
        .height(Length::Fill)
        .into()
}

/// The compact two-chip filter toggle at the top of the picker rail
/// (reports-picker-curve-filter). Mirrors the Lab source-toggle
/// (`widgets::source_toggle`) + the Audit filter chips
/// (`screens::audit::make_chip`): a `Row` of chip buttons where the ACTIVE
/// chip uses the `ACCENT` background + `FG_ON_ACCENT` text (the same hue as
/// the row "● curve" marker), and the inactive chip uses `PANEL_RAISED` +
/// muted text. Both chips dispatch the same niladic `ReportsToggleShowAll`.
///
/// - `show_all == false` → "Curve only (N)" is active (ACCENT).
/// - `show_all == true`  → "All (M)" is active (ACCENT).
///
/// The count is appended numerically at the call site so the prose stays in
/// `crate::strings` and the number is a runtime value (no inline copy).
fn filter_toggle(
    show_all: bool,
    curve_n: usize,
    all_m: usize,
    mode: ThemeMode,
) -> crate::Element<'static> {
    // `format!` here only joins a `strings::` prose constant with a count and
    // parentheses — the user-visible prose lives in `strings`; the template is
    // punctuation + a placeholder.
    let curve_label = format!("{REPORTS_FILTER_CURVE_ONLY} ({curve_n})");
    let all_label = format!("{REPORTS_FILTER_ALL} ({all_m})");

    let curve_chip = filter_chip(curve_label, !show_all, mode);
    let all_chip = filter_chip(all_label, show_all, mode);

    Row::new()
        .spacing(space::XXS)
        .push(curve_chip)
        .push(all_chip)
        .width(Length::Shrink)
        .into()
}

/// One filter chip. Active = `ACCENT` bg + `FG_ON_ACCENT` text + `ACCENT`
/// border; inactive = `PANEL_RAISED` bg + `FG_3` text + `BORDER_1` border —
/// the exact `widgets::source_toggle::chip_button` contract (reused tokens,
/// no new token, no new widget). Colour is never the only signal: the active
/// chip also gets the solid accent fill (shape/contrast), per the
/// accessibility minimum.
fn filter_chip(label: String, active: bool, mode: ThemeMode) -> crate::Element<'static> {
    let fg = if active {
        color::FG_ON_ACCENT.current(mode)
    } else {
        color::FG_3.current(mode)
    };
    let bg = if active {
        color::ACCENT.current(mode)
    } else {
        color::PANEL_RAISED.current(mode)
    };
    let border_color = if active {
        color::ACCENT.current(mode)
    } else {
        color::BORDER_1.current(mode)
    };
    Button::new(Text::new(label).size(text::SMALL).color(fg))
        .on_press(Message::ReportsToggleShowAll)
        .padding([space::XXS as u16, space::S as u16])
        .style(move |_t: &iced::Theme, _s: button::Status| button::Style {
            background: Some(bg.into()),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: radius::R3.into(),
            },
            text_color: fg,
            ..Default::default()
        })
        .into()
}

/// One selectable picker row. Active = raised bg + accent text/border;
/// inactive = muted text + hairline border. Colour is never the only active
/// signal — the active row also gets the raised background (shape), per the
/// accessibility minimum (the Baseline chip pattern).
///
/// **Has-curve marker (backtest-equity-companion UX follow-on).** When
/// `entry.has_companion`, a compact `ACCENT` "● curve" tag is pushed to the
/// trailing edge of the row so the operator can see at a glance which reports
/// paint a populated equity curve — without hunting through the picker. The
/// marker is `ACCENT`-coloured AND carries the explicit "curve" label, so
/// colour is never the only signal (accessibility minimum). No new theme
/// token, no new widget (AC7) — an existing `Text` in the `ACCENT` token.
fn picker_row(
    idx: usize,
    entry: &ReportEntry,
    is_active: bool,
    mode: ThemeMode,
) -> crate::Element<'_> {
    let label = format!("{} \u{00b7} {}", entry.slug, entry.file_stem);
    let label_text = Text::new(label)
        .size(text::SMALL)
        .color(if is_active {
            color::ACCENT.current(mode)
        } else {
            color::FG_3.current(mode)
        })
        .width(Length::Fill);

    // Row = label (filling) + optional trailing "● curve" marker. The marker
    // stays `ACCENT` on both active + inactive rows (it contrasts against the
    // muted/raised row backgrounds either way) so the has-curve hint is
    // legible regardless of selection state.
    let mut content = Row::new()
        .spacing(space::XS)
        .align_y(iced::alignment::Vertical::Center)
        .push(label_text);
    if entry.has_companion {
        content = content.push(
            Text::new(REPORTS_HAS_CURVE_MARKER)
                .size(text::SMALL)
                .color(color::ACCENT.current(mode)),
        );
    }

    Button::new(content)
        .on_press(Message::ReportsSelect(idx))
        .width(Length::Fill)
        .padding([space::XS as u16, space::S as u16])
        .style(move |_: &iced::Theme, _: button::Status| button::Style {
            background: if is_active {
                Some(color::PANEL_RAISED.current(mode).into())
            } else {
                None
            },
            text_color: if is_active {
                color::ACCENT.current(mode)
            } else {
                color::FG_3.current(mode)
            },
            border: Border {
                color: if is_active {
                    color::ACCENT.current(mode)
                } else {
                    color::BORDER_1.current(mode)
                },
                width: 1.0,
                radius: radius::R1.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Build the right detail pane (R2 / R3).
fn detail_pane(st: &crate::reports::ReportsScreenState, mode: ThemeMode) -> crate::Element<'_> {
    let body: crate::Element<'_> = match (&st.selected, &st.loaded) {
        // A loaded report → the verbatim viewer detail stack.
        (Some(_), PanelState::Ready(r)) => {
            let kpi = kpi_strip::view(&r.metrics, mode).map(|_| Message::ChartMarkerHoverEnded);
            let curve = equity_curve::view(&r.equity, mode).map(|_| Message::ChartMarkerHoverEnded);
            let band = drawdown_band::view(&r.equity, mode).map(|_| Message::ChartMarkerHoverEnded);
            let body = body_render(&r.body_markdown, mode);
            Column::new()
                .spacing(space::M)
                .push(kpi)
                .push(curve)
                .push(band)
                .push(body)
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
        }
        // Selected but the load failed (file vanished / unreadable) → Error
        // copy (R3). Never a blank pane.
        (Some(_), PanelState::Error(_)) => prompt(REPORTS_LOAD_ERROR, mode),
        // Nothing selected yet, OR selected-but-still-Loading (the latter is
        // transient — selection load is synchronous, so it is only seen in a
        // unit fixture). Both surface the cold-start "pick a report" prompt
        // (R3) — never a blank pane.
        (None | Some(_), _) => prompt(REPORTS_SELECT_PROMPT, mode),
    };

    Container::new(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

/// A centred prompt/empty/error message in the detail pane — never a blank
/// surface (R3).
fn prompt(copy: &str, mode: ThemeMode) -> crate::Element<'_> {
    let text = Text::new(copy)
        .size(text::BODY)
        .color(color::FG_3.current(mode));
    Container::new(
        Column::new()
            .push(Space::new().height(Length::Fixed(space::XL as f32)))
            .push(text),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

/// Bridge the lifted `reports::body_render::view` (returns `ViewerMessage`)
/// into the screen's `Message` via the never-fired no-op arm.
fn body_render(markdown: &str, mode: ThemeMode) -> crate::Element<'_> {
    crate::reports::body_render::view(markdown, mode).map(|_| Message::ChartMarkerHoverEnded)
}
