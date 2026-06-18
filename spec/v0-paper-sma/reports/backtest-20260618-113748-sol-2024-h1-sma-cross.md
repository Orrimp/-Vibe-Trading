---
scenario: sol-2024-h1-sma-cross
seed: 0xC0FFEE
generated: 2026-06-18T11:37:48Z
wall_clock_s: 0.0
data_source: real (Binance Vision)
baseline_report: n/a
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: sma_crossover
kind: compiled-in
content_hash: n/a
source: compiled-in
signal: sma_crossover(fast=20, slow=50)
---

# Backtest Report — sol-2024-h1-sma-cross

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | sol-2024-h1-sma-cross            |
| Symbol               | SOLUSDT                   |
| Start year           | 2024               |
| Bars replayed        | 17544                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $125496.41 USDT        |
| Total return         | 25.50%                  |
| Sharpe ratio (ann.)  | 12.0394                |
| Max drawdown         | 8.54%               |
| Trades               | 387                   |
| Buys                 | 194                     |
| Sells                | 193                    |
| Total fees           | $1767.768464 USDT            |
| Ledger imbalances    | 0                |
| LLM spend            | $0.00                      |
| Wall-clock time      | 0.1s              |
| Seed                 | 0xC0FFEE                 |
| Data source          | real (Binance Vision)              |

## Reconciliation

Minute-boundary reconciler ran at every bar close.
`ledger_imbalance_total == 0` — PASS.

## Notes

- v0 SMA crossover: fast=20, slow=50
- Slippage: 2 bps, Taker fee: 4 bps
- Size: fixed_fraction = 10%
- Risk: per-symbol exposure cap = 40%
