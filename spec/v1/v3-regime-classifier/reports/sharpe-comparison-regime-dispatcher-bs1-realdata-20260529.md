---
slug: v3-regime-classifier
scenario: sharpe-comparison-regime-dispatcher-bs1-realdata
generated: 2026-05-29T06:31:04Z
wall_clock_s: 287.1
host: M022517718D
git_commit: 7095182559d62bd2b0d34e5d81f233d25e8a2239
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Sharpe / drawdown comparison — v3.0.0-regime RegimeDispatcher vs v1 momentum baseline

## Methodology

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Baseline scenario | top10-2023-fy-momentum-realdata (v1 cross-sectional momentum, real Binance data) |
| Dispatcher scenario | top10-2023-fy-regime-dispatcher-realdata (v3.0.0-regime MarkovSwitching 4-state, real Binance data) |
| Bar interval      | 1h |
| Annualisation     | sqrt(24*365) = 92.601295 (hourly -> annual) |
| Risk-free rate    | 0.000000 (constant) |
| Sharpe formula    | (mean_r - r_f) / std_r * sqrt(24*365) |
| T-classifier      | ADR-0049 D4: net_delta >= 0.10 -> T-REG-ALPHA-UNLOCKED, [0.05,0.10) -> T-REG-MARGINAL, <0.05 -> T-REG-NO-ALPHA |
| Hypothesis H1     | Regime-dispatcher Sharpe delta vs v1 baseline >= +0.10 (alpha-unlock threshold) |

## Comparison table

| Scenario | Bars | Final equity | Total return | Max drawdown | Trades | Suppress rate | Sharpe (ann) | Sortino (ann) | Calmar |
|----------|------|--------------|--------------|--------------|--------|----------------|--------------|---------------|--------|
| top10-2023-fy-momentum-realdata | 87590 | $113479.98 | 13.48% | 73.73% | 6203 | 0.00% | 0.003098 | 0.004380 | 0.017263 |
| top10-2023-fy-regime-dispatcher-realdata | 87590 | $87431.52 | -12.57% | 40.49% | 6847 | 11.20% | -0.291015 | -0.435466 | -0.032953 |

## Verdict

| Field               | Value                                          |
|---------------------|------------------------------------------------|
| Sharpe baseline     | 0.003098 (top10-2023-fy-momentum-realdata) |
| Sharpe dispatcher   | -0.291015 (top10-2023-fy-regime-dispatcher-realdata) |
| Gross Sharpe delta  | -0.294113 (dispatcher - baseline) |
| Net Sharpe delta    | -0.294113 (gross delta, no turnover cost modelled) |
| T-classifier        | T-REG-NO-ALPHA |
| V-REG verdict       | See regime-verdict-bs1-realdata report (ADR-0049 § D4). |

## H1 Hypothesis Discharge

| Field               | Value                                          |
|---------------------|------------------------------------------------|
| Hypothesis H1       | Regime-dispatcher Sharpe delta >= +0.10 vs v1 momentum baseline. |
| H1 result           | REJECTED: net_delta < +0.05 — regime-dispatcher does not deliver alpha lift at v0.1.0. |
| Net Sharpe delta    | -0.294113 |
| T-REG verdict       | T-REG-NO-ALPHA |

## Notes

- Both scenarios use real Binance 2023 hourly data (10 USDT pairs) — apples-to-apples.
- Dispatcher suppress rate = fraction of active bars in CashHold (Volatile/Calm) regime.
- Follow-on per joint advisory table (ADR-0049 § D4):
  - T-REG-ALPHA-UNLOCKED: SHIP + spawn v1.5-MR follow-on.
  - T-REG-MARGINAL: SHIP-WITH-CAVEATS or HOLD (operator decides).
  - T-REG-NO-ALPHA: HOLD-FOR-OPERATOR; C2 retire + close v3 three-pick set.
- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown/suppress_rate).
