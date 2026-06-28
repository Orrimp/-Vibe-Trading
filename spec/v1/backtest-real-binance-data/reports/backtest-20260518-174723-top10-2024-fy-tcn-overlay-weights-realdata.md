---
scenario: top10-2024-fy-tcn-overlay-weights-realdata
seed: 0xC0FFEE
generated: 2026-05-18T17:47:23Z
wall_clock_s: 38.1
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
data_source: real (Binance Vision via data/binance/, v2.6.0-realdata)
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

# Backtest Report — top10-2024-fy-tcn-overlay-weights-realdata

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2024-fy-tcn-overlay-weights-realdata               |
| Universe             | 10 symbols      |
| Start year           | 2024                  |
| Bars (total)         | 87840                   |
| Initial capital      | $100000.00 USDT            |
| Final equity         | $105214.25 USDT           |
| Total return         | 5.21%                     |
| Max drawdown         | 78.82%                  |
| Trades               | 5917                      |
| Buys                 | 2960                        |
| Sells                | 2957                       |
| Total fees           | $14224.582735 USDT               |
| Seed                 | 0xC0FFEE                    |
| Data source          | real (Binance Vision via data/binance/, v2.6.0-realdata)                 |

## TCN Overlay Modulation

| Metric               | Value                         |
|----------------------|-------------------------------|
| Passed through       | 5800              |
| Dampened to Hold     | 0                    |
| Warming-up (no overlay) | 117                  |
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

## Data source

| Field                | Value                                |
|----------------------|--------------------------------------|
| Source               | Binance Vision via data/binance/     |
| Revision SHA         | 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7 |
| Universe size        | 10 symbols                           |
| Bar interval         | 1h                                   |
| Span (UTC, half-open) | 2024-01-01T00:00:00Z .. 2025-01-01T00:00:00Z |
| Expected bars        | 87840                                |
| Loaded bars          | 87840 (100.00% present)              |

## Notes

- v2.5 TCN overlay momentum: tcn_overlay_momentum_weights/tcn_overlay_momentum
- Forecaster: real TCN weights (tcn-bs2, v2.5.0-tcn-weights)
- Slippage: 2 bps, Taker fee: 4 bps
- Size: equal_weight, exposure_cap=50%, k_long=3
- Risk: per-symbol cap=40%, portfolio cap=50%
- Data: real Binance hourly OHLCV, see ## Data source section above.
