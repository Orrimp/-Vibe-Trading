---
slug: v25a-patchtst-overlay
status: shipped
owner: operator
updated: 2026-05-21
version: 0.1.0
parent: v25-dl-forecast-overlay v0.0.0 (roadmap)
predecessor: v25-tcn-horizon-bump-or-retire v0.1.0
---

# v2.5a — PatchTST forecast overlay (phase 2 of 4)

> **Phase 2 of the 4-phase DL roadmap** at
> [`v25-dl-forecast-overlay`](../v25-dl-forecast-overlay/feature.md).
> Model family: **patch-based Transformer** (Nie, Nguyen, Sinthong,
> Kalagnanam 2022 *A Time Series is Worth 64 Words* — PatchTST).
> Activates after operator decided **Q1 = (b) retire v2.5 TCN at 1h
> horizon** at the
> [`v25-tcn-horizon-bump-or-retire`](../v25-tcn-horizon-bump-or-retire/feature.md)
> scope-decision gate 2026-05-21. The multi-week budget that would
> have funded a TCN horizon-bump retrain pivots here.

> **Predecessor v2.5 TCN journey (closed 2026-05-21):**
>
> 1. [`v25-tcn-overlay v2.5.0`](../v25-tcn-overlay/feature.md) — TCN
>    overlay shipped; BS-1 (`d1c3696d…`) + BS-2 (`3fabcabe…`)
>    anchored checkpoints; 22 anchors landed.
> 2. [`backtest-real-binance-data v0.1.0`](../backtest-real-binance-data/feature.md)
>    — TCN overlay on real Binance hourly OHLCV reported
>    `dampened=0` across all four `-realdata` scenarios.
> 3. [`v25-tcn-alpha-investigation v0.1.0`](../v25-tcn-alpha-investigation/feature.md)
>    — F4 verdict (no signal at 1h horizon) under
>    [ADR-0033 § D3](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md);
>    σ_train calibration anomaly surfaced.
> 4. [`v25-tcn-recalibrate v0.1.0`](../v25-tcn-recalibrate/feature.md)
>    — σ_train 608× / 580× inflation eliminated via the metadata
>    overlay path; gate-survival jumped 0% → 40-89%; F-verdict
>    legitimately stays F4 under the immutable D3 priority tree
>    (`frac_inside_epsilon` 0.031/0.057 ≪ 0.5 F3 threshold).
>    [ADR-0035](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
>    locked the post-training σ_train recalibration contract
>    **across all v2.5 phases including PatchTST (D1)**.
> 5. [`v25-tcn-threshold-tuning v0.1.0`](../v25-tcn-threshold-tuning/feature.md)
>    — joint T-MARGINAL (BS-1 +0.018 / BS-2 +0.045 Sharpe-delta at
>    τ=0.1, ε=0.001; both well below the +0.10 T-ALPHA-UNLOCKED
>    threshold from [v25-tcn-overlay § success criterion](../v25-tcn-overlay/feature.md)).
> 6. [`v25-tcn-horizon-bump-or-retire v0.1.0`](../v25-tcn-horizon-bump-or-retire/feature.md)
>    — operator decided Q1 = (b) **retire v2.5 TCN at 1h horizon;
>    pivot multi-week budget to PatchTST**. Zero code change, zero
>    new anchor, 28 originals byte-identical.

## Why

The v2.5 TCN journey exhausted the 1h-horizon TCN-shape hypothesis on
real Binance hourly OHLCV. Three independent diagnostic ships
(alpha-investigation, recalibrate, threshold-tuning) converged on the
same verdict: TCN at 1h horizon has no learnable alpha for the
cross-sectional momentum overlay. The operator's
[Q1=(b) retire-promote decision](../v25-tcn-horizon-bump-or-retire/feature.md#q1--primary-scope-hard-blocker--no-safe-default)
chose **a different inductive bias** over more TCN horizon-sweep:
the next experiment is a different model family on the same data, same
backtest scenarios, same overlay composition.

### PatchTST inductive bias vs TCN

| Architecture | Inductive bias | Receptive field | Parameter shape |
|--------------|----------------|-----------------|-----------------|
| **TCN** (Bai 2018; v2.5 — retired at 1h) | dilated causal convolutions; local-to-distant via stacked dilations | 1021 bars ≈ 42 days | ~4.4M (8 blocks × 96 channels) |
| **PatchTST** (Nie et al 2022; this phase) | patch tokens + transformer self-attention; channel-independence | full attention across patches in the lookback window | configurable (analyst defaults below; ~5-10M target per ADR-0028) |
| iTransformer (Liu et al 2023; phase 2 alternative — Q1) | each variable is a token; attention across variables | full attention across feature channels | smaller per-channel (variates as tokens) |
| Vanilla decoder-only Transformer (phase 3; v2.5b) | autoregressive token-by-token over discretised OHLCV | causal mask | per-vocab |

The **patch-attention** paradigm is meaningfully orthogonal to TCN's
dilated convolutions. PatchTST chunks contiguous bars into "patches"
(e.g. `patch_len=16` bars), embeds each patch into a token, then runs
standard transformer self-attention across patch tokens within each
channel independently (Nie et al's "channel-independence" claim is
load-bearing for time-series — they argue cross-channel attention
**hurts** on most benchmarks). On crypto OHLCV with intraday session
structure (Asia / Europe / US trading sessions, weekend dampening),
**patch-attention may capture session-level structure that TCN's
smoothly-dilated convolution missed**.

This is a **paradigm test**, not a parameter sweep — if PatchTST also
F4s at the same horizon, that's evidence (jointly with TCN) that the
1h cross-sectional momentum overlay shape is structurally incapable of
extracting alpha on this data, and the v2.6 bake-off retirement gate
fires with high confidence.

### Quantitative-finance context (carry-forward from predecessor)

Per [`v25-tcn-horizon-bump-or-retire § Quantitative-finance context`](../v25-tcn-horizon-bump-or-retire/feature.md):

- Hourly crypto log-return std is ~0.005-0.015 (well within
  transaction-cost noise).
- 24h crypto log-return std is ~0.025-0.040 (roughly √24 ≈ 4.9× under
  i.i.d.; empirically 3-4× due to negative serial autocorrelation).
- Daily-cadence signal exists on the universe (the v1 cross-sectional
  momentum baseline operating on 20-bar lookback already extracts
  small alpha) — the open question is whether PatchTST's
  patch-attention can extract a 24h-forward prediction that the v1
  baseline's 20-bar-backward lookback cannot.

The retirement decision routes phase 2 to **chase signal at 24h
horizon** rather than re-test 1h. Reasons (Q4 below): (i) 1h already
exhausted; (ii) 24h log-return std exceeds the transaction-cost noise
floor by 3-4×; (iii) PatchTST's published ETT benchmarks use
multi-day forecast horizons — the architecture is well-matched to
multi-bar forward windows. **Q4 default = 24h horizon**, with H1
testable: PatchTST extracts ≥+0.10 Sharpe-delta on a (τ, ε) sweep,
where the v2.5 TCN at 1h failed to.

## Carry-forward invariants (from parent roadmap)

Per [`v25-dl-forecast-overlay`](../v25-dl-forecast-overlay/feature.md)
and shared infrastructure already on disk:

- **Same data**: 10 USDT pairs
  (ADA/AVAX/BNB/BTC/DOGE/DOT/ETH/LINK/SOL/XRP), hourly bars,
  2023 + 2024 full year, bootstrapped via
  [`crates/data/src/bin/fetch_binance_klines.rs`](../../crates/data/src/bin/fetch_binance_klines.rs).
- **Same backtest scenarios**: BS-1 (2023 full-year top-10 USDT
  hourly), BS-2 (2024 full-year top-10 USDT hourly).
- **Same overlay shape**: signal-level overlay on v1 cross-sectional
  momentum baseline per
  [`spec/architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md);
  the existing `overlay::combine()` pure function is model-agnostic.
- **Same `ForecastProvider` trait**:
  [`crates/forecast/src/lib.rs:42`](../../crates/forecast/src/lib.rs) —
  PatchTST implements the same async trait as TCN; existing audit and
  cost wiring carry forward.
- **Same audit shape**: `JournalEntry { kind: "forecast_emitted", … }`
  with `model_revision` SHA pinned per ADR-0029 (canonical-arch
  descriptor extended with PatchTST-shape fields per D2 below).
- **Same cost telemetry**: `CostEvent::Infra { line: "forecast_inference", … }`
  default-zero dollars.
- **Same hardware constraint**: Apple Silicon M-series via candle
  Metal backend; ~5-10M params per model ceiling per ADR-0028.
- **ML framework**: candle (no Python, no PyTorch, no ONNX) per
  ADR-0028.
- **σ_train contract**: post-training forward-pass derivation per
  [ADR-0035 § D1](../architecture/adr/0035-tcn-sigma-train-recalibration.md#d1-recalibrate-σ_train-in-a-frozen-weights-post-training-forward-pass)
  — **explicitly applies to v2.5a PatchTST**. The in-loop accumulator
  pattern from
  [`train_tcn.rs:606,676-678,733-741`](../../crates/forecast/src/bin/train_tcn.rs)
  is deprecated; PatchTST training scaffold MUST use the
  post-training pattern from the start (negative precedent codified
  in ADR-0035 § Negative precedent codified).
- **F-verdict algorithm**: ADR-0033 § D3 IMMUTABLE; PatchTST
  forecast-distribution reports use the same F1/F2/F3/F4 priority
  tree verbatim. The bin
  [`crates/forecast/src/bin/forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs)
  reuses the algorithm via the additive `--metadata-path` CLI flag
  (D3 of ADR-0035).

## Requirements (R1-R10)

> **MVP scope** — code + 1 BS-1 PatchTST checkpoint + F-verdict
> report (analyst-recommended Q2=(a)). Reach goals (BS-2 checkpoint,
> hyperparameter search, ensemble with TCN) defer to v0.1.1. The
> v0.1.0 ship is operator-decide-bounded by Q1, Q2, Q3, Q4, Q5, Q6,
> Q7, Q8 below — all defaults are analyst-recommended; "autoapprove"
> activates them.

### R1 — PatchTST model implementation (closes Q1)

A new file `crates/forecast/src/patchtst.rs` lands in the same
`forecast` crate as the existing TCN. The model implements PatchTST
per Nie et al 2022 in candle:

- **Input shape**: `[batch, channels, time]` matching the v2.5 TCN
  convention (`crates/forecast/src/tcn.rs:843`). For PatchTST the
  `channels` axis is the **input features** (5 per Q5(a)), the `time`
  axis is the lookback window (Q3-determined).
- **Patch embedding**: split the `time` axis into patches of length
  `patch_len` with stride `stride`. Each patch becomes a token of
  shape `[batch, channels, n_patches, patch_len]`; a linear
  projection of shape `[patch_len → d_model]` produces token
  embeddings `[batch, channels, n_patches, d_model]`.
- **Positional embedding**: learnable position encoding of shape
  `[n_patches, d_model]` added to the patch embeddings.
- **Channel-independent backbone**: reshape to
  `[batch * channels, n_patches, d_model]` and run a standard
  transformer encoder (multi-head self-attention + feed-forward +
  pre-LN, per Nie et al's PatchTST/42 default config). The
  channel-independence claim from Nie et al § 3.2 is load-bearing:
  cross-channel attention happens implicitly via overlay
  composition, not inside the model.
- **Projection head**: flatten the encoder output and project to a
  single scalar `r_hat` matching the TCN's output shape so
  `overlay::combine()` is reused verbatim.
- **Hyperparameter defaults** (analyst-locked, architect-confirm at
  M-T1 with operator-decide override via Q3-Q5):
  - `patch_len = 16` (Nie et al ETT default; per Q3(a)).
  - `stride = 8` (50% overlap; per Q3(a)).
  - `n_heads = 4`, `d_model = 128`, `d_ff = 256`, `n_layers = 3`,
    `dropout = 0.2` (PatchTST/42 small config; ~1.5-2M params at
    these defaults — well under the 5-10M ceiling).
  - `context_len = 336` (Nie et al ETT lookback default for h=24;
    ≈14 days at hourly; well below the v2.5 TCN's 1021-bar RF so
    the architect can confirm the comparison is fair).
- **Pure candle implementation** — no Python interop, no
  pre-trained weights, no ONNX. Uses `candle_nn::{linear, layer_norm}`
  + a custom `MultiHeadSelfAttention` block (architect spec at M-T1)
  or `candle_transformers::models::*` primitives if architect
  determines they fit (small attention block over a 42-token sequence
  is straightforward).
- **Determinism**: candle CPU + Metal forward passes byte-identical
  for the same seed + input — architect verifies via the existing
  `crates/forecast/tests/forward_determinism.rs` pattern (TCN
  precedent T-D-N4 of v25-tcn-overlay).

### R2 — Training scaffold (closes Q2)

A new binary `crates/forecast/src/bin/train_patchtst.rs` mirrors the
shape of [`train_tcn.rs`](../../crates/forecast/src/bin/train_tcn.rs)
but lands the **post-training σ_train derivation pattern from the
start** per ADR-0035 § D1 (no in-loop accumulator; no σ_train scalar
inside the per-epoch loop; σ_train is computed post-convergence via a
frozen-weights forward pass over the training data span).

- **CLI flags** (architect locks at M-T1 with operator-decide via Q4
  / Q6):
  - `--scenario {bs1|bs2}` — anchored-checkpoint family target.
  - `--target-horizon-bars <N>` — Q4 default 24.
  - `--span-start <YYYY-MM-DD>` / `--span-end <YYYY-MM-DD>` —
    training-data span. Default mirrors the chosen scenario per
    Q6(a): `bs1` → 2023-01-01..2023-12-31.
  - `--patch-len`, `--stride`, `--d-model`, `--n-heads`, `--d-ff`,
    `--n-layers`, `--dropout` — model hyperparameters with R1 defaults.
  - `--epochs <N>` (default 30, mirrors TCN); `--batch-size <N>`
    (default 128); `--seed <hex>` (default `0x00C0FFEE` per ADR-0002).
- **Optimiser**: AdamW (β₁=0.9, β₂=0.999, weight_decay=1e-4) — same
  as TCN.
- **LR schedule**: OneCycle (max LR 1e-3, pct_start 0.3) — same as
  TCN.
- **Loss**: Huber (δ=0.001 on log-returns) — same as TCN, anchor
  apples-to-apples comparability at the loss level.
- **σ_train computation**: post-training, frozen-weights forward
  pass over the training-data span per ADR-0035 § D1. Output is
  written to `<file_prefix>-<sha>.metadata.json` (NOT
  `.metadata.recalibrated.json` — PatchTST's σ_train is
  canonical-at-ship time, not retrofitted). The recalibrate path
  (overlay-file) is **not used** for the initial PatchTST ship
  because there is no pre-existing buggy σ_train to overlay; the
  contract reuses for future PatchTST corrections.
- **Audit emission**: `train_events` rows per epoch per
  [ADR-0034](../architecture/adr/0034-cockpit-training-control.md)
  so the cockpit training-control panel surfaces per-epoch progress
  (already shipped; reused verbatim by changing only the
  `model_family` audit column from `"tcn"` to `"patchtst"`).
- **Model revision SHA**: derived per ADR-0029 § 4 over
  (weights, canonical-arch-descriptor, training-data-revision-SHA,
  seed). The descriptor includes all PatchTST hyperparameters
  (patch_len, stride, d_model, n_heads, d_ff, n_layers, dropout,
  context_len, target_horizon_bars). Two PatchTST checkpoints with
  different hyperparameters have different SHAs by construction
  (K2 invariant inherited from v2.5 TCN).
- **Determinism**: 2-run byte-identity of `<file_prefix>-<sha>.safetensors`
  given identical CLI args + seed (architect verifies at M-T1; tester
  re-checks at M-FINAL on the actual training run).

### R3 — PatchTST forecaster (closes Q1 + carries the trait)

A new file `crates/forecast/src/patchtst.rs` (sibling to
`tcn.rs`) implements `PatchTstForecaster` with the same shape as
`TcnForecaster`:

- `PatchTstForecaster::load_anchor(scenario)` — loads the anchored
  checkpoint by scenario enum (`Bs1` for v0.1.0; `Bs2` when v0.1.1
  ships).
- `PatchTstForecaster::load_from_paths(safetensors, metadata)` —
  load by explicit paths (consumer of ADR-0035 § D3 metadata
  overlay convention when future recalibration runs land).
- `impl ForecastProvider for PatchTstForecaster` — the same async
  trait surface as TCN. The body calls `PatchTstModel::forward(...)`,
  extracts the scalar `r_hat`, divides by `sigma_train` for
  confidence, applies the ε deadband from R6 of v25-tcn-overlay.
- **Cache + replay wiring**: PatchTstForecaster reuses
  `crates/replay-cache/` verbatim (the namespace stays `"forecast"`;
  cache rows distinguish model families via the `model_revision`
  column).
- **Optional builder pairs**:
  - `with_cache(cache)` — strict-replay opt-in (mirror of
    `TcnForecaster::with_cache`).
  - `with_strict_replay(...)` — same.
- **Sigma-train-not-in-safetensors test**: a new test file
  `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs`
  asserts no PatchTST safetensors header contains a `sigma_train` /
  `output_scale` tensor (per ADR-0035 § D4 invariant inherited).

### R4 — At least 1 anchored checkpoint (closes Q7)

Q2 = (a) MVP scope: ship code + **one BS-1 PatchTST checkpoint** +
F-verdict report. Filename convention mirrors the TCN family per
ADR-0029:

```
crates/forecast/checkpoints/anchors/
  patchtst-bs1-<sha>.safetensors
  patchtst-bs1-<sha>.metadata.json
```

where `<sha>` is the `model_revision` SHA over (weights,
canonical-arch-descriptor, data-rev, seed). No
`.metadata.recalibrated.json` overlay at ship — PatchTST σ_train is
canonical-at-ship per R2.

Anchor under version `v2.5a.0-patchtst` (Q7 default):

- `forecast-distribution-patchtst-bs1-realdata` — body-SHA of
  the PatchTST BS-1 forecast-distribution report (F-verdict bin
  output per ADR-0033 § D3).
- (Optional, Q8 = (a)) `top10-2023-fy-patchtst-overlay-realdata` —
  body-SHA of the PatchTST BS-1 backtest report. Operator-decide
  whether to ship a strategy-integration anchor in v0.1.0 or defer
  to v0.1.1; analyst-recommended default is **ship it** (Q8 = (a)
  sibling strategy).

BS-2 PatchTST checkpoint + its anchors defer to v0.1.1 (Q2 = (a)
analyst recommendation). Rationale: H1 falsification on BS-1 alone
is sufficient to route to retirement; H1 confirmation on BS-1 makes
BS-2 a follow-on commitment, not a v0.1.0 prerequisite.

### R5 — Alpha-investigation cycle (closes Q2)

Apply the same ADR-0033 F-verdict algorithm and the same
threshold-tuning approach to the new BS-1 PatchTST checkpoint:

1. **Forecast-distribution report** — invoke
   [`crates/forecast/src/bin/forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs)
   against the BS-1 PatchTST checkpoint. The existing bin's
   `--scenario` enum extends (architect-decide at M-T1: enum variant
   `PatchtstBs1` or a string-based dispatch). Emit
   `forecast-distribution-patchtst-bs1-realdata-<date>.md` under
   `spec/v25a-patchtst-overlay/reports/`. F-verdict per the immutable
   ADR-0033 § D3 priority tree.
2. **Sharpe-comparison report** — invoke
   [`crates/forecast/src/bin/sharpe_comparison.rs`](../../crates/forecast/src/bin/sharpe_comparison.rs)
   against the new BS-1 PatchTST backtest scenario (R8 strategy
   integration). Emit `sharpe-comparison-patchtst-bs1-realdata-<date>.md`
   under the same reports directory. Verdict per the same body-shape
   contract (D2 of ADR-0033).
3. **Joint advisory verdict** — record per ADR-0033 § D3.c. v0.1.0
   ships BS-1 only, so the joint verdict is a **single-checkpoint
   advisory** — body section flagged `Verdict (BS-1 only; BS-2
   deferred to v0.1.1)`. The follow-on routing per F-verdict:
   - **F1** → retrain PatchTST (training pathology — feature
     `v25a-patchtst-retrain`).
   - **F2** → σ_train recalibrate via the existing
     `recalibrate_sigma_train` bin (already covers PatchTST per
     ADR-0035 D1 contract — the bin is model-agnostic by D1's
     frozen-weights forward-pass shape).
   - **F3** → spawn a `v25a-patchtst-threshold-tuning` feature
     mirroring `v25-tcn-threshold-tuning`.
   - **F4** → analyst spawn for the v2.6 bake-off decision (this
     phase's data is then load-bearing input to phase 4 retirement
     gate per
     [`v26-forecast-bakeoff`](../v26-forecast-bakeoff/feature.md)).

### R6 — Strategy integration (closes Q8)

Q8 default = (a) **sibling strategy** —
`crates/strategy/src/patchtst_overlay_momentum.rs` mirrors the shape of
[`crates/strategy/src/tcn_overlay_momentum.rs`](../../crates/strategy/src/tcn_overlay_momentum.rs).
Surface:

- `MomentumStrategy::with_patchtst_bs1(base) -> Result<Self, …>` —
  load the anchored checkpoint + wire as overlay (mirror of
  `with_tcn_bs1`).
- `MomentumStrategy::with_patchtst_bs1_ledger(base, ledger) -> …`
  — same plus audit ledger (mirror of `with_tcn_bs1_ledger`; behind
  the `feature = "forecast-audit-tick"` flag for parity).
- `MomentumStrategy::with_patchtst_bs1_tuned(base, tau, epsilon)` /
  `with_patchtst_bs1_ledger_tuned(...)` — operator-decide threshold
  override pair (mirror of the v25-tcn-threshold-tuning ship's
  shipped builders).

The existing TCN builders (`with_tcn_bs{1,2}{,_ledger}{,_tuned}{,_ledger_tuned}`)
stay byte-identical. PatchTST builders are **additive** — zero touch
on the existing 28-anchor surface (R9 / R10 enforce).

Rejected alternatives (Q8(b)/(c)) — extending the existing
TCN-overlay-momentum to be model-agnostic via a `Box<dyn ForecastProvider>`
abstraction OR shipping no strategy integration in v0.1.0. The
model-agnostic refactor risks moving the 28 anchored byte-SHAs
(any code change in `tcn_overlay_momentum.rs` `combine()` path is a
SHA risk); the no-strategy-integration path forecloses on the
operator's "what's the live Sharpe of PatchTST" question that
F-verdict alone cannot answer. The sibling-strategy path is
additive, low-anchor-risk, and lets v0.1.1 refactor toward
model-agnostic if the bake-off picks PatchTST as canonical.

### R7 — Backtest scenario integration

A new backtest scenario `top10-2023-fy-patchtst-overlay-realdata`
lands in `crates/backtest/src/scenarios/` (architect-decide at M-T1:
new file `patchtst_overlay_weights.rs` mirror of
`tcn_overlay_weights.rs`, or extend the existing module). The
scenario:

- Loads the BS-1 PatchTST anchored checkpoint via
  `PatchTstForecaster::load_anchor(AnchorScenario::Bs1)`.
- Wires the forecaster as overlay on the v1 momentum baseline via
  `MomentumStrategy::with_patchtst_bs1(...)`.
- Runs the same backtest harness as the TCN-overlay scenarios per
  [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md).
- Emits a `top10-2023-fy-patchtst-overlay-realdata-<date>.md` report
  under `spec/v25a-patchtst-overlay/reports/`.

The body bytes of this report **must be deterministic** across
2-run byte-identity for anchor-lock at M-FINAL.

### R8 — Watch recipe for long-running training (per MEMORY.md)

The developer at M-D MUST emit a copy-pasteable
`watch -n 60 '<probe>'` block when kicking off the PatchTST training
run per the operator's `MEMORY.md` directive. Suggested probe:

```bash
# PatchTST BS-1 training progress (replace <PID> with cargo PID via `pgrep -f train_patchtst`)
watch -n 60 'tail -30 /tmp/train_patchtst-bs1.log && \
             echo "---" && \
             ps -p <PID> -o pcpu,pmem,etime,command | tail -2 && \
             echo "---" && \
             ls -lh crates/forecast/checkpoints/anchors/patchtst-bs1-*.safetensors 2>/dev/null || echo "(checkpoint not yet written)"'
```

The R3 cost-tripwire from `v25-tcn-horizon-bump-or-retire` applies
verbatim: if a single epoch exceeds 24 wall-clock hours on Apple
Silicon Metal, the developer escalates to the operator before
continuing. K4 (compute-cost over-run) carries forward.

### R9 — Non-regression contract (load-bearing)

The PatchTST ship is **anchor-additive**. The 28 originals (26 PASS
+ 2 pre-existing glob-collision FAIL per
[`v25-tcn-horizon-bump-or-retire § Anchor gate baseline`](../v25-tcn-horizon-bump-or-retire/tasks.md))
stay byte-identical:

- 22 pre-recalibrate anchors (19 pre-investigation + 3
  alpha-investigation).
- 4 v2.6.1-alpha-investigation-recalibrated anchors.
- 2 v2.6.2-threshold-tuning anchors.

Specifically:

- The existing `tcn-bs{1,2}-<sha>.safetensors` files stay
  byte-identical.
- The existing `tcn-bs{1,2}-<sha>.metadata.json` files stay
  byte-identical.
- The existing `tcn-bs{1,2}-<sha>.metadata.recalibrated.json`
  files stay byte-identical.
- The 8 existing strategy builders
  (`with_tcn_bs{1,2}{,_ledger}{,_tuned}{,_ledger_tuned}`) stay
  byte-identical. New PatchTST builders are ADDITIVE.
- The existing `forecast_distribution` bin + `sharpe_comparison`
  bin stay backward-compatible (additive PatchTST dispatch is
  the only change).
- The existing `recalibrate_sigma_train` bin stays
  byte-identical — PatchTST σ_train is canonical-at-ship per R2,
  so this bin is unused at v0.1.0. Future use covered by the
  ADR-0035 D1 contract (model-agnostic by construction).
- No iced bump.
- No new external crate dependency (candle + candle-nn already in
  the workspace; PatchTST uses existing primitives).

`bash scripts/verify_anchors.sh` PRE-lock reports `26 PASS + 2
pre-existing glob-collision FAIL` (the baseline from
v25-tcn-horizon-bump-or-retire). POST-lock the count grows by the
2 new PatchTST anchors (under Q7 = (a) + Q8 = (a) ships). The 28
originals stay byte-identical.

### R10 — Verification gates

Tester confirms at M-FINAL:

1. `cargo fmt --check` + `cargo clippy --workspace -- -D warnings`
   PASS.
2. `cargo clippy -p forecast --features candle -- -D warnings` PASS.
3. `cargo test --workspace --lib` PASS, 0 failures.
4. `cargo test -p forecast --features candle --test sigma_train_not_in_safetensors_patchtst`
   PASS — D4 invariant inherited from ADR-0035 § D4.
5. `cargo test -p forecast --features candle --test forecast_distribution_verdict`
   PASS — F-verdict algorithm immutable from ADR-0033 § D3.
6. 2-run byte-identity determinism gate on the new
   `forecast-distribution-patchtst-bs1-realdata-*.md` report.
7. 2-run byte-identity determinism gate on the new
   `top10-2023-fy-patchtst-overlay-realdata-*.md` report (if Q8 = (a)).
8. `bash scripts/verify_anchors.sh` reports 26 PASS + 2 known-FAIL
   PRE; 28 PASS + 2 known-FAIL POST (Q7=(a) + Q8=(a) = +2 anchors).
   The 28 originals stay byte-identical.
9. `uv run scripts/spec_lint.py` matches the baseline (0 new
   categories).
10. Joint advisory verdict recorded in `feature.md § Verification`.

## Hypothesis register (H1-H4)

> Each hypothesis is testable; the tester gate closes / falsifies
> it. Listed in dependency order.

### H1 — PatchTST extracts more directional signal than TCN at same horizon

**Statement.** A BS-1 PatchTST checkpoint trained on the same data
span as the v2.5 TCN BS-1 checkpoint, evaluated at the chosen Q4
horizon (default 24h) via the immutable ADR-0033 § D3 F-verdict
algorithm, produces an F-verdict of **F1** OR **F3** (NOT F4) AND
a follow-on threshold-tuning sweep on the new checkpoint unlocks a
joint (τ, ε) cell with Sharpe-delta ≥ +0.10 vs v1 momentum baseline.

Equivalently: PatchTST clears the +0.10 T-ALPHA-UNLOCKED threshold
that the v2.5 TCN at 1h missed (+0.018 / +0.045 T-MARGINAL).

**Test.** R2 trains the PatchTST BS-1 checkpoint; R5's
forecast-distribution report records the F-verdict; if F-verdict is
non-F4 AND Q4 = 24h, a follow-on `v25a-patchtst-threshold-tuning`
feature tests Sharpe-delta. H1 is **confirmed** iff F-verdict is not
F4 AND a τ × ε cell unlocks ≥ +0.10 Sharpe-delta.

**Confidence at brief time**: **LOW-MEDIUM**. PatchTST's published
benchmarks on M4 / ETT / electricity are NOT crypto OHLCV. The v2.5
TCN at 1h F4-verdict is partial evidence that the cross-sectional
momentum overlay shape is hard for any forecaster on this data, not
just TCN-shape. **The conservative read is: PatchTST may also F4 at
24h** — the bake-off result will be informational either way, but
the operator's expectation for v0.1.0 should be set realistically
(not "PatchTST will win", but "PatchTST will tell us whether the
inductive bias matters on this data").

### H2 — Transformer attention captures session-level structure TCN's local conv missed

**Statement.** PatchTST's patch-attention extracts signal from
intraday session boundaries (Asia open ~00:00 UTC, Europe open
~07:00 UTC, US open ~13:00 UTC, weekend dampening) via attention
weights that concentrate on contiguous patch tokens crossing those
boundaries. Measurable as: the model's per-bar `r_hat` distribution
shows non-trivial heteroscedasticity across hour-of-week, vs the v2.5
TCN's `r_hat` distribution which was approximately stationary across
hour-of-week.

**Test.** Architect at M-T1 specifies a side-artifact in the R5
forecast-distribution report body: a per-hour-of-week breakdown of
`abs_p95(r_hat)` for both PatchTST and v2.5 TCN (BS-1 only).
Heteroscedasticity test: max/min across hours > 1.5× (operator-
decide tighten or loosen). Tester records the test outcome.

**Confidence at brief time**: **LOW**. This is an exploratory
hypothesis — the operator gets a side-finding regardless of
confirmation. H2 is decoupled from H1's pass/fail; H2 informs the
v0.1.1 design of which architectures to bake-off.

### H3 — ~4-6 weeks scope is feasible

**Statement.** The MVP (Q2 = (a) — code + 1 BS-1 PatchTST checkpoint
+ F-verdict report + Sharpe-comparison report + strategy
integration + 2 anchors) ships within 4-6 weeks of M-OD operator
approval on Apple Silicon Metal hardware. Decomposition:

| Wave | Cost | Owner |
|------|------|-------|
| M-OD operator-decide Q1-Q8 | minutes (autoapprove) | operator |
| M-T1 architect lock + ADR-0036 PatchTST training contract | 4-8 hr | architect |
| Wave A — `patchtst.rs` model + `train_patchtst.rs` scaffold + tests (no training run) | 3-7 days | developer |
| Wave B — PatchTST BS-1 training run on Apple Silicon Metal | **5-7 days wall-clock** | orchestrator-monitor |
| Wave C — σ_train derivation (post-training pattern; happens inside Wave B's final pass per R2) | 0 (included in B) | developer |
| Wave D — forecast_distribution + sharpe_comparison + backtest scenario + strategy integration | 1-2 days | developer |
| Wave E — tester gate (M-FINAL) | 0.5-1 day | tester |
| Wave F — presenter deck (M-PRESENTER) | 0.5 day | presenter |
| **Total wall-clock** | **~3-5 weeks if everything goes smoothly; ~5-7 weeks with one Wave-B retry** | |

**Test.** Calendar tracking from M-OD approval to presenter ship
date. H3 is **confirmed** iff total ≤ 6 weeks; **soft-failed** iff
6-9 weeks (cost-tripwire K1 triggers a mid-stream replan);
**hard-failed** iff > 9 weeks (analyst spawns a triage feature).

**Confidence at brief time**: **MEDIUM**. The TCN BS-1 took ~4-5
days on Apple Silicon Metal; PatchTST at the R1 default config
(~1.5-2M params) is **smaller** than TCN (~4.4M params), so training
wall-clock should be comparable or faster. The dominant risk is the
**candle-attention implementation** — PatchTST's multi-head
self-attention has more moving parts than TCN's `Conv1d` stack, and
candle's transformer primitives are less battle-tested than its
convolution path. K2 (candle-attention bugs) is the load-bearing
risk for the schedule.

### H4 — 24h horizon is the right test bed (Q4 default = (b))

**Statement.** Training PatchTST at the 24h target horizon
(`target_logret = (close_{t+24} / close_t).ln()`) produces a
checkpoint whose F-verdict is more informative than a 1h-horizon
PatchTST checkpoint, because (i) 1h is already exhausted by the v2.5
TCN journey, (ii) 24h log-return std exceeds transaction-cost noise
by 3-4×, (iii) PatchTST's published ETT benchmarks use multi-day
forecast horizons.

**Test.** R5 forecast-distribution F-verdict on the 24h-horizon
PatchTST BS-1 checkpoint. H4 is **confirmed** iff F-verdict is not
F4 (regardless of Sharpe-delta — F-verdict not-F4 means signal
exists). H4 is **falsified** iff F-verdict is F4 (no signal even at
24h with the PatchTST inductive bias).

**Confidence at brief time**: **MEDIUM**. The horizon-hypothesis
from
[`v25-tcn-horizon-bump-or-retire § H1`](../v25-tcn-horizon-bump-or-retire/feature.md#h1--24h-horizon-unlocks-signal-on-hourly-bars-scope-a--c)
is **untested** — the operator retired v2.5 TCN before running it.
PatchTST at 24h is the first test of the horizon hypothesis on this
data. **The conservative read is: H4 may falsify**, in which case
the joint H1+H4 falsification (PatchTST F4 at 24h) is **strong
evidence** for the v2.6 retirement gate on the entire DL overlay
direction at hourly cadence.

## Risk register (K1-K6)

| Risk | Mitigation |
|------|------------|
| **K1 — PatchTST training blows the compute budget** (e.g. >7 days per BS-1 checkpoint on Apple Silicon Metal due to candle-attention overhead or hyperparameter mistuning). | R8 cost tripwire (single epoch > 24h wall-clock = escalate). Architect at M-T1 formalises a developer-decide gate: if epoch N takes > 3× the median of epochs 1..N-1, developer pauses + emits diagnostic dump + escalates. Architect locks the R1 hyperparameter defaults to a **small config** (~1.5-2M params; well below the 5-10M ceiling) to keep wall-clock conservative. |
| **K2 — candle attention primitives are bugged or slow** (e.g. multi-head self-attention OOMs on Metal, or produces non-deterministic outputs across CPU/Metal). | Architect at M-T1 specifies the attention block primitive (candle-nn vs custom). Wave A unit-test gate: `forward_determinism_patchtst.rs` runs a fixed-seed forward pass on CPU + Metal and asserts byte-identical outputs. If the test fails at architect-design time, fall back to a manually-implemented attention block (~80 LoC of explicit `Tensor` ops). |
| **K3 — PatchTST F4s at 24h (H1 + H4 jointly falsified)**. | This is an **experimental outcome**, not a feature failure. Per R5 routing, F4-on-PatchTST-BS-1 routes immediately to the v2.6 bake-off retirement gate — the operator has now ruled out TCN-shape AND PatchTST-shape on this data. The follow-on `v25b-transformer-overlay` (phase 3) decides whether to proceed or also retire. |
| **K4 — Existing 28 anchors flip on the new PatchTST build** (some code path leaks into `tcn_overlay_momentum.rs` or `tcn.rs` or the forecast_distribution bin's TCN dispatch; the v2.6.0-realdata or v2.6.2-threshold-tuning anchors land different SHAs after merge). | R9 non-regression contract: CI gate at M-FINAL runs `verify_anchors.sh` PRE and POST and asserts the 28 originals are byte-identical. New `patchtst_overlay_weights.rs` scenario + `with_patchtst_bs1*` builders are **additive only**; architect designs the `forecast_distribution.rs` PatchTST dispatch as an additive enum variant (not a refactor of the existing TCN dispatch). Architect-confirm at M-T1; developer ships a unit test asserting the existing TCN scenario byte-output is unchanged. |
| **K5 — Patch / stride / d_model defaults are wrong for crypto OHLCV** (R1 defaults are PatchTST's ETT defaults, not crypto-tuned). | This is a known-unknown. Architect at M-T1 weighs operator-decide override on Q3 (patch_len/stride) and Q5 (feature set). If the analyst-recommended defaults produce H1=falsified, the conservative read is "PatchTST-paradigm doesn't fit crypto at hourly cadence at these defaults" — NOT "we need to sweep 50 hyperparameters." A future v0.1.1 can run a small hyperparameter sweep if v0.1.0 finishes T-MARGINAL (analogous to v25-tcn-threshold-tuning's τ × ε sweep). |
| **K6 — Scope creep into the v2.5 TCN crate** (developer tempted to refactor `forecast::lib` to share more code between TCN and PatchTST). | Hard analyst boundary: v0.1.0 ships `patchtst.rs` as a sibling to `tcn.rs` — zero refactor of `tcn.rs`. Architect formalises at M-T1 as a unit test: `git diff HEAD -- crates/forecast/src/tcn.rs` is empty after the PatchTST ship (or limited to comment-only annotations like the ADR-0035 deprecation comment). Refactor opportunities defer to v0.1.1+ or to the v2.6 bake-off feature. |

## Non-regression contract

This section consolidates the load-bearing invariants the tester
confirms at M-FINAL:

1. **28 anchored body-SHAs byte-identical.** `bash scripts/verify_anchors.sh`
   reports `26 PASS + 2 pre-existing glob-collision FAIL` PRE-lock
   (inherited from v25-tcn-horizon-bump-or-retire) and `28 PASS + 2
   known-FAIL` POST-lock (Q7 = (a) + Q8 = (a); 2 new PatchTST
   anchors). All 28 originals stay byte-identical.
2. **Original TCN `.safetensors` files byte-identical.** `git diff
   HEAD -- crates/forecast/checkpoints/anchors/tcn-*.safetensors`
   is empty.
3. **Original TCN `.metadata.json` files byte-identical.** Same diff.
4. **Original TCN `.metadata.recalibrated.json` overlay files
   byte-identical.** Same diff.
5. **`tcn.rs` body byte-identical.** `git diff HEAD --
   crates/forecast/src/tcn.rs` is empty (modulo comment-only
   annotations).
6. **Existing TCN strategy builders byte-identical.**
   `with_tcn_bs{1,2}{,_ledger}{,_tuned}{,_ledger_tuned}` stay
   byte-identical (any new PatchTST builders are additive).
7. **Existing forecast_distribution bin TCN dispatch byte-identical.**
   Default invocation `cargo run -p forecast --bin forecast_distribution
   -- --scenario bs1` produces a report whose body bytes match the
   anchored
   `forecast-distribution-bs1-realdata` body. Architect formalises
   at M-T1 (additive enum variant or string dispatch design).
8. **No new external crate dependencies.** Workspace `Cargo.toml`
   diff is limited to existing crates (candle, candle-nn,
   candle-transformers — all already in the workspace per ADR-0028).
9. **No iced bump.** Operator-locked per CLAUDE.md.
10. **F-verdict algorithm immutable.** ADR-0033 § D3 stays
    unchanged. PatchTST forecast-distribution reports use the same
    priority tree (one of F1 / F2 / F3 / F4).
11. **ADR-0035 σ_train contract honored.** PatchTST's σ_train is
    derived via the post-training frozen-weights forward-pass
    pattern (D1), NOT the deprecated in-loop accumulator. The new
    `train_patchtst.rs` does NOT contain a `Vec<f32>` accumulator
    declared outside the per-epoch loop (architect codifies as a
    code-review check at M-T1).
12. **ADR-0029 canonical-arch-descriptor extended additively.**
    The `model_revision` SHA computation for PatchTST adds the
    PatchTST-specific fields (patch_len, stride, d_model, n_heads,
    d_ff, n_layers, dropout, context_len, target_horizon_bars) to
    the existing canonicaliser. The v2.5 TCN checkpoints'
    `model_revision` SHAs are unchanged (their canonical descriptor
    doesn't reference PatchTST fields).

## Acceptance per milestone

The feature is **done** when all milestones land their gates.

### M-OD — Operator-decide (Q1-Q8 resolved)

> **Soft blocker.** All 8 questions carry analyst-recommended defaults.
> "Autoapprove" activates all defaults. The operator may override
> individual questions; the analyst recommends the bundled defaults.

1. Q1-Q8 answered by operator (or "autoapprove").
2. Frontmatter flips `status: draft → proposed`, `owner: analyst →
   architect`.

### M-T1 — Architect lock

1. § Design block appended to `feature.md` (between § Out of scope
   and § Changelog).
2. `spec/v25a-patchtst-overlay/decomp.md` complete with T-D / T-T
   row decomposition into 4 waves (A-D per H3).
3. **ADR-0036** written:
   `spec/architecture/adr/0036-patchtst-training-contract.md`.
   Codifies (D1) PatchTST architecture skeleton (patch embed +
   pre-LN transformer encoder + projection head); (D2)
   canonical-arch descriptor extension; (D3) σ_train post-training
   derivation (referenceing ADR-0035 § D1); (D4) cost tripwire (R8);
   (D5) K2 candle-attention determinism gate.
4. K4 anchor-neutrality unit test designed.
5. K6 tcn.rs-byte-identity unit test designed.
6. Frontmatter flips `status: proposed → in-progress`, `owner:
   architect → developer`.

### M-D — Developer Waves A-D

1. Wave A: `crates/forecast/src/patchtst.rs` + `train_patchtst.rs`
   + 4 unit tests (sigma_train_not_in_safetensors_patchtst,
   forward_determinism_patchtst, tcn_byte_identity, anchor_neutrality).
2. Wave B: PatchTST BS-1 training run (LONG-RUNNING ~5-7 days;
   developer emits watch recipe per R8; cockpit
   training-control panel surfaces progress per ADR-0034).
3. Wave C: σ_train derivation (folded into Wave B's training run
   per R2 — post-training frozen forward pass over the training
   span emits σ_train scalar to metadata at training-complete).
4. Wave D: forecast_distribution + sharpe_comparison + backtest
   scenario + strategy integration. Each emits a body-deterministic
   report under `spec/v25a-patchtst-overlay/reports/`.

### M-FORECAST-DIST — Forecast-distribution F-verdict

1. `forecast_distribution --scenario patchtst-bs1` runs against the
   new checkpoint.
2. `forecast-distribution-patchtst-bs1-realdata-<date>.md` emitted.
3. F-verdict (F1/F2/F3/F4) per the immutable ADR-0033 § D3
   algorithm recorded in the report body.

### M-SHARPE — Real-Binance backtest

1. `backtest --scenario top10-2023-fy-patchtst-overlay-realdata`
   runs against the new checkpoint via the new strategy builder.
2. `top10-2023-fy-patchtst-overlay-realdata-<date>.md` emitted
   under `spec/v25a-patchtst-overlay/reports/`.
3. Sharpe-delta vs v1 momentum baseline computed; T-classifier
   advisory verdict (T-ALPHA-UNLOCKED / T-MARGINAL / T-NO-ALPHA)
   recorded.
4. `sharpe-comparison-patchtst-bs1-realdata-<date>.md` emitted.

### M-FINAL — Tester gate

R10's 10 gates land green. Joint advisory verdict recorded in
`feature.md § Verification`.

### M-PRESENTER — Operator approval

1. Presenter deck under
   `spec/v25a-patchtst-overlay/presentations/v25a-patchtst-overlay-<YYYY-MM-DD>.md`
   carrying joint advisory verdict + recommended next routing
   (one of: H1 confirmed → spawn v25a-patchtst-threshold-tuning;
   H1 falsified → analyst spawn for v2.6 bake-off retirement
   decision; F1 → v25a-patchtst-retrain; F2 → recalibrate via
   existing bin).
2. Operator ticks approval. Frontmatter flips `status: in-progress
   → shipped`.
3. Trace row `REQ-V25A-PATCHTST-001` flips state.
4. Backlog entry moved Active → Recent.

## Open questions (Q1-Q8 — operator-decide)

> **All 8 questions carry analyst-recommended defaults. "Autoapprove"
> activates all defaults.** The analyst-recommended bundle is
> internally consistent (default Q1 + Q2 + Q4 + Q5 + Q6 + Q7 + Q8
> reinforce each other). Operator overrides on any individual
> question may cascade requirements (e.g. Q1 = (b) iTransformer
> changes R1 substantially).

### Q1 — Architecture choice

Which patch-based-transformer architecture to ship as the v0.1.0
forecaster?

- **(a)** **PatchTST** (Nie, Nguyen, Sinthong, Kalagnanam 2022 — *A
  Time Series is Worth 64 Words*). Patch embedding + transformer
  encoder + projection head. Channel-independence claim from § 3.2.
  **Analyst-recommended default.** Most mature reference
  implementation
  ([yuqinie98/PatchTST](https://github.com/yuqinie98/PatchTST), MIT
  license); most reproducible benchmarks; cleanest mapping to the
  existing `ForecastProvider` trait surface.
- **(b)** **iTransformer** (Liu, Hu, Yang, Liu, Cao, Long 2023 — *iTransformer:
  Inverted Transformers Are Effective for Time Series Forecasting*).
  Inverts: each input variable (here: each of the 5 features) is a
  token; attention across variables. Stronger on cross-channel
  relationships, weaker on temporal patterns. Mature reference
  implementation
  ([thuml/iTransformer](https://github.com/thuml/iTransformer), MIT).
  **Analyst rejects for v0.1.0** — the 5-feature input is too narrow
  to amortise variate-as-token's overhead; PatchTST's
  patch-as-token captures temporal patterns more naturally for an
  overlay that consumes a per-bar `r_hat`.
- **(c)** **Hybrid PatchTST + iTransformer ensemble**. Train both;
  combine via averaging at the `r_hat` level. **Analyst rejects for
  v0.1.0** — doubles training cost; the bake-off in v2.6 is the
  designed canonical ensemble decision point, not v0.1.0.

**Analyst default: (a) PatchTST.** Strongest reference impls;
cleanest fit for the per-bar `r_hat` overlay shape; most
reproducible.

### Q2 — v0.1.0 scope MVP shape

What ships in v0.1.0?

- **(a)** **Code + 1 trained BS-1 checkpoint + F-verdict report +
  Sharpe-comparison report + strategy integration + 2 anchors**.
  **Analyst-recommended default.** Operator wants alpha-answer,
  not just infrastructure. Q2=(a) is the analyst's read of the
  retirement-decision intent: the multi-week budget pivots to
  PatchTST to **answer the alpha question**, not to set up the
  scaffolding and stop.
- **(b)** **Code only** — no training run, no checkpoint, no
  F-verdict report. v0.1.1 ships the checkpoint + alpha-investigation
  as a follow-on feature. **Analyst rejects** — gives the operator
  infrastructure without the answer; the multi-week budget would be
  ~1-2 weeks of code + 4-5 weeks of waiting for v0.1.1.
- **(c)** **Code + 1 trained BS-1 checkpoint + F-verdict report
  only** — no Sharpe-comparison, no strategy integration, no
  anchors. v0.1.1 picks up the strategy + Sharpe side. **Analyst
  considers borderline** — F-verdict alone is informative, but
  without Sharpe-comparison the operator can't compare PatchTST's
  Sharpe-delta to the v2.5 TCN's +0.018 / +0.045 T-MARGINAL
  result. The marginal cost of Wave D (1-2 days) buys
  apples-to-apples comparison; not worth deferring.

**Analyst default: (a)** — full MVP. Wave D's marginal cost is
small; the operator's decision-relevant question (does PatchTST
beat TCN's +0.045 Sharpe-delta?) requires the strategy +
Sharpe-comparison side.

### Q3 — Patch length + stride

PatchTST hyperparameters?

- **(a)** **`patch_len=16, stride=8`** — Nie et al's ETT benchmark
  default (24h horizon @ hourly; patch_len=16 ≈ 16h history per
  patch, stride=8 = 50% overlap). **Analyst-recommended default.**
  Direct mapping from the paper's most-cited config.
- **(b)** `patch_len=24, stride=24` — patches align with the 24h
  horizon and don't overlap. Theoretically cleaner (each token is
  one "day"); but loses the 50% overlap information that
  reinforces local patterns. Untested in any reference implementation
  on hourly bars.
- **(c)** `patch_len=8, stride=4` — finer-grained patches; more
  tokens per lookback window (higher attention cost). Better
  intraday-session resolution but smaller per-patch context.
- **(d)** `patch_len=48, stride=24` — coarse-grained; 2-day
  patches with 1-day overlap. Closer to the daily-cadence v1
  momentum baseline's lookback.

**Analyst default: (a) `patch_len=16, stride=8`.** Direct mapping
from the paper. If H1 falsifies on this default, v0.1.1 sweeps
Q3 = (b) / (c) / (d) — patch-length sweep is the natural extension
of T-MARGINAL diagnosis.

### Q4 — Horizon target

Which forward-prediction horizon?

- **(a)** **1h target** (`target_logret = (close_{t+1} /
  close_t).ln()`) — match the v2.5 TCN's BS-1 horizon. Rules out
  the "PatchTST-specific 1h failure" hypothesis cleanly; gives a
  direct architecture-paradigm comparison at the same horizon. But
  the operator's retirement decision already concluded 1h is too
  noisy.
- **(b)** **24h target** (`target_logret = (close_{t+24} /
  close_t).ln()`) — different horizon than the retired v2.5 TCN.
  Tests both the "PatchTST paradigm helps" and "24h horizon helps"
  hypotheses jointly. **Analyst-recommended default.** Operator
  already retired 1h; chase signal at 24h.
- **(c)** **Test both 1h and 24h** to disambiguate model-family
  vs horizon. **Analyst rejects** — doubles training cost; if H1
  falsifies on (b), running (a) won't change the retirement
  decision. The joint H1+H4 question is a v0.1.1 follow-on if
  needed.

**Analyst default: (b) 24h.** Per the
[`v25-tcn-horizon-bump-or-retire § Quantitative-finance context`](../v25-tcn-horizon-bump-or-retire/feature.md#quantitative-finance-context),
24h log-return std exceeds transaction-cost noise by 3-4×, which
is the cleanest SNR test for a forecaster.

**Sub-Q4a — overlapping vs non-overlapping 24h targets**: under
Q4 = (b), emit a target every 1h (overlapping; ~87k samples) or
every 24h (non-overlapping; ~3,650 samples)?

Analyst default: **overlapping** (~87k samples) — the
autocorrelation in overlapping targets (~0.95) is well-known and
doesn't catastrophically overfit at PatchTST's ~1.5-2M parameter
count.

### Q5 — Input feature set

Same 5-feature input as v2.5 TCN?

- **(a)** **Yes — `logret/logrange/logvol_z/hour_sin/hour_cos`**
  (carry-forward from
  [`crates/forecast/src/features.rs`](../../crates/forecast/src/features.rs)).
  Clean comparison with v2.5 TCN: same input, different
  architecture.
- **(b)** Extend with crypto-specific signal: realized-vol bands
  (e.g. 24h-rolling realized vol), funding-rate proxies (where
  available), open-interest proxies. **Analyst considers tempting
  but rejects for v0.1.0** — adds data-loader complexity and
  feature-engineering scope that doesn't isolate the
  architecture-paradigm test.
- **(c)** **Start with (a); defer (b) to v0.1.1.**
  **Analyst-recommended default.**

**Analyst default: (c)** — v0.1.0 uses the existing 5-feature
input verbatim. The H1 test is "PatchTST paradigm vs TCN paradigm
on the same data" — extending features confounds the comparison.

### Q6 — Training-data span

Which calendar span for the BS-1 checkpoint?

- **(a)** **`2023-01-01..2023-12-31`** (mirror v2.5 TCN BS-1
  span). **Analyst-recommended default.** Apples-to-apples
  comparison with the TCN BS-1 F4 baseline. Honors the
  per-checkpoint convention.
- **(b)** Longer span — `2023-01-01..2024-12-31` (full available
  data). More samples but conflates train-span with
  architecture-paradigm effect; not anchor-friendly without a
  matching BS-2.
- **(c)** Walk-forward retraining (architect designs a multi-
  checkpoint rolling-window schedule). **Analyst rejects** — adds
  multi-week scope; the bake-off in v2.6 is the canonical place
  for walk-forward; v0.1.0 is the paradigm test.

**Analyst default: (a) BS-1 train span 2023-01-01..2023-12-31.**

### Q7 — Anchor strategy + version pin

- **(a)** **Anchor under version `v2.5a.0-patchtst`** with naming
  `{report-family}-patchtst-bs1-realdata` (e.g.
  `forecast-distribution-patchtst-bs1-realdata`,
  `top10-2023-fy-patchtst-overlay-realdata`). Existing 28 anchors
  byte-identical. **Analyst-recommended default.**
- **(b)** Wait until v2.6 bake-off and anchor jointly across all
  three model families. **Analyst rejects** — leaves the PatchTST
  ship un-anchored; no determinism gate; future regressions
  invisible.

**Analyst default: (a) anchor early, additive.** Version
`v2.5a.0-patchtst` mirrors the v2.5b / v2.6 stub naming
convention.

### Q8 — Strategy integration

How to wire PatchTST into the existing strategy library?

- **(a)** **Sibling strategy
  `crates/strategy/src/patchtst_overlay_momentum.rs`** mirroring
  `tcn_overlay_momentum.rs`. New builders `with_patchtst_bs1*`
  follow the existing `with_tcn_bs1*` convention. **Analyst-
  recommended default.** Additive only; zero anchor risk; mirrors
  the predecessor's design.
- **(b)** Extend the existing `tcn_overlay_momentum.rs` to be
  **model-agnostic** via a `Box<dyn ForecastProvider>` abstraction.
  **Analyst rejects** — refactor risk on the 28-anchor surface; any
  code change in the `combine()` path is a SHA risk for the existing
  `top10-{2023,2024}-fy-tcn-overlay-realdata` anchors. v0.1.1 or the
  v2.6 bake-off can do this refactor when there's evidence to
  motivate it.
- **(c)** Skip strategy integration in v0.1.0; only forecast-
  distribution + sharpe-comparison reports. **Analyst rejects** —
  Sharpe-comparison needs a backtest scenario, which needs a strategy
  builder. Q8 = (c) reduces to Q2 = (c) which the analyst already
  rejected.

**Analyst default: (a) sibling strategy.**

## Cost estimate (per scope branch)

| Scope branch (operator-decide) | Wall-clock | Owner |
|--------------------------------|------------|-------|
| Author this brief (R1-R10 + H1-H4 + K1-K6 + Q1-Q8) | done 2026-05-21 | analyst (this brief) |
| **Q2 = (a) — full MVP (analyst-recommended)** | | |
| Operator-decide Q1-Q8 | minutes (autoapprove) | operator |
| Architect lock + ADR-0036 + decomp.md | 4-8 hr | architect |
| Wave A — `patchtst.rs` + `train_patchtst.rs` + 4 unit tests | 3-7 days | developer |
| Wave B — BS-1 training run (Apple Silicon Metal, ~5-7 days/checkpoint) | **5-7 days wall-clock** | orchestrator-monitor |
| Wave C — σ_train derivation (folded into Wave B) | 0 (in B) | developer |
| Wave D — forecast_distribution + sharpe_comparison + backtest scenario + strategy integration | 1-2 days | developer |
| Tester gate + Presenter deck | 1-2 days | tester + presenter |
| **Q2=(a) total** | **~3-5 weeks (best case); ~5-7 weeks with one Wave-B retry** | |
| **Q2 = (b) — code only** | | |
| Operator-decide Q1-Q8 | minutes | operator |
| Architect lock + ADR-0036 | 4-8 hr | architect |
| Wave A — code + tests, no training | 3-7 days | developer |
| Tester gate + Presenter deck | 0.5-1 day | tester + presenter |
| **Q2=(b) total** | **~1-2 weeks** (followed by ~4 weeks of v0.1.1 for the checkpoint + investigation) | |
| **Q2 = (c) — code + checkpoint, no Sharpe / strategy** | | |
| As Q2=(a) without Wave D's strategy+backtest | -1 to -2 days | |
| **Q2=(c) total** | **~3-5 weeks** (1-2 days less than Q2=(a)) | |

**Analyst recommendation: Q2 = (a).** The marginal cost of Wave D
(1-2 days) buys the operator the alpha-answer; Q2 = (c) deferral
forecloses on the operator's "is PatchTST better than the +0.045
T-MARGINAL?" question.

## Out of scope

- **No BS-2 PatchTST checkpoint in v0.1.0** — defer to v0.1.1.
  Rationale (Q2 + R4): H1 falsification on BS-1 alone is
  sufficient to route; H1 confirmation on BS-1 makes BS-2 a
  follow-on commitment.
- **No iTransformer in v0.1.0** — Q1 default = (a) PatchTST.
  iTransformer is a parked Q1 alternative; v0.1.1 may revisit if
  PatchTST H1 falsifies.
- **No hyperparameter sweep in v0.1.0** — Q3 default = (a)
  Nie-et-al-ETT defaults. v0.1.1 may sweep patch_len / stride /
  d_model if H1 finishes T-MARGINAL.
- **No 1h-horizon PatchTST in v0.1.0** — Q4 default = (b) 24h.
  v0.1.1 may run 1h as a clean architecture-paradigm test if
  H4 falsifies.
- **No extended feature set in v0.1.0** — Q5 default = (a)
  carry-forward 5-feature input. Realized-vol bands / funding-rate /
  OI defer to v0.1.1.
- **No walk-forward retraining in v0.1.0** — Q6 default = (a)
  fixed BS-1 span.
- **No model-agnostic overlay refactor** — Q8 default = (a)
  sibling strategy. K6 enforces.
- **No mutation of v2.5 TCN code or anchors.** R9 + R10 are the
  load-bearing non-regression contracts.
- **No ADR-0033 or ADR-0035 amendment.** F-verdict algorithm and
  σ_train contract stay IMMUTABLE.
- **No vanilla Transformer / decoder-only Transformer** — that's
  v2.5b
  ([`v25b-transformer-overlay`](../v25b-transformer-overlay/feature.md)),
  phase 3 of the 4-phase roadmap.
- **No v2.6 bake-off** — phase 4
  ([`v26-forecast-bakeoff`](../v26-forecast-bakeoff/feature.md))
  is the canonical retirement gate; this feature contributes data
  to that gate without firing it.
- **No new external crate dependencies.** Per CLAUDE.md.
- **No iced bump.** Per CLAUDE.md.

## Sources cited

- **PatchTST paper**: Nie, Nguyen, Sinthong, Kalagnanam (2022) *A
  Time Series is Worth 64 Words: Long-term Forecasting with
  Transformers*. [arXiv:2211.14730](https://arxiv.org/abs/2211.14730).
  Reference implementation:
  [yuqinie98/PatchTST](https://github.com/yuqinie98/PatchTST)
  (MIT license).
- **iTransformer paper** (Q1 alternative): Liu, Hu, Yang, Liu, Cao,
  Long (2023) *iTransformer: Inverted Transformers Are Effective for
  Time Series Forecasting*. [arXiv:2310.06625](https://arxiv.org/abs/2310.06625).
  Reference implementation:
  [thuml/iTransformer](https://github.com/thuml/iTransformer) (MIT).
- [`spec/v25-tcn-horizon-bump-or-retire/feature.md`](../v25-tcn-horizon-bump-or-retire/feature.md)
  — direct predecessor; operator's Q1 = (b) retire-promote decision
  triggers this feature's activation. Quantitative-finance context
  for 24h horizon hypothesis carries forward.
- [`spec/v25-tcn-horizon-bump-or-retire/tasks.md`](../v25-tcn-horizon-bump-or-retire/tasks.md)
  — anchor gate baseline (26 PASS + 2 known-FAIL); the same baseline
  applies PRE-this-feature.
- [`spec/v25-tcn-threshold-tuning/feature.md`](../v25-tcn-threshold-tuning/feature.md)
  — predecessor; T-MARGINAL +0.018 / +0.045 Sharpe-delta result;
  the "beat +0.10" T-ALPHA-UNLOCKED threshold from
  [`v25-tcn-overlay`](../v25-tcn-overlay/feature.md) § success
  criterion that PatchTST must clear for H1 confirmation.
- [`spec/v25-tcn-recalibrate/feature.md`](../v25-tcn-recalibrate/feature.md)
  — σ_train recalibration; ADR-0035 emergence.
- [`spec/v25-tcn-alpha-investigation/feature.md`](../v25-tcn-alpha-investigation/feature.md)
  — F-verdict + 4-bucket failure-mode taxonomy; ADR-0033 emergence.
- [`spec/v25-tcn-overlay/feature.md`](../v25-tcn-overlay/feature.md)
  — parent feature (in-progress); R1-R12 cover TCN topology + size
  + loss + training schedule; PatchTST inherits the
  data/loss/optimiser/audit shape, replaces only the model.
- [`spec/v25-dl-forecast-overlay/feature.md`](../v25-dl-forecast-overlay/feature.md)
  — 4-phase DL roadmap; v0.1.0 of this feature is phase-2 ship.
- [`spec/v25b-transformer-overlay/feature.md`](../v25b-transformer-overlay/feature.md)
  — phase 3 sibling stub (queued; activates after v0.1.0 of this
  feature ships).
- [`spec/v26-forecast-bakeoff/feature.md`](../v26-forecast-bakeoff/feature.md)
  — phase 4 stub; canonical retirement gate.
- [ADR-0028](../architecture/adr/0028-v25-dl-forecast-overlay-candle.md)
  — candle ML framework choice; covers all 4 phases.
- [ADR-0029](../architecture/adr/0029-tcn-checkpoint-provenance.md)
  — checkpoint provenance; PatchTST canonical-arch-descriptor
  extends additively.
- [ADR-0032](../architecture/adr/0032-backtest-realdata-path-and-revision-pin.md)
  — backtest realdata path; PatchTST scenarios inherit.
- [ADR-0033](../architecture/adr/0033-tcn-alpha-investigation-report-shape.md)
  § D3 — F-verdict algorithm IMMUTABLE; PatchTST forecast-distribution
  reports use the same priority tree.
- [ADR-0034](../architecture/adr/0034-cockpit-training-control.md)
  — cockpit training control; `train_events` emission for the PatchTST
  training run.
- [ADR-0035](../architecture/adr/0035-tcn-sigma-train-recalibration.md)
  § D1 — post-training σ_train recalibration via metadata overlay;
  **explicitly named as cross-phase contract applying to v2.5a
  PatchTST**. In-loop accumulator pattern from
  [`train_tcn.rs:606,676-678,733-741`](../../crates/forecast/src/bin/train_tcn.rs)
  is DEPRECATED; PatchTST training scaffold uses the post-training
  pattern from the start (§ Negative precedent codified).
- [`crates/forecast/src/lib.rs:42`](../../crates/forecast/src/lib.rs)
  — `ForecastProvider` async trait; PatchTST implements verbatim.
- [`crates/forecast/src/tcn.rs`](../../crates/forecast/src/tcn.rs)
  — TCN model + forecaster impl; PatchTST mirrors the
  `load_anchor` / `load_from_paths` / `forecast()` surface.
- [`crates/forecast/src/bin/train_tcn.rs`](../../crates/forecast/src/bin/train_tcn.rs)
  — training scaffold; PatchTST gets a sibling `train_patchtst.rs`
  with the ADR-0035 § D1 post-training σ_train pattern from the
  start.
- [`crates/forecast/src/features.rs`](../../crates/forecast/src/features.rs)
  — 5-feature window + 1h-target derivation; PatchTST reuses
  (Q5 = (a)) with a configurable target horizon (Q4 = (b) 24h
  default).
- [`crates/forecast/src/overlay.rs`](../../crates/forecast/src/overlay.rs)
  — `combine()` pure function; reused verbatim.
- [`crates/strategy/src/tcn_overlay_momentum.rs`](../../crates/strategy/src/tcn_overlay_momentum.rs)
  — TCN overlay strategy; PatchTST gets a sibling
  `patchtst_overlay_momentum.rs` per Q8 = (a).
- [`crates/forecast/checkpoints/anchors/`](../../crates/forecast/checkpoints/anchors/)
  — TCN anchor naming convention
  (`tcn-bs{1,2}-<sha>.{safetensors,metadata.json,metadata.recalibrated.json}`);
  PatchTST follows the same with prefix `patchtst-bs1-`.
- [`crates/forecast/src/bin/forecast_distribution.rs`](../../crates/forecast/src/bin/forecast_distribution.rs)
  — F-verdict bin; extended additively for PatchTST dispatch.
- [`crates/forecast/src/bin/sharpe_comparison.rs`](../../crates/forecast/src/bin/sharpe_comparison.rs)
  — Sharpe-comparison bin; extended additively for PatchTST.
- [`crates/forecast/src/bin/recalibrate_sigma_train.rs`](../../crates/forecast/src/bin/recalibrate_sigma_train.rs)
  — σ_train recalibration bin; model-agnostic by ADR-0035 § D1
  construction; not used at v0.1.0 (PatchTST σ_train is
  canonical-at-ship) but available for future PatchTST recalibration.
- Apple Silicon Metal training-time empirical record: v2.5 TCN BS-1
  was ~4-5 days wall-clock at ~4.4M params; PatchTST at ~1.5-2M
  params (R1 defaults) projects ~5-7 days as a conservative
  estimate accounting for attention's higher per-step cost vs
  convolution. **Honest variance**: ±2 days. R8 cost tripwire
  bounds the worst case.

## Implementation

> Developer handoff summary — Wave A (T-D-N1..N16) + Wave D (T-D-N20..N26) complete 2026-05-21/22.

### Files created / modified

| File | Change | Lines |
|------|--------|-------|
| `crates/forecast/src/patchtst.rs` | NEW — full PatchTST model + ForecastProvider | ~800 |
| `crates/forecast/src/lib.rs` | EDITED — `pub mod patchtst` added | 1 |
| `crates/forecast/src/features.rs` | EDITED — `target_horizon_bars` field + WindowIterator update | ~25 |
| `crates/forecast/src/bin/train_patchtst.rs` | NEW — CLI training binary, AdamW+OneCycle+Huber, σ_train post-training | ~550 |
| `crates/forecast/Cargo.toml` | EDITED — `[[bin]] train_patchtst` + 4 `[[test]]` entries | 25 |
| `crates/forecast/tests/sigma_train_not_in_safetensors_patchtst.rs` | NEW — ADR-0035 D4 invariant (pre-Wave-B: 1 ignored) | ~130 |
| `crates/forecast/tests/forward_determinism_patchtst.rs` | NEW — K2 CPU byte-identity + Metal-vs-CPU drift (2 passed) | ~175 |
| `crates/forecast/tests/tcn_byte_identity.rs` | NEW — K6 scope-creep guard (1 passed) | ~130 |
| `crates/forecast/tests/patchtst_overlay_neutrality.rs` | NEW — K4 anchor-neutrality (#[ignore]d; run at M-D end) | ~200 |
| `crates/strategy/src/patchtst_overlay_momentum.rs` | NEW — PatchTST strategy sibling (7 unit tests pass) | ~320 |
| `crates/strategy/src/lib.rs` | EDITED — `pub mod patchtst_overlay_momentum` + re-exports | 6 |

### Key architectural decisions implemented

- **Channel-independence** via reshape `[batch, channels, n_patches, d_model]` → `[batch*channels, n_patches, d_model]` per Nie et al § 3.2 (ADR-0036 § D1).
- **Learnable position encoding** (NOT sinusoidal) — `[n_patches=41, d_model=128]` parameter, broadcast-expanded per batch (ADR-0036 § D1).
- **Custom MultiHeadSelfAttention** (~50 LoC) — avoids `candle_transformers::*` external API drift (ADR-0036 § D5). Added `.contiguous()?` after `transpose` to fix candle `MatMulUnexpectedStriding` error.
- **σ_train post-training** — `compute_sigma_train_post_training()` is called AFTER the training loop with frozen weights. `Vec<f32>` accumulator is INSIDE the function scope, NOT outside the per-epoch loop (ADR-0035 § D1, ADR-0036 § D3). Negative precedent: `train_tcn.rs:606,676-678` (in-loop accumulator, deprecated).
- **Cost tripwire** — `assert_epoch_budget(epoch_n, wall_clock_sec, history)`: hard limit 24h + 3× rolling median. On fire: log error + write `/tmp/train_patchtst-bs1-tripwire-epoch{N}.txt` + continue training (ADR-0036 § D4).
- **param count** — 431,105 (within 300k-600k range per ADR-0036 § D1; ~10× smaller than TCN 4.4M).

### Smoke test verified (T-D-N11)

```
INFO train_patchtst: epoch complete epoch=1 total_epochs=1 train_loss=0.004341 model_family="patchtst" scenario=bs1
INFO train_patchtst: sigma_train derived via post-training frozen pass sigma_train=2.088574
INFO train_patchtst: checkpoint written safetensors=crates/forecast/checkpoints/anchors/patchtst-bs1-6471e4dc...safetensors
```

Span 2023-01-01..2023-04-01 (3 months, 11030 windows). Smoke-test checkpoint removed after verification; Wave B uses full-year 2023 span.

### K6 invariant (T-D-N15)

`git diff --quiet HEAD -- crates/forecast/src/tcn.rs` exits 0 — TCN source unchanged.

### Wave D results (T-D-N20..N26) — developer complete 2026-05-21/22

| Report | SHA (body, deterministic) | F-verdict / headline |
|--------|--------------------------|----------------------|
| `forecast-distribution-patchtst-bs1-realdata-20260521.md` | `c55c6c5178374f230f5273df1e20d121589ff0b879c20062ee6cbdca7f4646dd` | **F4** — no collapse; σ_train calibrated; 55.8% bars survive confidence gate |
| `backtest-20260521-220035-top10-2023-fy-patchtst-overlay-realdata.md` | `5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c` | +31.13% total return; dampened=1745/6203 signals; Sharpe (ann) 0.009243 |
| `sharpe-comparison-patchtst-bs1-realdata-20260521.md` | `45140833cf13a9bcdcbe464684f61d1a8566c9d5d28b7667c2dc056b1063bfb9` | Sharpe delta (PatchTST vs v1 baseline): **+0.006144** — below +0.10 T-ALPHA-UNLOCKED |

**Candidate anchor SHAs for tester:**
- `forecast-distribution-patchtst-bs1-realdata`: `c55c6c5178374f230f5273df1e20d121589ff0b879c20062ee6cbdca7f4646dd`
- `top10-2023-fy-patchtst-overlay-realdata`: `5f303cc0812d421e6efdc40c0f412dd8cc0625891c677442bf2d7d2d5336ab4c`

Wave D files modified/created:
- `crates/forecast/src/bin/forecast_distribution.rs` — `PatchtstBs1` variant + `CheckpointHandle` dispatch
- `crates/forecast/src/patchtst.rs` — `AnchorScenario::Bs1::sha_prefix()` fixed to actual Wave B SHA
- `crates/strategy/src/patchtst_sync.rs` — NEW re-export module
- `crates/strategy/src/lib.rs` — `pub mod patchtst_sync` added
- `crates/backtest/src/scenarios/patchtst_overlay_weights.rs` — NEW backtest scenario
- `crates/backtest/src/scenarios/mod.rs` — `pub mod patchtst_overlay_weights` added
- `crates/backtest/src/main.rs` — `PatchtstOverlayMomentumWeights` variant + dispatch + `report_dir_for_scenario` mapping
- `crates/forecast/src/bin/sharpe_comparison.rs` — `SCENARIOS[5]`, `render_report(&[RerunResult;5])`, PatchTST verdict row, renamed filename to `sharpe-comparison-patchtst-bs1-realdata-*.md`

Anchors verified: 28/28 PASS via `scripts/verify_anchors.sh`.

### Wave B readiness

All 16 T-D-N rows ticked. Wave B training command:

```bash
RUST_LOG=info,forecast=debug \
  cargo run -p forecast --release --features candle --bin train_patchtst -- \
    --scenario bs1 \
    --target-horizon-bars 24 \
    --span-start 2023-01-01 --span-end 2023-12-31 \
    --patch-len 16 --stride 8 \
    --d-model 128 --n-heads 4 --d-ff 256 --n-layers 3 --dropout 0.2 \
    --context-len 336 \
    --epochs 30 --batch-size 128 \
    --seed 0x00C0FFEE \
    2>&1 | tee /tmp/train_patchtst-bs1.log &
```

Watch recipe (copy-pasteable):

```bash
watch -n 60 '
PID=$(pgrep -f train_patchtst | head -1)
[ -z "$PID" ] && echo "train_patchtst not running" && exit
N=$(grep -c "epoch complete" /tmp/train_patchtst-bs1.log 2>/dev/null || echo 0)
LAST=$(grep "epoch complete" /tmp/train_patchtst-bs1.log 2>/dev/null | tail -1 | grep -oE "epoch=[0-9]+" | cut -d= -f2 || echo 0)
ELAPSED=$(ps -o etime= -p $PID 2>/dev/null | awk "{gsub(/^ +/,\"\"); n=split(\$0,a,/[-:]/); if(n==2)print a[1]*60+a[2]; else if(n==3)print a[1]*3600+a[2]*60+a[3]; else if(n==4)print a[1]*86400+a[2]*3600+a[3]*60+a[4]}")
[ "$N" -gt 0 ] && echo "epoch $LAST/30 ($((N*100/30))%), elapsed ${ELAPSED}s, remaining ~$(((30-N)*ELAPSED/N/60)) min" || echo "warmup: 0 epochs (elapsed=${ELAPSED}s)"
'
```

## Changelog

- 2026-05-21 (analyst): initial brief authored, replacing the
  2026-05-17 stub. R1-R10 requirements, H1-H4 hypothesis register,
  K1-K6 risk register, Q1-Q8 operator-decide questions (all
  carrying analyst-recommended defaults; "autoapprove" activates
  the bundle). Non-regression contract (12 invariants).
  Acceptance per milestone (M-OD / M-T1 / M-D / M-FORECAST-DIST /
  M-SHARPE / M-FINAL / M-PRESENTER). MVP scope = Q2 = (a) full
  ship: code + 1 BS-1 PatchTST checkpoint + F-verdict report +
  Sharpe-comparison + strategy integration + 2 anchors under
  `v2.5a.0-patchtst`. Cost estimate: ~3-5 weeks best case;
  ~5-7 weeks with one Wave-B retry. Predecessor:
  `v25-tcn-horizon-bump-or-retire v0.1.0` (operator-decided Q1 =
  (b) retire-promote-PatchTST 2026-05-21). Parent:
  `v25-dl-forecast-overlay v0.0.0 (roadmap)`. Trace row
  `REQ-V25A-PATCHTST-001` promoted `roadmap → draft` with
  expanded title + scoped `crates` / `arch` columns. Promoted
  Queue/Strategy → Active in `spec/backlog.md` (ACTIVATION
  TRIGGERED tag cleared). HANDOFF → operator-decide (Q1-Q8) →
  architect for M-T1.
- 2026-05-17 (orchestrator): phase 2 stub opened as part of
  4-phase DL roadmap. Status: roadmap (pending phase 1 ship).
  (Superseded by this entry; preserved in version control.)
