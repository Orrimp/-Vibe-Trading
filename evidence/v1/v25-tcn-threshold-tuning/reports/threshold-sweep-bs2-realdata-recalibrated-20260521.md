---
slug: v25-tcn-threshold-tuning
scenario: threshold-sweep-bs2-realdata-recalibrated
generated: 2026-05-21T10:09:09Z
wall_clock_s: 224.6
host: M022517718D
git_commit: 447c0432b47bcc088ee0c5f4d7fe1fe14d9deb95
model_revision: 3fabcabecbee94d6acfbd6e8315627d43479359ce4d47287fb04b5dc42e5c21d
sigma_train_recalibrated: 0.011913909
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
verdict: T-MARGINAL
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
| v1 Sharpe (ann.)          | 0.001389 |
| v1 Sortino (ann.)         | 0.001965 |
| v1 Calmar                 | 0.006447 |
| v1 max drawdown           | 78.82% |
| v1 total return           | 5.21% |
| default-cell (τ=0.6, ε=0.0005) Sharpe | -0.003844 |
| default-cell total return | -6.74% |

Pre-feature defaults: τ=0.600000, ε=0.000500. Per-cell deltas signed against v1 momentum Sharpe.

## Heatmap A — Sharpe (ann.) delta vs v1 momentum

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | +0.044944 | +0.044944 | +0.044944 | -0.013192 | +0.010077 |
| 0.200000    | +0.031823 | +0.031823 | +0.031823 | -0.013192 | +0.010077 |
| 0.300000    | +0.031693 | +0.031693 | +0.031693 | -0.013192 | +0.010077 |
| 0.400000    | -0.011683 | -0.011683 | -0.011683 | -0.013192 | +0.010077 |
| 0.500000    | +0.009077 | +0.009077 | +0.009077 | +0.009077 | +0.010077 |
| 0.600000    | -0.005233 | -0.005233 | -0.005233 | -0.005233 | +0.010077 |
| 0.700000    | +0.007298 | +0.007298 | +0.007298 | +0.007298 | +0.010077 |
| 0.800000    | +0.008068 | +0.008068 | +0.008068 | +0.008068 | +0.010077 |
| 0.900000    | +0.010165 | +0.010165 | +0.010165 | +0.010165 | +0.010165 |

## Heatmap B — Total return delta vs v1 momentum (percentage points)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | +161.53% | +161.53% | +161.53% | -27.53% | +40.10% |
| 0.200000    | +87.07% | +87.07% | +87.07% | -27.53% | +40.10% |
| 0.300000    | +92.14% | +92.14% | +92.14% | -27.53% | +40.10% |
| 0.400000    | -24.83% | -24.83% | -24.83% | -27.53% | +40.10% |
| 0.500000    | +29.80% | +29.80% | +29.80% | +29.80% | +40.10% |
| 0.600000    | -11.95% | -11.95% | -11.95% | -11.95% | +40.10% |
| 0.700000    | +25.58% | +25.58% | +25.58% | +25.58% | +40.10% |
| 0.800000    | +30.30% | +30.30% | +30.30% | +30.30% | +40.10% |
| 0.900000    | +42.53% | +42.53% | +42.53% | +42.53% | +42.53% |

## Heatmap C — Max drawdown (absolute value per cell)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | 87.44% | 87.44% | 87.44% | 90.46% | 79.68% |
| 0.200000    | 88.36% | 88.36% | 88.36% | 90.46% | 79.68% |
| 0.300000    | 91.22% | 91.22% | 91.22% | 90.46% | 79.68% |
| 0.400000    | 89.13% | 89.13% | 89.13% | 90.46% | 79.68% |
| 0.500000    | 79.58% | 79.58% | 79.58% | 79.58% | 79.68% |
| 0.600000    | 94.63% | 94.63% | 94.63% | 94.63% | 79.68% |
| 0.700000    | 80.64% | 80.64% | 80.64% | 80.64% | 79.68% |
| 0.800000    | 79.87% | 79.87% | 79.87% | 79.87% | 79.68% |
| 0.900000    | 79.80% | 79.80% | 79.80% | 79.80% | 79.80% |

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

(Read from predecessor `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs2-realdata-recalibrated-20260521.md` body — NOT re-computed.)

## Headline cell

| Field              | Value                |
|--------------------|----------------------|
| arg-max(τ, ε)      | (0.100000, 0.001000) |
| Sharpe delta       | +0.044944 |
| Total return delta | +161.53% |
| Max drawdown       | 87.44% |
| Sharpe (cell)      | 0.046333 |
| Sortino (cell)     | 0.066368 |
| Calmar (cell)      | 0.117557 |
| Total return (cell)| 166.75% |
| Trades (cell)      | 2152 |
| Dampen rate (cell) | 43.53% |

## Smoothness statistic

| Field                        | Value       |
|------------------------------|-------------|
| Sharpe-delta range           | 0.058136 |
| max(|cell − 8-neighbour|)    | 0.058136 |
| Smoothness ratio             | 1.000000 |
| H2 verdict                   | falsified |

Per feature.md § H2 — smoothness ratio ≤ 0.25 ⇒ H2 confirmed; > 0.25 ⇒ H2 falsified.

## Verdict

T-classifier per feature.md § R3:

- `T-ALPHA-UNLOCKED` ⇔ max-cell Sharpe delta ≥ +0.10
- `T-MARGINAL`       ⇔ max-cell Sharpe delta ∈ [0.0, +0.10)
- `T-NO-ALPHA`       ⇔ max-cell Sharpe delta < 0

This checkpoint: **T-MARGINAL**.

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
