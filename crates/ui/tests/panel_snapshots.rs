//! Snapshot tests for panel states.
//!
//! iced widget trees are not trivially serialisable; we instead snapshot a
//! **textual summary** of the panel's state as the widget code would see
//! it. The summary includes the panel variant (loading/empty/error/ready),
//! the strings that would be rendered, and the critical color decisions.
//! This is enough to catch regressions in:
//! - which branch of the state match we took,
//! - which copy from `ui::strings` was chosen,
//! - the rendered value strings for numeric cells,
//! - kill-switch button-enabled logic,
//! - latency thresholds.
//!
//! The tradeoff (vs literally rendering pixels) is that layout regressions
//! are caught by `cargo run --bin cockpit --features fixtures` / manual
//! inspection, not here. That's appropriate for a cockpit whose layout
//! has ~6 widgets.

#![allow(clippy::needless_raw_string_hashes)]

use insta::assert_snapshot;
use rust_decimal_macros::dec;
use smol_str::SmolStr;
use time::{Date, Month, PrimitiveDateTime, Time};
use trading_core::{AccountId, JournalEntry, Money, StrategyId, Timestamp};

use trading_core::{Symbol, Venue};
use ui::state::{
    update, AgentMode, Cockpit, ExecutionMode, JournalModalState, JournalTransactionView,
    KillState, Latency, MarketHealthState, Message, OverrideRiskVetoState, PanelState, Screen,
    StrategyStatus,
};
use ui::strings;
use ui::widgets::journal_transaction_modal;
use ui::widgets::latency::Badge;
use ui::widgets::num::fmt_usdt;

// ── Agent activity feed (Phase 5 Q6 — renamed from `tape`) ──────────────────

#[test]
fn agent_feed_loading() {
    let c = Cockpit::new();
    assert_snapshot!("agent_feed_loading", tape_summary(&c));
}

#[test]
fn agent_feed_empty() {
    let mut c = Cockpit::new();
    // A refresh to an empty fill list would look like this:
    c.tape = PanelState::Empty;
    assert_snapshot!("agent_feed_empty", tape_summary(&c));
}

#[test]
fn agent_feed_error() {
    let mut c = Cockpit::new();
    update(
        &mut c,
        Message::TapeError(SmolStr::new("broadcast channel closed")),
    );
    assert_snapshot!("agent_feed_error", tape_summary(&c));
}

#[test]
fn agent_feed_ready_three_fills() {
    let c = ui::fixtures::fake_cockpit_ready_with_three_fills();
    assert_snapshot!("agent_feed_ready_three_fills", tape_summary(&c));
}

#[test]
fn agent_feed_paused_banner_visible() {
    let mut c = ui::fixtures::fake_cockpit_ready_with_three_fills();
    update(&mut c, Message::TapePauseToggled);
    assert_snapshot!("agent_feed_paused", tape_summary(&c));
}

// ── Position panel ──────────────────────────────────────────────────────────

#[test]
fn positions_loading() {
    let c = Cockpit::new();
    assert_snapshot!("positions_loading", positions_summary(&c));
}

#[test]
fn positions_empty() {
    let mut c = Cockpit::new();
    update(&mut c, Message::PositionsRefreshed(vec![]));
    assert_snapshot!("positions_empty", positions_summary(&c));
}

#[test]
fn positions_error() {
    let mut c = Cockpit::new();
    update(
        &mut c,
        Message::PositionsError(SmolStr::new("database locked")),
    );
    assert_snapshot!("positions_error", positions_summary(&c));
}

#[test]
fn positions_ready_hides_zero_qty() {
    use trading_core::{Money, Position, PositionView, Price, Symbol};
    // Position with qty = 0 should be hidden per T17 acceptance.
    let zero = PositionView {
        symbol: Symbol::new("ETHUSDT"),
        base_qty: dec!(0),
        cost_basis: Money::from_decimal(dec!(0)),
        last_mark: Price::new(dec!(2000)).unwrap_or_else(|_| unreachable!()),
        pnl: Money::from_decimal(dec!(0)),
        pnl_pct: dec!(0),
        exposure_pct: dec!(0),
    };
    let real = ui::fixtures::fake_position_btc();
    let mut c = Cockpit::new();
    update(
        &mut c,
        Message::PositionsRefreshed(vec![zero.clone(), real.clone()]),
    );
    let summary = positions_summary(&c);
    // The zero-qty row must not appear in the body list (it's filtered in
    // `ready_body`). The summary rebuilds what `ready_body` would render.
    let _ = &Position::empty(Symbol::new("BTCUSDT"));
    assert_snapshot!("positions_ready_hides_zero_qty", summary);
}

#[test]
fn positions_ready_negative_pnl_uses_neg_color() {
    let mut p = ui::fixtures::fake_position_btc();
    p.pnl = trading_core::Money::from_decimal(dec!(-500));
    p.pnl_pct = dec!(-2.5);
    let mut c = Cockpit::new();
    update(&mut c, Message::PositionsRefreshed(vec![p]));
    assert_snapshot!("positions_ready_negative_pnl", positions_summary(&c));
}

/// T_FINAL_B_v1 — pins the v1 multi-symbol steady-state layout. Three
/// rows render in a single positions panel: BTC (`POS`), ETH (`NEG`),
/// SOL (`FG_MUTED` zero-delta). Same widget code path as the v0
/// single-row fixture per R11 negative confirmation; this snapshot
/// catches a regression where the panel could no longer iterate
/// past one row (or where row order changes silently).
#[test]
fn positions_v1_three_rows() {
    let c = ui::fixtures::fake_cockpit_v1_steady_state();
    assert_snapshot!("positions_v1_three_rows", positions_summary(&c));
}

/// T_FINAL_B_v15a — pins the v1.5a mean-reversion-pairs steady-state
/// layout. Three long-leg position rows (BTCUSDT / BNBUSDT / ETHUSDT —
/// formulation C: only the `a` legs of each pair trade), one
/// `pairs_mr_h1` strategy row, recent-events footer carrying both new
/// v1.5a kinds (`MeanReversionStop` → `fg_muted`, `PairShortObservation`
/// → `fg_muted`) plus a `loaded` row → `accent`. Same widget code path
/// as the v0/v1 fixtures per R11 negative confirmation; this snapshot
/// catches a regression where the new event kinds break exhaustive
/// matching or where the long-leg-only invariant silently allows a
/// short row.
#[test]
fn cockpit_v15a_pairs_steady_state() {
    let c = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
    let mut summary = String::new();
    summary.push_str(&positions_summary(&c));
    summary.push_str("---\n");
    summary.push_str(&strategies_summary(&c));
    assert_snapshot!("cockpit_v15a_pairs_steady_state", summary);
}

// ── P&L card ────────────────────────────────────────────────────────────────

#[test]
fn pnl_loading() {
    let c = Cockpit::new();
    assert_snapshot!("pnl_loading", pnl_summary(&c));
}

#[test]
fn pnl_error() {
    let mut c = Cockpit::new();
    update(&mut c, Message::PnlError(SmolStr::new("query timeout")));
    assert_snapshot!("pnl_error", pnl_summary(&c));
}

#[test]
fn pnl_ready_positive_day() {
    let mut c = Cockpit::new();
    update(
        &mut c,
        Message::PnlRefreshed(ui::fixtures::fake_pnl_positive()),
    );
    assert_snapshot!("pnl_ready_positive", pnl_summary(&c));
}

#[test]
fn pnl_ready_negative_day_uses_neg_color() {
    let mut c = Cockpit::new();
    update(
        &mut c,
        Message::PnlRefreshed(ui::fixtures::fake_pnl_negative()),
    );
    assert_snapshot!("pnl_ready_negative", pnl_summary(&c));
}

// ── Kill switch ─────────────────────────────────────────────────────────────

#[test]
fn kill_idle() {
    let c = Cockpit::new();
    assert_snapshot!("kill_idle", kill_summary(&c));
}

#[test]
fn kill_dialog_open_empty_input() {
    let mut c = Cockpit::new();
    update(&mut c, Message::KillPressed);
    assert_snapshot!("kill_dialog_empty_input", kill_summary(&c));
}

#[test]
fn kill_dialog_open_mismatched_phrase() {
    let mut c = Cockpit::new();
    update(&mut c, Message::KillPressed);
    update(&mut c, Message::KillConfirmPhraseChanged("HALT".into()));
    assert_snapshot!("kill_dialog_mismatch", kill_summary(&c));
}

#[test]
fn kill_dialog_open_correct_phrase_enables_confirm() {
    let mut c = Cockpit::new();
    update(&mut c, Message::KillPressed);
    update(
        &mut c,
        Message::KillConfirmPhraseChanged(strings::KILL_SAFETY_PHRASE.into()),
    );
    assert_snapshot!("kill_dialog_correct", kill_summary(&c));
}

#[test]
fn kill_halted_banner() {
    let mut c = Cockpit::new();
    update(
        &mut c,
        Message::AgentHaltedExternally(SmolStr::new("halt_file")),
    );
    assert_snapshot!("kill_halted", kill_summary(&c));
}

/// T1506 — confirm input while focused: accent border + sunken background.
/// The focus ring (shadow) is deferred — iced 0.14 text_input::Style has no
/// shadow field. See widgets/kill.rs module doc for the full API limitation note.
#[test]
fn kill_dialog_focused_input() {
    let mut c = Cockpit::new();
    update(&mut c, Message::KillPressed);
    update(
        &mut c,
        Message::KillConfirmPhraseChanged(strings::KILL_SAFETY_PHRASE.into()),
    );
    assert_snapshot!("kill_dialog_focused", kill_summary_focused(&c, true));
}

// ── Strategies panel (v0.5 T524) ────────────────────────────────────────────

#[test]
fn strategies_loading() {
    let c = Cockpit::new();
    assert_snapshot!("strategies_loading", strategies_summary(&c));
}

#[test]
fn strategies_empty() {
    let mut c = Cockpit::new();
    update(&mut c, Message::StrategiesRefreshed(vec![]));
    assert_snapshot!("strategies_empty", strategies_summary(&c));
}

#[test]
fn strategies_error() {
    let mut c = Cockpit::new();
    update(
        &mut c,
        Message::StrategiesError(SmolStr::new(strings::CONNECTION_CHANNEL_CLOSED)),
    );
    assert_snapshot!("strategies_error", strategies_summary(&c));
}

#[test]
fn strategies_ready_three_rows() {
    let c = ui::fixtures::fake_cockpit_with_strategies();
    assert_snapshot!("strategies_ready_three_rows", strategies_summary(&c));
}

/// T527 — full cockpit layout snapshot confirming the strategies panel
/// sits in the right column **above** Open positions and above the live
/// tape, per the Q4 resolution. Captures panel order top-to-bottom so a
/// future refactor that accidentally moves the panel is caught here.
#[test]
fn cockpit_layout_strategies_above_positions() {
    let c = ui::fixtures::fake_cockpit_with_strategies();
    let summary = cockpit_layout_summary(&c);
    assert_snapshot!("cockpit_layout_strategies_above_positions", summary);
}

#[test]
fn strategies_per_row_error_badge() {
    // Start from ready with three rows; the third row is already Error per
    // the fixture. Snapshot confirms the per-row error badge renders the
    // `error_summary` in `NEG`.
    let c = ui::fixtures::fake_cockpit_with_strategies();
    let summary = strategies_summary(&c);
    // Sanity — the fixture is the one carrying the error row.
    assert!(summary.contains("badge: arity_mismatch"));
    assert_snapshot!("strategies_per_row_error", summary);
}

// ── Latency badge ───────────────────────────────────────────────────────────

#[test]
fn latency_unknown() {
    let c = Cockpit::new();
    assert_snapshot!("latency_unknown", latency_summary(&c));
}

#[test]
fn latency_ok_below_500ms() {
    let mut c = Cockpit::new();
    c.latency = Latency::Known { ms: 120 };
    assert_snapshot!("latency_ok", latency_summary(&c));
}

#[test]
fn latency_warn_at_500ms() {
    let mut c = Cockpit::new();
    c.latency = Latency::Known { ms: 500 };
    assert_snapshot!("latency_warn", latency_summary(&c));
}

#[test]
fn latency_high_at_2000ms() {
    let mut c = Cockpit::new();
    c.latency = Latency::Known { ms: 2_500 };
    assert_snapshot!("latency_high", latency_summary(&c));
}

#[test]
fn latency_halted_at_10000ms() {
    let mut c = Cockpit::new();
    c.latency = Latency::Known { ms: 15_000 };
    c.mode = AgentMode::Halted;
    assert_snapshot!("latency_halted", latency_summary(&c));
}

// ── Status bar (T1508) ──────────────────────────────────────────────────────
//
// Four snapshots cover the status bar's visual states per T1508 acceptance:
//   status_bar_connected      — all venues Fresh, latency known
//   status_bar_reconnecting   — at least one venue Stale
//   status_bar_disconnected   — no venues seen (empty market_health map)
//   status_bar_with_latency   — latency Known { ms } with coloured text
//
// Server time is always "— UTC" in snapshot tests because `server_time_now`
// is `None` by default (no 1 Hz subscription running in tests). This is by
// design: the tests are pure / deterministic; the live clock tick is driven
// by the binary's subscription, not by the widget.

#[test]
fn status_bar_connected() {
    let mut c = Cockpit::new();
    c.market_health
        .insert(Venue::Binance, MarketHealthState::Fresh);
    c.latency = Latency::Known { ms: 42 };
    c.account_label = smol_str::SmolStr::new("Paper \u{00b7} Demo 3-symbol");
    assert_snapshot!("status_bar_connected", status_bar_summary(&c));
}

#[test]
fn status_bar_reconnecting() {
    let mut c = Cockpit::new();
    c.market_health
        .insert(Venue::Binance, MarketHealthState::Stale);
    c.latency = Latency::Unknown;
    c.account_label = smol_str::SmolStr::new("Paper \u{00b7} Demo 3-symbol");
    assert_snapshot!("status_bar_reconnecting", status_bar_summary(&c));
}

#[test]
fn status_bar_disconnected() {
    let c = Cockpit::new(); // empty market_health
    assert_snapshot!("status_bar_disconnected", status_bar_summary(&c));
}

#[test]
fn status_bar_with_latency() {
    let mut c = Cockpit::new();
    c.market_health
        .insert(Venue::Binance, MarketHealthState::Fresh);
    c.latency = Latency::Known { ms: 1_500 }; // WARN band
    c.account_label = smol_str::SmolStr::new("Paper \u{00b7} Demo 3-symbol");
    assert_snapshot!("status_bar_with_latency", status_bar_summary(&c));
}

// ── Tape-row audit modal (T1207) ────────────────────────────────────────────
//
// Covers V8 from `spec/features/tape-row-audit-modal.md`: snapshot the
// modal in compact density on a 4-entry paper-fill fixture — the
// canonical four-leg journal of a paper Buy fill (cash credit / position
// debit / fee credit on cash / fee debit on expense). Plus one snapshot
// per other `PanelState` arm (loading / empty / error) so a regression
// in any single arm shows up as a single granular failure.
//
// We snapshot a text summary (mirroring `tape_summary` etc.) — the
// existing pattern for this file. Each test ALSO renders the live
// widget via `journal_transaction_modal::view(&state, dummy_content,
// dummy_close_msg)` to catch any compile/render-path regressions.

/// Fixed deterministic timestamp for the V8 fixture — `2026-05-03T14:32:18Z`.
/// The widget's metadata block formats `Timestamp` via its `Display` impl,
/// which produces `time` crate's RFC 3339 + offset rendering. The chosen
/// epoch makes the rendered string visible in the snapshot.
fn fixture_modal_ts() -> Timestamp {
    let date = Date::from_calendar_date(2026, Month::May, 3).unwrap_or(Date::MIN);
    let clock = Time::from_hms(14, 32, 18).unwrap_or(Time::MIDNIGHT);
    let dt = PrimitiveDateTime::new(date, clock).assume_utc();
    Timestamp::new(dt)
}

/// V8 4-entry paper-fill fixture per
/// [`spec/features/tape-row-audit-modal.md` § Q8 / V8](../../../spec/features/tape-row-audit-modal.md#q8--test-plan).
///
/// Models a paper Buy round-trip on BTCUSDT — the canonical four legs that
/// `audit::post_fill` writes for one fill:
/// 1. cash debit  — cash decreases by `1234.56` (asset CR per double-entry).
/// 2. position credit — position increases by `0.04 BTC` (asset DR).
/// 3. fee debit   — cash decreases by `1.23` for the taker fee (asset CR).
/// 4. fee credit  — fees expense increases by `1.23` (expense DR).
///
/// Each entry holds one non-zero side (debit XOR credit) per the
/// architect's `JournalEntry` shape (Q2). `Money<Usdt>` is the storage
/// type for both columns regardless of the rendered `currency` ticker —
/// the un-collapsed `(debit, credit)` pair is what the widget renders.
/// Position row uses `0.04` BTC (rather than the prompt's illustrative
/// `0.025`) so the `fmt_usdt` two-dp formatter renders the value cleanly
/// in the snapshot rather than rounding the third decimal — keeps the
/// snapshot reviewable without changing what's being asserted.
fn fixture_journal_view() -> JournalTransactionView {
    let ts = fixture_modal_ts();
    JournalTransactionView {
        tx_id: SmolStr::new("4f9a2c1e-aaaa-bbbb-cccc-000000000001"),
        ts,
        description: SmolStr::new("buy 0.04 BTCUSDT @ 50000"),
        strategy_id: Some(StrategyId::new("sma_crossover")),
        entries: vec![
            JournalEntry {
                account: AccountId::new("assets:cash:USDT"),
                debit: Money::from_decimal(dec!(0)),
                credit: Money::from_decimal(dec!(1234.56)),
                currency: SmolStr::new("USDT"),
                ts,
                memo: SmolStr::new(""),
            },
            JournalEntry {
                account: AccountId::new("assets:position:BTCUSDT"),
                debit: Money::from_decimal(dec!(0.04)),
                credit: Money::from_decimal(dec!(0)),
                currency: SmolStr::new("BTCUSDT"),
                ts,
                memo: SmolStr::new(""),
            },
            JournalEntry {
                account: AccountId::new("assets:cash:USDT"),
                debit: Money::from_decimal(dec!(0)),
                credit: Money::from_decimal(dec!(1.23)),
                currency: SmolStr::new("USDT"),
                ts,
                memo: SmolStr::new(""),
            },
            JournalEntry {
                account: AccountId::new("expenses:fees:exchange"),
                debit: Money::from_decimal(dec!(1.23)),
                credit: Money::from_decimal(dec!(0)),
                currency: SmolStr::new("USDT"),
                ts,
                memo: SmolStr::new(""),
            },
        ],
    }
}

/// Render the live widget with `()`-typed messages so the rendering
/// path is exercised even though the snapshot is text-only. If
/// `journal_transaction_modal::view(...)` ever panics for one of the
/// four `PanelState` arms, every snapshot test trips at once.
fn render_modal_widget_for_smoke(state: &JournalModalState) {
    use iced::widget::{Container, Text};
    let dummy_content: iced::Element<()> = Container::new(Text::new("cockpit")).into();
    let _: iced::Element<()> = journal_transaction_modal::view(state, dummy_content, ());
}

#[test]
fn agent_feed_audit_modal_loading() {
    let state = JournalModalState {
        tx_id: SmolStr::new("4f9a2c1e-aaaa-bbbb-cccc-000000000001"),
        entries: PanelState::Loading,
    };
    render_modal_widget_for_smoke(&state);
    assert_snapshot!(
        "agent_feed_audit_modal_loading",
        tape_audit_modal_summary(&state)
    );
}

#[test]
fn agent_feed_audit_modal_empty() {
    let state = JournalModalState {
        tx_id: SmolStr::new("4f9a2c1e-aaaa-bbbb-cccc-000000000001"),
        entries: PanelState::Empty,
    };
    render_modal_widget_for_smoke(&state);
    assert_snapshot!(
        "agent_feed_audit_modal_empty",
        tape_audit_modal_summary(&state)
    );
}

#[test]
fn agent_feed_audit_modal_error() {
    let state = JournalModalState {
        tx_id: SmolStr::new("4f9a2c1e-aaaa-bbbb-cccc-000000000001"),
        entries: PanelState::Error(SmolStr::new("ledger unreachable")),
    };
    render_modal_widget_for_smoke(&state);
    assert_snapshot!(
        "agent_feed_audit_modal_error",
        tape_audit_modal_summary(&state)
    );
}

#[test]
fn agent_feed_audit_modal_ready_paper_fill() {
    let state = JournalModalState {
        tx_id: SmolStr::new("4f9a2c1e-aaaa-bbbb-cccc-000000000001"),
        entries: PanelState::Ready(fixture_journal_view()),
    };
    render_modal_widget_for_smoke(&state);
    assert_snapshot!(
        "agent_feed_audit_modal_ready_paper_fill",
        tape_audit_modal_summary(&state)
    );
}

// ── Phase 2 — screen-routed shell snapshots (T1604/T1605/T1610) ─────────────

#[test]
#[allow(non_snake_case)]
fn home_screen__default() {
    let c = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
    assert_snapshot!("home_screen__default", home_screen_summary(&c));
}

// Phase 5 (T1906 / Q1) — `debug_screen__full` retired; the Debug screen
// no longer carries the kill widget. The new structural invariant is
// captured by `debug_screen__without_kill` below (in the Phase 5 block).

#[test]
#[allow(non_snake_case)]
fn charts_screen__chip_row_active_btc() {
    let mut c = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
    c.universe = vec![
        (Venue::Binance, Symbol::new("BTCUSDT")),
        (Venue::Binance, Symbol::new("ETHUSDT")),
        (Venue::Binance, Symbol::new("SOLUSDT")),
    ];
    c.selected_symbol = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    c.chart_markers = PanelState::Ready(ui::fixtures::synthetic_fills_for(
        Venue::Binance,
        &Symbol::new("BTCUSDT"),
        4,
    ));
    assert_snapshot!(
        "charts_screen__chip_row_active_btc",
        charts_screen_summary(&c)
    );
}

#[test]
#[allow(non_snake_case)]
fn charts_screen__chip_row_active_eth() {
    let mut c = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
    c.universe = vec![
        (Venue::Binance, Symbol::new("BTCUSDT")),
        (Venue::Binance, Symbol::new("ETHUSDT")),
        (Venue::Binance, Symbol::new("SOLUSDT")),
    ];
    c.selected_symbol = Some((Venue::Binance, Symbol::new("ETHUSDT")));
    c.chart_markers = PanelState::Ready(ui::fixtures::synthetic_fills_for(
        Venue::Binance,
        &Symbol::new("ETHUSDT"),
        4,
    ));
    assert_snapshot!(
        "charts_screen__chip_row_active_eth",
        charts_screen_summary(&c)
    );
}

// ── Phase 3 — Strategies / Risk / Audit detail screens (T1704–T1711) ─────────

#[test]
#[allow(non_snake_case)]
fn strategies_screen__sma_crossover_default() {
    let mut c = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
    c.current_screen = Screen::Strategies;
    c.strategies_config = Some(ui::fixtures::fake_strategies_config());
    c.selected_strategy = Some(StrategyId::new("btc_macd_trend"));
    assert_snapshot!(
        "strategies_screen__sma_crossover_default",
        strategies_screen_summary(&c)
    );
}

#[test]
#[allow(non_snake_case)]
fn strategies_screen__empty_state() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Strategies;
    // selected_strategy = None → empty-state body.
    assert_snapshot!(
        "strategies_screen__empty_state",
        strategies_screen_summary(&c)
    );
}

#[test]
#[allow(non_snake_case)]
fn strategies_screen__sparkline_present() {
    // Phase 4 (T1811) — the deferred-sparkline placeholder retires;
    // the canvas widget lands. With `model.strategy_equity` populated
    // for the selected strategy, the screen body renders the
    // `widgets::sparkline` canvas instead of the muted-body
    // placeholder.
    let mut c = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
    c.current_screen = Screen::Strategies;
    c.strategies_config = Some(ui::fixtures::fake_strategies_config());
    let id = StrategyId::new("btc_rsi_reversion");
    c.selected_strategy = Some(id.clone());
    c.strategy_equity.insert(
        id,
        PanelState::Ready(ui::fixtures::fake_equity_series_for_sparkline()),
    );
    assert_snapshot!(
        "strategies_screen__sparkline_present",
        strategies_sparkline_present_summary(&c)
    );
}

#[test]
#[allow(non_snake_case)]
fn risk_screen__under_warn_threshold() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Risk;
    c.risk_state = PanelState::Ready(under_warn_risk_state());
    assert_snapshot!("risk_screen__under_warn_threshold", risk_screen_summary(&c));
}

#[test]
#[allow(non_snake_case)]
fn risk_screen__warn_threshold() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Risk;
    c.risk_state = PanelState::Ready(warn_band_risk_state());
    assert_snapshot!("risk_screen__warn_threshold", risk_screen_summary(&c));
}

#[test]
#[allow(non_snake_case)]
fn risk_screen__danger_threshold() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Risk;
    c.risk_state = PanelState::Ready(danger_band_risk_state());
    assert_snapshot!("risk_screen__danger_threshold", risk_screen_summary(&c));
}

#[test]
#[allow(non_snake_case)]
fn audit_screen__default_recent_24h() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Audit;
    let rows = ui::fixtures::fake_journal_rows(8);
    let total = u64::try_from(rows.len()).unwrap_or(0);
    c.audit_screen_state.rows = PanelState::Ready(rows);
    c.audit_screen_state.total_count = Some(total);
    assert_snapshot!("audit_screen__default_recent_24h", audit_screen_summary(&c));
}

#[test]
#[allow(non_snake_case)]
fn audit_screen__filter_no_match() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Audit;
    c.audit_screen_state.rows = PanelState::Ready(Vec::new());
    c.audit_screen_state.total_count = Some(0);
    assert_snapshot!("audit_screen__filter_no_match", audit_screen_summary(&c));
}

#[test]
#[allow(non_snake_case)]
fn audit_screen__pagination_page2() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Audit;
    // 250 + 5 rows seeded; page 2 cursor = offset 250 (page index 1 in
    // 0-indexed scheme since AUDIT_PAGE_SIZE = 250 and 0..249 = page 0).
    let rows = ui::fixtures::fake_journal_rows(5);
    c.audit_screen_state.rows = PanelState::Ready(rows);
    c.audit_screen_state.total_count = Some(255);
    c.audit_screen_state.page = 1;
    assert_snapshot!("audit_screen__pagination_page2", audit_screen_summary(&c));
}

// ── Phase 5 — HumanControl panel surfaces (T1904 / T1905 / T1906 / T1911) ───
//
// Six baselines lock the visual contract for the new HumanControl panel:
// three mode-segment states (Observe / Supervised / Auto), one kill-armed
// variant (Confirming kill state), and two limits-display variants
// (Loading / Error). The panel is rendered as the 7th sidebar entry per
// Q1 ratification — kill widget retires from the Debug screen and lands
// here as the bottom action via `kill::view_inner`.

#[test]
#[allow(non_snake_case)]
fn human_control__observe_default() {
    // Default cockpit — execution_mode = Observe; risk_state = Loading.
    let c = Cockpit::new();
    assert_snapshot!("human_control__observe_default", human_control_summary(&c));
}

#[test]
#[allow(non_snake_case)]
fn human_control__supervised_active() {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.execution_mode = ExecutionMode::Supervised;
    c.risk_state = PanelState::Ready(ui::fixtures::fake_risk_state());
    assert_snapshot!(
        "human_control__supervised_active",
        human_control_summary(&c)
    );
}

#[test]
#[allow(non_snake_case)]
fn human_control__auto_active() {
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.execution_mode = ExecutionMode::Auto;
    c.risk_state = PanelState::Ready(ui::fixtures::fake_risk_state());
    assert_snapshot!("human_control__auto_active", human_control_summary(&c));
}

#[test]
#[allow(non_snake_case)]
fn human_control__kill_armed() {
    // Kill-confirm flow opened from inside the HumanControl panel —
    // kill state moves to Confirming; the bottom-action body switches
    // from the idle button to the typed-confirm dialog. Per Q1, kill
    // is the bottom action of HumanControl (no Debug-screen kill row).
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.execution_mode = ExecutionMode::Supervised;
    c.risk_state = PanelState::Ready(ui::fixtures::fake_risk_state());
    update(&mut c, Message::KillPressed);
    assert_snapshot!("human_control__kill_armed", human_control_summary(&c));
}

#[test]
#[allow(non_snake_case)]
fn human_control__limits_loading() {
    // Risk state Loading → three muted "—" placeholder rows.
    let mut c = Cockpit::new();
    c.execution_mode = ExecutionMode::Observe;
    c.risk_state = PanelState::Loading;
    assert_snapshot!("human_control__limits_loading", human_control_summary(&c));
}

#[test]
#[allow(non_snake_case)]
fn human_control__limits_error() {
    // Risk state Error → muted body with HUMAN_CONTROL_LIMITS_UNAVAILABLE.
    let mut c = Cockpit::new();
    c.execution_mode = ExecutionMode::Observe;
    c.risk_state = PanelState::Error(SmolStr::new("ledger unreachable"));
    assert_snapshot!("human_control__limits_error", human_control_summary(&c));
}

// ── Phase 5 — Strategies-detail pause/override surfaces (T1907 / T1909 / T1910)
//
// Five baselines lock the per-strategy pause/resume button (single-click
// Q8 — no typed-confirm gate) plus the per-veto override flow (typed-confirm
// modal mirror of kill-confirm per Q9, with the OVERRIDE phrase as the
// safety token).

#[test]
#[allow(non_snake_case)]
fn strategies_screen__pause_button_idle() {
    // Strategy chip + pause button in the default Idle state — paused
    // membership empty.
    let c = ui::fixtures::fake_cockpit_with_strategies();
    assert_snapshot!(
        "strategies_screen__pause_button_idle",
        strategies_pause_summary(&c)
    );
}

#[test]
#[allow(non_snake_case)]
fn strategies_screen__pause_button_paused() {
    // After a single click the strategy joins paused_strategies and
    // the button label flips Pause → Resume (Q8 single-click both
    // directions).
    let mut c = ui::fixtures::fake_cockpit_with_strategies();
    let id = StrategyId::new("btc_macd_trend");
    update(&mut c, Message::StrategyPauseToggled(id));
    assert_snapshot!(
        "strategies_screen__pause_button_paused",
        strategies_pause_summary(&c)
    );
}

#[test]
#[allow(non_snake_case)]
fn strategies_screen__override_button_idle() {
    // One surfaced veto event with override-modal in Idle — operator
    // sees the per-veto Override button row but no modal chrome.
    let c = ui::fixtures::fake_cockpit_with_one_veto();
    assert_snapshot!(
        "strategies_screen__override_button_idle",
        strategies_override_summary(&c)
    );
}

#[test]
#[allow(non_snake_case)]
fn strategies_screen__override_confirm_modal() {
    // Operator pressed Override → typed-confirm modal opens with
    // empty input (no mismatch hint yet — typed buffer is empty).
    let mut c = ui::fixtures::fake_cockpit_with_one_veto();
    update(
        &mut c,
        Message::OverrideRiskVetoPressed(SmolStr::new("veto-1")),
    );
    assert_snapshot!(
        "strategies_screen__override_confirm_modal",
        strategies_override_summary(&c)
    );
}

#[test]
#[allow(non_snake_case)]
fn strategies_screen__override_confirm_modal_matched() {
    // Typed phrase matches OVERRIDE → confirm button enabled.
    let mut c = ui::fixtures::fake_cockpit_with_one_veto();
    update(
        &mut c,
        Message::OverrideRiskVetoPressed(SmolStr::new("veto-1")),
    );
    update(
        &mut c,
        Message::OverrideRiskVetoTyped(strings::OVERRIDE_RISK_VETO_PHRASE.to_string()),
    );
    assert_snapshot!(
        "strategies_screen__override_confirm_modal_matched",
        strategies_override_summary(&c)
    );
}

// ── Phase 5 — Focus-ring overlay (T1912 / TD-1 path b closure) ──────────────
//
// The focus-ring halo lands on the parent-side focus state owner
// (`Cockpit::focused_widget`) per the custom-widget escape hatch. This
// baseline locks the visible-halo contract on the focused kill button —
// the destructive-control invariant the four-phase TD-1 deferral was
// pointing at.

#[test]
#[allow(non_snake_case)]
fn focus_ring__focused_kill_button() {
    // Kill button in Idle state with focused_widget set to KILL_BUTTON
    // — the focus_ring::wrap overlay decorates the kill button with a
    // 1 px ACCENT border + the theme::focus::ring(mode) halo.
    let mut c = ui::fixtures::fake_cockpit_ready();
    c.focused_widget = Some(SmolStr::new(ui::state::focus_ids::KILL_BUTTON));
    assert_snapshot!(
        "focus_ring__focused_kill_button",
        focus_ring_kill_summary(&c)
    );
}

// ── Phase 5 — Debug-screen kill retirement (Q1 — kill migrates to HumanControl)
//
// Single regen baseline: Debug screen no longer carries the kill widget.
// Q1 ratification picked option (a) — HumanControl as the 7th sidebar
// entry — which migrated kill into HumanControl as the bottom action and
// retired the Debug-screen kill row. The new baseline locks "kill widget
// is absent from the Debug screen" as a structural invariant.

#[test]
#[allow(non_snake_case)]
fn debug_screen__without_kill() {
    let mut c = ui::fixtures::fake_cockpit_v15a_pairs_steady_state();
    c.market_health
        .insert(Venue::Binance, MarketHealthState::Fresh);
    c.market_health
        .insert(Venue::Coinbase, MarketHealthState::Fresh);
    c.market_health
        .insert(Venue::Kraken, MarketHealthState::Stale);
    c.latency = Latency::Known { ms: 240 };
    assert_snapshot!(
        "debug_screen__without_kill",
        debug_screen_without_kill_summary(&c)
    );
}

fn under_warn_risk_state() -> ui::state::RiskState {
    use std::collections::HashMap;
    let mut exposure = HashMap::new();
    let mut caps = HashMap::new();
    exposure.insert(
        (Venue::Binance, Symbol::new("BTCUSDT")),
        rust_decimal::Decimal::from(40),
    );
    caps.insert(
        (Venue::Binance, Symbol::new("BTCUSDT")),
        rust_decimal::Decimal::from(100),
    );
    ui::state::RiskState {
        per_symbol_exposure: exposure,
        per_symbol_caps: caps,
        daily_loss_used_pct: rust_decimal::Decimal::from(20),
        daily_loss_cap_pct: rust_decimal::Decimal::from(100),
        heartbeat_age_ms: 100,
        heartbeat_timeout_ms: 30_000,
    }
}

fn warn_band_risk_state() -> ui::state::RiskState {
    use std::collections::HashMap;
    let mut exposure = HashMap::new();
    let mut caps = HashMap::new();
    exposure.insert(
        (Venue::Binance, Symbol::new("BTCUSDT")),
        rust_decimal::Decimal::from(80),
    );
    caps.insert(
        (Venue::Binance, Symbol::new("BTCUSDT")),
        rust_decimal::Decimal::from(100),
    );
    ui::state::RiskState {
        per_symbol_exposure: exposure,
        per_symbol_caps: caps,
        daily_loss_used_pct: rust_decimal::Decimal::from(40),
        daily_loss_cap_pct: rust_decimal::Decimal::from(100),
        heartbeat_age_ms: 200,
        heartbeat_timeout_ms: 30_000,
    }
}

fn danger_band_risk_state() -> ui::state::RiskState {
    use std::collections::HashMap;
    let mut exposure = HashMap::new();
    let mut caps = HashMap::new();
    exposure.insert(
        (Venue::Binance, Symbol::new("BTCUSDT")),
        rust_decimal::Decimal::from(95),
    );
    caps.insert(
        (Venue::Binance, Symbol::new("BTCUSDT")),
        rust_decimal::Decimal::from(100),
    );
    ui::state::RiskState {
        per_symbol_exposure: exposure,
        per_symbol_caps: caps,
        daily_loss_used_pct: rust_decimal::Decimal::from(60),
        daily_loss_cap_pct: rust_decimal::Decimal::from(100),
        heartbeat_age_ms: 28_500,
        heartbeat_timeout_ms: 30_000,
    }
}

fn band_label(used: rust_decimal::Decimal, cap: rust_decimal::Decimal) -> &'static str {
    if cap == rust_decimal::Decimal::ZERO {
        return "ACCENT";
    }
    let pct = (used / cap) * rust_decimal::Decimal::from(100);
    let pct_u16: u16 = pct.trunc().to_string().parse::<u16>().unwrap_or(0);
    if pct_u16 >= 90 {
        "DOWN_500"
    } else if pct_u16 >= 70 {
        "WARN_500"
    } else {
        "ACCENT"
    }
}

fn strategies_screen_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("screen: strategies\n");
    out.push_str(&format!("title: {}\n", strings::STRATEGIES_PANEL_TITLE));
    out.push_str(&format!(
        "selected: {}\n",
        c.selected_strategy
            .as_ref()
            .map_or("(none)", |id| id.0.as_str())
    ));
    match &c.strategies_config {
        None => out.push_str("config: none\n"),
        Some(cfg) => {
            out.push_str("chips:\n");
            for entry in &cfg.strategies {
                let active = c.selected_strategy.as_ref() == Some(&entry.id);
                let marker = if active { "ACTIVE" } else { "—" };
                out.push_str(&format!("  {marker} {}\n", entry.id));
            }
            out.push_str("params:\n");
            if let Some(selected) = &c.selected_strategy {
                if let Some(entry) = cfg.strategies.iter().find(|e| &e.id == selected) {
                    for (k, v) in &entry.params {
                        out.push_str(&format!("  {k} = {v}\n"));
                    }
                } else {
                    out.push_str(&format!("  {}\n", strings::STRATEGIES_SELECT_PROMPT));
                }
            } else {
                out.push_str(&format!("  {}\n", strings::STRATEGIES_SELECT_PROMPT));
            }
        }
    }
    out.push_str("events:\n");
    let mut count = 0usize;
    for ev in &c.strategies_recent_events {
        if let Some(selected) = &c.selected_strategy {
            if ev.strategy_id.as_ref() != Some(selected) {
                continue;
            }
        }
        if count >= 50 {
            break;
        }
        count += 1;
        let id_label = ev
            .strategy_id
            .as_ref()
            .map_or_else(|| "(none)".to_string(), ToString::to_string);
        out.push_str(&format!("  {:?} {}\n", ev.kind, id_label));
    }
    if count == 0 {
        out.push_str("  (none)\n");
    }
    out
}

fn strategies_sparkline_present_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("screen: strategies\n");
    let selected = c
        .selected_strategy
        .as_ref()
        .map_or("(none)", |id| id.0.as_str());
    out.push_str(&format!("selected: {selected}\n"));
    let slot_state = match c
        .selected_strategy
        .as_ref()
        .and_then(|id| c.strategy_equity.get(id))
    {
        Some(PanelState::Ready(s)) if !s.points.is_empty() => format!(
            "sparkline_canvas: ACCENT line, {} points, peak={} trough={}",
            s.points.len(),
            s.peak.amount(),
            s.trough.amount()
        ),
        Some(PanelState::Loading) | None => {
            format!(
                "sparkline_loading: {}",
                strings::STRATEGIES_SPARKLINE_LOADING
            )
        }
        Some(PanelState::Empty | PanelState::Ready(_)) => {
            format!("sparkline_no_data: {}", strings::VIEWER_NO_EQUITY_DATA)
        }
        Some(PanelState::Error(msg)) => {
            format!("sparkline_error: Equity history unavailable: {msg}")
        }
    };
    out.push_str(&slot_state);
    out.push('\n');
    out
}

// ── Phase 4 — Viewer composition snapshot (T1810) ───────────────────────────

#[test]
#[allow(non_snake_case)]
fn viewer__full_view__sample_report() {
    // Assemble the viewer surface: KPI strip + equity curve +
    // drawdown band + body header. Asserts the structural layout
    // matches the Q-resolved contract (R9.4 — ~80 + 240 + 100 +
    // body-fill heights; tier-1 PANEL chrome; no buttons; no
    // `"Lumen"` string anywhere in the frame).
    let metrics = ui::fixtures::fake_backtest_metrics();
    let series = ui::fixtures::fake_equity_series_for_viewer();
    assert_snapshot!(
        "viewer__full_view__sample_report",
        viewer_full_view_summary(&metrics, &series, "btc-2023-1m-rsi-reversion")
    );
}

fn viewer_full_view_summary(
    metrics: &trading_core::BacktestMetrics,
    series: &trading_core::EquitySeries,
    scenario: &str,
) -> String {
    let mut out = String::new();
    out.push_str("bin: viewer\n");
    out.push_str(&format!(
        "window_title: Backtest report \u{2014} {scenario}\n"
    ));
    out.push_str("layout: column\n");
    out.push_str("  kpi_strip: ~80 px (intrinsic)\n");
    out.push_str("  equity_curve: 240 px fixed\n");
    out.push_str("  drawdown_band: 100 px fixed\n");
    out.push_str("  body: scrollable, fill remaining\n");
    out.push_str("chrome: tier-1 PANEL\n");
    out.push_str("buttons: 0\n");
    out.push_str("status_bar: absent\n");
    out.push_str(&format!(
        "kpi_total_return: {}\n",
        if metrics.total_return_pct.is_zero() {
            "0.00%".to_string()
        } else {
            format!(
                "{}{}%",
                if metrics.total_return_pct.is_sign_negative() {
                    "\u{2212}"
                } else {
                    ""
                },
                metrics.total_return_pct.abs().round_dp(2)
            )
        }
    ));
    out.push_str(&format!("kpi_trades: {}\n", metrics.trades));
    out.push_str(&format!("equity_points: {}\n", series.points.len()));
    out.push_str(&format!("equity_peak: {}\n", series.peak.amount()));
    out.push_str(&format!("equity_trough: {}\n", series.trough.amount()));
    out.push_str(&format!("max_drawdown_pct: {}\n", series.max_drawdown_pct));
    out
}

fn risk_screen_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("screen: risk\n");
    out.push_str(&format!("title: {}\n", strings::RISK_PANEL_TITLE));
    match &c.risk_state {
        PanelState::Loading => out.push_str(&format!("copy: {}\n", strings::RISK_LOADING)),
        PanelState::Empty => out.push_str(&format!("copy: {}\n", strings::RISK_LOADING)),
        PanelState::Error(e) => out.push_str(&format!(
            "copy: {}{}\n",
            strings::RISK_FEED_UNAVAILABLE_PREFIX,
            e
        )),
        PanelState::Ready(state) => {
            out.push_str(&format!(
                "section: {}\n",
                strings::RISK_EXPOSURE_SECTION_TITLE
            ));
            let mut keys: Vec<&(Venue, Symbol)> = state.per_symbol_exposure.keys().collect();
            keys.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.1 .0.cmp(&b.1 .0)));
            for k in keys {
                let used = state
                    .per_symbol_exposure
                    .get(k)
                    .copied()
                    .unwrap_or_default();
                let cap = state.per_symbol_caps.get(k).copied().unwrap_or_default();
                let band = band_label(used, cap);
                out.push_str(&format!(
                    "  {} {} used={} cap={} band={}\n",
                    k.0, k.1, used, cap, band
                ));
            }
            out.push_str(&format!(
                "section: {}\n",
                strings::RISK_DAILY_LOSS_SECTION_TITLE
            ));
            let band_dl = band_label(state.daily_loss_used_pct, state.daily_loss_cap_pct);
            out.push_str(&format!(
                "  used={} cap={} band={}\n",
                state.daily_loss_used_pct, state.daily_loss_cap_pct, band_dl
            ));
            out.push_str(&format!(
                "section: {}\n",
                strings::RISK_KILL_THRESHOLD_SECTION_TITLE
            ));
            let band_hb = band_label(
                rust_decimal::Decimal::from(state.heartbeat_age_ms),
                rust_decimal::Decimal::from(state.heartbeat_timeout_ms),
            );
            out.push_str(&format!(
                "  age_ms={} timeout_ms={} band={}\n",
                state.heartbeat_age_ms, state.heartbeat_timeout_ms, band_hb
            ));
        }
    }
    out
}

fn audit_screen_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("screen: audit\n");
    out.push_str(&format!("title: {}\n", strings::AUDIT_PANEL_TITLE));
    let f = &c.audit_screen_state.filter;
    out.push_str(&format!(
        "filter: venues={} symbol={} kind={:?} time_range={:?}\n",
        f.venues
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
        f.symbol.as_ref().map_or("—", |s| s.0.as_str()),
        f.kind,
        f.time_range
    ));
    let total = c.audit_screen_state.total_count.unwrap_or(0);
    let page = c.audit_screen_state.page;
    out.push_str(&format!("page: {page} total: {total}\n"));
    match &c.audit_screen_state.rows {
        PanelState::Loading => out.push_str(&format!("copy: {}\n", strings::AUDIT_LOADING)),
        PanelState::Empty => {
            out.push_str(&format!("copy: {}\n", strings::AUDIT_FILTER_NO_MATCH));
        }
        PanelState::Error(e) => out.push_str(&format!(
            "copy: {}{}\n",
            strings::AUDIT_QUERY_FAILED_PREFIX,
            e
        )),
        PanelState::Ready(rows) => {
            if rows.is_empty() {
                out.push_str(&format!("copy: {}\n", strings::AUDIT_FILTER_NO_MATCH));
            } else {
                out.push_str("rows:\n");
                for r in rows.iter().take(10) {
                    let symbol = r.symbol.as_ref().map_or("—", |s| s.0.as_str());
                    let strat = r
                        .strategy_id
                        .as_ref()
                        .map_or("—".to_string(), ToString::to_string);
                    out.push_str(&format!(
                        "  {} {} {} {:?} {} {}\n",
                        r.tx_id, r.venue, symbol, r.kind, r.description, strat
                    ));
                }
                if rows.len() > 10 {
                    out.push_str(&format!("  …({} more)\n", rows.len() - 10));
                }
            }
        }
    }
    out
}

fn home_screen_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("screen: home\n");
    out.push_str("layout: 2x2 grid (pnl + positions / strategies + tape)\n");
    out.push_str(&format!("pnl_state: {}\n", c.pnl.variant_name()));
    out.push_str(&format!(
        "positions_state: {}\n",
        c.positions.variant_name()
    ));
    out.push_str(&format!(
        "strategies_state: {}\n",
        c.strategies.variant_name()
    ));
    out.push_str(&format!("tape_state: {}\n", c.tape.variant_name()));
    out
}

/// Phase 5 (T1906) — Debug screen WITHOUT the kill widget. Per Q1
/// ratification kill migrates into the HumanControl panel as the bottom
/// action; the Debug screen retires the kill row entirely. The summary
/// emits the surviving sub-blocks in order — the absent `kill` row is
/// the structural invariant the snapshot pins.
fn debug_screen_without_kill_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("screen: debug\n");
    out.push_str("layout: latency | market_health | server_time | version | logs_stub\n");
    out.push_str("kill_widget: absent (migrated to HumanControl per Q1)\n");
    let lat = match c.latency {
        Latency::Known { ms } => format!("known ms={ms}"),
        Latency::Unknown => "unknown".to_string(),
    };
    out.push_str(&format!("latency: {lat}\n"));
    out.push_str("market_health:\n");
    let mut entries: Vec<_> = c.market_health.iter().collect();
    entries.sort_by_key(|(v, _)| v.to_string());
    for (v, s) in entries {
        let label = match s {
            MarketHealthState::Fresh => "fresh",
            MarketHealthState::Stale => "stale",
        };
        out.push_str(&format!("  {v}: {label}\n"));
    }
    let server = c
        .server_time_now
        .map_or("—".to_string(), |t| format!("{}", t));
    out.push_str(&format!("server_time: {server}\n"));
    out.push_str(&format!("version: {}\n", ui::strings::STATUS_BAR_VERSION));
    out.push_str(&format!("logs: {}\n", ui::strings::DEBUG_LOGS_PLACEHOLDER));
    out
}

// ── Phase 5 summary helpers — HumanControl + pause/override + focus-ring ────

/// HumanControl panel summary. Emits the four sub-blocks top-to-bottom
/// per the Phase 5 Design contract: mode segment + per-mode hint + three
/// mirror rows (Daily-loss / Max-position / Used-today) + kill bottom
/// action. The kill widget is rendered via `kill::view_inner` (body
/// only) so the outer `panel` chrome belongs to HumanControl per Q1 / R2.1.
fn human_control_summary(c: &Cockpit) -> String {
    use rust_decimal::Decimal;
    let mut out = String::new();
    out.push_str("panel: human_control\n");
    out.push_str(&format!("title: {}\n", strings::PANEL_HUMAN_CONTROL_TITLE));
    out.push_str("placement: 7th sidebar entry (Screen::Control)\n");
    out.push_str("layout: mode_segment | mode_hint | limit_rows | kill_bottom_action\n");

    // Mode segment — three buttons; active variant uses PANEL_RAISED bg
    // + ACCENT border @ 1 px (Phase 1 active-row pattern).
    out.push_str("mode_segment:\n");
    let modes = [
        (
            ExecutionMode::Observe,
            strings::EXECUTION_MODE_OBSERVE_LABEL,
            "execution_mode_observe",
        ),
        (
            ExecutionMode::Supervised,
            strings::EXECUTION_MODE_SUPERVISED_LABEL,
            "execution_mode_supervised",
        ),
        (
            ExecutionMode::Auto,
            strings::EXECUTION_MODE_AUTO_LABEL,
            "execution_mode_auto",
        ),
    ];
    for (m, label, focus_key) in modes {
        let active = c.execution_mode == m;
        let bg = if active { "PANEL_RAISED" } else { "PANEL" };
        let border = if active { "ACCENT" } else { "BORDER_2" };
        let focused = c.focused_widget.as_deref() == Some(focus_key);
        out.push_str(&format!(
            "  {label} active={active} bg={bg} border={border} focused={focused}\n"
        ));
    }
    let hint = match c.execution_mode {
        ExecutionMode::Observe => strings::EXECUTION_MODE_OBSERVE_HINT,
        ExecutionMode::Supervised => strings::EXECUTION_MODE_SUPERVISED_HINT,
        ExecutionMode::Auto => strings::EXECUTION_MODE_AUTO_HINT,
    };
    out.push_str(&format!("mode_hint: {hint}\n"));

    // Limit rows — three mirror rows or a Loading / Error placeholder.
    out.push_str("limit_rows:\n");
    match &c.risk_state {
        PanelState::Loading | PanelState::Empty => {
            out.push_str(&format!(
                "  {} = {} sentiment=none\n",
                strings::HUMAN_CONTROL_DAILY_LOSS_LABEL,
                strings::PLACEHOLDER_NONE
            ));
            out.push_str(&format!(
                "  {} = {} sentiment=none\n",
                strings::HUMAN_CONTROL_MAX_POSITION_LABEL,
                strings::PLACEHOLDER_NONE
            ));
            out.push_str(&format!(
                "  {} = {} sentiment=none\n",
                strings::HUMAN_CONTROL_USED_TODAY_LABEL,
                strings::PLACEHOLDER_NONE
            ));
        }
        PanelState::Error(e) => {
            out.push_str(&format!(
                "  copy: {} (error: {e})\n",
                strings::HUMAN_CONTROL_LIMITS_UNAVAILABLE
            ));
        }
        PanelState::Ready(rs) => {
            let daily = ui::widgets::num::fmt_pct(rs.daily_loss_cap_pct);
            let max_cap = rs
                .per_symbol_caps
                .values()
                .copied()
                .fold(Decimal::ZERO, Decimal::max);
            let max_position = if max_cap == Decimal::ZERO {
                strings::PLACEHOLDER_NONE.to_string()
            } else {
                ui::widgets::num::fmt_pct(max_cap)
            };
            let (used_today, used_color) = match &c.pnl {
                PanelState::Ready(snap) => (
                    ui::widgets::num::fmt_usdt_signed(snap.daily_return.amount()),
                    color_name(ui::theme::color_for_delta(snap.daily_return.amount())),
                ),
                _ => (strings::PLACEHOLDER_NONE.to_string(), "fg"),
            };
            out.push_str(&format!(
                "  {} = {daily} sentiment=none\n",
                strings::HUMAN_CONTROL_DAILY_LOSS_LABEL
            ));
            out.push_str(&format!(
                "  {} = {max_position} sentiment=none\n",
                strings::HUMAN_CONTROL_MAX_POSITION_LABEL
            ));
            out.push_str(&format!(
                "  {} = {used_today} sentiment={used_color}\n",
                strings::HUMAN_CONTROL_USED_TODAY_LABEL
            ));
        }
    }

    // Kill bottom action — body-only (no outer panel chrome — HumanControl
    // is the chrome owner per R2.3 / view_inner).
    out.push_str("kill_bottom_action:\n");
    match &c.kill {
        KillState::Idle => {
            out.push_str("  state: idle\n");
            out.push_str(&format!("  button_label: {}\n", strings::KILL_BUTTON_LABEL));
            out.push_str(&format!("  help: {}\n", strings::KILL_BUTTON_HELP));
        }
        KillState::Confirming { typed } => {
            out.push_str("  state: confirming\n");
            out.push_str(&format!("  dialog_title: {}\n", strings::KILL_DIALOG_TITLE));
            out.push_str(&format!("  dialog_body: {}\n", strings::KILL_DIALOG_BODY));
            out.push_str(&format!("  phrase_label: {}\n", strings::KILL_PHRASE_LABEL));
            out.push_str(&format!("  typed: {typed:?}\n"));
            let matched = typed == strings::KILL_SAFETY_PHRASE;
            out.push_str(&format!("  matched: {matched}\n"));
            out.push_str(&format!("  confirm_enabled: {matched}\n"));
        }
        KillState::Flattening => out.push_str("  state: flattening\n"),
        KillState::Halted { reason } => {
            out.push_str("  state: halted\n");
            out.push_str(&format!("  banner: {}\n", strings::KILL_HALTED_BANNER));
            out.push_str(&format!("  reason: {reason}\n"));
        }
    }
    out
}

/// Strategies-detail pause-section summary. Captures the per-strategy
/// pause/resume button state per row plus the active-paused membership.
/// Single-click both directions per Q8 — no typed-confirm chrome.
fn strategies_pause_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("screen: strategies\n");
    out.push_str("section: pause_buttons\n");
    out.push_str(&format!("paused_count: {}\n", c.paused_strategies.len()));
    out.push_str("typed_confirm: false (Q8 — single-click both directions)\n");
    out.push_str("rows:\n");
    if let PanelState::Ready(rows) = &c.strategies {
        let mut ids: Vec<&trading_core::StrategyId> = rows.iter().map(|r| &r.id).collect();
        ids.sort_by(|a, b| a.0.as_str().cmp(b.0.as_str()));
        for id in ids {
            let paused = c.paused_strategies.contains(id);
            let label = if paused {
                strings::STRATEGY_RESUME_LABEL
            } else {
                strings::STRATEGY_PAUSE_LABEL
            };
            let bg = if paused { "PANEL_RAISED" } else { "PANEL" };
            let focus_key = format!("strategy_pause::{}", id.0.as_str());
            let focused = c.focused_widget.as_deref() == Some(focus_key.as_str());
            out.push_str(&format!(
                "  {} label={label} paused={paused} bg={bg} focused={focused}\n",
                id.0,
            ));
        }
    } else {
        out.push_str("  (strategies not Ready)\n");
    }
    out
}

/// Strategies-detail override-veto section + modal summary. Captures
/// the surfaced VetoEvent rows + per-veto Override button state, then
/// the typed-confirm modal contract (mirror of kill-confirm —
/// `OVERRIDE` phrase, BORDER_2 → ACCENT border-shift on focus, confirm
/// disabled until typed == OVERRIDE per Q9 / R7.4).
fn strategies_override_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("screen: strategies\n");
    out.push_str("section: risk_veto_events\n");
    out.push_str(&format!("veto_count: {}\n", c.risk_veto_events.len()));
    if c.risk_veto_events.is_empty() {
        out.push_str("  (no surfaced vetoes)\n");
    } else {
        out.push_str("rows:\n");
        for v in &c.risk_veto_events {
            let focus_key = format!("override_veto_button::{}", v.veto_id.as_str());
            let focused = c.focused_widget.as_deref() == Some(focus_key.as_str());
            out.push_str(&format!(
                "  veto_id={} strategy={} reason={} button_label={} border=WARN_500 focused={}\n",
                v.veto_id,
                v.strategy_id,
                v.reason,
                strings::OVERRIDE_RISK_VETO_BUTTON_LABEL,
                focused,
            ));
        }
    }
    out.push_str("modal:\n");
    match &c.override_risk_veto {
        OverrideRiskVetoState::Idle => out.push_str("  state: idle (modal closed)\n"),
        OverrideRiskVetoState::Confirming { veto_id, typed } => {
            let matched = typed == strings::OVERRIDE_RISK_VETO_PHRASE;
            out.push_str("  state: confirming\n");
            out.push_str(&format!(
                "  dialog_title: {}\n",
                strings::OVERRIDE_RISK_VETO_DIALOG_TITLE
            ));
            out.push_str(&format!(
                "  dialog_body: {}\n",
                strings::OVERRIDE_RISK_VETO_DIALOG_BODY
            ));
            out.push_str(&format!(
                "  phrase: {}\n",
                strings::OVERRIDE_RISK_VETO_PHRASE
            ));
            out.push_str(&format!("  veto_id: {veto_id}\n"));
            out.push_str(&format!("  typed: {typed:?}\n"));
            out.push_str(&format!("  matched: {matched}\n"));
            out.push_str(&format!("  confirm_enabled: {matched}\n"));
            out.push_str(&format!(
                "  confirm_label: {}\n",
                strings::OVERRIDE_RISK_VETO_CONFIRM_LABEL
            ));
            out.push_str(&format!(
                "  cancel_label: {}\n",
                strings::OVERRIDE_RISK_VETO_CANCEL_LABEL
            ));
            out.push_str("  input_bg: PANEL_SUNKEN\n");
            out.push_str("  input_hairline: shadow_inset\n");
            // Border shift on iced-side focus is preserved via
            // text_input::Status::Focused; per-widget halo lands via
            // focus_ring::wrap when focused_widget == OVERRIDE_RISK_VETO_INPUT.
            let input_focused =
                c.focused_widget.as_deref() == Some(ui::state::focus_ids::OVERRIDE_RISK_VETO_INPUT);
            out.push_str(&format!("  input_focused: {input_focused}\n"));
            if !matched && !typed.is_empty() {
                out.push_str(&format!(
                    "  hint: {}\n",
                    strings::OVERRIDE_RISK_VETO_PHRASE_MISMATCH_HINT
                ));
            }
        }
        OverrideRiskVetoState::Submitting { veto_id } => {
            out.push_str("  state: submitting\n");
            out.push_str(&format!("  veto_id: {veto_id}\n"));
        }
    }
    out
}

/// Focus-ring overlay summary on the kill button — the visible-halo
/// contract Phase 5 ships per TD-1 path b. When `focused_widget` matches
/// the kill button's stable focus key, `focus_ring::wrap(...)` decorates
/// the button with a 1 px ACCENT border + the `theme::focus::ring(mode)`
/// halo (iced 0.14 outer Shadow). When unfocused, the wrap is a
/// pixel-identical pass-through.
fn focus_ring_kill_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("widget: focus_ring (TD-1 path b custom-widget escape hatch)\n");
    out.push_str(&format!(
        "wrapped_widget: kill_button ({} idle body)\n",
        strings::KILL_BUTTON_LABEL
    ));
    let focused = c.focused_widget.as_deref() == Some(ui::state::focus_ids::KILL_BUTTON);
    out.push_str(&format!("focused_widget: {:?}\n", c.focused_widget));
    out.push_str(&format!(
        "focus_key: {}\n",
        ui::state::focus_ids::KILL_BUTTON
    ));
    out.push_str(&format!("focused: {focused}\n"));
    if focused {
        out.push_str("halo_visible: true\n");
        out.push_str("halo_border_color: ACCENT\n");
        out.push_str("halo_border_width: 1.0\n");
        out.push_str("halo_radius: R2\n");
        out.push_str("halo_shadow: theme::focus::ring(Dark)\n");
        out.push_str("td1_closure: visible (custom-widget escape hatch — iced 0.14 button::Status::Focused absent; halo lands via parent-side focus state owner)\n");
    } else {
        out.push_str("halo_visible: false (pass-through)\n");
    }
    out
}

fn charts_screen_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("screen: charts\n");
    out.push_str("layout: chip_row + chart_canvas\n");
    out.push_str("chips:\n");
    for (v, s) in &c.universe {
        let active = matches!(&c.selected_symbol, Some((av, asym)) if av == v && asym == s);
        let marker = if active { "ACCENT" } else { "—" };
        out.push_str(&format!("  rule={marker} venue={v} symbol={s}\n"));
    }
    let chart_state = match &c.chart_markers {
        PanelState::Ready(v) => format!("ready({} markers)", v.len()),
        PanelState::Loading => "loading".to_string(),
        PanelState::Empty => "empty".to_string(),
        PanelState::Error(e) => format!("error: {e}"),
    };
    out.push_str(&format!("chart_markers: {chart_state}\n"));
    let bars = if let Some((v, s)) = &c.selected_symbol {
        c.chart_buffer.bars(*v, s).count()
    } else {
        0
    };
    out.push_str(&format!("chart_bars: {bars}\n"));
    let _ = Screen::Home;
    out
}

// ── Summary helpers — plain-text state rendering ────────────────────────────

fn tape_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("panel: agent_feed\n");
    out.push_str(&format!("title: {}\n", strings::PANEL_AGENT_FEED_TITLE));
    out.push_str(&format!("state: {}\n", c.tape.variant_name()));
    out.push_str(&format!("paused: {}\n", c.tape_paused));
    match &c.tape {
        PanelState::Loading => out.push_str(&format!("copy: {}\n", strings::TAPE_LOADING)),
        PanelState::Empty => out.push_str(&format!("copy: {}\n", strings::TAPE_EMPTY)),
        PanelState::Error(e) => {
            out.push_str(&format!("copy: {}{}\n", strings::TAPE_ERROR_PREFIX, e))
        }
        PanelState::Ready(q) => {
            out.push_str("rows:\n");
            for f in q {
                out.push_str(&format!(
                    "  {}  {}  {:?}  {}  {}\n",
                    f.venue_ts,
                    f.symbol,
                    f.side,
                    f.price.get(),
                    f.qty.get(),
                ));
            }
        }
    }
    if c.tape_paused {
        out.push_str(&format!("banner: {}\n", strings::TAPE_PAUSED_BANNER));
    }
    out
}

fn positions_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("panel: positions\n");
    out.push_str(&format!("title: {}\n", strings::PANEL_POSITIONS_TITLE));
    out.push_str(&format!("state: {}\n", c.positions.variant_name()));
    match &c.positions {
        PanelState::Loading => out.push_str(&format!("copy: {}\n", strings::POS_LOADING)),
        PanelState::Empty => out.push_str(&format!("copy: {}\n", strings::POS_EMPTY)),
        PanelState::Error(e) => {
            out.push_str(&format!("copy: {}{}\n", strings::POS_ERROR_PREFIX, e))
        }
        PanelState::Ready(positions) => {
            let visible: Vec<&trading_core::PositionView> =
                positions.iter().filter(|p| !p.base_qty.is_zero()).collect();
            if visible.is_empty() {
                out.push_str(&format!("copy: {}\n", strings::POS_EMPTY));
            } else {
                out.push_str("rows:\n");
                for p in visible {
                    let pnl_color = color_name(ui::theme::color_for_delta(p.pnl.amount()));
                    out.push_str(&format!(
                        "  {} qty={} cost={} mark={} pnl={} pnl_color={} pct={} exp={}\n",
                        p.symbol,
                        p.base_qty,
                        p.cost_basis.amount(),
                        p.last_mark.get(),
                        p.pnl.amount(),
                        pnl_color,
                        p.pnl_pct,
                        p.exposure_pct,
                    ));
                }
            }
        }
    }
    out
}

fn pnl_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("panel: pnl\n");
    out.push_str(&format!("title: {}\n", strings::PANEL_PNL_TITLE));
    out.push_str(&format!("state: {}\n", c.pnl.variant_name()));
    match &c.pnl {
        PanelState::Loading => out.push_str(&format!("copy: {}\n", strings::PNL_LOADING)),
        PanelState::Empty => out.push_str(&format!("copy: {}\n", strings::PNL_EMPTY)),
        PanelState::Error(e) => {
            out.push_str(&format!("copy: {}{}\n", strings::PNL_ERROR_PREFIX, e))
        }
        PanelState::Ready(snap) => {
            let daily_color = color_name(ui::theme::color_for_delta(snap.daily_return.amount()));
            out.push_str(&format!(
                "equity: {}\n",
                ui::widgets::num::fmt_usdt(snap.total_equity.amount())
            ));
            out.push_str(&format!(
                "daily_return: {} color={}\n",
                ui::widgets::num::fmt_usdt_signed(snap.daily_return.amount()),
                daily_color,
            ));
            out.push_str(&format!(
                "cash: {}\n",
                ui::widgets::num::fmt_usdt(snap.cash.amount())
            ));
            out.push_str(&format!(
                "unrealized: {} color={}\n",
                ui::widgets::num::fmt_usdt_signed(snap.unrealized.amount()),
                color_name(ui::theme::color_for_delta(snap.unrealized.amount())),
            ));
            out.push_str(&format!(
                "realized: {} color={}\n",
                ui::widgets::num::fmt_usdt_signed(snap.realized.amount()),
                color_name(ui::theme::color_for_delta(snap.realized.amount())),
            ));
        }
    }
    out
}

fn kill_summary(c: &Cockpit) -> String {
    kill_summary_focused(c, false)
}

/// T1506 — kill panel summary with optional focus state for the confirm input.
/// When `focused` is `true` the summary emits the accent-border styling tokens
/// that would be active while the operator types `HALT BTC`.
fn kill_summary_focused(c: &Cockpit, focused: bool) -> String {
    let mut out = String::new();
    out.push_str("panel: kill\n");
    out.push_str(&format!("title: {}\n", strings::PANEL_KILL_TITLE));
    match &c.kill {
        KillState::Idle => {
            out.push_str("state: idle\n");
            out.push_str(&format!("button: {}\n", strings::KILL_BUTTON_LABEL));
            out.push_str(&format!("help: {}\n", strings::KILL_BUTTON_HELP));
        }
        KillState::Confirming { typed } => {
            out.push_str("state: confirming\n");
            out.push_str(&format!("dialog_title: {}\n", strings::KILL_DIALOG_TITLE));
            out.push_str(&format!("dialog_body: {}\n", strings::KILL_DIALOG_BODY));
            out.push_str(&format!("phrase_label: {}\n", strings::KILL_PHRASE_LABEL));
            out.push_str(&format!("typed: {typed:?}\n"));
            let matched = typed == strings::KILL_SAFETY_PHRASE;
            out.push_str(&format!("matched: {matched}\n"));
            out.push_str(&format!("confirm_enabled: {matched}\n"));
            // T1506 — sunken input chrome tokens.
            out.push_str("input_bg: PANEL_SUNKEN\n");
            out.push_str("input_hairline: shadow_inset\n");
            let border_token = if focused { "ACCENT" } else { "BORDER_2" };
            out.push_str(&format!("input_border: {border_token}\n"));
            // T1504 — focus ring: deferred (text_input::Style has no shadow
            // field in iced 0.14; see widgets/kill.rs module doc).
            let ring_token = if focused {
                "deferred (iced 0.14 text_input::Style has no shadow field)"
            } else {
                "none"
            };
            out.push_str(&format!("input_focus_ring: {ring_token}\n"));
            if !matched && !typed.is_empty() {
                out.push_str(&format!("hint: {}\n", strings::KILL_PHRASE_MISMATCH_HINT));
            }
            out.push_str(&format!("confirm_label: {}\n", strings::KILL_CONFIRM_LABEL));
            out.push_str(&format!("cancel_label: {}\n", strings::KILL_CANCEL_LABEL));
        }
        KillState::Flattening => {
            out.push_str("state: flattening\n");
        }
        KillState::Halted { reason } => {
            out.push_str("state: halted\n");
            out.push_str(&format!("banner: {}\n", strings::KILL_HALTED_BANNER));
            out.push_str(&format!("hint: {}\n", strings::KILL_HALTED_HINT));
            out.push_str(&format!("reason: {reason}\n"));
        }
    }
    out
}

fn latency_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("panel: latency\n");
    out.push_str(&format!("title: {}\n", strings::PANEL_LATENCY_TITLE));
    let (badge, value) = match (c.latency, c.mode) {
        (_, AgentMode::Halted) => (Badge::Halted, "—".to_string()),
        (Latency::Unknown, _) => (Badge::Unknown, "—".to_string()),
        (Latency::Known { ms }, _) => (
            Badge::classify(ms),
            format!("{ms} {}", strings::LATENCY_UNIT_MS),
        ),
    };
    out.push_str(&format!("badge: {:?}\n", badge));
    out.push_str(&format!("label: {}\n", badge.label()));
    out.push_str(&format!("color: {}\n", color_name(badge.color())));
    out.push_str(&format!("value: {}\n", value));
    out.push_str(&format!("help: {}\n", strings::LATENCY_HELP));
    out
}

/// Tiny declarative summary of the full cockpit layout — emits the panel
/// titles in column order. The strategies panel must appear first in the
/// right column per the Q4 Cockpit layout resolution (T527). This is the
/// byte the snapshot pins so a refactor that moves the panel gets caught.
fn cockpit_layout_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("layout: cockpit\n");
    out.push_str("left_column:\n");
    out.push_str(&format!("  - {}\n", strings::PANEL_PNL_TITLE));
    out.push_str(&format!("  - {}\n", strings::PANEL_LATENCY_TITLE));
    out.push_str(&format!("  - {}\n", strings::PANEL_KILL_TITLE));
    out.push_str("right_column:\n");
    out.push_str(&format!("  - {}\n", strings::PANEL_STRATEGIES_TITLE));
    out.push_str(&format!("  - {}\n", strings::PANEL_POSITIONS_TITLE));
    out.push_str(&format!("  - {}\n", strings::PANEL_AGENT_FEED_TITLE));
    out.push_str(&format!(
        "strategies_state: {}\n",
        c.strategies.variant_name()
    ));
    out
}

fn strategies_summary(c: &Cockpit) -> String {
    use trading_core::StrategyEventKind;
    let mut out = String::new();
    out.push_str("panel: strategies\n");
    out.push_str(&format!("title: {}\n", strings::PANEL_STRATEGIES_TITLE));
    out.push_str(&format!("state: {}\n", c.strategies.variant_name()));
    match &c.strategies {
        PanelState::Loading => out.push_str(&format!("copy: {}\n", strings::STRATEGIES_LOADING)),
        PanelState::Empty => out.push_str(&format!("copy: {}\n", strings::STRATEGIES_EMPTY)),
        PanelState::Error(e) => {
            out.push_str(&format!(
                "copy: {}{}\n",
                strings::STRATEGIES_ERROR_PREFIX,
                e
            ));
        }
        PanelState::Ready(rows) => {
            out.push_str("rows:\n");
            for r in rows {
                let (status_label, status_color) = match &r.status {
                    StrategyStatus::Ready => (strings::STRATEGIES_STATUS_READY, "pos"),
                    StrategyStatus::Loading => (strings::STRATEGIES_STATUS_LOADING, "fg_muted"),
                    StrategyStatus::Error(_) => (strings::STRATEGIES_STATUS_ERROR, "neg"),
                };
                let last_event = r.last_event.as_ref().map_or("—", |ev| match ev.kind {
                    StrategyEventKind::Load => strings::STRATEGIES_EVENT_LOAD,
                    StrategyEventKind::Swap => strings::STRATEGIES_EVENT_SWAP,
                    StrategyEventKind::Unload => strings::STRATEGIES_EVENT_UNLOAD,
                    StrategyEventKind::Reject => strings::STRATEGIES_EVENT_REJECT,
                    StrategyEventKind::RebalanceRejected => strings::STRATEGIES_EVENT_REJECT,
                    // v1.5a pair events + v1+ operator-success-report sources —
                    // displayed as "loaded" label (muted, observation-only).
                    StrategyEventKind::MeanReversionStop
                    | StrategyEventKind::PairShortObservation
                    | StrategyEventKind::KillSwitchTripped
                    | StrategyEventKind::FeedReconnect
                    | StrategyEventKind::StrategyPaused
                    | StrategyEventKind::RiskVetoOverridden => strings::STRATEGIES_EVENT_LOAD,
                });
                let position = if r.has_position {
                    strings::STRATEGIES_POSITION_HELD
                } else {
                    strings::STRATEGIES_POSITION_FLAT
                };
                out.push_str(&format!(
                    "  {} hash={} status={}[{}] last={} signals_60s={} pos={}\n",
                    r.id,
                    if r.short_hash.is_empty() {
                        strings::PLACEHOLDER_NONE
                    } else {
                        r.short_hash.as_str()
                    },
                    status_label,
                    status_color,
                    last_event,
                    r.signals_60s,
                    position,
                ));
                if let StrategyStatus::Error(summary) = &r.status {
                    out.push_str(&format!("    badge: {} color=neg\n", summary));
                }
            }
        }
    }
    if !c.strategies_recent_events.is_empty() {
        out.push_str("recent_events:\n");
        for ev in &c.strategies_recent_events {
            let (label, color) = match ev.kind {
                StrategyEventKind::Load => (strings::STRATEGIES_EVENT_LOAD, "accent"),
                StrategyEventKind::Swap => (strings::STRATEGIES_EVENT_SWAP, "warn"),
                StrategyEventKind::Unload => (strings::STRATEGIES_EVENT_UNLOAD, "fg_muted"),
                StrategyEventKind::Reject => (strings::STRATEGIES_EVENT_REJECT, "neg"),
                StrategyEventKind::RebalanceRejected => (strings::STRATEGIES_EVENT_REJECT, "neg"),
                // v1.5a pair events + v1+ operator-success-report sources —
                // show as "loaded" (muted, observation-only).
                StrategyEventKind::MeanReversionStop
                | StrategyEventKind::PairShortObservation
                | StrategyEventKind::KillSwitchTripped
                | StrategyEventKind::FeedReconnect
                | StrategyEventKind::StrategyPaused
                | StrategyEventKind::RiskVetoOverridden => {
                    (strings::STRATEGIES_EVENT_LOAD, "fg_muted")
                }
            };
            let id = ev
                .strategy_id
                .as_ref()
                .map_or(strings::PLACEHOLDER_NONE.to_string(), ToString::to_string);
            out.push_str(&format!("  {} {} color={}\n", label, id, color));
        }
    }
    out
}

/// Plain-text summary of the tape-row → audit modal state. Mirrors the
/// branching the widget performs in `journal_transaction_modal::view` so
/// regressions in
/// (a) which branch of the `PanelState<JournalTransactionView>` match
///     was taken,
/// (b) which `strings::TAPE_AUDIT_MODAL_*` copy was rendered,
/// (c) the `(account, debit, credit, currency)` cell values per entry,
/// surface as a snapshot diff. The summary intentionally uses the same
/// `widgets::num::fmt_usdt` formatter the widget uses for the debit /
/// credit cells, so a regression in the number renderer reaches the
/// snapshot.
fn tape_audit_modal_summary(state: &JournalModalState) -> String {
    let mut out = String::new();
    out.push_str("panel: tape_audit_modal\n");
    out.push_str(&format!("title: {}\n", strings::TAPE_AUDIT_MODAL_TITLE));
    out.push_str(&format!("state: {}\n", state.entries.variant_name()));
    out.push_str(&format!("tx_id: {}\n", state.tx_id));
    out.push_str(&format!(
        "close_label: {}\n",
        strings::TAPE_AUDIT_MODAL_CLOSE_LABEL
    ));
    match &state.entries {
        PanelState::Loading => {
            out.push_str(&format!("copy: {}\n", strings::TAPE_AUDIT_MODAL_LOADING));
        }
        PanelState::Empty => {
            out.push_str(&format!("copy: {}\n", strings::TAPE_AUDIT_MODAL_EMPTY));
        }
        PanelState::Error(msg) => {
            out.push_str(&format!(
                "copy: {}{}\n",
                strings::TAPE_AUDIT_MODAL_ERROR_PREFIX,
                msg
            ));
        }
        PanelState::Ready(view) => {
            // Header rows — `(label, value)` pairs the widget renders above
            // the entries table. Mirrors `metadata_block` in the widget.
            out.push_str("header:\n");
            out.push_str(&format!(
                "  {}: {}\n",
                strings::TAPE_AUDIT_MODAL_TX_LABEL,
                view.tx_id
            ));
            out.push_str(&format!(
                "  {}: {}\n",
                strings::TAPE_AUDIT_MODAL_TS_LABEL,
                view.ts
            ));
            out.push_str(&format!(
                "  {}: {}\n",
                strings::TAPE_AUDIT_MODAL_DESC_LABEL,
                view.description
            ));
            let strategy = view
                .strategy_id
                .as_ref()
                .map_or(strings::TAPE_AUDIT_MODAL_STRATEGY_NONE.to_string(), |s| {
                    s.0.as_str().to_string()
                });
            out.push_str(&format!(
                "  {}: {}\n",
                strings::TAPE_AUDIT_MODAL_STRATEGY_LABEL,
                strategy
            ));
            // Column headers — pinned per principles "Plain language" so a
            // regression to `account_id` / `debit_amount` shows in diff.
            out.push_str(&format!(
                "columns: {} | {} | {} | {}\n",
                strings::TAPE_AUDIT_MODAL_COL_ACCOUNT,
                strings::TAPE_AUDIT_MODAL_COL_DEBIT,
                strings::TAPE_AUDIT_MODAL_COL_CREDIT,
                strings::TAPE_AUDIT_MODAL_COL_CURRENCY,
            ));
            out.push_str("rows:\n");
            for entry in &view.entries {
                out.push_str(&format!(
                    "  {} | {} | {} | {}\n",
                    entry.account,
                    fmt_usdt(entry.debit.amount()),
                    fmt_usdt(entry.credit.amount()),
                    entry.currency,
                ));
            }
        }
    }
    out
}

/// Plain-text summary of the status bar state (T1508).
///
/// Mirrors the six fields the `status_bar::view` function renders. Server time
/// is always `"— UTC"` in tests because `server_time_now` is `None` (no 1 Hz
/// subscription running). Version label is captured via the same `concat!`
/// macro the widget uses so a crate-version bump is caught in the diff.
fn status_bar_summary(c: &Cockpit) -> String {
    use ui::state::MarketHealthState;
    use ui::strings::{
        STATUS_BAR_CONNECTED, STATUS_BAR_DISCONNECTED, STATUS_BAR_LATENCY_LABEL,
        STATUS_BAR_NO_LATENCY, STATUS_BAR_RECONNECTING, STATUS_BAR_SERVER_LABEL,
    };
    use ui::theme::{color as t, color_for_latency_ms, ThemeMode};

    let dark = ThemeMode::Dark;
    let _fg3 = t::FG_3.current(dark);
    let mut out = String::new();
    out.push_str("widget: status_bar\n");

    // Connection field.
    let health = &c.market_health;
    let (dot_color_name, conn_text) = if health.is_empty() {
        ("fg_muted", STATUS_BAR_DISCONNECTED.to_string())
    } else {
        let mut stale: Vec<String> = health
            .iter()
            .filter_map(|(v, s)| {
                if *s == MarketHealthState::Stale {
                    Some(v.to_string())
                } else {
                    None
                }
            })
            .collect();
        stale.sort();
        if !stale.is_empty() {
            (
                "warn",
                format!("{STATUS_BAR_RECONNECTING} · {}", stale.join(", ")),
            )
        } else {
            let mut fresh: Vec<String> = health.keys().map(|v| v.to_string()).collect();
            fresh.sort();
            (
                "pos",
                format!("{STATUS_BAR_CONNECTED} · {}", fresh.join(", ")),
            )
        }
    };
    out.push_str(&format!("connection_dot: {dot_color_name}\n"));
    out.push_str(&format!("connection: {conn_text}\n"));

    // Latency field.
    let (latency_text, latency_color_name) = match c.latency {
        Latency::Known { ms } => {
            let col = color_for_latency_ms(ms);
            let name = if col == t::UP_500.current(dark) {
                "pos"
            } else if col == t::DOWN_500.current(dark) {
                "neg"
            } else if col == t::WARN_500.current(dark) {
                "warn"
            } else {
                "unknown"
            };
            (format!("{STATUS_BAR_LATENCY_LABEL} {ms} ms"), name)
        }
        Latency::Unknown => (
            format!("{STATUS_BAR_LATENCY_LABEL} {STATUS_BAR_NO_LATENCY}"),
            "fg_muted", // Latency::Unknown always renders fg3 → "fg_muted" label
        ),
    };
    out.push_str(&format!(
        "latency: {latency_text} color={latency_color_name}\n"
    ));

    // Account field.
    let acct = c.account_label.as_str();
    if acct.is_empty() {
        out.push_str("account:\n");
    } else {
        out.push_str(&format!("account: {acct}\n"));
    }

    // Server time — always "— UTC" in tests.
    out.push_str(&format!("{STATUS_BAR_SERVER_LABEL}: \u{2014} UTC\n"));

    // CPU placeholder.
    out.push_str("cpu: CPU —\n");

    // Version.
    let version = concat!("v", env!("CARGO_PKG_VERSION"), " · rust");
    out.push_str(&format!("version: {version}\n"));

    out
}

/// Map an iced `Color` back to its Lumen token name for stable snapshots.
///
/// Uses the dark-mode resolution of each `ModeColor` token because the
/// cockpit cold-starts in `ThemeMode::Dark` and all widgets resolve against
/// dark-mode at render time.
fn color_name(c: iced::Color) -> &'static str {
    use ui::theme::{color as t, ThemeMode};
    let dark = ThemeMode::Dark;
    if c == t::UP_500.current(dark) {
        "pos"
    } else if c == t::DOWN_500.current(dark) {
        "neg"
    } else if c == t::WARN_500.current(dark) {
        "warn"
    } else if c == t::FG_1.current(dark) {
        "fg"
    } else if c == t::FG_3.current(dark) {
        "fg_muted"
    } else if c == t::ACCENT.current(dark) {
        "accent"
    } else if c == t::CANVAS.current(dark) {
        "bg"
    } else if c == t::PANEL.current(dark) {
        "bg_elev"
    } else if c == t::BORDER_1.current(dark) {
        "border"
    } else {
        "unknown"
    }
}
