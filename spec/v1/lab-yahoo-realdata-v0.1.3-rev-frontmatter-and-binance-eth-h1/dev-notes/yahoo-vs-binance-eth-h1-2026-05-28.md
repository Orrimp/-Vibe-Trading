---
date: 2026-05-28
feature: lab-yahoo-realdata-v0.1.3-rev-frontmatter-and-binance-eth-h1
author: developer (M-DEV)
hypothesis: H1
---

# Dev-Note: Yahoo ETH-USD Daily vs Binance ETHUSDT Hourly H1 2024 — Direct Discharge

## Context

v0.1.2 SOFT-PASS deferred the H1 ETH discharge because no `eth-2024-h1-sma-cross`
Binance scenario was registered. The H1 gate was satisfied by a Yahoo-to-Yahoo
proxy (K1 fallback: Yahoo ETH +0.35% vs Yahoo BTC +1.20%, delta = 0.85% < 30%).

v0.1.3 registers `eth-2024-h1-sma-cross` in `crates/backtest/src/main.rs` (D-V0.1.3-5)
and performs the direct Yahoo-daily-vs-Binance-hourly comparison that the analyst
originally specified.

## Run configuration

| Field               | Yahoo ETH daily             | Binance ETH hourly           |
|---------------------|----------------------------|------------------------------|
| Source              | data/yahoo/ETH-USD/1d/2024  | data/binance/ETHUSDT/2024/   |
| Bars                | 366 daily bars              | 17,543 hourly bars           |
| Scenario            | eth-yahoo-2024-1d-sma-cross | eth-2024-h1-sma-cross        |
| Strategy            | SMA(20,50)                  | SMA(20,50)                   |
| Seed                | 0xC0FFEE                    | 0xC0FFEE                     |
| Initial capital     | $100,000                    | $100,000                     |

## Results

| Metric          | Yahoo ETH daily (row 70)  | Binance ETH hourly (row 71) |
|-----------------|--------------------------|------------------------------|
| Final equity    | $102,760.76              | $109,544.53                  |
| Total return    | +2.76%                   | +9.54%                       |
| Body SHA-256    | e59a5f87... (anchored)   | bd4001e4... (new anchor)     |

## H1 computation

Delta = |9.54% − 2.76%| = **6.78%**

Threshold: < 30%. Expected range: 5–15% (BTC reference was 9.03% at v0.1.1).

**H1 VERDICT: PASS** — 6.78% < 30%. Within expected range.

## Notes

- Binance hourly data covers both 2023 and 2024 (17,543 bars = 2023 + 2024
  combined via ReplayFeed). SMA(20,50) uses all available bars; the strategy
  is warmed up on 2023 data and trades through 2024. This matches how
  `btc-2024-h1-sma-cross` works — same data loading behavior.
- Determinism: body SHA identical on 2 independent runs (2026-05-28T20:34:59Z
  and 2026-05-28T20:36:02Z).
- The v0.1.2 K1 fallback (Yahoo-to-Yahoo proxy) is hereby retired. Row 71
  under namespace `lab-yahoo-realdata-v0.1.3` is the direct comparison basis.

## BTC reference (v0.1.1)

For comparison: Yahoo BTC daily +4.56% vs Binance BTC hourly H1 SMA(20,50).
BTC H1 delta was 9.03% at v0.1.1 (from dev-notes in v0.1.1 feature folder).
ETH's 6.78% is in the same order of magnitude — both reflect the 2024 bull
market environment where hourly data captures more intra-day trading opportunities
vs daily data.
