---
slug: v3-volatility-forecaster-rebaseline
scenario: sharpe-comparison-vol-target-bs1-realbaseline
generated: 2026-10-01T14:46:27Z
wall_clock_s: 8.0
host: M022517718D
git_commit: 998f6db3828798859ce31f1cb9c53fdc944a5c21
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Sharpe / drawdown comparison — v3.0.0-volatility-rebaseline GARCH vol-targeting overlay

## Methodology

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Baseline scenario | top10-2023-fy-momentum-realdata (v1 cross-sectional momentum, real Binance data) |
| Overlay scenario  | top10-2023-fy-vol-target-overlay-realdata (GARCH BS-1 vol-targeting, real Binance data) |
| Bar interval      | 1h |
| Annualisation     | sqrt(8574.9998) = 92.601295 (hourly -> annual; NOT sqrt(24*365) = 93.594872) |
| Risk-free rate    | 0.000000 (constant) |
| Sharpe formula    | (mean_r - r_f) / std_r * sqrt(8574.9998) |
| T-classifier      | ADR-0038 D1.c: net_delta >= 0.10 -> T-VOL-ALPHA-UNLOCKED, [0.05,0.10) -> T-VOL-MARGINAL, <0.05 -> T-VOL-NO-ALPHA |

## Comparison table

| Scenario | Bars | Final equity | Total return | Max drawdown | Trades | Sharpe (ann) | Sortino (ann) | Calmar |
|----------|------|--------------|--------------|--------------|--------|--------------|---------------|--------|
| top10-2023-fy-momentum-realdata | 87590 | $95306.77 | -4.69% | 9.58% | 1138 | -0.328302 | -0.483775 | -0.050072 |
| top10-2023-fy-vol-target-overlay-realdata | 87590 | $89478.53 | -10.52% | 19.05% | 1138 | -0.407398 | -0.593449 | -0.058048 |

## Verdict

| Field               | Value                                          |
|---------------------|------------------------------------------------|
| Sharpe baseline     | -0.328302 (top10-2023-fy-momentum-realdata) |
| Sharpe overlay      | -0.407398 (top10-2023-fy-vol-target-overlay-realdata) |
| Gross Sharpe delta  | -0.079096 (overlay - baseline) |
| Net Sharpe delta    | -0.079096 (gross delta, no turnover cost modelled) |
| T-classifier        | T-VOL-NO-ALPHA |
| V-verdict (joint)   | V3 (mean_calibration_ratio = 2.952191 outside [0.7, 1.4] — see vol-verdict-bs1-realdata report) |

## Notes

- Baseline (top10-2023-fy-momentum-realdata) and overlay (top10-2023-fy-vol-target-overlay-realdata) both use real Binance 2023 hourly data — apples-to-apples comparison per v0.1.0-rebaseline disambiguation.
- V-verdict V3 fires because GARCH unconditioned-var overflow on AVAX/DOGE/DOT (non-convergence at 500 iters).
- Follow-on: v3-garch-calibration-tune to improve GARCH fitting for non-convergent symbols.
- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown); integer bar/trade counts.
