---
scenario: ada-2024-h1-sma-cross
seed: 0xC0FFEE
generated: 2026-06-18T11:38:07Z
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

# Backtest Report — ada-2024-h1-sma-cross

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | ada-2024-h1-sma-cross            |
| Symbol               | ADAUSDT                   |
| Start year           | 2024               |
| Bars replayed        | 17544                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $108660.53 USDT        |
| Total return         | 8.66%                  |
| Sharpe ratio (ann.)  | 5.2183                |
| Max drawdown         | 11.54%               |
| Trades               | 396                   |
| Buys                 | 198                     |
| Sells                | 198                    |
| Total fees           | $1624.728219 USDT            |
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
