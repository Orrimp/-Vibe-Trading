---
slug: v25-tcn-threshold-tuning
scenario: threshold-sweep-bs1-realdata-recalibrated
generated: 2026-05-21T18:47:05Z
wall_clock_s: 224.0
host: M022517718D
git_commit: a1ec9457b37ec57678d7229b30c657fde6d2f012
model_revision: d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2
sigma_train_recalibrated: 0.018015675
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
verdict: T-ALPHA-UNLOCKED
---
# Threshold sweep — BS1 (realdata, recalibrated σ_train)

## Inputs

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Anchor scenario   | bs1                                            |
| model_revision    | d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2 |
| weights_sha256    | 4ed9064a3871d8bc911ad8b288dccfc597caa6a09cca3b2395a9e1717b8c7025 |
| σ_train (recal)   | 0.018015675                                    |
| Eval span         | 2023-01-01T00:00:00Z .. 2024-01-01T00:00:00Z   |
| Data revision SHA | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7 |
| Cells             | 45 (9 τ × 5 ε)                                 |
| Bar count / cell  | 8760                                         |

## Baseline references

| Field                     | Value           |
|---------------------------|------------------|
| v1 Sharpe (ann.)          | -0.693194 |
| v1 Sortino (ann.)         | -1.016926 |
| v1 Calmar                 | -0.075770 |
| v1 max drawdown           | 41.80% |
| v1 total return           | -27.52% |
| default-cell (τ=0.6, ε=0.0005) Sharpe | -0.011788 |
| default-cell total return | -0.57% |

Pre-feature defaults: τ=0.600000, ε=0.000500. Per-cell deltas signed against v1 momentum Sharpe.

## Heatmap A — Sharpe (ann.) delta vs v1 momentum

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | +1.155769 | +1.155769 | +1.155769 | +1.009491 | +0.713758 |
| 0.200000    | +1.044382 | +1.044382 | +1.044382 | +1.009491 | +0.713758 |
| 0.300000    | +0.927268 | +0.927268 | +0.927268 | +0.927268 | +0.713758 |
| 0.400000    | +0.899315 | +0.899315 | +0.899315 | +0.899315 | +0.713758 |
| 0.500000    | +0.790263 | +0.790263 | +0.790263 | +0.790263 | +0.713758 |
| 0.600000    | +0.681406 | +0.681406 | +0.681406 | +0.681406 | +0.681406 |
| 0.700000    | +0.541770 | +0.541770 | +0.541770 | +0.541770 | +0.541770 |
| 0.800000    | +0.419068 | +0.419068 | +0.419068 | +0.419068 | +0.419068 |
| 0.900000    | +0.435369 | +0.435369 | +0.435369 | +0.435369 | +0.435369 |

## Heatmap B — Total return delta vs v1 momentum (percentage points)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | +52.90% | +52.90% | +52.90% | +44.30% | +28.53% |
| 0.200000    | +46.28% | +46.28% | +46.28% | +44.30% | +28.53% |
| 0.300000    | +39.71% | +39.71% | +39.71% | +39.71% | +28.53% |
| 0.400000    | +38.34% | +38.34% | +38.34% | +38.34% | +28.53% |
| 0.500000    | +32.41% | +32.41% | +32.41% | +32.41% | +28.53% |
| 0.600000    | +26.94% | +26.94% | +26.94% | +26.94% | +26.94% |
| 0.700000    | +20.47% | +20.47% | +20.47% | +20.47% | +20.47% |
| 0.800000    | +15.14% | +15.14% | +15.14% | +15.14% | +15.14% |
| 0.900000    | +15.94% | +15.94% | +15.94% | +15.94% | +15.94% |

## Heatmap C — Max drawdown (absolute value per cell)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | 22.34% | 22.34% | 22.34% | 21.10% | 27.62% |
| 0.200000    | 23.20% | 23.20% | 23.20% | 21.10% | 27.62% |
| 0.300000    | 21.51% | 21.51% | 21.51% | 21.51% | 27.62% |
| 0.400000    | 24.86% | 24.86% | 24.86% | 24.86% | 27.62% |
| 0.500000    | 26.05% | 26.05% | 26.05% | 26.05% | 27.62% |
| 0.600000    | 29.33% | 29.33% | 29.33% | 29.33% | 29.33% |
| 0.700000    | 32.39% | 32.39% | 32.39% | 32.39% | 32.39% |
| 0.800000    | 33.97% | 33.97% | 33.97% | 33.97% | 33.97% |
| 0.900000    | 32.95% | 32.95% | 32.95% | 32.95% | 32.95% |

## Heatmap D — Gate-survivor count (collapsed to 1-D row over τ; ε-invariant)

| τ           | Gate survivors |
|-------------|----------------|
| 0.100000    | 69085 |
| 0.200000    | 60339 |
| 0.300000    | 51964 |
| 0.400000    | 44375 |
| 0.500000    | 37386 |
| 0.600000    | 31177 |
| 0.700000    | 25973 |
| 0.800000    | 21684 |
| 0.900000    | 18087 |

(Read from predecessor `evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md` body — NOT re-computed.)

## Headline cell

| Field              | Value                |
|--------------------|----------------------|
| arg-max(τ, ε)      | (0.100000, 0.001000) |
| Sharpe delta       | +1.155769 |
| Total return delta | +52.90% |
| Max drawdown       | 22.34% |
| Sharpe (cell)      | 0.462575 |
| Sortino (cell)     | 0.674784 |
| Calmar (cell)      | 0.102409 |
| Total return (cell)| 25.38% |
| Trades (cell)      | 2345 |
| Dampen rate (cell) | 44.32% |

## Smoothness statistic

| Field                        | Value       |
|------------------------------|-------------|
| Sharpe-delta range           | 0.736701 |
| max(|cell − 8-neighbour|)    | 0.295734 |
| Smoothness ratio             | 0.401430 |
| H2 verdict                   | falsified |

Per feature.md § H2 — smoothness ratio ≤ 0.25 ⇒ H2 confirmed; > 0.25 ⇒ H2 falsified.

## Verdict

T-classifier per feature.md § R3:

- `T-ALPHA-UNLOCKED` ⇔ max-cell Sharpe delta ≥ +0.10
- `T-MARGINAL`       ⇔ max-cell Sharpe delta ∈ [0.0, +0.10)
- `T-NO-ALPHA`       ⇔ max-cell Sharpe delta < 0

This checkpoint: **T-ALPHA-UNLOCKED**.

**How to read this verdict.** The T-classifier is a point estimate — `max-cell Sharpe − v1 Sharpe`, with the maximum taken over every cell of the grid. It carries no bootstrap, no deflated Sharpe ratio and no multiple-testing correction, and it is NOT this project's robustness gate: nothing in this report calls `classify_verdict`, `verdict_bands`, `compute_robustness_flag` or `rank_candidates` (AD-1). A T-verdict is therefore not evidence of an edge.

Composition of this checkpoint's delta: best cell +0.462575 − v1 baseline -0.693194 = +1.155769, selected as the maximum of 45 cells.
The v1 baseline is NEGATIVE (-0.693194 Sharpe, -27.52% total return), so this delta measures a cell beating a baseline that loses money. Read it as that and not as a return.

(Advisory verdict — does NOT amend ADR-0033 § D3 F-verdict algorithm per Q4=(c).
The F-verdict for this checkpoint remains F4 per the predecessor's anchored
`forecast-distribution-bs1-realdata-recalibrated-20260521.md` body.)

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors`.
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.json`.
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json`.
- σ_train value sourced from `--metadata-path` overlay (ADR-0035 D3).
- Backtest seed fixed at `0xC0FFEE` per ADR-0032 § D4.
- Cell ordering: lexicographic by (τ, ε) — NOT completion order (R9 / K3 invariant).
