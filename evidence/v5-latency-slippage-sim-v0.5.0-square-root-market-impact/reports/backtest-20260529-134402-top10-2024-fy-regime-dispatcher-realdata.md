---
scenario: top10-2024-fy-regime-dispatcher-realdata
seed: 0xC0FFEE
generated: 2026-05-29T13:44:02Z
wall_clock_s: 309.4
data_source: real (Binance Vision via data/binance/, v3.0.0-regime)
data_revision_sha: 3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7
baseline_report: n/a
ledger_imbalance_total: 0
llm_spend_usd: 0.00
strategy:
id: regime_dispatcher_momentum/regime_dispatcher_momentum
kind: regime_dispatcher_momentum
source: config/strategies/top10_momentum_h1.toml
---

# Backtest Report — top10-2024-fy-regime-dispatcher-realdata

## Summary

| Metric               | Value                         |
|----------------------|-------------------------------|
| Scenario             | top10-2024-fy-regime-dispatcher-realdata               |
| Universe             | 10 symbols      |
| Start year           | 2024                  |
| Bars (total)         | 87840                   |
| Initial capital      | $100000.00 USDT            |
| Final equity         | $22463.08 USDT           |
| Total return         | -77.54%                     |
| Max drawdown         | 80.37%                  |
| Trades               | 6243                      |
| Buys                 | 3123                        |
| Sells                | 3120                       |
| Total fees           | $11390.719601 USDT               |
| Suppress rate        | 13.45%           |
| Suppressed bars      | 11816             |
| Momentum bars        | 75524               |
| Warmup bars          | 500                 |
| Seed                 | 0xC0FFEE                    |
| Data source          | real (Binance Vision via data/binance/, v3.0.0-regime)                 |

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

## Dispatcher

| Field                | Value                                            |
|----------------------|--------------------------------------------------|
| Classifier           | RegimeDispatcher(MarkovSwitching 4-state, confidence_gate=0.70, v3.0.0-regime, 10 symbols)                               |
| Routing              | Bull/Bear → MomentumStrategy; Volatile/Calm → CashHoldStrategy |
| Confidence gate      | max_p >= 0.70 (ADR-0049 § D6)                   |
| Cash-fallback        | SUPPRESSION-NOT-LIQUIDATION (ADR-0049 § D3)     |

## Notes

- v3.0.0-regime dispatcher: regime_dispatcher_momentum/regime_dispatcher_momentum
- Slippage: 2 bps, Taker fee: 4 bps
- Size: equal_weight fraction=10%, exposure_cap=50%
- Risk: per-symbol cap=40%, portfolio cap=50%
- Data: real Binance hourly bars, 10 symbols
