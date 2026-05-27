---
slug: cockpit-toast-queue
status: proposed
owner: analyst
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

_owner: architect (pending). Expected outcomes per analyst handoff:_

- [ ] **T-AR-1** — Lock storage shape for the queue. Analyst-
  recommended: `VecDeque<ToastEntry>` capped at 5. Architect
  picks the exact iced-friendly type (e.g. `SmallVec<[ToastEntry;
  5]>` if Clone overhead matters; profile if doubtful).
- [ ] **T-AR-2** — Lock the iced recipe pattern for the
  auto-dismiss ticker (R2.5). Two candidates: (i)
  `iced::time::every(Duration::from_millis(500))` subscription —
  simplest but always-on cost; (ii) per-toast `iced::Task::perform`
  scheduled timeout — only-on-demand cost but more state. Analyst-
  recommended (i) at v0.1.0 (the 500 ms tick cost on idle is
  negligible vs the existing activity-tape 100 ms tick).
- [ ] **T-AR-3** — Lock K5 clock injection pattern. Analyst-
  recommended: mirror `crates/agent/src/clock.rs::Clock` trait
  precedent for test-time fake clock; inject as `AppState`
  field.
- [ ] **T-AR-4** — Decide whether `ToastEntry::id: u64` is
  global-monotonic (one `AtomicU64` per process) or per-cockpit-
  instance (`Cell<u64>` on `AppState`). Analyst-recommended the
  latter (no cross-instance contention in tests; matches
  `training_log_recipe_salt` precedent).
- [ ] **T-AR-5** — Decide whether the back-compat
  `AppState::toast_message()` helper (R5.9) lives or whether the
  test suite migrates fully at v0.1.0. Analyst-recommended:
  keep the shim; remove at the v0.2.0 cleanup brief (smaller blast
  radius).
- [ ] **T-AR-6** — No new ADR expected (UI-only widget + queue
  shape, no cross-crate boundary change, no anchor touches). If
  K2 (tape/queue surface overlap) needs architectural resolution,
  spawn ADR-NNNN; otherwise close M-T1 without one.
- [ ] **T-AR-7** — Frontmatter flip `owner: analyst → developer`.
  Update trace row `REQ-COCKPIT-TOAST-QUEUE-001::arch` column
  with the chosen widget file paths.

## M-DEV — Developer execution

_owner: developer (pending). Expected 4 waves; small surface;
analyst-suggested decomp below — architect refines at M-T1._

### Wave A — queue + message arms (R1, ~80 LOC + 4 unit tests)

- [ ] **T-D-N1** — Replace `toast_message: Option<SmolStr>` with
  `toast_queue: VecDeque<ToastEntry>` on `AppState` at
  `crates/ui/src/state.rs:816`. Update both constructors
  (`state.rs:1055`, `state.rs:1159`) and the `Debug` impl
  (`state.rs:1000`).
- [ ] **T-D-N2** — Add `ToastEntry` struct + `ToastSeverity` enum
  to `crates/ui/src/state.rs` (near the `AppState` type). Add
  `Message::ShowToastWithSeverity(SmolStr, ToastSeverity)` and
  `Message::DismissToastById(u64)` variants.
- [ ] **T-D-N3** — Rewrite `Message::ShowToast` and
  `Message::DismissToast` arms at `state.rs:2056-2061` to use the
  new queue. Update the Lab Compare cap-hit producer at
  `state.rs:1983` (R3.1).
- [ ] **T-D-N4** — 4 unit tests in `state.rs::tests`:
  `toast_queue_enqueue_basic`,
  `toast_queue_overflow_drops_oldest`,
  `toast_queue_dismiss_by_id`,
  `show_toast_msg_back_compat`.

### Wave B — view widget + shell wiring (R2, ~150 LOC)

- [ ] **T-D-N5** — NEW file
  `crates/ui/src/widgets/toast_tray.rs` rendering a stacked
  vertical list of toast cards with severity-tinted borders + ×
  buttons.
- [ ] **T-D-N6** — Wire into `crates/ui/src/shell.rs` as a top-
  layer overlay (architect picks the iced stack mechanism at
  M-T1). Placement above the activity tape per Q4=(a).
- [ ] **T-D-N7** — Add `pub mod toast_tray;` to
  `crates/ui/src/widgets/mod.rs`.

### Wave C — producer migration + integration test (R3-R4, ~120 LOC)

- [ ] **T-D-N8** — Migrate training spawn-failure at
  `crates/ui/src/bin/cockpit_live.rs:1101` to
  `Message::ShowToastWithSeverity(.., Danger)` (R3.2).
- [ ] **T-D-N9** — NEW file
  `crates/ui/tests/cockpit_toast_queue.rs` with 4 integration
  tests:
  1. `queue_displays_multiple` — dispatch 3 ShowToast, assert
     `toast_queue.len() == 3`.
  2. `auto_dismiss_after_timeout` — inject 1 toast, advance
     fake clock by `TOAST_AUTODISMISS + 1s`, tick the
     dismiss ticker once, assert queue is empty.
  3. `two_completions_in_rapid_succession_both_visible` — R4.1
     stronger K5 contract.
  4. `overflow_drops_oldest_keeps_newest` — enqueue 6 with cap
     5; assert oldest dropped.

### Wave D — auto-dismiss ticker + back-compat shim (R2.5, R5.9, ~60 LOC)

- [ ] **T-D-N10** — Implement `ToastDismissTicker` recipe per
  T-AR-2 architect lock. Wire into `cockpit_live.rs::subscription()`
  alongside the existing 4 recipes.
- [ ] **T-D-N11** — Add `pub fn toast_message(&self) ->
  Option<&SmolStr>` helper on `AppState` returning
  `self.toast_queue.front().map(|t| &t.message)` (R5.9 back-compat
  shim). Doc-comment with `// MIGRATION: remove at v0.2.0`.
- [ ] **T-D-N12** — Verify the parent feature's K5 test still
  passes at the new API (R4.2):
  `cargo test -p ui --test cockpit_training_pressed_wiring k5_toast_non_clobber`.

### Wave E — gates

- [ ] **T-D-N13** — `cargo build --workspace --all-targets` PASS
  (no new warnings).
- [ ] **T-D-N14** — `cargo test -p ui` PASS (818+ workspace tests
  green per R5.6).
- [ ] **T-D-N15** — `bash scripts/verify_anchors.sh` →
  `ANCHORS PASS (34/34)` (hard gate per R5.8).
- [ ] **T-D-N16** — `bash scripts/cockpit_smoke.sh` → 0 panics
  (hard gate per R5.5).
- [ ] **T-D-N17** — Manual cockpit visual smoke at depth-3 +
  depth-5 (K1 mitigation; documented at M-FINAL with screenshots
  if disk allows).

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
