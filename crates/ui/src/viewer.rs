//! Viewer model + message types — Phase 4.
//!
//! Lives in the `ui` lib (not the `bin/viewer.rs` bin) because the
//! widgets `kpi_strip`, `equity_curve`, `drawdown_band` return
//! `Element<'_, ViewerMessage>` and bins can't import from each other.
//!
//! Sibling of [`crate::state::Cockpit`] / [`crate::state::Message`] —
//! the cockpit lives in `state.rs`; the viewer lives here. Both are
//! pure-data presentation models; `update` is a pure function.

use std::path::PathBuf;

use smol_str::SmolStr;
use trading_core::{BacktestMetrics, EquitySeries};

use crate::state::PanelState;
use crate::theme::ThemeMode;

/// Brief B M1 — anchor date for the viewer-bin's `iced_aw::date_picker`
/// smoke-test consumer.
///
/// **Must not** call `iced_aw::core::date::Date::today()` or
/// `iced_aw::date_picker::State::reset()`: both invoke
/// `chrono::Local::now()` per
/// `~/.cargo/registry/.../iced_aw-0.14.1/src/core/date.rs:31-34`
/// and `:173-175`, which would inject wall-clock into the read-only
/// viewer surface. Pinning a const date keeps the picker
/// snapshot-deterministic for `iced_test` (architect's
/// [`feature.md ## Q3 — Determinism trap`](../../spec/iced-aw-cherry-pick/feature.md#q3--iced_awdate_picker-message-payload-iced_awcoredatedate)).
pub const VIEWER_PICKER_ANCHOR: (i32, u32, u32) = (2024, 1, 1);

/// Brief B M1 — resolve [`VIEWER_PICKER_ANCHOR`] to a `time::Date`.
///
/// The const is validated at unit-test time by
/// `viewer_picker_anchor_is_a_valid_calendar_date`; the fallback to
/// `time::Date::MIN` is structurally unreachable with the current
/// const but routes through `unwrap_or` rather than `.expect()` to
/// satisfy the lib crate's `#![deny(expect_used)]` policy.
#[allow(clippy::cast_possible_truncation)]
#[must_use]
fn picker_anchor_date() -> time::Date {
    let (y, m, d) = VIEWER_PICKER_ANCHOR;
    let month = time::Month::try_from(m as u8).unwrap_or(time::Month::January);
    time::Date::from_calendar_date(y, month, d as u8).unwrap_or(time::Date::MIN)
}

/// Read-only mirror of the YAML front-matter fields the viewer's
/// title bar and body header surface. Only the load-bearing fields
/// (`scenario` for the title bar) are mirrored — anything else
/// becomes part of the markdown body and renders inline.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ReportFrontMatter {
    /// `scenario:` value from the front-matter; populates the
    /// window title `"Backtest report — {scenario}"`. Empty string
    /// when the file has no front-matter or the field is absent.
    pub scenario: SmolStr,
}

/// One-shot load result — fired at boot from `fn main` after the CLI
/// arg parses and the file reads succeed. Carries the four
/// sub-states so the model lands fully-populated on success and the
/// curve / strip / body all render together.
#[derive(Debug, Clone)]
pub struct ReportLoadResult {
    pub front_matter: ReportFrontMatter,
    pub metrics: PanelState<BacktestMetrics>,
    pub equity: PanelState<EquitySeries>,
    pub body_markdown: String,
}

/// Root model for the viewer bin. Owned by the iced `Application`.
///
/// Brief B M1 (T-M1-2 / -3) extends the model with a date-picker primitive
/// stub (`picker_open` + `picked_date`). Full backtest date-range wiring is
/// out-of-scope for Brief B per the analyst's brief — the field is plumbed
/// only so the picker has somewhere to land its `on_submit(Date)` payload.
#[derive(Debug, Clone)]
pub struct ViewerModel {
    pub mode: ThemeMode,
    pub report_path: PathBuf,
    pub front_matter: ReportFrontMatter,
    pub metrics: PanelState<BacktestMetrics>,
    pub equity: PanelState<EquitySeries>,
    pub body_markdown: String,
    /// Brief B M1 — is the date-picker overlay visible?  Cold-start
    /// false (the underlay button alone is rendered until the operator
    /// clicks it).
    pub picker_open: bool,
    /// Brief B M1 — last picked / anchor date for the date-picker
    /// overlay. Cold-start = `VIEWER_PICKER_ANCHOR` const (2024-01-01),
    /// **NOT** `time::OffsetDateTime::now_utc()` or
    /// `iced_aw::core::date::Date::today()` — see the const docstring.
    pub picked_date: time::Date,
}

impl ViewerModel {
    /// Construct a viewer model from a one-shot load result.
    ///
    /// Brief B M1 cold-starts `picker_open = false` and
    /// `picked_date = VIEWER_PICKER_ANCHOR` (a const date, NOT
    /// `Date::today()` — see the const docstring for the determinism
    /// trap).
    #[must_use]
    pub fn new(report_path: PathBuf, result: ReportLoadResult) -> Self {
        // Brief B M1 — `picked_date` cold-starts at the const anchor.
        // The const is validated at unit-test time by
        // `viewer_picker_anchor_is_a_valid_calendar_date`; if the const
        // ever flips to an invalid (year, month, day) tuple, the test
        // fails before the binary ever runs. The fallback returns
        // `time::Date::MIN` only on the would-be-impossible path —
        // it's never reachable with the current const, but the lib
        // crate's `#![deny(expect_used)]` policy requires we route via
        // `unwrap_or` rather than `.expect()`.
        let picked_date = picker_anchor_date();
        Self {
            mode: ThemeMode::Dark,
            report_path,
            front_matter: result.front_matter,
            metrics: result.metrics,
            equity: result.equity,
            body_markdown: result.body_markdown,
            picker_open: false,
            picked_date,
        }
    }
}

/// Every possible state mutation for the viewer. Exhaustive by
/// construction — `update` matches with no catch-all arm.
#[derive(Debug, Clone)]
pub enum ViewerMessage {
    /// One-shot load result fired at boot. Field-level errors
    /// degrade to `PanelState::Error` independently — a missing
    /// equity CSV does not invalidate the KPI strip (R3.5 / R11.3
    /// missing-field tolerance).
    ReportLoaded(Box<ReportLoadResult>),
    /// Theme toggle — flips `mode`. No status bar so the toggle
    /// surfaces only via the bin's keyboard shim if/when wired
    /// (Phase 4 ships no keyboard handler).
    ToggleTheme,
    /// Brief B M1 — operator clicked the picker underlay button;
    /// open the overlay.
    PickerOpened,
    /// Brief B M1 — operator clicked the picker overlay's Cancel
    /// button; close the overlay.
    PickerCanceled,
    /// Brief B M1 — operator clicked the picker overlay's Submit
    /// button. Carries a `time::Date` (the workspace's preferred
    /// date crate per `Cargo.toml`), converted from
    /// `iced_aw::core::date::Date` via
    /// `time::Date::from_calendar_date(year, month, day)` in the
    /// bin-side `on_submit` lambda.
    PickerDateSelected(time::Date),
}

/// Pure state-transition function. `update` is exhaustive over
/// `ViewerMessage`.
pub fn update(model: &mut ViewerModel, msg: ViewerMessage) {
    match msg {
        ViewerMessage::ReportLoaded(boxed) => {
            let ReportLoadResult {
                front_matter,
                metrics,
                equity,
                body_markdown,
            } = *boxed;
            model.front_matter = front_matter;
            model.metrics = metrics;
            model.equity = equity;
            model.body_markdown = body_markdown;
        }
        ViewerMessage::ToggleTheme => {
            model.mode = match model.mode {
                ThemeMode::Dark => ThemeMode::Light,
                ThemeMode::Light => ThemeMode::Dark,
            };
        }
        ViewerMessage::PickerOpened => {
            model.picker_open = true;
        }
        ViewerMessage::PickerCanceled => {
            model.picker_open = false;
        }
        ViewerMessage::PickerDateSelected(date) => {
            model.picked_date = date;
            model.picker_open = false;
        }
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::cast_possible_truncation)]
mod tests {
    use super::*;

    /// Brief B M1 — guarantee the const date is a valid
    /// calendar date so `ViewerModel::new`'s `from_calendar_date`
    /// call never falls back. Pins year=2024 month=1 day=1 (the
    /// constant designated by the architect's Q3 — see the
    /// `VIEWER_PICKER_ANCHOR` docstring).
    #[test]
    fn viewer_picker_anchor_is_a_valid_calendar_date() {
        let (y, m, d) = VIEWER_PICKER_ANCHOR;
        assert_eq!((y, m, d), (2024, 1, 1));
        let month = time::Month::try_from(m as u8).expect("month 1 = January");
        let date = time::Date::from_calendar_date(y, month, d as u8)
            .expect("2024-01-01 is a valid calendar date");
        assert_eq!(date.year(), 2024);
        assert_eq!(date.month(), time::Month::January);
        assert_eq!(date.day(), 1);
    }

    /// Brief B M1 — verify the `PickerOpened` → `PickerCanceled` →
    /// `PickerDateSelected` round-trip lands on the model fields.
    /// Smoke-test for the bin's date-picker wiring — no iced
    /// `Subscription` involvement, pure-data `update` round-trip.
    #[test]
    fn viewer_picker_round_trip_open_cancel_submit() {
        let load = ReportLoadResult {
            front_matter: ReportFrontMatter::default(),
            metrics: PanelState::Empty,
            equity: PanelState::Empty,
            body_markdown: String::new(),
        };
        let mut model = ViewerModel::new(PathBuf::from("/dev/null"), load);
        // Cold-start invariants.
        assert!(!model.picker_open, "cold-start: overlay hidden");
        assert_eq!(
            (
                model.picked_date.year(),
                model.picked_date.month(),
                model.picked_date.day()
            ),
            (2024, time::Month::January, 1),
            "cold-start: anchor const, NOT Date::today()",
        );
        // Open.
        update(&mut model, ViewerMessage::PickerOpened);
        assert!(model.picker_open, "after PickerOpened: overlay shown");
        // Cancel.
        update(&mut model, ViewerMessage::PickerCanceled);
        assert!(!model.picker_open, "after PickerCanceled: overlay hidden");
        // Submit a different date — must update both fields.
        let new_date =
            time::Date::from_calendar_date(2025, time::Month::June, 15).expect("valid date");
        update(&mut model, ViewerMessage::PickerDateSelected(new_date));
        assert!(!model.picker_open, "after Submit: overlay hidden");
        assert_eq!(model.picked_date, new_date, "submit updates picked_date");
    }
}
