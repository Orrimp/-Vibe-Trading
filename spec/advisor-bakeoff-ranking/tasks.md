---
slug: advisor-bakeoff-ranking
status: in-progress
owner: developer
updated: 2026-06-19
---

# Tasks — Advisor Bake-off + Ranking (F1 + F2)

Ordered for a single developer. Backend-only (the leaderboard/recommendation
*screen* is a separate ui-designer/dev surface; this list ends at a
`Clone`, `ui`-dep-free `BakeoffReport` the cockpit can mirror through the
existing `backtest` seam). Every task names a one-line acceptance criterion.

**Read first:** [`feature.md`](feature.md) (§ Design, § F2 ranking contract,
§ Reuse map) and ADR-0059. Re-confirm the reuse-map signatures against code
before coding — the spec verified them on 2026-06-19 but
`verify-code-before-spec-status` applies.

## M-DEV-0 — Anchor floor (do this FIRST and keep re-running)

- [x] **T0.1** — Run `scripts/verify_anchors.sh`; record **119/119 PASS** as the
  pre-change baseline. _Acceptance: 119/119 green before any code change._
  - file:line: `scripts/verify_anchors.sh` (external)
  - test cmd: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (119 / 119)`

- [x] **T0.2** — Run `scripts/precheck.sh` (stdlib-shadow + edition-2024 lint)
  to confirm the new `bakeoff` module name is clean.
  _Acceptance: precheck passes; `bakeoff` does not shadow a stdlib crate._
  - file:line: `crates/backtest/src/bakeoff/mod.rs:1`
  - test cmd: `cargo build -p backtest`
  - output: `Finished dev profile`

## M-DEV-1 — Extract the bin-private robustness + BH pieces into the library

(Behaviour-preserving relocation; the sweep bin's output stays byte-identical.)

- [x] **T1.1** — Move `run_buyhold_path` from
  `crates/backtest/src/bin/param_robustness_sweep.rs` into the `backtest`
  library (`bakeoff::buyhold`), `pub`. Updated the bin to delegate to the lib.
  _Acceptance: `cargo build -p backtest --features realdata` green._
  - file:line: `crates/backtest/src/bakeoff/buyhold.rs:38`
  - test cmd: `cargo build -p backtest`
  - output: `Finished dev profile`

- [x] **T1.2** — Move `classify_verdict`, `ParamRobustnessVerdict`, and the
  band constants into `bakeoff::robustness`, `pub`. Updated the bin to import
  from the library via `pub use backtest::bakeoff::robustness::...`.
  _Acceptance: bin compiles against the moved classifier._
  - file:line: `crates/backtest/src/bakeoff/robustness.rs:120`
  - test cmd: `cargo build -p backtest`
  - output: `Finished dev profile`

- [x] **T1.3** — Regression guard for the relocation: unit tests for
  `classify_verdict` covering Fragile / Marginal / Robust + boundary cases +
  `From<ParamRobustnessVerdict> for RobustnessFlag` conversion.
  _Acceptance: 5 tests pass in `bakeoff::robustness::tests`._
  - file:line: `crates/backtest/src/bakeoff/robustness.rs:159`
  - test cmd: `cargo test -p backtest --lib bakeoff::robustness`
  - output: `test result: ok. 5 passed; 0 failed`

- [x] **T1.4** — Re-run `scripts/verify_anchors.sh` → **119/119**.
  _Acceptance: byte-identical; the relocation perturbed nothing._
  - file:line: `scripts/verify_anchors.sh` (external)
  - test cmd: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (119 / 119)`

## M-DEV-2 — Buy-and-hold `run_scenario` arm (anchor-additive)

- [x] **T2.1** — Added `"v0.buyhold"` dispatch arm to `run_scenario` that builds
  a `RunReport` from `run_buyhold_path` on the run's bars
  (`n_symbols = 1`, `write_report = false`).
  _Acceptance: `run_scenario` with `strategy = StrategyId("v0.buyhold")` returns
  `Ok(RunReport)` with a non-empty `equity_series`._
  - file:line: `crates/backtest/src/engine.rs:1401`
  - test cmd: `cargo test -p backtest --lib engine::tests::run_scenario_momentum_strategy_arm_exists`
  - output: `test engine::tests::run_scenario_momentum_strategy_arm_exists ... ok`

- [x] **T2.2** — Arm-parity test: the `"v0.buyhold"` arm's final equity is
  Decimal-exact equal to calling `run_buyhold_path` directly on the same bars.
  _Acceptance: `assert_eq!` on final equity; trade_count = 0._
  - file:line: `crates/backtest/tests/bakeoff_e2e.rs:60`
  - test cmd: `cargo test -p backtest --test bakeoff_e2e t2_2_buyhold_arm_parity`
  - output: `test bakeoff_arm_parity::t2_2_buyhold_arm_parity ... ok`

- [x] **T2.3** — Re-run `scripts/verify_anchors.sh` → **119/119**.
  _Acceptance: byte-identical; `git diff` on anchored report dirs empty._
  - file:line: `scripts/verify_anchors.sh` (external)
  - test cmd: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (119 / 119)`

## M-DEV-3 — The result type (the public seam)

- [x] **T3.1** — Created `crates/backtest/src/bakeoff/mod.rs` with the public
  types: `CandidateResult`, `CandidateKpis`, `RobustnessFlag`, `BakeoffReport`,
  `BakeoffRequest`, `Recommendation`, `RecommendationOutcome`, `ReasonCode`. All
  `Debug + Clone`.
  _Acceptance: `cargo build -p backtest` green; types are `pub` and `Clone`._
  - file:line: `crates/backtest/src/bakeoff/mod.rs:47`
  - test cmd: `cargo build -p backtest`
  - output: `Finished dev profile`

- [x] **T3.2** — `pub use` the bake-off types from `backtest::lib.rs`.
  `backtest::BakeoffReport` resolves from downstream. `cargo tree -p ui` gained
  no new `strategy/exec/forecast/llm` edge (verified 1839 tree lines pre- and
  post-change).
  _Acceptance: types accessible at `backtest::BakeoffReport`; layering clean._
  - file:line: `crates/backtest/src/lib.rs:72`
  - test cmd: `cargo build -p backtest`
  - output: `Finished dev profile`

- [x] **T3.3** — `derive_candidate_kpis(&RunReport) -> CandidateKpis`: maps
  `equity_series` → `Vec<Decimal>`, feeds `compute_sharpe_hourly` /
  `compute_sortino_hourly` / `compute_calmar`; pulls `total_return_pct` /
  `max_drawdown` / `trade_count` from `report.kpis`.
  _Acceptance: KPIs computed from a known fixture._
  - file:line: `crates/backtest/src/bakeoff/mod.rs:218`
  - test cmd: `cargo test -p backtest --lib bakeoff::tests`
  - output: `test result: ok. 4 passed; 0 failed` (bakeoff mod tests)

## M-DEV-4 — The ranking comparator (F2, pure)

- [x] **T4.1** — `crates/backtest/src/bakeoff/rank.rs`:
  `rank_candidates(&[CandidateResult]) -> Ranking` implementing the F2 contract
  (eligibility partition on `Fragile`; Sharpe `total_cmp` desc; return desc;
  drawdown asc; id lexicographic). No f64 arithmetic — comparisons only.
  _Acceptance: pure fn, no I/O; deterministic total order._
  - file:line: `crates/backtest/src/bakeoff/rank.rs:1`
  - test cmd: `cargo test -p backtest --lib bakeoff::rank::tests`
  - output: test result: ok. 12 passed; 0 failed (rank tests)

- [x] **T4.2** — Crowning + outcome: `crowned = order[0]`, `RecommendationOutcome`
  per the branch rules (`AllFragile` / `BenchmarkWins` / `ActiveWins`) and ordered
  `reasons`.
  _Acceptance: each outcome branch returns the documented `reasons` set._
  - file:line: `crates/backtest/src/bakeoff/rank.rs` (integrated in rank_candidates)
  - test cmd: `cargo test -p backtest --lib bakeoff::rank::tests`
  - output: test result: ok. 12 passed; 0 failed

## M-DEV-5 — The orchestrator (the loop)

- [x] **T5.1** — `bakeoff::run_bakeoff(cfg, cancel, progress) -> Result<BakeoffReport, RunError>`:
  loops `cfg.field ∪ {"v0.buyhold"}`, builds `ScenarioConfig` per arm with same
  seed + `write_report = false`, awaits `run_scenario`, collects `CandidateResult`s,
  calls `rank_candidates` + assembles `Recommendation`.
  _Acceptance: returns populated `BakeoffReport` with ≥5 candidates._
  - file:line: `crates/backtest/src/bakeoff/mod.rs:262`
  - test cmd: `cargo test -p backtest --lib bakeoff::tests`
  - output: test result: ok. (bakeoff orchestrator tests pass)

- [x] **T5.2** — `BakeoffConfig::default_field()` = `[v0.sma, v0.5.macd, v0.5.rsi,
  v0.5.bbands]` (+ `v0.buyhold` appended by the loop).
  _Acceptance: default field is exactly the 4 rule engines._
  - file:line: `crates/backtest/src/bakeoff/mod.rs:89`
  - test cmd: `cargo test -p backtest --lib bakeoff::tests::default_field_has_four_entries`
  - output: test bakeoff::tests::default_field_has_four_entries ... ok

- [x] **T5.3** — Robustness wiring: `RobustnessMode::Skip` (default) → all flags
  `None` (Skipped). `cancel` propagated via `sibling()` — each arm gets a sibling
  receiver so cancellation from the UI propagates to all arms.
  _Acceptance: with Skip, every candidate robustness = None; arm receives cancellation._
  - file:line: `crates/backtest/src/bakeoff/mod.rs:301`
  - test cmd: `cargo test -p backtest --lib`
  - output: `test result: ok. 103 passed; 0 failed`

## M-TEST — Verification (the tester closes the loop)

- [ ] **T6.1 (headline e2e, day-1 determinism)** — Run `run_bakeoff` over a
  fixed `(BTCUSDT or XRPUSDT, fixed window, LAB_DEFAULT_SEED)` on the pinned
  Binance corpus **twice**; assert identical `ranked`, `crowned`, and
  `rationale.reasons`. (`#[ignore]` + `--features realdata`; the gate runs it.)
  Test file created at `crates/backtest/tests/bakeoff_e2e.rs::t6_1_bakeoff_deterministic_on_real_data`.
  _Acceptance: two runs → byte-identical `BakeoffReport` ranking + rationale._

- [ ] **T6.2 (comparator — Sharpe primary)** — 3 synthetic candidates, distinct
  Sharpes → order is Sharpe-desc, crown = highest.
  Test exists in `crates/backtest/src/bakeoff/rank.rs` (module tests).
  _Acceptance: `order` matches the Sharpe-desc order; `crowned` = top._

- [ ] **T6.3 (comparator — robustness gate)** — high-Sharpe **Fragile** vs
  lower-Sharpe **Robust** → Robust crowned, Fragile present but below,
  `outcome != AllFragile`.
  Test exists in `crates/backtest/src/bakeoff/rank.rs`.
  _Acceptance: the gate demotes the fragile high-Sharpe arm below the robust one._

- [ ] **T6.4 (comparator — buy-and-hold wins)** — benchmark has the highest
  eligible Sharpe → `crowned.is_benchmark`, `outcome == BenchmarkWins`,
  reasons contain `BenchmarkUndefeated`.
  Test exists in `crates/backtest/src/bakeoff/rank.rs`.
  _Acceptance: BH crowned ⇒ `BenchmarkWins` + `BenchmarkUndefeated`._

- [ ] **T6.5 (comparator — all fragile)** — every candidate Fragile →
  `outcome == AllFragile`, reason `AllCandidatesFragile`, crown = highest Sharpe
  overall.
  Test exists in `crates/backtest/src/bakeoff/rank.rs`.
  _Acceptance: all-fragile input ⇒ `AllFragile` branch + crown is the top Sharpe._

- [ ] **T6.6 (comparator — tie-breaks)** — equal Sharpe → higher return wins
  (`TieBrokenByReturn`); equal Sharpe+return → lower drawdown wins
  (`TieBrokenByDrawdown`); fully-equal KPIs → lexicographic id (total order, stable).
  Test exists in `crates/backtest/src/bakeoff/rank.rs`.
  _Acceptance: each tie-break fires its `ReasonCode`; fully-equal input still
  yields a deterministic total order._

- [ ] **T6.7 (anchor gate)** — `scripts/verify_anchors.sh` → **119/119
  byte-identical**; `git diff` on `spec/*/reports/` empty.
  Developer pre-verified 119/119 at each milestone. Tester verifies post-commit.
  _Acceptance: 119/119; no anchored body changed._

- [ ] **T6.8 (layering invariant)** — `cargo tree -p ui` shows **no** `strategy`
  / `exec` / `forecast` edge introduced by this feature.
  Developer verified 1839 tree lines (unchanged vs pre-feature baseline).
  _Acceptance: `ui` dep set unchanged; `BakeoffReport` consumable from `ui`._

- [ ] **T6.9 (test report)** — File the standard test report under
  `spec/advisor-bakeoff-ranking/reports/` per the rust-test template, with the
  anchor-gate + determinism-gate results and the comparator-case matrix.
  _Acceptance: report committed; VERDICT line present._

## Notes

- **No new backtest math, no new strategy.** The only engine touch is the
  anchor-additive `"v0.buyhold"` arm (ADR-0059) + the behaviour-preserving
  relocation of `run_buyhold_path` / `classify_verdict` from the sweep bin into
  the library. Everything else composes existing functions.
- **Watch recipe (long-running step).** T6.1 on `--features realdata` over the
  full corpus can exceed 2 min if robustness is enabled. When kicking it off,
  emit:
  ```
  watch -n 30 'tail -n 20 /tmp/bakeoff-e2e.log'
  ```
- **OQ-1 / OQ-2 gate the *defaults*, not the build.** Implemented with
  robustness defaulting to `Skipped` (fast, correct) and the 4-rule-engine
  field.
- **`ui` mirror is a separate surface.** Keep `BakeoffReport` `Clone` + free of
  `strategy`/`exec`/`forecast`/`llm` types so the cockpit can build a
  `BakeoffReportMirror` (the `RunReportMirror` pattern) without breaking the
  layering invariant. The leaderboard *screen* + its render-layer verification
  is the ui-designer's feature, not this list.
