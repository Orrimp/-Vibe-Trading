---
slug: v25-tcn-overlay
status: in-progress
owner: analyst
updated: 2026-05-17
version: 2.5.0
parent: v25-dl-forecast-overlay v2.5.0 (roadmap)
predecessor: v2-llm-strategy v2.0.0
---

# v2.5 — TCN forecast overlay (phase 1 of 4)

> **First phase of the 4-phase DL roadmap** at
> [`v25-dl-forecast-overlay`](../v25-dl-forecast-overlay/feature.md).
> Model family: **Temporal Convolutional Network** (Bai, Kolter, Koltun
> 2018, *An Empirical Evaluation of Generic Convolutional and Recurrent
> Networks for Sequence Modeling*). Built first because (a) simplest
> architecture, fastest to a working baseline; (b) establishes the
> training loop + audit + replay infrastructure that phases v2.5a / v2.5b
> reuse; (c) deterministic inference (no autoregressive sampling) — easier
> to anchor and audit.

## Why

Per [ADR-0028](../architecture/adr/0028-v25-dl-forecast-overlay-candle.md)
and the [4-phase roadmap](../v25-dl-forecast-overlay/feature.md): train a
small TCN on crypto K-line data using `candle` (Apple Silicon Metal
backend) so that (a) the training loop, checkpoint provenance hashing,
audit emission, and replay-cache wiring exist as **reusable infrastructure**
for phases v2.5a (PatchTST) and v2.5b (vanilla Transformer); (b) the TCN
itself produces directionally useful forecasts on real crypto K-line data
against the v1 cross-sectional momentum baseline on BS-1 and BS-2; (c) the
operator learns dilated causal convolutions, residual blocks,
receptive-field math, and a `candle` training loop end-to-end. The
project goal frame is "real, working, auditable agent architecture; the
operator learns by building it" — TCN is the lowest-complexity DL
architecture that satisfies that frame and seeds the bake-off.

## Requirements

All requirements below answer one of the eight open questions seeded by
the orchestrator on 2026-05-17. Sources are
[Bai, Kolter, Koltun 2018](https://arxiv.org/abs/1803.01271) (hereafter
**BKK18**), the
[locuslab/TCN](https://github.com/locuslab/TCN) reference PyTorch
impl, the [Keras-TCN](https://github.com/philipperemy/keras-tcn) impl
notes, and
[`candle-transformers`](https://github.com/huggingface/candle/tree/main/candle-transformers).
The v2.5 invariants (data, scenarios, overlay shape, audit, cost,
hardware) are not re-derived here — they live in the
[4-phase roadmap](../v25-dl-forecast-overlay/feature.md) and
[architecture/12-forecast-overlay.md](../architecture/12-forecast-overlay.md).

### R1 — TCN topology (closes Q1)

The default topology is **8 stacked residual blocks** with dilation
schedule `[1, 2, 4, 8, 16, 32, 64, 128]`, kernel size **k = 3**, dropout
0.1, two dilated causal Conv1d layers per residual block + a 1×1 skip
connection. This is the BKK18 Section 3 / locuslab `TemporalConvNet`
default shape (`num_channels = [H]*8`); Keras-TCN documents the same
default (`nb_stacks=1, dilations=[1,2,4,8,16,32,64,128]`). Receptive
field is `1 + 2 * (k-1) * sum(dilations) = 1 + 4*255 = 1021` bars — at
hourly cadence this is ~42 days, comfortably above the operator's
v1-momentum lookback of 20 bars.

- **Operator decision needed?** No — architect-locks. Default matches two
  reference impls and exceeds context-window need.
- **Cost if wrong**: receptive field too short → underfit long-range
  structure; too long → wasted parameters. Both fixable in a v2.5.x
  re-train, not a re-architecture, since the shape is config-driven.

### R2 — Model size (closes Q2)

Default **channels per layer H = 96**, 8 blocks, dropout 0.1 →
~**4.4 M parameters** (estimate: 8 blocks × (2 conv layers × 96 ×
96 × 3 weights + 96 bias) + skips + final linear projection). Sits
comfortably inside the 5–10M Metal-backend ceiling locked by ADR-0028.

- **Operator decision needed?** No — architect-locks; the param count is
  a downstream consequence of R1 + H. Architect may bump H to 128
  (~7.8M params) if backtest signal is weak on BS-1; either fits the
  hardware budget.
- **Cost if wrong**: too small → underfits; too large → OOM on Metal or
  slow training. Both observable in M1 training-loop smoke test before
  M5 backtest.

### R3 — Tokenisation / target shape (closes Q3)

**Continuous regression** on next-bar log-return `r_{t+1} = ln(C_{t+1} /
C_t)` — single scalar per bar per symbol. Rejected: full-OHLCV
regression (overconstrains; close-to-close return is what the consuming
v1 momentum strategy actually cares about). Rejected: quantile
classification on this phase — it lands at v2.5a (PatchTST) per the
bake-off intent so each phase teaches a distinct paradigm.

Rationale: BKK18 frames TCN as a sequence-to-sequence regressor; the
locuslab `word_cnn` and `adding_problem` examples use continuous
regression heads with MSE/MAE; Keras-TCN README leads with regression.
Log-returns are stationary in distribution where raw prices are not —
standard practice in quantitative finance and consistent with our
existing v1 momentum signal which already operates on returns.

- **Operator decision needed?** No — architect-locks. Quantile
  classification is reserved for v2.5a so the bake-off contrasts
  paradigms cleanly.
- **Cost if wrong**: regression on returns is the most-cited TCN
  finance application; if it fails on crypto OHLCV that itself is
  signal for the v2.6 bake-off retirement decision.

### R4 — Context window (closes Q4)

**N = 256 bars** of hourly OHLCV (≈10.7 days). Well under the receptive
field of 1021 from R1 (so the TCN sees the full context every step),
and well above v1 momentum's 20-bar lookback so the model has room to
learn structure the baseline cannot. Features per bar: **5** —
log-return, log-range `(H-L)/C`, log-volume-z-scored,
hour-of-week sin/cos (so the model can learn weekly seasonality without
embedding a categorical hour).

- **Operator decision needed?** No — architect-locks. N=256 is a
  reference-impl default and clean power-of-two for batching.
- **Cost if wrong**: too short → can't see weekly structure; too long
  → wasted compute. Re-train cost only, no re-architecture.

### R5 — Loss function (closes Q5)

**Huber loss (δ = 0.001 on log-returns)**. Rationale: crypto log-returns
have heavy tails — MSE explodes on outliers (BKK18's word-prediction
examples used MSE on bounded targets; not our regime); MAE is robust
but biased toward the median and the gradient at zero is
discontinuous. Huber is the standard middle ground in quantitative
finance regression. δ=0.001 ≈ ten basis points; values above this are
treated as outliers (linear region), below it as well-behaved
(quadratic region).

- **Operator decision needed?** No — architect-locks. Operator can
  inspect the train/val loss curves in the M3 training-loop report and
  request a switch.
- **Cost if wrong**: MSE / MAE are one-line swaps in `candle`. Re-train
  cost only.

### R6 — Output → `ForecastOverlay` (closes Q6)

The TCN head emits **one scalar prediction `r_hat`** per call (the
final-timestep activation of the last residual block, after a 1×1 linear
projection to dimension 1). Translation to `ForecastOverlay`:

- `direction = Direction::Up` if `r_hat > +ε`,
  `Direction::Down` if `r_hat < -ε`, else `Direction::Flat`.
  Default `ε = 0.0005` (5 basis points; below transaction-cost noise
  floor — flips to Flat rather than emitting a trade-cost-negative
  signal).
- `confidence = clamp(|r_hat| / sigma_train, 0, 1)`, where
  `sigma_train` is the stdev of `r_hat` observed on the training set
  (pinned at checkpoint time, stored in checkpoint metadata). This
  gives a calibrated `[0, 1]` confidence even though the model emits
  a single point estimate.

Rejected: emitting the full predictive distribution. The TCN as
specified does not estimate variance — that would require either MC
dropout, ensembling, or a heteroscedastic head, none of which are needed
for the bake-off comparison. Phases v2.5a (quantile bins) and v2.5b
(autoregressive sampling) naturally produce distributions; the bake-off
in v2.6 tests whether the cheaper TCN point-estimate-with-confidence is
competitive.

- **Operator decision needed?** No — architect-locks. Calibration
  question is empirical and observable in the M4 inference report.
- **Cost if wrong**: confidence is mis-calibrated → too many high-confidence
  forecasts disagree with v1 → over-damping. Tunable via `sigma_train`
  recalibration at checkpoint time; no re-train.

### R7 — Training schedule (closes Q7)

Defaults, all overridable via `train_tcn.toml`:

- **Optimiser**: AdamW (β₁=0.9, β₂=0.999, weight_decay=1e-4) — BKK18
  Section 4 and locuslab default.
- **LR schedule**: OneCycle (max LR 1e-3, pct_start 0.3) — standard
  modern TCN recipe; Keras-TCN README recommends a cyclical schedule.
- **Batch size**: 128 sequences. Each sequence is 256 bars × 5 features.
  Fits in Metal-backend memory at H=96.
- **Epochs**: 30 with early stopping on val Huber loss (patience 5).
- **Validation split**: **rolling-window walk-forward on 2023** (train
  on Jan–Sep 2023, validate on Oct–Dec 2023) for the **BS-1**
  checkpoint; **train on 2023 full year, validate on Q1 2024, test on
  Q2–Q4 2024** for the **BS-2** checkpoint. **Two checkpoints, one per
  backtest scenario** — keeps each scenario strictly out-of-sample for
  its evaluation period.
- **Seed**: `0x00C0FFEE` (project fixture seed; ADR-0002).
- **Determinism**: candle CPU + Metal both must give bit-identical
  forward passes for the same seed + input — architect to verify in
  M2.

- **Operator decision needed?** No — architect-locks. The two-checkpoint
  split is the load-bearing call: it preserves out-of-sample integrity
  for both BS-1 and BS-2 without requiring k-fold (which would balloon
  training cost on Metal).
- **Cost if wrong**: training time and Sharpe both observable; tunable
  per-config without re-architecture.

### R8 — Checkpoint provenance (closes Q8)

`model_revision` is **SHA-256 over the canonical JSON of**:

```json
{
  "architecture": {"blocks": 8, "channels": 96, "kernel": 3,
                    "dilations": [1,2,4,8,16,32,64,128], "dropout": 0.1},
  "tokenisation": {"context_bars": 256, "features": ["logret","logrange",
                    "logvol_z","hour_sin","hour_cos"]},
  "training": {"optimiser":"adamw", "lr_max": 0.001, "schedule":"onecycle",
                "batch": 128, "epochs": 30, "loss": "huber", "huber_delta": 0.001,
                "seed": 12648430},
  "data_span": {"start":"2023-01-01T00:00Z","end":"2023-12-31T23:00Z",
                  "symbols":["ADA","AVAX","BNB","BTC","DOGE","DOT","ETH","LINK","SOL","XRP"],
                  "interval":"1h","source":"binance"},
  "weights_sha256": "<sha256 of the safetensors file body>"
}
```

Stored at **`crates/forecast/checkpoints/<sha>.safetensors`** plus a
sibling **`<sha>.metadata.json`** carrying the canonical JSON above
verbatim (so an inspector can compute the SHA without loading
safetensors) plus `sigma_train` (from R6) and the
training-run metrics (final train/val Huber, epoch count). Both files
ship gitignored except for fixture-anchored checkpoints, which are
LFS-tracked under `crates/forecast/checkpoints/anchors/`.

- **Operator decision needed?** **YES — Operator-decide question 1:**
  do anchor checkpoints get committed to LFS, or kept out of git and
  reproduced from the seed each time the anchor is verified? Either
  is defensible; the former trades repo size for verification speed,
  the latter trades training cost (~30 min on M-series per the early
  M3 estimates) for repo cleanliness. Recommendation: **LFS-track
  anchor checkpoints** to keep anchor verification fast and
  deterministic across hardware (Metal vs CPU bit-identity is not yet
  proven; if it breaks, retraining from seed would diverge anchors).
- **Cost if wrong**: anchor reproduction breaks across operator
  machines if Metal-vs-CPU bit-identity is not preserved AND we don't
  ship the weights. Easy to flip after first ship.

### R9 — Crate placement & file shape (architect-fills detail)

- `crates/forecast/src/tcn.rs` (new) — `TcnForecaster` impl behind
  `ForecastProvider` trait.
- `crates/forecast/src/bin/train_tcn.rs` (new) — training loop binary;
  reads parquet from `data/binance/`, writes checkpoint + metadata to
  `crates/forecast/checkpoints/<sha>.{safetensors,metadata.json}`.
- `crates/forecast/src/features.rs` (new) — pure-function OHLCV →
  5-feature window builder, shared between train and inference paths
  for determinism.
- `crates/forecast/Cargo.toml` — add `candle-core` + `candle-nn`
  dependencies; gate Metal under feature flag for CI portability.
- `crates/strategy/src/tcn_overlay_momentum.rs` (new) — the consuming
  strategy that combines v1 momentum signal with `TcnForecaster`
  output via the existing `overlay::combine()` helper.

### R10 — Strict-replay determinism (carry-forward)

Per [architecture/12](../architecture/12-forecast-overlay.md): TCN
inference results land in `crates/replay-cache/` namespace `"forecast"`
keyed by SHA-256 over canonical JSON of `(model_revision, OHLCV window,
sampling params, seed)`. `SamplingParams` for a deterministic TCN is
`{temperature=1.0, top_p=1.0, top_k=0, max_tokens=1, seed=0xC0FFEE}` —
the seed is part of the key but with deterministic argmax inference it
has no effect (kept for type-compatibility with v2.5b autoregressive).
In strict-replay mode a cache miss returns
`ForecastError::ReplayMiss`; in live mode the model runs.

### R11 — Audit emission (carry-forward)

Per [architecture/12](../architecture/12-forecast-overlay.md): one
`JournalEntry { kind: "forecast_emitted", … }` per inference call.
Payload JSON carries `ForecastOverlay` serde + `cache_hit: bool` +
`inference_ms: u64`. `model_revision` is the R8 SHA. No new audit
schema.

### R12 — Cost telemetry (carry-forward)

One `CostEvent::Infra { line: "forecast_inference", usd: 0, period:
PerCall }` per inference. Default `energy_cost_per_kwh = 0` keeps all
existing anchored reports byte-identical (per ADR-0019 Q11 + ADR-0022).

### Operator-decide questions (must answer before architect lock)

1. **Anchor checkpoint storage** (R8): LFS-track in
   `crates/forecast/checkpoints/anchors/` (recommended), or
   regenerate-from-seed on every anchor verification?
2. **Two-checkpoint vs one-checkpoint split** (R7): preferred default is
   two checkpoints (one per backtest scenario, each strictly OOS for its
   evaluation period). Confirm — or prefer a single checkpoint trained
   on 2023 used for both BS-1 (in-sample for BS-1 — invalid as written)
   and BS-2 (OOS)? The two-checkpoint default is the analyst's strong
   recommendation; explicit confirmation requested because the operator
   may have a different opinion on what counts as a fair backtest.

All other Q1-Q8 answers are architect-lockable without operator input.

## Design

_architect fills this after analyst handoff_

Carry-forward from [`architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md):

- Signal-level overlay on v1 cross-sectional momentum.
- `ForecastProvider::forecast()` async trait implemented by `TcnForecaster`.
- Strict-replay determinism via `crates/replay-cache/` (namespace
  `"forecast"`); cache key includes `model_revision`.
- Audit row per call: `JournalEntry { kind: "forecast_emitted", … }`.

Architect to lock: Conv1d block class layout, residual-skip projection
shape (1×1 vs identity), Metal-vs-CPU determinism strategy, parquet →
feature-window iterator design, training-loop checkpointing cadence,
metadata-JSON canonicalisation (sort_keys + no-whitespace),
`tcn_overlay_momentum` confidence-threshold default
(suggest `0.6` matching the existing `overlay::combine` test default).

## Backtest Scenarios

Per the [4-phase invariants](../v25-dl-forecast-overlay/feature.md#per-phase-invariants-carried-through-all-four):

- **BS-1 (2023 full-year top-10 USDT)** — TCN checkpoint trained on
  Jan–Sep 2023 (val Oct–Dec 2023), evaluated on the full 2023 year via
  walk-forward retraining cadence (architect to lock cadence: quarterly
  retrain is the analyst's recommendation).
- **BS-2 (2024 full-year top-10 USDT)** — TCN checkpoint trained on
  2023 full year (val Q1 2024), evaluated on Q2–Q4 2024 with quarterly
  walk-forward retrain.

Anchors `top10-2023-fy-tcn-overlay` and `top10-2024-fy-tcn-overlay`
locked at ship per the carry-forward anchor strategy.

Success criterion (vs v1 momentum baseline on the same scenario):

- **Sharpe ratio**: TCN overlay variant ≥ v1 baseline Sharpe + 0.10
  (10% relative improvement on a typical baseline Sharpe of ~1.0).
- **Max drawdown**: not worse than v1 baseline by > 2 percentage points.
- **Trade count**: not more than 1.5× v1 baseline (excessive trading is
  evidence of confidence mis-calibration).

If TCN fails all three on both BS-1 and BS-2, that is evidence for the
v2.6 bake-off retirement decision — not a failure of v2.5 ship.

## Implementation

_developer fills this — see R9 for crate placement skeleton_

Suggested milestone order (architect to lock in T-AR-2):

- **M0** — Feature pipeline (parquet → 5-feature 256-bar window) +
  property tests for determinism (same input → same output).
- **M1** — `TcnForecaster` struct + forward pass in `candle` against a
  random checkpoint; verify Metal-vs-CPU bit identity OR document
  divergence and gate determinism behind CPU backend.
- **M2** — Training loop in `train_tcn.rs`; smoke test on 1 epoch + 1
  symbol; verify checkpoint write + metadata.json shape + SHA
  reproducibility.
- **M3** — Full BS-1 training run; observe train/val Huber loss curves;
  tune H or epoch count if needed (architect-led).
- **M4** — Inference path: load checkpoint, build feature window, emit
  `ForecastOverlay`. Strict-replay cache wired. Audit + cost emission.
- **M5** — `tcn_overlay_momentum` strategy impl + integration into
  backtest harness. BS-1 dry-run.
- **M6** — Full BS-1 + BS-2 backtests with anchored seeds. Reports
  authored.
- **M7** — Anchor locks (`top10-2023-fy-tcn-overlay`,
  `top10-2024-fy-tcn-overlay`) + ship.

## Verification

_tester fills this — anchor-locks BS-1 + BS-2 at ship + smoke test of
training loop + inference reproducibility against the replay cache_

Tester contract (per AGENT.md):

- **Determinism**: 100 inference calls on the same OHLCV window must
  produce byte-identical `ForecastOverlay` serde JSON.
- **Replay**: BS-1 + BS-2 must each replay to byte-identical PnL on a
  second run from the locked anchor.
- **Anchor lock**: both new anchors land in `spec/anchors.toml` at ship.
- **Existing anchors**: 11 existing anchors stay byte-identical
  (default-zero cost telemetry + opt-in overlay strategy → no
  divergence on existing scenarios).

## Changelog

- 2026-05-17 (analyst): full analyst pass. Closed Q1-Q8 with defaults
  (R1-R12). Two operator-decide questions surfaced (anchor checkpoint
  storage, two-checkpoint backtest split). Sources cited:
  [BKK18](https://arxiv.org/abs/1803.01271),
  [locuslab/TCN](https://github.com/locuslab/TCN),
  [Keras-TCN](https://github.com/philipperemy/keras-tcn),
  [candle-transformers](https://github.com/huggingface/candle/tree/main/candle-transformers).
  Status: draft → in-progress. Owner: pending-analyst → analyst.
  HANDOFF → operator-decide (2 Qs) → architect.
- 2026-05-17 (orchestrator): phase 1 of 4 opened. Direction locked
  (TCN family). 8 open questions seeded for the analyst. Awaits
  analyst pass (task #25 will retarget at this phase).
