---
slug: v3-volatility-forecaster-rebaseline
scenario: sharpe-comparison-vol-target-bs1-realbaseline
generated: 2026-05-22T09:53:20Z
wall_clock_s: 6.7
host: M022517718D
git_commit: 7be25ab19549901f7869f3fbfad3d80830992630
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Sharpe / drawdown comparison — v3.0.0-volatility-rebaseline GARCH vol-targeting overlay

## Methodology

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Baseline scenario | top10-2023-fy-momentum-realdata (v1 cross-sectional momentum, real Binance data) |
| Overlay scenario  | top10-2023-fy-vol-target-overlay-realdata (GARCH BS-1 vol-targeting, real Binance data) |
| Bar interval      | 1h |
| Annualisation     | sqrt(24*365) = 92.601295 (hourly -> annual) |
| Risk-free rate    | 0.000000 (constant) |
| Sharpe formula    | (mean_r - r_f) / std_r * sqrt(24*365) |
| T-classifier      | ADR-0038 D1.c: net_delta >= 0.10 -> T-VOL-ALPHA-UNLOCKED, [0.05,0.10) -> T-VOL-MARGINAL, <0.05 -> T-VOL-NO-ALPHA |

## Comparison table

| Scenario | Bars | Final equity | Total return | Max drawdown | Trades | Sharpe (ann) | Sortino (ann) | Calmar |
|----------|------|--------------|--------------|--------------|--------|--------------|---------------|--------|
| top10-2023-fy-momentum-realdata | 87590 | $0.00 | 13.48% | 73.73% | 6203 | 0.003098 | 0.004380 | 0.017263 |
| top10-2023-fy-vol-target-overlay-realdata | 87590 | $0.00 | 13.48% | 73.73% | 6203 | 0.003098 | 0.004380 | 0.017263 |

## Verdict

| Field               | Value                                          |
|---------------------|------------------------------------------------|
| Sharpe baseline     | 0.003098 (top10-2023-fy-momentum-realdata) |
| Sharpe overlay      | 0.003098 (top10-2023-fy-vol-target-overlay-realdata) |
| Gross Sharpe delta  | 0.000000 (overlay - baseline) |
| Net Sharpe delta    | 0.000000 (gross delta, no turnover cost modelled) |
| T-classifier        | T-VOL-NO-ALPHA |
| V-verdict (joint)   | V3 (mean_calibration_ratio = 2.952191 outside [0.7, 1.4] — see vol-verdict-bs1-realdata report) |

## Notes

- Baseline (top10-2023-fy-momentum-realdata) and overlay (top10-2023-fy-vol-target-overlay-realdata) both use real Binance 2023 hourly data — apples-to-apples comparison per v0.1.0-rebaseline disambiguation.
- V-verdict V3 fires because GARCH unconditioned-var overflow on AVAX/DOGE/DOT (non-convergence at 500 iters).
- Follow-on: v3-garch-calibration-tune to improve GARCH fitting for non-convergent symbols.
- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown); integer bar/trade counts.
