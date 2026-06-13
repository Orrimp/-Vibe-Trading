//! Phase E — Compare-matrix widget (ui-rethink-phase-e-compare R2).
//!
//! Renders the 6×≤10 strategy × pair matrix where each cell shows either:
//! - A **populated cell**: Sharpe KPI text + passive hairline border +
//!   hover tint (R2.3). K7 tooltip on multi-symbol cells (§1.4 of decomp.md).
//! - An **empty-but-legal cell**: centred "Run" button with `ACCENT_500`
//!   hairline (Q4=b). Click → `Message::OpenLabFromCompare` + auto-run.
//! - A **blanked cell**: centred em-dash + passive hairline (Q8=b — pair is
//!   outside this strategy's universe). Non-interactive.
//!
//! Layout primitive: `iced::Column<Row>` — no new `grid` widget (R2.5).
//! The row count = `model.strategies_config.strategies.len()` (Q7=a).
//! The column count = per-strategy universe length (Q8=b).
//!
//! **Zero string literals** — copy via `crate::strings`.
//! **Zero hex colours** — tokens via `crate::theme`.
//! **No new Lumen tokens** (R7.6).

#![allow(clippy::cast_possible_truncation)]

use iced::widget::{Button, Column, Container, Row, Text, Tooltip, button, container, tooltip};
use iced::{Border, Element, Length, Padding};
use smol_str::SmolStr;
use trading_core::{Symbol, Venue};

use crate::compare::state::CompareKpiAxis;
use crate::lab::state::DateRange;
use crate::state::{Cockpit, Message, StrategyConfigEntry};
use crate::strings::{
    COMPARE_CELL_BLANKED_LABEL, COMPARE_CELL_OVERLAY_ADD, COMPARE_CELL_OVERLAY_HINT,
    COMPARE_CELL_OVERLAY_SELECTED, COMPARE_CELL_RUN_LABEL, COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE,
};
use crate::theme::{ThemeMode, color, radius, space, text};

// ── Universe helpers ─────────────────────────────────────────────────────────
//
// Per Q8=(b): each strategy row shows only the pairs in that strategy's
// universe; pairs outside the universe render as blanked grey cells.
//
// Universe derivation mirrors `compare::cache::scenario_universe` but operates
// on the strategy *id* rather than the scenario name. The mapping is locked by
// the architect's H1 enumeration in decomp.md §1.2.

/// XRP-first universe for v1.momentum and v2.5.tcn strategies (10 symbols).
const TOP10_SYMBOLS: &[&str] = &[
    "XRPUSDT", "BTCUSDT", "ETHUSDT", "BNBUSDT", "SOLUSDT", "ADAUSDT", "DOTUSDT", "DOGEUSDT",
    "LINKUSDT", "AVAXUSDT",
];

/// Universe for v1.5a.pairs — BTC + ETH only.
const PAIRS_SYMBOLS: &[&str] = &["BTCUSDT", "ETHUSDT"];

/// BTC-only universe.
const BTC_SYMBOLS: &[&str] = &["BTCUSDT"];

/// Derive the (Venue, Symbol) universe for a strategy config entry.
///
/// Matches strategy IDs to their universe per the architect's H1 census:
/// - `top10_*` / `tcn_*` → 10-symbol universe
/// - `pairs_*` → (BTC, ETH)
/// - `btc_*` / others → BTC only (single-pair)
///
/// Returns `None` when no universe can be determined (e.g. v2.llm not yet
/// registered) — the entire row is blanked.
fn strategy_universe(config: &StrategyConfigEntry) -> Option<Vec<(Venue, Symbol)>> {
    let id = config.id.0.as_str();
    if id.starts_with("top10") || id.starts_with("tcn") {
        Some(
            TOP10_SYMBOLS
                .iter()
                .map(|s| (Venue::Binance, Symbol::new(*s)))
                .collect(),
        )
    } else if id.starts_with("pairs") {
        Some(
            PAIRS_SYMBOLS
                .iter()
                .map(|s| (Venue::Binance, Symbol::new(*s)))
                .collect(),
        )
    } else if id.starts_with("btc") || !id.is_empty() {
        // Fall back to BTC-only for unknown single-pair strategies.
        Some(
            BTC_SYMBOLS
                .iter()
                .map(|s| (Venue::Binance, Symbol::new(*s)))
                .collect(),
        )
    } else {
        None
    }
}

// ── Cell dimensions ───────────────────────────────────────────────────────────

/// Minimum cell width in logical pixels. At 6×10 the grid fits a 1920px
/// viewport with headroom (10 cols × 100 px = 1000 px + headers + gutters).
const CELL_MIN_W: f32 = 90.0;
/// Minimum cell height in logical pixels.
const CELL_MIN_H: f32 = 60.0;

// ── Public entry point ────────────────────────────────────────────────────────

/// Render the Compare matrix.
///
/// Called by `screens::compare::view`; reads `model.compare_screen_state` for
/// the cache and `model.strategies_config` for the row/column configuration.
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_, Message> {
    let range = &model.compare_screen_state.range;
    let cache = &model.compare_screen_state.cache;
    let kpi_axis = model.compare_screen_state.kpi_axis;

    // Collect all pairs across all strategies for the column headers.
    // We derive the full column set as the union (ordered, deduped) of every
    // strategy's universe — this ensures the header row always shows all
    // columns that any row might need.
    let all_strategies: &[StrategyConfigEntry] = model
        .strategies_config
        .as_ref()
        .map_or(&[], |c| c.strategies.as_slice());

    if all_strategies.is_empty() {
        return empty_state(mode);
    }

    // Build column headers: union of all strategy universes, XRP-first.
    // Since TOP10 is the superset, use it as the reference column order if
    // any strategy has it; otherwise fall back to the union of all per-strategy universes.
    let column_symbols: Vec<Symbol> = {
        let has_top10 = all_strategies
            .iter()
            .any(|s| s.id.0.as_str().starts_with("top10") || s.id.0.as_str().starts_with("tcn"));
        if has_top10 {
            TOP10_SYMBOLS.iter().map(|s| Symbol::new(*s)).collect()
        } else {
            // Collect and dedup.
            let mut seen = std::collections::BTreeSet::new();
            let mut cols = Vec::new();
            for strat in all_strategies {
                if let Some(uni) = strategy_universe(strat) {
                    for (_, sym) in uni {
                        if seen.insert(sym.clone()) {
                            cols.push(sym);
                        }
                    }
                }
            }
            cols
        }
    };

    // ── Header row ───────────────────────────────────────────────────────────
    let header_row = build_header_row(&column_symbols, mode);

    // ── Strategy rows ─────────────────────────────────────────────────────────
    let mut rows = Column::new().spacing(1);
    rows = rows.push(header_row);

    for strat in all_strategies {
        let strategy_id = SmolStr::new(strat.id.0.as_str());
        let uni = strategy_universe(strat);
        let uni_symbols: std::collections::BTreeSet<Symbol> = uni
            .as_ref()
            .map(|u| u.iter().map(|(_, s)| s.clone()).collect())
            .unwrap_or_default();

        let mut row = Row::new().spacing(1);

        // Row header (strategy ID label — non-interactive at v0.1.0).
        row = row.push(
            Container::new(
                Text::new(strat.id.to_string())
                    .size(text::SMALL)
                    .color(color::FG_2.current(mode)),
            )
            .width(Length::Fixed(120.0))
            .height(Length::Fixed(CELL_MIN_H))
            .padding(Padding::from([space::XS as u16, space::S as u16]))
            .style(move |_| container::Style {
                border: Border {
                    color: color::BORDER_1.current(mode),
                    width: 1.0,
                    radius: radius::R1.into(),
                },
                background: Some(color::PANEL.current(mode).into()),
                ..Default::default()
            }),
        );

        // Per-symbol cells.
        for col_sym in &column_symbols {
            let cell: Element<'_, Message> = if uni_symbols.contains(col_sym) {
                // Symbol is in this strategy's universe.
                let key = (strategy_id.clone(), col_sym.clone(), range.clone());
                if let Some(cached) = cache.get(&key) {
                    // Populated cell.
                    // lab-compare-equity-overlay T2 — the cell's slot in the
                    // overlay ring (`Some(0)` ⇒ ACCENT, `Some(1)` ⇒ ACCENT_2,
                    // `None` ⇒ not selected) drives the `+`/✓ chip styling.
                    let overlay_idx = model.compare_screen_state.overlay_slot_index(&key);
                    // Only cells that actually carry a timestamped series can be
                    // overlaid; a cell with no companion CSV gets no chip (its
                    // curve cannot draw) — honest affordance, no dead button.
                    let has_series = !cached.equity_series_ts.is_empty();
                    populated_cell(
                        cached.sharpe,
                        cached.is_multi_symbol,
                        kpi_axis,
                        strategy_id.clone(),
                        col_sym.clone(),
                        range.clone(),
                        overlay_idx,
                        has_series,
                        mode,
                    )
                } else {
                    // Empty-but-legal cell — "Run" affordance.
                    run_affordance_cell(strategy_id.clone(), col_sym.clone(), range.clone(), mode)
                }
            } else {
                // Blanked cell — pair not in this strategy's universe.
                blanked_cell(mode)
            };
            row = row.push(cell);
        }

        rows = rows.push(row);
    }

    rows.into()
}

// ── Cell builders ─────────────────────────────────────────────────────────────

/// Header row: empty strategy-label corner + one column header per symbol.
fn build_header_row<'a>(column_symbols: &[Symbol], mode: ThemeMode) -> Element<'a, Message> {
    let mut row = Row::new().spacing(1);

    // Corner cell (empty — aligns with the strategy-label column).
    row = row.push(
        Container::new(iced::widget::Space::new())
            .width(Length::Fixed(120.0))
            .height(Length::Fixed(24.0)),
    );

    // Column headers — NON-INTERACTIVE per R2.4 v0.1.0 (label only).
    for sym in column_symbols {
        row = row.push(
            Container::new(
                Text::new(sym.to_string())
                    .size(text::MICRO)
                    .color(color::FG_3.current(mode)),
            )
            .width(Length::Fixed(CELL_MIN_W))
            .height(Length::Fixed(24.0))
            .padding(Padding::from([2u16, space::XS as u16]))
            .align_x(iced::alignment::Horizontal::Center)
            .style(move |_| container::Style {
                background: None,
                ..Default::default()
            }),
        );
    }

    row.into()
}

/// Populated cell: Sharpe KPI text + hairline border + hover tint, with a
/// compact overlay-select chip (lab-compare-equity-overlay T2 / Q1) in the
/// top-right corner. K7 tooltip wraps the cell when `is_multi_symbol == true`
/// (§1.4).
///
/// - The KPI text is the PRIMARY click → `OpenLabFromCompare` (drill into Lab —
///   unchanged from v0.1.0; H5 round-trip stays green).
/// - The `+`/✓ chip → `CompareToggleOverlay` adds/removes this run from the
///   two-run equity overlay below the matrix. Rendered only when the cell has a
///   timestamped series (`has_series`) — a curve with no companion CSV cannot
///   be overlaid, so no dead button is shown.
#[allow(
    clippy::needless_pass_by_value,
    clippy::fn_params_excessive_bools,
    clippy::too_many_arguments
)]
fn populated_cell<'a>(
    sharpe: f64,
    is_multi_symbol: bool,
    kpi_axis: CompareKpiAxis,
    strategy_id: SmolStr,
    symbol: Symbol,
    range: DateRange,
    overlay_idx: Option<usize>,
    has_series: bool,
    mode: ThemeMode,
) -> Element<'a, Message> {
    // v0.1.0: only Sharpe is wired (Q3=a). Other axes fall back to Sharpe.
    let _ = kpi_axis; // Reserved — future multi-KPI.

    let sharpe_label = format!("{sharpe:.2}");
    let kpi_color = if sharpe > 0.5 {
        color::UP_500.current(mode)
    } else if sharpe < 0.0 {
        color::DOWN_500.current(mode)
    } else {
        color::WARN_500.current(mode)
    };

    let strat_clone = strategy_id.clone();
    let sym_clone = symbol.clone();
    let range_clone = range.clone();

    let kpi_btn = Button::new(Text::new(sharpe_label).size(text::BODY).color(kpi_color))
        .on_press(Message::OpenLabFromCompare {
            strategy: trading_core::StrategyId::new(strat_clone),
            pair: Some((Venue::Binance, sym_clone)),
            range: range_clone,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(Padding::from([space::XS as u16, space::S as u16]))
        .style(move |_theme: &iced::Theme, status: button::Status| {
            let bg = match status {
                button::Status::Hovered | button::Status::Pressed => {
                    Some(color::PANEL_RAISED.current(mode).into())
                }
                _ => None,
            };
            let border_color = match status {
                button::Status::Hovered | button::Status::Pressed => color::ACCENT.current(mode),
                _ => color::BORDER_1.current(mode),
            };
            button::Style {
                background: bg,
                text_color: kpi_color,
                border: Border {
                    color: border_color,
                    width: 1.0,
                    radius: radius::R1.into(),
                },
                ..Default::default()
            }
        });

    // Compose the KPI button + the overlay-select chip (when a series exists).
    // The chip floats in the cell's top-right via a right-aligned Row stacked
    // over the KPI button in a fixed-size container.
    let inner: Element<'a, Message> = if has_series {
        let chip = overlay_select_chip(strategy_id, symbol, range, overlay_idx, mode);
        let chip_row = Row::new()
            .width(Length::Fill)
            .push(iced::widget::Space::new().width(Length::Fill))
            .push(chip);
        Container::new(
            iced::widget::Stack::new().push(kpi_btn).push(
                Container::new(chip_row)
                    .width(Length::Fill)
                    .height(Length::Fill)
                    .padding(Padding::from([1u16, 2u16])),
            ),
        )
        .width(Length::Fixed(CELL_MIN_W))
        .height(Length::Fixed(CELL_MIN_H))
        .into()
    } else {
        Container::new(kpi_btn)
            .width(Length::Fixed(CELL_MIN_W))
            .height(Length::Fixed(CELL_MIN_H))
            .into()
    };

    if is_multi_symbol {
        // K7 tooltip — §1.4 of decomp.md.
        Tooltip::new(
            inner,
            Text::new(COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE)
                .size(text::MICRO)
                .color(color::FG_2.current(mode)),
            tooltip::Position::Bottom,
        )
        .style(move |_| container::Style {
            background: Some(color::OVERLAY.current(mode).into()),
            border: Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: radius::R2.into(),
            },
            text_color: Some(color::FG_2.current(mode)),
            ..Default::default()
        })
        .into()
    } else {
        inner.into()
    }
}

/// Compact overlay-select chip in a populated cell's top-right corner
/// (lab-compare-equity-overlay T2 / Q1). Emits [`Message::CompareToggleOverlay`]
/// with the cell's `(strategy, symbol, range)` identity.
///
/// Visual state:
/// - **Unselected** (`overlay_idx == None`): a muted `+` — "add to overlay".
/// - **Slot 0** (`Some(0)`): a `✓` in `ACCENT` — the primary overlay curve.
/// - **Slot 1** (`Some(1)`): a `✓` in `ACCENT_2` — the second overlay curve.
///
/// Colour is paired with the glyph (`+` vs `✓`), so selection is never
/// signalled by colour alone (accessibility — colour is not the only signal).
#[allow(clippy::needless_pass_by_value)]
fn overlay_select_chip<'a>(
    strategy_id: SmolStr,
    symbol: Symbol,
    range: DateRange,
    overlay_idx: Option<usize>,
    mode: ThemeMode,
) -> Element<'a, Message> {
    // Slot → colour: 0 = ACCENT (primary), 1 = ACCENT_2 (compare). Mirrors the
    // `chart::view` overlay (equity = ACCENT, compare[0] = ACCENT_2).
    let chip_color = match overlay_idx {
        Some(0) => color::ACCENT.current(mode),
        Some(_) => color::ACCENT_2.current(mode),
        None => color::FG_4.current(mode),
    };
    let glyph = if overlay_idx.is_some() {
        COMPARE_CELL_OVERLAY_SELECTED
    } else {
        COMPARE_CELL_OVERLAY_ADD
    };
    let slot = (strategy_id, symbol, range);

    let chip = Button::new(Text::new(glyph).size(text::MICRO).color(chip_color))
        .on_press(Message::CompareToggleOverlay(slot))
        .padding(Padding::from([0u16, 3u16]))
        .style(move |_theme: &iced::Theme, status: button::Status| {
            let bg = match (overlay_idx.is_some(), status) {
                (true, _) => Some(color::PANEL_RAISED.current(mode).into()),
                (false, button::Status::Hovered | button::Status::Pressed) => {
                    Some(color::PANEL_RAISED.current(mode).into())
                }
                _ => None,
            };
            button::Style {
                background: bg,
                text_color: chip_color,
                border: Border {
                    color: if overlay_idx.is_some() {
                        chip_color
                    } else {
                        color::BORDER_1.current(mode)
                    },
                    width: 1.0,
                    radius: radius::R1.into(),
                },
                ..Default::default()
            }
        });

    // Tooltip explains the affordance in plain language.
    Tooltip::new(
        chip,
        Text::new(COMPARE_CELL_OVERLAY_HINT)
            .size(text::MICRO)
            .color(color::FG_2.current(mode)),
        tooltip::Position::Top,
    )
    .style(move |_| container::Style {
        background: Some(color::OVERLAY.current(mode).into()),
        border: Border {
            color: color::BORDER_1.current(mode),
            width: 1.0,
            radius: radius::R2.into(),
        },
        text_color: Some(color::FG_2.current(mode)),
        ..Default::default()
    })
    .into()
}

/// Empty-but-legal cell: centred "Run" button with `ACCENT_500` hairline (Q4=b).
///
/// Click → `OpenLabFromCompare` + dispatch `LabRunRequested` in the binary
/// layer (per R4.3 — the pure state machine emits one message; the binary
/// chains `LabRunRequested` on the same tick via `Task::done`).
#[allow(clippy::needless_pass_by_value)]
fn run_affordance_cell<'a>(
    strategy_id: SmolStr,
    symbol: Symbol,
    range: DateRange,
    mode: ThemeMode,
) -> Element<'a, Message> {
    Button::new(
        Container::new(
            Text::new(COMPARE_CELL_RUN_LABEL)
                .size(text::SMALL)
                .color(color::ACCENT.current(mode)),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .align_x(iced::alignment::Horizontal::Center)
        .align_y(iced::alignment::Vertical::Center),
    )
    .on_press(Message::OpenLabFromCompare {
        strategy: trading_core::StrategyId::new(strategy_id),
        pair: Some((Venue::Binance, symbol)),
        range,
    })
    .width(Length::Fixed(CELL_MIN_W))
    .height(Length::Fixed(CELL_MIN_H))
    .style(move |_theme: &iced::Theme, status: button::Status| {
        let border_color = match status {
            button::Status::Hovered | button::Status::Pressed => color::ACCENT.current(mode),
            _ => color::BORDER_1.current(mode),
        };
        button::Style {
            background: None,
            text_color: color::ACCENT.current(mode),
            border: Border {
                color: border_color,
                width: 1.0,
                radius: radius::R1.into(),
            },
            ..Default::default()
        }
    })
    .into()
}

/// Blanked cell — pair is outside this strategy's universe (Q8=b).
/// Passive hairline; non-interactive; centred em-dash.
fn blanked_cell(mode: ThemeMode) -> Element<'static, Message> {
    Container::new(
        Text::new(COMPARE_CELL_BLANKED_LABEL)
            .size(text::SMALL)
            .color(color::FG_4.current(mode)),
    )
    .width(Length::Fixed(CELL_MIN_W))
    .height(Length::Fixed(CELL_MIN_H))
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .style(move |_| container::Style {
        border: Border {
            color: color::BORDER_1.current(mode),
            width: 1.0,
            radius: radius::R1.into(),
        },
        ..Default::default()
    })
    .into()
}

/// Empty-state placeholder when no strategy config is available.
fn empty_state(mode: ThemeMode) -> Element<'static, Message> {
    Container::new(
        Text::new(crate::strings::MATRIX_EMPTY_STATE)
            .size(text::BODY)
            .color(color::FG_3.current(mode)),
    )
    .width(Length::Fill)
    .height(Length::Fill)
    .align_x(iced::alignment::Horizontal::Center)
    .align_y(iced::alignment::Vertical::Center)
    .into()
}
