---
slug: advisor-combination-search
status: proposed
owner: analyst
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
_architect fills this (ADR). Seam pointers + the load-bearing locks are listed
in § Open architecture questions below._

## Backtest Scenarios
_architect + tester fill this. The decisive validation is a real-data bake-off
on the standard advisor corpus (BTCUSDT H1-2024, `BinanceCache`,
`RobustnessMode::Bootstrap{paths:1000}`) reporting, for all 13 arms, the
robustness flag + p5/p50 Sharpe + `RecommendationOutcome`. The pre-registered
prediction to record up front: **most or all combinations come back Fragile →
`BenchmarkWins`**; the experiment is whether ANY combination's decorrelation
lifts p5 Sharpe above 0 and clears the rest of the gate. Report the WHOLE slate,
win or lose._

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
