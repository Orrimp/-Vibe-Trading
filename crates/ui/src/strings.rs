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
pub const TAPE_COL_FEE: &str = "Fee";
pub const TAPE_PAUSE_LABEL: &str = "Pause";
pub const TAPE_RESUME_LABEL: &str = "Resume";
pub const TAPE_LOADING: &str = "Connecting to the fill stream…";
pub const TAPE_EMPTY: &str = "No fills yet. Waiting for the first bar from BTCUSDT.";
pub const TAPE_ERROR_PREFIX: &str = "Can't read the fill stream: ";
pub const TAPE_PAUSED_BANNER: &str = "Paused — updates buffered";

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
/// Placeholder body copy for the `Compare` screen (Phase E body).
pub const COMPARE_PLACEHOLDER: &str = "Compare view — coming in Phase E.";
/// Placeholder body copy for the `Memory` screen (Phase F body).
pub const MEMORY_PLACEHOLDER: &str = "Memory view — coming in Phase F.";
/// Placeholder body copy for the `Models` screen (Phase F body).
pub const MODELS_PLACEHOLDER: &str = "Models view — coming in Phase F.";
/// Placeholder body copy for the `Settings` screen (Phase C body).
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

/// Label on the Run backtest button in the Lab screen (T-D-14).
pub const LAB_RUN_BUTTON: &str = "Run";

/// Label shown on the Run button while a backtest is in-flight (T-D-14).
pub const LAB_RUN_BUTTON_RUNNING: &str = "Running\u{2026}";

/// Label on the Run button after a successful run — operator can re-run (T-D-14b).
pub const LAB_RUN_BUTTON_COMPLETED: &str = "Re-run";

/// Label on the Run button after a failed run — operator can retry (T-D-14b).
pub const LAB_RUN_BUTTON_FAILED: &str = "Retry";

// ── Run delta badge — ui-rethink-phase-b-lab-run T-D-N13 ─────────────────────

/// Short label for the `PnL` delta column of the `run_delta_badge` (R8.2 / D5).
/// Shows the change in total return between the last two runs on the same tuple.
pub const RUN_DELTA_BADGE_PNL_LABEL: &str = "P&L";

/// Short label for the max-drawdown delta column of the `run_delta_badge` (R8.2 / D5).
/// Shows the change in max drawdown between the last two runs on the same tuple.
pub const RUN_DELTA_BADGE_DD_LABEL: &str = "DD";

/// Short label for the Sharpe ratio delta column of the `run_delta_badge` (R8.2 / D5).
/// Shows the change in annualised Sharpe ratio between the last two runs.
pub const RUN_DELTA_BADGE_SHARPE_LABEL: &str = "SR";

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
#[allow(clippy::too_many_lines)]
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
        ("TAPE_COL_FEE", TAPE_COL_FEE),
        ("TAPE_PAUSE_LABEL", TAPE_PAUSE_LABEL),
        ("TAPE_RESUME_LABEL", TAPE_RESUME_LABEL),
        ("TAPE_LOADING", TAPE_LOADING),
        ("TAPE_EMPTY", TAPE_EMPTY),
        ("TAPE_ERROR_PREFIX", TAPE_ERROR_PREFIX),
        ("TAPE_PAUSED_BANNER", TAPE_PAUSED_BANNER),
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
        ("DATE_RANGE_SEPARATOR", DATE_RANGE_SEPARATOR),
        ("DATE_RANGE_CUSTOM_LABEL", DATE_RANGE_CUSTOM_LABEL),
        ("DATE_RANGE_START_PLACEHOLDER", DATE_RANGE_START_PLACEHOLDER),
        ("DATE_RANGE_END_PLACEHOLDER", DATE_RANGE_END_PLACEHOLDER),
        ("DATE_RANGE_INVALID_DATE", DATE_RANGE_INVALID_DATE),
        // Phase A — Lab Run button + compare overflow toast (T-D-14, T-D-16)
        ("LAB_COMPARE_CAP_HIT", LAB_COMPARE_CAP_HIT),
        ("LAB_RUN_BUTTON", LAB_RUN_BUTTON),
        ("LAB_RUN_BUTTON_RUNNING", LAB_RUN_BUTTON_RUNNING),
        ("LAB_RUN_BUTTON_COMPLETED", LAB_RUN_BUTTON_COMPLETED),
        ("LAB_RUN_BUTTON_FAILED", LAB_RUN_BUTTON_FAILED),
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
    ]
}

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
