---
scenario: btc-2024-h1-sma-cross
seed: 0xC0FFEE
generated: 2026-04-18T22:29:15Z
data_source: synthetic (seeded RNG, v0 fallback)
baseline_report: backtest-20260418-222833-btc-2023-1m-sma-cross.md
ledger_imbalance_total: 0
llm_spend_usd: 0.00
---

# Backtest Report — btc-2024-h1-sma-cross

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | btc-2024-h1-sma-cross            |
| Symbol               | BTCUSDT                   |
| Start year           | 2024               |
| Bars replayed        | 262801                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $67241.80 USDT        |
| Total return         | -32.76%                  |
| Sharpe ratio (ann.)  | -13.8684                |
| Max drawdown         | 32.99%               |
| Trades               | 6068                   |
| Buys                 | 3034                     |
| Sells                | 3034                    |
| Total fees           | $19934.338770 USDT            |
| Ledger imbalances    | 0                |
| LLM spend            | $0.00                      |
| Wall-clock time      | 0.1s              |
| Seed                 | 0xC0FFEE                 |
| Data source          | synthetic (seeded RNG, v0 fallback)              |

## Reconciliation

Minute-boundary reconciler ran at every bar close.
`ledger_imbalance_total == 0` — PASS.

## Notes

- v0 SMA crossover: fast=20, slow=50
- Slippage: 2 bps, Taker fee: 4 bps
- Size: fixed_fraction = 10%
- Risk: per-symbol exposure cap = 40%
