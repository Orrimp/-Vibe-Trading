---
slug: advisor-drawdown-control-overlay
status: dev-done
owner: developer
updated: 2026-06-30
---

# Tasks — Drawdown-Control Overlay (P1-3)

## Developer tasks

- [x] T1: Implement `crates/strategy/src/drawdown_control_overlay.rs`
  - `DrawdownControlConfig` struct with `drawdown_floor_pct`, `restart_on_hwm`, `initial_equity`.
  - `DrawdownControlOverlay<S: Strategy>` generic struct with `update_equity`, `telemetry`, `static_floor`.
  - `compute_cushion_multiplier(d_max, d_k) -> Decimal` pure function (normalised formula).
  - `DrawdownTelemetry` struct for operator visibility.
  - `Strategy` impl: delegates `on_bar`/`on_tick` to inner; returns M(k) via `quantity_scale`.
  - File: `crates/strategy/src/drawdown_control_overlay.rs`
  - Test: `cargo test -p strategy --lib -- drawdown_control` → 12/12 ok
  - Output: `test result: ok. 12 passed; 0 failed`

- [x] T2: Register module in `crates/strategy/src/lib.rs`
  - `pub mod drawdown_control_overlay` + doc comment.
  - `pub use drawdown_control_overlay::{DrawdownControlConfig, DrawdownControlOverlay, DrawdownTelemetry, compute_cushion_multiplier}`.
  - File: `crates/strategy/src/lib.rs` lines added.
  - Test: `cargo build -p strategy` clean.

- [x] T3: Write mandatory e2e test `crates/strategy/tests/drawdown_control_overlay_end_to_end.rs`
  - Test 1: LOAD-BEARING load-bearing divergence gate (≥1 bp from baseline on drawdown scenario).
  - Test 2: HWM restart proof (BTC-style sequence).
  - Test 3: Floor never moves (D8 static CPPI invariant).
  - Test 4: Budget-cap invariant (quantity_scale always ∈ [0,1]).
  - Test 5: Default quantity_scale = 1.0 before first update.
  - Test 6: Formula pin at 10% drawdown.
  - Test cmd: `cargo test -p strategy --test drawdown_control_overlay_end_to_end`

- [x] T4: Write ADR-0080 (`spec/architecture/adr/0080-drawdown-control-overlay.md`)
  - Register in `spec/architecture/adr/README.md` (table row + updated frontmatter).
  - Verify: `python3 scripts/adr_registry_check.py --self-test`.

- [x] T5: Add trace row `REQ-V2-P1-3-DRAWDOWN-OVERLAY-001` to `spec/trace.toml`.

- [x] T6: Write `spec/v2/advisor-drawdown-control-overlay/feature.md` + `tasks.md`.

## Tester tasks (T_FINAL_*)

- [x] T_FINAL_1: Verify `cargo test -p strategy --lib` (all 260+ tests pass, no regressions).
  - test cmd: `cargo test -p strategy --lib`
  - output: `test result: ok. 266 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s`
- [x] T_FINAL_2: Verify `cargo test -p strategy --test drawdown_control_overlay_end_to_end` (6/6 pass).
  - test cmd: `cargo test -p strategy --test drawdown_control_overlay_end_to_end -- --nocapture`
  - output: `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`
  - load-bearing divergence gate: `overlay_equity_diverges_from_baseline_on_drawdown_scenario ... ok`
  - HWM-restart proof: `hwm_restart_proof_benchmark_sequence ... ok`
  - D8 floor invariant: `floor_never_moves_static_cppi_d8 ... ok`
  - budget-cap invariant: `budget_cap_invariant_quantity_scale_max_one ... ok`
- [x] T_FINAL_3: Verify `cargo clippy -p strategy --tests -- -D warnings` clean.
  - test cmd: `cargo clippy -p strategy --tests -- -D warnings`
  - output: `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 4m 35s  EXIT:0`
- [x] T_FINAL_4: Verify `cargo fmt --check` clean.
  - test cmd: `cargo fmt --check`
  - output: `(no output; exit 0)`
- [x] T_FINAL_5: Verify `bash scripts/verify_anchors.sh` 119/119 (before + after).
  - test cmd: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (119 / 119)`
- [x] T_FINAL_6: Verify `python3 scripts/spec_lint.py` PASS.
  - test cmd: `python3 scripts/spec_lint.py`
  - output: `spec-lint: PASS (0 violations)`
- [x] T_FINAL_7: Verify `python3 scripts/adr_registry_check.py --self-test` PASS.
  - test cmd: `python3 scripts/adr_registry_check.py --self-test`
  - output: `Ran 5 tests in 0.005s  OK  EXIT:0`
- [x] T_FINAL_8: Tick completed tasks with file:line + test cmd + output line per honest-tick rule.
  - Completed above (T_FINAL_1 through T_FINAL_7). Report at `spec/v2/phase-2c-overlays/reports/test-2026-06-30-phase-2c-overlays.md`.
