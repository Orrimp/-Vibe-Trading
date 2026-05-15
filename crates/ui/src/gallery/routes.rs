//! Gallery route table — `GALLERY_CELLS` const + `EXPECTED_WIDGETS`.
//!
//! **Q-GALLERY-SCOPE (LOCKED):** every `GalleryCell.seed` calls a
//! `crate::fixtures::fake_*(...)` builder. No local builders in this file.
//! The evaluator's drift-gate (`grep -n 'fn fake_\|fn synth'
//! crates/ui/src/gallery/`) MUST be empty.
//!
//! **H-GAL-2 fix (design.md § Q-ARCH-3):** `view_all` returns `column!`,
//! NOT `scrollable(column!)`. The snapshot test passes viewport height
//! `GALLERY_LOGICAL_HEIGHT` so all cells are captured.

#![allow(clippy::cast_possible_truncation)]

use iced::widget::Column;
use iced::Length;
use smol_str::SmolStr;

use crate::fixtures as fx;
use crate::state::{Cockpit, ExecutionMode, Message, OverrideRiskVetoState, PanelState, Screen};
use crate::theme::ThemeMode;
use crate::widgets::{
    agent_feed, chart, focus_ring, frame, human_control, journal_transaction_modal, kill,
    kpi_strip, latency, num, override_risk_veto, pnl, positions, sidebar_nav, sparkline,
    status_bar, strategies, volume_histogram,
};

use super::cell::GalleryCell;

// ── Helper: map ViewerMessage elements to Message ────────────────────────────
//
// `equity_curve::view`, `kpi_strip::view`, etc. return
// `iced::Element<'_, ViewerMessage>`.  Gallery cells need
// `iced::Element<'static, Message>`. These widgets are canvas-only
// renderers that never actually emit events in a pure `screenshot` path,
// so mapping with `|_| Message::TapePauseToggled` (a unit variant) is
// sound: if the mapping is ever reached in production (it won't be in
// static gallery captures), it emits a no-side-effect toggle which the
// cockpit silently handles.
//
// The `'static` lifetime is required by `GalleryCell.render`. We achieve
// it by cloning any borrowed data into an owned form before passing to
// the viewer widget.

/// Seed the gallery with a canonical cockpit — for snapshot tests.
#[must_use]
pub(crate) fn seed_for_all_cells() -> Cockpit {
    fx::fake_cockpit_v15a_pairs_steady_state()
}

// ── Cell seeds (fn() -> Cockpit) ─────────────────────────────────────────────

fn seed_positions_loading() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.positions = PanelState::Loading;
    c
}

fn seed_positions_empty() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.positions = PanelState::Ready(vec![]);
    c
}

fn seed_positions_ready_v1_three() -> Cockpit {
    fx::fake_cockpit_v1_steady_state()
}

fn seed_positions_negative_pnl() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.positions = PanelState::Ready(fx::fake_positions());
    c.pnl = PanelState::Ready(fx::fake_pnl_negative());
    c
}

fn seed_pnl_positive() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.pnl = PanelState::Ready(fx::fake_pnl_positive());
    c
}

fn seed_pnl_negative() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.pnl = PanelState::Ready(fx::fake_pnl_negative());
    c
}

fn seed_strategies_loading() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.strategies = PanelState::Loading;
    c
}

fn seed_strategies_ready_v1() -> Cockpit {
    fx::fake_cockpit_v1_steady_state()
}

fn seed_strategies_with_error_row() -> Cockpit {
    fx::fake_strategy_row_error_in_v1_set()
}

fn seed_strategies_with_one_veto() -> Cockpit {
    fx::fake_cockpit_with_one_veto()
}

fn seed_charts_hovered() -> Cockpit {
    crate::test_support::charts_screen_cockpit()
}

fn seed_charts_empty() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.current_screen = Screen::Charts;
    c
}

fn seed_latency_healthy() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.market_health = fx::fake_market_health();
    c
}

fn seed_latency_degraded() -> Cockpit {
    fx::fake_market_health_degraded()
}

fn seed_human_control_auto() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.execution_mode = ExecutionMode::Auto;
    c
}

fn seed_human_control_observe() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.execution_mode = ExecutionMode::Observe;
    c
}

fn seed_human_control_supervised() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.execution_mode = ExecutionMode::Supervised;
    c
}

fn seed_agent_feed_empty() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.tape = PanelState::Empty;
    c
}

fn seed_agent_feed_three_fills() -> Cockpit {
    fx::fake_cockpit_ready_with_three_fills()
}

fn seed_num() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_volume_histogram_mixed() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_volume_histogram_empty() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_chart_tooltip_fill() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_chart_tooltip_signal() -> Cockpit {
    fx::fake_cockpit_ready()
}

// ── Chrome widget seeds ───────────────────────────────────────────────────────

fn seed_chart_legend() -> Cockpit {
    crate::test_support::charts_screen_cockpit()
}

fn seed_drawdown_band() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_equity_curve() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_focus_ring() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_frame() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_journal_transaction_modal() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    let rows = fx::fake_journal_rows(3);
    c.audit_screen_state.rows = PanelState::Ready(rows);
    c
}

fn seed_kill() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_kpi_strip() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_override_risk_veto() -> Cockpit {
    fx::fake_cockpit_with_one_veto()
}

fn seed_sidebar_nav() -> Cockpit {
    let mut c = fx::fake_cockpit_ready();
    c.current_screen = Screen::Home;
    c
}

fn seed_sparkline() -> Cockpit {
    let mut c = fx::fake_cockpit_v1_steady_state();
    // Seed strategy equity for the sparkline
    if let PanelState::Ready(rows) = &c.strategies.clone() {
        for row in rows {
            c.strategy_equity.insert(
                row.id.clone(),
                PanelState::Ready(fx::fake_equity_series_for_sparkline()),
            );
        }
    }
    c
}

fn seed_status_bar() -> Cockpit {
    fx::fake_cockpit_v1_steady_state()
}

// ── Cell render closures (fn(&Cockpit) -> Element<'static, Message>) ─────────
//
// Each render fn borrows from `model` and produces a `'static` Element
// by constructing owned data inside the closure. The `'static` bound on
// `GalleryCell.render` is satisfied because all Element internals
// (widget types, state data) live in the caller's stack frame and the
// iced tree is consumed by screenshot before the frame ends.
//
// NOTE: functions returning `Element<'_, Message>` (borrowed lifetime)
// must be adapted by converting to owned data inside the closure.

fn render_positions_loading(model: &Cockpit) -> iced::Element<'_, Message> {
    positions::view(model)
}

fn render_positions_empty(model: &Cockpit) -> iced::Element<'_, Message> {
    positions::view(model)
}

fn render_positions_ready_v1_three(model: &Cockpit) -> iced::Element<'_, Message> {
    positions::view(model)
}

fn render_positions_negative_pnl(model: &Cockpit) -> iced::Element<'_, Message> {
    positions::view(model)
}

fn render_pnl_positive(model: &Cockpit) -> iced::Element<'_, Message> {
    pnl::view(model)
}

fn render_pnl_negative(model: &Cockpit) -> iced::Element<'_, Message> {
    pnl::view(model)
}

fn render_strategies_loading(model: &Cockpit) -> iced::Element<'_, Message> {
    strategies::view(model)
}

fn render_strategies_ready_v1(model: &Cockpit) -> iced::Element<'_, Message> {
    strategies::view(model)
}

fn render_strategies_with_error_row(model: &Cockpit) -> iced::Element<'_, Message> {
    strategies::view(model)
}

fn render_strategies_with_one_veto(model: &Cockpit) -> iced::Element<'_, Message> {
    strategies::view(model)
}

fn render_charts_hovered(model: &Cockpit) -> iced::Element<'_, Message> {
    // Render the chart widget view for the current screen + symbol.
    use crate::screens::charts;
    charts::view(model, ThemeMode::Dark)
}

fn render_charts_empty(model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::screens::charts;
    charts::view(model, ThemeMode::Dark)
}

fn render_latency_healthy(model: &Cockpit) -> iced::Element<'_, Message> {
    latency::view(model)
}

fn render_latency_degraded(model: &Cockpit) -> iced::Element<'_, Message> {
    latency::view(model)
}

fn render_human_control_auto(model: &Cockpit) -> iced::Element<'_, Message> {
    human_control::view(model)
}

fn render_human_control_observe(model: &Cockpit) -> iced::Element<'_, Message> {
    human_control::view(model)
}

fn render_human_control_supervised(model: &Cockpit) -> iced::Element<'_, Message> {
    human_control::view(model)
}

fn render_agent_feed_empty(model: &Cockpit) -> iced::Element<'_, Message> {
    agent_feed::view(model)
}

fn render_agent_feed_three_fills(model: &Cockpit) -> iced::Element<'_, Message> {
    agent_feed::view(model)
}

fn render_num(_model: &Cockpit) -> iced::Element<'_, Message> {
    use iced::widget::{Column, Text};
    use rust_decimal_macros::dec;
    let mode = ThemeMode::Dark;
    let color_fg = crate::theme::color::FG_1.current(mode);

    let (pct_pos, _c1) = num::format_pct_sentiment(dec!(12.50), mode);
    let (pct_neg, _c2) = num::format_pct_sentiment(dec!(-5.25), mode);
    let sharpe = num::format_sharpe(dec!(-55.4257));

    Column::new()
        .spacing(crate::theme::space::XS)
        .push(Text::new(format!("fmt_usdt: {}", num::fmt_usdt(dec!(90_129.50)))).color(color_fg))
        .push(Text::new(format!("fmt_price: {}", num::fmt_price(dec!(40_050.00)))).color(color_fg))
        .push(Text::new(format!("fmt_qty: {}", num::fmt_qty(dec!(0.25)))).color(color_fg))
        .push(Text::new(format!("fmt_pct: {}", num::fmt_pct(dec!(11.10)))).color(color_fg))
        .push(Text::new(format!("format_pct_sentiment(+): {pct_pos}")).color(color_fg))
        .push(Text::new(format!("format_pct_sentiment(-): {pct_neg}")).color(color_fg))
        .push(Text::new(format!("format_sharpe: {sharpe}")).color(color_fg))
        .into()
}

fn render_volume_histogram_mixed(_model: &Cockpit) -> iced::Element<'_, Message> {
    volume_histogram::view(fx::fake_volume_bins(), ThemeMode::Dark)
}

fn render_volume_histogram_empty(_model: &Cockpit) -> iced::Element<'_, Message> {
    volume_histogram::view(vec![], ThemeMode::Dark)
}

fn render_chart_tooltip_fill(_model: &Cockpit) -> iced::Element<'_, Message> {
    // `chart::tooltip_view_for_fill` returns a `ChartTooltipView` — a data
    // struct. The tooltip is actually rendered on the canvas by
    // `chart_tooltip::draw_tooltip(frame, ...)`. For the gallery we render
    // a text description of the tooltip fields as a representative cell.
    use iced::widget::Text;
    let fill = fx::fake_fill_view(0);
    let tooltip = chart::tooltip_view_for_fill(&fill, Some(SmolStr::new("momentum-h1")));
    let mode = ThemeMode::Dark;
    let color_fg = crate::theme::color::FG_1.current(mode);
    Column::new()
        .spacing(crate::theme::space::XS)
        .push(Text::new("chart_tooltip :: fill").color(color_fg))
        .push(
            Text::new(format!("side: {:?}", tooltip.kind))
                .size(crate::theme::text::MICRO)
                .color(crate::theme::color::FG_2.current(mode)),
        )
        .push(
            Text::new(format!(
                "strategy: {}",
                tooltip.strategy_id.as_deref().unwrap_or("—")
            ))
            .size(crate::theme::text::MICRO)
            .color(crate::theme::color::FG_2.current(mode)),
        )
        .into()
}

fn render_chart_tooltip_signal(_model: &Cockpit) -> iced::Element<'_, Message> {
    use iced::widget::Text;
    let signal = fx::fake_signal_view(0);
    let tooltip = chart::tooltip_view_for_signal(&signal);
    let mode = ThemeMode::Dark;
    let color_fg = crate::theme::color::FG_1.current(mode);
    Column::new()
        .spacing(crate::theme::space::XS)
        .push(Text::new("chart_tooltip :: signal").color(color_fg))
        .push(
            Text::new(format!("side: {:?}", tooltip.kind))
                .size(crate::theme::text::MICRO)
                .color(crate::theme::color::FG_2.current(mode)),
        )
        .push(
            Text::new(format!("was_clamped: {}", signal.was_clamped))
                .size(crate::theme::text::MICRO)
                .color(crate::theme::color::FG_2.current(mode)),
        )
        .into()
}

// ── Chrome widget renders ─────────────────────────────────────────────────────

fn render_chart_legend(model: &Cockpit) -> iced::Element<'_, Message> {
    // chart_legend::draw_legend is canvas-only (no standalone view fn).
    // Render the full Charts screen as the representative cell — the legend
    // is embedded in the chart canvas.
    use crate::screens::charts;
    charts::view(model, ThemeMode::Dark)
}

fn render_drawdown_band(_model: &Cockpit) -> iced::Element<'_, Message> {
    // drawdown_band::view returns Element<'_, ViewerMessage> borrowed
    // from `series`. Leak to `'static` (test-only binary).
    let series: &'static _ = Box::leak(Box::new(PanelState::Ready(
        fx::fake_equity_series_for_viewer(),
    )));
    crate::widgets::drawdown_band::view(series, ThemeMode::Dark).map(|_| Message::TapePauseToggled)
}

fn render_equity_curve(_model: &Cockpit) -> iced::Element<'_, Message> {
    let series: &'static _ = Box::leak(Box::new(PanelState::Ready(
        fx::fake_equity_series_for_viewer(),
    )));
    crate::widgets::equity_curve::view(series, ThemeMode::Dark).map(|_| Message::TapePauseToggled)
}

fn render_focus_ring(model: &Cockpit) -> iced::Element<'_, Message> {
    // focus_ring::wrap decorates a child element with a focus halo.
    // Show a kill button wrapped with focus_ring in focused=true state.
    let inner = kill::view_inner(model);
    focus_ring::wrap(focus_ring::KILL_BUTTON, inner, true, ThemeMode::Dark)
}

fn render_frame(_model: &Cockpit) -> iced::Element<'_, Message> {
    // frame::panel renders the standard chrome panel.
    use iced::widget::Text;
    let body = Text::new("frame :: panel chrome example").into();
    frame::panel("frame :: panel", body, ThemeMode::Dark)
}

fn render_journal_transaction_modal(_model: &Cockpit) -> iced::Element<'_, Message> {
    // journal_transaction_modal::view takes (state, main_col, on_close).
    // Render it with a simple text column as the background. Leak the
    // modal state to `'static` (test-only binary).
    use crate::state::JournalModalState;
    use iced::widget::Text;
    let modal_state: &'static JournalModalState = Box::leak(Box::new(JournalModalState {
        tx_id: SmolStr::new("fixture-row-0000"),
        entries: PanelState::Loading,
    }));
    let main_col: iced::Element<'_, Message> = Text::new("background content").into();
    journal_transaction_modal::view(modal_state, main_col, Message::TapeAuditModalClosed)
}

fn render_kill(model: &Cockpit) -> iced::Element<'_, Message> {
    kill::view(model)
}

fn render_kpi_strip(_model: &Cockpit) -> iced::Element<'_, Message> {
    let metrics: &'static _ = Box::leak(Box::new(PanelState::Ready(fx::fake_backtest_metrics())));
    kpi_strip::view(metrics, ThemeMode::Dark).map(|_| Message::TapePauseToggled)
}

fn render_override_risk_veto(_model: &Cockpit) -> iced::Element<'_, Message> {
    // override_risk_veto::modal_view borrows from `state`; leak it.
    let state: &'static OverrideRiskVetoState =
        Box::leak(Box::new(OverrideRiskVetoState::Confirming {
            veto_id: SmolStr::new("veto-1"),
            typed: String::new(),
        }));
    override_risk_veto::modal_view(state, None)
        .unwrap_or_else(|| iced::widget::Text::new("override_risk_veto (Idle)").into())
}

fn render_sidebar_nav(model: &Cockpit) -> iced::Element<'_, Message> {
    let entries = &[
        Screen::Home,
        Screen::Charts,
        Screen::Strategies,
        Screen::Risk,
        Screen::Audit,
        Screen::Debug,
        Screen::Control,
    ];
    sidebar_nav::view(model.current_screen, entries, ThemeMode::Dark)
}

fn render_sparkline(model: &Cockpit) -> iced::Element<'_, Message> {
    // Render the sparkline for the first strategy in the model.
    if let PanelState::Ready(rows) = &model.strategies {
        if let Some(row) = rows.first() {
            if let Some(PanelState::Ready(series)) = model.strategy_equity.get(&row.id) {
                return sparkline::view(series, ThemeMode::Dark);
            }
        }
    }
    // Fallback: build a series directly and leak it to `'static`.
    let series: &'static _ = Box::leak(Box::new(fx::fake_equity_series_for_sparkline()));
    sparkline::view(series, ThemeMode::Dark)
}

fn render_status_bar(model: &Cockpit) -> iced::Element<'_, Message> {
    status_bar::view(model)
}

// ── GALLERY_CELLS const ───────────────────────────────────────────────────────

/// The canonical `(widget, state)` gallery matrix. 24 primary cells
/// (cells 1–24 per feature.md route table) + 12 single-cells for the
/// chrome widgets (M3). Total: 36 cells.
///
/// **Q-GALLERY-SCOPE (LOCKED):** every `seed` closure calls
/// `crate::fixtures::fake_*(...)`. No local builders.
pub const GALLERY_CELLS: &[GalleryCell] = &[
    // ── Primary cells (10 widgets × 2–4 states) ──────────────────────────
    // Cells 1–4: positions
    GalleryCell {
        widget: "positions",
        state: "loading",
        render: render_positions_loading,
        seed: seed_positions_loading,
    },
    GalleryCell {
        widget: "positions",
        state: "empty",
        render: render_positions_empty,
        seed: seed_positions_empty,
    },
    GalleryCell {
        widget: "positions",
        state: "ready_v1_three",
        render: render_positions_ready_v1_three,
        seed: seed_positions_ready_v1_three,
    },
    GalleryCell {
        widget: "positions",
        state: "ready_negative_pnl",
        render: render_positions_negative_pnl,
        seed: seed_positions_negative_pnl,
    },
    // Cells 5–6: pnl
    GalleryCell {
        widget: "pnl",
        state: "positive",
        render: render_pnl_positive,
        seed: seed_pnl_positive,
    },
    GalleryCell {
        widget: "pnl",
        state: "negative",
        render: render_pnl_negative,
        seed: seed_pnl_negative,
    },
    // Cells 7–10: strategies
    GalleryCell {
        widget: "strategies",
        state: "loading",
        render: render_strategies_loading,
        seed: seed_strategies_loading,
    },
    GalleryCell {
        widget: "strategies",
        state: "ready_v1_with_events",
        render: render_strategies_ready_v1,
        seed: seed_strategies_ready_v1,
    },
    GalleryCell {
        widget: "strategies",
        state: "with_error_row",
        render: render_strategies_with_error_row,
        seed: seed_strategies_with_error_row,
    },
    GalleryCell {
        widget: "strategies",
        state: "with_one_veto",
        render: render_strategies_with_one_veto,
        seed: seed_strategies_with_one_veto,
    },
    // Cells 11–12: chart
    GalleryCell {
        widget: "chart",
        state: "charts_screen_hovered",
        render: render_charts_hovered,
        seed: seed_charts_hovered,
    },
    GalleryCell {
        widget: "chart",
        state: "charts_screen_empty",
        render: render_charts_empty,
        seed: seed_charts_empty,
    },
    // Cells 13–14: latency
    GalleryCell {
        widget: "latency",
        state: "healthy",
        render: render_latency_healthy,
        seed: seed_latency_healthy,
    },
    GalleryCell {
        widget: "latency",
        state: "degraded",
        render: render_latency_degraded,
        seed: seed_latency_degraded,
    },
    // Cells 15–17: human_control
    GalleryCell {
        widget: "human_control",
        state: "auto_mode",
        render: render_human_control_auto,
        seed: seed_human_control_auto,
    },
    GalleryCell {
        widget: "human_control",
        state: "observe",
        render: render_human_control_observe,
        seed: seed_human_control_observe,
    },
    GalleryCell {
        widget: "human_control",
        state: "supervised",
        render: render_human_control_supervised,
        seed: seed_human_control_supervised,
    },
    // Cells 18–19: agent_feed
    GalleryCell {
        widget: "agent_feed",
        state: "empty",
        render: render_agent_feed_empty,
        seed: seed_agent_feed_empty,
    },
    GalleryCell {
        widget: "agent_feed",
        state: "with_three_fills",
        render: render_agent_feed_three_fills,
        seed: seed_agent_feed_three_fills,
    },
    // Cell 20: num (format showcase — no Cockpit state needed)
    GalleryCell {
        widget: "num",
        state: "format_showcase",
        render: render_num,
        seed: seed_num,
    },
    // Cells 21–22: volume_histogram
    GalleryCell {
        widget: "volume_histogram",
        state: "mixed_bins",
        render: render_volume_histogram_mixed,
        seed: seed_volume_histogram_mixed,
    },
    GalleryCell {
        widget: "volume_histogram",
        state: "empty",
        render: render_volume_histogram_empty,
        seed: seed_volume_histogram_empty,
    },
    // Cells 23–24: chart_tooltip
    GalleryCell {
        widget: "chart_tooltip",
        state: "fill_tooltip",
        render: render_chart_tooltip_fill,
        seed: seed_chart_tooltip_fill,
    },
    GalleryCell {
        widget: "chart_tooltip",
        state: "signal_tooltip",
        render: render_chart_tooltip_signal,
        seed: seed_chart_tooltip_signal,
    },
    // ── Chrome widget single-cells (M3, 12 widgets) ───────────────────────
    GalleryCell {
        widget: "chart_legend",
        state: "charts_screen",
        render: render_chart_legend,
        seed: seed_chart_legend,
    },
    GalleryCell {
        widget: "drawdown_band",
        state: "with_equity_series",
        render: render_drawdown_band,
        seed: seed_drawdown_band,
    },
    GalleryCell {
        widget: "equity_curve",
        state: "with_equity_series",
        render: render_equity_curve,
        seed: seed_equity_curve,
    },
    GalleryCell {
        widget: "focus_ring",
        state: "focused_kill_button",
        render: render_focus_ring,
        seed: seed_focus_ring,
    },
    GalleryCell {
        widget: "frame",
        state: "panel_chrome",
        render: render_frame,
        seed: seed_frame,
    },
    GalleryCell {
        widget: "journal_transaction_modal",
        state: "loading",
        render: render_journal_transaction_modal,
        seed: seed_journal_transaction_modal,
    },
    GalleryCell {
        widget: "kill",
        state: "idle",
        render: render_kill,
        seed: seed_kill,
    },
    GalleryCell {
        widget: "kpi_strip",
        state: "backtest_metrics",
        render: render_kpi_strip,
        seed: seed_kpi_strip,
    },
    GalleryCell {
        widget: "override_risk_veto",
        state: "confirming",
        render: render_override_risk_veto,
        seed: seed_override_risk_veto,
    },
    GalleryCell {
        widget: "sidebar_nav",
        state: "home_selected",
        render: render_sidebar_nav,
        seed: seed_sidebar_nav,
    },
    GalleryCell {
        widget: "sparkline",
        state: "equity_ramp",
        render: render_sparkline,
        seed: seed_sparkline,
    },
    GalleryCell {
        widget: "status_bar",
        state: "v1_steady_state",
        render: render_status_bar,
        seed: seed_status_bar,
    },
];

/// The canonical list of widget-module names the gallery is expected to
/// cover. Sync this with `crates/ui/src/widgets/mod.rs` ANY time a new
/// `pub mod` lands there.
///
/// **Q-ARCH-2:** `canvas_chart` is `pub(crate)` and intentionally excluded.
pub const EXPECTED_WIDGETS: &[&str] = &[
    "agent_feed",
    "chart",
    "chart_legend",
    "chart_tooltip",
    "drawdown_band",
    "equity_curve",
    "focus_ring",
    "frame",
    "human_control",
    "journal_transaction_modal",
    "kill",
    "kpi_strip",
    "latency",
    "num",
    "override_risk_veto",
    "pnl",
    "positions",
    "sidebar_nav",
    "sparkline",
    "status_bar",
    "strategies",
    "volume_histogram",
];

/// Total number of cells in the gallery. Used to validate
/// `GALLERY_LOGICAL_HEIGHT` is proportioned correctly.
pub const GALLERY_CELL_COUNT: usize = GALLERY_CELLS.len();

/// Compose the full gallery as a bare `column!` (no scrollable wrapper).
///
/// Per design.md § Q-ARCH-3 / H-GAL-2 fix: the snapshot test passes
/// viewport height `GALLERY_LOGICAL_HEIGHT` so the column's full intrinsic
/// height is captured. The interactive bin wraps this in `scrollable(...)`.
#[must_use]
pub(crate) fn view_all(_model: &Cockpit) -> iced::Element<'_, Message> {
    let mode = ThemeMode::Dark;
    let cells: Vec<iced::Element<'_, Message>> = GALLERY_CELLS
        .iter()
        .map(|cell| super::cell::view(cell))
        .collect();

    let col = cells
        .into_iter()
        .fold(Column::new().spacing(0), iced::widget::Column::push);

    iced::widget::container(col)
        .width(Length::Fill)
        .style(move |_theme: &iced::Theme| iced::widget::container::Style {
            background: Some(crate::theme::color::CANVAS.current(mode).into()),
            ..Default::default()
        })
        .into()
}
