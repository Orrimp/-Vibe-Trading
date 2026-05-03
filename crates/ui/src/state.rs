//! Cockpit state model and message enum.
//!
//! One `Cockpit` struct, one `Message` enum. No business logic lives here —
//! only presentation state. Data comes in via feed messages and ledger
//! refresh callbacks; `update` is pure.

use std::collections::{HashMap, VecDeque};

use smol_str::SmolStr;
use trading_core::{
    Bar, FillView, JournalEntry, PnlSnapshot, PositionView, StrategyEventKind, StrategyEventView,
    StrategyId, StrategyLoadError, StrategyLoaded, StrategySwapped, Tick, Timestamp,
};

use crate::theme::layout::TAPE_MAX_ROWS;

/// Maximum number of recent strategy events kept in the cockpit's in-memory
/// window. The panel only renders the top ten but we hold a small buffer so a
/// brief panel hiccup doesn't drop events the operator was about to read.
pub const STRATEGIES_RECENT_EVENT_CAP: usize = 10;

/// Rolling 60-second window size (bars). At 1s ticks this is enough; bars
/// drive refresh so the window is counted in seconds by the ringbuffer helper.
pub const STRATEGIES_SIGNAL_WINDOW_SECS: u64 = 60;

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

/// Modal-only sub-state for the tape-row → audit-modal feature
/// ([tape-row-audit-modal R8](../../spec/features/tape-row-audit-modal.md#r8)).
///
/// The full `PanelState<T>` machinery applies (`Loading` / `Empty` / `Error` /
/// `Ready`) — every modal render covers the four arms per the principles
/// "no blank screens" rule.
#[derive(Debug, Clone)]
pub struct JournalModalState {
    /// The transaction id the modal is rendering. Populated at click
    /// time and carried as the modal's identity until close.
    pub tx_id: SmolStr,
    /// The entries panel state — first arrives as `Loading`, flips to
    /// `Ready(view)` on `TapeAuditEntriesLoaded(Ok)`, `Error` on `Err`,
    /// `Empty` when `entries.is_empty()` (defensive — every transaction
    /// has ≥ 2 entries by audit invariant).
    pub entries: PanelState<JournalTransactionView>,
}

/// Header + entries view for the journal-transaction audit modal.
///
/// Header rows render as label-value pairs above the 4-column entries
/// table. The architect's design ([tape-row-audit-modal Q2](../../spec/features/tape-row-audit-modal.md#q2--journalentry-un-collapsed-lives-in-trading_core))
/// pins the field shape so the widget swap is mechanical.
#[derive(Debug, Clone)]
pub struct JournalTransactionView {
    pub tx_id: SmolStr,
    pub ts: Timestamp,
    pub description: SmolStr,
    pub strategy_id: Option<StrategyId>,
    pub entries: Vec<JournalEntry>,
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

/// Trip-the-kill-switch closure type used by the cockpit's
/// `Message::KillConfirmed` handler.
///
/// **Why a closure not the `Arc<KillSwitch>` directly:** `KillSwitch::trip`
/// uses `tokio::spawn` internally for its dual-write side effect (T809);
/// that requires a tokio runtime in scope at the call site. The iced
/// `update` arm runs on the iced thread, where there is no tokio runtime.
/// The closure injects the side-thread runtime's `tokio::runtime::Handle`
/// so the spawn lands on it instead.
///
/// Constructed by `cockpit_live` (the unified bin); left `None` for
/// `cockpit --features fixtures` (no kill switch to trip).
#[cfg(feature = "live")]
pub type KillTripFn = std::sync::Arc<dyn Fn(agent::HaltReason) + Send + Sync>;

/// Per-strategy status pill (R5). A row can carry an error badge (with
/// `error_summary` copy) while the overall panel is still `Ready` — this is
/// the "malformed TOML, old strategy keeps running" visual from R8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrategyStatus {
    /// Strategy is loading (first bar / initial boot).
    Loading,
    /// Strategy is healthy and producing signals.
    Ready,
    /// Strategy was rejected on the last load attempt. Carries the
    /// `error_summary` from the `StrategyLoadError`.
    Error(SmolStr),
}

/// Rolling timestamps of recent signals, used to compute the `Signals / 60s`
/// cell without keeping the entire history.
///
/// Designed for scans at UI-event rates, not the hot path — we use `VecDeque`
/// and prune from the front on every insert + query.
#[derive(Debug, Clone, Default)]
pub struct SignalWindow {
    /// Timestamps of individual signal events, newest at the back.
    observations: VecDeque<Timestamp>,
}

impl SignalWindow {
    /// Record a new signal observation. The window is bounded implicitly by
    /// `len_in_window`; callers prune on read.
    pub fn push(&mut self, ts: Timestamp) {
        self.observations.push_back(ts);
    }

    /// Number of observations within the last `STRATEGIES_SIGNAL_WINDOW_SECS`
    /// seconds relative to `now`, pruning older entries as a side effect.
    pub fn count_in_window(&mut self, now: Timestamp) -> u32 {
        let now_ms = now.unix_millis();
        // Window floor in milliseconds — anything older is pruned.
        let floor_ms = now_ms
            .saturating_sub(i64::try_from(STRATEGIES_SIGNAL_WINDOW_SECS).unwrap_or(60) * 1000);
        while let Some(front) = self.observations.front() {
            if front.unix_millis() < floor_ms {
                self.observations.pop_front();
            } else {
                break;
            }
        }
        u32::try_from(self.observations.len()).unwrap_or(u32::MAX)
    }
}

/// A single row in the strategies panel (R5.1). Carries enough for the
/// table cells plus the hash tooltip and the per-row error badge.
#[derive(Debug, Clone)]
pub struct StrategyRow {
    pub id: StrategyId,
    /// 7-char prefix of the full hex hash — what the table renders.
    pub short_hash: SmolStr,
    /// Full 64-char hex hash — tooltip only.
    pub full_hash: SmolStr,
    pub status: StrategyStatus,
    /// Most recent strategy-event applied to this row (load / swap / reject).
    pub last_event: Option<StrategyEventView>,
    /// Signals observed in the last 60 seconds (rolling).
    pub signals_60s: u32,
    /// Whether the strategy is currently holding a net position. Surfaces in
    /// the `Holds position` column; the cockpit toggles this via position-
    /// refresh messages driven by the existing positions bus channel.
    pub has_position: bool,
    /// Repo-relative TOML path; rendered under the hash in the tooltip.
    pub source_path: SmolStr,
}

/// Root cockpit model. Owned by the iced `Application`.
///
/// `Debug` and `Clone` are implemented manually so the optional
/// `kill_switch` closure (`#[cfg(feature = "live")]`) does not block the
/// derive — `Arc<dyn Fn(...)>` does not implement `Debug`.
#[derive(Clone)]
pub struct Cockpit {
    pub mode: AgentMode,

    // Panels (each carries its own PanelState).
    pub tape: PanelState<VecDeque<FillView>>,
    pub tape_paused: bool,
    /// Buffered fills received while paused; flushed on resume.
    pub tape_paused_buffer: VecDeque<FillView>,

    pub positions: PanelState<Vec<PositionView>>,
    pub pnl: PanelState<PnlSnapshot>,

    /// Strategies panel (R5, v0.5). The ordered `Vec<StrategyRow>` is the
    /// table source; individual bus events mutate the matching row in place.
    pub strategies: PanelState<Vec<StrategyRow>>,
    /// Per-strategy rolling 60s signal counter. Keyed by `StrategyId`. Grown
    /// lazily as strategies are loaded; never grows without bound because
    /// `Unload` events remove the entry.
    pub strategies_signal_counters: HashMap<StrategyId, SignalWindow>,
    /// Ring of recent strategy events for the footer list under the table.
    /// Newest-first. Capped at `STRATEGIES_RECENT_EVENT_CAP`.
    pub strategies_recent_events: VecDeque<StrategyEventView>,

    pub kill: KillState,
    pub latency: Latency,

    /// Most recent bar/tick timestamps for debug/telemetry.
    pub last_bar_ts: Option<Timestamp>,
    pub last_tick_ts: Option<Timestamp>,

    /// Tape-row → audit-modal sub-state. `None` while the modal is closed
    /// (the cockpit's `view` then renders the main column directly so the
    /// pre-modal panel snapshots stay byte-identical). `Some(state)` while
    /// the modal is open — the cockpit wraps its main column in
    /// `widgets::journal_transaction_modal::view(...)` and the
    /// modal-open-gated keyboard subscription routes `Esc` →
    /// `Message::TapeAuditModalClosed` per
    /// [tape-row-audit-modal Q6](../../spec/features/tape-row-audit-modal.md#q6--keyboard-absorption-subscription-on-modal-open).
    pub tape_audit_modal: Option<JournalModalState>,

    /// Trip-the-kill-switch closure (T906). When set, processing
    /// `Message::KillConfirmed` invokes this with `HaltReason::ManualOperator`
    /// before transitioning the UI into `KillState::Flattening`. Absent in
    /// fixture / standalone-cockpit modes — see `KillTripFn` doc.
    #[cfg(feature = "live")]
    pub kill_switch: Option<KillTripFn>,
}

impl std::fmt::Debug for Cockpit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut dbg = f.debug_struct("Cockpit");
        dbg.field("mode", &self.mode)
            .field("tape", &self.tape)
            .field("tape_paused", &self.tape_paused)
            .field("tape_paused_buffer", &self.tape_paused_buffer)
            .field("positions", &self.positions)
            .field("pnl", &self.pnl)
            .field("strategies", &self.strategies)
            .field(
                "strategies_signal_counters",
                &self.strategies_signal_counters,
            )
            .field("strategies_recent_events", &self.strategies_recent_events)
            .field("kill", &self.kill)
            .field("latency", &self.latency)
            .field("last_bar_ts", &self.last_bar_ts)
            .field("last_tick_ts", &self.last_tick_ts)
            .field("tape_audit_modal", &self.tape_audit_modal);
        #[cfg(feature = "live")]
        dbg.field(
            "kill_switch",
            &self.kill_switch.as_ref().map(|_| "<trip-fn>"),
        );
        dbg.finish()
    }
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
            strategies: PanelState::Loading,
            strategies_signal_counters: HashMap::new(),
            strategies_recent_events: VecDeque::with_capacity(STRATEGIES_RECENT_EVENT_CAP),
            kill: KillState::Idle,
            latency: Latency::Unknown,
            last_bar_ts: None,
            last_tick_ts: None,
            tape_audit_modal: None,
            #[cfg(feature = "live")]
            kill_switch: None,
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
            // Strategies panel defaults to Loading in this constructor.
            // Fixture helpers populate it via the dedicated `fake_strategy_rows`.
            strategies: PanelState::Loading,
            strategies_signal_counters: HashMap::new(),
            strategies_recent_events: VecDeque::with_capacity(STRATEGIES_RECENT_EVENT_CAP),
            kill: KillState::Idle,
            latency: Latency::Known { ms: 120 },
            last_bar_ts: None,
            last_tick_ts: None,
            tape_audit_modal: None,
            #[cfg(feature = "live")]
            kill_switch: None,
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

    // ── v0.5 strategies panel (R5) ───────────────────────────────────────────
    /// A new strategy was loaded for the first time. Upserts a `Ready` row.
    StrategyLoaded(StrategyLoaded),
    /// A strategy's configuration was hot-swapped. Updates the row's hash +
    /// `last_event` in place.
    StrategySwapped(StrategySwapped),
    /// A strategy's load attempt was rejected; if the strategy was previously
    /// `Ready`, its row stays `Ready` (old strategy keeps running) but the
    /// row's `status` flips to `Error` carrying the `error_summary`.
    StrategyLoadError(StrategyLoadError),
    /// Snapshot refresh from `audit::query::strategy_events_since` at
    /// `BarClose`. Replaces the panel body.
    StrategiesRefreshed(Vec<StrategyRow>),
    /// Bus channel closed or other read failure — flips the whole panel into
    /// the error state with operator-friendly copy (prefix + detail).
    StrategiesError(SmolStr),
    /// A fill was observed that originated from the given strategy; increment
    /// the rolling 60s counter for that row.
    StrategySignalObserved(StrategyId, Timestamp),

    // ── tape-row audit modal (R1, R2, R4 — `tape-row-audit-modal`) ──────────
    /// Operator clicked a tape row. Carries the `journal_transactions.id`
    /// UUID string of the fill's underlying transaction. The cockpit's
    /// binary issues the `audit::query::journal_entries_for_transaction`
    /// fetch via `iced::Task::perform` and routes the result back as
    /// `TapeAuditEntriesLoaded`. `update` only sets the modal sub-state to
    /// `Loading` — pure-function discipline (R5).
    TapeRowClicked(SmolStr),
    /// Operator dismissed the modal — funnel for `Esc`, click-outside, and
    /// the explicit Close button (R4 three close affordances).
    TapeAuditModalClosed,
    /// Async result of the `journal_entries_for_transaction` fetch issued
    /// after `TapeRowClicked`. `Ok(view)` flips the modal to
    /// `Ready(view)` (or `Empty` when `view.entries.is_empty()` per R8 /
    /// audit invariant); `Err(msg)` flips to `Error(msg)` so the operator
    /// sees `TAPE_AUDIT_MODAL_ERROR_PREFIX + msg` and can dismiss.
    TapeAuditEntriesLoaded(Result<JournalTransactionView, SmolStr>),
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
                    // T906: trip the real kill switch (when running under
                    // `cockpit_live`'s wired-bus path). The closure spawns
                    // `KillSwitch::trip` onto the side-thread tokio runtime
                    // so the T809 dual-write (audit memo + strategy_events
                    // row + incident-spawn helper) executes end-to-end.
                    // `cockpit --features fixtures` boots with
                    // `kill_switch = None`, preserving fixture-only
                    // smoke-test behaviour: the UI flips to
                    // `KillState::Flattening` without any agent contact.
                    #[cfg(feature = "live")]
                    if let Some(trip) = model.kill_switch.as_ref() {
                        trip(agent::HaltReason::ManualOperator);
                    }
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
            // Q9 — operator's attention belongs on the halt banner, not on
            // a stacked read-only modal. Audit data stays queryable
            // post-halt via the same row click.
            model.tape_audit_modal = None;
        }
        Message::StrategyLoaded(ev) => {
            apply_strategy_loaded(model, ev);
        }
        Message::StrategySwapped(ev) => {
            apply_strategy_swapped(model, &ev);
        }
        Message::StrategyLoadError(ev) => {
            apply_strategy_load_error(model, &ev);
        }
        Message::StrategiesRefreshed(rows) => {
            model.strategies = if rows.is_empty() {
                PanelState::Empty
            } else {
                PanelState::Ready(rows)
            };
        }
        Message::StrategiesError(e) => {
            model.strategies = PanelState::Error(e);
        }
        Message::StrategySignalObserved(id, ts) => {
            let entry = model
                .strategies_signal_counters
                .entry(id.clone())
                .or_default();
            entry.push(ts);
            let count = entry.count_in_window(ts);
            if let PanelState::Ready(rows) = &mut model.strategies {
                if let Some(row) = rows.iter_mut().find(|r| r.id == id) {
                    row.signals_60s = count;
                }
            }
        }
        Message::TapeRowClicked(tx_id) => {
            // Q9 — only one modal at a time; a second click while the
            // previous modal is still open replaces identity unconditionally.
            // No back-stack — the cockpit is an instrument, not a browser.
            model.tape_audit_modal = Some(JournalModalState {
                tx_id,
                entries: PanelState::Loading,
            });
            // The async `journal_entries_for_transaction` fetch is issued
            // by the binary's `update` wrapper via `iced::Task::perform`.
            // `update` here stays pure — R5 / pure-function discipline.
        }
        Message::TapeAuditModalClosed => {
            model.tape_audit_modal = None;
        }
        Message::TapeAuditEntriesLoaded(result) => {
            if let Some(modal) = model.tape_audit_modal.as_mut() {
                modal.entries = match result {
                    Ok(view) => {
                        if view.entries.is_empty() {
                            PanelState::Empty
                        } else {
                            PanelState::Ready(view)
                        }
                    }
                    Err(msg) => PanelState::Error(msg),
                };
            }
            // If the modal was closed before the async fetch returned
            // (operator hit Esc mid-flight), drop the result silently —
            // there is no panel to update and `update` stays pure.
        }
    }
}

// ── Strategy-panel state transitions ────────────────────────────────────────
//
// Kept as free functions rather than nested `match` arms so the `update`
// function stays under clippy's `too_many_lines` limit and the transitions
// are individually testable.

fn apply_strategy_loaded(model: &mut Cockpit, ev: StrategyLoaded) {
    let (short_hash, full_hash) = hash_strings(&ev.hash);
    let view = strategy_event_view_from_loaded(&ev);
    let row = StrategyRow {
        id: ev.id.clone(),
        short_hash,
        full_hash,
        status: StrategyStatus::Ready,
        last_event: Some(view.clone()),
        signals_60s: 0,
        has_position: false,
        source_path: ev.source_path.clone(),
    };
    upsert_row(model, row);
    push_recent_event(model, view);
    model.strategies_signal_counters.entry(ev.id).or_default();
}

fn apply_strategy_swapped(model: &mut Cockpit, ev: &StrategySwapped) {
    let (short_hash, full_hash) = hash_strings(&ev.new_hash);
    let view = strategy_event_view_from_swapped(ev);
    let replacement = StrategyRow {
        id: ev.id.clone(),
        short_hash,
        full_hash,
        // A successful swap clears any previous error badge.
        status: StrategyStatus::Ready,
        last_event: Some(view.clone()),
        signals_60s: 0,
        has_position: false,
        source_path: ev.source_path.clone(),
    };
    upsert_row(model, replacement);
    push_recent_event(model, view);
}

fn apply_strategy_load_error(model: &mut Cockpit, ev: &StrategyLoadError) {
    let view = strategy_event_view_from_load_error(ev);
    push_recent_event(model, view.clone());
    let Some(id) = ev.strategy_id.clone() else {
        // No strategy-id on the failed file (e.g. non-UTF8 filename). There
        // is no row to attach the error to; the event is visible only in the
        // footer list.
        return;
    };

    // If the row exists, flip its status to Error but keep the existing
    // hash / last_event. If it doesn't, create a placeholder `Error`-status
    // row so the operator sees what failed even on the very first load.
    if let PanelState::Ready(rows) = &mut model.strategies {
        if let Some(row) = rows.iter_mut().find(|r| r.id == id) {
            row.status = StrategyStatus::Error(ev.error_summary.clone());
            row.last_event = Some(view);
            return;
        }
    }

    // No existing row — surface the error as a freshly-failed load.
    let placeholder = StrategyRow {
        id,
        short_hash: SmolStr::new(""),
        full_hash: SmolStr::new(""),
        status: StrategyStatus::Error(ev.error_summary.clone()),
        last_event: Some(view),
        signals_60s: 0,
        has_position: false,
        source_path: ev.source_path.clone(),
    };
    upsert_row(model, placeholder);
}

/// Insert `row` into the panel's row list, replacing an existing row with the
/// same `id`. Transitions the panel from `Loading` / `Empty` / `Error` to
/// `Ready` on first row.
fn upsert_row(model: &mut Cockpit, row: StrategyRow) {
    let rows = match std::mem::replace(&mut model.strategies, PanelState::Loading) {
        PanelState::Ready(mut v) => {
            if let Some(existing) = v.iter_mut().find(|r| r.id == row.id) {
                *existing = row;
            } else {
                v.push(row);
            }
            v
        }
        _ => vec![row],
    };
    model.strategies = PanelState::Ready(rows);
}

fn push_recent_event(model: &mut Cockpit, view: StrategyEventView) {
    model.strategies_recent_events.push_front(view);
    while model.strategies_recent_events.len() > STRATEGIES_RECENT_EVENT_CAP {
        model.strategies_recent_events.pop_back();
    }
}

/// Build `(short_hash, full_hash)` `SmolStr` pair from a 32-byte sha256.
fn hash_strings(hash: &[u8; 32]) -> (SmolStr, SmolStr) {
    let mut full = String::with_capacity(64);
    for b in hash {
        // Lower-case hex, two chars per byte.
        let hi = (b >> 4) & 0x0F;
        let lo = b & 0x0F;
        full.push(hex_nibble(hi));
        full.push(hex_nibble(lo));
    }
    let short: SmolStr = full.chars().take(7).collect::<String>().into();
    (short, full.into())
}

const fn hex_nibble(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        10..=15 => (b'a' + (n - 10)) as char,
        _ => '?',
    }
}

fn strategy_event_view_from_loaded(ev: &StrategyLoaded) -> StrategyEventView {
    let (_short, full) = hash_strings(&ev.hash);
    StrategyEventView {
        id: SmolStr::new(""),
        ts: ev.ts,
        kind: StrategyEventKind::Load,
        strategy_id: Some(ev.id.clone()),
        old_hash: None,
        new_hash: Some(full),
        source_path: Some(ev.source_path.clone()),
        operator: SmolStr::new("system"),
        error_code: None,
        error_summary: None,
    }
}

fn strategy_event_view_from_swapped(ev: &StrategySwapped) -> StrategyEventView {
    let (_short_prev, full_prev) = hash_strings(&ev.old_hash);
    let (_short_curr, full_curr) = hash_strings(&ev.new_hash);
    StrategyEventView {
        id: SmolStr::new(""),
        ts: ev.ts,
        kind: StrategyEventKind::Swap,
        strategy_id: Some(ev.id.clone()),
        old_hash: Some(full_prev),
        new_hash: Some(full_curr),
        source_path: Some(ev.source_path.clone()),
        operator: SmolStr::new("system"),
        error_code: None,
        error_summary: None,
    }
}

fn strategy_event_view_from_load_error(ev: &StrategyLoadError) -> StrategyEventView {
    StrategyEventView {
        id: SmolStr::new(""),
        ts: ev.ts,
        kind: StrategyEventKind::Reject,
        strategy_id: ev.strategy_id.clone(),
        old_hash: None,
        new_hash: None,
        source_path: Some(ev.source_path.clone()),
        operator: SmolStr::new("system"),
        error_code: Some(ev.error_code.clone()),
        error_summary: Some(ev.error_summary.clone()),
    }
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used)]
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

    /// T906 — when `kill_switch` is wired with a real trip closure, the
    /// `Message::KillConfirmed` arm calls the closure exactly once with
    /// `HaltReason::ManualOperator` before flipping the UI to
    /// `KillState::Flattening`. This is the analyst's finding-#2 fix:
    /// pre-T906 the arm only updated UI state; post-T906 it actually
    /// halts the agent via the side-thread tokio runtime.
    #[cfg(feature = "live")]
    #[test]
    fn t906_kill_confirmed_calls_trip_closure_with_manual_operator() {
        use std::sync::Mutex;

        let captured: std::sync::Arc<Mutex<Vec<agent::HaltReason>>> =
            std::sync::Arc::new(Mutex::new(Vec::new()));
        let captured_clone = std::sync::Arc::clone(&captured);
        let trip: KillTripFn = std::sync::Arc::new(move |reason| {
            captured_clone
                .lock()
                .expect("test mutex unpoisoned")
                .push(reason);
        });

        let mut c = Cockpit::new();
        c.kill_switch = Some(trip);

        update(&mut c, Message::KillPressed);
        update(
            &mut c,
            Message::KillConfirmPhraseChanged(crate::strings::KILL_SAFETY_PHRASE.to_string()),
        );
        update(&mut c, Message::KillConfirmed);

        let calls = captured.lock().expect("test mutex unpoisoned");
        assert_eq!(
            calls.len(),
            1,
            "trip closure must be called exactly once on KillConfirmed",
        );
        assert!(
            matches!(calls[0], agent::HaltReason::ManualOperator),
            "trip closure must receive HaltReason::ManualOperator, got {:?}",
            calls[0],
        );
        assert_eq!(c.kill, KillState::Flattening);
    }

    /// T906 — wrong-phrase path must NOT call the trip closure (the
    /// safety-phrase gate is the operator's last guard against an
    /// accidental kill).
    #[cfg(feature = "live")]
    #[test]
    fn t906_kill_confirmed_with_wrong_phrase_does_not_call_trip() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let calls = std::sync::Arc::new(AtomicUsize::new(0));
        let calls_clone = std::sync::Arc::clone(&calls);
        let trip: KillTripFn = std::sync::Arc::new(move |_reason| {
            calls_clone.fetch_add(1, Ordering::SeqCst);
        });

        let mut c = Cockpit::new();
        c.kill_switch = Some(trip);

        update(&mut c, Message::KillPressed);
        update(&mut c, Message::KillConfirmPhraseChanged("HALT".into()));
        update(&mut c, Message::KillConfirmed);

        assert_eq!(
            calls.load(Ordering::SeqCst),
            0,
            "trip closure must NOT fire on phrase mismatch",
        );
        assert!(matches!(c.kill, KillState::Confirming { .. }));
    }

    /// T906 — fixture / standalone-cockpit boot leaves `kill_switch =
    /// None`; the arm must still flip to `Flattening` (UI-only effect),
    /// preserving the pre-T906 behavior under `cargo run --bin cockpit
    /// --features fixtures`.
    #[cfg(feature = "live")]
    #[test]
    fn t906_kill_confirmed_with_no_closure_still_advances_ui() {
        let mut c = Cockpit::new();
        assert!(c.kill_switch.is_none());

        update(&mut c, Message::KillPressed);
        update(
            &mut c,
            Message::KillConfirmPhraseChanged(crate::strings::KILL_SAFETY_PHRASE.to_string()),
        );
        update(&mut c, Message::KillConfirmed);

        assert_eq!(c.kill, KillState::Flattening);
    }

    // ── v0.5 strategies panel state tests (T523) ────────────────────────────

    fn dummy_loaded(id: &str) -> StrategyLoaded {
        StrategyLoaded {
            id: StrategyId::new(id),
            hash: [0xAAu8; 32],
            source_path: SmolStr::new(format!("config/strategies/{id}.toml")),
            ts: Timestamp::now(),
        }
    }

    fn dummy_swapped(id: &str) -> StrategySwapped {
        StrategySwapped {
            id: StrategyId::new(id),
            old_hash: [0xAAu8; 32],
            new_hash: [0xBBu8; 32],
            source_path: SmolStr::new(format!("config/strategies/{id}.toml")),
            ts: Timestamp::now(),
        }
    }

    fn dummy_load_error(id: Option<&str>) -> StrategyLoadError {
        StrategyLoadError {
            source_path: SmolStr::new("config/strategies/bad.toml"),
            strategy_id: id.map(StrategyId::new),
            error_code: SmolStr::new("toml_parse"),
            error_summary: SmolStr::new("unexpected token at line 3"),
            ts: Timestamp::now(),
        }
    }

    #[test]
    fn strategies_start_loading() {
        let c = Cockpit::new();
        assert_eq!(c.strategies.variant_name(), "loading");
        assert!(c.strategies_recent_events.is_empty());
    }

    #[test]
    fn strategy_loaded_upserts_ready_row() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::StrategyLoaded(dummy_loaded("btc_macd_trend")),
        );
        match &c.strategies {
            PanelState::Ready(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].id, StrategyId::new("btc_macd_trend"));
                assert_eq!(rows[0].status, StrategyStatus::Ready);
                assert_eq!(rows[0].short_hash.len(), 7);
                assert_eq!(rows[0].full_hash.len(), 64);
            }
            other => panic!("expected Ready, got {}", other.variant_name()),
        }
        assert_eq!(c.strategies_recent_events.len(), 1);
    }

    #[test]
    fn strategy_swapped_updates_hash_clears_error() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::StrategyLoaded(dummy_loaded("btc_macd_trend")),
        );
        // Flip the row into an error state.
        update(
            &mut c,
            Message::StrategyLoadError(dummy_load_error(Some("btc_macd_trend"))),
        );
        match &c.strategies {
            PanelState::Ready(rows) => {
                assert!(matches!(rows[0].status, StrategyStatus::Error(_)));
            }
            other => panic!("expected Ready, got {}", other.variant_name()),
        }
        // Successful swap clears the error.
        update(
            &mut c,
            Message::StrategySwapped(dummy_swapped("btc_macd_trend")),
        );
        match &c.strategies {
            PanelState::Ready(rows) => {
                assert_eq!(rows[0].status, StrategyStatus::Ready);
                // Hash has flipped from 0xAA… to 0xBB…
                assert!(rows[0].full_hash.starts_with("bb"));
            }
            other => panic!("expected Ready, got {}", other.variant_name()),
        }
    }

    #[test]
    fn strategy_load_error_without_id_pushes_footer_only() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::StrategyLoaded(dummy_loaded("btc_macd_trend")),
        );
        update(&mut c, Message::StrategyLoadError(dummy_load_error(None)));
        // No new row, existing row stays Ready.
        match &c.strategies {
            PanelState::Ready(rows) => {
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].status, StrategyStatus::Ready);
            }
            other => panic!("expected Ready, got {}", other.variant_name()),
        }
        // Footer records the rejection.
        assert_eq!(c.strategies_recent_events.len(), 2);
    }

    #[test]
    fn strategies_refreshed_empty_moves_to_empty_not_ready() {
        let mut c = Cockpit::new();
        update(&mut c, Message::StrategiesRefreshed(vec![]));
        assert_eq!(c.strategies.variant_name(), "empty");
    }

    #[test]
    fn strategies_error_surfaces_message() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::StrategiesError(SmolStr::new("channel closed")),
        );
        assert_eq!(c.strategies.variant_name(), "error");
        if let PanelState::Error(e) = &c.strategies {
            assert_eq!(e.as_str(), "channel closed");
        }
    }

    #[test]
    fn strategy_signal_observed_increments_counter() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::StrategyLoaded(dummy_loaded("btc_macd_trend")),
        );
        let ts = Timestamp::now();
        update(
            &mut c,
            Message::StrategySignalObserved(StrategyId::new("btc_macd_trend"), ts),
        );
        update(
            &mut c,
            Message::StrategySignalObserved(StrategyId::new("btc_macd_trend"), ts),
        );
        match &c.strategies {
            PanelState::Ready(rows) => assert_eq!(rows[0].signals_60s, 2),
            other => panic!("expected Ready, got {}", other.variant_name()),
        }
    }

    #[test]
    fn strategy_load_error_without_existing_row_creates_error_placeholder() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::StrategyLoadError(dummy_load_error(Some("btc_bbands"))),
        );
        match &c.strategies {
            PanelState::Ready(rows) => {
                assert_eq!(rows.len(), 1);
                assert!(matches!(rows[0].status, StrategyStatus::Error(_)));
            }
            other => panic!("expected Ready, got {}", other.variant_name()),
        }
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
                transaction_id: smol_str::SmolStr::default(),
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
