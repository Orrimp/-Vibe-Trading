---
scenario: arb-2024-sma-cross
seed: 0xC0FFEE
generated: 2026-06-19T05:42:33Z
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

# Backtest Report — arb-2024-sma-cross

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | arb-2024-sma-cross            |
| Symbol               | ARBUSDT                   |
| Start year           | 2024               |
| Bars replayed        | 8785                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $96346.18 USDT        |
| Total return         | -3.65%                  |
| Sharpe ratio (ann.)  | -3.8237                |
| Max drawdown         | 9.18%               |
| Trades               | 186                   |
| Buys                 | 93                     |
| Sells                | 93                    |
| Total fees           | $718.413698 USDT            |
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
