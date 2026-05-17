---
slug: v25-tcn-overlay
status: in-progress
owner: analyst
updated: 2026-05-17
---

# Tasks — v2.5 TCN forecast overlay

> Analyst pass landed 2026-05-17 — eight open questions closed in
> [feature.md § Requirements](feature.md#requirements) (R1–R12). Two
> **operator-decide** questions surfaced. Architect picks up after
> operator answers them.

## Pending — operator-decide (blocks architect)

- [ ] **T-OP-1** — Operator answers: anchor checkpoint storage
  (LFS-track vs regenerate-from-seed). See feature.md R8. Default
  recommendation: LFS-track.
- [ ] **T-OP-2** — Operator confirms two-checkpoint backtest split
  (one TCN checkpoint per backtest scenario, each strictly OOS for its
  evaluation period). See feature.md R7. Default recommendation:
  confirm.

## Pending — architect

- [ ] **T-AR-1** — Architect locks the Design section in feature.md:
  Conv1d residual-block layout, 1×1 skip projection vs identity,
  Metal-vs-CPU determinism strategy, parquet → feature-window iterator
  design, metadata-JSON canonicalisation rules, threshold defaults for
  `tcn_overlay_momentum`.
- [ ] **T-AR-2** — Architect decomposes the M0–M7 milestone skeleton in
  feature.md § Implementation into ordered developer tasks
  (T-D-1 … T-D-N), each with a one-line acceptance criterion.
- [ ] **T-AR-3** — Architect spawns ADR-0029 (or extends ADR-0028)
  documenting the locked TCN topology (R1), model size (R2), and
  checkpoint-provenance schema (R8). The schema is the contract the
  v2.5a / v2.5b phases must echo with model-family-appropriate changes.

## Pending — developer (placeholder; architect refines)

Skeleton mapping milestones → developer tasks. Architect re-numbers
and adds acceptance criteria in T-AR-2.

- [ ] **T-D-1 (M0)** — Build `crates/forecast/src/features.rs` —
  parquet → 5-feature 256-bar window iterator. Property tests for
  determinism.
- [ ] **T-D-2 (M1)** — `TcnForecaster` struct + forward pass in
  `candle`. Random-init checkpoint round-trip test. Verify Metal-vs-CPU
  bit-identity or document divergence.
- [ ] **T-D-3 (M2)** — `train_tcn.rs` training-loop binary. 1-epoch
  1-symbol smoke. Checkpoint + metadata.json write. SHA reproducibility.
- [ ] **T-D-4 (M3)** — Full BS-1 training run. Curve inspection.
  Hyperparameter tune-up if needed.
- [ ] **T-D-5 (M4)** — Inference path: load checkpoint, emit
  `ForecastOverlay`. Strict-replay cache wired. Audit + cost emission
  per architecture/12 R10–R12.
- [ ] **T-D-6 (M5)** — `crates/strategy/src/tcn_overlay_momentum.rs` —
  consuming strategy. Backtest harness integration. BS-1 dry-run.
- [ ] **T-D-7 (M6)** — Full BS-1 + BS-2 backtests. Reports authored
  under `spec/v25-tcn-overlay/reports/`.

## Pending — tester

- [ ] **T-T-1 (M7)** — Determinism + replay verification per
  feature.md § Verification. Anchor locks
  `top10-2023-fy-tcn-overlay` + `top10-2024-fy-tcn-overlay`. 11
  existing anchors stay byte-identical.

## Notes

- Phase invariants (data, scenarios, overlay shape, audit, cost,
  hardware) are inherited from
  [`../v25-dl-forecast-overlay/feature.md`](../v25-dl-forecast-overlay/feature.md)
  — DO NOT re-derive here.
- Shared infrastructure (`crates/forecast/`, `crates/replay-cache/`,
  `crates/core/src/forecast.rs`) is already in place per Wave A 2026-05-16
  and is model-agnostic — this phase only adds `TcnForecaster` impl +
  training-loop binary + consuming strategy.
