---
slug: advisor-overfitting-scorecard
status: dev-done
owner: developer
version: 0.1.0
updated: 2026-06-29
---

# Overfitting Scorecard (P0-1) — the credibility layer

The v2 #1 feature (surfaced #1 in 6 of 9 research topics): a **report-only**
overfitting scorecard surfaced next to every bake-off recommendation, answering
"did we fool ourselves by trying many strategies?" Closed-form
**N_eff → Deflated-Sharpe → MinBTL** (PBO deferred to the Tune surface). **Additive
to the FROZEN gate — never a veto.** This is the literal "traceable & plausible"
product thesis.

**Design + ratified decisions:** [`v2-architecture.md`](../v2-architecture.md) §1 P0-1
+ §3 (the `Scorecard` design sketch) + **§6.0** (operator-ratified: report-only;
closed-form N_eff frozen at the 24-config scale; no PBO / threshold / crown-veto in
v2). Analyst framing: [`v2-analysis.md`](../v2-analysis.md) §2. Research:
[`research/backtesting/application-overfitting-and-multiple-testing.md`](../../../research/backtesting/application-overfitting-and-multiple-testing.md)
§6 + [`research/evolution/application-anti-overfitting-and-search-discipline.md`](../../../research/evolution/application-anti-overfitting-and-search-discipline.md) §6.

## What shipped (this dev increment)

- `crates/backtest/src/bakeoff/scorecard.rs` — a pure module: the `Scorecard` struct +
  `n_eff` / `min_btl` / `dsr` closed forms, `normal_cdf` + a high-accuracy
  `normal_inv_cdf` (Acklam rational + one Halley refinement step), skew/kurtosis,
  `sharpe_variance`, `compute_scorecard`.
- `Recommendation.scorecard` carrier (`bakeoff/mod.rs`), computed in `run_bakeoff`
  from inputs that already exist (per-candidate Sharpe vector + the crown's bootstrap
  `DistributionSummary` + the crown's return skew/kurtosis). **Report-only** — logged
  + carried; never fed into crown/rank/verdict.

## Not in this increment (per §6.0)

- **PBO/CSCV** — deferred to the Tune/sweep surface where CSCV is statistically honest (D1).
- The **UI `ScorecardView`** / leaderboard display — follow-on ui-designer task.
- Any **DSR/PBO crown-veto** — report-only (D3); `Scorecard.crown_clears_dsr` is an
  informational flag, one-line-switch-ready for a future veto + its own ADR.

ADR-0075 (reserved — written/registered atomically when the increment lands).
