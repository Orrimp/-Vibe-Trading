---
slug: advisor-combination-search
status: proposed
owner: architect
updated: 2026-06-23
---

# Tasks — advisor-combination-search

> **Analyst-seeded stub.** The analyst brief
> ([`feature.md`](feature.md), trace `REQ-ADVISOR-COMBINATION-SEARCH-001`) is
> complete and at `proposed`; the **architect owns this task list** and fills
> it during ADR scoping. The skeleton below is a placeholder so the in-flight
> feature is not an orphan — the architect should replace it with the real,
> ordered, acceptance-tagged tasks once the ADR is drafted.

## Load-bearing constraints (carry into every task — from the brief)

- **Pre-registration is the overfit-safety contract.** v1 is a FIXED,
  code-declared slate of combination arms — **NO search** over membership /
  weights / thresholds. Overfit-safe by construction.
- **Robustness bands FROZEN.** Do NOT touch `classify_verdict` /
  `compute_robustness_flag` / `verdict_bands` (ADR-0059 §D4 / ADR-0063 §D4).
  This is NOT a B2/B3 band proposal.
- **Reuse-only.** `EnsembleStrategy` / `arbitrate` / `build_ensemble` /
  `run_bakeoff` / `rank_candidates` verbatim. Recommended vote arms need ZERO
  new arbitration math.
- **Anchor-safe by construction.** New `v0.8.vote.*` ids, `write_report=false`
  on the Bootstrap advisor path. `scripts/verify_anchors.sh` → 119/119 before
  the first seam + after the last (any non-119 = STOP-and-route-back).
- **`BenchmarkWins` / `AllFragile` reachability UNCHANGED** (F8 §4 / ADR-0066).
- **Day-1 baseline-equity-divergence e2e test** per the CLAUDE.md
  non-negotiable for each new arm.

## Proposed task skeleton (architect to ratify / re-order / acceptance-tag)

- [ ] T1 — ADR: lock the v1 combination slate (the 6 new pre-registered arms,
      or the architect's adjusted set) + the pre-registration contract —
      _acceptance: ADR names every arm's `(id, VoteMethod, members)` and states
      "no runtime search"._
- [ ] T2 — Resolve OQ-1 (literal `build_ensemble` ids vs generalized dispatch)
      and OQ-2 (field single-source-of-truth + 13-arm × 1000-path latency budget).
- [ ] T3 — Resolve OQ-3 (`Unanimous{n:2}` simultaneous-Long rarity is honest,
      not a bug) and OQ-4 (defer weighted/inverse-vol to v0.2 of this feature).
- [ ] T4 — Developer: add the new ids in `build_ensemble`; widen the `engine.rs`
      `"v0.8.vote.*"` dispatch arm; extend `default_ensemble_field`.
- [ ] T5 — Developer: per-arm day-1 divergence e2e (each new arm's equity
      diverges ≥1 bp from its members AND from buy-and-hold — no silent no-op).
- [ ] T6 — Tester: real-data bake-off (BTCUSDT H1-2024, `BinanceCache`,
      `Bootstrap{paths:1000}`) reporting all 13 arms' flag + p5/p50 Sharpe +
      `RecommendationOutcome`; record the pre-registered prediction up front;
      report the WHOLE slate, win or lose.
- [ ] T7 — ui-designer: leaderboard 13-row populated render-snapshot (OQ-6);
      new ensemble rows render the honest "≥ k of {…} agree" description.

## Notes

- Sibling backlog directions (NOT in this feature): short-selling (v2, engine
  work) + new signal types — see [`../backlog.md`](../backlog.md).
