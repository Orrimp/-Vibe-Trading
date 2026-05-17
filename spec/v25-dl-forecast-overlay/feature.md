---
slug: v25-dl-forecast-overlay
status: roadmap
owner: orchestrator
updated: 2026-05-17
version: 2.5.0
predecessor: v2-llm-strategy v2.0.0
supersedes: v25-kronos-forecast-overlay (dropped 2026-05-16 — see ADR-0028)
---

# v2.5 — DL forecast overlay (4-phase roadmap)

> **Umbrella roadmap, not an active feature.** This folder coordinates a
> multi-phase initiative that builds three model families in sequence so
> the operator can compare them empirically before retiring to a canonical
> v2.5 overlay. Each phase is its own feature folder + ship.

Operator-locked direction (2026-05-17): build **all three model families**
from [v25-dl-reading-list-2026-05-16](../dev-notes/v25-dl-reading-list-2026-05-16.md)
sequentially, sharing infrastructure (training loop, data loader,
checkpoint provenance, audit emission, replay-cache wiring). Each ship is
independently reviewable; v2.6 picks the canonical winner.

## Phases

| Phase | Slug | Model family | Status |
|-------|------|--------------|--------|
| **v2.5**  | [`v25-tcn-overlay`](../v25-tcn-overlay/feature.md) | Temporal Convolutional Network (Bai et al 2018) | active — first to ship |
| v2.5a | [`v25a-patchtst-overlay`](../v25a-patchtst-overlay/feature.md) | PatchTST / iTransformer (patch-based Transformer) | queued |
| v2.5b | [`v25b-transformer-overlay`](../v25b-transformer-overlay/feature.md) | Vanilla decoder-only Transformer | queued |
| v2.6  | [`v26-forecast-bakeoff`](../v26-forecast-bakeoff/feature.md) | bake-off + retirement (pick canonical, mark others research-mode) | queued |

Phases ship serially. Each later phase reuses the prior phase's training
infrastructure (data loader, checkpoint provenance hashing, audit emission,
replay-cache wiring). Code-level shared surface lives in `crates/forecast/`
(model-agnostic) — already built per Wave A 2026-05-16.

## Why all three

Per operator decision 2026-05-17 (in response to "all three sound nice; can
we build all?"):

- **Maximum learning** — three different DL paradigms (dilated convolution
  vs patch attention vs autoregressive attention) directly serves the
  reframed project goal "operator learns by building."
- **Empirical bake-off > literature claims** — academic time-series
  forecasting work uses standardised benchmarks (M4, ETT) that don't
  cover crypto OHLCV. Each architecture's crypto fit is unknown until
  we measure on real data.
- **Shared infrastructure compounds** — the `ForecastProvider` trait,
  audit hooks, replay-cache primitive, and training loop are built once
  and reused. Marginal cost of each additional model family drops.
- **Risk reduction via the v2.6 retirement gate** — the canonical v2.5
  forecaster lands in production *after* evidence, not before.

## Shared infrastructure (already shipped, model-agnostic)

| Crate / file | What |
|--------------|------|
| [`crates/forecast/`](../../crates/forecast/) | `ForecastProvider` async trait + `overlay::combine()` pure function |
| [`crates/replay-cache/`](../../crates/replay-cache/) | Generic SQLite WAL content-addressed cache (namespace = `"forecast"`) |
| [`crates/core/src/forecast.rs`](../../crates/core/src/forecast.rs) | Domain value types: `ForecastOverlay`, `Direction`, `ForecastRequest`, `ForecastResponse`, `ForecastError`, `OhlcvBar`, `SamplingParams` |
| [`crates/data/src/bin/fetch_binance_klines.rs`](../../crates/data/src/bin/fetch_binance_klines.rs) | Historical K-line downloader (one-shot bootstrap) |
| [`spec/architecture/12-forecast-overlay.md`](../architecture/12-forecast-overlay.md) | Cross-cutting overlay design pattern (signal-level composition) |
| [`spec/architecture/adr/0028-v25-dl-forecast-overlay-candle.md`](../architecture/adr/0028-v25-dl-forecast-overlay-candle.md) | Model-agnostic candle-direction decision (covers all 4 phases) |

## Per-phase invariants (carried through all four)

- **Same data**: 10 USDT pairs (ADA/AVAX/BNB/BTC/DOGE/DOT/ETH/LINK/SOL/XRP),
  hourly bars, 2023 + 2024 full year. Bootstrapped via
  `cargo run -p data --bin fetch_binance_klines`.
- **Same backtest scenarios**: BS-1 (2023 full-year top-10 USDT),
  BS-2 (2024 full-year top-10 USDT).
- **Same overlay shape**: signal-level overlay on v1 cross-sectional
  momentum baseline.
- **Same audit shape**: `JournalEntry { kind: "forecast_emitted", … }`
  per phase, with `model_revision` hashed from training-run artifacts.
- **Same cost telemetry**: `CostEvent::Infra { line: "forecast_inference", … }`
  default-zero dollars.
- **Same hardware constraint**: Apple Silicon M-series via candle Metal
  backend; ~5-10M params per model.

## Anchor strategy

Each phase locks two new anchors at its own ship — total +8 anchors
across v2.5/v2.5a/v2.5b/v2.6. Existing 11 anchors stay byte-identical
across all four phases (model defaults to zero-cost; opt-in operators
diverge deterministically per-config).

## Why this is a roadmap, not a feature

The `status: roadmap` matches the `lumen-design-adoption` precedent
(multi-phase initiative coordinator). No code lives here; each phase
owns its own feature folder, tasks, and reports.

## Changelog

- 2026-05-17 (orchestrator): reframed as 4-phase roadmap. v2.5 narrowed
  to TCN; v2.5a (PatchTST), v2.5b (vanilla Transformer), v2.6 (bake-off)
  opened as queued feature folders. Operator decision after reading
  [`spec/dev-notes/v25-dl-reading-list-2026-05-16.md`](../dev-notes/v25-dl-reading-list-2026-05-16.md):
  build all three families for empirical bake-off rather than picking
  one upfront. Status: draft → roadmap.
- 2026-05-16 (orchestrator): feature replaced — was
  `v25-kronos-forecast-overlay`, now `v25-dl-forecast-overlay`. Kronos
  dropped per [ADR-0028](../architecture/adr/0028-v25-dl-forecast-overlay-candle.md).
  Wave A crate scaffolding preserved; Kronos-specific files removed.
