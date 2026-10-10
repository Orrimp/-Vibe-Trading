---
slug: strategy-robustness-harness
scenario: v1-momentum-2023-block-bootstrap-real-fy-mc
generated: 2026-10-10T15:28:11Z
wall_clock_s: 223.7
host: M022517718D
pid: 80388
git_commit: 7a09d40cfbf934ddba475cca9444219d6dab5fb0
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
---
# Monte-Carlo Robustness Summary — v1-momentum-2023-block-bootstrap-real-fy-mc

## Ensemble parameters

| Field                    | Value                                      |
|--------------------------|--------------------------------------------|
| master_seed              | 0xC0FFEE                             |
| fill_seed                | 0xC0FFEE                             |
| n_paths                  | 500                                   |
| sub_seed_rule            | "master + j*0x9E3779B9"                    |
| reduction_rule           | "index-order mean/std; total_cmp sort; type-7 linear pct" |
| generator                | block-bootstrap-real                          |
| bootstrap_mode           | shared-index                           |
| block_length_policy      | auto                      |
| selected_block_length_L  | 281                                         |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                      |
| param_set                | lookback=60 rebalance=60 k_long=3 exposure_cap=0.50 drift=0.10 vol_floor=0.000001                                |

## Per-metric distribution

| metric       | mean     | std      | p5       | p25      | p50      | p75      | p95      | min      | max      |
|--------------|----------|----------|----------|----------|----------|----------|----------|----------|----------|
| sharpe       | -0.719814 | 0.501074 | -1.560451 | -1.036495 | -0.747309 | -0.374518 | 0.108039 | -2.237020 | 0.879305 |
| sortino      | -1.040269 | 0.715321 | -2.193443 | -1.489444 | -1.101810 | -0.554115 | 0.163720 | -3.025638 | 1.373624 |
| calmar       | -0.077106 | 0.056670 | -0.124548 | -0.110946 | -0.094711 | -0.066222 | 0.028543 | -0.136678 | 0.265576 |
| max_drawdown | 34.13% | 9.88% | 17.32% | 27.04% | 34.81% | 41.48% | 49.54% | 11.23% | 59.62% |
| total_return | -0.248743 | 0.164217 | -0.466509 | -0.357531 | -0.281737 | -0.161402 | 0.052812 | -0.572554 | 0.544181 |

## Ensemble robustness

| Field                          | Value       |
|--------------------------------|-------------|
| P(final_equity < initial)      | 0.924000 |
| P(Sharpe > 0)                  | 0.076000 |
| P(Sharpe > 1.0)                | 0.000000 |
| max_drawdown_tail p50          | 34.81% |
| max_drawdown_tail p95          | 49.54% |

## Verdict

Sharpe p50: -0.747309
Sharpe spread (p95-p5): 1.668490
Verdict: MC-P50-NONPOSITIVE: p50 Sharpe ≤ 0 — ensemble median is non-positive

This `MC-P50-*` label is a one-signal summary of the median Sharpe and nothing more. It is NOT this project's robustness verdict: that is the FROZEN 5-signal weakest-link rule in `bakeoff/robustness.rs::classify_verdict` (vocabulary FRAGILE / MARGINAL / ROBUST), which reads p5, p50, prob_loss, P(Sharpe>1) and p95_maxdd together and is not computed anywhere in this report. The labels are namespaced so the two cannot be confused; until 2026-09-29 this line read ROBUST-ABOVE-1 / MARGINAL / WEAK, and MARGINAL collided outright.

Notes:
- Drawdown tail (p95 MaxDD) is the headline paper→live gate number.
- Generator: `block-bootstrap-real` (only `block-bootstrap-real` is anchor-grade).
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
