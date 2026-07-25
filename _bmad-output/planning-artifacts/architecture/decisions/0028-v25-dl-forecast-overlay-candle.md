---
adr: 0028
title: v2.5 — DL forecast overlay trained in `candle` (model-agnostic crate scaffolding preserved)
status: accepted
date: 2026-05-16
supersedes: 0027
superseded-by: none
---

# ADR-0028: v2.5 — DL forecast overlay trained in `candle`

## Context

v2.5 fills the DL-forecaster slot in
[product.md § Strategy library roadmap](../../../../docs/archive/pre-bmad-spec/product.md#strategy-library--roadmap).
The original ADR-0027 picked the [Kronos](https://github.com/shiyu-coder/Kronos)
pre-trained foundation model with ONNX + `tract` in-process serving (Option B
of the three integration paths originally evaluated).

Wave A bootstrap (`crates/forecast/` + `crates/replay-cache/` + `crates/core/`
value types) landed cleanly on 2026-05-16, but the M2 ONNX-conversion step
surfaced three load-bearing problems with Kronos:

1. **Outside `transformers`.** Kronos's `config.json` on Hugging Face contains
   only model hyperparameters — no `model_type`, no `architectures`, no
   `auto_map`. `transformers.AutoModel.from_pretrained()` cannot route the
   load. Requires vendoring the upstream
   [shiyu-coder/Kronos](https://github.com/shiyu-coder/Kronos) repo's custom
   model class.
2. **Two-model architecture.** Kronos is `KronosTokenizer` (VQ-VAE-like
   encoder/decoder) + `Kronos` (autoregressive transformer over discrete
   tokens), with the sampling loop in a third class `KronosPredictor` that is
   plain Python — not an `nn.Module`. Option B as originally scoped ("one
   ONNX → `tract::run()` → done") is not viable. Realised Option B requires
   exporting two ONNX files plus reimplementing the autoregressive sampling
   loop (temperature / top-p / top-k) in Rust against `tract` — substantial
   ML engineering work that was never scoped.
3. **Domain-fit unvalidated.** The Kronos evaluation captured license,
   architecture, and integration paths. It never benchmarked Kronos forecasts
   against crypto K-line data. The training corpus is "K-line from 45+ global
   exchanges" — likely equities-heavy per the AAAI 2026 paper focus. The
   engineering-convenience pitch ("pre-trained = free win") masked an
   unmeasured product-fit risk.

The operator reframed the project goal as
*"a real, working, auditable agent architecture combining numeric models + DL
+ LLM with persistent memory + audit ledger — and the operator learns by
building it."* In that frame the pre-trained-foundation pitch loses much of
its appeal: black-box weights have low learning value, the engineering cost
to integrate Kronos is climbing, and the domain-fit risk is still there.

## Decision

**Train a small custom Transformer or TCN on crypto K-line data using
[`candle`](https://github.com/huggingface/candle), the project's named
prototyping framework per [CLAUDE.md](../../../../CLAUDE.md).**

Concrete shape (the v2.5 analyst will refine):

- Model: small Transformer or TCN, sized to fit the operator's local compute
  budget (~10M parameters as a starting point — concrete choice is the new
  analyst's call).
- Training: in-process via `candle`, on crypto K-line data already available
  via `crates/data/src/replay_feed.rs` (parquet + binance feed).
- Inference: pure-Rust via `candle` at runtime. No ONNX. No `tract`. No
  Python at runtime.
- Audit: every training run and every inference request is journalled
  (model_revision pinned by training-run hash; inference results
  audit-logged with `correlation_id`).
- Reflection: training rounds + inference quality feed the v1.8
  reflection-memory loop.

## Consequences

### Preserved from ADR-0027 / Wave A

- [`crates/forecast/`](../../../../crates/forecast) — `ForecastProvider` trait
  + `overlay::combine()` are model-agnostic by design; both stay.
- [`crates/replay-cache/`](../../../../crates/replay-cache) — generic SQLite WAL
  content-addressed cache; reused by the new DL forecaster for strict-replay
  determinism the same way it would have been used for Kronos.
- [`crates/core/src/forecast.rs`](../../../../crates/core/src/forecast.rs) —
  domain value types (`ForecastOverlay`, `Direction`, `ForecastRequest`,
  `ForecastResponse`, `ForecastError`, `OhlcvBar`, `SamplingParams`) are
  model-agnostic; all stay.
- [`docs/archive/pre-bmad-spec/architecture/12-forecast-overlay.md`](../../../../docs/archive/pre-bmad-spec/architecture/12-forecast-overlay.md) —
  cross-cutting overlay design pattern; stays (lightly genericised — drops
  Kronos-specific clauses).
- The signal-level overlay composition decision (overlay on v1 momentum;
  agree+confident → boost, disagree+confident → dampen, low-confidence →
  pass-through) is independent of model choice and stays.

### Removed (Kronos-specific)

- `crates/forecast/src/kronos.rs` — stub deleted.
- `crates/forecast/build.rs` — ONNX checksum gate deleted.
- `crates/forecast/assets/kronos-base.onnx.{sha256,license}` — deleted.
- `scripts/dev/kronos_torch_to_onnx.py` — deleted.
- `.gitattributes` — LFS rule for `*.onnx` deleted (no longer needed; candle
  weights are small `.safetensors`).
- `docs/archive/pre-bmad-spec/v25-kronos-forecast-overlay/` — renamed to
  `docs/archive/pre-bmad-spec/v1/v25-dl-forecast-overlay/`; body rewritten for the analyst's new pass.

### Anchor implications

No change to existing locked anchors (11/11). v2.5 still locks 2 new anchors
at ship: BS-1 (2023 full-year top-10 USDT) and BS-2 (2024 full-year top-10
USDT) — the operator-locked backtest baseline carries forward from the Kronos
ADR.

### Learning surface (new, vs. ADR-0027)

This decision **maximises the operator's learning** per the reframed project
goal. Building a Transformer/TCN from scratch in `candle` covers:

- Model architecture choice + sizing.
- Tokenisation / discretisation strategy for OHLCV (the operator's call, not
  inherited from Kronos's VQ-VAE).
- Training loop in pure Rust (`candle` + `candle-nn` + `candle-transformers`).
- Loss function + backtest-targeted evaluation.
- Inference + sampling loop in Rust (no `tract` op-set risk).
- Pinning weights + reproducibility via `replay-cache`.

ADR-0027 would have produced an integrated black box. ADR-0028 produces a
learnable system.

## Open questions (for the new analyst)

The 13 Kronos-specific open questions from ADR-0027 are mostly obsolete. The
new v2.5 analyst owns a fresh set: model size / architecture (Transformer vs
TCN vs hybrid), tokenisation strategy, training data span, loss function,
inference horizon, success criterion vs v1 momentum baseline, training
checkpoint storage, audit integration shape. The new analyst pass at
[`docs/archive/pre-bmad-spec/v1/v25-dl-forecast-overlay/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/v25-dl-forecast-overlay/feature.md)
authors these.

## References

- ADR-0027 (superseded) — Kronos ONNX + `tract` decision.
- [`docs/archive/pre-bmad-spec/v1/v25-dl-forecast-overlay/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/v25-dl-forecast-overlay/feature.md)
  — active v2.5 brief.
- [`docs/dev-notes/archive/2026-Q2/kronos-evaluation-2026-05-10.md`](../../../../docs/dev-notes/archive/2026-Q2/kronos-evaluation-2026-05-10.md)
  — preserved as "what-not-to-do" reference for the new analyst.
- [CLAUDE.md](../../../../CLAUDE.md) — `candle` named as the project's
  prototyping ML framework.
