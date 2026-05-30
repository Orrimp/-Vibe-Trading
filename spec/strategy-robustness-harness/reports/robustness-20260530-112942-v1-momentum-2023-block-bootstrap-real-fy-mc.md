---
slug: strategy-robustness-harness
scenario: v1-momentum-2023-block-bootstrap-real-fy-mc
generated: 2026-05-30T11:29:42Z
wall_clock_s: 183.5
host: M022517718D
pid: 53644
git_commit: f9d9cced868a26979488214a833e95c0eeaee79d
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
| selected_block_length_L  | 204                                         |
| source_revision_sha      | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7                      |
| param_set                | lookback=60 rebalance=60 k_long=3 exposure_cap=0.50 drift=0.10 vol_floor=0.000001                                |

## Per-metric distribution

| metric       | mean     | std      | p5       | p25      | p50      | p75      | p95      | min      | max      |
|--------------|----------|----------|----------|----------|----------|----------|----------|----------|----------|
| sharpe       | -0.030031 | 0.044594 | -0.067576 | -0.041437 | -0.021924 | -0.004752 | 0.003101 | -0.290850 | 0.038903 |
| sortino      | -0.039731 | 0.049662 | -0.094839 | -0.058373 | -0.030719 | -0.006705 | 0.004391 | -0.299594 | 0.055498 |
| calmar       | -0.133283 | 0.189822 | -0.311330 | -0.165741 | -0.109217 | -0.022870 | 0.016522 | -0.920567 | 0.111841 |
| max_drawdown | 82.16% | 11.66% | 61.32% | 73.39% | 85.29% | 90.93% | 100.00% | 47.21% | 100.00% |
| total_return | -0.487027 | 0.377616 | -0.975746 | -0.804599 | -0.605409 | -0.150770 | 0.115274 | -1.000000 | 1.614029 |

## Ensemble robustness

| Field                          | Value       |
|--------------------------------|-------------|
| P(final_equity < initial)      | 0.868000 |
| P(Sharpe > 0)                  | 0.132000 |
| P(Sharpe > 1.0)                | 0.000000 |
| max_drawdown_tail p50          | 85.29% |
| max_drawdown_tail p95          | 100.00% |

## Verdict

Sharpe p50: -0.021924
Sharpe spread (p95-p5): 0.070677
Verdict: WEAK: p50 Sharpe ≤ 0 — ensemble median is non-positive

Notes:
- Drawdown tail (p95 MaxDD) is the headline paper→live gate number.
- Generator: `block-bootstrap-real` (only `block-bootstrap-real` is anchor-grade).
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
