//! Cockpit state model and message enum.
//!
//! One `Cockpit` struct, one `Message` enum. No business logic lives here —
//! only presentation state. Data comes in via feed messages and ledger
//! refresh callbacks; `update` is pure.

use std::collections::{HashMap, HashSet, VecDeque};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use trading_core::{
    Bar, EquitySeries, FillView, JournalEntry, MarketHealth, PnlSnapshot, PositionView, Side,
    Signal, SignalView, StrategyEventKind, StrategyEventView, StrategyId, StrategyLoadError,
    StrategyLoaded, StrategySwapped, Symbol, Tick, Timestamp, Venue,
};

use crate::theme::layout::TAPE_MAX_ROWS;

/// Maximum number of recent strategy events kept in the cockpit's in-memory
/// window. The panel only renders the top ten but we hold a small buffer so a
/// brief panel hiccup doesn't drop events the operator was about to read.
pub const STRATEGIES_RECENT_EVENT_CAP: usize = 10;

/// Rolling 60-second window size (bars). At 1s ticks this is enough; bars
/// drive refresh so the window is counted in seconds by the ringbuffer helper.
pub const STRATEGIES_SIGNAL_WINDOW_SECS: u64 = 60;

/// Phase 2 — rolling chart-buffer capacity per `(Venue, Symbol)`. 60 bars =
/// 60 minutes of 1-minute bars. Sibling of `STRATEGIES_RECENT_EVENT_CAP`
/// because this is a state-shape constant, not a render constant. (Phase 2
/// Design — `ChartBuffer shape`, R10.3.)
pub const CHART_BUFFER_CAPACITY: usize = 60;

/// Phase 2 — screen-routed shell. `Home / Debug / Charts` ship in Phase 2;
/// `Strategies / Risk / Audit` are declared now (Phase 3 wires their
/// dispatch) so Phase 3's enum extension is a backlog item, not an enum
/// migration. Phase 5 adds `Control` (`HumanControl` panel — Q1 ratification:
/// 7th sidebar entry). (Phase 2 Design Q-resolutions; R2.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Screen {
    /// Phase 2 — pnl + positions + strategies + `agent_feed` grid.
    #[default]
    Home,
    /// Phase 2 — kill + latency + market-health + version + logs stub.
    /// Phase 5 (Q1) — kill widget migrates to the `HumanControl` bottom
    /// action; the `Debug` screen no longer renders the kill panel.
    Debug,
    /// Phase 2 — chip-row + canvas chart with audit markers.
    Charts,
    /// Phase 3 — declared now; dispatch returns "Not yet" placeholder.
    Strategies,
    /// Phase 3 — declared now; dispatch returns "Not yet" placeholder.
    Risk,
    /// Phase 3 — declared now; dispatch returns "Not yet" placeholder.
    Audit,
    /// Phase 5 (Q1) — `HumanControl` panel (mode + 3 limit mirror rows
    /// + kill bottom action). Sidebar's 7th entry.
    Control,
}

/// Per-`(Venue, Symbol)` rolling 60-bar buffer. Fed by the
/// `Message::BarReceived` arm; evicts the oldest bar on push when at
/// capacity. (Phase 2 Design — `ChartBuffer shape`.)
#[derive(Debug, Default, Clone)]
pub struct ChartBuffer {
    /// Each value is a `VecDeque<Bar>` capped at
    /// [`CHART_BUFFER_CAPACITY`]; oldest-front, newest-back so the canvas
    /// paints left-to-right by iterating `bars()` directly.
    pub series: HashMap<(Venue, Symbol), VecDeque<Bar>>,
}

impl ChartBuffer {
    /// Push a new bar onto the deque for `(venue, symbol)`, evicting the
    /// oldest bar if the deque is at capacity.
    pub fn push_bar(&mut self, bar: Bar) {
        let key = (bar.venue, bar.symbol.clone());
        let series = self.series.entry(key).or_default();
        if series.len() == CHART_BUFFER_CAPACITY {
            series.pop_front();
        }
        series.push_back(bar);
    }

    /// Iterate the buffered bars for `(venue, symbol)` oldest-first.
    /// Returns an empty iterator if the key has never been seen.
    pub fn bars(&self, venue: Venue, symbol: &Symbol) -> impl Iterator<Item = &Bar> {
        self.series
            .get(&(venue, symbol.clone()))
            .into_iter()
            .flat_map(|deque| deque.iter())
    }
}

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

// ── Phase 5 — focus-state-machine WidgetId constants ───────────────────────
//
// These live here (state.rs) rather than in `widgets/focus_ring.rs`
// because the consistency test (`crates/ui/tests/consistency.rs`) scans
// every file under `src/widgets/` for inline user-visible string
// literals. The focus-state-machine keys ARE string literals but they
// are NEVER operator-visible — they are stable identifiers for the
// `Cockpit::focused_widget` field. Hosting them under `state.rs`
// satisfies the consistency rule cleanly.
pub mod focus_ids {
    use smol_str::SmolStr;

    /// Stable focus key for the kill button.
    pub const KILL_BUTTON: &str = "kill_button";
    /// Stable focus key for the kill-confirm typed-phrase input.
    pub const KILL_CONFIRM_INPUT: &str = "kill_confirm_input";
    /// Stable focus key for the kill-confirm "Confirm stop" button.
    pub const KILL_CONFIRM_BUTTON: &str = "kill_confirm_button";
    /// Stable focus key for the kill-confirm "Cancel" button.
    pub const KILL_CANCEL_BUTTON: &str = "kill_cancel_button";

    /// Stable focus key for the override-veto confirm input.
    pub const OVERRIDE_RISK_VETO_INPUT: &str = "override_risk_veto_input";
    /// Stable focus key for the override-veto confirm button.
    pub const OVERRIDE_RISK_VETO_CONFIRM: &str = "override_risk_veto_confirm";
    /// Stable focus key for the override-veto cancel button.
    pub const OVERRIDE_RISK_VETO_CANCEL: &str = "override_risk_veto_cancel";

    /// Stable focus key for the execution-mode "Observe" segment.
    pub const EXECUTION_MODE_OBSERVE: &str = "execution_mode_observe";
    /// Stable focus key for the execution-mode "Supervised" segment.
    pub const EXECUTION_MODE_SUPERVISED: &str = "execution_mode_supervised";
    /// Stable focus key for the execution-mode "Auto" segment.
    pub const EXECUTION_MODE_AUTO: &str = "execution_mode_auto";

    /// Per-strategy pause-button focus key. Format
    /// `"strategy_pause::<strategy_id>"`.
    #[must_use]
    pub fn strategy_pause_id(strategy_id: &str) -> SmolStr {
        SmolStr::new(format!("strategy_pause::{strategy_id}"))
    }

    /// Per-veto override-button focus key. Format
    /// `"override_veto_button::<veto_id>"`.
    #[must_use]
    pub fn override_veto_button_id(veto_id: &str) -> SmolStr {
        SmolStr::new(format!("override_veto_button::{veto_id}"))
    }
}

// ── Phase 5 — HumanControl panel + per-strategy pause + override-risk-veto ──
//
// All four additions ratified by the Phase 5 Design's "Cockpit state diff"
// sub-section. The `Cockpit::tape` field name is **preserved** per Q14;
// only the module path renames from `tape.rs` → `agent_feed.rs` (R11).

/// Phase 5 — execution mode (Q4 — runtime-only). Cold-start = `Observe`
/// (safest default; v0–v4 are config-driven, so introducing a UI-write-to-
/// disk surface for session ergonomics is out of bounds). Mirrors
/// `AgentMode`'s derive set.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ExecutionMode {
    /// Cold-start default per Q4 — safest setting; agent observes
    /// signals but does not send orders.
    #[default]
    Observe,
    Supervised,
    Auto,
}

/// Phase 5 — typed-confirm state for the override-risk-veto modal
/// (R7.4 / R7.5). Mirror of `KillState::Confirming { typed }`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum OverrideRiskVetoState {
    #[default]
    Idle,
    Confirming {
        veto_id: SmolStr,
        typed: String,
    },
    Submitting {
        veto_id: SmolStr,
    },
}

/// Phase 5 — surfaced risk-engine veto event (R7.2). Live upstream is
/// the `default_risk_telemetry_stub` at
/// `crates/agent/src/runtime.rs:1023–1090` (TD-2); fixtures populate
/// for visual baselines.
///
/// `Signal` (the blocked payload) does not derive `PartialEq / Eq` (it
/// carries floating-point evidence that is not bitwise-equal-safe), so
/// `VetoEvent` mirrors that derive set: `Debug + Clone + Serialize +
/// Deserialize`. Tests compare on `veto_id` instead of structural
/// equality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VetoEvent {
    pub veto_id: SmolStr,
    pub ts: Timestamp,
    pub strategy_id: StrategyId,
    pub reason: SmolStr,
    pub blocked_signal: Signal,
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

/// UI-side per-venue market-health state. Derived from `MarketHealth` bus
/// events; maps the three-variant bus enum to the two states the status bar
/// needs (connected = Fresh/Recovered; reconnecting = Stale).
///
/// Phase 1 note: `Reconnecting` subsumes both `Stale` and the brief gap
/// between `Stale` and `Recovered`. The status bar has no "gap" state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MarketHealthState {
    /// Venue is producing fresh ticks (last seen within threshold). This is
    /// also the initial fixture state (no watchdog running).
    #[default]
    Fresh,
    /// Venue has not produced a tick within the stale threshold. Status bar
    /// shows "Reconnecting".
    Stale,
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

// ── Phase 3 — Strategies-detail / Risk / Audit screen types ─────────────────
//
// Lumen Phase 3 (Detail screens) — Q-resolutions documented in
// `spec/features/lumen-phase-3-detail-screens.md` § Design.
//
// `StrategiesConfig` is a **UI-local read-only mirror** of the relevant
// fields from `agent::config::StrategiesConfig`. It deliberately does NOT
// re-export the agent struct so the `ui` crate's default-feature build
// (no `agent` dep) still compiles.

/// Phase 3 — read-only mirror of the agent runtime's strategies config
/// surface used by the Strategies-detail screen (Q10 — read-only). Only
/// the fields the screen renders are mirrored; the live binary's boot
/// path translates `agent::config::StrategiesConfig` into this struct.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct StrategiesConfig {
    /// Per-strategy params — keyed by `StrategyId`. Each entry's
    /// `params` map is rendered as the read-only key-value rows in the
    /// Strategies-detail params block (Phase 3 R4.2).
    pub strategies: Vec<StrategyConfigEntry>,
}

/// Phase 3 — one entry in the read-only `StrategiesConfig.strategies`
/// list. The `id` matches the `StrategyId` keying `Cockpit::strategies`
/// rows so the params block is selectable from the chip row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyConfigEntry {
    pub id: StrategyId,
    /// Repo-relative TOML source path — matches `StrategyRow::source_path`
    /// so the Strategies-detail screen can show the operator where the
    /// config lives.
    pub source_path: SmolStr,
    /// Read-only key-value params. Order is preserved for deterministic
    /// snapshot baselines.
    pub params: Vec<(SmolStr, SmolStr)>,
}

/// Phase 3 — Risk-screen mirror. Shipped by the agent runtime's
/// `RiskTelemetry` snapshot (Q3 ratification — channel pattern, sibling
/// of Phase 1 `MarketHealth`). All numeric fields are `Decimal` or `u64`;
/// no `f64`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RiskState {
    pub per_symbol_exposure: HashMap<(Venue, Symbol), Decimal>,
    pub per_symbol_caps: HashMap<(Venue, Symbol), Decimal>,
    pub daily_loss_used_pct: Decimal,
    pub daily_loss_cap_pct: Decimal,
    pub heartbeat_age_ms: u64,
    pub heartbeat_timeout_ms: u64,
}

// Phase 3 — `AuditKindFilter` is defined in `trading_core::views` so
// the audit query crate can take it as a parameter without a back-edge
// into `ui`. Re-exported below for the existing
// `ui::state::AuditKindFilter` import path.
pub use trading_core::AuditKindFilter;

/// Phase 3 — Audit-screen single-select time-range chip.
///
/// `Last7D` is the default — widest window operators are likely to
/// want; chip row makes narrowing one click.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AuditTimeRange {
    Last1H,
    Last24H,
    #[default]
    Last7D,
}

/// Phase 3 — Audit-screen filter row composite. All fields are
/// session-scoped (Q5 — no on-disk persistence). Empty venue set
/// means "all venues"; `None` symbol means "all symbols".
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AuditFilter {
    pub venues: Vec<Venue>,
    pub symbol: Option<Symbol>,
    pub kind: AuditKindFilter,
    pub time_range: AuditTimeRange,
}

impl AuditFilter {
    /// Phase 3 — chip-click helper. Replaces `venues` and returns the
    /// new filter. Used by the screen's chip-press handler to emit a
    /// fresh `AuditFilterChanged` value on every interaction.
    #[must_use]
    pub fn with_venues(&self, venues: Vec<Venue>) -> Self {
        Self {
            venues,
            symbol: self.symbol.clone(),
            kind: self.kind,
            time_range: self.time_range,
        }
    }

    /// Phase 3 — symbol-input helper. `None` means "all symbols".
    #[must_use]
    pub fn with_symbol(&self, symbol: Option<Symbol>) -> Self {
        Self {
            venues: self.venues.clone(),
            symbol,
            kind: self.kind,
            time_range: self.time_range,
        }
    }

    /// Phase 3 — kind-chip helper.
    #[must_use]
    pub fn with_kind(&self, kind: AuditKindFilter) -> Self {
        Self {
            venues: self.venues.clone(),
            symbol: self.symbol.clone(),
            kind,
            time_range: self.time_range,
        }
    }

    /// Phase 3 — time-range chip helper.
    #[must_use]
    pub fn with_time_range(&self, time_range: AuditTimeRange) -> Self {
        Self {
            venues: self.venues.clone(),
            symbol: self.symbol.clone(),
            kind: self.kind,
            time_range,
        }
    }
}

// Phase 3 — `AuditKindLabel` and `JournalRow` are defined in
// `trading_core::views` so the audit query crate can return them
// without a back-edge into `ui`. Re-exported below for the
// existing `ui::state::JournalRow` import path.
pub use trading_core::{AuditKindLabel, JournalRow};

/// Phase 3 — Audit-screen sub-state. Filter, page cursor, loaded row
/// set, and total count all live here so the screen body is a pure
/// dispatch over `&Cockpit`.
#[derive(Debug, Clone)]
pub struct AuditScreenState {
    pub filter: AuditFilter,
    /// 0-indexed page cursor (Q4 — fixed 250 rows / page).
    pub page: u32,
    pub rows: PanelState<Vec<JournalRow>>,
    pub total_count: Option<u64>,
}

impl Default for AuditScreenState {
    fn default() -> Self {
        Self {
            filter: AuditFilter::default(),
            page: 0,
            rows: PanelState::Loading,
            total_count: None,
        }
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

// ── chart-buy-sell-emphasis v1.9 — tooltip + ghost-marker types (T2006) ─────

/// Discriminates which marker the cockpit's chart tooltip is currently
/// rendering for: an executed fill (Q4 — six fields, no truncated Tx ID)
/// or a strategy-intended ghost signal (R5.6 — fewer fields, "intent"
/// badge).
///
/// Sibling of `ChartMarkerIndex` — `Fill` here corresponds to the
/// `ChartMarkerIndex::Fill` variant; `Signal` corresponds to
/// `ChartMarkerIndex::Signal`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartTooltipKind {
    /// An executed-fill marker (R4.2 six fields).
    Fill,
    /// A strategy-intended ghost-signal marker (R5.6 reduced fields).
    Signal,
}

/// Index of the currently-hovered chart marker. Emitted by
/// `ChartProgram::update` on hit-rect transitions; carried by
/// `Message::ChartMarkerHovered`.
///
/// `Fill(usize)` indexes into the active symbol's `chart_markers`
/// `Vec<FillView>`. `Signal(usize)` indexes into the active symbol's
/// `chart_signals` `Vec<SignalView>`. The cockpit binary's update wrapper
/// uses the index to build a `ChartTooltipView` and dispatch it (no extra
/// async work — pure state lookup).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChartMarkerIndex {
    /// Index into `Cockpit.chart_markers` (executed-fill layer).
    Fill(usize),
    /// Index into `Cockpit.chart_signals` (strategy-intended ghost layer).
    Signal(usize),
}

/// Read-side view-data carried by the chart hover tooltip. Built at
/// hover-message-handle time from the corresponding `FillView` or
/// `SignalView`; the tooltip widget renders the six R4.2 fields verbatim
/// (or the reduced R5.6 ghost-variant fields).
///
/// Per Q4-operator-resolved 2026-05-10: no truncated transaction ID; the
/// full UUID is one click away via `Message::TapeRowClicked(transaction_id)`
/// → existing `JournalTransactionView` modal.
#[derive(Debug, Clone, PartialEq)]
pub struct ChartTooltipView {
    /// Drives layout: Fill renders all six fields; Signal renders the
    /// reduced set + the `CHART_TOOLTIP_GHOST_BADGE` row.
    pub kind: ChartTooltipKind,
    pub side: Side,
    /// `None` for ghost signals (R5.6 — strategy intent precedes price
    /// discovery for market-priced signals).
    pub price: Option<Decimal>,
    /// Intended quantity for ghosts; executed quantity for fills.
    pub qty: Decimal,
    /// `price × qty` for fills; `None` for ghosts.
    pub notional: Option<Decimal>,
    pub ts: Timestamp,
    /// Strategy that emitted the signal / produced the fill. Rendered as
    /// `CHART_TOOLTIP_STRATEGY_NONE` when absent.
    pub strategy_id: Option<SmolStr>,
    /// `true` for ghost signals that the risk engine clamped (R5.6 —
    /// surfaces "(clamped)" suffix on the side badge). Always `false`
    /// for the Fill variant.
    pub was_clamped: bool,
    /// Short clamp reason — rendered under the side row when present
    /// (`"per_symbol_cap"`, `"daily_loss_cap"`, etc.).
    pub clamp_reason: Option<SmolStr>,
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
    /// Live fills panel state. **Module renamed `tape.rs` → `agent_feed.rs`
    /// (Phase 5 R11). Field name preserved per Phase 5 Q14 — renaming
    /// the field would ripple through ~100+ test sites for cosmetic
    /// value. See `widgets::agent_feed` module doc-comment for the
    /// rename rationale.**
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

    // ── T1508 — Status bar state ─────────────────────────────────────────────
    /// Per-venue market-health state, updated by `Message::MarketHealthUpdated`.
    /// Empty on boot (no venues seen yet); fixture bin pre-populates with
    /// `Fresh` for all known venues.
    pub market_health: HashMap<Venue, MarketHealthState>,

    /// Most recently ticked server time. Updated 1 Hz by a `time::every`
    /// subscription via `Message::ServerTimeTick`. `None` until first tick.
    pub server_time_now: Option<Timestamp>,

    /// Account label displayed in the status bar.  Populated at boot from
    /// `Config` in the live path; set to the static fixture string
    /// `"Paper · Demo 3-symbol"` in fixtures mode. Static for the session.
    pub account_label: SmolStr,

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

    // ── Phase 2 — Shell IA + Charts ─────────────────────────────────────────
    /// Active screen for the routed shell. Default `Home` so cold-start
    /// lands the operator on trading data, not operations chrome.
    /// (Phase 2 Q8 — session-scoped, no on-disk persistence.)
    pub current_screen: Screen,

    /// Configured `(Venue, Symbol)` universe — populated once at boot
    /// from `agent::config::Config` in live mode, hard-coded to the
    /// 3-symbol Binance set in fixtures mode. Static for the session.
    /// (Phase 2 Q3.)
    pub universe: Vec<(Venue, Symbol)>,

    /// Currently-selected `(Venue, Symbol)` on the Charts screen.
    /// `None` until the operator first enters Charts; auto-set to the
    /// first universe entry on first paint of Charts (R6.5). Persists
    /// across Home ↔ Debug ↔ Charts switches; cleared only on cockpit
    /// restart (Q8).
    pub selected_symbol: Option<(Venue, Symbol)>,

    /// Per-`(Venue, Symbol)` rolling 60-bar buffer fed by the existing
    /// `Message::BarReceived` arm.
    pub chart_buffer: ChartBuffer,

    /// Marker layer for the Charts screen — fills filtered to the active
    /// `(venue, symbol, window)` triple. `Loading` until the first async
    /// fetch returns; `Ready(fills)` after; `Error(msg)` on query failure.
    pub chart_markers: PanelState<Vec<FillView>>,

    /// Ghost-marker layer for the Charts screen — strategy-intended
    /// signals filtered to the active `(venue, symbol, window)` triple
    /// (R5.4, M3 — T2018, chart-buy-sell-emphasis v1.9). Sibling of
    /// `chart_markers`; fed by `audit::query::recent_signals` via the
    /// `cockpit_live` `Task::perform` shim on `SelectSymbol` and after
    /// `BarClose` for the active symbol. `Loading` until the first
    /// async fetch returns; `Ready(signals)` (possibly empty) after.
    pub chart_signals: PanelState<Vec<SignalView>>,

    /// Currently-rendered hover tooltip for the chart canvas — `None` when
    /// the cursor is not over any marker (R4.1). Pure-state output of the
    /// `ChartMarkerHovered` / `ChartMarkerHoverEnded` message arms — the
    /// chart widget's `canvas::Program::update` impl drives the messages;
    /// `state::update` does the assignment.
    pub chart_tooltip: Option<ChartTooltipView>,

    // ── Phase 3 — Detail screens ────────────────────────────────────────
    /// Currently-selected strategy on the Strategies-detail screen.
    /// Set by `Message::SelectStrategy` (chip click on the Strategies
    /// screen, or row click on the Home → Strategies-summary panel
    /// followed by `SwitchScreen(Screen::Strategies)` — Q11b compound
    /// dispatch). Reset to `None` only on cockpit restart (Q5 —
    /// session-scoped).
    pub selected_strategy: Option<StrategyId>,

    /// Read-only mirror of the agent runtime's strategies config,
    /// populated once at boot in both bins (live: from
    /// `agent::config::Config.strategies`; fixtures: from a
    /// `fake_strategies_config()` helper). Static for the session — same
    /// precedent as `Cockpit::universe` (Phase 2 Q3) and
    /// `Cockpit::account_label` (Phase 1 R13.4). `None` if the binary
    /// boots before config loads; the screen renders the empty-state
    /// until populated.
    pub strategies_config: Option<StrategiesConfig>,

    /// Live-mirrored risk state. Populated by the new bus subscription on
    /// `RiskTelemetry` events (Q3 — channel pattern; mirrors Phase 1
    /// `MarketHealth`). `Loading` on cold-start until the first
    /// `RiskStateRefreshed` arm fires.
    pub risk_state: PanelState<RiskState>,

    /// Audit-screen sub-state. Filter, page cursor, loaded row set, and
    /// total count all live here so the screen body is a pure dispatch
    /// over `&Cockpit`.
    pub audit_screen_state: AuditScreenState,

    // ── Phase 4 — Backtest-panel cross-link ─────────────────────────────
    /// Read-only mirror of `audit::query::equity_curve_for_strategy`
    /// results, keyed on `StrategyId`. Entry inserted at first
    /// `SelectStrategy(id)`; replaced on subsequent re-selects of the
    /// same id (one-shot semantics — operator switching screens triggers
    /// a fresh fetch). Cleared only on cockpit restart (session-scoped
    /// per Phase 3 Q5).
    pub strategy_equity: HashMap<StrategyId, PanelState<EquitySeries>>,

    // ── Phase 5 — HumanControl panel + per-strategy pause + override ───
    /// Operator-selected execution mode (Q4 — runtime-only). Cold-start
    /// = `Observe`; restart returns to `Observe`; no `config/agent.toml`
    /// write; no audit writer.
    pub execution_mode: ExecutionMode,
    /// Per-strategy pause set (R4.3). Sibling of `tape_paused: bool`;
    /// single-click toggles membership.
    pub paused_strategies: HashSet<StrategyId>,
    /// Typed-confirm state for the per-veto override modal (R7).
    pub override_risk_veto: OverrideRiskVetoState,
    /// Surfaced risk-engine veto events (R7.2). Live upstream is the
    /// `default_risk_telemetry_stub` at
    /// `crates/agent/src/runtime.rs:1023–1090`; real wiring tracked as
    /// TD-2 (Phase 5 Design / Risk-engine veto-emit deferral).
    pub risk_veto_events: Vec<VetoEvent>,
    /// Phase 5 (T1912 / TD-1 path b) — currently keyboard-focused widget.
    /// Owned by the `widgets::focus_ring` Subscription wrapper. `None`
    /// when nothing is focused (cold-start; mouse-only interaction).
    pub focused_widget: Option<SmolStr>,
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
            .field("market_health", &self.market_health)
            .field("server_time_now", &self.server_time_now)
            .field("account_label", &self.account_label)
            .field("last_bar_ts", &self.last_bar_ts)
            .field("last_tick_ts", &self.last_tick_ts)
            .field("tape_audit_modal", &self.tape_audit_modal);
        #[cfg(feature = "live")]
        dbg.field(
            "kill_switch",
            &self.kill_switch.as_ref().map(|_| "<trip-fn>"),
        );
        dbg.field("current_screen", &self.current_screen)
            .field("universe", &self.universe)
            .field("selected_symbol", &self.selected_symbol)
            .field("chart_buffer", &self.chart_buffer)
            .field("chart_markers", &self.chart_markers)
            .field("chart_signals", &self.chart_signals)
            .field("chart_tooltip", &self.chart_tooltip)
            .field("selected_strategy", &self.selected_strategy)
            .field("strategies_config", &self.strategies_config)
            .field("risk_state", &self.risk_state)
            .field("audit_screen_state", &self.audit_screen_state)
            .field("strategy_equity", &self.strategy_equity)
            .field("execution_mode", &self.execution_mode)
            .field("paused_strategies", &self.paused_strategies)
            .field("override_risk_veto", &self.override_risk_veto)
            .field("risk_veto_events", &self.risk_veto_events)
            .field("focused_widget", &self.focused_widget);
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
            market_health: HashMap::new(),
            server_time_now: None,
            account_label: SmolStr::new(""),
            last_bar_ts: None,
            last_tick_ts: None,
            tape_audit_modal: None,
            #[cfg(feature = "live")]
            kill_switch: None,
            current_screen: Screen::default(),
            universe: Vec::new(),
            selected_symbol: None,
            chart_buffer: ChartBuffer::default(),
            chart_markers: PanelState::Loading,
            chart_signals: PanelState::Loading,
            chart_tooltip: None,
            selected_strategy: None,
            strategies_config: None,
            risk_state: PanelState::Loading,
            audit_screen_state: AuditScreenState::default(),
            strategy_equity: HashMap::new(),
            execution_mode: ExecutionMode::default(),
            paused_strategies: HashSet::new(),
            override_risk_veto: OverrideRiskVetoState::default(),
            risk_veto_events: Vec::new(),
            focused_widget: None,
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
            market_health: HashMap::new(),
            server_time_now: None,
            account_label: SmolStr::new(""),
            last_bar_ts: None,
            last_tick_ts: None,
            tape_audit_modal: None,
            #[cfg(feature = "live")]
            kill_switch: None,
            current_screen: Screen::default(),
            universe: Vec::new(),
            selected_symbol: None,
            chart_buffer: ChartBuffer::default(),
            chart_markers: PanelState::Loading,
            chart_signals: PanelState::Loading,
            chart_tooltip: None,
            selected_strategy: None,
            strategies_config: None,
            risk_state: PanelState::Loading,
            audit_screen_state: AuditScreenState::default(),
            strategy_equity: HashMap::new(),
            execution_mode: ExecutionMode::default(),
            paused_strategies: HashSet::new(),
            override_risk_veto: OverrideRiskVetoState::default(),
            risk_veto_events: Vec::new(),
            focused_widget: None,
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

    // ── T1508 — Status bar messages ──────────────────────────────────────────
    /// Market-health event from the watchdog bus channel. Updates
    /// `Cockpit::market_health` for the venue carried in the event.
    MarketHealthUpdated(MarketHealth),

    /// 1 Hz server-time tick from a `time::every` iced subscription.
    /// Updates `Cockpit::server_time_now` so the status bar always shows
    /// a fresh "Server … UTC" timestamp without re-rendering everything.
    ServerTimeTick(Timestamp),

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

    // ── Phase 2 — Shell IA + Charts ─────────────────────────────────────────
    /// Sidebar-nav row click. Pure assignment; no side effects.
    SwitchScreen(Screen),

    /// Symbol-selector chip click. Sets `selected_symbol` and resets the
    /// marker panel to `Loading`; the binary's `Task::perform` shim then
    /// dispatches the marker re-fetch (R8.3). Pure-function `update`
    /// discipline preserved — async work lives in the binary.
    SelectSymbol(Venue, Symbol),

    /// Async result of the `recent_fills_filtered` fetch issued after
    /// `SelectSymbol` or after `BarClose` for the active symbol.
    /// `Ok(fills)` flips `chart_markers` to `Ready(fills)`; `Err(msg)`
    /// flips it to `Error(msg)`.
    ChartMarkersLoaded(Result<Vec<FillView>, SmolStr>),

    // ── chart-buy-sell-emphasis v1.9 — ghost-signal layer + tooltip ────────
    /// Async result of the `audit::query::recent_signals` fetch issued
    /// after `SelectSymbol` or after `BarClose` for the active symbol
    /// (R5.4). Sibling of `ChartMarkersLoaded`. `Ok(signals)` flips
    /// `chart_signals` to `Ready(signals)`; `Err(msg)` flips to
    /// `Error(msg)`.
    ChartSignalsLoaded(Result<Vec<SignalView>, SmolStr>),
    /// Cursor entered (or moved into) the hit-rect of a chart marker. The
    /// chart canvas program emits this via custom pointer-tracking; the
    /// pure-state arm assigns `chart_tooltip = Some(view)` from the index
    /// (R4.1, Q3).
    ChartMarkerHovered(ChartMarkerIndex),
    /// Cursor left every marker's hit-rect. Clears `chart_tooltip`.
    ChartMarkerHoverEnded,

    // ── Phase 3 — Detail screens ────────────────────────────────────────
    /// Strategies-detail chip click OR Home → Strategies-summary row
    /// click. Pure assignment; `selected_strategy = Some(id)`. The Home-
    /// row variant follows up with `Message::SwitchScreen(
    /// Screen::Strategies)` via the binary's `Task::done` chain (Q11b
    /// compound dispatch — no new `OpenStrategy` variant).
    SelectStrategy(StrategyId),

    /// Risk telemetry refresh from the new agent-runtime channel
    /// (Q3 ratification). Pure assignment; `risk_state = Ready(state)`.
    /// `Subscription::batch` recipe in `crates/ui/src/live.rs` maps
    /// incoming `RiskTelemetry` bus events to this variant.
    RiskStateRefreshed(RiskState),

    /// Audit filter chip / input changed. Pure: resets `page` to 0,
    /// flips `rows` to `Loading`. The binary's `Task::perform` shim
    /// dispatches the `recent_journal_filtered` re-fetch.
    AuditFilterChanged(AuditFilter),

    /// Audit pagination Prev / Next. Pure: increments / decrements
    /// `page`, flips `rows` to `Loading`. Binary dispatches re-fetch.
    AuditPageChanged(u32),

    /// Async result of `recent_journal_filtered`. `Ok((rows, total))`
    /// → `rows = Ready(rows); total_count = Some(total)`; `Err(msg)`
    /// → `rows = Error(msg); total_count = None`.
    AuditRowsLoaded(Result<(Vec<JournalRow>, u64), SmolStr>),

    // ── Phase 4 — Strategies sparkline cross-link ───────────────────────
    /// Async result of `audit::query::equity_curve_for_strategy`.
    /// `Ok(series)` → `strategy_equity.insert(id, Ready(series))`;
    /// `Err(msg)` → `strategy_equity.insert(id, Error(msg))`. Pure
    /// assignment — async work lives in the binary's `Task::perform`
    /// shim. The series has already been
    /// `downsample(SPARKLINE_POINT_CAP)`-d before landing here (Q9).
    StrategyEquityRefreshed(StrategyId, Result<EquitySeries, SmolStr>),

    // ── Phase 5 — HumanControl panel ───────────────────────────────────
    /// Operator clicked one of the three execution-mode segments.
    /// Pure assignment to `Cockpit::execution_mode`; live mode also
    /// emits on `execution_mode_tx` (R10.3).
    ExecutionModeSelected(ExecutionMode),

    // ── Phase 5 — Pause-strategy ───────────────────────────────────────
    /// Operator clicked the per-row pause/resume button (R4.3 / Q8 —
    /// single-click both directions). Pure update flips set membership;
    /// live mode also emits on `pause_strategy_tx` (R4.6) and spawns
    /// the audit-writer task (R5.5).
    StrategyPauseToggled(StrategyId),

    // ── Phase 5 — Override-risk-veto (kill-confirm modal mirror) ───────
    /// Operator pressed the `Override` button on a surfaced veto event.
    /// Opens the typed-confirm modal in `Confirming { veto_id, typed: "" }`.
    OverrideRiskVetoPressed(SmolStr),
    /// Operator typed into the OVERRIDE input. Pure update.
    OverrideRiskVetoTyped(String),
    /// Operator pressed cancel on the modal. Returns to `Idle`.
    OverrideRiskVetoCancelled,
    /// Operator pressed confirm with the phrase matched. Spawns the
    /// audit-writer task (R8.5) + clears the matching `VetoEvent`.
    OverrideRiskVetoConfirmed(SmolStr),

    // ── Phase 5 — TD-1 path b — focus ring (T1912) ─────────────────────
    /// Synthetic focus-traversal message emitted by
    /// `widgets::focus_ring::subscription` on Tab / Arrow keypress.
    /// Pure assignment to `Cockpit::focused_widget`.
    FocusChanged(SmolStr),
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
            // Phase 2 R10.4 — push the bar into the rolling chart buffer.
            // Pure mutation on `Cockpit`; no async work, no bus event emitted.
            model.chart_buffer.push_bar(bar);
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

        // ── T1508 status-bar messages ────────────────────────────────────────
        Message::MarketHealthUpdated(health) => {
            // Translate the three-variant bus enum to the two-state UI enum:
            // Fresh + Recovered → MarketHealthState::Fresh;  Stale → Stale.
            let (venue, new_state) = match health {
                MarketHealth::Fresh { venue, .. } | MarketHealth::Recovered { venue, .. } => {
                    (venue, MarketHealthState::Fresh)
                }
                MarketHealth::Stale { venue, .. } => (venue, MarketHealthState::Stale),
            };
            model.market_health.insert(venue, new_state);
        }
        Message::ServerTimeTick(ts) => {
            model.server_time_now = Some(ts);
        }

        // ── Phase 2 — Shell IA + Charts ─────────────────────────────────────
        Message::SwitchScreen(s) => {
            model.current_screen = s;
        }
        Message::SelectSymbol(venue, symbol) => {
            model.selected_symbol = Some((venue, symbol));
            model.chart_markers = PanelState::Loading;
            model.chart_signals = PanelState::Loading;
            // Clear any stale tooltip from the previous symbol so a hover
            // on an empty canvas doesn't surface a tooltip referencing a
            // fill the operator can no longer see.
            model.chart_tooltip = None;
        }
        Message::ChartMarkersLoaded(result) => {
            model.chart_markers = match result {
                Ok(fills) => PanelState::Ready(fills),
                Err(msg) => PanelState::Error(msg),
            };
        }
        Message::ChartSignalsLoaded(result) => {
            model.chart_signals = match result {
                Ok(signals) => PanelState::Ready(signals),
                Err(msg) => PanelState::Error(msg),
            };
        }
        Message::ChartMarkerHovered(idx) => {
            // Build tooltip view from index against the currently-Ready
            // marker / signal slices. Out-of-range indices clear the
            // tooltip — the canvas could publish a stale index across an
            // async refresh boundary; defence-in-depth.
            model.chart_tooltip = build_tooltip_view(model, idx);
        }
        Message::ChartMarkerHoverEnded => {
            model.chart_tooltip = None;
        }

        // ── Phase 3 — Detail screens ────────────────────────────────────
        Message::SelectStrategy(id) => {
            model.selected_strategy = Some(id);
        }
        Message::RiskStateRefreshed(state) => {
            model.risk_state = PanelState::Ready(state);
        }
        Message::AuditFilterChanged(filter) => {
            model.audit_screen_state.filter = filter;
            model.audit_screen_state.page = 0;
            model.audit_screen_state.rows = PanelState::Loading;
        }
        Message::AuditPageChanged(page) => {
            model.audit_screen_state.page = page;
            model.audit_screen_state.rows = PanelState::Loading;
        }
        Message::AuditRowsLoaded(Ok((rows, total))) => {
            model.audit_screen_state.rows = PanelState::Ready(rows);
            model.audit_screen_state.total_count = Some(total);
        }
        Message::AuditRowsLoaded(Err(msg)) => {
            model.audit_screen_state.rows = PanelState::Error(msg);
            model.audit_screen_state.total_count = None;
        }

        // ── Phase 4 — Strategies sparkline cross-link ───────────────────
        Message::StrategyEquityRefreshed(id, Ok(series)) => {
            model.strategy_equity.insert(id, PanelState::Ready(series));
        }
        Message::StrategyEquityRefreshed(id, Err(msg)) => {
            model.strategy_equity.insert(id, PanelState::Error(msg));
        }

        // ── Phase 5 — HumanControl panel ────────────────────────────────
        Message::ExecutionModeSelected(mode) => {
            model.execution_mode = mode;
        }
        Message::StrategyPauseToggled(id) => {
            // Set-membership flip — the pure-update arm for both directions.
            // Live mode's binary-side closure spawns the audit writer + bus
            // emit after this arm runs (R4.6 / R5.5).
            if !model.paused_strategies.remove(&id) {
                model.paused_strategies.insert(id);
            }
        }

        // ── Phase 5 — Override-risk-veto modal ──────────────────────────
        Message::OverrideRiskVetoPressed(veto_id) => {
            model.override_risk_veto = OverrideRiskVetoState::Confirming {
                veto_id,
                typed: String::new(),
            };
        }
        Message::OverrideRiskVetoTyped(s) => {
            if let OverrideRiskVetoState::Confirming { typed, .. } = &mut model.override_risk_veto {
                *typed = s;
            }
        }
        Message::OverrideRiskVetoCancelled => {
            model.override_risk_veto = OverrideRiskVetoState::Idle;
        }
        Message::OverrideRiskVetoConfirmed(veto_id) => {
            // Pure-update arm clears the matching VetoEvent so the visual
            // state reflects immediately. The binary's wrapping update arm
            // then spawns audit::journal::risk_veto_overridden(...) under
            // #[cfg(feature = "live")] (R8.5). Forward-only per Q9 — the
            // agent does NOT re-emit the blocked signal.
            model.risk_veto_events.retain(|v| v.veto_id != veto_id);
            model.override_risk_veto = OverrideRiskVetoState::Idle;
        }

        // ── Phase 5 — TD-1 path b — focus ring (T1912) ──────────────────
        Message::FocusChanged(id) => {
            model.focused_widget = Some(id);
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

/// Build a `ChartTooltipView` from a hovered-marker index against the
/// currently-Ready marker / signal slices. Returns `None` for stale or
/// out-of-range indices — defence-in-depth across the async refresh
/// boundary (the canvas could publish a hover for an index that has since
/// been replaced by a `Loading` panel-state).
fn build_tooltip_view(model: &Cockpit, idx: ChartMarkerIndex) -> Option<ChartTooltipView> {
    match idx {
        ChartMarkerIndex::Fill(i) => {
            let fills = match &model.chart_markers {
                PanelState::Ready(v) => v,
                _ => return None,
            };
            let fill = fills.get(i)?;
            let strategy_id = lookup_strategy_for_fill(model, fill);
            Some(ChartTooltipView {
                kind: ChartTooltipKind::Fill,
                side: fill.side,
                price: Some(fill.price.get()),
                qty: fill.qty.get(),
                notional: Some(fill.price.get().saturating_mul(fill.qty.get())),
                ts: fill.venue_ts,
                strategy_id,
                was_clamped: false,
                clamp_reason: None,
            })
        }
        ChartMarkerIndex::Signal(i) => {
            let signals = match &model.chart_signals {
                PanelState::Ready(v) => v,
                _ => return None,
            };
            let signal = signals.get(i)?;
            Some(ChartTooltipView {
                kind: ChartTooltipKind::Signal,
                side: signal.side,
                price: None,
                qty: signal.intended_qty.get(),
                notional: None,
                ts: signal.signal_ts,
                strategy_id: Some(signal.strategy_id.0.clone()),
                was_clamped: signal.was_clamped,
                clamp_reason: signal.clamp_reason.clone(),
            })
        }
    }
}

/// Best-effort strategy lookup for a fill — Phase 5 onward `FillView`
/// doesn't carry a `strategy_id`, so the cockpit relies on the recent
/// strategy events list to attribute. Returns `None` when no attribution
/// is available (R4.7 — tooltip then renders the `CHART_TOOLTIP_STRATEGY_NONE`
/// dash placeholder).
fn lookup_strategy_for_fill(_model: &Cockpit, _fill: &FillView) -> Option<SmolStr> {
    // Future enrichment: cross-reference `model.strategies_recent_events`
    // by tx-id / venue+ts. For v1.9 we leave attribution to the
    // journal-transaction modal (one click away via R4.5 click-through);
    // the tooltip simply renders "—" when nothing is plumbed.
    None
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

    // ── Phase 2 — Shell IA + Charts (T1601) ─────────────────────────────────

    fn fixed_bar(symbol: &str, venue: Venue, offset_min: i64) -> Bar {
        use rust_decimal::Decimal;
        use trading_core::{Price, Quantity, Timeframe};
        let dt = time::OffsetDateTime::from_unix_timestamp(1_705_320_000 + offset_min * 60)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        let close_dt =
            time::OffsetDateTime::from_unix_timestamp(1_705_320_000 + offset_min * 60 + 59)
                .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        let open_ts = Timestamp::new(dt);
        let close_ts = Timestamp::new(close_dt);
        let p = |d: Decimal| Price::new(d).unwrap_or_else(|_| unreachable!());
        Bar {
            symbol: Symbol::new(symbol),
            tf: Timeframe::OneMinute,
            open_ts,
            close_ts,
            open: p(dec!(40_000)),
            high: p(dec!(40_100)),
            low: p(dec!(39_900)),
            close: p(dec!(40_050)),
            volume: Quantity::new(dec!(12.5)).unwrap_or_else(|_| unreachable!()),
            trade_count: 100,
            local_recv_ts: close_ts,
            venue,
        }
    }

    /// T1601 — `Message::SwitchScreen` mutates only `current_screen`.
    /// All other fields stay byte-identical (compared via `Debug`-format).
    #[test]
    fn switch_screen_is_pure() {
        for target in [
            Screen::Home,
            Screen::Debug,
            Screen::Charts,
            Screen::Strategies,
            Screen::Risk,
            Screen::Audit,
        ] {
            let baseline = Cockpit::new();
            let mut after = baseline.clone();
            update(&mut after, Message::SwitchScreen(target));
            assert_eq!(after.current_screen, target);
            // Force every other field to be byte-equal by overwriting the
            // mutated field on `after` and comparing the Debug rendering.
            let mut restored = after.clone();
            restored.current_screen = baseline.current_screen;
            assert_eq!(format!("{baseline:?}"), format!("{restored:?}"));
        }
    }

    /// T1601 — `chart_buffer` enforces 60-bar cap with FIFO eviction.
    #[test]
    fn chart_buffer_evicts_at_capacity() {
        let mut c = Cockpit::new();
        // Push 61 bars; only the most recent 60 should remain.
        for i in 0..61 {
            update(
                &mut c,
                Message::BarReceived(fixed_bar("BTCUSDT", Venue::Binance, i)),
            );
        }
        let bars: Vec<&Bar> = c
            .chart_buffer
            .bars(Venue::Binance, &Symbol::new("BTCUSDT"))
            .collect();
        assert_eq!(bars.len(), CHART_BUFFER_CAPACITY);
        // Oldest in the deque is offset 1 (offset 0 was evicted).
        assert_eq!(bars[0].open_ts, fixed_ts_min(1));
        // Newest is offset 60.
        assert_eq!(bars[bars.len() - 1].open_ts, fixed_ts_min(60));
    }

    /// T1601 — distinct `(Venue, Symbol)` keys carry disjoint deques.
    #[test]
    fn chart_buffer_keys_distinct_per_pair() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::BarReceived(fixed_bar("BTCUSDT", Venue::Binance, 0)),
        );
        update(
            &mut c,
            Message::BarReceived(fixed_bar("ETHUSDT", Venue::Binance, 0)),
        );
        update(
            &mut c,
            Message::BarReceived(fixed_bar("BTCUSDT", Venue::Coinbase, 0)),
        );
        assert_eq!(
            c.chart_buffer
                .bars(Venue::Binance, &Symbol::new("BTCUSDT"))
                .count(),
            1
        );
        assert_eq!(
            c.chart_buffer
                .bars(Venue::Binance, &Symbol::new("ETHUSDT"))
                .count(),
            1
        );
        assert_eq!(
            c.chart_buffer
                .bars(Venue::Coinbase, &Symbol::new("BTCUSDT"))
                .count(),
            1
        );
        // A symbol/venue pair never written shows zero bars.
        assert_eq!(
            c.chart_buffer
                .bars(Venue::Kraken, &Symbol::new("SOLUSDT"))
                .count(),
            0
        );
    }

    // ── Phase 3 — Detail screens (T1701) ────────────────────────────────────

    /// T1701 — `Message::SelectStrategy` survives `SwitchScreen` round-trips.
    #[test]
    fn select_strategy_persists_across_screen_switch() {
        let mut c = Cockpit::new();
        let id = StrategyId::new("btc_macd_trend");
        update(&mut c, Message::SelectStrategy(id.clone()));
        assert_eq!(c.selected_strategy.as_ref(), Some(&id));
        update(&mut c, Message::SwitchScreen(Screen::Home));
        update(&mut c, Message::SwitchScreen(Screen::Risk));
        update(&mut c, Message::SwitchScreen(Screen::Strategies));
        assert_eq!(
            c.selected_strategy.as_ref(),
            Some(&id),
            "selected_strategy must survive screen switches",
        );
    }

    /// T1701 — `RiskStateRefreshed` flips `risk_state` from Loading to Ready.
    #[test]
    fn risk_state_refresh_replaces_panel_state() {
        let mut c = Cockpit::new();
        assert_eq!(c.risk_state.variant_name(), "loading");
        let snap = RiskState {
            per_symbol_exposure: HashMap::new(),
            per_symbol_caps: HashMap::new(),
            daily_loss_used_pct: dec!(2.5),
            daily_loss_cap_pct: dec!(5.0),
            heartbeat_age_ms: 1_000,
            heartbeat_timeout_ms: 5_000,
        };
        update(&mut c, Message::RiskStateRefreshed(snap));
        assert_eq!(c.risk_state.variant_name(), "ready");
    }

    /// T1701 — `AuditFilterChanged` resets page → 0 + rows → Loading.
    #[test]
    fn audit_filter_changed_resets_page() {
        let mut c = Cockpit::new();
        // Pre-condition: bump page off zero so the reset is observable.
        c.audit_screen_state.page = 7;
        c.audit_screen_state.rows = PanelState::Ready(Vec::new());
        let new_filter = AuditFilter::default().with_kind(AuditKindFilter::Fill);
        update(&mut c, Message::AuditFilterChanged(new_filter.clone()));
        assert_eq!(c.audit_screen_state.page, 0, "page must reset to 0");
        assert_eq!(
            c.audit_screen_state.rows.variant_name(),
            "loading",
            "rows must flip to Loading"
        );
        assert_eq!(c.audit_screen_state.filter, new_filter);
    }

    /// T1701 — `AuditPageChanged` marks rows → Loading and applies the new
    /// page cursor.
    #[test]
    fn audit_page_changed_marks_rows_loading() {
        let mut c = Cockpit::new();
        c.audit_screen_state.rows = PanelState::Ready(Vec::new());
        update(&mut c, Message::AuditPageChanged(2));
        assert_eq!(c.audit_screen_state.page, 2);
        assert_eq!(c.audit_screen_state.rows.variant_name(), "loading");
    }

    /// T1701 — `AuditRowsLoaded(Ok((rows, total)))` sets Ready + `total_count`.
    #[test]
    fn audit_rows_loaded_ok_sets_ready_and_total_count() {
        let mut c = Cockpit::new();
        let rows: Vec<JournalRow> = Vec::new();
        update(&mut c, Message::AuditRowsLoaded(Ok((rows, 42))));
        assert_eq!(c.audit_screen_state.rows.variant_name(), "ready");
        assert_eq!(c.audit_screen_state.total_count, Some(42));
    }

    /// T1601 — `selected_symbol` survives `SwitchScreen` round-trips.
    #[test]
    fn select_symbol_persists_across_screen_switch() {
        let mut c = Cockpit::new();
        let pair = (Venue::Binance, Symbol::new("BTCUSDT"));
        update(&mut c, Message::SelectSymbol(pair.0, pair.1.clone()));
        assert_eq!(c.selected_symbol.as_ref(), Some(&pair));
        update(&mut c, Message::SwitchScreen(Screen::Home));
        update(&mut c, Message::SwitchScreen(Screen::Debug));
        update(&mut c, Message::SwitchScreen(Screen::Charts));
        assert_eq!(
            c.selected_symbol.as_ref(),
            Some(&pair),
            "selected_symbol must survive screen switches",
        );
    }

    fn fixed_ts_min(offset_min: i64) -> Timestamp {
        let dt = time::OffsetDateTime::from_unix_timestamp(1_705_320_000 + offset_min * 60)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
        Timestamp::new(dt)
    }

    // ── Phase 4 (T1801) — strategy_equity arm tests ─────────────────────
    fn fixture_equity_series() -> EquitySeries {
        let pts = vec![
            (
                fixed_ts_min(0),
                trading_core::Money::from_decimal(dec!(100)),
            ),
            (
                fixed_ts_min(1),
                trading_core::Money::from_decimal(dec!(110)),
            ),
        ];
        EquitySeries::from_points(pts).unwrap()
    }

    #[test]
    fn strategy_equity_refresh_inserts_ready_panel_state() {
        let mut c = Cockpit::new();
        let id = StrategyId::new("alpha");
        let series = fixture_equity_series();
        update(
            &mut c,
            Message::StrategyEquityRefreshed(id.clone(), Ok(series)),
        );
        assert!(c.strategy_equity.contains_key(&id));
        assert_eq!(
            c.strategy_equity.get(&id).map(PanelState::variant_name),
            Some("ready"),
        );
    }

    #[test]
    fn strategy_equity_refresh_err_inserts_error_panel_state() {
        let mut c = Cockpit::new();
        let id = StrategyId::new("alpha");
        update(
            &mut c,
            Message::StrategyEquityRefreshed(id.clone(), Err(SmolStr::new("boom"))),
        );
        assert_eq!(
            c.strategy_equity.get(&id).map(PanelState::variant_name),
            Some("error"),
        );
    }

    // ── Phase 5 — HumanControl + Override + Pause + Focus ring tests ───

    #[test]
    fn execution_mode_selected_assigns_field() {
        let mut c = Cockpit::new();
        assert_eq!(c.execution_mode, ExecutionMode::Observe);
        update(
            &mut c,
            Message::ExecutionModeSelected(ExecutionMode::Supervised),
        );
        assert_eq!(c.execution_mode, ExecutionMode::Supervised);
        update(&mut c, Message::ExecutionModeSelected(ExecutionMode::Auto));
        assert_eq!(c.execution_mode, ExecutionMode::Auto);
        update(
            &mut c,
            Message::ExecutionModeSelected(ExecutionMode::Observe),
        );
        assert_eq!(c.execution_mode, ExecutionMode::Observe);
    }

    #[test]
    fn strategy_pause_toggled_inserts_then_removes() {
        let mut c = Cockpit::new();
        let id = StrategyId::new("alpha");
        assert!(c.paused_strategies.is_empty());
        update(&mut c, Message::StrategyPauseToggled(id.clone()));
        assert!(c.paused_strategies.contains(&id));
        update(&mut c, Message::StrategyPauseToggled(id.clone()));
        assert!(!c.paused_strategies.contains(&id));
    }

    #[test]
    fn override_risk_veto_pressed_opens_confirming() {
        let mut c = Cockpit::new();
        assert!(matches!(c.override_risk_veto, OverrideRiskVetoState::Idle));
        update(
            &mut c,
            Message::OverrideRiskVetoPressed(SmolStr::new("veto-1")),
        );
        match &c.override_risk_veto {
            OverrideRiskVetoState::Confirming { veto_id, typed } => {
                assert_eq!(veto_id.as_str(), "veto-1");
                assert!(typed.is_empty());
            }
            other => panic!("expected Confirming, got {other:?}"),
        }
    }

    #[test]
    fn override_risk_veto_typed_updates_buffer() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::OverrideRiskVetoPressed(SmolStr::new("veto-1")),
        );
        update(&mut c, Message::OverrideRiskVetoTyped("OVERR".to_string()));
        match &c.override_risk_veto {
            OverrideRiskVetoState::Confirming { typed, .. } => {
                assert_eq!(typed, "OVERR");
            }
            other => panic!("expected Confirming, got {other:?}"),
        }
    }

    #[test]
    fn override_risk_veto_cancelled_returns_to_idle() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::OverrideRiskVetoPressed(SmolStr::new("veto-1")),
        );
        update(&mut c, Message::OverrideRiskVetoCancelled);
        assert!(matches!(c.override_risk_veto, OverrideRiskVetoState::Idle));
    }

    fn dummy_blocked_signal(strategy: &str, sym: &str) -> Signal {
        use trading_core::SignalEvidence;
        use trading_core::SignalKind;
        Signal {
            strategy_id: StrategyId::new(strategy),
            symbol: trading_core::Symbol::new(sym),
            ts: Timestamp::now(),
            kind: SignalKind::Hold,
            evidence: SignalEvidence::empty(),
            pair_data: None,
        }
    }

    #[test]
    fn override_risk_veto_confirmed_clears_event_and_returns_to_idle() {
        let mut c = Cockpit::new();
        // Seed two veto events; confirming one of them clears just that
        // event and leaves the other in place.
        let veto_id = SmolStr::new("veto-1");
        let other_id = SmolStr::new("veto-2");
        c.risk_veto_events.push(VetoEvent {
            veto_id: veto_id.clone(),
            ts: Timestamp::now(),
            strategy_id: StrategyId::new("alpha"),
            reason: SmolStr::new("daily_loss_cap"),
            blocked_signal: dummy_blocked_signal("alpha", "BTCUSDT"),
        });
        c.risk_veto_events.push(VetoEvent {
            veto_id: other_id.clone(),
            ts: Timestamp::now(),
            strategy_id: StrategyId::new("beta"),
            reason: SmolStr::new("per_symbol_cap"),
            blocked_signal: dummy_blocked_signal("beta", "ETHUSDT"),
        });

        update(&mut c, Message::OverrideRiskVetoPressed(veto_id.clone()));
        update(&mut c, Message::OverrideRiskVetoConfirmed(veto_id.clone()));
        assert!(matches!(c.override_risk_veto, OverrideRiskVetoState::Idle));
        assert_eq!(c.risk_veto_events.len(), 1);
        assert_eq!(c.risk_veto_events[0].veto_id, other_id);
    }

    #[test]
    fn focus_changed_assigns_focused_widget() {
        let mut c = Cockpit::new();
        assert!(c.focused_widget.is_none());
        update(&mut c, Message::FocusChanged(SmolStr::new("kill_button")));
        assert_eq!(c.focused_widget.as_deref(), Some("kill_button"));
    }
}
