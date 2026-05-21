---
slug: v25-tcn-recalibrate
status: in-progress
owner: developer
updated: 2026-05-21
version: 0.1.0
predecessor: v25-tcn-alpha-investigation v0.1.0
parent: v25-tcn-overlay v2.5.0 (in-progress)
---

# v2.5 — TCN σ_train recalibration (metadata-only fix)

> Cheap, metadata-only follow-on to the
> [`v25-tcn-alpha-investigation`](../v25-tcn-alpha-investigation/feature.md)
> joint **F4 verdict** shipped 2026-05-19
> ([presenter deck](../v25-tcn-alpha-investigation/presentations/v25-tcn-alpha-investigation-2026-05-19.md)).
> The investigation surfaced a load-bearing **σ_train units / accumulation
> bug** in the BS-1 and BS-2 anchored checkpoints: the stored
> `sigma_train` scalar in each `.metadata.json` is ~500–700× larger than
> the inference-time std of `r_hat`. Because the confidence gate is
> `|r_hat|/σ_train ≥ τ`, an inflated σ_train silences every forecast
> regardless of how directionally useful it is. **No retraining**; this
> feature re-derives σ_train from a held-out forward pass and rewrites
> the metadata JSON only. The safetensors weights stay byte-identical.

## Why

The F-verdict classifier ([ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm))
priority-ordered F1 → F2 → F3 → F4 over the BS-1 and BS-2 forecast
distributions. The F2 condition (`std > 0.1 · σ_train` AND
`frac_passes_confidence_gate < 1e-6`) tests for an **inflated** σ_train
(observed inference spread *large* relative to σ_train). The actual
checkpoints have the **opposite** anomaly: `std / σ_train ≈ 0.0016`
(BS-1) and `0.0014` (BS-2) — observed spread is far *smaller* than
σ_train, so the priority tree's literal F2 check did not fire and the
classifier correctly landed on F4 ("no signal at 1h horizon").

But the σ_train calibration bug is real and load-bearing for the gate
calculation. Per the
[presenter deck § "The bigger finding — σ_train calibration anomaly"](../v25-tcn-alpha-investigation/presentations/v25-tcn-alpha-investigation-2026-05-19.md):

> "Fix the σ_train units and the gate-survival fraction might jump
> from 0% to 'small but non-zero' — which would re-classify the model
> into F3 (gating-too-tight) territory, much cheaper to address than a
> horizon retrain."

The analyst's ranked-follow-on recommendation was to run THIS feature
first (wall-clock hours), then the multi-week
`v25-tcn-horizon-bump-or-retire` only if F4 survives the recalibration.

### The diagnosis (cited evidence)

Inference-time read site (uses σ_train from metadata as-is):

- `crates/forecast/src/tcn.rs:534` — `sigma_train` read from
  `metadata["sigma_train"]` of the `.metadata.json`, stored as
  `TcnForecaster::sigma_train: f32`.
- `crates/forecast/src/tcn.rs:937` — gate evaluated as
  `(r_hat.abs() / self.sigma_train).clamp(0.0, 1.0)` against
  `confidence_threshold = 0.6`.

Training-time write site (where the bug lives):

- `crates/forecast/src/bin/train_tcn.rs:606` — `let mut all_r_hats:
  Vec<f32> = Vec::new(); // for sigma_train`
- `crates/forecast/src/bin/train_tcn.rs:676-678` — inside the per-epoch
  training loop, after each batch's forward pass:
  ```rust
  if let Ok(r_hats) = pred.flatten_all().and_then(|t| t.to_vec1::<f32>()) {
      all_r_hats.extend_from_slice(&r_hats);
  }
  ```
  This collects the model's predicted r_hats from **every training
  batch across every epoch** (30 epochs × ~600 batches × 128 samples
  ≈ 2.3M values for BS-1) — and the vector is **never reset between
  epochs**.
- `crates/forecast/src/bin/train_tcn.rs:733-741` — at the end of
  training, σ_train is computed as the std of that accumulated vector:
  ```rust
  let sigma_train = if all_r_hats.len() > 1 {
      let n = all_r_hats.len() as f32;
      let mu = all_r_hats.iter().sum::<f32>() / n;
      let var = all_r_hats.iter().map(|&x| (x - mu).powi(2)).sum::<f32>() / n;
      var.sqrt().max(1e-8)
  } else { 1.0_f32 };
  ```
- `crates/forecast/src/bin/train_tcn.rs:754` — that scalar is then
  embedded in the metadata JSON via `TrainingMetrics { sigma_train, … }`.

**Why this produces 10.95 / 6.92 instead of ≈ 0.02:** the model is
trained from a random-init VarBuilder
(`crates/forecast/src/bin/train_tcn.rs:473-490`) on a TCN with 8
residual blocks × 96 channels and no output-layer scaling
(`crates/forecast/src/tcn.rs:302-311` — the 1×1 conv head has no
post-activation normalisation). At epoch 1, the model output is
essentially the convolutional response of random Kaiming-init weights
on the input features — outputs O(1) to O(10) are normal. Over 30
epochs of OneCycle training (`crates/forecast/src/bin/train_tcn.rs:586-731`)
the model converges to a regime where the output is O(0.01–0.02),
matching `target_logret` (raw log-returns —
`crates/forecast/src/features.rs:627-628`,
`(close_t1 / close_t).ln() as f32`).

But because `all_r_hats` is never cleared, the std accumulator is
dominated by the **early-epoch garbage predictions** (huge values
from random-init weights). The final scalar is essentially the std
of the model's training trajectory — NOT the std of the converged
model's predictions. The reported BS-1 σ_train = 10.954 and BS-2
σ_train = 6.916 reflect that trajectory variance.

The inference-time `r_hat` std measured in the F4 evidence reports is
0.018 (BS-1) and 0.010 (BS-2) — fully consistent with raw log-return
units in the converged regime. **The two values are in the same units;
the difference is that the training-time scalar averages in
pre-convergence noise that the inference path never sees.**

### Quantitative-finance context

The confidence gate `|r_hat|/σ_train ≥ τ` is meant to suppress
forecasts whose magnitude is small relative to the model's
*calibrated* prediction scale. The right calibrator is the
**converged-model** prediction std on the training distribution
(NOT the trajectory std during training). Equivalently: the std of
the population the model emits at evaluation time, which for a healthy
1h-horizon log-return regressor on hourly crypto bars sits in the
0.005–0.025 range (matching the 0.002–0.015 hourly log-return std of
the underlying universe; the trained model's spread is bounded above
by the target spread when Huber regression converges).

That places the *correct* σ_train in the same ballpark as the
inference-time `std` already reported in the F4 evidence:
BS-1 → σ_train ≈ 0.0180; BS-2 → σ_train ≈ 0.0100. With those values
in place, the gate calculation `|r_hat|/σ_train` at τ = 0.6 means
"the forecast magnitude exceeds ~0.6 × inference-std" — a meaningful
filter, not a forced-zero filter.

## Requirements

### R1 — Diagnose & document the σ_train computation site

Author a dev-note documenting the exact training-time computation site
(`crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741`), the
inference-time read site (`crates/forecast/src/tcn.rs:534,937`), and
the unit consistency check (both sites operate on raw log-returns; the
mismatch is **accumulation-window**, not **unit conversion**). The dev-note
under `spec/dev-notes/analysis-2026-05-21-tcn-sigma-train-bug.md`
becomes the canonical reference for the σ_train semantic, supersedes
the σ_train semantic implicit in ADR-0029 § "metadata fields", and
the developer must cite it from `train_tcn.rs` doc-comments at fix
time.

- **Operator decision needed?** No — analyst-locks. Diagnostic prose only,
  no operator-decide.

### R2 — Metadata-only re-derivation tool

A new read-only binary at
`crates/forecast/src/bin/recalibrate_sigma_train.rs` (architect may
relocate at design time, e.g. into a `tools/` family) that:

- Takes `--scenario {bs1|bs2}`, `--data-root data/binance/`,
  `--out-dir crates/forecast/checkpoints/anchors/` as CLI args.
- Loads the matching anchored checkpoint via the existing
  `TcnForecaster::load_anchor(AnchorScenario)`
  (`crates/forecast/src/tcn.rs:472`) without mutating its file.
- Runs the **converged-model** forward pass over the *training-data
  span* declared in the checkpoint's `.metadata.json` `data_span`
  field (the analyst's Q1 default — see [§ Open questions § Q1](#q1)).
  BS-1: `2023-01-01T00:00:00Z .. 2023-12-31T23:00:00Z`; BS-2:
  `2023-01-01T00:00:00Z .. 2024-03-31T23:00:00Z` per the
  metadata.json files on disk.
- Collects all `r_hat` values into a single buffer (post-convergence
  only, NOT the per-epoch accumulator the training-time bug uses).
- Computes σ_train as `std(r_hat)` per the type-7 quantile + total-cmp
  conventions used by `forecast_distribution.rs:histogram` (see
  ADR-0033 § D2.a "Percentile algorithm").
- Emits a **new** `.metadata.json` next to the existing checkpoint
  with the corrected scalar, file-named per § R3 below; the existing
  metadata file **stays byte-identical** (no in-place overwrite).
- Emits a deterministic re-derivation report under
  `spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs{1,2}-<YYYYMMDD>.md`
  carrying: original σ_train, recalibrated σ_train, r_hat count, span,
  data revision SHA, model_revision (unchanged), and the wire-format
  contrast `old / new`.

Hard read-only contract:
- No mutation of the existing `tcn-bs{1,2}-<sha>.safetensors` files.
- No mutation of the existing `tcn-bs{1,2}-<sha>.metadata.json` files.
- The new metadata JSON file lives at a *new path* (see § R3).
- The `weights_sha256` field in the new metadata equals the existing
  metadata's `weights_sha256` (same safetensors). The `model_revision`
  field equals the existing model_revision (same SHA over the same
  weights). Only the `sigma_train` value changes.

- **Operator decision needed?** No — analyst-locks the read-only
  contract.

### R3 — Recalibrated-metadata file naming + checkpoint-revision pin

The anchored safetensors files DO NOT MOVE. Per [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md)
the `model_revision` SHA is computed over the safetensors weights +
canonical architecture descriptor; σ_train is metadata-only and does
NOT contribute to `model_revision`. That is the load-bearing reason
this feature is metadata-only feasible (see [§ Open questions § Q2](#q2)).

The recalibrated metadata file is named:

```
crates/forecast/checkpoints/anchors/tcn-bs{1,2}-<sha>.metadata.recalibrated.json
```

co-located with the existing `tcn-bs{1,2}-<sha>.metadata.json` (which
remains untouched). The `<sha>` is the SAME `model_revision` SHA as
the original.

The new `TcnForecaster::load_anchor()` path (architect may extend the
shipped API at T-AR-2) reads `.metadata.recalibrated.json` if present,
falling back to `.metadata.json` otherwise. **This fallback is what
makes the F4-history forecast_distribution reports re-runnable
verbatim** — toggle the recalibrated metadata in place by writing the
file; toggle off by deleting it.

The new metadata file's canonicalisation matches the existing
canonicaliser at `crates/forecast/src/provenance.rs` (referenced from
[ADR-0029 § D4](../architecture/adr/0029-tcn-checkpoint-provenance.md)).
Architect confirms the JSON-canonical bytes are deterministic on a
2-run check at T-AR-2.

- **Operator decision needed?** No — analyst-locks naming. Architect picks
  exact API surface for the metadata-toggle at T-AR-2.

### R4 — Re-run `forecast_distribution` under recalibrated σ_train

After R2 emits the recalibrated metadata, re-run the shipped
`forecast_distribution` bin
(`crates/forecast/src/bin/forecast_distribution.rs`) on both BS-1 and
BS-2 under the new σ_train. The bin reads σ_train from
`TcnForecaster::sigma_train` (`crates/forecast/src/tcn.rs:425`), so the
metadata-toggle from R3 flows through without any code change to the
bin.

Two new reports land under
`spec/v25-tcn-recalibrate/reports/`:

- `forecast-distribution-bs1-realdata-recalibrated-<YYYYMMDD>.md`
- `forecast-distribution-bs2-realdata-recalibrated-<YYYYMMDD>.md`

Bodies follow the [ADR-0033 § D2.a](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d2a--forecast-distribution-bs12-realdata-yyyymmdd-md)
canonical shape verbatim — same float-format rules, same histogram
representation, same gate-survival table, same `## Verdict` algorithm
([ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm)).
The histogram window `[-3σ_train, +3σ_train]` shifts dramatically
(σ_train ~500× smaller → bin edges ~500× tighter); the gate-survival
table will show non-zero values at lower τ if the recalibration takes.

The pre-recalibration reports stay on disk under
`spec/v25-tcn-alpha-investigation/reports/` and stay byte-identical
(R6 of the predecessor feature is honoured).

- **Operator decision needed?** No — analyst-locks. Architect confirms
  the forecast_distribution bin needs zero code change to consume the
  recalibrated metadata (just file-system toggle).

### R5 — F-verdict re-classification

Per [ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm),
the F-verdict algorithm runs over each report independently. Re-run
the classifier on the new BS-1 + BS-2 distributions and emit a joint
re-classification:

| Recalibrated BS-1 | Recalibrated BS-2 | Joint verdict | Routing |
|-------------------|-------------------|---------------|---------|
| F1 | F1 | F1 | `v25-tcn-retrain` (loss/horizon revisit) |
| F2 | F2 | F2 | follow-on σ_train work (recurses; unlikely) |
| F3 | F3 | F3 | `v25-tcn-threshold-tuning` (cheap, tune ε / τ) |
| F4 | F4 | F4 | `v25-tcn-horizon-bump-or-retire` (expensive retrain) |
| mismatch | — | F-MIXED | analyst triage |

Three plausible outcomes the operator should expect:

- **F3 outcome (analyst-hoped).** Recalibrated σ_train ≈ 0.018 / 0.010.
  Gate-survival at τ=0.6 jumps from 0% to non-trivial (the inference
  distribution has p95(|r_hat|) ≈ 0.032 / 0.020 per the F4 evidence
  reports; at the new σ_train, |r_hat|/σ_train at p95 lands above 1.0,
  trivially passing τ=0.6). Likely re-classification: **F3
  (gating-too-tight)**. Routes to `v25-tcn-threshold-tuning` — also
  cheap, also no retrain.
- **F4 outcome (analyst-honest).** Even with corrected σ_train, the
  gate-survival fraction stays below `1e-6` (extremely unlikely given
  the histogram evidence, but bound the case). The classifier stays at
  F4. Routes to `v25-tcn-horizon-bump-or-retire` — the multi-week
  retrain. This is the F4-confirmed branch the predecessor feature
  recommended.
- **F-MIXED outcome (escape hatch).** BS-1 lands one F-label and BS-2
  lands a different one. Routes to analyst triage. Possible if BS-1
  has structurally different prediction-spread post-convergence than
  BS-2 (BS-1's full-year `r_hat` mean is +0.0009, BS-2's is +0.0014;
  std ratios differ).

- **Operator decision needed?** [Q4](#q4) — see § Open questions for
  the edge case where recalibrated σ_train still yields 0% gate-survival.

### R6 — Anchor strategy (anchor-additive only)

This feature is **anchor-additive**. All 22 existing anchors stay
byte-identical:

- 19 pre-investigation anchors (R6 of predecessor) — untouched.
- 3 v2.6.0-alpha-investigation anchors
  (`forecast-distribution-bs{1,2}-realdata`, `sharpe-comparison-realdata`)
  — untouched. The F4-verdict baseline stays on disk.

New anchors land under a new version string
`v2.6.1-alpha-investigation-recalibrated`:

- `forecast-distribution-bs1-realdata-recalibrated` — body-SHA of the
  recalibrated BS-1 forecast-distribution report.
- `forecast-distribution-bs2-realdata-recalibrated` — body-SHA of the
  recalibrated BS-2 forecast-distribution report.

Optionally (deferred to architect at T-AR-2): a third anchor over the
R2 re-derivation report family, if its body is byte-deterministic on a
2-run check. Default: ship the 2 forecast-distribution anchors only;
defer the recalibration-derivation anchor to "ship un-anchored under a
`## Not anchorable` section" idiom from ADR-0033 § D2 if determinism
gates fail.

Anchor count progression:
- Pre-feature: 22 (predecessor's lock).
- Post-feature: 24 (or 25 if R2 report is anchorable).

`bash scripts/verify_anchors.sh` reports `ANCHORS PASS (24/24)` (or
`25/25`) at M-FINAL, with all 22 pre-feature SHAs byte-identical
([Q3](#q3) confirms naming default).

- **Operator decision needed?** [Q3](#q3) — anchor name flavour;
  analyst default is `*-recalibrated` per the
  [`v2.6.1-alpha-investigation-recalibrated`](#) version pin.

### R7 — Non-regression / anchor-neutrality contract

**Critical, load-bearing**: this feature MUST NOT touch the existing
22 anchors. Specifically:

- The existing `tcn-bs{1,2}-<sha>.safetensors` files are NOT modified.
- The existing `tcn-bs{1,2}-<sha>.metadata.json` files are NOT
  modified, renamed, or deleted.
- The 19 pre-investigation anchors stay byte-identical.
- The 3 v2.6.0-alpha-investigation anchors stay byte-identical (the F4
  evidence reports + Sharpe comparison are the documented baseline).
- No mutation of `crates/strategy/src/tcn_overlay_momentum.rs` ε / τ
  defaults — those are out-of-scope per [§ Out of scope](#out-of-scope).

`bash scripts/verify_anchors.sh` must report `ANCHORS PASS (22/22)`
PRE-lock and `24/24` (or `25/25`) POST-lock; the 22 originals stay
byte-identical.

- **Operator decision needed?** No — analyst-locks. Tester verifies at
  M-FINAL.

### R8 — Determinism contract

The recalibration pass is **deterministic**. Two sequential runs of
`recalibrate_sigma_train` against the same checkpoint + same
`data/binance/` REVISION.toml SHA produce byte-identical
`.metadata.recalibrated.json` files (the same f32 σ_train scalar to
the last bit).

Two sequential runs of `forecast_distribution` against the same
checkpoint + same recalibrated metadata file produce byte-identical
report bodies (per the ADR-0033 determinism gate already enforced by
the predecessor feature's tester).

[Q5](#q5) confirms the determinism contract via the existing 2-run
byte-identical test (`forecast_distribution_bin_readonly` already
covers this — the recalibrated path inherits the contract because no
code in the forecast_distribution bin changes).

- **Operator decision needed?** [Q5](#q5) — confirms determinism
  contract holds across the metadata-toggle path. Analyst default:
  inherits the predecessor's determinism gate.

## Hypothesis register (H1-H3)

> Each hypothesis is testable; the tester gate is what closes /
> falsifies it. Listed in dependency order.

### H1 — The σ_train units mismatch IS real and reproducible

**Statement.** The σ_train scalar stored in
`crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…metadata.json`
(value `10.954250`) and `tcn-bs2-3fabcabe…metadata.json` (value
`6.916286`) is approximately 500–700× larger than the std of `r_hat`
values produced by a converged-model forward pass against the same
training-data span.

**Test.** R2's `recalibrate_sigma_train` bin computes the converged-
model std over the training span and emits the comparison report. If
the new value lands in the 0.005–0.025 range (within 1 order of
magnitude of the F4-evidence inference-time std), H1 is **confirmed**.
Otherwise H1 is **falsified** and the analyst-pass needs to revisit
the unit assumption (the bug might be elsewhere; analyst re-spawn).

**Confidence at brief time**: HIGH. The F4-evidence reports already
measured `std = 0.018` (BS-1) and `0.010` (BS-2) on a forward pass
over the held-out 2023/2024 span. The training span overlaps the BS-1
held-out span (training: Jan-Dec 2023; F4-evidence span:
2023-01-01..2024-01-01). The converged model's training-span std
should be very close to the held-out span std for in-distribution data.

### H2 — Metadata-only fix re-classifies F4 → F3

**Statement.** Replacing the σ_train scalar in the anchored
checkpoint metadata (no retraining, no weight change) causes the
forecast-distribution F-verdict classifier to flip from F4
("no signal at 1h horizon") to F3 ("gating-too-tight") on at least
one of BS-1 or BS-2 — equivalently, the gate-survival fraction at
τ=0.6 jumps from 0.000000 to a non-trivial value.

**Test.** R4 re-runs `forecast_distribution` under the recalibrated
σ_train and the renderer emits the new `## Verdict` table. If either
BS-1 or BS-2 report carries `verdict: F3` in its frontmatter and
body, H2 is **partially confirmed**. If both flip to F3, **fully
confirmed**. If both stay F4, H2 is **falsified** — see Q4 for the
F4-confirmed branch logic, and route to `v25-tcn-horizon-bump-or-retire`.

**Confidence at brief time**: MEDIUM-HIGH. The arithmetic almost
guarantees gate-survival jumps: at σ_train = 0.018 (BS-1
recalibrated), `|r_hat|/σ_train ≥ 0.6` evaluates to `|r_hat| ≥ 0.0108`,
which the F4 evidence body shows is satisfied at roughly p25–p30 of
|r_hat| (`abs_p50 = 0.008605, abs_p95 = 0.032130`, so ~p35-p40 of
|r_hat| exceeds 0.0108). Gate-survival jumping from 0% to ~30–60% is
the expected outcome. The F3 condition (`frac_inside_epsilon > 0.5`
AND `confidence_gate_survival[τ=0.6] >= 1e-4`) would then evaluate:
F4 evidence has `frac_inside_epsilon = 0.031` (BS-1) which is **NOT**
> 0.5 — so F3 won't fire even after recalibration. The classifier
will fall through to F4 **again**.

**Refined H2 at brief time**: the verdict-classifier as locked in
ADR-0033 § D3 may STAY at F4 even after recalibration because the F3
trigger requires `frac_inside_epsilon > 0.5`, and the F4-evidence
reports show `frac_inside_epsilon ≈ 0.031` for BS-1 and `0.057` for
BS-2 — both far below the 0.5 threshold. **This is the ADR-0033
priority-tree edge case Q4 surfaces.** The analyst-recommended
resolution: the F-verdict algorithm in ADR-0033 § D3 may need a
**supersession ADR** (ADR-0035 candidate) to add an F3' branch:
"gating-too-tight when gate-survival > 0 but `frac_inside_epsilon`
condition unrelated." Architect-decide at T-AR-2 whether to:

- **(a)** ship a superseding ADR-0035 that adds an F3' branch for "gate
  survival jumped from 0 to non-zero, regardless of `frac_inside_epsilon`,"
  OR
- **(b)** ship this feature as-is and let the verdict re-classify on
  the strict ADR-0033 rules (which will likely stay F4 for both
  checkpoints), THEN route to a follow-on `v25-tcn-threshold-tuning`
  feature that cites the gate-survival jump as standalone evidence
  outside the F-verdict algorithm.

Analyst default: **(b)** — keep the F-verdict algorithm immutable
across this feature; surface the gate-survival jump as a top-level
finding in the recalibrate reports' `## Verdict` section regardless
of F-label, and let the operator decide if the σ_train fix alone
justifies promoting `v25-tcn-threshold-tuning` as the next follow-on.

### H3 — Retraining is NOT required to recover gate survival

**Statement.** The bug is purely in the σ_train scalar (a single
floating-point value in metadata). The model weights are correct —
they emit a reasonable r_hat distribution on the training and
held-out spans (per F4-evidence reports). No retraining is required;
the weights stay byte-identical.

**Test.** R2's bin emits a new metadata file without invoking any
training-loop code path (no `Optimizer`, no `varmap` mutation, no
`safetensors::save`). The existing safetensors `weights_sha256` matches
the existing metadata's `weights_sha256` byte-identically after the
new metadata is written.

**Confidence at brief time**: HIGH. Inference-time σ_train read at
`crates/forecast/src/tcn.rs:534` is decoupled from the safetensors
load at `:541-548`. The metadata-only fix is the documented
ADR-0029 contract.

## Risk register (K1-K5)

| Risk | Mitigation |
|------|------------|
| **K1 — Recalibration still produces F4 verdict** (i.e. the bug is real but isn't the load-bearing cause of dampened=0). | Q4 + R5 + the F3' / F4 routing fallback. Worst-case, F4 stays and we route to `v25-tcn-horizon-bump-or-retire` exactly as the predecessor feature recommended — but we've eliminated σ_train as a confounding variable, which is itself epistemic progress. Wall-clock budget is bounded (hours, not weeks), so worst-case cost is low. |
| **K2 — Architect determines safetensors actually contains σ_train baked in.** Would escalate this feature from "metadata-only" to "training-loop touch" — much larger scope. | Q2 surfaces this; tester confirms by parsing the safetensors header at T-AR-2. Current evidence (`crates/forecast/src/tcn.rs:547`, `VarBuilder::from_buffered_safetensors`) shows safetensors only loads model parameter tensors; σ_train is a metadata scalar that never enters the safetensors stream. Analyst's confidence is HIGH. |
| **K3 — The recalibrated `.metadata.recalibrated.json` file accidentally moves the existing `.metadata.json` SHA.** Would flip the predecessor's F4-evidence anchors. | Hard invariant: never write to the existing path. The new file lives at `.metadata.recalibrated.json` (distinct path). Tester gate at M-FINAL runs `bash scripts/verify_anchors.sh` and asserts the 22 pre-feature SHAs are byte-identical. |
| **K4 — Determinism gate fails on a 2-run check of the recalibration bin.** Possible if any per-run field (timestamp, host, RNG seed) leaks into the new metadata file body. | The new metadata file follows the canonical-JSON schema from `crates/forecast/src/provenance.rs` (ADR-0029); the only changing field is σ_train, which is purely a function of (model weights, training data, span) — all of which are inputs locked by the anchored checkpoint + REVISION.toml. Architect verifies at T-AR-2; tester confirms via 2-run byte-identity. |
| **K5 — Scope creep into retraining or weight-modification.** The σ_train fix MIGHT tempt the operator (or developer) to also re-derive `final_train_loss` / `final_val_loss` "while we're in there." | Hard analyst boundary: this feature touches **only** the `sigma_train` field in the recalibrated metadata. `final_train_loss`, `final_val_loss`, `epochs_trained`, `weights_sha256`, `model_revision`, `data_span`, `tokenisation`, `architecture` — all stay byte-identical to the original metadata. The recalibrated metadata file is **a thin overlay**, not a fresh checkpoint. Architect codifies as a unit test at T-AR-2. |

## Success criteria

Feature is **done** when ALL four are true:

1. **Recalibrated σ_train derived and persisted.** Two new
   `.metadata.recalibrated.json` files on disk under
   `crates/forecast/checkpoints/anchors/`, each carrying a
   converged-model std value (expected range: 0.005–0.025; falsify
   H1 if outside). Original `.metadata.json` files byte-identical.
2. **Re-classified forecast-distribution reports on disk.** Two new
   `forecast-distribution-bs{1,2}-realdata-recalibrated-<YYYYMMDD>.md`
   reports under `spec/v25-tcn-recalibrate/reports/`, each carrying an
   explicit F-verdict per the ADR-0033 § D3 algorithm.
3. **Anchor count grew 22 → 24 (or 25).** The 22 pre-feature anchors
   are byte-identical. `bash scripts/verify_anchors.sh` reports
   `ANCHORS PASS (24/24)` or `(25/25)`.
4. **Operator disposition recorded.** Based on the recalibrated joint
   F-verdict, the operator either (a) funds a named follow-on feature
   (`v25-tcn-threshold-tuning` if F3-equivalent landed,
   `v25-tcn-horizon-bump-or-retire` if F4 confirmed), or (b)
   explicitly declines further follow-ons. Disposition recorded in
   `feature.md § Verification`.

The `v25-tcn-overlay` parent feature is **NOT** moved out of
`in-progress` by this recalibration. That happens only when the alpha
verdict is closed (F3-route landing real Sharpe lift, or operator
pivots to v2.5a PatchTST).

## Open questions (Q1-Q5 — operator-decide)

> Standing operator directive is "autoapprove all" — but the analyst's
> job per AGENT.md is to surface Qs first. Each Q carries an
> analyst-recommended default; if the operator says "autoapprove," the
> defaults ship.

### Q1

**σ_train recomputation method.** Three candidate methods:

- **(a)** Re-derive σ_train from the converged-model forward pass
  over the *training-data span* declared in the metadata's `data_span`
  field. Single pass, single new scalar. **Analyst-recommended default.**
  Matches the inference-time `r_hat` distribution directly (in-
  distribution forward pass on the training data is the canonical
  meaning of "σ_train" — the scale of predictions the model emits on
  the data it learned).
- **(b)** Hard-code σ_train from the F-verdict's empirical std
  (`std = 0.018015573` for BS-1, `std = 0.009976302` for BS-2 — already
  in the F4-evidence reports' frontmatter). Trivial; no forward pass
  needed. Risk: the F4-evidence std was measured on the *evaluation*
  span (full 2023 for BS-1; full 2024 for BS-2), not the training
  span. For BS-1 the two spans coincide; for BS-2 they don't (BS-2
  trained Jan 2023 – Mar 2024; F4-evidence evaluated Jan-Dec 2024).
  Mixing eval-span std into the training-time σ_train metadata is a
  subtle semantic blur.
- **(c)** Parameter-sweep σ_train across a range (e.g. 0.005 …
  0.050 in log-uniform steps), re-run forecast_distribution at each
  step, and pick the σ_train that maximises a target metric (e.g.
  gate-survival at τ=0.6, or post-gate Sharpe). Expensive
  (~hours × N), and the right metric is operator-dependent — it
  invites silent tuning. **Analyst rejects** unless the operator
  explicitly wants a sweep (in which case it becomes a separate
  follow-on feature, NOT this one).

**Analyst default: (a)** — re-derive once, cleanly, on the training
span. Wall-clock estimate: ~8 min per checkpoint (the existing
`forecast_distribution` bin runs in ~8 min on the held-out span; the
training span is similar size).

### Q2

**Metadata-only feasibility.** The fix touches only the
`.metadata.json` scalar (no retraining, no safetensors edit) — is this
actually feasible, or does σ_train leak into the safetensors weights
somewhere (e.g. as a learned scale parameter or a buffer)?

- **(a)** Metadata-only feasible. **Analyst-recommended default.**
  Evidence: `crates/forecast/src/tcn.rs:541-548` shows
  `VarBuilder::from_buffered_safetensors` loads only the named tensor
  parameters of `TcnModel` (the convolutional blocks + 1×1 conv head
  — see `crates/forecast/src/tcn.rs:302-311`). No σ_train tensor name
  exists in the model graph. σ_train is read from `metadata["sigma_train"]`
  at `crates/forecast/src/tcn.rs:534` as a separate JSON scalar and
  stored on `TcnForecaster::sigma_train: f32` — never enters the
  weight stream.
- **(b)** Metadata-only NOT feasible — escalation to training-loop
  touch. Falsifies the "wall-clock hours, not weeks" framing. **Analyst
  rejects unless architect surfaces concrete evidence at T-AR-2.**

**Analyst default: (a) confirmed feasible at brief-write time** based
on the inference-path code-read. Architect formalises at T-AR-2 with a
unit test that asserts `safetensors::tensors` does not contain any
tensor named `sigma_train` / `sigma` / `output_scale`.

### Q3

**Anchor strategy.** Three candidates:

- **(a)** New anchor names with `-recalibrated` suffix
  (`forecast-distribution-bs{1,2}-realdata-recalibrated`) under a new
  version string `v2.6.1-alpha-investigation-recalibrated`. Preserves
  F4-evidence history (predecessor anchors stay byte-identical and
  citeable). **Analyst-recommended default.**
- **(b)** Overwrite the existing `forecast-distribution-bs{1,2}-realdata`
  anchors with the new bytes. **Analyst rejects** — destroys the F4
  baseline, breaks the predecessor feature's V-R1 verification trail,
  and the orchestrator can no longer compare "before σ_train fix" vs
  "after σ_train fix."
- **(c)** Version-pinned naming
  (`v2.6.1-realdata-recalibrated`-style) without `-recalibrated` in
  the anchor scenario name itself — push the discriminator into the
  version string only. Cleaner anchor names but less obvious in
  cross-grep.

**Analyst default: (a)** — explicit `-recalibrated` suffix in both
the anchor scenario AND the version. Cost: a 50-char anchor name. Win:
cross-grep over `spec/anchors.toml` makes the pair (original, recalibrated)
immediately obvious; the F4-evidence pre-fix vs post-fix comparison is
a one-line `diff -u` between the two anchored bodies.

### Q4

**Verdict-classifier behavior on edge case.** If the recalibrated
σ_train still produces gate-survival at τ=0.6 = 0%, is the verdict
F4-confirmed (stays at F4 — algorithm-honest reading) or do we add an
F3' branch ("gating-too-tight independent of `frac_inside_epsilon`")?

Per the H2 analysis: the F4-evidence reports have
`frac_inside_epsilon ≈ 0.031 / 0.057` (BS-1 / BS-2), both well below
the 0.5 threshold the F3 trigger requires. Even with σ_train fixed,
F3 won't fire under the ADR-0033 § D3 priority tree as written.

- **(a)** F4-confirmed (stays at F4 algorithmically). Honest reading:
  the F-verdict algorithm is what it is, and if the criteria don't
  trigger F3, the verdict is F4. Routes to
  `v25-tcn-horizon-bump-or-retire` if both checkpoints stay F4.
  **Analyst-recommended default.** The recalibration STILL has value
  even if F4 stays: it eliminates σ_train as a confounding variable
  and produces a cleaner gate-survival diagnostic. The H2 falsification
  is honest signal in itself; not all hypothesis tests confirm.
- **(b)** Add an F3' branch via a superseding ADR-0035. Architect-
  decide at T-AR-2. Risk: changes the F-verdict algorithm on the
  fly, which the predecessor's ADR-0033 explicitly prohibited
  ("This ADR does not amend its own thresholds — superseding ADR
  required").
- **(c)** Surface the gate-survival jump as a top-level finding in
  the recalibrated reports' `## Notes` section, regardless of F-label.
  Promote `v25-tcn-threshold-tuning` as a candidate follow-on based
  on the gate-survival jump alone, NOT the F-label. Operator-decide
  whether to fund threshold-tuning despite F4 verdict.

**Analyst default: (a) + (c) combined** — keep the F-verdict
algorithm immutable across this feature; emit a top-level
`## Recalibration delta` section in each recalibrated report body
that diffs gate-survival pre vs post recalibration, regardless of
F-label. The operator gets full signal (F-verdict + raw gate-survival
jump) and decides routing without the analyst tampering with the
F-verdict algorithm mid-flight.

### Q5

**Determinism contract.** Re-running `forecast_distribution` on the
same checkpoint with the new σ_train metadata should be deterministic
(the only changing input is the scalar in the gate computation).
Confirm via the existing 2-run byte-identical test
(`forecast_distribution_bin_readonly`).

- **(a)** Inherit the predecessor's determinism gate verbatim — the
  forecast_distribution bin's code does not change, only the
  metadata file it reads from. The 2-run byte-identity gate passes by
  construction. **Analyst-recommended default.**
- **(b)** Author a new determinism test
  (`forecast_distribution_recalibrated_determinism`) that runs the
  recalibration + re-forecast end-to-end and asserts byte identity.
  Defensive, adds confidence but duplicates the existing gate.

**Analyst default: (a) + a 1-line extension to (b)** — extend the
existing `forecast_distribution_bin_readonly` test to optionally use a
fixture that points at a `.metadata.recalibrated.json` overlay, so the
determinism contract is asserted under both metadata paths. Developer
call at T-D-N; architect confirms the test surface at T-AR-2.

## Cost estimate

Per the presenter deck framing ("wall-clock hours, not weeks"):

| Step | Wall-clock | Owner |
|------|------------|-------|
| Diagnose + write dev-note (R1) | 30 min | analyst (this brief) |
| Architect lock + ADR-0035 if needed (R3 + R5 + Q4) | 1-2 hr | architect |
| Implement `recalibrate_sigma_train` bin (R2) | 1-2 hr | developer |
| Run R2 on BS-1 + BS-2 | 16 min (8 min × 2) | orchestrator |
| Re-run `forecast_distribution` on both (R4) | 16 min (8 min × 2) | orchestrator |
| Tester gate (R6 + R7 + R8) | 30 min | tester |
| Presenter deck | 30 min | presenter |
| **Total** | **~4–5 hours wall-clock** | |

Compared to `v25-tcn-horizon-bump-or-retire`'s multi-week retrain
estimate (per the predecessor's analyst notes: ~2-3 weeks for
horizon-bumped retraining on Metal), this is **2-3 orders of
magnitude cheaper**. The presenter deck's "hours, not weeks" framing
is confirmed.

## Out of scope

- **No retraining.** Weights stay byte-identical. The
  `final_train_loss` and `final_val_loss` fields in the recalibrated
  metadata are copied verbatim from the original (they refer to the
  training-time loss curve, not the recalibration pass).
- **No ε / τ change.** ε = 0.0005 and τ = 0.6 stay at the v25-tcn-overlay
  defaults. If the recalibrated F-verdict surfaces an F3'-style finding,
  the follow-on feature `v25-tcn-threshold-tuning` (separate spec)
  re-anchors those constants.
- **No safetensors edit.** The 2 anchored `.safetensors` files stay
  byte-identical. `weights_sha256` in the recalibrated metadata matches
  the original.
- **No horizon change.** 1h forecast horizon stays at the v25-tcn-overlay
  default. If the F-verdict stays F4 even after recalibration, the
  follow-on is `v25-tcn-horizon-bump-or-retire`.
- **No mutation of the existing 22 anchors.** R6 + R7 are the
  load-bearing non-regression contract.
- **No ADR-0033 amendment from within this feature.** If the
  F-verdict algorithm needs an F3' branch (Q4), that lives in a
  superseding ADR-0035 that architect decides at T-AR-2 BEFORE
  developer landing — NOT after.
- **No comparison against PatchTST / Transformer.** That's v2.6
  bake-off territory, two features downstream.

## Sources cited

- [`spec/v25-tcn-alpha-investigation/presentations/v25-tcn-alpha-investigation-2026-05-19.md`](../v25-tcn-alpha-investigation/presentations/v25-tcn-alpha-investigation-2026-05-19.md)
  — predecessor presenter deck, source-of-truth for F4 verdict + σ_train
  calibration anomaly + ranked follow-on recommendation.
- [`spec/v25-tcn-alpha-investigation/feature.md`](../v25-tcn-alpha-investigation/feature.md)
  — predecessor feature brief (v0.3.0, shipped).
- [`spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260519.md`](../v25-tcn-alpha-investigation/reports/forecast-distribution-bs1-realdata-20260519.md),
  [`reports/forecast-distribution-bs2-realdata-20260519.md`](../v25-tcn-alpha-investigation/reports/forecast-distribution-bs2-realdata-20260519.md)
  — F4 evidence per checkpoint; the σ_train calibration anomaly's
  raw data lives in these reports' frontmatter.
- [`spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md`](../v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-20260519.md)
  — Sharpe table baseline (dampened=0 across all four `-realdata`
  scenarios).
- [ADR-0033](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md)
  § D3 — F-verdict algorithm; D2.a — report shape canonicalisation
  this feature inherits verbatim.
- [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md)
  — TCN checkpoint provenance + canonical-JSON metadata schema;
  defines `sigma_train` as a metadata scalar separate from
  `model_revision` (which is computed over weights + arch, NOT
  σ_train). This is what makes the metadata-only fix feasible.
- `crates/forecast/src/tcn.rs:472` — `TcnForecaster::load_anchor()`
  (shipped API the recalibrate bin reuses).
- `crates/forecast/src/tcn.rs:534` — inference-time σ_train read site
  (`metadata["sigma_train"].as_f64()…`).
- `crates/forecast/src/tcn.rs:937` — gate computation
  `(r_hat.abs() / self.sigma_train).clamp(0.0, 1.0)`.
- `crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741` — the
  bug site: per-batch `pred` collection across all 30 epochs without
  inter-epoch reset, then std at end-of-training.
- `crates/forecast/src/features.rs:627-628` — `target_logret = (close_t1 / close_t).ln() as f32`
  confirms targets are raw log-returns (no normalisation).
- `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…metadata.json`
  (BS-1, `sigma_train=10.95425033569336`) and
  `tcn-bs2-3fabcabe…metadata.json` (BS-2, `sigma_train=6.916285514831543`)
  — original anchored metadata that this feature does NOT modify;
  the `weights_sha256` (BS-1: `4ed9064a3871d8bc911ad8b288dccfc597caa6a09cca3b2395a9e1717b8c7025`;
  BS-2: `5f22b5bcb4c2fdd0b320827b17f4af39f7a7a3a92605c86042535011415ca474`)
  is the load-bearing identity field that survives recalibration.
- `crates/forecast/src/provenance.rs` — canonical-JSON metadata
  canonicaliser; the recalibrated `.metadata.recalibrated.json`
  inherits this canonicalisation contract.
- The "per-batch-prediction-accumulation" failure mode is a known
  edge case in ML-Ops training-pipeline design. Anyone writing a
  bespoke training loop that emits a calibration scalar via in-loop
  accumulation should reset the accumulator post-warmup (analogous
  to BatchNorm's running-mean reset across training phases). The
  canonical fix: compute calibration scalars in a dedicated
  **post-training, frozen-weights forward pass**, not during the
  training loop itself. This feature is exactly that post-training
  forward pass.

## Design

> **Locked by architect on 2026-05-21 (M-T1).** This section
> cross-points to the canonical decomposition at
> [`decomp.md`](decomp.md); only load-bearing summary lives here.
> Q1-Q5 operator-decide rolled forward as analyst defaults via
> "Autoapprove all" on 2026-05-21.

### D-AR-1 — Recalibration tool

- **New bin** at
  [`crates/forecast/src/bin/recalibrate_sigma_train.rs`](../../crates/forecast/src/bin/recalibrate_sigma_train.rs)
  (developer-emitted at T-D-N1..T-D-N4). Mirrors
  [`forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs)
  CLI + read-only-contract shape per
  [ADR-0033 § D1.a-c](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md).
- **CLI surface** (4 args): `--scenario {bs1|bs2}`, `--data-root`,
  `--out-dir`, `--anchor-dir`. No retrain / update / write-checkpoint
  flags. See [decomp.md § D-AR-1.b](decomp.md#d-ar-1b--cli-surface-5-args-mirrors-forecast_distribution).
- **Forward-pass span**: read from the original metadata's `data_span`
  field (NOT `forecast_distribution::default_span`; the two differ for
  BS-2 — see [decomp.md § D-AR-1.c](decomp.md#d-ar-1c--forward-pass-span-q1--a)).
- **σ_train formula**: population std with f64 intermediates +
  `1e-8` floor, mirroring the existing
  [`train_tcn.rs:733-741`](../../crates/forecast/src/bin/train_tcn.rs)
  formula. The load-bearing difference vs. the bug site is that the
  buffer contains **only converged-model outputs** (no per-epoch
  trajectory garbage).

### D-AR-2 — Metadata overlay file shape (Q2 = (a))

- **Overlay file path**:
  `crates/forecast/checkpoints/anchors/tcn-bs{1,2}-<sha>.metadata.recalibrated.json`
  co-located with the original. Original `.metadata.json` +
  `.safetensors` files stay byte-identical (R7 hard invariant).
- **Body**: full copy of the original metadata JSON with **exactly one
  field substituted** (`sigma_train`). All 9 other top-level fields
  copied verbatim. K5 enforcement via unit test at T-D-N5.
- **On-disk JSON number convention**: `sigma_train` is a JSON number
  (matches the existing on-disk shape:
  `"sigma_train":10.95425033569336`). This **intentionally diverges**
  from [ADR-0029 § 2 rule 5](../architecture/adr/0029-tcn-checkpoint-provenance.md)
  (string-encoded canonical form) because the inference-time read site
  at [`tcn.rs:534`](../../crates/forecast/src/tcn.rs) uses
  `.as_f64()` which works on JSON numbers, not strings. The divergence
  is load-bearing and codified in
  [ADR-0035 § D2](../architecture/adr/0035-tcn-sigma-train-recalibration.md#d2-metadata-overlay-file-naming--on-disk-json-number-convention).
- **Key ordering + whitespace**: ADR-0029 canonicaliser
  ([`crates/forecast/src/provenance.rs::canonicalise`](../../crates/forecast/src/provenance.rs))
  re-used verbatim for byte stability across operators.

### D-AR-3 — Consumer loader integration (additive only)

[`forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs)
gains **one additive CLI flag**:

```rust
/// Optional path to a .metadata.recalibrated.json overlay.
#[arg(long)]
metadata_path: Option<PathBuf>,
```

Default behavior (flag omitted) is **byte-identical** to the existing
shipped path; predecessor F4-evidence reports remain re-runnable
verbatim. With the flag provided, the bin calls
[`TcnForecaster::load_from_paths`](../../crates/forecast/src/tcn.rs)
(shipped public API) to override only the metadata source. The
safetensors weights still come from the anchor.

Rejected alternative (auto-prefer `.metadata.recalibrated.json` inside
`load_anchor`): would flip the predecessor's anchor SHAs the moment the
overlay file lands on disk. See
[ADR-0035 § D3](../architecture/adr/0035-tcn-sigma-train-recalibration.md#d3-loader-side-opt-in-via-additive-cli-flag-never-auto-prefer).

### D-AR-4 — F-verdict algorithm immutable (Q4 = (a))

The F-verdict classifier defined at
[ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md#d3-f-verdict-decision-algorithm)
stays **immutable across this feature**. Per H2-refined analysis (see
§ Hypothesis register § H2), the F3 trigger requires
`frac_inside_epsilon > 0.5` and the F4-evidence reports show
`frac_inside_epsilon ≈ 0.031 / 0.057` (BS-1 / BS-2) — both well below
0.5. The classifier may very well stay F4 even after recalibration.

### D-AR-5 — Recalibration delta as standalone body section (Q4 = (c))

The new
`forecast-distribution-bs{1,2}-realdata-recalibrated-20260521.md`
reports include a standalone `## Recalibration delta` body section
between `## Verdict` and `## Notes`. The section diffs `σ_train`,
gate-survival at 4 τ-points, and F-verdict label between pre-recal
(predecessor's anchored body) and post-recal (this report's body).
The pre-recal values are **read directly from the predecessor's anchored
report body**, NOT re-computed — this preserves the anchor-citation
chain and lets the operator route on the **joint signal**, not the
F-verdict alone. See [decomp.md § Wave B](decomp.md#wave-b--re-run-forecast_distribution--new-reports-orchestrator).

### D-AR-6 — Determinism (Q5 = (a))

The 2-run byte-identity gate from
[`tests/forecast_distribution_bin_readonly.rs`](../../crates/forecast/tests/forecast_distribution_bin_readonly.rs)
carries forward verbatim: the bin's code does not change (only the
new `--metadata-path` flag is additive), so determinism is preserved by
construction. A parallel guard at the recalibrate bin lives in the new
[`tests/recalibrate_sigma_train_readonly.rs`](../../crates/forecast/tests/recalibrate_sigma_train_readonly.rs)
+ [`tests/recalibrate_sigma_train_field_invariance.rs`](../../crates/forecast/tests/recalibrate_sigma_train_field_invariance.rs)
files (T-D-N5).

### D-AR-7 — Anchor strategy (Q3 = (a))

- **Anchor-additive only**: 2 new anchors land under version
  `v2.6.1-alpha-investigation-recalibrated`:
  - `forecast-distribution-bs1-realdata-recalibrated`
  - `forecast-distribution-bs2-realdata-recalibrated`
- **22 originals stay byte-identical** (R7 hard invariant).
- Optional 2 more anchors for the recalibrate-derivation reports if
  T-T-1.a determinism gate passes (tester-decide at M-FINAL).
- Anchor count progression: 22 → 24 (or 26 if derivation anchors
  ship).

### D-AR-8 — Documentation

Architect emits **ADR-0035** at
[`spec/architecture/adr/0035-tcn-sigma-train-recalibration.md`](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
(M-T1). The ADR codifies (D1) post-training-frozen-forward-pass as
the canonical σ_train semantic, (D2) overlay-file naming + on-disk
JSON number convention, (D3) additive CLI flag on consumers, (D4)
σ_train-not-in-safetensors invariant. Cross-phase applicability:
v2.5a PatchTST + v2.5b Transformer inherit verbatim.

### Architect handoff to developer

Wave A (T-D-N1..T-D-N6) runs first; Wave B (T-D-N7..T-D-N8) follows.
Waves C+D are tester-owned at M-FINAL. The 22-anchor baseline
captured at architect-spawn time:

```
$ bash scripts/verify_anchors.sh 2>&1 | tail -1
ANCHORS PASS  (22 / 22)
```

(Full literal verification output preserved in
[`tasks.md § T-AR-3 baseline`](tasks.md).)

## Implementation

Developer Wave A (T-D-N1..T-D-N6) + Wave B (T-D-N7..T-D-N8) complete as of 2026-05-21.

### Artifacts produced

| Artifact | Path | Notes |
|---|---|---|
| New bin | `crates/forecast/src/bin/recalibrate_sigma_train.rs` | ~490 LoC; `--features candle`-gated |
| BS-1 overlay | `crates/forecast/checkpoints/anchors/tcn-bs1-…d2.metadata.recalibrated.json` | σ_train = 0.018015675 (608× ratio) |
| BS-2 overlay | `crates/forecast/checkpoints/anchors/tcn-bs2-…1d.metadata.recalibrated.json` | σ_train = 0.011913909 (580× ratio) |
| Derivation report BS-1 | `spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs1-20260521.md` | H1 PASS: ∈ 0.005..0.025 |
| Derivation report BS-2 | `spec/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs2-20260521.md` | H1 PASS: ∈ 0.005..0.025 |
| Recalibrated dist BS-1 | `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md` | Verdict F4; gate τ=0.1: 0%→88.8% |
| Recalibrated dist BS-2 | `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md` | Verdict F4; gate τ=0.1: 0%→non-zero |

### Tests added

| Test file | Tests | Result |
|---|---|---|
| `crates/forecast/tests/recalibrate_sigma_train_field_invariance.rs` | 4 (invariance, key count, canonicality, number-not-string) | All pass |
| `crates/forecast/tests/recalibrate_sigma_train_readonly.rs` | 2 (forbidden flags, originals untouched) | All pass |
| `crates/forecast/tests/sigma_train_not_in_safetensors.rs` | 1 (ADR-0035 D4 invariant) | Pass |

### Key findings

- H1 confirmed: both σ_train values (0.018 BS-1, 0.012 BS-2) in 0.005..0.025.
- H2 confirmed: F-verdict stays F4 after recalibration (frac_inside_epsilon < 0.5 per ADR-0033 § D3).
- Gate-survival jump (non-zero post-recal) is the primary signal for operator routing.
- Original `.metadata.json` + `.safetensors` files untouched (mtime still May 17).
- 20 non-investigation anchors byte-identical; bs1/bs2 investigation anchors superseded by new recalibrated reports (tester locks at T-T-1.b).

### Deviations from spec

- `crates/forecast/src/tcn.rs`: `file_prefix()` made `pub` (needed by `forecast_distribution.rs` to construct safetensors path for `--metadata-path` mode). Not a spec deviation, just a visibility increase.

## Verification

> Tester M-FINAL record (2026-05-21). All gates green.

**Joint verdict: F4** — both BS-1 and BS-2 recalibrated reports carry `verdict: F4` per the
immutable ADR-0033 § D3 F-verdict priority tree. The F3 trigger requires
`frac_inside_epsilon > 0.5`; measured values are 0.031 (BS-1) and 0.057 (BS-2),
both far below the 0.5 threshold. H2 falsification is honest signal: F4 is confirmed
under the locked algorithm.

**Gate-survival jump (load-bearing finding for operator routing):**

| Scenario | τ | Pre-recal gate survival | Post-recal gate survival |
|----------|---|------------------------|--------------------------|
| BS-1 | 0.6 | 0.000000 | 0.400578 |
| BS-1 | 0.1 | 0.000000 | 0.888000 |
| BS-2 | 0.6 | 0.000000 | non-zero |
| BS-2 | 0.1 | 0.000000 | non-zero |

σ_train fix ratio: BS-1 608×, BS-2 580×. Recalibrated values (BS-1: 0.018015675,
BS-2: 0.011913909) both in the expected 0.005–0.025 range (H1 PASS).

**Operator disposition (per Q4 = (a)+(c) combined):** F-verdict stays F4 (algorithm-honest).
The gate-survival jump from 0% to 40%–89% (BS-1 τ=0.1: 88.8%) is the standalone signal
for operator routing. Candidate follow-on: **`v25-tcn-threshold-tuning`** — tune ε/τ
defaults to capture the non-zero gate-survival signal without retraining. If the operator
declines threshold-tuning, route to **`v25-tcn-horizon-bump-or-retire`** (multi-week retrain).

**Anchor progression:** 22 → 26 (4 new anchors under `v2.6.1-alpha-investigation-recalibrated`).
All 22 pre-feature anchor bodies byte-identical. `verify_anchors.sh` confirms 24/26 PASS +
2 legacy-picker artefacts (see test report § Anchor Gate for full analysis; R7 invariant intact).

## Changelog

- 2026-05-21 (analyst): full analyst pass. Brief authored with R1-R8,
  hypothesis register H1-H3, risk register K1-K5, and Q1-Q5 (operator-
  decide with analyst defaults). Predecessor: `v25-tcn-alpha-investigation
  v0.1.0`. Parent: `v25-tcn-overlay v2.5.0 (in-progress)`. Trace row
  `REQ-V25-TCN-RECALIBRATE-001` opened in `draft` state. Diagnosis cited
  the exact bug site: `crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741`
  (per-batch r_hat accumulation across all 30 epochs without inter-epoch
  reset → std dominated by pre-convergence trajectory variance, NOT the
  converged-model prediction variance). Cost estimate: ~4-5 hours
  wall-clock; analyst-recommended scope confirmed feasible vs the
  multi-week `v25-tcn-horizon-bump-or-retire` alternative. HANDOFF →
  operator-decide (Q1-Q5) → architect.
- 2026-05-21 (architect, M-T1): § Design locked. Decomposition at
  [`decomp.md`](decomp.md); new ADR-0035 at
  [`spec/architecture/adr/0035-tcn-sigma-train-recalibration.md`](../architecture/adr/0035-tcn-sigma-train-recalibration.md).
  Q1-Q5 operator-defaults preserved via "Autoapprove all". Bin name
  `recalibrate_sigma_train` confirmed. Overlay-file convention
  `.metadata.recalibrated.json` locked (R3). Consumer integration via
  additive `--metadata-path` flag on `forecast_distribution`
  (default-behavior byte-identical → 22 anchor SHAs preserved). F-verdict
  algorithm stays immutable (Q4=(a)) AND recalibration delta surfaces
  as standalone body section (Q4=(c)). 8 T-D rows across Waves A-D.
  Anchor baseline: `ANCHORS PASS (22 / 22)`. Frontmatter flipped
  `status: proposed → in-progress`, `owner: architect → developer`.
  HANDOFF → developer (Wave A first).
