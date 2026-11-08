---
scenario: pairs-2024-h1-zscore-mr
seed: 0xC0FFEE
generated: 2026-11-08T15:22:05Z
wall_clock_s: 0.2
data_source: synthetic (seeded RNG, v1.5a multi-symbol)
baseline_report: n/a
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: pairs_mr_h1
kind: mean_reversion_pairs
content_hash: 442a2e46f96c547c596e8d85f565350de626ec1230ae1cbe96c0efe85a390a5d
source: config/strategies/pairs_mr_h1.toml
signal: zscore_spread(lookback=60,z_entry=2.0,z_exit=0.5)
---

# Backtest Report — pairs-2024-h1-zscore-mr

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | pairs-2024-h1-zscore-mr               |
| Universe             | 4 symbols      |
| Start year           | 2024                  |
| Bars (total)         | 17520                   |
| Initial capital      | $100000.00 USDT            |
| Final equity         | $93657.62 USDT           |
| Total return         | -6.34%                     |
| Buy and hold (equal-wt) | +37.22% ($137222.80 USDT, 4 legs, $40.00 fees)                 |
| vs buy and hold        | -43.57 pp                    |
| Max drawdown         | 14.05%                  |
| Trades               | 216                      |
| Buys                 | 108                        |
| Sells                | 108                       |
| Total fees           | $2146.834377 USDT               |
| Ledger imbalances    | 0                             |
| Seed                 | 0xC0FFEE                    |
| Data source          | synthetic (seeded RNG, v1.5a multi-symbol)                 |

## Per-Pair Summary (R8.5)

| Pair                 | Trades |
|----------------------|--------|
| (BNBUSDT, BTCUSDT) | 68 |
| (BTCUSDT, ETHUSDT) | 64 |
| (ETHUSDT, SOLUSDT) | 84 |

## Universe

- BNBUSDT
- BTCUSDT
- ETHUSDT
- SOLUSDT

## Reconciliation

Reconciler ran at every bar close.
`ledger_imbalance_total == 0` — PASS.

## Notes

- v1.5a mean-reversion pairs: pairs_mr_h1
- Formulation C: long-only on `a` leg; `b` leg is observed only.
- Slippage: 2 bps, Taker fee: 4 bps
- Size: binary_per_pair
- Risk: per-symbol cap=40% (ENFORCED in Order::new); portfolio gross cap: none declared on this lane (bug-log #95)
- Data: synthetic hourly bars, 4 independent ChaCha20Rng streams
