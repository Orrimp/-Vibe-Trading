//! advisor-forward-plan v0.1.0 (roadmap F6) — the forward buy/sell PLAN.
//!
//! Journey **step 4 (Plan)** of the single-coin investment-advisor (product
//! § journey: pick coin + budget → bake off → rank & pick → **plan** → watch
//! paper-trade). The plan sits between the crowned Leaderboard pick and the
//! Live view: a **conditional, reactive, rule-driven decision plan — NOT a
//! price forecast** (ADR-0062). It shows the crowned strategy's current
//! stance (dated to the latest bar), the standing IF/THEN entry/exit rules
//! (the same rules the F5 paper-trade runs), and the budget-aware €200
//! next-BUY sizing "at the last close", over a "planned through <date>"
//! horizon.
//!
//! Pure-`ui` over `core` + `std` — **no new crate edge**. `ui` MUST NOT
//! import `strategy` / `exec` / `forecast` / `llm`, and gains no *new*
//! `agent` edge in its default build: the real `core`-typed
//! `agent::config::ForwardPlan` is mirrored into a pure-`ui` [`ForwardPlanView`]
//! at the dispatch boundary (the `live`-feature [`adapter`]), exactly as
//! `BakeoffReportMirror::from_report` mirrors `backtest::BakeoffReport`.
//!
//! Module layout (sibling of `leaderboard/`):
//!
//! ```text
//! forward_plan/
//! ├── mod.rs      — this file, re-exports
//! ├── state.rs    — ForwardPlanScreenState + ForwardPlanView + the closed
//! │                 ui enums (PlanStanceView / PlanSignalView / PlanRuleView)
//! │                 + Loading/Empty/Error/Ready handling
//! └── adapter.rs  — from_plan: agent::config::ForwardPlan → ForwardPlanView
//!                   (the INVARIANT seam, #[cfg(feature = "live")] — the ONLY
//!                   place ui reads the agent plan type)
//! ```
//!
//! The screen body lives at `crate::screens::forward_plan`; the
//! `Screen::ForwardPlan` routing lives in `crate::state` / `crate::shell`.

pub mod state;

/// The `agent::config::ForwardPlan` → [`ForwardPlanView`] adapter — gated on
/// the `live` feature (where the optional `agent` dep is available). The
/// default `ui` build never compiles it, so `cargo tree -p ui` is unchanged.
#[cfg(feature = "live")]
pub mod adapter;

pub use state::{
    ForwardPlanScreenState, ForwardPlanView, PlanRuleView, PlanSignalView, PlanStanceView,
    PlanVoteMethodView,
};
