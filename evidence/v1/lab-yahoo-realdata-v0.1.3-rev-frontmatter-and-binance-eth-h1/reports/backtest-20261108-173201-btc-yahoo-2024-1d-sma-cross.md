---
scenario: btc-yahoo-2024-1d-sma-cross
seed: 0xC0FFEE
generated: 2026-11-08T17:32:01Z
wall_clock_s: 0.0
data_source: yahoo-cache:BTC-USD/1d/2024
revision_sha: fc2f7883ae505fc5141f6048bf2bdc2fe3069853e671b30ae7ad99924a314372
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

# Backtest Report — btc-yahoo-2024-1d-sma-cross

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | btc-yahoo-2024-1d-sma-cross            |
| Symbol               | BTC-USD                   |
| Start year           | 2024               |
| Bars replayed        | 367                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $104560.08 USDT        |
| Total return         | 4.56%                  |
| Buy and hold (equal-wt) | +111.45% ($211450.02 USDT, 1 legs, $40.00 fees)            |
| vs buy and hold      | -106.89 pp                 |
| Sharpe ratio (ann.)  | 34.3359                |
| Max drawdown         | 4.83%               |
| Trades               | 7                   |
| Buys                 | 4                     |
| Sells                | 3                    |
| Total fees           | $28.200175 USDT            |
| Ledger imbalances    | 0                |
| LLM spend            | $0.00                      |
| Wall-clock time      | 0.0s              |
| Seed                 | 0xC0FFEE                 |
| Data source          | yahoo-cache:BTC-USD/1d/2024              |

## Reconciliation

Minute-boundary reconciler ran at every bar close.
`ledger_imbalance_total == 0` — PASS.

## Notes

- v0 SMA crossover: fast=20, slow=50
- Slippage: 2 bps, Taker fee: 4 bps
- Size: fixed_fraction = 10%
- Risk: per-symbol exposure cap = 40%
