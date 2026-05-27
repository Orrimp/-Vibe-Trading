---
scenario: btc-2023-1m-bbands-mean-revert
seed: 0xC0FFEE
generated: 2026-05-27T18:13:31Z
wall_clock_s: 0.0
data_source: real (Binance Vision)
baseline_report: backtest-20260420-151944-btc-2023-1m-sma-baseline-refresh.md
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: btc_bbands_mean_revert
kind: composed
content_hash: ff25674dc79b2f6ce0be0cdc9c030b7f62c5c96c2066754e8f06776439447d92
source: config/strategies/btc_bbands_mean_revert.toml
signal: close < bollinger_lower(20,2) AND volume > 1.5 * avg(volume, 20)
---

# Backtest Report — btc-2023-1m-bbands-mean-revert

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | btc-2023-1m-bbands-mean-revert            |
| Symbol               | BTCUSDT                   |
| Start year           | 2023               |
| Bars replayed        | 17544                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $89723.52 USDT        |
| Total return         | -10.28%                  |
| Sharpe ratio (ann.)  | -32.8944                |
| Max drawdown         | 10.31%               |
| Trades               | 814                   |
| Buys                 | 407                     |
| Sells                | 407                    |
| Total fees           | $3087.832951 USDT            |
| Ledger imbalances    | 0                |
| LLM spend            | $0.00                      |
| Wall-clock time      | 6.2s              |
| Seed                 | 0xC0FFEE                 |
| Data source          | real (Binance Vision)              |

## Reconciliation

Minute-boundary reconciler ran at every bar close.
`ledger_imbalance_total == 0` — PASS.

## Notes

- Composed strategy: btc_bbands_mean_revert
- Slippage: 2 bps, Taker fee: 4 bps
- Size: fixed_fraction = 10%
- Risk: per-symbol exposure cap = 40%
