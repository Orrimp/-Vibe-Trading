---
slug: advisor-confidence-not-verdict
status: dev-done
owner: developer
version: 0.1.0
updated: 2026-06-30
---

# Tasks — advisor-confidence-not-verdict (P0-3)

## Completed

- [x] **T1** — `ScorecardSummary` struct + `Scorecard::summary()` method
  - file: `crates/backtest/src/bakeoff/scorecard.rs`
  - test cmd: `cargo test -p backtest --lib -- bakeoff::scorecard::tests::scorecard_summary`
  - output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 201 filtered out; finished in 0.00s`
  - impl: `ScorecardSummary` with 4 fields; returns `None` when `n_candidates == 0`

- [x] **T2** — `confidence` field on `ForwardRunConfig` + `ForwardPlan`
  - file: `crates/agent/src/config.rs`
  - test: `cargo test -p agent` (field present; `make_cfg()` helper updated)
  - impl: `pub confidence: Option<backtest::bakeoff::ScorecardSummary>` on both structs

- [x] **T3** — `ForwardPlan` construction in `plan.rs` propagates `confidence`
  - file: `crates/agent/src/plan.rs` — `confidence: cfg.confidence`
  - test: `cargo test -p agent` — `test_forward_plan_builds_from_cfg` passes

- [x] **T4** — `ConfidenceSummaryView` + `confidence` field on `ForwardPlanView`
  - file: `crates/ui/src/forward_plan/state.rs`
  - test: `cargo test -p ui --lib` — state module tests pass

- [x] **T5** — Mirror via `forward_plan/adapter.rs` (`#[cfg(feature = "live")]`)
  - file: `crates/ui/src/forward_plan/adapter.rs`
  - test: build with `--features live` (adapter compiles)
  - impl: `confidence_summary_view()` mapping function

- [x] **T6** — Populate `confidence` at cockpit_live.rs launch sites
  - file: `crates/ui/src/bin/cockpit_live.rs`
  - impl: two sites — bakeoff-completion path + promote path

- [x] **T7** — Export `ConfidenceSummaryView` from `forward_plan/mod.rs`
  - file: `crates/ui/src/forward_plan/mod.rs`

- [x] **T8** — UI copy relabel + 14 new P0-3 string constants
  - file: `crates/ui/src/strings.rs`
  - constants: `FORWARD_PLAN_HEADLINE` → "Confidence check", `FORWARD_PLAN_CAPTION`,
    and 14 new P0-3 constants (candidates / DSR / beats-holding / min BTL)
  - inventory: all new constants added to `all_strings()`

- [x] **T9** — Confidence summary block in `screens/forward_plan.rs`
  - file: `crates/ui/src/screens/forward_plan.rs`
  - impl: `confidence_block()` with 4 fact rows; added to `ready_pane()` after horizon block

- [x] **T10** — Fixtures: `confidence: None` on all prior `ForwardPlanView` constructors
  - file: `crates/ui/src/fixtures.rs` (8 struct literals updated)

- [x] **T11** — New fixtures: `fake_forward_plan_with_confidence()` + `fake_cockpit_forward_plan_with_confidence()`
  - file: `crates/ui/src/fixtures.rs`
  - impl: `n_candidates: 18, deflated_sharpe: 0.87, crown_clears_dsr: false, min_btl_years: 6.4`

- [x] **T12** — Render test: `forward_plan_confidence_render.rs`
  - file: `crates/ui/tests/forward_plan_confidence_render.rs`
  - test cmd: `cargo test -p ui --test forward_plan_confidence_render --features fixtures`
  - output: `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 87.94s`
  - PNGs: `/tmp/forward_plan_confidence_render.png` (with confidence) + `/tmp/forward_plan_no_confidence_render.png` (without)
  - render: headline "Confidence check" (relabelled) + 4-row block ("Strategies tried: 18 / Deflated confidence: 87% / Beats holding?: ⚠ Not yet / Min history: 6.4 yr")

- [x] **T13** — Spec scaffold: `feature.md` + `tasks.md`
  - files: `spec/v2/advisor-confidence-not-verdict/feature.md` + this file

- [x] **T14** — `REQ-V2-P0-CONFIDENCE-001` row in `spec/trace.toml`
  - file: `spec/trace.toml`

## Tester verification items (T_FINAL_*)

- [ ] **T_FINAL_1** — `cargo test -p backtest` clean
- [ ] **T_FINAL_2** — `cargo test -p ui --lib` clean
- [ ] **T_FINAL_3** — `cargo test -p ui --test forward_plan_confidence_render --features fixtures` PASS (describe PNG output)
- [ ] **T_FINAL_4** — `cargo clippy -p backtest --tests -- -D warnings` clean
- [ ] **T_FINAL_5** — `cargo clippy -p ui --tests --features fixtures -- -D warnings` clean
- [ ] **T_FINAL_6** — `cargo fmt --check` clean
- [ ] **T_FINAL_7** — `bash scripts/verify_anchors.sh` → 119/119 PASS
- [ ] **T_FINAL_8** — `python3 scripts/spec_lint.py` PASS
- [ ] **T_FINAL_9** — cockpit smoke: `cargo build -p ui --features fixtures,live` clean, 0 panics
