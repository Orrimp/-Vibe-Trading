---
slug: advisor-combination-search
status: arch-done
owner: architect
updated: 2026-06-23
---

# Expand the strategy-combination space in the bake-off (overfit-safely)

> **One-line framing:** F8 ([`../advisor-ensemble/feature.md`](../advisor-ensemble/feature.md))
> shipped **two** pre-registered vote ensembles as a proof of seam. This feature
> **widens that bounded, pre-registered set** to a fuller — still fixed,
> still declared-in-code — slate of decorrelated combinations, each scored
> through the **identical** frozen robustness gate + the **identical** buy-and-hold
> benchmark. The goal is to discover whether **any** decorrelated combination
> *survives the gate* on real crypto — **not** to manufacture a winner. If they
> all stay Fragile, "just hold" stands, and that is a real, honest finding.

## Why

**Operator request (2026-06-23):** *"combinations of the strategies could yield
good result — we need to calculate the combination of multiple strategies."*

Combining strategies is the one **legitimate, well-understood lever** that can
move a return distribution from Fragile → Robust without touching the gate:
**decorrelation tightens the path-spread, which lifts the p5 (tail) Sharpe** —
the single binding constraint in `classify_verdict` (`p5 Sharpe < 0 → Fragile`,
`crates/backtest/src/bakeoff/robustness.rs`). Two weak-but-uncorrelated edges
blended can have a materially higher worst-5%-path Sharpe than either alone,
because their bad paths don't coincide. That is real portfolio math, not
data-mining.

**But it is also the single sharpest honesty hazard in the whole product** —
F8 said this first and it is doubly true here. *Searching* combinations for the
best in-sample return is **textbook overfitting**: every member you add
multiplies researcher degrees of freedom, and an unconstrained search over
membership / weights / thresholds will **always** surface something that beat
buy-and-hold on the realized path. The robustness gate exists precisely to catch
that. So a combination feature built naively silently converts the product from
*"measured robustness, not asserted alpha"* into *"data-mining with extra
steps"* — destroying the one thing that makes the bake-off trustworthy
(`product.md § Why this is honest`, the 2026-06-08 ship-passive verdict).

**The design problem is therefore not "how to combine" (mechanically trivial —
`EnsembleStrategy` already does it) — it is "how to let more combinations
compete without letting them cheat."** The answer is **pre-registration**: a
FIXED, declared-up-front slate of combination arms, each scored through the SAME
existing bootstrap gate. No search ⇒ no in-sample optimization ⇒ overfit-safe by
construction. This brief recommends **pre-registered-only for v1** and quarantines
any search engine to a loudly-flagged, guarded follow-on.

**Honest expectation (do not skip this):** the robustness program already found
whole strategy *families* (momentum, mean-reversion, cross-sectional,
time-series-momentum, horizon variants) uniformly Fragile on real crypto
(`CHANGELOG.md`: *"Robustness program — CONCLUDED 2026-06-08 → ship passive"*),
and the live 7-arm advisor field (4 singles + 2 ensembles + buy-and-hold) comes
back **all-Fragile → modal `BenchmarkWins`** (ADR-0066, just shipped). This
feature is **not expected to magically find alpha.** Decorrelation only works if
the members carry **real, even weak, edge** — combining Fragile-AND-correlated
strategies cannot help (you can't diversify away a common lack of signal). The
deliverable is an **honest experiment with a pre-registered falsifier slate**,
not a winner-finder. A null result ("all combinations also Fragile, hold stands")
is a **valid, shippable outcome** and the most likely one.

## Relationship to F8 (this is an expansion, not a new mechanism)

F8 already built and froze every mechanism this feature needs:

| F8 asset (shipped, ADR-0063) | Reused verbatim here |
|---|---|
| `strategy::EnsembleStrategy` (fan bar → members, arbitrate, edge-triggered emit) | yes — new arms are new `(method, members)` tuples, same struct |
| `strategy::VoteMethod::{Majority{k,n}, Unanimous{n}}` | yes — **generic k-of-n is ALREADY representable**; new vote-threshold arms need **zero new arbitration code** |
| `strategy::arbitrate` (Unwarmed abstains; quorum before Long) | yes — unchanged pure function |
| `strategy::build_ensemble(id)` factory + `build_member(id)` | extend the `match` with new ids; same construction path (the real TOMLs) |
| `run_bakeoff` arm dispatch (`engine.rs` `"v0.8.vote.*"` match) | widen the match arm (or generalize id→`build_ensemble`) |
| `rank_candidates` Fragile-ineligible comparator + benchmark exemption (ADR-0066) | **unchanged** — a combination is just another `CandidateResult` |
| `RobustnessMode::Bootstrap` gate (Politis–White block length, frozen `classify_verdict`) | **unchanged + byte-frozen** — scored on each arm's own realized equity |
| `BakeoffConfig::default_ensemble_field()` / `advisor_field()` wiring | extend the returned `Vec<StrategyId>` |
| `PlanRuleShape::Ensemble` + `PlanVoteMethod` (F6 plan seam) | reused; new k-of-n arms slot in with no new variant |

The net new surface for **vote-threshold** combinations is: a few new ids in
`build_ensemble`, the widened engine match arm, the extended field list, and the
gate runs as-is. That is the MVP.

## Requirements

### R1 — v1 is **pre-registered-only**. No search. (the crux)

The candidate set is a **FIXED, code-declared slate** — declared in
`build_ensemble` + the bake-off field exactly like F8's two arms. There is
**NO runtime search** over membership, weights, k-thresholds, or any parameter.
Pre-registration is overfit-safe **by construction**: with no optimization step,
there is no in-sample fitting to leak. The slate is a **falsifier set** — chosen
*before* seeing results, frozen, and reported whether it wins or loses. This is
the same lock F8 used (its § "honest F8 definition" sentence 2) and it is
**non-negotiable for v1**.

### R2 — Recommended v1 combination slate (bounded, pre-registered)

All members are drawn from the existing 4 base signals
`{v0.sma, v0.5.macd, v0.5.rsi, v0.5.bbands}` — no new signal types in this
feature (that is a sibling backlog item, see § Non-goals). All arms below are
**vote ensembles over those 4**, so they need **zero new arbitration math**
(`VoteMethod::Majority{k,n}` / `Unanimous{n}` already express every one). The
recommended slate adds **6 new pre-registered arms** to the existing 2:

**Decorrelation pairings (the real lever — pair a trend follower with a
mean-reverter, whose bad paths are least likely to coincide):**

| New id | Method | Members | Rationale |
|---|---|---|---|
| `v0.8.vote.trend_pair` | `Unanimous{n:2}` | `[v0.5.macd, v0.sma]` | both trend — *control*: correlated members ⇒ expect little p5 lift (sanity check the thesis) |
| `v0.8.vote.tr_mr_macd_rsi` | `Unanimous{n:2}` | `[v0.5.macd, v0.5.rsi]` | trend ∧ mean-revert — only long when a trend-up AND an oversold-bounce agree |
| `v0.8.vote.tr_mr_sma_bb` | `Unanimous{n:2}` | `[v0.sma, v0.5.bbands]` | trend ∧ band-reversion — second decorrelated pairing |

**k-of-n threshold sweep over all 4 (pre-registered, NOT searched — the full
fixed ladder, so no "best k" is cherry-picked):**

| New id | Method | Members | Rationale |
|---|---|---|---|
| `v0.8.vote.any1of4` | `Majority{k:1, n:4}` | all 4 | loosest — long if *any* fires (most exposure, lowest decorrelation benefit) |
| `v0.8.vote.k2of4` | `Majority{k:2, n:4}` | all 4 | balanced quorum |
| `v0.8.vote.k3of4` | `Majority{k:3, n:4}` | all 4 | strict — long only on broad agreement (tightest spread, fewest trades) |

Existing arms kept unchanged: `v0.8.vote.majority` (`Majority{k:2,n:3}` over
macd/rsi/bbands) and `v0.8.vote.unanimous` (`Unanimous{n:4}` over all 4). Total
advisor field becomes **4 singles + 8 ensembles + buy-and-hold = 13 arms**.

**Why this exact set (the pre-registration rationale — must be recorded so it is
auditable as chosen-before-results):**
- It is **principled, not exhaustive.** It is NOT "every subset of 4" (= 11
  non-trivial vote combos × multiple k each = a search dressed as a slate). It is
  the **decorrelation hypothesis made falsifiable**: two trend∧mean-revert
  pairings (the real lever), one trend∧trend control (predicts *no* lift — if it
  lifts as much as the mixed pairs, the decorrelation thesis is wrong), and the
  **complete** k-ladder over the 4 (k∈{1,2,3} — k=4 is the existing unanimous
  arm) so we report the *whole* ladder, never a cherry-picked rung.
- Every arm is **falsifiable up front:** the trend-pair control has a *predicted*
  outcome (little p5 lift). That is what makes this an experiment, not a hunt.
- Bounded blast radius: 6 ids, all vote ensembles, all reusing frozen infra.

> **Architect may adjust the exact membership** — the load-bearing constraints
> are: (a) FIXED + declared in code, (b) all members carry the existing
> warmup/abstention semantics, (c) the slate includes at least one *predicted-null*
> control so the decorrelation thesis is falsifiable, (d) the k-ladder is reported
> *whole* (no per-arm k-selection). See § Open architecture questions OQ-1.

### R3 — Gating + crown-eligibility: unchanged, frozen

Each combination is **just another candidate.** It is scored by the frozen
`RobustnessMode::Bootstrap` gate on its **own** realized equity curve, and is
**crown-eligible only if it clears the gate** (`robustness != Some(Fragile)`),
exactly like every single strategy. The ADR-0059 § D5 anti-overfit lock and the
ADR-0063 § D4 classifier freeze are **UNCHANGED**. The robustness **bands are
FROZEN** — this feature does **NOT** loosen, asset-class-tune, or otherwise touch
`verdict_bands` / `classify_verdict` / `compute_robustness_flag`. (This is not a
B2/B3-style band proposal — those were operator-REJECTED. Frame everything as
"more candidates face the same bar," never "we moved the bar.")

### R4 — Honesty: `BenchmarkWins` / `AllFragile` stay reachable; null result is shippable

Widening the field must **NOT** change the reachability of the honest outcomes
(F8 § sentence 4, B1/ADR-0066). With buy-and-hold always in the field, an
all-active-Fragile field ⇒ `BenchmarkWins` (the modal real-crypto outcome). A
combination that wins one path but is Fragile under resampling is **shown but not
crowned.** The recommendation surface keeps the paper-only + not-financial-advice
framing on every output. **A null result — "every pre-registered combination is
also Fragile; hold stands" — is a valid, expected, shippable finding** and must
be reported as honestly as a win.

### R5 — Right-size it: MVP = pre-registered arms; search engine = guarded follow-on

**MVP (this feature, recommended):** add the R2 slate as new pre-registered
`EnsembleStrategy` arms, reusing 100% of existing infra (F8 + the gate + the
bake-off). No new orchestrator, no search, no new statistic.

**NOT in MVP (recorded as a follow-on with loud guards):** a
**combination-search engine** that optimizes membership / weights / thresholds.
If ever built, it is overfit-prone by definition and MUST ship with ALL of:
(1) a **walk-forward / out-of-sample split** (search on a train window, score
*only* on a held-out window the search never saw); (2) an explicit **complexity
penalty** (deflated Sharpe / `√(degrees-of-freedom)` haircut à la
Bailey–López de Prado, or a Bonferroni-style correction for the number of
combinations tried); (3) a **pre-registered search procedure** (the search
space + selection rule declared before running, so the experiment is auditable);
and (4) a **loud risk call-out** on every surface that a searched winner is a
weaker claim than a pre-registered survivor. For v1, **recommend pre-registered
only** — the search engine is explicitly out of scope. (See § Open architecture
questions OQ-4.)

### R6 — Anchor-safe + reuse-only (non-negotiables)

- **Anchor-safe by construction.** New arms are new strategy ids
  (`v0.8.vote.*`); they only run with `write_report = false` in the bake-off and
  on the `RobustnessMode::Bootstrap` advisor path. A new id cannot collide with
  any existing anchored report body. `scripts/verify_anchors.sh` must stay
  119/119 byte-identical — verify **before and after** (per the anchors-keyed-by-name
  memory note). No `anchors.toml` SHA / `REVISION.toml` / `spec/*/reports/` body
  is touched.
- **Reuse-only.** No new backtest math, no new gate, no `EnsembleStrategy`
  reinvention. The vote-threshold arms need only: new ids in `build_ensemble`,
  the widened `engine.rs` dispatch arm, and the extended `advisor_field()`.

## Design

_Architect, 2026-06-23. Full rationale + alternatives in
[ADR-0067](../architecture/adr/0067-pre-registered-combination-slate-expansion.md).
This feature is an **expansion of F8** (ADR-0063), not a new mechanism: it adds
**6 new pre-registered vote-ensemble arms** to the live advisor field, each
scored through the **identical byte-frozen** `RobustnessMode::Bootstrap` gate +
the **identical** buy-and-hold benchmark (ADR-0066). Zero new arbitration math,
zero band changes, anchor-safe by construction. The crux is **pre-registration
as an overfit defense**: a FIXED, code-declared falsifier slate (chosen before
results), reported WHOLE — win or lose. A null all-Fragile result
("hold stands") is an expected, valid, shippable outcome._

### The v1 arm-set (FROZEN, pre-registered, declared-in-code)

Membership is **ratified as the analyst's recommended slate** — it satisfies all
four load-bearing constraints (FIXED + declared; members carry the existing
warmup/abstention semantics; ≥1 predicted-null control; the k-ladder reported
whole). All 6 are vote ensembles over the existing 4 base signals
`{v0.sma, v0.5.macd, v0.5.rsi, v0.5.bbands}` ⇒ **zero new arbitration code**
(`VoteMethod::{Majority{k,n}, Unanimous{n}}` already express every one).

| New id | Method | Members | Role |
|---|---|---|---|
| `v0.8.vote.trend_pair` | `Unanimous{n:2}` | `[v0.5.macd, v0.sma]` | **predicted-null control** (both trend → expect little p5 lift; if it lifts as much as the mixed pairs, the decorrelation thesis is FALSE) |
| `v0.8.vote.tr_mr_macd_rsi` | `Unanimous{n:2}` | `[v0.5.macd, v0.5.rsi]` | trend ∧ mean-revert (the real decorrelation lever) |
| `v0.8.vote.tr_mr_sma_bb` | `Unanimous{n:2}` | `[v0.sma, v0.5.bbands]` | trend ∧ band-reversion (second decorrelated pairing) |
| `v0.8.vote.any1of4` | `Majority{k:1, n:4}` | all 4 | k-ladder rung 1 (loosest — long if ANY fires) |
| `v0.8.vote.k2of4` | `Majority{k:2, n:4}` | all 4 | k-ladder rung 2 (balanced quorum) |
| `v0.8.vote.k3of4` | `Majority{k:3, n:4}` | all 4 | k-ladder rung 3 (strict — broad agreement) |

The k-ladder over the 4 base signals is reported **complete**: k∈{1,2,3} here +
k=4 is the existing `v0.8.vote.unanimous` arm → no per-arm "best k" is ever
cherry-picked. Existing F8 arms kept unchanged: `v0.8.vote.majority`
(`Majority{k:2,n:3}` over macd/rsi/bbands) and `v0.8.vote.unanimous`
(`Unanimous{n:4}` over all 4). **Total live field: 4 singles + 8 ensembles +
buy-and-hold = 13 arms** (the field `Vec` carries 12 ids; `run_bakeoff` appends
buy-and-hold).

### OQ-1 — DECISION: literal `build_ensemble` ids (NOT a generalized parser)

Add the 6 ids as **literal `match` arms** in `strategy::build_ensemble`
(`crates/strategy/src/ensemble.rs`), mirroring the existing two. The engine
dispatch (`crates/backtest/src/engine.rs:1527`) already calls
`build_ensemble(strategy_str)` **generically** — the only literal thing there is
the `match` *pattern* `"v0.8.vote.majority" | "v0.8.vote.unanimous"`, which is
widened to alternate the 6 new ids. **No id grammar / no runtime parser.** A
`vote.k{K}of{N}`-style parser is REJECTED: an id grammar is a parameterized space
that edges toward search and breaks the pre-registration lock ("pre-registered =
a finite enum you can read in the diff"). Each arm's `(id, VoteMethod, members)`
tuple is visible in the `build_ensemble` source and in this table — that
auditability IS the overfit defense. Net new seam surface (all reuse-only):

```
crates/strategy/src/ensemble.rs   build_ensemble: +6 literal match arms
                                  member_id_to_rule_shape: already covers all 4 base ids (no edit)
crates/backtest/src/engine.rs     run_scenario "v0.8.vote.*" pattern: widen the | alternation (body unchanged — already calls build_ensemble(strategy_str))
crates/backtest/src/bakeoff/mod.rs  default_ensemble_field(): +6 StrategyIds
```

`advisor_field()` in `crates/ui/src/leaderboard/runner.rs:53` is
`default_field() ∪ default_ensemble_field()` — the 6 new ids flow into the live
cockpit field **automatically**, no UI edit. Confirmed pickup.

### OQ-2 — DECISION: `default_ensemble_field()` is the single source of truth; latency within budget

`default_ensemble_field()` is the **single arm-list source of truth** — no
duplicated list exists (`advisor_field()` concatenates it onto `default_field()`;
the runner's own config test reads it back). Adding the 6 ids there is the only
field edit. **One field-count test must move in lockstep**
(`runner.rs:238-242`): the `cfg.request.field.len()` assertion goes `6 → 12`, and
its `ids.contains(...)` set extends to the 6 new ids (this is a *test* update, not
a contract loosening — the assertion exists to pin the field, so it tracks the
field). This is a developer task (T4), not an afterthought.

**Latency.** 13 arms × 1000 bootstrap paths vs 7 today ≈ **+86% bake-off
wall-clock** (12 active field arms vs 6, + buy-and-hold both runs; the bootstrap
resample dominates and scales ~linearly in arm count). This is **within budget
and needs no new UX**: (a) the bake-off already shows a **determinate progress
bar** driven by `Progress{current_bar, total_bars}` per arm (the
`bakeoff_progress_render.rs` harness proves it paints), so a longer run is a
longer-but-bounded bar, not a frozen screen; (b) the advisor bake-off is an
**explicitly on-demand, operator-triggered** action (press "Run bake-off"), not a
per-frame or latency-critical path — there is no real-time SLA to breach. **No
progress-UX change required.** One honest copy note is added to the leaderboard
header context (developer task T7): the arm count is surfaced so the operator
understands a wider field takes proportionally longer. (If a future field grows
past ~20 arms, revisit parallelizing the per-arm bootstrap — explicitly out of
scope here.)

### OQ-3 — DECISION: `Unanimous{n:2}` mostly-Flat is HONEST, rendered truthfully

The `Unanimous{n:2}` trend∧mean-revert pairs (`tr_mr_macd_rsi`, `tr_mr_sma_bb`)
require **both** members simultaneously warmed AND Long to go Long. A trend
follower and a mean-reverter fire in **different regimes**, so simultaneous-Long
is genuinely rare → these arms may sit **mostly Flat → near-zero return → their
own Fragile / `p50 Sharpe < 0.5` flag**. This is **HONEST, not a bug**: it is the
literal, correct property of a strict-consensus combo of decorrelated members
(strict-consensus trades the cost of fewer trades for the benefit of a tighter
path-spread; whether that net-lifts p5 Sharpe is exactly the experiment). It is
**not** engineered around. It is rendered truthfully by the **existing surfaces**,
reusing the B1 "sat in cash" honesty copy:
- The leaderboard row shows the real (low) `trade_count` + the computed
  robustness flag (`Fragile`) verbatim — same path as today's Fragile single
  (`v0.5.rsi` in the fixture already exercises the Fragile warn tag).
- The recommendation surface keeps the `BenchmarkWins` / `AllFragile` framing
  (ADR-0066): an arm that rarely trades and stays Fragile is **shown but not
  crowned**, and "Nothing beat simply holding…" (the B1 modal copy) stands.
- The forward-plan `PlanRuleShape::Ensemble` honestly reads "Holds while **2 of
  {MACD trend, SMA cross}** agree…", which truthfully telegraphs the strict gate.

No new flag, no new state, no special-case. The honesty is that a low-trade
Fragile combo is reported as honestly as a win (R4).

### OQ-4 — DECISION: DEFER weighted / inverse-vol / regime blends to v0.2 of this feature

v1 ships **vote-threshold combinations only** (zero new arbitration code).
Weighted / inverse-vol / conditional-regime blends are **explicitly out of scope
for v1** and recorded as a v0.2 follow-on. Rationale (confirming the analyst
lean): each needs a **new `VoteMethod` variant + a new `arbitrate` branch + a new
`PlanVoteMethod` mirror + a new `ui` exhaustive-match arm** — a real cross-crate
surface — AND a **continuous weight parameter is a free knob = overfit risk**
unless pre-registered to a fixed/rule-derived value. The one defensible v0.2 case
is **inverse-vol weighting** (rule-derived from realized vol, not fitted); if/when
built it MUST ship with the CLAUDE.md **day-1 baseline-equity-divergence e2e**
(it is a sizing-modifier-adjacent overlay — the `vol_targeting_overlay` precedent
applies). **None of these enters v1.** This keeps v1's blast radius to 6 ids, all
expressible by the frozen arbiter.

### OQ-5 — DECISION: day-1 divergence e2e — `combination_slate_divergence_end_to_end.rs`

Per the CLAUDE.md non-negotiable, each new arm ships a **day-1
baseline-equity-divergence e2e** from day 1, modelled on the F8
[`ensemble_vote_divergence_end_to_end.rs`](../../crates/strategy/tests/ensemble_vote_divergence_end_to_end.rs).
A new test file `crates/strategy/tests/combination_slate_divergence_end_to_end.rs`
asserts, for **each of the 6 new arms**, two properties on a shared
deterministic bar series + position sim (the F8 harness's `run_strategy_equity`):
1. **Diverges from its members.** The arm's final equity differs from **at least
   one** of its own member curves by **≥ 1 bp** of initial capital — proves the
   vote actually gates trades and the arm is **not a silent passthrough/no-op**
   (the `v3-vol-overlay-noop` analogue).
2. **Not a duplicate / not buy-and-hold.** The arm's equity differs from
   buy-and-hold (always-long) by ≥ 1 bp, AND no two new arms produce identical
   curves on the same series — proves no arm is an accidental duplicate of an
   existing arm or of BH.

**Construction note (load-bearing, from the F8 precedent):** the TOML base
members (MACD/RSI/BBands) do **not** reliably fire on arbitrary synthetic bars
(their thresholds may never trip), so the **vote-mechanics** divergence is proven
with `SmaCrossover` members at distinct parameter pairs (guaranteed signals) —
exactly as F8 does — and a **factory smoke test** asserts each real
`build_ensemble("v0.8.vote.<arm>")` constructs over the real 4 base TOMLs without
error. The **end-to-end divergence of the real base-signal arms on real data** is
then proven by the T6 bake-off (each arm's realized equity curve is distinct in
the 13-arm report). Together these satisfy the non-negotiable's intent: no arm is
a computed-but-unapplied no-op, and no arm is a hidden duplicate. FAIL-before /
PASS-after: deleting any new `match` arm (or aliasing it to an existing id) makes
the test fail.

### OQ-6 — DECISION: leaderboard 13-row + ensemble-rule render-snapshots (pixel proof)

Per the verify-UI-at-render-layer non-negotiable, the proof is a **populated
render-snapshot**, not a unit test. Two surfaces, both with existing harnesses to
extend:
1. **Leaderboard scales to 13 rows.** Extend the populated fixture
   `ui::fixtures::fake_bakeoff_report_mirror()` (`crates/ui/src/fixtures.rs:1256`)
   to **13 `LeaderRow`s** (4 singles + 8 ensembles + buy-and-hold), including ≥1
   Fragile ensemble row (exercises the warn tag) and the crowned `★ best` accent.
   Add a guard in
   [`leaderboard_populated_render.rs`](../../crates/ui/tests/leaderboard_populated_render.rs)
   asserting the 13-row table paints (crowned ACCENT teal + the always-negative
   Max-DD clay across all rows + a healthy foreground-text floor) with the
   `Empty` negative control still painting no table. If 13 rows exceed the
   viewport, the existing scroll container carries them — the guard asserts the
   table band rendered, not that all 13 are simultaneously on-screen.
2. **The new ensemble rows render `PlanRuleShape::Ensemble` honestly.** The
   forward-plan member-naming surface is **already render-proven** by
   [`forward_f6_ensemble_named_render.rs`](../../crates/ui/tests/forward_f6_ensemble_named_render.rs)
   ("Holds while at least k of {…} agree…"); because the new arms reuse
   `PlanRuleShape::Ensemble` + the same `member_id_to_rule_shape` mapping (which
   already covers all 4 base ids), the honest "≥ k of {members} agree" description
   is produced for them with **no new code**. The render task adds one
   crowned-`tr_mr_macd_rsi` (or `k2of4`) plan fixture + asserts its RULES band
   paints the named-member brace-list (strict-exceedance vs the single-strategy
   SMA negative control), confirming a NEW combination arm draws its rule
   truthfully when crowned/forward-planned.

### Frozen surfaces honoured (the hard constraints)

- **Robustness bands FROZEN** — `classify_verdict` / `compute_robustness_flag` /
  `verdict_bands` (`crates/backtest/src/bakeoff/robustness.rs`) + `bootstrap.rs`
  are **byte-UNCHANGED**. This is NOT a B2/B3 band proposal (operator-rejected).
  Framed strictly as "more candidates face the same bar," never "we moved the
  bar." Every new arm is scored by the identical frozen gate on its OWN realized
  equity, crown-eligible only if `robustness != Some(Fragile)`.
- **Reuse-only arbitration** — `strategy::EnsembleStrategy` + `VoteMethod` +
  `arbitrate` (`crates/strategy/src/ensemble.rs`) express generic k-of-n today →
  ZERO new arbitration math. `rank_candidates` + the ADR-0066 benchmark exemption
  (`crates/backtest/src/bakeoff/rank.rs`) UNCHANGED — a combination is just
  another `CandidateResult`.
- **Anchor-safe by construction** — the new `v0.8.vote.*` ids run with
  `write_report=false` on the `RobustnessMode::Bootstrap` advisor path (the F8
  contract, ADR-0063 § D5); a new id cannot collide with any anchored report
  body. `scripts/verify_anchors.sh` must stay **119/119 byte-identical**, run
  **before the first seam AND after the last** (any non-119 = STOP-and-route-back;
  anchors are keyed by NAME not filename). No `anchors.toml` SHA /
  `REVISION.toml` / `spec/*/reports/` body is touched.
- **`BenchmarkWins` / `AllFragile` reachability UNCHANGED** (ADR-0066). A null
  all-Fragile field ⇒ `BenchmarkWins` (the modal real-crypto outcome) remains
  reachable WITH the 6 new arms present; the T6 bake-off records this as a
  pre-registered prediction and reports the WHOLE slate regardless.

## Backtest Scenarios

_Architect, 2026-06-23._ The decisive validation is **one real-data bake-off** on
the standard advisor corpus — no new anchored scenario (the advisor path runs
`write_report=false`, so this produces NO anchored body and touches NO
`anchors.toml` SHA).

| Field | Value |
|---|---|
| Corpus | BTCUSDT, H1-2024 (`DateRange::H1_2024`), `ScenarioDataSource::BinanceCache` (the real hourly corpus) |
| Field | the live `advisor_field()` = 4 singles + 8 ensembles (the 6 new + the 2 F8) + buy-and-hold appended by `run_bakeoff` = **13 arms** |
| Gate | `RobustnessMode::Bootstrap { paths: 1000, seed: <LAB_DEFAULT_SEED low-8> }` — the frozen Politis–White block bootstrap, byte-unchanged |
| Seed | `LAB_DEFAULT_SEED` (same-seed-every-arm — the apples-to-apples invariant) |
| Report (per arm) | robustness flag, p5 Sharpe, p50 Sharpe, total-return, max-drawdown, trade_count, plus the run-level `RecommendationOutcome` + crowned arm |

**Pre-registered prediction (record BEFORE running — this is what makes it an
experiment, not a hunt):**
1. Most or all combinations come back **Fragile** → run-level **`BenchmarkWins`**
   (the modal real-crypto outcome; the live 7-arm field already does this,
   ADR-0066). This is the **expected** result.
2. The **`trend_pair` control** (`Unanimous{n:2}` [macd,sma], both trend) shows
   **little-to-no p5-Sharpe lift** vs its members — if it lifts as much as the
   trend∧mean-revert pairs, the decorrelation thesis is **falsified** (a real,
   recordable finding either way).
3. The `Unanimous{n:2}` trend∧mean-revert pairs likely sit **mostly Flat**
   (low `trade_count`, near-zero return → Fragile) — honest, per OQ-3.

**The actual question:** does ANY pre-registered combination's decorrelation lift
its **p5 Sharpe above 0** (the binding `classify_verdict` constraint) AND clear
the rest of the frozen gate → a non-Fragile, crown-eligible combination? **Report
the WHOLE 13-arm slate, win or lose.** A null result ("every combination also
Fragile, hold stands") is a valid + expected + shippable finding and is reported
as honestly as a win. The tester's report records the prediction, the realized
13-arm table, and whether the prediction held.

## Implementation
_developer fills this._

## Verification
_tester links to reports here._

## Open architecture questions (for the architect)

- **OQ-1 — Generalize `build_ensemble` id dispatch, or add literal arms?** The
  engine arm is currently a literal `match` (`"v0.8.vote.majority" | "v0.8.vote.unanimous"`).
  Six new ids can be added literally (cheapest, ~F8-shaped) OR the dispatch can be
  generalized to parse `vote.k{K}of{N}` / membership from the id (more composable,
  but an id *grammar* edges toward a parameterized space — keep it a FIXED enum of
  ids, not a free parser, to preserve the pre-registration lock). **Analyst lean:
  literal arms** — preserves "pre-registered = a finite enum you can read in the
  diff." Flag if you disagree.
- **OQ-2 — Where does the field list live?** `default_ensemble_field()` returns the
  2 F8 ids; `advisor_field()` (ui) concatenates. Adding 6 ids to
  `default_ensemble_field()` flows everywhere automatically — confirm that is the
  intended single source of truth and that 13 arms × 1000 bootstrap paths is within
  the advisor's latency budget (8 arms today; +6 arms ≈ +75% bake-off wall-clock —
  quantify and decide if a progress-UX or arm-count note is needed).
- **OQ-3 — `Unanimous{n:2}` warmup interaction.** The trend∧mean-revert pairs use
  `Unanimous{n:2}`: both members must be *warmed AND Long* to go long. With a trend
  follower and a mean-reverter, simultaneous-Long may be **rare** (they fire in
  different regimes), so the arm may sit mostly Flat → near-zero return → its own
  Fragile/`p50 Sharpe < 0.5` flag. That is an **honest** outcome (the arm genuinely
  rarely agrees), but confirm it is not mistaken for a bug, and consider whether a
  `Majority{k:1,n:2}` ("either fires") sibling belongs in the slate as the
  decorrelation-via-OR counterpart. (Note: OR-blending *reduces* decorrelation
  benefit — it widens exposure — so it is the opposite hypothesis; include only if
  pre-registered as such.)
- **OQ-4 — Weighted / inverse-volatility / conditional-regime blends are NOT free.**
  `VoteMethod` has only `Majority`/`Unanimous`. **Weighted** or **inverse-vol**
  blends need a NEW `VoteMethod` variant + new `arbitrate` branch + a new
  `PlanVoteMethod` mirror + a new UI exhaustive-match arm — AND a continuous weight
  parameter is a **free knob = overfit risk** unless pre-registered to a fixed value
  (e.g. inverse-vol weighting is *rule-derived from realized vol*, not fitted, so it
  is defensible as pre-registered; arbitrary static weights are not). **Analyst
  lean: defer weighted/inverse-vol to a v0.2 of this feature** — v1 ships
  vote-threshold combinations only (zero new arbitration code). If the architect
  wants inverse-vol in v1, it must be framed as the rule-derived (non-fitted) case
  with the CLAUDE.md baseline-equity-divergence e2e test from day 1 (it is a
  sizing-modifier-adjacent overlay).
- **OQ-5 — Day-1 divergence e2e test.** Per the CLAUDE.md non-negotiable, any arm
  whose composite signal/sizing differs from a baseline must ship a day-1
  baseline-equity-divergence e2e test (≥ 1 bp divergence when the decision variable
  is non-trivial). For **vote-threshold** arms this is naturally satisfied (a
  k-of-n arm's equity provably diverges from any single member's on a window where
  the quorum gates a trade) — confirm the test asserts each new arm's equity
  diverges from its members' AND from buy-and-hold, so no arm is a silent no-op
  duplicate of an existing one.
- **OQ-6 — Does the leaderboard UI scale to 13 rows + render-prove the new rows?**
  The cockpit leaderboard renders per-candidate rows + sparklines. Confirm 13 rows
  fit / scroll, and that the new ensemble rows render their `PlanRuleShape::Ensemble`
  description honestly ("≥ k of {…} agree"). Per the verify-UI-at-render-layer
  memory note, a populated render-snapshot (not a unit test) is the proof.

## Changelog

- 2026-06-23 (analyst): brief created — `proposed`. Scopes the operator-requested
  expansion of the strategy-combination space as a **pre-registered-only** widening
  of F8's bounded vote-ensemble slate (6 new vote-threshold arms over the existing
  4 base signals, reusing `EnsembleStrategy` + the frozen `RobustnessMode::Bootstrap`
  gate + the buy-and-hold benchmark). Crux = pre-registration (no search = overfit-safe
  by construction); robustness bands FROZEN; anchor-safe; reuse-only. Search engine
  quarantined to a guarded follow-on (R5/OQ-4). Trace `REQ-ADVISOR-COMBINATION-SEARCH-001`.
  HANDOFF → architect.
- 2026-06-23 (architect): Design + ADR-0067 + the developer task list. → `designed`.
  Ratified the analyst's recommended 6-arm slate verbatim (3 `Unanimous{n:2}`
  decorrelation pairs incl. the `trend_pair` predicted-null control + the complete
  k∈{1,2,3}-of-4 ladder). OQ resolutions: **OQ-1** LITERAL `build_ensemble` ids
  (no parser — engine already dispatches generically; widen the `match` pattern
  only); **OQ-2** `default_ensemble_field()` is the single source of truth (+6
  ids → field len 6→12; one runner field-count test moves in lockstep), ~+86%
  wall-clock is within budget on the determinate-progress on-demand path (no UX
  change); **OQ-3** `Unanimous{n:2}` mostly-Flat is HONEST, rendered truthfully via
  the existing Fragile/trade_count + B1 "sat in cash" copy (no special-case);
  **OQ-4** DEFER weighted/inverse-vol/regime to v0.2 (new VoteMethod surface + free
  knob); **OQ-5** day-1 `combination_slate_divergence_end_to_end.rs` (each new arm
  diverges ≥1bp from a member AND from buy-and-hold + no-duplicate, SMA-member
  vote-mechanics per the F8 precedent + factory smoke over the real TOMLs);
  **OQ-6** populated render-snapshots — leaderboard 13-row guard + the ensemble-rule
  description (reuses F6's already-proven `PlanRuleShape::Ensemble` naming).
  Frozen: bands byte-unchanged (NOT B2/B3), `rank_candidates`/ADR-0066 unchanged,
  anchor-safe 119/119 by construction (`write_report=false`). HANDOFF → developer.
