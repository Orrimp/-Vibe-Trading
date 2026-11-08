---
scenario: top10-2023-fy-vol-target-overlay-realdata
seed: 0xC0FFEE
generated: 2026-11-08T15:36:46Z
wall_clock_s: 4.0
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
data_source: real (Binance Vision via data/binance/, v3.0.0-volatility)
baseline_report: n/a
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: garch_vol_target_overlay_momentum/vol_target_overlay_momentum
kind: tcn_overlay_momentum
content_hash: n/a
source: config/strategies/tcn_overlay_momentum.toml
signal: tcn_overlay(base=vol_adjusted_log_return,confidence_threshold=0.6)
---

# Backtest Report — top10-2023-fy-vol-target-overlay-realdata

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2023-fy-vol-target-overlay-realdata               |
| Universe             | 10 symbols      |
| Start year           | 2023                  |
| Bars (total)         | 87590                   |
| Initial capital      | $100000.00 USDT            |
| Final equity         | $43998.36 USDT           |
| Total return         | -56.00%                     |
| Buy and hold (equal-wt) | +196.10% ($296102.61 USDT, 10 legs, $40.00 fees)                 |
| vs buy and hold        | -252.10 pp                    |
| Max drawdown         | 62.60%                  |
| Trades               | 3264                      |
| Buys                 | 1633                        |
| Sells                | 1631                       |
| Total fees           | $15682.815379 USDT               |
| Seed                 | 0xC0FFEE                    |
| Data source          | real (Binance Vision via data/binance/, v3.0.0-volatility)                 |

## TCN Overlay Modulation

| Metric               | Value                         |
|----------------------|-------------------------------|
| Passed through       | 0              |
| Modified by overlay  | 6203                    |
| Warming-up (no overlay) | 0                  |
| Modification rate    | 100.00%             |
| Gross-cap refusals   | 1470              |

Metric definition (bug-log #146). "Modified by overlay" is ONE field
(`dampened_signals`) written by every overlay that uses this renderer, and it
means different things to them: for the TCN/PatchTST overlays a modified signal was
DAMPENED TO HOLD, while for the vol-targeting overlay it means the scale was != 1.0,
i.e. the position was RESIZED, not suppressed. The rows above were labelled
"Dampened to Hold" / "Dampen rate" until 2026-10-17, which was true for the TCN
consumers and false for the vol-target one -- and the two readings are opposite. A
vol-target body reading a 100.00%% modification rate means every bar was RESIZED,
which is the expected behaviour of a continuous vol-targeting overlay, NOT an inert
arm. The trade count in the table above is the check: an arm that had every signal
suppressed could not have placed those fills.

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

- v2.5 TCN overlay momentum: garch_vol_target_overlay_momentum/vol_target_overlay_momentum
- Forecaster: GARCH(1,1) vol-targeting overlay (garch-bs1, v3.0.0-volatility, 10 symbols)
- Slippage: 2 bps, Taker fee: 4 bps
- Size: equal_weight, k_long=3
- Risk: per-symbol cap=40% (ENFORCED in Order::new); portfolio gross cap: 50% — ENFORCED as a per-order entry gate on RESULTING exposure (bug-log #95, #71); see the Gross-cap refusals row
- Data: real Binance hourly OHLCV, see ## Data source section above.
