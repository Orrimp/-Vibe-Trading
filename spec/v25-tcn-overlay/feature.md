---
slug: v25-tcn-overlay
status: draft
owner: pending-analyst
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
and the [4-phase roadmap](../v25-dl-forecast-overlay/feature.md):
train a small TCN on crypto K-line data using `candle` so that:

1. The training loop, checkpoint provenance hashing, audit emission, and
   replay-cache wiring all exist as reusable infrastructure for phases
   v2.5a (PatchTST) and v2.5b (vanilla Transformer).
2. The TCN itself produces directionally useful forecasts on real crypto
   K-line data, measured against the v1 cross-sectional momentum baseline.
3. The operator learns how dilated causal convolutions work end-to-end —
   architecture choice, receptive-field math, residual blocks, training
   loop with MSE/MAE on regression or CE on quantised targets.

## Requirements

_analyst fills this_

Open question seeds for the analyst (operator-locked from the
4-phase decision; analyst refines):

- **Q1 — TCN topology.** Number of dilated conv blocks; dilation
  schedule (e.g. `[1, 2, 4, 8, 16, 32, 64, 128]`); kernel size (3 or 5
  typical). Receptive-field budget vs context-window length.
- **Q2 — Model size.** Channels per layer + number of layers → param
  count target (~3-10M for M-series Metal budget).
- **Q3 — Tokenisation / target shape.** Continuous regression (predict
  next-bar OHLCV as `Decimal`s) vs quantile classification (predict
  `P(return_t+1 ∈ bin_k)` over K bins). Continuous is simpler; quantile
  maps more cleanly to `Direction + confidence` in `ForecastOverlay`.
- **Q4 — Context window.** How many bars of history per inference. Should
  match TCN's effective receptive field, not exceed it.
- **Q5 — Loss function.** MSE / MAE / Huber on continuous targets, OR
  cross-entropy on quantile bins. Trade-off: outlier sensitivity (MSE) vs
  median-target stability (MAE) vs calibrated-confidence output (CE).
- **Q6 — Output → `ForecastOverlay`.** Direct emission of `Direction` +
  `confidence` from the model head, OR post-process from continuous next-
  bar prediction. Affects the `confidence` semantics.
- **Q7 — Training schedule.** Epochs, batch size, optimiser (AdamW
  default), learning-rate schedule, validation split (2023 train /
  2024 val? rolling-window CV?).
- **Q8 — Checkpoint provenance.** What gets SHA-256-hashed into
  `model_revision` (architecture config + weights + training-data span
  + seed). Stored where (`crates/forecast/checkpoints/<sha>.safetensors`?).

## Design

_architect fills this after analyst handoff_

Carry-forward from [`architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md):

- Signal-level overlay on v1 cross-sectional momentum.
- `ForecastProvider::forecast()` async trait implemented by `TcnForecaster`.
- Strict-replay determinism via `crates/replay-cache/` (namespace
  `"forecast"`); cache key includes `model_revision`.
- Audit row per call: `JournalEntry { kind: "forecast_emitted", … }`.

## Backtest Scenarios

Per the 4-phase invariants:
**BS-1 (2023 full-year top-10 USDT)** and
**BS-2 (2024 full-year top-10 USDT)**.

Anchors `top10-2023-fy-tcn-overlay` and `top10-2024-fy-tcn-overlay`
locked at ship.

## Implementation

_developer fills this_

Slot in `crates/forecast/src/tcn.rs` (new). Re-exports the
`TcnForecaster` impl behind the `ForecastProvider` trait. Training
loop in `crates/forecast/src/bin/train_tcn.rs` (new bin) — reads
parquet from `data/binance/`, writes checkpoint to
`crates/forecast/checkpoints/<sha>.safetensors`.

## Verification

_tester fills this — anchor-locks BS-1 + BS-2 at ship + smoke test of
training loop + inference reproducibility against the replay cache_

## Changelog

- 2026-05-17 (orchestrator): phase 1 of 4 opened. Direction locked
  (TCN family). 8 open questions seeded for the analyst. Awaits
  analyst pass (task #25 will retarget at this phase).
