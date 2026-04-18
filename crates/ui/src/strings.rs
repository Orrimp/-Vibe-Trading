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
pub const KILL_BUTTON_HELP: &str =
    "Cancels open orders, flattens every position, and halts the agent. Requires a typed \
     confirmation.";
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
