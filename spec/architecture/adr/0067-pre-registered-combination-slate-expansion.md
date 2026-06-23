---
adr: 0067
title: Pre-registered combination-slate expansion of the advisor field (6 new vote arms, falsifier-slate methodology)
status: accepted
date: 2026-06-23
supersedes: none
superseded-by: none
---

# ADR-0067: Pre-registered combination-slate expansion of the advisor field

## Context

The operator (2026-06-23): *"combinations of the strategies could yield good
result — we need to calculate the combination of multiple strategies."*
**Decorrelation** is the one legitimate lever that can move a return distribution
Fragile → Robust **without touching the gate**: blending two weak-but-uncorrelated
edges tightens the path-spread, which lifts the **p5 (tail) Sharpe** — the single
binding constraint in the frozen `classify_verdict`
(`crates/backtest/src/bakeoff/robustness.rs`). But *searching* combinations for
the best in-sample return is textbook overfitting; an unconstrained search over
membership/weights/thresholds will always surface something that beat
buy-and-hold on the realized path, silently converting the product from "measured
robustness, not asserted alpha" into "data-mining with extra steps".

F8 (ADR-0063) already built and froze every mechanism a combination needs
(`EnsembleStrategy`, `VoteMethod::{Majority{k,n}, Unanimous{n}}`, the pure
`arbitrate`, the `RobustnessMode::Bootstrap` gate, the anchor-additive
`write_report=false` advisor path) and shipped **two** pre-registered vote arms as
a proof of seam. This ADR governs **widening that bounded set** to a fuller — still
FIXED, still declared-in-code — slate, so that more candidates may **compete
without cheating**. The honest expectation (the robustness program concluded
2026-06-08 with whole families uniformly Fragile; the live 7-arm field is
all-Fragile → modal `BenchmarkWins`, ADR-0066) is that **this finds no alpha**;
the deliverable is an honest experiment with a pre-registered falsifier slate, and
a null result ("all combinations also Fragile, hold stands") is a valid + expected
+ shippable finding.

## Decision

Add **6 new pre-registered `EnsembleStrategy` arms** to the live advisor field,
each scored through the **byte-frozen** `RobustnessMode::Bootstrap` gate + the
ADR-0066 buy-and-hold benchmark. The methodology and the freeze are pinned by the
D-clauses below.

### D1 — The slate is a FIXED, code-declared falsifier set (the overfit defense)

The candidate set is exactly these 6 arms, declared as **literal `match` arms** in
`strategy::build_ensemble` (over the existing 4 base signals
`{v0.sma, v0.5.macd, v0.5.rsi, v0.5.bbands}`) — **NO runtime search** over
membership, weights, or k-thresholds:

| id | VoteMethod | members | role |
|---|---|---|---|
| `v0.8.vote.trend_pair` | `Unanimous{n:2}` | `[v0.5.macd, v0.sma]` | **predicted-null control** |
| `v0.8.vote.tr_mr_macd_rsi` | `Unanimous{n:2}` | `[v0.5.macd, v0.5.rsi]` | trend ∧ mean-revert |
| `v0.8.vote.tr_mr_sma_bb` | `Unanimous{n:2}` | `[v0.sma, v0.5.bbands]` | trend ∧ band-reversion |
| `v0.8.vote.any1of4` | `Majority{k:1,n:4}` | all 4 | k-ladder rung 1 |
| `v0.8.vote.k2of4` | `Majority{k:2,n:4}` | all 4 | k-ladder rung 2 |
| `v0.8.vote.k3of4` | `Majority{k:3,n:4}` | all 4 | k-ladder rung 3 |

Pre-registration is overfit-safe **by construction**: with no optimization step
there is no in-sample fitting to leak. The set is **principled, not exhaustive**
(NOT "every subset of 4" — that is a search dressed as a slate): two
trend∧mean-revert pairings (the real lever), one trend∧trend **control** with a
*predicted* outcome (little p5 lift — if it lifts as much as the mixed pairs, the
decorrelation thesis is **falsified**), and the **complete** k-of-4 ladder
(k∈{1,2,3}; k=4 is the existing `v0.8.vote.unanimous`) so the **whole** ladder is
reported and no "best k" is ever cherry-picked. Total live field becomes 4 singles
+ 8 ensembles + buy-and-hold = **13 arms**.

### D2 — LITERAL ids, NOT a parser (preserve the pre-registration lock)

The 6 ids are added as literal `build_ensemble` arms; the `run_scenario` engine
dispatch (`crates/backtest/src/engine.rs`) — which **already calls
`build_ensemble(strategy_str)` generically** — has only its `match` *pattern*
widened to alternate the new ids. An id **grammar / runtime parser**
(`vote.k{K}of{N}`, membership-from-id) is REJECTED: a grammar is a parameterized
space that edges toward search and breaks "pre-registered = a finite enum you can
read in the diff". Each arm's `(id, VoteMethod, members)` tuple is visible in the
source — that auditability is the overfit defense.

### D3 — `default_ensemble_field()` is the single source of truth

The 6 ids are added only to `BakeoffConfig::default_ensemble_field()`
(`crates/backtest/src/bakeoff/mod.rs`); `advisor_field()`
(`crates/ui/src/leaderboard/runner.rs`) = `default_field() ∪
default_ensemble_field()`, so they flow into the live cockpit field automatically
with NO `ui` edit and NO duplicated list. `default_field()` (the 4 singles) is
UNCHANGED. The one runner field-count test that pins the field moves in lockstep
(len 6→12) — a test tracking its contract, not a loosening. Latency: 13 arms ×
1000 paths ≈ +86% bake-off wall-clock vs 7 today; this is within budget on the
**determinate-progress, operator-triggered, on-demand** bake-off path (no
real-time SLA) → no progress-UX change, only an honest arm-count note in the
header.

### D4 — Gate + bands + comparator FROZEN; "more candidates face the same bar"

`classify_verdict` / `compute_robustness_flag` / `verdict_bands`
(`crates/backtest/src/bakeoff/robustness.rs`) and `bootstrap.rs` are
**byte-UNCHANGED**. `rank_candidates` + the ADR-0066 benchmark exemption
(`crates/backtest/src/bakeoff/rank.rs`) are **UNCHANGED** — a combination is just
another `CandidateResult`, scored on its own realized equity, crown-eligible only
if `robustness != Some(Fragile)`. This is **NOT** a B2/B3 band proposal (those
were operator-REJECTED); it is framed strictly as *more candidates facing the same
bar*, never *moving the bar*. The ADR-0059 §D5 anti-overfit eligibility lock + the
ADR-0063 §D4 classifier freeze + the 2026-05-30 pre-registration are reaffirmed.
`BenchmarkWins` / `AllFragile` reachability is UNCHANGED (ADR-0066): a null
all-Fragile field → `BenchmarkWins` remains reachable with the 6 new arms present,
and is the most likely + a fully shippable outcome.

### D5 — Honest mostly-Flat rendering (no engineering-around)

The `Unanimous{n:2}` trend∧mean-revert pairs require both members simultaneously
warmed AND Long; a trend follower and a mean-reverter fire in different regimes, so
these arms may sit **mostly Flat → near-zero return → their own Fragile flag**.
This is HONEST — the literal property of a strict-consensus combo of decorrelated
members — **not a bug**, and is **not** engineered around. It is rendered
truthfully by the existing surfaces (the real low `trade_count` + the computed
`Fragile` flag on the leaderboard row; the B1 "sat in cash" / "Nothing beat simply
holding…" copy; the forward-plan `PlanRuleShape::Ensemble` honest "≥ k of {…}
agree" description) with NO new flag, state, or special-case.

### D6 — Anchor-safe by construction + the day-1 divergence gate

The new `v0.8.vote.*` ids run with `write_report=false` on the
`RobustnessMode::Bootstrap` advisor path (the F8 contract, ADR-0063 §D5); a new id
cannot collide with any anchored report body. `scripts/verify_anchors.sh` stays
**119/119 byte-identical** (run before the first seam + after the last; anchors
keyed by NAME not filename); no `anchors.toml` SHA / `REVISION.toml` /
`spec/*/reports/` body is touched — the **9 anchor SHAs in `spec/anchors.toml` are
NOT mutated**, so no anchor-mutation ADR is triggered. Per the CLAUDE.md
non-negotiable, the slate ships a **day-1 baseline-equity-divergence e2e**
(`crates/strategy/tests/combination_slate_divergence_end_to_end.rs`, modelled on
the F8 `ensemble_vote_divergence_end_to_end.rs`): each new arm's equity diverges
≥ 1 bp from at least one member AND from buy-and-hold + no two new arms are
identical (no silent no-op, no accidental duplicate), with the vote-mechanics
proven via `SmaCrossover` members (TOML members don't fire on synthetic bars — the
F8 precedent) + a factory smoke over the real 4 base TOMLs. Determinism unchanged
(reuses the frozen ADR-0051 sub-seed path; no new f64 boundary, no new RNG). The
render proof is a populated 13-row leaderboard snapshot + the ensemble-rule
description snapshot (verify-UI-at-render-layer), not a unit test.

## Alternatives considered

- **Generalized id-grammar / runtime dispatch parser** — rejected (D2): an id
  grammar is a parameterized space that edges toward search and forfeits the
  finite-enum-in-the-diff pre-registration lock.
- **Exhaustive every-subset-of-4 slate** — rejected (D1): a search dressed as a
  slate; principled decorrelation hypotheses + the whole k-ladder is the
  falsifier set.
- **Weighted / inverse-vol / conditional-regime blends in v1** — DEFERRED to a
  v0.2 of this feature: each needs a new `VoteMethod` variant + `arbitrate`
  branch + `PlanVoteMethod` mirror + `ui` match arm, and a continuous weight is a
  free knob (overfit risk) unless rule-derived; the one defensible case
  (inverse-vol, rule-derived) must ship with the day-1 divergence e2e when built.
- **Loosen / asset-class-tune the robustness bands (B2/B3)** — rejected (D4),
  re-affirming the operator rejection: it would break the ≤18 θ-surface anchors
  and forfeit the pre-registration moat. The honest fix is to let more candidates
  face the SAME bar.
- **A combination-SEARCH engine** — out of scope (a guarded follow-on, feature
  R5): if ever built it MUST ship a walk-forward/out-of-sample split + a
  complexity penalty (deflated-Sharpe / Bonferroni) + a pre-registered search
  procedure + a loud risk call-out, and needs its own ADR.

## Consequences

- Breaking the freeze (any edit to `classify_verdict` / `verdict_bands` /
  `compute_robustness_flag` / `bootstrap.rs`, or to `rank_candidates`) violates D4
  and is caught by the existing F8 + ADR-0066 test suites
  (`crates/backtest/tests/robustness_bootstrap_bites.rs`,
  `crates/backtest/src/bakeoff/rank.rs` tests) plus the byte-frozen classifier
  unit tests.
- Anchor safety is mechanically enforced by `scripts/verify_anchors.sh` (must stay
  119/119 before + after); a non-119 means an arm wrongly wrote a report
  (`write_report=true`) — a wiring bug, not a band change.
- The day-1 divergence is enforced by
  `crates/strategy/tests/combination_slate_divergence_end_to_end.rs`
  (FAIL-before/PASS-after: aliasing any arm to an existing id breaks it).
- The render scaling + honest rule description are enforced at the pixel layer by
  `crates/ui/tests/leaderboard_populated_render.rs` (13-row guard + empty negative
  control) and the F6 ensemble-naming render guard.
- This ADR does NOT add or change any of the 9 anchor SHAs in `spec/anchors.toml`.

Leans on ADR-0063 (the ensemble + gate seam this expands), ADR-0066 (benchmark
exemption + `BenchmarkWins`/`AllFragile` reachability), ADR-0059 (the bake-off
orchestrator + comparator + classifier freeze).

## Changelog
- 2026-06-23 (architect): initial accept. Pins the pre-registered 6-arm
  combination slate (3 `Unanimous{n:2}` decorrelation pairs incl. the `trend_pair`
  predicted-null control + the complete k∈{1,2,3}-of-4 ladder), the
  literal-ids-not-parser dispatch (D2), the single-source-of-truth field (D3), the
  band/comparator freeze framed "more candidates, same bar" (D4), honest
  mostly-Flat rendering (D5), and anchor-safety-by-construction + the day-1
  divergence gate (D6). For feature `advisor-combination-search`.
