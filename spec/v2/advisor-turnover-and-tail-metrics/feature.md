---
slug: advisor-turnover-and-tail-metrics
status: dev-done
owner: developer
version: 0.1.0
updated: 2026-06-29
---

# Turnover KPI + Coherent Tail / Median Reporting (P1-1 + P1-2)

Two report-honesty increments shipped together because they share the same
additive code path and are near-free reductions over data already captured.

**Design:** [`v2-architecture.md`](../v2-architecture.md) §1 P1-1 + P1-2.
**Research:** `research/risk-and-sizing/application-position-sizing-and-bet-sizing.md` §6 P1;
`research/risk-and-sizing/application-vol-targeting-and-drawdown-overlays.md` §6 P2-D;
`research/backtesting/application-cost-and-impact-modeling.md` §6 A.

---

## P1-1 — Turnover formula (chosen and documented here)

**Formula:** `turnover = Σ(price × qty) / mean_equity`

That is, the sum of absolute trade notional (fill price × fill quantity, in
USDT) divided by the time-average equity over the backtest window.  The result
is a unitless ratio: "how many times did the strategy churn its capital?"

- A `turnover` of 1.0 means the strategy transacted its entire equity once.
- A `turnover` of 0.0 means no trades were executed (idle / buy-and-hold).
- A `turnover` of 10.0 means ten capital-equivalents of volume.

**Why this formula:** it maps directly to what the operator already has in
`RunReport.fills` (each fill has `price: Price` and `qty: Quantity`, both
`Decimal`-backed) and `report.equity_series` (from which mean equity is the
arithmetic mean of the equity values).  No new capture, no new engine fields.
The result is comparable across strategies with different position sizes because
it is normalised by mean equity — a strategy that holds a $1 000 position
transacting $500 notional scores 0.5, same as a $10 000 strategy transacting
$5 000.

**What it is NOT:** it is not a round-trip count (that's `trade_count / 2`); it
is not a per-day rate (annualising is the UI-designer's call); it is not P&L.

**Type:** `pub turnover: Decimal` on `CandidateKpis` — Decimal, consistent with
the existing `total_return_pct` / `max_drawdown` money fields.

---

## P1-2 — CVaR not VaR (rationale)

**CVaR (Expected Shortfall / Conditional Value-at-Risk) is the correct tail
metric because it is sub-additive (coherent), meaning the risk of a combined
portfolio never exceeds the sum of the individual risks.**  VaR is NOT
sub-additive: two individually "safe" portfolios at a given VaR level can
combine into a portfolio that exceeds that level — it rewards concentration
over diversification.  For a single-coin advisor this distinction is less
acute, but reporting a non-coherent measure is dishonest; the research and the
architect are both explicit on this point.

**CVaR_α:** the mean of the worst α-fraction of the bootstrap `total_return`
distribution across 1 000 paths.
- `cvar_95`: mean of the bottom 5% of paths by `total_return` (i.e. the α=0.05
  tail — "expected loss in the worst 5% of scenarios").
- `cvar_99`: mean of the bottom 1% of paths.

**Computed over `total_return`** (not `final_equity`) for two reasons:
1. `total_return` is already a fraction — comparable across budget sizes.
2. The existing `PathMetrics.total_return` is already captured as `f64`;
   `final_equity` is `Decimal` (kept for P(loss) integer comparisons).  Using
   `total_return` keeps all tail stats as `f64`, consistent with the existing
   statistical convention (ADR-0003 / R-NR.3).

**Median terminal wealth:** the p50 of `final_equity.to_f64()` across paths.
Answers "what does the middle outcome actually look like in dollars?" — more
intuitive than mean (which is pulled by extreme wins).

**Skew:** 3rd standardised central moment of `total_return` across the 1 000
bootstrap paths.  Positive skew = right tail (lottery-style); negative = left
tail (crash-prone).  Zero on a symmetric distribution.

---

## What shipped

### `crates/backtest/src/stats/mod.rs`

- `DistributionSummary` extended with four new fields:
  - `pub cvar_95: f64` — CVaR at α=0.05 (mean of worst 5% `total_return` paths).
  - `pub cvar_99: f64` — CVaR at α=0.01 (mean of worst 1% `total_return` paths).
  - `pub median_terminal_wealth: f64` — p50 of `final_equity` across paths.
  - `pub skew: f64` — 3rd standardised central moment of `total_return` across paths.
- `from_path_metrics` extended to compute all four.
- Unit tests: CVaR on a known hand-built path vector; median; skew (zero on
  symmetric, positive on right-skewed, negative on left-skewed).

### `crates/backtest/src/bakeoff/mod.rs`

- `CandidateKpis` extended with `pub turnover: Decimal`.
- `derive_candidate_kpis` computes turnover from `RunReport.fills` (sum of
  `price × qty`) / mean equity from `RunReport.equity_series`.
- `run_bakeoff` — default (zero) turnover for the benchmark arm fallback
  in the `CandidateKpis` literal (the unreachable branch).
- Unit tests: idle (zero fills → turnover 0); one round-trip; multi-trade.
- Frozen-gate-identity test: `rank_candidates` output is byte-identical before
  and after the new `turnover` field is populated.

### `crates/ui/src/leaderboard/state.rs`

- `LeaderRow` extended with `pub turnover: Decimal`.
- `BakeoffReportMirror::from_report` mirrors `c.kpis.turnover`.
- Fixture / test constructors updated (`row()` helper in the test module).

## Not in this increment

- UI column rendering (ui-designer's call — `LeaderRow.turnover` is carried
  for narration exactly as `sortino`/`calmar` are carried today).
- CVaR / tail metrics exposed in the leaderboard table columns (ui-designer).
- Annualised turnover rate (a future formatting decision).

## Implementation

**P1-1 Turnover:**
- `CandidateKpis.turnover: Decimal` at `crates/backtest/src/bakeoff/mod.rs:651`.
- `derive_candidate_kpis` computes `Σ(fill.price.get() × fill.qty.get()) / mean_equity`.
  Mean equity = arithmetic mean of the equity-series amounts. Zero when fills or equity empty.
- `LeaderRow.turnover: Decimal` at `crates/ui/src/leaderboard/state.rs:71`.
- `from_report` mirror updated at same file.

**P1-2 Tail metrics:**
- `DistributionSummary.{cvar_95, cvar_99, median_terminal_wealth, skew}` at
  `crates/backtest/src/stats/mod.rs:347-364`.
- `from_path_metrics` computes all four at same file (`use ToPrimitive` moved to function top per `items_after_statements` lint).
- Helper `compute_cvar` at `stats/mod.rs:617`; `compute_distribution_skew` at `stats/mod.rs:641`.

**Cascade fixes (struct literals):**
- `crates/backtest/src/bakeoff/rank.rs`: `make_candidate` test helper.
- `crates/backtest/src/bakeoff/scorecard.rs`: `make_candidate` closure.
- `crates/backtest/src/bakeoff/robustness.rs`: `make_summary` test helper.
- `crates/backtest/src/bakeoff/sweep.rs`: `fallback_distribution_summary`.
- `crates/backtest/src/bin/param_robustness_sweep.rs`: `make_summary`.
- `crates/backtest/tests/robustness_bootstrap_bites.rs`: `CandidateKpis` literal.
- `crates/ui/src/fixtures.rs`: 49 `LeaderRow` struct literals.
- `crates/ui/src/tune/state.rs`: `make_dist` and `make_kpis` helpers.

**Gate outputs (2026-06-29):**
- `cargo test -p backtest --lib`: `test result: ok. 193 passed; 0 failed; 8 ignored`
- `cargo clippy -p backtest --tests -- -D warnings`: `CLIPPY_EXIT: 0`
- `cargo fmt -- --check`: `FMT_CHECK_EXIT: 0`
- `bash scripts/verify_anchors.sh`: `ANCHORS PASS (119 / 119)`
- `python3 scripts/spec_lint.py`: `spec-lint: PASS (0 violations)`
