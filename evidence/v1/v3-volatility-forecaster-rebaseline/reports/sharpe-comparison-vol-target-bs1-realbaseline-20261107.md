---
slug: v3-volatility-forecaster-rebaseline
scenario: sharpe-comparison-vol-target-bs1-realbaseline
generated: 2026-11-07T15:15:42Z
wall_clock_s: 7.3
host: M022517718D
git_commit: bbeeb8a4e561f9e7f50c1e66309ab4877b2ef1b3
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

**What this label does and does not say.** `T-VOL-ALPHA-UNLOCKED` is ADR-0038 § D1.c's delta rule — `net_delta >= 0.10` — applied to `-0.505118 − -0.693194`. **Both arms LOSE money here — baseline -0.693194, overlay -0.505118, annualised Sharpe. The overlay loses LESS; it does not earn.** The baseline here is the REAL-data `top10-2023-fy-momentum-realdata`, which is why this report exists: the synthetic-baseline sibling `sharpe-comparison-vol-target-bs1-realdata` was judged the wrong reference. A label above the threshold still says only that the DELTA cleared it.

## Comparison table

| Scenario | Bars | Final equity | Total return | Max drawdown | Trades | Sharpe (ann) | Sortino (ann) | Calmar |
|----------|------|--------------|--------------|--------------|--------|--------------|---------------|--------|
| top10-2023-fy-momentum-realdata | 87590 | $72483.57 | -27.52% | 41.80% | 6203 | -0.693194 | -1.016926 | -0.075770 |
| top10-2023-fy-vol-target-overlay-realdata | 87590 | $72330.48 | -27.67% | 40.33% | 3266 | -0.505118 | -0.752492 | -0.079048 |

## Verdict

| Field               | Value                                          |
|---------------------|------------------------------------------------|
| Sharpe baseline     | -0.693194 (top10-2023-fy-momentum-realdata) |
| Sharpe overlay      | -0.505118 (top10-2023-fy-vol-target-overlay-realdata) |
| Gross Sharpe delta  | 0.188076 (overlay - baseline) |
| Net Sharpe delta    | 0.188076 (gross delta, no turnover cost modelled) |
| T-classifier        | T-VOL-ALPHA-UNLOCKED |
| V-verdict (joint)   | V3 (mean_calibration_ratio = 2.952191 outside [0.7, 1.4] — see vol-verdict-bs1-realdata report) |

## Notes

- Baseline (top10-2023-fy-momentum-realdata) and overlay (top10-2023-fy-vol-target-overlay-realdata) both use real Binance 2023 hourly data — apples-to-apples comparison per v0.1.0-rebaseline disambiguation.
- V-verdict V3 fires because GARCH unconditioned-var overflow on AVAX/DOGE/DOT (non-convergence at 500 iters).
- Follow-on: v3-garch-calibration-tune to improve GARCH fitting for non-convergent symbols.
- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown); integer bar/trade counts.
