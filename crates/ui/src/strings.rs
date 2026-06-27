//! All user-visible strings in one place.
//!
//! **Every** string the operator can read lives here. Zero string literals
//! inside widget files. This single-source-of-truth is what makes copy
//! reviewable as a table and localization a one-module change.
//!
//! Style guide for new strings:
//! - Plain language. Prefer "Stop trading" over "Halt agent".
//! - Empty-state copy tells the user what to expect next, not "no data".
//! - Error-state copy points at what to check, not just what broke.
//! - Sentence case, no trailing period inside labels; period only in full
//!   sentences (banners, empty-state explanations).

// ── App chrome ───────────────────────────────────────────────────────────────

pub const APP_TITLE: &str = "Trading Cockpit";

// ── Panel titles ─────────────────────────────────────────────────────────────

/// Phase 5 (R11.5) — title constant renamed from `PANEL_TAPE_TITLE`
/// (`"Live tape"`) per the Lumen `AgentFeed.jsx:71` reference. Module
/// path renamed `widgets::tape` → `widgets::agent_feed`. Field name
/// `Cockpit::tape` is **preserved** per Phase 5 Q14.
pub const PANEL_AGENT_FEED_TITLE: &str = "Agent activity";
pub const PANEL_POSITIONS_TITLE: &str = "Open positions";
pub const PANEL_PNL_TITLE: &str = "P&L";
pub const PANEL_KILL_TITLE: &str = "Stop trading";
pub const PANEL_LATENCY_TITLE: &str = "Feed latency";
pub const PANEL_STRATEGIES_TITLE: &str = "Strategies";

// ── Live tape ────────────────────────────────────────────────────────────────

pub const TAPE_COL_TIME: &str = "Time";
pub const TAPE_COL_SYMBOL: &str = "Symbol";
pub const TAPE_COL_SIDE: &str = "Side";
pub const TAPE_COL_PRICE: &str = "Price";
pub const TAPE_COL_QTY: &str = "Qty";
/// cockpit-live-tape-units-fix (2026-06-10) — notional column header.
/// Surfaces `qty × price` in USDT so the operator reads the real clip
/// size (e.g. a 10 %-of-equity ≈ 10,000 USDT fill) instead of mistaking
/// the rightmost USDT-suffixed number — the FEE — for the trade size.
pub const TAPE_COL_NOTIONAL: &str = "Notional";
pub const TAPE_COL_FEE: &str = "Fee";
pub const TAPE_PAUSE_LABEL: &str = "Pause";
pub const TAPE_RESUME_LABEL: &str = "Resume";
pub const TAPE_LOADING: &str = "Connecting to the fill stream…";
pub const TAPE_EMPTY: &str = "No fills yet. Waiting for the first bar from BTCUSDT.";
pub const TAPE_ERROR_PREFIX: &str = "Can't read the fill stream: ";
pub const TAPE_PAUSED_BANNER: &str = "Paused — updates buffered";

// ── Month abbreviations (cockpit-live-axis-density-fix, 2026-06-10) ──────────
//
// Three-letter English month abbreviations for the adaptive time-axis labels
// on the equity / drawdown curves (`widgets::chart::format_time_axis_label`).
// Kept here so the widget carries no inline month literals (the consistency
// gate forbids user-visible string literals inside `src/widgets`).

/// Three-letter month abbreviations, indexed `[0]=Jan … [11]=Dec`.
pub const MONTH_ABBREVS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Map a 1-based month number (`1..=12`, as returned by
/// `time::Month as u8`) to its three-letter abbreviation. Out-of-range
/// input clamps into range so the helper is total (never panics on a
/// malformed timestamp).
#[must_use]
pub fn month_abbrev(month_1_based: u8) -> &'static str {
    let idx = (month_1_based.clamp(1, 12) - 1) as usize;
    MONTH_ABBREVS[idx]
}

// ── Position panel ───────────────────────────────────────────────────────────

pub const POS_COL_SYMBOL: &str = "Symbol";
pub const POS_COL_QTY: &str = "Qty";
pub const POS_COL_COST: &str = "Cost";
pub const POS_COL_MARK: &str = "Mark";
pub const POS_COL_PNL: &str = "P&L";
pub const POS_COL_PNL_PCT: &str = "P&L %";
pub const POS_COL_EXPOSURE: &str = "Exposure %";
pub const POS_LOADING: &str = "Loading positions from the ledger…";
pub const POS_EMPTY: &str = "No open positions. Strategy is armed and watching.";
pub const POS_ERROR_PREFIX: &str = "Ledger error while reading positions: ";

// ── Short-selling direction badge (advisor-short-selling, ADR-0068 § D8) ──────
//
// A single-coin position is SIGNED: `base_qty > 0` is a long, `base_qty < 0`
// is a SHORT (a sell-to-open simulated short, paper/sim only). The Direction
// column carries a word badge so a short reads AS a short — never a malformed
// long — and colour is never the only signal (accessibility). The `ui` owns
// the words; the sign comes from the signed `PositionView.base_qty` the audit
// reader now emits (no engine string crosses the seam).

/// Position-panel Direction column header.
pub const POS_COL_DIRECTION: &str = "Direction";

/// Direction badge — a long position (`base_qty > 0`).
pub const POS_DIRECTION_LONG: &str = "LONG";

/// Direction badge — a SHORT position (`base_qty < 0`, sell-to-open). The
/// down-half lever: profits when price falls, loses without bound as it rises.
pub const POS_DIRECTION_SHORT: &str = "SHORT";

// ── The unbounded-loss disclaimer (advisor-short-selling, R-SS.4 LOAD-BEARING) ─
//
// MANDATORY on EVERY short surface (Live view + leaderboard + forward plan,
// ADR-0068 § D5/D8). A 1x perp short loses WITHOUT BOUND as price rises: a 2×
// price move wipes the €200 and then some (cash can go negative at the
// maintenance-margin liquidation floor). This is the honest behaviour R-SS.4
// mandates — the displayed P/L is allowed to print negative; it is NEVER
// clamped at 0. The copy is plain-language and non-euphemistic by design.

/// The load-bearing "a short can lose more than your €200" disclaimer. Rendered
/// on every surface where a short is in play. Honest + plain — names the
/// unbounded loss and the 2× wipe-out explicitly; no euphemism.
pub const SHORT_UNBOUNDED_LOSS_DISCLAIMER: &str = "A short can lose MORE than your \u{20ac}200 \u{2014} an unbounded loss. A 2\u{00d7} price move \
     wipes you out and then some. Simulated paper budget, not financial advice.";

// ── P&L card ─────────────────────────────────────────────────────────────────

pub const PNL_LABEL_CASH: &str = "Cash";
pub const PNL_LABEL_UNREALIZED: &str = "Unrealized";
pub const PNL_LABEL_REALIZED: &str = "Realized today";
pub const PNL_LABEL_EQUITY: &str = "Total equity";
pub const PNL_LABEL_DAILY_RETURN: &str = "P&L today";
pub const PNL_LOADING: &str = "Reading equity from the ledger…";
pub const PNL_EMPTY: &str = "No equity recorded yet. First reconciliation pending.";
pub const PNL_ERROR_PREFIX: &str = "Ledger error while reading equity: ";

// ── Kill switch ──────────────────────────────────────────────────────────────

pub const KILL_BUTTON_LABEL: &str = "Stop trading";
/// Help text shown next to the kill button (and used as the hover-tooltip
/// surface). Updated in T907 once T906 wired the button to the real
/// `KillSwitch::trip` path: clicking it now actually halts the agent and
/// writes an incident report — so the copy promises that explicitly.
pub const KILL_BUTTON_HELP: &str = "Halts the trading agent and writes an incident report. Cancels open orders and flattens \
     every position. Requires a typed confirmation.";
pub const KILL_DIALOG_TITLE: &str = "Confirm stop trading";
pub const KILL_DIALOG_BODY: &str = "This cancels every open order, sells each open position at market, and puts the agent \
     into a halted state. Type the phrase below to confirm.";
pub const KILL_PHRASE_LABEL: &str = "Type HALT BTC to confirm";
/// The exact phrase the operator must type to enable the Confirm button.
/// Carried in the spec as the safety token (task T19).
pub const KILL_SAFETY_PHRASE: &str = "HALT BTC";
pub const KILL_CONFIRM_LABEL: &str = "Confirm stop";
pub const KILL_CANCEL_LABEL: &str = "Cancel";
pub const KILL_PHRASE_MISMATCH_HINT: &str =
    "Phrase doesn't match. Type HALT BTC exactly (case-sensitive).";
pub const KILL_HALTED_BANNER: &str = "AGENT HALTED";
pub const KILL_HALTED_HINT: &str =
    "Remove .halt and re-arm from the operator runbook before resuming.";
pub const KILL_RUNBOOK_LINK_LABEL: &str = "Open kill-switch runbook";
/// Relative path from the workspace root to the kill-switch runbook.
/// Rendered next to the runbook link so operators without a clickable
/// terminal can still find the file. Used by `T_FINAL_B`.
pub const KILL_RUNBOOK_LINK_PATH: &str = "spec/runbooks/kill-switch.md";

// ── Strategies panel (v0.5 T522, R5 cockpit visibility) ─────────────────────
//
// Copy contract (feature brief R5.2 + architect Q4 resolution):
// - loading → explicit "connecting" verb, not "fetching".
// - empty   → actionable next step: where to drop a TOML.
// - error   → what broke + what to check; channel-closed copy is reused
//             from the existing CONNECTION_CHANNEL_CLOSED constant.
// - ready   → tabular rows; per-row error state reuses `STRATEGIES_STATUS_ERROR`
//             plus the `error_summary` carried by the `StrategyLoadError`.

pub const STRATEGIES_LOADING: &str = "Loading active strategies…";
pub const STRATEGIES_EMPTY: &str =
    "No strategies loaded. Drop a TOML under config/strategies/ to begin.";
/// Reused with `CONNECTION_CHANNEL_CLOSED` (or any other error detail) to
/// produce the full red-tinted error-state line. Matches the `*_ERROR_PREFIX`
/// pattern used by tape / positions / P&L.
pub const STRATEGIES_ERROR_PREFIX: &str = "Can't read strategies: ";

// Column headers for the strategies table (Ready state).
pub const STRATEGIES_COL_ID: &str = "Strategy";
pub const STRATEGIES_COL_HASH: &str = "Hash";
pub const STRATEGIES_COL_STATUS: &str = "Status";
pub const STRATEGIES_COL_LAST_EVENT: &str = "Last event";
pub const STRATEGIES_COL_SIGNALS_60S: &str = "Signals / 60s";
pub const STRATEGIES_COL_POSITION: &str = "Holds position";

// Status pill labels.
pub const STRATEGIES_STATUS_READY: &str = "Ready";
pub const STRATEGIES_STATUS_LOADING: &str = "Loading";
pub const STRATEGIES_STATUS_ERROR: &str = "Error";

// Event-kind labels (rendered in the `Last event` column and the recent-events
// footer list in the panel).
pub const STRATEGIES_EVENT_LOAD: &str = "loaded";
pub const STRATEGIES_EVENT_SWAP: &str = "swapped";
pub const STRATEGIES_EVENT_UNLOAD: &str = "unloaded";
pub const STRATEGIES_EVENT_REJECT: &str = "rejected";

/// Rendered in the `Holds position` column when a strategy is currently net
/// long or short. Pairs with `PLACEHOLDER_NONE` for flat.
pub const STRATEGIES_POSITION_HELD: &str = "yes";
pub const STRATEGIES_POSITION_FLAT: &str = "no";

// ── Tape audit modal ────────────────────────────────────────────────────────
//
// Copy contract (tape-row-audit-modal R7 + principles voice/copy):
// - title    → terse, declarative ("Journal transaction").
// - labels   → sentence case, no trailing period (form labels).
// - loading  → present-tense verb + ellipsis (unicode `…`, not `...`).
// - empty    → declarative full sentence with terminal period.
// - error    → `<what's broken>:` prefix matching `TAPE_ERROR_PREFIX`.
// - close    → imperative single word (no glyph — principles "no icons").

pub const TAPE_AUDIT_MODAL_TITLE: &str = "Journal transaction";
pub const TAPE_AUDIT_MODAL_TX_LABEL: &str = "Transaction ID";
pub const TAPE_AUDIT_MODAL_TS_LABEL: &str = "Time";
pub const TAPE_AUDIT_MODAL_DESC_LABEL: &str = "Description";
pub const TAPE_AUDIT_MODAL_STRATEGY_LABEL: &str = "Strategy";
/// Rendered in the strategy slot when `strategy_id` is `None` (fills not
/// attributable to a specific strategy — manual-operator fills, kill-switch
/// flatten, etc.). Same em-dash as `PLACEHOLDER_NONE` but kept as a distinct
/// constant so a future "Manual" / "—" copy split is one-line.
pub const TAPE_AUDIT_MODAL_STRATEGY_NONE: &str = "—";
pub const TAPE_AUDIT_MODAL_COL_ACCOUNT: &str = "Account";
pub const TAPE_AUDIT_MODAL_COL_DEBIT: &str = "Debit";
pub const TAPE_AUDIT_MODAL_COL_CREDIT: &str = "Credit";
pub const TAPE_AUDIT_MODAL_COL_CURRENCY: &str = "Currency";
pub const TAPE_AUDIT_MODAL_LOADING: &str = "Loading journal entries…";
pub const TAPE_AUDIT_MODAL_EMPTY: &str = "No entries for this transaction.";
pub const TAPE_AUDIT_MODAL_ERROR_PREFIX: &str = "Failed to load journal entries: ";
pub const TAPE_AUDIT_MODAL_CLOSE_LABEL: &str = "Close";

// ── Status bar (T1508) ───────────────────────────────────────────────────────
//
// Net-new strings for the status-bar widget. Not a rewrite of existing copy
// (Constraint 2 covers existing strings only). Each constant has a single
// display home — the status bar row at the bottom of the cockpit shell.

/// Rendered next to the coloured dot when all monitored venues are `Fresh`.
/// Appended by the widget with `· {venues}` (e.g. "Connected · binance").
pub const STATUS_BAR_CONNECTED: &str = "Connected";

/// Rendered next to the coloured dot when any venue is `Stale`.
/// Appended by the widget with `· {venue}` for the first stale venue.
pub const STATUS_BAR_RECONNECTING: &str = "Reconnecting";

/// Rendered when `cockpit.market_health` is empty (no venues seen yet).
pub const STATUS_BAR_DISCONNECTED: &str = "Disconnected";

/// Prefix for the latency field: `"Latency {n} ms"` or `"Latency —"`.
pub const STATUS_BAR_LATENCY_LABEL: &str = "Latency";

/// Prefix for the server-time field: `"Server {HH:MM:SS} UTC"`.
pub const STATUS_BAR_SERVER_LABEL: &str = "Server";

/// Prefix for the CPU field (Phase 1 placeholder).
pub const STATUS_BAR_CPU_LABEL: &str = "CPU";

/// Literal placeholder for the CPU field — deferred to R13.4.
pub const STATUS_BAR_CPU_PLACEHOLDER: &str = "CPU —";

/// Rendered when latency is `Latency::Unknown`.
/// Combined as `"{STATUS_BAR_LATENCY_LABEL} {STATUS_BAR_NO_LATENCY}"`.
pub const STATUS_BAR_NO_LATENCY: &str = "—";

/// UTC suffix for the server-time field: `" UTC"`.
pub const STATUS_BAR_UTC_SUFFIX: &str = "UTC";

/// Rendered when `server_time_now` is `None` (not yet received from the
/// 1 Hz subscription). Full label becomes `"{SERVER_LABEL} {NO_SERVER_TIME} UTC"`.
pub const STATUS_BAR_NO_SERVER_TIME: &str = "—";

/// Milliseconds unit suffix for the latency field (single word, no leading space).
pub const STATUS_BAR_MS: &str = "ms";

/// Version prefix — `"v"` — used in the `concat!` that builds the version label.
pub const STATUS_BAR_VERSION_PREFIX: &str = "v";

/// Version suffix — `" · rust"` — appended after the crate version number.
pub const STATUS_BAR_VERSION_SUFFIX: &str = " \u{00b7} rust";

/// Full compile-time version label: `"v{CARGO_PKG_VERSION} · rust"`.
/// Defined here (single-source-of-truth) via `concat!` + `env!` so widget code
/// never carries raw string literals for the version sigil or stack tag.
pub const STATUS_BAR_VERSION: &str = concat!("v", env!("CARGO_PKG_VERSION"), " \u{00b7} rust");

// ── Activity tape (cockpit-activity-status-bar v0.1.0 Wave B T-D-N6) ────────
//
// Zero inline literals contract (R7.2): every operator-visible string must
// live here, not inside `widgets/activity_tape.rs`.

/// Status-bar activity tape: label prefix for Yahoo data preload activities.
pub const ACTIVITY_KIND_YAHOO_LABEL: &str = "Loading data";

/// Status-bar activity tape: label prefix for Lab Run backtest activities.
pub const ACTIVITY_KIND_LAB_RUN_LABEL: &str = "Backtesting";

/// Status-bar activity tape: label prefix for Training subprocess activities.
pub const ACTIVITY_KIND_TRAINING_LABEL: &str = "Training";

// ── Activity tape — audit-ledger-writes producer (cockpit-activity-audit-ledger-producer v0.1.0) ──
//
// R-NR.4: all operator-visible strings live here; zero inline literals in widgets.
// R2.1 (Q2 = redacted): label carries only the write count, not event detail.

/// Status-bar activity tape: label prefix for audit-ledger-write activities.
///
/// Rendered as `"Audit: N writes"` (R2.1 / Q2=(a) — redacted label).
pub const ACTIVITY_KIND_AUDIT_LABEL: &str = "Audit";

/// Count format template for normal audit write counts (N ≤ 9999).
///
/// Combined with the window count in the aggregator: `"Audit: N writes"`.
pub const ACTIVITY_AUDIT_COUNT_FORMAT: &str = "{N} writes";

/// K2 flood-truncation label: emitted when the window count exceeds 9999.
///
/// Prevents the `"Audit: N writes"` label from exceeding the 64-char budget.
pub const ACTIVITY_AUDIT_FLOOD_TRUNCATION: &str = "9999+ writes";

/// Overflow chip prefix: rendered as `"+{n} more"` where n is the hidden count.
/// Combined with `ACTIVITY_TAPE_MORE_SUFFIX` in the widget.
pub const ACTIVITY_TAPE_MORE_PREFIX: &str = "+";

/// Overflow chip suffix: combined with `ACTIVITY_TAPE_MORE_PREFIX`.
pub const ACTIVITY_TAPE_MORE_SUFFIX: &str = " more";

// ── Sidebar nav (Phase 2 — T1602) ────────────────────────────────────────────
//
// Net-new strings for the screen-routed shell. Phase 2 wires the first
// three (Home / Debug / Charts); Phase 3 wakes the last three so the
// extension is additive. Operator-locked Constraint 2 (no voice rewrite)
// preserved — these are net-new strings, not rewrites.

pub const SIDEBAR_NAV_HOME: &str = "Home";
pub const SIDEBAR_NAV_DEBUG: &str = "Debug";
pub const SIDEBAR_NAV_CHARTS: &str = "Charts";
pub const SIDEBAR_NAV_STRATEGIES: &str = "Strategies";
pub const SIDEBAR_NAV_RISK: &str = "Risk";
pub const SIDEBAR_NAV_AUDIT: &str = "Audit";

// ── Lab screen (Phase A — ui-rethink-phase-a-lab T-D-1) ─────────────────────
//
// New sidebar + placeholder copy for the Phase A IA. Additive only —
// operator-locked Constraint 2 (no voice rewrite of existing copy)
// preserved. All strings live here; widgets never inline literals.

/// Sidebar nav label for the Phase A `Lab` screen (ex-`Charts`).
pub const LAB_TITLE: &str = "Lab";
/// Sidebar nav label for the Phase A `Live` screen (ex-`Home`).
pub const LIVE_TITLE: &str = "Live";
/// Sidebar nav label for the Phase A `Trail` screen (ex-`Audit`).
pub const TRAIL_TITLE: &str = "Trail";
/// Placeholder body copy for the `Compare` screen.
/// Phase E wires the real matrix body; this constant is retained for one cycle.
#[deprecated(
    since = "0.3.0",
    note = "Compare now renders the matrix body — Phase F removes this constant"
)]
pub const COMPARE_PLACEHOLDER: &str = "Compare view — coming in Phase E.";
/// Placeholder body copy for the `Memory` screen (Phase F body).
pub const MEMORY_PLACEHOLDER: &str = "Memory view — coming in Phase F.";
/// Placeholder body copy for the `Models` screen (Phase F body).
pub const MODELS_PLACEHOLDER: &str = "Models view — coming in Phase F.";
/// Placeholder body copy for the `Settings` screen (Phase C body).
/// Phase C wires the real rollup body; this constant is retained for one cycle.
#[deprecated(
    since = "0.3.0",
    note = "Settings now renders the rollup body — Phase D removes this constant"
)]
pub const SETTINGS_PLACEHOLDER: &str = "Settings — coming in Phase C.";

/// Sidebar nav label for the Phase A `Compare` screen.
pub const SIDEBAR_NAV_COMPARE: &str = "Compare";
/// Sidebar nav label for the Phase A `Memory` screen.
pub const SIDEBAR_NAV_MEMORY: &str = "Memory";
/// Sidebar nav label for the Phase A `Models` screen.
pub const SIDEBAR_NAV_MODELS: &str = "Models";
/// Sidebar nav label for the Phase A `Settings` screen.
pub const SIDEBAR_NAV_SETTINGS: &str = "Settings";

/// Badge rendered adjacent to the date-range picker when the loader
/// fell back to a superset report (R5.4). Combined with the report
/// name in the widget; the constant owns the prefix copy.
pub const LAB_NARROWED_FROM_BADGE: &str = "Narrowed from";

/// Tooltip / hint rendered on the `pair_chip` row when the Binance venue
/// prefix is shown alongside the symbol label (R3.1 — venue suffix when
/// ambiguous). Phase A universe is single-venue so the suffix is hidden by
/// default; the constant exists so the widget has no inline literals.
pub const PAIR_CHIP_VENUE_BINANCE: &str = "Binance";

/// Empty-state hint shown in the Lab body when no strategy has been
/// selected (cold start / R4.4). Rendered in the chart's centre overlay.
pub const LAB_NO_STRATEGY_HINT: &str = "Pick a strategy to see fills and equity";

/// Empty-state hint shown in the Lab body when no pair has been selected.
pub const LAB_NO_PAIR_HINT: &str = "Pick a pair to get started";

/// Date-range picker "Custom…" option label (R5.1 — Phase A text-field path).
pub const DATE_RANGE_CUSTOM_LABEL: &str = "Custom\u{2026}";

/// "+"-affordance label on strategy chips not in the compare set (T-D-6).
/// Non-alphabetic single char — structural UI symbol, not prose.
pub const STRATEGY_CHIP_COMPARE_ADD: &str = "+";
/// "×"-affordance label on strategy chips already in the compare set (T-D-6).
/// Using the Unicode multiplication sign U+00D7 (not an ASCII 'x').
pub const STRATEGY_CHIP_COMPARE_REMOVE: &str = "\u{00D7}";
/// Em-dash separator used in date-range picker between start and end fields.
/// Non-alphabetic punctuation — structural separator, not prose.
pub const DATE_RANGE_SEPARATOR: &str = "\u{2014}";

/// Date-range custom start-field placeholder text.
pub const DATE_RANGE_START_PLACEHOLDER: &str = "YYYY-MM-DD";

/// Date-range custom end-field placeholder text.
pub const DATE_RANGE_END_PLACEHOLDER: &str = "YYYY-MM-DD";

/// Error copy shown on the start / end date fields when the input is not
/// a valid ISO-8601 date (R5.1 parse-error highlight).
pub const DATE_RANGE_INVALID_DATE: &str = "Invalid date (use YYYY-MM-DD)";

/// Toast shown when the operator tries to add a 5th comparison strategy
/// (R8.2 / R4.2 / T-D-16): compare cap is 4.
pub const LAB_COMPARE_CAP_HIT: &str = "Compare cap reached — deselect one to add another.";

/// Dismiss button label on toast cards in the tray widget (cockpit-toast-queue v0.1.0).
/// Uses "×" (U+00D7 MULTIPLICATION SIGN) — not an icon, just a text glyph.
pub const TOAST_DISMISS_BUTTON: &str = "\u{00D7}";

/// Label on the Run backtest button in the Lab screen (T-D-14).
pub const LAB_RUN_BUTTON: &str = "Run";

/// Label shown on the Run button while a backtest is in-flight (T-D-14).
pub const LAB_RUN_BUTTON_RUNNING: &str = "Running\u{2026}";

/// Label on the Run button after a successful run — operator can re-run (T-D-14b).
pub const LAB_RUN_BUTTON_COMPLETED: &str = "Re-run";

/// Label on the Run button after a failed run — operator can retry (T-D-14b).
pub const LAB_RUN_BUTTON_FAILED: &str = "Retry";

/// Label on the Run button when not all selections (pair + strategy + range)
/// are made — button is disabled (lab-end-to-end-v2 Wave D-1.1 F10).
pub const LAB_RUN_BUTTON_DISABLED: &str = "Select pair & strategy";

/// Label on the Run button after a cancelled run — operator can re-run
/// (lab-end-to-end-v2 Wave D-3 T-D3.5 / Q8=(b)).
pub const LAB_RUN_BUTTON_CANCELLED: &str = "Cancelled — Run again";

/// Label on the Stop button rendered in the Lab top-bar while a run is
/// in-flight (lab-end-to-end-v2 Wave D-3 T-D3.4 / R6.3).
pub const LAB_STOP_BUTTON: &str = "Stop";

// ── Run delta badge — ui-rethink-phase-b-lab-run T-D-N13 ─────────────────────

/// Short label for the `PnL` delta column of the `run_delta_badge` (R8.2 / D5).
/// Shows the change in total return between the last two runs on the same tuple.
pub const RUN_DELTA_BADGE_PNL_LABEL: &str = "P&L";

/// Short label for the max-drawdown delta column of the `run_delta_badge` (R8.2 / D5).
/// Shows the change in max drawdown between the last two runs on the same tuple.
pub const RUN_DELTA_BADGE_DD_LABEL: &str = "DD";

// ── Phase E — Compare matrix (ui-rethink-phase-e-compare T-D-N5) ─────────────

/// Phase E — universe-aggregate KPI disclaimer (R2.3 / K7 mitigation).
/// Rendered as a subtitle under the Compare-matrix toolbar AND as a
/// per-cell tooltip on hover when the cell's source report is multi-symbol
/// (§1.4 of decomp.md). Both surfaces ship at v0.1.0.
pub const COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE: &str = "KPI is universe-aggregate, not per-pair (multi-symbol scenario). \
     Per-pair decomposition is v0.2.0 follow-up.";

/// Phase E — toolbar range-picker label (R1.2).
pub const COMPARE_TOOLBAR_RANGE_LABEL: &str = "Range";

/// Phase E — toolbar KPI-axis dropdown label (R1.2 / R6.3).
pub const COMPARE_TOOLBAR_KPI_LABEL: &str = "KPI";

/// Phase E — centred label on empty-but-legal matrix cells (Q4=b). The
/// active `ACCENT_500` hairline border distinguishes this from blanked cells.
pub const COMPARE_CELL_RUN_LABEL: &str = "Run";

/// Phase E — centred label on blanked matrix cells (Q8=b — outside strategy
/// universe). Passive hairline border; distinguishable from the "Run" affordance.
pub const COMPARE_CELL_BLANKED_LABEL: &str = "\u{2014}"; // em-dash

// ── lab-compare-equity-overlay T2 — two-run equity overlay ──────────────────

/// Overlay-select chip glyph on a populated cell (unselected). Clicking it adds
/// the cell to the two-run equity-overlay ring. Reuses the `+` add-metaphor of
/// the Lab compare chip (`STRATEGY_CHIP_COMPARE_ADD`).
pub const COMPARE_CELL_OVERLAY_ADD: &str = "+";

/// Overlay-select chip glyph on a populated cell when it IS selected (toggle
/// off on click). A check mark signals "in the overlay".
pub const COMPARE_CELL_OVERLAY_SELECTED: &str = "\u{2713}"; // check mark

/// Tooltip on the cell overlay-select chip — tells the operator what the `+`
/// does (plain language, no jargon).
pub const COMPARE_CELL_OVERLAY_HINT: &str = "Add this run to the equity overlay below";

/// Title above the two-run equity-overlay chart panel.
pub const COMPARE_OVERLAY_TITLE: &str = "Equity overlay";

/// Empty-state body for the overlay panel when no run is selected — tells the
/// operator the next action (no blank screens; plain language).
pub const COMPARE_OVERLAY_EMPTY: &str =
    "Pick up to two runs with the + on a cell to overlay their equity curves here.";

/// Caption under the overlay chart naming which run is which colour. The
/// `{primary}` / `{compare}` placeholders are filled with the selected cells'
/// strategy × pair labels at render time.
pub const COMPARE_OVERLAY_LEGEND_PRIMARY: &str = "Run A";

/// Second-slot legend label (the `ACCENT_2` curve).
pub const COMPARE_OVERLAY_LEGEND_COMPARE: &str = "Run B";

/// Coloured swatch glyph (filled circle) prefixing each overlay legend entry.
/// Decorative — tinted `ACCENT` / `ACCENT_2` at render time to match the curve.
pub const COMPARE_OVERLAY_LEGEND_SWATCH: &str = "\u{25CF}"; // ●

/// Shown in the overlay panel when a selected run has no companion equity CSV
/// (an older committed report) — so its curve cannot be drawn. Explains why
/// the overlay is missing a line rather than failing silently.
pub const COMPARE_OVERLAY_NO_SERIES: &str =
    "A selected run has no saved equity curve (older report) — re-run it in Lab to overlay.";

/// Short label for the Sharpe ratio delta column of the `run_delta_badge` (R8.2 / D5).
/// Shows the change in annualised Sharpe ratio between the last two runs.
pub const RUN_DELTA_BADGE_SHARPE_LABEL: &str = "SR";

// ── Phase F — Memory + Models + Assistant (ui-rethink-phase-f-memory-models-assistant T-D-N7) ──

/// Phase F — Memory screen: empty-state placeholder copy (R1.4).
/// Rendered when the reflection DB has zero lesson cards (cold-boot).
pub const MEMORY_EMPTY_STATE: &str =
    "No memory entries yet. Memory populates as strategies close trades.";

/// Phase F — Memory screen: toolbar Cards mode button label (Q1=(a)).
pub const MEMORY_TOOLBAR_CARDS_LABEL: &str = "Cards";

/// Phase F — Memory screen: toolbar Cluster mode button label (R1.2 reserved).
pub const MEMORY_TOOLBAR_CLUSTER_LABEL: &str = "Cluster";

/// Phase F — Memory screen: Cluster mode disabled tooltip (R1.2 v0.2.0 reservation).
pub const MEMORY_CLUSTER_MODE_DISABLED_TOOLTIP: &str =
    "Cluster view ships when distillation lands (v0.2.0)";

/// Phase F — Memory screen: card chevron tooltip / button label (R6.1).
pub const MEMORY_CARD_TRAIL_LINK_LABEL: &str = "View in Trail \u{2192}";

/// Phase F — Models screen: empty-state placeholder copy (R2.4 / Q3=(a)).
pub const MODELS_EMPTY_STATE: &str = "No models loaded yet. See `spec/v25-tcn-overlay/feature.md` for how to train v2.5.0 TCN checkpoints.";

/// Phase F — Models screen: sparkline deferred placeholder (K3 / R2.2).
pub const MODELS_SPARKLINE_DEFERRED_TOOLTIP: &str =
    "Forecast quality ships when residual cache populates (v0.2.0)";

/// Phase F — Models screen: sparkline em-dash placeholder cell value (K3).
pub const MODELS_SPARKLINE_PLACEHOLDER: &str = "\u{2014}";

/// Phase F — Models screen: status pill tooltip for Staged v0.1.0 (Q7=(c)).
pub const MODELS_STATUS_STAGED_TOOLTIP: &str = "Lifecycle classification ships in v0.2.0";

/// Phase F — Models screen: family chip "disabled" tooltip for `PatchTST`.
pub const MODELS_FAMILY_PATCHTST_DISABLED_TOOLTIP: &str = "Family ships in v2.5a";

/// Phase F — Models screen: family chip "disabled" tooltip for Transformer.
pub const MODELS_FAMILY_TRANSFORMER_DISABLED_TOOLTIP: &str = "Family ships in v2.5b";

/// Phase F — Models screen: toolbar family filter label.
pub const MODELS_TOOLBAR_FAMILY_LABEL: &str = "Family";

/// Phase F — Models screen: toolbar status filter label.
pub const MODELS_TOOLBAR_STATUS_LABEL: &str = "Status";

/// Phase F — Assistant slot: stub title (R3.2(a) / K7 mitigation).
pub const ASSISTANT_OFFLINE_TITLE: &str = "Assistant offline";

/// Phase F — Assistant slot: stub body copy (R3.2(a) / K7 mitigation).
pub const ASSISTANT_OFFLINE_BODY: &str = "v2 LLM wiring lands in v0.2.0. \
     See spec/v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md \
     for what shipped 2026-05-13.";

/// Phase F — status bar toggle button label when Assistant slot is closed.
pub const ASSISTANT_TOGGLE_OPEN_LABEL: &str = "Assistant (coming soon)";

/// Phase F — status bar toggle button label when Assistant slot is open.
pub const ASSISTANT_TOGGLE_CLOSE_LABEL: &str = "Close Assistant";

// ── Phase F — Assistant slot reasoning-trace body (v3-llm-forecaster Wave F) ──
//
// Strings for the `AssistantMode::ReasoningTrace` body composition (R9.2).
// Runtime-gated per R9.3: only rendered when `llm_forecaster_v3` is enabled
// in agent config — the default-disabled cockpit keeps the
// `ASSISTANT_OFFLINE_*` strings + byte-identical Phase F baseline.

/// Phase F — Assistant slot: panel title in reasoning-trace mode.
pub const ASSISTANT_REASONING_TITLE: &str = "Forecast";

/// Phase F — Assistant slot: header format string for the most-recent
/// forecast summary. Three placeholders: `{symbol}` / `{rating}` /
/// `{confidence}` (R9.2 bullet 1).
pub const ASSISTANT_REASONING_HEADER_FMT: &str =
    "{symbol} \u{00b7} {rating} \u{00b7} conf {confidence}";

/// Phase F — Assistant slot: cost line label (precedes the cumulative
/// spend display, R9.2 bullet 2).
pub const ASSISTANT_REASONING_COST_LABEL: &str = "LLM spend";

/// Phase F — Assistant slot: cost line fallback when no cost-event has
/// fired yet (cold-boot / first-warmup).
pub const ASSISTANT_REASONING_COST_PENDING: &str = "Awaiting first forecast";

/// Phase F — Assistant slot: section header above the reasoning-trace
/// card body (R9.2 bullet 3).
pub const ASSISTANT_REASONING_TRACE_LABEL: &str = "Reasoning";

/// Phase F — Assistant slot: section header above the cited-lessons
/// list (R9.2 bullet 4).
pub const ASSISTANT_REASONING_LESSONS_LABEL: &str = "Cited lessons";

/// Phase F — Assistant slot: empty-state copy when the LLM did not cite
/// any lessons in its forecast (the prompt allows the model to return
/// an empty `cited_lesson_ids` array if it found no relevant priors).
pub const ASSISTANT_REASONING_NO_LESSONS: &str = "No prior trades cited";

/// Phase F — Assistant slot: prefix for a cited-lesson reference row
/// when the operator's Memory cache has not yet hydrated the matching
/// card body (lookup miss). Keeps the row visible so the operator can
/// see what the model cited even before Memory loads.
pub const ASSISTANT_REASONING_LESSON_PENDING_FMT: &str = "{card_id} (loading\u{2026})";

/// Phase F — Assistant slot: history section header (R9.2 bullet 5).
pub const ASSISTANT_REASONING_HISTORY_LABEL: &str = "History";

/// Phase F — Assistant slot: history-empty fallback when only the
/// most-recent forecast exists.
pub const ASSISTANT_REASONING_HISTORY_EMPTY: &str = "No prior forecasts yet";

/// Phase F — Assistant slot: history-row format string. Two
/// placeholders: `{rating}` / `{confidence}` (compact list per R9.2).
pub const ASSISTANT_REASONING_HISTORY_ROW_FMT: &str = "{rating} \u{00b7} conf {confidence}";

/// Phase F — Assistant slot: chevron / trail-link affordance label.
/// Click opens the audit row for the underlying journal entry
/// (R9.2 bullet 6).
pub const ASSISTANT_REASONING_TRAIL_LINK_LABEL: &str = "Open audit trail";

/// Phase F — Assistant slot: warming-up empty state title when
/// `ReasoningTrace` mode is active but no forecast has fired yet
/// (R9.3 mode-on / data-empty path).
pub const ASSISTANT_REASONING_WARMING_TITLE: &str = "Warming up";

/// Phase F — Assistant slot: warming-up empty state body copy. Tells
/// the operator the LLM strategy is enabled but waiting for the first
/// fire (24-bar default cadence).
pub const ASSISTANT_REASONING_WARMING_BODY: &str =
    "LLM forecaster is enabled. The first forecast fires after the warm-up window completes.";

/// Phase F — deprecated `MEMORY_PLACEHOLDER` (Phase A placeholder — replaced by real body).
#[deprecated(
    since = "0.4.0",
    note = "Memory now renders the real body — replaced by screens::memory::view"
)]
pub const MEMORY_PLACEHOLDER_DEPRECATED: &str = "Memory view — coming in Phase F.";

/// Phase F — deprecated `MODELS_PLACEHOLDER` (Phase A placeholder — replaced by real body).
#[deprecated(
    since = "0.4.0",
    note = "Models now renders the real body — replaced by screens::models::view"
)]
pub const MODELS_PLACEHOLDER_DEPRECATED: &str = "Models view — coming in Phase F.";

// ── Training panel — cockpit-training-control T-D-N1/N2/N3/N4/N16 ────────────

/// Header chip label for the collapsed training panel.
pub const TRAINING_PANEL_HEADER: &str = "Train";

/// Train button label (primary action — starts training run).
pub const TRAINING_BUTTON_TRAIN: &str = "Train";

/// Cancel button label (visible only while training is in-flight).
pub const TRAINING_BUTTON_CANCEL: &str = "Cancel";

/// Clear log button label.
pub const TRAINING_BUTTON_CLEAR_LOG: &str = "Clear log";

/// Status strip — idle state.
pub const TRAINING_STATUS_IDLE: &str = "Idle";

/// Status strip — training in progress (Tier 1; no epoch counts).
pub const TRAINING_STATUS_RUNNING: &str = "Training\u{2026}";

/// Status strip — training in progress with epoch info (Tier 2 format string).
/// Placeholder: `{}` = current epoch, `{}` = total epochs, `{}` = elapsed seconds.
pub const TRAINING_STATUS_TRAINING_FMT: &str = "Training (epoch {}/{}, t={}s)";

/// Status strip — training cancelled.
pub const TRAINING_STATUS_CANCELLED: &str = "Cancelled";

/// Status strip — training failed (format string). Placeholder: `{}` = error.
pub const TRAINING_STATUS_FAILED_FMT: &str = "Failed: {}";

/// Status strip — training completed successfully (format string).
/// Placeholder: `{}` = short model revision SHA.
pub const TRAINING_STATUS_DONE_FMT: &str = "Done: {}";

/// Status strip — orphan run annotation, process still alive (Tier 2).
/// Placeholder: `{}` = `run_id` short prefix.
pub const ORPHAN_LIVE_FMT: &str =
    "Training orphan detected (run {}, pid alive) — click Train to reconnect";

/// Status strip — orphan run annotation, process dead (Tier 2).
/// Placeholder: `{}` = `run_id` short prefix.
pub const ORPHAN_DEAD_FMT: &str = "Training orphan (run {}) — process gone, check logs";

/// chart-x-axis-local-time v1.11 / chart-fixture-line-clipping v1.0.0 —
/// env-var name that integration test runners
/// (`tests/render_snapshots.rs`, `tests/visual_snapshots.rs`) set before
/// invoking `iced_test::screenshot`. The production
/// `widgets::chart::local_offset_or_utc` checks this env var to force
/// `UtcOffset::UTC` and keep visual baselines machine-independent. NOT
/// operator-visible — internal contract between chart canvas + test
/// harness.
pub const CHART_FORCE_UTC_ENV: &str = "UI_CHART_FORCE_UTC";

/// Empty state for the training log ring buffer.
pub const TRAINING_LOG_EMPTY: &str = "No training output yet — press Train to start.";

/// "Jump to bottom" chip label for the training log when scroll is frozen.
pub const TRAINING_LOG_JUMP_TO_BOTTOM: &str = "Jump to bottom";

/// Empty state for the training loss-curve plot (no run in flight).
pub const TRAINING_PLOT_EMPTY: &str = "No training run in flight";

/// Warming-up state for the training plot (run started, no epoch data yet).
pub const TRAINING_PLOT_WARMING_UP: &str = "Warming up \u{2014} first epoch landing shortly";

/// Per-epoch row format in the training plot text summary.
/// Substitutions: `epoch`, `train_loss`, `train_bar`, `val_loss`, `val_bar`.
pub const TRAINING_PLOT_EPOCH_ROW_FMT: &str = "E{:>3}  T:{:.4} {}  V:{:.4} {}";

/// Header line for the training plot running state.
/// Substitutions: `n_epochs`, `y_scale`.
pub const TRAINING_PLOT_HEADER_FMT: &str = "Loss curves \u{2014} {} epochs (y_scale: {})";

/// Footer line showing the latest epoch's loss values.
/// Substitutions: `train_loss`, `val_loss`, `epoch`.
pub const TRAINING_PLOT_LATEST_FMT: &str = "Latest: train={:.4}  val={:.4}  (epoch {})";

/// Build the per-epoch row string for the text-mode training plot.
///
/// Canonical format: `TRAINING_PLOT_EPOCH_ROW_FMT`.
/// Called from `widgets::training_plot` to avoid prose literals inside `widgets/`.
#[must_use]
pub fn fmt_training_plot_epoch_row(
    epoch: u32,
    train: f32,
    train_bar: &str,
    val: f32,
    val_bar: &str,
) -> String {
    format!("E{epoch:>3}  T:{train:.4} {train_bar}  V:{val:.4} {val_bar}")
}

/// Build the header line for the text-mode training plot running state.
///
/// Canonical format: `TRAINING_PLOT_HEADER_FMT`.
#[must_use]
pub fn fmt_training_plot_header(n_epochs: usize, y_scale_label: &str) -> String {
    format!("Loss curves \u{2014} {n_epochs} epochs (y_scale: {y_scale_label})")
}

/// Build the latest-epoch footer for the text-mode training plot.
///
/// Canonical format: `TRAINING_PLOT_LATEST_FMT`.
#[must_use]
pub fn fmt_training_plot_latest(train_loss: f32, val_loss: f32, epoch: u32) -> String {
    format!("Latest: train={train_loss:.4}  val={val_loss:.4}  (epoch {epoch})")
}

// ── Charts screen (Phase 2 — T1608, T1610) ───────────────────────────────────

/// Centred label rendered when the chart canvas has zero bars buffered.
pub const CHART_NO_DATA: &str = "No data";

// ── Chart tooltip + ghost-signal layer (chart-buy-sell-emphasis v1.9, T2007) ──
//
// Six tooltip fields per Q4-operator-resolved: Side / Price / Quantity /
// Notional / Timestamp / Strategy ID. Plus a one-row ghost badge for
// strategy-intended (not-yet-executed) markers per R5.6.

pub const CHART_TOOLTIP_SIDE_BUY: &str = "Buy";
pub const CHART_TOOLTIP_SIDE_SELL: &str = "Sell";
pub const CHART_TOOLTIP_PRICE_LABEL: &str = "Price";
pub const CHART_TOOLTIP_QTY_LABEL: &str = "Qty";
pub const CHART_TOOLTIP_NOTIONAL_LABEL: &str = "Notional";
pub const CHART_TOOLTIP_TS_LABEL: &str = "Time";
pub const CHART_TOOLTIP_STRATEGY_LABEL: &str = "Strategy";
pub const CHART_TOOLTIP_STRATEGY_NONE: &str = "—";
/// Top-row badge on ghost-signal tooltips ("Intent only — not executed").
pub const CHART_TOOLTIP_GHOST_BADGE: &str = "Intent — not executed";
/// Suffix appended to the side row when `was_clamped == true`.
pub const CHART_TOOLTIP_CLAMP_SUFFIX: &str = " (clamped)";

// ── Counter views (chart-buy-sell-emphasis v1.9, T2021) ──────────────────────
//
// Cumulative window-volume tile labels + per-bar histogram label + open-
// position-mirror labels for the Charts screen status strip + below-strip.

pub const CHART_VOLUME_TILE_BUYS_LABEL: &str = "Buys in window";
pub const CHART_VOLUME_TILE_SELLS_LABEL: &str = "Sells in window";
pub const CHART_VOLUME_TILE_NET_LABEL: &str = "Net";
pub const CHART_VOLUME_TILE_TRADES_SUFFIX: &str = "trades";
pub const CHART_VOLUME_HISTOGRAM_LABEL: &str = "Per-bar volume";
pub const CHART_POSITION_MIRROR_LABEL: &str = "Open position";
pub const CHART_POSITION_MIRROR_NONE: &str = "No open position on this symbol.";
/// lab-polish-round-2 R1 — label for the position-curve strip below the price
/// chart in the Lab screen. Analogous to `CHART_VOLUME_HISTOGRAM_LABEL`.
pub const LAB_POSITION_CURVE_LABEL: &str = "Position size";

// ── Trail node + drawer (silent-quarantine-fix-2026-05-26) ───────────────────
//
// `trail_node.rs` and `trail_drawer.rs` previously carried these strings as
// module-local `const` items, which still trip the `consistency` hygiene test
// (`no_inline_user_visible_strings_in_widgets`) because they scan the widget
// file content for any `"..."` literal regardless of whether it's pub or
// const-bound. The convention is to keep user-visible copy here.

pub const TRAIL_DRAWER_CLOSE_LABEL: &str = "Close";
pub const TRAIL_DRAWER_FILL_TITLE: &str = "Fill";
pub const TRAIL_DRAWER_SIGNAL_TITLE: &str = "Signal";
pub const TRAIL_DRAWER_FORECAST_TITLE: &str = "Forecast";
pub const TRAIL_DRAWER_LLM_TITLE: &str = "LLM Debate";
pub const TRAIL_DRAWER_LLM_PLACEHOLDER: &str = "(no transcript recorded)";

// Trail drawer — Signal payload labels
pub const TRAIL_SIGNAL_SIDE_LABEL: &str = "Side";
pub const TRAIL_SIGNAL_QTY_LABEL: &str = "Qty";
pub const TRAIL_SIGNAL_PRICE_LABEL: &str = "Price";
pub const TRAIL_SIGNAL_PRICE_MARKET: &str = "market";
pub const TRAIL_SIGNAL_CLAMPED_LABEL: &str = "Clamped";
pub const TRAIL_SIGNAL_CLAMP_REASON_LABEL: &str = "Clamp reason";

// Trail drawer — Forecast payload labels
pub const TRAIL_FORECAST_DIRECTION_LABEL: &str = "Direction";
pub const TRAIL_FORECAST_CONFIDENCE_LABEL: &str = "Confidence";
pub const TRAIL_FORECAST_MODEL_LABEL: &str = "Model";
pub const TRAIL_FORECAST_CACHE_HIT_LABEL: &str = "Cache hit";

// Trail node — kind labels (mirror drawer titles but kept separate so the
// IDs stay decoupled if the trail-node UI evolves independently).
pub const TRAIL_NODE_FILL_LABEL: &str = "Fill";
pub const TRAIL_NODE_SIGNAL_LABEL: &str = "Signal";
pub const TRAIL_NODE_FORECAST_LABEL: &str = "Forecast";
pub const TRAIL_NODE_LLM_LABEL: &str = "LLM Debate";

// Trail node — empty-state placeholders
pub const TRAIL_NODE_NO_UPSTREAM_FILL: &str = "(no upstream fill recorded)";
pub const TRAIL_NODE_NO_UPSTREAM_SIGNAL: &str = "(no upstream signal recorded)";
pub const TRAIL_NODE_NO_UPSTREAM_FORECAST: &str = "(no upstream forecast recorded)";
pub const TRAIL_NODE_NO_LLM_TRANSCRIPT: &str = "(no transcript recorded)";

// Shared boolean labels (used by both trail drawer's Signal "Clamped" row and
// Forecast "Cache hit" row).
pub const TRAIL_BOOL_YES: &str = "yes";
pub const TRAIL_BOOL_NO: &str = "no";

// (TRAIL_COL_REGIME / TRAIL_NO_REGIME_TAG removed 2026-05-29 — see
// `spec/dev-notes/post-v3-trail-ui-cleanup-2026-05-29.md`. v3-regime-classifier
// was operator-retired after Wave E proved -0.294 Sharpe-delta; the regime
// column scaffolding shipped by Wave D was never wired into the Trail view
// and no dispatcher emits `JournalEntry::RegimeTag` in production.)

// Compare matrix screen — empty state placeholder.
pub const MATRIX_EMPTY_STATE: &str =
    "No strategies registered — configure strategies to populate the matrix.";

// ── Activity tape (cockpit-activity-status-bar v0.1.0 R2/R7) ─────────────────

/// Format an elapsed duration into the operator-facing activity-tape label.
/// Output convention: `<1s` for sub-second, `Ns` for 1-59s, `NmNs` for ≥60s.
/// Kept here (not inline in `widgets/activity_tape.rs`) per the
/// `consistency::no_inline_user_visible_strings_in_widgets` hygiene contract.
#[must_use]
pub fn activity_tape_elapsed_label(elapsed: std::time::Duration) -> String {
    let total_secs = elapsed.as_secs();
    if total_secs == 0 {
        "<1s".to_owned()
    } else if total_secs < 60 {
        format!("{total_secs}s")
    } else {
        let m = total_secs / 60;
        let s = total_secs % 60;
        format!("{m}m{s}s")
    }
}

/// Trail drawer Forecast summary text. Captures the `direction` and
/// `confidence` fields from the active forecast payload. Kept as a helper
/// rather than a raw const so the `format!` template lives in this module
/// (widget files must not carry inline format strings with prose content per
/// the `consistency::no_inline_user_visible_strings_in_widgets` hygiene test).
#[must_use]
pub fn trail_forecast_summary(direction: &str, confidence: &str) -> String {
    format!("predicted {direction} with confidence {confidence}")
}

// ── Chart legend (chart-canvas-overhaul v1.10, T3015) ────────────────────────
//
// Five entries rendered in the top-right inset card over the chart canvas
// (Q5 — architect's pick): Buy fill / Sell fill / Buy signal (ghost) /
// Sell signal (ghost) / Price line.  Each label sits to the right of its
// glyph (a downsized sibling of the chart's executed-fill triangle) at
// `text::MICRO` size with `color::FG_2` colour.  Locale: English; no
// i18n for v1.10.  The widget at `crates/ui/src/widgets/chart_legend.rs`
// resolves these constants — never inline.

/// Legend row for the executed-fill **buy** marker (`UP_500` ▲ triangle).
pub const CHART_LEGEND_BUY_LABEL: &str = "Buy";
/// Legend row for the executed-fill **sell** marker (`DOWN_500` ▼ triangle).
pub const CHART_LEGEND_SELL_LABEL: &str = "Sell";
/// Legend row for the **buy-signal** ghost marker (`UP_400` ▲ at 60 % alpha).
pub const CHART_LEGEND_BUY_GHOST_LABEL: &str = "Buy signal";
/// Legend row for the **sell-signal** ghost marker (`DOWN_400` ▼ at 60 % alpha).
pub const CHART_LEGEND_SELL_GHOST_LABEL: &str = "Sell signal";
/// Legend row for the chart **price line** (`ACCENT` stroke).
pub const CHART_LEGEND_PRICE_LABEL: &str = "Price";
/// Equity legend row for the primary strategy equity curve (`ACCENT` line stub).
pub const CHART_LEGEND_EQUITY_LABEL: &str = "Equity";
/// Faded "no data" label shown in the compare legend when no report exists for
/// the selected pair (R8.4 / T-D-15).
pub const CHART_LEGEND_COMPARE_NO_DATA: &str = "No data";

/// Format suffix appended to equity axis labels when the value ≥ 1 000 (T-D-11).
/// Full format: `format!("${:.0}K", value / 1_000.0)`.
/// The `K` suffix is user-visible prose → routes through `strings`.
pub const CHART_EQUITY_AXIS_THOUSAND_SUFFIX: &str = "K";

// ── Debug screen (Phase 2 — T1605) ───────────────────────────────────────────

/// Placeholder copy for the Debug screen's logs/metrics surface (Q9 — the
/// real logs surface lands with a future structured-metrics brief).
pub const DEBUG_LOGS_PLACEHOLDER: &str = "Logs surface lands with a future metrics brief";

/// "Not yet" placeholder rendered for `Screen::Strategies / Risk / Audit`
/// dispatch in Phase 2 (Phase 3 lands the real screens).
pub const SCREEN_NOT_YET: &str = "Not yet";

// ── Strategies-detail screen (Phase 3 — T1704, T1706) ────────────────────────
//
// Net-new constants (additive — operator-locked Constraint 2 unchanged).

pub const STRATEGIES_PANEL_TITLE: &str = "Strategies";
pub const STRATEGIES_SELECT_PROMPT: &str = "Select a strategy";
pub const STRATEGIES_PARAMS_TITLE: &str = "Parameters";
pub const STRATEGIES_EVENTS_TITLE: &str = "Recent signal events";
/// Phase 4 (T1811) — rendered while the cockpit Strategies-detail
/// sparkline awaits the equity-curve fetch dispatched on
/// `Message::SelectStrategy`. Replaces the retired Phase 3
/// `STRATEGIES_SPARKLINE_DEFERRED` placeholder constant.
pub const STRATEGIES_SPARKLINE_LOADING: &str = "Loading equity history…";

// ── Backtest viewer (Phase 4 — T1805 / T1806) ───────────────────────────
//
// Net-new constants (additive — operator-locked Constraint 2 unchanged).
// All copy lives here so the viewer's surface stays grep-friendly and
// the consistency-test fixture allow-list can find every visible
// string in one place.

pub const KPI_TOTAL_RETURN_LABEL: &str = "Total return";
pub const KPI_CAGR_LABEL: &str = "CAGR";
pub const KPI_SHARPE_LABEL: &str = "Sharpe";
pub const KPI_MAX_DD_LABEL: &str = "Max DD";
pub const KPI_WIN_RATE_LABEL: &str = "Win rate";
pub const KPI_TRADES_LABEL: &str = "Trades";

// ── Lab single-run KPI strip (lab-end-to-end-v2 Wave D-1.1 F8) ───────────────
/// Label for the "Final equity" card in the Lab single-run KPI strip.
pub const LAB_KPI_FINAL_EQUITY_LABEL: &str = "Final equity";
/// Label for the "Max DD" card in the Lab single-run KPI strip (matches viewer).
pub const LAB_KPI_MAX_DD_LABEL: &str = "Max DD";
/// Label for the "Trades" card in the Lab single-run KPI strip.
pub const LAB_KPI_TRADES_LABEL: &str = "Trades";
/// Label for the "Fees" card in the Lab single-run KPI strip.
pub const LAB_KPI_FEES_LABEL: &str = "Fees";
/// Label for the "Sharpe" card — always shows em-dash (Phase C follow-up).
pub const LAB_KPI_SHARPE_LABEL: &str = "Sharpe";
/// Label for the "Return" card in the Lab single-run KPI strip.
pub const LAB_KPI_RETURN_LABEL: &str = "Return";

// lab-polish-round-2 R3 — densified KPI strip (2-row 4-column layout).
/// Label for the "Buys" card.
pub const LAB_KPI_BUYS_LABEL: &str = "Buys";
/// Label for the "Sells" card.
pub const LAB_KPI_SELLS_LABEL: &str = "Sells";
/// Label for the "Net Δ" card — `final_equity - initial_equity` (USDT).
pub const LAB_KPI_NET_DELTA_LABEL: &str = "Net Δ";

/// Rendered below the KPI strip when the parser returned a
/// `BacktestMetrics::all_absent()` shape OR the `metrics` panel state
/// is `Error` (R2.6 / Q3 graceful fallback).
pub const VIEWER_METRICS_UNAVAILABLE: &str = "Backtest metrics unavailable";

/// Centred label rendered when the equity curve / drawdown band canvas
/// has zero data points (R4.7 / R7.5).
pub const VIEWER_NO_EQUITY_DATA: &str = "No equity data";

/// Prefix rendered when the equity curve / drawdown band can't be
/// drawn because the underlying read failed (R4.7 / R7.5 error
/// branch). Combined with the underlying error message via
/// `format!("{prefix}{msg}")` so all "X unavailable: …" copy lives
/// in `ui::strings`.
pub const VIEWER_EQUITY_UNAVAILABLE_PREFIX: &str = "Equity curve unavailable: ";

/// Prefix rendered in the cockpit Strategies-detail sparkline slot
/// when the equity-curve fetch errors (T1811).
pub const STRATEGIES_EQUITY_HISTORY_UNAVAILABLE_PREFIX: &str = "Equity history unavailable: ";

/// Em-dash literal used in KPI strip cells when a metric is marked-
/// absent (R3.5 / R2.6 / Q3 graceful fallback). Constant rather than
/// inline literal so the consistency-test grep stays clean.
pub const KPI_DASH_PLACEHOLDER: &str = "—";

/// Unicode minus sign rendered as the negative-value prefix in the
/// KPI strip (R2.4 — `Total return` and `Max DD` use this prefix on
/// negatives). Distinct from ASCII `-` so the visual contrast matches
/// the Lumen reference component.
pub const MINUS_SIGN_LITERAL: &str = "\u{2212}";

// ── Risk / Limits screen (Phase 3 — T1708) ───────────────────────────────────

pub const RISK_PANEL_TITLE: &str = "Risk and limits";
pub const RISK_LOADING: &str = "Risk state loading";
pub const RISK_EXPOSURE_SECTION_TITLE: &str = "Per-venue exposure";
pub const RISK_DAILY_LOSS_SECTION_TITLE: &str = "Daily loss";
pub const RISK_KILL_THRESHOLD_SECTION_TITLE: &str = "Kill threshold proximity";
pub const RISK_FEED_UNAVAILABLE_PREFIX: &str = "Risk feed unavailable: ";

// ── Audit / Journal screen (Phase 3 — T1709, T1710) ──────────────────────────

pub const AUDIT_PANEL_TITLE: &str = "Audit journal";
pub const AUDIT_FILTER_VENUE_LABEL: &str = "Venue";
pub const AUDIT_FILTER_SYMBOL_LABEL: &str = "Symbol";
pub const AUDIT_FILTER_KIND_LABEL: &str = "Kind";
pub const AUDIT_FILTER_TIME_LABEL: &str = "Time range";
pub const AUDIT_FILTER_NO_MATCH: &str = "No journal rows match these filters";
pub const AUDIT_LOADING: &str = "Loading journal rows…";
pub const AUDIT_PREV_LABEL: &str = "Prev";
pub const AUDIT_NEXT_LABEL: &str = "Next";
pub const AUDIT_KIND_ALL: &str = "All";
pub const AUDIT_KIND_FILL: &str = "Fill";
pub const AUDIT_KIND_STRATEGY_EVENT: &str = "Strategy event";
pub const AUDIT_KIND_RECONCILIATION: &str = "Reconciliation";
pub const AUDIT_TIME_LAST_1H: &str = "Last 1 h";
pub const AUDIT_TIME_LAST_24H: &str = "Last 24 h";
pub const AUDIT_TIME_LAST_7D: &str = "Last 7 d";
pub const AUDIT_COL_TIME: &str = "Time";
pub const AUDIT_COL_VENUE: &str = "Venue";
pub const AUDIT_COL_SYMBOL: &str = "Symbol";
pub const AUDIT_COL_KIND: &str = "Kind";
pub const AUDIT_COL_DESCRIPTION: &str = "Description";
pub const AUDIT_COL_STRATEGY_ID: &str = "Strategy";
pub const AUDIT_QUERY_FAILED_PREFIX: &str = "Journal query failed: ";

// ── HumanControl panel (Phase 5 — T1904 / T1905 / T1911) ─────────────────────
//
// Net-new constants per the Phase 5 Design's "HumanControl panel widget
// contract" sub-section. Additive only — operator-locked Constraint 2
// (no voice rewrite of existing copy) preserved.

pub const PANEL_HUMAN_CONTROL_TITLE: &str = "You're in control";
pub const PANEL_HUMAN_CONTROL_META: &str = "Human-in-the-loop";

/// Phase 5 R3.4 — error-state copy for the three mirror rows when
/// `Cockpit::risk_state` is in `Error` or the panel can't read the
/// limits.
pub const HUMAN_CONTROL_LIMITS_UNAVAILABLE: &str = "Risk limits unavailable";

pub const HUMAN_CONTROL_DAILY_LOSS_LABEL: &str = "Daily loss limit";
pub const HUMAN_CONTROL_MAX_POSITION_LABEL: &str = "Max position";
pub const HUMAN_CONTROL_USED_TODAY_LABEL: &str = "Used today";

// ── Execution-mode segmented control (Phase 5 — T1911) ───────────────────────

pub const EXECUTION_MODE_OBSERVE_LABEL: &str = "Observe";
pub const EXECUTION_MODE_SUPERVISED_LABEL: &str = "Supervised";
pub const EXECUTION_MODE_AUTO_LABEL: &str = "Auto";

/// Per-mode hint copy below the segment row, mirrored from
/// `HumanControl.jsx:27–31` with the project's voice discipline.
pub const EXECUTION_MODE_OBSERVE_HINT: &str = "Watch only — no orders sent.";
pub const EXECUTION_MODE_SUPERVISED_HINT: &str = "Each decision needs your approval.";
pub const EXECUTION_MODE_AUTO_HINT: &str = "Within-envelope autonomy.";

// ── Pause-strategy per-row button (Phase 5 — T1907) ──────────────────────────

pub const STRATEGY_PAUSE_LABEL: &str = "Pause";
pub const STRATEGY_RESUME_LABEL: &str = "Resume";

// ── Override-risk-veto modal (Phase 5 — T1909) ───────────────────────────────
//
// Mirrors the kill-confirm typed-confirm pattern; phrase = `OVERRIDE`
// (not `HALT BTC`) because the surface differs (per-veto override vs
// global kill).

pub const OVERRIDE_RISK_VETO_PHRASE: &str = "OVERRIDE";
pub const OVERRIDE_RISK_VETO_DIALOG_TITLE: &str = "Override risk veto";
pub const OVERRIDE_RISK_VETO_DIALOG_BODY: &str =
    "This bypasses the risk engine for the surfaced veto. Type OVERRIDE exactly to confirm.";
pub const OVERRIDE_RISK_VETO_PHRASE_MISMATCH_HINT: &str = "Type OVERRIDE exactly to enable confirm";
pub const OVERRIDE_RISK_VETO_CONFIRM_LABEL: &str = "Override veto";
pub const OVERRIDE_RISK_VETO_CANCEL_LABEL: &str = "Cancel";
pub const OVERRIDE_RISK_VETO_BUTTON_LABEL: &str = "Override";

/// Sidebar nav label for the Phase 5 `HumanControl` screen (Q1 — 7th
/// sidebar entry).
pub const SIDEBAR_NAV_CONTROL: &str = "Control";

// ── Connection states (live broadcast bus, T32) ──────────────────────────────

/// Shown in every panel's error state when the cockpit can't reach the agent
/// process. Tells the operator exactly what to do — not just "connection
/// failed".
pub const CONNECTION_AGENT_UNREACHABLE: &str = "Can't reach the trading agent. Start it with `cargo run --bin agent` and re-launch the \
     cockpit.";
/// Shown when the cockpit falls behind the broadcast and the channel lags.
/// The agent keeps running; the cockpit skipped N events.
pub const CONNECTION_LAGGED: &str = "Cockpit fell behind — some updates were skipped.";
/// Shown when a broadcast channel closes unexpectedly (sender dropped —
/// agent shut down). Distinguishes from unreachable-at-startup.
pub const CONNECTION_CHANNEL_CLOSED: &str =
    "Trading agent disconnected. Check the agent log and restart it.";

// ── Latency badge ────────────────────────────────────────────────────────────

pub const LATENCY_OK_LABEL: &str = "OK";
pub const LATENCY_WARN_LABEL: &str = "Slow";
pub const LATENCY_HIGH_LABEL: &str = "High";
pub const LATENCY_HALTED_LABEL: &str = "Halted";
pub const LATENCY_UNIT_MS: &str = "ms";
pub const LATENCY_UNKNOWN: &str = "—";
pub const LATENCY_HELP: &str = "Venue timestamp vs local clock on the last tick.";

// ── Agent mode banner ────────────────────────────────────────────────────────

pub const MODE_RESEARCH: &str = "Research";
pub const MODE_PAPER: &str = "Paper";
pub const MODE_LIVE: &str = "Live";
pub const MODE_HALTED: &str = "Halted";

// ── Side labels (tape + positions) ───────────────────────────────────────────

pub const SIDE_BUY: &str = "BUY";
pub const SIDE_SELL: &str = "SELL";

// ── Currency unit suffixes (rendered next to numbers) ────────────────────────

pub const UNIT_USDT: &str = "USDT";
pub const UNIT_BTC: &str = "BTC";

/// Euro currency symbol — a PREFIX glyph for budget amounts (product § journey:
/// "a budget (e.g. €200)"). A typographic symbol, kept here (not inline in
/// `num.rs`) so the glyph is reviewable in one place alongside the other units.
pub const CURRENCY_EUR_SYMBOL: &str = "\u{20ac}";

// ── lab-yahoo-realdata — source toggle + cadence badge (T-C3 / R-UI-1) ──────

/// Source toggle chip label for Synthetic GBM bars (default).
pub const LAB_SOURCE_SYNTHETIC: &str = "Synthetic";

/// Source toggle chip label for Yahoo Finance real-data cache.
pub const LAB_SOURCE_YAHOO: &str = "Yahoo";

/// Source toggle chip label for the pinned Binance hourly parquet corpus
/// (simple-strategies-realdata T-B1 — the third real-data source).
pub const LAB_SOURCE_BINANCE: &str = "Binance";

/// Cadence badge label for 1-minute bars.
pub const LAB_CADENCE_1M: &str = "1m";

/// Cadence badge label for 1-hour bars.
pub const LAB_CADENCE_1H: &str = "1h";

/// Cadence badge label for 1-day bars.
pub const LAB_CADENCE_1D: &str = "1d";

/// Tooltip on the Run button when `data_source` = `YahooCache` and the cache
/// is missing for the selected (ticker, interval, range) combination.
/// The CLI hint is appended by the error path in the runner.
pub const LAB_YAHOO_CACHE_MISS_PREFIX: &str = "Yahoo cache miss — run: ";

// ── simple-strategies-realdata — Binance data-missing UX (Q-miss / AC4) ──────
//
// Binance is the PINNED corpus (revision 3a8b96c4…), gitignored + manually
// re-fetchable (ADR-0032). Unlike Yahoo there is NO in-Lab auto-fetch — on a
// cache miss / coverage shortfall the loader returns a typed Err with a
// re-fetch HINT (run the fetch tool). It NEVER silently synthesizes bars: a
// silent synthetic fallback would let the operator believe they are testing
// real BTC while seeing a random walk (the v3-vol-overlay-noop failure class).

/// Shown when the Binance parquet corpus is missing / has insufficient
/// coverage for the selected `(symbol, range)`. `{symbol}` and `{window}`
/// are substituted by `lab::runner::preload_binance_bars`. Points the
/// operator at the offline fetch tool (Binance does NOT auto-fetch in-Lab).
pub const LAB_BINANCE_CACHE_MISS_NOTICE: &str = "No pinned Binance data for {symbol} in {window} \
     \u{2014} re-fetch the corpus: `cargo run --bin fetch_binance_klines` (see data/binance/REVISION.toml).";

/// Shown when the on-disk Binance corpus fails its pinned revision-SHA check
/// (`data/binance/REVISION.toml` mismatch or missing). The corpus is the
/// determinism contract (ADR-0032): a tampered / re-fetched-divergent corpus
/// MUST fail loudly rather than produce a silently-wrong report. `{detail}`
/// carries the underlying `RevisionError` message.
pub const LAB_BINANCE_REVISION_ERROR: &str = "Binance data failed its pinned revision check \
     \u{2014} {detail}. Re-fetch the corpus from data/binance/REVISION.toml before running.";

/// Cache-state badge label when the Yahoo cache directory for the active
/// ticker is missing entirely (no parquet files at `data/yahoo/<TICKER>/`).
pub const LAB_CACHE_STATE_EMPTY: &str = "no cache";

/// Cache-state badge label when the Yahoo cache exists but its most recent
/// file's mtime is older than 24 h. Operator may want to refresh.
pub const LAB_CACHE_STATE_STALE: &str = "stale";

/// Cache-state badge label when the Yahoo cache exists and its most recent
/// file's mtime is within the last 24 h.
pub const LAB_CACHE_STATE_FRESH: &str = "fresh";

/// lab-yahoo-empty-range-ux v0.1.0 — no-data notice template (D-ER-3 / R2 / R-NR.4).
///
/// Rendered in a neutral/muted style (NOT the red ⚠ error treatment) when
/// Yahoo returns HTTP-200 but zero bars for the requested window — an expected
/// outcome for future-dated ranges or delisted tickers.
///
/// `{ticker}` and `{window}` are substituted by
/// `lab::runner::preload_notice::no_data_message`.
/// `{window}` is `start_label..end_label` from the post-clamp
/// `range_to_ms_pair` pair (R2 mandates the actual computed window is shown).
///
/// NO internal variant name (`CacheMiss`, `MissingData`), NO "Check network" hint.
pub const LAB_YAHOO_NO_DATA_NOTICE: &str = "No Yahoo data for {ticker} in {window} \
     \u{2014} the range may be future-dated or the ticker may be delisted.";

/// lab-yahoo-realdata v0.1.2 (T-DU2 / R-NR.2) — operator Q3 prefix for the
/// aggregate cache-state summary badge in the Lab toolbar. The badge label
/// is built via [`fmt_lab_cache_state_summary`] and resolves to
/// `"Yahoo cache: {N} tickers · last fetch {YYYY-MM-DD}"`.
///
/// **Operator lock 2026-05-27:** prefix is `"Yahoo cache: "` (not the
/// analyst's bare `"Cache: "`) to disambiguate from any future Binance /
/// synthetic cache surface. N=0 path reuses [`LAB_CACHE_STATE_EMPTY`].
pub const LAB_CACHE_STATE_SUMMARY_PREFIX: &str = "Yahoo cache: ";

/// Build the aggregate cache-state summary label.
///
/// - `populated_count == 0` → returns [`LAB_CACHE_STATE_EMPTY`] verbatim.
/// - `populated_count >= 1` AND `iso_date` is Some → returns
///   `"Yahoo cache: N tickers · last fetch YYYY-MM-DD"`.
/// - `populated_count >= 1` AND `iso_date` is None → returns
///   `"Yahoo cache: N tickers"` (defensive edge case; the production
///   `probe_summary` never returns count >= 1 without an mtime).
///
/// Lives in `strings` (not the widget) so the prose template "tickers ·
/// last fetch" stays out of `widgets/*` per the consistency hygiene
/// contract (R-NR.2 / `tests/consistency.rs::no_inline_user_visible_strings_in_widgets`).
#[must_use]
pub fn fmt_lab_cache_state_summary(populated_count: usize, iso_date: Option<&str>) -> String {
    if populated_count == 0 {
        return LAB_CACHE_STATE_EMPTY.to_string();
    }
    match iso_date {
        Some(date) => format!(
            "{LAB_CACHE_STATE_SUMMARY_PREFIX}{populated_count} tickers \u{00b7} last fetch {date}"
        ),
        None => format!("{LAB_CACHE_STATE_SUMMARY_PREFIX}{populated_count} tickers"),
    }
}

// (LAB_REGIME_VOLATILE / LAB_REGIME_CALM removed 2026-05-29 alongside the
// Wave D regime helpers — see
// `spec/dev-notes/post-v3-trail-ui-cleanup-2026-05-29.md`.)

// ── Fallbacks / placeholders ─────────────────────────────────────────────────

/// Rendered when a value is known to be "no data yet" rather than zero.
pub const PLACEHOLDER_NONE: &str = "—";

/// Stable ordered list of every string key declared above. Used by tests
/// to verify no string in the UI escapes this module and by tooling that
/// might later extract these for localization.
///
/// The function body is one big literal slice — clippy's
/// `too_many_lines` lint disagrees, we disagree back. Splitting this into
/// per-section helpers would obscure the single-source-of-truth shape
/// and force tests to call multiple accessors.
#[allow(clippy::too_many_lines, clippy::large_stack_arrays, deprecated)]
#[must_use]
pub fn all() -> &'static [(&'static str, &'static str)] {
    &[
        ("APP_TITLE", APP_TITLE),
        ("PANEL_AGENT_FEED_TITLE", PANEL_AGENT_FEED_TITLE),
        ("PANEL_POSITIONS_TITLE", PANEL_POSITIONS_TITLE),
        ("PANEL_PNL_TITLE", PANEL_PNL_TITLE),
        ("PANEL_KILL_TITLE", PANEL_KILL_TITLE),
        ("PANEL_LATENCY_TITLE", PANEL_LATENCY_TITLE),
        ("PANEL_STRATEGIES_TITLE", PANEL_STRATEGIES_TITLE),
        ("TAPE_COL_TIME", TAPE_COL_TIME),
        ("TAPE_COL_SYMBOL", TAPE_COL_SYMBOL),
        ("TAPE_COL_SIDE", TAPE_COL_SIDE),
        ("TAPE_COL_PRICE", TAPE_COL_PRICE),
        ("TAPE_COL_QTY", TAPE_COL_QTY),
        ("TAPE_COL_NOTIONAL", TAPE_COL_NOTIONAL),
        ("TAPE_COL_FEE", TAPE_COL_FEE),
        ("TAPE_PAUSE_LABEL", TAPE_PAUSE_LABEL),
        ("TAPE_RESUME_LABEL", TAPE_RESUME_LABEL),
        ("TAPE_LOADING", TAPE_LOADING),
        ("TAPE_EMPTY", TAPE_EMPTY),
        ("TAPE_ERROR_PREFIX", TAPE_ERROR_PREFIX),
        ("TAPE_PAUSED_BANNER", TAPE_PAUSED_BANNER),
        ("MONTH_JAN", MONTH_ABBREVS[0]),
        ("MONTH_FEB", MONTH_ABBREVS[1]),
        ("MONTH_MAR", MONTH_ABBREVS[2]),
        ("MONTH_APR", MONTH_ABBREVS[3]),
        ("MONTH_MAY", MONTH_ABBREVS[4]),
        ("MONTH_JUN", MONTH_ABBREVS[5]),
        ("MONTH_JUL", MONTH_ABBREVS[6]),
        ("MONTH_AUG", MONTH_ABBREVS[7]),
        ("MONTH_SEP", MONTH_ABBREVS[8]),
        ("MONTH_OCT", MONTH_ABBREVS[9]),
        ("MONTH_NOV", MONTH_ABBREVS[10]),
        ("MONTH_DEC", MONTH_ABBREVS[11]),
        ("POS_COL_SYMBOL", POS_COL_SYMBOL),
        ("POS_COL_QTY", POS_COL_QTY),
        ("POS_COL_COST", POS_COL_COST),
        ("POS_COL_MARK", POS_COL_MARK),
        ("POS_COL_PNL", POS_COL_PNL),
        ("POS_COL_PNL_PCT", POS_COL_PNL_PCT),
        ("POS_COL_EXPOSURE", POS_COL_EXPOSURE),
        ("POS_LOADING", POS_LOADING),
        ("POS_EMPTY", POS_EMPTY),
        ("POS_ERROR_PREFIX", POS_ERROR_PREFIX),
        ("POS_COL_DIRECTION", POS_COL_DIRECTION),
        ("POS_DIRECTION_LONG", POS_DIRECTION_LONG),
        ("POS_DIRECTION_SHORT", POS_DIRECTION_SHORT),
        (
            "SHORT_UNBOUNDED_LOSS_DISCLAIMER",
            SHORT_UNBOUNDED_LOSS_DISCLAIMER,
        ),
        ("PNL_LABEL_CASH", PNL_LABEL_CASH),
        ("PNL_LABEL_UNREALIZED", PNL_LABEL_UNREALIZED),
        ("PNL_LABEL_REALIZED", PNL_LABEL_REALIZED),
        ("PNL_LABEL_EQUITY", PNL_LABEL_EQUITY),
        ("PNL_LABEL_DAILY_RETURN", PNL_LABEL_DAILY_RETURN),
        ("PNL_LOADING", PNL_LOADING),
        ("PNL_EMPTY", PNL_EMPTY),
        ("PNL_ERROR_PREFIX", PNL_ERROR_PREFIX),
        ("KILL_BUTTON_LABEL", KILL_BUTTON_LABEL),
        ("KILL_BUTTON_HELP", KILL_BUTTON_HELP),
        ("KILL_DIALOG_TITLE", KILL_DIALOG_TITLE),
        ("KILL_DIALOG_BODY", KILL_DIALOG_BODY),
        ("KILL_PHRASE_LABEL", KILL_PHRASE_LABEL),
        ("KILL_SAFETY_PHRASE", KILL_SAFETY_PHRASE),
        ("KILL_CONFIRM_LABEL", KILL_CONFIRM_LABEL),
        ("KILL_CANCEL_LABEL", KILL_CANCEL_LABEL),
        ("KILL_PHRASE_MISMATCH_HINT", KILL_PHRASE_MISMATCH_HINT),
        ("KILL_HALTED_BANNER", KILL_HALTED_BANNER),
        ("KILL_HALTED_HINT", KILL_HALTED_HINT),
        ("KILL_RUNBOOK_LINK_LABEL", KILL_RUNBOOK_LINK_LABEL),
        ("KILL_RUNBOOK_LINK_PATH", KILL_RUNBOOK_LINK_PATH),
        ("TAPE_AUDIT_MODAL_TITLE", TAPE_AUDIT_MODAL_TITLE),
        ("TAPE_AUDIT_MODAL_TX_LABEL", TAPE_AUDIT_MODAL_TX_LABEL),
        ("TAPE_AUDIT_MODAL_TS_LABEL", TAPE_AUDIT_MODAL_TS_LABEL),
        ("TAPE_AUDIT_MODAL_DESC_LABEL", TAPE_AUDIT_MODAL_DESC_LABEL),
        (
            "TAPE_AUDIT_MODAL_STRATEGY_LABEL",
            TAPE_AUDIT_MODAL_STRATEGY_LABEL,
        ),
        (
            "TAPE_AUDIT_MODAL_STRATEGY_NONE",
            TAPE_AUDIT_MODAL_STRATEGY_NONE,
        ),
        ("TAPE_AUDIT_MODAL_COL_ACCOUNT", TAPE_AUDIT_MODAL_COL_ACCOUNT),
        ("TAPE_AUDIT_MODAL_COL_DEBIT", TAPE_AUDIT_MODAL_COL_DEBIT),
        ("TAPE_AUDIT_MODAL_COL_CREDIT", TAPE_AUDIT_MODAL_COL_CREDIT),
        (
            "TAPE_AUDIT_MODAL_COL_CURRENCY",
            TAPE_AUDIT_MODAL_COL_CURRENCY,
        ),
        ("TAPE_AUDIT_MODAL_LOADING", TAPE_AUDIT_MODAL_LOADING),
        ("TAPE_AUDIT_MODAL_EMPTY", TAPE_AUDIT_MODAL_EMPTY),
        (
            "TAPE_AUDIT_MODAL_ERROR_PREFIX",
            TAPE_AUDIT_MODAL_ERROR_PREFIX,
        ),
        ("TAPE_AUDIT_MODAL_CLOSE_LABEL", TAPE_AUDIT_MODAL_CLOSE_LABEL),
        ("CONNECTION_AGENT_UNREACHABLE", CONNECTION_AGENT_UNREACHABLE),
        ("CONNECTION_LAGGED", CONNECTION_LAGGED),
        ("CONNECTION_CHANNEL_CLOSED", CONNECTION_CHANNEL_CLOSED),
        ("STRATEGIES_LOADING", STRATEGIES_LOADING),
        ("STRATEGIES_EMPTY", STRATEGIES_EMPTY),
        ("STRATEGIES_ERROR_PREFIX", STRATEGIES_ERROR_PREFIX),
        ("STRATEGIES_COL_ID", STRATEGIES_COL_ID),
        ("STRATEGIES_COL_HASH", STRATEGIES_COL_HASH),
        ("STRATEGIES_COL_STATUS", STRATEGIES_COL_STATUS),
        ("STRATEGIES_COL_LAST_EVENT", STRATEGIES_COL_LAST_EVENT),
        ("STRATEGIES_COL_SIGNALS_60S", STRATEGIES_COL_SIGNALS_60S),
        ("STRATEGIES_COL_POSITION", STRATEGIES_COL_POSITION),
        ("STRATEGIES_STATUS_READY", STRATEGIES_STATUS_READY),
        ("STRATEGIES_STATUS_LOADING", STRATEGIES_STATUS_LOADING),
        ("STRATEGIES_STATUS_ERROR", STRATEGIES_STATUS_ERROR),
        ("STRATEGIES_EVENT_LOAD", STRATEGIES_EVENT_LOAD),
        ("STRATEGIES_EVENT_SWAP", STRATEGIES_EVENT_SWAP),
        ("STRATEGIES_EVENT_UNLOAD", STRATEGIES_EVENT_UNLOAD),
        ("STRATEGIES_EVENT_REJECT", STRATEGIES_EVENT_REJECT),
        ("STRATEGIES_POSITION_HELD", STRATEGIES_POSITION_HELD),
        ("STRATEGIES_POSITION_FLAT", STRATEGIES_POSITION_FLAT),
        ("LATENCY_OK_LABEL", LATENCY_OK_LABEL),
        ("LATENCY_WARN_LABEL", LATENCY_WARN_LABEL),
        ("LATENCY_HIGH_LABEL", LATENCY_HIGH_LABEL),
        ("LATENCY_HALTED_LABEL", LATENCY_HALTED_LABEL),
        ("LATENCY_UNIT_MS", LATENCY_UNIT_MS),
        ("LATENCY_UNKNOWN", LATENCY_UNKNOWN),
        ("LATENCY_HELP", LATENCY_HELP),
        ("MODE_RESEARCH", MODE_RESEARCH),
        ("MODE_PAPER", MODE_PAPER),
        ("MODE_LIVE", MODE_LIVE),
        ("MODE_HALTED", MODE_HALTED),
        ("SIDE_BUY", SIDE_BUY),
        ("SIDE_SELL", SIDE_SELL),
        ("UNIT_USDT", UNIT_USDT),
        ("UNIT_BTC", UNIT_BTC),
        ("CURRENCY_EUR_SYMBOL", CURRENCY_EUR_SYMBOL),
        ("PLACEHOLDER_NONE", PLACEHOLDER_NONE),
        ("STATUS_BAR_CONNECTED", STATUS_BAR_CONNECTED),
        ("STATUS_BAR_RECONNECTING", STATUS_BAR_RECONNECTING),
        ("STATUS_BAR_DISCONNECTED", STATUS_BAR_DISCONNECTED),
        ("STATUS_BAR_LATENCY_LABEL", STATUS_BAR_LATENCY_LABEL),
        ("STATUS_BAR_SERVER_LABEL", STATUS_BAR_SERVER_LABEL),
        ("STATUS_BAR_CPU_LABEL", STATUS_BAR_CPU_LABEL),
        ("STATUS_BAR_CPU_PLACEHOLDER", STATUS_BAR_CPU_PLACEHOLDER),
        ("STATUS_BAR_NO_LATENCY", STATUS_BAR_NO_LATENCY),
        ("STATUS_BAR_UTC_SUFFIX", STATUS_BAR_UTC_SUFFIX),
        ("STATUS_BAR_NO_SERVER_TIME", STATUS_BAR_NO_SERVER_TIME),
        ("STATUS_BAR_MS", STATUS_BAR_MS),
        ("STATUS_BAR_VERSION_PREFIX", STATUS_BAR_VERSION_PREFIX),
        ("STATUS_BAR_VERSION_SUFFIX", STATUS_BAR_VERSION_SUFFIX),
        ("STATUS_BAR_VERSION", STATUS_BAR_VERSION),
        ("SIDEBAR_NAV_HOME", SIDEBAR_NAV_HOME),
        ("SIDEBAR_NAV_DEBUG", SIDEBAR_NAV_DEBUG),
        ("SIDEBAR_NAV_CHARTS", SIDEBAR_NAV_CHARTS),
        ("SIDEBAR_NAV_STRATEGIES", SIDEBAR_NAV_STRATEGIES),
        ("SIDEBAR_NAV_RISK", SIDEBAR_NAV_RISK),
        ("SIDEBAR_NAV_AUDIT", SIDEBAR_NAV_AUDIT),
        ("CHART_NO_DATA", CHART_NO_DATA),
        ("CHART_TOOLTIP_SIDE_BUY", CHART_TOOLTIP_SIDE_BUY),
        ("CHART_TOOLTIP_SIDE_SELL", CHART_TOOLTIP_SIDE_SELL),
        ("CHART_TOOLTIP_PRICE_LABEL", CHART_TOOLTIP_PRICE_LABEL),
        ("CHART_TOOLTIP_QTY_LABEL", CHART_TOOLTIP_QTY_LABEL),
        ("CHART_TOOLTIP_NOTIONAL_LABEL", CHART_TOOLTIP_NOTIONAL_LABEL),
        ("CHART_TOOLTIP_TS_LABEL", CHART_TOOLTIP_TS_LABEL),
        ("CHART_TOOLTIP_STRATEGY_LABEL", CHART_TOOLTIP_STRATEGY_LABEL),
        ("CHART_TOOLTIP_STRATEGY_NONE", CHART_TOOLTIP_STRATEGY_NONE),
        ("CHART_TOOLTIP_GHOST_BADGE", CHART_TOOLTIP_GHOST_BADGE),
        ("CHART_TOOLTIP_CLAMP_SUFFIX", CHART_TOOLTIP_CLAMP_SUFFIX),
        ("CHART_VOLUME_TILE_BUYS_LABEL", CHART_VOLUME_TILE_BUYS_LABEL),
        (
            "CHART_VOLUME_TILE_SELLS_LABEL",
            CHART_VOLUME_TILE_SELLS_LABEL,
        ),
        ("CHART_VOLUME_TILE_NET_LABEL", CHART_VOLUME_TILE_NET_LABEL),
        (
            "CHART_VOLUME_TILE_TRADES_SUFFIX",
            CHART_VOLUME_TILE_TRADES_SUFFIX,
        ),
        ("CHART_VOLUME_HISTOGRAM_LABEL", CHART_VOLUME_HISTOGRAM_LABEL),
        ("CHART_POSITION_MIRROR_LABEL", CHART_POSITION_MIRROR_LABEL),
        ("CHART_POSITION_MIRROR_NONE", CHART_POSITION_MIRROR_NONE),
        ("LAB_POSITION_CURVE_LABEL", LAB_POSITION_CURVE_LABEL),
        ("TRAIL_DRAWER_CLOSE_LABEL", TRAIL_DRAWER_CLOSE_LABEL),
        ("TRAIL_DRAWER_FILL_TITLE", TRAIL_DRAWER_FILL_TITLE),
        ("TRAIL_DRAWER_SIGNAL_TITLE", TRAIL_DRAWER_SIGNAL_TITLE),
        ("TRAIL_DRAWER_FORECAST_TITLE", TRAIL_DRAWER_FORECAST_TITLE),
        ("TRAIL_DRAWER_LLM_TITLE", TRAIL_DRAWER_LLM_TITLE),
        ("TRAIL_DRAWER_LLM_PLACEHOLDER", TRAIL_DRAWER_LLM_PLACEHOLDER),
        ("TRAIL_SIGNAL_SIDE_LABEL", TRAIL_SIGNAL_SIDE_LABEL),
        ("TRAIL_SIGNAL_QTY_LABEL", TRAIL_SIGNAL_QTY_LABEL),
        ("TRAIL_SIGNAL_PRICE_LABEL", TRAIL_SIGNAL_PRICE_LABEL),
        ("TRAIL_SIGNAL_PRICE_MARKET", TRAIL_SIGNAL_PRICE_MARKET),
        ("TRAIL_SIGNAL_CLAMPED_LABEL", TRAIL_SIGNAL_CLAMPED_LABEL),
        (
            "TRAIL_SIGNAL_CLAMP_REASON_LABEL",
            TRAIL_SIGNAL_CLAMP_REASON_LABEL,
        ),
        (
            "TRAIL_FORECAST_DIRECTION_LABEL",
            TRAIL_FORECAST_DIRECTION_LABEL,
        ),
        (
            "TRAIL_FORECAST_CONFIDENCE_LABEL",
            TRAIL_FORECAST_CONFIDENCE_LABEL,
        ),
        ("TRAIL_FORECAST_MODEL_LABEL", TRAIL_FORECAST_MODEL_LABEL),
        (
            "TRAIL_FORECAST_CACHE_HIT_LABEL",
            TRAIL_FORECAST_CACHE_HIT_LABEL,
        ),
        ("TRAIL_NODE_FILL_LABEL", TRAIL_NODE_FILL_LABEL),
        ("TRAIL_NODE_SIGNAL_LABEL", TRAIL_NODE_SIGNAL_LABEL),
        ("TRAIL_NODE_FORECAST_LABEL", TRAIL_NODE_FORECAST_LABEL),
        ("TRAIL_NODE_LLM_LABEL", TRAIL_NODE_LLM_LABEL),
        ("TRAIL_NODE_NO_UPSTREAM_FILL", TRAIL_NODE_NO_UPSTREAM_FILL),
        (
            "TRAIL_NODE_NO_UPSTREAM_SIGNAL",
            TRAIL_NODE_NO_UPSTREAM_SIGNAL,
        ),
        (
            "TRAIL_NODE_NO_UPSTREAM_FORECAST",
            TRAIL_NODE_NO_UPSTREAM_FORECAST,
        ),
        ("TRAIL_NODE_NO_LLM_TRANSCRIPT", TRAIL_NODE_NO_LLM_TRANSCRIPT),
        ("TRAIL_BOOL_YES", TRAIL_BOOL_YES),
        ("TRAIL_BOOL_NO", TRAIL_BOOL_NO),
        ("MATRIX_EMPTY_STATE", MATRIX_EMPTY_STATE),
        // cockpit-baseline-panel v0.1.0
        ("BASELINE_SIDEBAR_LABEL", BASELINE_SIDEBAR_LABEL),
        ("BASELINE_HEADLINE", BASELINE_HEADLINE),
        ("BASELINE_CAPTION", BASELINE_CAPTION),
        ("BASELINE_YEAR_2023_LABEL", BASELINE_YEAR_2023_LABEL),
        ("BASELINE_YEAR_2024_LABEL", BASELINE_YEAR_2024_LABEL),
        ("BASELINE_DATA_UNAVAILABLE", BASELINE_DATA_UNAVAILABLE),
        ("BASELINE_RISK_DETAIL", BASELINE_RISK_DETAIL),
        // cockpit-reports-viewer v0.1.0
        ("REPORTS_SIDEBAR_LABEL", REPORTS_SIDEBAR_LABEL),
        ("REPORTS_PICKER_TITLE", REPORTS_PICKER_TITLE),
        ("REPORTS_EMPTY_LIST", REPORTS_EMPTY_LIST),
        ("REPORTS_SELECT_PROMPT", REPORTS_SELECT_PROMPT),
        ("REPORTS_LOAD_ERROR", REPORTS_LOAD_ERROR),
        ("REPORTS_HAS_CURVE_MARKER", REPORTS_HAS_CURVE_MARKER),
        ("REPORTS_FILTER_CURVE_ONLY", REPORTS_FILTER_CURVE_ONLY),
        ("REPORTS_FILTER_ALL", REPORTS_FILTER_ALL),
        ("REPORTS_FILTER_NO_CURVE_HINT", REPORTS_FILTER_NO_CURVE_HINT),
        // advisor-leaderboard-screen v0.1.0
        ("LEADERBOARD_SIDEBAR_LABEL", LEADERBOARD_SIDEBAR_LABEL),
        ("LEADERBOARD_HEADLINE", LEADERBOARD_HEADLINE),
        ("LEADERBOARD_CAPTION", LEADERBOARD_CAPTION),
        ("LEADERBOARD_RUN_BUTTON", LEADERBOARD_RUN_BUTTON),
        (
            "LEADERBOARD_RUN_BUTTON_RUNNING",
            LEADERBOARD_RUN_BUTTON_RUNNING,
        ),
        ("LEADERBOARD_EMPTY_PROMPT", LEADERBOARD_EMPTY_PROMPT),
        ("LEADERBOARD_LOADING", LEADERBOARD_LOADING),
        ("LEADERBOARD_PROGRESS_FMT", LEADERBOARD_PROGRESS_FMT),
        ("LEADERBOARD_ERROR_PREFIX", LEADERBOARD_ERROR_PREFIX),
        ("LEADERBOARD_RUN_NEEDS_LIVE", LEADERBOARD_RUN_NEEDS_LIVE),
        // advisor-dynamic-data fetch-error copy (ADR-0061 Wave C)
        (
            "LEADERBOARD_FETCH_NETWORK_ERROR",
            LEADERBOARD_FETCH_NETWORK_ERROR,
        ),
        (
            "LEADERBOARD_FETCH_RATE_LIMITED",
            LEADERBOARD_FETCH_RATE_LIMITED,
        ),
        (
            "LEADERBOARD_FETCH_UNKNOWN_SYMBOL",
            LEADERBOARD_FETCH_UNKNOWN_SYMBOL,
        ),
        ("LEADERBOARD_FETCH_NO_DATA", LEADERBOARD_FETCH_NO_DATA),
        ("LEADERBOARD_COL_RANK", LEADERBOARD_COL_RANK),
        ("LEADERBOARD_COL_STRATEGY", LEADERBOARD_COL_STRATEGY),
        ("LEADERBOARD_COL_RETURN", LEADERBOARD_COL_RETURN),
        ("LEADERBOARD_COL_SHARPE", LEADERBOARD_COL_SHARPE),
        ("LEADERBOARD_COL_MAX_DD", LEADERBOARD_COL_MAX_DD),
        ("LEADERBOARD_COL_TRADES", LEADERBOARD_COL_TRADES),
        ("LEADERBOARD_BENCHMARK_TAG", LEADERBOARD_BENCHMARK_TAG),
        (
            "LEADERBOARD_BENCHMARK_FRAGILE_NOTE",
            LEADERBOARD_BENCHMARK_FRAGILE_NOTE,
        ),
        ("LEADERBOARD_CROWN_TAG", LEADERBOARD_CROWN_TAG),
        ("LEADERBOARD_FRAGILE_TAG", LEADERBOARD_FRAGILE_TAG),
        ("LEADERBOARD_ROBUST_TAG", LEADERBOARD_ROBUST_TAG),
        ("LEADERBOARD_MARGINAL_TAG", LEADERBOARD_MARGINAL_TAG),
        // advisor-ensemble F8 (ADR-0063) — ensemble row labelling
        (
            "LEADERBOARD_ENSEMBLE_MAJORITY_LABEL",
            LEADERBOARD_ENSEMBLE_MAJORITY_LABEL,
        ),
        (
            "LEADERBOARD_ENSEMBLE_UNANIMOUS_LABEL",
            LEADERBOARD_ENSEMBLE_UNANIMOUS_LABEL,
        ),
        // advisor-combination-search (ADR-0067) — the 6 combination-arm labels
        (
            "LEADERBOARD_ENSEMBLE_TREND_PAIR_LABEL",
            LEADERBOARD_ENSEMBLE_TREND_PAIR_LABEL,
        ),
        (
            "LEADERBOARD_ENSEMBLE_TR_MR_MACD_RSI_LABEL",
            LEADERBOARD_ENSEMBLE_TR_MR_MACD_RSI_LABEL,
        ),
        (
            "LEADERBOARD_ENSEMBLE_TR_MR_SMA_BB_LABEL",
            LEADERBOARD_ENSEMBLE_TR_MR_SMA_BB_LABEL,
        ),
        (
            "LEADERBOARD_ENSEMBLE_ANY1OF4_LABEL",
            LEADERBOARD_ENSEMBLE_ANY1OF4_LABEL,
        ),
        (
            "LEADERBOARD_ENSEMBLE_K2OF4_LABEL",
            LEADERBOARD_ENSEMBLE_K2OF4_LABEL,
        ),
        (
            "LEADERBOARD_ENSEMBLE_K3OF4_LABEL",
            LEADERBOARD_ENSEMBLE_K3OF4_LABEL,
        ),
        (
            "LEADERBOARD_ENSEMBLE_VOTE_TAG",
            LEADERBOARD_ENSEMBLE_VOTE_TAG,
        ),
        (
            "LEADERBOARD_SHORT_SMA_CROSS_LS_LABEL",
            LEADERBOARD_SHORT_SMA_CROSS_LS_LABEL,
        ),
        (
            "LEADERBOARD_SHORT_MACD_LS_LABEL",
            LEADERBOARD_SHORT_MACD_LS_LABEL,
        ),
        (
            "LEADERBOARD_SHORT_RSI_LS_LABEL",
            LEADERBOARD_SHORT_RSI_LS_LABEL,
        ),
        (
            "LEADERBOARD_SHORT_BBANDS_LS_LABEL",
            LEADERBOARD_SHORT_BBANDS_LS_LABEL,
        ),
        (
            "LEADERBOARD_SHORT_ALWAYS_SHORT_LABEL",
            LEADERBOARD_SHORT_ALWAYS_SHORT_LABEL,
        ),
        ("LEADERBOARD_SHORT_TAG", LEADERBOARD_SHORT_TAG),
        ("LEADERBOARD_SHORT_FIELD_NOTE", LEADERBOARD_SHORT_FIELD_NOTE),
        (
            "LEADERBOARD_ENSEMBLE_SAT_IN_CASH",
            LEADERBOARD_ENSEMBLE_SAT_IN_CASH,
        ),
        (
            "LEADERBOARD_HEADLINE_BENCHMARK_WINS",
            LEADERBOARD_HEADLINE_BENCHMARK_WINS,
        ),
        (
            "LEADERBOARD_HEADLINE_ACTIVE_WINS",
            LEADERBOARD_HEADLINE_ACTIVE_WINS,
        ),
        (
            "LEADERBOARD_HEADLINE_ALL_FRAGILE",
            LEADERBOARD_HEADLINE_ALL_FRAGILE,
        ),
        (
            "LEADERBOARD_REASON_HIGHEST_ROBUST_SHARPE",
            LEADERBOARD_REASON_HIGHEST_ROBUST_SHARPE,
        ),
        (
            "LEADERBOARD_REASON_BEAT_BENCHMARK_SHARPE",
            LEADERBOARD_REASON_BEAT_BENCHMARK_SHARPE,
        ),
        (
            "LEADERBOARD_REASON_BENCHMARK_UNDEFEATED",
            LEADERBOARD_REASON_BENCHMARK_UNDEFEATED,
        ),
        (
            "LEADERBOARD_REASON_ALL_FRAGILE",
            LEADERBOARD_REASON_ALL_FRAGILE,
        ),
        (
            "LEADERBOARD_REASON_TIE_RETURN",
            LEADERBOARD_REASON_TIE_RETURN,
        ),
        (
            "LEADERBOARD_REASON_TIE_DRAWDOWN",
            LEADERBOARD_REASON_TIE_DRAWDOWN,
        ),
        ("LEADERBOARD_DISCLAIMER", LEADERBOARD_DISCLAIMER),
        (
            "LEADERBOARD_RECOMMENDATION_TITLE",
            LEADERBOARD_RECOMMENDATION_TITLE,
        ),
        (
            "LEADERBOARD_WINNER_ROBUST_CLAUSE",
            LEADERBOARD_WINNER_ROBUST_CLAUSE,
        ),
        (
            "LEADERBOARD_WINNER_FRAGILE_CLAUSE",
            LEADERBOARD_WINNER_FRAGILE_CLAUSE,
        ),
        // advisor-llm-narration F9 — the opt-in "why this one" narration
        ("LEADERBOARD_EXPLAIN_BUTTON", LEADERBOARD_EXPLAIN_BUTTON),
        ("LEADERBOARD_EXPLAIN_INFLIGHT", LEADERBOARD_EXPLAIN_INFLIGHT),
        (
            "LEADERBOARD_EXPLAIN_LLM_LABEL",
            LEADERBOARD_EXPLAIN_LLM_LABEL,
        ),
        ("LEADERBOARD_EXPLAIN_FELLBACK", LEADERBOARD_EXPLAIN_FELLBACK),
        // advisor-bakeoff-ranking F3 — guided input
        ("LEADERBOARD_PLAN_TITLE", LEADERBOARD_PLAN_TITLE),
        ("LEADERBOARD_COIN_LABEL", LEADERBOARD_COIN_LABEL),
        ("LEADERBOARD_BUDGET_LABEL", LEADERBOARD_BUDGET_LABEL),
        ("LEADERBOARD_LOOKBACK_LABEL", LEADERBOARD_LOOKBACK_LABEL),
        (
            "LEADERBOARD_BUDGET_PLACEHOLDER",
            LEADERBOARD_BUDGET_PLACEHOLDER,
        ),
        ("LEADERBOARD_BUDGET_HINT_FMT", LEADERBOARD_BUDGET_HINT_FMT),
        (
            "LEADERBOARD_BUDGET_CONTEXT_FMT",
            LEADERBOARD_BUDGET_CONTEXT_FMT,
        ),
        (
            "LEADERBOARD_CONTEXT_NO_BUDGET_FMT",
            LEADERBOARD_CONTEXT_NO_BUDGET_FMT,
        ),
        (
            "LEADERBOARD_FIELD_ARM_COUNT_FMT",
            LEADERBOARD_FIELD_ARM_COUNT_FMT,
        ),
        ("LEADERBOARD_LOOKBACK_2W", LEADERBOARD_LOOKBACK_2W),
        ("LEADERBOARD_LOOKBACK_1M", LEADERBOARD_LOOKBACK_1M),
        ("LEADERBOARD_LOOKBACK_3M", LEADERBOARD_LOOKBACK_3M),
        ("LEADERBOARD_LOOKBACK_6M", LEADERBOARD_LOOKBACK_6M),
        ("LEADERBOARD_LOOKBACK_1Y", LEADERBOARD_LOOKBACK_1Y),
        ("LEADERBOARD_LOOKBACK_2Y", LEADERBOARD_LOOKBACK_2Y),
        ("LEADERBOARD_LOOKBACK_4Y", LEADERBOARD_LOOKBACK_4Y),
        ("LEADERBOARD_LOOKBACK_H1_2024", LEADERBOARD_LOOKBACK_H1_2024),
        ("LEADERBOARD_LOOKBACK_H2_2024", LEADERBOARD_LOOKBACK_H2_2024),
        // advisor-leaderboard-tuning — timeframe + start-capital knobs
        ("LEADERBOARD_TIMEFRAME_LABEL", LEADERBOARD_TIMEFRAME_LABEL),
        ("LEADERBOARD_CAPITAL_LABEL", LEADERBOARD_CAPITAL_LABEL),
        (
            "LEADERBOARD_CAPITAL_PLACEHOLDER",
            LEADERBOARD_CAPITAL_PLACEHOLDER,
        ),
        ("LEADERBOARD_CAPITAL_HINT", LEADERBOARD_CAPITAL_HINT),
        // advisor-param-tuning (ADR-0069) — the gate-tied sweep editor
        ("TUNE_SIDEBAR_LABEL", TUNE_SIDEBAR_LABEL),
        ("TUNE_HEADLINE", TUNE_HEADLINE),
        ("TUNE_CAPTION", TUNE_CAPTION),
        ("TUNE_RUN_BUTTON", TUNE_RUN_BUTTON),
        ("TUNE_RUN_BUTTON_RUNNING", TUNE_RUN_BUTTON_RUNNING),
        ("TUNE_RUN_NEEDS_LIVE", TUNE_RUN_NEEDS_LIVE),
        ("TUNE_OPEN_AFFORDANCE", TUNE_OPEN_AFFORDANCE),
        ("TUNE_EMPTY_PROMPT", TUNE_EMPTY_PROMPT),
        ("TUNE_LOADING", TUNE_LOADING),
        ("TUNE_ERROR_PREFIX", TUNE_ERROR_PREFIX),
        ("TUNE_FORM_TITLE", TUNE_FORM_TITLE),
        ("TUNE_FAMILY_LABEL", TUNE_FAMILY_LABEL),
        ("TUNE_FAMILY_SMA", TUNE_FAMILY_SMA),
        ("TUNE_FAMILY_MACD", TUNE_FAMILY_MACD),
        ("TUNE_FAMILY_RSI", TUNE_FAMILY_RSI),
        ("TUNE_FAMILY_BOLLINGER", TUNE_FAMILY_BOLLINGER),
        ("TUNE_AXIS_FAST_LABEL", TUNE_AXIS_FAST_LABEL),
        ("TUNE_AXIS_SLOW_LABEL", TUNE_AXIS_SLOW_LABEL),
        ("TUNE_AXIS_MACD_FAST_LABEL", TUNE_AXIS_MACD_FAST_LABEL),
        ("TUNE_AXIS_MACD_SLOW_LABEL", TUNE_AXIS_MACD_SLOW_LABEL),
        ("TUNE_AXIS_MACD_SIGNAL_LABEL", TUNE_AXIS_MACD_SIGNAL_LABEL),
        ("TUNE_AXIS_RSI_PERIOD_LABEL", TUNE_AXIS_RSI_PERIOD_LABEL),
        ("TUNE_AXIS_RSI_OVERSOLD_LABEL", TUNE_AXIS_RSI_OVERSOLD_LABEL),
        (
            "TUNE_AXIS_BBANDS_PERIOD_LABEL",
            TUNE_AXIS_BBANDS_PERIOD_LABEL,
        ),
        ("TUNE_AXIS_BBANDS_K_LABEL", TUNE_AXIS_BBANDS_K_LABEL),
        ("TUNE_AXIS_MIN", TUNE_AXIS_MIN),
        ("TUNE_AXIS_MAX", TUNE_AXIS_MAX),
        ("TUNE_AXIS_STEP", TUNE_AXIS_STEP),
        ("TUNE_PRESET_NARROW", TUNE_PRESET_NARROW),
        ("TUNE_PRESET_SHIPPED", TUNE_PRESET_SHIPPED),
        ("TUNE_PRESET_WIDE", TUNE_PRESET_WIDE),
        ("TUNE_GRID_READOUT_FMT", TUNE_GRID_READOUT_FMT),
        ("TUNE_GRID_READOUT_EMPTY", TUNE_GRID_READOUT_EMPTY),
        ("TUNE_GRID_READOUT_BLANK", TUNE_GRID_READOUT_BLANK),
        ("TUNE_TRUNCATION_FMT", TUNE_TRUNCATION_FMT),
        ("TUNE_PROGRESS_FMT", TUNE_PROGRESS_FMT),
        ("TUNE_COL_CONFIG", TUNE_COL_CONFIG),
        ("TUNE_COL_VERDICT", TUNE_COL_VERDICT),
        ("TUNE_COL_RETURN", TUNE_COL_RETURN),
        ("TUNE_COL_SHARPE_SPREAD", TUNE_COL_SHARPE_SPREAD),
        ("TUNE_COL_PROB_LOSS", TUNE_COL_PROB_LOSS),
        ("TUNE_COL_PROB_SHARPE", TUNE_COL_PROB_SHARPE),
        ("TUNE_COL_MAXDD_P95", TUNE_COL_MAXDD_P95),
        ("TUNE_COL_USE", TUNE_COL_USE),
        ("TUNE_VERDICT_ROBUST", TUNE_VERDICT_ROBUST),
        ("TUNE_VERDICT_MARGINAL", TUNE_VERDICT_MARGINAL),
        ("TUNE_VERDICT_FRAGILE", TUNE_VERDICT_FRAGILE),
        ("TUNE_BASELINE_TAG", TUNE_BASELINE_TAG),
        ("TUNE_USE_CONFIG", TUNE_USE_CONFIG),
        ("TUNE_USE_CONFIG_FRAGILE", TUNE_USE_CONFIG_FRAGILE),
        ("TUNE_FRAGILE_PROMOTE_NOTE", TUNE_FRAGILE_PROMOTE_NOTE),
        ("TUNE_DISTRIBUTION_CAPTION", TUNE_DISTRIBUTION_CAPTION),
        ("TUNE_BENCHMARK_STRIP_FMT", TUNE_BENCHMARK_STRIP_FMT),
        ("TUNE_DISCLAIMER", TUNE_DISCLAIMER),
        // advisor-param-promotion (ADR-0070 § D6)
        ("TUNE_PROMOTE_CONFIRM_FMT", TUNE_PROMOTE_CONFIRM_FMT),
        ("TUNE_PROMOTE_WINDOW_FALLBACK", TUNE_PROMOTE_WINDOW_FALLBACK),
        // advisor-forward-plan v0.1.0 (roadmap F6)
        ("FORWARD_PLAN_SIDEBAR_LABEL", FORWARD_PLAN_SIDEBAR_LABEL),
        ("FORWARD_PLAN_HEADLINE", FORWARD_PLAN_HEADLINE),
        ("FORWARD_PLAN_CAPTION", FORWARD_PLAN_CAPTION),
        ("FORWARD_PLAN_EMPTY_PROMPT", FORWARD_PLAN_EMPTY_PROMPT),
        ("FORWARD_PLAN_LOADING", FORWARD_PLAN_LOADING),
        ("FORWARD_PLAN_ERROR_PREFIX", FORWARD_PLAN_ERROR_PREFIX),
        ("FORWARD_PLAN_STANCE_TITLE", FORWARD_PLAN_STANCE_TITLE),
        ("FORWARD_PLAN_STANCE_FLAT", FORWARD_PLAN_STANCE_FLAT),
        ("FORWARD_PLAN_STANCE_LONG", FORWARD_PLAN_STANCE_LONG),
        ("FORWARD_PLAN_AS_OF_FMT", FORWARD_PLAN_AS_OF_FMT),
        (
            "FORWARD_PLAN_LATEST_SIGNAL_FMT",
            FORWARD_PLAN_LATEST_SIGNAL_FMT,
        ),
        ("FORWARD_PLAN_SIGNAL_BUY", FORWARD_PLAN_SIGNAL_BUY),
        ("FORWARD_PLAN_SIGNAL_SELL", FORWARD_PLAN_SIGNAL_SELL),
        ("FORWARD_PLAN_SIGNAL_HOLD", FORWARD_PLAN_SIGNAL_HOLD),
        ("FORWARD_PLAN_RULES_TITLE", FORWARD_PLAN_RULES_TITLE),
        ("FORWARD_PLAN_RULE_IF", FORWARD_PLAN_RULE_IF),
        ("FORWARD_PLAN_RULE_THEN", FORWARD_PLAN_RULE_THEN),
        (
            "FORWARD_PLAN_RULE_SMA_ENTRY_IF_FMT",
            FORWARD_PLAN_RULE_SMA_ENTRY_IF_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_SMA_ENTRY_THEN",
            FORWARD_PLAN_RULE_SMA_ENTRY_THEN,
        ),
        (
            "FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT",
            FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_SMA_EXIT_THEN",
            FORWARD_PLAN_RULE_SMA_EXIT_THEN,
        ),
        (
            "FORWARD_PLAN_SHORT_RULES_HEADING",
            FORWARD_PLAN_SHORT_RULES_HEADING,
        ),
        (
            "FORWARD_PLAN_RULE_SHORT_OPEN_THEN",
            FORWARD_PLAN_RULE_SHORT_OPEN_THEN,
        ),
        (
            "FORWARD_PLAN_RULE_SHORT_OPEN_IF_GENERIC",
            FORWARD_PLAN_RULE_SHORT_OPEN_IF_GENERIC,
        ),
        (
            "FORWARD_PLAN_RULE_SHORT_COVER_IF",
            FORWARD_PLAN_RULE_SHORT_COVER_IF,
        ),
        (
            "FORWARD_PLAN_RULE_SHORT_COVER_THEN",
            FORWARD_PLAN_RULE_SHORT_COVER_THEN,
        ),
        (
            "FORWARD_PLAN_RULE_SHORT_LIQUIDATION",
            FORWARD_PLAN_RULE_SHORT_LIQUIDATION,
        ),
        (
            "FORWARD_PLAN_RULE_ALWAYS_SHORT",
            FORWARD_PLAN_RULE_ALWAYS_SHORT,
        ),
        (
            "FORWARD_PLAN_RULE_MACD_ENTRY_IF_FMT",
            FORWARD_PLAN_RULE_MACD_ENTRY_IF_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_MACD_ENTRY_THEN",
            FORWARD_PLAN_RULE_MACD_ENTRY_THEN,
        ),
        (
            "FORWARD_PLAN_RULE_MACD_EXIT_IF",
            FORWARD_PLAN_RULE_MACD_EXIT_IF,
        ),
        (
            "FORWARD_PLAN_RULE_MACD_EXIT_THEN",
            FORWARD_PLAN_RULE_MACD_EXIT_THEN,
        ),
        (
            "FORWARD_PLAN_RULE_RSI_ENTRY_IF_FMT",
            FORWARD_PLAN_RULE_RSI_ENTRY_IF_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_RSI_ENTRY_THEN",
            FORWARD_PLAN_RULE_RSI_ENTRY_THEN,
        ),
        (
            "FORWARD_PLAN_RULE_RSI_EXIT_IF_FMT",
            FORWARD_PLAN_RULE_RSI_EXIT_IF_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_RSI_EXIT_THEN",
            FORWARD_PLAN_RULE_RSI_EXIT_THEN,
        ),
        (
            "FORWARD_PLAN_RULE_BBANDS_ENTRY_IF_FMT",
            FORWARD_PLAN_RULE_BBANDS_ENTRY_IF_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_BBANDS_ENTRY_THEN",
            FORWARD_PLAN_RULE_BBANDS_ENTRY_THEN,
        ),
        (
            "FORWARD_PLAN_RULE_BBANDS_EXIT_IF",
            FORWARD_PLAN_RULE_BBANDS_EXIT_IF,
        ),
        (
            "FORWARD_PLAN_RULE_BBANDS_EXIT_THEN",
            FORWARD_PLAN_RULE_BBANDS_EXIT_THEN,
        ),
        (
            "FORWARD_PLAN_RULE_BUY_AND_HOLD",
            FORWARD_PLAN_RULE_BUY_AND_HOLD,
        ),
        (
            "FORWARD_PLAN_RULE_COMPOUND_CAVEAT",
            FORWARD_PLAN_RULE_COMPOUND_CAVEAT,
        ),
        ("FORWARD_PLAN_CADENCE_FMT", FORWARD_PLAN_CADENCE_FMT),
        // advisor-ensemble F8 (ADR-0063) — ensemble (signal-vote) plan copy
        (
            "FORWARD_PLAN_RULE_ENSEMBLE_MAJORITY_FMT",
            FORWARD_PLAN_RULE_ENSEMBLE_MAJORITY_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_ENSEMBLE_UNANIMOUS_FMT",
            FORWARD_PLAN_RULE_ENSEMBLE_UNANIMOUS_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_ENSEMBLE_TALLY_FMT",
            FORWARD_PLAN_RULE_ENSEMBLE_TALLY_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_ENSEMBLE_CAVEAT",
            FORWARD_PLAN_RULE_ENSEMBLE_CAVEAT,
        ),
        // F6 ensemble member-name enrichment — named-member rule copy
        (
            "FORWARD_PLAN_RULE_ENSEMBLE_MAJORITY_NAMED_FMT",
            FORWARD_PLAN_RULE_ENSEMBLE_MAJORITY_NAMED_FMT,
        ),
        (
            "FORWARD_PLAN_RULE_ENSEMBLE_UNANIMOUS_NAMED_FMT",
            FORWARD_PLAN_RULE_ENSEMBLE_UNANIMOUS_NAMED_FMT,
        ),
        ("FORWARD_PLAN_SIZING_TITLE", FORWARD_PLAN_SIZING_TITLE),
        ("FORWARD_PLAN_SIZING_FLAT_FMT", FORWARD_PLAN_SIZING_FLAT_FMT),
        ("FORWARD_PLAN_SIZING_LONG_FMT", FORWARD_PLAN_SIZING_LONG_FMT),
        (
            "FORWARD_PLAN_SIZING_BUY_AND_HOLD_FMT",
            FORWARD_PLAN_SIZING_BUY_AND_HOLD_FMT,
        ),
        ("FORWARD_PLAN_BUDGET_LINE_FMT", FORWARD_PLAN_BUDGET_LINE_FMT),
        (
            "FORWARD_PLAN_SIZING_CAPPED_NOTE",
            FORWARD_PLAN_SIZING_CAPPED_NOTE,
        ),
        ("FORWARD_PLAN_HORIZON_TITLE", FORWARD_PLAN_HORIZON_TITLE),
        ("FORWARD_PLAN_HORIZON_FMT", FORWARD_PLAN_HORIZON_FMT),
        (
            "FORWARD_PLAN_NOT_A_PREDICTION",
            FORWARD_PLAN_NOT_A_PREDICTION,
        ),
        ("FORWARD_PLAN_DISCLAIMER", FORWARD_PLAN_DISCLAIMER),
        ("CHART_LEGEND_BUY_LABEL", CHART_LEGEND_BUY_LABEL),
        ("CHART_LEGEND_SELL_LABEL", CHART_LEGEND_SELL_LABEL),
        ("CHART_LEGEND_BUY_GHOST_LABEL", CHART_LEGEND_BUY_GHOST_LABEL),
        (
            "CHART_LEGEND_SELL_GHOST_LABEL",
            CHART_LEGEND_SELL_GHOST_LABEL,
        ),
        ("CHART_LEGEND_PRICE_LABEL", CHART_LEGEND_PRICE_LABEL),
        ("DEBUG_LOGS_PLACEHOLDER", DEBUG_LOGS_PLACEHOLDER),
        ("SCREEN_NOT_YET", SCREEN_NOT_YET),
        ("STRATEGIES_PANEL_TITLE", STRATEGIES_PANEL_TITLE),
        ("STRATEGIES_SELECT_PROMPT", STRATEGIES_SELECT_PROMPT),
        ("STRATEGIES_PARAMS_TITLE", STRATEGIES_PARAMS_TITLE),
        ("STRATEGIES_EVENTS_TITLE", STRATEGIES_EVENTS_TITLE),
        ("STRATEGIES_SPARKLINE_LOADING", STRATEGIES_SPARKLINE_LOADING),
        ("KPI_TOTAL_RETURN_LABEL", KPI_TOTAL_RETURN_LABEL),
        ("KPI_CAGR_LABEL", KPI_CAGR_LABEL),
        ("KPI_SHARPE_LABEL", KPI_SHARPE_LABEL),
        ("KPI_MAX_DD_LABEL", KPI_MAX_DD_LABEL),
        ("KPI_WIN_RATE_LABEL", KPI_WIN_RATE_LABEL),
        ("KPI_TRADES_LABEL", KPI_TRADES_LABEL),
        ("LAB_KPI_FINAL_EQUITY_LABEL", LAB_KPI_FINAL_EQUITY_LABEL),
        ("LAB_KPI_MAX_DD_LABEL", LAB_KPI_MAX_DD_LABEL),
        ("LAB_KPI_TRADES_LABEL", LAB_KPI_TRADES_LABEL),
        ("LAB_KPI_FEES_LABEL", LAB_KPI_FEES_LABEL),
        ("LAB_KPI_SHARPE_LABEL", LAB_KPI_SHARPE_LABEL),
        ("LAB_KPI_RETURN_LABEL", LAB_KPI_RETURN_LABEL),
        ("LAB_KPI_BUYS_LABEL", LAB_KPI_BUYS_LABEL),
        ("LAB_KPI_SELLS_LABEL", LAB_KPI_SELLS_LABEL),
        ("LAB_KPI_NET_DELTA_LABEL", LAB_KPI_NET_DELTA_LABEL),
        ("VIEWER_METRICS_UNAVAILABLE", VIEWER_METRICS_UNAVAILABLE),
        ("VIEWER_NO_EQUITY_DATA", VIEWER_NO_EQUITY_DATA),
        (
            "VIEWER_EQUITY_UNAVAILABLE_PREFIX",
            VIEWER_EQUITY_UNAVAILABLE_PREFIX,
        ),
        (
            "STRATEGIES_EQUITY_HISTORY_UNAVAILABLE_PREFIX",
            STRATEGIES_EQUITY_HISTORY_UNAVAILABLE_PREFIX,
        ),
        ("KPI_DASH_PLACEHOLDER", KPI_DASH_PLACEHOLDER),
        ("MINUS_SIGN_LITERAL", MINUS_SIGN_LITERAL),
        ("RISK_PANEL_TITLE", RISK_PANEL_TITLE),
        ("RISK_LOADING", RISK_LOADING),
        ("RISK_EXPOSURE_SECTION_TITLE", RISK_EXPOSURE_SECTION_TITLE),
        (
            "RISK_DAILY_LOSS_SECTION_TITLE",
            RISK_DAILY_LOSS_SECTION_TITLE,
        ),
        (
            "RISK_KILL_THRESHOLD_SECTION_TITLE",
            RISK_KILL_THRESHOLD_SECTION_TITLE,
        ),
        ("RISK_FEED_UNAVAILABLE_PREFIX", RISK_FEED_UNAVAILABLE_PREFIX),
        ("AUDIT_PANEL_TITLE", AUDIT_PANEL_TITLE),
        ("AUDIT_FILTER_VENUE_LABEL", AUDIT_FILTER_VENUE_LABEL),
        ("AUDIT_FILTER_SYMBOL_LABEL", AUDIT_FILTER_SYMBOL_LABEL),
        ("AUDIT_FILTER_KIND_LABEL", AUDIT_FILTER_KIND_LABEL),
        ("AUDIT_FILTER_TIME_LABEL", AUDIT_FILTER_TIME_LABEL),
        ("AUDIT_FILTER_NO_MATCH", AUDIT_FILTER_NO_MATCH),
        ("AUDIT_LOADING", AUDIT_LOADING),
        ("AUDIT_PREV_LABEL", AUDIT_PREV_LABEL),
        ("AUDIT_NEXT_LABEL", AUDIT_NEXT_LABEL),
        ("AUDIT_KIND_ALL", AUDIT_KIND_ALL),
        ("AUDIT_KIND_FILL", AUDIT_KIND_FILL),
        ("AUDIT_KIND_STRATEGY_EVENT", AUDIT_KIND_STRATEGY_EVENT),
        ("AUDIT_KIND_RECONCILIATION", AUDIT_KIND_RECONCILIATION),
        ("AUDIT_TIME_LAST_1H", AUDIT_TIME_LAST_1H),
        ("AUDIT_TIME_LAST_24H", AUDIT_TIME_LAST_24H),
        ("AUDIT_TIME_LAST_7D", AUDIT_TIME_LAST_7D),
        ("AUDIT_COL_TIME", AUDIT_COL_TIME),
        ("AUDIT_COL_VENUE", AUDIT_COL_VENUE),
        ("AUDIT_COL_SYMBOL", AUDIT_COL_SYMBOL),
        ("AUDIT_COL_KIND", AUDIT_COL_KIND),
        ("AUDIT_COL_DESCRIPTION", AUDIT_COL_DESCRIPTION),
        ("AUDIT_COL_STRATEGY_ID", AUDIT_COL_STRATEGY_ID),
        ("AUDIT_QUERY_FAILED_PREFIX", AUDIT_QUERY_FAILED_PREFIX),
        // Phase 5 — HumanControl + execution-mode + pause + override
        // additive constants.
        ("PANEL_HUMAN_CONTROL_TITLE", PANEL_HUMAN_CONTROL_TITLE),
        ("PANEL_HUMAN_CONTROL_META", PANEL_HUMAN_CONTROL_META),
        (
            "HUMAN_CONTROL_LIMITS_UNAVAILABLE",
            HUMAN_CONTROL_LIMITS_UNAVAILABLE,
        ),
        (
            "HUMAN_CONTROL_DAILY_LOSS_LABEL",
            HUMAN_CONTROL_DAILY_LOSS_LABEL,
        ),
        (
            "HUMAN_CONTROL_MAX_POSITION_LABEL",
            HUMAN_CONTROL_MAX_POSITION_LABEL,
        ),
        (
            "HUMAN_CONTROL_USED_TODAY_LABEL",
            HUMAN_CONTROL_USED_TODAY_LABEL,
        ),
        ("EXECUTION_MODE_OBSERVE_LABEL", EXECUTION_MODE_OBSERVE_LABEL),
        (
            "EXECUTION_MODE_SUPERVISED_LABEL",
            EXECUTION_MODE_SUPERVISED_LABEL,
        ),
        ("EXECUTION_MODE_AUTO_LABEL", EXECUTION_MODE_AUTO_LABEL),
        ("EXECUTION_MODE_OBSERVE_HINT", EXECUTION_MODE_OBSERVE_HINT),
        (
            "EXECUTION_MODE_SUPERVISED_HINT",
            EXECUTION_MODE_SUPERVISED_HINT,
        ),
        ("EXECUTION_MODE_AUTO_HINT", EXECUTION_MODE_AUTO_HINT),
        ("STRATEGY_PAUSE_LABEL", STRATEGY_PAUSE_LABEL),
        ("STRATEGY_RESUME_LABEL", STRATEGY_RESUME_LABEL),
        ("OVERRIDE_RISK_VETO_PHRASE", OVERRIDE_RISK_VETO_PHRASE),
        (
            "OVERRIDE_RISK_VETO_DIALOG_TITLE",
            OVERRIDE_RISK_VETO_DIALOG_TITLE,
        ),
        (
            "OVERRIDE_RISK_VETO_DIALOG_BODY",
            OVERRIDE_RISK_VETO_DIALOG_BODY,
        ),
        (
            "OVERRIDE_RISK_VETO_PHRASE_MISMATCH_HINT",
            OVERRIDE_RISK_VETO_PHRASE_MISMATCH_HINT,
        ),
        (
            "OVERRIDE_RISK_VETO_CONFIRM_LABEL",
            OVERRIDE_RISK_VETO_CONFIRM_LABEL,
        ),
        (
            "OVERRIDE_RISK_VETO_CANCEL_LABEL",
            OVERRIDE_RISK_VETO_CANCEL_LABEL,
        ),
        (
            "OVERRIDE_RISK_VETO_BUTTON_LABEL",
            OVERRIDE_RISK_VETO_BUTTON_LABEL,
        ),
        ("SIDEBAR_NAV_CONTROL", SIDEBAR_NAV_CONTROL),
        // Phase A — Lab screen IA
        ("LAB_TITLE", LAB_TITLE),
        ("LIVE_TITLE", LIVE_TITLE),
        ("TRAIL_TITLE", TRAIL_TITLE),
        ("COMPARE_PLACEHOLDER", COMPARE_PLACEHOLDER),
        ("MEMORY_PLACEHOLDER", MEMORY_PLACEHOLDER),
        ("MODELS_PLACEHOLDER", MODELS_PLACEHOLDER),
        ("SETTINGS_PLACEHOLDER", SETTINGS_PLACEHOLDER),
        ("SIDEBAR_NAV_COMPARE", SIDEBAR_NAV_COMPARE),
        ("SIDEBAR_NAV_MEMORY", SIDEBAR_NAV_MEMORY),
        ("SIDEBAR_NAV_MODELS", SIDEBAR_NAV_MODELS),
        ("SIDEBAR_NAV_SETTINGS", SIDEBAR_NAV_SETTINGS),
        ("LAB_NARROWED_FROM_BADGE", LAB_NARROWED_FROM_BADGE),
        // Phase A — chip widget strings (T-D-5, T-D-6, T-D-7)
        ("PAIR_CHIP_VENUE_BINANCE", PAIR_CHIP_VENUE_BINANCE),
        ("LAB_NO_STRATEGY_HINT", LAB_NO_STRATEGY_HINT),
        ("LAB_NO_PAIR_HINT", LAB_NO_PAIR_HINT),
        ("STRATEGY_CHIP_COMPARE_ADD", STRATEGY_CHIP_COMPARE_ADD),
        ("STRATEGY_CHIP_COMPARE_REMOVE", STRATEGY_CHIP_COMPARE_REMOVE),
        // lab-compare-equity-overlay T2 — two-run equity overlay
        ("COMPARE_CELL_OVERLAY_ADD", COMPARE_CELL_OVERLAY_ADD),
        (
            "COMPARE_CELL_OVERLAY_SELECTED",
            COMPARE_CELL_OVERLAY_SELECTED,
        ),
        ("COMPARE_CELL_OVERLAY_HINT", COMPARE_CELL_OVERLAY_HINT),
        ("COMPARE_OVERLAY_TITLE", COMPARE_OVERLAY_TITLE),
        ("COMPARE_OVERLAY_EMPTY", COMPARE_OVERLAY_EMPTY),
        (
            "COMPARE_OVERLAY_LEGEND_PRIMARY",
            COMPARE_OVERLAY_LEGEND_PRIMARY,
        ),
        (
            "COMPARE_OVERLAY_LEGEND_COMPARE",
            COMPARE_OVERLAY_LEGEND_COMPARE,
        ),
        (
            "COMPARE_OVERLAY_LEGEND_SWATCH",
            COMPARE_OVERLAY_LEGEND_SWATCH,
        ),
        ("COMPARE_OVERLAY_NO_SERIES", COMPARE_OVERLAY_NO_SERIES),
        ("DATE_RANGE_SEPARATOR", DATE_RANGE_SEPARATOR),
        ("DATE_RANGE_CUSTOM_LABEL", DATE_RANGE_CUSTOM_LABEL),
        ("DATE_RANGE_START_PLACEHOLDER", DATE_RANGE_START_PLACEHOLDER),
        ("DATE_RANGE_END_PLACEHOLDER", DATE_RANGE_END_PLACEHOLDER),
        ("DATE_RANGE_INVALID_DATE", DATE_RANGE_INVALID_DATE),
        // cockpit-toast-queue v0.1.0 — toast tray dismiss button
        ("TOAST_DISMISS_BUTTON", TOAST_DISMISS_BUTTON),
        // Phase A — Lab Run button + compare overflow toast (T-D-14, T-D-16)
        ("LAB_COMPARE_CAP_HIT", LAB_COMPARE_CAP_HIT),
        ("LAB_RUN_BUTTON", LAB_RUN_BUTTON),
        ("LAB_RUN_BUTTON_RUNNING", LAB_RUN_BUTTON_RUNNING),
        ("LAB_RUN_BUTTON_COMPLETED", LAB_RUN_BUTTON_COMPLETED),
        ("LAB_RUN_BUTTON_FAILED", LAB_RUN_BUTTON_FAILED),
        ("LAB_RUN_BUTTON_DISABLED", LAB_RUN_BUTTON_DISABLED),
        ("LAB_RUN_BUTTON_CANCELLED", LAB_RUN_BUTTON_CANCELLED),
        ("LAB_STOP_BUTTON", LAB_STOP_BUTTON),
        // Phase B — run delta badge (T-D-N13)
        ("RUN_DELTA_BADGE_PNL_LABEL", RUN_DELTA_BADGE_PNL_LABEL),
        ("RUN_DELTA_BADGE_DD_LABEL", RUN_DELTA_BADGE_DD_LABEL),
        ("RUN_DELTA_BADGE_SHARPE_LABEL", RUN_DELTA_BADGE_SHARPE_LABEL),
        // Phase A — equity + compare legend (T-D-15)
        ("CHART_LEGEND_EQUITY_LABEL", CHART_LEGEND_EQUITY_LABEL),
        ("CHART_LEGEND_COMPARE_NO_DATA", CHART_LEGEND_COMPARE_NO_DATA),
        // Phase A — equity axis K-suffix (T-D-11)
        (
            "CHART_EQUITY_AXIS_THOUSAND_SUFFIX",
            CHART_EQUITY_AXIS_THOUSAND_SUFFIX,
        ),
        // Phase F — Memory + Models + Assistant (ui-rethink-phase-f-memory-models-assistant T-D-N7)
        ("MEMORY_EMPTY_STATE", MEMORY_EMPTY_STATE),
        ("MEMORY_TOOLBAR_CARDS_LABEL", MEMORY_TOOLBAR_CARDS_LABEL),
        ("MEMORY_TOOLBAR_CLUSTER_LABEL", MEMORY_TOOLBAR_CLUSTER_LABEL),
        (
            "MEMORY_CLUSTER_MODE_DISABLED_TOOLTIP",
            MEMORY_CLUSTER_MODE_DISABLED_TOOLTIP,
        ),
        ("MEMORY_CARD_TRAIL_LINK_LABEL", MEMORY_CARD_TRAIL_LINK_LABEL),
        ("MODELS_EMPTY_STATE", MODELS_EMPTY_STATE),
        (
            "MODELS_SPARKLINE_DEFERRED_TOOLTIP",
            MODELS_SPARKLINE_DEFERRED_TOOLTIP,
        ),
        ("MODELS_SPARKLINE_PLACEHOLDER", MODELS_SPARKLINE_PLACEHOLDER),
        ("MODELS_STATUS_STAGED_TOOLTIP", MODELS_STATUS_STAGED_TOOLTIP),
        (
            "MODELS_FAMILY_PATCHTST_DISABLED_TOOLTIP",
            MODELS_FAMILY_PATCHTST_DISABLED_TOOLTIP,
        ),
        (
            "MODELS_FAMILY_TRANSFORMER_DISABLED_TOOLTIP",
            MODELS_FAMILY_TRANSFORMER_DISABLED_TOOLTIP,
        ),
        ("MODELS_TOOLBAR_FAMILY_LABEL", MODELS_TOOLBAR_FAMILY_LABEL),
        ("MODELS_TOOLBAR_STATUS_LABEL", MODELS_TOOLBAR_STATUS_LABEL),
        ("ASSISTANT_OFFLINE_TITLE", ASSISTANT_OFFLINE_TITLE),
        ("ASSISTANT_OFFLINE_BODY", ASSISTANT_OFFLINE_BODY),
        ("ASSISTANT_TOGGLE_OPEN_LABEL", ASSISTANT_TOGGLE_OPEN_LABEL),
        ("ASSISTANT_TOGGLE_CLOSE_LABEL", ASSISTANT_TOGGLE_CLOSE_LABEL),
        // v3-llm-forecaster Wave F (T-D-N(F2)) — reasoning-trace body strings
        ("ASSISTANT_REASONING_TITLE", ASSISTANT_REASONING_TITLE),
        (
            "ASSISTANT_REASONING_HEADER_FMT",
            ASSISTANT_REASONING_HEADER_FMT,
        ),
        (
            "ASSISTANT_REASONING_COST_LABEL",
            ASSISTANT_REASONING_COST_LABEL,
        ),
        (
            "ASSISTANT_REASONING_COST_PENDING",
            ASSISTANT_REASONING_COST_PENDING,
        ),
        (
            "ASSISTANT_REASONING_TRACE_LABEL",
            ASSISTANT_REASONING_TRACE_LABEL,
        ),
        (
            "ASSISTANT_REASONING_LESSONS_LABEL",
            ASSISTANT_REASONING_LESSONS_LABEL,
        ),
        (
            "ASSISTANT_REASONING_NO_LESSONS",
            ASSISTANT_REASONING_NO_LESSONS,
        ),
        (
            "ASSISTANT_REASONING_LESSON_PENDING_FMT",
            ASSISTANT_REASONING_LESSON_PENDING_FMT,
        ),
        (
            "ASSISTANT_REASONING_HISTORY_LABEL",
            ASSISTANT_REASONING_HISTORY_LABEL,
        ),
        (
            "ASSISTANT_REASONING_HISTORY_EMPTY",
            ASSISTANT_REASONING_HISTORY_EMPTY,
        ),
        (
            "ASSISTANT_REASONING_HISTORY_ROW_FMT",
            ASSISTANT_REASONING_HISTORY_ROW_FMT,
        ),
        (
            "ASSISTANT_REASONING_TRAIL_LINK_LABEL",
            ASSISTANT_REASONING_TRAIL_LINK_LABEL,
        ),
        (
            "ASSISTANT_REASONING_WARMING_TITLE",
            ASSISTANT_REASONING_WARMING_TITLE,
        ),
        (
            "ASSISTANT_REASONING_WARMING_BODY",
            ASSISTANT_REASONING_WARMING_BODY,
        ),
        // cockpit-training-control T-D-N2/N16
        ("TRAINING_PANEL_HEADER", TRAINING_PANEL_HEADER),
        ("TRAINING_BUTTON_TRAIN", TRAINING_BUTTON_TRAIN),
        ("TRAINING_BUTTON_CANCEL", TRAINING_BUTTON_CANCEL),
        ("TRAINING_BUTTON_CLEAR_LOG", TRAINING_BUTTON_CLEAR_LOG),
        ("TRAINING_STATUS_IDLE", TRAINING_STATUS_IDLE),
        ("TRAINING_STATUS_RUNNING", TRAINING_STATUS_RUNNING),
        ("TRAINING_STATUS_TRAINING_FMT", TRAINING_STATUS_TRAINING_FMT),
        ("TRAINING_STATUS_CANCELLED", TRAINING_STATUS_CANCELLED),
        ("TRAINING_STATUS_FAILED_FMT", TRAINING_STATUS_FAILED_FMT),
        ("TRAINING_STATUS_DONE_FMT", TRAINING_STATUS_DONE_FMT),
        ("ORPHAN_LIVE_FMT", ORPHAN_LIVE_FMT),
        ("ORPHAN_DEAD_FMT", ORPHAN_DEAD_FMT),
        ("TRAINING_LOG_EMPTY", TRAINING_LOG_EMPTY),
        ("TRAINING_LOG_JUMP_TO_BOTTOM", TRAINING_LOG_JUMP_TO_BOTTOM),
        // cockpit-training-control T-D-N12/N16 — training_plot format strings
        ("TRAINING_PLOT_EMPTY", TRAINING_PLOT_EMPTY),
        ("TRAINING_PLOT_WARMING_UP", TRAINING_PLOT_WARMING_UP),
        ("TRAINING_PLOT_EPOCH_ROW_FMT", TRAINING_PLOT_EPOCH_ROW_FMT),
        ("TRAINING_PLOT_HEADER_FMT", TRAINING_PLOT_HEADER_FMT),
        ("TRAINING_PLOT_LATEST_FMT", TRAINING_PLOT_LATEST_FMT),
        // lab-yahoo-realdata T-C3 — source toggle + cadence badge
        ("LAB_SOURCE_SYNTHETIC", LAB_SOURCE_SYNTHETIC),
        ("LAB_SOURCE_YAHOO", LAB_SOURCE_YAHOO),
        // simple-strategies-realdata T-B1/Q-miss — Binance source + data-missing UX
        ("LAB_SOURCE_BINANCE", LAB_SOURCE_BINANCE),
        (
            "LAB_BINANCE_CACHE_MISS_NOTICE",
            LAB_BINANCE_CACHE_MISS_NOTICE,
        ),
        ("LAB_BINANCE_REVISION_ERROR", LAB_BINANCE_REVISION_ERROR),
        ("LAB_CADENCE_1M", LAB_CADENCE_1M),
        ("LAB_CADENCE_1H", LAB_CADENCE_1H),
        ("LAB_CADENCE_1D", LAB_CADENCE_1D),
        ("LAB_YAHOO_CACHE_MISS_PREFIX", LAB_YAHOO_CACHE_MISS_PREFIX),
        ("LAB_CACHE_STATE_EMPTY", LAB_CACHE_STATE_EMPTY),
        ("LAB_CACHE_STATE_STALE", LAB_CACHE_STATE_STALE),
        ("LAB_CACHE_STATE_FRESH", LAB_CACHE_STATE_FRESH),
        (
            "LAB_CACHE_STATE_SUMMARY_PREFIX",
            LAB_CACHE_STATE_SUMMARY_PREFIX,
        ),
        // cockpit-activity-status-bar v0.1.0 Wave B (T-D-N6)
        ("ACTIVITY_KIND_YAHOO_LABEL", ACTIVITY_KIND_YAHOO_LABEL),
        ("ACTIVITY_KIND_LAB_RUN_LABEL", ACTIVITY_KIND_LAB_RUN_LABEL),
        ("ACTIVITY_KIND_TRAINING_LABEL", ACTIVITY_KIND_TRAINING_LABEL),
        ("ACTIVITY_TAPE_MORE_PREFIX", ACTIVITY_TAPE_MORE_PREFIX),
        ("ACTIVITY_TAPE_MORE_SUFFIX", ACTIVITY_TAPE_MORE_SUFFIX),
        // F5 — Forward paper-trade P/L framing (ADR-0060 § D5)
        ("LIVE_FORWARD_PNL_LABEL", LIVE_FORWARD_PNL_LABEL),
        ("LIVE_FORWARD_BUDGET_LABEL", LIVE_FORWARD_BUDGET_LABEL),
        ("LIVE_FORWARD_FX_NOTE_FMT", LIVE_FORWARD_FX_NOTE_FMT),
        ("LIVE_FORWARD_DISCLAIMER", LIVE_FORWARD_DISCLAIMER),
        ("LIVE_FORWARD_RUNNING_FMT", LIVE_FORWARD_RUNNING_FMT),
    ]
}

// ── cockpit-baseline-panel v0.1.0 — passive-BH baseline screen ───────────────
// Honest-bounded-scope constraint (R3 / A3) is BINDING: BASELINE_CAPTION
// states "passive baseline; active ≤ passive in the reachable universe,
// this sample" and MUST NOT claim "optimal" / "unbeatable" / "none beat it"
// — that would overstate the program's bounded terminal verdict into a
// universal claim. Asserted by the no-overclaim string test (AC5).

/// Sidebar nav label for the Baseline screen (R6).
pub const BASELINE_SIDEBAR_LABEL: &str = "Baseline";

/// Headline / page title for the Baseline screen (R3).
pub const BASELINE_HEADLINE: &str = "Passive baseline";

/// Plain-language caption below the headline (R3 / A3). States the
/// construction (equal-weight buy-and-hold, bought once, never rebalanced)
/// and the honest bounded finding. MUST NOT overclaim — see the binding
/// constraint at the top of this block + the AC5 no-overclaim test.
pub const BASELINE_CAPTION: &str = "Equal-weight buy-and-hold across 10 large-cap pairs, \
    bought once at year-open and never rebalanced. Passive baseline; active \u{2264} passive \
    in the reachable universe, this sample.";

/// Year toggle chip label — 2023 (R2).
pub const BASELINE_YEAR_2023_LABEL: &str = "2023";

/// Year toggle chip label — 2024 (R2).
pub const BASELINE_YEAR_2024_LABEL: &str = "2024";

/// Error-state copy when the equity CSV is absent (R4 / R7). Tells the
/// operator what is missing and where it lives — never a bare "no data".
pub const BASELINE_DATA_UNAVAILABLE: &str = "Baseline equity data isn't bundled in this build. \
    The realized buy-and-hold curves live at \
    spec/runbooks/artifacts/passive-baseline-2026-06-08/.";

/// Caption-only risk-detail line (A2 / D1=c). Surfaces the §7.1 Sortino +
/// Calmar metrics, which have no KPI card in the six-card strip. Rendered
/// `FG_3` below the drawdown band.
pub const BASELINE_RISK_DETAIL: &str =
    "Sortino 2.51 / Calmar 5.68 (2023)  \u{00b7}  Sortino 1.20 / Calmar 1.85 (2024)";

// ── cockpit-reports-viewer v0.1.0 — browse + render backtest reports ─────────
// All Reports-screen copy lives here (R5 / AC6 — no inline literals). The
// picker title surfaces the scope ("Backtest reports") so the exclusion of
// the robustness-sweep / test-report families is not a mystery (§ Data
// contract).

/// Sidebar nav label for the Reports screen (R6 / D4 — Library group).
pub const REPORTS_SIDEBAR_LABEL: &str = "Reports";

/// Picker (left list) title. Names the scope — the picker browses the
/// committed `backtest-*.md` corpus only (R1 / § Data contract).
pub const REPORTS_PICKER_TITLE: &str = "Backtest reports";

/// Empty-list copy when no `backtest-*.md` reports are discovered under
/// `spec/` (R3) — never a blank screen; tells the operator the scope.
pub const REPORTS_EMPTY_LIST: &str = "No backtest reports found in spec/ yet.";

/// Detail-pane prompt when no report is selected yet (R3) — the cold-start
/// detail surface; tells the operator what to do next.
pub const REPORTS_SELECT_PROMPT: &str = "Select a report to view its results.";

/// Error-state copy for a selection that is missing on disk (deleted
/// between discovery + selection) or whose body is unreadable (R3). Never a
/// bare "no data" — says what happened and what to check.
pub const REPORTS_LOAD_ERROR: &str = "This report could not be read — it may have been moved \
    or its summary table is malformed.";

/// Picker-row marker for a report that has a stem-matched equity companion
/// CSV — i.e. selecting it paints a populated equity curve (backtest-equity-
/// companion UX follow-on). Rendered in the `ACCENT` token on the row so the
/// operator can see at a glance which reports have a curve, without hunting.
/// The filled-circle glyph is paired with the marker's `ACCENT` colour AND a
/// tooltip-able position so colour is never the only signal (accessibility
/// minimum). Kept to a single glyph to stay compact in the 320 px rail.
pub const REPORTS_HAS_CURVE_MARKER: &str = "\u{25CF} curve";

/// "Curve only" filter chip label (reports-picker-curve-filter). The DEFAULT,
/// active filter — the picker rail shows only reports that ship a stem-matched
/// equity companion (the `\u{25CF} curve` rows). The chip carries the count of
/// companion-bearing reports as a trailing `(N)`, formatted at the call site so
/// the prose stays here and the number stays a runtime value. Uses the `ACCENT`
/// hue when active (matching the `\u{25CF}` marker), so the filter that "shows
/// the curves" is visually tied to the curve marker itself.
pub const REPORTS_FILTER_CURVE_ONLY: &str = "Curve only";

/// "All" filter chip label (reports-picker-curve-filter). Reveals the FULL
/// discovered corpus (companion-bearing AND companion-less reports). Carries the
/// full discovered count as a trailing `(M)`, formatted at the call site.
pub const REPORTS_FILTER_ALL: &str = "All";

/// Hint shown in the picker rail when the "Curve only" filter is active but the
/// discovered corpus has ZERO companion-bearing reports (reports-picker-curve-
/// filter edge case). Never a blank list — tells the operator the curve-only
/// view is empty and to use the "All" toggle to see every report. Belt-and-
/// braces (the live corpus ships 14 companion reports), but honest if a pruned
/// checkout drops them all.
pub const REPORTS_FILTER_NO_CURVE_HINT: &str = "No reports have an equity curve yet \u{2014} switch to \u{201c}All\u{201d} to see every report.";

// ── advisor-leaderboard-screen v0.1.0 — strategy bake-off leaderboard ─────────
//
// Step 3 of the single-coin investment-advisor journey (rank & pick best).
// The UI owns ALL copy + the mandatory not-advice disclaimer (the engine ships
// a STRUCTURED `Recommendation`, never a pre-rendered string). Placeholders
// (`{coin}`, `{winner}`, `{window}`) are filled at the call site via `.replace`
// — the prose lives here, the runtime values stay values (the established
// `LAB_BINANCE_*` template discipline).

/// Sidebar nav label for the Leaderboard screen (Work group, after Baseline).
pub const LEADERBOARD_SIDEBAR_LABEL: &str = "Leaderboard";

/// Page headline — names the screen's job in plain language.
pub const LEADERBOARD_HEADLINE: &str = "Strategy bake-off";

/// Page caption — one line on what the bake-off does + how to read it.
pub const LEADERBOARD_CAPTION: &str = "Every strategy backtested on the same coin and window, ranked by risk-adjusted return. \
     Buy-and-hold is always in the field as the benchmark.";

/// Primary action button — runs the bake-off with the default coin + window.
pub const LEADERBOARD_RUN_BUTTON: &str = "Run bake-off";

/// Primary action button label while a bake-off is in flight (button disabled).
pub const LEADERBOARD_RUN_BUTTON_RUNNING: &str = "Running\u{2026}";

/// Empty-state prompt — the cold "no run yet" surface (never a blank screen).
/// Tells the operator exactly what to do next, reflecting the CURRENT
/// selection. `{coin}` / `{lookback}` are filled at the call site (F3).
pub const LEADERBOARD_EMPTY_PROMPT: &str = "No bake-off yet. Press \u{201c}Run bake-off\u{201d} to rank every strategy on {coin} over \
     {lookback}.";

/// Loading copy — shown beside the spinner while the bake-off runs. Sets the
/// expectation that backtesting the field takes a moment.
pub const LEADERBOARD_LOADING: &str =
    "Backtesting every strategy on the same window\u{2026} this takes a few seconds.";

/// Determinate progress label — shown above the bake-off progress bar once the
/// first candidate-level `BakeoffProgress` event arrives. Names the strategy now
/// running and the 1-based position in the field. `{current}` = the id about to
/// run; `{n}` = `done + 1` (1-based); `{total}` = the field size. Filled at the
/// call site (the runtime values stay values; the copy stays here).
pub const LEADERBOARD_PROGRESS_FMT: &str = "Running {current} \u{2014} {n} of {total}";

/// Error-state prefix — paired with the engine's failure detail (R: never a
/// bare "no data"; says what to check).
pub const LEADERBOARD_ERROR_PREFIX: &str = "The bake-off could not run";

/// Friendly message when the bake-off is triggered in a build without the
/// engine runtime (fixtures / no-`live`). Directs the operator to the live
/// build rather than hanging or panicking.
pub const LEADERBOARD_RUN_NEEDS_LIVE: &str = "Running a bake-off needs the live build. Launch the cockpit with \
     `cargo run -p ui --bin cockpit_live` to rank strategies on real data.";

/// Dynamic fetch failed — no network connectivity or Binance unreachable.
/// Maps to `BinanceFetchError::Network` / `::Timeout` (ADR-0061 Wave C).
pub const LEADERBOARD_FETCH_NETWORK_ERROR: &str =
    "Couldn't reach Binance to fetch market data. Check your connection and try again.";

/// Binance rate-limit (HTTP 429). Maps to `BinanceFetchError::RateLimited`.
pub const LEADERBOARD_FETCH_RATE_LIMITED: &str =
    "Binance is rate-limiting requests; wait a moment and try again.";

/// Unknown / delisted symbol. Maps to `BinanceFetchError::UnknownSymbol`.
/// The placeholder `{symbol}` is formatted at the call site.
pub const LEADERBOARD_FETCH_UNKNOWN_SYMBOL: &str = "Binance has no market data for that symbol.";

/// No klines returned for the requested window.
/// Maps to `BinanceFetchError::NoDataForRange` / `DynamicCacheError::NoData`.
pub const LEADERBOARD_FETCH_NO_DATA: &str =
    "No market data available for that symbol in the selected window.";

/// Table column header — the rank position (1 = the crowned pick).
pub const LEADERBOARD_COL_RANK: &str = "#";

/// Table column header — the strategy id.
pub const LEADERBOARD_COL_STRATEGY: &str = "Strategy";

/// Table column header — total return over the window.
pub const LEADERBOARD_COL_RETURN: &str = "Return";

/// Table column header — annualised Sharpe (the primary ranking metric).
pub const LEADERBOARD_COL_SHARPE: &str = "Sharpe";

/// Table column header — maximum drawdown over the window.
pub const LEADERBOARD_COL_MAX_DD: &str = "Max drawdown";

/// Table column header — executed trade count.
pub const LEADERBOARD_COL_TRADES: &str = "Trades";

/// Row tag for the buy-and-hold benchmark arm — names it the BASELINE the
/// active strategies are measured against (ADR-0066: the benchmark is the null
/// hypothesis, not a candidate that must clear the robustness bar). Reads as a
/// reference line, never a failed/fragile candidate. The operator's "BH is
/// always in the field" rule made plain.
pub const LEADERBOARD_BENCHMARK_TAG: &str = "baseline (buy & hold)";

/// Informational robustness note on the BENCHMARK row (ADR-0066 § D3) — the
/// benchmark's own Fragile flag is still computed + shown, but it is the
/// baseline (exempt from the candidate verdict), so the flag reads as context
/// ("the baseline itself is path-dependent on a single volatile asset"), NEVER
/// as disqualifying. Pairs with the muted informational treatment so it never
/// reads like the prominent "cannot be crowned" badge an ACTIVE fragile arm
/// gets.
pub const LEADERBOARD_BENCHMARK_FRAGILE_NOTE: &str = "baseline is path-dependent";

/// Row tag for the crowned pick — the `ACCENT` "best" marker, paired with the
/// row's accent treatment so colour is never the only signal (accessibility).
pub const LEADERBOARD_CROWN_TAG: &str = "\u{2605} best";

/// Robustness tag — fragile under resampling. Carries a word (not just colour)
/// so the warning is legible without colour (accessibility minimum).
pub const LEADERBOARD_FRAGILE_TAG: &str = "fragile";

/// Robustness tag — survived resampling.
pub const LEADERBOARD_ROBUST_TAG: &str = "robust";

/// Robustness tag — borderline under resampling.
pub const LEADERBOARD_MARGINAL_TAG: &str = "marginal";

// ── Ensemble (signal-vote) row labelling (F8 / ADR-0063) ──────────────────────
//
// The two frozen ensemble candidates carry opaque ids (`v0.8.vote.majority` /
// `v0.8.vote.unanimous`). The leaderboard renders a friendly, legible display
// label so the row reads AS an ensemble (a vote), not as a single indicator,
// plus a `vote` tag so the kind is unmistakable beyond the id. The `ui` owns
// the words; the id→label mapping is a closed `ui`-side match (no engine string
// crosses the seam).

/// Friendly display label for the majority-vote ensemble row — names the method
/// and the `k`-of-`n` quorum so the row is self-explanatory. Replaces the opaque
/// `v0.8.vote.majority` id in the strategy column.
pub const LEADERBOARD_ENSEMBLE_MAJORITY_LABEL: &str = "Majority vote (2-of-3)";

/// Friendly display label for the unanimous-vote ensemble row.
pub const LEADERBOARD_ENSEMBLE_UNANIMOUS_LABEL: &str = "Unanimous vote (4-of-4)";

// ── Combination-slate ensemble labels (advisor-combination-search, ADR-0067) ──
//
// The 6 pre-registered combination arms (3 decorrelation pairs + the complete
// k∈{1,2,3}-of-4 ladder) carry opaque `v0.8.vote.*` ids. Each gets a friendly,
// legible display label so the leaderboard row reads AS the specific vote (the
// method + the named members or the k-of-n quorum), never a raw id. The `ui`
// owns the words; the id→label mapping stays a closed `ui`-side match (no engine
// string crosses the seam). The pair labels NAME the members so the operator can
// see WHICH strategies the consensus combines (the decorrelation thesis is the
// whole point); the k-ladder labels show the quorum so the ladder reads as a
// ladder. They are intentionally consistent in voice with the two F8 labels.

/// `v0.8.vote.trend_pair` — `Unanimous{n:2}` over MACD + SMA (the predicted-null
/// control: both members trend-following, so little decorrelation lift expected).
pub const LEADERBOARD_ENSEMBLE_TREND_PAIR_LABEL: &str = "Unanimous vote (MACD + SMA trend)";

/// `v0.8.vote.tr_mr_macd_rsi` — `Unanimous{n:2}` over MACD trend + RSI reversion
/// (the trend ∧ mean-revert decorrelation lever).
pub const LEADERBOARD_ENSEMBLE_TR_MR_MACD_RSI_LABEL: &str = "Unanimous vote (MACD + RSI)";

/// `v0.8.vote.tr_mr_sma_bb` — `Unanimous{n:2}` over SMA trend + Bollinger
/// reversion (the second trend ∧ band-reversion decorrelation pair).
pub const LEADERBOARD_ENSEMBLE_TR_MR_SMA_BB_LABEL: &str = "Unanimous vote (SMA + Bollinger)";

/// `v0.8.vote.any1of4` — `Majority{k:1,n:4}` over all 4 base signals (the loosest
/// k-ladder rung: long if ANY member fires).
pub const LEADERBOARD_ENSEMBLE_ANY1OF4_LABEL: &str = "Majority vote (1-of-4)";

/// `v0.8.vote.k2of4` — `Majority{k:2,n:4}` over all 4 base signals (the balanced
/// quorum rung of the k-ladder).
pub const LEADERBOARD_ENSEMBLE_K2OF4_LABEL: &str = "Majority vote (2-of-4)";

/// `v0.8.vote.k3of4` — `Majority{k:3,n:4}` over all 4 base signals (the strict
/// rung: long only on broad agreement).
pub const LEADERBOARD_ENSEMBLE_K3OF4_LABEL: &str = "Majority vote (3-of-4)";

/// Row tag marking an ensemble candidate as a vote (so the kind is legible
/// beyond the friendly label — pairs with the label the way `benchmark` pairs
/// with the buy-and-hold row).
pub const LEADERBOARD_ENSEMBLE_VOTE_TAG: &str = "vote";

// ── Short-capable arm row labelling (advisor-short-selling, ADR-0068 § D9) ─────
//
// The FIXED pre-registered 5-arm short slate carries opaque `*_ls` /
// `always_short` ids. The leaderboard renders a friendly, legible display label
// so the row reads AS a long/short (directional) strategy — never a raw
// `sma_cross_ls` id — plus a `short` tag so the kind is unmistakable beyond the
// id (mirrors the `vote` ensemble tag). The `ui` owns the words; the id→label
// mapping is a closed `ui`-side match (no engine string crosses the seam) —
// learned from advisor-combination-search, where the engine adds the ids but
// the leaderboard mapping must be extended ui-side or they show raw ids.
//
// The four `_ls` arms are symmetric long/short variants of the existing rule
// engines (long on the bullish flip, SHORT instead of flat on the bearish
// flip); `always_short` is the always-short benchmark control (the down-side
// mirror of buy-and-hold — loses on any up-trend by construction).

/// `sma_cross_ls` — SMA crossover, symmetric long/short (long on the golden
/// cross, short on the death cross).
pub const LEADERBOARD_SHORT_SMA_CROSS_LS_LABEL: &str = "SMA crossover (long/short)";

/// `macd_ls` — MACD trend, symmetric long/short (long on the bullish flip,
/// short on the bearish flip).
pub const LEADERBOARD_SHORT_MACD_LS_LABEL: &str = "MACD trend (long/short)";

/// `rsi_ls` — RSI reversion, symmetric long/short (long oversold, short
/// overbought).
pub const LEADERBOARD_SHORT_RSI_LS_LABEL: &str = "RSI reversion (long/short)";

/// `bbands_ls` — Bollinger reversion, symmetric long/short (long on the lower
/// band, short on the upper band).
pub const LEADERBOARD_SHORT_BBANDS_LS_LABEL: &str = "Bollinger reversion (long/short)";

/// `always_short` — the always-short benchmark control (the down-side mirror of
/// buy-and-hold). Loses on any up-trending window by construction; anchors the
/// "what un-timed continuous shorting does" honest framing.
pub const LEADERBOARD_SHORT_ALWAYS_SHORT_LABEL: &str = "Always short (benchmark)";

/// Row tag marking a short-capable arm so the user sees the short field (pairs
/// with the friendly label the way `vote` pairs with an ensemble and `baseline`
/// pairs with buy-and-hold). The kind is legible beyond colour (accessibility).
pub const LEADERBOARD_SHORT_TAG: &str = "short";

// ── Signal-library expansion arm labels (advisor-signal-library-expansion, ─────
//    ADR-0071 § D6)
//
// The FIXED pre-registered 5-arm signal-library slate carries opaque `v0.*` ids
// (the bake-off emits e.g. `"v0.donchian_break"`). The leaderboard renders a
// friendly, legible display label so each row reads AS the strategy it is — a
// breakout / volume / momentum rule — never a raw `v0.donchian_break` id. The
// `ui` owns the words; the id→label mapping is a closed `ui`-side match (no
// engine string crosses the seam) — the same discipline as the ensemble +
// short labels, learned from advisor-combination-search where the engine adds
// the ids but the leaderboard mapping must be extended ui-side or they show
// raw ids.
//
// Each label names the rule AND its single pre-registered parameterization (the
// LOCKED literal) in parentheses, so the operator can read what the arm does
// without opening the plan — consistent in voice with the ensemble labels which
// name the k-of-n quorum.

/// `v0.donchian_break` — `close > max(high, 20)`: enter on a 20-bar-high
/// breakout (price-extreme trend-follow, fires the bar a new high prints).
pub const LEADERBOARD_SIGNAL_DONCHIAN_BREAK_LABEL: &str = "Donchian breakout (20-bar high)";

/// `v0.donchian_floor` — `close > min(low, 20)`: long while price holds above
/// the 20-bar support floor (the channel-floor / anti-breakdown rule).
pub const LEADERBOARD_SIGNAL_DONCHIAN_FLOOR_LABEL: &str = "Donchian floor (hold 20-bar support)";

/// `v0.vol_breakout` — `close > max(high, 20) AND volume > 2 * avg(volume, 20)`:
/// a 20-bar-high breakout CONFIRMED by a 2x volume surge (the volume-flow axis).
pub const LEADERBOARD_SIGNAL_VOL_BREAKOUT_LABEL: &str = "Volume-confirmed breakout (20-bar)";

/// `v0.roc_momentum` — `close > avg(close, 10) * 1.05`: price 5% above its
/// 10-bar mean (a short-horizon momentum burst).
pub const LEADERBOARD_SIGNAL_ROC_MOMENTUM_LABEL: &str = "Momentum burst (5% over 10 bars)";

/// `v0.obv` — `obv() > obv_avg(20) AND close > sma(close, 50)`: on-balance
/// volume above its own 20-bar average, gated by a 50-bar trend filter (the
/// cumulative volume-flow / accumulation rule).
pub const LEADERBOARD_SIGNAL_OBV_LABEL: &str = "On-balance-volume accumulation";

/// `v0.dvol_regime` — hold the coin while Deribit DVOL (the BTC/ETH 30-day
/// implied-vol index) sits BELOW its own trailing 30-day median (the calm
/// regime), else step to cash (ADR-0072, the options/implied-vol fresh-channel
/// probe; BTC+ETH only — the arm is absent for other coins).
pub const LEADERBOARD_SIGNAL_DVOL_REGIME_LABEL: &str =
    "Implied-vol regime (hold when DVOL < 30-day median)";

/// The short-field disclaimer carried on the leaderboard when one or more
/// short-capable arms are in the field (R-SS.9 / ADR-0068 § D8). Frames the
/// honest "a short's drawdown can be brutal" signal + the unbounded-loss
/// caution. Distinct from (and additional to) the persistent not-advice
/// disclaimer every result surface already carries.
pub const LEADERBOARD_SHORT_FIELD_NOTE: &str = "Short-capable arms (tagged \u{201c}short\u{201d}) can bet on a decline. A short's drawdown can \
     be brutal \u{2014} it loses without bound as price rises.";

/// Row note for an ensemble that executed ZERO trades — its quorum
/// (e.g. 4-of-4 unanimous agreement on a single volatile asset) was never
/// reached, so it stayed in cash the whole window. Renders the honest "why it's
/// flat" instead of a bare Sharpe-0 row that looks indistinguishable from a
/// strategy that traded and lost (analyst § 1.4). The ensemble didn't fail — it
/// never found consensus to act on.
pub const LEADERBOARD_ENSEMBLE_SAT_IN_CASH: &str = "sat in cash \u{2014} consensus never reached";

/// Recommendation headline — buy-and-hold won (`BenchmarkWins`, the honest modal
/// crypto outcome per ADR-0066). `{coin}` is filled at the call site. Frames
/// buy-and-hold as the BASELINE that won, not a failed candidate: no active
/// strategy cleared the robustness bar, so simply holding is the least-bad
/// choice on this window. The operator's honesty rule made literal — "measured
/// robustness, not asserted alpha": if holding is the least-bad, say so. NOT
/// "everything is broken".
pub const LEADERBOARD_HEADLINE_BENCHMARK_WINS: &str = "No active strategy cleared the robustness bar on {coin} \u{2014} simply holding (buy-and-hold) \
     is the least-bad choice on this window.";

/// Recommendation headline — an active strategy won. `{winner}` filled at call.
pub const LEADERBOARD_HEADLINE_ACTIVE_WINS: &str = "{winner} is the best risk-adjusted pick.";

/// Recommendation headline — the residual `AllFragile` (ADR-0066 § D5 row 3):
/// nothing ACTIVE cleared the robustness bar AND holding was not even the best
/// by Sharpe, so the field has no crownable arm. Frames it as the honest "no
/// robust active edge here" conclusion — a ranking + least-bad surface, NOT
/// "everything is hopeless / do nothing". Says ACTIVE (the benchmark is the
/// baseline these are measured against, exempt from the candidate verdict per
/// ADR-0066), never "every strategy".
pub const LEADERBOARD_HEADLINE_ALL_FRAGILE: &str = "No active strategy cleared the robustness bar on this window \u{2014} none held up across \
     resampled price paths.";

/// Supporting reason — crowned on Sharpe among the non-fragile arms.
pub const LEADERBOARD_REASON_HIGHEST_ROBUST_SHARPE: &str =
    "Highest Sharpe among the strategies that held up under resampling.";

/// Supporting reason — winner's Sharpe beat the benchmark's Sharpe.
pub const LEADERBOARD_REASON_BEAT_BENCHMARK_SHARPE: &str =
    "Beat buy-and-hold on risk-adjusted return.";

/// Supporting reason — no active arm beat buy-and-hold (ADR-0066: the benchmark
/// is the baseline, so "active" is exact — buy-and-hold doesn't "beat" itself).
pub const LEADERBOARD_REASON_BENCHMARK_UNDEFEATED: &str =
    "No active strategy beat simply holding the coin.";

/// Supporting reason — the robustness gate found nothing robust among the active
/// arms (ADR-0066: the benchmark is the baseline, exempt from this verdict).
pub const LEADERBOARD_REASON_ALL_FRAGILE: &str =
    "No active strategy stayed positive across resampled price paths.";

/// Supporting reason — a Sharpe tie was resolved by the higher total return.
pub const LEADERBOARD_REASON_TIE_RETURN: &str = "Tie on Sharpe broken by the higher total return.";

/// Supporting reason — a Sharpe + return tie was resolved by lower drawdown.
pub const LEADERBOARD_REASON_TIE_DRAWDOWN: &str =
    "Tie on Sharpe and return broken by the smaller drawdown.";

/// The persistent NOT-ADVICE + simulated-results disclaimer (product § D5).
/// Shown on every recommendation surface — this is a research tool over
/// historical/simulated data, not financial advice.
pub const LEADERBOARD_DISCLAIMER: &str = "Not financial advice. Results are simulated on historical data and do not predict future \
     performance. Past returns are not a guarantee.";

/// Title above the recommendation block.
pub const LEADERBOARD_RECOMMENDATION_TITLE: &str = "Recommendation";

/// Clause appended to the headline when the winner is robust under resampling.
pub const LEADERBOARD_WINNER_ROBUST_CLAUSE: &str = "It held up under resampling.";

/// Clause appended to the headline when the winner is fragile under resampling.
pub const LEADERBOARD_WINNER_FRAGILE_CLAUSE: &str =
    "But it looked fragile under resampling \u{2014} treat with caution.";

// ── advisor-llm-narration F9 — the opt-in "why this one" narration (ADR-0064) ──

/// The opt-in "Explain" control on the crowned recommendation block — the
/// trigger for the LLM "why this one" narration (shown only in the
/// `NotRequested` state). Plain language: not "Generate narration".
pub const LEADERBOARD_EXPLAIN_BUTTON: &str = "Explain in plain language";

/// The in-flight affordance shown next to the templated copy while the
/// narration is being generated (the `InFlight` state). The templated copy
/// stays the floor — this only adds a quiet "working" line.
pub const LEADERBOARD_EXPLAIN_INFLIGHT: &str = "Writing a plain-language summary\u{2026}";

/// The label above the LLM-generated prose (the `Ready` state) — names it as an
/// LLM summary of the numbers ON SCREEN (not new analysis), so the operator
/// always sees the structured result the words describe (ADR-0064 § D7 / R4).
pub const LEADERBOARD_EXPLAIN_LLM_LABEL: &str =
    "Plain-language summary of the result above (AI-generated)";

/// The quiet fallback note shown in the `FellBack` state — the templated copy
/// is already visible (the honest floor); this just explains why no prose
/// appeared, without alarming the operator. Deliberately understated.
pub const LEADERBOARD_EXPLAIN_FELLBACK: &str = "Couldn\u{2019}t generate a plain-language summary \u{2014} the numbers above are the full result.";

// ── advisor-bakeoff-ranking F3 — guided input (coin + budget + lookback) ──────

/// Title of the guided-input panel above the leaderboard table.
pub const LEADERBOARD_PLAN_TITLE: &str = "Plan your bake-off";

/// Label above the coin picker — plain language for "which coin".
pub const LEADERBOARD_COIN_LABEL: &str = "Coin";

/// Label above the budget field.
pub const LEADERBOARD_BUDGET_LABEL: &str = "Budget";

/// Label above the lookback picker.
pub const LEADERBOARD_LOOKBACK_LABEL: &str = "Lookback";

/// Placeholder in the empty budget field — shows the default the run uses.
pub const LEADERBOARD_BUDGET_PLACEHOLDER: &str = "200";

/// Helper under the budget field (F7 / ADR-0065): the honest EUR→USDT FX note.
/// Placeholders: `{eur}` = euro amount (e.g. "200"), `{usdt}` = converted USDT
/// amount (e.g. "216.00"), `{rate}` = EUR/USD rate (e.g. "1.08"),
/// `{source}` = provenance label (e.g. "config").
/// Example: "€200 ≈ $216.00 (at 1.08 EUR/USD, config)"
pub const LEADERBOARD_BUDGET_HINT_FMT: &str =
    "\u{20ac}{eur} \u{2248} ${usdt} (at {rate} EUR/USD, {source})";

/// Budget-context header shown above the leaderboard once a budget is set
/// (R: "ranking strategies for €200 in XRPUSDT"). `{budget}` / `{coin}` are
/// filled at the call site. Carries the budget forward visually even though the
/// ranking itself is budget-independent.
pub const LEADERBOARD_BUDGET_CONTEXT_FMT: &str = "Ranking strategies for {budget} in {coin}.";

/// Budget-context header when the budget field is blank/unparseable — the coin
/// is still named so the header never goes empty. `{coin}` filled at the call
/// site.
pub const LEADERBOARD_CONTEXT_NO_BUDGET_FMT: &str = "Ranking strategies in {coin}.";

/// Field arm-count note shown under the budget-context line
/// (advisor-combination-search OQ-2). Surfaces HOW MANY strategies the bake-off
/// puts head-to-head, so a wider field (now 13 arms: 4 single rule engines + 8
/// vote ensembles + the buy-and-hold benchmark) is self-explanatory — and an
/// operator understands a longer field takes proportionally longer to run.
/// `{count}` is filled at the call site from the real field size (the closed
/// `ui`-side field count — no engine string crosses the seam).
pub const LEADERBOARD_FIELD_ARM_COUNT_FMT: &str =
    "{count} strategies head-to-head: rule engines, vote ensembles, and buy-and-hold.";

// ── Lookback chip labels (F3) — one per `LeaderboardLookback` ──────────────────

/// Lookback chip — ~2 weeks to today.
pub const LEADERBOARD_LOOKBACK_2W: &str = "2 weeks";
/// Lookback chip — ~1 month to today.
pub const LEADERBOARD_LOOKBACK_1M: &str = "1 month";
/// Lookback chip — ~3 months to today.
pub const LEADERBOARD_LOOKBACK_3M: &str = "3 months";
/// Lookback chip — ~6 months to today.
pub const LEADERBOARD_LOOKBACK_6M: &str = "6 months";
/// Lookback chip — ~1 year to today.
pub const LEADERBOARD_LOOKBACK_1Y: &str = "1 year";
/// Lookback chip — ~2 years to today.
pub const LEADERBOARD_LOOKBACK_2Y: &str = "2 years";
/// Lookback chip — ~4 years to today.
pub const LEADERBOARD_LOOKBACK_4Y: &str = "4 years";
/// Lookback chip — fixed preset, first half of 2024.
pub const LEADERBOARD_LOOKBACK_H1_2024: &str = "2024 H1";
/// Lookback chip — fixed preset, second half of 2024.
pub const LEADERBOARD_LOOKBACK_H2_2024: &str = "2024 H2";

// ── advisor-leaderboard-tuning — timeframe + start-capital knobs ─────────────
//
// These labels support the two new "Tune" knobs added to the leaderboard
// guided-input panel. The timeframe knob DOES change ranking (H4/D1 fold
// bars → different signal patterns → different Sharpe rankings). The
// start-capital knob does NOT change ranking (all arms use the same capital,
// so relative KPIs are unchanged); it only scales absolute equity values and
// the forward sizing estimate. Honesty about which knob affects what is a
// HARD requirement (CLAUDE.md — "UI must say so honestly").

/// Field label for the timeframe chip row (bake-off bar size).
pub const LEADERBOARD_TIMEFRAME_LABEL: &str = "Bar size (changes ranking)";

/// Field label for the start-capital text field.
pub const LEADERBOARD_CAPITAL_LABEL: &str = "Start capital (USDT)";

/// Placeholder text for the start-capital field — the legacy default.
pub const LEADERBOARD_CAPITAL_PLACEHOLDER: &str = "100000";

/// Honest hint under the start-capital field — states clearly that ranking
/// is not affected so the operator understands what the knob actually does.
pub const LEADERBOARD_CAPITAL_HINT: &str = "Does not affect ranking — all arms use the same capital. \
     Scales absolute equity values and forward sizing.";

// ── advisor-param-tuning (ADR-0069) — the gate-tied hyperparameter sweep ──────
//
// The Tune editor lets the operator sweep a strategy family's parameter grid and
// see each config's ROBUSTNESS VERDICT through the same frozen gate the bake-off
// uses. The honest framing is load-bearing: a FRAGILE config is OVERFIT (it won
// fit to noise that resampling dissolves) and cannot be promoted. Every line
// below is written so the anti-overfitting point is unmissable.

/// Sidebar/route label for the Tune editor (navigable, not sidebar-default).
pub const TUNE_SIDEBAR_LABEL: &str = "Tune";

/// Screen headline — plain language for "what this is".
pub const TUNE_HEADLINE: &str = "Tune parameters";

/// Caption under the headline — frames the gate-tied sweep + the honest point.
pub const TUNE_CAPTION: &str = "Sweep a strategy's parameters and see how each config holds up under \
     resampling. A config that wins in-sample but is flagged fragile is overfit — it cannot be \
     promoted.";

/// The "Run sweep" action button — the right default action.
pub const TUNE_RUN_BUTTON: &str = "Run sweep";

/// Run button label while a sweep is in flight (legible beyond colour).
pub const TUNE_RUN_BUTTON_RUNNING: &str = "Running\u{2026}";

/// Fixtures / no-`live` build: the friendly "needs the live build" error.
pub const TUNE_RUN_NEEDS_LIVE: &str = "Running a sweep needs the live build. Launch the cockpit with \
     `cargo run -p ui --bin cockpit_live` to sweep parameters on real data.";

/// The "Tune…" drill-down affordance on a leaderboard row (opens the editor for
/// that row's family). `{family}` is filled at the call site.
pub const TUNE_OPEN_AFFORDANCE: &str = "Tune\u{2026}";

/// Cold-start Empty prompt — what the operator should do next (no blank screen).
pub const TUNE_EMPTY_PROMPT: &str = "Set the parameter ranges above and press \u{201c}Run sweep\u{201d} to score each config \
     through the robustness gate.";

/// Loading copy while a sweep runs.
pub const TUNE_LOADING: &str = "Running the sweep\u{2026}";

/// Error-pane prefix — followed by the engine's detail (never a bare "no data").
pub const TUNE_ERROR_PREFIX: &str = "The sweep could not run.";

/// Title of the range-form panel.
pub const TUNE_FORM_TITLE: &str = "Choose a parameter grid";

/// Label above the family picker.
pub const TUNE_FAMILY_LABEL: &str = "Strategy family";

/// Family chip labels.
pub const TUNE_FAMILY_SMA: &str = "SMA crossover";
/// MACD family chip label.
pub const TUNE_FAMILY_MACD: &str = "MACD";
/// RSI family chip label.
pub const TUNE_FAMILY_RSI: &str = "RSI";
/// Bollinger family chip label.
pub const TUNE_FAMILY_BOLLINGER: &str = "Bollinger bands";

/// Axis labels (SMA fast / slow window).
pub const TUNE_AXIS_FAST_LABEL: &str = "Fast window (shipped 20)";
/// SMA slow-window axis label.
pub const TUNE_AXIS_SLOW_LABEL: &str = "Slow window (shipped 50)";

/// MACD axis labels (fast / slow / signal period) — shipped 12 / 26 / 9.
pub const TUNE_AXIS_MACD_FAST_LABEL: &str = "Fast period (shipped 12)";
/// MACD slow-period axis label.
pub const TUNE_AXIS_MACD_SLOW_LABEL: &str = "Slow period (shipped 26)";
/// MACD signal-period axis label.
pub const TUNE_AXIS_MACD_SIGNAL_LABEL: &str = "Signal period (shipped 9)";

/// RSI axis labels (lookback period / oversold threshold) — shipped 14 / 30.
pub const TUNE_AXIS_RSI_PERIOD_LABEL: &str = "Period (shipped 14)";
/// RSI oversold-threshold axis label.
pub const TUNE_AXIS_RSI_OVERSOLD_LABEL: &str = "Oversold threshold (shipped 30)";

/// Bollinger lookback-period axis label — shipped 20.
pub const TUNE_AXIS_BBANDS_PERIOD_LABEL: &str = "Period (shipped 20)";
/// Bollinger `k` band-multiplier multi-select label — shipped 2.0. Plain
/// language: "Band width (k)" so the operator isn't left guessing what k is.
pub const TUNE_AXIS_BBANDS_K_LABEL: &str = "Band width \u{00d7} (k, shipped 2.0)";

/// `{min, max, step}` field captions.
pub const TUNE_AXIS_MIN: &str = "min";
/// Axis max-field caption.
pub const TUNE_AXIS_MAX: &str = "max";
/// Axis step-field caption.
pub const TUNE_AXIS_STEP: &str = "step";

/// Preset chip labels (narrow / shipped / wide one-click ranges).
pub const TUNE_PRESET_NARROW: &str = "narrow";
/// Shipped-default preset chip label.
pub const TUNE_PRESET_SHIPPED: &str = "shipped";
/// Wide preset chip label.
pub const TUNE_PRESET_WIDE: &str = "wide";

/// Live grid-size readout — "{n} configs → ~{runs} bootstrap runs". `{n}` is the
/// runnable cell count; `{runs}` ≈ n × 1000 paths. Filled at the call site.
pub const TUNE_GRID_READOUT_FMT: &str = "{n} configs \u{2192} ~{runs} bootstrap runs";

/// Grid readout when the grid is empty (min>max / fast≥slow everywhere).
pub const TUNE_GRID_READOUT_EMPTY: &str =
    "No valid configs — widen the ranges (fast must be less than slow).";

/// Grid readout when a field is blank — prompt to fill it rather than run.
pub const TUNE_GRID_READOUT_BLANK: &str = "Fill every min / max / step field to run the sweep.";

/// Honest truncation banner — the grid exceeded the cap. `{shown}` = the cap
/// (configs run), `{requested}` = the full valid count. Filled at the call site.
pub const TUNE_TRUNCATION_FMT: &str = "Showing {shown} of {requested} configs — narrow your \
     ranges or increase the step to see the rest. Sweeps are capped to keep each run interactive.";

/// Determinate progress copy — "Scoring {current} — {n} of {total}".
pub const TUNE_PROGRESS_FMT: &str = "Scoring {current} \u{2014} {n} of {total}";

/// Result-grid column headers.
pub const TUNE_COL_CONFIG: &str = "Config";
/// Verdict column header.
pub const TUNE_COL_VERDICT: &str = "Verdict";
/// In-sample return column header.
pub const TUNE_COL_RETURN: &str = "Return";
/// Sharpe-spread column header (p5 / p50 / p95).
pub const TUNE_COL_SHARPE_SPREAD: &str = "Sharpe p5\u{2009}/\u{2009}p50\u{2009}/\u{2009}p95";
/// P(loss) column header.
pub const TUNE_COL_PROB_LOSS: &str = "P(loss)";
/// P(Sharpe>1) column header.
pub const TUNE_COL_PROB_SHARPE: &str = "P(Sharpe>1)";
/// Max-DD p95 column header.
pub const TUNE_COL_MAXDD_P95: &str = "Max-DD p95";
/// "Use this config" action column header.
pub const TUNE_COL_USE: &str = "Promote";

/// Verdict pill labels (word always present — colour is never the only signal).
pub const TUNE_VERDICT_ROBUST: &str = "robust";
/// Marginal verdict pill label.
pub const TUNE_VERDICT_MARGINAL: &str = "marginal";
/// Fragile verdict pill label (the promotion-blocking state).
pub const TUNE_VERDICT_FRAGILE: &str = "fragile";

/// Tag on the shipped-config baseline row.
pub const TUNE_BASELINE_TAG: &str = "shipped";

/// The enabled "Use this config" affordance label (non-fragile rows).
pub const TUNE_USE_CONFIG: &str = "Use this config";

/// The disabled "Use this config" affordance label (fragile rows — greyed).
pub const TUNE_USE_CONFIG_FRAGILE: &str = "Fragile — locked";

/// Inline note on a fragile row explaining why promotion is disabled.
pub const TUNE_FRAGILE_PROMOTE_NOTE: &str =
    "Fragile under resampling — promoting it would be overfitting.";

/// Caption under the result grid — the distribution is what the gate judges.
pub const TUNE_DISTRIBUTION_CAPTION: &str = "Return and Sharpe p50 are in-sample point estimates. \
     The p5\u{2013}p95 spread is what the gate judges — a config with a gaudy return but a \
     negative p5 Sharpe loses money in the tail, so it is fragile.";

/// Buy-and-hold benchmark header strip — "vs just holding {coin}: {return} return, \
/// Sharpe {sharpe}". Filled at the call site.
pub const TUNE_BENCHMARK_STRIP_FMT: &str =
    "vs just holding {coin}: {return} return, Sharpe {sharpe}.";

/// The persistent, non-dismissible honesty footer (ADR-0069 § 7). `{coin}` is
/// filled at the call site.
pub const TUNE_DISCLAIMER: &str = "Tuning is paper/sim research, not advice. A config that looks \
     great in-sample but is flagged fragile is overfit — it won fit to noise that resampling \
     dissolves. The bake-off already searches sensible defaults; a tuned config is only worth \
     carrying forward if it is robust AND beats just holding {coin}.";

// ── advisor-param-promotion (ADR-0070 § D6) — promotion provenance framing ─────

/// The promote-provenance header on the forward-plan screen when the active plan
/// came from a PROMOTION (vs a crowned bake-off pick). Distinct from the crowned
/// "best of the bake-off" provenance — this is the ONLY live promote-vs-crown
/// signal (ADR-0070 § R5). `{family}` / `{params}` / `{window}` are filled at the
/// call site from `PromoteParams.label()` + the sweep window. Honest framing: a
/// config robust on ONE window is NOT a guarantee, and not advice.
pub const TUNE_PROMOTE_CONFIRM_FMT: &str = "You tuned this {family} config ({params}). It survived \
     resampling on {window} — that is not a guarantee, and not advice. Paper-trading your \u{20ac}200.";

/// Defensive fallback window label for the promotion honesty copy when no sweep
/// result is on screen (promotion is only reachable from a `Ready` grid row, so
/// this is never expected — but the copy must never read "on {window}" literally).
pub const TUNE_PROMOTE_WINDOW_FALLBACK: &str = "the tuned window";

// ── advisor-forward-plan v0.1.0 (roadmap F6) — the forward buy/sell plan ───────
//
// The plan is a CONDITIONAL, REACTIVE, rule-driven decision plan — NOT a price
// forecast (ADR-0062 / OQ-D). Every line below is written to make the
// conditional ("IF … THEN …"), reactive ("re-checked each bar"), and
// not-a-prediction nature unmistakable. The disclaimers are INTEGRAL copy, not
// fine-print: the not-a-prediction framing leads the surface, and the
// not-financial-advice + simulated-budget line is always visible.

/// Sidebar nav label for the forward-plan screen.
pub const FORWARD_PLAN_SIDEBAR_LABEL: &str = "Plan";

/// Page headline — names the screen's job in plain language. "Forward plan",
/// not "forecast" — deliberately avoids any prediction connotation.
pub const FORWARD_PLAN_HEADLINE: &str = "Forward plan";

/// Page caption — the one-line "what this is", framed as conditional rules,
/// NOT a prediction. This is the first not-a-prediction signal on the surface.
pub const FORWARD_PLAN_CAPTION: &str = "What the crowned strategy will do as new bars arrive \u{2014} the standing buy/sell rules, \
     not a forecast of price. The same rules your simulated \u{20ac}200 paper-trade runs.";

/// Empty-state prompt — no crowned pick yet → no plan (the clean tautology
/// guard). Tells the operator exactly what to do next (never a blank screen).
pub const FORWARD_PLAN_EMPTY_PROMPT: &str = "No plan yet. Run a bake-off and crown a strategy first \u{2014} the plan shows what that \
     strategy will do over the coming days.";

/// Loading copy — shown beside the spinner while the agent resolves the plan
/// from the crowned selection at the launch boundary.
pub const FORWARD_PLAN_LOADING: &str =
    "Reading the crowned strategy\u{2019}s standing rules\u{2026}";

/// Error-state prefix — paired with the failure detail (never a bare "no
/// data"; says what to check).
pub const FORWARD_PLAN_ERROR_PREFIX: &str = "The plan could not be built";

// ── Stance badge (R1 — current stance, dated to the latest bar) ────────────────

/// Section label above the current-stance badge.
pub const FORWARD_PLAN_STANCE_TITLE: &str = "Right now";

/// Stance badge — the strategy holds no position (waiting for an entry).
pub const FORWARD_PLAN_STANCE_FLAT: &str = "Flat \u{2014} no position";

/// Stance badge — the strategy is holding a position (watching for an exit).
pub const FORWARD_PLAN_STANCE_LONG: &str = "Long \u{2014} holding";

/// "As of" line under the stance badge — the honest-staleness stamp. `{close}`
/// (last close) + `{as_of}` (bar date/time) filled at the call site. Makes the
/// stance explicitly a snapshot of the last bar, not a live or future claim.
pub const FORWARD_PLAN_AS_OF_FMT: &str = "As of the last close {close} ({as_of}).";

/// Latest-signal sub-line — the most recent BUY/SELL/HOLD reading, shown so the
/// operator sees what the last bar did. `{signal}` filled at the call site.
pub const FORWARD_PLAN_LATEST_SIGNAL_FMT: &str = "Latest signal on that bar: {signal}.";

/// Signal word — the latest bar fired a BUY.
pub const FORWARD_PLAN_SIGNAL_BUY: &str = "buy";
/// Signal word — the latest bar fired a SELL.
pub const FORWARD_PLAN_SIGNAL_SELL: &str = "sell";
/// Signal word — the latest bar fired no action (conditions unmet).
pub const FORWARD_PLAN_SIGNAL_HOLD: &str = "hold (no action)";

// ── Standing rules (R2 — the IF/THEN entry/exit conditions) ────────────────────

/// Section label above the standing IF/THEN rules.
pub const FORWARD_PLAN_RULES_TITLE: &str = "Standing rules";

/// The "IF" lead-in word for an entry rule line (rendered as a labelled
/// condition, deliberately NOT a timeline — the conditional framing, OQ-D).
pub const FORWARD_PLAN_RULE_IF: &str = "IF";

/// The "THEN" lead-in word for the action half of a rule line.
pub const FORWARD_PLAN_RULE_THEN: &str = "THEN";

/// SMA entry rule — IF condition. `{fast}` / `{slow}` filled at the call site.
pub const FORWARD_PLAN_RULE_SMA_ENTRY_IF_FMT: &str =
    "the {fast}-bar average crosses above the {slow}-bar average";
/// SMA entry rule — THEN action.
pub const FORWARD_PLAN_RULE_SMA_ENTRY_THEN: &str = "buy (open a position)";
/// SMA exit rule — IF condition. `{fast}` / `{slow}` filled at the call site.
pub const FORWARD_PLAN_RULE_SMA_EXIT_IF_FMT: &str =
    "the {fast}-bar average crosses back below the {slow}-bar average";
/// SMA exit rule — THEN action.
pub const FORWARD_PLAN_RULE_SMA_EXIT_THEN: &str = "sell (close the position)";

// ── Short-capable forward-plan rules (advisor-short-selling, ADR-0068 § D8) ────
//
// For a crowned SHORT-CAPABLE arm (`*_ls` / `always_short`), the forward plan
// describes the down-half rules honestly via the existing IF/THEN plan-render
// path: it SELLS-TO-OPEN a short on the bearish flip (instead of sitting flat),
// COVERS (buys to close) on the bullish flip, and is FORCE-LIQUIDATED if the
// loss reaches the maintenance-margin floor. Plain-language, not-a-prediction /
// not-advice framing already leads the surface; these add the short half +
// the unbounded-loss caution. Paper/sim only — no real orders, no real margin.

/// Short-rules section sub-heading — frames the down-half rules as the
/// directional (long/short) extension, so the operator sees they can bet on a
/// decline, not just go flat.
pub const FORWARD_PLAN_SHORT_RULES_HEADING: &str = "This strategy can also bet on a decline:";

/// Short entry rule — THEN action. The bearish-flip "open a short" half (the
/// IF condition reuses the rule family's own exit/bearish copy).
pub const FORWARD_PLAN_RULE_SHORT_OPEN_THEN: &str = "sell-to-open a short (bet on a decline)";

/// Short entry rule — generic IF condition (the bearish flip), used for rule
/// families without a parameterised bearish clause of their own (MACD / RSI /
/// Bollinger). SMA reuses its own parameterised exit copy instead.
pub const FORWARD_PLAN_RULE_SHORT_OPEN_IF_GENERIC: &str =
    "the trend turns bearish (the entry condition reverses to the downside)";

/// Cover rule — IF condition. The bullish flip that closes an open short.
pub const FORWARD_PLAN_RULE_SHORT_COVER_IF: &str =
    "the trend flips back up (the entry condition reverses)";
/// Cover rule — THEN action.
pub const FORWARD_PLAN_RULE_SHORT_COVER_THEN: &str = "buy-to-cover (close the short)";

/// Liquidation rule line — the honest maintenance-margin force-cover. Not an
/// IF/THEN choice the strategy makes; a risk floor that closes the short for
/// you when the loss is severe. Names the unbounded loss plainly.
pub const FORWARD_PLAN_RULE_SHORT_LIQUIDATION: &str = "If the loss reaches the maintenance-margin floor the short is force-liquidated \u{2014} the loss \
     is not capped at your \u{20ac}200.";

/// The always-short benchmark's standing rule — the down-side mirror of
/// buy-and-hold. It opens a short and holds it the whole horizon (no cover
/// trigger), so it loses on any up-trending window by construction. The honest
/// control, rendered as obviously the same KIND of object as buy-and-hold.
pub const FORWARD_PLAN_RULE_ALWAYS_SHORT: &str = "Open a short now and hold it the whole horizon \u{2014} the down-side mirror of buy-and-hold. \
     There is no cover trigger; it loses on any up-trend by construction.";

/// MACD trend entry rule — IF condition. `{fast}`/`{slow}`/`{signal}`.
///
/// The primary signal is a positive MACD histogram — this is the headline
/// indicator shown here.  The strategy also applies a trend filter (price
/// above EMA(200)); see `FORWARD_PLAN_RULE_COMPOUND_CAVEAT`.
pub const FORWARD_PLAN_RULE_MACD_ENTRY_IF_FMT: &str =
    "the MACD ({fast}/{slow}/{signal}) histogram is positive";
/// MACD entry rule — THEN action.
pub const FORWARD_PLAN_RULE_MACD_ENTRY_THEN: &str = "buy (open a position)";
/// MACD exit rule — IF condition.  The exit fires when the entry compound
/// condition (MACD histogram positive AND price above EMA(200)) flips false.
pub const FORWARD_PLAN_RULE_MACD_EXIT_IF: &str =
    "the MACD histogram turns negative or price falls below the 200-bar EMA";
/// MACD exit rule — THEN action.
pub const FORWARD_PLAN_RULE_MACD_EXIT_THEN: &str = "sell (close the position)";

/// RSI reversion entry rule — IF condition. `{len}`/`{lower}`.
///
/// The primary signal is RSI falling below the oversold threshold — this is
/// the headline indicator.  The strategy also requires the close to be above
/// the recent support floor; see `FORWARD_PLAN_RULE_COMPOUND_CAVEAT`.
pub const FORWARD_PLAN_RULE_RSI_ENTRY_IF_FMT: &str =
    "the {len}-bar RSI falls below {lower} (oversold)";
/// RSI entry rule — THEN action.
pub const FORWARD_PLAN_RULE_RSI_ENTRY_THEN: &str = "buy (open a position)";
/// RSI exit rule — IF condition. `{lower}`.
///
/// This is a flip-to-false exit: the strategy exits when the RSI climbs back
/// above the same oversold threshold (`lower`) and the entry condition clears.
/// There is NO overbought threshold (no RSI-70 or similar) in this strategy.
pub const FORWARD_PLAN_RULE_RSI_EXIT_IF_FMT: &str =
    "the RSI climbs back above {lower} (the oversold condition clears)";
/// RSI exit rule — THEN action.
pub const FORWARD_PLAN_RULE_RSI_EXIT_THEN: &str = "sell (close the position)";

/// Bollinger reversion entry rule — IF condition. `{len}`/`{k}`.
///
/// The primary signal is price closing below the lower band — this is the
/// headline indicator.  The strategy also requires a volume surge to confirm
/// the move; see `FORWARD_PLAN_RULE_COMPOUND_CAVEAT`.
pub const FORWARD_PLAN_RULE_BBANDS_ENTRY_IF_FMT: &str =
    "price closes below the lower band ({len}-bar, {k}\u{03c3})";
/// Bollinger entry rule — THEN action.
pub const FORWARD_PLAN_RULE_BBANDS_ENTRY_THEN: &str = "buy (open a position)";
/// Bollinger exit rule — IF condition.
///
/// This is a flip-to-false exit: the strategy exits when price closes back
/// inside the band and the entry condition clears.  It is NOT a reverse
/// upper-band cross.
pub const FORWARD_PLAN_RULE_BBANDS_EXIT_IF: &str =
    "price closes back inside the band (the lower-band condition clears)";
/// Bollinger exit rule — THEN action.
pub const FORWARD_PLAN_RULE_BBANDS_EXIT_THEN: &str = "sell (close the position)";

/// Compound-condition caveat — shown below the IF/THEN rules for composed
/// strategies (MACD, RSI, `BBands`).  Makes the simplification honest: each
/// strategy's entry is a compound AND; only the primary indicator is shown
/// above.
pub const FORWARD_PLAN_RULE_COMPOUND_CAVEAT: &str = "This shows the strategy\u{2019}s primary signal. It may also apply \
     additional trend or volume confirmation before acting.";

/// Buy-and-hold degenerate plan — the single standing "rule" (D5). No IF/THEN,
/// no sell trigger, no re-evaluation: the honest degenerate case stated plainly.
pub const FORWARD_PLAN_RULE_BUY_AND_HOLD: &str = "Buy once now and hold the whole horizon. There is no sell trigger and no \
     re-evaluation \u{2014} nothing beat simply holding over the tested window.";

/// Cadence line under the rules — restates that the rules are re-checked every
/// bar (the reactive framing), NOT a dated schedule. Shown for active rules
/// only (buy-and-hold has no re-evaluation). `{horizon}` filled at the call site.
pub const FORWARD_PLAN_CADENCE_FMT: &str = "These rules stay in force and are re-checked on every new bar for the next {horizon} days \u{2014} \
     this is not a day-by-day schedule.";

// ── Ensemble (signal-vote) plan copy (F8 / ADR-0063 § D3) ─────────────────────
//
// The ensemble plan describes the VOTE faithfully — method + members + live
// tally — NOT a fabricated single-indicator rule. The `ui` owns every word; the
// engine crosses only the structured `method` + `member_count` (no `String`).
//
// **v0.2 extension point (member-rule list):** the developer's shipped
// `agent::config::PlanRuleKind::Ensemble` carries only `method` + a scalar
// `member_count` (NOT each member's own `PlanRuleShape`), so this plan
// describes the vote at the consensus level. A per-member rule list ("MACD
// trend — buys when …") becomes possible once the agent boundary carries the
// members; its copy (a members-title + per-member summaries) would be added
// here at that point.

/// Ensemble headline rule line — MAJORITY vote. Reads the structured method as
/// "Holds while at least {k} of {n} agree; goes flat when the majority flips."
/// `{k}`/`{n}` filled at the call site from the structured `PlanVoteMethodView`.
pub const FORWARD_PLAN_RULE_ENSEMBLE_MAJORITY_FMT: &str = "Holds a position while at least {k} of {n} member strategies agree to be in the \
     market; goes flat when the agreement drops below {k}.";

/// Ensemble headline rule line — UNANIMOUS vote. `{n}` filled at the call site.
pub const FORWARD_PLAN_RULE_ENSEMBLE_UNANIMOUS_FMT: &str = "Holds a position only while ALL {n} member strategies agree to be in the \
     market; goes flat the moment any one of them disagrees.";

/// The live-tally line under an ensemble's rule — shows how many members are
/// currently in the market against the quorum, and the resulting stance.
/// `{long}`/`{n}`/`{stance}` filled at the call site (the `ui` owns the words).
pub const FORWARD_PLAN_RULE_ENSEMBLE_TALLY_FMT: &str =
    "Current vote: {long} of {n} member strategies are in the market \u{2192} {stance}.";

/// Honest caveat under the ensemble rule — names the vote as a combination, not
/// a new signal source, and restates the not-better framing (R4 non-goal).
pub const FORWARD_PLAN_RULE_ENSEMBLE_CAVEAT: &str = "This is a vote over the member strategies \u{2014} it is measured against \
     buy-and-hold like every other candidate, with no assumption that combining \
     them does better.";

// ── Ensemble MEMBER NAMING (F6 member-name enrichment) ────────────────────────
//
// The agent boundary now carries each ensemble member's human-readable DISPLAY
// LABEL (`members: Vec<SmolStr>`, e.g. ["MACD trend", "RSI reversion", "Bollinger
// reversion"]), so the plan can NAME the members instead of saying "3 member
// strategies" abstractly. The headline vote rule becomes "Holds while ≥ 2 of
// {MACD trend, RSI reversion, Bollinger reversion} agree…", with the member set
// rendered as a brace-list from the structured labels. The labels themselves come
// from the agent (sourced from `strategy::EnsembleStrategy::describe_plan`); only
// the SURROUNDING rule prose lives here (the `ui` owns the connective words).

/// Ensemble headline rule line — MAJORITY vote, with the members NAMED. Reads
/// "Holds while at least {k} of {members} agree to be in the market; goes flat
/// when the agreement drops below {k}." `{k}` + `{members}` (a brace-list like
/// "{MACD trend, RSI reversion, Bollinger reversion}", built from the structured
/// member labels) are filled at the call site.
pub const FORWARD_PLAN_RULE_ENSEMBLE_MAJORITY_NAMED_FMT: &str = "Holds a position while at least {k} of {members} agree to be in the market; \
     goes flat when the agreement drops below {k}.";

/// Ensemble headline rule line — UNANIMOUS vote, with the members NAMED. Reads
/// "Holds while ALL of {members} agree…; goes flat the moment any one of them
/// disagrees." `{members}` filled at the call site from the member labels.
pub const FORWARD_PLAN_RULE_ENSEMBLE_UNANIMOUS_NAMED_FMT: &str = "Holds a position only while ALL of {members} agree to be in the market; \
     goes flat the moment any one of them disagrees.";

// ── Projected sizing (R3 — budget-aware €200 next-BUY, "at the last close") ────

/// Section label above the projected-sizing line.
pub const FORWARD_PLAN_SIZING_TITLE: &str = "If it buys next";

/// Projected next-BUY sizing for an active strategy currently FLAT. `{units}`
/// (projected units) + `{close}` (last close) filled at the call site. Labelled
/// "at the last close" (an estimate at the last price), NOT "you will buy at".
pub const FORWARD_PLAN_SIZING_FLAT_FMT: &str = "On the next buy it would deploy about {units} units at the last close {close}. The actual \
     fill price will be the next bar\u{2019}s \u{2014} this is an estimate at the last close, not a promised fill.";

/// Projected held sizing for an active strategy currently LONG. `{units}` +
/// `{close}` filled at the call site.
pub const FORWARD_PLAN_SIZING_LONG_FMT: &str = "It is already holding about {units} units (bought near {close}); the standing exit rule \
     above is what closes it.";

/// Buy-and-hold sizing — deploy the FULL €200 now (D5). `{units}` + `{close}`.
pub const FORWARD_PLAN_SIZING_BUY_AND_HOLD_FMT: &str = "Deploy the full \u{20ac}200 now \u{2014} about {units} units at the last close {close} \u{2014} \
     and hold for the horizon.";

/// Budget line — the honest EUR→USDT conversion + the hard cap (F7 / ADR-0065).
/// Always shown so the operator sees the budget framing and the cap.
/// Placeholders: `{eur}` = euro amount, `{usdt}` = converted USDT,
/// `{rate}` = rate, `{source}` = provenance.
/// The hard-cap clause ("It never deploys more than …") is preserved verbatim (R3).
/// Example: "€200 ≈ $216.00 (at 1.08 EUR/USD, config). It never deploys more than €200 — a hard cap."
pub const FORWARD_PLAN_BUDGET_LINE_FMT: &str = "\u{20ac}{eur} \u{2248} ${usdt} (at {rate} EUR/USD, {source}). It never deploys more than \u{20ac}{eur} \u{2014} a hard cap.";

/// Appended when the F4 budget cap actually bound the projected units — so the
/// operator knows the cap bit (shown only when `sizing_capped`).
pub const FORWARD_PLAN_SIZING_CAPPED_NOTE: &str = "The \u{20ac}200 cap limited this size.";

// ── Horizon (R4 — "planned through <date>") ────────────────────────────────────

/// Section label above the horizon framing.
pub const FORWARD_PLAN_HORIZON_TITLE: &str = "Horizon";

/// Horizon framing line — "planned through <date>", restating that the horizon
/// is the window the rules are in force (the planning frame), not a forecast
/// window. `{days}` (horizon days) + `{through}` (the through-date) filled at
/// the call site.
pub const FORWARD_PLAN_HORIZON_FMT: &str = "Planned through {through} \u{2014} the next {days} days. That is how long these rules are in \
     force; it is not a prediction of where the price will be.";

// ── Disclaimers (R6 — integral, not fine-print) ───────────────────────────────

/// The not-a-prediction framing — the central honesty line (OQ-D). Rendered
/// prominently near the top of the plan (integral to the layout), NOT a
/// footnote: the plan is conditional rules, not a forecast or an implied return.
pub const FORWARD_PLAN_NOT_A_PREDICTION: &str = "This is a conditional, rule-based plan \u{2014} not a price prediction, and not an implied or \
     expected return. It only describes what the strategy will do when its conditions are met.";

/// The standing not-financial-advice + simulated-budget disclaimer (product
/// § D5). Always present at the foot of the plan surface.
pub const FORWARD_PLAN_DISCLAIMER: &str = "Not financial advice. The \u{20ac}200 is a simulated paper budget on historical/live data \u{2014} \
     no real orders are placed. Past behaviour does not guarantee future results.";

// ── Phase C — Live / Strategy registry / Settings ────────────────────────────
// ui-rethink-phase-c-sidebar-ia T-D-N05

/// Headline / page title for the Live screen (R7.2).
pub const LIVE_HEADLINE: &str = "Live";

/// Section label for the system-health strip at the top of the Live screen.
pub const LIVE_SYSTEM_HEALTH_LABEL: &str = "System health";

/// KPI strip label for the LLM daily spend tile (R7.2).
pub const LIVE_LLM_SPEND_LABEL: &str = "LLM spend";

/// Placeholder value for the LLM daily spend tile when the budget tracker
/// is not yet wired (Q4b — Phase F wires the real source).
pub const LIVE_LLM_SPEND_PLACEHOLDER: &str = "\u{2014}";

/// cockpit-live-dashboard-wiring v0.1.0 (R5 / AC5) — honest scope caption for
/// the Live KPI strip's Total-return card. The live figure is
/// **session-to-date** (the first accumulated equity point is the session
/// open), NOT an annualized / multi-year / characterized result. MUST NOT
/// imply the live session is the "baseline result".
pub const LIVE_SESSION_RETURN_CAPTION: &str = "Session to date";

/// live-equity-history-durable v0.1.0 (R6 / D5) — honest scope caption for the
/// Live KPI strip's Total-return card when a **durable paper/live history** has
/// been hydrated on boot (`live_equity_hydrated`). The figure is measured from
/// the **first persisted point (account inception)** and may span multiple
/// sessions / days — so the scope is "Since inception", NOT "session to date".
/// Still honest: it is a continuous real paper/live equity history, NOT an
/// annualized / characterized / baseline result. Research mode never hydrates,
/// so it keeps `LIVE_SESSION_RETURN_CAPTION`.
pub const LIVE_SINCE_INCEPTION_CAPTION: &str = "Since inception";

/// Panel title for the Strategy registry screen (R7.2).
pub const STRATEGY_REGISTRY_PANEL_TITLE: &str = "Strategy registry";

/// Empty-state copy for the Strategy registry screen (R3.6 / K3 mitigation).
pub const STRATEGY_REGISTRY_EMPTY: &str =
    "No strategies registered. Run a backtest in Lab to register one.";

/// Label on the primary action button of each strategy card (R3.4).
pub const STRATEGY_REGISTRY_OPEN_IN_LAB_LABEL: &str = "Open in Lab";

/// Status pill copy for a shipped strategy (A6 — uniform at Phase C).
pub const STRATEGY_REGISTRY_STATUS_SHIPPED: &str = "shipped";

/// Status pill copy for a candidate strategy (unused at Phase C; ready for Phase D).
pub const STRATEGY_REGISTRY_STATUS_CANDIDATE: &str = "candidate";

/// Status pill copy for an archived strategy (unused at Phase C; ready for Phase D).
pub const STRATEGY_REGISTRY_STATUS_ARCHIVED: &str = "archived";

/// Prefix for the last backtest anchor line on a strategy card.
pub const STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX: &str = "Anchor: ";

/// Prefix for the last live-run timestamp line on a strategy card.
pub const STRATEGY_REGISTRY_LAST_RUN_PREFIX: &str = "Last run: ";

/// Prefix for the universe (symbols list) line on a strategy card.
pub const STRATEGY_REGISTRY_UNIVERSE_PREFIX: &str = "Universe: ";

/// Tab label for the Risk sub-tab inside the Settings rollup (Q2a).
pub const SETTINGS_TAB_RISK: &str = "Risk";

/// Tab label for the Control sub-tab inside the Settings rollup (Q2a).
pub const SETTINGS_TAB_CONTROL: &str = "Control";

/// Tab label for the Debug sub-tab inside the Settings rollup (Q2a).
pub const SETTINGS_TAB_DEBUG: &str = "Debug";

// ── F5 — Forward paper-trade P/L framing (ADR-0060 § D5) ────────────────────

/// Label for the forward-budget P/L card headline.
/// Shown when a forward run is active (`forward_budget = Some`).
pub const LIVE_FORWARD_PNL_LABEL: &str = "P/L";

/// Label for the budget card — the starting capital of the forward run.
pub const LIVE_FORWARD_BUDGET_LABEL: &str = "Budget";

/// FX note under the budget card — the honest EUR→USDT conversion (F7 / ADR-0065).
/// Unicode: € = U+20AC, ≈ = U+2248.
/// Placeholders: `{eur}` = euro amount, `{usdt}` = converted USDT,
/// `{rate}` = EUR/USD rate, `{source}` = provenance label.
/// Example: "€200 ≈ $216.00 (at 1.08 EUR/USD, config)"
pub const LIVE_FORWARD_FX_NOTE_FMT: &str =
    "\u{20ac}{eur} \u{2248} ${usdt} (at {rate} EUR/USD, {source})";

/// Persistent not-advice + simulated-budget disclaimer (product § D5).
pub const LIVE_FORWARD_DISCLAIMER: &str = "Simulated paper budget. Not financial advice. \
     This is not a real trade.";

/// Caption shown above the P/L row in the Live screen when a forward run
/// is active. Carries the strategy id; caller fills `{strategy}`.
pub const LIVE_FORWARD_RUNNING_FMT: &str = "Running {strategy} on simulated budget.";

#[cfg(test)]
mod tests {
    use super::*;

    /// Every key in `all()` is unique.
    #[test]
    fn all_keys_unique() {
        let mut seen = std::collections::HashSet::new();
        for (k, _) in all() {
            assert!(seen.insert(*k), "duplicate key: {k}");
        }
    }

    /// No value is empty.
    #[test]
    fn all_values_non_empty() {
        for (k, v) in all() {
            assert!(!v.is_empty(), "empty value for key: {k}");
        }
    }
}
