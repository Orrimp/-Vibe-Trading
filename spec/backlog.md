---
slug: backlog
status: living
owner: orchestrator
updated: 2026-06-19
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

## Active — Single-Coin Investment Advisor (2026-06-19 pivot)

The product was **redefined 2026-06-19** (see [`product.md`](product.md)): a paper
decision-support tool for a retail investor — *pick a coin + budget → bake off all
strategies → rank & pick the best → forward plan → watch it paper-trade your €200*. The
shipped engine (backtest, strategy library, LLM, paper-sim, Live view, ledger, reflection,
cockpit) is **reused**; the queue below is the new connective tissue + UX.

**Decisions (operator-set 2026-06-19):** rank by risk-adjusted return (Sharpe) + a
robustness gate, with buy-and-hold always the benchmark; ship the single best strategy
first, mixes / LLM-ML ensemble next; treat €200 ≈ 200 USDT (FX not modelled in the MVP);
paper-only, not-advice on every recommendation.

### MVP — the end-to-end loop (build in dependency order)
- **F1 — bake-off orchestrator** (M, NEW) — loop every strategy + buy-and-hold on one
  `(coin, lookback)`, collect KPIs. Lives in `agent`/`backtest` (the `ui`-never-imports-`strategy`
  invariant); wraps the existing Lab runner / `run_scenario`, no new backtest math.
- **F2 — ranking + recommendation** (M, NEW) — leaderboard by Sharpe + robustness gate; one
  highlighted pick + a plain-language "why this one".
- **F3 — guided "new investment" input** (S, reuses Lab pickers) — coin + budget + lookback.
- **F4 — budget-aware €200 sizing** (S-M, NEW; ships with the day-1 baseline-equity-divergence
  e2e test per the CLAUDE.md non-negotiable).
- **F5 — forward paper-trade of the selection** (M, reuses Live view + paper agent) — run the
  chosen strategy forward, show running €200 P/L.  ← **MVP complete.**

### v0.2 enhancements
- **F6 — forward buy/sell plan detail** (M) — today's stance + entry/exit rules + projected sizing.
- **F7 — EUR→USD fixed-rate** (S) — convert €200 at a config rate before sizing.
- **F8 — strategy mix / LLM-ML ensemble** (L) — capital split across top-K; LLM-as-analyst confirm.
- **F9 — guided UX polish + LLM-narrated "why"** (M).

Full rationale + reuse-vs-new mapping + the ranked product decisions: [`product.md`](product.md).

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
