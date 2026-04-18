//! Cockpit state model and message enum.
//!
//! One `Cockpit` struct, one `Message` enum. No business logic lives here —
//! only presentation state. Data comes in via feed messages and ledger
//! refresh callbacks; `update` is pure.

use std::collections::VecDeque;

use smol_str::SmolStr;
use trading_core::{Bar, FillView, PnlSnapshot, PositionView, Tick, Timestamp};

use crate::theme::layout::TAPE_MAX_ROWS;

/// Agent mode banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentMode {
    #[default]
    Research,
    Paper,
    Live,
    Halted,
}

/// Generic panel state — **every** panel renders one of these four variants.
/// "No data" is never a valid state; use `Empty` with an explanatory copy.
#[derive(Debug, Clone)]
pub enum PanelState<T> {
    /// Waiting for first data — explicit skeleton.
    Loading,
    /// Successfully got data, but the data itself is legitimately empty.
    /// Distinct from `Loading` so the copy is honest (e.g. "no positions,
    /// strategy is armed" vs "waiting for feed").
    Empty,
    /// Something broke; show what to check, not just the exception.
    Error(SmolStr),
    /// Happy path — actual data.
    Ready(T),
}

impl<T> PanelState<T> {
    /// Rough discriminator for tests and telemetry.
    #[must_use]
    pub fn variant_name(&self) -> &'static str {
        match self {
            PanelState::Loading => "loading",
            PanelState::Empty => "empty",
            PanelState::Error(_) => "error",
            PanelState::Ready(_) => "ready",
        }
    }
}

/// Kill-switch state machine, rendered by `widgets::kill`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum KillState {
    /// No dialog open, no halt. Default.
    #[default]
    Idle,
    /// Dialog open; operator typing the safety phrase.
    Confirming { typed: String },
    /// Confirm was sent to the agent; waiting for halted broadcast.
    Flattening,
    /// Agent acknowledged halted. Sticky — only operator action clears it.
    Halted { reason: SmolStr },
}

/// Latency reading derived from the most recent tick.
#[derive(Debug, Clone, Copy, Default)]
pub enum Latency {
    /// No tick observed yet.
    #[default]
    Unknown,
    /// Delta between venue ts and local clock, milliseconds.
    Known { ms: i64 },
}

/// Root cockpit model. Owned by the iced `Application`.
#[derive(Debug, Clone)]
pub struct Cockpit {
    pub mode: AgentMode,

    // Panels (each carries its own PanelState).
    pub tape: PanelState<VecDeque<FillView>>,
    pub tape_paused: bool,
    /// Buffered fills received while paused; flushed on resume.
    pub tape_paused_buffer: VecDeque<FillView>,

    pub positions: PanelState<Vec<PositionView>>,
    pub pnl: PanelState<PnlSnapshot>,

    pub kill: KillState,
    pub latency: Latency,

    /// Most recent bar/tick timestamps for debug/telemetry.
    pub last_bar_ts: Option<Timestamp>,
    pub last_tick_ts: Option<Timestamp>,
}

impl Default for Cockpit {
    fn default() -> Self {
        Self {
            mode: AgentMode::default(),
            tape: PanelState::Loading,
            tape_paused: false,
            tape_paused_buffer: VecDeque::new(),
            positions: PanelState::Loading,
            pnl: PanelState::Loading,
            kill: KillState::Idle,
            latency: Latency::Unknown,
            last_bar_ts: None,
            last_tick_ts: None,
        }
    }
}

impl Cockpit {
    /// Fresh cockpit. All panels start in `Loading`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Test / fixture constructor that boots every panel into Ready state
    /// with the provided data.
    #[must_use]
    pub fn ready(
        fills: impl IntoIterator<Item = FillView>,
        positions: Vec<PositionView>,
        pnl: PnlSnapshot,
    ) -> Self {
        let mut tape = VecDeque::with_capacity(TAPE_MAX_ROWS);
        for f in fills {
            if tape.len() == TAPE_MAX_ROWS {
                tape.pop_back();
            }
            tape.push_front(f);
        }
        Self {
            mode: AgentMode::Paper,
            tape: if tape.is_empty() {
                PanelState::Empty
            } else {
                PanelState::Ready(tape)
            },
            tape_paused: false,
            tape_paused_buffer: VecDeque::new(),
            positions: if positions.is_empty() {
                PanelState::Empty
            } else {
                PanelState::Ready(positions)
            },
            pnl: PanelState::Ready(pnl),
            kill: KillState::Idle,
            latency: Latency::Known { ms: 120 },
            last_bar_ts: None,
            last_tick_ts: None,
        }
    }
}

/// Every possible state mutation. Exhaustive by construction — `update`
/// matches on this enum with no catch-all arm.
#[derive(Debug, Clone)]
pub enum Message {
    // Feed events.
    BarReceived(Bar),
    TickReceived(Tick),
    FillReceived(FillView),
    /// Triggered at bar close; cockpit refreshes positions & P&L.
    BarClose(Timestamp),
    /// Observed clock-skew in ms (from `data::clock_skew_detector`).
    ClockSkew(i64),

    // Ledger query results (async; delivered via Subscription).
    PnlRefreshed(PnlSnapshot),
    PositionsRefreshed(Vec<PositionView>),
    PnlError(SmolStr),
    PositionsError(SmolStr),
    TapeError(SmolStr),

    // Operator actions on the kill switch (R7).
    KillPressed,
    KillConfirmPhraseChanged(String),
    KillConfirmed,
    KillCancelled,

    // Tape controls.
    TapePauseToggled,

    // Agent lifecycle.
    AgentModeChanged(AgentMode),
    AgentHaltedExternally(SmolStr),
}

/// Pure state-transition function. Never spawns async work directly —
/// that's the `Subscription`'s job in the binary.
///
/// Exhaustiveness is enforced: every variant of `Message` has its own
/// arm. Do **not** add a `_ =>` catch-all.
///
/// The function is long because every `Message` variant gets its own arm
/// — splitting it into sub-functions would obscure the one-place view of
/// the state machine. `clippy::too_many_lines` disagrees; we disagree.
#[allow(clippy::too_many_lines)]
pub fn update(model: &mut Cockpit, msg: Message) {
    match msg {
        Message::BarReceived(bar) => {
            model.last_bar_ts = Some(bar.close_ts);
        }
        Message::TickReceived(tick) => {
            let venue_ms = tick.venue_ts.unix_millis();
            let local_ms = tick.local_recv_ts.unix_millis();
            let ms = (local_ms - venue_ms).abs();
            model.latency = Latency::Known { ms };
            model.last_tick_ts = Some(tick.venue_ts);
        }
        Message::FillReceived(fill) => {
            if model.tape_paused {
                model.tape_paused_buffer.push_front(fill);
                while model.tape_paused_buffer.len() > TAPE_MAX_ROWS {
                    model.tape_paused_buffer.pop_back();
                }
            } else if let PanelState::Ready(q) = &mut model.tape {
                q.push_front(fill);
                while q.len() > TAPE_MAX_ROWS {
                    q.pop_back();
                }
            } else {
                let mut q = VecDeque::with_capacity(TAPE_MAX_ROWS);
                q.push_front(fill);
                model.tape = PanelState::Ready(q);
            }
        }
        Message::BarClose(ts) => {
            model.last_bar_ts = Some(ts);
            // UI doesn't recompute P&L itself (R3.6); the binary's
            // Subscription issues audit::query calls and routes the result
            // back as PnlRefreshed / PositionsRefreshed.
        }
        Message::ClockSkew(ms) => {
            model.latency = Latency::Known { ms };
        }
        Message::PnlRefreshed(snap) => {
            model.pnl = PanelState::Ready(snap);
        }
        Message::PositionsRefreshed(list) => {
            model.positions = if list.is_empty() {
                PanelState::Empty
            } else {
                PanelState::Ready(list)
            };
        }
        Message::PnlError(e) => {
            model.pnl = PanelState::Error(e);
        }
        Message::PositionsError(e) => {
            model.positions = PanelState::Error(e);
        }
        Message::TapeError(e) => {
            model.tape = PanelState::Error(e);
        }
        Message::KillPressed => {
            if matches!(model.kill, KillState::Idle) {
                model.kill = KillState::Confirming {
                    typed: String::new(),
                };
            }
        }
        Message::KillConfirmPhraseChanged(s) => {
            if let KillState::Confirming { typed } = &mut model.kill {
                *typed = s;
            }
        }
        Message::KillConfirmed => {
            if let KillState::Confirming { typed } = &model.kill {
                if typed == crate::strings::KILL_SAFETY_PHRASE {
                    model.kill = KillState::Flattening;
                }
            }
        }
        Message::KillCancelled => {
            if matches!(model.kill, KillState::Confirming { .. }) {
                model.kill = KillState::Idle;
            }
        }
        Message::TapePauseToggled => {
            model.tape_paused = !model.tape_paused;
            if !model.tape_paused {
                // Flush paused buffer into the tape, preserving order.
                let drained: Vec<FillView> = model.tape_paused_buffer.drain(..).collect();
                let mut existing = match std::mem::replace(&mut model.tape, PanelState::Loading) {
                    PanelState::Ready(q) => q,
                    _ => VecDeque::with_capacity(TAPE_MAX_ROWS),
                };
                for fill in drained {
                    existing.push_front(fill);
                    while existing.len() > TAPE_MAX_ROWS {
                        existing.pop_back();
                    }
                }
                model.tape = if existing.is_empty() {
                    PanelState::Empty
                } else {
                    PanelState::Ready(existing)
                };
            }
        }
        Message::AgentModeChanged(m) => {
            model.mode = m;
            if matches!(m, AgentMode::Halted) {
                model.kill = KillState::Halted {
                    reason: SmolStr::new("operator"),
                };
            }
        }
        Message::AgentHaltedExternally(reason) => {
            model.mode = AgentMode::Halted;
            model.kill = KillState::Halted { reason };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    fn pnl_snap() -> PnlSnapshot {
        PnlSnapshot {
            cash: trading_core::Money::from_decimal(dec!(1)),
            unrealized: trading_core::Money::from_decimal(dec!(0)),
            realized: trading_core::Money::from_decimal(dec!(0)),
            total_equity: trading_core::Money::from_decimal(dec!(1)),
            daily_return: trading_core::Money::from_decimal(dec!(0)),
            as_of: Timestamp::now(),
        }
    }

    #[test]
    fn fresh_cockpit_is_loading_everywhere() {
        let c = Cockpit::new();
        assert_eq!(c.tape.variant_name(), "loading");
        assert_eq!(c.positions.variant_name(), "loading");
        assert_eq!(c.pnl.variant_name(), "loading");
        assert_eq!(c.kill, KillState::Idle);
    }

    #[test]
    fn pnl_refresh_moves_to_ready() {
        let mut c = Cockpit::new();
        update(&mut c, Message::PnlRefreshed(pnl_snap()));
        assert_eq!(c.pnl.variant_name(), "ready");
    }

    #[test]
    fn positions_refresh_empty_moves_to_empty_not_ready() {
        let mut c = Cockpit::new();
        update(&mut c, Message::PositionsRefreshed(vec![]));
        assert_eq!(c.positions.variant_name(), "empty");
    }

    #[test]
    fn kill_flow_rejects_wrong_phrase() {
        let mut c = Cockpit::new();
        update(&mut c, Message::KillPressed);
        update(&mut c, Message::KillConfirmPhraseChanged("HALT".into()));
        update(&mut c, Message::KillConfirmed);
        // Still confirming — wrong phrase should not advance.
        assert!(matches!(c.kill, KillState::Confirming { .. }));
    }

    #[test]
    fn kill_flow_accepts_exact_phrase() {
        let mut c = Cockpit::new();
        update(&mut c, Message::KillPressed);
        update(
            &mut c,
            Message::KillConfirmPhraseChanged(crate::strings::KILL_SAFETY_PHRASE.to_string()),
        );
        update(&mut c, Message::KillConfirmed);
        assert_eq!(c.kill, KillState::Flattening);
    }

    #[test]
    fn kill_cancel_returns_to_idle() {
        let mut c = Cockpit::new();
        update(&mut c, Message::KillPressed);
        update(&mut c, Message::KillCancelled);
        assert_eq!(c.kill, KillState::Idle);
    }

    #[test]
    fn pause_buffers_fills_and_resume_flushes() {
        use trading_core::{FeeTier, Price, Quantity, Side, Symbol};
        fn fill(id: u64) -> FillView {
            FillView {
                symbol: Symbol::new("BTCUSDT"),
                side: Side::Buy,
                price: Price::new(dec!(100) + rust_decimal::Decimal::from(id))
                    .unwrap_or_else(|_| unreachable!()),
                qty: Quantity::new(dec!(1)).unwrap_or_else(|_| unreachable!()),
                fee: trading_core::Money::from_decimal(dec!(0)),
                fee_tier: FeeTier::Taker,
                venue_ts: Timestamp::now(),
            }
        }
        let mut c = Cockpit::new();
        update(&mut c, Message::FillReceived(fill(1)));
        update(&mut c, Message::TapePauseToggled);
        update(&mut c, Message::FillReceived(fill(2)));
        update(&mut c, Message::FillReceived(fill(3)));
        if let PanelState::Ready(q) = &c.tape {
            assert_eq!(q.len(), 1);
        } else {
            panic!("tape should be Ready with one fill");
        }
        assert_eq!(c.tape_paused_buffer.len(), 2);
        update(&mut c, Message::TapePauseToggled);
        if let PanelState::Ready(q) = &c.tape {
            assert_eq!(q.len(), 3);
        } else {
            panic!("tape should have all three fills after resume");
        }
    }
}
