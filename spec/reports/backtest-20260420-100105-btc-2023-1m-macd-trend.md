---
scenario: btc-2023-1m-macd-trend
seed: 0xC0FFEE
generated: 2026-04-20T10:01:05Z
wall_clock_s: 2.5
data_source: synthetic (seeded RNG, v0 fallback)
baseline_report: backtest-20260420-050422-btc-2023-1m-sma-baseline-refresh.md
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: btc_macd_trend
kind: composed
content_hash: 847d9303c6c652ffc2f085060c3f5380904fd35ea8746ddcc189e2aaa0544651
source: config/strategies/btc_macd_trend.toml
signal: macd_hist(12,26,9) > 0 AND close > ema(200)
---

# Backtest Report — btc-2023-1m-macd-trend

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | btc-2023-1m-macd-trend            |
| Symbol               | BTCUSDT                   |
| Start year           | 2023               |
| Bars replayed        | 525601                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $20550.94 USDT        |
| Total return         | -79.45%                  |
| Sharpe ratio (ann.)  | -40.3994                |
| Max drawdown         | 79.49%               |
| Trades               | 25952                   |
| Buys                 | 12976                     |
| Sells                | 12976                    |
| Total fees           | $52277.583899 USDT            |
| Ledger imbalances    | 0                |
| LLM spend            | $0.00                      |
| Wall-clock time      | 2.5s              |
| Seed                 | 0xC0FFEE                 |
| Data source          | synthetic (seeded RNG, v0 fallback)              |

## Reconciliation

Minute-boundary reconciler ran at every bar close.
`ledger_imbalance_total == 0` — PASS.

## Notes

- Composed strategy: btc_macd_trend
- Slippage: 2 bps, Taker fee: 4 bps
- Size: fixed_fraction = 10%
- Risk: per-symbol exposure cap = 40%
