---
slug: v3-regime-classifier
doc-type: h1-hypothesis-discharge
date: 2026-05-29
author: developer (Wave E)
status: FINAL
---

# H1 Hypothesis Discharge — Regime-Dispatcher Sharpe Delta (v3.0.0-regime)

## Hypothesis

**H1**: The v3.0.0-regime `RegimeDispatcher` (Markov-switching 4-state classifier routing
Bull/Bear → MomentumStrategy, Volatile/Calm → CashHoldStrategy) achieves a net Sharpe delta
of **≥ +0.10** versus the v1 cross-sectional momentum baseline on the same real Binance 2023
hourly data.

**Source**: `spec/v3-regime-classifier/feature.md § H1` + ADR-0049 § D4 T-REG gate.

## Evidence

### Scenario pair (2023 train window, real Binance data)

| Metric          | v1 Baseline (top10-2023-fy-momentum-realdata) | RegimeDispatcher (top10-2023-fy-regime-dispatcher-realdata) |
|-----------------|-----------------------------------------------|-------------------------------------------------------------|
| Bars            | 87590                                         | 87590                                                       |
| Initial capital | $100,000.00 USDT                              | $100,000.00 USDT                                            |
| Final equity    | $113,479.98 USDT                              | $87,431.52 USDT                                             |
| Total return    | +13.48%                                       | -12.57%                                                     |
| Max drawdown    | 73.73%                                        | 40.49%                                                      |
| Trades          | 6203                                          | 6847                                                        |
| Suppress rate   | 0.00% (no dispatcher)                         | 11.20% (Volatile/Calm suppressed)                           |
| Sharpe (ann)    | 0.003098                                      | -0.291015                                                   |
| Sortino (ann)   | 0.004380                                      | -0.435466                                                   |
| Calmar          | 0.017263                                      | -0.032953                                                   |

### T-REG classifier

| Field              | Value                    |
|--------------------|--------------------------|
| Net Sharpe delta   | -0.294113                |
| T-REG verdict      | **T-REG-NO-ALPHA**       |
| ADR-0049 threshold | net_delta < +0.05        |

### V-REG verdict (2024 held-out val window)

| Field         | Value          |
|---------------|----------------|
| V-REG verdict | **V-REG-5**    |
| Suppress rate | 13.53%         |
| Switch rate   | 15.07/week (UB)|

## Discharge Decision

**H1: REJECTED** at v0.1.0.

Net Sharpe delta = **-0.294113** (dispatcher significantly underperforms the raw momentum
baseline on 2023). T-REG = T-REG-NO-ALPHA.

### Root cause analysis

The 2023 crypto market was a recovery year (BTC +155%, ETH +91%). The RegimeDispatcher
suppressed momentum signals during 11.20% of active bars (classified as Volatile/Calm).
In a strong bull-recovery context, suppression hurt performance: the dispatcher held cash
while momentum was active, missing upside. The degenerate `CashHoldStrategy` at v0.1.0
is conservative by design (ADR-0049 § D3 option (i)) — it holds existing positions but
emits no new signals, causing underperformance in trending conditions.

### Positive findings at v0.1.0

1. **Max drawdown reduced**: dispatcher 40.49% vs baseline 73.73% — the suppression did
   reduce peak drawdown by 33 percentage points.
2. **V-REG-5 on 2024 val window**: the classifier is healthy (non-trivial, low flicker,
   converged). The classifier WORKS — it just doesn't provide alpha on top of v1 momentum
   in the training year.
3. **Determinism confirmed**: both 2-run determinism gates PASS with byte-identical body SHAs.

### Joint advisory per ADR-0049 § D4

| V-REG | T-REG           | Advisory                                           |
|-------|-----------------|-----------------------------------------------------|
| V-REG-5 | T-REG-NO-ALPHA | C2 retire + close v3 three-pick set (HOLD-FOR-OPERATOR) |

The operator must decide per ADR-0049 § D4: T-REG-NO-ALPHA with V-REG-5 → C2 retire
(retire the RegimeDispatcher approach at v0.1.0) + close the v3 three-pick set, OR
investigate the v0.2.0 path (MeanReversionStrategy for Volatile/Calm regimes which may
recover performance).

### Follow-on options

1. **v0.2.0: MeanReversionStrategy for Volatile/Calm** — the `CashHoldStrategy` placeholder
   was always temporary (ADR-0049 § D3). A proper mean-reversion strategy for volatile regimes
   may flip T-REG positive. Prerequisite: `v1.5-mean-reversion-for-regime-dispatcher`.
2. **Classifier tuning**: adjust confidence gate (currently 0.70), refit interval (100 bars),
   or prior values to reduce false-positive suppression during bull trends.
3. **Drawdown metric**: if the operator values drawdown reduction over return, the dispatcher
   may be useful as a risk-management overlay despite negative Sharpe delta.

## Anchors (Wave E — 4 new anchors under v3.0.0-regime namespace)

| Scenario                                        | SHA-256 body hash                                                |
|-------------------------------------------------|------------------------------------------------------------------|
| top10-2023-fy-regime-dispatcher-realdata        | f37bbb8d3520c7bae2ff1d48fa71d704a8b122d84a3d843d443bafa359664775 |
| top10-2024-fy-regime-dispatcher-realdata        | 691a70568f4d0e6e74e51e7318f55236b7c3e0f97968bf6aabfdacd308ba9f4e |
| regime-verdict-bs1-realdata                     | 2d248f4e9df358c24f49d1fce246c72aa7b00f2f28293edcae6fa0323a2eda1d |
| sharpe-comparison-regime-dispatcher-bs1-realdata | a9e001399edbfe0325cbc403626698892ad949179aa2518c8d33304e5531ab97 |

## Cross-references

- `spec/v3-regime-classifier/feature.md` § H1 (hypothesis definition)
- `spec/architecture/adr/0049-v3-regime-classifier-markov-switching-verdict-shape.md` § D4 (T-REG gate)
- `spec/v3-regime-classifier/reports/backtest-20260529-055141-top10-2023-fy-regime-dispatcher-realdata.md`
- `spec/v3-regime-classifier/reports/backtest-20260529-055810-top10-2024-fy-regime-dispatcher-realdata.md`
- `spec/v3-regime-classifier/reports/regime-verdict-bs1-realdata-20260529.md`
- `spec/v3-regime-classifier/reports/sharpe-comparison-regime-dispatcher-bs1-realdata-20260529.md`
- `spec/anchors.toml` § v3.0.0-regime (anchors 72-75; all 75/75 PASS)
