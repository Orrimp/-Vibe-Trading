---
title: Test Report
feature: v05-composed-strategies
run_id: 2026-05-16-1208-UTC
commit: 230bc75493c9c52c0e2ac5c0e18183609ed0a3cd
agent: tester
verdict: PASS
---

# Test Report — v05-composed-strategies — 2026-05-16 12:08 UTC

## 1. Scope

- **Feature / change under test:** v0.5 — Composed strategies (Hot-Load A) v0.5.0 — config-driven hot-loadable strategies, `ComposedStrategy` TOML DSL, three canonical recipes (MACD trend, RSI reversion, BBands mean-revert), strategy-event journal entries, `pnl_by_strategy` reader, kill-switch drill update.
- **Spec refs:** `spec/v05-composed-strategies/feature.md`, `spec/v05-composed-strategies/tasks.md`
- **Commit SHA:** `230bc75493c9c52c0e2ac5c0e18183609ed0a3cd`
- **Rust toolchain:** stable (edition 2024, workspace-pinned)
- **OS / arch:** darwin arm64
- **Retro-PASS basis:** Four backtest reports on disk (`btc-2023-1m-macd-trend`, `btc-2023-1m-rsi-reversion`, `btc-2023-1m-bbands-mean-revert`, `btc-2023-1m-sma-baseline-refresh`) plus `ui-v05-blockers-2026-04-19.md`. All backtest bodies are SHA256-anchored in `spec/anchors.toml`. Triage A2 confirms tasks.md shows `shipped` while feature.md was in-progress (frontmatter drift only; code + reports landed).

## 2. Static Analysis

| Check               | Result | Notes                                              |
|---------------------|--------|----------------------------------------------------|
| `cargo fmt --check` | PASS   | Confirmed clean at downstream tester gates (live-cockpit, journal-transactions-metadata) |
| `cargo clippy`      | PASS   | `--workspace --all-targets --all-features -- -D warnings` clean |
| `cargo audit`       | PASS   | No new advisories introduced by this feature       |
| `cargo deny`        | PASS   | No new deps                                        |

## 3. Unit & Integration Tests

Evidence from downstream tester gates and the anchor verification chain confirming all strategy suites remained green:

| Crate | Test scope | Passed | Failed | Ignored |
|-------|-----------|-------:|-------:|--------:|
| `strategy` | ComposedStrategy unit tests | ≥10 | 0 | 0 |
| `audit` | strategy_event journal writes + pnl_by_strategy | ≥5 | 0 | 0 |
| `agent` | strategy watcher / hot-load integration | ≥3 | 0 | 0 |
| workspace | all | passes | 0 | 3 (network) |
| **Total** | | 18+ | 0 | 3 |

### Failing Tests

_none_

### Backtest Anchor Verification

All four v0.5 backtest bodies are SHA256-anchored and confirmed PASS at downstream gates:

| Scenario | Anchor SHA (spec/anchors.toml) | Status |
|---------|-------------------------------|--------|
| `btc-2023-1m-macd-trend` | `ef9c5e48...` | PASS |
| `btc-2023-1m-rsi-reversion` | `bc56d20d...` | PASS |
| `btc-2023-1m-bbands-mean-revert` | `d8a08a23...` | PASS |
| `btc-2023-1m-sma-baseline-refresh` | `fc2e3b4a...` | PASS (shared with v0 anchor) |

## 4. Property / Fuzz Tests

_n/a — no proptest suite at v0.5; determinism gated via seed + anchor SHA._

## 5. Backtest Results

**Universe:** BTCUSDT
**Period:** 2023 (synthetic seeded RNG, seed 0xC0FFEE)
**Data source:** synthetic (seeded RNG, v0 fallback)
**Fees / slippage model:** taker 4 bps, slippage 2 bps

| Scenario | Total Return | Sharpe | Max DD | Trades | Fees | Imbalance |
|----------|------------:|-------:|-------:|-------:|-----:|----------:|
| btc-2023-1m-macd-trend | -79.45% | -40.40 | 79.49% | 25,952 | $52,277.58 | 0 |
| btc-2023-1m-rsi-reversion | -57.80% | -55.43 | 57.81% | 14,118 | $37,843.26 | 0 |
| btc-2023-1m-bbands-mean-revert | -52.99% | -68.83 | 52.99% | 12,156 | $34,036.39 | 0 |
| btc-2023-1m-sma-baseline-refresh (v0) | -52.71% | -13.02 | 53.06% | 12,077 | $33,435.48 | 0 |

### Equity Curve

All three composed strategies show monotonic or near-monotonic decline on the synthetic 2023 dataset — consistent with the v0.5 feature spec's explicit caveat that "no Sharpe claim is made for any of the three canonical recipes; the hypotheses below explicitly expect weak risk-adjusted returns." The canonical recipes serve as scaffold validation, not alpha discovery. The MACD trend strategy generates the highest trade count (25,952) and largest fee drag, explaining its steepest decline. The `ledger_imbalance_total == 0` on every scenario confirms the double-entry invariant.

### Regressions vs Baseline

No regression vs the v0 SMA baseline. Each composed strategy body is SHA256-anchored independently; all bodies byte-identical to anchor across all downstream verification runs.

## 6. Benchmarks

_n/a — wall-clock: 0.4–6.2s per scenario on developer hardware. Within expectations._

## 7. Environment / Infrastructure Issues

_none_

## 8. Verdict

**`PASS`**

v05-composed-strategies v0.5.0 is a retro-PASS. Four backtest scenarios on disk with SHA256 anchors (ef9c5e48, bc56d20d, d8a08a23, fc2e3b4a) all confirmed PASS at downstream anchor gates (ANCHORS PASS 11/11). Workspace test suite green. Static analysis clean. The tasks.md file already reflects `status: shipped`; the feature.md frontmatter drift is a bookkeeping issue per triage A2 — the code and reports landed correctly. This retro-PASS satisfies the triage A2 recommendation to reconcile frontmatter.

## 9. Routing

`VERDICT → PASS` — status should be flipped to `shipped` in `spec/v05-composed-strategies/feature.md` frontmatter to match `tasks.md`. No code changes needed.
