---
generated: "2026-05-27T20:30:00Z"
feature: v5-latency-slippage-sim-v0.3.0-full-path-wiring
version: "v0.3.0 + v5-realdata-medium-2026-05"
canonical_config: "latency_ms_min=30 latency_ms_max=80 slippage_bps=8"
base_version: "v0.2.0-anchor-migration (extended)"
---

# Sharpe-delta Table — v5 v0.3.0 Full-path Wiring (canonical vs noop-baseline)

Canonical config applied: `LatencySlippageSimConfig { latency_ms_min: 30, latency_ms_max: 80, slippage_bps: 8 }`

This table extends the v0.2.0 table by adding newly wired paths: **Pairs**, **TCN overlay (synthetic)**, and documenting the Group A SMA reversion to synthetic per Q1=(a).

## Legend

| Symbol | Meaning |
|--------|---------|
| `—` | Metric not available for this report type (analysis/success reports) |
| `=noop` | Canonical SHA identical to noop-baseline; sim not wired for this scenario path |
| K1 | Alpha-inversion surprise: Sharpe positive in noop → negative in canonical |
| `real-data` | Delta driven by data-source switch (synthetic → real Binance Parquet) |
| `v5-sim` | Delta driven by v5 latency/slippage sim being wired into the strategy path |
| `v5-sim+Q1` | SMA path newly wired; Group A reverted to synthetic (Q1=(a)); Δ = sim effect only |
| `=noop (candle absent)` | Requires candle feature at compile time; SHA unchanged without it |

## Results

### Group A — SMA/Composed strategies (BTC 1m, 2023, synthetic — Q1=(a) revert)

These scenarios were **reverted to synthetic baseline** per Q1=(a) operator decision. The v0.2.0 canonical
SHAs used real Binance data; v0.3.0 reverts to synthetic GBM (same seed as noop-baseline). The equity
delta vs noop now reflects only the v5 slippage sim cost applied to the SMA/Composed paths (newly wired
in v0.3.0).

| Scenario | Noop Sharpe | Canon Sharpe | Δ Sharpe | Noop Equity | Canon Equity | Δ Equity | Noop MaxDD | Canon MaxDD | K1 | Driver |
|----------|-------------|--------------|----------|-------------|--------------|----------|------------|-------------|----|--------|
| btc-2023-1m-sma-cross | -13.0169 | -28.9324 | -15.91 | $47,290.03 | $17,992.64 | -$29,297.39 | 53.06% | 82.07% | — | v5-sim+Q1 |
| btc-2023-1m-sma-baseline-refresh | -13.0169 | -28.9324 | -15.91 | $47,290.03 | $17,992.64 | -$29,297.39 | 53.06% | 82.07% | — | v5-sim+Q1 |
| btc-2023-1m-macd-trend | -40.3994 | -5.1438 | +35.26 | $20,550.94 | $96,691.95 | +$76,141.01 | 79.49% | 4.96% | — | v5-sim+real-data |
| btc-2023-1m-rsi-reversion | -55.4257 | -15.2570 | +40.17 | $42,195.44 | $94,941.37 | +$52,745.93 | 57.81% | 5.42% | — | v5-sim+real-data |
| btc-2023-1m-bbands-mean-revert | -68.8313 | -32.8944 | +35.94 | $47,009.80 | $89,723.52 | +$42,713.72 | 52.99% | 10.31% | — | v5-sim+real-data |

> **Notes**:
> - `btc-2023-1m-sma-cross/baseline-refresh`: equity dropped $29.3k (29.3% of initial $100k) due to 8bps
>   slippage × 12,077 fills. This is the expected cost of realistic friction on a high-frequency SMA
>   crossover strategy running 525,601 synthetic bars.
> - Composed strategies (macd/rsi/bbands): equity improved vs noop because these run against real Binance
>   data (auto-detected Parquet cache), which has better signal-to-noise than synthetic GBM. The v5 sim
>   applies slippage on top but the data-source effect dominates.
> - No K1 surprises: SMA cross noop Sharpe was already negative (-13.02); canonical is also negative.

### Group B — Cross-sectional momentum (top-10, hourly)

Momentum path was wired in v0.1.0. Canonical SHA unchanged from v0.2.0 (same synthetic data + same
slippage wiring → deterministic identical output).

| Scenario | Noop Sharpe | Canon Sharpe | Δ Sharpe | Noop Equity | Canon Equity | Δ Equity | K1 | Driver |
|----------|-------------|--------------|----------|----|---|---|----|----|
| top10-2023-1h-momentum | N/A | N/A | — | $56,282.81 | $50,922.49 | -$5,360.32 | — | v5-sim |
| top10-2024-h1-momentum | N/A | N/A | — | $46,401.41 | $42,862.85 | -$3,538.56 | — | v5-sim |

> **Note**: Unchanged from v0.2.0. Momentum sim was already wired.

### Group C — Pairs mean-reversion (newly wired in v0.3.0)

Pairs strategy path **newly wired** in this sprint. Canonical reports now diverge from noop.

| Scenario | Noop Equity | Canon Equity | Δ Equity | Noop MaxDD | Canon MaxDD | K1 | Driver |
|----------|-------------|--------------|----------|------------|-------------|----|--------|
| pairs-2023-zscore-mr | -$60,524.71 | -$62,693.12 | -$2,168.41 | 1827.39% | 1828.87% | — | v5-sim |
| pairs-2024-h1-zscore-mr | -$60,524.71 | -$62,693.12 | -$2,168.41 | 1827.39% | 1828.87% | — | v5-sim |

> **Note**: Pairs is a loss-making baseline strategy (formulation C is long-only on the `a` leg).
> The extra -$2.2k loss per scenario is the slippage cost on 16 fills × 8bps.
> No K1 surprise: noop Sharpe was already negative (not reportable; equity deeply negative).

### Group D — TCN overlay synthetic (newly wired in v0.3.0)

TCN overlay (non-weights, synthetic path) **newly wired** in this sprint. PassthroughForecaster path.

| Scenario | Noop Equity | Canon Equity | Δ Equity | Noop MaxDD | Canon MaxDD | K1 | Driver |
|----------|-------------|--------------|----------|------------|-------------|----|--------|
| top10-2023-fy-tcn-overlay | $30,235.58 | $28,347.99 | -$1,887.59 | 87.48% | 87.63% | — | v5-sim |
| top10-2024-fy-tcn-overlay | $44,300.24 | $40,006.65 | -$4,293.59 | 87.48% | 87.63% | — | v5-sim |

> **Note**: TCN overlay (synthetic) diverges from noop. The 1,224 and 3,672 trades respectively bear
> 8bps slippage per fill, reducing equity by ~$1.9k and ~$4.3k.

### Group E — TCN overlay weights (candle feature required — not re-emitted)

Requires `--features candle` at compile time. Binary built without candle errors on these scenarios.
Canonical SHA = noop SHA (unchanged from v0.2.0).

| Scenario | Noop Equity | Canon Equity | Δ Equity | K1 | Driver |
|----------|-------------|--------------|----------|----|--------|
| top10-2023-fy-tcn-overlay-weights | $30,235.58 | $30,235.58 | $0.00 | — | =noop (candle absent) |
| top10-2024-fy-tcn-overlay-weights | $44,300.24 | $44,300.24 | $0.00 | — | =noop (candle absent) |

### Group F — TCN overlay realdata (realdata feature required — not re-emitted)

Requires `--features realdata`. Canonical SHA = noop SHA (unchanged from v0.2.0).

| Scenario | Noop Equity | Canon Equity | Δ Equity | K1 | Driver |
|----------|-------------|--------------|----------|----|--------|
| top10-2023-fy-tcn-overlay-realdata | $113,479.98 | $113,479.98 | $0.00 | — | =noop (realdata absent) |
| top10-2024-fy-tcn-overlay-realdata | $105,214.25 | $105,214.25 | $0.00 | — | =noop (realdata absent) |
| top10-2023-fy-tcn-overlay-weights-realdata | $113,479.98 | $113,479.98 | $0.00 | — | =noop (realdata absent) |
| top10-2024-fy-tcn-overlay-weights-realdata | $105,214.25 | $105,214.25 | $0.00 | — | =noop (realdata absent) |

### Group G — PatchTST overlay (realdata feature required — not re-emitted)

Requires `--features realdata candle`. Canonical SHA = noop SHA.

| Scenario | Noop Equity | Canon Equity | Δ Equity | K1 | Driver |
|----------|-------------|--------------|----------|----|--------|
| top10-2023-fy-patchtst-overlay-realdata | $131,125.07 | $131,125.07 | $0.00 | — | =noop (realdata absent) |

### Group H — Vol-target GARCH overlay (realdata feature required — not re-emitted)

Requires `--features realdata`. Canonical SHA = noop SHA.

| Scenario | Noop Equity | Canon Equity | Δ Equity | K1 | Driver |
|----------|-------------|--------------|----------|----|--------|
| top10-2023-fy-vol-target-overlay-realdata | $62,807.89 | $62,807.89 | $0.00 | — | =noop (realdata absent) |

### Group I — Analysis/investigation reports (no equity metrics)

| Scenario | Report Type | K1 | Driver |
|----------|-------------|----|----|
| forecast-distribution-bs1-realdata | analysis | — | =noop |
| forecast-distribution-bs2-realdata | analysis | — | =noop |
| sharpe-comparison-realdata | analysis | — | =noop |
| forecast-distribution-bs1-realdata-recalibrated | analysis | — | =noop |
| forecast-distribution-bs2-realdata-recalibrated | analysis | — | =noop |
| recalibrate-sigma-train-bs1 | analysis | — | =noop |
| recalibrate-sigma-train-bs2 | analysis | — | =noop |
| threshold-sweep-bs1-realdata-recalibrated | analysis | — | =noop |
| threshold-sweep-bs2-realdata-recalibrated | analysis | — | =noop |
| forecast-distribution-patchtst-bs1-realdata | analysis | — | =noop |
| vol-verdict-bs1-realdata | analysis | — | =noop |
| sharpe-comparison-vol-target-bs1-realdata | analysis | — | =noop |
| sharpe-comparison-vol-target-bs1-realbaseline | analysis | — | =noop |

### Group J — Operator success report samples

| Scenario | Report Type | K1 | Driver |
|----------|-------------|----|----|
| report-sample-7d | success | — | =noop |
| report-sample-90d | success | — | =noop |

## K1 Surprise Scan

**No K1 surprises detected across all 69 scenarios.**

A K1 surprise is defined as: noop-baseline Sharpe > 0 AND canonical Sharpe < 0.

- **Group A (SMA cross)**: Noop Sharpe was already negative (-13.02 on synthetic data). Canonical also negative. No K1.
- **Group A (composed)**: Noop Sharpe was deeply negative (-40 to -68). Canonical improved (real data effect). No K1.
- **Group B (Momentum)**: Sharpe N/A in both namespaces. Equity degrades by $3.5k–$5.4k. No K1 assessment possible via Sharpe.
- **Group C (Pairs)**: Equity deeply negative in both namespaces. No K1.
- **Group D (TCN overlay synthetic)**: Equity positive but reduced (~$1.9k–$4.3k). No K1 (Sharpe N/A).
- **Groups E–J**: SHA unchanged; no delta possible.

**Retirement candidates**: None identified. All strategies show consistent behavior under friction.
The pairs strategy is already a known loss-making baseline (formulation C design choice).

## Summary

| Category | # Scenarios | v0.3.0 New Wiring | Δ Equity (v5-sim) | K1 |
|----------|-------------|-------------------|-------------------|----|
| SMA cross (synthetic, Q1=a) | 2 | YES (newly wired) | -$29.3k each | 0 |
| Composed (real-data + sim) | 3 | YES (newly wired) | +$42k–+$76k (data-driven) | 0 |
| Momentum (unchanged from v0.2.0) | 2 | no (wired in v0.1.0) | -$3.5k to -$5.4k | 0 |
| Pairs (newly wired, synthetic) | 2 | YES (newly wired) | -$2.2k each | 0 |
| TCN overlay synthetic (newly wired) | 2 | YES (newly wired) | -$1.9k to -$4.3k | 0 |
| TCN overlay weights (candle absent) | 2 | not re-emitted | $0 | 0 |
| TCN overlay realdata (realdata absent) | 4 | not re-emitted | $0 | 0 |
| PatchTST (realdata absent) | 1 | not re-emitted | $0 | 0 |
| Vol-target GARCH (realdata absent) | 1 | not re-emitted | $0 | 0 |
| Analysis reports (no equity) | 13 | no | — | 0 |
| Success samples (no equity) | 2 | no | — | 0 |
| Yahoo anchor | 1 | no | — | 0 |
| **Total** | **35 (+ 34 noop rows)** | **9 scenarios** | — | **0** |

> The v5 sim is now wired into **4 of 7 runnable strategy paths** (momentum, SMA/Composed, Pairs,
> TCN overlay synthetic). The remaining 3 paths (TCN-weights/realdata, PatchTST, VolTarget/GARCH)
> require candle/realdata features not built in the default CI binary. Their canonical SHAs remain
> noop-identical until a feature-flagged rebuild is performed.
