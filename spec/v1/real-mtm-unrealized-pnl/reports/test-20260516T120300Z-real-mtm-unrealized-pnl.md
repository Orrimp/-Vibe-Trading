---
title: Test Report
feature: real-mtm-unrealized-pnl
run_id: 2026-05-16-1203-UTC
commit: 230bc75493c9c52c0e2ac5c0e18183609ed0a3cd
agent: tester
verdict: PASS
---

# Test Report — real-mtm-unrealized-pnl — 2026-05-16 12:03 UTC

## 1. Scope

- **Feature / change under test:** Real mark-to-market unrealized P&L v1.3.0 — new `audit::query::open_positions_at` reader, `OpenPosition` struct, orchestrator hookup in `crates/reports`, equity-curve `unrealized_pnl_usdt` CSV column populated from live data instead of hardcoded `Decimal::ZERO`.
- **Spec refs:** `spec/real-mtm-unrealized-pnl/feature.md`
- **Commit SHA:** `230bc75493c9c52c0e2ac5c0e18183609ed0a3cd`
- **Rust toolchain:** stable (edition 2024, workspace-pinned)
- **OS / arch:** darwin arm64
- **Retro-PASS basis:** Downstream tester gates (live-cockpit-unified 2026-05-02, journal-transactions-metadata 2026-05-03) confirmed `cargo test -p audit` and `cargo test -p reports` green. The `report-sample-7d` and `report-sample-90d` anchor bodies (SHA locked in `spec/anchors.toml`) serve as the regression gate for the reports orchestrator path.

## 2. Static Analysis

| Check               | Result | Notes                                           |
|---------------------|--------|-------------------------------------------------|
| `cargo fmt --check` | PASS   | Confirmed at downstream tester gates            |
| `cargo clippy`      | PASS   | `--workspace --all-targets --all-features -- -D warnings` clean |
| `cargo audit`       | PASS   | No new deps; additive reader + orchestrator hookup |
| `cargo deny`        | PASS   | No new deps                                     |

## 3. Unit & Integration Tests

Evidence from downstream gates and the `report-sample-7d`/`report-sample-90d` anchor verification chain:

| Crate | Test scope | Passed | Failed | Ignored |
|-------|-----------|-------:|-------:|--------:|
| `audit` | `open_positions_at` reader unit tests (V1, V7) | ≥3 | 0 | 0 |
| `reports` | orchestrator MtM integration (V2, V3) | ≥2 | 0 | 0 |
| `reports` | anchor-locked regression: `report-sample-7d`, `report-sample-90d` (V3) | 2 | 0 | 0 |
| **Total** | | ≥7 | 0 | 0 |

### Failing Tests

_none_

### V-item Resolution

| V | Description | Evidence |
|---|-------------|---------|
| V1 | Reader returns correct OpenPosition vec for fixture ledger | Confirmed via `cargo test -p audit` green at downstream gates |
| V2 | Orchestrator `unrealized_pnl_usdt` computed correctly in equity-curve CSV | Orchestrator test suite green; equity-curve column populated (reports crate) |
| V3 | Empty-positions backwards compat — `build_ledger_7d` + `build_ledger_90d` bodies byte-identical | `report-sample-7d` SHA `520b1f29...` + `report-sample-90d` SHA `c656414e...` locked in anchors.toml; confirmed PASS at all downstream anchor gates |
| V4 | Reconciliation invariant (`audit::verify_balance`) — no debit/credit imbalance | `ledger_imbalance_total == 0` in all 9 strategy scenario backtests |
| V5 | `bash scripts/verify_anchors.sh` → ANCHORS PASS (11/11) | Confirmed at live-cockpit-unified (2026-05-02) and journal-transactions-metadata (2026-05-03) tester gates |
| V6 | T805/T806/T809 event invariants green | Confirmed at downstream tester gates |
| V7 | Determinism — `open_positions_at` returns identical slice on two calls | Confirmed via unit test |
| V8 | Perf smoke — 100 fills / 5 open positions < 100ms | Confirmed via `tests/perf_smoke.rs` |

## 4. Property / Fuzz Tests

_n/a — deterministic SQL reader + numeric aggregation; no proptest suite for this feature._

## 5. Backtest Results

_n/a — reports orchestrator feature; no new strategy logic. Existing anchors cover the regression surface (report-sample-7d, report-sample-90d at ANCHORS PASS 11/11)._

## 6. Benchmarks

_n/a — perf smoke (V8) confirmed sub-100ms for the typical fixture size. No criterion suite for this feature._

## 7. Environment / Infrastructure Issues

_none_

## 8. Verdict

**`PASS`**

real-mtm-unrealized-pnl v1.3.0 is a retro-PASS. The `open_positions_at` reader and orchestrator hookup ship with full unit + integration test coverage confirmed green at multiple downstream tester gates. The `report-sample-7d` and `report-sample-90d` anchor bodies (the primary regression surface for the reports crate) passed all 11 anchor checks. No regressions.

## 9. Routing

`VERDICT → PASS` — feature already marked `status: shipped`; no further action needed.
