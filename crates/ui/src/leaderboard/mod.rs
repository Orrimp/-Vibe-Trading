//! advisor-leaderboard-screen v0.1.0 — the strategy bake-off LEADERBOARD.
//!
//! Step 3 of the single-coin investment-advisor journey (product § journey:
//! pick coin + budget → bake off all strategies → **rank & pick best** → plan
//! → watch paper-trade). This module makes the just-landed
//! `backtest::bakeoff` engine result visible and clickable in the cockpit.
//!
//! Pure-`ui` over `trading_core` + `backtest` (the engine seam `ui` already
//! imports) + `std` — **no new crate edge**. `ui` MUST NOT import `strategy` /
//! `exec` / `forecast` / `llm`; `backtest::BakeoffReport` is consumed through
//! the existing `backtest` dep and mirrored into a pure-`ui` shape at the
//! dispatch boundary (see [`state::BakeoffReportMirror::from_report`]).
//!
//! Module layout (sibling of `reports/`):
//!
//! ```text
//! leaderboard/
//! ├── mod.rs      — this file, re-exports
//! ├── state.rs    — LeaderboardScreenState + BakeoffReportMirror + the
//! │                 engine→ui mirror (the INVARIANT seam) + Loading/Empty/
//! │                 Error/Ready handling
//! └── runner.rs   — spawn_bakeoff: async-dispatch run_bakeoff (mirrors
//!                   lab::runner::spawn_lab_run) + default_bakeoff_config
//! ```
//!
//! The screen body lives at `crate::screens::leaderboard`; the sidebar IA +
//! `Screen::Leaderboard` routing live in `crate::theme` / `crate::state` /
//! `crate::shell`.

pub mod runner;
pub mod state;

pub use state::{
    BAKEOFF_COIN_UNIVERSE, BakeoffReportMirror, LeaderRow, LeaderboardLookback,
    LeaderboardScreenState, OutcomeKind, ReasonLabel, RecommendationMirror, RobustnessLabel,
    parse_budget,
};
