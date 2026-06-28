---
slug: v25-tcn-threshold-tuning
scenario: threshold-sweep-bs1-realdata-recalibrated
generated: 2026-05-21T10:05:14Z
wall_clock_s: 428.8
host: M022517718D
git_commit: 447c0432b47bcc088ee0c5f4d7fe1fe14d9deb95
model_revision: d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2
sigma_train_recalibrated: 0.018015675
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
verdict: T-MARGINAL
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
| v1 Sharpe (ann.)          | 0.003098 |
| v1 Sortino (ann.)         | 0.004380 |
| v1 Calmar                 | 0.017263 |
| v1 max drawdown           | 73.73% |
| v1 total return           | 13.48% |
| default-cell (τ=0.6, ε=0.0005) Sharpe | 0.007701 |
| default-cell total return | 27.96% |

Pre-feature defaults: τ=0.600000, ε=0.000500. Per-cell deltas signed against v1 momentum Sharpe.

## Heatmap A — Sharpe (ann.) delta vs v1 momentum

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | +0.018254 | +0.018254 | +0.018254 | +0.010881 | +0.004545 |
| 0.200000    | +0.013099 | +0.013099 | +0.013099 | +0.010881 | +0.004545 |
| 0.300000    | +0.010405 | +0.010405 | +0.010405 | +0.010405 | +0.004545 |
| 0.400000    | +0.010696 | +0.010696 | +0.010696 | +0.010696 | +0.004545 |
| 0.500000    | +0.008314 | +0.008314 | +0.008314 | +0.008314 | +0.004545 |
| 0.600000    | +0.004603 | +0.004603 | +0.004603 | +0.004603 | +0.004603 |
| 0.700000    | +0.003293 | +0.003293 | +0.003293 | +0.003293 | +0.003293 |
| 0.800000    | +0.002884 | +0.002884 | +0.002884 | +0.002884 | +0.002884 |
| 0.900000    | +0.006344 | +0.006344 | +0.006344 | +0.006344 | +0.006344 |

## Heatmap B — Total return delta vs v1 momentum (percentage points)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | +50.71% | +50.71% | +50.71% | +31.39% | +13.41% |
| 0.200000    | +37.97% | +37.97% | +37.97% | +31.39% | +13.41% |
| 0.300000    | +30.01% | +30.01% | +30.01% | +30.01% | +13.41% |
| 0.400000    | +34.67% | +34.67% | +34.67% | +34.67% | +13.41% |
| 0.500000    | +27.64% | +27.64% | +27.64% | +27.64% | +13.41% |
| 0.600000    | +14.48% | +14.48% | +14.48% | +14.48% | +14.48% |
| 0.700000    | +10.10% | +10.10% | +10.10% | +10.10% | +10.10% |
| 0.800000    | +9.55% | +9.55% | +9.55% | +9.55% | +9.55% |
| 0.900000    | +26.84% | +26.84% | +26.84% | +26.84% | +26.84% |

## Heatmap C — Max drawdown (absolute value per cell)

| τ \ ε       | 0.000100 | 0.000500 | 0.001000 | 0.005000 | 0.010000 |
|-------------|----------|----------|----------|----------|-----------|
| 0.100000    | 73.20% | 73.20% | 73.20% | 76.58% | 78.35% |
| 0.200000    | 79.59% | 79.59% | 79.59% | 76.58% | 78.35% |
| 0.300000    | 76.63% | 76.63% | 76.63% | 76.63% | 78.35% |
| 0.400000    | 78.50% | 78.50% | 78.50% | 78.50% | 78.35% |
| 0.500000    | 78.34% | 78.34% | 78.34% | 78.34% | 78.35% |
| 0.600000    | 77.83% | 77.83% | 77.83% | 77.83% | 77.83% |
| 0.700000    | 77.68% | 77.68% | 77.68% | 77.68% | 77.68% |
| 0.800000    | 77.05% | 77.05% | 77.05% | 77.05% | 77.05% |
| 0.900000    | 76.30% | 76.30% | 76.30% | 76.30% | 76.30% |

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

(Read from predecessor `spec/v25-tcn-recalibrate/reports/forecast-distribution-bs1-realdata-recalibrated-20260521.md` body — NOT re-computed.)

## Headline cell

| Field              | Value                |
|--------------------|----------------------|
| arg-max(τ, ε)      | (0.100000, 0.001000) |
| Sharpe delta       | +0.018254 |
| Total return delta | +50.71% |
| Max drawdown       | 73.20% |
| Sharpe (cell)      | 0.021352 |
| Sortino (cell)     | 0.030293 |
| Calmar (cell)      | 0.069457 |
| Total return (cell)| 64.19% |
| Trades (cell)      | 2347 |
| Dampen rate (cell) | 44.32% |

## Smoothness statistic

| Field                        | Value       |
|------------------------------|-------------|
| Sharpe-delta range           | 0.015370 |
| max(|cell − 8-neighbour|)    | 0.007373 |
| Smoothness ratio             | 0.479683 |
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
`forecast-distribution-bs1-realdata-recalibrated-20260521.md` body.)

## Notes

- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.safetensors`.
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.json`.
- Read-only against `crates/forecast/checkpoints/anchors/tcn-bs1-d1c3696d79933c8d97695e5fff671f645f810e7961becb2333475fb9cc44fcd2.metadata.recalibrated.json`.
- σ_train value sourced from `--metadata-path` overlay (ADR-0035 D3).
- Backtest seed fixed at `0xC0FFEE` per ADR-0032 § D4.
- Cell ordering: lexicographic by (τ, ε) — NOT completion order (R9 / K3 invariant).
