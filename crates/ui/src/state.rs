//! Cockpit state model and message enum.
//!
//! One `Cockpit` struct, one `Message` enum. No business logic lives here —
//! only presentation state. Data comes in via feed messages and ledger
//! refresh callbacks; `update` is pure.

use std::cell::Cell;
use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use smol_str::SmolStr;
use trading_core::{
    BacktestMetrics, Bar, EquitySeries, FillView, JournalEntry, MarketHealth, Money, PnlSnapshot,
    PositionView, Side, Signal, SignalView, StrategyEventKind, StrategyEventView, StrategyId,
    StrategyLoadError, StrategyLoaded, StrategySwapped, Symbol, Tick, Timestamp, Usdt, Venue,
};

use agent::ActivityEvent;

use crate::lab::activity::ActivityTape;
use crate::lab::state::{DateRange, LabState};

/// Operator-locked XRP-first pair ordering (ui-rethink-phase-a-lab R3.2 /
/// T-D-8). Re-exported from `lab::universe` so call sites can use
/// `state::LAB_PAIR_ORDER` without knowing the internal module layout.
/// Type: `&'static [(Venue, &'static str)]` — `Symbol` is not `const`-
/// compatible (contains `SmolStr`), so the raw `&str` form is used.
pub use crate::lab::universe::XRP_FIRST_UNIVERSE as LAB_PAIR_ORDER;
use crate::theme::layout::{LIVE_EQUITY_BUFFER_CAP, TAPE_MAX_ROWS};

// ── cockpit-toast-queue v0.1.0 — bounded queue, types, and constants ─────────

/// Maximum number of simultaneously visible toast cards (ADR-0046 § Decision).
/// Overflow policy: drop OLDEST (FIFO ring) — newest is most operator-relevant.
pub const MAX_TOAST_QUEUE_LEN: usize = 5;

/// Auto-dismiss timeout for toast cards (ADR-0046 § Decision / Q3=(b)).
/// Evaluated at each 500 ms `ToastTick` tick against `ToastEntry::created_at`.
pub const TOAST_AUTODISMISS: Duration = Duration::from_secs(5);

/// Fixed pixel width of each toast card in the tray (ADR-0046 § Design).
pub const TOAST_CARD_WIDTH_PX: f32 = 320.0;

/// Type alias for toast IDs — monotonic u64, per-`AppState` counter.
pub type ToastId = u64;

/// Severity level for a toast card. Maps to existing Lumen color tokens;
/// zero new tokens introduced (ADR-0046 § Decision / R-NR.4 / K7).
///
/// Color mapping (view-side): `Info → FG_2` / `Success → UP_500` /
/// `Warning → INFO_400` / `Danger → DOWN_500`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ToastSeverity {
    /// Informational — `color::FG_2` left border.
    Info,
    /// Positive outcome — `color::UP_500` left border.
    Success,
    /// Caution — `color::INFO_400` left border.
    Warning,
    /// Error / failure — `color::DOWN_500` left border.
    Danger,
}

/// A single toast notification entry in the queue.
///
/// `Clone + Debug + PartialEq` — enables unit-test assertions.
/// `created_at: Instant` is the auto-dismiss seam: the `ToastTick(now)` arm
/// compares `now.duration_since(created_at)` rather than calling
/// `Instant::now()` inside the arm (ADR-0046 clock-injection pattern).
#[derive(Clone, Debug, PartialEq)]
pub struct ToastEntry {
    /// Monotonic ID from `Cockpit::toast_next_id` (per-instance `Cell<u64>`).
    /// Stable across clone; used by `DismissToastById` and the `×` button.
    pub id: ToastId,
    /// Message text. `SmolStr` for small-string-optimised allocation.
    pub message: SmolStr,
    /// Severity tag — drives the left-border color in the tray widget.
    pub severity: ToastSeverity,
    /// Wall-clock stamp at enqueue time. The `ToastTick` arm uses this to
    /// implement auto-dismiss without a per-entry timer task.
    pub created_at: Instant,
}

// ── end of cockpit-toast-queue types ─────────────────────────────────────────

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
/// Phase A — screen routing variants. New variants added; legacy
/// variants kept as `#[deprecated]` aliases that route to their
/// successors via the `shell::screen_body` match for one cycle.
/// (ui-rethink-phase-a-lab R9.3 / Design § 6.)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    // ── Phase A active routes ─────────────────────────────────────────
    /// Phase A default — chart-centric workshop (ex-`Charts`). Default
    /// screen at cockpit boot per R1.2.
    Lab,
    /// Phase A — renamed from `Home`. Live-trading dashboard body
    /// unchanged at Phase A.
    Live,
    /// Phase A placeholder — Compare view (Phase E body).
    Compare,
    /// Phase A placeholder — Memory view (Phase F body).
    Memory,
    /// Phase A placeholder — Model registry (Phase F body).
    Models,
    /// Phase A placeholder — Trail / audit journal (Phase D body).
    Trail,
    /// Phase A placeholder — Settings rollup (Phase C body).
    Settings,

    // ── Unchanged active route ────────────────────────────────────────
    /// Strategies detail — unchanged from Phase 3.
    Strategies,

    // ── cockpit-baseline-panel v0.1.0 ─────────────────────────────────
    /// Passive buy-and-hold baseline panel (cockpit-baseline-panel R6).
    /// Navigable from the Work sidebar group, after `Compare` (D2 —
    /// navigable, not default-routed). Surfaces the shipped passive-BH
    /// result: realized equity curve + drawdown band + the six-card KPI
    /// strip, with a 2023/2024 year toggle.
    Baseline,

    // ── advisor-leaderboard-screen v0.1.0 ─────────────────────────────
    /// The strategy bake-off LEADERBOARD (single-coin investment-advisor
    /// journey, step 3: rank & pick best). Navigable from the **Work**
    /// sidebar group, after `Baseline` (navigable, not default-routed).
    /// Renders a `backtest::bakeoff` result: a ranked table (crowned row
    /// highlighted, buy-and-hold labelled as the benchmark) + a
    /// plain-language recommendation rendered from the structured
    /// `Recommendation` + a persistent not-advice + simulated disclaimer.
    /// A "Run bake-off" action dispatches `backtest::run_bakeoff` async.
    Leaderboard,

    // ── cockpit-reports-viewer v0.1.0 ─────────────────────────────────
    /// Browse + render any committed `spec/*/reports/backtest-*.md`
    /// (cockpit-reports-viewer R6 / D4). Navigable from the **Library**
    /// sidebar group, after `Models` (D5 — navigable, not default-routed).
    /// List-detail: a left picker over the discovered corpus + a right
    /// detail pane rendering the selected report's KPI strip + markdown
    /// body (the equity curve / drawdown band render Empty-by-data for the
    /// current corpus — § Data contract).
    Reports,

    // ── Deprecated aliases — kept for one cycle (Phase A → Phase C) ──
    /// @deprecated — routes to `Live`. Kept for test-harness compat.
    #[deprecated(since = "0.2.0", note = "use Screen::Live")]
    Home,
    /// @deprecated — routes to `Lab`. Kept for test-harness compat.
    #[deprecated(since = "0.2.0", note = "use Screen::Lab")]
    Charts,
    /// @deprecated — routes to `Trail`. Kept for test-harness compat.
    #[deprecated(since = "0.2.0", note = "use Screen::Trail")]
    Audit,
    /// @deprecated — routes to `Settings`. Kept for test-harness compat.
    #[deprecated(since = "0.2.0", note = "use Screen::Settings")]
    Risk,
    /// @deprecated — routes to `Settings`. Kept for test-harness compat.
    #[deprecated(since = "0.2.0", note = "use Screen::Settings")]
    Debug,
    /// @deprecated — routes to `Settings`. Kept for test-harness compat.
    #[deprecated(since = "0.2.0", note = "use Screen::Settings")]
    Control,
}

impl Default for Screen {
    /// Cold-start default: `Lab` per R1.2.
    fn default() -> Self {
        Screen::Lab
    }
}

/// cockpit-baseline-panel v0.1.0 — selected year on the Baseline screen
/// (R2). Two variants only; `Default = Y2024` (most-recent, R2). The
/// year toggle is a typed message arm — `Message::BaselineSelectYear(
/// BaselineYear)` — never a `String` payload.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BaselineYear {
    /// Calendar year 2023.
    Y2023,
    /// Calendar year 2024 — cold-start default (most recent).
    #[default]
    Y2024,
}

/// Phase C — Settings rollup sub-tab selector. Renders the three
/// existing screen bodies (Risk / Control / Debug) unchanged inside
/// `screens::settings::view`. Cold-start default `Risk` per Q2a.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    /// Risk / limits (most-consulted) — `screens::risk::view`.
    #[default]
    Risk,
    /// `HumanControl` (mode toggle + kill) — `screens::control::view`.
    Control,
    /// Operations chrome (latency, market health, server time, logs) —
    /// `screens::debug::view`.
    Debug,
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

// ── Phase D — Trail screen state (ui-rethink-phase-d-trail T-D-N14) ─────────

/// Trail-screen sub-state (Phase D R2.3-R2.5 / decomp §4 Wave D).
///
/// `selected_audit_id = None` → list mode (cold-start default, R2.5).
/// `selected_audit_id = Some(id)` → trail mode: vertical node stack rendered.
/// `drawer_selected_node = Some(kind)` → side-drawer open to that node.
///
/// The LRU cache lives in the trail-mirror crate (crates/reflection), NOT
/// here — this struct holds only the rendering selection state.
#[derive(Debug, Clone, Default)]
pub struct TrailScreenState {
    /// The audit-row id currently showing in trail mode. `None` = list mode.
    pub selected_audit_id: Option<SmolStr>,
    /// Which node's side-drawer is open. `None` = drawer closed.
    pub drawer_selected_node: Option<crate::widgets::trail_node::TrailNodeKind>,
    /// Phase D+ (ui-rethink-phase-d-trail-followup R1.4) — last reconstructed
    /// trail delivered by `Message::TrailMirrorTick(TrailReady(...))`.
    /// `None` while the trail has not yet been hydrated by the mirror (list
    /// mode or chevron click not yet answered). `Some(trail)` once the mirror
    /// responds; rendered by `screens::trail` trail-mode body.
    pub reconstructed_trail: Option<ReconstructedTrailUi>,
    /// Phase D+ (ui-rethink-phase-d-trail-followup R1.4) — set when the
    /// operator clicks the chevron (`OpenTrailFor` arm); cleared when the
    /// mirror responds with `TrailReady`. While `Some`, the trail-mode body
    /// renders a `frame::loading_body` placeholder (R3.4 reuse).
    pub pending_trail_audit_id: Option<SmolStr>,
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
    /// Active screen for the routed shell. Default `Lab` (Phase A R1.2).
    /// (Phase 2 Q8 — session-scoped, no on-disk persistence.)
    pub current_screen: Screen,

    // ── Phase C — Settings rollup sub-tab (ui-rethink-phase-c-sidebar-ia) ──
    /// Active sub-tab inside the Settings rollup screen.
    /// Cold-start default `Risk` per Q2a; session-scoped (no persistence
    /// in Phase C).
    pub settings_active_tab: SettingsTab,

    // ── Phase A — Lab screen state (ui-rethink-phase-a-lab T-D-4) ───────
    /// Lab screen per-session state: selected (strategy, pair, range,
    /// params) + comparison set. Cold-start defaults to empty; Phase
    /// M-FINAL adds persistence via `lab::persistence`.
    pub lab_state: LabState,

    /// cockpit-activity-status-bar v0.1.0 Wave B (T-D-N4) — activity tape.
    /// In-flight background ops (Yahoo preload, Lab Run, Training).
    /// Updated by `Message::ActivityEventReceived` and purged at ~1 Hz by
    /// `Message::ActivityTapePurgeTick`. Read by `widgets::activity_tape`.
    pub activity_tape: ActivityTape,

    /// `true` while a Lab backtest run is in-flight (T-D-14 / M2.5).
    /// The Run button greys out while this is set; cleared on
    /// `LabRunCompleted`.
    pub lab_run_inflight: bool,

    // ── cockpit-toast-queue v0.1.0 — bounded queue ───────────────────────────
    /// Bounded FIFO queue of visible toast notifications (ADR-0046).
    /// Capacity capped at `MAX_TOAST_QUEUE_LEN = 5`; overflow drops the
    /// OLDEST entry.
    pub toast_queue: VecDeque<ToastEntry>,

    /// Monotonic ID counter for `ToastEntry::id` (per-instance, test-safe).
    /// Matches the `training_log_recipe_salt` precedent — no global `AtomicU64`.
    /// Not `Debug`-derived (excluded from manual `Debug` impl below).
    pub toast_next_id: Cell<u64>,

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

    /// Phase D — Trail-screen sub-state (R2.3-R2.5 / T-D-N14).
    /// `selected_audit_id = None` = list mode (cold-start, R2.5).
    pub trail_screen_state: TrailScreenState,

    /// Phase E — Compare-screen per-session state (ui-rethink-phase-e-compare
    /// R6.1 / T-D-N4). Sibling of `trail_screen_state` (Phase D) and
    /// `lab_state` (Phase A). Cold-start: empty cache (R3.5 cold-boot-only).
    pub compare_screen_state: crate::compare::state::CompareScreenState,

    /// cockpit-baseline-panel v0.1.0 — Baseline-screen per-session state.
    /// Sibling of `compare_screen_state`. Holds the two realized BH equity
    /// curves (loaded once at boot via `baseline::state::load_into`) + the
    /// active-year toggle. Metrics are NOT stored here — they are pulled
    /// from the `const` `baseline::baseline_metrics(active_year)` at view
    /// time (D1=c). Cold-start: `active_year = Y2024`, both curves
    /// `Loading`.
    pub baseline_screen_state: crate::baseline::BaselineScreenState,

    /// cockpit-reports-viewer v0.1.0 — Reports-screen per-session state (D1).
    /// Sibling of `baseline_screen_state`. Holds the discovered
    /// `backtest-*.md` corpus (scanned once at boot via
    /// `reports::state::load_into`), the active selection index, and the
    /// active selection's `ReportLoadResult`. Cold-start: `discovered:
    /// Loading`, `selected: None`, `loaded: Loading`.
    pub reports_screen_state: crate::reports::ReportsScreenState,

    /// advisor-leaderboard-screen v0.1.0 — Leaderboard-screen per-session
    /// state. Sibling of `reports_screen_state`. Holds the strategy
    /// bake-off result behind a `PanelState` (Loading / Empty / Error /
    /// Ready) — the `backtest::BakeoffReport` mirrored into a pure-`ui`
    /// shape at the dispatch boundary (the INVARIANT seam; `ui` never holds
    /// an engine type). Cold-start: `result: Empty` (the "press Run
    /// bake-off" prompt), `running: false`.
    pub leaderboard_screen_state: crate::leaderboard::LeaderboardScreenState,

    /// Phase F — Memory-screen per-session state (ui-rethink-phase-f-memory-models-assistant
    /// R4.1 / T-D-N4). Sibling of `compare_screen_state` (Phase E). Cold-start:
    /// empty cache (R5.3 cold-boot-only); real screen body replaces Phase A placeholder.
    pub memory_screen_state: crate::memory::state::MemoryScreenState,

    /// Phase F — Models-screen per-session state (ui-rethink-phase-f-memory-models-assistant
    /// R4.2 / T-D-N4). Sibling of `memory_screen_state` (Phase F). Cold-start:
    /// empty checkpoints list (R5.3 cold-boot-only).
    pub models_screen_state: crate::models::state::ModelsScreenState,

    /// Phase F — Assistant-slot state (Lumen Phase 6 wake, Q4=(a) stub-only,
    /// R4.3 / T-D-N4). Right-rail slot is closed by default; K6 Option A:
    /// `RIGHT_RAIL_WIDTH_PX = 0.0` preserved; `RIGHT_RAIL_OPEN_WIDTH_PX = 320.0` added.
    pub assistant_state: crate::assistant::state::AssistantState,

    // ── Phase 4 — Backtest-panel cross-link ─────────────────────────────
    /// Read-only mirror of `audit::query::equity_curve_for_strategy`
    /// results, keyed on `StrategyId`. Entry inserted at first
    /// `SelectStrategy(id)`; replaced on subsequent re-selects of the
    /// same id (one-shot semantics — operator switching screens triggers
    /// a fresh fetch). Cleared only on cockpit restart (session-scoped
    /// per Phase 3 Q5).
    pub strategy_equity: HashMap<StrategyId, PanelState<EquitySeries>>,

    // ── cockpit-live-dashboard-wiring v0.1.0 — live equity curve + KPI strip ──
    /// Session-scoped live equity buffer. Raw `(Timestamp, Money<Usdt>)`
    /// points appended one-per-`PnlRefreshed` from `(snap.as_of,
    /// snap.total_equity)`, bounded ring (`LIVE_EQUITY_BUFFER_CAP`). Empty on
    /// each `cockpit_live` boot — session-scoped, the correct live-monitor
    /// session-open state; durable agent-side history is deferred to the
    /// `live-equity-history-durable` follow-on (D1). **NOT serialized.**
    pub live_equity_buffer: VecDeque<(Timestamp, Money<Usdt>)>,

    /// cockpit-live-equity-render-guard (2026-06-11, approach A) — the
    /// **wallclock** `as_of` of the last appended `PnlRefreshed`, kept
    /// SEPARATE from the buffer (whose points store the **data/bar** x-coord
    /// now). It is the out-of-order-delivery guard key: a snapshot whose
    /// `as_of` is earlier than this is a late/duplicate delivery and is
    /// dropped. Tracked apart from the buffer so the delivery guard keys on
    /// the monotone wallclock while the plotted x-axis uses bar time.
    /// `None` until the first snapshot; reset empty on each boot
    /// (session-scoped). **NOT serialized.**
    pub live_equity_last_as_of: Option<Timestamp>,

    /// cockpit-live-trades-counter (2026-06-11, TODO #2) — session-scoped
    /// count of fills received over the live bus. The KPI strip's "Trades"
    /// card renders THIS (a true session total), not `tape.len()` — the tape
    /// is a capped/evicting display deque (`TAPE_MAX_ROWS`), so its length is
    /// a sliding window, not a total. Counts every `FillReceived`, including
    /// fills routed to the paused-tape buffer (pausing the tape display does
    /// not pause trading). Semantic: FILLS, not round-trip trades — honest
    /// for a monitor; a round-trip counter would need exec-side pairing.
    /// Reset to 0 each boot (session-scoped, like the equity buffer).
    /// **NOT serialized.**
    pub live_fill_count: u64,

    /// live-equity-history-durable (A4 / R6) — `true` once a durable paper/live
    /// equity history has been hydrated into the buffer on boot
    /// (`Message::PnlHydrated`). It is the honesty switch for the Live screen's
    /// return caption: a hydrated buffer is a continuous *since-inception*
    /// paper/live history (may span sessions/days), so the caption reads
    /// "Since inception" (`LIVE_SINCE_INCEPTION_CAPTION`); an un-hydrated buffer
    /// (research mode — no hydrate issued — or a paper boot with no prior
    /// history) is session-scoped and keeps "Session to date"
    /// (`LIVE_SESSION_RETURN_CAPTION`). Set only by the hydrate arm; never by a
    /// live `PnlRefreshed`; reset `false` on each boot (session-scoped, like the
    /// buffer). **NOT serialized.**
    pub live_equity_hydrated: bool,

    /// Derived-on-append (D5) render state for the Live equity curve.
    /// `Loading` until the first point; `Ready(series)` from ≥1 point;
    /// `Error(msg)` on `PnlError`; `Empty` only if the channel closes with
    /// 0 points. Built via `EquitySeries::from_points` — the view reads this
    /// cached state, it does not recompute per frame.
    pub live_equity_curve: PanelState<EquitySeries>,

    /// Derived-on-append (D5) render state for the Live KPI strip.
    /// `Loading` until ≥2 points (the `kpi_strip::is_all_absent` bootstrap
    /// trap — a 1-point series is byte-identical to the all-absent sentinel,
    /// D2); `Ready(metrics)` after; `Error(msg)` on `PnlError`; `Empty`
    /// mirrors the curve. Total-return + Max-DD are live; Sharpe/CAGR/Win-rate
    /// render `—`; Trades = 0 (no live counter — D2 / follow-on).
    pub live_kpi: PanelState<BacktestMetrics>,

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

    // ── Phase B — Lab equity overlay cache (T-D-N11) ────────────────────
    /// In-memory cache for loaded equity series from backtest reports.
    /// `RefCell` provides interior mutability so the cache can be filled
    /// on cache-miss during a `view(&Cockpit, …)` call, which borrows
    /// `Cockpit` immutably. The iced update/view cycle is single-threaded
    /// (only the iced thread ever calls `view`), so `RefCell` is safe here.
    pub equity_cache: std::cell::RefCell<crate::lab::equity_loader::EquityCache>,

    // ── F5 — Forward paper-trade budget context ──────────────────────────
    /// The budget the forward paper run is trading against (F5).
    ///
    /// `Some(b)` when the user launched a forward run with the leaderboard's
    /// €200 budget (or a custom amount). The Live screen renders the running
    /// P/L = equity − budget from this value. `None` → no forward run has
    /// been launched yet, or the run uses the default capital (no budget
    /// framing rendered — the P/L card shows the raw session equity instead).
    ///
    /// Set by `Message::ForwardPaperTradeStarted(budget)` and cleared to
    /// `None` on a new paper run without a budget (or on cockpit restart).
    /// ADR-0060 § D5 / F5 Live P/L framing.
    pub forward_budget: Option<Money<Usdt>>,
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
            .field("lab_state", &self.lab_state)
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
            .field("trail_screen_state", &self.trail_screen_state)
            .field("compare_screen_state", &self.compare_screen_state)
            .field("baseline_screen_state", &self.baseline_screen_state)
            .field("reports_screen_state", &self.reports_screen_state)
            .field("leaderboard_screen_state", &self.leaderboard_screen_state)
            .field("memory_screen_state", &self.memory_screen_state)
            .field("models_screen_state", &self.models_screen_state)
            .field("assistant_state", &self.assistant_state)
            .field("strategy_equity", &self.strategy_equity)
            .field("live_equity_buffer_len", &self.live_equity_buffer.len())
            .field("live_equity_last_as_of", &self.live_equity_last_as_of)
            .field("live_equity_hydrated", &self.live_equity_hydrated)
            .field("live_fill_count", &self.live_fill_count)
            .field("live_equity_curve", &self.live_equity_curve)
            .field("live_kpi", &self.live_kpi)
            .field("execution_mode", &self.execution_mode)
            .field("paused_strategies", &self.paused_strategies)
            .field("override_risk_veto", &self.override_risk_veto)
            .field("risk_veto_events", &self.risk_veto_events)
            .field("focused_widget", &self.focused_widget)
            .field("lab_run_inflight", &self.lab_run_inflight)
            .field("toast_queue_len", &self.toast_queue.len())
            .field("toast_next_id", &self.toast_next_id)
            .field("activity_tape", &"<ActivityTape>")
            .field("equity_cache", &"<EquityCache>")
            .field("settings_active_tab", &self.settings_active_tab)
            .field("forward_budget", &self.forward_budget);
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
            settings_active_tab: SettingsTab::default(),
            lab_state: LabState::default(),
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
            trail_screen_state: TrailScreenState::default(),
            compare_screen_state: crate::compare::state::CompareScreenState::default(),
            baseline_screen_state: crate::baseline::BaselineScreenState::default(),
            reports_screen_state: crate::reports::ReportsScreenState::default(),
            leaderboard_screen_state: crate::leaderboard::LeaderboardScreenState::default(),
            memory_screen_state: crate::memory::state::MemoryScreenState::default(),
            models_screen_state: crate::models::state::ModelsScreenState::default(),
            assistant_state: crate::assistant::state::AssistantState::default(),
            strategy_equity: HashMap::new(),
            // cockpit-live-dashboard-wiring — session-scoped, empty on boot.
            live_equity_buffer: VecDeque::new(),
            live_equity_last_as_of: None,
            // live-equity-history-durable — no history hydrated yet on boot.
            live_equity_hydrated: false,
            live_fill_count: 0,
            live_equity_curve: PanelState::Loading,
            live_kpi: PanelState::Loading,
            execution_mode: ExecutionMode::default(),
            paused_strategies: HashSet::new(),
            override_risk_veto: OverrideRiskVetoState::default(),
            risk_veto_events: Vec::new(),
            focused_widget: None,
            lab_run_inflight: false,
            toast_queue: VecDeque::with_capacity(MAX_TOAST_QUEUE_LEN),
            toast_next_id: Cell::new(0),
            activity_tape: ActivityTape::new(),
            equity_cache: std::cell::RefCell::new(crate::lab::equity_loader::EquityCache::new()),
            forward_budget: None,
        }
    }
}

impl Cockpit {
    /// Fresh cockpit. All panels start in `Loading`.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Boot cockpit with persistence restore (T-D-14c).
    ///
    /// Identical to `new()` except that `lab_state` is populated from the
    /// on-disk `cockpit-lab-state.json` (Design § 5). If the file is absent
    /// or corrupt, falls back to the Q-A3 cold-start defaults
    /// (`v1.momentum × XRPUSDT × Last 90d`).
    ///
    /// `state_path_override` redirects the state file to a caller-supplied
    /// path (used by integration tests to point at a temp dir).
    #[must_use]
    pub fn boot(state_path_override: Option<&std::path::Path>) -> Self {
        use crate::lab::persistence;
        let path = persistence::lab_state_path(state_path_override);
        let lab_state = persistence::restore_or_default(&path);
        Self {
            lab_state,
            ..Self::default()
        }
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
            settings_active_tab: SettingsTab::default(),
            lab_state: LabState::default(),
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
            trail_screen_state: TrailScreenState::default(),
            compare_screen_state: crate::compare::state::CompareScreenState::default(),
            baseline_screen_state: crate::baseline::BaselineScreenState::default(),
            reports_screen_state: crate::reports::ReportsScreenState::default(),
            leaderboard_screen_state: crate::leaderboard::LeaderboardScreenState::default(),
            memory_screen_state: crate::memory::state::MemoryScreenState::default(),
            models_screen_state: crate::models::state::ModelsScreenState::default(),
            assistant_state: crate::assistant::state::AssistantState::default(),
            strategy_equity: HashMap::new(),
            // cockpit-live-dashboard-wiring — session-scoped, empty on boot.
            // `ready()` is a fixture/test constructor; tests that exercise the
            // live curve/strip seed it via `Message::PnlRefreshed` updates.
            live_equity_buffer: VecDeque::new(),
            live_equity_last_as_of: None,
            // live-equity-history-durable — fixtures never hydrate durable history.
            live_equity_hydrated: false,
            live_fill_count: 0,
            live_equity_curve: PanelState::Loading,
            live_kpi: PanelState::Loading,
            execution_mode: ExecutionMode::default(),
            paused_strategies: HashSet::new(),
            override_risk_veto: OverrideRiskVetoState::default(),
            risk_veto_events: Vec::new(),
            focused_widget: None,
            lab_run_inflight: false,
            toast_queue: VecDeque::with_capacity(MAX_TOAST_QUEUE_LEN),
            toast_next_id: Cell::new(0),
            activity_tape: ActivityTape::new(),
            equity_cache: std::cell::RefCell::new(crate::lab::equity_loader::EquityCache::new()),
            forward_budget: None,
        }
    }

    /// cockpit-live-dashboard-wiring v0.1.0 (D5) — append one live equity
    /// point and rebuild the derived curve + KPI-strip render state.
    ///
    /// Called from the `Message::PnlRefreshed` arm once per bar. Pure
    /// mutation on the model; no async work, no bus event. The two derived
    /// `PanelState`s are cached here so the per-frame `view` reads them
    /// without recomputing (the iced `view` runs on every message / hover).
    ///
    /// ## Two timestamps (cockpit-live-equity-render-guard, 2026-06-11 — approach A)
    ///
    /// - `as_of` — the snapshot's **wallclock** publish time
    ///   (`Timestamp::now()`). It is the **out-of-order-delivery guard** key:
    ///   monotone by construction (a clock never goes back), so a snapshot that
    ///   arrives with an earlier `as_of` than the last one is a late/duplicate
    ///   delivery and is dropped. This is the guard the architect pinned to
    ///   `as_of` (NOT to the data time) — stamping `as_of` with bar time broke
    ///   the curve once (reverted I1) precisely because it conflated the
    ///   delivery key with the plotted coordinate.
    /// - `x_coord` — the **data/bar** time the chart plots on its x-axis
    ///   (`snap.bar_ts`, i.e. `bar.close_ts`; falls back to `as_of` when a
    ///   snapshot carries no bar context). Stored as the buffer point's
    ///   timestamp → becomes `EquityPoint.ts` → drives the span-adaptive axis
    ///   labels (`MMM 'YY` / `MMM DD`). During a fast replay this is the 2023-24
    ///   data date, so the axis is meaningful instead of "all the same wallclock
    ///   minute".
    ///
    /// Honors the must-honor edges:
    /// 1. **Delivery guard on `as_of`** (above) — drop a strictly-earlier-
    ///    delivered snapshot.
    /// 2. **`from_points` monotone-`ts` invariant** — `EquitySeries::from_points`
    ///    rejects a stored `ts` that goes backwards. In forward replay `bar_ts`
    ///    is chronological, so this never fires; but because the delivery guard
    ///    keys on `as_of` (not the stored `x_coord`), we additionally **clamp**
    ///    the stored coordinate to be ≥ the last stored coordinate. That makes
    ///    the buffer's `ts` sequence monotone *by construction* — `from_points`
    ///    can never error on it — without ever panicking the rasterizer or
    ///    dropping a delivered point. (No-op in practice; pure defense.)
    /// 3. **`is_all_absent` 1-point trap** — a single point yields
    ///    `total_return = max_dd = 0` (and `trades = 0` until the first fill
    ///    lands), byte-identical to the all-absent sentinel
    ///    (`kpi_strip::is_all_absent`), which would render six dashes. The
    ///    KPI strip therefore stays `Loading` until ≥2 points; the curve
    ///    renders from ≥1 (a 1-point curve is valid).
    fn push_live_equity_point(
        &mut self,
        as_of: Timestamp,
        x_coord: Timestamp,
        equity: Money<Usdt>,
    ) {
        // (1) Delivery guard — drop a snapshot delivered strictly out of order
        // (earlier wallclock `as_of` than the last delivered point). `as_of` is
        // monotone on the live path, so this only fires on a genuine late /
        // duplicate delivery. The guard keys on `as_of` per the architect's
        // pin, NOT on the plotted `x_coord`.
        if let Some(back_as_of) = self.live_equity_last_as_of
            && as_of.unix_millis() < back_as_of.unix_millis()
        {
            return;
        }
        self.live_equity_last_as_of = Some(as_of);

        // (2) Clamp the stored x-coordinate to be monotone non-decreasing so
        // `from_points` can never error (defense — see edge 2 above). In
        // forward replay `x_coord` is already monotone, so the clamp is a
        // no-op; it only bites in the impossible "wallclock advanced but bar
        // time went backwards" case, where it keeps the curve crash-free.
        let stored_ts = match self.live_equity_buffer.back() {
            Some((last_ts, _)) if x_coord.unix_millis() < last_ts.unix_millis() => *last_ts,
            _ => x_coord,
        };
        self.live_equity_buffer.push_back((stored_ts, equity));

        // (2) Ring bound — evict oldest past the cap.
        while self.live_equity_buffer.len() > LIVE_EQUITY_BUFFER_CAP {
            self.live_equity_buffer.pop_front();
        }

        // (3) Rebuild the curve (≥1 point). `from_points` only errors on the
        // guarded-empty case (impossible here — we just pushed) or a
        // non-monotone pair (impossible — the append guard enforces order),
        // so on the unexpected `Err` we leave the curve `Loading` rather than
        // panic.
        let points: Vec<(Timestamp, Money<Usdt>)> =
            self.live_equity_buffer.iter().copied().collect();
        if let Ok(series) = EquitySeries::from_points(points) {
            // `EquitySeries::max_drawdown_pct` is a FRACTION (0.40 = 40 %);
            // `BacktestMetrics.max_drawdown_pct` carries PERCENT units (the
            // baseline const stores `34.57` to render "−34.57%", and the
            // kpi_strip's `format_pct_max_dd` appends `%` to the value
            // verbatim). Scale ×100 at this wiring seam so the live Max-DD
            // card reads "−40.00%", not "−0.40%"
            // (cockpit-live-kpi-units-fix, 2026-06-10).
            let max_drawdown_pct = series.max_drawdown_pct * Decimal::ONE_HUNDRED;
            self.live_equity_curve = PanelState::Ready(series);

            // (4) Rebuild the KPI strip — Loading until ≥2 points (trap).
            if self.live_equity_buffer.len() < 2 {
                self.live_kpi = PanelState::Loading;
            } else {
                // Session return = (latest − first) / first, a FRACTION
                // (0.015 = +1.5 %). `BacktestMetrics.total_return_pct` carries
                // PERCENT units (the baseline const stores `196.22` to render
                // "+196.22%"; `format_pct_sentiment` appends `%` verbatim), so
                // we scale ×100 here — otherwise a +1.5 % session rendered as
                // "0.01%" (100× too small), which is exactly the "Total return
                // 0.01–0.02" the operator reported
                // (cockpit-live-kpi-units-fix, 2026-06-10). The first
                // accumulated point is the session open. Guard first ≠ 0 (the
                // agent's starting equity is non-zero, but divide-guard anyway).
                let first = self.live_equity_buffer[0].1.amount();
                let latest = self.live_equity_buffer[self.live_equity_buffer.len() - 1]
                    .1
                    .amount();
                let total_return_pct = if first.is_zero() {
                    Decimal::ZERO
                } else {
                    (latest - first) / first * Decimal::ONE_HUNDRED
                };
                self.live_kpi = PanelState::Ready(BacktestMetrics {
                    total_return_pct,
                    max_drawdown_pct,
                    // Session fill count (cockpit-live-trades-counter). Also
                    // updated in place by the `FillReceived` arm between
                    // per-bar rebuilds.
                    trades: self.live_fill_count,
                    // No live Sharpe/CAGR/Win-rate math (no `core` source,
                    // out of scope for a monitor — D2). Render `—`.
                    cagr_pct: Decimal::ZERO,
                    cagr_present: false,
                    sharpe: Decimal::ZERO,
                    sharpe_present: false,
                    win_rate_pct: Decimal::ZERO,
                    win_rate_present: false,
                });
            }
        } else {
            // `from_points` only errors on the guarded-empty case (impossible
            // — we just pushed) or a non-monotone pair (impossible — the
            // append guard enforces order). Leave both panels Loading rather
            // than panic.
            self.live_equity_curve = PanelState::Loading;
            self.live_kpi = PanelState::Loading;
        }
    }
}

// ── Phase D+ — UI-local trail-mirror types (ui-rethink-phase-d-trail-followup) ─
//
// Q2 resolution (b): these types mirror `reflection::trail_mirror::{TrailMirrorTick,
// ReconstructedTrail, TrailStage}` at a UI-local boundary so the `Message` enum
// carries no direct `reflection` type. The `Message` variant thus stays in the
// default (non-`live`) ui-crate build's public API without dragging a `reflection`
// dep into it. The crate-boundary `From` conversion lives in `crates/ui/src/live.rs`
// under `#[cfg(feature = "live")]`.

/// UI-local mirror of `reflection::trail_mirror::TrailMirrorTick` (Q2 (b)).
///
/// Keeps the `Message` payload free of a direct `reflection` type so the
/// default (non-`live`) ui-crate build retains the v0.1.0 edge set
/// (`ui → {trading_core, audit, agent, backtest, reports}`). The
/// crate-boundary conversion lives in `crates/ui/src/live.rs` under
/// `#[cfg(feature = "live")]`.
#[derive(Debug, Clone)]
pub enum TrailMirrorUiTick {
    /// A reconstructed trail is ready (LRU hit or SQL backfill completed).
    /// Boxed for parity with the upstream enum (`large_enum_variant`).
    TrailReady(Box<ReconstructedTrailUi>),
    /// Steady-state update: re-fetch the trail for this `audit_id`.
    TrailUpdated(SmolStr),
}

/// UI-local mirror of `reflection::trail_mirror::TrailStage`.
///
/// `raw_payload` stores the full drawer payload forwarded to the side-drawer.
/// Other fields use `Option<String>` matching `TrailNode`'s field types.
#[derive(Debug, Clone, Default)]
pub struct TrailStageUi {
    pub timestamp: Option<String>,
    pub actor: Option<String>,
    pub headline: Option<String>,
    pub raw_payload: Option<String>,
}

/// UI-local mirror of `reflection::trail_mirror::ReconstructedTrail`.
///
/// Also contains pre-built `TrailNode` structs (Phase D+ T-D-N10) so
/// `screens::trail::view` can borrow `&TrailNode` with the Cockpit's lifetime
/// rather than from a local variable — `trail_node::view<'a>(node: &'a TrailNode)`
/// returns `Element<'a>` so the element must borrow from something that outlives
/// the `view` function's return. Storing nodes here achieves that.
#[derive(Debug, Clone, Default)]
pub struct ReconstructedTrailUi {
    pub audit_id: SmolStr,
    pub fill: TrailStageUi,
    pub signal: TrailStageUi,
    pub forecast: TrailStageUi,
    pub debate: TrailStageUi,
    /// Pre-built trail nodes for the four stages, in upstream-first order:
    /// `[Forecast, LlmDebate, Signal, Fill]`. Populated alongside the stage
    /// fields from the same mirror tick. Borrowing `&self.nodes[i]` from a
    /// `ReconstructedTrailUi` stored in `Cockpit` gives the correct `'model`
    /// lifetime for `trail_node::view<'a>(node: &'a TrailNode) -> Element<'a>`.
    /// `Vec` (not array) because `TrailNode` doesn't derive `Default`.
    pub nodes: Vec<crate::widgets::trail_node::TrailNode>,
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
    /// live-equity-history-durable (A4 / A5) — boot-time batch hydrate of the
    /// durable paper/live equity series. Each tuple is `(bar_ts /*plotted
    /// x-coord*/, as_of /*delivery key*/, total_equity)`, in monotone `bar_ts`
    /// order, capped at the buffer cap by the reader's `LIMIT`. Seeds
    /// `live_equity_buffer` through `push_live_equity_point` in ONE mutation
    /// (one curve/KPI rebuild — distinct from a per-bar live tick), seeds the
    /// delivery guard from the MAX hydrated `as_of` so the first live
    /// `PnlRefreshed(now())` still lands, and flips `live_equity_hydrated` so
    /// the return caption reads "Since inception". Issued only in paper/live
    /// mode (the boot site gates `mode != Research`); research stays
    /// session-scoped.
    PnlHydrated(Vec<(Timestamp, Timestamp, Money<Usdt>)>),
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

    // ── Phase C — Settings rollup sub-tab (ui-rethink-phase-c-sidebar-ia) ──
    /// Settings tab-strip click. Pure assignment to
    /// `Cockpit::settings_active_tab`; no I/O.
    SwitchSettingsTab(SettingsTab),

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

    // ── Phase A — Lab screen (ui-rethink-phase-a-lab T-D-4) ─────────────
    /// Operator selected a `(Venue, Symbol)` pair chip on the Lab screen.
    /// Pure assignment to `Cockpit::lab_state.pair`.
    LabSelectPair(Venue, Symbol),
    /// Operator clicked a primary-strategy chip on the Lab screen.
    /// Pure assignment to `Cockpit::lab_state.strategy`.
    LabSelectPrimaryStrategy(StrategyId),
    /// Operator pressed the compare toggle on a strategy chip.
    /// Delegates to `LabState::toggle_compare` — no-op when set is full
    /// (returns `false`; caller emits a toast in M2.5 / Wave 2).
    LabToggleCompare(StrategyId),
    /// Operator selected a date-range preset or committed a custom range.
    /// Pure assignment to `Cockpit::lab_state.range`.
    LabSelectRange(DateRange),
    /// Operator toggled the data-source chip between `Synthetic` and `YahooCache`
    /// (lab-yahoo-realdata T-C3.5 / R3.1). Pure assignment to
    /// `Cockpit::lab_state.data_source`.
    LabSelectDataSource(crate::lab::state::LabDataSource),
    /// Operator typed into the SMA fast-window input (lab-polish-round-2 R2).
    /// String value lives in `sma_fast_input`; if it parses to `usize`,
    /// `sma_fast_len = Some(parsed)`, else `sma_fast_len = None`.
    LabSetSmaFast(String),
    /// Operator typed into the SMA slow-window input (lab-polish-round-2 R2).
    LabSetSmaSlow(String),
    /// Operator pressed "Run backtest". Fires `lab::runner::spawn_lab_run`
    /// on the binary side (M2.5 / T-D-14). Pure state marks `run_inflight`.
    LabRunRequested,
    /// Async backtest-run result (M2.5 / T-D-14).
    /// Carries `Ok(RunSummary)` on success, `Err(message)` on failure.
    LabRunCompleted(crate::lab::runner::LabRunResult),
    /// Operator pressed the Stop button while a Lab run is in-flight.
    ///
    /// Handled by the binary-side wrapper in `cockpit_live.rs::update` which
    /// drops `lab_state.run_cancel` (the Drop fires cancel at the engine's next
    /// poll boundary). Pure state: `run_inflight` stays `true` until
    /// `LabRunCompleted(Err("cancelled"))` arrives.
    ///
    /// lab-end-to-end-v2 T-D3.4 / R6.3.
    LabRunStopRequested,
    /// Progress update from the in-flight backtest engine.
    ///
    /// Delivered by `LabProgressRecipe::stream()` as bars complete.
    /// Pure state: stored in `LabState::run_progress`.
    ///
    /// lab-end-to-end-v2 T-D4.6 / R9.
    LabRunProgress(backtest::progress::Progress),
    /// Progress channel closed — engine completed or was cancelled.
    ///
    /// Belt-and-suspenders clear of `run_progress` before `LabRunCompleted`
    /// arrives. Pure state: clears `LabState::run_progress`.
    ///
    /// lab-end-to-end-v2 T-D4.6 / R9.4.
    LabRunProgressDone,
    /// Show a transient toast notification (R4.2 / T-D-16).
    /// The string is a `&'static str` via `crate::strings`; no inline literals.
    /// Maps to `ToastSeverity::Info` — back-compat with existing producers.
    ShowToast(SmolStr),
    /// Clear the FRONT toast entry (T-D-16 back-compat — dismisses oldest visible).
    DismissToast,

    // ── cockpit-toast-queue v0.1.0 — NEW message variants ────────────────────
    /// Enqueue a typed-severity toast notification (ADR-0046 / R1.5).
    /// Producers should prefer this over `ShowToast` so the severity tint is
    /// accurate. `ShowToast` continues to work and maps to `Info` severity.
    ShowToastWithSeverity(SmolStr, ToastSeverity),
    /// Dismiss a specific toast card by its `ToastEntry::id` (ADR-0046 / R1.4).
    /// Emitted by the `×` button on each card in `widgets::toast_tray`.
    DismissToastById(ToastId),
    /// Auto-dismiss sweep trigger (ADR-0046 / R2.5).
    /// Emitted every 500 ms by `ToastDismissRecipe`. Carries the "now" instant
    /// as a payload so tests can inject a synthetic instant without a clock
    /// field on `AppState` (ADR-0046 clock-injection pattern / K5).
    ToastTick(Instant),

    // ── Training panel — cockpit-training-control T-D-N4 ─────────────────────
    /// Operator pressed the Train button — start a new training run.
    TrainingPressed,
    /// Operator pressed the Cancel button — SIGKILL the in-flight subprocess.
    TrainingCancelPressed,
    /// A log line arrived from the `train_tcn` subprocess's stdout/stderr.
    TrainingLogLine(SmolStr),
    /// The `train_tcn` subprocess exited (success or failure).
    TrainingExited(std::process::ExitStatus),
    /// Operator toggled the Train panel collapsed/expanded header chip.
    TrainingPanelToggled,
    /// Operator pressed "Clear log" — clears the ring buffer without affecting
    /// the in-flight subprocess.
    TrainingClearLog,
    /// Operator clicked inside the training log pane — freezes auto-scroll.
    TrainingLogClicked,
    /// Operator clicked "Jump to bottom" chip — restores auto-scroll anchoring.
    TrainingLogJumpToBottom,
    /// Audit-DB poller delivered new training-event rows (T-D-N11).
    /// Appended to `LabState::training_events` (capacity 1024).
    /// Only available with `--features live` (audit crate dependency).
    #[cfg(feature = "live")]
    TrainingEventsRefreshed(Vec<trading_core::views::TrainingEventRow>),

    // ── Phase D — Trail view (ui-rethink-phase-d-trail) ──────────────────────
    /// Operator clicked the chevron on a trail node widget. Opens / focuses
    /// the side-drawer to that node's payload (R4.3 / Q3 = chevron-click).
    TrailNodeChevronClicked(crate::widgets::trail_node::TrailNodeKind),
    /// Operator clicked the Trail chevron on an agent-feed row or audit-table row.
    /// Compound dispatch: expands to `SelectTrailRow(id)` + `SwitchScreen(Trail)`.
    /// The **only** new public-surface Message variant (R5.3).
    OpenTrailFor(SmolStr),
    /// Internal: select a trail row by `audit_id` (part of compound dispatch from
    /// `OpenTrailFor`; not emitted directly from UI widgets).
    SelectTrailRow(SmolStr),
    /// Operator dismissed the trail side-drawer. Clears `drawer_selected_node`
    /// but preserves `selected_audit_id` so the trail node stack stays visible.
    TrailDrawerClosed,
    /// Trail-mirror tick delivered via the iced Subscription bridge (Phase D+).
    /// Carries the structured UI-local tick type (Q2 (b) — no direct
    /// `reflection` type in the `Message` API).
    TrailMirrorTick(TrailMirrorUiTick),

    // ── Phase F — Memory + Models + Assistant (ui-rethink-phase-f-memory-models-assistant) ──
    /// Cold-boot hydrate: reflection DB read result delivered via the side-thread
    /// tokio runtime in `cockpit_live`. Populates `memory_screen_state.cache`
    /// and sets `last_indexed` to the current ISO-8601 timestamp (K1 + Q8=(b)
    /// architect-refined placement in `crates/reflection/src/query.rs`).
    MemoryHydrate(Vec<crate::memory::state::LessonCardCard>),
    /// Memory card chevron clicked — open the side-drawer for `card_id` (Q5=(b)).
    MemoryOpenDrawer(smol_str::SmolStr),
    /// Memory drawer close button clicked — collapse the drawer (Q5=(b) / K4).
    MemoryCloseDrawer,
    /// Memory toolbar view-mode toggle (R8.1). Pure assignment.
    MemoryToggleMode(crate::memory::state::MemoryViewMode),
    /// Memory toolbar filter chip toggled (R8.1). `None` = clear filter.
    MemorySetFilter(Option<crate::memory::state::MemoryFilter>),
    /// Cold-boot hydrate: checkpoint discovery result delivered via the side-thread
    /// tokio runtime in `cockpit_live`. Populates `models_screen_state.checkpoints`
    /// and sets `last_indexed` (R5.2 / T-D-N9).
    ModelsHydrate(Vec<crate::models::state::CheckpointMeta>),
    /// Models toolbar family filter updated (R8.1). Pure assignment.
    ModelsSetFamilyFilter(Vec<crate::models::state::ModelFamily>),
    /// Models toolbar status filter updated (R8.1). Pure assignment.
    ModelsSetStatusFilter(Vec<crate::models::state::ModelStatus>),
    /// Toggle the right-rail Assistant slot open/closed (R3.3 / R8.1).
    /// K6 Option A: flips `assistant_state.is_open`; shell picks width.
    ToggleAssistantSlot,
    /// v3-llm-forecaster Wave F (T-D-N(F3)) — a new LLM forecast was
    /// produced upstream and should be rendered in the Assistant slot
    /// reasoning-trace body.
    ///
    /// The arm:
    /// - Sets `assistant_state.last_forecast = Some(view)`.
    /// - Prepends the previous `last_forecast` (if any) onto
    ///   `assistant_state.history`, capped at
    ///   `assistant::state::HISTORY_CAP` (R9.2 bullet 5).
    /// - Does **not** flip `assistant_state.mode` — the runtime gate
    ///   (R9.3) is owned by the `cockpit_live` boot path that sets
    ///   `mode == ReasoningTrace` once when it observes the
    ///   `llm_forecaster_v3` strategy enabled in agent config.
    AssistantReasoningTraceUpdate(crate::assistant::state::LlmForecastView),

    // ── Phase E — Compare matrix (ui-rethink-phase-e-compare) ────────────────
    /// Compound dispatch: switches to Lab screen + seeds strategy/pair/range.
    ///
    /// Expands to: `SwitchScreen(Screen::Lab)` → `SelectStrategy(strategy)`
    /// → `LabSelectPair(venue, symbol)` (when `pair` is `Some`) →
    /// `LabSelectRange(range)`.
    ///
    /// Order matches K4 mitigation (verbatim `OpenTrailFor:1902-1910` pattern):
    /// `SelectStrategy` clears `last_run_report`; the pair and range writes
    /// happen after so the seeded Lab renders correctly.
    ///
    /// R4.1 / R4.2 / R4.3 — populated cell: no auto-run. Empty cell:
    /// caller emits an additional `LabRunRequested` after this message.
    OpenLabFromCompare {
        strategy: StrategyId,
        pair: Option<(Venue, Symbol)>,
        range: crate::lab::state::DateRange,
    },

    /// Operator toggled the Compare date-range picker (R1.2 toolbar).
    /// Pure assignment to `Cockpit::compare_screen_state.range` (R3.4
    /// isolation — MUST NOT mutate `lab_state.range`).
    CompareSelectRange(crate::lab::state::DateRange),

    /// Operator selected a KPI axis from the Compare toolbar dropdown (R6.3).
    /// Pure assignment to `Cockpit::compare_screen_state.kpi_axis`.
    /// v0.1.0: only `Sharpe` is wired; other variants fall back to Sharpe
    /// with a `tracing::warn!`.
    CompareSelectKpiAxis(crate::compare::state::CompareKpiAxis),

    /// lab-compare-equity-overlay T2 (Q1) — operator clicked a populated cell's
    /// `+` overlay chip to add/remove it from the two-run equity-overlay
    /// selection ring. Typed payload (the cell's `(strategy_id, symbol, range)`
    /// identity) — no `String` catch-all. Pure: toggles
    /// `compare_screen_state.overlay_selection` via `toggle_overlay`.
    CompareToggleOverlay(crate::compare::state::OverlaySlot),

    // ── cockpit-baseline-panel v0.1.0 — year toggle ──────────────────────────
    /// Operator clicked a year chip (`2023` | `2024`) on the Baseline
    /// screen (R2). Pure assignment to
    /// `Cockpit::baseline_screen_state.active_year`. Typed payload — no
    /// `String` catch-all.
    BaselineSelectYear(BaselineYear),

    // ── cockpit-reports-viewer v0.1.0 — report picker ────────────────────────
    /// Operator picked the report at index `usize` in the discovered list
    /// on the Reports screen (R1). Synchronous one-file load in the arm
    /// (parse the `## Summary` table + scan for the companion CSV). The
    /// `PathBuf` lives in `reports_screen_state.discovered[idx].path` — the
    /// message key is the typed index, NEVER a `String`/`PathBuf` payload
    /// (R1, mirroring `BaselineSelectYear`'s typed-message discipline).
    ReportsSelect(usize),

    // ── reports-picker-curve-filter — picker rail filter toggle ──────────────
    /// Operator flipped the Reports picker filter between "Curve only" (the
    /// default — companion-bearing rows only) and "All" (the full discovered
    /// corpus). Pure flag flip in the handler; affects the picker LIST only,
    /// never the current selection (`selected` is a FULL-list index that stays
    /// valid regardless of which rows are displayed). Niladic — the two chips
    /// both dispatch this one toggle, so there is no payload to mis-route.
    ReportsToggleShowAll,

    // ── advisor-leaderboard-screen v0.1.0 — strategy bake-off ────────────────
    /// Operator pressed "Run bake-off" on the Leaderboard screen. Flips the
    /// result `PanelState` to `Loading` + sets `running` (pure, in the update
    /// arm); the BINARY-side intercept (mirroring the `LabRunRequested`
    /// precedent in `cockpit_live.rs`) builds the cancel/progress pair and
    /// dispatches `leaderboard::runner::spawn_bakeoff`, which awaits
    /// `backtest::run_bakeoff` on the side-thread runtime. Niladic — the
    /// default coin (BTCUSDT) + lookback (2024 H1) are config, not payload (the
    /// full guided coin/budget input is the next feature, F3).
    BakeoffRunRequested,
    /// A bake-off completed (or failed). Carries the `backtest::BakeoffReport`
    /// already mirrored into the pure-`ui` `BakeoffReportMirror` (the engine
    /// type never crosses into iced state — the INVARIANT seam). Lands
    /// `Ready(mirror)` / `Empty` (zero rows) / `Error(msg)` in the result
    /// `PanelState` and clears `running`.
    BakeoffRunCompleted(crate::leaderboard::runner::BakeoffRunResult),

    // ── advisor-bakeoff-ranking F3 — guided input (coin + budget + lookback) ──
    /// Operator chose a coin in the guided-input coin picker. Stores it on the
    /// leaderboard state (drives the next bake-off + the budget-context
    /// header). Typed `Symbol` payload — no `String` catch-all.
    BakeoffSelectCoin(Symbol),
    /// Operator typed in the budget field. Stores the raw input verbatim (the
    /// parse happens at render time via `parse_budget`). The bake-off ranking
    /// does NOT use the budget — it carries forward to F4/F5 sizing and is
    /// shown in the header for context.
    BakeoffSetBudget(String),
    /// Operator chose a lookback window in the guided-input picker. Stores the
    /// `LeaderboardLookback`; it is mapped to a `backtest::engine::DateRange`
    /// at dispatch time. Typed enum payload.
    BakeoffSelectLookback(crate::leaderboard::LeaderboardLookback),

    // ── cockpit-activity-status-bar v0.1.0 Wave B (T-D-N4) ───────────────────
    /// An `ActivityEvent` arrived from the broadcast channel via
    /// `ActivityRecipe`. Delegates to `ActivityTape::apply`.
    ActivityEventReceived(ActivityEvent),
    /// 1 Hz tick to purge expired failed-activity rows from the tape.
    /// Driven by the same 1 Hz `ServerTimeTick` subscription (the binary
    /// can piggyback the existing time recipe or add a dedicated one;
    /// the update arm is stateless — it only calls `tape.purge(now)`).
    ActivityTapePurgeTick,

    // ── F5 — Forward paper-trade budget framing ───────────────────────────────
    /// The cockpit binary emits this ONCE when it starts a forward paper run
    /// with a concrete budget (€200 ≈ 200 USDT). The update arm sets
    /// `Cockpit::forward_budget = Some(budget)` so the Live screen can
    /// render the P/L = equity − budget framing.
    ///
    /// `budget=None`/`forward=None` paper runs (the legacy research/soak
    /// path) never emit this message, keeping those paths byte-identical.
    ///
    /// ADR-0060 § D5 / F5 Live P/L framing.
    ForwardPaperTradeStarted(Money<Usdt>),
}

/// lab-polish-round-2 R2 — parse SMA window text input.
///
/// Returns `Some(n)` only when `s` parses to a `usize` in `[2, 500]`.
/// Below 2: degenerate (single bar). Above 500: silly for daily/hourly
/// strategies and almost certainly an operator typo. Empty / non-numeric
/// returns `None` → engine falls back to the compiled-in (20, 50) default.
fn parse_sma_window(s: &str) -> Option<usize> {
    let n: usize = s.trim().parse().ok()?;
    if (2..=500).contains(&n) {
        Some(n)
    } else {
        None
    }
}

// ── cockpit-toast-queue v0.1.0 — private enqueue helper ─────────────────────

/// Enqueue a new toast entry into `model.toast_queue`.
///
/// - Bumps `toast_next_id` (per-instance monotonic counter via `Cell<u64>`).
/// - If the queue is at `MAX_TOAST_QUEUE_LEN`, drops the FRONT (oldest) entry
///   before pushing the new one (FIFO ring / drop-oldest policy).
/// - Stamps `created_at: Instant::now()` so the `ToastTick` auto-dismiss arm
///   can compare elapsed time against `TOAST_AUTODISMISS`.
///
/// Called by the `ShowToast`, `ShowToastWithSeverity`, and `LabToggleCompare`
/// arms in `update`.
fn enqueue_toast(model: &mut Cockpit, message: SmolStr, severity: ToastSeverity) {
    let id = model.toast_next_id.get();
    model.toast_next_id.set(id.wrapping_add(1));
    if model.toast_queue.len() == MAX_TOAST_QUEUE_LEN {
        model.toast_queue.pop_front();
    }
    model.toast_queue.push_back(ToastEntry {
        id,
        message,
        severity,
        created_at: Instant::now(),
    });
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
            // cockpit-live-trades-counter (TODO #2) — every fill counts toward
            // the session total, INCLUDING fills buffered while the tape
            // display is paused (pausing the display doesn't pause trading).
            // If the KPI strip is already Ready, update its Trades card
            // in place so the count is current the instant the fill lands,
            // not only at the next per-bar PnL rebuild.
            model.live_fill_count = model.live_fill_count.saturating_add(1);
            if let PanelState::Ready(m) = &mut model.live_kpi {
                m.trades = model.live_fill_count;
            }
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
            // cockpit-live-dashboard-wiring — append the live equity point
            // and rebuild the derived curve + KPI strip (D5) BEFORE moving
            // `snap` into `model.pnl`.
            //
            // Two timestamps (cockpit-live-equity-render-guard, approach A):
            // the WALLCLOCK `as_of` is the out-of-order-delivery guard key
            // (monotone), while the DATA-time `bar_ts` (fallback `as_of`) is
            // the x-axis coordinate the chart plots — so a fast replay shows
            // real 2023-24 dates instead of one repeated wallclock minute.
            let x_coord = snap.bar_ts.unwrap_or(snap.as_of);
            model.push_live_equity_point(snap.as_of, x_coord, snap.total_equity);
            model.pnl = PanelState::Ready(snap);
        }
        Message::PnlHydrated(rows) => {
            // live-equity-history-durable (A4 / A5) — boot-time batch hydrate of
            // the durable paper/live equity series. Seeds the buffer through the
            // SAME `push_live_equity_point` guard a live tick uses (so the
            // monotone-x clamp + ring + `is_all_absent` ≥2-point KPI trap all
            // apply identically), then reconciles the delivery guard per the
            // pinned `as_of` contract.
            //
            // An empty hydrate (no durable history — fresh ledger, or the
            // fail-soft `Ok(vec![])` path) is a no-op: the buffer stays empty,
            // both panels stay Loading, and the caption stays session-scoped.
            if rows.is_empty() {
                return;
            }

            // THE pinned guard contract (A4): after this hydrate,
            // `live_equity_last_as_of` MUST equal the MAX hydrated `as_of`, so
            // the FIRST live `PnlRefreshed(now())` (fresh wallclock ≥ every
            // prior-session `as_of`) passes the delivery guard and lands — while
            // a late/duplicate re-delivery of an already-hydrated row is dropped.
            //
            // We seed every row through the SAME `push_live_equity_point` used by
            // a live tick (so its monotone-x clamp, ring bound, and the
            // `is_all_absent` ≥2-point KPI trap all apply identically) — but we
            // pass the batch's MAX `as_of` as the delivery key for EVERY row.
            // This is the correct semantics for a boot batch: all rows arrived
            // together at boot, so none is "out of order" relative to another,
            // and the per-point `as_of` guard must therefore never drop a hydrate
            // row (even when the rows' own `as_of` values are non-monotone vs.
            // their `bar_ts` order — a backed-up-clock prior session). The PLOTTED
            // x-coordinate is each row's own `bar_ts` (the contract's x-axis); the
            // delivery key is uniformly the batch max. After the loop the guard is
            // pinned to that max by construction — no separate re-set needed.
            let max_as_of = rows
                .iter()
                .map(|(_, as_of, _)| *as_of)
                .max_by_key(trading_core::Timestamp::unix_millis);
            let Some(max_as_of) = max_as_of else {
                // `rows` is non-empty (guarded above), so `max` is always `Some`.
                // Defensive: an empty max means nothing to seed — bail without
                // flipping the hydrate switch.
                return;
            };
            for (bar_ts, _row_as_of, equity) in rows {
                model.push_live_equity_point(max_as_of, bar_ts, equity);
            }

            // The hydrated buffer is a continuous *since-inception* paper/live
            // history (may span sessions/days) — flip the honesty switch so the
            // Live screen's return caption reads "Since inception", not the
            // session-scoped "Session to date" (R6). `push_live_equity_point`
            // already built the curve `Ready` (≥1 row) and the KPI strip `Ready`
            // (≥2 rows) / `Loading` (1 row) — the `is_all_absent` trap is intact.
            model.live_equity_hydrated = true;
        }
        Message::PositionsRefreshed(list) => {
            model.positions = if list.is_empty() {
                PanelState::Empty
            } else {
                PanelState::Ready(list)
            };
        }
        Message::PnlError(e) => {
            // cockpit-live-dashboard-wiring — degrade the live curve + KPI
            // strip consistently with the P&L panel (no panic). The `pnl`
            // channel closing / erroring routes through here; both derived
            // panels render their muted error body.
            model.live_equity_curve = PanelState::Error(e.clone());
            model.live_kpi = PanelState::Error(e.clone());
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
            if let KillState::Confirming { typed } = &model.kill
                && typed == crate::strings::KILL_SAFETY_PHRASE
            {
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
            if let PanelState::Ready(rows) = &mut model.strategies
                && let Some(row) = rows.iter_mut().find(|r| r.id == id)
            {
                row.signals_60s = count;
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
            // R5.2 deep-link: deprecated Risk/Debug/Control aliases pre-select
            // the matching Settings tab on the way through (Design § A4).
            #[allow(deprecated)]
            match s {
                Screen::Risk => model.settings_active_tab = SettingsTab::Risk,
                Screen::Control => model.settings_active_tab = SettingsTab::Control,
                Screen::Debug => model.settings_active_tab = SettingsTab::Debug,
                _ => {}
            }
            // lab-run-save-compare T5 / R5 / Q6 — Compare cold-boot index. On
            // the FIRST navigation to Compare (cache un-indexed), scan the
            // two-root union (`lab-runs/` FIRST, then `spec/`) so persisted Lab
            // runs AND committed reports populate the matrix. Synchronous read
            // of small Markdown files (the existing `scan_spec_tree` budget,
            // R3.5); re-scans only after `invalidate_compare_index` clears the
            // tag (e.g. on a fresh Lab run completing).
            if s == Screen::Compare && model.compare_screen_state.last_indexed_at.is_none() {
                let roots = crate::lab::equity_loader::default_report_roots();
                model.compare_screen_state.cache = crate::compare::cache::scan_report_roots(&roots);
                model.compare_screen_state.last_indexed_at = Some(time::OffsetDateTime::now_utc());
            }
        }

        // ── Phase C — Settings rollup sub-tab ────────────────────────────────
        Message::SwitchSettingsTab(t) => {
            model.settings_active_tab = t;
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

        // ── Phase A — Lab screen (ui-rethink-phase-a-lab T-D-4) ─────────
        Message::LabSelectPair(venue, symbol) => {
            // R1.1 (lab-end-to-end-v2 T-D1.1) — keep `selected_symbol` in
            // sync so the chart's `chart_buffer.bars(...)` read at
            // screens/lab.rs:243 returns bars for the pair-chip-selected pair.
            // Phase A R3.3 closure. Clone before the move into `pair`.
            model.selected_symbol = Some((venue, symbol.clone()));
            model.lab_state.pair = Some((venue, symbol));
            // T-D-N10: tuple changed — clear both run report mirrors.
            model.lab_state.last_run_report = None;
            model.lab_state.prev_run_report = None;
        }
        Message::LabSelectPrimaryStrategy(id) => {
            model.lab_state.strategy = Some(id);
            // T-D-N10: tuple changed — clear both run report mirrors.
            model.lab_state.last_run_report = None;
            model.lab_state.prev_run_report = None;
        }
        Message::LabToggleCompare(id) => {
            // Returns `false` when cap hit — emit a toast (T-D-16 / R4.2 / R3.1).
            let changed = model.lab_state.toggle_compare(id);
            if !changed {
                // R3.1: migrate from direct field-write to enqueue with Warning severity.
                enqueue_toast(
                    model,
                    SmolStr::new(crate::strings::LAB_COMPARE_CAP_HIT),
                    ToastSeverity::Warning,
                );
            }
        }
        Message::LabSelectRange(range) => {
            model.lab_state.range = range;
            // T-D-N10: tuple changed — clear both run report mirrors.
            model.lab_state.last_run_report = None;
            model.lab_state.prev_run_report = None;
        }
        // lab-yahoo-realdata T-C3.5 / R3.1 — data-source toggle.
        Message::LabSelectDataSource(src) => {
            model.lab_state.data_source = src;
            // Clear run reports — data source change invalidates previous results.
            model.lab_state.last_run_report = None;
            model.lab_state.prev_run_report = None;
            // lab-yahoo-realdata v0.1.2 (T-DU3.5 / D-V0.1.2-1) — invalidate
            // and immediately re-populate the aggregate cache-state summary
            // so `view()` reads `&LabState` immutably (no RefCell on the
            // hot-path read). Bounded by ~30 stats on the 10-row mirror —
            // see `probe_summary` doc.
            model.lab_state.cache_summary = Some(crate::lab::cache_state::probe_summary(
                &crate::lab::cache_state::default_cache_root(),
                crate::lab::cache_state::ALL_YAHOO_TICKERS,
                std::time::SystemTime::now(),
            ));
        }
        // lab-polish-round-2 R2 — SMA param edits.
        // Validation: parse to `usize`; only update the typed `sma_*_len`
        // when the parse succeeds + the value is in [2, 500]. Empty string
        // resets to `None` (= back to compiled-in 20/50 default).
        Message::LabSetSmaFast(s) => {
            model.lab_state.sma_fast_input.clone_from(&s);
            model.lab_state.sma_fast_len = parse_sma_window(&s);
        }
        Message::LabSetSmaSlow(s) => {
            model.lab_state.sma_slow_input.clone_from(&s);
            model.lab_state.sma_slow_len = parse_sma_window(&s);
        }
        // Wave 2 (M2.5 / T-D-14) — run-inflight tracking.
        // Pure state: the binary side wires the Task::perform.
        Message::LabRunRequested => {
            model.lab_run_inflight = true;
            // R9.3 — clear stale progress from any prior run.
            model.lab_state.run_progress = None;
            // Bug #54 — clear stale error from previous failed run so the
            // Run button transitions Failed → Running cleanly.
            model.lab_state.last_run_error = None;
            // lab-yahoo-empty-range-ux v0.1.0 — D-ER-3 (M-DEV.9a): clear stale notice.
            model.lab_state.last_run_notice = None;
        }
        Message::LabRunCompleted(outcome) => {
            model.lab_run_inflight = false;
            // R9.3 — clear progress on run completion.
            model.lab_state.run_progress = None;
            // Bug #54 — track success/failure so Run button can render
            // RunState::Failed and the screen can show the error message
            // instead of silently flipping back to "Run".
            // lab-yahoo-empty-range-ux v0.1.0 — D-ER-3 (M-DEV.9b): replace the
            // flat Err→last_run_error assignment with the typed classifier so that
            // a sentinel-tagged no-data message routes to last_run_notice (muted)
            // and a genuine error routes to last_run_error (red ⚠).
            match &outcome {
                Ok(_) => {
                    model.lab_state.last_run_error = None;
                    model.lab_state.last_run_notice = None;
                    // lab-run-save-compare T5 / R5 — a successful run persists a
                    // new report under `lab-runs/`. Clear the Compare cold-boot
                    // tag so the next Compare navigation re-scans the two-root
                    // union and surfaces the just-persisted run (no restart
                    // needed). Cheap: the re-scan only fires on the next visit
                    // to Compare, not here.
                    model.compare_screen_state.last_indexed_at = None;
                }
                Err(raw) => {
                    use crate::lab::runner::preload_notice::{RunMessageKind, classify};
                    match classify(raw) {
                        RunMessageKind::Notice(msg) => {
                            model.lab_state.last_run_notice = Some(msg);
                            model.lab_state.last_run_error = None;
                        }
                        RunMessageKind::Error(msg) => {
                            model.lab_state.last_run_error = Some(msg);
                            model.lab_state.last_run_notice = None;
                        }
                    }
                }
            }
            // lab-yahoo-realdata v0.1.2 (T-DU3.5 / D-V0.1.2-1) — invalidate +
            // re-populate the aggregate cache summary on Lab-Run-complete.
            // The Lab run does not write cache mtimes itself
            // (`fetch_yahoo_klines` runs externally), but operators commonly
            // run a fetch + a backtest back-to-back; recomputing here means
            // the next badge render reflects the most-recent on-disk state
            // without forcing `view()` to take a `RefCell` or recompute.
            model.lab_state.cache_summary = Some(crate::lab::cache_state::probe_summary(
                &crate::lab::cache_state::default_cache_root(),
                crate::lab::cache_state::ALL_YAHOO_TICKERS,
                std::time::SystemTime::now(),
            ));
            // T-D-N10: The equity cache invalidation + repaint is triggered by
            // the binary-side `update` wrapper after pure-state `update` returns.
            // Pure state only clears the inflight flag here.
            // NOTE: RunReportMirror rotation (last→prev, set last=new) is done
            // by the binary-side update wrapper which has access to the full
            // RunSummary + equity series. The pure update cannot build a
            // RunReportMirror because it has no equity data or BacktestKpis
            // (those come from the async run result stored in the binary layer).
        }
        // T-D3.4 / R6.3 — Stop button: pure state is unchanged (inflight stays
        // true until LabRunCompleted arrives). The binary-side wrapper drops
        // the cancel handle which fires cancel at the engine's next poll.
        Message::LabRunStopRequested => {
            // Pure state: no change needed here. The binary-side wrapper in
            // cockpit_live.rs::update drops `lab_state.run_cancel`.
        }
        // T-D4.6 / R9 — progress update from the in-flight engine.
        Message::LabRunProgress(progress) => {
            model.lab_state.run_progress = Some(progress);
        }
        // T-D4.6 / R9.4 — channel closed; belt-and-suspenders clear.
        Message::LabRunProgressDone => {
            model.lab_state.run_progress = None;
        }
        Message::ShowToast(msg) => {
            // Back-compat: ShowToast maps to Info severity (ADR-0046 § Back-compat).
            enqueue_toast(model, msg, ToastSeverity::Info);
        }
        Message::DismissToast => {
            // Dismiss FRONT entry (oldest visible) — back-compat for DismissToast callers.
            model.toast_queue.pop_front();
        }
        // ── cockpit-toast-queue v0.1.0 — new toast arms ──────────────────────
        Message::ShowToastWithSeverity(msg, sev) => {
            enqueue_toast(model, msg, sev);
        }
        Message::DismissToastById(id) => {
            model.toast_queue.retain(|t| t.id != id);
        }
        Message::ToastTick(now) => {
            // Auto-dismiss: drop entries whose age exceeds TOAST_AUTODISMISS.
            // `now` is the Instant carried by the message — not Instant::now()
            // inside this arm (K5 clock-injection via payload per ADR-0046).
            model
                .toast_queue
                .retain(|t| now.duration_since(t.created_at) < TOAST_AUTODISMISS);
        }

        // ── Training panel — cockpit-training-control T-D-N4 ─────────────────
        Message::TrainingPressed => {
            // Actual subprocess spawn lives in the binary (needs rt_handle).
            // The update fn is pure — the Message is dispatched from the binary's
            // `update` wrapper which calls `lab::trainer::spawn_training_run`
            // before forwarding. Here we just ensure state is consistent.
            // (No-op if already in-flight — button is disabled in that case.)
        }
        Message::TrainingCancelPressed => {
            // Drop the handle — Drop impl calls start_kill() → SIGKILL.
            model.lab_state.training_inflight = None;
        }
        Message::TrainingLogLine(line) => {
            crate::widgets::training_log::push_line(&mut model.lab_state.training_log, line);
        }
        Message::TrainingExited(_status) => {
            // Subprocess has exited; clear the inflight handle.
            model.lab_state.training_inflight = None;
        }
        Message::TrainingPanelToggled => {
            model.lab_state.training_panel_collapsed = !model.lab_state.training_panel_collapsed;
        }
        Message::TrainingClearLog => {
            model.lab_state.training_log.clear();
        }
        Message::TrainingLogClicked => {
            model.lab_state.training_log_anchored = false;
        }
        Message::TrainingLogJumpToBottom => {
            model.lab_state.training_log_anchored = true;
        }
        #[cfg(feature = "live")]
        Message::TrainingEventsRefreshed(rows) => {
            // Append new rows to the ring buffer; evict oldest when over capacity.
            const TRAINING_EVENTS_CAPACITY: usize = 1024;
            for row in rows {
                model.lab_state.training_events.push_back(row);
                while model.lab_state.training_events.len() > TRAINING_EVENTS_CAPACITY {
                    model.lab_state.training_events.pop_front();
                }
            }
        }

        // ── Phase D — Trail view (ui-rethink-phase-d-trail T-D-N15, N17) ─────
        Message::TrailNodeChevronClicked(kind) => {
            // Chevron on a trail node → open / toggle the side-drawer (R4.3).
            model.trail_screen_state.drawer_selected_node = Some(kind);
        }
        Message::SelectTrailRow(id) => {
            // Internal: select a trail row by audit_id; part of compound dispatch
            // from `OpenTrailFor`. Not emitted directly from UI widgets (R5.3).
            // Empty SmolStr is the "back to list" sentinel emitted by the
            // breadcrumb button in `screens::trail` — clears the selection.
            if id.is_empty() {
                model.trail_screen_state.selected_audit_id = None;
                model.trail_screen_state.drawer_selected_node = None;
            } else {
                model.trail_screen_state.selected_audit_id = Some(id);
            }
        }
        Message::TrailDrawerClosed => {
            // Dismiss drawer; preserve selected_audit_id so trail stack stays visible.
            model.trail_screen_state.drawer_selected_node = None;
        }
        Message::OpenTrailFor(id) => {
            // Compound dispatch per R5.1 + Phase C `OpenStrategyInLab` precedent.
            // Expands to: SelectTrailRow(id) + SwitchScreen(Trail).
            // Phase D+ (R1.4): also set pending_trail_audit_id so the trail-mode
            // body renders a loading placeholder until the mirror responds.
            model.trail_screen_state.pending_trail_audit_id = Some(id.clone());
            model.trail_screen_state.selected_audit_id = Some(id);
            model.current_screen = Screen::Trail;
        }
        Message::TrailMirrorTick(tick) => {
            // Phase D+ — Subscription bridge tick (ui-rethink-phase-d-trail-followup R1.2).
            // Two real branches per Q2 (b) resolution:
            //   TrailReady   → hydrate trail_screen_state.reconstructed_trail + clear pending
            //   TrailUpdated → mark cached audit_id stale (flag for re-fetch on next render)
            match tick {
                TrailMirrorUiTick::TrailReady(boxed_trail) => {
                    model.trail_screen_state.pending_trail_audit_id = None;
                    model.trail_screen_state.reconstructed_trail = Some(*boxed_trail);
                }
                TrailMirrorUiTick::TrailUpdated(_audit_id) => {
                    // Steady-state update: the trail for this audit_id has new data.
                    // Clear reconstructed_trail so the next render re-fetches from the
                    // mirror via TrailMirrorRequest::Open (R1.2 stale-flag policy).
                    model.trail_screen_state.reconstructed_trail = None;
                }
            }
        }

        // ── Phase F — Memory + Models + Assistant (ui-rethink-phase-f-memory-models-assistant T-D-N6) ──
        Message::MemoryHydrate(cards) => {
            // Cold-boot hydrate: store the lesson cards + record indexed timestamp.
            let now = time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| String::from("unknown"));
            model.memory_screen_state.cache = cards;
            model.memory_screen_state.last_indexed = Some(smol_str::SmolStr::new(&now));
        }
        Message::MemoryOpenDrawer(card_id) => {
            model.memory_screen_state.drawer_open = Some(card_id);
        }
        Message::MemoryCloseDrawer => {
            model.memory_screen_state.drawer_open = None;
        }
        Message::MemoryToggleMode(mode) => {
            model.memory_screen_state.mode = mode;
        }
        Message::MemorySetFilter(filter) => {
            model.memory_screen_state.filter = filter;
        }
        Message::ModelsHydrate(checkpoints) => {
            // Cold-boot hydrate: store the checkpoint list + record indexed timestamp.
            let now = time::OffsetDateTime::now_utc()
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| String::from("unknown"));
            model.models_screen_state.checkpoints = checkpoints;
            model.models_screen_state.last_indexed = Some(smol_str::SmolStr::new(&now));
        }
        Message::ModelsSetFamilyFilter(filter) => {
            model.models_screen_state.family_filter = filter;
        }
        Message::ModelsSetStatusFilter(filter) => {
            model.models_screen_state.status_filter = filter;
        }
        Message::ToggleAssistantSlot => {
            // K6 Option A: flip is_open; shell picks RIGHT_RAIL_OPEN_WIDTH_PX vs 0.0.
            model.assistant_state.is_open = !model.assistant_state.is_open;
        }
        Message::AssistantReasoningTraceUpdate(view) => {
            // v3-llm-forecaster Wave F T-D-N(F3) — new forecast arrived.
            // Prepend previous `last_forecast` onto history (most-recent
            // first); cap at HISTORY_CAP. Mode is NOT flipped here — it
            // is set once at boot by `cockpit_live` when it observes the
            // `llm_forecaster_v3` strategy enabled (R9.3 runtime gate).
            if let Some(prev) = model.assistant_state.last_forecast.take() {
                model.assistant_state.history.insert(0, prev);
                if model.assistant_state.history.len() > crate::assistant::state::HISTORY_CAP {
                    model
                        .assistant_state
                        .history
                        .truncate(crate::assistant::state::HISTORY_CAP);
                }
            }
            model.assistant_state.last_forecast = Some(view);
        }

        // ── Phase E — Compare matrix (ui-rethink-phase-e-compare T-D-N3) ─────
        Message::OpenLabFromCompare {
            strategy,
            pair,
            range,
        } => {
            // Compound dispatch per R4.1 + K4 mitigation.
            // Order is FIXED: screen + strategy first (SelectStrategy clears
            // last_run_report at state.rs:~1793-1799); then pair; then range.
            // Mirrors OpenTrailFor at state.rs:1902-1910 verbatim.
            model.current_screen = Screen::Lab;
            model.selected_strategy = Some(strategy.clone());
            model.lab_state.strategy = Some(strategy);
            // Clear last_run_report (K4 mitigation — SelectStrategy semantics).
            model.lab_state.last_run_report = None;
            model.lab_state.prev_run_report = None;
            if let Some((venue, symbol)) = pair {
                model.lab_state.pair = Some((venue, symbol));
            }
            model.lab_state.range = range.clone();
            // R3.4 isolation: also write the range to compare_screen_state
            // so subsequent Compare-screen opens reflect the last-used range.
            model.compare_screen_state.range = range;
        }
        Message::CompareSelectRange(range) => {
            // R3.4: pure assignment to compare_screen_state.range ONLY.
            // MUST NOT touch lab_state.range.
            model.compare_screen_state.range = range;
        }
        Message::CompareSelectKpiAxis(axis) => {
            // R6.3: v0.1.0 wires Sharpe only; other variants accepted but
            // the view falls back to Sharpe (tracing::warn! below).
            #[allow(clippy::wildcard_enum_match_arm)]
            if axis != crate::compare::state::CompareKpiAxis::Sharpe {
                tracing::warn!(
                    "CompareSelectKpiAxis: axis {:?} is not wired at v0.1.0 — falling back to Sharpe",
                    axis
                );
            }
            model.compare_screen_state.kpi_axis = axis;
        }
        Message::CompareToggleOverlay(slot) => {
            // lab-compare-equity-overlay T2 (Q1): add/remove the cell from the
            // two-run equity-overlay ring (bounded at OVERLAY_CAP; rotate-oldest
            // on overflow). The single mutation point lives on the state struct
            // so this arm stays a one-liner.
            model.compare_screen_state.toggle_overlay(slot);
        }

        // ── cockpit-baseline-panel v0.1.0 — year toggle ──────────────────────
        Message::BaselineSelectYear(year) => {
            // R2 — pure assignment. The curves for both years are already
            // loaded (boot-load via `baseline::state::load_into`); the view
            // pulls the active year's curve + the `const` metrics. No I/O.
            model.baseline_screen_state.active_year = year;
        }

        // ── cockpit-reports-viewer v0.1.0 — report picker ────────────────────
        Message::ReportsSelect(idx) => {
            // R1/R2 — record the selection + synchronously load the report
            // (parse the `## Summary` table + scan for the companion CSV).
            // Lifted loader; never panics (parse-miss → the loaded field's
            // metrics PanelState carries Error; vanished file → loaded Error).
            // The files are small (<100 ms parse), so no async `Task` —
            // synchronous in the arm, matching the Baseline precedent.
            model.reports_screen_state.selected = Some(idx);
            model.reports_screen_state.load_selection(idx);
        }

        // ── reports-picker-curve-filter — picker rail filter toggle ──────────
        Message::ReportsToggleShowAll => {
            // Pure flag flip — no I/O, no reload. The picker view re-filters
            // the FULL discovered list at render time; the current selection
            // (a full-list index) is untouched, so the detail pane keeps
            // showing whatever is loaded. Curve-only (false) is the default;
            // this flips to show every discovered report and back.
            let st = &mut model.reports_screen_state;
            st.show_all_reports = !st.show_all_reports;
        }

        // ── advisor-leaderboard-screen v0.1.0 — strategy bake-off ────────────
        Message::BakeoffRunRequested => {
            // Pure-state half: flip to Loading + set the in-flight token so the
            // Run button disables and the spinner shows. The actual async
            // dispatch (cancel/progress pair + `spawn_bakeoff`) is wired
            // BINARY-side in `cockpit_live.rs` (the `LabRunRequested` precedent)
            // — `update` stays pure (no I/O, no Task spawn from inside update).
            // Guard against double-dispatch while a run is already in flight.
            if !model.leaderboard_screen_state.running {
                model.leaderboard_screen_state.begin_run();
            }
        }
        Message::BakeoffRunCompleted(outcome) => {
            // Land the mirrored result (Ready / Empty / Error) + clear running.
            // The engine `BakeoffReport` was already mirrored into the pure-`ui`
            // `BakeoffReportMirror` at the dispatch boundary in `spawn_bakeoff`.
            model.leaderboard_screen_state.finish_run(outcome);
        }

        // ── advisor-bakeoff-ranking F3 — guided input ────────────────────────
        Message::BakeoffSelectCoin(symbol) => {
            // The chosen coin drives the NEXT bake-off + the header context.
            // We do NOT clear the existing result — the operator may be
            // comparing the prior coin's leaderboard while picking the next;
            // pressing Run re-ranks for the new coin.
            model.leaderboard_screen_state.coin = symbol;
        }
        Message::BakeoffSetBudget(raw) => {
            // Store the keystrokes verbatim (parse is a render-time concern).
            // Budget is context only — it does not invalidate the ranking.
            model.leaderboard_screen_state.budget_input = raw;
        }
        Message::BakeoffSelectLookback(lookback) => {
            model.leaderboard_screen_state.lookback = lookback;
        }

        // ── cockpit-activity-status-bar v0.1.0 Wave B (T-D-N4) ───────────────
        Message::ActivityEventReceived(event) => {
            // Delegate to ActivityTape::apply — O(1) for Start/Tick;
            // O(32) for End. All are well within the iced update budget.
            model.activity_tape.apply(event);
        }
        Message::ActivityTapePurgeTick => {
            // Remove expired red-hold rows (Q5=(a) 3-second hold).
            // Called by the 1 Hz purge tick subscription.
            model.activity_tape.purge(std::time::Instant::now());
        }
        Message::ForwardPaperTradeStarted(budget) => {
            // F5 — store the forward-run budget so the Live screen can render
            // P/L = equity − budget.  Budget=None / forward=None paths never
            // emit this message; those runs leave forward_budget = None and
            // the Live screen falls back to raw equity display (pre-F5 behaviour).
            model.forward_budget = Some(budget);
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
    if let PanelState::Ready(rows) = &mut model.strategies
        && let Some(row) = rows.iter_mut().find(|r| r.id == id)
    {
        row.status = StrategyStatus::Error(ev.error_summary.clone());
        row.last_event = Some(view);
        return;
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
            let PanelState::Ready(fills) = &model.chart_markers else {
                return None;
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
            let PanelState::Ready(signals) = &model.chart_signals else {
                return None;
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
#[allow(deprecated)] // tests that exercise deprecated Screen aliases are intentional backward-compat checks
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
            bar_ts: None,
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

    // ── cockpit-live-dashboard-wiring v0.1.0 — Live equity curve + KPI strip ──

    /// Build a `PnlSnapshot` at a chosen `(secs, equity)` so live-curve tests
    /// can drive a deterministic monotone (or out-of-order) sequence.
    ///
    /// `bar_ts` is left `None` here, so the buffer's stored x-coordinate falls
    /// back to `as_of` (= the `secs` timestamp) AND the delivery guard keys on
    /// that same value — preserving the pre-approach-A behavior these tests
    /// assert. The `bar_ts`-driven x-axis path is covered separately by
    /// `live_equity_curve_plots_bar_ts_not_wallclock` + the render harness.
    fn pnl_snap_at(secs: i64, equity: Decimal) -> PnlSnapshot {
        let as_of =
            Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs));
        PnlSnapshot {
            cash: Money::<Usdt>::from_decimal(equity),
            unrealized: Money::<Usdt>::from_decimal(dec!(0)),
            realized: Money::<Usdt>::from_decimal(dec!(0)),
            total_equity: Money::<Usdt>::from_decimal(equity),
            daily_return: Money::<Usdt>::from_decimal(dec!(0)),
            as_of,
            bar_ts: None,
        }
    }

    /// AC1 (core wiring proof) — a sequence of `PnlRefreshed` messages
    /// populates the live equity curve point-by-point, and the curve
    /// transitions Loading → Ready at the **first** point.
    #[test]
    fn pnl_refresh_sequence_populates_live_equity_curve() {
        let mut c = Cockpit::new();
        // Fresh boot: both live panels Loading, buffer empty.
        assert!(c.live_equity_buffer.is_empty());
        assert_eq!(c.live_equity_curve.variant_name(), "loading");
        assert_eq!(c.live_kpi.variant_name(), "loading");

        // Point 1 — curve goes Ready (1-point curve is valid); strip stays
        // Loading (the is_all_absent 1-point trap).
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(0, dec!(1000))));
        assert_eq!(c.live_equity_buffer.len(), 1);
        assert_eq!(c.live_equity_curve.variant_name(), "ready");
        assert_eq!(c.live_kpi.variant_name(), "loading");

        // Points 2..=5 — curve grows one point per snapshot; the cached
        // series length tracks the buffer.
        for (i, eq) in [(60i64, dec!(1010)), (120, dec!(1025)), (180, dec!(1005))]
            .into_iter()
            .enumerate()
        {
            update(&mut c, Message::PnlRefreshed(pnl_snap_at(eq.0, eq.1)));
            assert_eq!(c.live_equity_buffer.len(), i + 2);
        }
        assert_eq!(c.live_equity_buffer.len(), 4);
        match &c.live_equity_curve {
            PanelState::Ready(series) => assert_eq!(series.points.len(), 4),
            other => panic!("expected Ready curve, got {}", other.variant_name()),
        }
    }

    /// approach-A core proof (cockpit-live-equity-render-guard, 2026-06-11) —
    /// the live equity buffer plots the **data/bar** time (`bar_ts`) on its
    /// x-axis, while the out-of-order-delivery guard keys on the **wallclock**
    /// `as_of`. This split is what makes a fast replay show real 2023-24 dates
    /// instead of one repeated wallclock minute. The reverted I1 conflated the
    /// two (stamped `as_of` with bar time) and emptied the curve; this pins them
    /// SEPARATE. Complements the rasterized `tests/live_equity_render.rs`
    /// harness (which proves the curve actually draws).
    #[test]
    fn live_equity_curve_plots_bar_ts_not_wallclock() {
        // A snapshot whose wallclock `as_of` and data `bar_ts` live in DISJOINT
        // epoch ranges, so a mix-up of the two is unmistakable.
        fn snap_split(as_of_secs: i64, bar_secs: i64, equity: Decimal) -> PnlSnapshot {
            PnlSnapshot {
                cash: Money::<Usdt>::from_decimal(equity),
                unrealized: Money::<Usdt>::from_decimal(dec!(0)),
                realized: Money::<Usdt>::from_decimal(dec!(0)),
                total_equity: Money::<Usdt>::from_decimal(equity),
                daily_return: Money::<Usdt>::from_decimal(dec!(0)),
                as_of: Timestamp::new(
                    time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(as_of_secs),
                ),
                bar_ts: Some(Timestamp::new(
                    time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(bar_secs),
                )),
            }
        }

        let mut c = Cockpit::new();

        // `as_of`: a ~2026 monotone wallclock sequence (the delivery key).
        // `bar_ts`: a 2023 monotone data sequence (the plotted x-coord).
        let wall_base: i64 = 1_749_600_000; // ~2026-06-11 (wallclock now-ish)
        let bar_base: i64 = 1_673_789_400; // 2023-01-15 12:30:00 UTC
        // (as_of_secs, bar_secs, equity) — disjoint epoch ranges, both monotone.
        let rows: [(i64, i64, Decimal); 3] = [
            (wall_base, bar_base, dec!(100000)),
            (wall_base + 1, bar_base + 60, dec!(100800)),
            (wall_base + 2, bar_base + 120, dec!(101500)),
        ];
        for &(as_of_secs, bar_secs, eq) in &rows {
            update(
                &mut c,
                Message::PnlRefreshed(snap_split(as_of_secs, bar_secs, eq)),
            );
        }

        // The buffer's stored x-coordinates are the DATA times (`bar_ts`)…
        let stored: Vec<i64> = c
            .live_equity_buffer
            .iter()
            .map(|(ts, _)| ts.unix_millis())
            .collect();
        let expect_bar: Vec<i64> = rows
            .iter()
            .map(|&(_, bar_secs, _)| bar_secs * 1000)
            .collect();
        assert_eq!(
            stored, expect_bar,
            "equity buffer must plot bar_ts (2023 data time) on its x-axis"
        );

        // …and NEVER the wallclock `as_of` (the I1 conflation this guards against).
        let wall_millis: Vec<i64> = rows
            .iter()
            .map(|&(as_of_secs, _, _)| as_of_secs * 1000)
            .collect();
        assert_ne!(
            stored, wall_millis,
            "stored x-coords must be bar_ts, never the wallclock as_of"
        );

        // The delivery guard tracked the latest wallclock `as_of`, kept separate
        // from the plotted coordinate.
        assert_eq!(
            c.live_equity_last_as_of.map(|t| t.unix_millis()),
            Some((wall_base + 2) * 1000),
            "delivery guard must track the latest wallclock as_of"
        );
    }

    /// AC2 (the `is_all_absent` proof) — the KPI strip stays Loading at 1 point
    /// and becomes Ready only at ≥2 points, with live Total-return + Max-DD
    /// and absent Sharpe/CAGR/Win-rate + Trades = 0.
    #[test]
    fn live_kpi_strip_loading_at_one_point_ready_at_two() {
        let mut c = Cockpit::new();

        // 1 point: strip Loading (would otherwise be byte-identical to the
        // all-absent six-dash sentinel).
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(0, dec!(1000))));
        assert_eq!(c.live_kpi.variant_name(), "loading");

        // 2 points: strip Ready with a real session delta.
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(60, dec!(1100))));
        match &c.live_kpi {
            PanelState::Ready(m) => {
                // Session return = (1100 − 1000) / 1000 = 0.10 fraction →
                // ×100 = 10.00 PERCENT (the units `BacktestMetrics` /
                // `format_pct_sentiment` expect; renders "+10.00%", not the
                // 100×-too-small "0.10%" the operator saw — cockpit-live-
                // kpi-units-fix 2026-06-10).
                assert_eq!(m.total_return_pct, dec!(10));
                // Max DD = 0 (monotone up so far) — but present (real).
                assert_eq!(m.max_drawdown_pct, Decimal::ZERO);
                // Trades = 0, and Sharpe/CAGR/Win-rate are absent (`—`).
                assert_eq!(m.trades, 0);
                assert!(!m.sharpe_present);
                assert!(!m.cagr_present);
                assert!(!m.win_rate_present);
            }
            other => panic!("expected Ready strip, got {}", other.variant_name()),
        }

        // A non-zero session metric makes the strip distinguishable from the
        // all-absent sentinel — the kpi_strip widget's `is_all_absent` guard
        // would not mask it.
        assert!(!matches!(c.live_kpi, PanelState::Loading));
    }

    /// cockpit-live-trades-counter (TODO #2, 2026-06-11) — the KPI strip's
    /// "Trades" card shows the SESSION fill total (`live_fill_count`), not the
    /// tape window (`tape.len()` is a capped/evicting deque). Fills count
    /// immediately (in-place update on a Ready strip), survive the per-bar
    /// KPI rebuild, and include fills received while the tape display is
    /// paused (pausing the display doesn't pause trading).
    #[test]
    fn live_trades_counter_counts_session_fills() {
        use trading_core::{FeeTier, Price, Quantity, Side, Symbol};
        fn fill(id: u64) -> FillView {
            FillView {
                symbol: Symbol::new("BTCUSDT"),
                side: Side::Buy,
                price: Price::new(dec!(100) + Decimal::from(id)).unwrap_or_else(|_| unreachable!()),
                qty: Quantity::new(dec!(1)).unwrap_or_else(|_| unreachable!()),
                fee: Money::from_decimal(dec!(0)),
                fee_tier: FeeTier::Taker,
                venue_ts: Timestamp::now(),
                transaction_id: smol_str::SmolStr::default(),
            }
        }

        let mut c = Cockpit::new();
        // Two equity points → strip Ready, trades = 0 (no fills yet).
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(0, dec!(1000))));
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(60, dec!(1100))));
        match &c.live_kpi {
            PanelState::Ready(m) => assert_eq!(m.trades, 0),
            other => panic!("expected Ready strip, got {}", other.variant_name()),
        }

        // Three fills — the middle one while the tape display is paused.
        update(&mut c, Message::FillReceived(fill(1)));
        update(&mut c, Message::TapePauseToggled);
        update(&mut c, Message::FillReceived(fill(2)));
        update(&mut c, Message::TapePauseToggled);
        update(&mut c, Message::FillReceived(fill(3)));
        assert_eq!(c.live_fill_count, 3, "paused fills must still count");
        // In-place update: the card is current BEFORE the next PnL rebuild.
        match &c.live_kpi {
            PanelState::Ready(m) => assert_eq!(m.trades, 3),
            other => panic!("expected Ready strip, got {}", other.variant_name()),
        }

        // The per-bar KPI rebuild carries the counter (doesn't reset it).
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(120, dec!(1200))));
        match &c.live_kpi {
            PanelState::Ready(m) => assert_eq!(m.trades, 3),
            other => panic!("expected Ready strip, got {}", other.variant_name()),
        }
    }

    /// AC2 — live Max-DD is real: drive a drawdown and assert the strip's
    /// `max_drawdown_pct` reflects it (free from `EquitySeries::from_points`).
    #[test]
    fn live_kpi_strip_max_drawdown_is_live() {
        let mut c = Cockpit::new();
        // 1000 → 1200 (peak) → 900 (trough) → max DD = (1200−900)/1200 = 0.25.
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(0, dec!(1000))));
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(60, dec!(1200))));
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(120, dec!(900))));
        match &c.live_kpi {
            PanelState::Ready(m) => {
                // Max DD = (1200−900)/1200 = 0.25 fraction → ×100 = 25.00
                // PERCENT (renders "−25.00%", not "−0.25%").
                assert_eq!(m.max_drawdown_pct, dec!(25));
                // Session return = (900 − 1000) / 1000 = −0.10 fraction →
                // ×100 = −10.00 PERCENT (negative, live).
                assert_eq!(m.total_return_pct, dec!(-10));
            }
            other => panic!("expected Ready strip, got {}", other.variant_name()),
        }
    }

    /// cockpit-live-kpi-units-fix (2026-06-10) — PIN the percent-unit
    /// semantics end-to-end through the KPI strip's actual formatters.
    ///
    /// Regression guard for the operator-reported "Total return 0.01–0.02"
    /// bug: a fraction was fed where `BacktestMetrics`/`format_pct_sentiment`
    /// expect percent units, so a real +1.5 % session rendered as "0.01%"
    /// (100× too small). This test drives the production `PnlRefreshed` path
    /// (equity 100k → 101.5k, a +1.5 % session, then a dip to 99k for a live
    /// Max-DD) and asserts the *rendered card text* — not just the raw
    /// Decimal — so a future reversion of the ×100 is caught at the surface
    /// the operator actually reads.
    #[test]
    fn live_kpi_units_render_percent_not_fraction() {
        use crate::theme::ThemeMode;
        use crate::widgets::num::{format_pct_max_dd, format_pct_sentiment};
        let mode = ThemeMode::Dark;

        let mut c = Cockpit::new();
        // 100_000 → 101_500: a +1.5 % session (the operator's real magnitude).
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(0, dec!(100000))));
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(60, dec!(101500))));
        match &c.live_kpi {
            PanelState::Ready(m) => {
                // Raw field is in PERCENT units now.
                assert_eq!(
                    m.total_return_pct,
                    dec!(1.5),
                    "raw return must be 1.5 (percent), not 0.015"
                );
                // The card renders "1.50%", NEVER the 100×-too-small "0.01%"
                // / "0.02%" the operator saw.
                let (tr_text, _) = format_pct_sentiment(m.total_return_pct, mode);
                assert_eq!(tr_text, "1.50%");
                assert_ne!(tr_text, "0.01%");
                assert_ne!(tr_text, "0.02%");
            }
            other => panic!("expected Ready strip, got {}", other.variant_name()),
        }

        // Drive a dip to 99_000 → peak 101_500, trough 99_000:
        // Max-DD = (101_500 − 99_000) / 101_500 = 0.02463… fraction
        // → ×100 = 2.4630…% — the card must read "−2.46%", not "−0.02%".
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(120, dec!(99000))));
        match &c.live_kpi {
            PanelState::Ready(m) => {
                let (mdd_text, _) = format_pct_max_dd(m.max_drawdown_pct, mode);
                // Max-DD is in percent units (≈ 2.46), rendered "−2.46%".
                assert_eq!(mdd_text, "\u{2212}2.46%");
                assert_ne!(mdd_text, "\u{2212}0.02%");
                // Return now negative: (99_000 − 100_000)/100_000 = −1.0 %.
                let (tr_text, _) = format_pct_sentiment(m.total_return_pct, mode);
                assert_eq!(tr_text, "\u{2212}1.00%");
            }
            other => panic!("expected Ready strip, got {}", other.variant_name()),
        }
    }

    /// T3 must-honor — the monotone guard drops a strictly-earlier late
    /// snapshot (so `from_points` never errors on out-of-order input); an
    /// equal-timestamp snapshot is allowed (appends fine).
    #[test]
    fn live_equity_buffer_drops_out_of_order_and_allows_equal_ts() {
        let mut c = Cockpit::new();
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(60, dec!(1000))));
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(120, dec!(1010))));
        assert_eq!(c.live_equity_buffer.len(), 2);

        // Strictly-earlier `as_of` (30 < 120) — dropped, length unchanged,
        // curve stays Ready (no NonMonotoneTimestamps error).
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(30, dec!(9999))));
        assert_eq!(c.live_equity_buffer.len(), 2);
        assert_eq!(c.live_equity_curve.variant_name(), "ready");

        // Equal-timestamp (120 == 120) — allowed by `from_points`'s `<` check.
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(120, dec!(1020))));
        assert_eq!(c.live_equity_buffer.len(), 3);
        assert_eq!(c.live_equity_curve.variant_name(), "ready");
    }

    /// AC2 (Error, no panic) — `PnlError` drives BOTH the live curve and the
    /// KPI strip to `Error`, alongside the existing `model.pnl` error.
    #[test]
    fn pnl_error_drives_live_panels_to_error_no_panic() {
        let mut c = Cockpit::new();
        // Seed a couple points so both panels are Ready first.
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(0, dec!(1000))));
        update(&mut c, Message::PnlRefreshed(pnl_snap_at(60, dec!(1100))));
        assert_eq!(c.live_equity_curve.variant_name(), "ready");
        assert_eq!(c.live_kpi.variant_name(), "ready");

        update(&mut c, Message::PnlError("pnl channel closed".into()));
        assert_eq!(c.pnl.variant_name(), "error");
        assert_eq!(c.live_equity_curve.variant_name(), "error");
        assert_eq!(c.live_kpi.variant_name(), "error");
    }

    /// R3 / D-buffer — the buffer is a bounded ring: past
    /// `LIVE_EQUITY_BUFFER_CAP` it evicts the oldest (`pop_front`), so the
    /// length never exceeds the cap and the newest point survives.
    #[test]
    fn live_equity_buffer_is_bounded_ring() {
        let mut c = Cockpit::new();
        // Push cap + 5 monotone points; assert the buffer caps and the oldest
        // evicts (front advances past ts=0).
        #[allow(clippy::cast_possible_wrap)] // test-only: cap is tiny, no wrap risk
        for i in 0..(LIVE_EQUITY_BUFFER_CAP as i64 + 5) {
            update(
                &mut c,
                Message::PnlRefreshed(pnl_snap_at(i * 60, dec!(1000) + Decimal::from(i))),
            );
        }
        assert_eq!(c.live_equity_buffer.len(), LIVE_EQUITY_BUFFER_CAP);
        // Oldest five evicted — the front is no longer ts=0.
        let front_ts = c.live_equity_buffer.front().expect("non-empty").0;
        assert!(front_ts.unix_millis() > 0);
        assert_eq!(c.live_equity_curve.variant_name(), "ready");
    }

    /// R1 / D5 — the live panels are session-scoped: a fresh `Cockpit` (a new
    /// `cockpit_live` boot) starts with an empty buffer and both panels
    /// Loading. (Reset = `Cockpit::new()`; the buffer is not serialized.)
    #[test]
    fn live_panels_reset_on_fresh_cockpit() {
        let c = Cockpit::new();
        assert!(c.live_equity_buffer.is_empty());
        assert_eq!(c.live_equity_curve.variant_name(), "loading");
        assert_eq!(c.live_kpi.variant_name(), "loading");
        // live-equity-history-durable — un-hydrated on a fresh boot.
        assert!(!c.live_equity_hydrated);
    }

    // ── live-equity-history-durable (T7-contract) — PnlHydrated batch arm ──

    /// Build a durable-hydrate tail row `(bar_ts, as_of, equity)` from realistic
    /// **2023-era** `bar_ts` data times paired with **prior-session** wallclock
    /// `as_of` stamps (all ≤ `now()`), exactly the shape
    /// `audit::query::equity_snapshot_tail` returns. The two timestamps live in
    /// disjoint, plausible epoch ranges so the guard-reconciliation logic is
    /// exercised against true two-timestamp rows (not the test shorthand where
    /// `bar_ts == as_of`).
    fn hydrate_row(
        bar_secs: i64,
        as_of_secs: i64,
        equity: Decimal,
    ) -> (Timestamp, Timestamp, Money<Usdt>) {
        (
            Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(bar_secs)),
            Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(as_of_secs)),
            Money::<Usdt>::from_decimal(equity),
        )
    }

    /// A monotone ≥2-row hydrate tail: 2023 `bar_ts` data times (2023-01-15
    /// onward), prior-session `as_of` wallclock stamps (~2026-05, all in the
    /// past relative to a live `now()`).
    fn hydrate_tail() -> Vec<(Timestamp, Timestamp, Money<Usdt>)> {
        let bar_base: i64 = 1_673_789_400; // 2023-01-15 12:30:00 UTC
        let wall_base: i64 = 1_746_000_000; // ~2025-04-30 (a prior paper session)
        vec![
            hydrate_row(bar_base, wall_base, dec!(100000)),
            hydrate_row(bar_base + 60, wall_base + 60, dec!(100800)),
            hydrate_row(bar_base + 120, wall_base + 120, dec!(101500)),
            hydrate_row(bar_base + 180, wall_base + 180, dec!(101200)),
            hydrate_row(bar_base + 240, wall_base + 240, dec!(102600)),
        ]
    }

    /// **AC4 (model).** A boot-time `PnlHydrated` of M≥2 rows seeds the buffer
    /// to `min(M, cap)`, plots each row's `bar_ts` (not `as_of`) on the x-axis,
    /// and brings BOTH the curve and the KPI strip up `Ready` — all BEFORE any
    /// live `PnlRefreshed`. The since-inception caption switch flips on.
    #[test]
    fn pnl_hydrated_seeds_buffer_curve_and_strip_ready() {
        let mut c = Cockpit::new();
        // Fresh boot: buffer empty, both panels Loading, un-hydrated.
        assert!(c.live_equity_buffer.is_empty());
        assert_eq!(c.live_equity_curve.variant_name(), "loading");
        assert_eq!(c.live_kpi.variant_name(), "loading");
        assert!(!c.live_equity_hydrated);

        let tail = hydrate_tail();
        let m = tail.len();
        // Capture the expected plotted x-coords (bar_ts) and the max as_of.
        let expect_bar_ms: Vec<i64> = tail.iter().map(|(b, _, _)| b.unix_millis()).collect();
        let max_as_of_ms = tail
            .iter()
            .map(|(_, a, _)| a.unix_millis())
            .max()
            .expect("non-empty");

        update(&mut c, Message::PnlHydrated(tail));

        // Buffer seeded to min(M, cap) (M < cap here).
        assert_eq!(
            c.live_equity_buffer.len(),
            m.min(LIVE_EQUITY_BUFFER_CAP),
            "hydrate must seed min(M, cap) rows"
        );
        // The plotted x-coordinates are the bar_ts data times, NOT as_of.
        let stored_ms: Vec<i64> = c
            .live_equity_buffer
            .iter()
            .map(|(ts, _)| ts.unix_millis())
            .collect();
        assert_eq!(
            stored_ms, expect_bar_ms,
            "hydrate must plot each row's bar_ts on the x-axis"
        );
        // Curve + strip both Ready (≥2 rows clears the is_all_absent trap),
        // with zero live ticks delivered.
        assert_eq!(c.live_equity_curve.variant_name(), "ready");
        assert_eq!(c.live_kpi.variant_name(), "ready");
        // The delivery guard is seeded from the MAX hydrated as_of (A4).
        assert_eq!(
            c.live_equity_last_as_of.map(|t| t.unix_millis()),
            Some(max_as_of_ms),
            "delivery guard must seed from the MAX hydrated as_of"
        );
        // The since-inception honesty switch is on.
        assert!(c.live_equity_hydrated);
    }

    /// **AC5 (model) — the guard-reconciliation case.** After a hydrate whose
    /// `as_of` values are all in the past, the FIRST live `PnlRefreshed(now())`
    /// (a fresh wallclock ≥ every hydrated `as_of`) MUST be appended — never
    /// dropped by the delivery guard. This pins the A4 `as_of` sub-decision at
    /// the model layer (the render layer proves it in `live_equity_render.rs`).
    #[test]
    fn live_append_after_hydrate_lands_not_dropped() {
        let mut c = Cockpit::new();
        let tail = hydrate_tail();
        let seeded = tail.len();
        update(&mut c, Message::PnlHydrated(tail));
        assert_eq!(c.live_equity_buffer.len(), seeded);

        // A live snapshot with a fresh `now()` wallclock as_of and a fresh
        // bar_ts AFTER the hydrated 2023 tail (forward in data time too).
        let live_bar: i64 = 1_673_789_400 + 300; // next bar after the 2023 tail
        let live = PnlSnapshot {
            cash: Money::<Usdt>::from_decimal(dec!(103100)),
            unrealized: Money::<Usdt>::from_decimal(dec!(0)),
            realized: Money::<Usdt>::from_decimal(dec!(0)),
            total_equity: Money::<Usdt>::from_decimal(dec!(103100)),
            daily_return: Money::<Usdt>::from_decimal(dec!(0)),
            as_of: Timestamp::now(), // strictly ≥ every prior-session as_of
            bar_ts: Some(Timestamp::new(
                time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(live_bar),
            )),
        };
        update(&mut c, Message::PnlRefreshed(live));

        // The live point landed — the guard did NOT drop it.
        assert_eq!(
            c.live_equity_buffer.len(),
            seeded + 1,
            "the first live snapshot after hydrate must append (not be dropped \
             by the as_of guard seeded from the max hydrated as_of)"
        );
        // The newest stored x-coord is the live bar_ts.
        assert_eq!(
            c.live_equity_buffer.back().map(|(ts, _)| ts.unix_millis()),
            Some(live_bar * 1000),
            "the appended point plots the live bar_ts"
        );
        assert_eq!(c.live_equity_curve.variant_name(), "ready");
        assert_eq!(c.live_kpi.variant_name(), "ready");
        // Still flagged hydrated — a live tick does not clear the switch.
        assert!(c.live_equity_hydrated);
    }

    /// An EMPTY hydrate (fresh ledger / fail-soft `Ok(vec![])`) is a no-op: the
    /// buffer stays empty, both panels stay Loading, the caption stays
    /// session-scoped. Mirrors the boot path where the durable table has no rows.
    #[test]
    fn pnl_hydrated_empty_is_noop() {
        let mut c = Cockpit::new();
        update(&mut c, Message::PnlHydrated(vec![]));
        assert!(c.live_equity_buffer.is_empty());
        assert_eq!(c.live_equity_curve.variant_name(), "loading");
        assert_eq!(c.live_kpi.variant_name(), "loading");
        assert!(c.live_equity_last_as_of.is_none());
        assert!(!c.live_equity_hydrated);
    }

    /// A single-row hydrate brings the curve `Ready` (1-point curve is valid)
    /// but keeps the KPI strip `Loading` — the `is_all_absent` ≥2-point trap is
    /// intact through the hydrate path, identical to the live append path.
    #[test]
    fn pnl_hydrated_one_row_curve_ready_strip_loading() {
        let mut c = Cockpit::new();
        update(
            &mut c,
            Message::PnlHydrated(vec![hydrate_row(
                1_673_789_400,
                1_746_000_000,
                dec!(100000),
            )]),
        );
        assert_eq!(c.live_equity_buffer.len(), 1);
        assert_eq!(c.live_equity_curve.variant_name(), "ready");
        assert_eq!(
            c.live_kpi.variant_name(),
            "loading",
            "a 1-row hydrate keeps the strip Loading (is_all_absent trap intact)"
        );
        // Still flips the honesty switch: a durable single-row history is a real
        // since-inception series, even if the strip waits for the 2nd point.
        assert!(c.live_equity_hydrated);
    }

    /// The hydrate respects the buffer cap: a tail longer than
    /// `LIVE_EQUITY_BUFFER_CAP` is bounded to the cap (the reader `LIMIT`s to
    /// the cap in production; the ring enforces it defensively here too).
    #[test]
    fn pnl_hydrated_respects_buffer_cap() {
        let mut c = Cockpit::new();
        let bar_base: i64 = 1_673_789_400;
        let wall_base: i64 = 1_700_000_000;
        let over_cap = LIVE_EQUITY_BUFFER_CAP + 50;
        #[allow(clippy::cast_possible_wrap)] // test-only: cap is tiny, no wrap risk
        let rows: Vec<_> = (0..over_cap as i64)
            .map(|i| {
                hydrate_row(
                    bar_base + i * 60,
                    wall_base + i * 60,
                    dec!(100000) + Decimal::from(i),
                )
            })
            .collect();
        update(&mut c, Message::PnlHydrated(rows));
        assert_eq!(
            c.live_equity_buffer.len(),
            LIVE_EQUITY_BUFFER_CAP,
            "hydrate must not exceed the buffer cap"
        );
        assert_eq!(c.live_equity_curve.variant_name(), "ready");
        assert_eq!(c.live_kpi.variant_name(), "ready");
        assert!(c.live_equity_hydrated);
    }

    /// Guard-reconciliation robustness: even if a hydrate tail's `as_of` values
    /// are NOT monotone vs. their `bar_ts` order (a backed-up-clock prior
    /// session), every row is seeded (none dropped by the per-point guard during
    /// the seed pass) and the guard ends pinned to the MAX `as_of` — so the
    /// first live tick at `now()` still lands.
    #[test]
    fn pnl_hydrated_non_monotone_as_of_seeds_all_and_pins_max() {
        let mut c = Cockpit::new();
        let bar_base: i64 = 1_673_789_400;
        // bar_ts strictly increasing; as_of deliberately NON-monotone.
        let rows = vec![
            hydrate_row(bar_base, 1_746_000_300, dec!(100000)), // as_of high
            hydrate_row(bar_base + 60, 1_746_000_100, dec!(100800)), // as_of dips
            hydrate_row(bar_base + 120, 1_746_000_500, dec!(101500)), // as_of high (max)
        ];
        update(&mut c, Message::PnlHydrated(rows));
        // All three rows seeded despite the non-monotone as_of.
        assert_eq!(
            c.live_equity_buffer.len(),
            3,
            "all hydrate rows must seed even when as_of is non-monotone"
        );
        // Guard pinned to the MAX as_of (1_746_000_500), not the last row's.
        assert_eq!(
            c.live_equity_last_as_of.map(|t| t.unix_millis()),
            Some(1_746_000_500 * 1000),
            "guard must pin to the MAX hydrated as_of, not the last row's"
        );
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

    /// T1601 — `Message::SwitchScreen` mutates `current_screen` and
    /// (for deprecated Screen aliases) `settings_active_tab` as a deep-link
    /// pre-selection side-effect (Design § Q2a). All other fields are
    /// byte-identical (compared via `Debug`-format).
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
            // Force the two intentionally-mutated fields back so the
            // Debug comparison covers every other field.
            let mut restored = after.clone();
            restored.current_screen = baseline.current_screen;
            restored.settings_active_tab = baseline.settings_active_tab;
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

    // ── T-D-14c — Cockpit::boot persistence integration tests ────────────────

    /// T-D-14c — boot with a pre-written state file restores the saved tuple.
    ///
    /// Writes a `cockpit-lab-state.json` to a temp dir, boots a `Cockpit` via
    /// `boot(Some(&path))`, asserts the `lab_state` tuple matches.
    #[test]
    fn boot_restores_persisted_state() {
        use crate::lab::persistence;
        use crate::lab::state::{DateRange, Preset};
        use trading_core::{StrategyId, Symbol, Venue};

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("cockpit-lab-state.json");

        // Build a non-default state and persist it.
        let state = crate::lab::state::LabState::with_selection(
            Some(StrategyId(smol_str::SmolStr::new("v0.5.macd"))),
            Some((Venue::Binance, Symbol::new("ETHUSDT"))),
            DateRange::Preset(Preset::H1_2024),
        );
        persistence::write_sync(&state, &path).unwrap();

        // Boot with the override path.
        let cockpit = Cockpit::boot(Some(&path));

        assert_eq!(
            cockpit.lab_state.strategy.as_ref().map(|s| s.0.as_str()),
            Some("v0.5.macd"),
            "boot must restore persisted strategy"
        );
        assert_eq!(
            cockpit.lab_state.pair.as_ref().map(|(_, s)| s.0.as_str()),
            Some("ETHUSDT"),
            "boot must restore persisted pair"
        );
        assert_eq!(
            cockpit.lab_state.range,
            DateRange::Preset(Preset::H1_2024),
            "boot must restore persisted range"
        );
    }

    // ── cockpit-training-control T-D-N4 training_arms ────────────────────────

    mod training_arms {
        use super::*;

        /// T-D-N4 arm 1 — `TrainingPanelToggled` flips `training_panel_collapsed`.
        #[test]
        fn training_panel_toggled_flips_collapsed() {
            let mut c = Cockpit::new();
            assert!(
                c.lab_state.training_panel_collapsed,
                "panel must start collapsed"
            );
            update(&mut c, Message::TrainingPanelToggled);
            assert!(
                !c.lab_state.training_panel_collapsed,
                "after toggle: panel must be expanded"
            );
            update(&mut c, Message::TrainingPanelToggled);
            assert!(
                c.lab_state.training_panel_collapsed,
                "after second toggle: panel must be collapsed again"
            );
        }

        /// T-D-N4 arm 2 — `TrainingLogLine` pushes to the ring buffer.
        #[test]
        fn training_log_line_pushes_to_ring_buffer() {
            let mut c = Cockpit::new();
            assert_eq!(c.lab_state.training_log.len(), 0, "log must start empty");
            update(
                &mut c,
                Message::TrainingLogLine(SmolStr::new("[info] epoch 1 done")),
            );
            assert_eq!(c.lab_state.training_log.len(), 1, "log must have 1 line");
            assert_eq!(
                c.lab_state.training_log.front().unwrap().as_str(),
                "[info] epoch 1 done"
            );
        }

        /// T-D-N4 arm 3 — `TrainingCancelPressed` drops the inflight handle
        /// (setting it to `None`). No handle is present in this test since
        /// spawning a real process requires the `live` feature. We verify the
        /// arm itself runs without panic and clears `None` cleanly.
        #[test]
        fn training_cancel_clears_inflight() {
            let mut c = Cockpit::new();
            // No inflight handle — cancel is a safe no-op.
            update(&mut c, Message::TrainingCancelPressed);
            assert!(c.lab_state.training_inflight.is_none());
        }

        /// T-D-N4 arm 4 — `TrainingClearLog` empties the ring buffer.
        #[test]
        fn training_clear_log_empties_buffer() {
            let mut c = Cockpit::new();
            update(&mut c, Message::TrainingLogLine(SmolStr::new("line1")));
            update(&mut c, Message::TrainingLogLine(SmolStr::new("line2")));
            assert_eq!(c.lab_state.training_log.len(), 2);
            update(&mut c, Message::TrainingClearLog);
            assert_eq!(
                c.lab_state.training_log.len(),
                0,
                "clear log must empty buffer"
            );
        }

        /// T-D-N4 arm 5 — `TrainingLogClicked` freezes auto-scroll.
        #[test]
        fn training_log_clicked_freezes_autoscroll() {
            let mut c = Cockpit::new();
            assert!(c.lab_state.training_log_anchored, "starts anchored");
            update(&mut c, Message::TrainingLogClicked);
            assert!(
                !c.lab_state.training_log_anchored,
                "after click: not anchored"
            );
        }

        /// T-D-N4 arm 6 — `TrainingLogJumpToBottom` re-anchors.
        #[test]
        fn training_log_jump_to_bottom_re_anchors() {
            let mut c = Cockpit::new();
            update(&mut c, Message::TrainingLogClicked);
            assert!(!c.lab_state.training_log_anchored);
            update(&mut c, Message::TrainingLogJumpToBottom);
            assert!(
                c.lab_state.training_log_anchored,
                "after jump-to-bottom: re-anchored"
            );
        }
    }

    /// T-D-14c — boot with absent state file falls back to cold-start defaults.
    ///
    /// Removes the file (or uses a non-existent path), boots a `Cockpit`,
    /// asserts the Q-A3 cold-start tuple.
    #[test]
    fn boot_cold_start_when_file_absent() {
        use crate::lab::state::{DateRange, Preset};

        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("no-such-file.json");
        // File does NOT exist.
        assert!(!path.exists());

        let cockpit = Cockpit::boot(Some(&path));

        assert_eq!(
            cockpit.lab_state.strategy.as_ref().map(|s| s.0.as_str()),
            Some("v0.sma"),
            "Bug #54 cold-start strategy = v0.sma (compiled-in, no CWD dep)"
        );
        assert_eq!(
            cockpit.lab_state.pair.as_ref().map(|(_, s)| s.0.as_str()),
            Some("BTCUSDT"),
            "Bug #54 cold-start symbol = BTCUSDT"
        );
        assert_eq!(
            cockpit.lab_state.range,
            DateRange::Preset(Preset::Last90d),
            "absent file must yield cold-start range Last90d"
        );
    }

    // ── Phase C — Settings rollup (ui-rethink-phase-c-sidebar-ia T-D-N03/04) ──

    /// T-D-N03 — Default `settings_active_tab` is Risk.
    #[test]
    fn settings_tab_default_is_risk() {
        assert_eq!(SettingsTab::default(), SettingsTab::Risk);
        let c = Cockpit::new();
        assert_eq!(c.settings_active_tab, SettingsTab::Risk);
    }

    /// T-D-N03 — `SwitchSettingsTab` assigns the field.
    #[test]
    fn switch_settings_tab_assigns_field() {
        let mut c = Cockpit::new();
        update(&mut c, Message::SwitchSettingsTab(SettingsTab::Control));
        assert_eq!(c.settings_active_tab, SettingsTab::Control);
        update(&mut c, Message::SwitchSettingsTab(SettingsTab::Debug));
        assert_eq!(c.settings_active_tab, SettingsTab::Debug);
        update(&mut c, Message::SwitchSettingsTab(SettingsTab::Risk));
        assert_eq!(c.settings_active_tab, SettingsTab::Risk);
    }

    /// T-D-N04 — `SwitchScreen(Screen::Risk)` pre-selects Risk tab.
    #[test]
    #[allow(deprecated)]
    fn switch_screen_to_risk_alias_preselects_risk_tab() {
        let mut c = Cockpit::new();
        update(&mut c, Message::SwitchSettingsTab(SettingsTab::Debug)); // move away
        update(&mut c, Message::SwitchScreen(Screen::Risk));
        assert_eq!(
            c.current_screen,
            Screen::Risk,
            "current_screen must be set to the deprecated Risk variant"
        );
        assert_eq!(
            c.settings_active_tab,
            SettingsTab::Risk,
            "settings_active_tab must be pre-selected to Risk"
        );
    }

    /// T-D-N04 — `SwitchScreen(Screen::Debug)` pre-selects Debug tab.
    #[test]
    #[allow(deprecated)]
    fn switch_screen_to_debug_alias_preselects_debug_tab() {
        let mut c = Cockpit::new();
        update(&mut c, Message::SwitchScreen(Screen::Debug));
        assert_eq!(c.settings_active_tab, SettingsTab::Debug);
    }

    /// T-D-N04 — `SwitchScreen(Screen::Control)` pre-selects Control tab.
    #[test]
    #[allow(deprecated)]
    fn switch_screen_to_control_alias_preselects_control_tab() {
        let mut c = Cockpit::new();
        update(&mut c, Message::SwitchScreen(Screen::Control));
        assert_eq!(c.settings_active_tab, SettingsTab::Control);
    }

    // ── T-D-N28 — Trail compound-dispatch round-trip (K6 gate) ──────────────

    /// T-D-N28 (a) — `OpenTrailFor(uuid)` sets `current_screen == Trail`
    /// AND `trail_screen_state.selected_audit_id == Some(uuid)`.
    ///
    /// This is the K6 mitigation test — identical pattern to Phase C's
    /// `OpenStrategyInLab` round-trip (tasks.md ref).
    #[test]
    fn open_trail_for_sets_screen_and_selected_audit_id() {
        let mut c = Cockpit::new();
        // Cold start: list mode (no row selected).
        assert!(c.trail_screen_state.selected_audit_id.is_none());
        assert_eq!(c.current_screen, Screen::default());

        let uuid = smol_str::SmolStr::new("test-audit-id-abc123");
        update(&mut c, Message::OpenTrailFor(uuid.clone()));

        assert_eq!(
            c.current_screen,
            Screen::Trail,
            "current_screen must switch to Trail"
        );
        assert_eq!(
            c.trail_screen_state.selected_audit_id.as_deref(),
            Some("test-audit-id-abc123"),
            "selected_audit_id must match the dispatched id"
        );
    }

    /// Phase D+ T-D-N4 — `OpenTrailFor` sets `pending_trail_audit_id` alongside
    /// `selected_audit_id`. Cleared by `TrailMirrorTick(TrailReady)`.
    #[test]
    fn open_trail_for_sets_pending_audit_id() {
        let mut c = Cockpit::new();
        assert!(c.trail_screen_state.pending_trail_audit_id.is_none());

        let uuid = smol_str::SmolStr::new("pending-id-xyz");
        update(&mut c, Message::OpenTrailFor(uuid.clone()));

        assert_eq!(
            c.trail_screen_state.pending_trail_audit_id.as_deref(),
            Some("pending-id-xyz"),
            "pending_trail_audit_id must be set after OpenTrailFor"
        );

        // Simulate TrailReady response — should clear pending.
        let trail = Box::new(ReconstructedTrailUi {
            audit_id: uuid.clone(),
            ..Default::default()
        });
        update(
            &mut c,
            Message::TrailMirrorTick(TrailMirrorUiTick::TrailReady(trail)),
        );

        assert!(
            c.trail_screen_state.pending_trail_audit_id.is_none(),
            "pending_trail_audit_id must be cleared after TrailReady"
        );
        assert!(
            c.trail_screen_state.reconstructed_trail.is_some(),
            "reconstructed_trail must be populated by TrailReady"
        );
    }

    /// Phase D+ T-D-N3 — `TrailMirrorTick(TrailUpdated)` clears `reconstructed_trail`.
    #[test]
    fn trail_mirror_tick_updated_clears_reconstructed_trail() {
        let mut c = Cockpit::new();
        // Seed a trail first.
        let trail = Box::new(ReconstructedTrailUi {
            audit_id: smol_str::SmolStr::new("some-id"),
            ..Default::default()
        });
        update(
            &mut c,
            Message::TrailMirrorTick(TrailMirrorUiTick::TrailReady(trail)),
        );
        assert!(c.trail_screen_state.reconstructed_trail.is_some());

        // Updated tick should clear it.
        update(
            &mut c,
            Message::TrailMirrorTick(TrailMirrorUiTick::TrailUpdated(smol_str::SmolStr::new(
                "some-id",
            ))),
        );
        assert!(
            c.trail_screen_state.reconstructed_trail.is_none(),
            "TrailUpdated must clear reconstructed_trail to trigger re-fetch"
        );
    }

    /// T-D-N28 (b) — `SelectTrailRow(empty)` clears `selected_audit_id` (back-to-list).
    #[test]
    fn select_trail_row_empty_clears_selection() {
        let mut c = Cockpit::new();
        // Put it into trail mode first.
        let uuid = smol_str::SmolStr::new("some-audit-id");
        update(&mut c, Message::OpenTrailFor(uuid));
        assert!(c.trail_screen_state.selected_audit_id.is_some());

        // Send empty sentinel → back to list mode.
        update(
            &mut c,
            Message::SelectTrailRow(smol_str::SmolStr::default()),
        );
        assert!(
            c.trail_screen_state.selected_audit_id.is_none(),
            "empty SelectTrailRow must clear selected_audit_id (back-to-list)"
        );
        // Drawer should also be cleared.
        assert!(
            c.trail_screen_state.drawer_selected_node.is_none(),
            "drawer_selected_node must be cleared on back-to-list"
        );
    }

    /// T-D-N28 (c) — `TrailDrawerClosed` clears `drawer_selected_node` but
    /// preserves `selected_audit_id`.
    #[test]
    fn trail_drawer_closed_clears_drawer_not_selection() {
        use crate::widgets::trail_node::TrailNodeKind;
        let mut c = Cockpit::new();
        let uuid = smol_str::SmolStr::new("some-audit-id");
        update(&mut c, Message::OpenTrailFor(uuid));
        // Open the drawer.
        update(
            &mut c,
            Message::TrailNodeChevronClicked(TrailNodeKind::Fill),
        );
        assert!(c.trail_screen_state.drawer_selected_node.is_some());
        // Close the drawer.
        update(&mut c, Message::TrailDrawerClosed);
        assert!(
            c.trail_screen_state.drawer_selected_node.is_none(),
            "TrailDrawerClosed must clear drawer_selected_node"
        );
        assert!(
            c.trail_screen_state.selected_audit_id.is_some(),
            "TrailDrawerClosed must NOT clear selected_audit_id"
        );
    }

    // ── Phase E — Compare round-trip (T-D-N15 / H5) ──────────────────────────

    /// T-D-N15 / H5 — `OpenLabFromCompare` atomically sets `current_screen =
    /// Lab`, `lab_state.strategy`, `lab_state.pair`, and `lab_state.range`.
    ///
    /// This is the K4 mitigation test — mirrors the Phase D `OpenTrailFor`
    /// round-trip pattern at `:3234-3254`. Falsifies H5 ("`OpenLabFromCompare`
    /// round-trip is atomic with respect to the lab seeding contract").
    #[test]
    fn open_lab_from_compare_sets_lab_strategy_pair_and_range() {
        use crate::lab::state::{DateRange, Preset};

        let mut c = Cockpit::new();
        // Cold start — on Lab screen by default.
        assert_eq!(c.current_screen, Screen::default());
        assert!(c.lab_state.strategy.is_none());
        assert!(c.lab_state.pair.is_none());

        let strategy = trading_core::StrategyId::new("top10_momentum_h1");
        let venue = trading_core::Venue::Binance;
        let symbol = trading_core::Symbol::new("XRPUSDT");
        let range = DateRange::Preset(Preset::H1_2024);

        update(
            &mut c,
            Message::OpenLabFromCompare {
                strategy: strategy.clone(),
                pair: Some((venue, symbol.clone())),
                range: range.clone(),
            },
        );

        assert_eq!(
            c.current_screen,
            Screen::Lab,
            "OpenLabFromCompare must switch current_screen to Lab"
        );
        assert_eq!(
            c.lab_state.strategy.as_ref().map(|s| s.0.as_str()),
            Some("top10_momentum_h1"),
            "OpenLabFromCompare must set lab_state.strategy"
        );
        assert_eq!(
            c.lab_state.pair.as_ref().map(|(v, s)| (*v, s.clone())),
            Some((trading_core::Venue::Binance, symbol.clone())),
            "OpenLabFromCompare must set lab_state.pair"
        );
        assert_eq!(
            c.lab_state.range, range,
            "OpenLabFromCompare must set lab_state.range"
        );
    }

    /// T-D-N15 extension — `OpenLabFromCompare` with `pair = None` still
    /// switches to Lab and sets strategy/range but leaves `lab_state.pair`
    /// unchanged (no pair pre-selection when the caller passes None).
    #[test]
    fn open_lab_from_compare_no_pair_leaves_pair_unchanged() {
        use crate::lab::state::{DateRange, Preset};

        let mut c = Cockpit::new();
        let range = DateRange::Preset(Preset::Last30d);

        update(
            &mut c,
            Message::OpenLabFromCompare {
                strategy: trading_core::StrategyId::new("btc_sma"),
                pair: None,
                range: range.clone(),
            },
        );

        assert_eq!(c.current_screen, Screen::Lab);
        assert_eq!(
            c.lab_state.strategy.as_ref().map(|s| s.0.as_str()),
            Some("btc_sma"),
        );
        // pair was None before and is still None — not mutated.
        assert!(
            c.lab_state.pair.is_none(),
            "pair = None must leave lab_state.pair unchanged"
        );
        assert_eq!(c.lab_state.range, range);
    }

    // ── Phase F round-trip tests (T-D-N20) ──────────────────────────────────

    /// T-D-N20 (1/3) — `MemoryHydrate` populates cache + sets `last_indexed`.
    ///
    /// Sends 3 fixture `LessonCardCard` items via `Message::MemoryHydrate`.
    /// Asserts: `cache.len() == 3`, `cache[0].card_id == "card_e"`,
    /// `last_indexed` is `Some(...)`.
    #[test]
    fn memory_hydrate_populates_cache_and_indexed() {
        use smol_str::SmolStr;
        let mut c = Cockpit::new();
        assert!(
            c.memory_screen_state.cache.is_empty(),
            "pre-hydrate cache must be empty"
        );
        assert!(
            c.memory_screen_state.last_indexed.is_none(),
            "pre-hydrate last_indexed must be None"
        );

        let cards = vec![
            crate::memory::state::LessonCardCard {
                card_id: SmolStr::new("card_e"),
                symbol_or_pair: SmolStr::new("BTCUSDT"),
                closed_at: SmolStr::new("2026-01-05T12:00:00Z"),
                strategy_id: SmolStr::new("v1.momentum"),
                signed_pnl_display: SmolStr::new("+85.00 USDT"),
                outcome_class: SmolStr::new("Win"),
                note: None,
                close_transaction_id: None,
            },
            crate::memory::state::LessonCardCard {
                card_id: SmolStr::new("card_d"),
                symbol_or_pair: SmolStr::new("ETHUSDT"),
                closed_at: SmolStr::new("2026-01-04T09:30:00Z"),
                strategy_id: SmolStr::new("v1.momentum"),
                signed_pnl_display: SmolStr::new("-23.50 USDT"),
                outcome_class: SmolStr::new("Loss"),
                note: None,
                close_transaction_id: None,
            },
            crate::memory::state::LessonCardCard {
                card_id: SmolStr::new("card_c"),
                symbol_or_pair: SmolStr::new("SOLUSDT"),
                closed_at: SmolStr::new("2026-01-03T15:00:00Z"),
                strategy_id: SmolStr::new("sma_crossover"),
                signed_pnl_display: SmolStr::new("+2.10 USDT"),
                outcome_class: SmolStr::new("Scratch"),
                note: None,
                close_transaction_id: None,
            },
        ];

        update(&mut c, Message::MemoryHydrate(cards));

        assert_eq!(
            c.memory_screen_state.cache.len(),
            3,
            "post-hydrate cache must have 3 cards"
        );
        assert_eq!(c.memory_screen_state.cache[0].card_id.as_str(), "card_e");
        assert!(
            c.memory_screen_state.last_indexed.is_some(),
            "last_indexed must be Some(...) after hydrate"
        );
    }

    /// T-D-N20 (2/3) — `MemoryOpenDrawer` sets `drawer_open`.
    ///
    /// Sends `Message::MemoryOpenDrawer(SmolStr::new("card_e"))`.
    /// Asserts: `drawer_open == Some("card_e")`.
    /// Then sends `Message::MemoryCloseDrawer`.
    /// Asserts: `drawer_open == None`.
    #[test]
    fn memory_open_drawer_sets_drawer_open() {
        use smol_str::SmolStr;
        let mut c = Cockpit::new();
        assert!(
            c.memory_screen_state.drawer_open.is_none(),
            "pre-open drawer_open must be None"
        );

        update(&mut c, Message::MemoryOpenDrawer(SmolStr::new("card_e")));
        assert_eq!(
            c.memory_screen_state.drawer_open.as_deref(),
            Some("card_e"),
            "after MemoryOpenDrawer, drawer_open must be Some('card_e')"
        );

        update(&mut c, Message::MemoryCloseDrawer);
        assert!(
            c.memory_screen_state.drawer_open.is_none(),
            "after MemoryCloseDrawer, drawer_open must be None"
        );
    }

    /// T-D-N20 (3/3) — `ToggleAssistantSlot` flips `is_open`.
    ///
    /// Starting from `is_open = false`, one toggle sets it to `true`;
    /// a second toggle returns it to `false`.
    #[test]
    fn toggle_assistant_slot_flips_is_open() {
        let mut c = Cockpit::new();
        assert!(
            !c.assistant_state.is_open,
            "default assistant is_open must be false"
        );

        update(&mut c, Message::ToggleAssistantSlot);
        assert!(
            c.assistant_state.is_open,
            "after first toggle, is_open must be true"
        );

        update(&mut c, Message::ToggleAssistantSlot);
        assert!(
            !c.assistant_state.is_open,
            "after second toggle, is_open must return to false"
        );
    }

    /// v3-llm-forecaster Wave F (T-D-N(F3)) —
    /// `AssistantReasoningTraceUpdate` sets `last_forecast` and rotates
    /// the previous forecast onto `history` (most-recent first).
    #[test]
    fn assistant_reasoning_trace_update_rotates_history() {
        use crate::assistant::state::LlmForecastView;

        fn make(rating: &str) -> LlmForecastView {
            LlmForecastView {
                symbol: smol_str::SmolStr::new("BTCUSDT"),
                rating: smol_str::SmolStr::new(rating),
                confidence_display: smol_str::SmolStr::new("0.50"),
                reasoning_trace: smol_str::SmolStr::new("trace"),
                cited_lesson_ids: vec![],
                cost_line: None,
                audit_id: None,
            }
        }

        let mut c = Cockpit::new();
        assert!(c.assistant_state.last_forecast.is_none());
        assert!(c.assistant_state.history.is_empty());

        update(&mut c, Message::AssistantReasoningTraceUpdate(make("BUY")));
        assert_eq!(
            c.assistant_state
                .last_forecast
                .as_ref()
                .map(|f| f.rating.as_str()),
            Some("BUY"),
        );
        assert!(c.assistant_state.history.is_empty());

        update(&mut c, Message::AssistantReasoningTraceUpdate(make("HOLD")));
        assert_eq!(
            c.assistant_state
                .last_forecast
                .as_ref()
                .map(|f| f.rating.as_str()),
            Some("HOLD"),
        );
        assert_eq!(c.assistant_state.history.len(), 1);
        assert_eq!(c.assistant_state.history[0].rating.as_str(), "BUY");
    }

    /// v3-llm-forecaster Wave F (T-D-N(F3)) — history is capped at
    /// `HISTORY_CAP = 20` to bound memory.
    #[test]
    fn assistant_reasoning_trace_update_caps_history() {
        use crate::assistant::state::{HISTORY_CAP, LlmForecastView};

        fn make(idx: usize) -> LlmForecastView {
            LlmForecastView {
                symbol: smol_str::SmolStr::new("BTCUSDT"),
                rating: smol_str::SmolStr::new("HOLD"),
                confidence_display: smol_str::SmolStr::new(format!("0.{idx:02}")),
                reasoning_trace: smol_str::SmolStr::new("trace"),
                cited_lesson_ids: vec![],
                cost_line: None,
                audit_id: None,
            }
        }

        let mut c = Cockpit::new();
        for i in 0..(HISTORY_CAP + 5) {
            update(&mut c, Message::AssistantReasoningTraceUpdate(make(i)));
        }
        assert_eq!(
            c.assistant_state.history.len(),
            HISTORY_CAP,
            "history must cap at HISTORY_CAP"
        );
        assert!(
            c.assistant_state.last_forecast.is_some(),
            "last_forecast must always carry the most-recent value"
        );
    }

    /// v3-llm-forecaster Wave F (T-D-N(F4)) — Runtime gate sanity:
    /// `AssistantReasoningTraceUpdate` does NOT flip
    /// `assistant_state.mode`. The mode is owned by the boot-path
    /// runtime gate (R9.3); a stray update from the message bus must
    /// not force the slot into `ReasoningTrace`.
    #[test]
    fn assistant_runtime_gate_preserves_offline_default() {
        use crate::assistant::state::{AssistantMode, LlmForecastView};

        let mut c = Cockpit::new();
        assert_eq!(c.assistant_state.mode, AssistantMode::Offline);

        update(
            &mut c,
            Message::AssistantReasoningTraceUpdate(LlmForecastView {
                symbol: smol_str::SmolStr::new("BTCUSDT"),
                rating: smol_str::SmolStr::new("BUY"),
                confidence_display: smol_str::SmolStr::new("0.80"),
                reasoning_trace: smol_str::SmolStr::new("trace"),
                cited_lesson_ids: vec![],
                cost_line: None,
                audit_id: None,
            }),
        );
        assert_eq!(
            c.assistant_state.mode,
            AssistantMode::Offline,
            "AssistantReasoningTraceUpdate must NOT flip mode — runtime gate owns it"
        );
        // The forecast is still stored in `last_forecast`, but the
        // `Offline` view fn doesn't read it (the byte-identity guard
        // short-circuits before touching the payload).
        assert!(c.assistant_state.last_forecast.is_some());
    }

    // ── lab-end-to-end-v2 Wave D-1 tests (T-D1.1 / T-AR-2) ─────────────────

    /// T-D1.1 / R1.1 — `LabSelectPair` updates BOTH `lab_state.pair` AND
    /// `selected_symbol`.  Phase A R3.3 closure.
    ///
    /// This is the forensic-gate test: it MUST HAVE FAILED before the fix
    /// at state.rs:1893 (which only wrote `lab_state.pair` and left
    /// `selected_symbol` unchanged).
    #[test]
    fn lab_select_pair_updates_selected_symbol() {
        let mut c = Cockpit::new();
        assert!(
            c.selected_symbol.is_none(),
            "cold-start: selected_symbol must be None"
        );

        let venue = trading_core::Venue::Binance;
        let symbol = trading_core::Symbol::new("XRPUSDT");

        update(&mut c, Message::LabSelectPair(venue, symbol.clone()));

        assert_eq!(
            c.lab_state.pair,
            Some((venue, symbol.clone())),
            "LabSelectPair must set lab_state.pair"
        );
        assert_eq!(
            c.selected_symbol,
            Some((venue, symbol.clone())),
            "LabSelectPair must also set selected_symbol (R1.1 fix)"
        );
    }

    /// T-D1.1 extension — a second `LabSelectPair` overwrites `selected_symbol`
    /// (not just `lab_state.pair`).
    #[test]
    fn lab_select_pair_overwrites_selected_symbol_on_subsequent_click() {
        let mut c = Cockpit::new();
        let v = trading_core::Venue::Binance;

        update(
            &mut c,
            Message::LabSelectPair(v, trading_core::Symbol::new("XRPUSDT")),
        );
        update(
            &mut c,
            Message::LabSelectPair(v, trading_core::Symbol::new("ETHUSDT")),
        );

        assert_eq!(
            c.selected_symbol,
            Some((v, trading_core::Symbol::new("ETHUSDT"))),
            "second LabSelectPair must overwrite selected_symbol with ETHUSDT"
        );
        assert_eq!(
            c.lab_state.pair,
            Some((v, trading_core::Symbol::new("ETHUSDT"))),
        );
    }

    // ── cockpit-toast-queue v0.1.0 — unit tests (T-D-N4) ─────────────────────

    /// T-D-N4 / R1 acceptance: enqueue 3 distinct messages, expect len==3
    /// and ids [0, 1, 2] (monotonic, 0-indexed counter).
    #[test]
    fn toast_queue_enqueue_basic() {
        let mut c = Cockpit::new();
        update(&mut c, Message::ShowToast(SmolStr::new("msg1")));
        update(&mut c, Message::ShowToast(SmolStr::new("msg2")));
        update(&mut c, Message::ShowToast(SmolStr::new("msg3")));

        assert_eq!(c.toast_queue.len(), 3, "queue must hold all 3 entries");
        let ids: Vec<_> = c.toast_queue.iter().map(|t| t.id).collect();
        assert_eq!(ids, vec![0, 1, 2], "ids must be monotonic 0-indexed");
        assert!(
            c.toast_queue
                .iter()
                .all(|t| t.severity == ToastSeverity::Info),
            "ShowToast must map to Info severity"
        );
    }

    /// T-D-N4 / R1 acceptance: enqueue 6 messages with cap 5; front entry
    /// ("m1") is dropped; front becomes "m2" (oldest retained = 2nd enqueued).
    #[test]
    fn toast_queue_overflow_drops_oldest() {
        let mut c = Cockpit::new();
        for i in 1..=6u32 {
            update(&mut c, Message::ShowToast(SmolStr::new(format!("m{i}"))));
        }
        assert_eq!(
            c.toast_queue.len(),
            MAX_TOAST_QUEUE_LEN,
            "cap must be respected"
        );
        assert_eq!(
            c.toast_queue.front().map(|t| t.message.as_str()),
            Some("m2"),
            "oldest (m1) must have been dropped; front must be m2"
        );
        assert_eq!(
            c.toast_queue.back().map(|t| t.message.as_str()),
            Some("m6"),
            "newest (m6) must be at the back"
        );
    }

    /// T-D-N4 / R1 acceptance: enqueue 3, dispatch `DismissToastById` on the
    /// middle entry; queue len == 2 with the middle id gone.
    #[test]
    fn toast_queue_dismiss_by_id() {
        let mut c = Cockpit::new();
        update(&mut c, Message::ShowToast(SmolStr::new("a")));
        update(&mut c, Message::ShowToast(SmolStr::new("b")));
        update(&mut c, Message::ShowToast(SmolStr::new("c")));

        let middle_id = c.toast_queue[1].id;
        update(&mut c, Message::DismissToastById(middle_id));

        assert_eq!(c.toast_queue.len(), 2, "middle must be dismissed");
        assert!(
            c.toast_queue.iter().all(|t| t.id != middle_id),
            "dismissed id must not be in the queue"
        );
        // Surviving messages in order.
        assert_eq!(c.toast_queue[0].message, SmolStr::new("a"));
        assert_eq!(c.toast_queue[1].message, SmolStr::new("c"));
    }

    /// T-D-N4 / R1 acceptance: `ShowToast` back-compat — dispatching
    /// `Message::ShowToast(...)` enqueues with `Info` severity.
    #[test]
    fn show_toast_msg_back_compat() {
        let mut c = Cockpit::new();
        update(&mut c, Message::ShowToast(SmolStr::new("hello")));

        assert_eq!(c.toast_queue.len(), 1, "ShowToast must enqueue one entry");
        assert_eq!(
            c.toast_queue.front().map(|t| t.severity),
            Some(ToastSeverity::Info),
            "ShowToast must map to Info severity"
        );
        assert_eq!(
            c.toast_queue.front().map(|t| t.message.as_str()),
            Some("hello"),
            "message must match"
        );
    }

    /// lab-run-save-compare T5 — Compare cold-boot index gating. The cache is
    /// un-indexed on cold start; the FIRST `SwitchScreen(Compare)` runs the
    /// two-root scan exactly once (stamps `last_indexed_at`); a second
    /// navigation does NOT re-stamp (no redundant re-scan); a successful Lab
    /// run clears the tag so the NEXT Compare visit re-scans the just-persisted
    /// report.
    #[test]
    fn compare_cold_boot_index_gating() {
        let mut c = Cockpit::new();
        assert!(
            c.compare_screen_state.last_indexed_at.is_none(),
            "cache must be un-indexed on cold start"
        );

        // First navigation → scan runs, tag stamped.
        update(&mut c, Message::SwitchScreen(Screen::Compare));
        let first_stamp = c.compare_screen_state.last_indexed_at;
        assert!(
            first_stamp.is_some(),
            "first Compare navigation must run the cold-boot scan and stamp last_indexed_at"
        );

        // Second navigation (away then back) → tag unchanged (no re-scan).
        update(&mut c, Message::SwitchScreen(Screen::Live));
        update(&mut c, Message::SwitchScreen(Screen::Compare));
        assert_eq!(
            c.compare_screen_state.last_indexed_at, first_stamp,
            "re-navigating to Compare must NOT re-scan while already indexed"
        );

        // A successful Lab run clears the tag → next Compare visit re-scans.
        let summary = crate::lab::runner::RunSummary {
            strategy_id: SmolStr::new("v1.momentum"),
            symbol: SmolStr::new("XRPUSDT"),
            report_path: None,
            equity_series: Vec::new(),
            fills: Vec::new(),
            kpis: backtest::BacktestKpis::default(),
            bars: std::sync::Arc::new(Vec::new()),
            position_curve: Vec::new(),
        };
        update(&mut c, Message::LabRunCompleted(Ok(summary)));
        assert!(
            c.compare_screen_state.last_indexed_at.is_none(),
            "a successful Lab run must clear the Compare index so the freshly-persisted \
             report is picked up on the next Compare visit"
        );
    }
}
