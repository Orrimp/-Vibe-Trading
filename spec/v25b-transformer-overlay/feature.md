---
slug: v25b-transformer-overlay
status: roadmap
owner: pending-analyst
updated: 2026-05-17
version: 2.5.2
parent: v25-dl-forecast-overlay v2.5.0 (roadmap)
predecessor: v25a-patchtst-overlay v2.5.1
---

# v2.5b — Vanilla decoder-only Transformer overlay (phase 3 of 4)

> **Queued phase 3 of the 4-phase DL roadmap** at
> [`v25-dl-forecast-overlay`](../v25-dl-forecast-overlay/feature.md).
> Model family: **vanilla decoder-only Transformer** (Radford et al
> 2019 GPT-2-style architecture, applied to discretised OHLCV tokens).

## Why

Per the [4-phase roadmap](../v25-dl-forecast-overlay/feature.md): a
vanilla autoregressive Transformer rounds out the three paradigms:

- TCN — dilated causal convolutions (phase 1).
- PatchTST / iTransformer — patch-based attention (phase 2).
- Vanilla decoder-only Transformer — autoregressive next-token over a
  discretised OHLCV alphabet (this phase).

This is closest in spirit to the Kronos shape that was dropped — but
built from scratch in `candle` with the operator's own tokenisation
choices, no pre-trained weights, no Python at runtime. Where Kronos
was a black-box pre-trained model, this is the operator's hand-built
equivalent — full provenance, full audit, full understanding.

Stays `status: roadmap` until phases 1 + 2 ship.

## Carry-forward invariants

Same data, scenarios, overlay shape, audit shape, cost telemetry as
the [parent roadmap](../v25-dl-forecast-overlay/feature.md).

## Changelog

- 2026-05-17 (orchestrator): phase 3 stub opened. Status: roadmap
  (pending phases 1 + 2).
