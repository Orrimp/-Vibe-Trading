---
slug: advisor-cost-model-opt-in
status: tester-done
owner: tester
updated: 2026-07-01
---

# Tasks — P1-6 Cost-Model Hardening + Venue-Trust Map

## Implementation tasks

- [x] **T1 — `SlippageModel::VolScaledSpread` variant**
  - File: `crates/cost/src/slippage.rs` (variant + `DEFAULT_VOL_SCALED_SPREAD` const)
  - Test command: `cargo test -p cost --lib slippage::tests`
  - Output: `test result: ok. 39 passed` (incl. all vol_scaled_* and anchor_safety_* tests)

- [x] **T2 — `apply_slippage_vol_scaled_bps` private body**
  - File: `crates/cost/src/slippage.rs:342–392`
  - Test command: `cargo test -p cost --lib slippage::tests::vol_scaled_constant_vol_closed_form`
  - Output: `test slippage::tests::vol_scaled_constant_vol_closed_form ... ok`

- [x] **T3 — `apply_slippage_model_with_returns` full dispatcher**
  - File: `crates/cost/src/slippage.rs:200–250`
  - Test command: `cargo test -p cost --lib slippage::tests::vol_scaled_widens_vs_linear_on_volatile_returns`
  - Output: `test slippage::tests::vol_scaled_widens_vs_linear_on_volatile_returns ... ok`

- [x] **T4 — `fee_sensitivity_report` helper**
  - File: `crates/cost/src/slippage.rs:254–295`
  - Test command: `cargo test -p cost --lib slippage::tests::fee_sensitivity_report_known_value`
  - Output: `test slippage::tests::fee_sensitivity_report_known_value ... ok`

- [x] **T5 — `lib.rs` re-exports updated**
  - File: `crates/cost/src/lib.rs:17`
  - Test command: `cargo build -p cost`
  - Output: `Finished dev profile`

- [x] **T6 — Unit tests: backward-compat default (load-bearing)**
  - File: `crates/cost/src/slippage.rs` (test `default_is_linear_bps_8`)
  - Test command: `cargo test -p cost --lib slippage::tests::default_is_linear_bps_8`
  - Output: `test slippage::tests::default_is_linear_bps_8 ... ok`

- [x] **T7 — Unit tests: zero-vol → base_bps only**
  - File: `crates/cost/src/slippage.rs` (test `vol_scaled_zero_vol_gives_base_bps`)
  - Test command: `cargo test -p cost --lib slippage::tests::vol_scaled_zero_vol_gives_base_bps`
  - Output: `test slippage::tests::vol_scaled_zero_vol_gives_base_bps ... ok`

- [x] **T8 — Unit tests: high-vol widens vs low-vol**
  - File: `crates/cost/src/slippage.rs` (test `vol_scaled_high_vol_widens_vs_low_vol`)
  - Test command: `cargo test -p cost --lib slippage::tests::vol_scaled_high_vol_widens_vs_low_vol`
  - Output: `test slippage::tests::vol_scaled_high_vol_widens_vs_low_vol ... ok`

- [x] **T9 — Anchor-safety proof test**
  - File: `crates/cost/src/slippage.rs` (test `anchor_safety_linear_unchanged_by_vol_scaled_variant`)
  - Test command: `cargo test -p cost --lib slippage::tests::anchor_safety_linear_unchanged_by_vol_scaled_variant`
  - Output: `test slippage::tests::anchor_safety_linear_unchanged_by_vol_scaled_variant ... ok`

- [x] **T10 — Venue-trust map dev-note**
  - File: `spec/dev-notes/venue-trust-map-2026-07-01.md`

- [x] **T11 — Spec feature.md + tasks.md**
  - Files: `spec/v2/advisor-cost-model-opt-in/feature.md` + `tasks.md`

- [x] **T12 — ADR-0081 + README update**
  - Files: `spec/architecture/adr/0081-cost-model-opt-in.md`
          + `spec/architecture/adr/README.md` (row added + `updated:` bumped)

- [x] **T13 — trace.toml row added**
  - File: `spec/trace.toml` (REQ-V2-P1-6-COST-MODEL-OPT-IN-001)

## Tester-owned (T_FINAL)

- [x] **T_FINAL_1 — `cargo test -p cost` full run** (all 39 pass)
  - Re-verified independently 2026-07-05: `cargo test -p cost --lib` →
    `test result: ok. 39 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.21s`.
    Confirms `default_is_linear_bps_8` and `anchor_safety_linear_unchanged_by_vol_scaled_variant`
    both PASS — the D6 opt-in-forever contract holds.
- [x] **T_FINAL_2 — `cargo test -p backtest --lib` incl. frozen-gate identity tests**
  - `test result: ok. 195 passed; 0 failed; 8 ignored; 0 measured; 0 filtered out; finished in 0.66s`.
    Targeted re-run: `cargo test -p backtest --lib does_not_change_ranking` →
    both `bakeoff::tests::turnover_does_not_change_ranking` and
    `bakeoff::scorecard::tests::scorecard_does_not_change_ranking` PASS.
- [x] **T_FINAL_3 — `cargo clippy -p cost --tests -- -D warnings` clean**
  - Run combined as `cargo clippy -p cost -p agent -p llm -p backtest --tests -- -D warnings`
    (all four Phase 2D crates in one invocation) — `Finished dev profile
    [unoptimized + debuginfo] target(s) in 12m 09s`, exit 0, zero warnings.
- [x] **T_FINAL_4 — `bash scripts/verify_anchors.sh` → 119/119 AFTER**
  - `ANCHORS PASS  (119 / 119)`, verified both at session start and after this
    tasks.md edit.
- [x] **T_FINAL_5 — `python3 scripts/spec_lint.py` PASS**
  - `spec-lint: PASS (0 violations)`.
- [x] **T_FINAL_6 — `python3 scripts/adr_registry_check.py --self-test` OK**
  - `Ran 5 tests in 0.011s\n\nOK`; production check (`adr_registry_check.py`,
    no flag) exits 0.

Full bundled report: `spec/v2/phase-2d/reports/test-2026-07-01-phase-2d.md`.
