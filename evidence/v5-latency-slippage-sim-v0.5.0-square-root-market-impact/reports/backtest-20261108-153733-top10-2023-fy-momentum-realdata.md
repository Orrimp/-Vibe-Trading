---
scenario: top10-2023-fy-momentum-realdata
seed: 0xC0FFEE
generated: 2026-11-08T15:37:33Z
wall_clock_s: 3.3
data_source: real (Binance Vision via data/binance/, v3.0.0-volatility-rebaseline)
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
baseline_report: n/a
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: top10_momentum_h1
kind: cross_sectional_momentum
content_hash: 52ba625567f18c9b4d3eeb2f104520bcd9ba4c4709eb04da56983a41d098ac2f
source: config/strategies/top10_momentum_h1.toml
signal: vol_adjusted_log_return(lookback=60)
---

# Backtest Report — top10-2023-fy-momentum-realdata

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2023-fy-momentum-realdata               |
| Universe             | 10 symbols      |
| Start year           | 2023                  |
| Bars (total)         | 87590                   |
| Initial capital      | $100000.00 USDT            |
| Final equity         | $11344.04 USDT           |
| Total return         | -88.66%                     |
| Buy and hold (equal-wt) | +196.10% ($296102.61 USDT, 10 legs, $40.00 fees)                 |
| vs buy and hold        | -284.76 pp                    |
| Max drawdown         | 89.18%                  |
| Trades               | 6203                      |
| Buys                 | 3103                        |
| Sells                | 3100                       |
| Total fees           | $8339.938318 USDT               |
| Seed                 | 0xC0FFEE                    |
| Data source          | real (Binance Vision via data/binance/, v3.0.0-volatility-rebaseline)                 |

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

- v1 cross-sectional momentum: top10_momentum_h1
- Slippage: 2 bps, Taker fee: 4 bps
- Size: equal_weight, k_long=3
- Risk: per-symbol cap=40% (ENFORCED in Order::new); portfolio gross cap: none declared on this lane (bug-log #95)
- Data: synthetic hourly bars, 10 independent ChaCha20Rng streams
