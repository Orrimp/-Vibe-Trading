//! Audit / Journal screen — Phase 3 (T1709, T1710, T1711).
//!
//! Layout (Phase 3 Design § Audit / Journal screen contract):
//!
//! 1. **Filter row** — venue chips (multi-select) + symbol input +
//!    kind chips + time-range chips. Active chips use
//!    `frame::active_chip` for the T1609 bottom-edge rule.
//! 2. **Pagination header** — `widgets::num`-rendered "Showing N–M of T"
//!    + Prev / Next buttons. Fixed 250 rows / page (Q4 — `AUDIT_PAGE_SIZE` in `theme::layout`).
//! 3. **Journal table** — newest-first rows from
//!    `audit_screen_state.rows` (when `Ready`). Columns: timestamp,
//!    venue, symbol, kind, description, `strategy_id`. Per-row click
//!    emits `Message::TapeRowClicked(row.tx_id.clone())` — the literal
//!    Phase 1 variant per R11.4 / Q11 (T1711 reuse).
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

// `cast_possible_truncation`: `space::*` constants are `u32` with bounded
// values 0..64; cast to `u16` for iced padding is safe by construction.
// `elidable_lifetime_names`: explicit `'a` lifetimes document the
// borrow chain through helper functions.
#![allow(
    clippy::cast_possible_truncation,
    clippy::elidable_lifetime_names,
    clippy::needless_pass_by_value
)]

use iced::widget::{button, Button, Column, Container, Row, Text};
use iced::{Border, Length};

use crate::state::{
    AuditFilter, AuditKindFilter, AuditKindLabel, AuditTimeRange, Cockpit, JournalRow, Message,
    PanelState,
};
use crate::strings::{
    AUDIT_COL_DESCRIPTION, AUDIT_COL_KIND, AUDIT_COL_STRATEGY_ID, AUDIT_COL_SYMBOL, AUDIT_COL_TIME,
    AUDIT_COL_VENUE, AUDIT_FILTER_KIND_LABEL, AUDIT_FILTER_NO_MATCH, AUDIT_FILTER_SYMBOL_LABEL,
    AUDIT_FILTER_TIME_LABEL, AUDIT_FILTER_VENUE_LABEL, AUDIT_KIND_ALL, AUDIT_KIND_FILL,
    AUDIT_KIND_RECONCILIATION, AUDIT_KIND_STRATEGY_EVENT, AUDIT_LOADING, AUDIT_NEXT_LABEL,
    AUDIT_PANEL_TITLE, AUDIT_PREV_LABEL, AUDIT_QUERY_FAILED_PREFIX, AUDIT_TIME_LAST_1H,
    AUDIT_TIME_LAST_24H, AUDIT_TIME_LAST_7D, PLACEHOLDER_NONE,
};
use crate::theme::layout::AUDIT_PAGE_SIZE;
use crate::theme::{color, layout, radius, space, text, ThemeMode};
use crate::widgets::frame::{self, active_chip, col_header, muted_body, panel};

/// Render the Audit / Journal screen body.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let filter_row = filter_row_section(&model.audit_screen_state.filter, mode);
    let pagination = pagination_header(model, mode);
    let table = match &model.audit_screen_state.rows {
        PanelState::Loading => muted_body(AUDIT_LOADING),
        PanelState::Empty => muted_body(AUDIT_FILTER_NO_MATCH),
        PanelState::Error(e) => frame::error_body(AUDIT_QUERY_FAILED_PREFIX, e.as_str()),
        PanelState::Ready(rows) => {
            if rows.is_empty() {
                muted_body(AUDIT_FILTER_NO_MATCH)
            } else {
                table_body(rows, mode)
            }
        }
    };

    let inner = Column::new()
        .spacing(space::M)
        .push(filter_row)
        .push(pagination)
        .push(table);

    let panel_body = Container::new(inner)
        .width(Length::Fill)
        .padding(layout::PANEL_PADDING as u16);

    Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(panel(AUDIT_PANEL_TITLE, panel_body.into(), mode))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn filter_row_section<'a>(filter: &'a AuditFilter, mode: ThemeMode) -> iced::Element<'a, Message> {
    let venue_row = labeled_row(AUDIT_FILTER_VENUE_LABEL, venue_chips(filter, mode), mode);
    let symbol_row = labeled_row(AUDIT_FILTER_SYMBOL_LABEL, symbol_value(filter, mode), mode);
    let kind_row = labeled_row(AUDIT_FILTER_KIND_LABEL, kind_chips(filter, mode), mode);
    let time_row = labeled_row(AUDIT_FILTER_TIME_LABEL, time_chips(filter, mode), mode);
    Column::new()
        .spacing(space::S)
        .push(venue_row)
        .push(symbol_row)
        .push(kind_row)
        .push(time_row)
        .into()
}

fn labeled_row<'a>(
    label: &'static str,
    body: iced::Element<'a, Message>,
    mode: ThemeMode,
) -> iced::Element<'a, Message> {
    let label_text = Text::new(label)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    Row::new()
        .spacing(space::M)
        .push(Container::new(label_text).width(Length::Fixed(120.0)))
        .push(Container::new(body).width(Length::Fill))
        .into()
}

fn venue_chips<'a>(filter: &'a AuditFilter, mode: ThemeMode) -> iced::Element<'a, Message> {
    use trading_core::Venue;
    let mut row = Row::new().spacing(space::S);
    for venue in [Venue::Binance, Venue::Coinbase, Venue::Kraken] {
        let active = filter.venues.contains(&venue);
        let chip = make_chip(venue.to_string(), active, mode, {
            let filter = filter.clone();
            let mut new_venues = filter.venues.clone();
            if active {
                new_venues.retain(|v| *v != venue);
            } else {
                new_venues.push(venue);
            }
            Message::AuditFilterChanged(filter.with_venues(new_venues))
        });
        row = row.push(active_chip(chip, active, mode));
    }
    row.into()
}

fn symbol_value<'a>(filter: &'a AuditFilter, mode: ThemeMode) -> iced::Element<'a, Message> {
    // Phase 3 ships read display only — no operator-write text input
    // wired to messaging in this pass. The shape is a sunken-styled
    // container ready for a `text_input` swap-in.
    let value = filter
        .symbol
        .as_ref()
        .map_or_else(|| PLACEHOLDER_NONE.to_string(), |s| s.0.to_string());
    Container::new(
        Text::new(value)
            .size(text::SMALL)
            .color(color::FG_2.current(mode)),
    )
    .padding([space::XS as u16, space::M as u16])
    .style(move |_theme: &iced::Theme| iced::widget::container::Style {
        background: Some(color::PANEL_SUNKEN.current(mode).into()),
        border: Border {
            radius: radius::R2.into(),
            ..Default::default()
        },
        ..Default::default()
    })
    .into()
}

fn kind_chips<'a>(filter: &'a AuditFilter, mode: ThemeMode) -> iced::Element<'a, Message> {
    let mut row = Row::new().spacing(space::S);
    for (label, kind) in [
        (AUDIT_KIND_ALL, AuditKindFilter::All),
        (AUDIT_KIND_FILL, AuditKindFilter::Fill),
        (AUDIT_KIND_STRATEGY_EVENT, AuditKindFilter::StrategyEvent),
        (AUDIT_KIND_RECONCILIATION, AuditKindFilter::Reconciliation),
    ] {
        let active = filter.kind == kind;
        let chip = make_chip(
            label.to_string(),
            active,
            mode,
            Message::AuditFilterChanged(filter.with_kind(kind)),
        );
        row = row.push(active_chip(chip, active, mode));
    }
    row.into()
}

fn time_chips<'a>(filter: &'a AuditFilter, mode: ThemeMode) -> iced::Element<'a, Message> {
    let mut row = Row::new().spacing(space::S);
    for (label, range) in [
        (AUDIT_TIME_LAST_1H, AuditTimeRange::Last1H),
        (AUDIT_TIME_LAST_24H, AuditTimeRange::Last24H),
        (AUDIT_TIME_LAST_7D, AuditTimeRange::Last7D),
    ] {
        let active = filter.time_range == range;
        let chip = make_chip(
            label.to_string(),
            active,
            mode,
            Message::AuditFilterChanged(filter.with_time_range(range)),
        );
        row = row.push(active_chip(chip, active, mode));
    }
    row.into()
}

fn make_chip<'a>(
    label: String,
    active: bool,
    mode: ThemeMode,
    on_press: Message,
) -> iced::Element<'a, Message> {
    let label_widget = Text::new(label).size(text::SMALL).color(if active {
        color::FG_1.current(mode)
    } else {
        color::FG_2.current(mode)
    });
    Button::new(label_widget)
        .on_press(on_press)
        .padding([space::XS as u16, space::M as u16])
        .style(move |_theme: &iced::Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: if active {
                    color::FG_1.current(mode)
                } else {
                    color::FG_2.current(mode)
                },
                border: Border {
                    radius: radius::R3.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn pagination_header<'a>(model: &'a Cockpit, mode: ThemeMode) -> iced::Element<'a, Message> {
    let page = model.audit_screen_state.page;
    let total = model.audit_screen_state.total_count.unwrap_or(0);
    let start = u64::from(page) * u64::from(AUDIT_PAGE_SIZE) + 1;
    let end = ((u64::from(page) + 1) * u64::from(AUDIT_PAGE_SIZE)).min(total);
    let summary = if total == 0 {
        "Showing 0–0 of 0".to_string()
    } else {
        format!("Showing {start}–{end} of {total}")
    };
    let summary_text = Text::new(summary)
        .size(text::SMALL)
        .color(color::FG_2.current(mode));

    let prev_disabled = page == 0;
    let next_disabled = (u64::from(page) + 1) * u64::from(AUDIT_PAGE_SIZE) >= total;

    let prev_button = pagination_button(
        AUDIT_PREV_LABEL,
        !prev_disabled,
        mode,
        page.saturating_sub(1),
    );
    let next_button = pagination_button(
        AUDIT_NEXT_LABEL,
        !next_disabled,
        mode,
        page.saturating_add(1),
    );

    Row::new()
        .spacing(space::M)
        .push(Container::new(summary_text).width(Length::Fill))
        .push(prev_button)
        .push(next_button)
        .into()
}

fn pagination_button<'a>(
    label: &'static str,
    enabled: bool,
    mode: ThemeMode,
    target_page: u32,
) -> iced::Element<'a, Message> {
    let label_widget = Text::new(label).size(text::SMALL).color(if enabled {
        color::FG_1.current(mode)
    } else {
        color::FG_4.current(mode)
    });
    let mut btn = Button::new(label_widget).padding([space::XS as u16, space::M as u16]);
    if enabled {
        btn = btn.on_press(Message::AuditPageChanged(target_page));
    }
    btn.style(move |_theme: &iced::Theme, status: button::Status| {
        let bg = if enabled {
            match status {
                button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
                _ => None,
            }
        } else {
            None
        };
        button::Style {
            background: bg,
            text_color: if enabled {
                color::FG_1.current(mode)
            } else {
                color::FG_4.current(mode)
            },
            border: Border {
                radius: radius::R3.into(),
                ..Default::default()
            },
            ..Default::default()
        }
    })
    .into()
}

fn table_body<'a>(rows: &'a [JournalRow], mode: ThemeMode) -> iced::Element<'a, Message> {
    let header = Row::new()
        .spacing(space::M)
        .push(col_header(AUDIT_COL_TIME))
        .push(col_header(AUDIT_COL_VENUE))
        .push(col_header(AUDIT_COL_SYMBOL))
        .push(col_header(AUDIT_COL_KIND))
        .push(col_header(AUDIT_COL_DESCRIPTION))
        .push(col_header(AUDIT_COL_STRATEGY_ID));
    let mut col = Column::new().spacing(space::XS).push(header);
    for r in rows {
        let row = row_for(r, mode);
        col = col.push(row);
    }
    col.into()
}

fn row_for<'a>(r: &'a JournalRow, mode: ThemeMode) -> iced::Element<'a, Message> {
    let ts = format!(
        "{:02}:{:02}:{:02}",
        r.ts.inner().hour(),
        r.ts.inner().minute(),
        r.ts.inner().second(),
    );
    let venue = r.venue.to_string();
    let symbol = r
        .symbol
        .as_ref()
        .map_or_else(|| PLACEHOLDER_NONE.to_string(), |s| s.0.to_string());
    let kind_label = match r.kind {
        AuditKindLabel::Fill => AUDIT_KIND_FILL,
        AuditKindLabel::StrategyEvent => AUDIT_KIND_STRATEGY_EVENT,
        AuditKindLabel::Reconciliation => AUDIT_KIND_RECONCILIATION,
    };
    let description = r.description.to_string();
    let strategy_id = r
        .strategy_id
        .as_ref()
        .map_or_else(|| PLACEHOLDER_NONE.to_string(), ToString::to_string);

    let row_content = Row::new()
        .spacing(space::M)
        .push(cell(ts, mode))
        .push(cell(venue, mode))
        .push(cell(symbol, mode))
        .push(cell(kind_label.to_string(), mode))
        .push(cell(description, mode))
        .push(cell(strategy_id, mode));

    Button::new(row_content)
        .on_press(Message::TapeRowClicked(r.tx_id.clone()))
        .padding([space::XS as u16, space::S as u16])
        .style(move |_theme: &iced::Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: color::FG_1.current(mode),
                border: Border {
                    radius: radius::R2.into(),
                    ..Default::default()
                },
                ..Default::default()
            }
        })
        .into()
}

fn cell<'a>(s: String, mode: ThemeMode) -> iced::Element<'a, Message> {
    Text::new(s)
        .size(text::SMALL)
        .color(color::FG_1.current(mode))
        .into()
}
