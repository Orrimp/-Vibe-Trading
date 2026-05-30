---
slug: momentum-parameter-robustness-sweep
version: 0.1.0
status: draft
owner: analyst
priority: P2
updated: 2026-05-30
---

# Momentum parameter-robustness sweep — distribution-per-θ over the momentum family — v0.1.0

> **Monte-Carlo robustness lane — C3 (parameter axis).** Per the operator's
> 4 locked strategic decisions (2026-05-30) and the C2 FRAGILE verdict. C2
> ([`strategy-robustness-harness`](../strategy-robustness-harness/feature.md))
> proved v1 cross-sectional momentum is **FRAGILE at its one shipped θ\***
> (lookback 60, k_long 3, the fixed `top10_momentum_h1.toml` config): p50
> Sharpe ≈ −0.01, P(loss) 75.2%, p95 MaxDD 91.5%, P(Sharpe>1) = 0. C3 asks the
> next question: **is the fragility specific to that one θ\*, or is the WHOLE
> momentum family fragile?** It sweeps the momentum parameters across a bounded
> grid, runs the C2 robustness harness at each θ, and reports a **per-θ
> FRAGILE / MARGINAL / ROBUST verdict** against the pre-registered decision rule
> ([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md)).
>
> **This brief is reversible DESIGN work.** The operator may still redirect to a
> pivot or to C5 (PBO/Deflated-Sharpe) before any C3 code is written. It exists
> to make the C3-vs-pivot-vs-C5 fork a decision-grade choice, NOT to pre-commit
> to building.

---

## TL;DR for the operator (the one decision this brief frames)

- **What C3 answers:** "ROBUST at θ\* was false (C2). Is ROBUST true at *any*
  θ in the momentum family, or is long-only top-K 1h-crypto momentum
  structurally a cost-bleed machine regardless of parameters?"
- **The prior is strong and points at FRAGILE-everywhere.** The C2 adversarial
  review ([`robustness-verdict-adversarial-review-2026-05-30.md`](../dev-notes/robustness-verdict-adversarial-review-2026-05-30.md))
  already swept the single most important parameter — the bootstrap block
  length L — across **four orders of magnitude (L ∈ {1 … 4000})** and found p50
  Sharpe pinned flat at ≈ −0.02 / P(Sharpe>1) = 0.000 at *every* L, while a
  passive buy-and-hold control scored **p50 Sharpe +1.78** on the *same* paths.
  That isolates the fragility to the strategy's **turnover / fee bleed**, not to
  the null. C3's job is to confirm or refute that the same fragility holds
  across the strategy's *own* parameters (lookback, k_long, rebalance cadence,
  hold-band).
- **THE methodology decision (front-and-center, §0):** sweeping θ and reporting
  "the best θ looks robust" is textbook **multiple-testing / selection bias** —
  the more θ tried, the more likely one clears the bar by chance. C3 must NOT let
  cherry-picking manufacture a false ROBUST. **Recommendation: Option (a) —
  C3 reports the FULL θ-surface (every θ's distribution + verdict, no
  cherry-picking) and answers only the family-level question; any "best θ is
  robust" claim is explicitly DEFERRED to a C5 deflation pass.** Rationale +
  rejected alternatives in §0.
- **The expected outcome (state honestly up front):** given the L-sweep + the
  buy-and-hold dominance, the most likely C3 result is **"the family is
  uniformly fragile"** — every θ FRAGILE, no deflation needed, momentum-v1
  retired on the robustness axis. The valuable surprise would be a *cluster* of
  θ that escape the turnover trap (e.g. very low turnover via a wide hold-band
  + long lookback); that cluster, if it appears, is what triggers C5.

---

## 0. THE methodology decision — multiple testing / selection bias (settle this first)

This is why C3 needs scoping and not just coding. The integrity principle is the
direct meta-lesson of the v3-vol-overlay no-op era and the pre-registered
decision rule: **a number interpreted only after it is seen can be talked into
meaning whatever the author wants.** A θ-sweep is the single most fertile ground
for that failure mode.

### The hazard, quantified

The C2 decision rule's ROBUST bar includes `P(Sharpe>1) ≥ 60%` and `p5 Sharpe ≥
+0.5`. Suppose (counter to the prior) that the true family is genuinely fragile
but each θ's per-path noise is non-trivial. If you run a G-cell grid and pick
`argmax`, the probability that **at least one** cell clears a bar by chance rises
with G — the classic `1 − (1−α)^G` inflation. At G = 24 cells and a nominal
per-cell false-ROBUST rate of even 5%, the family-wise false-ROBUST probability
is `1 − 0.95^24 ≈ 71%`. **A 24-cell sweep that reports "the best cell is ROBUST"
is more likely than not to be reporting noise.** This is exactly the failure C5
(PBO / Deflated-Sharpe) exists to correct.

### The three candidate paths (the operator-decide question)

- **(a) C3 reports the FULL θ-surface; "best θ is robust" is DEFERRED to C5.**
  C3 answers exactly one question: *"Is the momentum family uniformly fragile?"*
  Every θ's full distribution + per-θ verdict is reported, no `argmax`
  cherry-pick, no "winner" crowned. Two clean exits:
  - **Uniform FRAGILE** (every θ FRAGILE) ⇒ a strong, deflation-free conclusion:
    the family is fragile, retire momentum-v1 on the robustness axis. **No
    multiple-testing correction is needed for a uniform-negative result** — you
    are not selecting a winner, you are reporting that no cell cleared the bar.
  - **Some θ looks ROBUST/MARGINAL** ⇒ C3 reports the surface and **explicitly
    flags those cells as candidates that REQUIRE a C5 deflation pass before any
    promotion**. C3 makes **no** "this θ is robust" claim — it hands the
    surviving cluster to C5.

- **(b) C3 bakes in a multiple-testing correction (Deflated-Sharpe across the
  grid).** Effectively merges part of C5 into C3: compute the Deflated Sharpe
  Ratio (López de Prado) using the *number of trials = grid size* and report a
  deflation-adjusted verdict per θ.

- **(c) C3 and C5 merge into one feature.** Build the sweep and the
  PBO/CPCV/Deflated-Sharpe deflation as a single deliverable.

### Recommendation — **(a)**, decisively. `(Recommended)`

The `(Recommended)` tag is on **(a)** and this is the durable choice, not merely
the cheap one — the justification is *separation of concerns under
pre-registration*, not blast radius:

1. **(a) keeps each feature's claim falsifiable and single-purpose.** C3's claim
   becomes "no θ cleared the bar" (a uniform-negative, which needs no
   deflation — you cannot overfit your way to a *negative*) OR "these θ are
   undeflated candidates" (an explicit hand-off, not a verdict). Neither claim
   can be inflated by selection because **C3 never selects.** This is the
   anti-cherry-pick guarantee the task demands, achieved by *construction* rather
   than by a correction that itself has free parameters (the "number of trials"
   in DSR is itself a modelling choice that can be gamed).

2. **The prior makes (a) almost certainly sufficient.** The L-sweep already
   shows flat fragility across 4 orders of magnitude of the *null's* parameter,
   and buy-and-hold dominates by +1.8 Sharpe. The overwhelmingly likely C3
   outcome is uniform FRAGILE — in which case deflation is moot and merging C5
   in (paths b/c) would be **building a correction for a winner that never
   appears.** Paying for C5's full CPCV/PBO machinery now, on spec, is the
   *quick-feeling-but-wasteful* path if the family is uniformly fragile.

3. **(a) preserves C5 as the dedicated, correctly-scoped overfit guard.** C5 is
   already a Queue feature with its own design (CPCV perturbs the *partition*;
   the bootstrap perturbs *paths* — orthogonal axes, backlog § C5). Folding a
   half-baked DSR into C3 (path b) would create a second, weaker overfit guard
   that the project then has to reconcile with the real C5. One correct C5 beats
   two partial deflations.

4. **(a) is anchor-clean.** C3 adds ONE θ-surface report + ONE anchor (ADR-0051
   D4 shape preserved — §D-C3.5). Path (b)/(c) would add DSR columns whose
   "number of trials" input is a new hashed body field with its own
   determinism/justification burden, and (c) would couple two anchor namespaces.

**If-budget-tightens annotation (the cheaper fallback, NOT recommended):** there
is no cheaper path that preserves integrity — (a) IS the minimal integrity-
preserving design. The only cost lever is **grid size + N** (§D-C3.2), not the
methodology. If wall-clock is the binding constraint, shrink the coarse grid to
the ~9-cell `lookback × k_long` core (§D-C3.2 Tier-1) at N=300 rather than
weakening the no-cherry-pick rule. Do NOT downgrade to path (b)/(c) to "save a
feature" — that trades integrity for a merge that the prior says is unnecessary.

> **Pre-registration commitment (frozen now, before C3 emits any θ-surface):**
> C3 will NOT report an `argmax`-selected "best θ" as ROBUST. The deliverable is
> the full surface + per-θ verdict + (if any cell is non-FRAGILE) an explicit
> "→ C5 deflation required" flag. Any change to this after seeing the surface
> must be logged in the Changelog with operator signoff (mirrors the C2 decision
> rule's pre-registration discipline).

---

## Why

### From path-robustness (C2) to parameter-robustness (C3) — the epistemic delta

C2 answered "is the outcome at θ\* a property of the strategy or of the 2023 path
ordering?" (answer: the strategy is fragile to resampled paths). C2's scope note
(§5) is explicit that it judges **path** robustness, NOT **parameter**
robustness — "a strategy can pass [the C2] rule and still be a sharp peak in
θ-space." C3 is the dual: it judges whether the FRAGILE verdict is a property of
the *one shipped parameterization* or of the *whole family*.

| Question the operator has after C2 | C2 (path axis) | C3 (parameter axis) |
|---|---|---|
| "Is θ\*=60 a curve-fit peak or a fragile plateau?" | silent (single θ\*) | the θ-surface shape: flat-fragile vs a robust island |
| "Would lookback 120 / k_long 5 / a wide hold-band have been robust?" | silent | a per-θ verdict at each grid cell |
| "Is the turnover bleed (5343 trades/yr at θ\*) structural or tunable away?" | hinted by the buy-and-hold gap | directly: does a low-turnover θ (wide hold-band, long lookback) escape it? |
| "Should momentum-v1 be retired or re-parameterized?" | retire *this* θ\* | retire the *family* (uniform FRAGILE) or re-aim at a surviving cluster |

### The harness already exists — C3 is ~85% wiring (the reuse story)

This is the single most important scoping finding and it mirrors the C1→C2
"~80% built" finding. C3 sits on **two** working seams:

1. **The C2 robustness harness** (`crates/backtest/src/bin/monte_carlo.rs` +
   `scenarios::montecarlo::run_path` + `stats::DistributionSummary`) is exactly
   "for a fixed θ: run N bootstrap paths → one distribution summary + verdict."
   C3 wraps this in an **outer θ-loop**: for each θ in a grid, call the C2 inner
   harness, collect the per-θ `DistributionSummary`, tag a verdict.

2. **The threshold_sweep seam** (`crates/backtest/src/bin/threshold_sweep.rs` +
   `scenarios::threshold_sweep::run_cell`) is the proven **grid-enumerate →
   per-cell run → sort-before-render → one summary report = one anchor** pattern.
   C3 is the **product** of the two: `threshold_sweep`'s outer grid loop ⊗ C2's
   inner N-path distribution (instead of `threshold_sweep`'s inner single-path
   backtest).

The genuinely NEW code (§D-C3.6) is small and bounded:
- the **outer θ-grid enumerator** + per-θ sub-seeding (compose ADR-0051 D1 on a
  second axis);
- a **per-θ verdict classifier** that encodes the §0/decision-rule bands in code
  (C2's `render_report` has only a 3-line directional `WEAK/MARGINAL/ROBUST`
  string — C3 needs the full 5-primary-signal weakest-link composite);
- the **θ-surface report renderer** (one report, G rows, sorted by θ);
- the **config-injection seam**: C2's `run_one_path` hardcodes the
  `top10_momentum_h1.toml` load (`monte_carlo.rs:852-855`); C3 must build N
  in-memory `CrossSectionalMomentumConfig` variants and pass each to
  `MomentumStrategy::from_config` (the injection point already exists —
  `crates/strategy/src/cross_sectional/momentum.rs:57`).

### Operator framings already locked (NOT re-asked here)

- **Q2 (locked):** seed the ensemble → anchor ONE distribution summary. C3
  inherits this: ONE θ-surface report = ONE anchor (§D-C3.5).
- **Q3 (locked):** harness-first / learning-loop-last. C3 is on the harness axis
  (Phase MC-2 per the backlog), upstream of C4.
- The pre-registered decision rule's **bands are frozen** (C2 lane,
  2026-05-30). C3 SCORES each θ against them; it does not redefine them. C3 adds
  only the **composite per-θ classifier in code** (the rule defines the read; C3
  mechanizes it per cell).

---

## Requirements

### R1 — The θ-grid sweep runner (the outer loop)

- **R1.1** A sweep entry point — RECOMMENDED a dedicated
  `bin/param_robustness_sweep.rs` driver (mirrors the `monte_carlo.rs` and
  `threshold_sweep.rs` bin precedents; architect M-T1 confirms bin-vs-flag).
  For each θ in a bounded grid (R1.4), it runs the **C2 inner harness**
  (N bootstrap paths → `DistributionSummary`) and collects a per-θ result.
- **R1.2** Each θ-cell runs **exactly the C2 N-path harness** — no change to
  `run_path`, `PaperEngine`, `MatchingEngine`, `DistributionSummary`, or the
  reduction order (ADR-0051 D2). The ONLY per-θ difference is the
  `CrossSectionalMomentumConfig` passed to `MomentumStrategy::from_config`.
  (R-NR.2 conformance: the engine and reducer are untouched.)
- **R1.3** The sweep MUST reuse C1's `BlockBootstrapPathGen` with
  `BlockLengthPolicy::Auto` (the shared-index, anchor-grade generator) — the
  C2 generator, unchanged. (NB: `selected_block_length_L` is auto-derived from
  the *source* return series, which is θ-independent, so L is constant across the
  θ-grid — see §D-C3.3 / Open-Q OQ-3.)
- **R1.4** v0.1.0 sweeps the **cross-sectional momentum family** over the grid
  defined in §D-C3.2. The grid is **bounded** (coarse-then-refine; total cells
  and N chosen so the full sweep fits a ~1-2 hr wall-clock budget). No other
  strategy family in v0.1.0.
- **R1.5** The per-θ inner-harness N is a CLI parameter (default per §D-C3.2);
  it is a hashed body field (a different N = a different surface).

### R2 — The θ-surface report + per-θ verdict (the verdict surface)

- **R2.1** ONE report = the **θ-surface**: a table with one row per θ-cell,
  carrying that cell's headline distribution numbers (the 5 primary signals of
  the decision rule: p5 Sharpe, p50 Sharpe, prob-of-loss, P(Sharpe>1), p95
  MaxDD tail) + the cell's composite **FRAGILE / MARGINAL / ROBUST** verdict.
- **R2.2** A per-θ **composite verdict classifier** in code, encoding the
  pre-registered rule's §4 weakest-link procedure: composite = the worst band
  any *primary* signal lands in (p5 Sharpe, prob-of-loss, p95 MaxDD, p50 Sharpe,
  P(Sharpe>1)); spread + p50-vs-real-path are interpretive, not verdict-forcing.
  The bands are the frozen rule's numbers (§0 reuse; do NOT re-derive).
- **R2.3** A **family-level summary line**: one of
  - `FAMILY-UNIFORM-FRAGILE` — every θ FRAGILE (the deflation-free conclusion);
  - `FAMILY-HAS-NON-FRAGILE-CELLS` — ≥1 θ is MARGINAL/ROBUST → **each such cell
    is flagged `→ C5 DEFLATION REQUIRED`** and C3 makes NO promotion claim
    (the §0 pre-registration commitment, mechanized).
- **R2.4** The report MUST carry a **buy-and-hold passive control row** under the
  same N paths and the same auto-L bootstrap (the adversarial review's clincher:
  passive scored p50 +1.78 vs momentum's −0.02). This is the family's honest
  benchmark — "does ANY active θ beat passively holding the same 10 coins?" — and
  is the single most decision-relevant number on the surface.
- **R2.5** The report is anchored under the existing `mc-robustness-2026-06`
  namespace (ADR-0051 D4; +1 anchor — the θ-surface body-SHA).

### R3 — Determinism + anchoring (compose ADR-0051, do not re-litigate)

- **R3.1** Per-θ sub-seeding composes ADR-0051 D1 on a **second axis**. The
  master ensemble seed is fixed; each θ-cell's inner harness uses a θ-derived
  master so cells are independent yet deterministic. RECOMMENDED rule (§D-C3.4):
  `theta_master_g = ensemble_seed.wrapping_add((g as u64).wrapping_mul(0x9E3779B9))`
  where `g` is the **θ-cell index** (bound to index, never completion order —
  the D1 invariant), then within cell `g` the existing D1 rule derives the N
  per-path seeds `path_seed_{g,j} = theta_master_g.wrapping_add(j*0x9E3779B9)`.
  Reuses the ONE golden-ratio idiom on a third axis (path, symbol, now θ-cell).
  **Architect ratifies at M-T1** (two-axis seed composition is an ADR-0051
  amendment candidate — see § ADR flag).
- **R3.2** ONE θ-surface report, sorted by θ-cell index before render
  (order-invariant body → byte-identical across runs — the `threshold_sweep`
  sort-before-render discipline). The reduction inside each cell is unchanged
  (ADR-0051 D2 index-order). NOT N per-θ reports (ADR-0051 D4 — lean to ONE,
  confirmed; §D-C3.5).
- **R3.3** Every sweep input is in the hashed body: the θ-grid definition (axes +
  ranges + resolution), per-θ N, the ensemble seed, the fill seed, the generator
  label, the source revision SHA. A different grid = a different surface = a
  different SHA (K3). Determinism scope = Apple-Silicon canonical box (ADR-0051
  D5, inherited).

### R-NR — Non-regression + the MANDATORY day-1 gate

- **R-NR.1** `verify_anchors.sh` → all existing anchors (85 at C2 ship)
  byte-identical pre/post. C3 adds +1 (the θ-surface), touches no existing
  anchored code path (the C2 inner harness is called unchanged).
- **R-NR.2** Zero behaviour change to `run_path`, `PaperEngine`,
  `MatchingEngine`, `DistributionSummary`, the reduction order, or any scenario
  `run()`. C3 is a strict outer wrapper + a new renderer.
- **R-NR.3** Money math stays `Decimal`; only the statistical metric layer + the
  verdict classifier use f64, order-fixed per ADR-0051 D2. `cargo clippy --
  -D warnings` + `cargo fmt` clean; no `.unwrap()` in library code.
- **R-NR.6 — THE day-1 falsification gate (CLAUDE.md non-negotiable, adapted to
  the sweep).** Per the v3-vol-overlay-noop precedent and the C2 R-NR.6
  adaptation: C3 adds sweep/config-injection logic, so it MUST ship a day-1 e2e
  test proving the new logic is NOT a no-op. The C3-specific failure mode is the
  **θ-injection no-op**: if the config-injection seam is mis-wired so every
  θ-cell silently runs the *same* (default θ\*) config, the whole θ-surface
  collapses to G identical rows and the sweep is a no-op in sweep's clothing.
  The mandatory gate (full design §D-C3.7) is:
  - **(a) θ-divergence gate (FP-C3.1):** two θ-cells with **materially different
    parameters** (e.g. lookback 24 + wide hold-band vs lookback 720 + tight
    hold-band) MUST produce **distinguishable distribution summaries** — assert
    `|p50_sharpe(θ_a) − p50_sharpe(θ_b)| ≥ epsilon` OR the trade-count /
    turnover differs by ≥ epsilon. If config injection is a no-op, both cells are
    byte-identical and this FAILS. **This is the falsification probe FP-C3.1.**
  - **(b) two-run byte-identity of the θ-surface body-SHA (FP-C3.3 / ADR-0051
    D2/D3):** run the whole sweep twice at the same seeds; assert identical
    `report_body_hash`.

---

## Design
_architect fills the binding M-T1 design; the analyst proposes the shape below
for the architect to ratify / amend. Everything here is a recommendation, not a
lock._

### D-C3.1 — Reuse map (what is reused verbatim vs genuinely new)

| Component | Status | Source |
|---|---|---|
| N-path inner harness (`run_path`) | **REUSE verbatim** | `crates/backtest/src/scenarios/montecarlo.rs` |
| Distribution reducer (`DistributionSummary`) | **REUSE verbatim** | `crates/backtest/src/stats/mod.rs` |
| `compute_*` metric calculators | **REUSE verbatim** | `crates/backtest/src/stats/mod.rs` |
| C1 `BlockBootstrapPathGen` (shared-index, auto-L) | **REUSE verbatim** | `crates/data/src/synth/` |
| Per-path D1 sub-seeding | **REUSE** (composed on a 2nd axis) | `monte_carlo.rs:149` `derive_path_seed` |
| Grid-enumerate → sort-before-render → 1 report | **REUSE pattern** | `threshold_sweep.rs` |
| FM/body split + fixed-precision floats | **REUSE pattern** | ADR-0051 D3 / `monte_carlo.rs:render_report` |
| **Outer θ-grid loop** | **NEW** | C3 |
| **Config-injection (in-memory θ variants)** | **NEW** (small) | C3 — replaces `monte_carlo.rs:852-855` hardcoded TOML load |
| **Per-θ composite verdict classifier** (5-signal weakest-link) | **NEW** | C3 — mechanizes decision-rule §4 |
| **θ-surface report renderer** | **NEW** | C3 |
| **Buy-and-hold control row** | **NEW** (tiny — passive equity curve) | C3 |
| **Two-axis seed composition** | **NEW rule** (ADR-0051 amendment candidate) | C3 |

Estimate: **~85% reuse**. The new surface is the outer loop + classifier +
renderer + the config-injection seam.

### D-C3.2 — The θ-grid (which params, ranges, resolution, N, wall-clock)

**The momentum family's actual tunables** (from
`CrossSectionalMomentumConfig`, `crates/strategy/src/cross_sectional/config.rs`):
`lookback_minutes`, `rebalance_minutes`, `k_long`, `exposure_cap`,
`drift_rebalance_threshold`, `vol_floor`, `size` (fixed = equal_weight). There is
**no raw "entry/exit threshold"** — momentum is rank-based top-K, so the task's
"entry/exit threshold" maps to **`k_long`** (selection breadth = the entry
cutoff) and **`drift_rebalance_threshold`** (the no-trade hold band = the turnover
/ exit control). This mapping is load-bearing and should be stated in the deck.

**Which axes to sweep, and WHY (the hypothesis-driven choice — not a blind grid):**
the C2 adversarial review localized the fragility to **turnover/fee bleed** (5343
trades/yr; buy-and-hold dominates). The grid should therefore be aimed at the
axes that change turnover and signal horizon, NOT a dense sweep of every param:

| Axis | Role / hypothesis | Coarse range (Tier-1) | Resolution |
|---|---|---|---|
| `lookback_minutes` | signal horizon. Short = noisy/high-churn; long = smoother trend. Hypothesis: longer may reduce churn. | {24, 60, 168, 720} (1d, 60-bar shipped, 1w, 1mo at 1h) | 4 |
| `k_long` | selection breadth / entry cutoff. Wider = more diversified, more legs to churn. | {1, 3, 5} | 3 |
| `drift_rebalance_threshold` | **the turnover lever** (no-trade hold band). Wide = fewer rebalances = less fee bleed. Hypothesis: the most likely escape axis. | {0.10 (shipped), 0.30, 0.50} | 3 |

- **Tier-1 (coarse) grid = 4 × 3 × 3 = 36 cells**, but trimmed: hold the two
  least-promising axes' off-values out of the full cross. RECOMMENDED coarse
  grid = **the 3 axes swept one-at-a-time around θ\* + the diagonal "low-churn
  corner"** ≈ **12-16 cells** (architect finalizes the exact cell list at M-T1;
  the principle is *hypothesis-aimed, bounded, not a dense 36-cube*).
- **`rebalance_minutes`**: held at 60 (1h) in Tier-1 — it co-moves with lookback
  for turnover and a full cross would explode the grid. A Tier-2 refine can add
  it IF Tier-1 shows a low-churn cell worth zooming into.
- **`exposure_cap`, `vol_floor`**: held at shipped values in v0.1.0 (they scale
  sizing/denominator, not the turnover or horizon hypothesis). Out of the v0.1.0
  grid; named as a Tier-3 follow-on only if needed.

**N per θ + wall-clock budget (the bounded-cost contract):**
- C2's headline N is 500 (≈ 30-40s/path-set at full rayon parallelism per the
  adversarial review's method note — NOT 3 min; the review measured
  ~30-40s/L for N=500 on the canonical box). At ~35s × 16 cells ≈ **~9-10 min**
  for a 16-cell Tier-1 sweep at N=500. **This is comfortably inside the budget**
  — the task's "3 min/θ → 1 hr" estimate was conservative; the measured C2
  throughput is ~5× faster.
- **RECOMMENDED plan: coarse-then-refine.**
  - **Tier-1 coarse:** ~12-16 cells at **N=500** ≈ ~10 min. Establishes the
    surface shape (uniform-fragile vs a low-churn island).
  - **Tier-2 refine (conditional):** ONLY if Tier-1 surfaces a non-FRAGILE
    cluster — zoom a finer grid (e.g. ±1 step on lookback + hold-band) around it
    at N=500. Skipped entirely on a uniform-FRAGILE Tier-1 (the likely case).
- **N=500 is the right per-cell N** (not lower): the decision-rule bands weight
  the p5/p95 tail heavily and N=500 is the C2-ratified tail-stability N
  (decision-rule §6 assumption 4). Dropping to N=300 is the *if-budget-tightens*
  fallback (§0), accepted only under a wall-clock constraint, with the noisier
  tail read flagged.

> **Anchored grid is FROZEN.** The exact cell list is a hashed body field (R3.3).
> Once the architect locks the Tier-1 cell list, that list IS the anchor; a
> Tier-2 refine is a SEPARATE run / SEPARATE anchor (it has a different grid), not
> a mutation of the Tier-1 surface. (ADR-0051 D4 one-report-per-grid, same logic
> as the C2 one-report-per-N.)

### D-C3.3 — Note on L (block length) being θ-independent

The auto-selected block length L is computed by Politis–White on the
**universe-average |log-return|** of the *source* series
(`crates/data/src/synth/bootstrap.rs`), which does NOT depend on the momentum θ.
So across the entire θ-grid, **L is constant** (the same resampled paths feed
every cell at a given path-seed). This is *correct and desirable*: it means the
θ-surface isolates **strategy-parameter** sensitivity on a **fixed family of
adverse paths** — the cleanest possible comparison. (Contrast: the adversarial
review's L-sweep varied L at fixed θ\*; C3 varies θ at fixed-L-policy. The two
sweeps are orthogonal and together cover both axes.) Open-Q OQ-3 asks whether to
print the single selected L once in the surface header (yes — it is a shared
input).

### D-C3.4 — Two-axis sub-seeding (compose ADR-0051 D1)

Recommended composition (architect ratifies — § ADR flag):
```text
theta_master_g  = ensemble_seed.wrapping_add((g as u64).wrapping_mul(0x9E3779B9))   # θ-cell index g
path_seed_{g,j} = theta_master_g.wrapping_add((j as u64).wrapping_mul(0x9E3779B9))  # existing D1, per cell
fill_seed       = 0xC0FFEE  (HELD CONSTANT across all cells AND all paths — D1 orthogonality)
```
- Bound to indices `(g, j)`, never to completion order (the D1 invariant on both
  axes). Cells run in parallel (rayon) and paths within a cell run in parallel;
  the seed any (cell, path) gets is a pure function of `(g, j)`.
- **Decision needed (OQ-1):** should distinct θ-cells see the **same** N paths
  (share ONE ensemble seed across cells, so every θ is judged on the *identical*
  resampled histories — maximizes comparability, isolates θ as the only varying
  input) OR **independent** paths per cell (the composition above)? The analyst
  **leans to SAME paths across cells** (set `theta_master_g = ensemble_seed` for
  all g; vary only θ) — it makes the surface a controlled experiment where the
  ONLY thing that changes between rows is θ, which is the strongest causal read
  and the cleanest anti-confounder. The composed-independent rule above is the
  fallback if the architect wants per-cell path independence for variance
  reasons. **This is a genuine M-T1 + possible operator call** (see Open-Qs).

### D-C3.5 — Anchor unit = ONE θ-surface report (confirm ADR-0051 D4)

ONE report, +1 anchor under `mc-robustness-2026-06`. Same rationale as C2's
one-report-per-N: the *surface* is the deliverable, not any single cell; every
cell is reproducible from the seeds + grid def; an N-per-θ anchor set would be a
G×-explosion that re-locks on any grid change. **Lean to ONE — confirmed.**
(ADR-0051 D4 Option C — a sampled-cell digest embedded in the hashed body for
extra tamper-evidence — remains an acceptable future upgrade, not v0.1.0.)

### D-C3.6 — The genuinely-new code (bounded list for the developer)

1. `bin/param_robustness_sweep.rs` — CLI (grid def, N, seed, out-dir, year) +
   outer θ-loop calling the C2 inner harness per cell + sort-before-render.
2. Config-injection: build N in-memory `CrossSectionalMomentumConfig` from a
   base + per-θ overrides; pass to `MomentumStrategy::from_config`. (Replaces the
   hardcoded TOML load at `monte_carlo.rs:852-855` — likely a small refactor of
   `run_one_path` to accept a caller-supplied config, OR a C3-local copy of the
   path-runner glue that takes a config. Architect picks the least-invasive seam
   that keeps `run_path` byte-identical — R-NR.2.)
3. `ParamRobustnessVerdict` classifier — the 5-signal weakest-link composite
   (decision-rule §4) returning FRAGILE/MARGINAL/ROBUST per cell.
4. θ-surface renderer (one report; G rows + buy-and-hold control row + family
   summary line + per-cell `→ C5` flags).
5. Buy-and-hold control: a passive equal-weight equity curve over the same paths
   (the adversarial review already has the reference numbers; this is a ~30-line
   passive run, NOT a strategy).

### D-C3.7 — The MANDATORY day-1 gate (R-NR.6, full design)

Mirror C2's `montecarlo_e2e.rs` two-part structure exactly:
- **FP-C3.1 (θ-divergence — the headline anti-no-op):** an e2e test runs the
  sweep on a 2-cell grid `{θ_low_churn, θ_high_churn}` (materially different
  lookback + hold-band) at small N and asserts the two cells' distribution
  summaries are **distinguishable** (`|Δp50_sharpe| ≥ ε` OR `|Δtrade_count| ≥ ε`).
  THEN, as the falsification dry-run, force the injection to ignore the override
  (both cells run θ\*) and assert the test goes RED — proving the gate detects
  the injection no-op (exactly the FP-C2.1 discipline: test the gate on BOTH the
  real and the degenerate case).
- **FP-C3.2 (grid sensitivity / K3):** two sweeps over DIFFERENT grids at the
  same seeds ⇒ different θ-surface body-SHAs (the grid def is in the hashed body).
- **FP-C3.3 (two-run determinism / R3.2):** same grid + same seeds twice ⇒
  byte-identical surface body-SHA.

> Pattern reference (single-path analogue, CLAUDE.md non-negotiable):
> `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`. Distribution
> analogue: `crates/backtest/tests/montecarlo_e2e.rs` (C2).

### § ADR flag (for the architect — C3 does NOT author it)

C3's **two-axis seed composition** (θ-cell axis ⊗ path axis, D-C3.4) and the
**θ-surface report shape** (one report, G rows, per-cell verdict) are an
**ADR-0051 amendment** candidate, NOT a new ADR — the determinism+anchoring
contract is already ADR-0051's; C3 extends it to a second sweep axis. The
architect decides at M-T1 whether this is (i) an ADR-0051 Changelog amendment
(if the composition is a clean reuse of D1 on a new axis with no new
determinism risk — analyst's read is YES, this is the cheap-and-correct case the
analyst-defaults exception allows) or (ii) a small sibling ADR-0052 (if the
two-axis composition needs its own decision record). **Analyst recommendation:
an ADR-0051 amendment** — the seed idiom, the FM/body split, the fixed-precision
formatting, and the one-report anchor unit are all reused verbatim; only the axis
count changes. No carve-out, no MIGRATION, no v0.X+1 follow-on is spawned by the
amendment route, so the cheap path is provably rework-free here.

---

## Backtest Scenarios
_analyst + architect fill using the backtest/scenario template._

The v0.1.0 anchored scenario: **cross-sectional momentum (v1) swept over the
Tier-1 θ-grid (§D-C3.2) at N=500 per cell, over the shared-index block-bootstrap
of 2023-FY real Binance returns (revision `3a8b96c4…`), producing ONE θ-surface
report under `mc-robustness-2026-06`.** Each cell scored against the frozen
decision-rule bands; family-level verdict + per-cell `→ C5` flags. Buy-and-hold
control row included. (2024-FY is an optional second scenario, same shape, IF the
operator wants both years — mirrors the C2 / threshold_sweep bs1/bs2 pattern.)

---

## Open questions (operator / architect — resolve before build)

- **OQ-1 (same vs independent paths across θ-cells) — leans SAME, M-T1 +
  possible operator call.** Should every θ be judged on the *identical* N
  resampled histories (one ensemble seed shared across cells; θ the only varying
  input — strongest causal read) or independent paths per cell (the D-C3.4
  composition)? Analyst leans SAME (controlled experiment, anti-confounder). Has
  a determinism consequence (the hashed seed rule differs) so the architect must
  ratify; the operator may have a view on the "controlled experiment" framing.
- **OQ-2 (grid finalization) — architect M-T1.** The exact Tier-1 cell list
  (the §D-C3.2 "~12-16 hypothesis-aimed cells"). The analyst gives the axes,
  ranges, and the low-churn-corner principle; the architect locks the precise
  cells (which IS the anchor).
- **OQ-3 (L printing) — trivial, architect.** Print the single auto-selected L
  once in the surface header (it is θ-independent, D-C3.3) rather than per-row.
  Analyst recommends yes (one shared-input line).
- **OQ-4 (ADR amendment vs ADR-0052) — architect M-T1.** Per § ADR flag —
  analyst recommends an ADR-0051 amendment (provably rework-free reuse).
- **OQ-5 (one year or two) — operator.** 2023-FY only (matches C2's anchored
  scenario) vs 2023 + 2024 (matches threshold_sweep). Analyst leans 2023-only
  for v0.1.0 (the C2 comparison is apples-to-apples) with 2024 as a fast
  follow-on, since adding a year doubles wall-clock but the Tier-1 budget is tiny.

---

## Falsification probes (C3 — for the developer M-DEV dry-run)
_(Restated compactly; full design in §D-C3.7.)_

- **FP-C3.1 — θ-injection no-op detector (the headline gate).** Force config
  injection to ignore the per-θ override → the θ-divergence test MUST go red
  (all cells identical). Proves the sweep is not a no-op in sweep's clothing.
- **FP-C3.2 — grid sensitivity (K3).** Different grid def → different surface
  body-SHA. Proves the grid is a hashed input (no missing body field).
- **FP-C3.3 — two-run determinism.** Same grid + seeds twice → identical
  body-SHA. Proves no unordered fold snuck into the outer loop.
- **FP-C3.4 — buy-and-hold control sanity.** The passive control row reproduces
  the adversarial review's reference (p50 Sharpe ≈ +1.78, P(loss) ≈ 4%, p95
  MaxDD ≈ 51% at auto-L, N=500). A wildly different control row means the control
  wiring (not the sweep) is wrong.
- **FP-C3.5 — the integrity probe (anti-cherry-pick).** Assert C3 emits NO
  "best θ is ROBUST" claim: the family summary line is one of the two §R2.3
  values, and any non-FRAGILE cell carries the `→ C5 DEFLATION REQUIRED` flag.
  This is the mechanized §0 pre-registration commitment — the gate that the
  multiple-testing discipline is enforced in code, not just prose.

---

## Implementation
_developer fills this._

## Verification
_tester links to reports here. The day-1 gates: FP-C3.1 (θ-divergence /
anti-no-op) + FP-C3.3 (two-run byte-identity), both MANDATORY per CLAUDE.md._

---

## Proposed trace.toml [[req]] row (for the orchestrator/architect to add on greenlight)

> **NOT written by this brief** — the task constraint is read-only on
> `spec/trace.toml`, and C3 is pre-greenlight. The analyst normally owns the
> `[[req]]` creation; here it is staged for the orchestrator to commit IF the
> operator picks C3 over a pivot/C5. Proposed minimum row:

```toml
[[req]]
id          = "REQ-MOMENTUM-PARAMETER-ROBUSTNESS-SWEEP-001"
title       = "C3 — momentum parameter-robustness sweep: for each θ in a bounded hypothesis-aimed grid (lookback × k_long × drift/hold-band) run the C2 N-path robustness harness → per-θ DistributionSummary → per-θ FRAGILE/MARGINAL/ROBUST verdict (frozen decision-rule §4 weakest-link composite). ONE anchored θ-surface report + buy-and-hold control row. Anti-cherry-pick by construction: reports the FULL surface + family verdict; defers any 'best θ is robust' claim to a C5 deflation pass (Option a, pre-registered). Reuses run_path + DistributionSummary + BlockBootstrapPathGen + threshold_sweep grid pattern verbatim; new = outer θ-loop + config-injection + composite classifier + surface renderer."
feature     = "momentum-parameter-robustness-sweep"
product     = "spec/product.md"
arch        = []   # architect fills (ADR-0051 amendment OR ADR-0052 — OQ-4)
crates      = []   # developer fills (likely crates/backtest: bin/param_robustness_sweep.rs)
tests       = []   # developer fills (FP-C3.1 θ-divergence + FP-C3.3 two-run identity, mandatory)
anchors     = []   # tester fills (+1 θ-surface under mc-robustness-2026-06)
state       = "proposed"
```

---

## Changelog

- 2026-05-30 (analyst, monte-carlo-robustness-lane C3): feature.md authored as
  decision-grade SCOPING for C3 (the parameter-robustness axis), reversible
  pre-greenlight design. **Front-and-center methodology decision settled:
  Option (a) — C3 reports the FULL θ-surface + family verdict + per-cell
  `→ C5 DEFLATION REQUIRED` flags; NO argmax-selected "best θ is robust" claim;
  deflation deferred to C5 (pre-registered, anti-cherry-pick by construction).**
  Quantified the multiple-testing hazard (1−0.95^24 ≈ 71% family-wise false-
  ROBUST at G=24). Recommended a hypothesis-aimed bounded θ-grid (lookback ×
  k_long × drift/hold-band, ~12-16 Tier-1 cells, coarse-then-refine) aimed at the
  turnover/fee-bleed axis the C2 adversarial review localized; corrected the
  wall-clock estimate (~35s/cell at N=500 measured, not 3 min → ~10 min Tier-1,
  not 1 hr). Mapped the momentum family's real tunables (no raw entry/exit
  threshold; k_long = entry cutoff, drift_rebalance_threshold = turnover/exit
  lever). Specified two-axis sub-seeding (ADR-0051 D1 composed on the θ-cell
  axis), ONE anchored θ-surface report (D4 confirmed), the day-1 θ-injection-
  no-op gate (R-NR.6 adapted; FP-C3.1), and an ~85% reuse / bounded-new split.
  Flagged ADR-0051 amendment (not a new ADR) for the architect. R2.4 buy-and-hold
  control row carried forward from the adversarial review as the family's honest
  benchmark. Proposed (not written) the REQ-MOMENTUM-PARAMETER-ROBUSTNESS-SWEEP-001
  trace row for the orchestrator to add on greenlight (trace.toml read-only this
  pass). Depends on C2 (REQ-STRATEGY-ROBUSTNESS-HARNESS-001, tested/PASS) + C1.
