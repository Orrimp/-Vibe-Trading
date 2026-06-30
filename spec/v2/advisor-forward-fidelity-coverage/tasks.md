---
slug: advisor-forward-fidelity-coverage
status: dev-done
owner: developer
updated: 2026-06-30
---

# Tasks — R1 Forward-Fidelity Coverage

## Implementation tasks

- [x] **T1** — Extend `build_registry_for` (`crates/agent/src/runtime.rs`) with the 5 ADR-0071 DSL arms (donchian_break, donchian_floor, vol_breakout, roc_momentum, obv) — load TOML exactly like MACD/RSI/BBands arms.
  - file:line: `crates/agent/src/runtime.rs:476–530` (the 5 new DSL match arms)
  - test: `cargo test -p agent --test forward_run_engine_fidelity r1_donchian_break_builds_not_bails r1_donchian_floor_builds_not_bails r1_vol_breakout_builds_not_bails r1_roc_momentum_builds_not_bails r1_obv_builds_not_bails`
  - output: `test result: ok. 5 passed; 0 failed`

- [x] **T2** — Extend `build_registry_for` with the 6 ADR-0067 ensemble arms — route all `v0.8.vote.*` to `build_ensemble` (already knows all 8).
  - file:line: `crates/agent/src/runtime.rs:532–552` (the 6-arm widening block)
  - test: `cargo test -p agent --test forward_run_engine_fidelity r1_ensemble_trend_pair_builds_not_bails r1_ensemble_tr_mr_macd_rsi_builds_not_bails r1_ensemble_tr_mr_sma_bb_builds_not_bails r1_ensemble_any1of4_builds_not_bails r1_ensemble_k2of4_builds_not_bails r1_ensemble_k3of4_builds_not_bails`
  - output: `test result: ok. 6 passed; 0 failed`

- [x] **T3** — Extend `build_registry_for` with ADR-0072 `v0.dvol_regime` (DvolRegimeStrategy, empty as_of) and ADR-0073 `v0.macro_riskon` (AlwaysLongStrategy graceful degradation).
  - file:line: `crates/agent/src/runtime.rs:554–595` (dvol + macro arms)
  - test: `cargo test -p agent --test forward_run_engine_fidelity r1_dvol_regime_builds_not_bails r1_macro_riskon_builds_not_bails`
  - output: `test result: ok. 2 passed; 0 failed`

- [x] **T4** — Mirror all 14 ids in `build_forward_plan_from_registry` (`crates/agent/src/plan.rs`).
  - file:line: `crates/agent/src/plan.rs:284–370` (R1 plan describer arms)
  - test: `cargo test -p agent --test forward_run_engine_fidelity` (all 21 pass)
  - output: `test result: ok. 21 passed; 0 failed`

- [x] **T5** — Extend `crates/agent/tests/forward_run_engine_fidelity.rs` with 13 new `r1_*` forward-buildability tests (one per id family).
  - file:line: `crates/agent/tests/forward_run_engine_fidelity.rs:363–558` (R1 test block)
  - test: `cargo test -p agent --test forward_run_engine_fidelity`
  - output: `test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`

- [x] **T6** — Write ADR-0077 at `spec/architecture/adr/0077-forward-fidelity-coverage.md` and register in `spec/architecture/adr/README.md`.
  - file:line: `spec/architecture/adr/0077-forward-fidelity-coverage.md` (new file)
  - test: `python3 scripts/adr_registry_check.py --self-test`
  - output: (see ADR registry check in final report)

- [x] **T7** — Add `REQ-V2-R1-FORWARD-COVERAGE-001` to `spec/trace.toml`.
  - file:line: `spec/trace.toml` (new [[req]] row)

- [x] **T8** — Create `spec/v2/advisor-forward-fidelity-coverage/feature.md` + `tasks.md`.
  - file:line: `spec/v2/advisor-forward-fidelity-coverage/feature.md`

## Verification gates (tester)

- [ ] **T_FINAL_01** — `cargo test -p agent` all targets clean (21+ tests pass).
- [ ] **T_FINAL_02** — `cargo test -p backtest --lib` clean (195/195, no regressions).
- [ ] **T_FINAL_03** — `cargo test -p ui --lib --features fixtures` clean (583/583).
- [ ] **T_FINAL_04** — `cargo clippy -p agent --tests -- -D warnings` clean.
- [ ] **T_FINAL_05** — `cargo clippy -p backtest --tests -- -D warnings` clean.
- [ ] **T_FINAL_06** — `cargo clippy -p ui --tests --features fixtures -- -D warnings` clean.
- [ ] **T_FINAL_07** — `cargo fmt --check` clean.
- [ ] **T_FINAL_08** — `bash scripts/verify_anchors.sh` → 119/119.
- [ ] **T_FINAL_09** — `python3 scripts/spec_lint.py` PASS.
- [ ] **T_FINAL_10** — `python3 scripts/adr_registry_check.py --self-test` PASS.
