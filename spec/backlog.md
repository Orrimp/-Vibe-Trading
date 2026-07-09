---
slug: backlog
status: living
owner: orchestrator
updated: 2026-07-09
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

> **Status (2026-07-09): FEATURE-COMPLETE.** The advisor MVP (F1–F9 + EUR-FX + dynamic data),
> the **v2 research-driven tranche** (11 features, ADRs 0075–0081), and the **v3 "prove it's
> done" close-out** (Calibrate-stage stepper ADR-0083, do-not-build register, DSR report-only
> decision, end-to-end demo runbook) have all shipped — see [`../CHANGELOG.md`](../CHANGELOG.md).
> The MVP/v0.2 roadmap and the proposed arm-class expansions below are **retained as the
> shipped-history record**; the only genuinely-open items are in § Queue. Post-v2 scoping
> verdict (there is no coherent add-more-features v3): [`dev-notes/post-v2-scoping-2026-07-09.md`](dev-notes/post-v2-scoping-2026-07-09.md).

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
- **F8 — strategy mix / LLM-ML ensemble** (L) — _shipped_ (the 2-arm pre-registered vote-ensemble
  proof-of-seam + robustness-gate activation; `EnsembleStrategy`, ADR-0063). See CHANGELOG.
- **F9 — guided UX polish + LLM-narrated "why"** (M).

### Combination-space expansion (proposed 2026-06-23 — operator request)
- **advisor-combination-search** (M, NEW, `proposed` — [feature.md](v1/advisor-combination-search/feature.md),
  trace `REQ-ADVISOR-COMBINATION-SEARCH-001`) — **expand the strategy-COMBINATION space, overfit-safely.**
  Widen F8's bounded, **pre-registered** vote-ensemble slate from 2 → 8 arms (3 decorrelation pairings +
  the complete k-of-4 ladder over the 4 base signals) → advisor field 4 singles + 8 ensembles +
  buy-and-hold = 13 arms. **Crux = pre-registration** (a FIXED, code-declared falsifier slate; no search =
  overfit-safe by construction), each arm scored through the **frozen** `RobustnessMode::Bootstrap` gate +
  the buy-and-hold benchmark. Reuses `EnsembleStrategy`/`arbitrate`/`build_ensemble`/`run_bakeoff`/
  `rank_candidates` VERBATIM (vote arms need ZERO new arbitration math). **Robustness bands FROZEN**
  (NOT a B2/B3 band proposal); BenchmarkWins/AllFragile reachability UNCHANGED; anchor-safe by construction
  (new `v0.8.vote.*` ids, `write_report=false`). Honest goal: discover whether ANY decorrelated combination
  *survives the gate* — **not** manufacture a winner; a null all-Fragile result ("hold stands") is valid +
  expected. **Out of v1 (recorded here):** a combination-**search** engine (overfit-prone — only with
  walk-forward/OOS split + complexity penalty + pre-registered procedure + loud risk call-out); weighted /
  inverse-vol / conditional-regime blends (need a new `VoteMethod` variant + a continuous knob = overfit
  risk → defer to a v0.2 of this feature).

### Single-coin directional short-selling (proposed 2026-06-23 — operator directive "do the expensive short selling")
- **advisor-short-selling** (L, NEW, `proposed` — [feature.md](v1/advisor-short-selling/feature.md),
  trace `REQ-ADVISOR-SHORT-SELLING-001`) — **give the long-or-flat single-coin advisor the down-half
  lever.** A bounded, **pre-registered** set of short-capable single-coin strategies (v1 slate:
  `sma_cross_ls` / `macd_ls` / `rsi_ls` / `bbands_ls` symmetric long/short variants of the existing
  rule engines + an `always_short` benchmark control) that sell-to-open a **simulated** short on the
  bearish flip + buy-to-cover on the bullish flip, with correct signed short P&L, judged by the
  **same frozen robustness gate + same buy-and-hold benchmark** (ADR-0066) as every long arm. **KEY
  de-risking finding:** a complete, tested, *shipped* short-side engine (open / cover /
  maintenance-margin liquidation with honest cash-can-go-negative / per-bar funding) ALREADY EXISTS
  in `montecarlo.rs::run_path` from the market-neutral perp-basis feature
  (`REQ-PERP-BASIS-MARKET-NEUTRAL-001`, science verdict FAMILY-UNIFORM-FRAGILE) — but only in the
  **multi-symbol cross-sectional** path; the single-coin `run_scenario`/`sma_composed_run` path is
  hard long-only by 3 explicit clamps (`engine.rs:1632-1640`/`:1713-1715`, `cli_types.rs:632-635`,
  `sma_composed_run.rs:554`) while its equity formula `cash + qty·mark` is **already short-correct**.
  So the feature is **port-and-adapt the proven signed-position model into the single-coin path, NOT
  invent it.** 5 forks resolved: perp-funding instrument; **1x leverage** (already `MAX_LEVERAGE=ONE`);
  configurable constant **`FxRate`-style per-bar funding cost** (no live feed); **honest unbounded-loss**
  via the shipped maintenance-margin liquidation (cash may go negative — does NOT cap losses at 0) + a
  "a short can lose more than your €200" disclaimer; the bounded **symmetric long/short + always_short**
  arm slate (long-only arms untouched). **Honest framing load-bearing:** shorts are **very likely ALSO
  Fragile** (the MN long/short basis spread was FAMILY-UNIFORM-FRAGILE; single-coin directional shorts
  inherit full inverse market beta + a real funding cost) — a **null result ("all short arms also
  Fragile, hold stands") is the expected, valid, shippable outcome**; the goal is an honest test of
  whether directional shorts add robust value where long-only can't, NOT to manufacture a winner.
  **HARD non-goals + anchor safety:** gate/bands/benchmark FROZEN (NOT a band proposal); new arms run
  `write_report=false` → anchor-safe by construction (119/119, run before AND after); the single-coin
  long-only path re-proven byte-identical (mirror the MN `run_path` re-proof); **NO live trading / NO
  real orders / NO real margin** (the €200 is SIMULATED — standing constraint); >1x leverage + a live
  funding feed + a short-capable *combination* slate are explicit follow-ons. **Engine-surface
  estimate (candid):** ~5-8 dev-days for the short-side engine + the long-only byte-identity re-proof
  (the MN estimate — the financial core is a *port*) PLUS the single-coin **audit-ledger sign-handling**
  (`OpenPosition` asserts `qty>0` + raises `LedgerError::Database` on net-negative qty,
  `core/position.rs:71-73` — the dominant NEW, isolation-sensitive, non-ported risk) PLUS the
  honest-negative-P&L UI short surfaces. Touches: backtest P&L core, `core::signal`, `crates/strategy`,
  the funding constant, `crates/audit` + `core::position`, `crates/agent` + `crates/exec/paper.rs`
  (paper-sim parity), `crates/ui` (render-layer-verified). 6 OQs (`Q-SS-1..6`) for the architect M-T1;
  an ADR-0051-style anchor-additive amendment owed (2nd feature to touch the single-coin engine's short
  clamps after the MN feature touched `run_path`). A separate analyst spawn from combination-search.

#### Remaining sibling strategy direction (operator-raised 2026-06-23 — one-liner only, NOT scoped yet)
- **Expand the single-coin strategy library with new signal types** (M, future) — add new base signals
  beyond the current 4 (SMA / MACD / RSI / Bollinger). Each new signal would be a new bake-off arm scored
  by the frozen gate; would also enlarge the decorrelation menu the combination feature draws from.
  A separate analyst spawn.

Full rationale + reuse-vs-new mapping + the ranked product decisions: [`product.md`](product.md).

## Remediation plan — RATIFIED 2026-07-09 (operator: "lets start with it")

> Source: the 2026-07-09 orchestrator critique (product/integrity/data/governance review,
> after feature-completeness). **This is the active forward queue.** Sequenced by leverage;
> each phase runs the normal analyst → architect → developer ‖ ui-designer → tester →
> presenter pipeline with anchors 119/119 + spec-lint gated per commit. **FROZEN gate stays
> byte-frozen throughout** (sole ADR-gated exception would be a future D1=(b) reversal).
> Ratified defaults: **D1=(a)** presentation-layer, **D2=in** (wording ratification built
> into P5), **D3=CI stays parked**, **D4 deferred to P8**.

- **P0 — ops unblock** (hours, in progress) — kill the recurring 1Password SSH relock push
  wedge (repo-scoped keychain deploy key or HTTPS credential helper; operator generates the
  key) + `spec/runbooks/ops-push-and-cache.md` incl. cargo cache-corruption recovery.
- **P1 — crown/scorecard integrity alignment** (2-3 days, **D1=(a) ratified**) — the gate
  crowns noise ~1/5 seeds (P2-2 empirical); DSR catches every one but only in a side panel.
  Fix at the PRESENTATION layer: the recommendation banner co-presents the credibility
  verdict — a crowned pick failing deflated-Sharpe renders an unmissable "fails overfitting
  check — weak evidence" state on the crown itself. No gate change (D3 report-only stands);
  additive UI; render-verified with negative control.
- **P2 — data corpus expansion + verdict re-run** (1-2 wks, mostly compute; in progress) —
  Binance hourly back to 2017-08 (mania/COVID/recent regimes) + a second reconcilable venue
  (Kraken per the venue-trust map) as a cross-check corpus; new pinned SHAs; re-run the full
  bake-off + the P2-2 null CI on the extended corpora (`write_report=false` → anchors
  untouched by construction). Either ship-passive survives (stronger claim, MinBTL improves)
  or it breaks somewhere (real signal). Both outcomes are product value.
- **P3 — PIT/as-of discipline** (3-4 days) — the do-not-build register's ONE open gap: an
  `as_of` join helper + a look-ahead lint that makes future-peeking impossible by
  construction; retrofit onto the DVOL/macro exogenous joins.
- **P4 — €200 realism** (2 days) — min-notional + lot-size rounding as an **opt-in**
  exec-sim mode (the `VolScaledSpread` pattern; default byte-unchanged, anchors safe by
  construction) + the mandatory day-1 divergence e2e.
- **P5 — SUGGEST → manual hand-off export** (3 days, wording operator-ratified before
  build) — a checklist export at journey end ("following this plan manually means X"):
  plan rules, sizing, disclaimers; **NO order placement, NO venue API** (register B-2
  intact). The cheapest change that makes the product usable by a human at all.
- **P6 — governance debt** (3-4 days) — (a) **CHANGELOG-completeness lint** (shipped trace
  row ⇒ CHANGELOG line; the exact hole R3-4b found; ADR-0082 lint pattern + self-test);
  (b) dev-notes consolidation (index + archive, v1-stub discipline); (c) a
  current-architecture rollup doc superseding serial reads of 83 ADRs.
- **P7 — CI activation** (1 day, **operator-gated — stays PARKED**) — `git mv
  ci.yml.deferred → ci.yml` starts the 3-OS matrix; operator declined at the v3 close-out;
  do NOT activate without explicit direction. Standing recommendation: activate once P1-P4
  add code deserving the matrix.
- **P8 — identity fork** (**operator decision, after P1-P6 land**) — (A) instrument: done
  after P1-P7; (B) multi-asset cross-sectional track: register B-1 named TRACK CHANGE, a
  new product, only edge family the research found surviving; (C) honesty-as-a-service:
  package the inoculation demo as the shareable output.

## Queue (open / deferred)

### Deferred by decision
- **cockpit-cross-platform CI** — Linux/Windows source shipped + macOS-verified; the
  3-OS GitHub Actions matrix is parked inert at `.github/workflows/ci.yml.deferred`.
  Activation deferred to the **near-done project milestone**; re-affirmed as remediation
  **P8 → P7** above (do not `git mv` it live before an explicit operator go).
- **`lab-recipe-test-harness v0.3.0+`** — Recipe / subscription harness extension;
  robustness gate cleared, awaiting an analyst spawn. **Still wanted** — re-confirmed in the
  v3 close-out (2026-07-09) as the one genuinely-open forward *build* item; it is infra, NOT
  required for product feature-completeness (see [`../CHANGELOG.md`](../CHANGELOG.md) § Deferred).

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
