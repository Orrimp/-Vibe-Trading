---
title: Test Report
feature: v1-cross-sectional-momentum
run_id: 2026-05-16-1209-UTC
commit: 230bc75493c9c52c0e2ac5c0e18183609ed0a3cd
agent: tester
verdict: PASS
---

# Test Report — v1-cross-sectional-momentum — 2026-05-16 12:09 UTC

## 1. Scope

- **Feature / change under test:** v1 cross-sectional momentum v1.0.0 — 10-symbol universe (top10_momentum_h1), vol-adjusted log-return signal, equal-weight portfolio construction, per-symbol cap (40%) + portfolio cap (50%), 1h bars, cross-sectional ranking on 60-bar lookback. T612 (multi-symbol live BinanceFeed) deferred to v1.5 by operator decision.
- **Spec refs:** `spec/v1-cross-sectional-momentum/feature.md`, `spec/v1-cross-sectional-momentum/tasks.md`
- **Commit SHA:** `230bc75493c9c52c0e2ac5c0e18183609ed0a3cd`
- **Rust toolchain:** stable (edition 2024, workspace-pinned)
- **OS / arch:** darwin arm64
- **Retro-PASS basis:** Two backtest reports on disk (`top10-2023-1h-momentum`, `top10-2024-h1-momentum`) and `dev-v1-closeout-notes-2026-04-29.md`. Both scenario bodies SHA256-anchored in `spec/anchors.toml`. T_FINAL_A_v1 and T_FINAL_B_v1 both `[x]` per triage A3 / feature.md.

## 2. Static Analysis

| Check               | Result | Notes                                              |
|---------------------|--------|----------------------------------------------------|
| `cargo fmt --check` | PASS   | Confirmed at downstream tester gates               |
| `cargo clippy`      | PASS   | `--workspace --all-targets --all-features -- -D warnings` clean |
| `cargo audit`       | PASS   | No new advisories introduced by this feature       |
| `cargo deny`        | PASS   | No new deps beyond workspace baseline              |

## 3. Unit & Integration Tests

Evidence from downstream tester gates (live-cockpit-unified V4: `cargo test --workspace --all-targets` ~605 PASS / 0 FAIL, which post-dates v1):

| Crate | Test scope | Passed | Failed | Ignored |
|-------|-----------|-------:|-------:|--------:|
| `strategy` | cross-sectional momentum unit tests | ≥10 | 0 | 0 |
| `audit` | multi-symbol position + pnl_by_symbol | ≥8 | 0 | 0 |
| workspace | all | passes | 0 | 3 (network) |
| **Total** | | 18+ | 0 | 3 |

### Failing Tests

_none_

### T612 Deferred Task Note

T612 (multi-symbol live `BinanceFeed` — real WebSocket fan-out for the 10-symbol universe) was explicitly deferred to v1.5 by operator decision and recorded in feature.md as:

> `[DEFERRED TO v1.5 — operator confirmed: T612 stays [ ] and is NOT a v1 blocker]`

T_FINAL_A_v1 and T_FINAL_B_v1 are both `[x]`. The single open task (T612) remains open under v1.5 lineage (landed in `v1-5b-multi-venue` v1.2.0 as T1405/T1406 multi-symbol BinanceFeed fan-out). This retro-PASS applies to v1.0.0 scope only.

### Backtest Anchor Verification

Both v1 backtest scenario bodies are SHA256-anchored:

| Scenario | Anchor SHA (spec/anchors.toml) | Status |
|---------|-------------------------------|--------|
| `top10-2023-1h-momentum` | `3b60ef07...` | PASS |
| `top10-2024-h1-momentum` | `1f33534f...` | PASS |

## 4. Property / Fuzz Tests

_n/a — determinism gated via seed + anchor SHA._

## 5. Backtest Results

**Universe:** BTCUSDT, ETHUSDT, BNBUSDT, ADAUSDT, SOLUSDT, DOTUSDT, LINKUSDT, AVAXUSDT, XRPUSDT, DOGEUSDT (10 symbols)
**Period:** 2023 (H1) and 2024 H1 (synthetic seeded RNG, seed 0xC0FFEE)
**Data source:** synthetic (seeded RNG, v1 multi-symbol, 10 independent ChaCha20Rng streams)
**Fees / slippage model:** taker 4 bps, slippage 2 bps

| Metric           | 2023 H1 | 2024 H1 | Delta |
|------------------|--------:|--------:|------:|
| Total return     | -43.72% | -53.60% | -9.88pp |
| Max drawdown     | 87.48% | 87.48% | 0 |
| Trades           | 4,809 | 2,490 | -2,319 |
| Fees             | $3,810.09 | $3,102.99 | -$707.10 |
| Imbalance        | 0 | 0 | 0 |

### Equity Curve

Both periods show drawdowns reaching 87.48% maximum — the cross-sectional ranking with equal weighting and a 60-bar lookback on synthetic data produces high portfolio turnover and fee drag at 4 bps taker. The 2024 run covers only H1 (43,800 bars vs 87,600 for full-year 2023), explaining lower trade count. The feature spec explicitly notes "no alpha claim for v1; the strategy framework validation is the objective."

### Regressions vs Baseline

Both scenario bodies are SHA256-anchored and confirmed PASS at all downstream verification runs. No regression.

## 6. Benchmarks

_n/a — 2023 run: 4.2s wall-clock; 2024 run: 2.0s wall-clock. Within expectations._

## 7. Environment / Infrastructure Issues

_none_

## 8. Verdict

**`PASS`**

v1-cross-sectional-momentum v1.0.0 is a retro-PASS. Two backtest scenarios on disk with SHA256 anchors (3b60ef07, 1f33534f) confirmed PASS at all downstream anchor gates (ANCHORS PASS 11/11). T_FINAL_A_v1 and T_FINAL_B_v1 are `[x]`. Single open task T612 is operator-confirmed deferred to v1.5 and has since landed in `v1-5b-multi-venue`. Workspace test suite green. Static analysis clean. Triage A3 recommends SHIP; this retro-PASS satisfies that recommendation.

## 9. Routing

`VERDICT → PASS` — status should be flipped to `shipped` in `spec/v1-cross-sectional-momentum/feature.md` frontmatter per triage A3. T612 remains open under v1.5 lineage. No code changes needed.
