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
//! StyleFn has no consumer on the native v0.14 `Table` until iced adds
//! a `Table::style(StyleFn)` setter or a `Themer` wrap lands. Positions
//! ships with the default Catalog (palette-derived separator), tracked
//! upstream as a deferred visual-parity item for v0.2.
//!
//! Column alignment per the pre-migration layout: SYMBOL left, the six
//! numeric columns (QTY, COST, MARK, PNL, PNL_PCT, EXPOSURE) right via
//! `Column::align_x(alignment::Horizontal::Right)`. Width portions are
//! not set explicitly — `Table::new` auto-promotes the first column to
//! `Length::Fill` (`table.rs:129-133`) when no other column has Fill,
//! and the table-level `.width(Length::Fill)` handles outer fit.

use iced::alignment::Horizontal;
use iced::widget::{table, Text};
use iced::{Element, Length};
use rust_decimal::Decimal;

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    PANEL_POSITIONS_TITLE, POS_COL_COST, POS_COL_EXPOSURE, POS_COL_MARK, POS_COL_PNL,
    POS_COL_PNL_PCT, POS_COL_QTY, POS_COL_SYMBOL, POS_EMPTY, POS_ERROR_PREFIX, POS_LOADING,
};
use crate::theme::{color, color_for_delta, text, ThemeMode};

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

    // T1.1 / T1.2 — 7 columns, alignment per pre-migration layout.
    let columns = [
        // SYMBOL — left aligned. Becomes the implicit Fill column per
        // `table.rs:129-133` (no other column declares Fill).
        table::column(
            col_header(POS_COL_SYMBOL),
            |p: trading_core::PositionView| cell(p.symbol.0.to_string()),
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
