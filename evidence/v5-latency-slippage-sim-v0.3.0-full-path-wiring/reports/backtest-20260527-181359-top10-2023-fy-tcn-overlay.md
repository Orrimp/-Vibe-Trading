---
scenario: top10-2023-fy-tcn-overlay
seed: 0xC0FFEE
generated: 2026-05-27T18:13:59Z
wall_clock_s: 0.9
data_revision_sha: n/a
data_source: synthetic (seeded RNG, v2.5 tcn-overlay)
baseline_report: n/a
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: tcn_overlay_momentum/tcn_overlay_momentum
kind: tcn_overlay_momentum
content_hash: n/a
source: config/strategies/tcn_overlay_momentum.toml
signal: tcn_overlay(base=vol_adjusted_log_return,confidence_threshold=0.6)
---

# Backtest Report — top10-2023-fy-tcn-overlay

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2023-fy-tcn-overlay               |
| Universe             | 10 symbols      |
| Start year           | 2023                  |
| Bars (total)         | 22080                   |
| Initial capital      | $100000.00 USDT            |
| Final equity         | $28347.99 USDT           |
| Total return         | -71.65%                     |
| Max drawdown         | 87.63%                  |
| Trades               | 1224                      |
| Buys                 | 614                        |
| Sells                | 610                       |
| Total fees           | $2612.134515 USDT               |
| Seed                 | 0xC0FFEE                    |
| Data source          | synthetic (seeded RNG, v2.5 tcn-overlay)                 |

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

- v2.5 TCN overlay momentum: tcn_overlay_momentum/tcn_overlay_momentum
- Forecaster: passthrough (no-candle mode — degrades to v1 momentum)
- Slippage: 2 bps, Taker fee: 4 bps
- Size: equal_weight, exposure_cap=50%, k_long=3
- Risk: per-symbol cap=40%, portfolio cap=50%
- Data: synthetic hourly bars, 10 independent ChaCha20Rng streams
