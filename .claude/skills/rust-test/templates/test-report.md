---
title: Test Report
feature: <feature-slug>
run_id: <YYYY-MM-DD-HHMM-UTC>
commit: <git sha or "uncommitted">
agent: tester
verdict: <PASS | FAIL | REGRESSION>
---

# Test Report — <feature-slug> — <YYYY-MM-DD HH:MM UTC>

## 1. Scope

- **Feature / change under test:** <short description>
- **Spec refs:** `spec/features/<slug>.md`, `spec/tasks/<slug>.md`
- **Commit SHA:** `<sha>`
- **Rust toolchain:** `<rustc --version>`
- **OS / arch:** `<uname -a short>`

## 2. Static Analysis

| Check              | Result | Notes                       |
|--------------------|--------|-----------------------------|
| `cargo fmt --check`| PASS/FAIL | <diff size if fail>      |
| `cargo clippy`     | PASS/FAIL | <warnings count, top 3>  |
| `cargo audit`      | PASS/FAIL | <advisories if any>      |
| `cargo deny`       | PASS/FAIL | <bans / licenses>        |

## 3. Unit & Integration Tests

| Crate | Passed | Failed | Ignored | Duration |
|-------|-------:|-------:|--------:|---------:|
| `<crate>` |  |  |  |  |
| **Total** |  |  |  |  |

### Failing Tests

_List each failure with name, file, and a 20-line excerpt. Write `_none_` if all passed._

## 4. Property / Fuzz Tests

_Skip if not applicable — write `_n/a_`._

| Suite | Cases | Shrunk failures | Seed |
|-------|------:|----------------:|------|

## 5. Backtest Results

_Skip if this change did not touch strategy logic — write `_n/a_` and say why._

**Universe:** <symbols>
**Period:** <start> — <end>
**Data source:** <feed>
**Fees / slippage model:** <model>

| Metric           | Current | Baseline | Δ |
|------------------|--------:|---------:|--:|
| Total return     |         |          |   |
| CAGR             |         |          |   |
| Sharpe           |         |          |   |
| Sortino          |         |          |   |
| Max drawdown     |         |          |   |
| Hit rate         |         |          |   |
| Turnover         |         |          |   |
| Trades           |         |          |   |
| Avg trade P&L    |         |          |   |

### Equity Curve

_Describe in 2–3 sentences: shape, notable regimes, worst drawdown window._

### Regressions vs Baseline

_List any metric worse by more than the tolerance in `spec/architecture.md` risk budget._

## 6. Benchmarks

_Skip if this change did not touch hot paths._

| Benchmark | Current (µs) | Baseline (µs) | Δ% |
|-----------|-------------:|--------------:|---:|

## 7. Environment / Infrastructure Issues

_Flaky tests, infra outages, data gaps. Write `_none_` if clean._

## 8. Verdict

**`<PASS | FAIL | REGRESSION>`**

One-paragraph justification.

## 9. Routing

`HANDOFF → <analyst | architect | developer>` — <one-line reason>

_or_

`VERDICT → PASS` — ready to ship.
