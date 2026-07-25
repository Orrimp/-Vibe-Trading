---
scenario: btc-2023-1m-sma-baseline-refresh
seed: 0xC0FFEE
generated: 2026-05-27T06:51:24Z
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

# Backtest Report — btc-2023-1m-sma-cross

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | btc-2023-1m-sma-cross            |
| Symbol               | BTCUSDT                   |
| Start year           | 2023               |
| Bars replayed        | 17544                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $111248.17 USDT        |
| Total return         | 11.25%                  |
| Sharpe ratio (ann.)  | 11.6219                |
| Max drawdown         | 3.65%               |
| Trades               | 441                   |
| Buys                 | 221                     |
| Sells                | 220                    |
| Total fees           | $1882.298229 USDT            |
| Ledger imbalances    | 0                |
| LLM spend            | $0.00                      |
| Wall-clock time      | 0.2s              |
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
