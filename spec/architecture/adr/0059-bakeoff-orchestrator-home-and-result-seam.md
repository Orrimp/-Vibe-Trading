---
adr: 0059
title: Bake-off orchestrator home, BakeoffReport seam, and the buy-and-hold run_scenario arm
status: accepted
date: 2026-06-19
supersedes: none
superseded-by: none
---

# ADR-0059: Bake-off orchestrator home, `BakeoffReport` seam, and the buy-and-hold `run_scenario` arm

## Context

The 2026-06-19 product pivot (single-coin investment advisor,
[`spec/product.md`](../../product.md)) makes the **strategy bake-off + ranking**
(roadmap F1+F2) the first and core feature: run every strategy on one
`(coin, window)`, rank them, crown the best. The analyst confirmed the bake-off
is a **loop over the existing per-strategy path** (`backtest::run_scenario`) and
that the ranking inputs already exist (`stats::compute_sharpe/sortino/calmar`,
`BacktestKpis.total_return_pct` / `max_drawdown`, the Monte-Carlo
robustness machinery). No new backtest math is in scope.

Three structural decisions must be locked before the developer starts, because
they set crate boundaries + a public seam + an engine touch:

1. **Where does the orchestrator live**, given the standing invariant that the
   `ui` crate must never depend on `strategy` / `exec` / `forecast` / `llm`
   (ADR-0041 layering; product § Constraints)?
2. **What public type does the cockpit consume**, and through which seam?
3. **How does buy-and-hold become a bake-off candidate**, given that — verified
   against code — BH is **not** a `run_scenario` arm today (it exists only as
   `run_buyhold_path` in `crates/backtest/src/bin/param_robustness_sweep.rs`
   plus a read-only example), yet the operator requires BH as an always-present
   benchmark arm?

## Decision

**D1 — Orchestrator home = `crates/backtest`** (a new `bakeoff` module).
`backtest` already owns `run_scenario`, the `stats` layer
(`DistributionSummary`, `compute_*`), and depends on `data` (the
`BlockBootstrapPathGen`) and `strategy`. Critically, **`ui` already depends on
`backtest`** and already consumes `backtest::engine::run_scenario` /
`RunReport` through `crates/ui/src/lab/runner.rs`. Homing the orchestrator in
`backtest` lets the cockpit read the bake-off result through the **identical
sanctioned seam** with **zero** new `ui` dependency, preserving
ui-never-imports-strategy *by construction*. `crates/agent` is **rejected** as
the home: `ui` lists `agent` only as `optional`/dev, so an `agent`-homed
orchestrator would force the cockpit to take a hard `agent` dependency to read
the result type — strictly worse layering than `backtest`.

**D2 — The result seam is `backtest::bakeoff::BakeoffReport`**, a `Debug + Clone`
public type: `{ request, candidates: Vec<CandidateResult>, ranked: Vec<usize>,
crowned: Option<usize>, rationale: Recommendation }`, with `CandidateResult`
carrying the strategy id, an `is_benchmark` flag, `CandidateKpis` (Sharpe /
Sortino / Calmar / total-return / max-drawdown / trade-count), the equity curve,
and an optional `RobustnessFlag`. The rationale is a **structured `Recommendation`
data type** (outcome enum + reason codes + winner/benchmark KPIs), **not** a
pre-rendered string: the UI owns the plain-language copy + the mandatory
not-advice disclaimer (product § What this product IS NOT), the rationale stays
deterministic + unit-testable, and the optional v0.2 LLM "why this one"
narration (product § D3) becomes an additive consumer of `Recommendation`, not a
rewrite. The cockpit mirrors `BakeoffReport` exactly as it mirrors `RunReport`
today (the `RunReportMirror` / `RunSummary` precedent).

**D3 — Buy-and-hold becomes an anchor-additive `"v0.buyhold"` `run_scenario`
arm**, reusing the **extracted** `run_buyhold_path`, so the bake-off loop is
uniform (every candidate is one `run_scenario` call producing a real equity
curve through the same path). For a single coin the equal-weight BH is 100% of
budget held from bar-0 close, marked to market. The arm is anchor-additive: a new
`match` arm on a new id touches no existing arm; the bake-off runs it with
`write_report = false` so no `spec/*/reports/` body is created or perturbed; a
new id cannot collide with an existing anchored body. The hard gate is
`scripts/verify_anchors.sh` → **119/119 byte-identical**, run before the arm
lands and after.

**D4 — The robustness classifier and the BH path fn move from the sweep `bin`
into the `backtest` library.** `run_buyhold_path`, `classify_verdict`,
`ParamRobustnessVerdict`, and the `P5_SHARPE_FRAGILE = 0.0` band constants are
bin-private today; the bake-off needs them in the library. This is a
**behaviour-preserving relocation** — the sweep bin is updated to call the moved
items and its output stays byte-identical (identical logic). The FRAGILE band
(p5 Sharpe < 0) and the verdict semantics are unchanged; this ADR does not
re-derive any threshold (those live in the ParamRobustness D-clauses /
ADR-0051's robustness lineage and are merely relocated).

**D5 — The ranking comparator is a pure, total, deterministic function**
(`rank_candidates(&[CandidateResult]) -> Ranking`): eligibility partition on
`Fragile` (a candidate is ineligible to be crowned iff `robustness ==
Some(Fragile)`), then Sharpe descending (`f64::total_cmp`), then total-return
descending (`Decimal`), then max-drawdown ascending (`Decimal`), then strategy
id lexicographic (determinism backstop). The crown is `order[0]`; it is Fragile
iff *all* candidates are Fragile. Buy-and-hold is ranked by the same rule as
every candidate and special-cased only in *copy* (the `BenchmarkWins` outcome)
when it wins. **No f64 arithmetic** is introduced — the comparator only
*compares* metrics already produced by the anchored `stats` layer, so no new f64
determinism boundary is created. Full normative statement: feature.md § F2
ranking contract.

## Alternatives considered

- **Orchestrator in `crates/agent`** — rejected: forces a hard `ui → agent`
  dependency (today optional/dev) to read `BakeoffReport`; `backtest` is already
  a hard `ui` dep so it is the lower-coupling home.
- **Orchestrator in a new `crates/advisor`** — rejected for the MVP: a new crate
  for one loop + one comparator is premature; everything it needs already lives
  in `backtest`. Revisit if the advisor grows a forward-plan + sizing + LLM
  surface that would bloat `backtest`.
- **Rationale as a pre-rendered `String`** — rejected: couples engine to UI copy,
  defeats deterministic snapshotting, and blocks the not-advice-disclaimer
  ownership + the v0.2 LLM-narration layering.
- **Buy-and-hold computed inline in the orchestrator (not a `run_scenario`
  arm)** — rejected: makes BH a different code path from every other candidate
  (no `RunReport`, bespoke equity handling), increasing the chance the benchmark
  diverges from how strategies are measured. A uniform `run_scenario` arm keeps
  the benchmark apples-to-apples.
- **Leave the robustness classifier in the bin and duplicate it** — rejected:
  two copies of the FRAGILE rule is exactly the drift the durable contracts
  guard against; relocate-and-share instead.

## Consequences

- The cockpit gains a bake-off leaderboard/recommendation surface that reads
  `backtest::bakeoff::BakeoffReport` through the existing `backtest` seam — **no
  new `ui` dependency** on `strategy`/`exec`/`forecast`/`llm`. Enforced by
  `cargo tree -p ui` staying unchanged (tasks T3.2 / T6.8) and by the layering
  lint that already guards the invariant.
- The 119 anchored backtest body-SHAs stay byte-identical: the `"v0.buyhold"`
  arm is additive + `write_report = false`, and the bin relocation is
  behaviour-preserving. Enforced by `scripts/verify_anchors.sh` → 119/119, run
  before/after (tasks T0.1 / T1.4 / T2.3 / T6.7). If a future change makes the
  bake-off *write* an anchored report, this ADR's anchor-additive proof no
  longer covers it and a fresh anchor decision is required.
- The ranking is reproducible: a fixed `(symbol, window, seed, field)` yields a
  byte-stable `Ranking`. Enforced by the day-1 deterministic e2e (task T6.1) +
  the comparator unit-test matrix (T6.2–T6.6).
- If the bake-off later coarsens bar cadence (4h/daily), it MUST use
  `compute_*_periodic` (ADR-0051 § D6.8), not the `_hourly` fns — feeding coarse
  bars to `_hourly` silently inflates Sharpe ~2–5×. The MVP field is hourly-only;
  this is a guard for the next cadence change.
- Two open questions remain (feature.md § Open questions) and gate *defaults*,
  not the build: OQ-1 (does the single-coin field include the cross-sectional /
  ML arms — analyst) and OQ-2 (robustness-gate interactive cost — operator).
- This ADR does not add, remove, or mutate any of the 9 anchor SHAs in
  `spec/anchors.toml`; the `"v0.buyhold"` arm produces no anchored artifact.

## Changelog
- 2026-06-19 (architect): initial accept. Homes the bake-off orchestrator in
  `crates/backtest`, defines the `BakeoffReport` public seam (structured
  rationale, UI-mirrorable, ui-dep-free), adds the anchor-additive
  `"v0.buyhold"` `run_scenario` arm + relocates `run_buyhold_path` /
  `classify_verdict` from the sweep bin into the library, and fixes the
  deterministic Sharpe-primary + robustness-gated + BH-as-benchmark ranking
  comparator. Feature: `advisor-bakeoff-ranking`.
