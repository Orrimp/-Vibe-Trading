# crates/forecast

Model-agnostic `ForecastProvider` trait + overlay-composition helpers for v2.5+.

## Status

Scaffold-only as of 2026-05-16. The concrete forecaster implementation lands
per-feature. v2.5 currently targets a small custom Transformer/TCN trained in
`candle` (the project's named prototyping framework per `CLAUDE.md`).

See [`spec/v25-dl-forecast-overlay/feature.md`](../../spec/v25-dl-forecast-overlay/feature.md)
for the active brief and [`spec/architecture/12-forecast-overlay.md`](../../spec/architecture/12-forecast-overlay.md)
for the overlay design pattern.

## Public surface

- `ForecastProvider` — async trait every backend implements (one method, no
  streaming, no tool-use).
- `overlay::combine()` — pure function fusing a base-strategy signal with a
  forecast overlay (agree+confident → boost; disagree+confident → dampen;
  flat/low-confidence → pass-through).

Value types (`ForecastOverlay`, `Direction`, `ForecastRequest`,
`ForecastResponse`, `ForecastError`, `OhlcvBar`, `SamplingParams`) live in
`crates/core/src/forecast.rs` so they cross every consumer cleanly.

## Caching

Strict-replay mode reuses `crates/replay-cache/` (the generic SQLite WAL
content-addressed cache extracted from the v2 LLM record/replay pattern).
