---
adr: 0076
title: Turnover (Churn) + coherent tail/median KPIs — additive report-only KPIs alongside the bake-off verdict; CVaR not VaR
status: accepted
date: 2026-06-30
supersedes: none
superseded-by: none
---

# ADR-0076: The cost story + the risk story (P1-1 + P1-2)

## Context

After ADR-0075 (the credibility layer), the leaderboard still stopped at
**Return / Sharpe / Max-DD / Trades + a headline crown** — enough to
*pick* a strategy, not enough to *explain why*. The v2 analyst flagged
this as a workflow gap (`spec/v2/v2-analysis.md` §1): the operator can
see *that* a verdict was crowned but not *why* the gross-vs-net spread
exists, nor *what the tail looks like* behind the average return.

The v2 architect (`spec/v2/v2-architecture.md` §1 P1-1 + §1 P1-2) scoped
two additive surfaces. Both are reductions over data the bake-off already
captures — no new bootstrap pass, no new gate input, no anchored report
side-effect.

## Decision

Land **two additive KPI surfaces** — the **cost story** (P1-1: turnover
as a `Churn` column) and the **risk story** (P1-2: a six-fact "Risk
story" panel). Both **report-only**, never read by the FROZEN gate.

### P1-1 — Turnover ("Churn")

- New field `CandidateKpis.turnover: Decimal` — chosen formula
  `Σ(fill.price × fill.qty) / mean_equity`, the unitless ratio "how many
  capital-equivalents transacted" (e.g. `5.0×`). Computed in
  `derive_candidate_kpis` from data `RunReport` already carries. No new
  engine capture.
- Mirrored as plain `Decimal` into `LeaderRow.turnover` via
  `BakeoffReportMirror::from_report` — zero new `ui` dep edge.
- Rendered as a new rightmost numeric column on the leaderboard's ranked
  table, formatted `"N.N×"` (one decimal, unicode `×`).

### P1-2 — Coherent tail + median ("Risk story")

- `DistributionSummary` extended with four `f64` fields, all reductions
  over the *existing* 1000-path `PathMetrics` vector:

  | Field | Definition | Source |
  |---|---|---|
  | `cvar_95: f64` | Mean of the worst 5 % of bootstrap `total_return` paths | Rockafellar–Uryasev / `risk-and-sizing[…]` |
  | `cvar_99: f64` | Mean of the worst 1 % of paths | sibling |
  | `median_terminal_wealth: f64` | Median of `final_equity` across paths | `risk-and-sizing[…]` |
  | `skew: f64` | 3rd standardised central moment of `total_return` across paths | sibling |

- A `TailSummary` struct is computed for the **crown only** at
  `run_bakeoff` exit, carried on `Recommendation.crown_tail:
  Option<TailSummary>`. Mirrored to `ui` as `TailSummaryView` over the
  same `from_report` boundary as plain `f64`. **Zero new `ui` dep edge.**

- The leaderboard renders a **"Risk story"** panel (`frame::panel`, same
  chrome as the scorecard) under the scorecard with six facts —
  *Typical outcome (median)* · *Average loss in the worst 5 % of paths
  (CVaR-95)* · *Average loss in the worst 1 % of paths (CVaR-99)* ·
  *Surprise shape (skew)* · *Downside-only Sharpe (Sortino)* · *Return
  vs worst drawdown (Calmar)* — each with a plain-language gloss. The
  Sortino + Calmar facts surface `CandidateKpis` fields that already
  exist but were previously narration-only. Footer reads "Informational,
  not a gate — these never change the pick above."

### CVaR — not VaR

**Coherent risk measures only.** CVaR (Expected Shortfall — the *mean*
of the worst α-fraction of paths) is sub-additive: the risk of a
combined portfolio never exceeds the sum of the parts' risks. Plain VaR
(the α-quantile) is not coherent — it rewards concentration over
diversification, and reporting a non-coherent measure is dishonest. The
UI copy explicitly says "Expected shortfall (CVaR) — coherent, unlike
plain VaR." The architect's §1 P1-2 names this constraint as
non-negotiable.

### Anchor safety

The advisor bake-off path runs `write_report=false` → both the `Churn`
column and the `Risk story` panel are **anchor-safe by construction**.
`verify_anchors.sh` stays 119/119 before and after.

### Frozen-gate identity

`turnover_does_not_change_ranking` (unit test) asserts `rank_candidates`
is byte-identical with and without `CandidateKpis.turnover` populated.
`classify_verdict` continues to read only its five frozen signals — the
new tail/median scalars never enter the verdict.

### Operator-ratified constraints (`v2-architecture.md` §6.0 carry-over)

- **Report-only — never a veto.** Same discipline as ADR-0075 (D3). A
  future tail-based veto (e.g. "crown disqualified if CVaR-99 < X")
  would be a FROZEN-gate change and needs its own ADR + an operator call.
- **Crown-only tail.** `TailSummary` is computed for the crown only —
  one panel per bake-off, mirroring the scorecard precedent. Per-row
  tail tooltips are a future polish (the `PathMetrics` vector is already
  captured per candidate, so this is UI-only work when wanted).
- **No annualised turnover rate yet.** `"N.N×"` is the operator-natural
  framing. Per-year scaling is a future formatting decision.
- **CVaR coherence is the architectural invariant** — never replace
  with VaR even on operator request without an ADR amendment.

## Status: accepted

- Tester `VERDICT → PASS` at commit `decbcc4`
  (`spec/v2/advisor-turnover-and-tail-metrics/reports/test-2026-06-29-advisor-turnover-and-tail-metrics.md`).
- 783 tests pass (193 backtest incl. 4 turnover tests + `turnover_does_not_change_ranking`;
  583 ui --lib; 7 render integration incl. 3 risk-story).
- Anchors 119/119 byte-immutable; spec-lint PASS; clippy `-D warnings` clean.
- Bonus: pre-existing `bakeoff_progress_render` 1/3 y-band drift was
  fixed in the same UI commit; that target is now 3/3 PASS.
- Operator-approved 2026-06-30 — status flipped `tester-done → shipped`.

## Consequences

- **The cost story + risk story make the null legible** — the cockpit
  can now explain *why* holding wins on most windows (zero churn; deeper
  CVaR-99 tail than average return suggests). That is the product thesis.
- **A future tail-based veto remains possible** without re-architecting;
  the `TailSummary` carrier is already in place. The change would be a
  single read in `classify_verdict` *plus* a new ADR + operator call.
- **Per-row tail tooltips** are unblocked — the `PathMetrics` vector
  exists per candidate; today's UI only surfaces the crown's tail.
- **The architectural invariant** of `ui` purity (no dep on
  strategy/exec/llm/models) is preserved verbatim — both new surfaces
  cross the existing `BakeoffReportMirror::from_report` boundary as plain
  scalars.

## References

- Spec: `spec/v2/advisor-turnover-and-tail-metrics/feature.md`.
- Tester report: `spec/v2/advisor-turnover-and-tail-metrics/reports/test-2026-06-29-advisor-turnover-and-tail-metrics.md`.
- Presenter deck: `spec/v2/advisor-turnover-and-tail-metrics/presentations/advisor-turnover-and-tail-metrics-2026-06-30.md`.
- Architect: `spec/v2/v2-architecture.md` §1 P1-1 + §1 P1-2 + §6.0.
- Research: `research/risk-and-sizing/application-position-sizing-and-bet-sizing.md` §6 P1;
  `research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md` §6 P2-D;
  `research/backtesting/application-cost-and-impact-modeling.md` §6 A.
- Commits: `66286e2` (backend) · `00240ed` (UI Churn + Risk story) · `decbcc4` (tester VERDICT PASS) · `b50f2f5` (presenter deck).
