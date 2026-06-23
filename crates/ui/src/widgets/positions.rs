//! Position panel — open positions with cost basis, mark, P&L, exposure (R6.2).
//! Positions with zero qty are hidden; that's the "empty" state copy job.
//!
//! ## T1.1-T1.4 — Brief A R1 native-table migration (2026-05-13, Lane 1)
//!
//! The legacy hand-rolled body — a 7-column `Row::new()` header
//! (`positions.rs:38-46` pre-migration) plus a `Scrollable<Column>` of
//! per-position `Row::new()`s (`positions.rs:48-58` + the `row_for`
//! helper at `positions.rs:62-78` pre-migration) — is replaced by
//! `iced::widget::table::Table`.
//!
//! Per H-arch-A2 REFINED ([`feature.md`](../../../../spec/iced-native-widgets/feature.md)
//! refinement-pass 2026-05-13), `Table::new(columns, rows)` accepts any
//! `IntoIterator<Item = T> where T: Clone`. `PositionView` is `Clone`
//! per [`trading_core::PositionView`](../../../../crates/core/src/views.rs)
//! `:98-99`, so `positions.iter().filter(...).cloned()` flows straight
//! into `Table::new` with no intermediate `Vec`.
//!
//! Per H-arch-A4 RESOLVED-PARTIAL-FALSIFIED, native `Table::new` v0.14
//! does NOT expose a `.style()` builder — the upstream Catalog impl
//! pre-bakes `Theme::default()` at construction time (see
//! `iced_widget-0.14.2/src/table.rs:704-714`). Lane 2's
//! [`crate::theme::iced_widget_catalogs::cockpit_table_style_fn`] factory
//! is consequently **not consumed** here — the cockpit-tinted separator
//! `StyleFn` has no consumer on the native v0.14 `Table` until iced adds
//! a `Table::style(StyleFn)` setter or a `Themer` wrap lands. Positions
//! ships with the default Catalog (palette-derived separator), tracked
//! upstream as a deferred visual-parity item for v0.2.
//!
//! Column alignment per the pre-migration layout: SYMBOL left, the six
//! numeric columns (QTY, COST, MARK, PNL, `PNL_PCT`, EXPOSURE) right via
//! `Column::align_x(alignment::Horizontal::Right)`. Width portions are
//! not set explicitly — `Table::new` auto-promotes the first column to
//! `Length::Fill` (`table.rs:129-133`) when no other column has Fill,
//! and the table-level `.width(Length::Fill)` handles outer fit.

use iced::alignment::Horizontal;
use iced::widget::{Container, Text, table};
use iced::{Border, Element, Length};
use rust_decimal::Decimal;

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    PANEL_POSITIONS_TITLE, POS_COL_COST, POS_COL_DIRECTION, POS_COL_EXPOSURE, POS_COL_MARK,
    POS_COL_PNL, POS_COL_PNL_PCT, POS_COL_QTY, POS_COL_SYMBOL, POS_DIRECTION_LONG,
    POS_DIRECTION_SHORT, POS_EMPTY, POS_ERROR_PREFIX, POS_LOADING,
};
use crate::theme::{ThemeMode, color, color_for_delta, radius, space, text};

use super::frame::{col_header, error_body, loading_with_spinner, muted_body, panel};
use super::num::{fmt_pct, fmt_price, fmt_qty, fmt_usdt_signed};

#[must_use]
pub fn view(model: &Cockpit) -> Element<'_, Message> {
    let body: Element<Message> = match &model.positions {
        PanelState::Loading => loading_with_spinner(POS_LOADING, ThemeMode::Dark),
        PanelState::Empty => muted_body(POS_EMPTY),
        PanelState::Error(e) => error_body(POS_ERROR_PREFIX, e.as_str()),
        PanelState::Ready(positions) => ready_body(positions),
    };
    panel(PANEL_POSITIONS_TITLE, body, ThemeMode::Dark)
}

fn ready_body(positions: &[trading_core::PositionView]) -> Element<'_, Message> {
    // T17 — hide zero-qty positions. If everything is zero we degrade to
    // the empty-state copy (same as pre-migration shape).
    let has_visible = positions.iter().any(|p| !p.base_qty.is_zero());
    if !has_visible {
        return muted_body(POS_EMPTY);
    }

    // T1.1 / H-arch-A2 REFINED: pass the filtered+cloned iterator directly.
    // `PositionView` derives `Clone` (`views.rs:98-99`), so no intermediate
    // `Vec` is required.
    let visible_iter = positions.iter().filter(|p| !p.base_qty.is_zero()).cloned();

    // advisor-short-selling (T-U1 / ADR-0068 § D8) — 8 columns: a DIRECTION
    // badge column is inserted after SYMBOL so a SHORT (signed `base_qty < 0`)
    // reads AS a short, never a malformed long. The signed qty + the
    // (possibly negative) P&L render HONESTLY through the existing `fmt_qty` /
    // `color_for_delta` path — they are NOT clamped.
    let columns = [
        // SYMBOL — left aligned. Becomes the implicit Fill column per
        // `table.rs:129-133` (no other column declares Fill).
        table::column(
            col_header(POS_COL_SYMBOL),
            |p: trading_core::PositionView| cell(p.symbol.0.to_string()),
        ),
        // DIRECTION — LONG / SHORT badge keyed on the sign of `base_qty`
        // (advisor-short-selling, R-SS.9). Left-aligned next to the symbol so
        // the direction is the first thing read after the ticker.
        table::column(
            col_header(POS_COL_DIRECTION),
            |p: trading_core::PositionView| direction_badge(p.base_qty),
        ),
        // QTY — right aligned numeric.
        table::column(col_header(POS_COL_QTY), |p: trading_core::PositionView| {
            cell(fmt_qty(p.base_qty))
        })
        .align_x(Horizontal::Right),
        // COST — right aligned numeric.
        table::column(col_header(POS_COL_COST), |p: trading_core::PositionView| {
            cell(fmt_price(p.cost_basis.amount()))
        })
        .align_x(Horizontal::Right),
        // MARK — right aligned numeric.
        table::column(col_header(POS_COL_MARK), |p: trading_core::PositionView| {
            cell(fmt_price(p.last_mark.get()))
        })
        .align_x(Horizontal::Right),
        // PNL — right aligned, sentiment color (UP / DOWN / FG_1).
        table::column(col_header(POS_COL_PNL), |p: trading_core::PositionView| {
            let pnl_color = color_for_delta(p.pnl.amount());
            colored_cell(fmt_usdt_signed(p.pnl.amount()), pnl_color)
        })
        .align_x(Horizontal::Right),
        // PNL_PCT — right aligned, sentiment color.
        table::column(
            col_header(POS_COL_PNL_PCT),
            |p: trading_core::PositionView| {
                let pnl_pct_color = color_for_delta(p.pnl_pct);
                colored_cell(fmt_pct(p.pnl_pct), pnl_pct_color)
            },
        )
        .align_x(Horizontal::Right),
        // EXPOSURE — right aligned numeric.
        table::column(
            col_header(POS_COL_EXPOSURE),
            |p: trading_core::PositionView| cell(fmt_pct(p.exposure_pct)),
        )
        .align_x(Horizontal::Right),
    ];

    table::Table::new(columns, visible_iter)
        .width(Length::Fill)
        .into()
}

fn cell<'a>(s: String) -> Element<'a, Message> {
    Text::new(s)
        .size(text::BODY)
        .color(color::FG_1.current(ThemeMode::Dark))
        .into()
}

/// LONG / SHORT direction badge keyed on the SIGN of `base_qty`
/// (advisor-short-selling, ADR-0068 § D8). A `base_qty < 0` is a SHORT (a
/// sell-to-open simulated short); anything else is a LONG. The badge is a
/// `PILL`-radius soft-tinted pill carrying the WORD (LONG / SHORT) so colour is
/// never the only signal (accessibility), the same pattern the strategies
/// `status_badge_cell` + the leaderboard `fragile_badge` use. SHORT wears the
/// `DOWN_50` clay backdrop + `DOWN_500` label (the down-half treatment); LONG
/// wears a quiet `ACCENT_SOFT` backdrop + `FG_2` label so the short pops as the
/// notable case without making every long row shout.
// The `space::* as u16` padding casts are bounded design tokens (4/6) — safe;
// the same fn-level allow the strategies `status_badge_cell` uses.
#[allow(clippy::cast_possible_truncation)]
fn direction_badge<'a>(base_qty: Decimal) -> Element<'a, Message> {
    let mode = ThemeMode::Dark;
    let (label, background, fg) = if base_qty.is_sign_negative() {
        (
            POS_DIRECTION_SHORT,
            color::DOWN_50.current(mode),
            color::DOWN_500.current(mode),
        )
    } else {
        (
            POS_DIRECTION_LONG,
            color::ACCENT_SOFT.current(mode),
            color::FG_2.current(mode),
        )
    };
    Container::new(Text::new(label).size(text::SMALL).color(fg))
        .padding([space::XXS as u16, space::XS as u16])
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(background.into()),
            border: Border {
                color: iced::Color::TRANSPARENT,
                width: 0.0,
                radius: radius::PILL.into(),
            },
            text_color: Some(fg),
            ..Default::default()
        })
        .into()
}

fn colored_cell<'a>(s: String, c: iced::Color) -> Element<'a, Message> {
    Text::new(s).size(text::BODY).color(c).into()
}

// Kept available for an exposure-pill or column sorting hook later.
#[allow(dead_code)]
fn warn_if_over<'a>(s: String, value: Decimal, cap: Decimal) -> Element<'a, Message> {
    let c = if value > cap {
        color::WARN_500.current(ThemeMode::Dark)
    } else {
        color::FG_1.current(ThemeMode::Dark)
    };
    Text::new(s).size(text::BODY).color(c).into()
}
