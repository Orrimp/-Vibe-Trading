---
slug: lab-recipe-test-harness
status: in-progress
owner: developer
updated: 2026-05-28
---

# Tasks — lab-recipe-test-harness v0.1.0

## M0 — Brief authored (analyst-equivalent)
- [x] M0.1 — feature.md authored; Why + Requirements + Design folded in
  by architect (analyst pass merged per orchestrator brief).
- [x] M0.2 — backlog Active row appended.
- [x] M0.3 — trace row `REQ-LAB-RECIPE-TEST-HARNESS-001` opened at
  `arch-done`.

## M-OD — Operator-decide
_Empty — architect locked all decisions at sensible defaults via
ADR-0048 D1–D6. No operator-decide questions raised at v0.1.0._

## M-T1 — Architect ratification
- [x] M-T1.1 — ADR-0048 authored at
  `spec/architecture/adr/0048-lab-recipe-test-harness.md`.
- [x] M-T1.2 — Harness pattern picked: **(d) Combination** (boundary
  test + gating state-machine test).
- [x] M-T1.3 — `YahooBarSource` trait extraction sketched in Design.

## M-DEV — Developer pass
- [ ] T-D1 — Extract `pub trait YahooBarSource` in
  `crates/ui/src/lab/runner.rs`; production default impl wraps the
  existing parquet+http path. _Acceptance: anchor row 70 SHA
  byte-identical via `cargo test -p backtest --test determinism`._
- [ ] T-D2 — Create `crates/ui/tests/spawn_lab_run_yahoo_harness.rs`
  with `MockYahooBarSource` + 3 test fns covering categories A + B
  per ADR-0048 D3. _Acceptance: 3/3 PASS; wall-clock ≤ 5 s combined._
- [ ] T-D3 — Create `crates/ui/tests/lab_stop_button_gating.rs` with
  2 test fns covering category C. _Acceptance: 2/2 PASS;
  `lab_run_inflight` lifecycle transitions assert correctly._
- [ ] T-D4 — `cargo test --workspace` green; test count rises by
  exactly 5 (3 + 2); no pre-existing test regressions.
- [ ] T-D5 — `cargo clippy --workspace -- -D warnings` clean
  (excluding the 9 pre-existing `ui/lab/*` warnings out of scope).
- [ ] T-D6 — `scripts/verify_anchors.sh` PASS 70/70.

## M-FINAL — Tester verification
- [ ] T-T1 — Run the two new test files in isolation + as part of
  the workspace suite. Capture report at
  `spec/lab-recipe-test-harness/reports/test-final-2026-05-29-lab-recipe-test-harness.md`.
- [ ] T-T2 — Anchor gate: `scripts/verify_anchors.sh` confirms 70/70
  PASS post-merge.
- [ ] T-T3 — Workspace gate: `cargo test --workspace` PASS with no
  new flakes vs the recorded pre-existing `lab_run_engine` baseline.
- [ ] T-T4 — Falsification probe: revert the Bug #64 attempt 1 commit
  `5f9f920` onto HEAD with the harness present; confirm at least ONE
  of the new tests FAILS. (Mandatory — this is the proof the harness
  catches the regression class it was designed to catch.)

## M-PRESENTER — Presenter pass
- [ ] T-P1 — Assemble
  `spec/lab-recipe-test-harness/presentations/lab-recipe-test-harness-2026-05-29.md`.
- [ ] T-P2 — Surface falsification-probe evidence as the headline ship
  criterion. The harness is meaningful iff T-T4 PASS.

## Notes
- **Sequencing**: this brief lands BEFORE the Bug #64 re-attempt. The
  re-attempt becomes a downstream feature that uses the harness.
- **Scope guard**: do NOT touch `crates/ui/src/lab/runner.rs::spawn_lab_run`
  beyond the `YahooBarSource` extraction. Do NOT touch
  `crates/ui/src/state.rs`. The 9 pre-existing clippy errors in
  `ui/lab/*` are OUT of scope.
- **Test pattern reference**: Surface 2 follows the K5
  `cockpit_training_pressed_wiring.rs` shape. Surface 1 follows the
  `cockpit_live_lab_run_smoke.rs` shape but pivots upstream from
  `run_scenario` to `spawn_lab_run` itself.
