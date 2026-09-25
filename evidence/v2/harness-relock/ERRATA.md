# Errata — harness re-lock regeneration (story 1-26)

**Status: AC4 / AC5 measured. One finding is ESCALATED to the operator and is NOT
published into any narrative document until ruled on — see § 6.**

This errata re-prices the 34 contaminated θ-surfaces of the bug-log #67 inventory against
clean execution arithmetic. The old rows stay byte-frozen where they are; nothing under
`evidence/v1/**/reports/` or `evidence/v2/perp-basis-mn-spread/**` was touched, and
`scripts/verify_anchors.sh` reports **ANCHORS PASS (119 / 119)** both before and after
(AD-2, ADR-0038 § D6 anchor-additive contract).

## 1. Provenance

| field | value |
|---|---|
| driver | `scripts/relock/run_surfaces.sh --out-dir evidence/v2/harness-relock/reports` |
| manifest | `scripts/relock/surfaces.tsv` — 34 rows, derived from `GridKind` and gate-tested by `crates/backtest/tests/relock_manifest.rs` |
| code under test | `2d63ddef7964e6a4d5bce6082478ab06f6d04fd8` |
| binary | `param_robustness_sweep`, `--release --features candle,realdata` |
| window | 2026-09-25 16:49:29Z → 18:03:12Z |
| wall clock | 4462 s = **74.4 min** at `RAYON_NUM_THREADS=8`, `nice -n 19` |
| outcome | 34 / 34 `ok`, 0 failures (`logs/progress.tsv`) |
| out-dir | `evidence/v2/harness-relock/reports/` — a NEW namespace, per AC3 |

The story's Dev Notes budgeted **10.3 h as a floor, 15–20 h realistic**. The measured figure is
**74 minutes**. The estimate was extrapolated from a single long-only momentum surface
(1087 s) and assumed the other 33 were comparable. They are not: two surfaces
(`v1-momentum-2023` at 1461 s and `v1-mr-2023` at 2258 s) account for **83 % of the entire
run**; the remaining 32 finish in 12 minutes together. The estimate was wrong by an order of
magnitude, in the safe direction.

## 2. Scope and join

Every one of the 34 new surfaces joins **1:1** to an existing anchored report by the
`scenario:` front-matter key — 34 matched, 0 missing, 0 ambiguous. Two lanes, 156 θ-cells
total.

**Not one of the 34 is numerically unchanged.** Every surface moved. The #67 inventory did
not over-count.

## 3. AC4 — per-scenario old vs new

| # | scenario | slug | family verdict old → new | cells FRAGILE→MARGINAL | Δ p50 (median) | Δ p95_maxdd (median) | cells beating its own BUYHOLD |
|---|---|---|---|---|---|---|---|
| 1 | `v1-basis-reversal-fee00bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-signal-robustness | **FAMILY-UNIFORM-FRAGILE → FAMILY-HAS-NON-FRAGILE-CELLS** | 5 | +0.8802 | -65.88 pp | 0 |
| 2 | `v1-basis-reversal-fee00bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-signal-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.6341 | -45.38 pp | 0 |
| 3 | `v1-basis-reversal-fee02bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-signal-robustness | **FAMILY-UNIFORM-FRAGILE → FAMILY-HAS-NON-FRAGILE-CELLS** | 5 | +0.8509 | -65.86 pp | 0 |
| 4 | `v1-basis-reversal-fee02bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-signal-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.5818 | -45.11 pp | 0 |
| 5 | `v1-basis-reversal-fee05bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-signal-robustness | **FAMILY-UNIFORM-FRAGILE → FAMILY-HAS-NON-FRAGILE-CELLS** | 2 | +0.7993 | -64.72 pp | 0 |
| 6 | `v1-basis-reversal-fee05bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-signal-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.5059 | -44.29 pp | 0 |
| 7 | `v1-basis-reversal-fee10bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-signal-robustness | **FAMILY-UNIFORM-FRAGILE → FAMILY-HAS-NON-FRAGILE-CELLS** | 2 | +0.6575 | -63.63 pp | 0 |
| 8 | `v1-basis-reversal-fee10bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-signal-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.3846 | -42.64 pp | 0 |
| 9 | `v1-carry-horizon-4h-theta-surface-2023-block-bootstrap-real-fy` | horizon-retest-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.8900 | -66.24 pp | 0 |
| 10 | `v1-carry-horizon-4h-theta-surface-2024-block-bootstrap-real-fy` | horizon-retest-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.4605 | -54.01 pp | 0 |
| 11 | `v1-carry-horizon-daily-theta-surface-2023-block-bootstrap-real-fy` | horizon-retest-robustness | **FAMILY-UNIFORM-FRAGILE → FAMILY-HAS-NON-FRAGILE-CELLS** | 4 | +0.8483 | -62.06 pp | 0 |
| 12 | `v1-carry-horizon-daily-theta-surface-2024-block-bootstrap-real-fy` | horizon-retest-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.3981 | -46.59 pp | 0 |
| 13 | `v1-carry-theta-surface-2023-block-bootstrap-real-fy` | carry-strategy | **FAMILY-UNIFORM-FRAGILE → FAMILY-HAS-NON-FRAGILE-CELLS** | 2 | +0.7387 | -71.22 pp | 0 |
| 14 | `v1-carry-theta-surface-2024-block-bootstrap-real-fy` | carry-strategy | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.4558 | -47.10 pp | 0 |
| 15 | `v1-momentum-theta-surface-2023-block-bootstrap-real-fy` | momentum-parameter-robustness-sweep | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | -0.5426 | -45.59 pp | 0 |
| 16 | `v1-mr-theta-surface-2023-block-bootstrap-real-fy` | cross-sectional-mean-reversion-strategy | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.5273 | -64.94 pp | 0 |
| 17 | `v1-ts-horizon-4h-theta-surface-2023-block-bootstrap-real-fy` | horizon-retest-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.7836 | -64.04 pp | 0 |
| 18 | `v1-ts-horizon-4h-theta-surface-2024-block-bootstrap-real-fy` | horizon-retest-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.4706 | -55.95 pp | 0 |
| 19 | `v1-ts-horizon-daily-theta-surface-2023-block-bootstrap-real-fy` | horizon-retest-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.7278 | -68.88 pp | 0 |
| 20 | `v1-ts-horizon-daily-theta-surface-2024-block-bootstrap-real-fy` | horizon-retest-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.4407 | -52.61 pp | 0 |
| 21 | `v1-ts-momentum-theta-surface-2023-block-bootstrap-real-fy` | time-series-momentum-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.5650 | -66.52 pp | 0 |
| 22 | `v1-ts-momentum-theta-surface-2024-block-bootstrap-real-fy` | time-series-momentum-robustness | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.4342 | -48.47 pp | 0 |
| 23 | `v2-mn-basis-fee00bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.1771 | -80.19 pp | 0 |
| 24 | `v2-mn-basis-fee00bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.0948 | -63.73 pp | 0 |
| 25 | `v2-mn-basis-fee05bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.0875 | -78.62 pp | 0 |
| 26 | `v2-mn-basis-fee05bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.0182 | -61.44 pp | 0 |
| 27 | `v2-mn-basisperp-fee00bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | -0.0154 | -74.18 pp | 0 |
| 28 | `v2-mn-basisperp-fee00bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | -0.0598 | -65.24 pp | 0 |
| 29 | `v2-mn-basisperp-fee05bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | -0.1719 | -69.54 pp | 0 |
| 30 | `v2-mn-basisperp-fee05bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | -0.2005 | -61.15 pp | 0 |
| 31 | `v2-mn-funding-fee00bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.1552 | -80.84 pp | 0 |
| 32 | `v2-mn-funding-fee00bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.0949 | -62.83 pp | 0 |
| 33 | `v2-mn-funding-fee05bps-theta-surface-2023-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.0856 | -79.43 pp | 0 |
| 34 | `v2-mn-funding-fee05bps-theta-surface-2024-block-bootstrap-real-fy` | perp-basis-mn-spread | FAMILY-UNIFORM-FRAGILE (unchanged) | 0 | +0.0295 | -61.22 pp | 0 |

**Totals — 34 surfaces, 156 cells: 20 cell verdict flips, all FRAGILE → MARGINAL, none in the other direction. Cells beating their own buy-and-hold control: 0.**

### 3.1 What moved, by primary signal

| primary signal | old median | new median | Δ median | Δ min | Δ max | direction |
|---|---|---|---|---|---|---|
| `p5_sharpe` | -0.0437 | -0.3043 | -0.2732 | -2.1479 | +0.2816 | mixed |
| `p50_sharpe` | 0.0252 | 0.5213 | +0.4950 | -1.3238 | +1.0636 | mixed |
| `prob_loss` | 0.2120 | 0.1700 | -0.0475 | -0.5050 | +0.4650 | mixed |
| `P(Sharpe>1)` | 0.0000 | 0.2050 | +0.2050 | +0.0000 | +0.5750 | uniform |
| `p95_maxdd` | 85.5250pp | 26.7000pp | -57.6850pp | -81.3700pp | -31.2900pp | all 156 improved |

The dominant movement is `p95_maxdd`: **improved in all 156 cells without exception**, from a
corpus median of 85.53 % to 26.70 %. The old corpus contained cells at **100.00 %** tail
drawdown — total account destruction. That is the signature of bug-log #94 (the sizer sized
resizes to the target instead of the delta), fixed in `723ca742`. The old drawdown numbers
were an artefact of the sizer, not a property of the strategies.

`p5_sharpe` moves the other way (median −0.2732). This is not a contradiction: the old engine
barely traded, so its distributions were compressed around zero. The clean engine trades, so
the distributions have real dispersion — a worse 5th percentile *and* a much better median and
much better tail drawdown.

### 3.2 Turnover direction — ADR-0089 corrected by measurement

The story's Task note requires this to be reported from measurement rather than carried
forward as an expectation, because ADR-0089's "turnover falls" was written against an engine
that could not resize a held leg at all.

**Measured: turnover RISES.** ADR-0089's directional claim is wrong and is corrected here.

| scenario | Δ trades, median | range |
|---|---|---|
| `v1-basis-reversal-fee00bps-theta-surface-2023-block-bootstrap-real-fy` | **+5.62%** | +4.07% … +11.02% |
| `v1-basis-reversal-fee00bps-theta-surface-2024-block-bootstrap-real-fy` | **+5.47%** | +2.24% … +17.01% |
| `v1-basis-reversal-fee02bps-theta-surface-2023-block-bootstrap-real-fy` | **+7.55%** | +4.47% … +11.57% |
| `v1-basis-reversal-fee02bps-theta-surface-2024-block-bootstrap-real-fy` | **+5.57%** | +2.53% … +17.03% |
| `v1-basis-reversal-fee05bps-theta-surface-2023-block-bootstrap-real-fy` | **+7.58%** | +4.48% … +11.57% |
| `v1-basis-reversal-fee05bps-theta-surface-2024-block-bootstrap-real-fy` | **+5.57%** | +2.52% … +17.03% |
| `v1-basis-reversal-fee10bps-theta-surface-2023-block-bootstrap-real-fy` | **+7.50%** | +4.49% … +11.64% |
| `v1-basis-reversal-fee10bps-theta-surface-2024-block-bootstrap-real-fy` | **+5.66%** | +2.54% … +17.04% |
| `v1-mr-theta-surface-2023-block-bootstrap-real-fy` | **+7.12%** | +3.78% … +12.85% |

**Honest limit on this number:** only **9 of the 34** surfaces publish a `trades` column — the
8 basis-reversal surfaces plus `v1-mr`. For the other 25 (all `carry`, `ts`, `mn` and
`momentum` surfaces) the report format carries no turnover figure, so this errata makes **no
turnover claim** about them. It is not "unchanged"; it is unmeasured.

## 4. AC4 — verdict re-derivation

Every verdict in both corpora was re-derived independently from the published cell numbers
using the frozen `verdict_bands` thresholds, and compared against the verdict the report
printed. **312 classifications, 0 mismatches.** The frozen rule is genuinely the rule that
produced both corpora, and the parse behind every table here is sound.

The re-derivation is one-directional:

- **136 cells** FRAGILE → FRAGILE
- **20 cells** FRAGILE → MARGINAL
- **0 cells** in any other transition. Nothing became ROBUST; nothing got worse.

At surface level, **6 of 34** move from `FAMILY-UNIFORM-FRAGILE` to
`FAMILY-HAS-NON-FRAGILE-CELLS`. All six are on the **2023** half:

| slug | flipped | held |
|---|---|---|
| `perp-basis-signal-robustness` | all four 2023 fee tiers (0/2/5/10 bps) | all four 2024 tiers |
| `carry-strategy` | 2023 | 2024 |
| `horizon-retest-robustness` | `carry-horizon-daily` 2023 | the other 7 |
| `perp-basis-mn-spread` | — | **all 12** |
| `momentum-parameter-robustness-sweep` | — | 1 |
| `cross-sectional-mean-reversion-strategy` | — | 1 |
| `time-series-momentum-robustness` | — | 2 |

The 2023/2024 split is not a separate phenomenon. The 2023 half moved further everywhere
(median Δp50 +0.66…+0.89 against +0.38…+0.63 for 2024); the flips are simply the cells that
crossed a threshold the 2024 cells did not reach.

### 4.1 The control that did not move

**Across all 156 cells, in both corpora, exactly ZERO beat their own buy-and-hold control.**

The strongest flipped cell is `v1-basis-reversal-fee00bps-2023` g=3 at p50 Sharpe **1.1033**,
against a BUYHOLD p50 of **1.7353** on the same 200 paths and the same bootstrap. Passive wins
every single cell of every single surface, at every fee tier, in both years.

`MARGINAL` is not an edge claim, and the reports say so in their own words: C3 makes no
"this θ is robust" claim, and each non-FRAGILE cell is flagged `→ C5 DEFLATION REQUIRED` and
handed to the PBO / Deflated-Sharpe pass before any promotion. None of the 20 flipped cells
has been through that pass.

### 4.2 The #86 / #87 disclosure

The anchored narratives for `#86` and `#87` attribute cell results to a drift hold band that
was **inert when those surfaces ran**. `size_portfolio_target` implements both the gross
exposure cap and the drift band, and it had no production caller until `723ca742`
(2026-08-23). The rows in this re-lock are the first in which that third axis is real.

Anything the old narratives conclude *about the drift dimension specifically* rests on a
column that did not vary. That is a defect of the narrative, not of the grid: the grid always
declared the axis.

## 5. AC5 — band re-examination

The question AC5 asks is whether the frozen `classify_verdict` / `verdict_bands` thresholds
classify the CLEAN surfaces the way they classified the contaminated ones. They do not, and
the difference is in **kind**, not degree.

| primary signal | trips in OLD corpus | trips in CLEAN corpus | sole binding signal, CLEAN |
|---|---|---|---|
| `p5` | 153 / 156 | 134 / 156 | 22 |
| `p50` | 156 / 156 | 71 / 156 | 0 |
| `prob_loss` | 38 / 156 | 26 / 156 | 0 |
| `P(Sh>1)` | 156 / 156 | 114 / 156 | 2 |
| `p95_maxdd` | 149 / 156 | 0 / 156 | 0 |

Cells in the OLD corpus where exactly one signal was binding: **0**. In the clean corpus: **24**.

Read that table as a statement about how much work each arm of the gate is doing.

**In the old corpus the gate was saturated.** `p50` and `P(Sharpe>1)` tripped in **156 of 156**
cells — every cell, without exception — and `p95_maxdd` in 149. No cell anywhere was decided by
a single signal. A weakest-link rule in which four of five links fail everywhere is not
discriminating between cells; it is reporting that the corpus is uniformly broken. Which it
was.

**In the clean corpus the gate discriminates.** `p5_sharpe` becomes the dominant binding signal
(134/156) and is the *sole* reason for the verdict in 22 cells. `p50` falls from 156 to 71.

**`p95_maxdd` now trips zero times.** It went from 149/156 to 0/156. The highest tail drawdown
anywhere in the clean corpus is **61.55 %** against a FRAGILE band of **70 %**, and the median
is 26.70 %. On this corpus the drawdown arm of the frozen gate is inert: it cannot change a
single verdict. That is a calibration observation, not a defect — but a band that fires on
everything and then on nothing has never once been the thing that separated two cells, and AC5
exists precisely to say so out loud.

### 5.1 Cells sitting on a band edge — escalated

`P(Sharpe>1)` is a probability over **N = 200 paths**, so its resolution is **0.005 — one path**.
The FRAGILE band sits at **0.350**. Three clean cells land within one path of it:

| P(Sharpe>1) | distance | verdict | scenario |
|---|---|---|---|
| 0.350 | exactly on the edge | FRAGILE | `v1-basis-reversal-fee05bps-…-2024` g=3 |
| 0.355 | +1 path | MARGINAL | `v1-basis-reversal-fee02bps-…-2023` g=1 |
| 0.345 | −1 path | FRAGILE | `v1-basis-reversal-fee05bps-…-2023` g=0 |

The third one matters most. `v1-basis-reversal-fee05bps-2023` g=0 is FRAGILE **solely** because
of this signal — every other primary signal clears its band comfortably (p5 +0.0616, p50
+0.8063, prob_loss 0.035, p95_maxdd 17.23 %). Its verdict rests on **one resampled path**. One
more path above Sharpe 1.0 and it reads MARGINAL.

The same shape, less acutely: `v1-ts-horizon-daily-2023` g=0 is sole-binding on `P(Sharpe>1)`
at 0.325, five paths below the edge.

And on the other arm, of the 22 cells that are FRAGILE on `p5_sharpe` alone, the closest sits at
**p5 = −0.0117** — against a band of exactly 0.0.

**The gate stays byte-frozen (AD-1).** Nothing here is a proposal to move a threshold. The
deliverable AC5 asked for is the answer, and the answer is: on the clean corpus at N=200, a
handful of verdicts are decided by a margin finer than the measurement's own resolution.

## 6. ESCALATION — operator ruling required before any narrative change

AC4 requires that any flip touching the era-qualified thesis's supporting narrative goes to the
operator **before** publication. This one does, so it has not been published. No narrative
document has been edited.

**The finding.** Six 2023 surfaces across three slugs no longer read `FAMILY-UNIFORM-FRAGILE`.
The sentence "uniformly fragile" is no longer what the evidence says for the 2023 basis-reversal
and carry families.

**Why the thesis itself is not in question.** Zero of 156 cells beat buy-and-hold, in either
corpus. Nothing reached ROBUST. `MARGINAL` is explicitly a "must face C5 deflation" label, not an
edge. The ship-passive conclusion for the current era is untouched by this measurement.

**What is in question is the supporting sentence,** and the honest re-statement is arguably
*stronger* than the old one: these arms are not dismissed because their numbers are noise — they
are dismissed because they trade, they produce real distributions, and they **still lose to doing
nothing**.

**Blast radius, checked file by file.** The one narrative claim in `README.md` (line 77) concerns
the *market-neutral* perp-basis spread — that is `perp-basis-mn-spread`, all **12** surfaces of
which held `FAMILY-UNIFORM-FRAGILE`. **The README needs no change.** `CHANGELOG.md` line 137
carries the claim for `momentum-parameter-robustness-sweep`, which also held. **No change.**
The flip lands on stories `1-20` (perp-basis-signal-robustness), `carry-strategy` and
`horizon-retest-robustness`.

**The ruling needed:** whether to re-state those three slugs' verdict lines from
`FAMILY-UNIFORM-FRAGILE` to the measured `FAMILY-HAS-NON-FRAGILE-CELLS (2023) /
FAMILY-UNIFORM-FRAGILE (2024)`, with the buy-and-hold dominance stated alongside it — or to
route the 20 flipped cells through the C5 deflation pass first and re-state afterwards.

## 6b. ESCALATION — the re-lock namespace contradicts ADR-0038 § D6.b

Found while establishing whether these 34 bodies should be anchored. It is a ratified-decision
conflict, so it goes to the operator rather than being resolved here (AD-18).

**Story 1-26 AC3 says:** *"Regeneration goes to a NEW namespace per ADR-0038/0045 § D6; old rows
stay byte-frozen."*

**ADR-0038 § D6.b step 4 says the opposite**, and names this case. D6.b is the *wiring-bug-fix
re-emission protocol* — adopted for exactly the situation this re-lock is in, where "the recorded
body reflects a demonstrated wiring bug". Its step 4: the new SHAs land in `evidence/anchors.toml`
**"in-place under the existing namespaces (Q2=(a) default — never bifurcate the namespace; never
silently delete a row)"**. And its "Not in scope" list names **namespace bifurcation** as rejected,
with the reason: *"bifurcation invites future readers to consume stale bodies."*

That reason is live here. The old rows under `evidence/v1/**/reports/` remain the anchored,
gate-verified corpus. This errata shows their drawdown figures to be artefacts of bug-log #94 — the
old corpus contains cells at 100.00 % tail drawdown that the clean engine puts at 26.70 % median.
A future reader who greps the anchored corpus finds the artefact first and the correction only if
they happen to find this directory.

**Which is right is the operator's call, and D6.b names the mechanism for changing it:** *"If the
protocol itself needs revision … the revision lands as **D6.c** (additive amendment subsection, not
in-place mutation of D6.b)."*

The two coherent outcomes:

- **(A) Follow D6.b as ratified** — re-emit the 34 bodies in place under their existing namespaces,
  with the architect sign-off and the negative invariant D6.b step 5 requires (every unrelated row
  byte-identical, the diff captured). One corpus, no stale bodies, but 34 anchored files change
  their SHAs and `evidence/v1/**` stops being byte-frozen.
- **(B) Ratify D6.c** — an additive amendment permitting a re-lock namespace for a corpus-wide
  re-pricing, with a mandatory back-pointer from every superseded row so a reader cannot consume a
  stale body unaware. AC3's intent, made architecturally legitimate.

**Nothing has been done in either direction.** `evidence/anchors.toml` is untouched, still 119 rows,
still `ANCHORS PASS (119 / 119)`. The 34 new bodies are committed as measurement output in a
non-anchored directory, which prejudges neither outcome.

**What is already proven, either way — these bodies are anchor-grade.** Reproducibility was verified
rather than assumed: four surfaces drawn from three different slugs were re-run independently
(different process, different pid, ~25 min later) and every body-SHA came back **byte-identical**.

| surface | body-SHA | second run |
|---|---|---|
| `v1-ts-horizon-daily-…-2023` | `1c4506ce…` | identical |
| `v2-mn-funding-fee05bps-…-2023` | `2ca27795…` | identical |
| `v1-basis-reversal-fee10bps-…-2024` | `e916b43c…` | identical |
| `v1-carry-horizon-4h-…-2023` | `ec963782…` | identical |

No body carries run-varying metadata: checked mechanically across all 34, **0 of 34** contain a
wall-clock, pid, host or timestamp below the front-matter, which `scripts/hash_report.py` strips.
Anchoring these would not create the bug-log #106 failure mode.

## 7. What this errata does NOT cover

- **AC7 — the four `#[ignore]`d drift gates.** AC7 asks for their pins to be "re-derived from
  the regenerated surfaces". That is not possible as written: the four gates cover
  `top10-2023-1h-momentum`, `top10-2024-h1-momentum`, `top10-2023-fy-tcn-overlay` and
  `top10-2024-fy-tcn-overlay` — **none of which is among the 34 regenerated surfaces**. The
  in-code comment at `crates/backtest/tests/determinism.rs:730` states the reason and says it
  first: those four run through `scenarios/momentum.rs` and `scenarios/tcn_overlay.rs`, lanes
  that `run_path` — and therefore this whole re-lock — never touches. AC7 and the story's own
  Task note contradict each other; the Task note is the later and better-informed one.

  **Re-measured 2026-09-25, after this re-lock, at `2d63ddef`** — all four still RED, and the
  produced hashes are *unchanged from the 2026-08-22 and 2026-08-23 measurements*:

  | gate | pinned | produced 2026-09-25 |
  |---|---|---|
  | `top10-2023-1h-momentum` | `0f6f6eb8…` | `b655e5e7…` — byte-identical to 2026-08-22 |
  | `top10-2024-h1-momentum` | `78976062…` | `37ce69e9…` |
  | `top10-2023-fy-tcn-overlay` | `1460fcc7…` | `64f51802…` |
  | `top10-2024-fy-tcn-overlay` | `b8e9186b…` | `908b66e1…` |

  This is the confirming measurement, not an assumption: the re-lock moved all 34 inventory
  surfaces and moved these four **not at all**. The two sources of movement are disjoint, exactly
  as `determinism.rs` predicted. Three consequences:

  1. **The `#[ignore]` stays.** Removing it now would put CI permanently red on a defect this
     story does not fix and cannot fix.
  2. **The pins stay.** Re-baselining them to `b655e5e7…` et al. is bug-log #77's exact failure
     mode — converting a truthful gate into a rubber stamp — and the in-code comment forbids it
     by name.
  3. **The drift is stable and therefore bisectable.** The same hash across three measurements
     spanning the ADR-0089 D1 sizer wiring and this entire re-lock means the cause is
     deterministic and reproducible. It wants its own story with a bisect, not a line in this
     errata.
- **bug-log #95** — eight lanes still declare a `portfolio_exposure_cap` they cannot enforce.
  None produces an inventory anchor, so it does not block this re-lock. **This errata makes no
  claim that the exposure cap binds engine-wide.** It binds on `run_path`, which is the lane
  behind all 34 of these surfaces.
- **Promotion of any cell.** No cell here is promoted, recommended, or advanced. The 20
  MARGINAL cells are handed to C5, which has not run.

## 8. AC8 standing floor — verified, not assumed

| gate | result |
|---|---|
| `bash scripts/verify_anchors.sh` | **ANCHORS PASS (119 / 119)** — run before and after the regeneration and after every write in this pass |
| `python3 scripts/spec_lint.py` | **PASS (0 violations)** |
| advisor-gate independence | `robustness_bootstrap_bites` **17 passed** — `bakeoff/bootstrap.rs` inputs and outputs unchanged |
| FDR annex identity | `fdr_annex_identity` **2 passed** — ranking unchanged across missing / empty / populated / corrupt ledgers |
| manifest gate | `relock_manifest` **2 passed** — the 34 invocations still derive from `GridKind` |
| FROZEN files (AD-1) | `robustness.rs`, `rank.rs`, `bootstrap.rs` — **byte-untouched**, confirmed against the working tree |

Nothing in this re-lock edits the frozen gate. The verdicts moved because the *inputs* moved.

## 9. Open items this re-lock hands on

| item | where |
|---|---|
| Verdict-narrative ruling for 3 slugs | § 6 — operator |
| Re-lock namespace vs ADR-0038 § D6.b | § 6b, bug-log `#108` — operator/architect |
| AC7 unsatisfiable as written | § 7, bug-log `#109` — needs an AC amendment |
| The four drift gates' own cause | bug-log `#93`/`#109` — wants its own story with a bisect |
| bug-log `#95` exposure cap on 8 lanes | out of scope here, stated explicitly |
| AC6 — unblocking story 1-21 | depends on the § 6 ruling |
