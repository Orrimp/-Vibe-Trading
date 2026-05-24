//! Lab screen — ui-rethink-phase-a-lab Phase A (ex-`charts.rs`).
//!
//! Renamed from `screens/charts.rs` per T-D-2 (verbatim move at M0 —
//! no body changes). Phase A M1 extends this screen with the three-row
//! top bar (pair chips → strategy chips → date-range picker); chart
//! canvas extensions land in M2.
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

use iced::widget::{Column, Container, Row, Text, container};
use iced::{Border, Length};
use rust_decimal::Decimal;
use trading_core::{FillView, PositionView, Side, Symbol};

use crate::lab::equity_loader::{LabTuple, route_equity_overlay};
use crate::lab::state::{LabDataSource, StrategyFamily};
use crate::state::{Cockpit, PanelState};
use crate::strings::{
    CHART_POSITION_MIRROR_LABEL, CHART_POSITION_MIRROR_NONE, CHART_VOLUME_HISTOGRAM_LABEL,
    CHART_VOLUME_TILE_BUYS_LABEL, CHART_VOLUME_TILE_NET_LABEL, CHART_VOLUME_TILE_SELLS_LABEL,
    CHART_VOLUME_TILE_TRADES_SUFFIX,
};
use crate::theme::{ThemeMode, color, color_for_delta, radius, space, text};
use crate::widgets::num::{fmt_pct, fmt_price, fmt_qty, fmt_usdt_signed};
use crate::widgets::run_button::{self, RunState};
use crate::widgets::volume_histogram::{self, VolumeBin};
use crate::widgets::{
    cadence_badge, chart, date_range, kpi_strip, pair_chip, source_toggle, strategy_chip,
};

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

/// Approximate strategy-chip row height (T-D-6 / T-D-8 — one row of
/// `SMALL`-sized buttons with `XS`/`M` padding). Same budget as `CHIP_ROW_HEIGHT_PX`.
const STRATEGY_ROW_HEIGHT_PX: f32 = 32.0;

/// Approximate date-range picker row height (T-D-7 / T-D-8 — one row
/// of preset chips at `SMALL` size with `XS`/`M` padding).
const DATE_RANGE_ROW_HEIGHT_PX: f32 = 32.0;

/// Approximate histogram-label height (`text::MICRO` + `space::XXS`
/// gap) — the volume-histogram column's first child.
const HISTOGRAM_LABEL_HEIGHT_PX: f32 = 14.0;

/// Approximate Run button row height (T-D-14b — button in a Row with
/// `space::M` spacing, `SMALL`-sized text, `S`/`L` padding).
const RUN_BUTTON_ROW_HEIGHT_PX: f32 = 36.0;

/// Approximate Lab single-run KPI strip height (two text lines per card
/// + M padding on each side ≈ 80 px; same budget as `STATUS_STRIP_HEIGHT_PX`).
///
/// lab-end-to-end-v2 Wave D-1.1 F8.
const LAB_KPI_STRIP_HEIGHT_PX: f32 = 80.0;

/// Training panel header height when collapsed (header chip only, T-D-N3).
const TRAINING_PANEL_COLLAPSED_HEIGHT_PX: f32 = 32.0;

/// Training panel height when expanded (log + status strip + buttons, T-D-N3).
const TRAINING_PANEL_EXPANDED_HEIGHT_PX: f32 = 240.0;

/// lab-yahoo-realdata T-C3.4 — strategies compatible with the Yahoo data path.
///
/// Cross-sectional strategies (v1.*, v2.*) require the Binance hourly
/// multi-symbol universe and reject `YahooCache` at the engine level with
/// `RunError::UnsupportedDataSource`. Only these four single-symbol
/// strategies are shown when `data_source == YahooCache`.
const SINGLE_SYMBOL_STRATEGIES: &[&str] = &["v0.sma", "v0.5.macd", "v0.5.rsi", "v0.5.bbands"];

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
/// Compute the chart canvas's vertical allocation given a body height.
///
/// `training_collapsed` should be `model.lab_state.training_panel_collapsed`.
/// When `true`, only the header chip (~32 px) is deducted; when `false`,
/// the expanded panel (~240 px) is deducted instead.
#[must_use]
pub fn chart_canvas_height_for_body(body_height_px: f32) -> f32 {
    chart_canvas_height_for_body_with_training(body_height_px, true)
}

/// Full version used by the view (accounts for training panel state).
#[must_use]
pub fn chart_canvas_height_for_body_with_training(
    body_height_px: f32,
    training_collapsed: bool,
) -> f32 {
    #[allow(clippy::cast_precision_loss)]
    let padding = (space::L as f32) * 2.0;
    // 10 children: pair_row, source_toggle_row, strategy_row, date_range_row,
    // run_button_row, status_strip, lab_kpi_strip, chart (Fill), histogram,
    // training_panel → 9 gaps.
    // lab-yahoo-realdata T-C3.4: added source_toggle_row (+1 child, +1 gap).
    #[allow(clippy::cast_precision_loss)]
    let spacing = (space::M as f32) * 9.0;
    let training_height = if training_collapsed {
        TRAINING_PANEL_COLLAPSED_HEIGHT_PX
    } else {
        TRAINING_PANEL_EXPANDED_HEIGHT_PX
    };
    let fixed = CHIP_ROW_HEIGHT_PX
        + CHIP_ROW_HEIGHT_PX  // source_toggle_row (~same height as pair chip row)
        + STRATEGY_ROW_HEIGHT_PX
        + DATE_RANGE_ROW_HEIGHT_PX
        + RUN_BUTTON_ROW_HEIGHT_PX
        + STATUS_STRIP_HEIGHT_PX
        + LAB_KPI_STRIP_HEIGHT_PX
        + HISTOGRAM_LABEL_HEIGHT_PX
        + HISTOGRAM_HEIGHT_PX
        + training_height;
    (body_height_px - padding - spacing - fixed).max(0.0)
}

/// Render the Lab screen body.
#[allow(
    clippy::cast_possible_truncation,
    clippy::needless_pass_by_value,
    clippy::too_many_lines
)]
#[must_use]
pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    let active = model
        .selected_symbol
        .clone()
        .or_else(|| model.universe.first().cloned());

    // ── Phase A top-bar row 1: pair chips (T-D-5 / T-D-8) ──────────────
    // Use XRP-first universe (operator-locked order per R3.2). The chips
    // dispatch `LabSelectPair`; the chart canvas continues to read from
    // `selected_symbol` (M2 will wire `lab_state.pair` → chart).
    // lab-yahoo-realdata T-C3.4: when data_source == YahooCache, render
    // the Yahoo crypto-mirror universe (Venue::Yahoo chips) instead of
    // the Binance universe.
    let is_yahoo = model.lab_state.data_source == LabDataSource::YahooCache;
    let mut pair_chip_row = Row::new().spacing(space::S);
    let pair_universe: &[(trading_core::Venue, &str)] = if is_yahoo {
        crate::lab::universe::YAHOO_CRYPTO_UNIVERSE
    } else {
        crate::lab::universe::XRP_FIRST_UNIVERSE
    };
    for (v, s) in pair_universe {
        let sym = Symbol::new(*s);
        let is_active = model
            .lab_state
            .pair
            .as_ref()
            .is_some_and(|(pv, ps)| pv == v && ps == &sym);
        pair_chip_row = pair_chip_row.push(pair_chip::view(*v, sym, is_active, false, mode));
    }
    let chip_row = pair_chip_row;

    // ── lab-yahoo-realdata T-C3.4 — Source toggle row ──────────────────
    // Inserted between the pair chip row and the strategy chip row.
    let source_toggle_row = Row::new()
        .spacing(space::S)
        .push(source_toggle::view(model.lab_state.data_source, mode))
        .width(Length::Fill);

    // ── Phase A top-bar row 2: strategy chips (T-D-6 / T-D-8) ──────────
    // Collect strategy ids from the strategies panel; fall back to empty
    // at cold start (strategies panel is Loading). No family-registry at
    // Phase A — all strategies default to `Rule` family.
    //
    // lab-yahoo-realdata T-C3.4: when data_source == YahooCache, only show
    // the 4 single-symbol strategies (v0.sma, v0.5.macd, v0.5.rsi, v0.5.bbands).
    // Cross-sectional strategies (v1.*, v2.*) are hidden because they require
    // the Binance hourly multi-symbol universe and reject YahooCache at the
    // engine level (RunError::UnsupportedDataSource).
    // (Filter list defined at module level: SINGLE_SYMBOL_STRATEGIES.)
    let strategy_ids: Vec<trading_core::StrategyId> = match &model.strategies {
        PanelState::Ready(rows) => rows
            .iter()
            .filter(|r| {
                if is_yahoo {
                    SINGLE_SYMBOL_STRATEGIES
                        .iter()
                        .any(|s| r.id.0.as_str() == *s)
                } else {
                    true
                }
            })
            .map(|r| r.id.clone())
            .collect(),
        _ => Vec::new(),
    };
    // Build a minimal family map: all Rule at Phase A (R4.1 — family pill
    // requires a registry lookup which ships in Phase B). This is the
    // Wave 1 stub; Phase B wires the real family map.
    let families: std::collections::HashMap<trading_core::StrategyId, StrategyFamily> =
        strategy_ids
            .iter()
            .map(|id| (id.clone(), StrategyFamily::default()))
            .collect();
    let strategy_row = strategy_chip::row(
        &strategy_ids,
        &families,
        model.lab_state.strategy.as_ref(),
        model.lab_state.compare_set(),
        mode,
    );

    // ── Phase A top-bar row 3: date-range picker (T-D-7 / T-D-8) ───────
    // lab-yahoo-realdata T-C3.4: when data_source == YahooCache, append a
    // cadence badge (e.g. "1d", "1h") derived from the selected range.
    let range_row = if is_yahoo {
        // Derive cadence from the current date range for the badge.
        let (start_ms, end_ms) = derive_range_ms_for_badge(&model.lab_state.range);
        let cadence = cadence_badge::CadenceLabel::derive_from_range(start_ms, end_ms);
        Row::new()
            .spacing(space::S)
            .push(date_range::view(&model.lab_state.range, None, mode))
            .push(cadence_badge::view(cadence, mode))
            .width(Length::Fill)
    } else {
        Row::new()
            .spacing(space::S)
            .push(date_range::view(
                &model.lab_state.range,
                None, // narrowed_from badge is M2 (equity_loader)
                mode,
            ))
            .width(Length::Fill)
    };
    // Keep `range_picker` for the column push below.
    let range_picker = range_row;

    // ── Phase A top-bar row 4: Run button (T-D-14b) + delta badge (T-D-N13) ─
    // F10 (lab-end-to-end-v2 Wave D-1.1): use the selection-gated variant so
    // the button is Disabled until BOTH pair AND strategy are selected.
    let strategy_selected = model.lab_state.strategy.is_some();
    let pair_selected = model.lab_state.pair.is_some();
    tracing::trace!(
        target: "lab.view",
        strategy = ?model.lab_state.strategy,
        pair = ?model.lab_state.pair,
        last_run_report_present = model.lab_state.last_run_report.is_some(),
        "lab::view selection gate"
    );
    let run_state = RunState::from_cockpit_with_selection(
        model.lab_run_inflight,
        None,
        strategy_selected,
        pair_selected,
    );
    // T-D-N13: show delta badge when both last + prev reports are present
    // and share the same tuple (same (strategy, pair, range) selection).
    let delta_badge = if let (Some(last), Some(prev)) = (
        model.lab_state.last_run_report.as_ref(),
        model.lab_state.prev_run_report.as_ref(),
    ) {
        if last.tuple == prev.tuple {
            Some(crate::widgets::run_delta_badge::view(last, prev, mode))
        } else {
            None
        }
    } else {
        None
    };
    let mut run_button_row = Row::new()
        .spacing(crate::theme::space::M)
        .push(run_button::view(&run_state, model.lab_run_inflight, mode));
    if let Some(badge) = delta_badge {
        run_button_row = run_button_row.push(badge);
    }
    let run_button_row = run_button_row.width(Length::Fill);

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

    // Lab single-run KPI strip (lab-end-to-end-v2 Wave D-1.1 F8).
    // Renders absolute KPIs from the most-recent completed run.
    // When `last_run_report` is None (no run yet), renders placeholder
    // em-dashes so the visual structure is stable.
    let lab_kpi_strip = kpi_strip::view_for_lab(
        model.lab_state.last_run_report.as_ref().map(|m| &m.kpis),
        mode,
    );

    // Chart canvas — full width, fills remaining vertical space.
    // T-D-N11: route equity overlay from in-memory last_run_report first,
    // then fall through to EquityCache (Phase A behaviour). Uses interior
    // mutability (RefCell) so `view` can stay `&Cockpit` (immutable).
    // F9 tracing (lab-end-to-end-v2 Wave D-1.1): trace! level so these only
    // appear when RUST_LOG=trace — kept opt-in per the brief.
    let equity_overlay = if let (Some(strategy), Some((venue, symbol))) = (
        model.lab_state.strategy.as_ref(),
        model.lab_state.pair.as_ref(),
    ) {
        let current_tuple = LabTuple::new(strategy, *venue, symbol, model.lab_state.range.clone());
        let spec_root = crate::lab::equity_loader::default_spec_root();
        let mut cache = model.equity_cache.borrow_mut();
        let overlay =
            route_equity_overlay(&model.lab_state, &mut cache, &current_tuple, &spec_root);
        tracing::trace!(
            target: "lab.equity_overlay",
            strategy = %strategy.0,
            symbol = %symbol.0,
            overlay_present = overlay.is_some(),
            samples_count = overlay.as_ref().map_or(0, |o| o.samples.len()),
            bars_in_chart = bars.len(),
            "lab::view equity overlay route result"
        );
        overlay
    } else {
        tracing::trace!(
            target: "lab.equity_overlay",
            "lab::view equity overlay skipped: strategy or pair is None"
        );
        None
    };
    let chart_body = if let Some((_, _)) = active {
        chart::view(
            bars,
            active_markers,
            active_signals,
            model.chart_tooltip.clone(),
            equity_overlay,
            vec![], // compare curves — Phase B
            mode,
        )
    } else {
        chart::view(Vec::new(), Vec::new(), Vec::new(), None, None, vec![], mode)
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
        .push(source_toggle_row)
        .push(strategy_row)
        .push(range_picker)
        .push(run_button_row)
        .push(status_strip)
        // F8 (lab-end-to-end-v2 Wave D-1.1) — Lab single-run KPI strip.
        // Inserted between the status strip and the chart so the operator
        // sees absolute run KPIs without needing to click a second run
        // (that would show zero deltas in the run-delta-badge).
        .push(lab_kpi_strip)
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
        // cockpit-training-control T-D-N3 — Training panel (collapsed by
        // default per R1.2 / Q4). Always rendered; collapsed = header-chip
        // only (~32 px); expanded = header chip + log + status strip (~240 px).
        .push(training_panel(model, mode))
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

// ── lab-yahoo-realdata T-C3.4 — cadence badge helper ────────────────────────

/// Derive `(start_ms, end_ms)` UTC epoch-millis from a `DateRange` for the
/// cadence badge. Uses fixed calendar boundaries for `H1_2024`/`H2_2024`;
/// uses wall-clock `now()` for rolling presets (`Last30d`/`Last90d`).
///
/// This is view-layer only — does not affect the engine or anchors.
fn derive_range_ms_for_badge(range: &crate::lab::state::DateRange) -> (i64, i64) {
    use crate::lab::state::{DateRange, Preset};
    const MS_PER_DAY: i64 = 86_400_000;
    let now_ms = time::OffsetDateTime::now_utc().unix_timestamp() * 1_000;
    match range {
        DateRange::Preset(Preset::Last30d) => (now_ms - 30 * MS_PER_DAY, now_ms),
        DateRange::Preset(Preset::Last90d) => (now_ms - 90 * MS_PER_DAY, now_ms),
        DateRange::Preset(Preset::H1_2024) => (1_704_067_200_000, 1_719_792_000_000),
        DateRange::Preset(Preset::H2_2024) => (1_719_792_000_000, 1_735_689_600_000),
        DateRange::Custom { start_raw, end_raw } => {
            // Parse ISO-8601 date strings; fall back to (now-90d, now) on parse failure.
            let parse_date = |s: &str| -> i64 {
                let padded = format!("{s}T00:00:00Z");
                time::OffsetDateTime::parse(&padded, &time::format_description::well_known::Rfc3339)
                    .map(|dt| dt.unix_timestamp() * 1_000)
                    .unwrap_or(now_ms - 90 * MS_PER_DAY)
            };
            (parse_date(start_raw.as_str()), parse_date(end_raw.as_str()))
        }
    }
}

// ── Training panel (cockpit-training-control T-D-N3) ─────────────────────────

/// Render the Training panel.
///
/// Collapsed (default): a single header chip "Train ▶" dispatching
/// `Message::TrainingPanelToggled`. Height ≈ 32 px.
///
/// Expanded: header chip + status strip + log + buttons.
/// Height ≈ 240 px (fixed by `TRAINING_PANEL_EXPANDED_HEIGHT_PX`).
///
/// Buttons (R3.4):
/// - **Train** — primary; disabled when `training_inflight.is_some()`.
/// - **Cancel** — visible only when in-flight.
/// - **Clear log** — always visible when expanded.
///
/// Status strip (R3.5, Tier 1): "Idle" / "Training…" / "Done" / "Failed: …"
/// derived from `training_inflight` presence.
#[must_use]
#[allow(clippy::too_many_lines)]
fn training_panel(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_> {
    use crate::state::Message;
    use crate::strings;
    use crate::widgets::training_log;

    let collapsed = model.lab_state.training_panel_collapsed;
    let inflight = model.lab_state.training_inflight.is_some();

    let accent = color::ACCENT.current(mode);
    let fg1 = color::FG_1.current(mode);
    let fg3 = color::FG_3.current(mode);

    // Header chip: shows "Train ▾" (expanded) or "Train ▸" (collapsed).
    let arrow = if collapsed { " ▸" } else { " ▾" };
    let header_label = format!("{}{arrow}", strings::TRAINING_PANEL_HEADER);
    let header_chip =
        iced::widget::button(iced::widget::text(header_label).size(12).style(move |_| {
            iced::widget::text::Style {
                color: Some(accent),
            }
        }))
        .on_press(Message::TrainingPanelToggled)
        .padding([4, 10])
        .style(move |_theme, _status| iced::widget::button::Style {
            background: Some(iced::Background::Color(color::PANEL.current(mode))),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: accent,
            ..Default::default()
        });

    if collapsed {
        // Collapsed: just the header chip.
        iced::widget::row![header_chip]
            .width(Length::Fill)
            .height(Length::Fixed(TRAINING_PANEL_COLLAPSED_HEIGHT_PX))
            .into()
    } else {
        // Status strip (Tier 1: Idle / Training… / Done / Failed: …).
        let status_text = if inflight {
            strings::TRAINING_STATUS_RUNNING.to_string()
        } else {
            strings::TRAINING_STATUS_IDLE.to_string()
        };

        let status_strip =
            iced::widget::text(status_text)
                .size(11)
                .style(move |_| iced::widget::text::Style {
                    color: Some(if inflight { accent } else { fg3 }),
                });

        // Buttons row.
        let train_btn = {
            let mut b = iced::widget::button(
                iced::widget::text(strings::TRAINING_BUTTON_TRAIN)
                    .size(12)
                    .style(move |_| iced::widget::text::Style { color: Some(fg1) }),
            )
            .padding([4, 12])
            .style(move |_theme, _status| iced::widget::button::Style {
                background: Some(iced::Background::Color(color::ACCENT.current(mode))),
                border: iced::Border::default(),
                text_color: color::FG_ON_ACCENT.current(mode),
                ..Default::default()
            });
            if !inflight {
                b = b.on_press(Message::TrainingPressed);
            }
            b
        };

        let clear_btn = iced::widget::button(
            iced::widget::text(strings::TRAINING_BUTTON_CLEAR_LOG)
                .size(12)
                .style(move |_| iced::widget::text::Style { color: Some(fg3) }),
        )
        .on_press(Message::TrainingClearLog)
        .padding([4, 10])
        .style(move |_theme, _status| iced::widget::button::Style {
            background: Some(iced::Background::Color(color::PANEL.current(mode))),
            border: iced::Border {
                color: color::BORDER_1.current(mode),
                width: 1.0,
                radius: 4.0.into(),
            },
            text_color: fg3,
            ..Default::default()
        });

        let mut btn_row = iced::widget::row![train_btn, clear_btn].spacing(space::XS);

        if inflight {
            let cancel_btn = iced::widget::button(
                iced::widget::text(strings::TRAINING_BUTTON_CANCEL)
                    .size(12)
                    .style(move |_| iced::widget::text::Style {
                        color: Some(color::DOWN_400.current(mode)),
                    }),
            )
            .on_press(Message::TrainingCancelPressed)
            .padding([4, 10])
            .style(move |_theme, _status| iced::widget::button::Style {
                background: Some(iced::Background::Color(color::PANEL.current(mode))),
                border: iced::Border {
                    color: color::DOWN_400.current(mode),
                    width: 1.0,
                    radius: 4.0.into(),
                },
                text_color: color::DOWN_400.current(mode),
                ..Default::default()
            });
            btn_row = btn_row.push(cancel_btn);
        }

        // Log widget.
        let log = training_log::view(
            &model.lab_state.training_log,
            model.lab_state.training_log_anchored,
            mode,
        );

        iced::widget::column![header_chip, status_strip, btn_row, log,]
            .spacing(space::XS)
            .width(Length::Fill)
            .height(Length::Fixed(TRAINING_PANEL_EXPANDED_HEIGHT_PX))
            .into()
    }
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

    #[allow(clippy::cast_precision_loss)]
    // space::M = 12, which is exactly representable as f32
    let padding_m = space::M as f32;
    Container::new(
        Row::new()
            .spacing(space::M)
            .push(buys_cell)
            .push(sells_cell)
            .push(net_cell),
    )
    .padding(padding_m)
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

fn number_card(
    label: &str,
    value: String,
    value_color: iced::Color,
    mode: ThemeMode,
) -> crate::Element<'_> {
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

    #[allow(clippy::cast_precision_loss)]
    // space::M = 12, exactly representable as f32
    let padding_m2 = space::M as f32;
    Container::new(Column::new().spacing(space::XS).push(label).push(body))
        .padding(padding_m2)
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

    /// T2032 — chart canvas height MUST grow with body height.
    #[test]
    fn chart_canvas_height_grows_with_body_height() {
        let h_720 = chart_canvas_height_for_body(720.0);
        let h_1080 = chart_canvas_height_for_body(1080.0);
        assert!(
            h_1080 > h_720,
            "chart canvas height MUST grow with body height: 720 → {h_720}, 1080 → {h_1080}"
        );
        assert!(
            h_720 > 0.0,
            "chart canvas must have non-zero allocation at the 720-px floor: got {h_720}"
        );
        let delta = h_1080 - h_720;
        assert!(
            (delta - 360.0).abs() < f32::EPSILON,
            "delta should equal body-height delta (1080-720=360); got {delta}"
        );
    }

    /// T2024 — `position_for_symbol` returns the matching position when
    /// present, `None` otherwise.
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
        let cockpit = Cockpit {
            positions: PanelState::Ready(vec![btc.clone(), eth.clone()]),
            ..Default::default()
        };

        let m = position_for_symbol(&cockpit, &Symbol::new("BTCUSDT"));
        assert!(m.is_some());
        assert_eq!(m.unwrap().symbol, btc.symbol);

        let m_none = position_for_symbol(&cockpit, &Symbol::new("SOLUSDT"));
        assert!(m_none.is_none());
    }

    /// T-D-2 — default screen is Lab per R1.2.
    #[test]
    fn default_screen_is_lab() {
        use crate::state::Screen;
        assert_eq!(Screen::default(), Screen::Lab);
    }

    // ── T-D-N14 — orphan annotation rendering tests ──────────────────────────

    /// Orphan annotation renders when the pid is alive (current process).
    ///
    /// We test the pid_alive helper directly (the rendering itself is snapshot-
    /// tested in T-D-N18). This verifies the liveness-check path that controls
    /// whether we show ORPHAN_LIVE_FMT or ORPHAN_DEAD_FMT.
    #[test]
    fn orphan_annotation_renders_when_pid_alive() {
        use crate::lab::pid_alive::pid_alive;
        let my_pid = std::process::id() as i64;
        assert!(
            pid_alive(my_pid),
            "orphan annotation path: pid_alive must return true for current process"
        );
        // The rendered string would use ORPHAN_LIVE_FMT.
        // We test the string-building logic directly (not the iced element).
        let run_prefix = "run-001";
        let annotation = crate::strings::ORPHAN_LIVE_FMT.replace("{}", run_prefix);
        assert!(
            annotation.contains("run-001"),
            "annotation must contain run_id prefix"
        );
    }

    /// Orphan annotation renders "dead" when the pid is nonexistent.
    #[test]
    fn orphan_annotation_renders_dead_when_pid_dead() {
        use crate::lab::pid_alive::pid_alive;
        let impossible_pid = i64::from(i32::MAX);
        assert!(
            !pid_alive(impossible_pid),
            "orphan annotation path: impossible pid must be dead"
        );
        let run_prefix = "run-001";
        let annotation = crate::strings::ORPHAN_DEAD_FMT.replace("{}", run_prefix);
        assert!(
            annotation.contains("run-001"),
            "dead annotation must contain run_id prefix"
        );
    }

    /// T-D-8 — snapshot: `lab__top_bar_xrp_first`.
    ///
    /// Records the XRP-first pair ordering pinned from
    /// `lab::universe::XRP_FIRST_UNIVERSE`. Since iced elements are opaque
    /// structs, we snapshot the ordered pair labels as a descriptor — the
    /// order comes from the `XRP_FIRST_UNIVERSE` const slice, not the
    /// element. Any re-ordering of the universe breaks this snapshot
    /// deliberately, surfacing the change for review.
    #[test]
    fn lab__top_bar_xrp_first() {
        use crate::lab::universe::XRP_FIRST_UNIVERSE;
        let pairs: Vec<&str> = XRP_FIRST_UNIVERSE.iter().map(|(_, s)| *s).collect();
        let summary = format!(
            "top_bar pairs=[{}] first={} second={} third={}",
            pairs.join(", "),
            pairs[0],
            pairs[1],
            pairs[2],
        );
        insta::assert_snapshot!("lab__top_bar_xrp_first", summary);
    }
}
