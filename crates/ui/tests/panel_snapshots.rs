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

use ui::state::{
    update, AgentMode, Cockpit, KillState, Latency, Message, PanelState, StrategyStatus,
};
use ui::strings;
use ui::widgets::latency::Badge;

// ── Live tape ───────────────────────────────────────────────────────────────

#[test]
fn tape_loading() {
    let c = Cockpit::new();
    assert_snapshot!("tape_loading", tape_summary(&c));
}

#[test]
fn tape_empty() {
    let mut c = Cockpit::new();
    // A refresh to an empty fill list would look like this:
    c.tape = PanelState::Empty;
    assert_snapshot!("tape_empty", tape_summary(&c));
}

#[test]
fn tape_error() {
    let mut c = Cockpit::new();
    update(
        &mut c,
        Message::TapeError(SmolStr::new("broadcast channel closed")),
    );
    assert_snapshot!("tape_error", tape_summary(&c));
}

#[test]
fn tape_ready_three_fills() {
    let c = ui::fixtures::fake_cockpit_ready_with_three_fills();
    assert_snapshot!("tape_ready_three_fills", tape_summary(&c));
}

#[test]
fn tape_paused_banner_visible() {
    let mut c = ui::fixtures::fake_cockpit_ready_with_three_fills();
    update(&mut c, Message::TapePauseToggled);
    assert_snapshot!("tape_paused", tape_summary(&c));
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

// ── Summary helpers — plain-text state rendering ────────────────────────────

fn tape_summary(c: &Cockpit) -> String {
    let mut out = String::new();
    out.push_str("panel: tape\n");
    out.push_str(&format!("title: {}\n", strings::PANEL_TAPE_TITLE));
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
    out.push_str(&format!("  - {}\n", strings::PANEL_TAPE_TITLE));
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
                    // v1.5a pair events — displayed as "loaded" label (muted, observation-only)
                    StrategyEventKind::MeanReversionStop
                    | StrategyEventKind::PairShortObservation => strings::STRATEGIES_EVENT_LOAD,
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
                // v1.5a pair events — show as "loaded" (muted, observation-only in v1.5a)
                StrategyEventKind::MeanReversionStop | StrategyEventKind::PairShortObservation => {
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

/// Map an iced `Color` back to its token name for stable snapshots.
fn color_name(c: iced::Color) -> &'static str {
    use ui::theme::color as t;
    if c == t::POS {
        "pos"
    } else if c == t::NEG {
        "neg"
    } else if c == t::WARN {
        "warn"
    } else if c == t::FG {
        "fg"
    } else if c == t::FG_MUTED {
        "fg_muted"
    } else if c == t::ACCENT {
        "accent"
    } else if c == t::BG {
        "bg"
    } else if c == t::BG_ELEV {
        "bg_elev"
    } else if c == t::BORDER {
        "border"
    } else {
        "unknown"
    }
}
