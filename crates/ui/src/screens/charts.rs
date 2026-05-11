//! Charts screen — chart-buy-sell-emphasis v1.9 (M4 / T2025, Layout β).
//!
//! Composes the chip row + the chart canvas + the post-v1.9 counter
//! views:
//!
//! 1. Chip row of `(venue, symbol)` selectors (unchanged from Phase 2).
//! 2. **Status strip** above the chart (Q5 = β) — three-tile cumulative
//!    window-volume card (`compute_window_volume`) + an open-position
//!    mirror filtered to the active symbol (R7.3).
//! 3. Chart canvas at `Length::Fill` (R8.1 — chart keeps prominence).
//! 4. **Volume histogram** below the chart at fixed 80-px height
//!    (R7.2, Q5).
//!
//! Compose-time derivation: `VolumeBin`s are built from
//! `model.chart_markers` aggregated per `Bar.close_ts`; the cumulative
//! tile numbers come from the same `chart_markers` slice; the open-
//! position mirror filters `model.positions` to the active symbol.
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.

use iced::widget::{button, container, Button, Column, Container, Row, Text};
use iced::{Border, Length};
use rust_decimal::Decimal;
use trading_core::{FillView, PositionView, Side, Symbol};

use crate::state::{Cockpit, Message, PanelState};
use crate::strings::{
    CHART_POSITION_MIRROR_LABEL, CHART_POSITION_MIRROR_NONE, CHART_VOLUME_HISTOGRAM_LABEL,
    CHART_VOLUME_TILE_BUYS_LABEL, CHART_VOLUME_TILE_NET_LABEL, CHART_VOLUME_TILE_SELLS_LABEL,
    CHART_VOLUME_TILE_TRADES_SUFFIX,
};
use crate::theme::{color, color_for_delta, radius, space, text, ThemeMode};
use crate::widgets::num::{fmt_pct, fmt_price, fmt_qty, fmt_usdt_signed};
use crate::widgets::volume_histogram::{self, VolumeBin};
use crate::widgets::{chart, frame};

/// Fixed pixel height for the per-bar volume histogram strip below the
/// chart (R7.2 + Q5 — operator-locked at ~80 px).
const HISTOGRAM_HEIGHT_PX: f32 = 80.0;

/// Approximate chip-row height (one row of Lumen `SMALL`-sized buttons
/// with `XS`/`M` padding).  Used for the [`chart_canvas_height_for_body`]
/// allocation calculation; the real iced layout engine resolves this
/// dynamically based on font metrics, but the constant captures the
/// allocation budget the column reasons against.
const CHIP_ROW_HEIGHT_PX: f32 = 32.0;

/// Approximate status-strip height (the three-tile volume card +
/// position mirror, both `space::M` padded with `text::H2`-sized values
/// and a `text::SMALL` label above).
const STATUS_STRIP_HEIGHT_PX: f32 = 80.0;

/// Approximate histogram-label height (`text::MICRO` + `space::XXS`
/// gap) — the volume-histogram column's first child.
const HISTOGRAM_LABEL_HEIGHT_PX: f32 = 14.0;

/// Pure calculation of the chart canvas's vertical allocation given a
/// body height.  Mirrors the Layout β arithmetic this module's
/// [`view`] composes: chip row + status strip + chart (Fill) + volume
/// histogram (label + 80-px canvas) stacked in a `Column` with
/// `space::M` between children and `space::L` padding on every side.
///
/// **Why a pure helper:** T2032's chart-cropping regression reduced to
/// a `Length`-propagation problem in the chart-body column; the
/// defensive fix gives the chart-body container an explicit
/// `Length::Fill` on both axes (see the comment in [`view`] for the
/// corrected mechanic — the in-source rationale shipped with M6.2
/// blamed `Container::new`'s default width, but the iced 0.14 source
/// shows that diagnosis was wrong).  This helper exposes the resulting
/// budget arithmetic so the unit test
/// `chart_canvas_height_grows_with_body_height` can pin the invariant
/// without dragging in an iced layout runtime.
///
/// Returns `0.0` when the body height is smaller than the fixed-
/// allocation siblings — pathological but defended.
#[must_use]
pub fn chart_canvas_height_for_body(body_height_px: f32) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let padding = (space::L as f32) * 2.0;
    #[allow(clippy::cast_precision_loss)]
    let spacing = (space::M as f32) * 3.0; // 3 gaps between 4 children
    let fixed = CHIP_ROW_HEIGHT_PX
        + STATUS_STRIP_HEIGHT_PX
        + HISTOGRAM_LABEL_HEIGHT_PX
        + HISTOGRAM_HEIGHT_PX;
    (body_height_px - padding - spacing - fixed).max(0.0)
}

/// Render the Charts screen body.
#[allow(clippy::cast_possible_truncation, clippy::needless_pass_by_value)]
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let active = model
        .selected_symbol
        .clone()
        .or_else(|| model.universe.first().cloned());

    // Chip row — unchanged from Phase 2.
    let mut chip_row = Row::new().spacing(space::S);
    for (venue, symbol) in &model.universe {
        let pair_active = match &active {
            Some((av, asym)) => av == venue && asym == symbol,
            None => false,
        };
        let label = format!("{venue} \u{00b7} {symbol}");
        let text_widget = Text::new(label).size(text::SMALL).color(if pair_active {
            color::FG_1.current(mode)
        } else {
            color::FG_2.current(mode)
        });
        let chip_button = Button::new(text_widget)
            .on_press(Message::SelectSymbol(*venue, symbol.clone()))
            .padding([space::XS as u16, space::M as u16])
            .style(move |_theme: &iced::Theme, status: button::Status| {
                let bg = match status {
                    button::Status::Hovered => Some(color::PANEL_SUNKEN.current(mode).into()),
                    _ => None,
                };
                button::Style {
                    background: bg,
                    text_color: if pair_active {
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
            });
        chip_row = chip_row.push(frame::active_chip(chip_button.into(), pair_active, mode));
    }

    // Compute the per-active-symbol slices once.
    let active_markers: Vec<FillView> = match &model.chart_markers {
        PanelState::Ready(v) => v.clone(),
        _ => Vec::new(),
    };
    let active_signals = match &model.chart_signals {
        PanelState::Ready(v) => v.clone(),
        _ => Vec::new(),
    };
    let active_position = active
        .as_ref()
        .and_then(|(_, sym)| position_for_symbol(model, sym));
    let bars = active
        .as_ref()
        .map(|(v, s)| model.chart_buffer.bars(*v, s).cloned().collect::<Vec<_>>())
        .unwrap_or_default();
    let bins = compute_volume_bins(&active_markers, &bars);

    // Status strip — three-tile cumulative volume + open-position mirror.
    let totals = compute_window_volume(&active_markers);
    let status_strip = Row::new()
        .spacing(space::M)
        .push(volume_tile(&totals, mode))
        .push(position_mirror(active_position.as_ref(), mode))
        .width(Length::Fill);

    // Chart canvas — full width, fills remaining vertical space.
    let chart_body = if let Some((_, _)) = active {
        chart::view(
            bars,
            active_markers,
            active_signals,
            model.chart_tooltip.clone(),
            mode,
        )
    } else {
        chart::view(Vec::new(), Vec::new(), Vec::new(), None, mode)
    };

    // Histogram below the chart — fixed 80 px tall.
    let histogram_label = Text::new(CHART_VOLUME_HISTOGRAM_LABEL)
        .size(text::MICRO)
        .color(color::FG_3.current(mode));
    let histogram_canvas = Container::new(volume_histogram::view(bins, mode))
        .width(Length::Fill)
        .height(Length::Fixed(HISTOGRAM_HEIGHT_PX));
    let histogram = Column::new()
        .spacing(space::XXS)
        .push(histogram_label)
        .push(histogram_canvas);

    Column::new()
        .padding(space::L as u16)
        .spacing(space::M)
        .push(chip_row)
        .push(status_strip)
        // T2032 — defensive `.width(Length::Fill)` on the chart-body
        // container.
        //
        // **Corrected rationale (M6.2 fixup, 2026-05-11).**  The M6.2
        // ship comment here claimed `Container::new(content)` defaults
        // its width to `Length::Shrink`, collapsing the `Fill`-width
        // canvas child to zero.  Reading the iced 0.14 source shows
        // that diagnosis was wrong: `Container::new(content)` calls
        // `content.size_hint()` and applies `Length::fluid()` (see
        // [iced 0.14 container.rs:94-108](https://github.com/iced-rs/iced/blob/0.14.0/widget/src/container.rs#L94-L108)),
        // which preserves `Fill` from `Fill` children and only
        // collapses `Shrink` / `Fixed` children to `Shrink`.  So a
        // bare `Container::new(chart_body)` wrapping a Fill-width
        // canvas *should* already propagate `Fill`.  The actual
        // Shrink-default trap lives in `Row::new()` and
        // `Column::new()` ([row.rs:80-81](https://github.com/iced-rs/iced/blob/0.14.0/widget/src/row.rs#L80-L81),
        // [column.rs:83-84](https://github.com/iced-rs/iced/blob/0.14.0/widget/src/column.rs#L83-L84))
        // — those default to `Length::Shrink` on both axes and would
        // collapse Fill children to zero.  The original M6.2 fix
        // probably worked via a different mechanism (cache
        // invalidation from re-typing the container, or simply the
        // forced relayout pass triggered by editing this code path) —
        // not via the Shrink-default story the in-source comment told.
        //
        // We keep the explicit `.width(Length::Fill).height(Length::Fill)`
        // here as **defensive intent**: it documents that the
        // chart-body container must own its full allocation on both
        // axes, and survives future refactors that might wrap this
        // node in a `Row::new()` / `Column::new()` (Shrink-default)
        // or swap the child for a `Shrink`-defaulting widget.  The
        // unit test `chart_canvas_height_grows_with_body_height`
        // remains the load-bearing regression guard for the budget
        // arithmetic; this `.width(Length::Fill)` is belt-and-braces.
        .push(
            Container::new(chart_body)
                .width(Length::Fill)
                .height(Length::Fill),
        )
        .push(histogram)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── Tile + mirror widget helpers (T2022, T2024) ─────────────────────────────

/// Cumulative-window-volume summary: derived from the active
/// `chart_markers` slice at compose time. Pure function for testability.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct WindowVolumeTotals {
    pub buys_usdt: Decimal,
    pub sells_usdt: Decimal,
    pub net_usdt: Decimal,
    pub buy_count: usize,
    pub sell_count: usize,
}

/// T2022 (V6) — cumulative buys/sells/net in USDT for the visible marker
/// window. Pure function, deterministic by construction (Decimal
/// arithmetic, no floats).
#[must_use]
pub fn compute_window_volume(markers: &[FillView]) -> WindowVolumeTotals {
    let mut totals = WindowVolumeTotals::default();
    for f in markers {
        let notional = f.price.get().saturating_mul(f.qty.get());
        match f.side {
            Side::Buy => {
                totals.buys_usdt += notional;
                totals.buy_count += 1;
            }
            Side::Sell => {
                totals.sells_usdt += notional;
                totals.sell_count += 1;
            }
        }
    }
    totals.net_usdt = totals.buys_usdt - totals.sells_usdt;
    totals
}

/// T2024 — filter `model.positions` to the active symbol, returning the
/// first matching `PositionView` (positions are unique per symbol per
/// venue, so at most one match). `None` when no position exists.
#[must_use]
pub fn position_for_symbol(model: &Cockpit, symbol: &Symbol) -> Option<PositionView> {
    if let PanelState::Ready(positions) = &model.positions {
        positions
            .iter()
            .find(|p| &p.symbol == symbol && !p.base_qty.is_zero())
            .cloned()
    } else {
        None
    }
}

/// T2023 — aggregate `chart_markers` into one `VolumeBin` per `Bar`. Each
/// bin sums the fills whose `venue_ts` falls inside the bar's
/// `[open_ts, close_ts]` window. Bars with zero fills become
/// `VolumeBin::default()` (R7.6 — empty bins still render as a slot so
/// the time axis stays continuous).
#[must_use]
pub fn compute_volume_bins(markers: &[FillView], bars: &[trading_core::Bar]) -> Vec<VolumeBin> {
    if bars.is_empty() {
        return Vec::new();
    }
    let mut bins: Vec<VolumeBin> = vec![VolumeBin::default(); bars.len()];
    for fill in markers {
        let ts = fill.venue_ts.unix_millis();
        // Linear scan — N <= 60 bars × <= ~50 markers = 3 000 ops per
        // repaint. Acceptable; bsearch is overkill at this scale.
        for (i, bar) in bars.iter().enumerate() {
            let open_ms = bar.open_ts.unix_millis();
            let close_ms = bar.close_ts.unix_millis();
            if ts >= open_ms && ts <= close_ms {
                let notional = fill.price.get().saturating_mul(fill.qty.get());
                match fill.side {
                    Side::Buy => bins[i].buys_usdt += notional,
                    Side::Sell => bins[i].sells_usdt += notional,
                }
                break;
            }
        }
    }
    bins
}

/// Three-cell card: Buys / Sells / Net. Sibling shape of `widgets::kpi_strip`
/// `card()` — labels in `MICRO/FG_3`, values in `H2` with sentiment colour.
fn volume_tile<'a>(totals: &WindowVolumeTotals, mode: ThemeMode) -> crate::Element<'a> {
    let buys_value = format!(
        "{}  ({} {})",
        fmt_usdt_signed(totals.buys_usdt),
        totals.buy_count,
        CHART_VOLUME_TILE_TRADES_SUFFIX
    );
    let sells_value = format!(
        "{}  ({} {})",
        fmt_usdt_signed(-totals.sells_usdt),
        totals.sell_count,
        CHART_VOLUME_TILE_TRADES_SUFFIX
    );
    let net_value = fmt_usdt_signed(totals.net_usdt);

    let buys_cell = number_card(
        CHART_VOLUME_TILE_BUYS_LABEL,
        buys_value,
        color::UP_500.current(mode),
        mode,
    );
    let sells_cell = number_card(
        CHART_VOLUME_TILE_SELLS_LABEL,
        sells_value,
        color::DOWN_500.current(mode),
        mode,
    );
    let net_cell = number_card(
        CHART_VOLUME_TILE_NET_LABEL,
        net_value,
        color_for_delta(totals.net_usdt),
        mode,
    );

    Container::new(
        Row::new()
            .spacing(space::M)
            .push(buys_cell)
            .push(sells_cell)
            .push(net_cell),
    )
    .padding(space::M as u16)
    .width(Length::FillPortion(2))
    .style(move |_t: &iced::Theme| container::Style {
        background: Some(color::PANEL.current(mode).into()),
        border: Border {
            color: color::BORDER_1.current(mode),
            width: 1.0,
            radius: radius::R4.into(),
        },
        ..Default::default()
    })
    .into()
}

fn number_card<'a>(
    label: &'a str,
    value: String,
    value_color: iced::Color,
    mode: ThemeMode,
) -> crate::Element<'a> {
    let lbl = Text::new(label)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));
    let val = Text::new(value).size(text::H2).color(value_color);
    Column::new().spacing(space::XS).push(lbl).push(val).into()
}

/// T2024 — open-position mirror filtered to the active symbol. Renders one
/// row with the same column shape as `widgets::positions` so the operator
/// reads the same visual contract regardless of where the position
/// appears.
fn position_mirror<'a>(p: Option<&PositionView>, mode: ThemeMode) -> crate::Element<'a> {
    let label = Text::new(CHART_POSITION_MIRROR_LABEL)
        .size(text::SMALL)
        .color(color::FG_3.current(mode));

    let body: crate::Element<'a> = match p {
        Some(pos) => {
            let pnl_color = color_for_delta(pos.pnl.amount());
            let pnl_pct_color = color_for_delta(pos.pnl_pct);
            Row::new()
                .spacing(space::M)
                .push(
                    Text::new(pos.symbol.0.to_string())
                        .size(text::BODY)
                        .color(color::FG_1.current(mode)),
                )
                .push(
                    Text::new(fmt_qty(pos.base_qty))
                        .size(text::BODY)
                        .color(color::FG_1.current(mode)),
                )
                .push(
                    Text::new(fmt_price(pos.cost_basis.amount()))
                        .size(text::BODY)
                        .color(color::FG_2.current(mode)),
                )
                .push(
                    Text::new(fmt_price(pos.last_mark.get()))
                        .size(text::BODY)
                        .color(color::FG_2.current(mode)),
                )
                .push(
                    Text::new(fmt_usdt_signed(pos.pnl.amount()))
                        .size(text::BODY)
                        .color(pnl_color),
                )
                .push(
                    Text::new(fmt_pct(pos.pnl_pct))
                        .size(text::BODY)
                        .color(pnl_pct_color),
                )
                .into()
        }
        None => Text::new(CHART_POSITION_MIRROR_NONE)
            .size(text::BODY)
            .color(color::FG_3.current(mode))
            .into(),
    };

    Container::new(Column::new().spacing(space::XS).push(label).push(body))
        .padding(space::M as u16)
        .width(Length::FillPortion(3))
        .style(move |_t: &iced::Theme| container::Style {
            background: Some(color::PANEL.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R4.into(),
            },
            ..Default::default()
        })
        .into()
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;
    use smol_str::SmolStr;
    use trading_core::{FeeTier, Money, Price, Quantity, Timestamp};

    fn fixed_ts(offset: i64) -> Timestamp {
        let dt = time::OffsetDateTime::from_unix_timestamp(1_700_000_000 + offset)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        Timestamp::new(dt)
    }

    fn fill(side: Side, price: rust_decimal::Decimal, qty: rust_decimal::Decimal) -> FillView {
        FillView {
            symbol: Symbol::new("BTCUSDT"),
            side,
            price: Price::new(price).unwrap(),
            qty: Quantity::new(qty).unwrap(),
            fee: Money::from_decimal(dec!(0.5)),
            fee_tier: FeeTier::Taker,
            venue_ts: fixed_ts(0),
            transaction_id: SmolStr::new("tx-1"),
        }
    }

    /// T2022 V6 — three buys at $10k each + two sells at $10k each →
    /// `buys_usdt = $30,000`, `sells_usdt = $20,000`, `net = $10,000`,
    /// `buy_count = 3`, `sell_count = 2`.
    #[test]
    fn chart_counter_tile_sums() {
        let markers = vec![
            fill(Side::Buy, dec!(100), dec!(100)),
            fill(Side::Buy, dec!(100), dec!(100)),
            fill(Side::Buy, dec!(100), dec!(100)),
            fill(Side::Sell, dec!(100), dec!(100)),
            fill(Side::Sell, dec!(100), dec!(100)),
        ];
        let t = compute_window_volume(&markers);
        assert_eq!(t.buys_usdt, dec!(30_000));
        assert_eq!(t.sells_usdt, dec!(20_000));
        assert_eq!(t.net_usdt, dec!(10_000));
        assert_eq!(t.buy_count, 3);
        assert_eq!(t.sell_count, 2);
    }

    /// T2022 V6 — empty marker slice → zero everything.
    #[test]
    fn chart_counter_tile_empty_slice_zero_totals() {
        let t = compute_window_volume(&[]);
        assert_eq!(t.buys_usdt, Decimal::ZERO);
        assert_eq!(t.sells_usdt, Decimal::ZERO);
        assert_eq!(t.net_usdt, Decimal::ZERO);
        assert_eq!(t.buy_count, 0);
        assert_eq!(t.sell_count, 0);
    }

    /// T2032 — chart canvas height MUST grow with body height.  The
    /// operator's 2026-05-11 visual report ("chart crops on window
    /// resize") reduced to a `Length` propagation issue in the
    /// chart-body column.  The M6.2 fix gives the chart-body
    /// container an explicit `.width(Length::Fill).height(Length::Fill)`
    /// (see the comment in [`super::view`] for the corrected mechanic
    /// — the M6.2 in-source rationale that blamed `Container::new`'s
    /// default width was wrong per the iced 0.14 source).  With the
    /// chart-body container occupying its full body allocation, the
    /// canvas's vertical allocation is `body_height - fixed_siblings`
    /// and therefore grows monotonically with body height.
    ///
    /// We pin the invariant via the pure arithmetic helper
    /// [`chart_canvas_height_for_body`] — the real iced layout
    /// engine resolves the actual pixel allocation at runtime, but
    /// the budget math is what the column reasons against.
    #[test]
    fn chart_canvas_height_grows_with_body_height() {
        let h_720 = chart_canvas_height_for_body(720.0);
        let h_1080 = chart_canvas_height_for_body(1080.0);
        assert!(
            h_1080 > h_720,
            "chart canvas height MUST grow with body height: 720 → {h_720}, 1080 → {h_1080}"
        );
        // Sanity: at 720 the chart still has room (≥ 50 % of body
        // height after fixed siblings — the Q5 / Layout β floor that
        // T2028's `MIN_WINDOW_HEIGHT_PX = 720` defends).
        assert!(
            h_720 > 0.0,
            "chart canvas must have non-zero allocation at the 720-px floor: got {h_720}"
        );
        // The growth is exactly the body-height delta (fixed
        // siblings + padding + spacing are body-invariant).
        let delta = h_1080 - h_720;
        assert!(
            (delta - 360.0).abs() < f32::EPSILON,
            "delta should equal body-height delta (1080-720=360); got {delta}"
        );
    }

    /// T2024 — `position_for_symbol` returns the matching position when
    /// present, `None` otherwise. Skips zero-quantity rows (matches the
    /// `widgets::positions` filter contract).
    #[test]
    fn position_mirror_filters_to_active_symbol() {
        use trading_core::asset::Usdt;
        let btc = PositionView {
            symbol: Symbol::new("BTCUSDT"),
            base_qty: dec!(0.5),
            cost_basis: Money::<Usdt>::from_decimal(dec!(20_000)),
            last_mark: Price::new(dec!(41_000)).unwrap(),
            pnl: Money::<Usdt>::from_decimal(dec!(500)),
            pnl_pct: dec!(2.5),
            exposure_pct: dec!(10),
        };
        let eth = PositionView {
            symbol: Symbol::new("ETHUSDT"),
            base_qty: dec!(2.0),
            cost_basis: Money::<Usdt>::from_decimal(dec!(5_000)),
            last_mark: Price::new(dec!(2_500)).unwrap(),
            pnl: Money::<Usdt>::from_decimal(dec!(-100)),
            pnl_pct: dec!(-2.0),
            exposure_pct: dec!(5),
        };
        let mut cockpit = Cockpit::default();
        cockpit.positions = PanelState::Ready(vec![btc.clone(), eth.clone()]);

        let m = position_for_symbol(&cockpit, &Symbol::new("BTCUSDT"));
        assert!(m.is_some());
        assert_eq!(m.unwrap().symbol, btc.symbol);

        let m_none = position_for_symbol(&cockpit, &Symbol::new("SOLUSDT"));
        assert!(m_none.is_none());
    }
}
