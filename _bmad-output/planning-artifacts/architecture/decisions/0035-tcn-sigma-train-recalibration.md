---
adr: 0035
title: Post-training σ_train recalibration via metadata overlay (cross-phase contract for v2.5 / v2.5a / v2.5b)
status: accepted
date: 2026-05-21
supersedes: none
superseded-by: none
---

# ADR-0035: Post-training σ_train recalibration via metadata overlay

## Context

The v2.5 TCN alpha-investigation
([feature.md](../../../../docs/archive/pre-bmad-spec/v1/v25-tcn-alpha-investigation/feature.md),
[presenter deck 2026-05-19](../../../../docs/archive/presentations-2026-Q2.tar.gz))
shipped a joint **F4 verdict** ("no signal at 1h horizon") across BS-1 +
BS-2 on real Binance hourly OHLCV. The forensic deep-dive in the
predecessor's reports surfaced a load-bearing **σ_train units /
accumulation bug**:

- `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d…metadata.json`:
  `sigma_train = 10.954250` (BS-1).
- `crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabe…metadata.json`:
  `sigma_train = 6.916286` (BS-2).
- Both ~500–700× the inference-time `r_hat` std measured on the
  same span (BS-1: 0.018; BS-2: 0.010).

The bug location is at
[`crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741`](../../../../crates/forecast/src/bin/train_tcn.rs)
— a `Vec<f32>` accumulator declared **outside** the per-epoch loop,
appended per-batch inside the per-epoch loop, never reset between
epochs, then std-computed at end-of-training:

```rust
// :606
let mut all_r_hats: Vec<f32> = Vec::new(); // for sigma_train

// :676-678  (inside per-epoch loop, per-batch)
if let Ok(r_hats) = pred.flatten_all().and_then(|t| t.to_vec1::<f32>()) {
    all_r_hats.extend_from_slice(&r_hats);
}

// :733-741  (after 30 epochs)
let sigma_train = if all_r_hats.len() > 1 {
    let n = all_r_hats.len() as f32;
    let mu = all_r_hats.iter().sum::<f32>() / n;
    let var = all_r_hats.iter().map(|&x| (x - mu).powi(2)).sum::<f32>() / n;
    var.sqrt().max(1e-8)
} else { 1.0_f32 };
```

The final σ_train scalar is dominated by **early-epoch garbage
predictions** (random-init weights produce O(1) to O(10) outputs at
epoch 1; the model converges to O(0.01) by epoch 30 but the
accumulator already has 600 batches × 128 samples ≈ 76,800 samples of
trajectory noise per epoch baked in). The result is a scalar that
silences every gate-check `|r_hat|/σ_train ≥ τ` regardless of how
directionally useful `r_hat` is.

The inference-time read site reads this scalar straight off metadata
without recomputation:

- [`crates/forecast/src/tcn.rs:534`](../../../../crates/forecast/src/tcn.rs)
  — `sigma_train = metadata["sigma_train"].as_f64().unwrap_or(1.0_f64) as f32`.
- [`crates/forecast/src/tcn.rs:937`](../../../../crates/forecast/src/tcn.rs)
  — `(r_hat.abs() / self.sigma_train).clamp(0.0, 1.0)`.

The predecessor's
[ADR-0029](0029-tcn-checkpoint-provenance.md) documents the metadata
schema (σ_train is a top-level scalar) but does not assert that σ_train
is exclusively a metadata field — i.e. it does not codify the
"safetensors does not contain σ_train" invariant that this ADR depends
on. Likewise, [ADR-0033](0033-tcn-alpha-investigation-report-shape.md)
locks the F-verdict algorithm (§ D3) but explicitly does NOT amend its
own thresholds — any verdict-classifier change requires a superseding
ADR (which this ADR is **not**; per `v25-tcn-recalibrate` Q4 = (a) the
F-verdict algorithm stays immutable).

This ADR fills the gap between ADR-0029 (metadata schema) and ADR-0033
(F-verdict algorithm) for the v2.5 forecaster family: it codifies the
post-training-recalibration contract that makes metadata-only fixes
feasible across all forecaster phases under ADR-0028 (v2.5 TCN, v2.5a
PatchTST, v2.5b vanilla Transformer).

## Decision

### D1. Recalibrate σ_train in a frozen-weights post-training forward pass

Every v2.5/v2.5a/v2.5b forecaster checkpoint that exposes a
`sigma_train` scalar in its metadata must compute that scalar via a
**dedicated frozen-weights forward pass over the training-data span**,
NOT via an in-loop accumulator across training epochs.

The post-training pass:

1. Loads the converged model + frozen weights via the shipped
   `load_anchor()` / `load_from_paths()` API.
2. Iterates `windows_for_symbol()` over the metadata's `data_span`
   (start/end inclusive of training-data symbols).
3. Calls `forward(&x, /*train=*/ false)` (no dropout, no BatchNorm
   running-mean update — pure inference path).
4. Collects all `r_hat` scalars into a single buffer (no per-epoch
   accumulator).
5. Computes σ_train as population std with f64 intermediates and a
   `1e-8` floor:
   ```rust
   let mu = r.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
   let var = r.iter().map(|&x| (x as f64 - mu).powi(2)).sum::<f64>() / n as f64;
   let sigma = (var.sqrt().max(1e-8)) as f32;
   ```

This is **the canonical σ_train semantic** for the v2.5 family. The
predecessor's in-loop accumulator pattern is hereby **deprecated** for
all future training scaffolds — see § Negative precedent below.

### D2. Metadata overlay file naming + on-disk JSON number convention

When recalibrating an **existing** anchored checkpoint (i.e. fixing the
σ_train scalar without retraining), the recalibration tool MUST write a
**new file** at:

```
<checkpoint_dir>/<file_prefix>-<sha>.metadata.recalibrated.json
```

co-located with the original `<file_prefix>-<sha>.metadata.json`. The
original metadata file MUST stay byte-identical.

The recalibrated file's body is a **full copy of the original metadata
JSON** with **exactly one field substituted** (`sigma_train`). All other
top-level fields (`architecture`, `data_span`, `epochs_trained`,
`final_train_loss`, `final_val_loss`, `model_revision`, `tokenisation`,
`training`, `weights_sha256`) are copied verbatim. The `model_revision`
SHA does NOT recompute — by [ADR-0029 § 4](0029-tcn-checkpoint-provenance.md)
`model_revision` is computed over weights + canonical-architecture
descriptor; σ_train does not contribute.

**On-disk JSON shape**: the recalibrated file emits `sigma_train` as a
JSON **number** (not a string), matching the existing on-disk
convention (e.g. `"sigma_train":10.95425033569336` in the originals).
This **diverges from ADR-0029 § 2 — Canonicalisation rules § 5**, which
specifies float fields as string-encoded with `format!("{:.6}", x)` for
the `model_revision`-hashed canonical form.

The divergence is **intentional and load-bearing**: the on-disk metadata
file is the input to
[`TcnForecaster::load_anchor`](../../../../crates/forecast/src/tcn.rs)
which reads `sigma_train` via `.as_f64()` (works on JSON numbers, not
strings). A string-encoded sigma_train would silently coerce to `1.0`
(the default) and break the gate. Future forecaster phases inherit this
shape convention.

The recalibrated file's **key ordering + whitespace** still uses the
ADR-0029 canonicaliser
([`crates/forecast/src/provenance.rs::canonicalise`](../../../../crates/forecast/src/provenance.rs))
to preserve deterministic byte content across operators and platforms.

### D3. Loader-side opt-in via additive CLI flag (never auto-prefer)

Consumers of recalibrated metadata (e.g. the
`forecast_distribution` bin, future `forecast_eval_<family>` bins)
MUST consume the overlay file via an **explicit, additive CLI flag**:

```rust
/// Optional path to a .metadata.recalibrated.json overlay.
#[arg(long)]
metadata_path: Option<PathBuf>,
```

When the flag is omitted, the consumer uses the default
`load_anchor(scenario)` path (which reads the original `.metadata.json`).
When the flag is provided, the consumer uses
`load_from_paths(safetensors_path, &metadata_path)` to override only the
metadata source — the safetensors weights still come from the anchor.

The loader API itself (`TcnForecaster::load_anchor`) does **NOT**
auto-prefer `.metadata.recalibrated.json` if present. Reasons:

- **Anchor-byte preservation**: an auto-prefer rule would flip the
  predecessor's anchored report SHAs the moment a recalibrated file
  lands on disk. Explicit opt-in preserves the toggle: with overlay
  present + flag omitted, the predecessor's F4 reports are re-runnable
  verbatim. With overlay present + flag supplied, the new recalibrated
  reports are produced.
- **Auditability**: the explicit flag value surfaces in the report
  frontmatter (`metadata_path: <path>` advisory field), so the
  provenance chain is greppable.
- **Symmetry across phases**: v2.5a PatchTST + v2.5b Transformer
  inherit the same opt-in convention without needing per-family
  loader changes.

### D4. σ_train-not-in-safetensors invariant (codified as test)

For metadata-only recalibration to be feasible, σ_train MUST NOT appear
as a named tensor inside the safetensors weight stream. This invariant
is asserted by a unit test in each forecaster crate:

- TCN: `crates/forecast/tests/sigma_train_not_in_safetensors.rs`
  (introduced at `v25-tcn-recalibrate` T-D-N6).
- PatchTST (when it ships): equivalent test under
  `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs`.
- Vanilla Transformer (when it ships): ditto.

Each test parses the anchored safetensors header via
`safetensors::SafeTensors::deserialize` (no full tensor load) and
asserts no tensor name contains `sigma` / `output_scale` / `sigma_train`.
A future change that bakes a calibration scalar into the weight stream
breaks the test and forces the change-author to author a superseding
ADR.

## Alternatives considered

1. **In-place metadata rewrite** — overwrite the existing
   `.metadata.json` file with the corrected σ_train. **Rejected**: would
   flip the predecessor's anchored report SHAs
   (`forecast-distribution-bs{1,2}-realdata`), breaking the F4-baseline
   reproducibility contract and violating the
   `v25-tcn-recalibrate` R7 hard invariant. Operator-decide Q3 = (a)
   explicitly picked overlay-file naming over in-place rewrite for
   exactly this reason.

2. **σ_train as a tensor in safetensors** — bake the calibration scalar
   into the weight stream as a 1×1 tensor named `sigma_train`. **Rejected**:
   would force retraining (or a safetensors-edit, which itself is
   ill-defined under `model_revision` provenance). Q2 = (a) (metadata-only
   feasible) is the entire feature's load-bearing axis.

3. **Extend ADR-0033 § D3 with an F3' branch** — add a verdict-
   classifier rule like "F3' when gate-survival jumped from 0 to
   non-zero but `frac_inside_epsilon ≤ 0.5`." **Rejected**: ADR-0033
   § D3 is explicitly load-bearing-immutable
   ("This ADR does not amend its own thresholds — superseding ADR
   required"). Operator-decide Q4 = (a)+(c) picked the analyst default:
   keep the F-verdict algorithm immutable AND surface the gate-survival
   delta as a standalone `## Recalibration delta` body section,
   independent of F-label.

4. **Recalibrate via the F-evidence eval-span instead of training span** —
   use the held-out evaluation span's `r_hat` distribution (already
   measured in the predecessor's F4 reports' frontmatter:
   `std = 0.018015573` for BS-1, `std = 0.009976302` for BS-2) as the
   σ_train scalar directly. **Rejected (Q1 alternative (b))**: for BS-1
   the two spans coincide so this would work; for BS-2 they don't (BS-2
   trained Jan 2023 – Mar 2024; F-evidence evaluated Jan-Dec 2024).
   Mixing eval-span std into the training-time σ_train metadata is a
   subtle semantic blur. Q1 = (a) (re-derive on training span) is the
   canonical fix.

5. **Parameter-sweep σ_train across a range** — re-run
   forecast_distribution at each σ_train value and pick the one that
   maximises gate-survival. **Rejected (Q1 alternative (c))**: invites
   silent tuning and the right target metric is operator-dependent.
   Routes to a separate follow-on feature
   (`v25-tcn-threshold-tuning`) if needed.

## Consequences

### New files (this ADR + `v25-tcn-recalibrate` v0.1.0 scope)

- This file: `_bmad-output/planning-artifacts/architecture/decisions/0035-tcn-sigma-train-recalibration.md`.
- `crates/forecast/src/bin/recalibrate_sigma_train.rs` (~350 LoC, new bin
  per `v25-tcn-recalibrate` T-D-N1..T-D-N4).
- `crates/forecast/tests/recalibrate_sigma_train_readonly.rs` (~120 LoC, T-D-N5).
- `crates/forecast/tests/recalibrate_sigma_train_field_invariance.rs` (~80 LoC, T-D-N5).
- `crates/forecast/tests/sigma_train_not_in_safetensors.rs` (~40 LoC, T-D-N6).
- `crates/forecast/checkpoints/anchors/tcn-bs{1,2}-<sha>.metadata.recalibrated.json`
  (output of T-D-N3, byte-identical on 2-run check).
- `evidence/v1/v25-tcn-recalibrate/reports/recalibrate-sigma-train-bs{1,2}-20260521.md`
  (output of T-D-N4).
- `evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs{1,2}-realdata-recalibrated-20260521.md`
  (output of T-D-N8).

### Modified files

- `crates/forecast/src/bin/forecast_distribution.rs` (+12 lines): one
  additive `--metadata-path` CLI flag per D3.
- `crates/forecast/Cargo.toml` (+5 lines): `[[bin]]` entry for
  `recalibrate_sigma_train`.
- `_bmad-output/planning-artifacts/architecture/decisions/README.md` (+2 lines): registry row for
  ADR-0035.
- `evidence/anchors.toml` (+2 to +4 rows): new `forecast-distribution-bs{1,2}-realdata-recalibrated`
  anchors under version `v2.6.1-alpha-investigation-recalibrated`
  (additive only; 22 originals stay byte-identical).

### Cross-phase implications

- **v2.5a (PatchTST) alpha-investigation**, when it ships, inherits
  the D1 (post-training frozen forward pass) + D2 (overlay file
  naming) + D3 (additive CLI flag) + D4 (test invariant) contracts
  verbatim. Substitute `tcn-bs{1,2}` with `patchtst-bs{1,2}` and pick
  the PatchTST-family checkpoints; the rest is mechanical.
- **v2.5b (vanilla Transformer)** — ditto.
- **v2.6 bake-off** — comparing TCN / PatchTST / Transformer Sharpe
  across recalibrated checkpoints is well-defined because all three
  families share the σ_train semantic from D1.

### Negative precedent codified

The
[`train_tcn.rs:606,676-678,733-741`](../../../../crates/forecast/src/bin/train_tcn.rs)
in-loop accumulator pattern is **deprecated** for all future training
scaffolds in the v2.5 forecaster family. Future analyst passes
spawning `v25-patchtst-overlay` / `v25-tx-overlay` should code-read
their training scaffold for the same pattern and reject it at design
time. A `// DEPRECATED — see ADR-0035 § D1` comment lands at the bug
site in the same `v25-tcn-recalibrate` developer commit (Wave A
T-D-N3 — comment-only annotation; no functional change to the
training loop, which is not re-run in this feature).

### Enforced by

- `cargo test -p forecast --features candle --test sigma_train_not_in_safetensors`
  — D4 invariant.
- `cargo test -p forecast --features candle --test recalibrate_sigma_train_field_invariance`
  — D2 invariant (only `sigma_train` field changes between original
  and overlay).
- `cargo test -p forecast --features candle --test recalibrate_sigma_train_readonly`
  — read-only contract (no mutation of original `.metadata.json` or
  `.safetensors`).
- `bash scripts/verify_anchors.sh` — must report `ANCHORS PASS (22 /
  22)` before this ADR lands, `ANCHORS PASS (24 / 24)` (or `26 / 26`)
  after `v25-tcn-recalibrate` ships.

### What breaks if this is violated

- A developer overwrites `.metadata.json` in place (D2 violation) →
  predecessor anchors flip; `verify_anchors.sh` fails immediately;
  `forecast-distribution-bs{1,2}-realdata` SHAs diverge from
  `ef73cb8d…` / `d7cd08e6…`.
- A developer adds `auto-prefer` logic to `load_anchor` (D3 violation)
  → predecessor reports cease to be reproducible without explicit
  overlay deletion; auditability of "which metadata was active for
  this report" collapses.
- A future training scaffold reintroduces the per-batch accumulator
  pattern (D1 violation) → the resulting checkpoints will exhibit the
  same σ_train inflation; the `sigma_train_not_in_safetensors` test
  passes (because safetensors is unchanged) but the resulting
  forecaster will silence its gate. The deprecation-comment annotation
  at the `train_tcn.rs:606` bug site is the safety net.

### What this enables

- Metadata-only fix of the BS-1 + BS-2 σ_train inflation in
  **wall-clock hours**, not the multi-week retrain alternative
  (`v25-tcn-horizon-bump-or-retire`).
- A cross-phase contract for v2.5a/v2.5b that drops their σ_train
  calibration scope to "inherit ADR-0035 verbatim."
- A deterministic, anchor-additive workflow that preserves the F4
  evidence baseline (predecessor's 3 alpha-investigation anchors stay
  byte-identical and citeable).

## References

- [ADR-0028](0028-v25-dl-forecast-overlay-candle.md) — v2.5 forecaster
  framework decision (candle-based small custom models).
- [ADR-0029](0029-tcn-checkpoint-provenance.md) — checkpoint provenance
  + LFS-anchor + canonical JSON metadata schema. D2 of this ADR
  intentionally diverges from § 2 rule 5 (on-disk JSON number vs.
  string-encoded canonical form); the divergence is load-bearing.
- [ADR-0033](0033-tcn-alpha-investigation-report-shape.md) — F-verdict
  decision algorithm (§ D3). This ADR does NOT supersede ADR-0033;
  per `v25-tcn-recalibrate` Q4 = (a) the F-verdict algorithm stays
  immutable.
- [`docs/archive/pre-bmad-spec/v1/v25-tcn-recalibrate/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/v25-tcn-recalibrate/feature.md)
  — analyst R1-R8 + H1-H3 + K1-K5 + Q1-Q5 (operator-resolved
  2026-05-21).
- [`docs/archive/pre-bmad-spec/v1/v25-tcn-recalibrate/decomp.md`](../../../../docs/archive/pre-bmad-spec/v1/v25-tcn-recalibrate/decomp.md)
  — architect M-T1 decomposition (this ADR's wave-level decomposition).
- [`docs/archive/pre-bmad-spec/v1/v25-tcn-alpha-investigation/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/v25-tcn-alpha-investigation/feature.md)
  — predecessor F4 verdict.
- [`evidence/v1/v25-tcn-alpha-investigation/presentations/v25-tcn-alpha-investigation-2026-05-19.md`](../../../../docs/archive/presentations-2026-Q2.tar.gz)
  — presenter deck where the σ_train calibration anomaly was first
  surfaced as a top-level finding.
- Bug site: [`crates/forecast/src/bin/train_tcn.rs:606,676-678,733-741`](../../../../crates/forecast/src/bin/train_tcn.rs).
- Inference-time read sites:
  [`crates/forecast/src/tcn.rs:534,937`](../../../../crates/forecast/src/tcn.rs).
- Shipped loader APIs: `TcnForecaster::load_anchor`
  ([`tcn.rs:491`](../../../../crates/forecast/src/tcn.rs)),
  `TcnForecaster::load_from_paths`
  ([`tcn.rs:522`](../../../../crates/forecast/src/tcn.rs)).
- Canonicalise helper:
  [`crates/forecast/src/provenance.rs`](../../../../crates/forecast/src/provenance.rs).

## Changelog

- 2026-05-21 (architect, M-T1): initial accept. Covers `v25-tcn-recalibrate`
  v0.1.0 decomp. Codifies the post-training-recalibration contract
  across the v2.5 forecaster family (TCN now; PatchTST + vanilla
  Transformer inherit verbatim when they ship). Cross-refs
  `REQ-V25-TCN-RECALIBRATE-001` in
  [`_bmad-output/planning-artifacts/trace.toml`](../../../../_bmad-output/planning-artifacts/trace.toml).
