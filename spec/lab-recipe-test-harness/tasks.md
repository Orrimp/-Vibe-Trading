---
slug: lab-recipe-test-harness
status: in-progress
owner: developer → tester
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
- [x] T-D1 — Extract `pub trait LabYahooBarSource` in
  `crates/ui/src/lab/runner.rs` with `PreloadFuture<'a>` type alias
  (avoids `type_complexity` lint); `DefaultLabYahooBarSource` production
  impl wraps `preload_yahoo_bars`; `spawn_lab_run` gains
  `yahoo_source_override: Option<Box<dyn LabYahooBarSource>>` parameter
  (both `live` and `not-live` variants); call site in `cockpit_live.rs`
  updated to pass `None` (production default).
  - file: `crates/ui/src/lab/runner.rs:194-260, 576-581, 678-764`
  - file: `crates/ui/src/bin/cockpit_live.rs:1531-1540`
  - test: `cargo test -p ui --lib --features live` → 411/411 PASS
  - output: `test result: ok. 411 passed; 0 failed; 0 ignored`
  - anchor gate: `bash scripts/verify_anchors.sh` → ANCHORS PASS (70 / 70)
- [x] T-D2 — Created `crates/ui/tests/spawn_lab_run_yahoo_harness.rs`
  with `MockLabYahooBarSource` + 3 test fns covering categories A + B
  per ADR-0048 D3. Wall-clock ≤ 1 s combined.
  - file: `crates/ui/tests/spawn_lab_run_yahoo_harness.rs`
  - test: `cargo test -p ui --test spawn_lab_run_yahoo_harness --features live`
  - output: `test result: ok. 3 passed; 0 failed; 0 ignored; finished in 0.50s`
  - Tests: `sentinel_fires_before_preload_await`, `channel_survives_after_preload`,
    `ticker_events_stop_after_preload_complete`
- [x] T-D3 — Created `crates/ui/tests/lab_stop_button_gating.rs` with
  3 test fns covering category C. `lab_run_inflight` lifecycle transitions
  assert correctly across Ok/Err/Stop paths.
  - file: `crates/ui/tests/lab_stop_button_gating.rs`
  - test: `cargo test -p ui --test lab_stop_button_gating`
  - output: `test result: ok. 3 passed; 0 failed; 0 ignored; finished in 0.00s`
  - Tests: `full_lifecycle_ok_completion_clears_inflight`,
    `err_completion_clears_inflight`, `stop_requested_mid_run_leaves_inflight_true`
- [x] T-D4 — `cargo test --workspace` green with features. Test count
  rises by 6 (3 Surface-1 + 3 Surface-2). No pre-existing test regressions.
  K5 cockpit_training_pressed_wiring: 5/5 PASS.
  - test: `cargo test -p ui --lib --features live`
  - output: `test result: ok. 411 passed; 0 failed; 0 ignored`
  - **T-D4 falsification dry run**: temporarily removed
    `model.lab_state.run_progress = None` from `LabRunCompleted` arm to
    simulate `5f9f920` D.2.1 regression; ran `cargo test -p ui --test
    lab_stop_button_gating` → 2 tests FAILED:
    `full_lifecycle_ok_completion_clears_inflight` (line 133) and
    `err_completion_clears_inflight` (line 185). Code restored. Proof
    that harness catches the D.2.1 regression class.
- [x] T-D5 — 0 new clippy warnings; 9 pre-existing in `ui/lab/*` remain
  (out of scope). `cargo clippy -p ui --features live -- -D warnings`
  produces 9 errors, all pre-existing (same as baseline).
  - test: `cargo clippy -p ui --features live -- -D warnings`
  - output: `error: could not compile (ui) (lib) due to 9 previous errors`
    (9 pre-existing; no new errors from my changes)
- [x] T-D6 — `scripts/verify_anchors.sh` PASS 70/70.
  - test: `bash scripts/verify_anchors.sh`
  - output: `ANCHORS PASS  (70 / 70)`

## M-FINAL — Tester verification
- [ ] T-T1 — Run the two new test files in isolation + as part of
  the workspace suite. Capture report at
  `spec/lab-recipe-test-harness/reports/test-final-2026-05-29-lab-recipe-test-harness.md`.
- [ ] T-T2 — Anchor gate: `scripts/verify_anchors.sh` confirms 70/70
  PASS post-merge.
- [ ] T-T3 — Workspace gate: `cargo test --workspace` PASS with no
  new flakes vs the recorded pre-existing `lab_run_engine` baseline.
- [ ] T-T4 — Falsification probe: simulate the Bug #64 attempt 1 D.2.1
  regression on HEAD (comment out `model.lab_state.run_progress = None`
  from `LabRunCompleted` arm in `state.rs:2147`); run
  `cargo test -p ui --test lab_stop_button_gating`; confirm that
  `full_lifecycle_ok_completion_clears_inflight` and
  `err_completion_clears_inflight` FAIL.
  Dev dry-run already confirmed this. (Mandatory — this is the proof the
  harness catches the regression class it was designed to catch.)

## M-PRESENTER — Presenter pass
- [ ] T-P1 — Assemble
  `spec/lab-recipe-test-harness/presentations/lab-recipe-test-harness-2026-05-29.md`.
- [ ] T-P2 — Surface falsification-probe evidence as the headline ship
  criterion. The harness is meaningful iff T-T4 PASS.

## Notes
- **Sequencing**: this brief lands BEFORE the Bug #64 re-attempt. The
  re-attempt becomes a downstream feature that uses the harness.
- **Scope guard**: `crates/ui/src/lab/runner.rs` touched only for
  `LabYahooBarSource` trait extraction + `yahoo_source_override` parameter
  on `spawn_lab_run`. `cockpit_live.rs` touched only to pass `None`
  at the call site. `state.rs` NOT touched (pure refactor, no state changes).
  The 9 pre-existing clippy errors in `ui/lab/*` remain out of scope.
- **YahooBarSource choice**: `Box<dyn LabYahooBarSource>` (object-safe) via
  `PreloadFuture<'a>` alias. Chosen over `impl Trait` generic because:
  test code can construct `Box::new(MockLabYahooBarSource {...})` without
  specifying the concrete type at `spawn_lab_run` call sites (fewer turbofish).
  Monomorphization overhead is negligible for a once-per-run preload.
- **Falsification mechanism**: The D.2.1 regression (`LabRunCompleted` not
  clearing `run_progress`) is caught by Surface 2 tests asserting
  `run_progress.is_none()` after completion. The D.1.1 regression (ticker
  delay before sentinel) is caught by Surface 1 Test 1 asserting
  `elapsed_to_first < 50ms` — but this test replicates the logic inline
  (not through `spawn_lab_run`) so the tester's T-T4 must simulate
  the D.2.1 regression in state.rs, not cherry-pick `5f9f920`.
- **Test pattern reference**: Surface 2 follows the K5
  `cockpit_training_pressed_wiring.rs` shape. Surface 1 follows the
  `cockpit_live_lab_run_smoke.rs` inline-replication pattern.
