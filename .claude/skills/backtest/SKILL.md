---
name: backtest
description: Run a historical backtest for a strategy feature and emit a metrics block for embedding into the test report. Use when the tester agent verifies a strategy change, when the analyst needs baseline performance, or whenever spec/features/<slug>.md specifies a backtest scenario.
---

# backtest

## Inputs

- `feature_slug` — matches `spec/features/<slug>.md`
- `scenario` — section within that feature file, e.g. `btc-2023-regime`
- `baseline` — optional report path to diff against; defaults to most recent
  `spec/reports/test-*-<slug>.md` with a Backtest section.

## Procedure

1. Resolve the scenario: read `spec/features/<slug>.md` → `## Backtest Scenarios` block.
   Each scenario defines universe, period, data source, fees, and entry config.

2. Invoke the workspace's backtest binary (the architect defines this; default
   assumption `cargo run --release --bin backtest -- --scenario <scenario>`).
   Abort if the binary does not exist and route to architect/developer.

3. Capture the JSON metrics the binary emits. Validate it includes at minimum:
   `total_return, cagr, sharpe, sortino, max_drawdown, hit_rate, turnover,
   trades, avg_trade_pnl`.

4. Load the baseline report (if any) and compute deltas per metric.

5. Produce a markdown block matching the "Backtest Results" section of
   `rust-test/templates/test-report.md`. Embed it into the caller's report.

6. Save raw artifacts (equity curve CSV, trade log) under
   `spec/reports/artifacts/<run_id>/` so they survive for future diffs.

## Failure Modes

- Missing market data → `HANDOFF → analyst` with the data gap described.
- Backtest binary crash → `HANDOFF → developer` with stack trace.
- All metrics present but Sharpe < baseline − tolerance → flag as
  `REGRESSION` in the verdict; do not auto-route — the tester decides.

## Templates

See [templates/scenario.md](templates/scenario.md) for the scenario-definition
format used inside `spec/features/<slug>.md`.
