---
slug: lab-recipe-test-harness-v0.2.0-cross-surface-extension
status: in-progress
owner: developer
updated: 2026-05-29
---

# Tasks — lab-recipe-test-harness v0.2.0 cross-surface extension

## M0 — Analyst (DONE 2026-05-29)

- [x] T-M0.1 — Recipe inventory at R1 (9 surfaces enumerated) — _accept: each Recipe has file:line + regression class + coverage status_
- [x] T-M0.2 — feature.md R1-R4 + R-NR + K1-K4 + H1-H4 + Q1-Q2 + 4-cell verdict tree — _accept: <200 lines, durable contract framing_
- [x] T-M0.3 — backlog Active row appended under § Process / tooling — _accept: PROMOTED Idea → Active 2026-05-29 annotation_
- [x] T-M0.4 — trace row `REQ-LAB-RECIPE-TEST-HARNESS-V0-2-0-001` opened `proposed` — _accept: appended at EOF spec/trace.toml_

## M-T1 — Architect (DONE 2026-05-29)

- [x] T-T1.1 — ratify Q1 + Q2 — _outcome: Q1=(a) + Q2=(a) both DURABLE, locked at § Design intro_
- [x] T-T1.2 — lock R3 mock-pattern decision — _outcome: per-Recipe-specific mocks (D-V0.2.0-1); rationale 4 points; rejects single shared trait_
- [x] T-T1.3 — decompose M-DEV into waves — _outcome: Waves A→D dependency-ordered (D-V0.2.0-5); A‖B parallel, C extracts `SubscriptionBatchDescriptor` seam, D depends on C; ~940 LoC tests + ~130 LoC src deltas; ~1 week dev + 1 day tester_
- [x] T-T1.4 — ADR-0048 § Changelog amendment — _outcome: ADR-0048 carries forward verbatim (D-V0.2.0-4); ONE Changelog row appended on ADR-0048 referencing this brief_

## M-DEV — Developer (Waves A→D per D-V0.2.0-5; falsification probe per file)

**Wave A — TrainingLogRecipe S1 + S2 (HIGHEST URGENCY — exact Bug #64 shape; ~200 LoC + 0 src delta)**

- [x] T-D-A1 — `crates/ui/tests/training_log_recipe_harness.rs` (S1 boundary, ~250 LoC) — _file: crates/ui/tests/training_log_recipe_harness.rs; 3 tests: `sentinel_log_line_emitted_before_subprocess_spawn` + `salt_bump_survives_arc_mutex_take` + `log_stream_survives_recipe_drop`; MockTrainingLogChannel wraps real `std::sync::mpsc::sync_channel`; T-T4 falsification probe P1 (training_log.rs:124) verified — all 3 tests FAIL under probe; DEV-CONFIRM-1 line numbers verified (87=.take(), 124=yield); test cmd: `cargo test -p ui --test training_log_recipe_harness --no-default-features --features live`; output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`_
- [x] T-D-A2 — `crates/ui/tests/training_log_state_gating.rs` (S2 gating, ~240 LoC) — _file: crates/ui/tests/training_log_state_gating.rs; 3 tests: `training_log_panel_visibility_gated_on_inflight` + `training_log_panel_clears_on_completion` + `training_log_panel_state_after_cancellation`; T-T4 falsification probe P3 (state.rs:2232) verified — `training_log_panel_visibility_gated_on_inflight` + `training_log_panel_clears_on_completion` FAIL under probe; pattern mirrors `lab_stop_button_gating.rs`; test cmd: `cargo test -p ui --test training_log_state_gating --no-default-features --features live`; output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.11s`_

**Wave B — ActivityAuditAggregator S1 (parallel to A; ~150 LoC + ~20 LoC src delta)**

- [x] T-D-B1 — Src delta: extract `pub async fn run_aggregator_loop(rx, bus)` from `Aggregator::run` body in `crates/agent/src/activity_audit_aggregator.rs`; promote `Aggregator::new` to `pub` — _file: crates/agent/src/activity_audit_aggregator.rs:91 (Aggregator pub struct), :112 (Aggregator::new pub fn), :167 (pub async fn run_aggregator_loop); production `spawn_aggregator` delegates via `agg.run()` → `run_aggregator_loop(self)`; test cmd: `cargo test -p agent --test activity_audit_aggregator`; output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.71s`_
- [x] T-D-B2 — `crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs` (~190 LoC) — _file: crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs; 3 tests: `recv_arm_increments_after_interval_fires` + `recv_arm_survives_n_interval_boundaries` + `recv_arm_increments_counter`; MockAuditTickBus wraps real `broadcast::channel::<AuditTick<AuditEvent>>(16)`; `start_paused = true` + `tokio::time::advance()` for interval control; P-B1 probe (recv arm = pending): all 3 tests FAIL — confirms recv-arm starvation detection; P-B2 probe (interval arm no-op): tests 1+2 PASS (negative control confirmed), test 3 FAILs (expected — interval arm body required for Start event emission); test cmd: `cargo test -p agent --test activity_audit_aggregator_select_arm_survival`; output: `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s`_

**Wave C — Extract SubscriptionBatchDescriptor seam + ServerTime S2 + ToastDismiss S1 + S2 (sequential after A or B; ~240 LoC + ~80 LoC src delta)**

- [x] T-D-C1 — Src delta: extract `pub fn build_subscription_batch_descriptor(...) -> SubscriptionBatchDescriptor` from `crates/ui/src/bin/cockpit_live.rs::subscription()`; production calls `build_subscription_batch_descriptor(...).into_iced_subscription()`; descriptor is `Vec<SubscriptionVariant>` enum (one variant per recipe) — _file: crates/ui/src/live.rs:926 (build_subscription_batch_descriptor) + crates/ui/src/live.rs:879 (SubscriptionVariant enum); production subscription() at cockpit_live.rs:1549 calls build_subscription_batch_descriptor and converts each variant; DEV-CONFIRM-2 note: full extraction taken (not fallback) — seam added to live.rs not cockpit_live.rs binary, to enable test reachability; anchor-clean (84/84 PASS); test cmd: `cargo test -p ui --test cockpit_live_lab_run_smoke --no-default-features --features live,yahoo`; output: `test result: ok. 5 passed; 0 failed`_
- [x] T-D-C2 — `crates/ui/tests/cockpit_subscription_server_time_always_batched.rs` (~110 LoC, 2 tests) — _file: crates/ui/tests/cockpit_subscription_server_time_always_batched.rs; tests: `server_time_recipe_in_every_screen_batch` (iterates 5 Screen variants) + `server_time_present_with_all_optional_recipes_active`; T-T4 probe: comment out `desc.push(SubscriptionVariant::ServerTime)` → 2 tests FAIL with `ServerTime not found in descriptor`; probe dry-run confirmed RED; test cmd: `cargo test -p ui --test cockpit_subscription_server_time_always_batched --no-default-features --features live`; output: `test result: ok. 2 passed; 0 failed`_
- [x] T-D-C3 — `crates/ui/tests/toast_dismiss_recipe_stream.rs` (S1 boundary, ~190 LoC, 3 tests) — _file: crates/ui/tests/toast_dismiss_recipe_stream.rs; tests: `stream_yields_toast_tick_every_500ms` + `toast_tick_instants_are_monotone` + `toast_dismiss_stream_remains_open`; uses `start_paused = true` + yield_now (skip t=0 tick) + advance(500ms) × N; T-T4 probe: insert `continue;` before `yield Message::ToastTick(...)` → all 3 tests FAIL with `left: 0, right: 3`; probe dry-run confirmed RED; test cmd: `cargo test -p ui --test toast_dismiss_recipe_stream --no-default-features --features live`; output: `test result: ok. 3 passed; 0 failed`_
- [x] T-D-C4 — `crates/ui/tests/cockpit_subscription_toast_dismiss_always_batched.rs` (~160 LoC, 3 tests) — _file: crates/ui/tests/cockpit_subscription_toast_dismiss_always_batched.rs; tests: `toast_dismiss_in_every_screen_batch` (5 Screen variants) + `toast_dismiss_present_with_all_optional_recipes_active` + `toast_dismiss_present_regardless_of_toast_queue_emptiness`; T-T4 probe: comment out `desc.push(SubscriptionVariant::ToastDismiss)` → all 3 tests FAIL with `ToastDismiss not found in descriptor`; probe dry-run confirmed RED; test cmd: `cargo test -p ui --test cockpit_subscription_toast_dismiss_always_batched --no-default-features --features live`; output: `test result: ok. 3 passed; 0 failed`_

**Wave D — TrailMirror S2 + Activity S1 + TrainingPoller S1+S2 (sequential after C for TrailMirror; ~350 LoC + ~30 LoC src delta)**

- [ ] T-D-D1 — `crates/ui/tests/trail_mirror_subscription_handle_gating.rs` (S2, ~80 LoC) — _accept: 2 tests — handle present → batched; handle absent → omitted; uses `SubscriptionBatchDescriptor` from Wave C; T-T4 probe per D-V0.2.0-3 row 9_
- [ ] T-D-D2 — `crates/ui/tests/activity_recipe_stream.rs` (S1 boundary, ~120 LoC) — _accept: ≥ 3 tests covering happy-path stream + Lagged warning path + Closed EOF; uses real `EventBus::new(BusConfig::default())`; T-T4 probe per D-V0.2.0-3 row 10_
- [ ] T-D-D3 — Src delta + test: extract `pub async fn training_poller_stream_impl(ledger, run_id, last_seen_ts) -> BoxStream<Message>` from `crates/ui/src/lab/training_subscription.rs:108-144`; production `Recipe::stream` delegates to it. New `crates/ui/tests/training_poller_subscription.rs` (~150 LoC) — _accept: `MockAuditLedger = Ledger::in_memory()`; tests cover happy-path 3-row refresh + idempotent second-poll + run_id filter (rows for OTHER run_id ignored); T-T4 probe per D-V0.2.0-3 row 11; DEV-CONFIRM-3 fallback (b) acceptable (inline-body replication, zero src delta) if extraction balloons LoC_

**Per-wave acceptance gate:**

- All new tests PASS green.
- Per-Recipe T-T4 falsification probe documented in module docstring at exact line numbers per D-V0.2.0-3.
- `bash scripts/verify_anchors.sh` → 71/71 PASS byte-identical (no anchor drift).
- v0.1.0 harness tests (`spawn_lab_run_yahoo_harness.rs`, `lab_stop_button_gating.rs`) stay PASS byte-identical.
- `cargo clippy --workspace --all-features -- -D warnings` clean.

**Wall-clock estimate:** A (1.5d) ‖ B (1.5d) → C (2d) → D (1.5d) ≈ 6 dev days net; +1 tester day.

## M-FINAL — Tester (per-Recipe T-T4 falsification table)

- [ ] T-T-FINAL — run all new tests + falsification probes; emit per-Recipe FAIL → restore → PASS table; verify anchors 71/71 byte-identical pre/post; verify v0.1.0 harness tests stay PASS — _accept: test-final-2026-MM-DD-<slug>.md with per-Recipe T-T4 evidence; VERDICT → PASS or SOFT-PASS_

## M-PRESENT — Presenter (operator review deck)

- [ ] T-P1 — deck `presentations/<slug>-<date>.md`: per-Recipe T-T4 evidence; durable-coverage outcome statement; v0.3.0 backlog row planted iff Q1=(b) chosen — _accept: operator-decide-ready deck_

## Notes

- **Anchor contract**: 71/71 byte-identical pre/post. Zero file
  output from any new test. Same as v0.1.0 D6.
- **Falsification stub per Recipe**: each new test file MUST include
  a module docstring section "T-T4 falsification probe" that names the
  exact source line to comment out + the expected FAIL assertion.
  This is the v0.1.0 lesson made durable per Q2=(a).
- **Wave parallelism**: Waves A-D are independent (different Recipes,
  different test files). Waves E-F can run alongside any of A-D.
  Architect may schedule them concurrently if dev-bench available.
- **Cargo build budget**: per K4, total new test LoC ≤ 800 (8 surfaces
  × ~100 LoC each); cargo test wall-clock budget per Recipe ≤ 1.5 s
  per ADR-0048 D4.
