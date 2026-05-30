---
slug: strategy-robustness-harness
mode: release
status: draft
audience: human-operator
updated: 2026-05-30
generated: 2026-05-30T12:10:00Z
covers: [monte-carlo-bootstrap-path-generator, strategy-robustness-harness]
---

# Monte-Carlo robustness lane (C1 + C2) — release review

## TL;DR

We built the machine that asks *"is this strategy genuinely good, or did it
just get lucky on one slice of history?"* — and its first verdict is in: **v1
momentum is FRAGILE**. It looked fine on the one real 2023 path (Sharpe ≈ 1.4),
but across **500 fair alternate-history replays it loses money 87% of the
time** and clears the bar we care about **0% of the time**. This is a
**methodology win** — the harness caught false confidence a single backtest
would have hidden.

## What changed

- **A reusable "alternate-history" generator (C1).** Given a year of real
  Binance prices, it manufactures plausible *different* versions of that same
  year by re-shuffling real market blocks — keeping crashes crash-like and
  keeping all coins moving together the way they really do (so a resampled
  crash is a *fair* hard test, not a strawman). Any future strategy gets tested
  against this for free.
- **A robustness harness (C2).** It runs v1 momentum over 500 of those alternate
  histories and boils 500 outcomes down to **one** report: the worst-case /
  typical / best-case Sharpe, drawdown, and probability of losing money. That
  one report is byte-locked (an "anchor") so the result can never silently drift.
- **The first verdict, on v1 cross-sectional momentum: FRAGILE** on all 5
  primary robustness signals. The strategy's good-looking single backtest was
  substantially a lucky path ordering, not a durable edge.

## Why

The operator set the direction: *"the strategies are currently fixed and
limited; check whether the strategy behaves differently with different inputs"*
— and ratified reframing the product so **strategy-robustness is a CORE
pillar** (LLM = support). A single backtest reports one number on one ordering
of history; it physically **cannot** tell you whether a Sharpe of 1.4 is a
property of the *strategy* or a property of *that one path*. The Monte-Carlo
distribution answers exactly that: it measures the *variance of outcome under
input perturbation*. No alpha is claimed from synthetic data — this is
uncertainty quantification of an already-shipped strategy, not prediction
(see [`feature.md`](../feature.md) and the
[direction note](../../dev-notes/strategy-robustness-monte-carlo-direction-2026-05-29.md)).

## The one picture: a lucky path vs the distribution

The single real-2023 backtest sat near the top of what 500 fair replays
produce. The ensemble — the honest picture — is centred essentially at zero and
mostly underwater.

```
v1 momentum — Sharpe across 500 alternate 2023 histories
(each * is a band of resampled paths; the real backtest is marked ↓)

                                                         real 2023 path (≈ +1.40)
                                                                 (off-chart →)
   p5        p25      p50      p75      p95
 -0.068    -0.041   -0.022   -0.005   +0.003
   |---------|--------|--------|--------|
   ****  ********  **********  *******  *        0  (break-even Sharpe)
 ──┴────────┴────────┴──●─────┴────────┴──────────┼──────────── ... ──────►
   worst   .          typical .        best      0.0                  +1.40
   case               (median ≈ 0)     case              ↑ the single backtest
                                                            lived way out here

 P(loss) = 86.8%  ·  P(Sharpe > 1.0) = 0.0%  ·  p95 max-drawdown = 100%
```

Plain-language reading of the percentiles: **p5 = worst-case** (the bad 1-in-20
history), **p50 = typical** (the median replay), **p95 = best-case** (the lucky
1-in-20). For v1 momentum, *even the best case (p95) barely clears zero* and the
typical case loses money. The real backtest you'd normally trust on was the
outlier, not the rule.

## What you can do now

| Action | Command |
|--------|---------|
| Reproduce the FRAGILE verdict end-to-end (~3 min) | `cargo run --release -p backtest --features "candle realdata" --bin monte_carlo -- --generator block-bootstrap-real --paths 500 --ensemble-seed 0xC0FFEE --year 2023` |
| GBM smoke-test variant (fast, no real data, NOT anchored) | `cargo run --release -p backtest --bin monte_carlo -- --generator gbm-smoke --paths 100` |
| Re-verify the byte-locked result against the anchor | `bash scripts/verify_anchors.sh` |
| Read the full anchored distribution report | open [`reports/robustness-20260530-112942-v1-momentum-2023-block-bootstrap-real-fy-mc.md`](../reports/robustness-20260530-112942-v1-momentum-2023-block-bootstrap-real-fy-mc.md) |
| Test the SAME machine against a future strategy | point the harness at a new `Strategy` impl — C1+C2 are now reusable primitives |

## Live demo

This is a **fresh, 4th independent run** by the presenter (not a quote of the
dev/tester runs). It produced a byte-identical result — the same anchor SHA as
all prior runs:

```
$ cargo run --release -p backtest --features "candle realdata" --bin monte_carlo -- \
    --generator block-bootstrap-real --paths 500 --ensemble-seed 0xC0FFEE \
    --year 2023 --out-dir /tmp/mc-presenter-run/

    Finished `release` profile [optimized] target(s) in 0.60s
     Running `target/release/monte_carlo ... --generator block-bootstrap-real --paths 500 ...`
monte_carlo DONE
  report:       /tmp/mc-presenter-run/robustness-20260530-120701-v1-momentum-2023-block-bootstrap-real-fy-mc.md
  body_sha:     72fc7089c5f04885e8a2169d91c242a50e47b7820eea38b446a4dfaa2c1938c4
  wall_clock_s: 181.0
  n_paths:      500
  sharpe p5/p50/p95: -0.0676 / -0.0219 / 0.0031
  max_dd tail p50/p95: 85.29% / 100.00%
  P(loss): 0.8680  P(Sharpe>0): 0.1320  P(Sharpe>1): 0.0000
```

Notice the `body_sha: 72fc7089...` — identical to dev run 1, dev run 2, and the
tester's reproduction run. **Four independent runs, one SHA.** The full stdout
+ SHA-verification is saved under
[`artifacts/robustness-lane-c1-c2-2026-05-30/monte_carlo-live-run-stdout.txt`](artifacts/robustness-lane-c1-c2-2026-05-30/monte_carlo-live-run-stdout.txt).

### The distribution summary block (from the anchored report)

```
## Per-metric distribution
| metric       | p5        | p25       | p50       | p75       | p95       |
|--------------|-----------|-----------|-----------|-----------|-----------|
| sharpe       | -0.067576 | -0.041437 | -0.021924 | -0.004752 | 0.003101  |
| sortino      | -0.094839 | -0.058373 | -0.030719 | -0.006705 | 0.004391  |
| calmar       | -0.311330 | -0.165741 | -0.109217 | -0.022870 | 0.016522  |
| max_drawdown | 61.32%    | 73.39%    | 85.29%    | 90.93%    | 100.00%   |
| total_return | -97.57%   | -80.46%   | -60.54%   | -15.08%   | +11.53%   |

## Ensemble robustness
| P(final_equity < initial)  | 0.868000 |   <- 86.8% of replays lose money
| P(Sharpe > 0)              | 0.132000 |
| P(Sharpe > 1.0)            | 0.000000 |   <- 0% clear the promotion bar
| max_drawdown_tail p95      | 100.00%  |   <- a full wipeout in the bad tail
```

## The decision rule was frozen BEFORE the numbers existed (integrity section)

This is the part that lets you trust the verdict instead of suspecting a
just-so story. The pass/fail ruler was **pre-registered on 2026-05-30, while C2
was still in flight and had produced no number**
([`robustness-decision-rule-2026-05-30.md`](../../dev-notes/robustness-decision-rule-2026-05-30.md)).
The bands were frozen first; the result was scored against them — not the
reverse. This is the direct meta-lesson of the v3-vol-overlay no-op era: a
number interpreted only *after* it is seen can be talked into meaning anything.

**The pre-registered bands (lifted verbatim from the decision rule §0):**

| Signal | ROBUST (edge is real) | MARGINAL (inconclusive) | FRAGILE (one lucky path) |
|---|---|---|---|
| **p5 Sharpe** (tail floor) | **≥ +0.5** | `0.0 … +0.5` | **< 0** (the tail loses money) |
| **p50 Sharpe** (central) | ≥ 1.0 | `0.5 … 1.0` | < 0.5 |
| **p95−p5 Sharpe spread** (dispersion) | ≤ ~1.5 | `~1.5 … ~2.5` | > ~2.5 (wildly path-dependent) |
| **prob-of-loss** `P(equity<start)` | **≤ 15%** | `15% … 35%` | **> 35%** (coin-flip-ish) |
| **P(Sharpe > 1.0)** (gate fraction) | ≥ 60% | `35% … 60%` | < 35% |
| **p95 max-drawdown tail** | ≤ ~50% | `~50% … ~70%` | **> ~70%** (≈ the single-path 73%) |
| **p50 vs single-real-path Sharpe** | p50 within ~0.3 of the real path | — | real path sits **above p75** (real path was favourable) |

**How v1 momentum scored against the frozen ruler** (from the
[tester report](../reports/test-2026-05-30-1155-v0.1.0.md) §5):

| Signal | Value | Band landed | Score |
|--------|-------|-------------|-------|
| p5 Sharpe (tail floor) | −0.068 | < 0 | **FRAGILE** |
| p50 Sharpe (central) | −0.022 | < 0.5 | **FRAGILE** |
| P(loss) | 86.8% | > 35% | **FRAGILE** |
| P(Sharpe > 1.0) | 0.0% | < 35% | **FRAGILE** |
| p95 MaxDD tail | 100.0% | > ~70% | **FRAGILE** |
| p95−p5 spread | 0.071 | ≤ ~1.5 | ROBUST (interpretive) |
| p50 vs real path | real ≈ +1.40, far above p75 | real path was **favourable** | interpretive |

**Composite = FRAGILE (5/5 primary signals).** Robustness is a weakest-link
property: any one primary signal in the FRAGILE band forces a FRAGILE verdict.
Here all five do. The real 2023 path (≈ +1.40 Sharpe) sat far above the
ensemble p75 — the single backtest *flattered* the strategy.

### Two more reasons to trust it

- **Determinism — the result is byte-locked.** Four independent runs (2 dev,
  1 tester, 1 presenter) all produced the identical body-SHA
  `72fc7089...`, which is the locked anchor in `spec/anchors.toml`
  (namespace `mc-robustness-2026-06`). The regression gate is `85 / 85` PASS.
  The verdict cannot silently change.
- **The guards are genuine, not theater** (both mutation-tested by the tester):
  - **C1's shared-index null (FP-C1.5):** when the tester forced
    per-symbol-*independent* resampling (the wrong null), cross-symbol
    correlation collapsed from >0.95 to **−0.079** and the guard caught it. This
    is *why the bad tail is a fair adversary* — coins crash together in the
    replays the way they do in reality, so the strategy can't hide behind fake
    diversification.
  - **C2's divergence gate (FP-C2.1):** distinct-seed path spread = **0.079**
    vs degenerate (identical-seed) spread = **0.0** — a 9-order-of-magnitude
    falsification gap. The harness would have caught a no-op collapse where all
    paths secretly came out identical.

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
| R-NR.6(a) / FP-C2.1 | C2 divergence-from-single-path gate (no-op falsifier) | VERIFIED | [C2 report](../reports/test-2026-05-30-1155-v0.1.0.md) §6 Gate 3 — distinct-seed spread 0.079 vs degenerate 0.0 |
| R-NR.6(b) | C2 two-run byte-identity | VERIFIED | C2 report §5 — 3 runs identical body-SHA `72fc7089` (+ presenter 4th run) |
| K3 / FP-C2.2 | C2 anchor is sensitive to different inputs | VERIFIED | C2 report §3 — `fp_c2_2_anchor_sensitive_to_different_inputs` PASS |
| ADR-0051 D2 | Deterministic reducer (no par_iter, total_cmp sort, type-7 pct) | VERIFIED | C2 report §6 Gate 6 — code inspection, all 6 sub-checks pass |
| R-NR.5 | `threshold_sweep` calculators lifted verbatim (no anchor drift) | VERIFIED | C2 report §6 Gate 7 — all 84 prior anchors byte-identical |
| R3.2 | Exactly one new anchor (the distribution summary) | VERIFIED | `verify_anchors.sh` — 85/85, the +1 is `v1-momentum-2023-block-bootstrap-real-fy-mc` |
| Pre-flight (decision rule §4.1) | Generator = block-bootstrap-real AND mode = shared-index (verdict not void) | VERIFIED | anchored report header — `generator: block-bootstrap-real`, `bootstrap_mode: shared-index` |

## Numbers that matter

- **Headline verdict:** v1 momentum = **FRAGILE** (5/5 primary signals).
- **p50 (typical) Sharpe = −0.022** · **p5 (worst-case) = −0.068** · **p95
  (best-case) = +0.003**.
- **P(loss) = 86.8%** · **P(Sharpe > 0) = 13.2%** · **P(Sharpe > 1.0) = 0.0%**.
- **p95 max-drawdown tail = 100%** (full wipeout in the bad 1-in-20 history);
  p50 max-DD = 85.3%.
- **Paths:** N = 500 shared-index block-bootstrap replays of 2023-FY real
  Binance returns; auto-selected block length L = 204.
- **Tests:** C1 = 23/23 unit + 4 mutation-verified probes; C2 = 58/58 lib
  (11 targeted) + 6/6 e2e. Zero failures.
- **Anchors:** **85 / 85 PASS** (the +1 is the new C2 distribution anchor).
- **Determinism:** body-SHA `72fc7089...` across **4 independent runs**.
- **Perf:** ~181–193 s wall clock per N=500 run (well under the 10-min budget),
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
- **Timing:** ~181 s of compute after build (3 min budget is safe).
- **Expected result:** stdout shows
  `body_sha: 72fc7089c5f04885e8a2169d91c242a50e47b7820eea38b446a4dfaa2c1938c4`,
  `sharpe p5/p50/p95: -0.0676 / -0.0219 / 0.0031`, `P(loss): 0.8680`,
  `P(Sharpe>1): 0.0000`, `max_dd tail p50/p95: 85.29% / 100.00%`. The **p50
  Sharpe = −0.0219** (≈ −0.022) and the body-SHA matching the anchor are the
  two things to confirm — they prove FRAGILE-and-reproducible.
- **Failure diagnosis:**
  - *Different body-SHA* → you are NOT on the Apple-Silicon canonical box;
    cross-platform byte-parity is not contracted (ADR-0051 D5) — the
    *percentiles* should still match to ~6dp, which is what the verdict rests on.
  - *`block-bootstrap-real` data error* → real Binance parquet missing under
    `data/binance/`; the pinned `--expected-revision-sha` did not resolve.
  - *Report header says `bootstrap_mode: per-symbol-independent` or
    `generator: gbm-smoke`* → the verdict is **void** (the tail is no longer a
    fair adversary); re-run with the exact flags above.
- **Cleanup:** `rm -rf /tmp/mc-verify/` — the throwaway report is not anchored
  and is safe to delete (the canonical anchored copy lives in
  `spec/strategy-robustness-harness/reports/`).

## Open decisions

Two decisions. The first is the load-bearing one; the second only matters once
the first is settled.

1. **Accept the FRAGILE verdict?** Per the pre-registered rule, FRAGILE means
   **v1 cross-sectional momentum does NOT advance to paper/live as-is** — its
   `paper→live` gate is BLOCKED on the robustness axis. (Cost gates, the 30-day
   paper requirement, and PM signoff were always independent criteria; this
   verdict is specifically the *robustness* axis.) Accepting this is accepting
   that the single 2023 backtest substantially over-stated the edge. *This is a
   methodology win, not a project failure — the harness did exactly its job.*

2. **What is the next step?** (Decide only after #1.) The ordered candidates:
   - **C3 — parameter sweep** *(natural next)*: re-runs this same harness across
     the whole θ grid to answer *"is the fragility specific to this one tuned
     θ\*, or is the entire momentum family fragile?"* The current verdict judges
     **path** robustness at one θ; C2 explicitly does **not** measure parameter
     robustness (that section is a stub at v0.1.0). C3 closes that gap and is the
     most direct follow-on.
   - **Pivot strategy family**: if you suspect momentum is structurally fragile,
     skip the sweep and put a different family (e.g. mean-reversion / carry)
     through the same C1+C2 machine.
   - **C5 — PBO / Deflated-Sharpe** (López de Prado overfit guards): a different,
     heavier axis of robustness. C2 does **not** emit these.
   - Note: per Q3, the **learning loop (C4) is ordered LAST** — it is not a
     candidate here.

## Transparency notes (no surprises)

- **Pre-existing clippy debt (NOT a C2 regression).** Under the canonical strict
  gate `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
  there are ~37–42 pre-existing errors in *pre-C2* backtest files
  (`sma_composed_run.rs`, `engine.rs`, `paths.rs`, etc.), masked when only the
  bare `clippy -p backtest` lib gate is run. **C2's own four new files are
  clippy-clean** (tester verified zero errors in `stats/`, `montecarlo`,
  `monte_carlo`, `montecarlo_e2e`). The orchestrator filed a separate cleanup
  chip; it does not touch this verdict.
- **Spec-lint:** `FAIL (99 violations in 4 categories)` — **all pre-existing**
  (87 dead-link, 3 missing-frontmatter, 2 shipped-no-tests, 7 trace-broken-path)
  and **zero new vs the tester PASS state** (the tester itemized the identical
  99; the one transient `unreferenced-anchor` was resolved when the C2 anchor was
  cited). No structural regression landed since `VERDICT → PASS`.
- **Scope honesty:** this verdict is **path-robustness at one θ over resampled
  real 2023 history**. It does NOT speak to parameter robustness (→ C3), PBO /
  Deflated-Sharpe (→ C5), or out-of-history regimes the bootstrap can't
  synthesize (a block bootstrap resamples real blocks; it cannot invent a
  2025-style event 2023 never contained).

## Reusability (the lasting payoff)

C1 (`BlockBootstrapPathGen` + `MonteCarloPathGen` trait, in `crates/data`) and
C2 (the harness + `DistributionSummary` reducer, in `crates/backtest`) are now
**primitives**. Every future strategy can be run through the identical "500
fair alternate histories → one anchored verdict" pipeline at near-zero marginal
cost. The robustness question is no longer bespoke per strategy — it is a button.

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback
_empty until operator fills_

## Changelog
- 2026-05-30 (presenter): initial release deck for the C1+C2 Monte-Carlo
  robustness lane. Headline: v1 momentum FRAGILE (5/5 primary signals,
  pre-registered rule). Embedded a fresh 4th-independent live run (body-SHA
  `72fc7089...` matches the anchor), the distribution summary block, the
  pre-registered decision bands, the verification matrix (FP-C1.x/FP-C2.x +
  R-NR.6 + ADR-0051 D2), a self-contained reproduction recipe, and the two
  operator decisions (accept FRAGILE; choose next step C3 vs pivot vs C5).
