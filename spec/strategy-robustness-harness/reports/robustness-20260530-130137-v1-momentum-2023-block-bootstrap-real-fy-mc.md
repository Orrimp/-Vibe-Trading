---
slug: strategy-robustness-harness
scenario: v1-momentum-2023-block-bootstrap-real-fy-mc
generated: 2026-05-30T13:01:37Z
wall_clock_s: 179.4
host: M022517718D
pid: 78505
git_commit: 8ed3b1bff2e050d9838ce20bc9e5e3c5c37e8f6c
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
| sharpe       | -0.016325 | 0.020708 | -0.050256 | -0.033527 | -0.010446 | -0.000096 | 0.009047 | -0.079927 | 0.055429 |
| sortino      | -0.022955 | 0.029127 | -0.070643 | -0.047306 | -0.014738 | -0.000136 | 0.012771 | -0.110318 | 0.079575 |
| calmar       | -0.065929 | 0.079061 | -0.187118 | -0.142670 | -0.049283 | -0.000543 | 0.043848 | -0.221307 | 0.146961 |
| max_drawdown | 79.52% | 9.70% | 61.32% | 73.06% | 81.39% | 87.64% | 91.50% | 47.21% | 94.89% |
| total_return | -0.304323 | 0.451104 | -0.841754 | -0.729693 | -0.315157 | -0.004257 | 0.393117 | -0.901654 | 2.149302 |

## Ensemble robustness

| Field                          | Value       |
|--------------------------------|-------------|
| P(final_equity < initial)      | 0.752000 |
| P(Sharpe > 0)                  | 0.248000 |
| P(Sharpe > 1.0)                | 0.000000 |
| max_drawdown_tail p50          | 81.39% |
| max_drawdown_tail p95          | 91.50% |

## Verdict

Sharpe p50: -0.010446
Sharpe spread (p95-p5): 0.059303
Verdict: WEAK: p50 Sharpe ≤ 0 — ensemble median is non-positive

Notes:
- Drawdown tail (p95 MaxDD) is the headline paper→live gate number.
- Generator: `block-bootstrap-real` (only `block-bootstrap-real` is anchor-grade).
- Determinism scope: Apple-Silicon canonical box (ADR-0051 D5).
