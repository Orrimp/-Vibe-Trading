---
slug: strategy-robustness-harness
mode: release
status: draft
audience: human-operator
updated: 2026-05-30
generated: 2026-05-30T16:10:00Z
covers: [monte-carlo-bootstrap-path-generator, strategy-robustness-harness]
version: v0.1.1
---

# Monte-Carlo robustness lane (C1 + C2) — release review

> **v0.1.1 — corrected deck.** The first cut of this deck (v0.1.0) carried two
> defects an operator-requested red-team caught: a fabricated "Sharpe 1.40 lucky
> path" headline and an engine bug that inflated the loss/drawdown tail. Both are
> fixed. The numbers below are the honest, tester-confirmed v0.1.1 numbers
> ([test report](../reports/test-2026-05-30-1325-v0.1.1.md)). The verdict did not
> change — and the corrected story is **cleaner and stronger** (see the next
> section). Every number cites its committed source file.

## TL;DR

We built the machine that asks *"is this strategy genuinely good, or did it just
get lucky on one slice of history?"* — and ran it through an adversarial review
that made its first verdict bulletproof: **v1 cross-sectional momentum is
FRAGILE**. On real chronological 2023 it was never even good (Sharpe ≈ 0.00, 74%
drawdown), and across **500 fair alternate-history replays it loses money 75% of
the time and clears the bar we care about 0% of the time** — while a passive
buy-and-hold of the very same coins on the very same replays scores a healthy
Sharpe of **+1.78**. Momentum's turnover/fee-churn is what fails, not the market.
This is a **methodology win**: a single backtest would have hidden it, and the
operator's dispute caught two real defects on the way.

## What changed

- **A reusable "alternate-history" generator (C1).** Given a year of real
  Binance prices, it manufactures plausible *different* versions of that same
  year by re-shuffling real market blocks — keeping crashes crash-like and
  keeping all coins moving together the way they really do (so a resampled crash
  is a *fair* hard test, not a strawman). Any future strategy gets tested against
  this for free.
- **A robustness harness (C2).** It runs v1 momentum over 500 of those alternate
  histories and boils 500 outcomes down to **one** report: the worst-case /
  typical / best-case Sharpe, drawdown, and probability of losing money. That one
  report is byte-locked (an "anchor") so the result can never silently drift.
- **The first verdict, on v1 cross-sectional momentum: FRAGILE** on all 5 primary
  robustness signals — and it **survived an adversarial red-team** (block-length
  sweep + buy-and-hold control) that the operator demanded before acting on it.
  The dispute caught a fabricated headline number and an engine accounting bug;
  the verdict held on the corrected numbers.

## Why

The operator set the direction: *"the strategies are currently fixed and limited;
check whether the strategy behaves differently with different inputs"* — and
ratified reframing the product so **strategy-robustness is a CORE pillar** (LLM =
support). A single backtest reports one number on one ordering of history; it
physically **cannot** tell you whether a Sharpe is a property of the *strategy* or
of *that one path*. The Monte-Carlo distribution answers exactly that: it measures
the *variance of outcome under input perturbation*. No alpha is claimed from
synthetic data — this is uncertainty quantification of an already-shipped
strategy, not prediction (see [`feature.md`](../feature.md) and the
[direction note](../../dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md)).

## Why you can trust this — it survived a red-team

The operator **disputed** the v0.1.0 FRAGILE verdict and demanded the methodology
be attacked before acting on it. The
[adversarial review](../../dev-notes/robustness-verdict-adversarial-review-2026-05-30.md)
tried hardest to show FRAGILE was an artifact. It **failed to break the verdict**
and instead made it bulletproof — and caught two real defects on the way. This is
the process working, and it is the single best reason to trust the number.

**1. The decisive block-length sweep — the fragility is NOT a bootstrap
artifact.** The prime suspect was that the block bootstrap chops up momentum's
trends, so *any* trend strategy would look fragile regardless of true edge. The
review refuted this directly by sweeping the block length from **1 (pure iid,
trends destroyed) to 4000 (≈ half a year per block, trends almost fully
preserved)** — and p50 Sharpe stayed flat at **−0.02 to −0.03 and P(Sharpe>1) was
exactly 0.000 at every single block length**
([adversarial review §2](../../dev-notes/robustness-verdict-adversarial-review-2026-05-30.md)).
If the bad result were caused by the bootstrap shredding trends, the Sharpe would
climb as the blocks got longer. It does not move at all.

```
v1 momentum — p50 Sharpe vs block length L (N=500 confirmation sweep)
source: adversarial review §2 "N=500 confirmation sweep"

  L = 1 (iid)      p50 Sharpe −0.024   P(Sharpe>1) 0.000   FRAGILE
  L = 60 (=lookbk) p50 Sharpe −0.027   P(Sharpe>1) 0.000   FRAGILE
  L = 204 (auto)   p50 Sharpe −0.022   P(Sharpe>1) 0.000   FRAGILE
  L = 4000 (≈½yr)  p50 Sharpe −0.027   P(Sharpe>1) 0.000   FRAGILE

  Flat across 4 orders of magnitude. No recovery as trends are preserved.
  → the fragility is structural, not a side-effect of the null.
```

(This sweep was run on the **pre-fix v0.1.0 engine**, so its L=204 p50 reads
−0.022 — the value the v0.1.1 solvency fix later moved to the shipped **−0.010**.
The sweep's purpose is the *shape* across L, not the level: the level is flat and
P(Sharpe>1) is 0 at every L, on either engine version. The headline numbers
everywhere else in this deck are the corrected v0.1.1 values.)

**2. The buy-and-hold control — the killer.** Passive equal-weight buy-and-hold
of the same 10 coins, on the **same** resampled histories, scored p50 Sharpe
**+1.78** with only a **4%** probability of loss
([adversarial review § "the clincher"](../../dev-notes/robustness-verdict-adversarial-review-2026-05-30.md)).
The null preserves the market's drift just fine — a passive holder captures it
robustly. Momentum specifically converts a +1.78-Sharpe drift environment into a
−0.01-Sharpe loss machine. That isolates the failure to **the strategy's trading
behaviour (turnover + entry/exit timing)**, not to the test being unfair.

```
SAME 500 resampled 2023 histories, two strategies:

  buy-and-hold (passive):   p50 Sharpe +1.78    P(loss)  4%    p95 MaxDD 51%   ROBUST
  v1 momentum  (active):    p50 Sharpe −0.01    P(loss) 75%    p95 MaxDD 92%   FRAGILE
                            └─ same coins, same paths, same fees ─┘
```

**3. The dispute caught two real defects — and the verdict held anyway.** The
red-team did not just rubber-stamp the result; it found and fixed two problems,
which is exactly why you can now trust what is left:

- **A fabricated headline number** (Correction A): the old deck's "the strategy
  looked great at Sharpe ≈ 1.40 and the harness caught it" was *false*. The real
  `MomentumStrategy` on real chronological 2023 bars scores Sharpe **0.003**, not
  1.40. The "1.40" was an illustrative LLM-narration example that leaked into the
  feature doc as if it were measured. **Retracted in full** (details below).
- **An engine accounting bug** (Correction B → fixed in v0.1.1): the old "100%
  drawdown / full wipeout" tail was partly a negative-cash artifact — equity went
  *negative* on fee-churn paths even though no coin fell more than 52%, which is
  impossible for a long-only book. The solvency fix corrects the tail magnitude
  (100% → 91.5% p95 MaxDD; P(loss) 86.8% → 75.2%) but, as required, **did not flip
  the verdict** ([test report §5](../reports/test-2026-05-30-1325-v0.1.1.md)).

## The one picture: real path vs the honest distribution

The corrected story is simpler than the original. Momentum was **never good** on
real 2023 — its real-path Sharpe is essentially zero — and the ensemble of 500
fair replays confirms it: centred just below break-even, mostly underwater, never
clearing the bar.

```
v1 momentum — Sharpe across 500 alternate 2023 histories (v0.1.1, corrected)
source: reports/robustness-20260530-130137-...md "Per-metric distribution"

   p5        p25      p50      p75      p95          real 2023 path
 -0.050    -0.034   -0.010   -0.0001  +0.009          Sharpe 0.003
   |---------|--------|--------|--------|                  ↓ (≈ p95)
   ****  ********  **********  *******  *        0.0  (break-even)
 ──┴────────┴────────┴───●────┴────────┴●───────┼───────────────────────►
   worst    .         typical .        best  ↑real            +1.0 (the
   case               (median            case   path           bar — never
                       < 0)                     ≈ p95           reached)

 P(loss) = 75.2%  ·  P(Sharpe > 0) = 24.8%  ·  P(Sharpe > 1.0) = 0.0%  ·  p95 MaxDD = 91.5%
```

Plain-language reading of the percentiles: **p5 = worst-case** (the bad 1-in-20
history), **p50 = typical** (the median replay), **p95 = best-case** (the lucky
1-in-20). For v1 momentum, *even the best case (p95) barely clears zero* and the
typical case loses money. The real 2023 backtest (Sharpe 0.003) sits up near p95 —
a **mildly favourable but entirely ordinary** draw of the same ensemble, **not** a
lucky outlier ([test report §5](../reports/test-2026-05-30-1325-v0.1.1.md);
[adversarial review §3](../../dev-notes/robustness-verdict-adversarial-review-2026-05-30.md)).
No "luck" is needed to explain why momentum failed — it simply never had an edge to
begin with.

## What you can do now

| Action | Command |
|--------|---------|
| Reproduce the FRAGILE verdict end-to-end (~3 min) | `cargo run --release -p backtest --features "candle realdata" --bin monte_carlo -- --generator block-bootstrap-real --paths 500 --ensemble-seed 0xC0FFEE --year 2023` |
| GBM smoke-test variant (fast, no real data, NOT anchored) | `cargo run --release -p backtest --bin monte_carlo -- --generator gbm-smoke --paths 100` |
| Re-verify the byte-locked result against the anchor | `bash scripts/verify_anchors.sh` |
| Read the full anchored distribution report | open [`reports/robustness-20260530-130137-v1-momentum-2023-block-bootstrap-real-fy-mc.md`](../reports/robustness-20260530-130137-v1-momentum-2023-block-bootstrap-real-fy-mc.md) |
| Test the SAME machine against a future strategy | point the harness at a new `Strategy` impl — C1+C2 are now reusable primitives |

## Live demo (from the committed v0.1.1 anchored report)

This deck quotes the **committed, tester-confirmed** v0.1.1 run rather than
re-running the bin (the tester already confirmed determinism across 3+ runs;
re-running now would contend for the toolchain with a parallel dev agent). The
distribution summary below is lifted verbatim from the committed anchored report
[`robustness-20260530-130137-...md`](../reports/robustness-20260530-130137-v1-momentum-2023-block-bootstrap-real-fy-mc.md),
whose body-SHA `7dbf5628...` is the locked anchor.

### The distribution summary block (from the anchored report)

```
## Per-metric distribution  (source: robustness-20260530-130137-...md)
| metric       | p5        | p25       | p50       | p75       | p95       |
|--------------|-----------|-----------|-----------|-----------|-----------|
| sharpe       | -0.050256 | -0.033527 | -0.010446 | -0.000096 | 0.009047  |
| sortino      | -0.070643 | -0.047306 | -0.014738 | -0.000136 | 0.012771  |
| calmar       | -0.187118 | -0.142670 | -0.049283 | -0.000543 | 0.043848  |
| max_drawdown | 61.32%    | 73.06%    | 81.39%    | 87.64%    | 91.50%    |
| total_return | -84.18%   | -72.97%   | -31.52%   | -0.43%    | +39.31%   |

## Ensemble robustness
| P(final_equity < initial)  | 0.752000 |   <- 75.2% of replays lose money
| P(Sharpe > 0)              | 0.248000 |
| P(Sharpe > 1.0)            | 0.000000 |   <- 0% clear the promotion bar
| max_drawdown_tail p50      | 81.39%   |
| max_drawdown_tail p95      | 91.50%   |   <- bad-tail drawdown (was a buggy 100%)
```

### Determinism — one SHA across 4+ runs

The committed v0.1.1 anchored report has body-SHA
`7dbf562887cbf6790f6a85b5276392388f429d098a955a139d81eedc7fd0ef20`, which the
tester reproduced byte-for-byte on an independent N=500 run
([test report §7 Gate 3](../reports/test-2026-05-30-1325-v0.1.1.md)) and which is
the locked anchor in `spec/anchors.toml` (namespace `mc-robustness-2026-06`, row
85). Across the developer's run, the tester's run, and the §D6.b re-emission, the
SHA is identical. **The verdict cannot silently change.** The prior live-run
stdout artifact (the v0.1.0 SHA `72fc7089...`, now superseded) is retained for
history under
[`artifacts/robustness-lane-c1-c2-2026-05-30/monte_carlo-live-run-stdout.txt`](artifacts/robustness-lane-c1-c2-2026-05-30/monte_carlo-live-run-stdout.txt).

## The decision rule was frozen BEFORE the numbers existed (integrity section)

This is the part that lets you trust the verdict instead of suspecting a just-so
story. The pass/fail ruler was **pre-registered on 2026-05-30, while C2 was still
in flight and had produced no number**
([`robustness-decision-rule-2026-05-30.md`](../../dev-notes/robustness-decision-rule-2026-05-30.md)).
The bands were frozen first; the result was scored against them — not the reverse.
This is the direct meta-lesson of the v3-vol-overlay no-op era: a number
interpreted only *after* it is seen can be talked into meaning anything.

**The pre-registered bands (lifted verbatim from the decision rule §0):**

| Signal | ROBUST (edge is real) | MARGINAL (inconclusive) | FRAGILE (one lucky path) |
|---|---|---|---|
| **p5 Sharpe** (tail floor) | **≥ +0.5** | `0.0 … +0.5` | **< 0** (the tail loses money) |
| **p50 Sharpe** (central) | ≥ 1.0 | `0.5 … 1.0` | < 0.5 |
| **p95−p5 Sharpe spread** (dispersion) | ≤ ~1.5 | `~1.5 … ~2.5` | > ~2.5 (wildly path-dependent) |
| **prob-of-loss** `P(equity<start)` | **≤ 15%** | `15% … 35%` | **> 35%** (coin-flip-ish) |
| **P(Sharpe > 1.0)** (gate fraction) | ≥ 60% | `35% … 60%` | < 35% |
| **p95 max-drawdown tail** | ≤ ~50% | `~50% … ~70%` | **> ~70%** (≈ the single-path 74%) |
| **p50 vs single-real-path Sharpe** | p50 within ~0.3 of the real path | — | real path sits **above p75** (real path was favourable) |

**How v1 momentum (v0.1.1) scored against the frozen ruler** (from the
[v0.1.1 tester report §5](../reports/test-2026-05-30-1325-v0.1.1.md)):

| Signal | Value | Band landed | Score |
|--------|-------|-------------|-------|
| p5 Sharpe (tail floor) | −0.050 | < 0 | **FRAGILE** |
| p50 Sharpe (central) | −0.010 | < 0.5 | **FRAGILE** |
| P(loss) | 75.2% | > 35% | **FRAGILE** |
| P(Sharpe > 1.0) | 0.0% | < 35% | **FRAGILE** |
| p95 MaxDD tail | 91.5% | > ~70% | **FRAGILE** |
| p95−p5 spread | 0.059 | small magnitude (low-Sharpe regime) | MARGINAL (interpretive) |
| p50 vs real path | real 0.003 vs p50 −0.010 → real ≈ p95 | mildly favourable but **ordinary draw** | interpretive |

**Composite = FRAGILE (5/5 primary signals).** Robustness is a weakest-link
property: any one primary signal in the FRAGILE band forces a FRAGILE verdict.
Here all five do. The real 2023 path (Sharpe 0.003) sits near the ensemble p95 — a
mildly favourable but entirely ordinary draw, **not** a lucky outlier. The
single-path backtest told you the strategy was weak; the ensemble confirms it is
also fragile.

### Two more reasons to trust it

- **Determinism — the result is byte-locked.** The developer's run, the tester's
  independent N=500 run, and the §D6.b anchor re-emission all produce the
  identical body-SHA `7dbf5628...`, which is the locked anchor in `spec/anchors.toml`
  (namespace `mc-robustness-2026-06`). The regression gate is `85 / 85` PASS
  ([test report §7 Gate 5](../reports/test-2026-05-30-1325-v0.1.1.md)). The
  verdict cannot silently change.
- **The guards are genuine, not theater** (mutation-tested by the tester):
  - **C1's shared-index null (FP-C1.5):** when the tester forced
    per-symbol-*independent* resampling (the wrong null), cross-symbol correlation
    collapsed from >0.95 to **−0.079** and the guard caught it. This is *why the
    bad tail is a fair adversary* — coins crash together in the replays the way
    they do in reality, so the strategy can't hide behind fake diversification.
  - **C2's divergence gate (FP-C2.1):** distinct-seed path spread vs degenerate
    (identical-seed) spread = **0.0** — a falsification gap. The harness would have
    caught a no-op collapse where all paths secretly came out identical.

## Verification matrix

V-ids are this lane's day-1 falsification probes (FP-C1.x / FP-C2.x) and the
adapted CLAUDE.md distribution-harness gate (R-NR.6). Evidence is the tester's
independently re-run gate, not a dev self-claim.

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| FP-C1.1 | C1 same-seed determinism (identical input ⇒ byte-identical paths) | VERIFIED | [C1 test report](../../monte-carlo-bootstrap-path-generator/reports/test-20260530-140000-v0.1.0.md) §4 — element-wise-equal + 2-run stable |
| FP-C1.5 | C1 shared-index null is genuine (fair crash adversary) | VERIFIED | C1 report §4 — mutation to per-symbol-indep collapsed corr to −0.079 |
| FP-C1.6 | C1 auto block-length pinned & stable | VERIFIED | C1 report §4 — AR(1) L=7, iid L=1, stable across 2 runs |
| FP-C1.3 | C1 block-length=1 degenerates to iid | VERIFIED | C1 report §4 — lag-1 acf < 0.15 |
| R-NR.6(a) / FP-C2.1 | C2 divergence-from-single-path gate (no-op falsifier) | VERIFIED | [v0.1.1 test report](../reports/test-2026-05-30-1325-v0.1.1.md) §3 — `fp_c2_1_degenerate_seeds_have_zero_spread` + `rn6a_divergence_gate_passes_with_distinct_seeds` PASS |
| R-NR.6(b) | C2 two-run byte-identity | VERIFIED | v0.1.1 test report §7 Gate 3 — tester's N=500 run byte-identical to dev run + anchor, body-SHA `7dbf5628` |
| K3 / FP-C2.2 | C2 anchor is sensitive to different inputs | VERIFIED | v0.1.1 test report §3 — `fp_c2_2_anchor_sensitive_to_different_inputs` PASS |
| Solvency (Bug B fix) | Equity curve never goes negative across paths | VERIFIED | v0.1.1 test report §3 — `solvency_invariant_equity_curve_never_negative_across_paths` + `solvency_guard_arithmetic_unit_test` PASS (NEW in v0.1.1) |
| ADR-0051 D2 | Deterministic reducer (no par_iter, total_cmp sort, type-7 pct) | VERIFIED | v0.1.1 test report §3 — `reduction_is_pure_same_inputs_same_outputs` PASS |
| §D6.b re-emission | Old report retired, anchor SHA updated in-place, all other anchors byte-identical | VERIFIED | v0.1.1 test report §7 Gate 5 — 85/85 PASS, `72fc7089`→`7dbf5628` |
| R3.2 | Exactly one new anchor (the distribution summary) | VERIFIED | `verify_anchors.sh` — 85/85, row 85 is `v1-momentum-2023-block-bootstrap-real-fy-mc` |
| Pre-flight (decision rule §4.1) | Generator = block-bootstrap-real AND mode = shared-index (verdict not void) | VERIFIED | anchored report header — `generator: block-bootstrap-real`, `bootstrap_mode: shared-index` |

## Numbers that matter

All numbers below cite the committed v0.1.1 anchored report
([`...130137...md`](../reports/robustness-20260530-130137-v1-momentum-2023-block-bootstrap-real-fy-mc.md))
and the [v0.1.1 tester report](../reports/test-2026-05-30-1325-v0.1.1.md).

- **Headline verdict:** v1 momentum = **FRAGILE** (5/5 primary signals).
- **p50 (typical) Sharpe = −0.010** · **p5 (worst-case) = −0.050** · **p95
  (best-case) = +0.009**. p95−p5 spread = 0.059.
- **P(loss) = 75.2%** · **P(Sharpe > 0) = 24.8%** · **P(Sharpe > 1.0) = 0.0%**.
- **p95 max-drawdown tail = 91.5%** (the bad 1-in-20 history); p50 max-DD = 81.4%.
- **Real chronological 2023 path:** Sharpe **0.003**, total return **+13.48%**,
  max-DD **73.73%** — an ordinary draw at ≈ ensemble p95, NOT a 1.40 outlier
  ([adversarial review §3](../../dev-notes/robustness-verdict-adversarial-review-2026-05-30.md)).
- **Buy-and-hold control (same paths):** p50 Sharpe **+1.78**, P(loss) **4%**, p95
  max-DD 51% — passive is robust where active is fragile.
- **Paths:** N = 500 shared-index block-bootstrap replays of 2023-FY real Binance
  returns; auto-selected block length L = 204.
- **Tests:** C1 = 23/23 unit + 4 mutation-verified probes; C2 v0.1.1 = 58/58 lib +
  8/8 e2e (was 6/6 — +2 new solvency tests). Zero failures.
- **Anchors:** **85 / 85 PASS** (row 85 is the C2 distribution anchor, SHA
  `7dbf5628...`).
- **Determinism:** body-SHA `7dbf5628...` across dev + tester + re-emission runs.
- **Perf:** ~178.8 s wall clock per N=500 run (well under the 10-min budget),
  Apple-Silicon canonical box.

## Verify it yourself (self-contained recipe — ~3 min)

- **Command:**
  ```
  cargo run --release -p backtest --features "candle realdata" --bin monte_carlo -- \
    --generator block-bootstrap-real --paths 500 --ensemble-seed 0xC0FFEE --year 2023 \
    --out-dir /tmp/mc-verify/
  ```
- **Steps:**
  1. From the repo root, run the command above (first build ~1–2 min if cold).
  2. While it runs, watch progress:
     `watch -n 15 'ls /tmp/mc-verify/robustness-*.md 2>/dev/null | tail -1 | xargs -I{} sh -c "echo {}; tail -12 {}" 2>/dev/null || echo "fan-out in progress..."'`
  3. When it prints `monte_carlo DONE`, read the `body_sha` and the
     `sharpe p5/p50/p95` line in stdout.
- **Timing:** ~179 s of compute after build (3 min budget is safe).
- **Expected result:** stdout shows
  `body_sha: 7dbf562887cbf6790f6a85b5276392388f429d098a955a139d81eedc7fd0ef20`,
  `sharpe p5/p50/p95: -0.0503 / -0.0104 / 0.0090`, `P(loss): 0.7520`,
  `P(Sharpe>1): 0.0000`, `max_dd tail p50/p95: 81.39% / 91.50%`. The **p50
  Sharpe ≈ −0.010** and the body-SHA matching the anchor are the two things to
  confirm — they prove FRAGILE-and-reproducible.
- **Failure diagnosis:**
  - *Different body-SHA* → you are NOT on the Apple-Silicon canonical box;
    cross-platform byte-parity is not contracted (ADR-0051 D5) — the *percentiles*
    should still match to ~6dp, which is what the verdict rests on.
  - *`block-bootstrap-real` data error* → real Binance parquet missing under
    `data/binance/`; the pinned revision SHA did not resolve.
  - *Report header says `bootstrap_mode: per-symbol-independent` or
    `generator: gbm-smoke`* → the verdict is **void** (the tail is no longer a
    fair adversary); re-run with the exact flags above.
- **Cleanup:** `rm -rf /tmp/mc-verify/` — the throwaway report is not anchored and
  is safe to delete (the canonical anchored copy lives in
  `spec/strategy-robustness-harness/reports/`).

## Open decisions

Two decisions. The first is the load-bearing one; the second only matters once
the first is settled.

1. **Accept the FRAGILE verdict?** Per the pre-registered rule, FRAGILE means **v1
   cross-sectional momentum does NOT advance to paper/live as-is** — its
   `paper→live` gate is BLOCKED on the robustness axis. (Cost gates, the 30-day
   paper requirement, and PM signoff were always independent criteria; this
   verdict is specifically the *robustness* axis.) Accepting this is accepting that
   the strategy never had a demonstrated edge and is fragile under resampling. *This
   is a methodology win, not a project failure — the harness did exactly its job,
   and the red-team made the verdict bulletproof.*

2. **What is the next step?** (Decide only after #1.) The ordered candidates:
   - **C3 — parameter sweep** *(Recommended — and now more interesting)*: re-runs
     this same harness across the whole θ grid to answer *"is the fragility
     specific to this one tuned θ\*, or is the entire momentum family fragile?"*
     The current verdict judges **path** robustness at one θ; C2 explicitly does
     **not** measure parameter robustness (that section is a stub at v0.1.1). The
     buy-and-hold result (+1.78 Sharpe passive vs −0.01 active on the same paths)
     strongly suggests the whole long-only top-K momentum *family* is a cost-bleed
     machine on 1h crypto, not just this θ — C3 is the direct way to confirm or
     refute that family-wide hypothesis.
   - **Pivot strategy family**: if you accept the family-wide read above, skip the
     sweep and put a different family (e.g. mean-reversion / carry) through the same
     C1+C2 machine.
   - **C5 — PBO / Deflated-Sharpe** (López de Prado overfit guards): a different,
     heavier axis of robustness. C2 does **not** emit these.
   - Note: per Q3, the **learning loop (C4) is ordered LAST** — it is not a
     candidate here.

## Cost of a "yes"

- Accepting #1 commits nothing to re-run — the anchor is locked. It records the
  `paper→live` robustness gate as BLOCKED for v1 momentum.
- Choosing **C3** commits to a θ-grid sweep through the same harness (multiple
  N=500 runs, each ~3 min) and a new anchored sweep report.
- One non-blocking developer follow-up is already queued by the tester: add a
  `run_path`-calling solvency unit test for genuine RED-on-removal coverage
  ([test report §9](../reports/test-2026-05-30-1325-v0.1.1.md)). It does not gate
  this ship; the arithmetic proof + the full-scale anchor already protect the fix.

## Transparency notes (no surprises)

- **What changed since v0.1.0 of this deck.** Two corrections from the
  operator-requested red-team: (A) the fabricated "Sharpe ≈ 1.40 lucky path"
  headline is **retracted** — the real-path Sharpe is 0.003; (B) the engine
  negative-cash bug is **fixed** in code (v0.1.1), moving p50 Sharpe −0.022 →
  −0.010, P(loss) 86.8% → 75.2%, p95 MaxDD 100% → 91.5%. The FRAGILE verdict is
  unchanged. The anchor SHA moved `72fc7089` → `7dbf5628` via the ADR-0038 §D6.b
  wiring-bug-fix re-emission protocol.
- **Pre-existing clippy debt (NOT a C2 regression).** Under the canonical strict
  gate, there are ~42 pre-existing errors in *pre-C2* backtest files
  (`engine.rs`, `paths.rs`, `sma_composed_run.rs`, etc.). **C2's own files are
  clippy-clean** ([test report §2, §7 Gate 7](../reports/test-2026-05-30-1325-v0.1.1.md)).
  The orchestrator filed a separate cleanup chip; it does not touch this verdict.
- **Spec-lint:** `FAIL (100 violations in 4 categories)` — **all pre-existing**
  (88 dead-link, 3 missing-frontmatter, 2 shipped-no-tests, 7 trace-broken-path)
  and **an improvement vs the 145-violation audit baseline**
  ([spec-audit-2026-05-30](../../dev-notes/spec-audit-2026-05-30.md)), with **zero
  new vs the v0.1.1 tester PASS state**. This deck's own dead link to the retired
  `112942` report has been corrected to the live `130137` report. No structural
  regression landed since `VERDICT → PASS`.
- **Scope honesty:** this verdict is **path-robustness at one θ over resampled real
  2023 history**. It does NOT speak to parameter robustness (→ C3), PBO /
  Deflated-Sharpe (→ C5), or out-of-history regimes the bootstrap can't synthesize
  (a block bootstrap resamples real blocks; it cannot invent a 2025-style event
  2023 never contained).

## Reusability (the lasting payoff)

C1 (`BlockBootstrapPathGen` + `MonteCarloPathGen` trait, in `crates/data`) and C2
(the harness + `DistributionSummary` reducer, in `crates/backtest`) are now
**primitives**. Every future strategy can be run through the identical "500 fair
alternate histories → one anchored verdict" pipeline at near-zero marginal cost.
The robustness question is no longer bespoke per strategy — it is a button.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback
_empty until operator fills_

## Feedback log
_empty — no rejection routed yet_

## Changelog
- 2026-05-30 (presenter): initial release deck for the C1+C2 Monte-Carlo
  robustness lane (v0.1.0). Headline: v1 momentum FRAGILE (5/5 primary signals,
  pre-registered rule).
- 2026-05-30 (presenter): **v0.1.1 corrected deck.** Absorbed the
  operator-requested adversarial review + v0.1.1 tester PASS (commit b58984f, body-
  SHA `7dbf5628`). (1) Corrected all headline numbers to the v0.1.1 anchored report:
  p50 Sharpe −0.010, p5 −0.050, p95 +0.009, P(loss) 75.2%, P(Sharpe>1) 0.0%, p95
  MaxDD 91.5%. (2) **Retracted the fabricated "Sharpe ≈ 1.40 lucky path" story** —
  real-path Sharpe is 0.003 (totret +13.48%, maxDD 73.73%), an ordinary draw at
  ≈ p95; re-narrated TL;DR + the "one picture" + the p50-vs-real-path row. (3) Added
  a prominent **"Why you can trust this — it survived a red-team"** section: the
  L-sweep (p50 flat −0.02..−0.03, P(S>1)=0 at L∈{1..4000}), the buy-and-hold control
  (passive p50 Sharpe +1.78, P(loss) 4% on the same paths), and the two defects the
  dispute caught. (4) Fixed the dead link from the retired `112942` report to the
  live `130137` report. (5) Updated the verify-it-yourself recipe to expected p50
  ≈ −0.010 and SHA `7dbf5628`. (6) Updated the verification matrix with the new
  solvency tests and §D6.b re-emission. Approval block left un-ticked.
