---
slug: strategy-robustness-harness
scenario: v1-momentum-2023-block-bootstrap-real-fy-mc
generated: 2026-09-27T15:54:36Z
wall_clock_s: 225.4
host: M022517718D
pid: 66086
git_commit: da82050a940702fead8e7eaec2107c1f49c3d865
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
| sharpe       | -0.730859 | 0.488670 | -1.522504 | -1.032574 | -0.718332 | -0.410990 | 0.082100 | -2.106435 | 0.964779 |
| sortino      | -1.055373 | 0.694608 | -2.138735 | -1.505111 | -1.055901 | -0.611412 | 0.124304 | -2.867845 | 1.512820 |
| calmar       | -0.079505 | 0.052477 | -0.126091 | -0.111482 | -0.093979 | -0.067298 | 0.015103 | -0.138015 | 0.362038 |
| max_drawdown | 34.12% | 9.49% | 18.49% | 27.08% | 34.68% | 40.91% | 50.24% | 10.61% | 58.77% |
| total_return | -0.252930 | 0.156889 | -0.460859 | -0.359754 | -0.268198 | -0.169439 | 0.040304 | -0.553781 | 0.661141 |

## Ensemble robustness

| Field                          | Value       |
|--------------------------------|-------------|
| P(final_equity < initial)      | 0.924000 |
| P(Sharpe > 0)                  | 0.076000 |
| P(Sharpe > 1.0)                | 0.000000 |
| max_drawdown_tail p50          | 34.68% |
| max_drawdown_tail p95          | 50.24% |

## Verdict

Sharpe p50: -0.718332
Sharpe spread (p95-p5): 1.604605
Verdict: WEAK: p50 Sharpe ≤ 0 — ensemble median is non-positive

Notes:
- Drawdown tail (p95 MaxDD) is the headline paper→live gate number.
- Generator: `block-bootstrap-real` (only `block-bootstrap-real` is anchor-grade).
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
