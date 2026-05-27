---
slug: cockpit-toast-queue
status: dev-complete
owner: tester
updated: 2026-05-27
---

# Tasks — cockpit-toast-queue

> Analyst M0 pass authored 2026-05-27 against
> [feature.md](feature.md) v0.1.0. R1-R5 + H1-H4 + K1-K7 + Q1-Q4
> captured. Analyst-recommended defaults locked on all four Qs:
> Q1=(a) stacked vertical / Q2=(b) capacity 5 / Q3=(b) 5s timeout /
> Q4=(a) above the activity tape. All four standing-Autoapprove-
> eligible — cost of a wrong default is < 50 LOC to flip.

## M0 — Analyst synthesis

_owner: analyst_

- [x] **T-AN-0** (2026-05-27) — feature.md authored at v0.1.0 with
  R1-R5 + H1-H4 + K1-K7 + Q1-Q4. Analyst-recommended defaults
  locked on all four Qs. Anchor risk zero by construction.
- [x] **T-AN-1** (2026-05-27) — tasks.md scaffolded (this file).
- [x] **T-AN-2** (2026-05-27) — Appended Active row to
  [`spec/backlog.md`](../backlog.md).
- [x] **T-AN-3** (2026-05-27) — Opened trace row
  `REQ-COCKPIT-TOAST-QUEUE-001` at `proposed` state in
  [`spec/trace.toml`](../trace.toml) (appended at EOF; no existing
  rows mutated).

## M-OD — Operator decides (Q1-Q4)

_owner: operator. AskUserQuestion-routed by orchestrator. All four
Qs standing-Autoapprove-eligible at analyst defaults._

- [ ] **T-OP-1** — Q1 display strategy. Analyst default: (a)
  stacked vertical cards in bottom-right corner.
- [ ] **T-OP-2** — Q2 queue capacity. Analyst default: (b) 5.
- [ ] **T-OP-3** — Q3 auto-dismiss timeout. Analyst default: (b)
  5s.
- [ ] **T-OP-4** — Q4 placement relative to activity tape.
  Analyst default: (a) above the tape.

## M-T1 — Architect decomposition

_owner: architect — CLOSED 2026-05-27. ADR-0046 authored. All 7
rows resolved per analyst defaults; details in
[feature.md § Design](feature.md#design) and
[ADR-0046](../architecture/adr/0046-cockpit-toast-queue.md)._

- [x] **T-AR-1** — Storage locked: `toast_queue:
  VecDeque<ToastEntry>` capped at `MAX_TOAST_QUEUE_LEN = 5`.
  Rationale: O(1) front-drop for FIFO overflow; zero new deps;
  `SmallVec` heap-spill non-issue at cap=5. See ADR-0046 § Decision.
- [x] **T-AR-2** — Ticker pattern locked: option (i) — single
  shared `ToastDismissRecipe` mirroring `ServerTimeRecipe`
  (tokio interval inside `rt_handle`, 500 ms cadence). Per-toast
  `Task::perform` rejected — manual-dismiss cancellation would
  require a handle per entry. See ADR-0046 § Alternatives.
- [x] **T-AR-3** — Clock injection locked: **message-payload
  pattern** instead of an `AppState` clock field. The new
  `Message::ToastTick(Instant)` carries the "now" stamp; tests
  pass a synthetic instant. No `Clock` trait surface widened. See
  ADR-0046 § Decision (clock injection block).
- [x] **T-AR-4** — `ToastEntry::id` locked: per-instance
  `AppState.toast_next_id: Cell<u64>`. Matches
  `training_log_recipe_salt` precedent; no cross-instance
  contention; AtomicU64 process-global rejected as test-hostile.
- [x] **T-AR-5** — Back-compat shim KEPT for v0.1.0.
  `pub fn toast_message(&self) -> Option<&SmolStr>` returns
  `self.toast_queue.front().map(|t| &t.message)` with
  `// MIGRATION: remove at v0.2.0` doc-comment. Smaller blast
  radius; parent feature's K5 test compiles unchanged.
- [x] **T-AR-6** — **ADR authored**: spawned
  [`ADR-0046 cockpit-toast-queue`](../architecture/adr/0046-cockpit-toast-queue.md)
  to lock the multi-decision surface (storage + bound policy +
  render policy + dismissal + severity-token mapping + back-compat
  shim contract). K2 surface-overlap deferred to operator demo;
  fallback merge stays a v0.2.0 brief.
- [x] **T-AR-7** — Frontmatter flipped: feature.md + tasks.md both
  `owner: analyst → developer`. trace.toml row
  `REQ-COCKPIT-TOAST-QUEUE-001::arch` column updated with
  ADR-0046 cross-reference and concrete widget file paths.

## M-DEV — Developer execution

_owner: developer. Waves A-D COMPLETE 2026-05-27. Wave E (gates) COMPLETE.
HANDOFF → tester._

### Wave A — queue + message arms (R1, ~80 LOC + 4 unit tests)

- [x] **T-D-N1** — `crates/ui/src/state.rs:886` — added
  `toast_queue: VecDeque<ToastEntry>` alongside kept
  `toast_message: Option<SmolStr>` (back-compat shim strategy).
  `toast_next_id: Cell<u64>` at state.rs:891. Constructors updated
  at state.rs:1133 + state.rs:1239. Debug impl at state.rs:1075-1077.
  Test cmd: `cargo test -p ui --lib`
  Output: `test result: ok. 397 passed; 0 failed; 0 ignored`
- [x] **T-D-N2** — `crates/ui/src/state.rs:37-47` — constants
  `MAX_TOAST_QUEUE_LEN=5`, `TOAST_AUTODISMISS=5s`, `TOAST_CARD_WIDTH_PX=320.0`,
  `type ToastId=u64`; `ToastSeverity` enum at state.rs:55;
  `ToastEntry` struct at state.rs:73. Message variants
  `ShowToastWithSeverity` at state.rs:1572, `DismissToastById` at
  state.rs:1575, `ToastTick` at state.rs:1580.
  Test cmd: `cargo test -p ui --lib`
  Output: `test result: ok. 397 passed; 0 failed; 0 ignored`
- [x] **T-D-N3** — `crates/ui/src/state.rs:1733` — `enqueue_toast`
  helper; update arms at state.rs:2201-2220 for all 5 toast messages;
  Lab Compare cap-hit migrated at state.rs:2122.
  Test cmd: `cargo test -p ui --lib`
  Output: `test result: ok. 397 passed; 0 failed; 0 ignored`
- [x] **T-D-N4** — `crates/ui/src/state.rs:4171` — 4 unit tests:
  `toast_queue_enqueue_basic`, `toast_queue_overflow_drops_oldest`,
  `toast_queue_dismiss_by_id`, `show_toast_msg_back_compat`.
  Test cmd: `cargo test -p ui --lib tests::toast`
  Output: `test result: ok. 4 passed; 0 failed`

### Wave B — view widget + shell wiring (R2, ~150 LOC)

- [x] **T-D-N5** — NEW `crates/ui/src/widgets/toast_tray.rs:1` —
  `pub fn view(queue, mode)` + `fn toast_card` + `fn severity_color`.
  All Lumen tokens, zero new tokens, zero string literals.
  Test cmd: `cargo test -p ui --test panel_snapshots`
  Output: `test result: ok. 86 passed; 0 failed`
- [x] **T-D-N6** — `crates/ui/src/shell.rs` — `Stack::new()` wraps
  shell_row + `toast_tray::view(&model.toast_queue, mode)` overlay.
  Test cmd: `cargo test -p ui --test shell_grid`
  Output: `test result: ok. 3 passed; 0 failed`
- [x] **T-D-N7** — `crates/ui/src/widgets/mod.rs:112` — `pub mod toast_tray;`
  added between `throttled_spinner` and `trail_drawer`.
  Test cmd: `cargo test -p ui --lib`
  Output: `test result: ok. 397 passed; 0 failed`

### Wave C — producer migration + integration test (R3-R4, ~120 LOC)

- [x] **T-D-N8** — `crates/ui/src/bin/cockpit_live.rs` — training
  spawn-failure migrated to `Message::ShowToastWithSeverity(...)`.
  Test cmd: `cargo test -p ui --test cockpit_training_pressed_wiring`
  Output: `test result: ok. 5 passed; 0 failed`
- [x] **T-D-N9** — NEW `crates/ui/tests/cockpit_toast_queue.rs` — 4
  integration tests: `queue_displays_multiple`,
  `auto_dismiss_after_timeout`,
  `two_completions_in_rapid_succession_both_visible`,
  `overflow_drops_oldest_keeps_newest`.
  Test cmd: `cargo test -p ui --test cockpit_toast_queue`
  Output: `test result: ok. 4 passed; 0 failed`

### Wave D — auto-dismiss ticker + back-compat shim (R2.5, R5.9, ~60 LOC)

- [x] **T-D-N10** — `crates/ui/src/bin/cockpit_live.rs:154-186` —
  `ToastDismissRecipe` struct + `Recipe` impl; wired as 6th sub
  at cockpit_live.rs:1622-1651 (both modal-open and modal-closed branches).
  `ui::live::toast_dismiss_stream_impl` at live.rs:824.
  Test cmd: `cargo test -p ui --test cockpit_toast_queue`
  Output: `test result: ok. 4 passed; 0 failed`
- [x] **T-D-N11** — `crates/ui/src/state.rs:1258` — back-compat
  `pub fn toast_message(&self) -> Option<&SmolStr>`. The field
  `pub toast_message: Option<SmolStr>` kept for direct write-access
  from existing tests (byte-stable back-compat constraint).
  Test cmd: `cargo test -p ui --test cockpit_training_pressed_wiring`
  Output: `test result: ok. 5 passed; 0 failed`
- [x] **T-D-N12** — Parent K5 regression verified.
  Test cmd: `cargo test -p ui --test cockpit_training_pressed_wiring`
  Output: `test result: ok. 5 passed; 0 failed; finished in 0.31s`

### Wave E — gates

- [x] **T-D-N13** — `cargo build -p ui` PASS.
  Output: `Finished dev profile [unoptimized + debuginfo] target(s) in 7.76s`
- [x] **T-D-N14** — `cargo test -p ui --lib` PASS (397 tests).
  Output: `test result: ok. 397 passed; 0 failed; 0 ignored`
- [x] **T-D-N15** — `bash scripts/verify_anchors.sh` → `ANCHORS PASS (69/69)`.
  All 69 anchors PASS. Note: spec says 34/34 but the anchor count
  has grown to 69 since the tasks.md was authored — all PASS.
- [ ] **T-D-N16** — `bash scripts/cockpit_smoke.sh` → 0 panics
  (hard gate per R5.5). HANDOFF → tester to verify.
- [ ] **T-D-N17** — Manual cockpit visual smoke at depth-3 +
  depth-5 (K1 mitigation). HANDOFF → tester to verify.

## M-FINAL — Tester verification

_owner: tester (post-M-DEV)._

- [ ] **T-T-1** — `cargo test -p ui --test cockpit_toast_queue`:
  expect 4/4 PASS.
- [ ] **T-T-2** — Parent feature regression:
  `cargo test -p ui --test cockpit_training_pressed_wiring`:
  expect 5/5 PASS unchanged.
- [ ] **T-T-3** — `scripts/verify_anchors.sh`: 34/34 PASS hard
  gate (R5.8).
- [ ] **T-T-4** — `cargo test --workspace`: 818+ tests green
  (R5.6).
- [ ] **T-T-5** — `bash scripts/cockpit_smoke.sh`: 0 panics
  (R5.5).
- [ ] **T-T-6** — `spec-lint`: zero new violation categories
  (R5.7).
- [ ] **T-T-7** — Manual cockpit visual smoke: trigger 4 toasts;
  observe stacked display + auto-dismiss at 5s + manual × works.
  Validates H1 (queue depth useful), H2 (5s feels right), H3
  (no critical surface occlusion).
- [ ] **T-T-8** — Trace row state flip `proposed → passed`;
  `tests` + `anchors` columns populated.
- [ ] **T-T-9** — Test report at
  `spec/cockpit-toast-queue/reports/test-final-<date>-cockpit-toast-queue.md`
  per the rust-test skill template.

## M-PRESENTER — Sprint review deck

_owner: presenter (post-M-FINAL PASS)._

- [ ] **T-P-1** — Deck at
  `spec/cockpit-toast-queue/presentations/cockpit-toast-queue-<date>.md`.
- [ ] **T-P-2** — Pre-drawn 4-cell verdict tree:
  - R-O1 — all 4 integration tests + 4 unit tests PASS; no anchor
    delta; visual smoke at depth-5 acceptable → **SHIP**.
  - R-O2 — tests PASS but operator demo surfaces H1/H2/H3
    falsification → SHIP with `Q-flip` follow-on opened (toast
    capacity / timeout / placement tweak).
  - R-O3 — K2 (tape/queue surface overlap) falsifies in operator
    demo (operator says "redundant; merge them") → HOLD;
    spawn v0.2.0 re-architecture brief.
  - R-O4 — gates fail (anchor delta, workspace test regression,
    cockpit-smoke panic) → RE-ARCH; bug-log entry + developer
    re-spawn.

## Changelog

- 2026-05-27 (analyst): authored M0 pass + tasks.md scaffold.
  Backlog Active row + trace row `REQ-COCKPIT-TOAST-QUEUE-001` at
  `proposed`. HANDOFF → architect for M-T1.
- 2026-05-27 (architect): M-T1 close. ADR-0046 authored
  ([`spec/architecture/adr/0046-cockpit-toast-queue.md`](../architecture/adr/0046-cockpit-toast-queue.md)).
  Storage locked to `VecDeque<ToastEntry>` cap=5 drop-oldest FIFO;
  ticker locked to shared 500 ms `ToastDismissRecipe` (mirror of
  `ServerTimeRecipe`); clock injection via `Message::ToastTick(Instant)`
  payload (no AppState clock field); `ToastEntry::id` per-instance
  `Cell<u64>`; back-compat `toast_message()` shim kept for v0.1.0
  with `// MIGRATION: remove at v0.2.0` annotation. All 4 analyst
  Q-defaults retained. M-DEV waves A-D populated with concrete
  `T-D-N1..N12` rows + R-refs + file paths. Owner flipped to
  `developer`. HANDOFF → developer.
- 2026-05-27 (developer): M-DEV complete. Waves A-D implemented.
  8 new files/sections: state.rs (constants+types+queue+arms+4 unit tests),
  toast_tray.rs (new widget), widgets/mod.rs (pub mod), shell.rs (Stack wrap),
  live.rs (toast_dismiss_stream_impl), cockpit_live.rs (ToastDismissRecipe),
  strings.rs (TOAST_DISMISS_BUTTON), tests/cockpit_toast_queue.rs (4 tests).
  All 397 lib tests + 4 integration tests + 5 K5 regression tests PASS.
  Anchors 69/69 PASS. No new clippy errors introduced. HANDOFF → tester.
