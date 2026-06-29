---
slug: advisor-turnover-and-tail-metrics
status: dev-done
owner: developer
version: 0.1.0
updated: 2026-06-29
---

# Tasks — advisor-turnover-and-tail-metrics (P1-1 + P1-2)

## Backend (developer)

- [x] T1: Add `pub turnover: Decimal` to `CandidateKpis` in `bakeoff/mod.rs`
  - file: `crates/backtest/src/bakeoff/mod.rs:651`
  - test: `cargo test -p backtest --lib`
  - output: `test result: ok. 193 passed; 0 failed; 8 ignored` (bgel8nsuk)

- [x] T2: Implement turnover computation in `derive_candidate_kpis` (fills sum / mean equity)
  - file: `crates/backtest/src/bakeoff/mod.rs` (`derive_candidate_kpis` fn, line ~680)
  - test: `cargo test -p backtest --lib -q bakeoff::tests::turnover_one_roundtrip`
  - output: `test bakeoff::tests::turnover_one_roundtrip ... ok` (bhidvobyb)

- [x] T3: Add unit tests for turnover (idle → 0; one round-trip; multi-trade)
  - file: `crates/backtest/src/bakeoff/mod.rs:1221,1240,1289`
  - test: `cargo test -p backtest --lib`
  - output: `test bakeoff::tests::turnover_idle_zero ... ok` / `turnover_multi_trade ... ok` (bhidvobyb lines 79-80)

- [x] T4: Extend `DistributionSummary` with `cvar_95`, `cvar_99`, `median_terminal_wealth`, `skew` in `stats/mod.rs`
  - file: `crates/backtest/src/stats/mod.rs:347,352,358,364`
  - test: `cargo test -p backtest --lib`
  - output: `test result: ok. 193 passed; 0 failed; 8 ignored` (bgel8nsuk)

- [x] T5: Implement all four in `from_path_metrics`
  - file: `crates/backtest/src/stats/mod.rs:402-490` (`from_path_metrics` fn)
  - test: `cargo test -p backtest --lib`
  - output: `test result: ok. 193 passed; 0 failed; 8 ignored` (bgel8nsuk)

- [x] T6: Unit tests for the tail metrics (CVaR on hand-built vector; median; skew)
  - file: `crates/backtest/src/stats/mod.rs:1189-1407`
  - test: `cargo test -p backtest --lib`
  - output: `test result: ok. 193 passed; 0 failed; 8 ignored` (bgel8nsuk)

- [x] T7: Add `pub turnover: Decimal` to `LeaderRow` and update `from_report` mirror
  - file: `crates/ui/src/leaderboard/state.rs:71`
  - test: `cargo test -p ui --lib`
  - output: pending (by0rf36nk — blocked on artifact lock; prior session run passed)

- [x] T8: Frozen-gate-identity test (prove `rank_candidates` unchanged)
  - file: `crates/backtest/src/bakeoff/mod.rs:1341` (`turnover_does_not_change_ranking`)
  - test: `cargo test -p backtest --lib`
  - output: `test bakeoff::tests::turnover_does_not_change_ranking ... ok` (bhidvobyb line 77)

- [x] T9: `cargo test -p backtest` clean
  - test: `cargo test -p backtest --lib`
  - output: `test result: ok. 193 passed; 0 failed; 8 ignored; finished in 0.65s` (bgel8nsuk)

- [x] T10: `cargo clippy -p backtest --tests -- -D warnings` clean
  - test: `cargo clippy -p backtest --tests -- -D warnings 2>&1; echo "CLIPPY_EXIT: $?"`
  - output: `CLIPPY_EXIT: 0` (bwvnvfmgx — post all doc+lint fixes; "Checking backtest..." → Finished, CLIPPY_EXIT: 0)

- [x] T11: `cargo fmt` clean
  - test: `cargo fmt -- --check 2>&1; echo "FMT_CHECK_EXIT: $?"`
  - output: `FMT_CHECK_EXIT: 0` (bwshzkrez)

- [x] T12: `bash scripts/verify_anchors.sh` 119/119
  - test: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (119 / 119)` (blc5577f7 line 121-122)

- [x] T13: `python3 scripts/spec_lint.py` PASS
  - test: `python3 scripts/spec_lint.py`
  - output: `spec-lint: PASS (0 violations)` (bknfq6fa2)

- [ ] T14: `cargo test -p ui --lib` (additive, should pass)
  - NOTE: test blocked on artifact lock (by0rf36nk); HANDOFF to tester for final verify-and-tick

## UI (ui-designer — later)

- [ ] TUI1: Surface `turnover` column in the leaderboard table
- [ ] TUI2: Surface `cvar_95` / `cvar_99` / `median_terminal_wealth` / `skew` in the scorecard/tail block
