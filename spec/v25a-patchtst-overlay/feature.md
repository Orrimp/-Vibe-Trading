---
slug: v25a-patchtst-overlay
status: roadmap
owner: pending-analyst
updated: 2026-05-17
version: 2.5.1
parent: v25-dl-forecast-overlay v2.5.0 (roadmap)
predecessor: v25-tcn-overlay v2.5.0
---

# v2.5a — PatchTST / iTransformer overlay (phase 2 of 4)

> **Queued phase 2 of the 4-phase DL roadmap** at
> [`v25-dl-forecast-overlay`](../v25-dl-forecast-overlay/feature.md).
> Model family: **patch-based Transformer** (Nie et al 2023 *A Time
> Series is Worth 64 Words* — PatchTST; Liu et al 2024 *iTransformer*).
> Built second so it reuses the training loop, checkpoint provenance
> hashing, audit emission, and replay-cache wiring established by
> phase 1 (TCN).

## Why

Per the [4-phase roadmap](../v25-dl-forecast-overlay/feature.md): a patch-
based Transformer is a meaningfully different paradigm from TCN's dilated
convolutions. Both architectures forecast time series, but they exploit
different inductive biases:

- TCN — local-to-distant via dilated causal convolutions.
- PatchTST / iTransformer — patches of contiguous bars as tokens; channel-
  independence; full self-attention across patches.

Building both lets the operator measure where each wins on the same
data, same baseline, same evaluation. Empirical bake-off > literature
claims.

Stays `status: roadmap` until phase 1 (TCN) ships, at which point this
phase activates and gets its own analyst pass.

## Carry-forward invariants

Same data (10 USDT pairs, 2023+2024 hourly), same backtest scenarios
(BS-1, BS-2), same overlay shape (signal-level on v1 momentum), same
audit shape, same cost telemetry — see
[parent roadmap](../v25-dl-forecast-overlay/feature.md) for the full
list.

## Changelog

- 2026-05-17 (orchestrator): phase 2 stub opened as part of 4-phase
  DL roadmap. Operator chose to build all three model families per
  reading list 2026-05-16. Status: roadmap (pending phase 1 ship).
