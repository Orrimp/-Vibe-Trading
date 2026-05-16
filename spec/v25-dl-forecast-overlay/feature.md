---
slug: v25-dl-forecast-overlay
status: draft
owner: pending-analyst
updated: 2026-05-16
version: 2.5.0
predecessor: v2-llm-strategy v2.0.0
supersedes: v25-kronos-forecast-overlay (dropped 2026-05-16 — see ADR-0028)
---

# v2.5 — DL forecast overlay (candle-trained, model-agnostic scaffold preserved)

> **Stub awaiting analyst.** This feature replaces the dropped Kronos
> approach. Direction is locked by [ADR-0028](../architecture/adr/0028-v25-dl-forecast-overlay-candle.md):
> **train a small custom Transformer/TCN on crypto K-line data using
> [`candle`](https://github.com/huggingface/candle)** — the project's named
> prototyping ML framework per [CLAUDE.md](../../CLAUDE.md). No pre-trained
> foundation model. No ONNX. No `tract`. No Python at runtime.

## Why

Fills the v2.5 row in
[`spec/product.md` § Strategy library roadmap](../product.md#strategy-library--roadmap).
The original entry was "TCN or small Transformer"; ADR-0027 substituted
Kronos as a pre-trained shortcut, then was superseded by ADR-0028 after
Wave A bootstrap surfaced three load-bearing problems with Kronos (lives
outside `transformers`, two-model architecture requires Rust-side sampling
loop, crypto-fit unvalidated).

The reframed goal is:

> *A real, working, auditable agent architecture combining numeric models +
> DL + LLM with persistent memory + audit ledger — and the operator learns
> by building it.*

Training a small Transformer/TCN end-to-end in Rust delivers all three:
**real working** (no LFS-vendored 410 MB black box; weights live in repo
as small `.safetensors`), **auditable** (every training run + every
inference is journalled with `model_revision` hashed from the run), and
**learnable** (operator builds model + training loop + inference end-to-end
in Rust).

## Requirements

_analyst fills this_

Open question seeds (the new analyst owns these; not exhaustive):

- Model family (Transformer vs TCN vs hybrid) — what fits the operator's
  local compute budget? what fits OHLCV time-series with audit-friendly
  provenance?
- Model size (params) — target inference latency + checkpoint size.
- Tokenisation / discretisation of OHLCV — continuous regression vs
  discrete-token classification vs hybrid.
- Training data span — which symbols, which years, which timeframes. Reuse
  the v1 momentum top-10 USDT universe?
- Loss function + evaluation criterion.
- Inference horizon — single-bar (matches v1 momentum cadence) vs N-bar
  rolling.
- Success criterion — what does "better than v1 momentum baseline" mean
  measurably?
- Training checkpoint storage shape — `.safetensors` in `assets/`?
  `crates/forecast/checkpoints/`? Versioned by training-run hash.
- Audit integration — every inference posts `JournalEntry { kind:
  "forecast_emitted", model_revision, correlation_id, … }`.

## Design

_architect fills this after analyst handoff_

Carried forward from the superseded Kronos design (model-agnostic clauses
only):

- **Signal-level overlay on v1 momentum** — agree+confident → boost,
  disagree+confident → dampen, flat/low-confidence → pass-through. See
  [`spec/architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md).
- **Inheritance of v2 LLM Q8 record/replay** — strict-replay determinism
  via `crates/replay-cache/` (already built).
- **`ForecastProvider` trait** — concrete forecaster implements this.
  Already built at `crates/forecast/src/lib.rs`.

## Backtest Scenarios

Carried forward from the operator-locked decision:
**BS-1 (2023 full-year top-10 USDT)** and
**BS-2 (2024 full-year top-10 USDT)**.

The architect re-confirms these once the analyst lands the model + universe
choices.

## Implementation

_developer fills this_

Crate scaffolding already in place (Wave A, 2026-05-16):

- `crates/forecast/` — `ForecastProvider` trait + `overlay::combine()`.
- `crates/replay-cache/` — generic SQLite WAL content-addressed cache.
- `crates/core/src/forecast.rs` — value types (`ForecastOverlay`,
  `Direction`, `ForecastRequest`, `ForecastResponse`, `ForecastError`,
  `OhlcvBar`, `SamplingParams`).

The candle-specific forecaster lands inside `crates/forecast/src/` once the
analyst + architect close on the model choice.

## Verification

_tester fills this — anchor-locks BS-1 and BS-2 at ship_

## Changelog

- 2026-05-16 (orchestrator): feature replaced — was `v25-kronos-forecast-overlay`,
  now `v25-dl-forecast-overlay`. Kronos dropped per [ADR-0028](../architecture/adr/0028-v25-dl-forecast-overlay-candle.md).
  Wave A crate scaffolding preserved; Kronos-specific files removed.
  HANDOFF → analyst.
