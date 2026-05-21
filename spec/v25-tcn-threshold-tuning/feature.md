---
slug: v25-tcn-threshold-tuning
status: proposed
owner: architect
updated: 2026-05-21
version: 0.1.0
predecessor: v25-tcn-recalibrate v0.1.0
parent: v25-tcn-overlay v2.5.0 (in-progress)
---

# v2.5 — TCN threshold tuning (cheap τ × ε sweep over recalibrated checkpoints)

> Cheap-first follow-on to the
> [`v25-tcn-recalibrate v0.1.0`](../v25-tcn-recalibrate/feature.md) ship
> on 2026-05-21
> ([presenter deck](../v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md)).
> The recalibrate ship eliminated the σ_train 608× / 580× inflation
> (BS-1 10.954 → 0.018, BS-2 6.916 → 0.012; both ∈ 0.005..0.025 — H1
> confirmed) but the joint F-verdict legitimately stays **F4** under
> the immutable
> [ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm)
> priority tree (`frac_inside_epsilon` 0.031 / 0.057 < 0.5 F3
> threshold). **However**, the confidence-gate that was silencing every
> forecast at 0% survival now lets 40.1% of BS-1 forecasts through at
> τ=0.6, 88.8% at τ=0.1; BS-2 34.5% at τ=0.6, 86.4% at τ=0.1. That
> jump from 0% to 40-89% is the substantive new signal that justifies a
> cheap τ × ε sweep BEFORE committing to a multi-week retrain. This
> feature is the τ-sweep half of operator-decided routing (c) from the
> recalibrate deck. The horizon-bump-or-retire fallback is queued
> under [§ Strategy](../backlog.md#strategy) as a stub.

## Why

The recalibrate ship cleanly separated two failure modes that the
predecessor F4-investigation could not distinguish:

1. **σ_train was 608× / 580× inflated** — confirmed and eliminated.
   The gate denominator `|r_hat| / σ_train ≥ τ` now uses the
   converged-model prediction std (`0.018` BS-1, `0.012` BS-2)
   instead of the in-loop training-trajectory variance (`10.954` BS-1,
   `6.916` BS-2). Gate survival jumps from 0% to non-trivial across
   the full τ grid.
2. **F-verdict stays F4** under immutable
   [ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm).
   F3 requires `frac_inside_epsilon > 0.5` — measured 0.031 / 0.057 —
   so the priority tree falls through to F4 by construction. This is
   honest signal; H2 was honestly falsified in the recalibrate ship.

These two findings are **decoupled**. The model emits a reasonable-
magnitude `r_hat` distribution (mean ≈ 0.0009 BS-1, 0.0014 BS-2; std
≈ 0.018 / 0.010; p95 of |r_hat| ≈ 0.032 / 0.020 per the
predecessor's F4-evidence reports — see
[forecast-distribution-bs1-realdata-recalibrated-20260521](../v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md)).
The gate at τ=0.6 + ε=0.0005 (`v25-tcn-overlay` shipped defaults — see
[`with_tcn_bs1_ledger` builder](../../crates/strategy/src/tcn_overlay_momentum.rs#L417-L420)
and the mirror `with_tcn_bs2_ledger` at lines 434-437, both passing
`dec!(0.6)` to `Self::new`) was tuned BEFORE σ_train was known to be
inflated. With the gate denominator now correctly calibrated, the
question becomes: **does the v2.5 TCN model carry directional signal
that the current (τ, ε) pair is filtering out**, or is the F4 verdict
substantive (no signal, period)?

The cheapest way to answer that is a parameter sweep over a small
τ × ε grid, run on top of the recalibrated checkpoints (no retraining,
no weight change, no σ_train change). For each (τ, ε) cell, compute
gate-survivor count + backtest Sharpe + drawdown + total return against
the v1 momentum baseline. If any cell unlocks a Sharpe delta of
non-trivial magnitude vs the `top10-2023-fy-tcn-overlay-realdata` /
`top10-2024-fy-tcn-overlay-realdata` baselines, the v2.5 TCN is salvaged
without retraining. If no cell does, the horizon-bump-or-retire fallback
is funded with clean signal that gate-tuning alone is insufficient.

### Quantitative-finance context

The confidence gate `|r_hat| / σ_train ≥ τ` and the deadband `|direction|
≤ ε` are two of the three load-bearing knobs of the v2.5 TCN overlay
(the third is the σ_train denominator, now fixed). They were locked at
τ=0.6 + ε=0.0005 in
[`v25-tcn-overlay/feature.md § D5`](../v25-tcn-overlay/feature.md)
based on synthetic-GBM calibration BEFORE the recalibrate ship made the
gate denominator interpretable on the real distribution.

The τ knob trades off **precision vs recall** on the directional
forecaster. At low τ, the overlay listens to many low-confidence
forecasts (high recall, low precision — possibly net-noise). At high τ,
the overlay listens only to the highest-confidence forecasts (low
recall, possibly high precision — but if the model's confidence is
mis-calibrated, high-confidence forecasts can be just-as-wrong as
low-confidence ones). The recalibrated `confidence_gate_survival` array
in the predecessor's reports gives us 9 sample points along this trade-
off; the sweep extends to 9 backtests per checkpoint.

The ε knob is the deadband for the OVERLAY direction (combined with
the base momentum signal — see
[`crates/strategy/src/overlay.rs::combine`](../../crates/strategy/src/overlay.rs)).
When `|combined_direction| ≤ ε`, the overlay is silent (no position
change). ε = 0.0005 was sized to the synthetic-GBM forecasts' typical
magnitude; the real-data forecasts have p95(|r_hat|) ≈ 0.032 / 0.020
which is 64× / 40× larger than ε, so ε is structurally very loose under
the current default. The sweep tests whether a tighter ε (more silence)
or a looser ε (more action) extracts alpha.

τ and ε interact: a tight ε (more silence) combined with a low τ (more
forecasts pass) might let medium-confidence forecasts through that a
tight ε alone would have silenced. The sweep is a 2-D grid because the
1-D marginals are not the same as the joint surface.

## Requirements

### R1 — Parameter sweep tool

A new read-only binary, or an extension of the existing
[`forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs)
+ [`sharpe_comparison.rs`](../../crates/forecast/src/bin/sharpe_comparison.rs)
bin family — **architect-decide at M-T1** (see § Open questions § Q1).
Analyst-recommended location: `crates/forecast/src/bin/threshold_sweep.rs`
(new bin, mirrors the recalibrate-bin family shape; keeps the existing
two investigation bins byte-identical so their anchors stay byte-identical).

The bin sweeps a 2-D grid:

- **τ grid (9 cells)** — `{0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9}`.
  Matches the existing
  [`confidence_gate_survival` array](../../crates/forecast/src/bin/forecast_distribution.rs#L325)
  in `forecast_distribution.rs`, so the gate-survivor count per cell is
  already in the recalibrated reports' bodies (re-readable, no re-pass
  required). See Q1 for finer grain alternative.
- **ε grid (5 cells)** — `{0.0001, 0.0005 baseline, 0.001, 0.005, 0.01}`.
  Covers the magnitude range of `r_hat` inference-time std (~0.01-0.02);
  the baseline is in the middle. See Q2 for finer grain alternative.

For each `(τ, ε)` tuple per checkpoint, the bin:

1. Counts gate-survivors using the recalibrated σ_train + the cell's τ
   (read from the existing recalibrated histogram body, NOT re-computed).
2. Runs the real-Binance backtest via the
   [`top10-2023-fy-tcn-overlay-realdata`](../backtest-real-binance-data/reports/backtest-20260519-074730-top10-2023-fy-tcn-overlay-realdata.md)
   (BS-1) /
   [`top10-2024-fy-tcn-overlay-realdata`](../backtest-real-binance-data/reports/backtest-20260519-074732-top10-2024-fy-tcn-overlay-realdata.md)
   (BS-2) scenario contracts, parameterised by the cell's (τ, ε).
3. Extracts Sharpe (ann), Sortino (ann), max drawdown, total return,
   final equity, trades, dampen rate.
4. Records the result row keyed by `(checkpoint, τ, ε)`.

Architect-decide whether to:

- **(a)** Wire (τ, ε) into the existing backtest CLI as new
  flags (`--tcn-tau`, `--tcn-epsilon`); the sweep bin shells out per
  cell. **Analyst-recommended default** — keeps the backtest path
  byte-identical at default values; predecessor 22 anchors stay
  byte-identical.
- **(b)** Add a new `with_tcn_bs{1,2}_ledger_tuned(τ, ε)` builder on
  `TcnOverlayMomentumStrategy` (additive, mirrors existing
  `with_tcn_bs{1,2}_ledger`); the sweep bin constructs the strategy in-
  process per cell. See Q5.

**Operator decision needed?** No — analyst defers the bin location +
backtest wiring to architect at M-T1. The CLI surface is whatever the
architect picks; the deliverable contract (R2) is what locks the
operator's experience.

### R2 — Report shape — `threshold-sweep-bs{1,2}-realdata-recalibrated-20260521.md`

Two new report files land under `spec/v25-tcn-threshold-tuning/reports/`:

- `threshold-sweep-bs1-realdata-recalibrated-20260521.md`
- `threshold-sweep-bs2-realdata-recalibrated-20260521.md`

Both follow the
[ADR-0033 § D2.a](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d2a--forecast-distribution-bs12-realdata-yyyymmdd-md)
frontmatter-vs-body discipline verbatim: run-varying fields in YAML
frontmatter (advisory only, NOT hashed); deterministic content in the
body (hashed by the anchor).

**Body structure** (each report):

1. **Checkpoint section** — `model_revision`, `sigma_train`
   (recalibrated value), recalibrated metadata overlay path
   (`tcn-bs{1,2}-<sha>.metadata.recalibrated.json`).
2. **Baseline reference** — Sharpe / Sortino / Calmar / drawdown / total
   return from the v1 momentum baseline AND the τ=0.6 + ε=0.0005
   shipped-default cell of THIS checkpoint (read from the predecessor's
   anchored `sharpe-comparison-realdata` body — NOT re-computed). Pins
   the comparison reference so the heatmap deltas are signed against
   the same baseline the operator already saw in the alpha-investigation
   ship.
3. **Heatmap A — Sharpe (ann) delta vs v1 momentum baseline** —
   9 × 5 grid (τ rows × ε cols). Each cell = `(Sharpe_cell -
   Sharpe_v1_baseline)`. Cells coloured monochrome by sign (cell text
   `+0.123` for positive, `-0.123` for negative; 6-decimal canonical).
4. **Heatmap B — Total return delta vs v1 momentum baseline** — same
   shape, cell = `(total_return_cell - total_return_v1_baseline)` as
   percentage.
5. **Heatmap C — Max drawdown** — same shape, cell =
   `max_drawdown_cell` (not a delta; absolute value, sign is always
   negative or zero). Surfaces "did some τ make things WORSE on
   drawdown."
6. **Heatmap D — Gate survivor count** — same shape, cell = number of
   bars surviving the gate at the given τ (independent of ε; ε filters
   the OVERLAY direction, not the GATE). 5 columns redundant; emitted
   as a 1-D row for completeness, then the 2-D heatmap uses the
   1-column-by-9-rows shape. Architect may collapse to 1-D at M-T1.
7. **Headline cell** — the single (τ, ε) cell that maximises Sharpe
   delta. If max Sharpe delta < +0.10 (analyst-recommended alpha
   threshold — see Q4), report "no alpha unlocked at any (τ, ε)."
8. **Verdict** — one of `T-ALPHA-UNLOCKED` (max Sharpe delta ≥ +0.10),
   `T-MARGINAL` (max Sharpe delta ∈ [0, +0.10)), `T-NO-ALPHA` (max
   Sharpe delta < 0). See Q4 for verdict-shape alternatives.

**Floating-point canonicalisation** (locked here to forestall K3 drift):

| Field family            | Format                            |
|-------------------------|-----------------------------------|
| `sigma_train`, ε, τ     | `format!("{:.6}", x)` (6 decimals)|
| Sharpe / Sortino / Calmar | `format!("{:.6}", x)` (6 decimals) |
| Return / drawdown / dampen rate | `format!("{:.2}%", x*100.0)` (2 decimals, %) |
| Trade counts            | `format!("{}", x)` (integer)      |
| Bar counts              | `format!("{}", x)` (integer)      |
| Gate-survivor counts    | `format!("{}", x)` (integer)      |

ASCII-only, LF-only line endings, fixed-precision floats — inherit the
ADR-0033 § D2.a canonicalisation contract.

**Operator decision needed?** No — analyst-locks report shape. Tester
confirms the 2-run byte-identity gate at M-FINAL.

### R3 — Verdict logic — `T-classifier` (advisory, NOT amending ADR-0033)

The 2-D Sharpe-delta heatmap collapses to a single **T-verdict label**
per checkpoint:

```
T-ALPHA-UNLOCKED — max over (τ, ε) of Sharpe_delta ≥ +0.10
T-MARGINAL       — max over (τ, ε) of Sharpe_delta ∈ [0.0, +0.10)
T-NO-ALPHA       — max over (τ, ε) of Sharpe_delta < 0.0
```

**Why +0.10 as the alpha threshold?** Annualised hourly-bar Sharpe lift
of +0.10 corresponds to ~+0.0001 hourly excess return at typical hourly
return std — far above per-bar transaction-cost noise (~0.001-0.002
per round-trip on liquid USDT pairs; the sweep should produce a
detectable signal-to-noise ratio at this magnitude). +0.10 is also
empirically the floor used in
[`v25-tcn-overlay/feature.md`](../v25-tcn-overlay/feature.md) success-
criterion language. See Q4 for alternative threshold defaults.

The **joint** verdict across BS-1 + BS-2 is the routing trigger:

| BS-1 verdict       | BS-2 verdict       | Joint               | Routing                                   |
|--------------------|--------------------|---------------------|-------------------------------------------|
| T-ALPHA-UNLOCKED   | T-ALPHA-UNLOCKED   | T-ALPHA-UNLOCKED    | Lock winning (τ, ε); ship as v2.5.1 cell  |
| T-ALPHA-UNLOCKED   | T-MARGINAL/T-NO-ALPHA | T-ALPHA-MIXED    | Analyst triage (BS-1 only)                |
| T-MARGINAL/T-NO-ALPHA | T-ALPHA-UNLOCKED | T-ALPHA-MIXED       | Analyst triage (BS-2 only)                |
| T-MARGINAL         | T-MARGINAL         | T-MARGINAL          | Operator-decide — ship as advisory, or queue retrain |
| T-NO-ALPHA         | T-NO-ALPHA         | T-NO-ALPHA          | Route to `v25-tcn-horizon-bump-or-retire` |
| any other mismatch | —                  | T-MIXED             | Analyst triage                            |

The T-classifier is **advisory** — it does NOT amend the immutable
ADR-0033 F-verdict (which stays F4 across this feature regardless of
T-label). See Q4 for whether to author a new ADR-0036 codifying the
T-classifier vs. embedding it in the report body only. Analyst default
in Q4 = (c) — embed in body, defer ADR-0036 until we have empirical
evidence of multiple unlocking cells (or zero unlocking cells, which
collapses the classifier need entirely).

**Operator decision needed?** Q4 — operator-decide whether to formalise
the T-classifier as ADR-0036 or leave it embedded in the report body.
Analyst default = (c).

### R4 — Lock-winner contract (additive only, no behavioural change)

If the joint T-verdict is `T-ALPHA-UNLOCKED`, the winning `(τ*, ε*)`
tuple per checkpoint is the headline finding. Analyst-recommended
contract for landing it without disturbing the 26 anchored bodies (see
§ Open questions § Q5):

- **Anchor strategy** — add two new "tuned" anchors under a new
  version `v2.6.2-threshold-tuning`:
  - `top10-2023-fy-tcn-overlay-realdata-tuned-bs1-{τ*,ε*}`
  - `top10-2024-fy-tcn-overlay-realdata-tuned-bs2-{τ*,ε*}`

  These are the Sharpe-winning backtest reports themselves (the
  reports the sweep emits at the winning cell, re-run anchored).
- **Strategy-builder shape** — add an ADDITIVE
  `with_tcn_bs{1,2}_ledger_tuned(τ, ε)` builder to
  `TcnOverlayMomentumStrategy` (mirrors
  [`with_tcn_bs1_ledger`](../../crates/strategy/src/tcn_overlay_momentum.rs#L413-L421)
  + [`with_tcn_bs2_ledger`](../../crates/strategy/src/tcn_overlay_momentum.rs#L431-L440)
  but takes `(τ, ε)` as args). The existing builders stay byte-identical
  (still `dec!(0.6)`). Existing callers see no behavioural change; the
  26 predecessor anchors stay byte-identical by construction.
- **No default change** — `τ = 0.6 + ε = 0.0005` stays the shipped
  default. Operator can flip the default in a follow-on v2.5.2 once
  the tuned cell has been live-traded long enough to confirm the
  Sharpe lift on out-of-sample data. NOT in this feature's scope.

If the joint T-verdict is `T-MARGINAL` or `T-NO-ALPHA`, NO builder is
added; the report itself is the deliverable; the operator routes to
`v25-tcn-horizon-bump-or-retire` (queued under § Strategy stub).

**Operator decision needed?** Q5 — operator-decide the lock-winner
contract shape. Analyst default = (c) additive tuned builder, no
default change.

### R5 — No retraining; no σ_train changes; no weight modification

Hard scope guard. This feature is **purely parameter-sweep** on top of
the recalibrated checkpoints. Read-only inputs:

- `crates/forecast/checkpoints/anchors/tcn-bs{1,2}-<sha>.metadata.json`
  — original metadata, byte-identical.
- `crates/forecast/checkpoints/anchors/tcn-bs{1,2}-<sha>.safetensors`
  — original weights, byte-identical.
- `crates/forecast/checkpoints/anchors/tcn-bs{1,2}-<sha>.metadata.recalibrated.json`
  — recalibrate ship's overlay metadata, byte-identical
  (re-consumed by the `--metadata-path` flag on
  [`forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs)).

No call site in this feature mutates any of these files. The sweep
operates on the `(τ, ε)` knobs of the OVERLAY layer
(`tcn_overlay_momentum.rs`), NOT the model layer (`tcn.rs`). Per ADR-0035
§ D4 the σ_train scalar is not in safetensors; per R7 of the recalibrate
feature the original `.metadata.json` files are not modified. All three
invariants carry forward.

**Operator decision needed?** No — analyst-locks. K6 enforces via the
read-only contract test (architect formalises at M-T1).

### R6 — Backtest cost — realdata path (not synthetic GBM)

The sweep uses the **v2.6.0-realdata** real-Binance backtest path (per
[ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)),
NOT the synthetic GBM path that the v2.5 / v2.6.0 anchors are pinned
to. Rationale:

- The recalibrate ship's F-verdict was on real-Binance data; the
  τ-sweep must compare against the same data distribution.
- The predecessor's `sharpe-comparison-realdata` baseline (BS-1 2023 FY +
  BS-2 2024 FY) is the reference point for the Sharpe delta in the
  heatmap. Both baselines are real-Binance.
- Synthetic GBM is a debugging-only path post-ADR-0032; the operator
  has already moved out of it.

Wall-clock budget: ~30s per backtest run (M3 backtest-real-binance-data
ballpark). Total: 9 τ × 5 ε × 2 checkpoints = 90 backtest runs.
Estimated 90 × 30s ≈ 45 min single-threaded. The architect may choose
to parallelise across cells at M-T1 (4-way local on the dev machine =
~12 min wall-clock). See § Cost estimate.

**Operator decision needed?** Q3 — operator-decide realdata-only or
realdata + synthetic-GBM sanity-check. Analyst default = (a) realdata
only.

### R7 — Anchor strategy (anchor-additive only)

This feature is **anchor-additive**. All 26 existing anchors stay
byte-identical:

- 22 pre-recalibrate anchors (R6 of recalibrate predecessor + 19
  pre-investigation anchors) — untouched.
- 4 v2.6.1-alpha-investigation-recalibrated anchors (2
  forecast-distribution + 2 derivation reports) — untouched. The σ_train
  fix evidence stays on disk.

New anchors land under a new version string `v2.6.2-threshold-tuning`:

- `threshold-sweep-bs1-realdata-recalibrated` — body-SHA of the BS-1
  sweep heatmap report.
- `threshold-sweep-bs2-realdata-recalibrated` — body-SHA of the BS-2
  sweep heatmap report.

If R4 fires (`T-ALPHA-UNLOCKED` joint verdict), TWO additional anchors
under the same version pin:

- `top10-2023-fy-tcn-overlay-realdata-tuned-bs1` — the winning
  cell's BS-1 backtest report body.
- `top10-2024-fy-tcn-overlay-realdata-tuned-bs2` — the winning
  cell's BS-2 backtest report body.

Anchor count progression:

- Pre-feature: 26 (recalibrate ship's lock).
- Post-feature (T-NO-ALPHA / T-MARGINAL): 28 (just the 2 sweep heatmaps).
- Post-feature (T-ALPHA-UNLOCKED): 30 (heatmaps + 2 tuned backtests).

`bash scripts/verify_anchors.sh` reports `ANCHORS PASS (28/28)` or
`ANCHORS PASS (30/30)` at M-FINAL, with all 26 pre-feature SHAs
byte-identical.

**Operator decision needed?** Q6 — operator-decide whether to anchor
the heatmaps eagerly or skip until we see the joint verdict (sweep
reports may be exploratory and replaced by tuned-winner reports as the
canonical anchor). Analyst default = (a) anchor heatmaps eagerly under
`v2.6.2-threshold-tuning`.

### R8 — Non-regression / anchor-neutrality contract

**Critical, load-bearing**: this feature MUST NOT touch the existing
26 anchors. Specifically:

- The existing `tcn-bs{1,2}-<sha>.safetensors` files are NOT modified.
- The existing `tcn-bs{1,2}-<sha>.metadata.json` files are NOT
  modified, renamed, or deleted.
- The existing `tcn-bs{1,2}-<sha>.metadata.recalibrated.json` overlay
  files are NOT modified, renamed, or deleted (recalibrate ship's R7
  invariant carries forward).
- The 22 pre-recalibrate anchors stay byte-identical.
- The 4 v2.6.1-alpha-investigation-recalibrated anchors stay
  byte-identical.
- No mutation of the existing `with_tcn_bs{1,2}_ledger` builders in
  `crates/strategy/src/tcn_overlay_momentum.rs:413-421` /
  `:431-440` — those keep `dec!(0.6)` byte-identical. The new
  `_tuned(τ, ε)` builder (R4) is ADDITIVE.

`bash scripts/verify_anchors.sh` must report `ANCHORS PASS (26/26)`
PRE-lock and `28/28` (or `30/30`) POST-lock; the 26 originals stay
byte-identical.

**Operator decision needed?** No — analyst-locks. Tester verifies at
M-FINAL.

### R9 — Determinism contract

The sweep is **deterministic**. Two sequential runs of `threshold_sweep`
against the same recalibrated checkpoints + same `data/binance/`
REVISION.toml SHA produce byte-identical heatmap report bodies — the
backtest path is already deterministic on a fixed seed
(see [ADR-0032 § D4](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)),
and the heatmap-rendering is a function over the deterministic
backtest outputs.

If the architect chooses parallelisation across cells (R6), the
parallelisation MUST be order-invariant — the final report assembles
cells by `(τ, ε)` key, NOT by completion order. Architect verifies at
M-T1; tester confirms via 2-run byte-identity gate at M-FINAL.

The existing `forecast_distribution_bin_readonly` 2-run gate from the
predecessor inherits forward via the recalibrated `--metadata-path`
toggle (no change to that path).

**Operator decision needed?** No — analyst-locks. Architect formalises
parallelisation ordering contract at M-T1.

## Hypothesis register (H1-H3)

> Each hypothesis is testable; the tester gate is what closes /
> falsifies it. Listed in dependency order.

### H1 — Some (τ, ε) tuple unlocks Sharpe delta ≥ +0.10 vs v1 baseline

**Statement.** Sweeping τ × ε over the recalibrated checkpoints'
overlay gate, at least one cell `(τ*, ε*)` (per checkpoint) produces a
Sharpe-annualised lift of at least +0.10 vs the v1 momentum baseline
(`top10-2023-fy-tcn-overlay-realdata` for BS-1 baseline equity curve;
`top10-2024-fy-tcn-overlay-realdata` for BS-2). Equivalently: the v2.5
TCN model carries directional signal that the current default
(τ=0.6 + ε=0.0005) was filtering inappropriately, AND the signal is
strong enough to overcome real-Binance transaction-cost noise.

**Test.** R1 produces the 9 × 5 heatmap per checkpoint. R3's T-classifier
returns `T-ALPHA-UNLOCKED` iff the max-cell Sharpe delta ≥ +0.10. If
both checkpoints return `T-ALPHA-UNLOCKED`, H1 is **fully confirmed**.
If one returns `T-ALPHA-UNLOCKED` and the other does not, H1 is
**partially confirmed** (T-ALPHA-MIXED — see § R3 routing table). If
neither does, H1 is **falsified**.

**Confidence at brief time**: MEDIUM. The recalibrate ship's gate-
survival jump (0% → 40-88%) is necessary-but-not-sufficient for alpha:
the surviving forecasts must also be directionally correct. The v2.5
TCN model's converged loss was `~1.5e-5` Huber on hourly log-returns,
which the predecessor flagged as "suspiciously tiny — possible
predict-≈zero collapse" — and the recalibrated `r_hat` distribution
mean is 0.0009 / 0.0014 (close to zero but not zero; small directional
bias toward up). Whether 0.0009 mean × directionally-correct sign is
enough to overcome ~0.001-0.002 round-trip transaction cost is the open
empirical question; the sweep answers it cheaply.

### H2 — The τ × ε surface is convex / interpretable

**Statement.** The 9 × 5 Sharpe-delta heatmap is **smooth** (not
ragged): the Sharpe-delta value at neighbouring cells differs by less
than the Sharpe-delta range / 4. Equivalently, the maximum is in the
interior of the grid (not at the corner), or — if at the corner — the
gradient pointing outward is small (the operator knows whether to bid
the grid wider). If the surface is ragged (max-cell adjacent to a
deep-negative cell), the operator should distrust the local max as a
real optimum and consider the sweep noisy / overfit.

**Test.** R1's body emits the full 9 × 5 grid; the tester's report
includes a smoothness statistic (max(|cell - neighbour|) / range over
all 8-connected neighbour pairs). If smoothness ≤ 0.25, H2 is
**confirmed**. If > 0.25, H2 is **falsified** and the operator routes
to analyst triage regardless of T-verdict (the sweep produced an
optimisation surface too noisy to trust on the 90-run budget).

**Confidence at brief time**: MEDIUM-HIGH. Sharpe is a robust statistic
over ~87,500 hourly bars (the BS-1 / BS-2 spans). Small changes in
(τ, ε) gates fractions of bars on and off the overlay; the equity-curve
response should be roughly continuous in (τ, ε). Hourly-bar
transaction-cost noise on the ~6,000 trades-per-year scale is bounded;
Sharpe-delta noise should be at most ~±0.02-0.05 per cell. A 9 × 5 grid
with cell variance ~±0.05 should yield a smoothness statistic well
below 0.25. The risk is path-dependent equity-curve drift (e.g. a
short bull-run reversal in March 2024 catches some cells and not
others) which the sweep cannot detect; H2 is the guard against that.

### H3 — The cheap sweep finds alpha before horizon-bump becomes necessary

**Statement.** EITHER H1 fires (some cell unlocks +0.10 Sharpe; the
v2.5 TCN is salvaged without retraining) OR H1 falsifies (no cell
unlocks alpha; the v2.5 TCN at 1h horizon genuinely lacks signal and
the multi-week `v25-tcn-horizon-bump-or-retire` is the right next
step). Either way, the cheap sweep provides actionable signal in
~hours, not weeks — which the predecessor presenter deck's option (c)
routing requires.

**Test.** Implicit in the joint verdict outcome. The threshold-tuning
sweep produces either `T-ALPHA-UNLOCKED` (H3 confirmed; ship the
winning cell + queue v2.5.2 default-flip) or `T-NO-ALPHA` / `T-MARGINAL`
(H3 confirmed; sweep proved gate-tuning insufficient, route to
horizon-bump-or-retire with clean signal).

**Confidence at brief time**: HIGH. This is a tautology in the design
of the sweep — regardless of outcome, the operator gets a clean
routing decision in ~hours. The only failure mode H3 cannot survive is
H2 falsification (ragged surface = sweep is uninterpretable); H2 is
the upstream guard.

## Risk register (K1-K6)

| Risk | Mitigation |
|------|------------|
| **K1 — No cell unlocks alpha; all Sharpe deltas < 0.** H1 falsified; sweep proves the model has no signal at any (τ, ε). | Per H3, this IS the routing signal the operator needs to fund horizon-bump-or-retire with confidence. K1 is not a feature failure — it's an experimental outcome. The `v25-tcn-horizon-bump-or-retire` stub under § Strategy queues the fallback. Wall-clock budget is bounded (hours, not weeks), so worst-case cost is low. |
| **K2 — Heatmap is ragged (H2 falsified); sweep is noisy / overfit.** Possible if the 9 × 5 grid is too coarse, or if the 87,500-bar span has too much path dependence for Sharpe to converge per cell. | Analyst defers to architect at M-T1 whether to expand the grid (finer τ step from 0.1 to 0.05; finer ε step from baseline to log-uniform). H2's smoothness statistic is the tripwire; if it fires, operator routes to analyst triage (not horizon-bump). |
| **K3 — Determinism gate fails on a 2-run check of the sweep heatmap.** Possible if cell ordering or per-cell timestamp leaks into the body. | The body is a function over the deterministic backtest outputs. Architect formalises ordering contract at M-T1 (cells sorted by (τ, ε) key, never by completion). Existing backtest determinism gate from ADR-0032 § D4 carries forward. Tester confirms at M-FINAL via 2-run byte-identity. |
| **K4 — The new `_tuned(τ, ε)` builder accidentally flips the existing `with_tcn_bs{1,2}_ledger` defaults.** Existing builders use `dec!(0.6)`; a developer might be tempted to refactor the default into a constant and reuse it. | Hard analyst boundary: the `_tuned` builder is **purely additive** to the impl block. The existing builders' bodies stay byte-identical (still `dec!(0.6)` literal). The 26 predecessor anchors are the load-bearing invariant; a unit test at M-T1 asserts byte-identity of the predecessor backtest reports under default-builder invocation. |
| **K5 — Scope creep into retraining or weight modification.** The "alpha is just under the surface" hypothesis might tempt the developer to also bump the model's horizon "while we're in here." | Hard analyst boundary: this feature touches **only** the (τ, ε) knobs of the overlay layer (`tcn_overlay_momentum.rs`), NOT the model layer (`tcn.rs`) or training scaffold (`train_tcn.rs`). The `v25-tcn-horizon-bump-or-retire` follow-on is a separate feature, separate spec folder, separate spawn. Architect codifies as a "no train / no weights / no metadata mutation" unit test at M-T1. |
| **K6 — Backtest path divergence from the predecessor's anchored body.** The sweep parameterises `--tcn-tau` + `--tcn-epsilon` on the backtest CLI (Q1=(a)); a default invocation (no flags) MUST produce byte-identical output to the predecessor's `top10-2023-fy-tcn-overlay-realdata` anchor body. | Architect-decide at M-T1 whether to wire flags on the backtest CLI or in-process via the strategy builder. Either way, the default behaviour test (NO flag = identical to predecessor anchor body) is the load-bearing CI check. Tester verifies at M-FINAL via direct hash. |

## Non-regression contract

This section consolidates the load-bearing invariants that the tester
gate confirms at M-FINAL:

1. **26 anchored body-SHAs byte-identical.** `bash scripts/verify_anchors.sh`
   reports `26/26` PRE-lock and `28/28` (or `30/30`) POST-lock. All 26
   originals stay byte-identical.
2. **Original `.metadata.json` + `.safetensors` files byte-identical.**
   `git diff HEAD -- crates/forecast/checkpoints/anchors/*.metadata.json
   crates/forecast/checkpoints/anchors/*.safetensors` is empty
   (recalibrate ship's R7 invariant carries forward).
3. **`tcn-bs{1,2}-<sha>.metadata.recalibrated.json` overlay files
   byte-identical.** Same diff covers them; this feature does not write
   to anchor-checkpoint directories.
4. **Default-builder behaviour byte-identical.** A backtest invoked
   without `--tcn-tau` / `--tcn-epsilon` flags (or invoked in-process
   via `with_tcn_bs{1,2}_ledger`, NOT the new `_tuned` variant)
   produces a body byte-identical to the predecessor
   `top10-2023-fy-tcn-overlay-realdata` /
   `top10-2024-fy-tcn-overlay-realdata` anchor bodies.
5. **No new external crate dependencies.** Workspace `Cargo.toml`
   diff is limited to existing crates (per CLAUDE.md "no new external
   crate deps" constraint).
6. **No iced bump.** Operator-locked per CLAUDE.md.
7. **Read-only against the 4 recalibrated-overlay anchors.** The sweep
   reads `forecast-distribution-bs{1,2}-realdata-recalibrated` +
   `recalibrate-sigma-train-bs{1,2}` bodies for the σ_train value and
   for the per-τ gate-survivor counts; the sweep does NOT re-emit any
   of those reports.
8. **Spec-lint baseline 87/2 maintained.** This feature should add 0
   new lint categories. The spec-lint output should match the
   recalibrate-ship baseline (potentially +0..N file-counts on
   existing categories from new spec files).

## Acceptance per milestone

The feature is **done** when all milestones land their gates:

### M-T1 — Architect lock (Q1-Q6 resolved, design + decomp.md complete)

1. `feature.md § Design` block appended (between § Out of scope and §
   Changelog).
2. `spec/v25-tcn-threshold-tuning/decomp.md` complete with T-D / T-T
   row decomposition.
3. ADR-0036 EITHER written (Q4 = (b)) OR explicitly skipped (Q4 =
   (a) or (c)). If skipped, a one-line rationale lives in feature.md
   § Design.
4. Frontmatter flips `status: draft → in-progress`, `owner: analyst →
   architect → developer`.

### M-D — Developer wave (sweep bin + heatmap reports + tuned builder if R4 fires)

1. `crates/forecast/src/bin/threshold_sweep.rs` (or extension to existing
   bins per architect M-T1) compiles + passes clippy under
   `--features candle`.
2. `crates/strategy/src/tcn_overlay_momentum.rs` gains
   `with_tcn_bs{1,2}_ledger_tuned(τ, ε)` (additive only; existing
   builders byte-identical).
3. 2 new heatmap reports on disk under
   `spec/v25-tcn-threshold-tuning/reports/`.
4. If R4 fires: 2 new tuned backtest reports + the additive builder
   wiring lands.
5. 1-3 new integration tests for the sweep bin (readonly + determinism
   + builder-default-invariance) — architect decomposes at M-T1.

### M-FINAL — Tester gate (anchor lock + non-regression + verdict)

1. `cargo fmt --check` + `cargo clippy --workspace -- -D warnings` PASS.
2. `cargo clippy -p forecast --features candle -- -D warnings` PASS.
3. `cargo clippy -p strategy --features forecast,forecast-audit-tick --
   -D warnings` PASS.
4. `cargo test --workspace --lib` PASS, 0 failures.
5. New integration tests PASS.
6. `bash scripts/verify_anchors.sh` reports `26/26` PRE + `28/28`
   (or `30/30` if R4 fires) POST.
7. 2-run byte-identity determinism gate on the new heatmap reports.
8. `uv run scripts/spec_lint.py` matches the 87/2 baseline (0 new
   categories).
9. Joint T-verdict recorded in `feature.md § Verification`.

### M-PRESENTER — Operator approval

1. Presenter deck under
   `spec/v25-tcn-threshold-tuning/presentations/v25-tcn-threshold-tuning-<YYYY-MM-DD>.md`
   carrying the joint T-verdict + the headline (τ*, ε*) cell + the
   recommended next routing.
2. Operator ticks approval. Frontmatter flips `status: in-progress →
   shipped`.
3. Trace row `REQ-V25-TCN-THRESHOLD-TUNING-001` flips `draft →
   shipped`.
4. Backlog entry moved Active → Recent.

## Open questions (Q1-Q6 — operator-decide)

> Standing operator directive is "autoapprove all" — but the analyst's
> job per AGENT.md is to surface Qs first. Each Q carries an analyst-
> recommended default; if the operator says "autoapprove," the defaults
> ship.

### Q1

**τ grid resolution.** Three candidates:

- **(a)** 9 cells, integer-tenths: `{0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7,
  0.8, 0.9}`. Matches the existing
  [`confidence_gate_survival` array](../../crates/forecast/src/bin/forecast_distribution.rs#L325)
  in `forecast_distribution.rs` — the gate-survivor counts per τ are
  already in the predecessor's recalibrated report bodies, so the
  sweep's gate-survivor heatmap row is reusable without re-pass.
  **Analyst-recommended default.**
- **(b)** 17 cells, half-tenths: `{0.1, 0.15, 0.20, …, 0.85, 0.90}`.
  Twice the resolution; doubles the backtest run count (170 vs 90); the
  in-between τ values would require a re-pass of `forecast_distribution`
  to populate the gate-survivor row.
- **(c)** Log-spaced: `{0.05, 0.1, 0.2, 0.4, 0.6, 0.8}` (6 cells, tighter
  near zero where the recalibrated gate is now permissive). Fewer cells,
  but a different shape that's harder to compare to existing
  `confidence_gate_survival` rows.

**Analyst default: (a)** — 9 integer-tenths. 9 × 5 = 45 cells per
checkpoint × 2 checkpoints = 90 backtest runs total. The existing
`confidence_gate_survival` row in the predecessor's body gives the
gate-survivor heatmap row for free. Wall-clock estimate: 45 min single-
threaded; 12 min on 4-way local parallelism.

### Q2

**ε grid resolution.** Three candidates:

- **(a)** 5 cells, integer-thousandths with a fine bottom:
  `{0.0001, 0.0005 baseline, 0.001, 0.005, 0.01}`. Covers the
  magnitude range of inference-time `r_hat` std (~0.01-0.02). Baseline
  in the middle. **Analyst-recommended default.**
- **(b)** 7 cells, log-uniform: `{0.00005, 0.0001, 0.0002, 0.0005,
  0.001, 0.002, 0.005}` (truncated at 0.005 because larger values
  silence too much of the model). More resolution at the small end.
- **(c)** 3 cells, coarse: `{0.0001, 0.0005, 0.001}`. Cheap (54 backtest
  runs total); risks missing the optimum if it's well outside the
  baseline.

**Analyst default: (a)** — 5 ε cells covering 2 orders of magnitude.
The baseline is in the middle, which makes the heatmap easy to read.

### Q3

**Backtest mode.** Two candidates:

- **(a)** Real-Binance `-realdata` path only (per ADR-0032). 90
  backtest runs. **Analyst-recommended default** — the recalibrate
  ship's F-verdict was on real-Binance data, so the sweep must compare
  against the same distribution.
- **(b)** Real-Binance + synthetic GBM sanity check. 180 backtest runs.
  Double the cost; the operator gets a "does the optimum transfer
  across distributions" sanity check. Risk: synthetic GBM was the
  M3-falsified path; operator moved out of it.

**Analyst default: (a)** — realdata only. The sweep should compare
apples-to-apples against the recalibrate ship's recalibrated reports.

### Q4

**Verdict shape — `T-classifier` codification.** Three candidates:

- **(a)** Extend ADR-0033's F-verdict to add an `F5` branch
  (`F5 — alpha-unlocked-at-τ-tuned`). Risk: ADR-0033 has an explicit
  immutability clause ("This ADR does not amend its own thresholds —
  superseding ADR required"). Q4 = (a) thus requires writing a
  superseding ADR-0036 anyway.
- **(b)** Write a new ADR-0036 codifying the T-classifier as an
  independent advisory verdict alongside the immutable F-verdict.
  Defensible but architecturally heavyweight — the T-classifier only
  has 3 labels (`T-ALPHA-UNLOCKED` / `T-MARGINAL` / `T-NO-ALPHA`) and
  may collapse to "always T-NO-ALPHA" if H1 falsifies.
- **(c)** Embed the T-classifier in the report body only (no ADR;
  defer ADR-0036 until we have empirical evidence of multiple
  unlocking cells across multiple feature spawns). Cheapest; the
  operator-routing logic lives in the report body's `## Verdict`
  section per checkpoint, mirroring how the F-verdict already lives
  in the forecast-distribution bodies. **Analyst-recommended default.**

**Analyst default: (c)** — embed in report body, defer ADR-0036.
Rationale: if H1 falsifies (joint T-NO-ALPHA), the classifier is
single-label and an ADR codifying it is overkill. If H1 fires, a
follow-on v2.5.1 feature can ADR-codify the T-classifier at that
point (with empirical evidence in hand). The cheap path is
information-preserving.

### Q5

**Lock-winner contract.** Three candidates:

- **(a)** Update the `with_tcn_bs{1,2}_ledger` builders to use the
  winning `(τ*, ε*)` as the default. Risk: flips the 26 predecessor
  anchors immediately (the `top10-2023-fy-tcn-overlay-realdata` body
  would no longer match the locked anchor SHA). **Analyst rejects.**
- **(b)** Leave builders untouched; the operator commits to a retrain
  follow-on later. The winning cell is documented in the heatmap report
  body only; no code change ships. Minimum risk, minimum value.
- **(c)** Add an ADDITIVE `with_tcn_bs{1,2}_ledger_tuned(τ, ε)` builder
  alongside the existing builders. The existing builders stay
  byte-identical (still `dec!(0.6)`); new callers can opt into the
  tuned variant. **Analyst-recommended default.**

**Analyst default: (c)** — additive tuned builder. No behavioural
change for existing callers; 26 predecessor anchors stay byte-identical
by construction; the winning cell becomes available to future callers
without an immediate default flip. A follow-on v2.5.2 can promote the
tuned default after live-trading validation.

### Q6

**Anchor strategy.** Two candidates:

- **(a)** New anchor names `threshold-sweep-bs{1,2}-realdata-recalibrated`
  under new version string `v2.6.2-threshold-tuning`. Lock the heatmap
  reports eagerly at M-FINAL. If R4 fires, also lock the tuned
  backtest reports. 26 originals stay byte-identical.
  **Analyst-recommended default.**
- **(b)** Skip anchoring the heatmap reports (treat them as exploratory
  / replaceable). Only anchor the tuned backtest reports if R4 fires.
  Risk: heatmaps may be re-run in follow-on features and the
  byte-history isn't preserved.

**Analyst default: (a)** — anchor heatmaps eagerly. The heatmaps are
the load-bearing operator-routing artefact; anchoring them creates a
durable byte-history for cross-feature comparison (e.g. if a future
ADR adjusts the (τ, ε) grid resolution, the heatmap-at-grid-resolution-X
vs heatmap-at-grid-resolution-Y is a one-line `diff -u` between
anchored bodies).

## Cost estimate

| Step | Wall-clock | Owner |
|------|------------|-------|
| Author this brief (R1-R9 + H1-H3 + K1-K6 + Q1-Q6) | 30 min | analyst (this brief) |
| Architect lock + decomp.md + (optional) ADR-0036 (Q4) | 1-2 hr | architect |
| Implement `threshold_sweep` bin + heatmap renderer (R1 + R2) | 2-3 hr | developer |
| Add `_tuned(τ, ε)` builder (R4) | 30 min | developer |
| Run 90 backtests on realdata (R6) | 45 min single-threaded; 12 min 4-way local | orchestrator |
| Tester gate (R7 + R8 + R9) | 30 min | tester |
| Presenter deck | 30 min | presenter |
| **Total** | **~6–10 hours wall-clock** | |

Compared to `v25-tcn-horizon-bump-or-retire` (multi-week retrain on
Metal: ~2-3 weeks per the predecessor's analyst notes), this is
**1-2 orders of magnitude cheaper**. The presenter deck's "hours, not
weeks" framing from the recalibrate ship carries forward.

## Out of scope

- **No retraining.** Weights stay byte-identical. If H1 falsifies, the
  follow-on is `v25-tcn-horizon-bump-or-retire` (separate spec).
- **No σ_train change.** Recalibrated metadata overlay
  (`tcn-bs{1,2}-<sha>.metadata.recalibrated.json`) is read-only input
  to this feature.
- **No safetensors edit.** Same as recalibrate ship's R7 invariant.
- **No mutation of the existing 26 anchors.** R7 + R8 are the load-
  bearing non-regression contract.
- **No ADR-0033 amendment from within this feature.** F-verdict stays
  F4 regardless of T-verdict (operator routes on T-verdict, not on
  F-verdict, when the σ_train-fix-but-F4 condition holds — per
  Q4 = (c)).
- **No default `(τ, ε)` flip.** Even if R4 fires, the shipped default
  stays `τ=0.6 + ε=0.0005`; the additive `_tuned` builder is the
  opt-in path. A future v2.5.2 feature can promote the default after
  live-trading validation.
- **No horizon change.** 1h forecast horizon stays at the
  `v25-tcn-overlay` default. The horizon-bump fallback is a separate
  follow-on.
- **No PatchTST / Transformer comparison.** That's v2.6 bake-off
  territory.
- **No new external crate deps.** Per CLAUDE.md.

## Sources cited

- [`spec/v25-tcn-recalibrate/feature.md`](../v25-tcn-recalibrate/feature.md)
  — predecessor feature brief (v0.1.0, shipped); R1-R8 + H1-H3 + K1-K5
  + Q1-Q5 with analyst defaults; § Verification records joint F4 +
  gate-survival jump.
- [`spec/v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md`](../v25-tcn-recalibrate/presentations/v25-tcn-recalibrate-2026-05-21.md)
  — predecessor presenter deck; routing option (c) chosen by operator;
  τ-sweep cheap-first sequencing recommended.
- [`spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md`](../v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md),
  [`reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md`](../v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md)
  — recalibrated F-verdict reports; carry the σ_train value + the
  `confidence_gate_survival` array per checkpoint that the sweep
  reads as input.
- [`spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs1-20260521.md`](../v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs1-20260521.md),
  [`reports/recalibrate-sigma-train-bs2-20260521.md`](../v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs2-20260521.md)
  — wire-format diff + field-invariance derivation reports.
- [`spec/v25-tcn-alpha-investigation/feature.md`](../v25-tcn-alpha-investigation/feature.md)
  — original F4-investigation brief; 4-bucket framing of (a-d) and the
  R4 follow-on routing table.
- [`spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md`](../v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md)
  — Sharpe baseline (the v1 momentum + dampened=0 TCN-overlay reference
  the sweep's heatmap deltas are computed against).
- [`spec/v25-tcn-overlay/feature.md`](../v25-tcn-overlay/feature.md)
  § R6, § D5 — canonical ε / τ definitions; the (τ, ε) knobs this
  feature sweeps.
- [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md) —
  TCN checkpoint provenance; metadata canonicaliser.
- [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
  — backtest realdata path + revision pin; the determinism gate this
  feature inherits.
- [ADR-0033](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md)
  § D3 — F-verdict algorithm (IMMUTABLE across this feature per Q4 =
  (c)); § D2.a — report shape canonicalisation this feature inherits.
- [ADR-0035](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
  — post-training σ_train recalibration; D4 σ_train-not-in-safetensors
  invariant (still holds).
- `crates/forecast/src/bin/forecast_distribution.rs:325` — existing
  `confidence_gate_survival` array sweep over τ ∈ {0.1..0.9}; the
  τ-sweep heatmap row reuses this.
- `crates/forecast/src/bin/sharpe_comparison.rs` — Sharpe / Sortino /
  Calmar / drawdown computation; the per-cell Sharpe in the heatmap
  reuses the formula.
- `crates/strategy/src/tcn_overlay_momentum.rs:413-421,431-440` — the
  shipped `with_tcn_bs1_ledger` + `with_tcn_bs2_ledger` builders;
  passing `dec!(0.6)` to `Self::new` is the τ-default the additive
  `_tuned` builder replaces with a CLI-driven arg.
- `crates/strategy/src/overlay.rs` — `combine()` semantics for the
  deadband ε (the zero-band where the overlay is silent).
- `crates/strategy/src/tcn_overlay_momentum.rs:~145-170` (architect
  confirms exact line range at M-T1) — the `combine_with_direction`
  gate body that consumes `confidence_threshold` + ε.

## Changelog

- 2026-05-21 (analyst): initial brief authored. R1-R9, H1-H3, K1-K6,
  Q1-Q6 with analyst-recommended defaults. Predecessor:
  `v25-tcn-recalibrate v0.1.0`. Parent: `v25-tcn-overlay v2.5.0
  (in-progress)`. Trace row `REQ-V25-TCN-THRESHOLD-TUNING-001` opened
  in `draft` state. Diagnostic finding carried forward from the
  recalibrate ship: gate-survival jump 0% → 40-89% under recalibrated
  σ_train is necessary-but-not-sufficient for alpha; the τ × ε sweep
  is the cheap empirical answer. Cost estimate: ~6-10 hours wall-clock;
  analyst-recommended scope confirmed feasible vs the multi-week
  `v25-tcn-horizon-bump-or-retire` alternative. Stub for the fallback
  feature added to `spec/backlog.md § Strategy`. HANDOFF →
  operator-decide (Q1-Q6) → architect.
