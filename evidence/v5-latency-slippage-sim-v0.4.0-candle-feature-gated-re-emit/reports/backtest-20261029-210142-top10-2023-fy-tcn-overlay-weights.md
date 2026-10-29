---
scenario: top10-2023-fy-tcn-overlay-weights
seed: 0xC0FFEE
generated: 2026-10-29T21:01:42Z
wall_clock_s: 8.9
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
| Final equity         | $93776.80 USDT           |
| Total return         | -6.22%                     |
| Max drawdown         | 12.63%                  |
| Trades               | 1247                      |
| Buys                 | 625                        |
| Sells                | 622                       |
| Total fees           | $4753.857141 USDT               |
| Seed                 | 0xC0FFEE                    |
| Data source          | synthetic (seeded RNG, v2.5 tcn-overlay-weights)                 |

## TCN Overlay Modulation

| Metric               | Value                         |
|----------------------|-------------------------------|
| Passed through       | 1142              |
| Modified by overlay  | 0                    |
| Warming-up (no overlay) | 105                  |
| Modification rate    | 0.00%             |
| Gross-cap refusals   | 0              |

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

## Notes

- v2.5 TCN overlay momentum: tcn_overlay_momentum_weights/tcn_overlay_momentum
- Forecaster: real TCN weights (tcn-bs1, v2.5.0-tcn-weights)
- Slippage: 2 bps, Taker fee: 4 bps
- Size: equal_weight, k_long=3
- Risk: per-symbol cap=40% (ENFORCED in Order::new); NO portfolio gross cap on this lane (bug-log #95)
- Data: synthetic hourly bars, 10 independent ChaCha20Rng streams
