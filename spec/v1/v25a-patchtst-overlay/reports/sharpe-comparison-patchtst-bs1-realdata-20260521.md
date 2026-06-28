---
slug: v25a-patchtst-overlay
scenario: sharpe-comparison-patchtst-bs1-realdata
generated: 2026-05-21T22:09:46Z
wall_clock_s: 123.4
host: M022517718D
git_commit: a0dee41ebc1352891518cd8ac11d6826cd8992de
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
sources:
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2023-fy-tcn-overlay-realdata.md
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2024-fy-tcn-overlay-realdata.md
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2023-fy-tcn-overlay-weights-realdata.md
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2024-fy-tcn-overlay-weights-realdata.md
  - spec/backtest-real-binance-data/reports/backtest-…-top10-2023-fy-patchtst-overlay-realdata.md
---
# Sharpe / drawdown comparison — v2.6.0-realdata + v2.5a-patchtst-overlay scenarios

## Methodology

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Source equity     | Re-run of the five -realdata scenarios (four TCN + one PatchTST BS-1, Option α per ADR-0033 § D2.b.i). |
| Bar interval      | 1h |
| Annualisation     | √(24·365) = 92.601295 (hourly → annual) |
| Risk-free rate    | 0.000000 (constant) |
| Sharpe formula    | (mean_r - r_f) / std_r * √(24·365), arithmetic returns |
| Sortino formula   | (mean_r - r_f) / std_downside_r * √(24·365), downside vs r_f |
| Calmar formula    | (CAGR) / abs(max_drawdown), where CAGR = (final/initial)^(1/years) - 1, years = bars/8760 |
| Max drawdown      | max over t of (peak_equity_t - equity_t) / peak_equity_t, on the realised equity curve |
| Equity series     | Per-bar equity_curve: Vec<Decimal> from --emit-equity-bin, starting at $100000.00 |
| compute_sharpe_hourly | New helper in sharpe_comparison.rs (NOT crates/backtest::compute_sharpe, which annualises by sqrt(525_600) for minute bars — see ADR-0033 § D4 alt-7). |

## Comparison table

| Scenario | Variant | Bars | Final equity | Total return | Max drawdown | Trades | Dampen rate | Sharpe (ann) | Sortino (ann) | Calmar |
|----------|---------|------|--------------|--------------|--------------|--------|-------------|--------------|---------------|--------|
| top10-2023-fy-tcn-overlay-realdata | passthrough | 87590 | $0.00 | 13.48% | 73.73% | 6203 | 0.00% | 0.003098 | 0.004380 | 0.017263 |
| top10-2024-fy-tcn-overlay-realdata | passthrough | 87840 | $0.00 | 5.21% | 78.82% | 5917 | 0.00% | 0.001389 | 0.001965 | 0.006447 |
| top10-2023-fy-tcn-overlay-weights-realdata | real-weights | 87590 | $0.00 | 13.48% | 73.73% | 6203 | 0.00% | 0.003098 | 0.004380 | 0.017263 |
| top10-2024-fy-tcn-overlay-weights-realdata | real-weights | 87840 | $0.00 | 5.21% | 78.82% | 5917 | 0.00% | 0.001389 | 0.001965 | 0.006447 |
| top10-2023-fy-patchtst-overlay-realdata | patchtst-real-weights | 87590 | $0.00 | 31.13% | 77.97% | 3187 | 0.00% | 0.009243 | 0.013133 | 0.035234 |

## Verdict

| Field             | Value                                          |
|-------------------|------------------------------------------------|
| Honest reading    | dampen rate = 0.00% across all five scenarios — overlay models are no-ops; equity curves are byte-identical between passthrough and real-weights variants per year. |
| Sharpe delta (TCN)      | 0.000000 (passthrough vs. real-weights, 2023) / 0.000000 (2024) |
| Sharpe delta (PatchTST) | 0.006144 (passthrough-2023 vs. patchtst-bs1-2023) |
| Conclusion        | TCN and PatchTST at v2.5a produce no alpha lift over the v1 momentum baseline. PatchTST F-verdict: F4 (see forecast-distribution-patchtst-bs1 report). |
| Recommended follow-on | (a) if both models land F4, fund v25-tcn-horizon-bump OR retire TCN at v2.6 bake-off; (b) PatchTST 24h horizon may need longer backtest burn-in (336-bar warmup). |

## Notes

- Read-only against the five -realdata reports listed in frontmatter.
- This report re-runs five backtest scenarios (four TCN + one PatchTST BS-1, Option α per ADR-0033 § D2.b.i).
- ASCII-only, LF-only line endings; floats %.6f (Sharpe/Sortino/Calmar) or %.2f%% (returns/drawdown/dampen rate); integer bar/trade counts.
