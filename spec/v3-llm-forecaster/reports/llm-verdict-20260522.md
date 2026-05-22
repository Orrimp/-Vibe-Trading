---
slug: v3-llm-forecaster
scenario: llm-verdict-audit-db-window
generated: 2026-05-22T20:00:14Z
wall_clock_s: 0.0
host: M022517718D
git_commit: 2da745cb85ec59abb1c02dd8ca7dd04b592eac10
audit_db: data/audit.db
verdict: L2
---
# LLM-forecaster L-verdict report (ADR-0039 § D1)

## Window summary

| Field                     | Value                                |
|---------------------------|--------------------------------------|
| window_label              | audit-db-window                                   |
| window_bars_requested     | 1000                                   |
| n_calls                   | 0                                   |
| n_unique_traces           | 0                                   |
| n_traces_below_50_chars   | 0                                   |
| mean_trace_len_chars      | 0.000000                            |
| cost_actual_usd           | 0.000000                            |
| cost_projected_usd        | 0.100000                            |
| cost_cap_usd              | 100.000000                            |
| confidence_outcome_corr   | 0.000000                            |

## Rating distribution

| Rating       | Count | Fraction |
|--------------|-------|----------|
| STRONG_SELL  | 0     | 0.000000 |
| SELL         | 0     | 0.000000 |
| HOLD         | 0     | 0.000000 |
| BUY          | 0     | 0.000000 |
| STRONG_BUY   | 0     | 0.000000 |

## Computed metrics (L-verdict inputs)

| Metric                  | Value      | Threshold         |
|-------------------------|------------|-------------------|
| hold_frac               | 0.000000   | >= 0.95 fires L1  |
| |confidence_outcome_corr| | 0.000000   | < 0.05 fires L2   |
| overrun_ratio           | 0.000000   | > 2.0 fires L3    |
| short_frac              | 0.000000   | > 0.50 fires L4   |
| duplicate_frac          | 1.000000   | > 0.50 fires L4   |

## Verdict

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Case              | L2                                             |
| Trigger evidence  | |confidence_outcome_corr| = 0.000000 < 0.05 (calibration failure) |
| Routes to         | v3-llm-forecaster-calibrate-or-retire |

## Notes

- L-verdict algorithm: see [ADR-0039 § D1](../../architecture/adr/0039-llm-forecaster-verdict-criteria.md#d1-l-verdict-priority-tree).
- Read-only against the audit DB; no writes to any table.
- `confidence_outcome_corr` requires realised returns (not in audit DB). Pass via `--confidence-outcome-corr` if known from a backtest run; default 0.0 triggers L2 as a conservative fallback.
- **WARNING**: zero rows found in `llm_forecast_entries`. The audit DB may not have migration 012 applied or the LLM forecaster has not been run yet. L-verdict result reflects an empty window.
