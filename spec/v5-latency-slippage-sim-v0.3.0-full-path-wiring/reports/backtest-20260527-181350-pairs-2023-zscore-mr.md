---
scenario: pairs-2023-zscore-mr
seed: 0xC0FFEE
generated: 2026-05-27T18:13:50Z
wall_clock_s: 0.4
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

# Backtest Report — pairs-2023-zscore-mr

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | pairs-2023-zscore-mr               |
| Universe             | 4 symbols      |
| Start year           | 2023                  |
| Bars (total)         | 35040                   |
| Initial capital      | $100000.00 USDT            |
| Final equity         | $-62693.12 USDT           |
| Total return         | -162.69%                     |
| Max drawdown         | 1828.87%                  |
| Trades               | 16                      |
| Buys                 | 8                        |
| Sells                | 8                       |
| Total fees           | $5434.010928 USDT               |
| Ledger imbalances    | 0                             |
| Seed                 | 0xC0FFEE                    |
| Data source          | synthetic (seeded RNG, v1.5a multi-symbol)                 |

## Per-Pair Summary (R8.5)

| Pair                 | Trades |
|----------------------|--------|
| (BNBUSDT, BTCUSDT) | 6 |
| (BTCUSDT, ETHUSDT) | 2 |
| (ETHUSDT, SOLUSDT) | 8 |

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
- Size: binary_per_pair, exposure_cap_per_pair=25%
- Risk: per-symbol cap=40%, portfolio cap=75% (v1.5a)
- Data: synthetic hourly bars, 4 independent ChaCha20Rng streams
