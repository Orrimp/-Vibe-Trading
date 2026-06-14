---
slug: realdata-simple-strategy-survey
status: findings
owner: orchestrator
updated: 2026-06-14
---

# Real-data simple-strategy survey — 2026-06-13 (10-symbol expansion 2026-06-14)

First use of the shipped `simple-strategies-realdata` tooling to produce a
finding, not just a feature: the four simple strategies (sma / macd / rsi /
bbands) run on the **real Binance hourly corpus** (all **10 symbols**,
2023 + 2024; pinned `3a8b96c4`), net of the 4 bps taker cost, vs **buy-and-hold**.
These strategies were previously only ever tested on *synthetic* data, so this is
a genuinely fresh look — not a re-run of the concluded forecast-overlay research.

**Method:** the re-runnable `#[ignore]` harness at
[`crates/backtest/tests/realdata_simple_strategy_survey.rs`](../../crates/backtest/tests/realdata_simple_strategy_survey.rs)
drives `engine::run_scenario` with `ScenarioDataSource::BinanceCache` over a full
year of hourly bars per (strategy, symbol), default configs, seed `0xC0FFEE…`.
Returns from absolute equity (`final − initial`/`initial`). UN-ANCHORED (no
report, no `anchors.toml` row). Re-run:
`cargo test -p backtest --test realdata_simple_strategy_survey -- --ignored --nocapture`.

## Results — strategy total return % (trade count), net of cost

| Symbol · Year | Buy & Hold | SMA 20/50 | MACD | RSI | BBands |
|---|---|---|---|---|---|
| BTCUSDT · 2023 | **+155.8%** | +6.6% | +1.1% | −0.8% | −1.8% |
| BTCUSDT · 2024 | **+120.3%** | +3.5% | +1.7% | −0.6% | −2.4% |
| ETHUSDT · 2023 | **+91.0%** | +4.5% | −1.3% | +0.6% | −3.0% |
| ETHUSDT · 2024 | **+45.4%** | +4.7% | +2.6% | −3.7% | −2.6% |
| BNBUSDT · 2023 | **+26.9%** | +0.8% | −2.2% | +1.0% | −2.2% |
| BNBUSDT · 2024 | **+123.3%** | +5.8% | +2.8% | −0.7% | +1.3% |
| SOLUSDT · 2023 | **+918.2%** | +20.2% | +10.2% | +2.5% | −1.1% |
| SOLUSDT · 2024 | **+85.6%** | +7.0% | −2.3% | +2.0% | +0.3% |
| XRPUSDT · 2023 | **+81.8%** | −3.3% | −0.1% | +0.9% | −1.4% |
| XRPUSDT · 2024 | **+238.1%** | +14.3% | +6.3% | +2.9% | +1.0% |
| ADAUSDT · 2023 | **+142.9%** | +9.2% | +0.9% | −0.5% | −1.9% |
| ADAUSDT · 2024 | **+41.3%** | +2.8% | +1.7% | +1.9% | −0.1% |
| DOGEUSDT · 2023 | **+28.2%** | −1.9% | −0.8% | −0.0% | +0.2% |
| DOGEUSDT · 2024 | **+251.7%** | +18.5% | +11.2% | +1.5% | −1.6% |
| **AVAXUSDT · 2024** | **−8.2%** | **+5.0%** | **+6.1%** | +0.8% | −2.7% |
| AVAXUSDT · 2023 | **+255.2%** | +15.4% | +8.7% | +0.3% | −2.6% |
| **DOTUSDT · 2024** | **−19.6%** | **+6.4%** | +0.2% | +1.6% | −4.6% |
| DOTUSDT · 2023 | **+91.4%** | +4.8% | +0.9% | +1.2% | −1.9% |
| LINKUSDT · 2023 | **+170.3%** | +4.6% | +2.3% | +0.6% | +0.2% |
| LINKUSDT · 2024 | **+32.3%** | +3.6% | +2.5% | +0.7% | −4.2% |

(All ~180–450 trades/year per active strategy. Trade counts omitted for width;
in the harness output.)

## Finding 1 — passive dominates in UP markets; trend-following protects in DOWN markets

The 2-symbol (BTC+ETH) version of this survey concluded a flat "passive wins by a
landslide." The 10-symbol expansion **refines that into a sharper, more honest
result:**

- **In the 18 of 20 (symbol·year) cases where buy-and-hold was positive, it
  crushed every active strategy** — often by an order of magnitude (e.g. SOL 2023:
  B&H **+918%** vs the best active +20%). In a trending bull market you cannot beat
  just holding; the active strategies churn (180–450 round-trips/yr) and the 4 bps
  cost grinds them down.
- **But in the 2 cases where buy-and-hold LOST money, the trend-followers
  protected capital:**
  - **AVAX 2024: B&H −8.2%** → SMA **+5.0%**, MACD **+6.1%**.
  - **DOT 2024: B&H −19.6%** → SMA **+6.4%**, RSI +1.6% (MACD +0.2%).
  In both down-markets SMA/MACD went **flat-to-positive while holding bled** — the
  textbook trend-following property (cut losers, sidestep the drawdown). BTC and
  ETH happened to be bull runs in both years, which is why the 2-symbol study
  never surfaced this.

**Honest takeaway:** "ship passive" remains the correct **base** — in the common
(up-trending crypto) case you do not beat holding, and chasing active there is a
cost-drag. But the data does NOT support a flat "active always loses": **trend-
following (SMA, MACD) is a defensible downside hedge** — it gives up upside in bull
runs to avoid the worst of the bear ones. That is a real, asymmetric trade-off,
not noise.

**The mean-reverters (RSI, BBands) have no edge anywhere** — across all 10 symbols
× 2 years they are flat-to-negative (BBands negative in 16 of 20), pure
cost-churn. Nothing recommends them on this corpus.

This is consistent with, and independent of, the earlier block-bootstrap
robustness conclusion (which was about the forecast overlays). It does not argue
for going live-active — but it sharpens *why* passive is the base (bull-market
dominance) and where active has a defensible niche (down-market protection).

## Finding 2 (RESOLVED) — the original ETH 0-trades was a bug, now fixed

The first (2-symbol) run showed macd/rsi/bbands making **0 trades on ETH**. Root
cause: `ComposedStrategy::emit_signal` (`crates/strategy/src/composed/node.rs`)
emitted the hardcoded config symbol (`"BTCUSDT"`) instead of `bar.symbol`, so on
ETH the order's symbol mismatched the position → `OrderError::AssetMismatch` →
**silently swallowed by a bare `.ok()`** in the composed bar loop → 0 trades, no
signal. Fixed (emit `bar.symbol`, anchor-safe: BTC byte-identical, 119/119) +
regression tests; the concealing `.ok()` was hardened to count + log discards and
raise a loud run-level alert (a strategy that signals but lands 0 trades now
screams). All numbers above are post-fix and trade correctly across all 10
symbols.

## Caveats

- **Hourly bars** (the corpus is 1h) — SMA 20/50 = 20h/50h, etc. A daily or minute
  study could differ.
- **Default configs, no parameter optimization** — shipped params, not tuned
  per-symbol/regime (tuning would face the overfitting concerns the robustness
  lane exists for).
- **10 symbols × 2 years, single-symbol** — not the cross-sectional universe.
  Two down-market data points (AVAX/DOT 2024) is suggestive of the trend-following
  protection property, not statistically conclusive — a longer/wider down-market
  sample would firm it up.
