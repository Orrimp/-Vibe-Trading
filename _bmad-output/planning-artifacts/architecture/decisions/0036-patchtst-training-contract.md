---
adr: 0036
title: PatchTST training contract — patch-embed shape, σ_train post-training pattern, candle attention determinism gate, cost tripwire (v2.5a)
status: proposed
date: 2026-05-21
supersedes: none
superseded-by: none
---

# ADR-0036: PatchTST training contract (v2.5a)

## Context

[ADR-0028](0028-v25-dl-forecast-overlay-candle.md) commits the v2.5
DL-forecaster slot to four phases trained in candle; this ADR locks
the **phase-2 (v2.5a — PatchTST per Nie et al 2022)** training contract.
[ADR-0029](0029-tcn-checkpoint-provenance.md) locks the cross-phase
provenance schema; this ADR extends it additively for PatchTST-specific
architecture fields.
[ADR-0033](0033-tcn-alpha-investigation-report-shape.md) § D3 locks the
F-verdict algorithm; this ADR re-states that the algorithm is
**immutable** for PatchTST reports.
[ADR-0034](0034-cockpit-training-control.md) locks the
`train_events` audit emission; this ADR adds `model_family = "patchtst"`
to the variant set.
[ADR-0035](0035-tcn-sigma-train-recalibration.md) § D1 locks the
**post-training frozen-weights forward-pass σ_train derivation** as a
cross-phase contract; this ADR cites D1 verbatim and adds an
architect-locked code-review check that the deprecated in-loop
accumulator at
[`train_tcn.rs:606,676-678,733-741`](../../../../crates/forecast/src/bin/train_tcn.rs)
is **not** replicated.

The
[`v25a-patchtst-overlay v0.1.0`](../../../../docs/archive/pre-bmad-spec/v1/v25a-patchtst-overlay/feature.md)
feature.md R1-R10 + the architect M-T1 decomp at
[`v25a-patchtst-overlay/decomp.md`](../../../../docs/archive/pre-bmad-spec/v1/v25a-patchtst-overlay/decomp.md)
spell the build-out. This ADR is the single citable contract for
PatchTST that the developer code-reviews against, that the tester gates
against at M-FINAL, and that v2.5b vanilla Transformer + v2.6 bake-off
inherit when they ship.

The phase-2 ship is **paradigm test, not parameter sweep** — the v2.5
TCN journey (closed 2026-05-21 via operator's `v25-tcn-horizon-bump-or-retire`
Q1=(b) retirement decision) shipped joint T-MARGINAL with Sharpe-deltas
of +0.018 / +0.045, well below the +0.10 T-ALPHA-UNLOCKED threshold.
PatchTST shifts inductive bias (patch-attention vs dilated convolution)
on the same data; whether it clears the threshold is the H1 question.

## Decision

### D1. PatchTST architecture skeleton (anchor-friendly canonical shape)

The v2.5a PatchTST model implementation under
`crates/forecast/src/patchtst.rs` locks this layer stack (Nie et al
2022 § 3.1, channel-independence per § 3.2):

```text
Input:    [batch, channels=5, time=336]
  │
  ▼  PatchEmbed (patch_len=16, stride=8)
       — Tensor::unfold(2, 16, 8) → [batch, channels, n_patches=41, patch_len=16]
       — Linear projection [patch_len → d_model=128]
       — Output: [batch, channels=5, n_patches=41, d_model=128]
  │
  ▼  + LearnablePositionEncoding [n_patches, d_model]  (broadcast over batch + channels)
       — Output: [batch, 5, 41, 128]
  │
  ▼  Reshape (channel-independence per Nie § 3.2)
       — Output: [batch * channels = batch * 5, n_patches=41, d_model=128]
  │
  ▼  TransformerEncoder × n_layers=3
       │
       │ Each block (pre-LN order; Nie et al § 3.1):
       │   y = LayerNorm(x)
       │   y = MultiHeadSelfAttention(y, n_heads=4)   ← custom impl, ~50 LoC
       │   x = x + y                                   ← residual 1
       │   y = LayerNorm(x)
       │   y = Linear(d_model, d_ff=256).gelu()
       │   y = Linear(d_ff=256, d_model)               ← FFN
       │   x = x + y                                   ← residual 2
       │
       — Output: [batch * 5, 41, 128]
  │
  ▼  Reshape back
       — Output: [batch, 5, 41, 128]
  │
  ▼  Flatten last 2 dims per channel  →  [batch, 5, 41 * 128 = 5248]
  │
  ▼  ProjectionHead: Linear (5 * 5248 → 1) over flattened channel-stack
       — Output: [batch, 1]  (scalar r_hat per sample)
```

**Lock points.**
- **Patch dimensions** — `patch_len = 16`, `stride = 8` (Q3=(a)). Yields
  `n_patches = floor((336 - 16) / 8) + 1 = 41` raw, plus padding-by-1
  per Nie et al § 3.1 → 42 in some published configs; the architect
  locks the **41-patch (no extra pad)** variant for v0.1.0 simplicity
  (the `+1` patch is sometimes added by Nie's reference impl via a
  reflection-pad before unfold; we elide it).
- **d_model = 128, n_heads = 4, d_ff = 256, n_layers = 3, dropout =
  0.2** (PatchTST/42 small config; Nie et al § 4.2).
- **Channel-independence** — `[batch, 5, 41, 128]` reshapes to
  `[batch * 5, 41, 128]` for the encoder, then reshapes back. The
  encoder sees each channel's patch tokens independently — there is no
  cross-channel attention inside the model. Cross-channel mixing
  happens at the ProjectionHead (flatten + linear over `5 * 41 * 128`
  features → 1 scalar).
- **Pre-LN order** (Nie et al § 3.1) over post-LN (original Vaswani
  2017). Pre-LN is the modern default and the published PatchTST
  reference impl uses pre-LN.
- **Activation** — GELU in the FFN (Nie et al default).
- **Position encoding** — learnable parameter `[n_patches=41,
  d_model=128]` over sinusoidal (Nie et al ETT default). Architect
  rejects sinusoidal because learnable PE generalises better when the
  lookback window is fixed (it is here).
- **Final output** — single scalar `r_hat` per sample, shape `[batch,
  1]`, matching `TcnModel::forward`'s output shape so `overlay::combine()`
  is reused verbatim.

**Parameter count target.** ~410k params (small custom config; ~10×
smaller than TCN's 4.4M). See decomp.md § T-AR-1 for the
layer-by-layer breakdown. Well under the ADR-0028 5-10M ceiling. The
architect codifies a developer-callable test
`patchtst::tests::parameter_count_estimate()` that asserts `300_000 <
model.num_parameters() < 600_000`.

### D2. Canonical-architecture descriptor extension (ADR-0029-compatible)

The metadata JSON's `architecture` and `tokenisation` sub-objects (per
ADR-0029 § 1) extend additively for PatchTST. Schema for a PatchTST
checkpoint:

```json
{
  "architecture": {
    "model_family": "patchtst",
    "patch_len": 16,
    "stride": 8,
    "d_model": 128,
    "n_heads": 4,
    "d_ff": 256,
    "n_layers": 3,
    "dropout": "0.200000",
    "context_len": 336
  },
  "tokenisation": {
    "context_bars": 336,
    "target_horizon_bars": 24,
    "features": ["logret","logrange","logvol_z","hour_sin","hour_cos"]
  },
  "training": {
    "optimiser": "adamw",
    "lr_max": "0.001000",
    "schedule": "onecycle",
    "batch": 128,
    "epochs": 30,
    "loss": "huber",
    "huber_delta": "0.001000",
    "seed": 12648430
  },
  "data_span": {"start": "2023-01-01T00:00:00Z", "end": "2023-12-31T23:00:00Z",
                  "symbols": ["ADA","AVAX","BNB","BTC","DOGE","DOT","ETH","LINK","SOL","XRP"],
                  "interval": "1h", "source": "binance"},
  "weights_sha256": "<sha256 of safetensors body>",
  "sigma_train": <JSON number, NOT string — per ADR-0035 § D2 on-disk convention>,
  "metrics": {"final_train_huber": "<f>", "final_val_huber": "<f>",
               "epochs_run": <int>}
}
```

**Schema delta vs ADR-0029.**
- `architecture.model_family = "patchtst"` (NEW required field; TCN
  checkpoints emit `"tcn"` going forward — this is **a forward-additive
  field**. Existing TCN safetensors + metadata files DO NOT need
  regeneration; the existing TCN canonicaliser at
  [`crates/forecast/src/provenance.rs`](../../../../crates/forecast/src/provenance.rs)
  is unchanged, so existing TCN `model_revision` SHAs stay identical.
  For PatchTST checkpoints, `model_family` is part of the canonicalised
  descriptor that contributes to `model_revision`).
- `architecture.{patch_len, stride, d_model, n_heads, d_ff, n_layers,
  dropout, context_len}` replace TCN's `{blocks, channels, kernel,
  dilations, dropout}`.
- `tokenisation.target_horizon_bars` is NEW (default 1 for TCN to
  preserve byte-identity; explicit 24 for PatchTST per Q4=(b)).
- Everything else (training, data_span, weights_sha256, sigma_train as
  on-disk JSON number per ADR-0035 § D2, metrics) is unchanged.

**Canonicalisation rules (ADR-0029 § 2) carry forward verbatim**: keys
sorted lexicographically, no whitespace, integer fields integer, float
fields string-encoded `format!("{:.6}", x)` except `sigma_train` (which
follows ADR-0035 § D2 on-disk JSON number convention).

**Existing TCN checkpoints' `model_revision` SHAs do NOT change.** The
canonicaliser at `crates/forecast/src/provenance.rs` only sees a
PatchTST descriptor when it's hashing a PatchTST checkpoint; TCN
descriptors are byte-identical to their pre-this-ADR form.

### D3. σ_train post-training derivation (ADR-0035 § D1 cross-phase contract)

ADR-0035 § D1 applies **verbatim** to PatchTST. Quote:

> Every v2.5/v2.5a/v2.5b forecaster checkpoint that exposes a
> `sigma_train` scalar in its metadata must compute that scalar via a
> **dedicated frozen-weights forward pass over the training-data span**,
> NOT via an in-loop accumulator across training epochs.

Concrete instantiation for `crates/forecast/src/bin/train_patchtst.rs`:

1. After the per-epoch training loop converges (typically epoch 30 per
   the AdamW + OneCycle defaults), the bin proceeds to the σ_train
   derivation block.
2. Load the just-written safetensors via the candle `VarBuilder`
   in `Device::Cpu` mode + `train = false` (no dropout, no BN
   running-mean update).
3. Iterate `windows_for_symbol()` over the training span (`2023-01-01..2023-12-31`
   per Q6=(a)) for all 10 symbols.
4. Collect all `r_hat` scalars into a single `Vec<f32>` (declared
   INSIDE the σ_train derivation block scope — NOT outside the per-epoch
   training loop).
5. Compute population std with f64 intermediates and `1e-8` floor (per
   ADR-0035 § D1):
   ```rust
   let mu = r.iter().map(|&x| x as f64).sum::<f64>() / n as f64;
   let var = r.iter().map(|&x| (x as f64 - mu).powi(2)).sum::<f64>() / n as f64;
   let sigma = (var.sqrt().max(1e-8)) as f32;
   ```
6. Write the canonical `patchtst-bs1-<sha>.metadata.json` with the
   computed σ_train scalar (on-disk JSON number per ADR-0035 § D2). NO
   `.metadata.recalibrated.json` overlay file is emitted at v0.1.0 ship
   (PatchTST's σ_train is canonical-at-ship time, not retrofitted).

**Architect's code-review check.** Before approving the M-D handoff,
the architect (or developer, self-audit) greps `train_patchtst.rs` for:

```bash
# Negative check — no in-loop accumulator declared outside per-epoch scope.
grep -nE '^\s*let mut [a-z_]+:?\s*Vec<f32>\s*=\s*Vec::new\(\)' crates/forecast/src/bin/train_patchtst.rs
```

Expected: **zero matches outside the post-training σ_train derivation
block**. (The σ_train block itself contains a `let mut r_hats: Vec<f32>
= Vec::with_capacity(...)` declaration; that's expected and correct
because it's the post-training-pass buffer, not the per-epoch
accumulator.)

### D4. Cost tripwire (K1 / R8)

The PatchTST BS-1 training run is the longest-running task in v0.1.0
(~5-7 days wall-clock on Apple Silicon Metal). Two cost-tripwire
levels:

1. **Hard limit** — a single epoch's wall-clock time exceeds 24 hours.
   → developer pauses the run, emits a diagnostic dump to
   `/tmp/train_patchtst-bs1-tripwire-epoch{N}.txt`, and escalates to
   the operator before continuing. Implemented via
   `assert_epoch_budget(epoch_n, epoch_wall_clock_sec, history)` helper
   per decomp.md § T-AR-8.
2. **Median-multiple limit** — epoch N's wall-clock exceeds 3× the
   rolling median of epochs 1..N-1. → same diagnostic + escalation
   procedure.

**Tripwire policy on fire**: **the bin continues training** (the
operator owns the "stop or continue" decision after escalation —
automatic stop would lose multi-day progress if the operator wants to
investigate a transient).

**Auditability**: each tripwire fire writes a `train_events` row with
`kind = "tripwire_warning"` per ADR-0034 schema (additive variant; no
schema migration; the cockpit panel surfaces tripwire rows as a
yellow annotation per ADR-0034 § Q4).

### D5. K2 candle-attention determinism gate

The PatchTST MultiHeadSelfAttention is the **most novel** candle code
in the v2.5 family — TCN's `Conv1d` stack is battle-tested by the v2.5
ship + the broader candle ecosystem; transformer self-attention has
more moving parts (scaled dot-product, softmax over a non-trivially-
shaped tensor, head-dimension reshapes).

**Determinism gate.** Wave A.5 of decomp.md ships
`crates/forecast/tests/forward_determinism_patchtst.rs`:

```rust
#[test]
fn forward_determinism_cpu() {
    let device = candle_core::Device::Cpu;
    let forecaster = PatchTstForecaster::random_init_with_seed(device, 0x00C0FFEE)?;
    let x = fixed_seed_input();
    let y1 = forecaster.forward(&x, /*train=*/ false)?;
    let y2 = forecaster.forward(&x, /*train=*/ false)?;
    let delta = (y1 - y2)?.abs()?.max_all()?.to_scalar::<f32>()?;
    assert_eq!(delta, 0.0, "CPU forward pass must be byte-deterministic");
}

#[test]
fn forward_metal_cpu_drift() {
    let cpu_device = candle_core::Device::Cpu;
    let metal_device = match candle_core::Device::new_metal(0) {
        Ok(d) => d,
        Err(_) => return,  // skip on non-Metal hardware (CI)
    };
    let weights = random_seeded_weights(0x00C0FFEE);
    let cpu_forecaster = PatchTstForecaster::from_weights(weights.clone(), cpu_device)?;
    let metal_forecaster = PatchTstForecaster::from_weights(weights, metal_device)?;
    let x_cpu = fixed_seed_input_on(cpu_device);
    let x_metal = fixed_seed_input_on(metal_device);
    let y_cpu = cpu_forecaster.forward(&x_cpu, false)?;
    let y_metal = metal_forecaster.forward(&x_metal, false)?.to_device(cpu_device)?;
    let drift = (y_cpu - y_metal)?.abs()?.max_all()?.to_scalar::<f32>()?;
    assert!(drift < 1e-4, "Metal-vs-CPU drift {drift} exceeds 1e-4 tolerance (ADR-0029 § 4)");
}
```

**On gate failure.** If `forward_determinism_cpu` fails, the
implementation is buggy (non-deterministic CPU path); developer fixes
before Wave B. If `forward_metal_cpu_drift` fails (delta ≥ 1e-4), the
LFS-anchor strategy from ADR-0029 § 4 becomes load-bearing — Wave B
ships the BS-1 safetensors via LFS, and the determinism oracle
(`Device::Cpu`) is the only one CI's anchor-verification gate runs
against. (This is the predecessor's policy; nothing changes.)

### D6. Anchor strategy (Q7=(a))

Two anchors land under version `v2.5a.0-patchtst`:

```toml
[[anchors]]
scenario = "forecast-distribution-patchtst-bs1-realdata"
version  = "v2.5a.0-patchtst"
sha256   = "<locked at Wave E by tester>"

[[anchors]]
scenario = "top10-2023-fy-patchtst-overlay-realdata"
version  = "v2.5a.0-patchtst"
sha256   = "<locked at Wave E by tester>"
```

The 28 predecessor anchors stay byte-identical (K4 + R9 from
feature.md). `sharpe-comparison-patchtst-bs1-realdata` is NOT
anchored at v0.1.0 — defer to v2.6 bake-off to lock the
cross-model comparison family (mirror of the v25-tcn-overlay
precedent at 2026-05-19).

BS-2 PatchTST checkpoint + its anchors defer to v0.1.1 per Q2=(a)
analyst recommendation.

### D7. Strategy integration (Q8=(a))

`crates/strategy/src/patchtst_overlay_momentum.rs` ships as a **sibling**
to `tcn_overlay_momentum.rs`, NOT as a refactor of the existing TCN
strategy. The 4 builder pairs per feature.md § R6:

```rust
// In crates/strategy/src/patchtst_overlay_momentum.rs
impl PatchTstOverlayMomentumStrategy {
    #[cfg(feature = "forecast")]
    pub fn with_patchtst_bs1(base: MomentumStrategy)
        -> Result<Self, forecast::patchtst::PatchTstForecasterError> { ... }

    #[cfg(feature = "forecast-audit-tick")]
    pub fn with_patchtst_bs1_ledger(base: MomentumStrategy, ledger: audit::Ledger)
        -> Result<Self, forecast::patchtst::PatchTstForecasterError> { ... }

    #[cfg(feature = "forecast")]
    pub fn with_patchtst_bs1_tuned(base: MomentumStrategy,
                                     confidence_threshold: Decimal,
                                     direction_epsilon: Decimal)
        -> Result<Self, forecast::patchtst::PatchTstForecasterError> { ... }

    #[cfg(feature = "forecast-audit-tick")]
    pub fn with_patchtst_bs1_ledger_tuned(base: MomentumStrategy,
                                            ledger: audit::Ledger,
                                            confidence_threshold: Decimal,
                                            direction_epsilon: Decimal)
        -> Result<Self, forecast::patchtst::PatchTstForecasterError> { ... }
}
```

Strategy ID: `"patchtst_overlay_momentum"`. The existing 8 TCN
builders + `tcn_overlay_momentum.rs` body stay byte-identical (K6).

Future refactor to model-agnostic via `Box<dyn ForecastProvider>`
defers to v0.1.1 or v2.6 bake-off when there's evidence to motivate
it — feature.md § Q8 rejected the refactor at v0.1.0 because any
code change in `combine()` is an SHA risk for the existing 4
`-realdata` anchors.

## Alternatives considered

1. **iTransformer (Liu et al 2023) over PatchTST** — variates-as-tokens
   instead of patches-as-tokens. **Rejected (Q1 alternative (b))** —
   the 5-feature input is too narrow to amortise variate-as-token's
   overhead; PatchTST's patch-as-token captures temporal patterns more
   naturally for the per-bar `r_hat` overlay shape.

2. **Hybrid PatchTST + iTransformer ensemble at v0.1.0** — train both;
   average at the `r_hat` level. **Rejected (Q1 alternative (c))** —
   doubles training cost; the v2.6 bake-off is the designed canonical
   ensemble decision point.

3. **`candle_transformers::*` primitives over custom MultiHeadSelfAttention** —
   reuse upstream encoder blocks. **Rejected** — larger surface for
   K2 determinism gate; the v2.5 TCN ship's experience with custom
   `TemporalBlock` (~80 LoC) shows that small bespoke blocks
   minimise external-API drift risk and are easier to step-through-debug.
   The custom MHSA at ~50 LoC is comparable in size + scope.

4. **Sinusoidal position encoding over learnable** — Vaswani 2017
   default. **Rejected** — Nie et al's PatchTST/42 ETT default is
   learnable PE; learnable generalises better when the lookback
   window is fixed (it is here at 336 bars).

5. **In-place σ_train accumulator (the deprecated TCN pattern)** —
   declare a `Vec<f32>` outside the per-epoch loop, append per-batch
   inside the loop, std-compute at end-of-training. **Rejected per
   ADR-0035 § D1 verbatim** — the predecessor's σ_train inflation
   bug at `train_tcn.rs:606,676-678,733-741` is the canonical
   negative precedent.

6. **Train BS-1 + BS-2 in parallel at v0.1.0** — 2 checkpoints in
   parallel on 2 GPUs. **Rejected (Q2 alternative)** — Apple Silicon
   Metal has one GPU per machine; sequential is the only path. BS-2
   defers to v0.1.1 per Q2=(a) analyst recommendation (H1 falsification
   on BS-1 alone routes to v2.6 retirement; H1 confirmation on BS-1
   makes BS-2 a follow-on commitment, not a v0.1.0 prerequisite).

7. **Extend `target_horizon_bars` via a CLI flag without changing
   `FeatureConfig`** — pass the horizon as an argument to
   `windows_for_symbol()`. **Rejected** — the iterator's `min_bars`
   check needs the horizon at construction time, not per-call;
   `FeatureConfig` is the natural carrier. Per decomp.md § T-AR-3.

## Consequences

### New files (this ADR + v25a-patchtst-overlay v0.1.0 scope)

- This file: `_bmad-output/planning-artifacts/architecture/decisions/0036-patchtst-training-contract.md`.
- `crates/forecast/src/patchtst.rs` (~700 LoC; PatchTstModel + PatchTstForecaster + tests).
- `crates/forecast/src/bin/train_patchtst.rs` (~600 LoC; training scaffold).
- `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs` (~40 LoC; D3-derived).
- `crates/forecast/tests/forward_determinism_patchtst.rs` (~80 LoC; D5-derived).
- `crates/forecast/tests/tcn_byte_identity.rs` (~60 LoC; K6 scope-creep guard).
- `crates/forecast/tests/patchtst_overlay_neutrality.rs` (~80 LoC; K4 anchor neutrality).
- `crates/forecast/checkpoints/anchors/patchtst-bs1-<sha>.{safetensors,metadata.json}` (Wave B output; LFS-tracked).
- `crates/strategy/src/patchtst_overlay_momentum.rs` + `patchtst_sync.rs` (~250 + ~80 LoC; sibling strategy + sync wrapper).
- `crates/backtest/src/scenarios/patchtst_overlay_weights.rs` (~180 LoC; sibling backtest scenario).
- `evidence/v1/v25a-patchtst-overlay/reports/{forecast-distribution-patchtst-bs1-realdata,top10-2023-fy-patchtst-overlay-realdata,sharpe-comparison-patchtst-bs1-realdata}-20260521.md` (Wave D outputs).

### Modified files

- `crates/forecast/src/lib.rs` (+1 line): `pub mod patchtst;`.
- `crates/forecast/src/features.rs` (+6 lines): `target_horizon_bars` field on `FeatureConfig`; default 1 for TCN byte-compat.
- `crates/forecast/Cargo.toml` (+5 lines): `[[bin]] name = "train_patchtst"`.
- `crates/forecast/src/bin/forecast_distribution.rs` (~30 lines): additive `Scenario::PatchtstBs1` enum arm.
- `crates/forecast/src/bin/sharpe_comparison.rs` (~10 lines): additive PatchTST source-path in frontmatter `sources` list.
- `crates/strategy/src/lib.rs` (+2 lines): 2 new `pub mod` decls.
- `crates/backtest/src/scenarios/mod.rs` (~5 lines): additive scenario enum arm.
- `_bmad-output/planning-artifacts/architecture/decisions/README.md` (+2 lines): registry row for ADR-0036 + changelog entry.
- `evidence/anchors.toml` (+2 anchor rows at end-of-file; 28 originals byte-immutable).

### Cross-phase implications

- **v2.5b (vanilla Transformer)** when it ships inherits the D2
  canonical-arch descriptor extension pattern (substitute
  `model_family = "transformer"` + decoder-only fields) + D3 σ_train
  pattern + D4 cost tripwire + D5 candle-attention determinism gate
  verbatim. The patch-specific D1 layer stack is replaced with the
  vanilla decoder-only stack.
- **v2.6 bake-off** compares TCN / PatchTST / Transformer Sharpe across
  three model families, all consuming the same σ_train semantic (D3 +
  ADR-0035 § D1) so the comparison is well-defined.

### Negative precedent codified

The PatchTST training scaffold MUST NOT replicate the deprecated
in-loop σ_train accumulator pattern from
[`train_tcn.rs:606,676-678,733-741`](../../../../crates/forecast/src/bin/train_tcn.rs)
(ADR-0035 § Negative precedent codified). The architect's grep-based
code-review check at D3 closes this loop.

### Enforced by

- `cargo test -p forecast --features candle --test sigma_train_not_in_safetensors_patchtst`
  — D3 invariant (no σ_train tensor in safetensors).
- `cargo test -p forecast --features candle --test forward_determinism_patchtst`
  — D5 invariant (CPU determinism + Metal-vs-CPU drift < 1e-4).
- `cargo test -p forecast --features candle --test forecast_distribution_verdict`
  — F-verdict algorithm IMMUTABLE per ADR-0033 § D3.
- `cargo test --workspace --test tcn_byte_identity` — K6 scope-creep
  guard.
- `cargo test --features candle --test patchtst_overlay_neutrality`
  — K4 anchor neutrality.
- `bash scripts/verify_anchors.sh` — 26 PASS + 2 known-glob-collision-FAIL PRE-lock; 28 PASS + 2 known-FAIL POST-lock (additive +2).
- Manual grep check at architect M-D-approval time: no `Vec<f32>::new()`
  declaration outside the per-epoch loop scope in `train_patchtst.rs`.

### What breaks if this is violated

- **D1 violated** (e.g. cross-channel attention added inside the
  encoder) → PatchTST loses Nie et al's channel-independence
  benefit; the `r_hat` distribution shape diverges from the
  paper's published benchmarks; the H2 attention-pattern hypothesis
  becomes confounded.
- **D2 violated** (e.g. existing TCN `model_revision` SHAs change
  because the canonicaliser is refactored to include `model_family`
  in TCN's descriptor) → the 4 anchored TCN `-realdata` anchors
  flip; `verify_anchors.sh` fails; the 28-original byte-immutability
  contract breaks.
- **D3 violated** (e.g. PatchTST training scaffold replicates the
  in-loop σ_train accumulator) → the resulting checkpoint exhibits
  the same σ_train inflation as the pre-recalibrate TCN BS-1/BS-2;
  the gate-survival drops to ~0; F-verdict prematurely reports F2
  on what may be a recoverable signal.
- **D4 violated** (e.g. no cost tripwire) → Wave B runs unbounded;
  multi-week compute budget overruns; K1 fires silently.
- **D5 violated** (e.g. forward_determinism_patchtst skipped or
  ignored) → CPU/Metal divergence above the ε deadband; anchored
  backtest reproducibility breaks across operators.
- **D6 violated** (e.g. anchors land but at the wrong version pin)
  → cross-feature anchor namespace pollution; v2.6 bake-off comparison
  becomes hard to query.
- **D7 violated** (e.g. PatchTST refactor touches
  `tcn_overlay_momentum.rs`) → K6 scope-creep; the existing 4
  `tcn-overlay`-family anchors at risk of SHA flip.

### What this enables

- A pure-additive, anchor-safe phase-2 ship: 28 predecessor anchors
  byte-identical; 2 new anchors land additively.
- A cross-phase contract that v2.5b vanilla Transformer + v2.6
  bake-off inherit verbatim with mechanical substitutions.
- A determinism-strong training scaffold that the predecessor's
  σ_train inflation cannot reoccur on — the architect's grep-based
  code-review check is the safety net.
- A K1-bounded compute envelope (24h hard limit + 3× median tripwire)
  that prevents Wave B from silently consuming multi-month wall-clock.

## References

- [ADR-0028](0028-v25-dl-forecast-overlay-candle.md) — v2.5
  DL-forecast overlay framework (candle).
- [ADR-0029](0029-tcn-checkpoint-provenance.md) — cross-phase
  provenance schema (D2 extends this additively).
- [ADR-0032](0032-backtest-realdata-path-and-revision-pin.md) — realdata
  path + REVISION.toml (PatchTST scenarios inherit verbatim).
- [ADR-0033](0033-tcn-alpha-investigation-report-shape.md) § D3 —
  F-verdict algorithm IMMUTABLE (PatchTST reports use same priority
  tree).
- [ADR-0034](0034-cockpit-training-control.md) — training-events
  emission (D4 cost tripwire emits via this contract).
- [ADR-0035](0035-tcn-sigma-train-recalibration.md) § D1 — σ_train
  post-training derivation cross-phase contract (D3 of this ADR
  cites verbatim).
- [`docs/archive/pre-bmad-spec/v1/v25a-patchtst-overlay/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/v25a-patchtst-overlay/feature.md)
  — R1-R10 / H1-H4 / K1-K6 / Q1-Q8 (analyst pass 2026-05-21).
- [`docs/archive/pre-bmad-spec/v1/v25a-patchtst-overlay/decomp.md`](../../../../docs/archive/pre-bmad-spec/v1/v25a-patchtst-overlay/decomp.md)
  — architect M-T1 decomp (sibling deliverable to this ADR).
- Nie, Y., Nguyen, N.H., Sinthong, P., Kalagnanam, J. (2022). *A Time
  Series is Worth 64 Words: Long-term Forecasting with Transformers.*
  arXiv:2211.14730. https://arxiv.org/abs/2211.14730
- Reference implementation:
  [yuqinie98/PatchTST](https://github.com/yuqinie98/PatchTST) (MIT
  license; PyTorch — referenced for layer-stack correctness, not
  copied).
- Existing TCN mirror sources:
  [`crates/forecast/src/tcn.rs`](../../../../crates/forecast/src/tcn.rs),
  [`crates/forecast/src/bin/train_tcn.rs`](../../../../crates/forecast/src/bin/train_tcn.rs),
  [`crates/strategy/src/tcn_overlay_momentum.rs`](../../../../crates/strategy/src/tcn_overlay_momentum.rs),
  [`crates/backtest/src/scenarios/tcn_overlay_weights.rs`](../../../../crates/backtest/src/scenarios/tcn_overlay_weights.rs).

## Changelog

- 2026-05-21 (architect, M-T1): initial proposal. Status `proposed`
  pending developer's M-D ship + tester's M-FINAL pass. Cross-refs
  `REQ-V25A-PATCHTST-001` in
  [`_bmad-output/planning-artifacts/trace.toml`](../../../../_bmad-output/planning-artifacts/trace.toml). Flips to `accepted` at
  M-FINAL on the M-FINAL operator-approval tick (mirror of the
  ADR-0035 pattern, which was accepted at `v25-tcn-recalibrate`
  M-FINAL).
