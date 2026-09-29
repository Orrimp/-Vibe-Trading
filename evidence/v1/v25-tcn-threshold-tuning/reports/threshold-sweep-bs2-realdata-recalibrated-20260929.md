---
slug: v25-tcn-threshold-tuning
scenario: threshold-sweep-bs2-realdata-recalibrated
generated: 2026-05-21T18:51:21Z
wall_clock_s: 255.1
host: M022517718D
git_commit: a1ec9457b37ec57678d7229b30c657fde6d2f012
model_revision: 3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d
sigma_train_recalibrated: 0.011913909
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
verdict: T-ALPHA-UNLOCKED
---
# Threshold sweep — BS2 (realdata, recalibrated σ_train)

## Inputs

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Anchor scenario   | bs2                                            |
| model_revision    | 3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d |
| weights_sha256    | 5f22b5bcb4c2fdd0b320827b17f4af39f7a7a3a92605c86042535011415ca474 |
| σ_train (recal)   | 0.011913909                                    |
| Eval span         | 2024-01-01T00:00:00Z .. 2025-01-01T00:00:00Z   |
| Data revision SHA | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7 |
| Cells             | 45 (9 τ × 5 ε)                                 |
| Bar count / cell  | 8784                                         |

## Baseline references

| Field                     | Value           |
|---------------------------|------------------|
| v1 Sharpe (ann.)          | -0.447133 |
| v1 Sortino (ann.)         | -0.652743 |
| v1 Calmar                 | -0.056131 |
| v1 max drawdown           | 44.31% |
| v1 total return           | -22.32% |
| default-cell (τ=0.6, ε=0.0005) Sharpe | 0.048180 |
| default-cell total return | 3.29% |

Pre-feature defaults: τ=0.600000, ε=0.000500. Per-cell deltas signed against v1 momentum Sharpe.

## Heatmap A — Sharpe (ann.) delta vs v1 momentum

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | +1.086583 | +1.086583 | +1.086583 | +0.832600 | +0.352248 |
| 0.200000    | +1.073305 | +1.073305 | +1.073305 | +0.832600 | +0.352248 |
| 0.300000    | +0.911841 | +0.911841 | +0.911841 | +0.832600 | +0.352248 |
| 0.400000    | +0.866926 | +0.866926 | +0.866926 | +0.832600 | +0.352248 |
| 0.500000    | +0.620780 | +0.620780 | +0.620780 | +0.620780 | +0.352248 |
| 0.600000    | +0.495313 | +0.495313 | +0.495313 | +0.495313 | +0.352248 |
| 0.700000    | +0.420767 | +0.420767 | +0.420767 | +0.420767 | +0.352248 |
| 0.800000    | +0.322460 | +0.322460 | +0.322460 | +0.322460 | +0.352248 |
| 0.900000    | +0.343797 | +0.343797 | +0.343797 | +0.343797 | +0.343797 |

## Heatmap B — Total return delta vs v1 momentum (percentage points)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | +80.66% | +80.66% | +80.66% | +52.75% | +16.48% |
| 0.200000    | +77.70% | +77.70% | +77.70% | +52.75% | +16.48% |
| 0.300000    | +60.97% | +60.97% | +60.97% | +52.75% | +16.48% |
| 0.400000    | +55.85% | +55.85% | +55.85% | +52.75% | +16.48% |
| 0.500000    | +34.84% | +34.84% | +34.84% | +34.84% | +16.48% |
| 0.600000    | +25.61% | +25.61% | +25.61% | +25.61% | +16.48% |
| 0.700000    | +20.61% | +20.61% | +20.61% | +20.61% | +16.48% |
| 0.800000    | +14.61% | +14.61% | +14.61% | +14.61% | +16.48% |
| 0.900000    | +15.99% | +15.99% | +15.99% | +15.99% | +15.99% |

## Heatmap C — Max drawdown (absolute value per cell)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | 36.05% | 36.05% | 36.05% | 36.94% | 38.81% |
| 0.200000    | 36.20% | 36.20% | 36.20% | 36.94% | 38.81% |
| 0.300000    | 39.88% | 39.88% | 39.88% | 36.94% | 38.81% |
| 0.400000    | 37.15% | 37.15% | 37.15% | 36.94% | 38.81% |
| 0.500000    | 37.73% | 37.73% | 37.73% | 37.73% | 38.81% |
| 0.600000    | 37.67% | 37.67% | 37.67% | 37.67% | 38.81% |
| 0.700000    | 39.27% | 39.27% | 39.27% | 39.27% | 38.81% |
| 0.800000    | 40.19% | 40.19% | 40.19% | 40.19% | 38.81% |
| 0.900000    | 40.19% | 40.19% | 40.19% | 40.19% | 40.19% |

## Heatmap D — Gate-survivor count (collapsed to 1-D row over τ; ε-invariant)

| τ           | Gate survivors |
|-------------|----------------|
| 0.100000    | 67419 |
| 0.200000    | 57339 |
| 0.300000    | 48054 |
| 0.400000    | 39785 |
| 0.500000    | 32899 |
| 0.600000    | 26972 |
| 0.700000    | 22159 |
| 0.800000    | 18301 |
| 0.900000    | 15056 |

(Read from predecessor `evidence/v1/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md` body — NOT re-computed.)

## Headline cell

| Field              | Value                |
|--------------------|----------------------|
| arg-max(τ, ε)      | (0.100000, 0.001000) |
| Sharpe delta       | +1.086583 |
| Total return delta | +80.66% |
| Max drawdown       | 36.05% |
| Sharpe (cell)      | 0.639450 |
| Sortino (cell)     | 0.922444 |
| Calmar (cell)      | 0.130117 |
| Total return (cell)| 58.34% |
| Trades (cell)      | 2158 |
| Dampen rate (cell) | 43.53% |

## Smoothness statistic

| Field                        | Value       |
|------------------------------|-------------|
| Sharpe-delta range           | 0.764123 |
| max(|cell − 8-neighbour|)    | 0.480352 |
| Smoothness ratio             | 0.628632 |
| H2 verdict                   | falsified |

Per feature.md § H2 — smoothness ratio ≤ 0.25 ⇒ H2 confirmed; > 0.25 ⇒ H2 falsified.

## Verdict

T-classifier per feature.md § R3:

- `T-ALPHA-UNLOCKED` ⇔ max-cell Sharpe delta ≥ +0.10
- `T-MARGINAL`       ⇔ max-cell Sharpe delta ∈ [0.0, +0.10)
- `T-NO-ALPHA`       ⇔ max-cell Sharpe delta < 0

This checkpoint: **T-ALPHA-UNLOCKED**.

**How to read this verdict.** The T-classifier is a point estimate — `max-cell Sharpe − v1 Sharpe`, with the maximum taken over every cell of the grid. It carries no bootstrap, no deflated Sharpe ratio and no multiple-testing correction, and it is NOT this project's robustness gate: nothing in this report calls `classify_verdict`, `verdict_bands`, `compute_robustness_flag` or `rank_candidates` (AD-1). A T-verdict is therefore not evidence of an edge.

Composition of this checkpoint's delta: best cell +0.639450 − v1 baseline -0.447133 = +1.086583, selected as the maximum of 45 cells.
The v1 baseline is NEGATIVE (-0.447133 Sharpe, -22.32% total return), so this delta measures a cell beating a baseline that loses money. Read it as that and not as a return.

(Advisory verdict — does NOT amend ADR-0033 § D3 F-verdict algorithm per Q4=(c).
The F-verdict for this checkpoint remains F4 per the predecessor's anchored
`forecast-distribution-bs2-realdata-recalibrated-20260521.md` body.)

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.safetensors`.
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.json`.
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs2-3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d.metadata.recalibrated.json`.
- σ_train value sourced from `--metadata-path` overlay (ADR-0035 D3).
- Backtest seed fixed at `0xC0FFEE` per ADR-0032 § D4.
- Cell ordering: lexicographic by (τ, ε) — NOT completion order (R9 / K3 invariant).
