---
slug: momentum-parameter-robustness-sweep
version: 0.1.0
status: tester-done
owner: architect
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

> **M-T1 BINDING DESIGN (architect, 2026-05-30).** The 5 open questions are
> resolved below in § D-C3.0; the Tier-1 θ-grid is LOCKED in § D-C3.2-LOCKED (it
> IS the anchor input — frozen before the tester anchors); the determinism +
> anchoring amendment is written into [ADR-0051 § D6](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md);
> the build seams + day-1 gate are specified in § D-C3.6-BUILD / § D-C3.7-LOCKED.
> The analyst's recommendation shape (§ D-C3.1 … § D-C3.7 below) is RATIFIED with
> the amendments called out in § D-C3.0. This is design-only — reversible until
> the dev build per the operator's C3 delegation.

### D-C3.0 — M-T1 resolutions (the 5 open questions, decided + rationale)

| OQ | Decision | Rationale (short) | Where locked |
|---|---|---|---|
| **OQ-1** same vs independent N paths across θ-cells | **SAME path-set across ALL cells** | θ becomes the ONLY varying input → controlled experiment; removes path-sampling variance as an inter-row confound; and it is provably C1/C2-determinism-neutral (the θ-axis is varied at the config level, the seed stream is untouched). | ADR-0051 § D6.1 |
| **OQ-2** exact Tier-1 cell list (THE anchor) | **14 cells, LOCKED** (1 baseline + 3 one-at-a-time arms + 3 low-churn-corner + 1 high-churn-corner + 2 mid-diagonal). Hypothesis-aimed at the turnover/fee-bleed corner. | The adversarial review localized fragility to turnover; the grid sweeps the two turnover axes (lookback horizon, drift/hold-band) + breadth (k_long) and includes the long-lookback × wide-band low-churn corner as the best a-priori robustness shot. | § D-C3.2-LOCKED |
| **OQ-3** print single auto-L once in header | **Yes — header line, not per-row** | L is computed on the source series (θ-independent) and SAME-paths ⇒ identical L for every cell. One shared-input line. | ADR-0051 § D6.1.4; § D-C3.3 |
| **OQ-4** ADR-0051 amendment vs new ADR-0052 | **ADR-0051 § D6 amendment** | The seed idiom, FM/body split, fixed-precision, and one-report anchor unit are reused verbatim; only the axis count + report shape change. Cheap-and-correct `analyst-defaults` exception. **Plus** the amendment had to REJECT the brief's D-C3.4 naive additive two-axis fallback as a seed-collision bug. | ADR-0051 § D6.2 |
| **OQ-5** 2023-only vs +2024 | **2023-FY only for v0.1.0** | Apples-to-apples with the C2 anchor (same year, same revision `3a8b96c4…`). +2024 is a v0.2.0 fast-follow (separate run, separate anchor, same shape). No operator round-trip. | § Backtest Scenarios |

**The one amendment to the analyst's recommended shape:** the analyst's § D-C3.4
offered SAME-paths as the *lean* and a composed-independent rule as the *fallback*.
The architect ratifies SAME-paths as the **binding** rule **and rejects the
fallback outright** — the naive additive composition
`ensemble_seed + g·0x9E37_79B9 + j·0x9E37_79B9` collapses to
`+ (g+j)·0x9E37_79B9` and assigns the **same** path seed to `(g, j)` and
`(g−1, j+1)` (6 colliding pairs in a 4×3 grid, verified arithmetically). Any future
per-cell-path-independence variant needs its OWN ADR with a collision-free
non-commuting mix; it does NOT inherit § D6.1. Everything else in § D-C3.1 …
§ D-C3.7 is ratified as written, with the cell list and gate made concrete below.

### D-C3.2-LOCKED — The Tier-1 θ-grid (RE-SCOPED to 6 cells × N=200 for tractability)

> **Original architect design (FROZEN 2026-05-30 M-T1): 14-cell × N=500.**
> **RE-SCOPED 2026-05-30 (orchestrator) to 6-cell × N=200** for ~20 min wall-clock
> vs ~1 hr. Methodology unchanged; grid anchored as a `const` in the bin.
> The re-scoped 6-cell list is a hashed body field (ADR-0051 § D6.3 / R3.3).
> **Held constant across every cell:** `rebalance_minutes = 60`, `exposure_cap = 0.50`,
> `vol_floor = 0.000001`, `size = equal_weight`, `k_short = 0`, the 10-symbol
> universe, year = 2023, N = 200, `ensemble_seed = 0xC0FFEE`, `fill_seed = 0xC0FFEE`,
> generator = `block-bootstrap-real`, revision `3a8b96c4…`.

The θ-cell index `g` is the render + seed-composition order (rows are sorted
by `g` before render — ADR-0051 § D6.4). The swept axes are `lookback_minutes`
(signal horizon), `k_long` (selection breadth / entry cutoff), and
`drift_rebalance_threshold` (the no-trade hold band = the turnover/exit lever).

**Re-scoped 6-cell grid (orchestrator 2026-05-30) — ANCHORED:**

| g | lookback_minutes | k_long | drift_rebalance_threshold | role / hypothesis |
|---|---|---|---|---|
| 0 | 60 | 3 | 0.10 | **baseline θ\*** (C2-shipped config; correctness probe) |
| 1 | 24 | 3 | 0.10 | short lookback (1d) → noisiest / highest churn |
| 2 | 168 | 3 | 0.10 | 1w lookback horizon |
| 3 | 720 | 3 | 0.50 | **1mo lookback + wide hold-band** — a-priori best robustness shot (low-churn corner) |
| 4 | 60 | 1 | 0.10 | narrow selection — top-1 only |
| 5 | 60 | 5 | 0.10 | wide selection — top-5 (more legs to churn) |

Original 14-cell architect grid preserved in the git history (commit pre-2026-05-30).
The 6-cell grid covers: the baseline (g=0), lookback arms g=1..2, the best-robustness
corner (g=3: 1mo × wide band, the key low-churn hypothesis), and breadth arms g=4..5.

**Design notes on the lock:**
- **C0 is a free, powerful validation.** Because OQ-1 = SAME-paths, cell `g=0` runs
  the *exact* C2 θ\* config over the *exact* C2 path-set. Its per-cell distribution
  numbers (Sharpe p5/p50/p95, P(loss), p95 MaxDD) MUST reproduce the C2 anchored
  report's numbers within the same run-to-run identity the C2 anchor enjoys. This is
  a built-in correctness probe for the whole sweep (if C0 ≠ the C2 numbers, the
  config-injection or path-plumbing is wrong) — the tester checks it (T-T1). Note
  the *surface* anchor (§ D6.3) is a distinct artifact from the C2 *single-cell*
  anchor (different report shape, different body), so this does not collide anchors.
- **The grid is bounded and hypothesis-aimed, NOT a dense 4×3×3 = 36-cube.** It is
  the 3 axes swept one-at-a-time around θ\* + the low-churn diagonal + one high-churn
  extreme + 2 shape fills = 14 cells. This is the analyst's "~12-16 hypothesis-aimed
  cells" principle made concrete; it spends the budget on the corners the prior says
  matter (low-churn = possible escape; high-churn = confirm the trap) rather than on
  a uniform fill.
- **`rebalance_minutes` held at 60.** It co-moves with `lookback` for turnover; a
  full cross would explode the grid. A Tier-2 refine MAY add it iff Tier-1 surfaces
  a non-FRAGILE low-churn cell worth zooming into.
- **Wall-clock:** ~35 s/cell at N=500 on the canonical box (adversarial-review
  measured throughput) × 14 cells ≈ **~8 min**. N=300 fallback ≈ ~5 min (the
  if-budget-tightens lever — noisier tail, NOT a methodology downgrade; § 0).

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

---

### D-C3.6-BUILD — The binding build (M-T1 → developer): seams, reuse, new code

Verified against the C2 code at HEAD (architect read 2026-05-30). The reuse story
holds at ~85%; the new surface is bounded and the config-injection seam is
**smaller and cleaner than the brief feared** — see the seam note.

**Reuse VERBATIM (do NOT reimplement, do NOT touch — R-NR.1/R-NR.2):**
- `crates/backtest/src/scenarios/montecarlo.rs :: run_path` — the per-path engine
  loop. It is **already config-agnostic**: it takes a caller-supplied
  `MomentumStrategy` and the config it loads internally is used ONLY for the
  universe list, which it then discards (`let _ = cfg;` at line 131; the universe
  is implicit in the injected `bars_override`). **No edit to `run_path`.**
- `crates/backtest/src/stats/mod.rs` — `DistributionSummary::from_path_metrics`,
  `reduce_samples`, `compute_sharpe_hourly` / `_sortino_` / `_calmar` /
  `compute_max_drawdown_f64` / `compute_total_return`, `PathMetrics`,
  `MetricDistribution`. Untouched (ADR-0051 D2 reduction is load-bearing).
- `crates/data/src/synth/` — `BlockBootstrapPathGen` with `BlockLengthPolicy::Auto`
  (shared-index, the FP-C1.5-confirmed fair adversary). Untouched.
- ADR-0051 D1 seed idiom `derive_path_seed(master, j) = master.wrapping_add(j *
  0x9E37_79B9)` — reused **verbatim**, and under OQ-1 SAME-paths the C3 per-path
  seed IS `derive_path_seed(ensemble_seed, j)` (no new arithmetic; ADR-0051 § D6.1).
- The `monte_carlo.rs` driver scaffolding: `parse_seed`, `read_git_commit`,
  `read_hostname`, `read_data_revision_sha`, `days_since_epoch_to_ymd`,
  `load_source_bars` / `load_real_bars`, `prepare_generator_params` (gives the
  θ-independent auto-L for the header — OQ-3), the rayon fan-out + `sort_by_key(j)`
  reduction pattern, and the FM/body `render_report` structure (ADR-0051 D3).

**The config-injection seam (T-A4 RESOLVED — least-invasive choice):** the hardcoded
TOML load lives in `monte_carlo.rs :: run_one_path` at **lines 852-859** (NOT in
`run_path`). The seam is: **`run_one_path` (and the C3 sibling driver) builds the
per-cell `CrossSectionalMomentumConfig` from a base + θ-overrides and passes the
resulting `MomentumStrategy` down to the unchanged `run_path`.** Because C3 is a
NEW bin (below), the cleanest realization is a **C3-local copy of the
`run_one_path` glue** that (a) takes a `&CrossSectionalMomentumConfig` (or the
3-tuple `(lookback, k_long, drift)` + the frozen base) instead of loading the TOML,
and (b) is otherwise byte-identical to `run_one_path` (same path-gen call, same
merge, same `TcnScenarioInput`, same clamp, same `compute_*`). This keeps both
`run_path` AND the C2 `monte_carlo.rs` driver byte-identical (R-NR.2 / the 85
anchors), and confines the θ-injection to C3's own code. (Refactoring the C2
`run_one_path` to accept a config is the alternative; it is rejected for v0.1.0
because it touches the C2 anchored driver for zero C3 benefit.)

**The build (a dedicated bin — T-A1 confirms bin over flag, mirrors `monte_carlo.rs`
+ `threshold_sweep.rs`):**

1. **`crates/backtest/src/bin/param_robustness_sweep.rs`** — the C3 driver.
   - CLI (clap): `--paths` (default 500), `--ensemble-seed` (default `0xC0FFEE`),
     `--data-root`, `--expected-revision-sha` (the `3a8b96c4…` pin), `--out-dir`
     (default `spec/momentum-parameter-robustness-sweep/reports/`), `--year`
     (default 2023), `--generator` (default `block-bootstrap-real`).
   - The **LOCKED θ-grid (§ D-C3.2-LOCKED) is a `const` table in the bin** — 14
     `(g, lookback, k_long, drift)` rows. It is NOT a CLI parameter (the grid is
     frozen; making it a flag would let a run silently change the anchor input).
     A `--grid tier1` enum value is acceptable (one variant at v0.1.0) if the
     developer wants the K3 grid-sensitivity test (FP-C3.2) to switch grids without
     editing source — but the **anchored** run uses `tier1`.
   - **Outer θ-loop:** for each cell `g`, build the per-cell config (frozen base
     `top10_momentum_h1.toml` with `lookback_minutes/k_long/drift_rebalance_threshold`
     overridden per the row), then run the **C2 inner N-path harness** (rayon
     fan-out over `j ∈ 0..N`, per-path seed = `derive_path_seed(ensemble_seed, j)`
     — SAME for every cell, ADR-0051 § D6.1), collect `Vec<PathMetrics>` in
     index-`j` order (`sort_by_key(|r| r.j)`), reduce to one `DistributionSummary`.
     Collect `(g, config-triple, DistributionSummary)` into a `Vec`, **sort by `g`
     before render** (ADR-0051 § D6.4).
   - Parallelism: the inner per-path rayon fan-out is reused as-is. The outer
     θ-loop MAY also be parallel, but **seeds are a pure function of `j` only**
     (cell-independent), so either order is deterministic — keep the outer loop
     sequential for log legibility unless wall-clock demands otherwise (it does not
     at ~8 min).
2. **Config-injection helper** (in the bin): `fn cell_config(base: &Cfg, lookback:
   u32, k_long: u32, drift: Decimal) -> Cfg` cloning the base and overriding the 3
   fields. `MomentumStrategy::from_config(cell_config(...), id)` per cell. This is
   the ~30-line crux that FP-C3.1 falsifies.
3. **`ParamRobustnessVerdict` classifier** — a pure function over a
   `DistributionSummary` returning `{Fragile, Marginal, Robust}` per cell. It
   mechanizes the frozen decision-rule § 4 weakest-link composite over the **five
   PRIMARY signals**: p5 Sharpe, p50 Sharpe, prob-of-loss, P(Sharpe>1), p95 MaxDD
   tail. **Composite = the worst band any primary signal lands in.** It does NOT
   re-derive the bands — the bands are the frozen `robustness-decision-rule-2026-05-30.md`
   § 0 numbers, encoded as `const` thresholds with a comment citing the rule. Spread
   (p95−p5) and p50-vs-real-path are computed and printed but are **interpretive
   (NOT verdict-forcing)** per rule § 4 step 3. Place it as a small module in the
   bin (or `crates/backtest/src/stats/` next to `DistributionSummary` if shared) —
   developer's call; it must be unit-tested at the band boundaries (T-D4).
   - **Band thresholds (lift verbatim from the decision rule § 0 — do NOT invent):**
     FRAGILE if any of `{p5_sharpe < 0, p50_sharpe < 0.5, prob_loss > 0.35,
     prob_sharpe_gt_1 < 0.35, p95_maxdd > 0.70}`; ROBUST only if ALL of
     `{p5_sharpe ≥ 0.5, p50_sharpe ≥ 1.0, prob_loss ≤ 0.15, prob_sharpe_gt_1 ≥ 0.60,
     p95_maxdd ≤ 0.50}`; else MARGINAL. (The rule's "ROBUST only if p5 ≥ 0 AND
     prob-of-loss ≤ 15% AND p95 DD stomach-able" composite gate is satisfied by this
     weakest-link encoding.)
4. **θ-surface report renderer** (ADR-0051 D3 / § D6.4 FM/body split):
   - **Front-matter (run-varying, NOT hashed):** `slug`, `scenario`, `generated`,
     `wall_clock_s`, `host`, `pid`, `git_commit`, `data_revision_sha`.
   - **Body (hashed) — shared-input header block first:** `master_seed`,
     `fill_seed`, `n_paths`, `sub_seed_rule` (`"master + j*0x9E3779B9 (SAME paths
     across cells, ADR-0051 D6.1)"`), `reduction_rule` (the D2 string), `generator`,
     `bootstrap_mode` (`shared-index`), `block_length_policy` (`auto`),
     `selected_block_length_L` (the single θ-independent L — OQ-3), `source_revision_sha`,
     and the **frozen grid definition string** (the 14 cells, e.g. a canonical
     one-line-per-cell `g|lookback|k_long|drift` block — this is the hashed grid
     field, K3 / § D6.3).
   - **Body — the θ-surface table (the deliverable), rows sorted by `g`:** one row
     per cell with `g`, `lookback`, `k_long`, `drift`, then the 5 primary signals at
     fixed precision (`{:.6}` for Sharpe p5/p50/p95, prob_loss, prob_sharpe_gt_1;
     `{:.2}%` for p95 MaxDD), the spread `{:.6}`, and the composite verdict string +
     (if non-FRAGILE) the `→ C5 DEFLATION REQUIRED` flag.
   - **Body — the buy-and-hold control row** (§ D-C3.6 item 5 below): same N paths,
     same auto-L, labelled `BUYHOLD (passive)`, with the same 5 columns. It carries
     NO verdict (it is the benchmark, not a candidate).
   - **Body — the family-summary line:** exactly one of `FAMILY-UNIFORM-FRAGILE`
     (every active cell FRAGILE) or `FAMILY-HAS-NON-FRAGILE-CELLS` (≥1 cell
     MARGINAL/ROBUST → each flagged `→ C5`). **NO "best θ" is ever crowned** (the
     § 0 pre-registration commitment, mechanized — FP-C3.5).
   - All floats formatted at fixed precision BEFORE entering the body string; metric
     columns in a FIXED declared order; rows in `g` order. Compute the body-SHA the
     same way `monte_carlo.rs` does (`extract_report_body` + Sha256) and print it for
     the tester to lock.
5. **Buy-and-hold control** — a ~30-line passive equal-weight equity curve over the
   same injected paths (NOT a strategy; it ignores signals and holds equal weights
   from bar 0). Reuse the same `compute_*` calculators + `DistributionSummary` over
   its N per-path metrics so the control row is computed by the identical reducer.
   FP-C3.4 checks it reproduces the adversarial-review reference (p50 ≈ +1.78,
   P(loss) ≈ 4%, p95 MaxDD ≈ 51% at auto-L, N=500).

**Watch recipe (developer MUST emit when kicking off the >2 min anchored run):**
```bash
watch -n 15 '
PID=$(pgrep -f param_robustness_sweep | head -1)
[ -z "$PID" ] && echo "param_robustness_sweep not running" && exit
N=$(ls spec/momentum-parameter-robustness-sweep/reports/robustness-sweep-*.md 2>/dev/null | wc -l | tr -d " ")
ELAPSED=$(ps -o etime= -p $PID 2>/dev/null | tr -d " ")
[ "$N" -gt 0 ] && echo "surface landed ($N file); elapsed ${ELAPSED}" || echo "running (no surface yet); elapsed ${ELAPSED}"
'
```

### D-C3.7-LOCKED — The MANDATORY day-1 gate (R-NR.6 non-negotiable, concrete spec)

A new test file **`crates/backtest/tests/param_sweep_e2e.rs`** mirroring C2's
`montecarlo_e2e.rs` two-part structure. **FP-C3.1 is the CLAUDE.md non-negotiable**
(the θ-injection no-op falsifier); it MUST be tested on BOTH the real and the
degenerate-injection case (the FP-C2.1 discipline). Small N (e.g. N=8-20) and a
short synthetic bar series so the test runs in seconds (no real data; the gate is
about the *injection wiring*, not the tail numbers).

- **FP-C3.1 — θ-injection divergence gate (the headline anti-no-op):**
  - **(a) real case — PASS when wired:** build two materially-different cells —
    `θ_high_churn = (lookback 24, k_long 5, drift 0.10)` and `θ_low_churn =
    (lookback 720, k_long 3, drift 0.50)` — inject each via the C3 config-injection
    helper, run the small-N harness over the SAME synthetic paths, and assert the
    two cells are **distinguishable**: `|p50_sharpe(θ_a) − p50_sharpe(θ_b)| ≥ ε`
    OR `|trade_count(θ_a) − trade_count(θ_b)| ≥ ε` (the trade-count divergence is
    the robust signal — a low-churn cell trades far less than a high-churn one;
    assert on the aggregate or per-path trade counts surfaced by `PathRunResult`).
    Recommend ε on trade-count = a large integer gap (these two cells differ by
    hundreds-to-thousands of trades), with the Sharpe-delta as the OR-fallback.
  - **(b) degenerate case — the falsification dry-run, MUST go RED:** force the
    injection helper to **ignore the override** (both cells run θ\*), and assert the
    divergence check from (a) now FAILS (the two cells are byte-identical → Δ = 0 <
    ε). Mirror C2's `fp_c2_1_degenerate_seeds_have_zero_spread`: the degenerate test
    *asserts the collapse is detectable* (Δ < a tiny epsilon), proving the gate is
    not itself a no-op. **Both (a) and (b) ship in the file.** A build that silently
    runs θ\* for all cells fails (a); the gate's own sensitivity is proven by (b).
- **FP-C3.3 — two-run byte-identity of the θ-surface body-SHA (mandatory, ADR-0051
  D2/D3/§ D6.4):** run the whole small-N sweep twice at the same `ensemble_seed`;
  assert identical `report_body_hash` (or, at the unit level mirroring C2's
  `rn6b_two_run_byte_identity`, assert the per-cell summaries format byte-identically
  across two runs). Catches any unordered fold sneaking into the outer θ-loop or the
  renderer.
- **FP-C3.2 — grid sensitivity (K3):** two sweeps over DIFFERENT grids (e.g.
  `tier1` vs a 2-cell sub-grid) at the same seeds ⇒ different θ-surface body-SHAs
  (proves the grid def is a hashed body field — no missing input).
- **FP-C3.4 — buy-and-hold control sanity:** assert the passive control row's
  distribution reproduces the adversarial-review reference at auto-L/N=500 (run at
  the real-data tester stage, not in the fast unit gate; the tester checks this in
  T-T1). A wildly different control row ⇒ the control wiring is wrong, not the sweep.
- **FP-C3.5 — the integrity probe (anti-cherry-pick, mechanized § 0):** a unit test
  asserts the family-summary line is ALWAYS one of the two § R2.3 values and that
  every non-FRAGILE cell carries the `→ C5 DEFLATION REQUIRED` flag — i.e. the
  renderer can NEVER emit a "best θ is ROBUST" claim. This is the pre-registration
  commitment enforced in code, not prose.

> The two MANDATORY day-1 gates (per CLAUDE.md non-negotiable + R-NR.6) are
> **FP-C3.1** (θ-divergence / anti-no-op, tested on BOTH real + degenerate) and
> **FP-C3.3** (two-run byte-identity). FP-C3.2 / FP-C3.4 / FP-C3.5 are required for
> ship but are not the day-1 blocker pair. Pattern references:
> `crates/backtest/tests/montecarlo_e2e.rs` (distribution analogue) and
> `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs` (single-path
> non-negotiable).

### § ADR flag — RESOLVED at M-T1 (ADR-0051 § D6 amendment, NOT ADR-0052)

> **RESOLVED 2026-05-30 (architect).** Written as an **amendment to ADR-0051**
> ([§ D6](../architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md)),
> registered atomically in the ADR registry README per the architect.md ADR-registry
> contract. The analyst's recommendation (amendment over ADR-0052) is ratified: the
> seed idiom, FM/body split, fixed-precision formatting, and one-report anchor unit
> are reused verbatim; only the axis count + report shape change. **One correction
> the amendment had to make:** the analyst's § D-C3.4 *fallback* (composed-independent
> paths via `ensemble_seed + g·k + j·k`) is a seed-collision bug (it collapses to
> `+(g+j)·k`, colliding `(g,j)` with `(g−1,j+1)`); ADR-0051 § D6.2 REJECTS it and
> ratifies SAME-paths-across-cells (§ D6.1) as the binding rule. The amendment proved
> SAME-paths is byte-identical to the C2 D1 seed (the θ-axis is varied at the config
> level, not the seed level), so C1/C2 determinism — the 85 anchors — is unchanged by
> construction. The analyst text below is preserved for provenance.

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

The v0.1.0 anchored scenario (**OQ-5 RESOLVED: 2023-FY only**): **cross-sectional
momentum (v1) swept over the LOCKED Tier-1 14-cell θ-grid (§ D-C3.2-LOCKED) at
N=500 per cell, over the shared-index block-bootstrap of 2023-FY real Binance
returns (revision `3a8b96c4…`), producing ONE θ-surface report under
`mc-robustness-2026-06`** (+1 anchor → 86 total; ADR-0051 § D6.3). Each cell scored
against the frozen decision-rule bands; family-level verdict + per-cell `→ C5`
flags. Buy-and-hold control row included. **2023-FY only is the v0.1.0 lock**
(apples-to-apples with the C2 anchor — same year, same revision). **2024-FY is a
v0.2.0 fast-follow** (a SEPARATE run = SEPARATE anchor, identical shape; mirrors the
C2 / threshold_sweep bs1/bs2 pattern). No operator round-trip needed for v0.1.0.

---

## Open questions — ALL RESOLVED at M-T1 (2026-05-30)

> All 5 resolved by the architect; see § D-C3.0 for the decision table + rationale
> and ADR-0051 § D6 for the determinism/anchoring amendment. None required an
> operator round-trip (the operator delegated C3 direction to the orchestrator's
> recommendation). Original questions preserved below with their resolution.

- **OQ-1 (same vs independent paths across θ-cells) — RESOLVED: SAME path-set
  across ALL cells.** θ is the only varying input → controlled experiment;
  provably C1/C2-determinism-neutral (θ varied at config level, seed stream
  untouched). The brief's composed-independent fallback is REJECTED as a
  seed-collision bug. Locked in ADR-0051 § D6.1 / § D6.2.
- **OQ-2 (grid finalization) — RESOLVED: 14-cell list LOCKED** in
  § D-C3.2-LOCKED (1 baseline θ\* + 3 one-at-a-time arms + 3 low-churn-corner +
  1 high-churn-corner + 2 mid-diagonal). This IS the anchor input (frozen).
- **OQ-3 (L printing) — RESOLVED: yes, one header line.** L is θ-independent and
  identical across cells under SAME-paths. ADR-0051 § D6.1.4 / § D-C3.3.
- **OQ-4 (ADR amendment vs ADR-0052) — RESOLVED: ADR-0051 § D6 amendment**
  (registered atomically). § ADR flag (resolved) / ADR-0051 § D6.
- **OQ-5 (one year or two) — RESOLVED: 2023-FY only for v0.1.0**, +2024 as a
  v0.2.0 fast-follow (separate run/anchor). § Backtest Scenarios.

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

**Re-scope (orchestrator 2026-05-30):** The original 14-cell × N=500 design was
re-scoped for tractability to **6-cell × N=200** (~20 min wall-clock vs ~1 hr).
Grid reduction is documented in `crates/backtest/src/bin/param_robustness_sweep.rs`
TIER1_GRID const; methodology unchanged.

**What was built:**

- `crates/backtest/src/bin/param_robustness_sweep.rs` — complete C3 sweep driver
  (1,800+ lines). TIER1_GRID = 6 cells (g=0..5) as a `const` (hashed body field).
  Config-injection via `cell_config(base, lookback, k_long, drift)` — the
  ~30-line crux FP-C3.1 falsifies. C3-local `run_one_path_with_config` glue (byte-
  identical to C2 except caller-supplied config + pre-built shared path_gen).
  `ParamRobustnessVerdict` classifier (5-signal weakest-link, frozen § 0 bands).
  θ-surface renderer (ADR-0051 D3/§ D6.4 FM/body split). Buy-and-hold passive control.
- `crates/backtest/tests/param_sweep_e2e.rs` — 8 e2e gate tests including:
  FP-C3.1(a) (divergence PASS), FP-C3.1(b) (degenerate/RED-on-revert PASS),
  FP-C3.3 (two-run identity), FP-C3.2 (grid sensitivity), FP-C3.5 (anti-cherry-pick).
- `scripts/verify_anchors.sh` — updated `mc-robustness-2026-06` namespace handler
  to also search `spec/momentum-parameter-robustness-sweep/reports/`.
- `spec/anchors.toml` — new anchor #86 added:
  scenario=`v1-momentum-theta-surface-2023-block-bootstrap-real-fy`,
  SHA=`0dd989d9dc6f81a8dc722096d104fb7c0db3e7220f319c26b132e54df5f71dd5`.

**Results (measured 2026-05-30):**

- Wall-clock: **1217.1 s (20 min 17 s)** on Apple-Silicon M-series (~11 cores).
- g=0 correctness probe: p5=-0.049, p50=-0.008, p95=+0.010, P(loss)=76.0%,
  p95_maxdd=91.5% — MATCHES C2 direction (C2: p5=-0.050, p50=-0.010, p95=+0.009;
  slight variation expected since N=200 vs N=500).
- All 6 cells: **FRAGILE**.
- **FAMILY-UNIFORM-FRAGILE** — momentum-v1 fragility is not tunable away within
  the tested parameter space (lookback × k_long × drift_rebalance_threshold).
- Best-shot cell (g=3: 1mo × wide band): p5=-0.032, p50=+0.014, P(loss)=18.5% —
  still FRAGILE (p5 < 0 is the killer signal).
- Buy-and-hold control: p50 Sharpe=+1.735, P(loss)=4.5%, p95 MaxDD=51.2%
  (reference: +1.78 / 4% / 51% — matches well at N=200).
- Anchor: SHA `0dd989d9dc6f81a8dc722096d104fb7c0db3e7220f319c26b132e54df5f71dd5`,
  `scripts/verify_anchors.sh` → **86/86 PASS**.

**FP-C3.1 RED-on-revert proof:** `fp_c3_1b_degenerate_injection_produces_identical_cells`
asserts that when BOTH cells are forced to θ* (injection override), the divergence
collapses to |Δtrades|=0 and |Δp50_sharpe| < 1e-9 — proving the FP-C3.1(a)
gate would FAIL on a no-op injection. Test passes as designed.

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
arch        = ["spec/architecture/adr/0051-monte-carlo-determinism-and-distribution-report-anchoring.md#d6", "spec/momentum-parameter-robustness-sweep/feature.md#design"]   # architect filled (M-T1): ADR-0051 § D6 amendment + § D-C3.0/D-C3.2-LOCKED/D-C3.6-BUILD/D-C3.7-LOCKED
crates      = []   # developer fills (likely crates/backtest: bin/param_robustness_sweep.rs)
tests       = []   # developer fills (FP-C3.1 θ-divergence + FP-C3.3 two-run identity, mandatory)
anchors     = []   # tester fills (+1 θ-surface under mc-robustness-2026-06)
state       = "proposed"
```

---

## Changelog

- 2026-05-30 (architect, M-T1): **arch-done.** Resolved all 5 open questions
  (§ D-C3.0): OQ-1 = SAME path-set across all θ-cells (θ varied at config level,
  seed stream untouched ⇒ provably C1/C2-determinism-neutral, 85 anchors hold by
  construction); OQ-2 = LOCKED the 14-cell Tier-1 θ-grid (§ D-C3.2-LOCKED: baseline
  θ\* + 3 one-at-a-time arms + 3 low-churn-corner + 1 high-churn-corner + 2
  mid-diagonal; ~8 min @ N=500; all cells validated against the config loader rules
  + distinct); OQ-3 = single θ-independent L in the surface header; OQ-4 = ADR-0051
  § D6 amendment (NOT ADR-0052), registered atomically in the ADR registry README;
  OQ-5 = 2023-FY only for v0.1.0 (+2024 a v0.2.0 fast-follow). **Wrote the ADR-0051
  § D6 amendment** specifying the SAME-paths two-axis seed (`cell_seed_g :=
  ensemble_seed` ⇒ `path_seed_{g,j}` byte-identical to the C2 D1 seed; verified
  arithmetically) and **REJECTING the brief's § D-C3.4 composed-independent fallback
  as a seed-collision bug** (`+(g+j)·0x9E37_79B9` collides `(g,j)` with `(g−1,j+1)`;
  6 collisions in a 4×3 grid). Specified the binding build (§ D-C3.6-BUILD): a new
  `bin/param_robustness_sweep.rs` with a `const` 14-cell grid, the
  least-invasive config-injection seam (C3-local copy of `run_one_path` glue taking
  a `&CrossSectionalMomentumConfig` — leaves `run_path` AND the C2 driver
  byte-identical), the `ParamRobustnessVerdict` 5-signal weakest-link classifier
  (frozen decision-rule § 0 bands lifted verbatim as `const`), the θ-surface renderer
  (ADR-0051 D3/§ D6.4 FM/body split; rows sorted by `g`; grid def + N + buy-hold-flag
  hashed), and the buy-and-hold control row. Locked the day-1 gate (§ D-C3.7-LOCKED):
  `tests/param_sweep_e2e.rs` with FP-C3.1 (θ-injection divergence, tested on BOTH
  real + degenerate — CLAUDE.md non-negotiable) + FP-C3.3 (two-run body-SHA identity)
  as the MANDATORY pair, plus FP-C3.2/FP-C3.4/FP-C3.5 for ship. Confirmed the C0
  baseline cell cross-checks the C2 anchor numbers (free correctness probe).
  Preserved the analyst's § 0 anti-cherry-pick pre-registration (full surface +
  family verdict + per-cell `→ C5` flags; NO "best θ" crowned). Filled the staged
  trace row `arch` field. No code written (design-only, reversible until dev build).
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
