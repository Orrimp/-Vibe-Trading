---
scenario: top10-2023-fy-patchtst-overlay-realdata
seed: 0xC0FFEE
generated: 2026-05-27T06:56:43Z
wall_clock_s: 36.9
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
data_source: real (Binance Vision via data/binance/, v2.5a.0-patchtst)
baseline_report: n/a
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: patchtst_overlay_momentum_weights/patchtst_overlay_momentum
kind: tcn_overlay_momentum
content_hash: n/a
source: config/strategies/tcn_overlay_momentum.toml
signal: tcn_overlay(base=vol_adjusted_log_return,confidence_threshold=0.6)
---

# Backtest Report — top10-2023-fy-patchtst-overlay-realdata

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2023-fy-patchtst-overlay-realdata               |
| Universe             | 10 symbols      |
| Start year           | 2023                  |
| Bars (total)         | 87590                   |
| Initial capital      | $100000.00 USDT            |
| Final equity         | $131125.07 USDT           |
| Total return         | 31.13%                     |
| Max drawdown         | 77.97%                  |
| Trades               | 3187                      |
| Buys                 | 1595                        |
| Sells                | 1592                       |
| Total fees           | $11157.699068 USDT               |
| Seed                 | 0xC0FFEE                    |
| Data source          | real (Binance Vision via data/binance/, v2.5a.0-patchtst)                 |

## TCN Overlay Modulation

| Metric               | Value                         |
|----------------------|-------------------------------|
| Passed through       | 4281              |
| Dampened to Hold     | 1745                    |
| Warming-up (no overlay) | 177                  |
| Dampen rate          | 28.96%             |

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
| Span (UTC, half-open) | 2023-01-01T00:00:00Z .. 2024-01-01T00:00:00Z |
| Expected bars        | 87600                                |
| Loaded bars          | 87590 (99.99% present)               |

## Notes

- v2.5 TCN overlay momentum: patchtst_overlay_momentum_weights/patchtst_overlay_momentum
- Forecaster: real PatchTST weights (patchtst-bs1, v2.5a.0-patchtst)
- Slippage: 2 bps, Taker fee: 4 bps
- Size: equal_weight, exposure_cap=50%, k_long=3
- Risk: per-symbol cap=40%, portfolio cap=50%
- Data: real Binance hourly OHLCV, see ## Data source section above.
