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

- [ ] T-D-A1 — `crates/ui/tests/training_log_recipe_stream.rs` (S1 boundary, ~120 LoC) — _accept: ≥ 3 tests covering happy-path stream + take-ownership semantics (`recipe_takes_receiver_via_arc_mutex_option`) + sender-drop EOF; module docstring `## T-T4 falsification probe` table per D-V0.2.0-3 rows 1 + 2; MockTrainingLogChannel wraps real `std::sync::mpsc::sync_channel`; DEV-CONFIRM-1 line numbers verified_
- [ ] T-D-A2 — `crates/ui/tests/training_log_inflight_gating.rs` (S2 gating, ~80 LoC) — _accept: lifecycle assertions on `lab_state.training_inflight` predicate: `Default → None → Some after TrainingPressed → None after TrainingExited → None after TrainingCancelPressed`; T-T4 probe per D-V0.2.0-3 row 3 (state.rs:2232); pattern mirrors `lab_stop_button_gating.rs`_

**Wave B — ActivityAuditAggregator S1 (parallel to A; ~150 LoC + ~20 LoC src delta)**

- [ ] T-D-B1 — Src delta: extract `pub async fn run_aggregator_loop(rx, bus)` from `Aggregator::run` body in `crates/agent/src/activity_audit_aggregator.rs`; promote `Aggregator::new` to `pub` — _accept: production `spawn_aggregator` re-calls the extracted fn; all existing 5+ unit tests in this file stay PASS_
- [ ] T-D-B2 — `crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs` (~150 LoC) — _accept: ≥ 2 tests: `recv_arm_increments_after_interval_fires` + `recv_arm_survives_n_interval_boundaries`; uses `tokio::time::pause()` + `advance()` to interleave `tx.send(tick)` between interval boundaries; MockAuditTickBus wraps real `broadcast::channel`; T-T4 probe per D-V0.2.0-3 rows 4 + 5_

**Wave C — Extract SubscriptionBatchDescriptor seam + ServerTime S2 + ToastDismiss S1 + S2 (sequential after A or B; ~240 LoC + ~80 LoC src delta)**

- [ ] T-D-C1 — Src delta: extract `pub fn build_subscription_batch_descriptor(...) -> SubscriptionBatchDescriptor` from `crates/ui/src/bin/cockpit_live.rs::subscription()`; production calls `build_subscription_batch_descriptor(...).into_iced_subscription()`; descriptor is `Vec<SubscriptionVariant>` enum (one variant per recipe) — _accept: API-additive, anchor-clean; all `cockpit_live_lab_run_smoke.rs` + existing subscription tests stay PASS; DEV-CONFIRM-2 fallback acceptable if extraction proves invasive_
- [ ] T-D-C2 — `crates/ui/tests/cockpit_subscription_server_time_always_batched.rs` (~60 LoC) — _accept: assert `SubscriptionVariant::ServerTime` present in descriptor across all 5 `Screen::*` variants; T-T4 probe per D-V0.2.0-3 row 6_
- [ ] T-D-C3 — `crates/ui/tests/toast_dismiss_recipe_stream.rs` (S1 boundary, ~120 LoC) — _accept: `tokio::test(start_paused = true)` + `advance(500ms)` × N; assert N `Message::ToastTick` with monotone `Instant`; T-T4 probe per D-V0.2.0-3 row 7_
- [ ] T-D-C4 — `crates/ui/tests/cockpit_subscription_toast_dismiss_always_batched.rs` (~60 LoC) — _accept: assert `SubscriptionVariant::ToastDismiss` present across all 5 `Screen::*` variants AND regardless of `toast_queue.is_empty()`; T-T4 probe per D-V0.2.0-3 row 8_

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
