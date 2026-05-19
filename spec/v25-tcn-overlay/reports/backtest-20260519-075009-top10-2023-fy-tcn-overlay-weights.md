---
scenario: top10-2023-fy-tcn-overlay-weights
seed: 0xC0FFEE
generated: 2026-05-19T07:50:09Z
wall_clock_s: 226.6
data_revision_sha: n/a
data_source: synthetic (seeded RNG, v2.5 tcn-overlay-weights)
baseline_report: n/a
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: tcn_overlay_momentum_weights/tcn_overlay_momentum
kind: tcn_overlay_momentum
content_hash: n/a
source: config/strategies/tcn_overlay_momentum.toml
signal: tcn_overlay(base=vol_adjusted_log_return,confidence_threshold=0.6)
---

# Backtest Report — top10-2023-fy-tcn-overlay-weights

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2023-fy-tcn-overlay-weights               |
| Universe             | 10 symbols      |
| Start year           | 2023                  |
| Bars (total)         | 22080                   |
| Initial capital      | $100000.00 USDT            |
| Final equity         | $30235.58 USDT           |
| Total return         | -69.76%                     |
| Max drawdown         | 87.48%                  |
| Trades               | 1224                      |
| Buys                 | 614                        |
| Sells                | 610                       |
| Total fees           | $2681.670646 USDT               |
| Seed                 | 0xC0FFEE                    |
| Data source          | synthetic (seeded RNG, v2.5 tcn-overlay-weights)                 |

## TCN Overlay Modulation

| Metric               | Value                         |
|----------------------|-------------------------------|
| Passed through       | 1142              |
| Dampened to Hold     | 0                    |
| Warming-up (no overlay) | 105                  |
| Dampen rate          | 0.00%             |

## Universe

- ADAUSDT
- AVAXUSDT
- BNBUSDT
- BTCUSDT
- DOGEUSDT
- DOTUSDT
- ETHUSDT
- LINKUSDT
- SOLUSDT
- XRPUSDT

## Notes

- v2.5 TCN overlay momentum: tcn_overlay_momentum_weights/tcn_overlay_momentum
- Forecaster: real TCN weights (tcn-bs1, v2.5.0-tcn-weights)
- Slippage: 2 bps, Taker fee: 4 bps
- Size: equal_weight, exposure_cap=50%, k_long=3
- Risk: per-symbol cap=40%, portfolio cap=50%
- Data: synthetic hourly bars, 10 independent ChaCha20Rng streams
