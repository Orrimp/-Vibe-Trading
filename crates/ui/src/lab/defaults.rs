//! Cold-start Lab defaults — ui-rethink-phase-a-lab T-D-17.
//!
//! Defines `LAB_COLD_START_STRATEGY`, `LAB_COLD_START_SYMBOL`,
//! `LAB_COLD_START_VENUE`, and `LAB_COLD_START_RANGE` per operator-decision
//! Q-A3 (2026-05-17): `v1.momentum × XRPUSDT × Last 90d`.
//!
//! These constants are separate from `LabState::default()` because `LabState`
//! owns `Option` fields — the cold-start is the first pre-populated state the
//! operator sees after `persistence::restore_or_default` falls back.

use smol_str::SmolStr;
use trading_core::{StrategyId, Symbol, Venue};

use crate::lab::state::{DateRange, Preset};

/// Cold-start strategy — `v1.momentum` (operator-decision Q-A3).
pub const LAB_COLD_START_STRATEGY_ID: &str = "v1.momentum";

/// Cold-start symbol — `XRPUSDT` (operator-decision Q-A3).
pub const LAB_COLD_START_SYMBOL_STR: &str = "XRPUSDT";

/// Cold-start venue — `Binance` (Phase A universe is single-venue).
pub const LAB_COLD_START_VENUE: Venue = Venue::Binance;

/// Cold-start date range — `Last 90d` (operator-decision Q-A3).
pub const LAB_COLD_START_RANGE: DateRange = DateRange::Preset(Preset::Last90d);

/// Default RNG seed for cockpit-initiated Lab runs (ADR-0030).
/// Shown in the run metadata strip so the operator can reproduce exactly.
///
/// `[0u8; 32]` is explicitly rejected by `engine::run_scenario` — this
/// non-zero constant is the Lab's default.
pub const LAB_DEFAULT_SEED: [u8; 32] = {
    let mut seed = [0u8; 32];
    seed[0] = 0xC0;
    seed[1] = 0xFF;
    seed[2] = 0xEE;
    seed
};

/// Build the cold-start strategy id.
#[must_use]
pub fn cold_start_strategy() -> StrategyId {
    StrategyId(SmolStr::new(LAB_COLD_START_STRATEGY_ID))
}

/// Build the cold-start symbol.
#[must_use]
pub fn cold_start_symbol() -> Symbol {
    Symbol::new(LAB_COLD_START_SYMBOL_STR)
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    /// T-D-17 — cold-start tuple matches the operator-locked Q-A3 decision.
    #[test]
    fn cold_start_tuple_matches_qa3() {
        let strat = cold_start_strategy();
        let sym = cold_start_symbol();
        assert_eq!(strat.0.as_str(), "v1.momentum", "Q-A3: strategy must be v1.momentum");
        assert_eq!(sym.0.as_str(), "XRPUSDT", "Q-A3: symbol must be XRPUSDT");
        assert_eq!(LAB_COLD_START_VENUE, Venue::Binance, "Q-A3: venue must be Binance");
        assert_eq!(
            LAB_COLD_START_RANGE,
            DateRange::Preset(Preset::Last90d),
            "Q-A3: range must be Last 90d"
        );
    }

    /// T-D-17 — default seed is non-zero (ADR-0030 rejects all-zero seed).
    #[test]
    fn default_seed_is_non_zero() {
        assert_ne!(
            LAB_DEFAULT_SEED,
            [0u8; 32],
            "LAB_DEFAULT_SEED must not be all-zero (ADR-0030 rejects [0u8; 32])"
        );
    }
}
