---
scenario: btc-2023-1m-sma-baseline-refresh
seed: 0xC0FFEE
generated: 2026-05-27T18:13:24Z
wall_clock_s: 0.2
data_source: synthetic (seeded RNG, v0 fallback)
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
| Bars replayed        | 525601                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $17992.64 USDT        |
| Total return         | -82.01%                  |
| Sharpe ratio (ann.)  | -28.9324                |
| Max drawdown         | 82.07%               |
| Trades               | 12077                   |
| Buys                 | 6039                     |
| Sells                | 6038                    |
| Total fees           | $22739.926597 USDT            |
| Ledger imbalances    | 0                |
| LLM spend            | $0.00                      |
| Wall-clock time      | 0.2s              |
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
