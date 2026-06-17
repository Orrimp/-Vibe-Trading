---
slug: backlog
status: living
owner: orchestrator
updated: 2026-06-17
---

# Backlog

> **What has been built lives in [CHANGELOG.md](../CHANGELOG.md)** — one line per
> feature, grouped by subsystem/version. This file is now the lean **forward-looking
> queue** only. The concluded-program archaeology (the ~11 measured-and-retired
> strategy bets, the v2.5 DL chain, the active-vs-passive wind-down record, the
> on-chain fork) was compressed out 2026-06-17; it remains in **git history** and in
> `spec/dev-notes/`.
>
> **Strategy research is CONCLUDED (2026-06-08) → ship passive.** Across all three
> reachable channels (price/OHLCV, derivatives-positioning, on-chain) no active
> strategy beat passive buy-and-hold net of cost under the frozen block-bootstrap
> robustness rule. **No active-strategy bets remain.** Terminal verdict + scope:
> [`spec/product.md`](product.md).

## Active

The program is at a clean terminal state. One open operator decision:

- **Passive-baseline rebalance cadence + weighting** — confirm the proposed default
  (monthly / equal-weight) for [`spec/runbooks/passive-baseline.md`](runbooks/passive-baseline.md)
  and record it in that runbook's changelog. Everything else in the wind-down is closed.

## Queue (open / deferred)

### Deferred by decision
- **cockpit-cross-platform CI** — Linux/Windows source shipped + macOS-verified; the
  3-OS GitHub Actions matrix is parked inert at `.github/workflows/ci.yml.deferred`.
  Activation deferred to the **near-done project milestone** (do not `git mv` it live before then).
- **`lab-recipe-test-harness v0.3.0+`** — Recipe / subscription harness extension;
  robustness gate cleared, awaiting an analyst spawn. The one genuinely-open build item.

### Gated on the parked v2 LLM strategy
- **Lumen Phase 6 — right-rail Assistant slot** — reserved column-track in the shell grid;
  hidden until the v2 LLM strategy is enabled.
- **v2.1 cockpit LLM-budget tile + pedantic clippy cleanup** — deferred indefinitely (program concluded).
- **v2 LLM evolution** (`v2x-trading-state-bus`, `v26-bakeoff-llm-arbiter`) — deferred; gated on
  re-activating the LLM desk, which is support-layer scope, not alpha.

### Future fresh program (NOT a continuation of the concluded hunt)
- **C4 — deterministic learning loop** (reflection-feedback decision seam; `product.md` core
  pillar 3) — never built; would adapt param/route selection from the reflection store through
  the sanctioned ADR-0041 layering seam. Moot while passive is the shipped strategy.
- **Untested orthogonal channels** — options/implied-vol (Deribit DVOL), cross-asset/macro
  (DXY, rates, SPX), social/sentiment. Out of scope for the concluded hunt; each would be a
  **fresh** program with its own data adapter and backtest, not a re-open of this one.

> Speculative UI test-infra candidates (AccessKit shadow-tree assertions, VLM second-opinion
> judge, comet debugger, inspect-MCP shim, mutation-testing pass, …) lived here as unscheduled
> ideas; they are preserved in git history and re-proposable on demand rather than carried inline.

## Recent (shipped)

See **[CHANGELOG.md](../CHANGELOG.md)** for the full per-version shipped index, and
`git log -- spec/<slug>/` for any feature's narrative history.

## Conventions

- This file holds the **forward-looking queue only**; shipped work is recorded in
  [CHANGELOG.md](../CHANGELOG.md), not here.
- One-line entries; a queued item is promoted to a `spec/<slug>/feature.md` brief
  only when an analyst picks it up.
- The orchestrator owns this file; agents may suggest additions, the operator approves promotions.
- Items can stay indefinitely; stale items get a `_decayed_` tag rather than silent deletion.
