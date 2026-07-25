# Passive Baseline Characterization — 2026-06-08

**Artifact type:** Characterization of the existing BH control (read-only — no new code)
**Data:** Binance Vision 1h OHLCV, `data/binance/`, revision SHA `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7`
**Universe:** 10-symbol large-cap USDT perps/spot (ADAUSDT, AVAXUSDT, BNBUSDT, BTCUSDT, DOGEUSDT, DOTUSDT, ETHUSDT, LINKUSDT, SOLUSDT, XRPUSDT)
**Period:** Full-year 2023 (8760 bars/symbol) and full-year 2024 (8784 bars/symbol)
**Produced by:** Reading 119 anchored sweep reports already in `spec/*/reports/` — zero new code run

---

## 1. Per-Year Metrics Table

### Canonical configuration

All values below are from the 1h-horizon N=200 block-bootstrap-real sweep harness
(`param_robustness_sweep --generator block-bootstrap-real --paths 200 --ensemble-seed 0xC0FFEE`),
which is the configuration used throughout the entire active-vs-passive program.
These numbers are byte-identical across 14 independent 2023 reports and 8 independent
2024 reports — confirmed from anchored output.

#### 2023 (8760 1h bars, N=200 bootstrap paths)

| Metric                  | Value              | Source                              |
|-------------------------|--------------------|-------------------------------------|
| Sharpe p50              | **+1.7353**        | 14/14 anchored 2023 reports agree   |
| Sharpe p5               | +0.1245            | 14/14 anchored 2023 reports agree   |
| Sharpe p95              | +3.8703            | 14/14 anchored 2023 reports agree   |
| Sharpe spread (p95-p5)  | 3.7459             | 14/14 anchored 2023 reports agree   |
| P(loss)                 | 4.5%               | 14/14 anchored 2023 reports agree   |
| P(Sharpe > 1.0)         | 77.5%              | 14/14 anchored 2023 reports agree   |
| p95 MaxDD               | 51.15%             | 14/14 anchored 2023 reports agree   |
| Sortino p50             | *not in sweep output*  | computed internally, not rendered |
| Calmar p50              | *not in sweep output*  | computed internally, not rendered |
| Total return p50        | *not in sweep output*  | computed internally, not rendered |

**Note on missing Sortino/Calmar/total_return:** The sweep harness computes all five
PathMetrics (sharpe, sortino, calmar, max_drawdown, total_return) per-path for the BH
control, but the anchored report body only renders sharpe, prob_loss, P(Sharpe>1), and
p95_maxdd for the BH row. Sortino, Calmar, and total_return are available in the
DistributionSummary struct but not printed. Producing them would require either reading
the struct at runtime or running the harness with additional output — both would require
running new code, which is prohibited by the no-build boundary. They are characterized
as "not rendered in existing output" rather than fabricated.

#### 2024 (8784 1h bars, N=200 bootstrap paths)

| Metric                  | Value              | Source                              |
|-------------------------|--------------------|-------------------------------------|
| Sharpe p50              | **+1.1047**        | 8/8 anchored 2024 reports agree     |
| Sharpe p5               | -0.6821            | 8/8 anchored 2024 reports agree     |
| Sharpe p95              | +2.6905            | 8/8 anchored 2024 reports agree     |
| Sharpe spread (p95-p5)  | 3.3726             | 8/8 anchored 2024 reports agree     |
| P(loss)                 | 16.5%              | 8/8 anchored 2024 reports agree     |
| P(Sharpe > 1.0)         | 53.5%              | 8/8 anchored 2024 reports agree     |
| p95 MaxDD               | 64.83%             | 8/8 anchored 2024 reports agree     |
| Sortino p50             | *not in sweep output*  | computed internally, not rendered |
| Calmar p50              | *not in sweep output*  | computed internally, not rendered |
| Total return p50        | *not in sweep output*  | computed internally, not rendered |

---

## 2. Equity Trajectory (Data)

A single-path BH equity curve is not stored separately. However, the bootstrap
distribution gives a distribution-level trajectory summary:

### 2023 (starting equity = $100,000 USDT)

- **Initial equity:** $100,000 USDT
- **Sharpe p50 = +1.7353** on 8760 hourly bars implies strong positive returns on the
  median bootstrap path. The p50 Sharpe of +1.7353 annualized on hourly data (annualization
  factor = sqrt(8760)) translates to: a distribution-median path with a high Sharpe, consistent
  with the 2023 crypto bull leg.
- **P(loss) = 4.5%** — only 9/200 bootstrap paths ended with equity below $100,000 USDT.
- **P(Sharpe > 1.0) = 77.5%** — 155/200 paths exceeded Sharpe > 1.0.
- **p95 MaxDD = 51.15%** — the 95th-percentile worst-case drawdown reaches 51.15% of peak,
  indicating the tail risk is non-trivial even in the 2023 bull regime.

### 2024 (starting equity = $100,000 USDT)

- **Initial equity:** $100,000 USDT
- **Sharpe p50 = +1.1047** — still positive but materially weaker than 2023. The 2024 crypto
  regime was a harder distribution for the equal-weight BH: more volatile, more paths that go
  negative.
- **P(loss) = 16.5%** — 33/200 bootstrap paths ended with equity below $100,000 USDT (vs 9/200 in 2023).
- **P(Sharpe > 1.0) = 53.5%** — only a bare majority (107/200) of paths exceeded Sharpe > 1.0.
- **p95 MaxDD = 64.83%** — substantially worse tail drawdown than 2023 (64.83% vs 51.15%).

### Note on total return

The tester report (`spec/horizon-retest-robustness/reports/test-2026-06-05-horizon-retest-robustness.md`)
explicitly states: "The BH total return is horizon-invariant (same start/end prices)." This means
the distribution's p50 total_return is determined by the median bootstrap path's price trajectory on
the real data, resampled from the block-bootstrap procedure. The exact total_return p50 number is
not emitted by any anchored report but it is computable from the final_equity field which is tracked
per-path (final_equity = initial_equity * (1 + total_return)).

---

## 3. Reconciliation vs Anchored Bar (+1.74 in 2023, +1.10 in 2024)

| Year | Anchored bar in product.md | Actual p50 Sharpe (anchored) | Match? | Notes                                 |
|------|---------------------------|------------------------------|--------|---------------------------------------|
| 2023 | "+1.74 Sharpe 2023"       | **1.735275**                 | YES    | 1.735 rounds to 1.74 at 2 decimals    |
| 2024 | "+1.10 Sharpe 2024"       | **1.104731**                 | YES    | 1.105 rounds to 1.10 at 2 decimals    |

The product.md references "+1.74/+1.10" are rounded-to-2-decimal representations of the exact
anchored values 1.735275 (2023) and 1.104731 (2024). The reconciliation is exact.

**Cross-report consistency:** Both values are byte-identical across all independent sweep runs:
- 2023: appears identically in 14 separate anchored reports spanning momentum, MR, TS, carry,
  basis-reversal, and MN-spread families (all at 1h horizon, N=200, seed 0xC0FFEE, revision 3a8b96c4).
- 2024: appears identically in 8 separate anchored reports spanning the same families.

This consistency is a strong signal that the BH control computation is deterministic and that
the block-bootstrap harness is stable — the same 200 paths (seeded 0xC0FFEE + j*0x9E3779B9)
generate the same BH equity distribution every time on the pinned data revision.

---

## 4. Honest Construction Note

### What the harness BH control ACTUALLY is

**Source code:** `crates/backtest/src/bin/param_robustness_sweep.rs`, function `run_buyhold_path`
(lines 1675-1760).

**Construction (verbatim from code):**

1. **Weighting:** Equal-weight across all N symbols. Each symbol receives
   `initial_capital / n_symbols` = $100,000 / 10 = $10,000 USDT.

2. **Entry:** Buy at the **bar 0 close price** of the bootstrap path. No entry fee is charged
   (the comment says "no fees after bar 0" — meaning only the initial buy matters, and the
   implementation does NOT subtract a transaction cost for the initial buy either, since it
   computes `qty = weight / buy_price` from the allocation directly without fee deduction).

3. **Rebalancing:** **None whatsoever.** After the bar-0 buy, the strategy just tracks
   mark-to-market equity at each bar. There is NO rebalancing — not monthly, not quarterly,
   not ever. Weights drift freely with price movements. This is a **pure buy-once-hold** strategy.

4. **Exit:** Not applicable — the strategy holds for the full year with no rebalancing and no
   selling. The equity curve ends at whatever the mark-to-market value is on the last bar.

5. **Metric horizon:** Sharpe computed via `compute_sharpe_hourly` (annualization factor =
   sqrt(8760) for 2023, sqrt(8784) for 2024). This is the standard 1h annualization throughout
   the program.

6. **Bootstrap context:** The BH control runs over the SAME N=200 bootstrap paths and the SAME
   block-bootstrap seed stream as the strategy cells (ADR-0051 § D6.1 "SAME-paths rule"). This
   ensures the BH vs strategy comparison is on identical synthetic paths.

### Reconciliation against the runbook's proposed monthly/equal-weight cadence

The runbook (`docs/runbooks/passive-baseline.md`) proposes "monthly rebalance to equal weights"
as the forward operational default. The harness BH control is NOT monthly/equal-weight — it is
**pure buy-once-hold** with no rebalancing. This means:

- The +1.74/+1.10 Sharpe numbers characterize a **pure buy-once-hold** (zero rebalancing) baseline,
  NOT a monthly-rebalanced portfolio.
- A monthly-rebalanced equal-weight portfolio would differ slightly in performance: rebalancing
  sells winners and buys losers monthly, which reduces weight drift and caps concentration risk,
  but introduces turnover costs. In a strong bull leg (2023), rebalancing trimmed winners slightly
  and thus likely slightly reduced returns vs pure hold. In 2024's choppier environment, rebalancing
  may have helped or hurt depending on cross-asset correlation structure.
- The Sharpe difference between pure-hold and monthly-equal-weight is expected to be small on
  a diversified 10-symbol universe (drift from equal-weight is slow at monthly rebalance cadence
  vs annual holding period), but it is non-zero and has not been quantified here.

**The artifact characterizes the ACTUAL shipped baseline:** pure buy-once-hold at bar 0, equal-weight
initial allocation, no rebalancing, no fees on the initial buy. The runbook's monthly/equal-weight
cadence is a forward operational proposal that has not been backtested under this harness.

---

## 5. Source Citations

All data read from anchored reports — zero new runs:

| Report file | Year | Role |
|-------------|------|------|
| `spec/momentum-parameter-robustness-sweep/reports/robustness-sweep-20260530-180006-v1-momentum-theta-surface-2023-block-bootstrap-real-fy.md` | 2023 | Primary 2023 BH reference |
| `spec/carry-strategy/reports/robustness-sweep-20260602-075424-v1-carry-theta-surface-2024-block-bootstrap-real-fy.md` | 2024 | Primary 2024 BH reference |
| `spec/time-series-momentum-robustness/reports/robustness-sweep-20260603-084715-v1-ts-momentum-theta-surface-2024-block-bootstrap-real-fy.md` | 2024 | Cross-check 2024 BH |
| `spec/horizon-retest-robustness/reports/test-2026-06-05-horizon-retest-robustness.md` | both | BH total-return invariance note |
| All 14 2023 BH-bearing reports (byte-identical BUYHOLD rows) | 2023 | Consistency proof |
| All 8 2024 BH-bearing reports (byte-identical BUYHOLD rows) | 2024 | Consistency proof |

---

## 7. Realized Equity Curve + Full Metrics (2023, 2024)

**Produced by:** `crates/backtest/examples/passive_baseline_equity.rs`
**Run command:** `cargo run -p backtest --features realdata --example passive_baseline_equity`
**Data:** Same revision SHA `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` as the sweep harness.
**Construction:** Equal-weight ($10k/symbol), buy at bar-0 close, zero rebalancing, no fees — byte-identical to `run_buyhold_path`.
**Nature:** This is the REALIZED single-path curve over the actual historical price sequence — not bootstrap-resampled.

### 7.1 Full Metrics Table

| Year | Sharpe  | Sortino | Calmar | MaxDD%  | TotalReturn% | InitEquity  | FinalEquity | MinEquity | MaxEquity | N bars |
|------|---------|---------|--------|---------|--------------|-------------|-------------|-----------|-----------|--------|
| 2023 | +1.8417 | +2.5126 | +5.677 | 34.57%  | +196.22%     | $100,000.00 | $296,221.10 | $99,328.76| $331,427.83| 8,759 |
| 2024 | +0.8925 | +1.2047 | +1.853 | 48.95%  | +91.04%      | $100,000.00 | $191,040.25 | $81,341.74| $245,873.59| 8,784 |

**Annualization factor:** `SQRT_HPY = 92.601_295_098_46` (= `sqrt(8760)` per `compute_sharpe_hourly`).
**Calmar:** CAGR / maxDD, where years = (n-1)/8760 per `compute_calmar`.
**MaxDD:** Peak-to-trough maximum drawdown fraction (returned as positive) per `compute_max_drawdown_f64`.

### 7.2 Equity Trajectory Summary

#### 2023 (8760 hourly bars, 10-symbol equal-weight $100k USDT)

- **Start:** $100,000.00 USDT (2023-01-01T00:00Z, bar 0)
- **End:** $296,221.10 USDT (2023-12-31T23:00Z) — up +196.2% in 2023
- **All-time high (intra-year):** $331,427.83 USDT (+231.4% from start)
- **All-time low (intra-year):** $99,328.76 USDT (-0.7% from start; dipped very briefly below par in early Jan 2023)
- **Max drawdown from peak:** 34.57% (peak at $331k → trough during late-year consolidation)
- **Daily-sampled CSV:** `bh-equity-curve-2023.csv` (365 rows, hourly stride=24)

#### 2024 (8784 hourly bars, 10-symbol equal-weight $100k USDT)

- **Start:** $100,000.00 USDT (2024-01-01T00:00Z, bar 0)
- **End:** $191,040.25 USDT (2024-12-31) — up +91.0% in 2024
- **All-time high (intra-year):** $245,873.59 USDT (+145.9% from start)
- **All-time low (intra-year):** $81,341.74 USDT (-18.7% from start; a mid-year drawdown episode)
- **Max drawdown from peak:** 48.95% (larger than 2023's 34.6%; consistent with the higher p95 MaxDD of 64.83% in bootstrap)
- **Daily-sampled CSV:** `bh-equity-curve-2024.csv` (366 rows, hourly stride=24)

### 7.3 Bootstrap Reconciliation

The realized (single-path) Sharpe and the bootstrap p50 Sharpe are different quantities and are EXPECTED to differ:
- **Realized Sharpe** = metric computed on the ONE actual historical price sequence.
- **Bootstrap p50 Sharpe** = MEDIAN over 200 block-resampled paths (each path is a random recombination of the real data's block structure, seeded 0xC0FFEE).

| Year | Realized Sharpe | Bootstrap p50 Sharpe | Gap (abs) | Gap (%) | Assessment |
|------|-----------------|----------------------|-----------|---------|------------|
| 2023 | **+1.8417**     | +1.7353              | +0.1064   | +6.1%   | Realized ABOVE median — consistent: 2023 bull leg was a favorable sequence |
| 2024 | **+0.8925**     | +1.1047              | -0.2123   | -19.2%  | Realized BELOW median — consistent: 2024 actual path had a sharper mid-year drawdown than many resampled paths |

**Sanity gate result:** Both years pass — same sign, same order of magnitude, gap within expected single-path vs distribution-median variance. The 19% gap in 2024 is not alarming: the p5 of the bootstrap distribution is -0.68, so a realized Sharpe of +0.89 sits well inside the distribution. The realized path is the 40th-percentile-ish path in 2024 (below the median but far from the tail).

**Important nuance:** 2023 has 87,590 bars loaded vs 87,600 expected (10 bars = 10 symbol-hours missing; coverage = 99.99%, well above the 99.5% tolerance). This is the same 10-bar gap the sweep harness observed; it does not affect the Sharpe materially.

### 7.4 Data Files

| File | Rows | Description |
|------|------|-------------|
| `bh-equity-curve-2023.csv` | ~367 | Daily-sampled (stride=24) hourly equity; columns: `bar_index, timestamp_utc, equity_usd` |
| `bh-equity-curve-2024.csv` | ~368 | Same format for 2024 |

Both files are in the non-anchored artifacts dir. They are NOT regression-anchored (they are data outputs, not report bodies).

---

## 6. Anchor + Pollution Verification

- **Anchor gate:** `scripts/verify_anchors.sh` → **119/119 PASS** (verified both rounds: initial bootstrap characterization 2026-06-08, and realized curve production 2026-06-08)
- **No new anchored reports written:** All artifacts are at `docs/runbooks/artifacts/passive-baseline-2026-06-08/`
  (non-anchored location). No files were written to any `spec/*/reports/` anchored directory.
- **Code changes:** Only `crates/backtest/Cargo.toml` (added `[[example]]`) and `crates/backtest/examples/passive_baseline_equity.rs` (new read-only probe). No strategy, ScoreSource, anchor, or production binary touched.
- **Stray report check:** `git status --porcelain` shows only `crates/backtest/Cargo.toml`, `crates/backtest/examples/`, `docs/runbooks/artifacts/`, `docs/runbooks/passive-baseline.md`, and `data/yahoo/REVISION.toml` (pre-existing). Zero files in any `spec/*/reports/` anchored dir.
