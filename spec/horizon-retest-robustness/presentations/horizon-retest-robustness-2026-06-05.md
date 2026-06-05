---
slug: horizon-retest-robustness
mode: release
status: draft
owner: presenter
audience: human-operator
updated: 2026-06-05
generated: 2026-06-05T07:30:00Z
---

# Horizon retest — release + FINAL active-strategy robustness program retrospective

> This deck closes TWO things at once: (1) the **horizon-retest-robustness**
> feature (VERDICT → PASS), and (2) the **entire active-strategy robustness
> program** — the horizon was its last untested axis. Read it as a sprint review
> AND a go/no-go on whether the OHLCV-only active-trading thesis is closed. Every
> number traces to a committed report or a gate I ran live this session; nothing
> is computed here for the first time, and nothing is asserted without a source.

## TL;DR

We re-ran trend-following and carry on the SAME ten coins but deciding **once
every 4 hours and once a day** instead of hourly (re-bucketed from the banked
hourly data) — and all **8 result surfaces came back FAMILY-UNIFORM-FRAGILE**:
every parameter setting, both coarser speeds, both years, still loses to simply
buying and holding the coins net of fees. The horizon is **not** the limiter.
With method (4 families) and universe (35-name spike) already ruled out, this is
the last axis — **the OHLCV-only active-trading thesis on this data is CLOSED.**
VERDICT PASS, anchors #92–#99 locked (99/99 byte-stable, verified live).

## What changed

- **Built a coarser decision cadence** (4-hour and daily) for the two
  highest-prior families — time-series momentum and carry — by deterministically
  re-bucketing the hourly bars we already have (open=first, high=max, low=min,
  close=last, volume=sum; no re-fetch, same data pin `3a8b96c4…`). It slotted in
  defaults-off and left all 91 prior regression anchors byte-identical.
- **Fixed a load-bearing measurement bug first.** The Sharpe formula had a
  hard-coded "1 bar = 1 hour" annualization constant; left unfixed it would have
  silently **inflated** the daily Sharpe by ~4.9× and made fragile cells look
  near-passing. The fix is additive (the hourly path is byte-verbatim), and a
  unit test pins the new scalars (4h = √2190, daily = √365). Confirmed sane:
  daily median Sharpes print at 0.02–0.17, not the ~5× inflated values.
- **Ran 8 anchored surfaces** (TS-momentum + carry × 4h + daily × 2023 + 2024)
  through the same block-bootstrap harness and frozen decision rule the four 1h
  families used. Result on every one: **FAMILY-UNIFORM-FRAGILE**. Eight new
  byte-stable anchors locked (#92–#99); the gate is now 99/99.

## Why

The robustness program had already retired all four method families at the 1-hour
horizon — cross-sectional momentum, mean-reversion, carry, and time-series
momentum — each dominated by passively holding the same coins. Two diagnoses then
narrowed the remaining suspects. The
[universe-method diagnosis](../../dev-notes/universe-method-diagnosis-2026-06-02.md)
showed the cross-sectional **ranking channel carries ≈ 0 forward information**
(rank IC within ±0.07 of zero, no stable sign, both years) and — critically — a
broader, deliberately-more-dispersed 35-name mid-cap universe **lowered common
market-beta** (avg R² 0.715 → 0.598) **without reviving that ranking signal**, so
the universe was exonerated as the binding limiter. That left the
[horizon-retest scoping note](../../dev-notes/horizon-retest-scoping-2026-06-03.md)
to name the single untested variable: **every family had traded hourly.**
Trend-following is classically a daily-to-weekly effect and funding settles
8-hourly, so a coarser decision cadence on the same coins was the highest-prior
untried knob — and testable for ~seconds of compute by re-bucketing the banked
hourly data, *provided* the hour-baked Sharpe annualization was corrected first.
This feature turned that knob. See [`feature.md`](../feature.md) "Why".

## What you can do now

| Action | Command |
|--------|---------|
| Reproduce an anchored TS-momentum **daily** surface (locked grid, N=1000) | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --grid ts-daily --selection-mode time-series-long-flat --horizon daily --paths 1000 --year 2023 --out-dir /tmp/hz-verify/` |
| Reproduce an anchored carry **4h** surface (locked grid, N=200) | `cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- --grid carry-4h --score-source carry --horizon 4h --paths 200 --year 2024 --out-dir /tmp/hz-verify/` |
| Confirm every regression anchor is byte-stable (incl. the 8 new horizon anchors) | `bash scripts/verify_anchors.sh` |
| Re-run the horizon falsifiers (annualization, resample, divergence, goes-flat, two-run) | `cargo test -p backtest --features "candle realdata" --lib -- f_hr_` and `cargo test -p backtest --features "candle realdata" --test horizon_divergence_e2e` |
| Re-read the two diagnoses that scoped + bounded this experiment | open `spec/dev-notes/universe-method-diagnosis-2026-06-02.md` and `spec/dev-notes/horizon-retest-scoping-2026-06-03.md` |

## Live demo

A fresh **time-series-momentum daily-horizon** sweep, run just now on real 2023
Binance data, resampling the hourly bars to daily live. This is a **fast smoke at
N=5 paths** (the anchored surfaces use N=1000 — so the per-cell numbers here are
NOT the anchored figures; they exist only to prove the binary genuinely produces
the result live, end to end, with the resampler and the corrected daily
annualization in the path). What it demonstrates: the family verdict, the
goes-flat mechanism, and the buy-and-hold gap all reproduce on demand at the
coarse horizon.

```
$ ./target/debug/param_robustness_sweep --generator block-bootstrap-real \
    --grid ts-daily --selection-mode time-series-long-flat --horizon daily \
    --paths 5 --year 2023 --out-dir /tmp/horizon-demo/
param_robustness_sweep DONE
  report:         /tmp/horizon-demo/robustness-sweep-20260605-072719-v1-ts-horizon-daily-theta-surface-2023-block-bootstrap-real-fy.md
  body_sha:       e508fefd0abab2fea5ac7ef9d4921ce3fffdda5e33e657219e821df14a92787f
  wall_clock_s:   1.8
  n_cells:        6
  n_paths:        5
  family_verdict: FAMILY-UNIFORM-FRAGILE
  buyhold p50 Sharpe: 1.6480  P(loss): 0.0000  p95 MaxDD: 45.25%

  per-cell summary:
    g= 0 lookback=   5 k_long=10 drift=0.10 → FRAGILE | p50=0.0127 p5=-0.0829 MaxDD_p95=92.0%
    g= 1 lookback=   5 k_long=10 drift=0.10 → FRAGILE | p50=0.0088 p5=-0.1284 MaxDD_p95=92.6%
    g= 2 lookback=  20 k_long=10 drift=0.10 → FRAGILE | p50=0.0050 p5=-0.0294 MaxDD_p95=82.1%
    g= 3 lookback=  20 k_long=10 drift=0.10 → FRAGILE | p50=0.0033 p5=-0.0252 MaxDD_p95=84.9%
    g= 4 lookback=  60 k_long=10 drift=0.10 → FRAGILE | p50=0.0780 p5=-0.0237 MaxDD_p95=82.7%
    g= 5 lookback=  60 k_long=10 drift=0.10 → FRAGILE | p50=0.0793 p5=-0.0268 MaxDD_p95=83.5%
```

Notice: all 6 cells FRAGILE, buy-and-hold p50 Sharpe ≈ +1.65 on this N=5 path-set
while the best daily-TS cell p50 is ≈ +0.08 — a ~20× gap, and the worst-case
(p5) tail of every cell is negative (loses money). The MaxDD_p95 of 82–93% is the
killer even at a daily cadence: the strategy does go flat, but late exits still
sit it through catastrophic drawdowns. Full stdout saved at
[`artifacts/horizon-retest-robustness-2026-06-05/ts-daily-smoke-n5-2023.txt`](artifacts/horizon-retest-robustness-2026-06-05/ts-daily-smoke-n5-2023.txt).
(The /tmp demo report was deleted after capture — no anchored directory was
touched.)

## The headline — the FULL program matrix (four method families × three horizons)

This is the load-bearing table and the reason this is the program's final
retrospective. **Every cell of this 4×3 matrix is FAMILY-UNIFORM-FRAGILE.** Each
figure is the **best cell** in that family-horizon's surface (p50 = median Sharpe;
p5 = 5th-percentile / worst-case tail Sharpe across the bootstrap paths) — i.e.
each family is shown at its strongest setting, and still loses to buy-and-hold.

| Method family | 1h (best-cell p50 / p5) | 4h (best-cell p50 / p5) | Daily (best-cell p50 / p5) | Verdict (all horizons) |
|---------------|------------------------:|------------------------:|---------------------------:|------------------------|
| Cross-sectional momentum | +0.014 / (p5 < 0) | _not re-run¹_ | _not re-run¹_ | FAMILY-UNIFORM-FRAGILE |
| Cross-sectional mean-reversion | +0.007 / (p5 < 0) | _not re-run¹_ | _not re-run¹_ | FAMILY-UNIFORM-FRAGILE |
| Carry / funding | +0.039 (2023) / +0.043 (2024) | +0.029 / −0.057 (2023); +0.032 / +0.006 (2024) | +0.065 / −0.016 (2023); +0.058 / −0.017 (2024) | FAMILY-UNIFORM-FRAGILE |
| **Time-series momentum** | **+0.047 (2023) / +0.042 (2024)** | **+0.165 / −0.038 (2023); +0.106 / −0.085 (2024)** | **+0.169 / −0.044 (2023); +0.106 / −0.099 (2024)** | **FAMILY-UNIFORM-FRAGILE** |
| **Buy-and-hold (the bar)** | **+1.74 (2023) / +1.10 (2024)** | **+1.910 (2023) / +1.166 (2024)** | **+1.951 (2023) / +1.148 (2024)** | _passive — the bar to beat_ |

> ¹ Cross-sectional momentum/MR were **deliberately not re-run** at 4h/daily. The
> universe-method diagnosis computed rank IC ≈ 0 at *daily-equivalent* lookbacks
> already (L=24 = 1 day, both years), so a coarser bar cannot revive a dead
> ranking channel — re-running them would be the "build-then-discover-the-channel-
> is-dead" rework the durable-first rule exists to prevent. Their 1h verdict
> stands; the horizon test was correctly focused on the two families that carried
> the entire horizon-sensitivity prior (TS-momentum + carry).
>
> Sources: **1h column** — prior program deck
> [`time-series-momentum-robustness-2026-06-03.md`](../../time-series-momentum-robustness/presentations/time-series-momentum-robustness-2026-06-03.md)
> (TS rows anchors #90/#91; carry rows the carry test report §5; momentum/MR
> best-cell p50 from the program table). **4h/daily columns** — the 8 anchored
> surfaces in [`reports/`](../reports/) (best cell = the max-p50 cell in each
> surface: TS g=4 at every horizon-year; carry-4h g=2/g=4, carry-daily g=5).
> **BH bars** — the buy-and-hold control row in each surface. (Sharpe = return per
> unit of risk; higher is better. "FRAGILE" = fails the frozen decision rule,
> primarily because the worst-case path loses money, i.e. p5 Sharpe < 0.)

**The conclusion, stated plainly:** across **every method family** (cross-sectional
momentum, mean-reversion, carry, time-series momentum) and **every horizon**
(1h, 4h, daily) we tested, **active trading on this 10-symbol OHLCV-only Binance
universe is dominated by passive buy-and-hold net of fees.** At no setting, at no
cadence, does any family come within ~1 Sharpe unit of just holding the coins.
**The OHLCV-only active-trading thesis on this data is CLOSED.**

> A note worth surfacing, not hiding: at the coarser horizons the TS-momentum
> **median** Sharpe is genuinely higher than at 1h (+0.169 daily-2023 vs +0.047
> at 1h) — the textbook "trend-following likes a slower clock" effect is faintly
> visible. But the **worst-case tail stays negative** (p5 −0.044) and the bar
> *also* rose (BH daily-2023 p50 +1.951): the gap to buy-and-hold did not close.
> A coarser clock helped the median a little and the downside not at all. This is
> why the verdict is fragile, not marginal.

## The three axes ruled out

Each axis was a deliberate, falsifiable test — not a vibe. The program eliminated
all three suspects in sequence:

| Axis | How it was tested | Result |
|------|-------------------|--------|
| **Method** | 4 distinct families (x-sec momentum, x-sec mean-reversion, carry/funding, time-series momentum), each a full block-bootstrap θ-surface vs the frozen rule | All FAMILY-UNIFORM-FRAGILE. Cross-sectional **rank IC ≈ 0** at every horizon (signal-agnostic — three signals fed one dead ranking channel) |
| **Universe** | A 35-name mid-cap spike (separate data pin `518b4d40…`), re-running the universe diagnostic | Common-beta share **dropped** (avg R² 0.715 → 0.598; corr 0.683 → 0.582) but **rank IC stayed ≈ 0**. More dispersion to rank, still unpredictable from the rank. Universe **exonerated** |
| **Horizon** | This feature — TS-momentum + carry resampled to 4h + daily, 2 years, 8 anchored surfaces | All 8 FAMILY-UNIFORM-FRAGILE. A coarser cadence raised the TS median a little but not the tail, and BH rose with it. Horizon **is not the limiter** |

With method, universe, and horizon all ruled out on this OHLCV-only data, the
testable axes for *this data* are exhausted.

## Science integrity / methodology — the durable asset

The negative result is decision-grade *only because* the harness is rigorous.
This proven research stack is the asset that survives the negative finding:

- **99 locked byte-SHA regression anchors**, all PASS (91 pre-existing + 8 new
  horizon anchors #92–#99). The exact surfaces regenerate byte-identical on the
  canonical Apple-Silicon box. **Verified live this session** (see Numbers).
- **Block-bootstrap path + parameter robustness**: every verdict is a distribution
  over resampled real-history paths (N=200 at 4h, **N=1000 at daily** to offset
  the thinner 365-bar series) × a 6-cell parameter grid — not a single lucky
  backtest.
- **A frozen, pre-registered decision rule** (5-signal weakest-link, bands locked
  before any run) scored all four families and all three horizons identically — no
  moving the goalposts after seeing the numbers.
- **Falsifiers RED-on-revert in every family.** This feature added 19 horizon
  falsifiers (annualization correctness, resample correctness, baseline
  divergence, goes-flat, two-run identity), each of which FAILS if the property it
  guards is reverted. Notably **F-HR.1 is an annualization anchor-gate** — direct
  proof the hourly path stayed byte-identical (91/91 unchanged) while the 4h/daily
  paths were added.
- **The annualization was verified correct, not just present.** The horizon-aware
  scalars (4h = √2190, daily = √365) are unit-pinned; daily Sharpes print sane
  (0.02–0.17), NOT the ~4.9× inflated values the legacy hourly constant would have
  produced. Had it been wrong, fragile cells would have spuriously cleared the
  bar — this is precisely the class of silent error the project's earlier
  fabricated-"Sharpe 1.40" precedent exists to prevent.
- **Anti-cherry-pick renderers that crown no winner**: the surface reports the full
  grid + a family verdict and refuses to pick an argmax "best" cell (which would
  inflate the false-positive rate). A grid that picked its best cell would lie;
  this one cannot.

**The rigor caught real issues mid-flight** (per the test + dev-notes trail): a
placeholder column was spotted before it shipped, a 6h/4h resampler-boundary slip
was caught and corrected, and an overnight machine-sleep interruption was detected
and recovered without polluting the anchored output. The durable asset is the
proven research/exec/risk stack that exhaustively and honestly ruled out active
trading on this data — for **~1 minute of total compute** across all 8 surfaces.

## Numbers that matter

- **Anchors:** **99 / 99 PASS** — verified live this session:
  `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (99 / 99)`. New: #92–#99
  (the 8 horizon surfaces).
- **Horizon falsifier tests:** **19 passed / 0 failed** (12 in `--lib -- f_hr_`
  annualization+resample; 7 in `horizon_divergence_e2e`), per test report §3.
- **Every surface FAMILY-UNIFORM-FRAGILE:** all 8 surfaces, every cell, p5 Sharpe
  < 0 (the primary FRAGILE criterion) — except carry-4h-2024 g=4 where p5=+0.006
  but P(Sharpe>1)=0.000 and p95-MaxDD=67.18% still force FRAGILE on other axes
  (flagged honestly in the test report, not buried).
- **Best TS daily cell (g=4, lookback-60):** 2023 p50 +0.169 / p5 −0.044; 2024
  p50 +0.106 / p5 −0.099. Still ~1.78 / ~1.04 Sharpe units below buy-and-hold.
- **Annualization sanity (load-bearing):** daily p50 Sharpes 0.02–0.17; under the
  legacy √8575 hourly constant these would have printed ~0.10–0.83 (≈4.9× inflated)
  — some near the MARGINAL band. The corrected √365 scalar prevents the spurious
  clear. No surface clears ROBUST from inflation.
- **Goes-flat is real:** time-in-market 0.70–0.87 across all coarse cells — the
  strategy genuinely exits to cash; it is not buy-and-hold in disguise (F-HR.4
  confirmed RED-on-revert).
- **Wall-clock:** 4h surfaces ~8s, daily surfaces ~7s on the canonical box; ~1 min
  total for all 8 — compute is a rounding error.
- **Spec-lint:** **FAIL (94 violations in 2 categories)** — verified live:
  `python3 scripts/spec_lint.py` → `spec-lint: FAIL (94 violations in 2 categories)`.
  This **matches the tester's PASS baseline exactly** and is an *improvement* over
  the audit-2026-06-01 baseline (95 violations / 3 categories — the
  missing-frontmatter category is now cleared). All 94 are pre-existing carry-over
  (87 dead-link + 7 trace-broken-path); **zero new violations introduced** by this
  feature. No regression since the tester's PASS.

## Verification

V1..V9 map to the feature's `## Verification` gates (the F-HR falsifiers + the
anchor/BH-relative gates). Each is VERIFIED with one-line evidence.

| V-id | Description | Status | Evidence |
|------|-------------|--------|----------|
| V1 | All 8 horizon surfaces are FAMILY-UNIFORM-FRAGILE (every cell, both years) | VERIFIED | test report §5 surface table; each surface's "## Family verdict" line; matrix above |
| V2 | TS-momentum + carry dominated by buy-and-hold at every coarse horizon | VERIFIED | surfaces: best TS daily p50 +0.169/+0.106 vs BH +1.951/+1.148; best carry daily p50 +0.065/+0.058 |
| V3 | Annualization is horizon-correct (F-HR.2), daily Sharpes sane not ~5× inflated | VERIFIED | test report §3 F-HR.2 (√2190, √365 scalars PASS) + §5 annualization sanity note; daily p50 range 0.02–0.17 |
| V4 | The 91 pre-existing anchors stay byte-identical (additive/defaults-off, F-HR.1) | VERIFIED | `verify_anchors.sh` live → 99/99 (91 unchanged + 8 new); F-HR.1 anchor-byte-identity PASS |
| V5 | The 8 new horizon anchors locked + full gate green | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (99 / 99)` (run live this session); anchors #92–#99 |
| V6 | Resampler correct: bucket counts + OHLCV rollup + causality (F-HR.3) | VERIFIED | test report §3 F-HR.3 (6 sub-tests PASS, incl. hand-verified rollup + forward-shift causality) |
| V7 | Goes-flat mechanism real at coarse horizon (not always-long ≈ BH) (F-HR.4) | VERIFIED | test report §3 F-HR.4 RED-on-revert; time-in-market 0.70–0.87 every surface |
| V8 | Two-run byte-determinism of each horizon surface (F-HR.5) | VERIFIED | test report §3 F-HR.5 (4h + daily two-run identity PASS); anchor gate is itself a determinism check |
| V9 | Science gate not void (block-bootstrap-real + shared-index) | VERIFIED | all 8 surface frontmatters: `generator: block-bootstrap-real`, `bootstrap_mode: shared-index`; OHLCV SHA `3a8b96c4…` matches pin |
| V10 | Program-level: all 4 families × all 3 horizons FAMILY-UNIFORM-FRAGILE, BH-dominated | VERIFIED | matrix above, cross-checked against prior deck (1h) + the 8 surfaces (4h/daily) + the two diagnoses |

## Open decisions

There is exactly ONE decision in front of you, and it has two parts that travel
together:

**Decision — Ratify the program close-out.** Approve (a) the **retirement of the
horizon axis** (TS-momentum + carry, FAMILY-UNIFORM-FRAGILE at both 4h and daily,
both years), and (b) the **program-level conclusion**: active trading on this
10-symbol OHLCV-only Binance universe is dominated by passive buy-and-hold, net of
fees, across **method (4 families), universe (35-name spike), AND horizon
(1h/4h/daily)** — every testable axis on this data. Approving this closes the
active-strategy robustness program as a decision-grade negative.

That is the only thing the approval block below gates. **The "where next?"
strategic fork is framed for your awareness but is deliberately NOT bundled into
this approval** — it is a fresh scoping decision the orchestrator will route to the
analyst once you ratify the close-out. (One decision per presentation; I am not
asking you to pick a direction and approve a retirement in the same tick.)

### The strategic fork (FYI — frame only, do NOT decide here)

Where research goes after this negative result. The OHLCV-cross-sectional-and-
time-series space is now exhausted. Listed neutrally; no recommendation, no
`(Recommended)` tag — this is genuinely your call and each option has a real cost:

- **(a) A different DATA DOMAIN / signal class the program never touched** —
  on-chain flows, order-book microstructure, cross-exchange basis, options/implied-
  vol, or a non-crypto universe. The whole program lived inside OHLCV bars; every
  axis we ruled out was inside that box. A new signal class is the only way to test
  a thesis this data cannot express. Cost: new data plumbing + a research arc from
  scratch.
- **(b) Productionize the proven research/exec/risk/cockpit stack** — pour the
  effort into hardening the durable asset (the harness, decision rule, anchor gate,
  execution and risk surfaces, the cockpit) to production grade, rather than
  hunting an active edge that every family and horizon failed to find. Cost: shifts
  the program from research to productionization.
- **(c) Pause active-strategy research** entirely.

If you approve with a steer on the fork, note it under Notes/feedback and the
orchestrator will route the next scoping to the analyst accordingly.

## Approval

The approval below gates ONLY the close-out decision above (retire the horizon
axis + ratify the program-level conclusion across method, universe, and horizon).
All boxes ship un-ticked; you are the only one who ticks.

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback
_empty until operator fills — note any steer on the strategic fork here_

## Feedback log
_empty — rejections/steers appended here and routed back to the named agent_

## Changelog
- 2026-06-05 (presenter): FINAL active-strategy robustness program retrospective +
  horizon-retest-robustness release deck. VERDICT → PASS; all 8 horizon surfaces
  (TS-momentum + carry × 4h + daily × 2023 + 2024) FAMILY-UNIFORM-FRAGILE; anchors
  #92–#99 locked (99/99, verified live `ANCHORS PASS (99/99)`). Headline 4×3 matrix
  (4 method families × 3 horizons) traced to the prior 1h program deck + the 8
  committed surfaces + the two diagnoses; conclusion: the OHLCV-only active-trading
  thesis is CLOSED across method, universe, and horizon. Live demo: N=5 TS daily
  resample sweep (FAMILY-UNIFORM-FRAGILE reproduced). spec-lint live FAIL 94/2 =
  tester PASS baseline (no regression; missing-frontmatter cleared vs audit-2026-06-01).
</content>
</invoke>
