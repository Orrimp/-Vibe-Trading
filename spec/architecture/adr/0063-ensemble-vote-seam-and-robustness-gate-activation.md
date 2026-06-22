---
adr: 0063
title: Ensemble signal-vote seam + robustness-gate activation (Bootstrap mode)
status: accepted
date: 2026-06-21
supersedes: none
superseded-by: none
---

# ADR-0063: Ensemble signal-vote seam + robustness-gate activation (Bootstrap mode)

## Context

The 2026-06-19 single-coin-advisor pivot named v0.2 "a mix of strategies". F8
(`spec/advisor-ensemble/feature.md`, `REQ-ADVISOR-ENSEMBLE-001`) is that "mix"
capability, scoped by the analyst and operator-locked to: **combination =
signal vote** (not capital split); **candidate set = exactly two, pre-registered
in code, NO runtime search** (`v0.8.vote.majority` = ≥2-of-3 over
{macd, rsi, bbands}; `v0.8.vote.unanimous` = 4-of-4 over
{sma, macd, rsi, bbands}); **LLM/ML stay narration-only** (nothing from
`crates/llm` or `crates/forecast` enters the ranking path).

Two facts force this ADR's shape:

1. **The robustness gate is inert.** `RobustnessMode` has only a `Skip` variant
   (`crates/backtest/src/bakeoff/mod.rs`), so every live bake-off flag is `None`
   and **nothing is ever `Fragile`**. The classifier `robustness::classify_verdict`
   is correct and FROZEN (ADR-0059 § D4); only the per-candidate *compute-and-feed*
   path is missing. Shipping ensembles — the easiest-to-overfit candidates — into
   a gate that cannot reject them would convert the product from "measured
   robustness" into "data-mining with extra steps". So F8 bundles gate activation.

2. **The `Strategy` trait is frozen** (ADR-0005). An ensemble id has no single
   `Strategy` to register, yet must be reachable identically from the three
   id-string dispatch seams that today know only the singles: `run_scenario`
   (bake-off), `build_registry_for` (F5b forward-paper, ADR-0060 § D6), and
   `StrategyRegistry`.

The hard constraint across both: **`scripts/verify_anchors.sh` must stay
119/119** — activating the gate changes bake-off *outputs* (flags become
`Some(verdict)`) and adding two arms widens the field, and neither may mutate a
byte of any anchored report under `spec/*/reports/`.

## Decision

### D1 — `EnsembleStrategy` is a first-class `Strategy`, the ONE primitive across all three seams

Add `strategy::EnsembleStrategy` — a concrete struct that **implements the frozen
`Strategy` trait** by holding `members: Vec<Box<dyn Strategy>>` + a pure arbiter,
and reduces their per-bar signals to its own `Vec<Signal>` inside `on_bar`. This
is the `RegimeDispatcher` precedent (`crates/strategy/src/regime_dispatcher.rs`:
a `Strategy` that wraps inner strategies and arbitrates per bar) generalised to N
homogeneous members + a consensus rule. Because it *is* a `Strategy`, it becomes
a normal registry citizen: all three seams construct it from ONE id→members
mapping and never special-case "ensemble" anywhere downstream. The trait is NOT
modified (ADR-0005 honoured).

- **Members are reused, not reimplemented.** The ensemble constructs each member
  through the **existing per-id construction** — the same rule engines + their
  TOMLs that `build_registry_for` already loads (`SmaCrossover` for `v0.sma`;
  `ComposedStrategy` from `config/strategies/{btc_macd_trend,btc_rsi_reversion,
  btc_bbands_mean_revert}.toml` for the v0.5 arms). One shared
  `build_member(id) -> anyhow::Result<Box<dyn Strategy>>` constructor is the
  single source of member truth, called by the ensemble factory at all three
  seams. No member logic is duplicated.

- **The arbiter is a pure, deterministic free function.** `arbitrate(method,
  &member_stances) -> Stance` where `Stance ∈ {Long, Flat}`. The ensemble tracks
  each member's **current LONG/FLAT stance** (edge-triggered Buy flips it Long,
  Sell flips it Flat — the same stance the singles' equity curves express), then
  on each bar recomputes the consensus and emits a Buy/Sell **only on the
  ensemble's own stance transition** (edge-triggered, matching `ComposedStrategy`
  emission semantics). Two methods, frozen in code:
  - `VoteMethod::Majority { k, n }` (2-of-3): ensemble Long iff `long_count ≥ k`.
  - `VoteMethod::Unanimous { n }` (4-of-4): ensemble Long iff `long_count == n`.

- **Warmup / abstention rule (the honest count).** A member that has not warmed
  up yet (its indicators have produced no value — `last_rule_value == None` /
  SMA `current() == None`) is **NOT counted as a "no"/FLAT vote**. It is
  **abstaining**: it contributes neither to `long_count` nor to the effective
  denominator. The consensus is evaluated against the **warmed-member quorum**:
  - The ensemble is **FLAT (no signal, abstains itself) until ALL members it
    needs are warmed.** Concretely: `Majority{k=2,n=3}` requires ≥ `k` warmed
    members before it can ever vote Long; `Unanimous{n=4}` requires all 4 warmed.
    Until the quorum is warm the ensemble emits nothing and reports stance Flat.
    This is the only honest rule: counting an un-warmed member as FLAT would let
    a 2-of-3 majority be *manufactured* by two early-warming members while the
    third is structurally silent, and would make `unanimous` trivially never-Long
    in a way that hides rather than measures consensus. Rationale recorded so the
    developer cannot "simplify" it to treat `None` as FLAT.
  - Determinism: warmup is a pure function of the bar sequence each member sees,
    which is identical (the registry fans the same bar to all). Same bars ⇒ same
    warmup ⇒ same votes.

- **`quantity_scale` is the default `1.0`** (ensembles are signal combiners, not
  sizing modifiers — sizing is F4's `budget_cap`, untouched).

### D2 — `EnsembleStrategy` carries the `PlanDescribe` impl (OQ-ARCH-3 carrier)

The ensemble implements the ADR-0062 read-only sibling trait `PlanDescribe`
(NOT a `Strategy` method). `describe_plan` is a non-mutating read of each member's
already-warmed stance + the arbiter, returning `PlanRuleShape::Ensemble`
(D3). Same non-mutation contract as the singles: it reads member stances via
their existing non-mutating getters (`ComposedStrategy::last_rule_value`,
`SmaCrossover` fast/slow `current()`), pushes nothing, and is deterministic.

### D3 — `PlanRuleShape::Ensemble { method, members }` (closed-enum extension, no free text)

Extend the closed `PlanRuleShape` enum (`crates/strategy/src/plan.rs`) with one
variant carrying **structured data only**:

```
Ensemble {
    method: PlanVoteMethod,                 // closed: Majority { k, n } | Unanimous { n }
    members: Vec<PlanRuleShape>,            // each member's own rule shape
}
```

`PlanVoteMethod` is a new closed enum (NOT a string). The `ui` exhaustive match
gains an `Ensemble` arm and renders honest copy that **names the method + members**
— e.g. "Holds when ≥ 2 of {MACD trend, RSI reversion, Bollinger reversion} agree;
goes flat when the majority flips" + the live tally — NOT a fabricated
single-indicator rule. The `members` carry the real per-member shapes so the UI
can list each member's own rule. The `agent` boundary maps
`strategy::PlanRuleShape::Ensemble` → a new `agent::PlanRuleKind::Ensemble` (and
`ui` mirrors it), in lockstep, preserving the no-`String`-crosses-the-seam
discipline (ADR-0059 / ADR-0062 § D1, D4). The no-free-text seam is kept end to
end; `cargo tree -p ui` is unchanged.

### D4 — `RobustnessMode::Bootstrap` — the missing compute-and-feed path (OQ-ARCH-2)

Add `RobustnessMode::Bootstrap { paths: usize, seed: u64 }` (default stays
`Skip`). Per candidate, AFTER its `run_scenario` produces a realized equity
curve, compute the verdict and populate `CandidateResult.robustness`:

1. Map the candidate's realized equity curve → a log-return series `r[0..N]`
   (the existing `derive_candidate_kpis` mapping `equity → Vec<Decimal>` →
   returns; reuse, do not re-derive).
2. **Block length** is chosen automatically by the **existing**
   `data::synth::block_length::politis_white_block_length(&returns_f64)` (the
   Politis–White PWSD selector already in the tree, used by the monte_carlo gate)
   — NOT a magic constant. This adapts the block to the series' own
   autocorrelation and is the project's established choice (ADR-0051).
3. **Resample**: draw `paths` moving-block-bootstrap resamples of the return
   series. Determinism is mandatory and uses the **frozen ADR-0051 D1 sub-seed
   rule**: `path_seed_j = master_seed.wrapping_add((j as u64).wrapping_mul(
   0x9E37_79B9))`, each path seeding **one** `ChaCha20Rng::seed_from_u64(
   path_seed_j)` (the `BlockBootstrapPathGen` precedent). `master_seed` is
   **derived from the bake-off seed** so the gate is reproducible for a given
   bake-off run: `master_seed = seed_to_u64(req.seed) ^ candidate_index_salt`
   (a fixed per-candidate salt so two candidates do not share a resample draw;
   the salt is a frozen constant table, not a search).
4. Per path: rebuild a synthetic equity curve from the resampled returns, compute
   `PathMetrics` (Sharpe/Sortino/Calmar/max_dd/total_return + final/initial for
   P(loss)) via the **existing** annualised stat fns.
5. Reduce the `paths` `PathMetrics` via the **existing**
   `DistributionSummary::from_path_metrics` and classify via the **FROZEN**
   `classify_verdict(&summary)` → `RobustnessFlag`. The classifier and its bands
   are NOT touched (ADR-0059 § D4 freeze).

`paths` default = **1000** (the monte_carlo daily default; tractable for a
≤7-arm field on one window). The whole compute is a **pure function of (returns,
master_seed, paths)** — re-running a bake-off with the same seed yields the same
flags.

**Multiple-comparisons hardening — re-evaluated, decision: NOT YET (no
correction at a 7-arm field).** Re-evaluated rather than assumed:
- The gate is **not a winner-selection p-value** — `classify_verdict` is an
  absolute per-candidate band rule (p5 Sharpe < 0, p50 < 0.5, prob_loss > 0.35,
  etc.), not a "is this the best of K" significance test. Widening the field does
  NOT inflate a per-candidate false-positive rate the way K independent
  significance tests would; each candidate is judged against fixed thresholds on
  its **own** resampled distribution.
- The pre-registration lock (R3) already bounds researcher DoF: 2 fixed votes +
  4 fixed singles + buy-and-hold = 7, none tuned. The classic multiple-comparisons
  hazard (search K configs, crown the luckiest) is structurally absent — there is
  no search.
- The crown still must clear `BeatBenchmarkSharpe` AND be non-Fragile; an
  ensemble that wins one path but is Fragile under resampling cannot be crowned
  (unless all-Fragile). That is the anti-overfit lock, and it bites per-candidate
  regardless of field size.
- **Trigger to revisit (recorded):** if a future version makes the candidate set
  *grow with data* or introduces any selection-over-configs, a Bonferroni/Šidák
  band tightening or a White's-Reality-Check / SPA test becomes required and MUST
  get its own ADR. At a fixed, pre-registered 7-arm field it is over-engineering.

### D5 — ANCHOR SAFETY: the new mode + arms are ADDITIVE; anchored paths stay frozen

`verify_anchors.sh` stays **119/119 BY CONSTRUCTION**, guaranteed by *which call
sites change and which stay frozen*:

- **`RobustnessMode::default()` stays `Skip`.** Every existing caller that
  constructs a `BakeoffConfig` without naming the mode — and every anchored CLI
  path — keeps `Skip`, so flags stay `None`, so ranking is byte-unchanged. Only
  the **advisor bake-off path** (the cockpit `spawn_bakeoff`, which writes NO
  report — ADR-0059 § D3 `write_report = false`) opts into
  `RobustnessMode::Bootstrap`. The bootstrap compute reads equity and writes only
  in-memory `RobustnessFlag`s; it **writes no file**.
- **The ensemble arms are opt-in field entries**, exactly like the `v0.buyhold`
  arm (ADR-0059 § D3), the dynamic-data cache (ADR-0061 § D4), and the F4
  `budget_cap` (ADR-0060 § D1) — "new path is opt-in, existing anchored bytes
  untouched". `BakeoffConfig::default_field()` is **unchanged** (the 4 singles);
  a new `default_ensemble_field()` adds the two votes, and the advisor caller
  composes `default_field() ∪ default_ensemble_field()`. No anchored test or CLI
  invocation passes the ensemble ids, so no anchored report path runs an ensemble.
- **`run_scenario`'s ensemble dispatch arm writes no report.** The new id arm
  (D6) is reached only by the advisor bake-off (`write_report = false`) and tests;
  it constructs the `EnsembleStrategy` and runs the same single-position paper
  engine the singles use. The existing single-id arms are **byte-untouched** (no
  edit to the `v0.sma` / `v0.5.*` blocks).
- **No anchored report file is opened, edited, or re-emitted; no `anchors.toml`
  SHA changes; no `data/*/REVISION.toml` is touched.** F8 introduces **no new
  anchored scenario** (feature § Backtest Scenarios). The developer MUST run
  `scripts/verify_anchors.sh` before the first seam and after the last and
  confirm 119/119; any non-119 result is a STOP-and-route-back.

This is the operator's locked precedent applied verbatim: the new
`RobustnessMode::Bootstrap` and the ensemble arms are additive; existing anchored
CLI/report paths keep `RobustnessMode::Skip` / the 5-arm (4-singles + buyhold)
field and stay byte-identical.

### D6 — `run_scenario` + `build_registry_for` ensemble dispatch

- **`run_scenario` (`crates/backtest/src/engine.rs`)** gains ONE arm matching the
  two ensemble ids (`v0.8.vote.majority` / `v0.8.vote.unanimous`). The arm builds
  the `EnsembleStrategy` via the shared factory and runs it through the **same
  single-symbol per-bar paper engine** the `v0.5.*` composed arms use (one
  position, one equity curve) — producing a normal `RunReport` consumed by
  `derive_candidate_kpis` unchanged. `write_report = false` on this path.
- **`build_registry_for` (`crates/agent/src/runtime.rs`)** gains the same two ids,
  each registering ONE `EnsembleStrategy` (whose members are built from the SAME
  TOMLs the bake-off scored — F5b byte-for-byte fidelity, ADR-0060 § D6). A
  crowned ensemble therefore forward paper-trades through the unchanged
  `paper_loop_supervisor` hot-swap, and its plan is described by the ensemble's
  `PlanDescribe` impl — all three seams reached from one primitive, zero
  special-casing.

### D7 — TWO day-1 divergence e2e gates (CLAUDE.md non-negotiable)

Per the `v3-volatility-forecaster-noop` precedent, the developer ships F8 with
TWO e2e gates (the tester owns running them; both are FAIL-before / PASS-after):

- **(a) The vote actually combines** —
  `crates/strategy/tests/ensemble_vote_divergence_end_to_end.rs`: drive the
  `v0.8.vote.majority` `EnsembleStrategy` AND each of its three members
  individually over a fixture window engineered so the members **disagree on ≥ 1
  bar**, and assert the ensemble's realized equity curve **diverges from each
  member's curve (and from any single member's pass-through) by ≥ 1 bp**. This
  catches a silent passthrough (the arbiter computed but emitting one member's
  signal — the v3-vol no-op analogue). Modelled on
  `vol_targeting_overlay_end_to_end.rs`.
- **(b) The gate actually bites (anti-`Skip`-regression)** —
  `crates/backtest/tests/robustness_bootstrap_bites.rs`: feed a deliberately
  overfit / fragile candidate (an equity curve whose resampled p5 Sharpe < 0)
  through `RobustnessMode::Bootstrap` and assert it is flagged `Fragile` AND
  loses the crown to a robust single; AND feed a robust candidate (resampled
  distribution clears all ROBUST bands) and assert it is **NOT** flagged (no
  false-positive). This proves the gate is no longer inert and is the regression
  guard against anyone reverting the advisor path to `Skip`.

Plus the F2 reachability regression (R4.4): a field where buy-and-hold has the top
eligible Sharpe still yields `BenchmarkWins` **with ensembles present**, and an
all-Fragile field (singles + ensembles) still yields `AllFragile`.

## Alternatives considered

- **External arbiter driving N registry members (R5.1.ii)** — rejected: it
  changes the `StrategyRegistry`/supervisor loop and special-cases ensembles at
  every call site, the exact 3-site duplication D1 avoids. The
  `EnsembleStrategy: Strategy` adapter makes the vote a first-class citizen with
  one primitive.
- **Capital-split combination** — out of scope (operator-locked to signal-vote);
  named v0.3 fallback in the feature. Its summed-sleeve equity has murkier
  block-bootstrap semantics and triples the F4/F5/F6 surface.
- **Treat un-warmed members as FLAT votes** — rejected: manufactures false
  majorities from early-warming members and hides (rather than measures)
  consensus. The abstention-quorum rule (D1) is the honest count.
- **A magic fixed block length for the bootstrap** — rejected: the tree already
  has the Politis–White automatic selector (ADR-0051); a hand-picked constant is
  a hidden researcher DoF.
- **Re-running the strategy per bootstrap path (bar-level resample,
  `BlockBootstrapPathGen`)** — rejected for the gate: heavier (re-runs the engine
  N×K times) and unnecessary — the feature scopes the gate to resampling the
  candidate's **realized returns**, which is the cheaper, standard block-bootstrap
  robustness read and reuses `DistributionSummary` + `classify_verdict` directly.
- **Adding a multiple-comparisons correction now** — rejected at a fixed 7-arm
  pre-registered field (D4 reasoning); recorded as a must-ADR trigger if the set
  ever grows with data or gains a selection-over-configs step.
- **A free-text plan string for the ensemble** — rejected: violates the ADR-0059
  / ADR-0062 no-`String`-crosses-the-seam discipline. `PlanRuleShape::Ensemble`
  carries structured `method` + `members`; the `ui` owns the copy.

## Consequences

- **What breaks if D5 is violated:** any edit to the existing single-id
  `run_scenario` arms, any change to `default_field()`, any anchored caller
  opting into `Bootstrap`, or any report write on the ensemble path → mutates an
  anchored body-SHA → `verify_anchors.sh` < 119 → REGRESSION. Mechanically caught
  by `scripts/verify_anchors.sh` (run before+after) — the developer gate.
- **What breaks if D1's warmup rule is "simplified":** treating `None` as FLAT
  silently changes vote outcomes (manufactured majorities) — caught by the
  divergence e2e (D7a) only if the fixture exercises the warmup boundary; the rule
  is recorded here so it is not "optimised away" in review.
- **What breaks if the gate is reverted to `Skip` on the advisor path:** D7b
  (`robustness_bootstrap_bites`) FAILS — the overfit candidate would no longer be
  flagged. This is the explicit anti-`Skip`-regression gate.
- **Frozen surfaces honoured:** `Strategy` trait (ADR-0005), `classify_verdict` +
  bands (ADR-0059 § D4), the F2 `rank_candidates` comparator (ADR-0059 § D5), the
  F5 supervisor contract (ADR-0060 § D6), the F6 `PlanDescribe` non-mutation
  contract (ADR-0062). F8 adds members to these contracts; it amends none.
- `cargo tree -p ui` unchanged (ensemble result reaches the cockpit through the
  existing `BakeoffReport` mirror; the plan through the existing `ForwardPlan`
  mirror).

## Changelog
- 2026-06-21 (architect): initial accept. Ensemble signal-vote seam
  (`EnsembleStrategy: Strategy` + pure arbiter + abstention-quorum warmup rule,
  D1), `PlanDescribe`/`PlanRuleShape::Ensemble` extension (D2, D3),
  `RobustnessMode::Bootstrap` compute-and-feed reusing Politis–White block length
  + ADR-0051 sub-seed determinism + frozen `classify_verdict` (D4, multiple-
  comparisons NOT-yet decision recorded), anchor-safety-by-construction additive
  contract (D5, 119/119), `run_scenario` + `build_registry_for` dispatch (D6), and
  the two day-1 divergence/anti-`Skip` e2e gates (D7). Leans on ADR-0059
  (bake-off field + `rank_candidates` + `RobustnessFlag` seam), ADR-0060 (§ D6
  forward-paper hot-swap), ADR-0062 (`PlanDescribe` sibling trait), ADR-0005
  (`Strategy` freeze), ADR-0051 (block-bootstrap determinism + Politis–White
  selector).
