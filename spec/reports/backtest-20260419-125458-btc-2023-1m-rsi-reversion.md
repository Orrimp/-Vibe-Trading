---
scenario: btc-2023-1m-rsi-reversion
seed: 0xC0FFEE
generated: 2026-04-19T12:54:58Z
data_source: synthetic (seeded RNG, v0 fallback)
baseline_report: backtest-20260419-123140-btc-2023-1m-sma-baseline-refresh.md
ledger_imbalance_total: 0
llm_spend_usd: 0.00
---

# Backtest Report — btc-2023-1m-rsi-reversion

## Strategy

| Field        | Value                                                    |
|--------------|----------------------------------------------------------|
| ID           | btc_rsi_reversion                                               |
| Kind         | composed                                             |
| Hash         | 336e0c0970645643631562140b3644b73260f376215d8aedf1804f670a667355                                             |
| Source       | config/strategies/btc_rsi_reversion.toml                                           |
| Signal       | rsi(14) < 30 AND close > min(low, 20)                                           |

## Summary

| Metric               | Value                      |
|----------------------|----------------------------|
| Scenario             | btc-2023-1m-rsi-reversion            |
| Symbol               | BTCUSDT                   |
| Start year           | 2023               |
| Bars replayed        | 525601                     |
| Initial capital      | $100000.00 USDT         |
| Final equity         | $42195.44 USDT        |
| Total return         | -57.80%                  |
| Sharpe ratio (ann.)  | -55.4257                |
| Max drawdown         | 57.81%               |
| Trades               | 14118                   |
| Buys                 | 7059                     |
| Sells                | 7059                    |
| Total fees           | $37843.260548 USDT            |
| Ledger imbalances    | 0                |
| LLM spend            | $0.00                      |
| Wall-clock time      | 0.3s              |
| Seed                 | 0xC0FFEE                 |
| Data source          | synthetic (seeded RNG, v0 fallback)              |

## Reconciliation

Minute-boundary reconciler ran at every bar close.
`ledger_imbalance_total == 0` — PASS.

## Notes

- - Composed strategy: btc_rsi_reversion
- Slippage: 2 bps, Taker fee: 4 bps
- Size: fixed_fraction = 10%
- Risk: per-symbol exposure cap = 40%
