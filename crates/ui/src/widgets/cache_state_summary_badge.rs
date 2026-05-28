//! Cache-state summary badge widget — lab-yahoo-realdata v0.1.2 (T-DU1).
//!
//! Aggregate sibling of the per-pair [`crate::widgets::cache_state_badge`].
//! Renders the operator's at-a-glance view of the *whole* Yahoo cache:
//! how many of the 10 crypto-mirror tickers are populated, and when the
//! newest parquet was written.
//!
//! Operator-locked copy (Q3, 2026-05-27):
//!
//! - `populated_count == 0` → reuse [`strings::LAB_CACHE_STATE_EMPTY`]
//!   ("no cache").
//! - `populated_count >= 1` →
//!   `"Yahoo cache: {N} tickers · last fetch {YYYY-MM-DD}"`.
//!
//! `LAB_CACHE_STATE_SUMMARY_PREFIX = "Yahoo cache: "` is the only new
//! string in this widget. The count `N`, the ISO-8601 date, and the
//! middle-dot separator are runtime-derived; they are not string-table
//! eligible (R-NR.2).
//!
//! ## Design
//!
//! Mirrors `cache_state_badge` shape byte-identical (R3.4):
//!
//! - `PANEL_RAISED` background.
//! - `BORDER_1` outline, 1 px, `R3` radius.
//! - `MICRO` text, `FG_2` foreground (muted but legible — neutral
//!   sentiment by design; the summary is *informational*, not a
//!   freshness signal like the per-pair pill).
//! - `XXS` vertical / `S` horizontal padding.
//!
//! Date formatting uses the `time` crate (`OffsetDateTime`) — never
//! `chrono` per workspace convention.
//!
//! **Zero hex literals** — colors via `crate::theme`.
//! **Zero string literals** — copy via `crate::strings`.

use iced::widget::{Container, Text, container};

use crate::lab::cache_state::CacheSummary;
use crate::strings::fmt_lab_cache_state_summary;
use crate::theme::{ThemeMode, color, radius, space, text};

/// Render the aggregate cache-state summary badge.
///
/// Caller is responsible for placement (the badge ships into the Lab
/// toolbar row, regardless of `data_source`; per Q2 lock 2026-05-27).
#[must_use]
#[allow(clippy::cast_possible_truncation)]
pub fn view(summary: &CacheSummary, mode: ThemeMode) -> crate::Element<'static> {
    let label = format_label(summary);
    let fg = color::FG_2.current(mode);
    let bg = color::PANEL_RAISED.current(mode);
    let border_color = color::BORDER_1.current(mode);

    Container::new(Text::new(label).size(text::MICRO).color(fg))
        .padding([space::XXS as u16, space::S as u16])
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(bg.into()),
            border: iced::Border {
                color: border_color,
                width: 1.0,
                radius: radius::R3.into(),
            },
            ..Default::default()
        })
        .into()
}

/// Build the label string the badge renders.
///
/// Pure function — wraps [`crate::strings::fmt_lab_cache_state_summary`]
/// with the widget's date-formatting concern. Kept here so the snapshot
/// and unit tests can lock the (summary → label) mapping without
/// dragging in `strings::*` plus an iced runtime.
///
/// - `summary.populated_count == 0` → `LAB_CACHE_STATE_EMPTY`.
/// - `summary.populated_count >= 1` AND `newest_mtime.is_some()` →
///   `"Yahoo cache: N tickers · last fetch YYYY-MM-DD"`.
/// - `summary.populated_count >= 1` AND `newest_mtime.is_none()` (an
///   edge case that should not occur from `probe_summary`, but defended
///   here) → `"Yahoo cache: N tickers"` (no date suffix).
#[must_use]
pub fn format_label(summary: &CacheSummary) -> String {
    let iso_date = summary.newest_mtime.map(format_iso_date);
    fmt_lab_cache_state_summary(summary.populated_count, iso_date.as_deref())
}

/// Format a `SystemTime` as `YYYY-MM-DD` (UTC) via the `time` crate.
///
/// Workspace convention is `time`, not `chrono`. On the impossible
/// failure path (`SystemTime` -> `OffsetDateTime` conversion error —
/// only happens for timestamps outside [-9999-01-01, +9999-12-31]) we
/// return the literal `"—"` so the badge stays legible.
fn format_iso_date(mtime: std::time::SystemTime) -> String {
    use time::OffsetDateTime;
    match OffsetDateTime::from(mtime).date().to_string().as_str() {
        // `Date::to_string()` returns ISO-8601 (`YYYY-MM-DD`) on the
        // happy path. Defensive: empty / malformed → em-dash.
        "" => crate::strings::PLACEHOLDER_NONE.to_string(),
        s => s.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::strings::{LAB_CACHE_STATE_EMPTY, LAB_CACHE_STATE_SUMMARY_PREFIX};
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    /// `populated_count == 0` → reuses the existing empty label.
    #[test]
    fn format_label_empty_uses_existing_constant() {
        let s = CacheSummary {
            populated_count: 0,
            newest_mtime: None,
        };
        assert_eq!(format_label(&s), LAB_CACHE_STATE_EMPTY);
    }

    /// `populated_count == 0` ignores any non-None `newest_mtime` — the
    /// empty label is the canonical N=0 render.
    #[test]
    fn format_label_empty_ignores_newest_mtime() {
        let s = CacheSummary {
            populated_count: 0,
            newest_mtime: Some(SystemTime::now()),
        };
        assert_eq!(format_label(&s), LAB_CACHE_STATE_EMPTY);
    }

    /// `populated_count == 1` → "Yahoo cache: 1 tickers · last fetch YYYY-MM-DD".
    /// (We intentionally keep "tickers" plural even at N=1; matches the
    /// analyst's spec wording — operator can revisit if singular is preferred.)
    #[test]
    fn format_label_one_ticker_with_date() {
        // 2024-12-31 00:00:00 UTC = unix epoch 1_735_603_200.
        let mtime = UNIX_EPOCH + Duration::from_secs(1_735_603_200);
        let s = CacheSummary {
            populated_count: 1,
            newest_mtime: Some(mtime),
        };
        let label = format_label(&s);
        assert!(
            label.starts_with(LAB_CACHE_STATE_SUMMARY_PREFIX),
            "label must start with operator prefix; got {label:?}"
        );
        assert!(
            label.contains("1 tickers"),
            "label must contain the count; got {label:?}"
        );
        assert!(
            label.contains("last fetch 2024-12-31"),
            "label must contain ISO date; got {label:?}"
        );
    }

    /// `populated_count == 10` (full mirror) → label contains "10 tickers".
    #[test]
    fn format_label_ten_tickers() {
        let mtime = UNIX_EPOCH + Duration::from_secs(1_735_603_200);
        let s = CacheSummary {
            populated_count: 10,
            newest_mtime: Some(mtime),
        };
        assert!(format_label(&s).contains("10 tickers"));
    }

    /// `populated_count >= 1` but `newest_mtime == None` → drop the
    /// date clause (edge case defended).
    #[test]
    fn format_label_count_without_mtime_omits_date() {
        let s = CacheSummary {
            populated_count: 2,
            newest_mtime: None,
        };
        let label = format_label(&s);
        assert!(label.contains("2 tickers"));
        assert!(
            !label.contains("last fetch"),
            "no mtime → no last-fetch clause; got {label:?}"
        );
    }

    /// ISO date helper accepts the UNIX epoch (1970-01-01).
    #[test]
    fn format_iso_date_unix_epoch() {
        assert_eq!(format_iso_date(UNIX_EPOCH), "1970-01-01");
    }

    /// View does not panic for any (count, mode) combination.
    #[test]
    fn view_does_not_panic() {
        let cases = [
            CacheSummary::empty(),
            CacheSummary {
                populated_count: 1,
                newest_mtime: Some(UNIX_EPOCH),
            },
            CacheSummary {
                populated_count: 10,
                newest_mtime: Some(UNIX_EPOCH + Duration::from_secs(1_700_000_000)),
            },
        ];
        for summary in &cases {
            let _ = view(summary, ThemeMode::Dark);
            let _ = view(summary, ThemeMode::Light);
        }
    }

    /// The summary prefix carries the operator-locked `"Yahoo "` token —
    /// guards against an accidental copy revert to the analyst's bare
    /// `"Cache: "`.
    #[test]
    fn prefix_constant_has_yahoo_disambiguator() {
        assert!(
            LAB_CACHE_STATE_SUMMARY_PREFIX.starts_with("Yahoo "),
            "operator Q3 lock: prefix must disambiguate with 'Yahoo ' (got {LAB_CACHE_STATE_SUMMARY_PREFIX:?})"
        );
    }
}
