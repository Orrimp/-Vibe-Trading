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

pub const PANEL_TAPE_TITLE: &str = "Live tape";
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
pub const KILL_BUTTON_HELP: &str =
    "Halts the trading agent and writes an incident report. Cancels open orders and flattens \
     every position. Requires a typed confirmation.";
pub const KILL_DIALOG_TITLE: &str = "Confirm stop trading";
pub const KILL_DIALOG_BODY: &str =
    "This cancels every open order, sells each open position at market, and puts the agent \
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

// ── Connection states (live broadcast bus, T32) ──────────────────────────────

/// Shown in every panel's error state when the cockpit can't reach the agent
/// process. Tells the operator exactly what to do — not just "connection
/// failed".
pub const CONNECTION_AGENT_UNREACHABLE: &str =
    "Can't reach the trading agent. Start it with `cargo run --bin agent` and re-launch the \
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
#[must_use]
pub fn all() -> &'static [(&'static str, &'static str)] {
    &[
        ("APP_TITLE", APP_TITLE),
        ("PANEL_TAPE_TITLE", PANEL_TAPE_TITLE),
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
