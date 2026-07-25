---
scenario: eth-yahoo-2024-1d-sma-cross
seed: 0xC0FFEE
generated: 2026-05-27T21:56:52Z
wall_clock_s: 0.0
data_source: yahoo-cache:ETH-USD/1d/2024 rev=e018f876c36a
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

# Backtest Report — eth-yahoo-2024-1d-sma-cross

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | eth-yahoo-2024-1d-sma-cross            |
| Symbol               | ETH-USD                   |
| Start year           | 2024               |
| Bars replayed        | 367                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $102760.76 USDT        |
| Total return         | 2.76%                  |
| Sharpe ratio (ann.)  | 20.5436                |
| Max drawdown         | 4.96%               |
| Trades               | 7                   |
| Buys                 | 4                     |
| Sells                | 3                    |
| Total fees           | $27.594794 USDT            |
| Ledger imbalances    | 0                |
| LLM spend            | $0.00                      |
| Wall-clock time      | 0.0s              |
| Seed                 | 0xC0FFEE                 |
| Data source          | yahoo-cache:ETH-USD/1d/2024 rev=e018f876c36a              |

## Reconciliation

Minute-boundary reconciler ran at every bar close.
`ledger_imbalance_total == 0` — PASS.

## Notes

- v0 SMA crossover: fast=20, slow=50
- Slippage: 2 bps, Taker fee: 4 bps
- Size: fixed_fraction = 10%
- Risk: per-symbol exposure cap = 40%
