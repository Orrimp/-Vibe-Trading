---
slug: advisor-confidence-not-verdict
status: dev-done
owner: developer
version: 0.1.0
updated: 2026-06-30
---

# P0-3 Confidence check, not verdict

Forward paper-trade formerly "read as a fresh verdict"; v2's honest framing
is that it is a **confidence check on a crown already decided by the bakeoff**.
Putting the scorecard summary alongside the forward plan + relabelling the
surface as "Confidence check" makes that explicit.

## What shipped

- `ScorecardSummary` (4 fields: `n_candidates`, `deflated_sharpe`,
  `crown_clears_dsr`, `min_btl_years`) + `Scorecard::summary() -> Option<ScorecardSummary>`
  in `crates/backtest/src/bakeoff/scorecard.rs`.
- `confidence: Option<ScorecardSummary>` wired into `ForwardRunConfig` +
  `ForwardPlan` in `crates/agent/src/config.rs`.
- `ConfidenceSummaryView` + `confidence: Option<ConfidenceSummaryView>` in
  `crates/ui/src/forward_plan/state.rs`; mirrored via
  `crates/ui/src/forward_plan/adapter.rs` (#[cfg(feature = "live")]).
- UI copy relabel: `FORWARD_PLAN_HEADLINE` → "Confidence check",
  `FORWARD_PLAN_CAPTION` updated to honest "confidence check on that pick"
  framing; 14 new P0-3 string constants in `crates/ui/src/strings.rs`.
- Confidence summary block in `crates/ui/src/screens/forward_plan.rs` (4 fact
  rows: candidates, deflated confidence, beats-holding?, min BTL).
- Fixtures: `fake_forward_plan_with_confidence()` + `fake_cockpit_forward_plan_with_confidence()`
  in `crates/ui/src/fixtures.rs`; `confidence: None` on all prior fixtures.
- Render tests in `crates/ui/tests/forward_plan_confidence_render.rs`.
- Unit tests in `crates/backtest/src/bakeoff/scorecard.rs`.

## Implementation

See CHANGELOG.md entry for `advisor-confidence-not-verdict` (v2 P0-3).
