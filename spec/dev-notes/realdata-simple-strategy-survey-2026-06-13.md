---
slug: realdata-simple-strategy-survey
status: findings
owner: orchestrator
updated: 2026-06-13
---

# Real-data simple-strategy survey — 2026-06-13

First use of the just-shipped `simple-strategies-realdata` tooling: the four
simple strategies (sma / macd / rsi / bbands) run on the **real Binance hourly
corpus** (BTC + ETH, 2023 + 2024; pinned `3a8b96c4`), net of the 4 bps taker
cost, vs **buy-and-hold**. These strategies were previously only ever tested on
*synthetic* data, so this is a genuinely fresh look — not a re-run of the
concluded forecast-overlay / robustness research.

**Method:** the re-runnable `#[ignore]` harness at
[`crates/backtest/tests/realdata_simple_strategy_survey.rs`](../../crates/backtest/tests/realdata_simple_strategy_survey.rs)
drives `engine::run_scenario` with `ScenarioDataSource::BinanceCache` over a full
year of hourly bars per (strategy, symbol), default configs, seed `0xC0FFEE…`.
Returns are computed from absolute equity (`final − initial`/`initial`).
UN-ANCHORED (no report, no `anchors.toml` row). Re-run:
`cargo test -p backtest --test realdata_simple_strategy_survey -- --ignored --nocapture`.

## Results — strategy total return % (trade count), net of cost

| Symbol · Year | Buy & Hold | SMA 20/50 | MACD | RSI | BBands |
|---|---|---|---|---|---|
| BTCUSDT · 2023 (8759h) | **+155.8%** | +6.6% (229t) | +1.1% (400t) | −0.8% (224t) | −1.8% (388t) |
| BTCUSDT · 2024 (8784h) | **+120.3%** | +3.5% (213t) | +1.7% (408t) | −0.6% (242t) | −2.4% (426t) |
| ETHUSDT · 2023 (8759h) | **+91.0%** | +4.5% (204t) | +0.0% (0t) | +0.0% (0t) | +0.0% (0t) |
| ETHUSDT · 2024 (8784h) | **+45.4%** | +4.7% (196t) | +0.0% (0t) | +0.0% (0t) | +0.0% (0t) |

## Finding 1 (headline) — passive wins by a landslide, on REAL data

Across both symbols and both years, **no active simple strategy comes remotely
close to buy-and-hold.** B&H returned **+45 % to +156 %**; the best active
strategy (SMA) returned **+3.5 % to +6.6 %** — an order of magnitude behind — and
the mean-reverters (RSI, BBands) **lost money net of fees** (−0.6 % to −2.4 %).
The strategies churn (200–400 round-trips/year) and the 4 bps cost grinds them
down while they fail to capture the strong directional trend that B&H rode for
free.

This is the **passive-baseline thesis confirmed on real data for the simple
strategies** — the channel the operator wanted ("check trading strategies with
real data") delivering a clear answer. It is consistent with, and independent of,
the earlier block-bootstrap robustness conclusion (which was about the forecast
overlays). Nothing here argues for going active.

## Finding 2 (RESOLVED 2026-06-14) — macd/rsi/bbands 0 trades on ETH was a bug

**Root cause (confirmed, fixed):** `ComposedStrategy::emit_signal` in
`crates/strategy/src/composed/node.rs:1199` emitted `symbol: self.symbol.clone()`
where `self.symbol` is loaded from the TOML config field `symbol = "BTCUSDT"`.
When running on ETHUSDT bars, `sig.symbol = "BTCUSDT"` but `position.symbol =
"ETHUSDT"` (initialised from `input.symbol = cfg.pair.1`). `Order::new`
(crates/core/src/order.rs:142) returned `OrderError::AssetMismatch`; the
`.ok()` call at sma_composed_run.rs:478 silently discarded every order → 0 trades.
`v0.sma` was unaffected because `SmaCrossover::on_bar` emits `bar.symbol` directly.

**Fix (anchor-safe):** Changed `emit_signal` to emit `bar.symbol.clone()` instead
of `self.symbol.clone()`. For BTC bars `bar.symbol == self.symbol == "BTCUSDT"`,
so all 119 anchored BTC report body-SHAs remain byte-identical (verified:
`scripts/verify_anchors.sh` → 119/119 PASS after fix).

**Regression tests added:** `crates/backtest/tests/binance_cache_dispatch.rs`:
- `composed_strategy_macd_trades_on_ethusdt_not_zero`
- `composed_strategy_rsi_trades_on_ethusdt_not_zero`
- `composed_strategy_bbands_trades_on_ethusdt_not_zero`

**Updated survey table (re-run 2026-06-14 after fix):**

| Symbol · Year | Buy & Hold | SMA 20/50 | MACD | RSI | BBands |
|---|---|---|---|---|---|
| BTCUSDT · 2023 (8759h) | **+155.8%** | +6.6% (229t) | +1.1% (400t) | −0.8% (224t) | −1.8% (388t) |
| BTCUSDT · 2024 (8784h) | **+120.3%** | +3.5% (213t) | +1.7% (408t) | −0.6% (242t) | −2.4% (426t) |
| ETHUSDT · 2023 (8759h) | **+91.0%** | +4.5% (204t) | −1.3% (414t) | +0.6% (234t) | −3.0% (446t) |
| ETHUSDT · 2024 (8784h) | **+45.4%** | +4.7% (196t) | +2.6% (374t) | −3.7% (230t) | −2.6% (432t) |

BTC cells are byte-identical to the original survey (bug did not affect BTC).
ETH now shows 200–450 trades/year for all three strategies.

**Finding 1 is unchanged** and in fact reinforced: the corrected ETH numbers
show the same pattern — MACD/RSI/BBands underperform buy-and-hold (−3.7% to
+2.6% vs +45–91% B&H), with RSI and BBands losing money net of fees. The
original ETH cells (0 trades, equity flat) were uninformative due to the bug.

## Caveats

- Hourly bars (the Binance corpus is 1h) — so SMA 20/50 = 20h/50h, MACD/RSI/etc.
  on hourly periods. Different from a daily or minute study.
- **Default configs, no parameter optimization** — these are the shipped
  strategy params, not tuned per-symbol/regime. A tuned strategy could differ
  (but would then face the overfitting concerns the robustness lane exists for).
- 2 symbols × 2 years, single-symbol (not the 10-symbol cross-sectional universe,
  which these strategies don't address).
