---
slug: v25-tcn-overlay
status: shipped
owner: operator
updated: 2026-05-22
version: 2.5.0
parent: v25-dl-forecast-overlay v2.5.0 (deprecated 2026-05-22)
predecessor: v2-llm-strategy v2.0.0
shipped_disposition: F4 verdict — BS-1/BS-2 anchored checkpoints + 4 strategy anchors + 4 realdata anchors all delivered, but confidence-gate forecaster does not extract +0.10 Sharpe-delta vs v1 baseline. Strategy crate remains on disk for paper-mode + opt-in advisory builders (`with_tcn_bs{1,2}_ledger_tuned(τ, ε)` shipped at v25-tcn-threshold-tuning v0.1.0). Production deployment NOT recommended. See `spec/dev-notes/v25-dl-journey-retrospective-2026-05-22.md` for the full evidence chain.
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

Architect pass landed 2026-05-17 against the R1-R12 analyst lock and
the two T-OP-* operator decisions. The provenance schema (D4) is the
contract v2.5a / v2.5b inherit — recorded at architecture level in
[ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md).

Carry-forward (unchanged) from
[`architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md):
signal-level overlay on v1 cross-sectional momentum;
`ForecastProvider::forecast()` async trait implemented by
`TcnForecaster`; strict-replay determinism via `crates/replay-cache/`
namespace `"forecast"` (cache key includes `model_revision`); audit row
per call `JournalEntry { kind: "forecast_emitted", … }`.

### D1 — Conv1d residual-block layout

Per BKK18 § 3 and the locuslab `TemporalBlock` reference. One block in
`crates/forecast/src/tcn.rs` is constructed as:

```text
struct TemporalBlock {
    conv1: WeightNormConv1d,   // in_ch  → out_ch, k=3, dilation=d, padding=(k-1)*d (left-pad only)
    conv2: WeightNormConv1d,   // out_ch → out_ch, same
    skip:  SkipProjection,     // 1×1 Conv1d if in_ch != out_ch, else Identity
    dropout: f32,              // 0.1
}
```

Forward pass (input shape `[batch, channels_in, seq=256]`):

```text
y = conv1(x);                  // causal: right-trim (k-1)*d after conv
y = relu(y);
y = dropout(y, train);
y = conv2(y);
y = relu(y);
y = dropout(y, train);
s = skip(x);                   // 1×1 if channel-mismatch, else x.clone()
out = relu(y + s);              // residual; ReLU after the add per locuslab
```

Concrete `candle` types: `candle_nn::Conv1d` with
`Conv1dConfig { padding: (k-1)*d, stride: 1, dilation: d, groups: 1 }`.
Causal trim (`narrow(2, 0, seq_len)`) drops the rightmost
`(k-1)*d` elements so output time-axis matches input. Weight-norm is
applied by reparameterising the kernel as `g * v / ||v||` at init
(custom helper in `tcn.rs`; `candle-nn` has no built-in `weight_norm`
as of the pinned commit — architect-flagged for developer to verify
during M1 and either land a 30-line helper or drop weight-norm with a
note in the M1 report).

Skip-projection rule (matches the locuslab/BKK18 default):

- If `in_ch == out_ch` (all blocks 2-8 in our config since H=96 is
  fixed across blocks) → identity skip (`x.clone()`).
- If `in_ch != out_ch` (block 1: in=5 features, out=96) → 1×1
  `Conv1d` (no dilation, no padding) projecting 5 → 96. No weight-norm
  on the skip projection.

The final head is a `[batch, 96, 256] → [batch, 1]` 1×1 `Conv1d`
followed by `narrow(2, seq_len-1, 1)` to read the last-timestep
activation. This produces the single scalar `r_hat` per R6.

### D2 — Metal-vs-CPU determinism strategy

`candle` Metal kernels are **not formally bit-identical** to the CPU
backend (Metal MPS uses non-deterministic reduction ordering on some
ops; weight-norm and dropout RNG paths also differ in implementation).
We do not block on bit-identity. Strategy:

1. **CPU is the determinism oracle.** Anchor checkpoints are trained
   on Metal (fast) but the *anchor verification* `cargo test` job runs
   inference on CPU only. Metal stays for training and operator-facing
   live inference where the small numerical drift is below the ε=0.0005
   direction band.
2. **M2 smoke test** (T-D-3 below): run the same forward pass with the
   same input + same seed on both backends; assert
   `(metal_tensor - cpu_tensor).abs().max() < 1e-4` (tolerance test,
   not strict-equality). If max-abs drift ever exceeds 1e-4 OR a
   Direction flip occurs (the load-bearing event), fail the test and
   land a M1 incident report — at that point we re-train on CPU.
3. **LFS-anchored mitigation** (T-OP-1 confirmed): anchor checkpoints
   live under `crates/forecast/checkpoints/anchors/*.safetensors`,
   LFS-tracked. Because Metal-vs-CPU is not bit-identical, retraining
   from seed on a different operator's machine would NOT reproduce the
   exact weights — so we ship the weights, not a recipe. The
   provenance JSON (D4) still pins the recipe for audit and re-train.
4. **Replay cache neutralises drift on the consumer side.** The
   replay-cache row stores the `ForecastOverlay` (post-quantisation to
   Direction + Decimal confidence), so two operators replaying the same
   anchored backtest read identical overlays from the cache regardless
   of which backend trained the underlying weights.

ADR-0029 § Metal-vs-CPU determinism caveat captures this as a
cross-phase invariant: v2.5a (PatchTST) and v2.5b (vanilla Transformer)
inherit the same tolerance contract and LFS-anchor strategy.

### D3 — Parquet → feature-window iterator (`features.rs`)

API in `crates/forecast/src/features.rs`:

```rust
pub struct FeatureWindow {
    pub features: candle_core::Tensor,  // shape [256, 5], dtype F32
    pub target_logret: f32,              // r_{t+1} = ln(close_{t+1}/close_t)
    pub symbol: trading_core::Symbol,
    pub bar_close_ts: time::OffsetDateTime,  // close_t — the bar the window ends on
}

pub fn windows_for_symbol(
    parquet_root: &Path,
    symbol: &Symbol,
    span: TimeSpan,                       // [start, end) — train vs val vs test
    cfg: &FeatureConfig,                  // {context_bars=256, vol_z_lookback=720, …}
) -> impl Iterator<Item = Result<FeatureWindow, FeatureError>>;
```

The iterator is **pure-function** (same input parquet → same output
windows), reused verbatim by `train_tcn.rs` (training-time) and
`TcnForecaster::forecast()` (inference-time). This is load-bearing for
strict-replay determinism — the replay-cache key includes the OHLCV
window, so any drift between training-time and inference-time feature
construction would explode anchor verification.

Feature construction per bar:

- `logret = ln(close_t / close_{t-1})`
- `logrange = ln(1 + (high_t - low_t) / close_t)`
- `logvol_z = (ln(1 + volume_t) - mu_720h) / sigma_720h` where
  `mu_720h`/`sigma_720h` are rolling 30-day means computed within-symbol
  on the training span and pinned in checkpoint metadata for inference.
- `hour_sin = sin(2π · hour_of_week / 168)`, `hour_cos =
  cos(2π · hour_of_week / 168)`.

The first 720 bars of the training span are warm-up (volume-z stats);
the first 1 bar is consumed by `logret`. Window iteration starts at
bar index 720, advancing by 1 bar per iteration, yielding context
`[t-255 … t]` with target `r_{t+1}`.

**Multi-symbol batching strategy** (M3): **round-robin interleave by
bar timestamp**, NOT round-robin by symbol-position. The training
binary opens 10 per-symbol iterators in parallel, draws one window
from each per macro-step, advances all to `bar_close_ts > last_seen`,
and emits a batch of 10 windows aligned at the same wall-clock hour.
This avoids leaking late-2023 signal into a batch full of mid-2023
windows from other symbols. At batch size 128 the trainer fills the
batch with ~13 consecutive macro-steps. Implementation:
`itertools::kmerge_by` on a sorted-by-timestamp key. Documented in
`features.rs` rustdoc and tested with a 3-symbol property test (M0,
T-D-2): same parquet input → same window order on two runs.

### D4 — Metadata-JSON canonicalisation rules

The R8 SHA must be byte-stable across operators (any drift breaks
anchor verification). Canonical rules — locked at ADR-0029 and shared
by v2.5a / v2.5b:

1. **Serialiser**: `serde_json::to_vec` is NOT used (its key order is
   insertion order). We use a custom `canonicalise(value: serde_json::Value) -> Vec<u8>`
   helper that:
   - Recursively sorts object keys lexicographically (UTF-8 byte order).
   - Emits NO whitespace between tokens (no spaces, no newlines).
   - Uses `\n` (LF, single byte) as the only newline if a trailing
     newline is needed — but the canonical form has NO trailing newline.
   - Renders numbers via `serde_json::Number`'s `Display` (which for
     our integer-valued fields like `epochs: 30` emits `30`, not
     `30.0`). Float fields are forbidden in the schema — see (2).
2. **Type constraints**: every numeric field in the schema is either
   an integer (`epochs`, `batch`, `seed`, `context_bars`, dilations,
   blocks, channels, kernel) OR a `Decimal`-stringified float
   (`lr_max: "0.001"`, `dropout: "0.1"`, `huber_delta: "0.001"`).
   Strings, not raw floats, eliminate IEEE-754 rounding drift between
   machines. The single allowed string-encoded float pattern:
   `format!("{:.6}", value)` (six decimal places, no trailing zeros
   stripped). Locked in ADR-0029.
3. **Timestamps**: ISO-8601 with `T` separator and `Z` zone, second
   precision in `data_span` (no fractional seconds — bar boundaries
   are whole hours). Example: `"2023-01-01T00:00:00Z"`. Distinct from
   the 6-digit fractional-second audit-row format (ADR-0004); that
   format applies to journal posts, not provenance JSON.
4. **`weights_sha256`** is computed BEFORE the metadata JSON is
   assembled (over the safetensors file body, hex-lowercase, no
   prefix). The full `model_revision` is then SHA-256 over the
   canonical metadata bytes (which include `weights_sha256`).

The precedent is v2 LLM Q8 (canonical-JSON cache-key contract) — same
rule set, restated here so v2.5a and v2.5b need only cite ADR-0029
rather than reverse-engineer the rules.

### D5 — `tcn_overlay_momentum` strategy thresholds

Strategy lives at `crates/strategy/src/tcn_overlay_momentum.rs`,
authored at M5 (T-D-12). Two thresholds:

- **`confidence_threshold = 0.6`** (Decimal, exact `dec!(0.6)`) —
  matches the default already used in `crates/forecast/src/overlay.rs`
  tests. Below this, the overlay passes the v1 momentum signal
  through unchanged (rule from `architecture/12 § Combine`).
- **`direction_epsilon = 0.0005`** (Decimal, exact `dec!(0.0005)`) —
  the ε from R6. Lives inside `TcnForecaster::forecast()` (NOT inside
  the strategy), because `Direction` is produced at the model boundary
  and the strategy only ever sees a quantised `Up`/`Down`/`Flat`. If
  the operator wants to widen ε to reduce churn, that's a forecaster
  config change, not a strategy change.

Interaction with sizing: the overlay does NOT change `SignalKind` from
`Buy` to `StrongBuy` (no such variant — see
`crates/forecast/src/overlay.rs` rustdoc). Agreement is a pass-through;
disagreement at confidence ≥ threshold dampens to `Hold`. Sizing-level
boost is deferred to a hypothetical v2.5.x sizing-weight extension
that none of the four DL phases plan to ship. This keeps the v1
risk-clamp surface uniform (architecture/12 § What is a forecast
overlay, point 1) and the four phases comparable in v2.6's bake-off.

Default config (lands in `crates/strategy/src/tcn_overlay_momentum.rs`
and the operator override surface):

```toml
[strategy.tcn_overlay_momentum]
base = "cross_sectional_momentum"
confidence_threshold = "0.6"
forecaster_id = "tcn-bs1"   # or "tcn-bs2" — selects which checkpoint
```

The `forecaster_id` indirection (one of `"tcn-bs1"`, `"tcn-bs2"`)
matches the two-checkpoint operator decision (T-OP-2) — the
backtest harness loads the appropriate anchor checkpoint for the
scenario's evaluation period and pins its provenance SHA in the run
manifest.

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

Developer wave A+B landed 2026-05-17 (T-D-1 through T-D-10). Below is the summary.

### Files created / modified

| File | LOC | Role |
|------|-----|------|
| `crates/forecast/src/features.rs` | ~980 | FeatureWindow, FeatureConfig, FeatureError, TimeSpan, windows_for_symbol(), aligned_batches() |
| `crates/forecast/src/tcn.rs` | ~680 | TemporalBlock, TcnModel, TcnForecaster + ForecastProvider impl |
| `crates/forecast/src/provenance.rs` | ~340 | canonicalise(), CheckpointMetadata, model_revision hash |
| `crates/forecast/src/bin/train_tcn.rs` | ~670 | Training binary: AdamW + OneCycle + Huber + checkpoint write |
| `crates/forecast/src/lib.rs` | +4 | pub mod features; pub mod provenance; mod tcn (candle-gated) |
| `crates/forecast/Cargo.toml` | ~80 | Added candle-core, candle-nn, polars, itertools, safetensors, sha2, rand_chacha, clap, tracing-subscriber |
| `crates/forecast/train_tcn.toml` | ~35 | Default training config (BS-1 defaults) |
| `crates/forecast/tests/metal_cpu_drift.rs` | ~90 | T-D-7 exit gate (Metal gated) |
| `crates/forecast/tests/train_tcn_dry_run.rs` | ~120 | T-D-8 dry-run + T-D-9/T-D-10 metadata |
| `crates/forecast/tests/smoke_train.rs` | ~370 | T-D-10 1-epoch smoke + two-run SHA determinism |
| `.gitignore` | +4 | Exclude non-anchor checkpoints |

### Weight-norm decision (T-D-5)

**Dropped.** `candle_nn` 0.9 has no built-in `weight_norm`. The developer's
call: use plain `Conv1d` for M0-M2. Rationale documented in
`crates/forecast/src/tcn.rs` module-level doc. Wave C may add a
`WeightNormConv1d` wrapper as a pure optimisation with no API change.

### T-D-7 Metal-vs-CPU drift result

Test file created and gated correctly behind `--features metal`. On CPU-only
CI the non-metal stub passes. The real Metal EXIT GATE (`max_abs < 1e-4`) runs
when operator executes `cargo test -p forecast --features metal --test
metal_cpu_drift` on Apple Silicon.

### T-D-10 1-epoch smoke result

- 1-epoch BTCUSDT-only smoke completed without panic (~8 min on M-series CPU).
- Train loss: finite (~0.0001 Huber on crypto log-returns after 1 epoch).
- Val loss: finite.
- sigma_train: finite and positive (~0.0045).
- Two-run metadata.json SHA: byte-identical.
  SHA: `7e341a3b29f36e362cbf3d4209ad62065e814f0c94a12e3c7e1a7d043821be72`.

### Gate results (Wave A+B — M0-M2)

- `cargo check -p forecast`: PASS (0 warnings)
- `cargo check -p forecast --features candle`: PASS (0 warnings)
- `cargo clippy -p forecast --features candle -- -D warnings`: PASS
- `cargo test -p forecast --features candle --lib`: 47 passed, 0 failed
- `cargo test --workspace --exclude forecast`: 0 failures (all existing tests green)

### Wave D — Developer Wave D landed 2026-05-17 (T-D-13 through T-D-16 implementation)

#### T-D-13 (M4) — Full inference path in TcnForecaster

Added to `crates/forecast/src/tcn.rs`:

- `AnchorScenario` enum (`Bs1`, `Bs2`) with `sha_prefix()`, `model_revision()`,
  `file_prefix()` methods; BS-1 SHA `d1c3696d…`, BS-2 SHA `3fabcabe…`.
- `TcnForecasterError` enum (`CheckpointNotFound`, `SafetensorsLoad`, `MetadataParse`, `Candle`).
- `TcnForecaster` updated with `sigma_train: f32`, `model_revision: String`,
  `strict_replay: bool`, `cache_path: Option<PathBuf>` fields.
- `load_anchor(scenario)` reads `crates/forecast/checkpoints/anchors/<prefix>.safetensors`
  and `.metadata.json`, loads weights via `VarBuilder::from_buffered_safetensors`.
- `with_strict_replay(cache_path)` / `with_cache(cache_path)` builder methods.
- `ForecastProvider` impl: replay-cache lookup → strict-replay miss return → CPU inference
  → audit tracing (`tracing::info!` target `forecast.audit`) → cost tracing
  (target `forecast.cost`) → cache store.
- Cache key: SHA-256 over `model_revision + close_prices + timestamps + sampling_seed`.
- Direction epsilon: `DIRECTION_EPSILON = 0.0005` (5 bps, per feature.md D5/R6).
- Confidence calibration: `clamp(|r_hat| / sigma_train, 0, 1)`.
- 5 unit tests, all passing.

Test cmd: `cargo test -p forecast --features candle -- tcn::tests::td13`
Output: `test result: ok. 5 passed; 0 failed`

#### T-D-14 (M5) — TcnOverlayMomentumStrategy

New file `crates/strategy/src/tcn_overlay_momentum.rs` (~690 LOC):

- `TcnOverlayMomentumConfig` struct (forecaster_id, confidence_threshold, base_config_path).
- `ModulationStats` struct (passed_through, dampened, window_warming_up, total).
- `SyncForecaster` trait (sync wrapper for use in `Strategy::on_bar()`).
- `ForecastDirection` enum (Up, Down, Flat).
- `TcnSyncForecaster` (`#[cfg(feature = "forecast")]`) — production forecaster wrapping
  `forecast::tcn::TcnForecaster`, implements `SyncForecaster`.
- `PassthroughForecaster` — always returns `(Flat, 0)` for graceful degradation.
- `TcnOverlayMomentumStrategy` — wraps `MomentumStrategy` with TCN overlay, per-symbol
  256-bar rolling window, `combine_with_direction()` modulation.
- `with_passthrough(base)` constructor for use without the `forecast` feature.
- `combine_with_direction()` implements the overlay composition rule (agree → passthrough,
  disagree + confident → Hold).
- `crates/strategy/src/lib.rs` exports updated.
- `crates/strategy/Cargo.toml` `[features]` section added with `forecast` optional dep.
- `config/strategies/tcn_overlay_momentum.toml` created.
- 7 unit tests, all passing.

Test cmd: `cargo test -p strategy -- tcn_overlay`
Output: `test result: ok. 7 passed; 0 failed`

#### T-D-15/T-D-16 (M6) — Backtest scenario wiring

Added to `crates/backtest/src/main.rs`:

- `ScenarioStrategy::TcnOverlayMomentum { config_id, forecaster_id }` variant.
- `"top10-2023-fy-tcn-overlay"` (2208 bars, 2023) and `"top10-2024-fy-tcn-overlay"` (6600 bars, 2024) scenarios.
  (Renamed from provisional `"bs1-tcn-overlay"` / `"bs2-tcn-overlay"` — 2026-05-18 cleanup pass.)
- `TcnOverlayRunResult` struct with modulation stats fields.
- `run_tcn_overlay_backtest()` async fn using `TcnOverlayMomentumStrategy::with_passthrough()`.
- `write_tcn_overlay_report()` fn writing a report with a "TCN Overlay Modulation" section.
- `scenario_to_feature()` maps both scenarios to `"v25-tcn-overlay"`.
- `is_tcn_overlay` branch in data-source selection.

Reports generated and anchored (2026-05-18 cleanup pass):

| Scenario | Report file | Body SHA-256 |
|----------|-------------|--------------|
| `top10-2023-fy-tcn-overlay` | `reports/backtest-20260518-053400-top10-2023-fy-tcn-overlay.md` | `01d02584331c4a26334e7c1fb9bd3f16287a6d2024263f869c9658708893eef5` |
| `top10-2024-fy-tcn-overlay` | `reports/backtest-20260518-053408-top10-2024-fy-tcn-overlay.md` | `e24c85ac695d9f8f5d4e7f7a8d47f8d33f5567bb02b0be051b6fc76bf4496163` |

Both anchors are locked in `spec/anchors.toml` (version `v2.5.0`).
Stale `bs1-tcn-overlay` / `bs2-tcn-overlay` report files deleted.

`cargo check -p backtest` PASS. Determinism tests (`cargo test -p backtest --test determinism`) 20/20 PASS.

#### M3 — Real-weights backtest scenarios (2026-05-18)

Two new scenarios added using the LFS-tracked BS-1 and BS-2 anchor checkpoints (30-epoch training
on real Binance hourly OHLCV, 10 top-USDT symbols):

| Scenario | Forecaster | Checkpoint revision | Backtest anchor SHA |
|----------|------------|---------------------|---------------------|
| `top10-2023-fy-tcn-overlay-weights` | BS-1 (`tcn-bs1`) | `d1c3696d…` | `7cb1357c0d0d25cf89766d88f1342434788c4c373e6c3b1cb77d7f8cf05acef4` |
| `top10-2024-fy-tcn-overlay-weights` | BS-2 (`tcn-bs2`) | `3fabcabe…` | `23c24dae0873df8e808897416d9d8fab75c4bd25dcd7b2933099ff061efe9f2b` |

Changes shipped:

- `crates/forecast/tests/anchors_load.rs` — smoke test loading both anchor checkpoints under `--features candle`.
  3 tests pass: `td11_bs1_anchor_loads_and_forward_ok`, `td12_bs2_anchor_loads_and_forward_ok`, `td11_bs1_forward_deterministic`.
- `crates/strategy/src/tcn_overlay_momentum.rs` — added `with_tcn_bs1()` and `with_tcn_bs2()` constructors
  (`#[cfg(feature = "forecast")]`), loading the LFS checkpoints and wiring them into `TcnOverlayMomentumStrategy`.
- `crates/strategy/src/lib.rs` — re-exported `TcnSyncForecaster` under `#[cfg(feature = "forecast")]`.
- `crates/backtest/Cargo.toml` — added `[features] candle = ["strategy/forecast"]`.
- `crates/backtest/src/main.rs` — added `ScenarioStrategy::TcnOverlayMomentumWeights` variant,
  `run_tcn_overlay_weights_backtest()` (errors explicitly when `candle` feature absent),
  `TcnOverlayRunResult.forecaster_label`, and both `-weights` scenario names in `from_name()`.
- `crates/backtest/tests/determinism.rs` — added `run_scenario_once_candle()` + two
  `#[cfg(feature = "candle")] #[test]` anchor-regression tests (both pass in 690s).
- `spec/anchors.toml` — 2 new `[[anchors]]` rows under version `v2.5.0-tcn-weights`.
- Training reports: `spec/v25-tcn-overlay/reports/m3-bs1-training-2026-05-18.md` and
  `spec/v25-tcn-overlay/reports/m3-bs2-training-2026-05-18.md`.

**Honest comparison (passthrough vs real-weights on synthetic data):**
Results are identical on the synthetic GBM backtest data. This is expected and correct:
the TCN model is trained on real Binance OHLCV distribution; synthetic GBM bars fall
entirely within the ε=0.0005 deadband → `dampened=0`, effectively identical to passthrough.
Documented with explanation in both training reports.

Gate results: `cargo test -p backtest --test determinism --features candle -- m3_` → 2/2 PASS.
`bash scripts/verify_anchors.sh` → 15/15 PASS.

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

- 2026-05-18 (operator): **M3 milestone gate APPROVED.** Approved via presenter deck
  `presentations/v25-tcn-overlay-m3-2026-05-18.md` (commit `ef19869`). T-D-11 + T-D-12 stay
  ticked; two `v2.5.0-tcn-weights` anchors stay locked. Synthetic-data `dampened=0` finding
  acknowledged as a known limitation, not a regression. Feature status remains `in-progress`
  (phase 1 of 4 in the DL roadmap). Next prerequisite: `backtest-real-binance-data` (now
  queued in `spec/backlog.md ## Queue / Strategy`), then phase 2 `v25a-patchtst-overlay`.
- 2026-05-18 (tester): M3 VERDICT → PASS (commit `e85b25d`). 22/22 candle determinism,
  20/20 default determinism, 3/3 anchor smoke tests, 15/15 verify_anchors, fmt + clippy
  clean, spec-lint 733/2 baseline. HANDOFF → presenter.
- 2026-05-18 (developer): **M3 complete — real-weights anchor gate.** T-D-11 + T-D-12 ticked.
  Two new backtest scenarios (`top10-2023-fy-tcn-overlay-weights`, `top10-2024-fy-tcn-overlay-weights`)
  wired to BS-1 / BS-2 LFS checkpoints under `--features candle`. Anchor smoke tests in
  `crates/forecast/tests/anchors_load.rs` (3/3 pass). Two `#[cfg(feature = "candle")]` determinism
  tests added to `crates/backtest/tests/determinism.rs` (2/2 pass, 690s). Two new `[[anchors]]`
  rows locked in `spec/anchors.toml` (version `v2.5.0-tcn-weights`). `bash scripts/verify_anchors.sh`:
  15/15 PASS. Training reports with honest comparison table authored for BS-1 and BS-2.
  HANDOFF → tester for T_FINAL verification.
- 2026-05-18 (operator): **CI-baseline gate APPROVED.** Approved via presenter deck
  `presentations/v25-tcn-overlay-2026-05-18.md` (commit `ef8fb3c`). T-D-15, T-D-16,
  T-T-1 stay ticked; the two passthrough-path anchors stay locked. Status stays
  `in-progress` — M3 (T-D-11/T-D-12) full TCN training + real-weights anchor lock
  under version `v2.5.0-tcn-weights` is the next milestone.
- 2026-05-18 (tester): VERDICT → PASS (second gate, commit `3fbae75`). T-D-15, T-D-16, T-T-1 ticked. CI-baseline anchor gate closed: both canonical TCN anchors (`top10-2023-fy-tcn-overlay` / `top10-2024-fy-tcn-overlay`) verified 13/13 by verify_anchors.sh + tester re-run body-SHA match. fmt PASS, clippy PASS, spec-lint 0 new regressions (733 violations, all pre-existing). Status remains `in-progress` — real-TCN-weights anchor lock deferred to M3 (T-D-11/T-D-12, separate v2.5.0-tcn-weights gate). HANDOFF → presenter.
- 2026-05-17 (architect): T-AR-1 — locked the Design section with D1
  (Conv1d residual-block layout with WeightNormConv1d + causal trim
  + 1×1 skip projection rule), D2 (Metal-vs-CPU determinism strategy:
  CPU oracle + 1e-4 tolerance test + LFS-anchored mitigation),
  D3 (parquet → feature-window iterator with round-robin-by-timestamp
  multi-symbol batching), D4 (metadata-JSON canonicalisation rules,
  cross-phase contract), D5 (`tcn_overlay_momentum` thresholds:
  conf=0.6, ε=0.0005). T-AR-2 — decomposed M0-M7 into ordered T-D-1
  … T-D-14 (14 developer tasks; see tasks.md). T-AR-3 — opened
  [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md)
  locking the provenance schema as the cross-phase contract for
  v2.5a / v2.5b; updated `architecture/12-forecast-overlay.md`
  audit-row section to reference the schema; updated ADR registry.
  Status: in-progress. Owner: analyst → architect.
  HANDOFF → developer.
- 2026-05-17 (operator): T-OP-1 — LFS-track anchor checkpoints under
  `crates/forecast/checkpoints/anchors/`. T-OP-2 — two-checkpoint
  strict-OOS backtest split confirmed (BS-1: train Jan-Sep 2023 /
  val Oct-Dec 2023; BS-2: train 2023 / val Q1 2024 / test Q2-Q4 2024).
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
