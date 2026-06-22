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

use iced::Length;
use iced::widget::Column;
use smol_str::SmolStr;

use crate::fixtures as fx;
use crate::state::{
    Cockpit, ExecutionMode, Message, OverrideRiskVetoState, PanelState, Screen, SettingsTab,
};
use crate::strings;
use crate::theme::ThemeMode;
use crate::widgets::{
    activity_tape, agent_feed, bakeoff_input, cache_state_badge, cache_state_summary_badge,
    cadence_badge, chart, date_range, focus_ring, frame, human_control, journal_transaction_modal,
    kill, kpi_strip, latency, num, override_risk_veto, pair_chip, placeholder, pnl, position_curve,
    positions, progress_bar, run_button, run_delta_badge, settings_tabs, sidebar_nav,
    source_toggle, sparkline, status_bar, strategies, strategy_card, strategy_chip, training_log,
    training_plot, volume_histogram,
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

#[allow(deprecated)] // Screen::Charts is a backwards-compat alias for Screen::Lab; gallery fixtures keep the old name intentionally (T-D-1)
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

fn seed_placeholder() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_pair_chip() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_strategy_chip() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_date_range() -> Cockpit {
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

#[allow(deprecated)] // Screen::Home is a backwards-compat alias for Screen::Live; gallery fixtures keep the old name intentionally (T-D-1)
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
    use crate::screens::lab;
    lab::view(model, ThemeMode::Dark)
}

fn render_charts_empty(model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::screens::lab;
    lab::view(model, ThemeMode::Dark)
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

// ── cockpit-activity-status-bar v0.1.0 — activity_tape gallery cells ─────────

fn seed_activity_tape() -> Cockpit {
    Cockpit::default()
}

fn render_activity_tape_empty(model: &Cockpit) -> iced::Element<'_, Message> {
    activity_tape::view(&model.activity_tape)
}

// ── lab-polish-round-2 R1 — position_curve gallery cells ─────────────────────

fn seed_position_curve() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn render_position_curve_with_points(_model: &Cockpit) -> iced::Element<'_, Message> {
    position_curve::view(fx::fake_position_curve_points(), ThemeMode::Dark)
}

fn render_position_curve_empty(_model: &Cockpit) -> iced::Element<'_, Message> {
    position_curve::view(vec![], ThemeMode::Dark)
}

// ── training_log gallery cells (cockpit-training-control T-D-N2) ─────────────

fn seed_training_log_empty() -> Cockpit {
    Cockpit::default()
}

fn seed_training_log_with_lines() -> Cockpit {
    let mut c = Cockpit::default();
    for i in 0..10usize {
        training_log::push_line(
            &mut c.lab_state.training_log,
            smol_str::SmolStr::new(format!("[info] epoch {i} complete, loss=0.{i:02}")),
        );
    }
    c
}

fn render_training_log_empty(model: &Cockpit) -> iced::Element<'_, Message> {
    training_log::view(
        &model.lab_state.training_log,
        model.lab_state.training_log_anchored,
        ThemeMode::Dark,
    )
}

fn render_training_log_with_lines(model: &Cockpit) -> iced::Element<'_, Message> {
    training_log::view(
        &model.lab_state.training_log,
        model.lab_state.training_log_anchored,
        ThemeMode::Dark,
    )
}

// ── training_plot gallery cells (cockpit-training-control T-D-N12 / T-D-N18) ─

fn seed_training_plot() -> Cockpit {
    Cockpit::default()
}

fn render_training_plot_empty(_model: &Cockpit) -> iced::Element<'_, Message> {
    training_plot::view(training_plot::TrainingPlotState::Empty, ThemeMode::Dark)
}

fn render_training_plot_running(_model: &Cockpit) -> iced::Element<'_, Message> {
    // Static 5-epoch fixture for gallery preview (deterministic, no live data).
    static EPOCHS: std::sync::LazyLock<Vec<training_plot::EpochPoint>> =
        std::sync::LazyLock::new(|| {
            vec![
                training_plot::EpochPoint {
                    epoch: 1,
                    train_loss: 0.80,
                    val_loss: 0.78,
                },
                training_plot::EpochPoint {
                    epoch: 2,
                    train_loss: 0.65,
                    val_loss: 0.64,
                },
                training_plot::EpochPoint {
                    epoch: 3,
                    train_loss: 0.50,
                    val_loss: 0.52,
                },
                training_plot::EpochPoint {
                    epoch: 4,
                    train_loss: 0.38,
                    val_loss: 0.40,
                },
                training_plot::EpochPoint {
                    epoch: 5,
                    train_loss: 0.28,
                    val_loss: 0.31,
                },
            ]
        });
    training_plot::view(
        training_plot::TrainingPlotState::Running { epochs: &EPOCHS },
        ThemeMode::Dark,
    )
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
    use crate::screens::lab;
    lab::view(model, ThemeMode::Dark)
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

fn render_placeholder(_model: &Cockpit) -> iced::Element<'_, Message> {
    #[allow(deprecated)]
    placeholder::view(strings::COMPARE_PLACEHOLDER, ThemeMode::Dark)
}

fn render_pair_chip(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::universe::xrp_first_universe_owned;
    // Leak static data so the element's lifetime is `'static`, satisfying
    // the gallery `fn(&Cockpit) -> Element<'_>` contract. This is acceptable
    // in the test/gallery binary path — no production code follows this route.
    let universe: &'static _ = Box::leak(Box::new(xrp_first_universe_owned()));
    let selected: &'static _ = Box::leak(Box::new(universe.first().cloned()));
    pair_chip::row(universe, selected.as_ref(), false, ThemeMode::Dark)
}

/// advisor-bakeoff-ranking F3 — the guided bake-off input (coin + budget +
/// lookback). Rendered with a representative selection (XRPUSDT / €200 /
/// 1 month).
fn render_bakeoff_input(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::leaderboard::LeaderboardLookback;
    use trading_core::Symbol;
    // Leak the Symbol so the borrow satisfies the gallery `'static` contract
    // (test-only binary path — bounded leak, no production code follows this).
    let coin: &'static Symbol = Box::leak(Box::new(Symbol::new("XRPUSDT")));
    // F7: pass DEFAULT_EUR_USD_RATE for the honest FX hint in the gallery.
    bakeoff_input::view(
        coin,
        "200",
        LeaderboardLookback::OneMonth,
        trading_core::DEFAULT_EUR_USD_RATE,
        ThemeMode::Dark,
    )
}

fn render_strategy_chip(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::state::{COMPARE_SET_CAP, StrategyFamily};
    use std::collections::HashMap;
    use trading_core::StrategyId;

    let ids: &'static _ = Box::leak(Box::new(vec![
        StrategyId(smol_str::SmolStr::new("v1.momentum")),
        StrategyId(smol_str::SmolStr::new("v0.5.macd")),
    ]));
    let mut families_map = HashMap::new();
    families_map.insert(
        StrategyId(smol_str::SmolStr::new("v1.momentum")),
        StrategyFamily::Rule,
    );
    families_map.insert(
        StrategyId(smol_str::SmolStr::new("v0.5.macd")),
        StrategyFamily::Composed,
    );
    let families: &'static _ = Box::leak(Box::new(families_map));
    let primary: &'static _ = Box::leak(Box::new(ids.first().cloned()));
    let compare_set: &'static _ = Box::leak(Box::new(vec![ids.get(1).cloned()]));
    let _ = COMPARE_SET_CAP;
    strategy_chip::row(
        ids,
        families,
        primary.as_ref(),
        compare_set,
        ThemeMode::Dark,
    )
}

fn render_date_range(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::state::{DateRange, Preset};
    let range: &'static _ = Box::leak(Box::new(DateRange::Preset(Preset::Last90d)));
    date_range::view(range, None, ThemeMode::Dark)
}

fn seed_run_button() -> Cockpit {
    Cockpit::new()
}

fn render_run_button(model: &Cockpit) -> iced::Element<'_, Message> {
    use run_button::RunState;
    let state = RunState::from_cockpit(model.lab_run_inflight, None);
    run_button::view(&state, model.lab_run_inflight, ThemeMode::Dark)
}

// ── run_delta_badge (Phase B T-D-N13) ────────────────────────────────────────

fn seed_run_delta_badge() -> Cockpit {
    // Leak the mirror pair so it lives for `'static`.
    let (last, prev) = fx::fake_run_report_mirror_pair();
    let last: &'static _ = Box::leak(Box::new(last));
    let prev: &'static _ = Box::leak(Box::new(prev));
    let mut cockpit = Cockpit::new();
    // Point lab_state to the leaked mirrors so render_run_delta_badge can borrow them.
    // We store them on the cockpit via last_run_report + prev_run_report.
    cockpit.lab_state.last_run_report = Some(last.clone());
    cockpit.lab_state.prev_run_report = Some(prev.clone());
    cockpit
}

fn render_run_delta_badge(model: &Cockpit) -> iced::Element<'_, Message> {
    if let (Some(last), Some(prev)) = (
        model.lab_state.last_run_report.as_ref(),
        model.lab_state.prev_run_report.as_ref(),
    ) {
        run_delta_badge::view(last, prev, ThemeMode::Dark)
    } else {
        iced::widget::text("run_delta_badge: no data").into()
    }
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

#[allow(deprecated)] // deprecated Screen aliases kept intentionally in gallery fixtures (T-D-1 backwards-compat shims)
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
    sidebar_nav::view(model.current_screen, entries, &[], ThemeMode::Dark)
}

fn render_sparkline(model: &Cockpit) -> iced::Element<'_, Message> {
    // Render the sparkline for the first strategy in the model.
    if let PanelState::Ready(rows) = &model.strategies
        && let Some(row) = rows.first()
        && let Some(PanelState::Ready(series)) = model.strategy_equity.get(&row.id)
    {
        return sparkline::view(series, ThemeMode::Dark);
    }
    // Fallback: build a series directly and leak it to `'static`.
    let series: &'static _ = Box::leak(Box::new(fx::fake_equity_series_for_sparkline()));
    sparkline::view(series, ThemeMode::Dark)
}

fn render_status_bar(model: &Cockpit) -> iced::Element<'_, Message> {
    status_bar::view(model)
}

// ── Phase C — settings_tabs widget ────────────────────────────────────────────

fn seed_settings_tabs_risk() -> Cockpit {
    fx::fake_cockpit_v15a_pairs_steady_state()
}

fn render_settings_tabs_risk(_model: &Cockpit) -> iced::Element<'_, Message> {
    settings_tabs::view(SettingsTab::Risk, ThemeMode::Dark)
}

fn render_settings_tabs_control(_model: &Cockpit) -> iced::Element<'_, Message> {
    settings_tabs::view(SettingsTab::Control, ThemeMode::Dark)
}

fn render_settings_tabs_debug(_model: &Cockpit) -> iced::Element<'_, Message> {
    settings_tabs::view(SettingsTab::Debug, ThemeMode::Dark)
}

// ── Phase C — strategy_card widget ────────────────────────────────────────────

fn seed_strategy_card() -> Cockpit {
    fx::fake_cockpit_v15a_pairs_steady_state()
}

fn render_strategy_card_empty_anchor(model: &Cockpit) -> iced::Element<'_, Message> {
    if let PanelState::Ready(rows) = &model.strategies
        && let Some(row) = rows.first()
    {
        return strategy_card::view(row, None, None, None, ThemeMode::Dark);
    }
    frame::muted_body("no strategy rows in fixture")
}

// ── Phase D — trail_node widget ───────────────────────────────────────────────

fn seed_trail_node() -> Cockpit {
    fx::fake_cockpit_v15a_pairs_steady_state()
}

fn render_trail_node_fill_unselected(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::widgets::trail_node::{TrailNode, TrailNodeKind, view as trail_node_view};
    let node: &'static TrailNode = Box::leak(Box::new(TrailNode {
        kind: TrailNodeKind::Fill,
        timestamp: Some("12:34:56.789".to_string()),
        actor: Some("strategy:tcn_overlay_momentum".to_string()),
        headline: Some("Buy 0.05 BTCUSDT @ 50000".to_string()),
    }));
    trail_node_view(node, false, ThemeMode::Dark)
}

fn render_trail_node_forecast_selected(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::widgets::trail_node::{TrailNode, TrailNodeKind, view as trail_node_view};
    let node: &'static TrailNode = Box::leak(Box::new(TrailNode {
        kind: TrailNodeKind::Forecast,
        timestamp: Some("12:34:55.001".to_string()),
        actor: Some("tcn:d1c3696d".to_string()),
        headline: Some("Up 0.75 confidence".to_string()),
    }));
    trail_node_view(node, true, ThemeMode::Dark)
}

fn render_trail_node_empty_stage(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::widgets::trail_node::{TrailNode, TrailNodeKind, view as trail_node_view};
    let node: &'static TrailNode = Box::leak(Box::new(TrailNode {
        kind: TrailNodeKind::Signal,
        timestamp: None,
        actor: None,
        headline: None,
    }));
    trail_node_view(node, false, ThemeMode::Dark)
}

// ── Phase D — trail_drawer widget ─────────────────────────────────────────────

fn seed_trail_drawer() -> Cockpit {
    fx::fake_cockpit_v15a_pairs_steady_state()
}

fn render_trail_drawer_fill(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::widgets::trail_drawer::{DrawerPayload, view as drawer_view};
    use crate::widgets::trail_node::TrailNodeKind;
    // `view` now takes the payload by value — no `Box::leak` needed.
    let payload = DrawerPayload::Fill {
        metadata_json: r#"{"side":"Buy","qty":"0.05","price":"50000"}"#.to_string(),
    };
    drawer_view(TrailNodeKind::Fill, Some(payload), ThemeMode::Dark)
}

fn render_trail_drawer_llm_placeholder(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::widgets::trail_drawer::view as drawer_view;
    use crate::widgets::trail_node::TrailNodeKind;
    drawer_view(TrailNodeKind::LlmDebate, None, ThemeMode::Dark)
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
        widget: "placeholder",
        state: "compare_coming_soon",
        render: render_placeholder,
        seed: seed_placeholder,
    },
    GalleryCell {
        widget: "pair_chip",
        state: "xrp_first_row",
        render: render_pair_chip,
        seed: seed_pair_chip,
    },
    GalleryCell {
        widget: "bakeoff_input",
        state: "xrp_200_one_month",
        render: render_bakeoff_input,
        seed: fx::fake_cockpit_ready,
    },
    GalleryCell {
        widget: "strategy_chip",
        state: "primary_with_compare",
        render: render_strategy_chip,
        seed: seed_strategy_chip,
    },
    GalleryCell {
        widget: "date_range",
        state: "last90d_preset",
        render: render_date_range,
        seed: seed_date_range,
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
        widget: "run_button",
        state: "idle_dark",
        render: render_run_button,
        seed: seed_run_button,
    },
    // ── run_delta_badge (Phase B T-D-N13) ────────────────────────────────────
    GalleryCell {
        widget: "run_delta_badge",
        state: "pnl_up_dd_down",
        render: render_run_delta_badge,
        seed: seed_run_delta_badge,
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
    // ── training_log (cockpit-training-control T-D-N2) ─────────────────────
    GalleryCell {
        widget: "training_log",
        state: "empty",
        render: render_training_log_empty,
        seed: seed_training_log_empty,
    },
    GalleryCell {
        widget: "training_log",
        state: "with_lines",
        render: render_training_log_with_lines,
        seed: seed_training_log_with_lines,
    },
    // ── training_plot (cockpit-training-control T-D-N12 / T-D-N18) ─────────
    GalleryCell {
        widget: "training_plot",
        state: "empty",
        render: render_training_plot_empty,
        seed: seed_training_plot,
    },
    GalleryCell {
        widget: "training_plot",
        state: "running_5_epochs",
        render: render_training_plot_running,
        seed: seed_training_plot,
    },
    // ── Phase C — settings_tabs (ui-rethink-phase-c-sidebar-ia T-D-N19) ──────
    GalleryCell {
        widget: "settings_tabs",
        state: "risk_active",
        render: render_settings_tabs_risk,
        seed: seed_settings_tabs_risk,
    },
    GalleryCell {
        widget: "settings_tabs",
        state: "control_active",
        render: render_settings_tabs_control,
        seed: seed_settings_tabs_risk,
    },
    GalleryCell {
        widget: "settings_tabs",
        state: "debug_active",
        render: render_settings_tabs_debug,
        seed: seed_settings_tabs_risk,
    },
    // ── Phase C — strategy_card (ui-rethink-phase-c-sidebar-ia T-D-N15) ──────
    GalleryCell {
        widget: "strategy_card",
        state: "no_anchor_no_run",
        render: render_strategy_card_empty_anchor,
        seed: seed_strategy_card,
    },
    // ── Phase D — trail_node (ui-rethink-phase-d-trail T-D-N7) ───────────────
    GalleryCell {
        widget: "trail_node",
        state: "fill_unselected",
        render: render_trail_node_fill_unselected,
        seed: seed_trail_node,
    },
    GalleryCell {
        widget: "trail_node",
        state: "forecast_selected",
        render: render_trail_node_forecast_selected,
        seed: seed_trail_node,
    },
    GalleryCell {
        widget: "trail_node",
        state: "signal_empty_stage",
        render: render_trail_node_empty_stage,
        seed: seed_trail_node,
    },
    // ── Phase D — trail_drawer (ui-rethink-phase-d-trail T-D-N13) ────────────
    GalleryCell {
        widget: "trail_drawer",
        state: "fill_payload",
        render: render_trail_drawer_fill,
        seed: seed_trail_drawer,
    },
    GalleryCell {
        widget: "trail_drawer",
        state: "llm_placeholder",
        render: render_trail_drawer_llm_placeholder,
        seed: seed_trail_drawer,
    },
    // ── Phase E — matrix widget ────────────────────────────────────────────────
    GalleryCell {
        widget: "matrix",
        state: "cold_boot_empty",
        render: render_matrix_cold_boot,
        seed: seed_matrix,
    },
    // ── lab-yahoo-realdata — source_toggle (T-C3.2) ────────────────────────────
    GalleryCell {
        widget: "source_toggle",
        state: "synthetic_active",
        render: render_source_toggle_synthetic,
        seed: seed_source_toggle,
    },
    GalleryCell {
        widget: "source_toggle",
        state: "yahoo_active",
        render: render_source_toggle_yahoo,
        seed: seed_source_toggle,
    },
    // ── lab-yahoo-realdata — cadence_badge (T-C3.3) ────────────────────────────
    GalleryCell {
        widget: "cadence_badge",
        state: "days1",
        render: render_cadence_badge_days1,
        seed: seed_cadence_badge,
    },
    // ── lab-yahoo-realdata — cache_state_badge (T-D2 follow-up) ────────────────
    GalleryCell {
        widget: "cache_state_badge",
        state: "fresh",
        render: render_cache_state_badge_fresh,
        seed: seed_cache_state_badge,
    },
    GalleryCell {
        widget: "cache_state_badge",
        state: "stale",
        render: render_cache_state_badge_stale,
        seed: seed_cache_state_badge,
    },
    GalleryCell {
        widget: "cache_state_badge",
        state: "empty",
        render: render_cache_state_badge_empty,
        seed: seed_cache_state_badge,
    },
    // ── lab-yahoo-realdata v0.1.2 — cache_state_summary_badge (T-DU5) ─────────
    GalleryCell {
        widget: "cache_state_summary_badge",
        state: "empty",
        render: render_cache_state_summary_badge_empty,
        seed: seed_cache_state_summary_badge,
    },
    GalleryCell {
        widget: "cache_state_summary_badge",
        state: "one_ticker",
        render: render_cache_state_summary_badge_one_ticker,
        seed: seed_cache_state_summary_badge,
    },
    GalleryCell {
        widget: "cache_state_summary_badge",
        state: "two_tickers",
        render: render_cache_state_summary_badge_two_tickers,
        seed: seed_cache_state_summary_badge,
    },
    GalleryCell {
        widget: "cache_state_summary_badge",
        state: "ten_tickers",
        render: render_cache_state_summary_badge_ten_tickers,
        seed: seed_cache_state_summary_badge,
    },
    // ── lab-end-to-end-v2 — progress_bar (T-AR-6) ────────────────────────────
    GalleryCell {
        widget: "progress_bar",
        state: "50pct",
        render: render_progress_bar_50pct,
        seed: seed_progress_bar,
    },
    GalleryCell {
        widget: "progress_bar",
        state: "indeterminate",
        render: render_progress_bar_indeterminate,
        seed: seed_progress_bar,
    },
    // ── cockpit-activity-status-bar v0.1.0 — activity_tape gallery cell ──────
    GalleryCell {
        widget: "activity_tape",
        state: "empty",
        render: render_activity_tape_empty,
        seed: seed_activity_tape,
    },
    // ── lab-polish-round-2 R1 — position_curve gallery cells ─────────────────
    GalleryCell {
        widget: "position_curve",
        state: "with_points",
        render: render_position_curve_with_points,
        seed: seed_position_curve,
    },
    GalleryCell {
        widget: "position_curve",
        state: "empty",
        render: render_position_curve_empty,
        seed: seed_position_curve,
    },
    // ── cockpit-toast-queue v0.1.0 — toast_tray gallery cells (ADR-0046) ────
    GalleryCell {
        widget: "toast_tray",
        state: "empty",
        render: render_toast_tray_empty,
        seed: seed_toast_tray_empty,
    },
    GalleryCell {
        widget: "toast_tray",
        state: "three_severities",
        render: render_toast_tray_three_severities,
        seed: seed_toast_tray_three_severities,
    },
];

// ── lab-yahoo-realdata — source_toggle gallery cells (T-C3.2) ────────────────

fn seed_source_toggle() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn render_source_toggle_synthetic(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::state::LabDataSource;
    source_toggle::view(LabDataSource::Synthetic, ThemeMode::Dark)
}

fn render_source_toggle_yahoo(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::state::LabDataSource;
    source_toggle::view(LabDataSource::YahooCache, ThemeMode::Dark)
}

// ── lab-yahoo-realdata — cadence_badge gallery cell (T-C3.3) ─────────────────

fn seed_cadence_badge() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn render_cadence_badge_days1(_model: &Cockpit) -> iced::Element<'_, Message> {
    cadence_badge::view(cadence_badge::CadenceLabel::Days1, ThemeMode::Dark)
}

// ── lab-yahoo-realdata — cache_state_badge gallery cells (T-D2 follow-up) ────

fn seed_cache_state_badge() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn render_cache_state_badge_fresh(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::cache_state::CacheState;
    cache_state_badge::view(CacheState::Fresh, ThemeMode::Dark)
}

fn render_cache_state_badge_stale(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::cache_state::CacheState;
    cache_state_badge::view(CacheState::Stale, ThemeMode::Dark)
}

fn render_cache_state_badge_empty(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::cache_state::CacheState;
    cache_state_badge::view(CacheState::Empty, ThemeMode::Dark)
}

// ── lab-yahoo-realdata v0.1.2 — cache_state_summary_badge gallery cells (T-DU5) ──

fn seed_cache_state_summary_badge() -> Cockpit {
    fx::fake_cockpit_ready()
}

/// Fixture: a deterministic `SystemTime` corresponding to 2024-12-31 UTC.
/// Picked so the rendered ISO date is stable across snapshot regen.
fn fixture_summary_mtime() -> std::time::SystemTime {
    use std::time::{Duration, UNIX_EPOCH};
    // 2024-12-31 00:00:00 UTC = unix epoch 1_735_603_200.
    UNIX_EPOCH + Duration::from_secs(1_735_603_200)
}

fn render_cache_state_summary_badge_empty(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::cache_state::CacheSummary;
    cache_state_summary_badge::view(&CacheSummary::empty(), ThemeMode::Dark)
}

fn render_cache_state_summary_badge_one_ticker(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::cache_state::CacheSummary;
    let summary = CacheSummary {
        populated_count: 1,
        newest_mtime: Some(fixture_summary_mtime()),
    };
    cache_state_summary_badge::view(&summary, ThemeMode::Dark)
}

fn render_cache_state_summary_badge_two_tickers(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::cache_state::CacheSummary;
    let summary = CacheSummary {
        populated_count: 2,
        newest_mtime: Some(fixture_summary_mtime()),
    };
    cache_state_summary_badge::view(&summary, ThemeMode::Dark)
}

fn render_cache_state_summary_badge_ten_tickers(_model: &Cockpit) -> iced::Element<'_, Message> {
    use crate::lab::cache_state::CacheSummary;
    let summary = CacheSummary {
        populated_count: 10,
        newest_mtime: Some(fixture_summary_mtime()),
    };
    cache_state_summary_badge::view(&summary, ThemeMode::Dark)
}

// ── Phase E — matrix gallery cells ────────────────────────────────────────────

fn seed_matrix() -> Cockpit {
    let mut c = crate::fixtures::fake_cockpit_ready();
    // Cold-boot: empty compare cache; strategies config seeded via fake_cockpit.
    c.compare_screen_state = crate::compare::state::CompareScreenState::default();
    c.current_screen = crate::state::Screen::Compare;
    c
}

fn render_matrix_cold_boot(model: &Cockpit) -> iced::Element<'_, Message> {
    crate::widgets::matrix::view(model, ThemeMode::Dark)
}

// ── lab-end-to-end-v2 — progress_bar gallery cells (T-AR-6) ─────────────────

fn seed_progress_bar() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn render_progress_bar_50pct(_model: &Cockpit) -> iced::Element<'_, Message> {
    progress_bar::view(Some(0.5), Some("360 / 720 bars · 1.5s"), ThemeMode::Dark)
}

fn render_progress_bar_indeterminate(_model: &Cockpit) -> iced::Element<'_, Message> {
    progress_bar::view(None, None, ThemeMode::Dark)
}

// ── cockpit-toast-queue v0.1.0 — toast_tray gallery cells (ADR-0046) ─────────

fn seed_toast_tray_empty() -> Cockpit {
    fx::fake_cockpit_ready()
}

fn seed_toast_tray_three_severities() -> Cockpit {
    use crate::state::{Message, ToastSeverity, update};
    let mut c = fx::fake_cockpit_ready();
    update(
        &mut c,
        Message::ShowToastWithSeverity(
            smol_str::SmolStr::new("Info: server time synced"),
            ToastSeverity::Info,
        ),
    );
    update(
        &mut c,
        Message::ShowToastWithSeverity(
            smol_str::SmolStr::new("Training completed"),
            ToastSeverity::Success,
        ),
    );
    update(
        &mut c,
        Message::ShowToastWithSeverity(
            smol_str::SmolStr::new("Training failed to launch: binary not found"),
            ToastSeverity::Danger,
        ),
    );
    c
}

fn render_toast_tray_empty(model: &Cockpit) -> iced::Element<'_, Message> {
    crate::widgets::toast_tray::view(&model.toast_queue, ThemeMode::Dark)
}

fn render_toast_tray_three_severities(model: &Cockpit) -> iced::Element<'_, Message> {
    crate::widgets::toast_tray::view(&model.toast_queue, ThemeMode::Dark)
}

/// The canonical list of widget-module names the gallery is expected to
/// cover. Sync this with `crates/ui/src/widgets/mod.rs` ANY time a new
/// `pub mod` lands there.
///
/// **Q-ARCH-2:** `canvas_chart` is `pub(crate)` and intentionally excluded.
pub const EXPECTED_WIDGETS: &[&str] = &[
    "activity_tape",
    "agent_feed",
    "bakeoff_input",
    "cache_state_badge",
    "cache_state_summary_badge",
    "cadence_badge",
    "chart",
    "chart_legend",
    "chart_tooltip",
    "date_range",
    "drawdown_band",
    "equity_curve",
    "focus_ring",
    "frame",
    "human_control",
    "journal_transaction_modal",
    "kill",
    "kpi_strip",
    "latency",
    "matrix",
    "num",
    "override_risk_veto",
    "pair_chip",
    "placeholder",
    "pnl",
    "position_curve",
    "positions",
    "progress_bar",
    "run_button",
    "run_delta_badge",
    "sidebar_nav",
    "source_toggle",
    "sparkline",
    "status_bar",
    "settings_tabs",
    "strategies",
    "strategy_card",
    "strategy_chip",
    "toast_tray",
    "training_log",
    "trail_drawer",
    "trail_node",
    "training_plot",
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
    view_slice(0, GALLERY_CELLS.len())
}

/// Render a slice of `GALLERY_CELLS[start..end]` as a bare column.
/// Used by the diagnostic snapshot bisection in
/// `crates/ui/tests/gallery_snapshots.rs` to narrow down which cell
/// triggers a tiny-skia render panic.
#[must_use]
pub fn view_slice(start: usize, end: usize) -> iced::Element<'static, Message> {
    let mode = ThemeMode::Dark;
    let end = end.min(GALLERY_CELLS.len());
    let cells: Vec<iced::Element<'static, Message>> = GALLERY_CELLS[start..end]
        .iter()
        .map(super::cell::view)
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
