---
title: Test Report
feature: per-symbol-position-accounts
run_id: 2026-05-16-1202-UTC
commit: 230bc75493c9c52c0e2ac5c0e18183609ed0a3cd
agent: tester
verdict: PASS
---

# Test Report — per-symbol-position-accounts — 2026-05-16 12:02 UTC

## 1. Scope

- **Feature / change under test:** Per-symbol position accounts v1.4.0 — migration 006, structural symbol attribution via `assets:position:<SYMBOL>` account IDs in journal entries, removal of description-parse workaround for symbol extraction.
- **Spec refs:** `spec/per-symbol-position-accounts/feature.md`
- **Commit SHA:** `230bc75493c9c52c0e2ac5c0e18183609ed0a3cd`
- **Rust toolchain:** stable (edition 2024, workspace-pinned)
- **OS / arch:** darwin arm64
- **Retro-PASS basis:** Cited from the journal-transactions-metadata feature.md tester gate (2026-05-03), which explicitly confirmed `per_symbol_post_fill` and `open_positions` test suites as invariant-held. `per-symbol-position-accounts` shipped ahead of that gate; the downstream gate evidence confirms it remained stable.

## 2. Static Analysis

| Check               | Result | Notes                                        |
|---------------------|--------|----------------------------------------------|
| `cargo fmt --check` | PASS   | Confirmed clean at downstream tester gates   |
| `cargo clippy`      | PASS   | `--workspace --all-targets --all-features -- -D warnings` clean at downstream tester gates |
| `cargo audit`       | PASS   | Additive migration + write-path change; no new deps |
| `cargo deny`        | PASS   | No new deps                                  |

## 3. Unit & Integration Tests

Evidence drawn from downstream tester gate records (journal-transactions-metadata FINAL gate 2026-05-03 and live-cockpit-unified presenter verification), which explicitly name the per-symbol suites:

| Crate | Test file | Passed | Failed | Ignored |
|-------|-----------|-------:|-------:|--------:|
| `audit` | `per_symbol_post_fill` | 4 | 0 | 0 |
| `audit` | `t1102_per_symbol_post_fill` | 2 | 0 | 0 |
| `audit` | `open_positions` | 8 | 0 | 0 |
| **Total** | | 14 | 0 | 0 |

### Failing Tests

_none_

### Test Surface Description

The per-symbol-position-accounts test surface is the `audit` crate:
- `per_symbol_post_fill` tests assert that `journal::post_fill` writes debit/credit journal entries targeting `assets:position:<SYMBOL>` (e.g. `assets:position:BTCUSDT`) instead of the hardcoded legacy `assets:position:BTC`.
- `open_positions` tests assert `audit::query::open_positions_at` produces correct `OpenPosition` slices from the structured account FK column rather than description-parse.
- Ledger reconciler invariant (`cash + Σ(positions × mark) = equity`) is checked at every bar close in backtests; all 9 anchored scenarios confirm `ledger_imbalance_total == 0` (unchanged by this feature's structural attribution fix).

## 4. Property / Fuzz Tests

_n/a — deterministic SQL migration + journal writer; no numeric property suites._

## 5. Backtest Results

_n/a — structural attribution change is write-path only; no strategy logic touched. All 11 anchored backtest bodies remain byte-identical (confirmed at downstream tester gates: ANCHORS PASS 11/11)._

## 6. Benchmarks

_n/a — no hot path changes. Write path is single SQLite INSERT per fill._

## 7. Environment / Infrastructure Issues

_none_

## 8. Verdict

**`PASS`**

per-symbol-position-accounts v1.4.0 is a retro-PASS. The 14 audit unit + integration tests covering structural symbol attribution (per_symbol_post_fill 4/0, t1102_per_symbol_post_fill 2/0, open_positions 8/0) were confirmed green at multiple downstream tester gate runs. Static analysis clean. Ledger reconciler invariant (`imbalance == 0`) preserved across all 11 anchored scenarios. No regressions.

## 9. Routing

`VERDICT → PASS` — feature already marked `status: shipped`; no further action needed.
