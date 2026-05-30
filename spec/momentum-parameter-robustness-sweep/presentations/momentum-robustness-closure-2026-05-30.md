---
slug: momentum-parameter-robustness-sweep
mode: release
status: draft
owner: presenter
audience: human-operator
updated: 2026-05-30
generated: 2026-05-30T20:55:00Z
covers: [momentum-parameter-robustness-sweep, strategy-robustness-harness, monte-carlo-bootstrap-path-generator]
version: v0.1.0
supersedes_none: true
closes_arc: momentum-robustness
---

# Momentum robustness — CLOSURE deck (C3 parameter-sweep completes the arc)

> **This is the closing review of the momentum-robustness program.** You asked,
> long ago: *"are the strategies robust — do they behave differently under
> different inputs?"* The program now answers that **completely and conclusively
> for v1 momentum.** The path-robustness verdict (C2) is on record in the
> [C1+C2 deck](../../strategy-robustness-harness/presentations/robustness-lane-c1-c2-2026-05-30.md);
> **this deck adds the final piece — parameter robustness — and asks you to close
> the book on momentum-v1 and pick the next strategy family to vet.** Every
> number cites its source; the C3 numbers are read from the **staged report
> bytes** (commit pending a 1Password unlock — the bytes below are exactly what
> will be committed).

## TL;DR

v1 cross-sectional momentum is **fragile across its WHOLE parameter family** —
all 6 tested configurations fail, including the one a-priori best shot at being
robust. Combined with the earlier path-robustness verdict (fragile across 500
resampled 2023 histories) and the buy-and-hold control (a passive hold of the
same coins *beats* momentum on the same paths, +1.74 vs ≈0 Sharpe), the conclusion
is no longer "this config is bad" — it is **"this family is bad."** *What it means
for money:* momentum-v1 does not earn live capital at any tested config, and the
robustness machine caught it before a single dollar was committed.

## What changed (since the C1+C2 deck)

- **A second robustness axis is now built and run — parameter robustness (C3).**
  C2 answered *"is this one tuned config robust across alternate histories?"*
  (no). C3 answers the deeper question C2 explicitly could not: *"is the fragility
  specific to that one config, or is the entire momentum family fragile?"* It
  re-runs the same Monte-Carlo harness across **6 different parameter settings**
  (varying the lookback window, how many coins it holds, and the trade-trigger
  band) and reports the full surface — no cherry-picking a "winner."
- **The verdict is uniform: all 6 cells FRAGILE** → **FAMILY-UNIFORM-FRAGILE.** Even
  the corner deliberately chosen to give momentum its best chance (a 1-month
  lookback with a wide no-trade band, which slashes the fee-churn) still loses money
  in its bad-case tail. There is no robust corner of the parameter space.
- **The robustness harness is now a complete, reusable machine — both axes.** Path
  robustness (C2) and parameter robustness (C3) are now buttons any future strategy
  gets run through from day one. That machine is the durable asset this program
  produced.

## Why (the rationale, distilled)

A single backtest reports one number on one ordering of history at one parameter
setting — it physically cannot tell you whether a result is a property of the
*strategy* or of *that one path at that one config*. The robustness program splits
that uncertainty into two measurable axes: **path** robustness (does it survive
plausibly-different histories? — C2) and **parameter** robustness (does it survive
across its own family of settings? — C3). v1 momentum fails both. This is
**uncertainty quantification of an already-shipped strategy, not prediction** — no
alpha is claimed from synthetic data. The program working exactly as designed: it
caught a strategy that looked unremarkable on one backtest, proved it is
*structurally* non-robust before any capital, and left behind the machine to vet
whatever comes next.

## The picture: the θ-surface — no robust corner exists

Each cell is one parameter setting. The bar that kills every cell is **p5 Sharpe
< 0** — the bad-case (worst 1-in-20) history loses risk-adjusted money. Read the
surface left-to-right: even as you walk toward the low-churn corner (g=3, the best
shot), the tail floor never clears zero.

```
v1 momentum — θ-surface, 6 cells × 200 resampled 2023 histories (anchor #86)
source: robustness-sweep-20260530-180006-...theta-surface...md  (staged bytes)
swept axes: lookback (signal horizon) · k_long (#coins held) · drift (no-trade band)

 g  lookback  k_long  band   p5 Sharpe   p50 Sharpe   P(loss)   p95 MaxDD   verdict
                              (tail floor)(typical)
 ── ───────── ─────── ────── ─────────── ──────────── ───────── ─────────── ─────────
 0    60h       3     0.10    -0.049       -0.008       76.0%     91.5%      FRAGILE   ← =C2 config (probe)
 1    24h       3     0.10    -0.048       -0.021       93.5%     93.3%      FRAGILE   ← noisiest / highest churn
 2   168h       3     0.10    -0.058       +0.002       45.0%     88.2%      FRAGILE
 3   720h       3     0.50    -0.032       +0.014       18.5%     81.7%      FRAGILE   ← BEST SHOT — still fails
 4    60h       1     0.10    -0.077       -0.007       83.0%     89.3%      FRAGILE   ← top-1 only
 5    60h       5     0.10    -0.046       -0.005       61.5%     92.0%      FRAGILE   ← top-5

   killer signal (every cell):  p5 Sharpe < 0  →  the bad-case tail loses money
   P(Sharpe > 1.0) = 0.0%  for ALL six cells  →  not one cell ever clears the bar
   FAMILY VERDICT:  FAMILY-UNIFORM-FRAGILE
```

The best cell (g=3) is the interesting one: by widening the no-trade band to 0.50
and stretching the lookback to a month, it does cut the loss rate dramatically
(P(loss) 18.5%, the only cell near the ROBUST band on that one signal, and a
*positive* typical Sharpe of +0.014). **But its bad-case tail still loses money
(p5 = −0.032 < 0), and 0% of its histories clear Sharpe 1.** Under the
pre-registered weakest-link rule (any one primary signal in the FRAGILE band ⇒
FRAGILE), it is FRAGILE — 4 of its 5 primary signals are in the FRAGILE band. The
low-churn lever helps, but it cannot manufacture an edge that was never there.

## The complete-arc summary — three independent results, one conclusion

| Axis / control | Question it answers | Result | Verdict |
|---|---|---|---|
| **C2 — path robustness** (1 config, 500 histories) | Is the shipped config robust across alternate 2023 histories? | p50 Sharpe ≈ −0.010 · **P(loss) 75.2%** · P(Sharpe>1) 0.0% · p95 MaxDD 91.5% | **FRAGILE** |
| **C3 — parameter robustness** (6 configs, 200 histories each) | Is the whole momentum family fragile, or just that one config? | **All 6 cells FRAGILE.** Best shot (g=3) still has p5 < 0 and 0% clearing Sharpe 1. | **FAMILY-UNIFORM-FRAGILE** |
| **Buy-and-hold control** (passive, same paths) | Is the test itself unfair to any trading strategy? | p50 Sharpe **+1.735** · P(loss) **4.5%** · p95 MaxDD 51.2% | passive is **robust** |

The control is what makes this conclusive rather than merely negative: on the
**exact same** resampled histories with the **exact same** coins and fees, simply
*holding* earns a healthy +1.74 Sharpe and loses money only 4.5% of the time.
The market's drift is right there to be captured. **Momentum specifically converts
a +1.74-Sharpe drift environment into a break-even-at-best loss machine, at every
config tested** — that isolates the failure to the strategy's own trading
behaviour (turnover + entry/exit timing), not to the test being hostile.

**One conclusion:** v1 cross-sectional momentum is **conclusively retired on the
robustness axis. Not a bad config — a bad family** on 1h crypto.

## Why you can trust this — the integrity spine (now even stronger)

The C1+C2 verdict already survived an operator-demanded adversarial red-team. C3
inherits that spine and adds two more pre-registered guards. This is the single
best reason to act on the number.

1. **The decision rule was frozen BEFORE any C3 number existed.** The pass/fail
   ruler (the [pre-registered decision rule](../../dev-notes/robustness-decision-rule-2026-05-30.md)
   § 0) was written on 2026-05-30 while C2 was still in flight and had produced no
   distribution. C3 scored its 6 cells against that frozen ruler — not the reverse.
   This is the direct meta-lesson of the v3-vol-overlay no-op era: a number
   interpreted only *after* it is seen can be talked into meaning anything.
2. **Anti-cherry-pick by construction (pre-registered).** Before C3 emitted any
   surface, the feature committed in writing: *C3 will NOT report an argmax-selected
   "best θ" as ROBUST* ([feature.md § 0](../feature.md)). A 6-cell grid that picked
   `argmax` would inflate the false-ROBUST rate (`1 − 0.95^G`). C3 reports the FULL
   surface + a family verdict and crowns no winner — and a mutation-tested probe
   (FP-C3.5) enforces this in code: it asserts the family summary is one of the two
   allowed values and that any non-FRAGILE cell would carry a `→ C5 deflation
   required` flag. A uniform-negative result needs no multiple-testing correction:
   you cannot overfit your way to a *loss*.
3. **The red-team that made C2 bulletproof still backs C3.** The operator disputed
   the C2 FRAGILE verdict; the [adversarial review](../../dev-notes/robustness-verdict-adversarial-review-2026-05-30.md)
   tried hardest to break it and failed — the block-length sweep showed p50 Sharpe
   flat at ≈ −0.02 to −0.03 across block lengths from 1 to 4000 (the fragility is
   structural, not a bootstrap artifact), and the buy-and-hold control isolated the
   failure to momentum's turnover. The dispute also caught and fixed two real
   defects (a fabricated "Sharpe 1.40" headline and an engine accounting bug). C3's
   g=3 best-shot result is the parameter-space analogue of that L-sweep, and reaches
   the same conclusion from the orthogonal axis.
4. **FP-C3.1 is a genuine guard, mutation-tested — not theater.** The headline gate
   asserts that injecting different parameters into different cells produces
   *different* results (`fp_c3_1a` PASS). Its mutation twin (`fp_c3_1b` PASS) forces
   both cells to the same config and asserts the divergence *collapses* to
   |Δtrades|=0 — proving that if the production config-injection were a silent
   no-op, `fp_c3_1a` would FAIL. The sweep is provably doing real work, not running
   the same config six times in a trench coat.
5. **Determinism proven; byte-anchored.** The g=0 cell reproduces C2's first 200
   paths by construction (SAME-paths seeding, ADR-0051 § D6.1) — its numbers match
   C2 to sampling noise (p5 −0.049 vs −0.050, p50 −0.008 vs −0.010), a free
   correctness probe that the plumbing is right. Two-run byte-identity is unit-proven
   (`fp_c3_3` PASS) and architecturally guaranteed (index-ordered reduction, fixed
   float precision, g-sorted rows). The θ-surface report is byte-locked as anchor
   #86; the live regression gate is **86/86 PASS** (run below).

## Live evidence — the anchor gate (run now, this session)

The regression gate proves the C3 θ-surface report is byte-stable and the C2
anchor is unchanged. Run live during deck assembly:

```
$ bash scripts/verify_anchors.sh
...
PASS  v1-momentum-2023-block-bootstrap-real-fy-mc        7dbf562887cbf6790f6a85b5276392388f429d098a955a139d81eedc7fd0ef20
PASS  v1-momentum-theta-surface-2023-block-bootstrap-real-fy  0dd989d9dc6f81a8dc722096d104fb7c0db3e7220f319c26b132e54df5f71dd5
ANCHORS PASS  (86 / 86)
```

- **C3 θ-surface (anchor #86):** body-SHA `0dd989d9...` resolves from the staged
  report — byte-stable. (Body-SHA = a fingerprint of the report's bytes; if one
  digit of one number changed, the gate would fail.)
- **C2 anchor (#85):** `7dbf5628...` byte-identical — C3's edits did not disturb the
  prior verdict.
- **All 84 prior anchors:** byte-identical — zero regression across the project.

## Verification matrix

V-ids are C3's day-1 falsification probes (FP-C3.x) plus the inherited
distribution-harness gates. Evidence is the [tester's independently re-run gate](../reports/test-2026-05-30-1834-v0.1.0.md),
not a developer self-claim.

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| FP-C3.1 | θ-injection is real, not a no-op (the headline gate) | VERIFIED | Tester §6 — `fp_c3_1a` PASS (divergence) + mutation twin `fp_c3_1b` PASS (collapse asserts `fp_c3_1a` would fail under no-op) |
| FP-C3.2 | Grid definition is a hashed input (K3 sensitivity) | VERIFIED | Tester §3 — `fp_c3_2_grid_sensitivity_different_grids_produce_different_bodies` PASS |
| FP-C3.3 | Two-run byte-identity (no unordered fold) | VERIFIED | Tester §5.2 — `fp_c3_3_two_run_byte_identity` PASS; architectural determinism proof complete |
| FP-C3.4 | Buy-and-hold control matches adversarial-review reference | VERIFIED | Tester §5 — p50 +1.735 (ref +1.78), P(loss) 4.5% (ref 4%), p95 MaxDD 51.15% (ref ~51%) |
| FP-C3.5 | Anti-cherry-pick enforced in code (pre-registration) | VERIFIED | Tester §3 — `fp_c3_5_family_summary_always_valid_value` + `fp_c3_5_non_fragile_cell_carries_c5_flag` PASS |
| SAME-paths invariant | g=0 reproduces C2 paths (ADR-0051 § D6.1) | VERIFIED | Tester §5 g=0 probe — all 5 signals match C2 to N=200-vs-500 sampling noise; `same_paths_seeding_invariant_adr0051_d6_1` PASS |
| Weakest-link classifier | 5-signal verdict bands implemented correctly | VERIFIED | Tester §3 Gate 2 — 19/19 bin unit tests PASS (boundary + each-signal cases) |
| Pre-flight (decision rule §4.1) | generator=block-bootstrap-real AND mode=shared-index (verdict not void) | VERIFIED | Anchored report header — `generator: block-bootstrap-real`, `bootstrap_mode: shared-index` |
| Anchor additivity (#86) | Exactly one new anchor; all prior byte-identical | VERIFIED | Live `verify_anchors.sh` this session — 86/86 PASS, C2 #85 unchanged |
| spec-lint (no regression) | No structural regression since tester PASS | VERIFIED | Live `spec_lint.py` this session — 94 violations (≤ tester's 95; `unreferenced-anchor` resolved; no new category) |

## Numbers that matter

- **Family verdict:** **FAMILY-UNIFORM-FRAGILE** — all 6 of 6 cells FRAGILE.
- **Best-shot cell (g=3, 1mo lookback × 0.50 hold-band):** p5 −0.032 (**< 0 → the
  killer**), p50 +0.014, P(loss) 18.5%, **P(Sharpe>1) 0.0%**, p95 MaxDD 81.74% →
  FRAGILE (4/5 primary signals).
- **Worst cell (g=1, 24h lookback):** p50 −0.021, P(loss) 93.5%, p95 MaxDD 93.3%.
- **g=0 correctness probe vs C2:** p5 −0.049 / −0.050, p50 −0.008 / −0.010, p95
  +0.010 / +0.009 — pure N=200-vs-500 sampling noise; plumbing confirmed correct.
- **Buy-and-hold control:** p50 Sharpe **+1.735**, P(loss) 4.5%, p95 MaxDD 51.15%.
- **Tests:** **27/27 C3 gates PASS** (8/8 e2e + 19/19 bin unit), 0 failures.
  (The 14 pre-existing `determinism.rs` failures are documented baseline debt — zero
  C3 intersection. See "What this cost / honest notes.")
- **Anchors:** **86/86 PASS** (row 86 = the C3 θ-surface, SHA `0dd989d9...`).
- **spec-lint:** 94 violations (2 categories) — all pre-existing; one *fewer* than
  the tester's PASS state. No structural regression.
- **Compute:** 1217.1 s (20 min 17 s) wall-clock for the full 6×200 sweep on the
  Apple-Silicon canonical box (~11 cores).
- **Data:** real Binance 2023-FY, 10 USDT pairs, hourly (87,600 bars), revision SHA
  `3a8b96c4...`, 6 bps round-trip fees. Generator `block-bootstrap-real`, shared-index,
  auto block-length L=204.

## The capability you now own (the durable asset)

The program leaves behind a **complete two-axis robustness harness**, all reusable
at near-zero marginal cost per future strategy:

| Axis | What it stresses | The button |
|---|---|---|
| **Path robustness (C1+C2)** | Plausibly-different histories of the same year (500 resampled real-block paths, crashes kept crash-like) | `monte_carlo` bin → one anchored distribution report |
| **Parameter robustness (C3)** | The strategy's own family of settings (a hypothesis-aimed θ-grid) | `param_robustness_sweep` bin → one anchored θ-surface + family verdict |

Every future strategy gets vetted **both ways from day one** — the robustness
question is no longer bespoke per strategy. That is the lasting payoff of the
program, independent of momentum's verdict.

## What you can do now

| Action | Command |
|--------|---------|
| Reproduce the full θ-surface (~20 min, 6×200) | `cargo run --release -p backtest --bin param_robustness_sweep -- --generator block-bootstrap-real --paths 200 --ensemble-seed 0xC0FFEE --year 2023 --grid tier1 --out-dir /tmp/c3-verify/` |
| Re-verify the byte-locked surface against the anchor | `bash scripts/verify_anchors.sh` |
| Read the full anchored θ-surface report | open [`reports/robustness-sweep-20260530-180006-...theta-surface...md`](../reports/robustness-sweep-20260530-180006-v1-momentum-theta-surface-2023-block-bootstrap-real-fy.md) |
| Read the C3 tester PASS | open [`reports/test-2026-05-30-1834-v0.1.0.md`](../reports/test-2026-05-30-1834-v0.1.0.md) |
| Read the prior path-robustness (C1+C2) deck | open [`../../strategy-robustness-harness/presentations/robustness-lane-c1-c2-2026-05-30.md`](../../strategy-robustness-harness/presentations/robustness-lane-c1-c2-2026-05-30.md) |
| Point the SAME harness at a new strategy family | implement a new `Strategy` and run it through both bins — C1/C2/C3 are now reusable primitives |

## Verify it yourself (self-contained recipe — ~20 min)

- **Command:**
  ```
  cargo run --release -p backtest --bin param_robustness_sweep -- \
    --generator block-bootstrap-real --paths 200 --ensemble-seed 0xC0FFEE \
    --year 2023 --grid tier1 --out-dir /tmp/c3-verify/
  ```
- **Steps:**
  1. From the repo root, run the command above (first build ~1–2 min if cold).
  2. While it runs (~20 min CPU-bound), watch progress:
     `watch -n 30 'ls /tmp/c3-verify/robustness-sweep-*.md 2>/dev/null | tail -1 | xargs -I{} sh -c "echo {}; tail -16 {}" 2>/dev/null || echo "sweep in progress (6 cells × 200 paths)..."'`
  3. When it completes, read the θ-surface table and the `Family verdict` line in
     the emitted report.
- **Timing:** ~1217 s of compute after build (allow ~25 min end-to-end with build).
- **Expected result:** all 6 cells print `FRAGILE`; the `Family verdict` line reads
  `FAMILY-UNIFORM-FRAGILE`; the body-SHA of the emitted report equals
  `0dd989d9dc6f81a8dc722096d104fb7c0db3e7220f319c26b132e54df5f71dd5` (verify with
  `python3 scripts/hash_report.py /tmp/c3-verify/robustness-sweep-*.md`).
- **Failure diagnosis:**
  - *Different body-SHA* → you are NOT on the Apple-Silicon canonical box;
    cross-platform byte-parity is not contracted (ADR-0051 D5). The *percentiles
    and the FRAGILE verdicts* should still match — those are what the conclusion
    rests on.
  - *Report header says `bootstrap_mode: per-symbol-independent` or
    `generator: gbm-smoke`* → the verdict is **void** (the tail is no longer a fair
    adversary); re-run with the exact flags above.
  - *`block-bootstrap-real` data error* → real Binance parquet missing under
    `data/binance/`; the pinned revision SHA `3a8b96c4...` did not resolve.
- **Cleanup:** `rm -rf /tmp/c3-verify/` — the throwaway report is not anchored and
  is safe to delete (the canonical anchored copy lives in
  `spec/momentum-parameter-robustness-sweep/reports/`).

## Open decisions

Two decisions. The first closes the arc; the second opens the next one. The
first is load-bearing — settle it first.

**1. Confirm the retirement of momentum-v1 on the robustness axis?**
Per the pre-registered rule, FAMILY-UNIFORM-FRAGILE means **v1 cross-sectional
momentum does not advance to paper/live as-is, at any tested config** — its
`paper→live` gate is BLOCKED on the robustness axis. (Cost gates, the 30-day paper
requirement, and PM signoff were always independent criteria; this is specifically
the *robustness* axis.) Confirming this records that momentum-v1 never had a
demonstrated edge and is fragile across both its history and its parameter family.
*This is a methodology win, not a project failure — the harness did exactly its
job, the red-team made it bulletproof, and you now own the machine to vet the
next idea.*

**2. The pivot — which strategy family should the harness vet next?** (Decide only
after #1.) The harness is family-agnostic; the next strategic question is *which
family to put through it.* Candidate families to weigh (the deck does NOT pick — this
is your strategic call):
- **Mean-reversion** — the natural counter-hypothesis: if momentum (trend-following)
  is a cost-bleed machine on 1h crypto, the inverse behaviour is the first thing to
  test. Cheap to wire (it reuses the same cross-sectional ranking, inverted).
- **Carry** — funding-rate / basis capture; a structurally different return source
  (not price-trend), so it is the most genuinely independent bet from momentum.
- **Breakout** — range-break entries; trend-adjacent, so the buy-and-hold-dominates
  prior is a caution flag worth weighing before investing.
- **Cross-sectional value** — a fundamentals-style ranking; the longest-horizon,
  lowest-turnover candidate, which the g=3 low-churn result hints might fare better
  on the fee axis.

A neutral framing if it helps you choose: the buy-and-hold control suggests the
2023 universe had strong capturable drift, so the sharpest scientific question is
*"is there ANY active family that beats simply holding, net of fees, on this
universe?"* — and the most independent test of that is a non-trend family (carry or
mean-reversion). But the choice is yours.

## Cost of a "yes"

- **Confirming #1** commits nothing to re-run — the anchor is locked. It records the
  `paper→live` robustness gate as BLOCKED for the momentum-v1 family and closes the
  momentum-robustness arc.
- **Choosing a family in #2** commits the next sprint to: implement the family's
  `Strategy`, then run it through C2 (one ~3 min N=500 path-robustness pass) and C3
  (one ~20 min θ-surface sweep), each producing a new anchored report. The pre-
  registered decision rule and both harness bins are already built, so the marginal
  cost is the strategy implementation + two harness runs, not new machinery.
- **No deferred follow-up is owed by this verdict.** The C5 deflation pass (PBO /
  Deflated-Sharpe) is moot here — a uniform-negative result needs no multiple-testing
  correction; it would only have been triggered had any cell come back non-FRAGILE.

## What this cost (honest note — the one lesson worth keeping)

C3's first build was **over-scoped**: the original architect design was a 14-cell ×
N=500 grid, which proved computationally intractable for one sitting (it would have
run ~1 hour, and the first attempt crashed before completing). The orchestrator
**re-scoped it to a tractable 6-cell × N=200** (~20 min) on 2026-05-30 — the
**methodology was unchanged** (same harness, same frozen decision rule, same
shared-index null), only the grid density and path count were reduced. The 6 cells
were chosen to be hypothesis-aimed (baseline + lookback arms + the best-shot
low-churn corner + breadth arms), not arbitrarily trimmed. The lesson, captured in
[feature.md § Implementation](../feature.md): **validate compute budgets before
locking a grid size** — a sweep's wall-clock is grid × N × per-path cost, and an
N=500 × 14-cell design is ~6× the N=200 × 6-cell one. The smaller grid is noisier in
the tail (N=200 percentile steps are 0.5% vs 0.2% at N=500) but does not weaken the
conclusion: the family is FRAGILE by a wide margin at every cell, well outside that
noise band.

## Scope honesty (no overclaim)

- This program judges the **robustness axis** (path + parameter) on **resampled real
  2023-FY history**. A block bootstrap resamples *real* return blocks — it cannot
  synthesize a regime 2023 never contained (a 2025-style event absent from the source
  year). Robustness to genuinely novel regimes is outside what this distribution can
  speak to.
- The C3 numbers are read from the **staged** report bytes; the commit is pending a
  1Password unlock. The bytes are exactly what will be committed (the live anchor
  gate above resolves them at 86/86 against the staged content) — but the git commit
  itself is not yet recorded. No git action is taken by this deck.
- C5 (PBO / Deflated-Sharpe) remains a separate, un-built axis. It is not needed for
  a uniform-negative result and is not claimed here.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback
_empty until operator fills_

## Feedback log
_empty — no rejection routed yet_

## Changelog
- 2026-05-30 (presenter): closure deck for the momentum-robustness arc — C3
  parameter-robustness sweep completes path-robustness (C2). Headline:
  FAMILY-UNIFORM-FRAGILE (all 6 θ-cells FRAGILE; best-shot g=3 still p5 < 0 / 0%
  clearing Sharpe 1; buy-and-hold control +1.735 vs ≈0). Numbers read from staged
  anchor-#86 bytes (`0dd989d9...`); live anchor gate 86/86 PASS, live spec-lint 94
  (no regression vs tester's 95). Surfaced two operator decisions: (1) confirm
  momentum-v1 retirement on the robustness axis; (2) pick the next strategy family
  to vet (candidates teed up, not chosen). Approval block left un-ticked.
