<!-- Paste this block under "## Backtest Scenarios" in the feature's story file -->


### Scenario: `<scenario-slug>`

- **Universe:** `BTCUSDT, ETHUSDT`
- **Period:** `2023-01-01` → `2023-12-31`
- **Granularity:** `1m`
- **Data source:** `binance-spot` (via `data/binance/<symbol>/2023/*.parquet`)
- **Fees:** `0.04%` taker, `0.02%` maker
- **Slippage model:** `half-spread` or `bps: 2`
- **Initial capital:** `100_000 USDT`
- **Position sizing:** `fixed-fraction 0.1` _or_ `kelly-clipped 0.25`
- **Risk limits:**
  - Max leverage: `3x`
  - Max drawdown stop: `-15%`
  - Per-symbol exposure cap: `40%`
- **Baseline report:** `evidence/<slug>/reports/test-<prev>-<slug>.md` _(or "none")_

**Expected outcome (analyst hypothesis):** one sentence.
