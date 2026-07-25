//! Screen-routed shell bodies (Phase 2 + Phase 3 + Phase A).
//!
//! One module per screen. Each module exposes a `view()` function that
//! takes `&Cockpit + ThemeMode` and returns an `Element<Message>`. The
//! shell (`crate::shell::view`) dispatches on `Cockpit::current_screen`
//! to pick the right body. Phase 2 shipped `home / debug / charts`;
//! Phase 3 lands `strategies / risk / audit`; Phase A renames `charts`
//! → `lab` and adds placeholder routes.

pub mod audit;
/// cockpit-baseline-panel v0.1.0 — passive buy-and-hold baseline screen.
/// Headline + year toggle + KPI strip + equity curve + drawdown band,
/// reusing the existing widgets verbatim. Navigable via the Work sidebar
/// group (after Compare); not default-routed (D2).
pub mod baseline;
/// Phase E — Compare matrix screen (ui-rethink-phase-e-compare R1.1-R1.4).
/// Toolbar + matrix body. Replaces the Phase A `placeholder::view` route.
pub mod compare;
pub mod control;
pub mod debug;
/// advisor-forward-plan v0.1.0 — the forward buy/sell plan (single-coin
/// investment-advisor journey, step 4). A conditional, reactive, rule-driven
/// decision plan over a `ForwardPlanView` (mirrored from the `core`-typed
/// `agent::config::ForwardPlan`): the dated current-stance badge, the standing
/// IF/THEN entry/exit rules, the budget-aware €200 next-BUY sizing "at the last
/// close", the "planned through <date>" horizon, and the mandatory
/// not-a-prediction + not-advice disclaimers — presented as decision-support,
/// NOT a forecast (OQ-D). Navigable via the Work sidebar group (after
/// Leaderboard, before Live); not default-routed.
pub mod forward_plan;
pub mod home;
/// Phase A — Lab screen (ex-`charts.rs`, T-D-2 rename). New default route
/// (R1.2). The legacy `Screen::Charts` variant auto-routes to `lab::view`
/// via the shell match arm (deprecated alias for backward compatibility).
pub mod lab;
/// advisor-leaderboard-screen v0.1.0 — the strategy bake-off leaderboard
/// (single-coin investment-advisor journey, step 3). Ranked table over a
/// `backtest::bakeoff` result (best-first, crowned row highlighted, benchmark
/// labelled) + a recommendation headline rendered from the structured
/// `Recommendation` + the persistent not-advice disclaimer. Navigable via the
/// Work sidebar group (after Baseline); not default-routed.
pub mod leaderboard;
/// Phase C — Live trading dashboard (ui-rethink-phase-c-sidebar-ia R2.1).
/// Replaces the legacy `home::view` 2×2 grid for the `Screen::Live` route.
/// `Screen::Home` (deprecated) also routes here via the compat shim (R5.2).
pub mod live;
/// Phase F — Memory screen (ui-rethink-phase-f-memory-models-assistant R1.1-R1.4).
/// Toolbar + cards list + optional side-drawer. Replaces the Phase A
/// `placeholder::view` route for `Screen::Memory`.
pub mod memory;
/// Phase F — Models screen (ui-rethink-phase-f-memory-models-assistant R2.1-R2.4).
/// Toolbar (family + status chips) + checkpoint list. Replaces the Phase A
/// `placeholder::view` route for `Screen::Models`.
pub mod models;
/// cockpit-reports-viewer v0.1.0 — browse + render committed backtest
/// reports. List-detail: left picker over the discovered
/// `evidence/*/reports/backtest-*.md` corpus + right detail pane reusing
/// `kpi_strip` / `equity_curve` / `drawdown_band` + the markdown body.
/// Navigable via the Library sidebar group (after Models); not
/// default-routed (D5).
pub mod reports;
pub mod risk;
/// Phase C — Settings rollup (ui-rethink-phase-c-sidebar-ia R4.1).
/// Three-tab chrome wrapping `risk::view`, `control::view`, `debug::view`.
pub mod settings;
pub mod strategies;
/// Phase C — Strategy registry (ui-rethink-phase-c-sidebar-ia R3.1).
/// List-of-cards replacing the legacy `strategies::view` detail panel.
pub mod strategy_registry;
/// Phase D — Trail view screen (ui-rethink-phase-d-trail R2.1-R2.5).
/// List mode delegates verbatim to `screens::audit::view` (R10.1 byte-identity gate).
/// Trail mode renders the upstream node stack + side-drawer.
pub mod trail;
/// advisor-param-tuning (ADR-0069) — the gate-tied hyperparameter sweep editor
/// ("Tune"). The range form (family picker + per-axis {min,max,step} inputs +
/// presets) + the live grid-size readout + the result grid (one row per swept
/// config: params · verdict · return · Sharpe p5/p50/p95 · P(loss) ·
/// P(Sharpe>1) · Max-DD p95) with FRAGILE prominently flagged + promotion-
/// blocked, the shipped-baseline row, the buy-and-hold strip, the truncation
/// banner, and the persistent honesty footer. Navigable via the "Tune…" row
/// drill-down off the Leaderboard; not sidebar-default-routed.
pub mod tune;
