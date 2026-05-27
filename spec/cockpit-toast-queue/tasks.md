---
slug: cockpit-toast-queue
status: in-progress
owner: developer
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

_owner: developer (pending). 4 waves locked at M-T1 close
2026-05-27. Architect-locked references inline (ADR-0046, R-refs,
file paths). Suggested execution order: A → B → C → D → E._

### Wave A — queue + message arms (R1, ~80 LOC + 4 unit tests)

- [ ] **T-D-N1** — `crates/ui/src/state.rs:816` — replace
  `toast_message: Option<SmolStr>` with
  `toast_queue: VecDeque<ToastEntry>`. Update constructors at
  `state.rs:1055`, `state.rs:1159` to
  `VecDeque::with_capacity(MAX_TOAST_QUEUE_LEN)`. Update `Debug`
  impl at `state.rs:1000` to the new field name. Add field
  `toast_next_id: Cell<u64>` initialized to `Cell::new(0)`.
  _Acceptance:_ workspace builds; existing tests fail expectedly
  on the type change (fixed in T-D-N11).
- [ ] **T-D-N2** — `crates/ui/src/state.rs` (near `AppState`
  type) — add the public types per
  [feature.md § Design](feature.md#storage--types):
  ```rust
  pub const MAX_TOAST_QUEUE_LEN: usize = 5;
  pub const TOAST_AUTODISMISS: Duration = Duration::from_secs(5);
  pub struct ToastEntry { pub id, message, severity, created_at }
  pub enum ToastSeverity { Info, Success, Warning, Danger }
  ```
  Add `Message::ShowToastWithSeverity(SmolStr, ToastSeverity)`,
  `Message::DismissToastById(u64)`, `Message::ToastTick(Instant)`
  variants near `state.rs:1466`.
- [ ] **T-D-N3** — `crates/ui/src/state.rs:2056-2061` — rewrite
  the four (now five) toast arms in `update()`. Introduce private
  helper `fn enqueue_toast(model: &mut Cockpit, msg: SmolStr, sev:
  ToastSeverity)` that:
  1. Bumps `toast_next_id` (`get()` + `set(prev+1)`).
  2. Pops the front if `len() == MAX_TOAST_QUEUE_LEN`.
  3. Pushes `ToastEntry { id, message: msg, severity: sev,
     created_at: Instant::now() }` to the back.
  Wire `ShowToast(msg) → enqueue_toast(_, msg, Info)`,
  `ShowToastWithSeverity(msg, sev) → enqueue_toast(_, msg, sev)`,
  `DismissToast → pop_front()`,
  `DismissToastById(id) → retain(|t| t.id != id)`,
  `ToastTick(now) → retain(|t| now.duration_since(t.created_at)
  < TOAST_AUTODISMISS)`. Also migrate the Lab Compare cap-hit
  producer at `state.rs:1983` from
  `model.toast_message = Some(...)` to
  `enqueue_toast(model, SmolStr::new(LAB_COMPARE_CAP_HIT),
  ToastSeverity::Warning)` (R3.1).
- [ ] **T-D-N4** — `state.rs::tests` — 4 unit tests (R1 acceptance):
  - `toast_queue_enqueue_basic` — enqueue 3 distinct, expect
    `len() == 3` + ids `[1, 2, 3]`.
  - `toast_queue_overflow_drops_oldest` — enqueue 6 messages
    `"m1".."m6"`; expect `len() == 5`, front message is `"m2"`
    (not `"m1"`).
  - `toast_queue_dismiss_by_id` — enqueue 3, dispatch
    `DismissToastById(middle.id)`, expect `len() == 2`, surviving
    ids in order.
  - `show_toast_msg_back_compat` — dispatch
    `Message::ShowToast(...)`, expect front entry's severity ==
    `Info`.

### Wave B — view widget + shell wiring (R2, ~150 LOC)

- [ ] **T-D-N5** — NEW file `crates/ui/src/widgets/toast_tray.rs`.
  Public entry `pub fn view<'a>(queue: &'a VecDeque<ToastEntry>,
  mode: ThemeMode) -> Element<'a, Message>` per
  [feature.md § Design](feature.md#view-cratesuisrcwidgetstoast_trayrs--new-150-loc).
  Empty-queue path returns a 0×0 `Container::new(Space::new(0,
  0))` so the Stack layer is structurally present but visually
  silent. Card layout: `Row[4 px severity-tinted left border |
  text(message) | × button]` inside a `Container` with `radius::SM`
  + `color::PANEL_RAISED` + `space::SM` pad +
  `TOAST_CARD_WIDTH_PX = 320.0` fixed width. Outer column reversed
  (newest at bottom). All strings via `crate::strings::*` — no
  inline literals (zero-string-literals discipline). Severity →
  Lumen token map per ADR-0046 § Decision (zero new tokens).
- [ ] **T-D-N6** — `crates/ui/src/shell.rs:88-97` — wrap the
  existing `Container::new(shell_row)` body in
  `iced::widget::Stack` with two layers (lowest → highest z-order):
  1. The current `shell_row` Container (untouched).
  2. `widgets::toast_tray::view(&model.toast_queue, mode)`.
  Outer styling closure (background `color::CANVAS`, text color
  `color::FG_1`) stays on the wrapped Container so the shell-grid
  invariant test stays green. Tray padding-bottom = 28 px (clears
  24 px activity tape + 4 px gap).
- [ ] **T-D-N7** — `crates/ui/src/widgets/mod.rs` — add
  `pub mod toast_tray;`. Alphabetical placement between
  `throttled_spinner` and `trail_drawer`.

### Wave C — producer migration + integration test (R3-R4, ~120 LOC)

- [ ] **T-D-N8** — `crates/ui/src/bin/cockpit_live.rs:1143` —
  migrate the training spawn-failure mutation
  (`self.cockpit.toast_message = Some(...)` direct write) to route
  through the message dispatcher:
  `self.update(Message::ShowToastWithSeverity(SmolStr::new(
  format!("Training failed to launch: {e}")), ToastSeverity::Danger))`
  (R3.2). _Acceptance:_ existing
  `cockpit_training_pressed_wiring.rs::spawn_failure_surfaces_toast`
  passes via the back-compat shim (assertion shape unchanged).
- [ ] **T-D-N9** — NEW file `crates/ui/tests/cockpit_toast_queue.rs`.
  4 integration tests (R2/R4 acceptance):
  1. `queue_displays_multiple` — dispatch 3 `ShowToast` messages,
     assert `toast_queue.len() == 3` + ordering matches dispatch.
  2. `auto_dismiss_after_timeout` — enqueue 1 toast at
     `Instant::now()`; dispatch
     `Message::ToastTick(t0 + TOAST_AUTODISMISS + Duration::from_millis(1))`;
     assert queue is empty. No fake-clock infrastructure — the
     message-payload Instant is the test seam.
  3. `two_completions_in_rapid_succession_both_visible` — R4.1
     stronger K5 contract: dispatch two completion-style toasts
     within the same logical instant; assert both are queue-
     resident; assert the back-compat `toast_message()` shim
     returns the FIRST (front) entry.
  4. `overflow_drops_oldest_keeps_newest` — enqueue 6 with cap 5;
     assert `len() == 5`; assert front id corresponds to the 2nd
     enqueued (oldest dropped).

### Wave D — auto-dismiss ticker + back-compat shim (R2.5, R5.9, ~60 LOC)

- [ ] **T-D-N10** — `crates/ui/src/bin/cockpit_live.rs` (top of
  file, alongside `ServerTimeRecipe` at lines 127-152) — add
  `ToastDismissRecipe { rt_handle: tokio::runtime::Handle }`
  + `Recipe` impl emitting `Message::ToastTick(Instant::now())`
  every 500 ms. Stream body delegates to
  `ui::live::toast_dismiss_stream_impl(&self.rt_handle)` (new
  helper colocated with `server_time_stream_impl` for test reach).
  Wire as the 6th batched subscription in
  `cockpit_live.rs::subscription()` (alongside `bus_sub`,
  `time_sub`, `trail_sub`, `progress_sub`, `activity_sub`,
  `training_log_sub`) — both the modal-open and modal-closed
  batches. Always-on (no salt / no per-run gating).
- [ ] **T-D-N11** — `crates/ui/src/state.rs` — add
  `pub fn toast_message(&self) -> Option<&SmolStr>` on `AppState`
  returning `self.toast_queue.front().map(|t| &t.message)` with
  doc-comment `// MIGRATION: remove at v0.2.0 cleanup brief`
  (R5.9). _Acceptance:_ the 6 `cockpit.toast_message` reads in
  `cockpit_training_pressed_wiring.rs` (lines 125, 196-197, 323,
  329, 332, 365-368) all compile + pass unchanged.
- [ ] **T-D-N12** — Verify parent feature K5 regression:
  `cargo test -p ui --test cockpit_training_pressed_wiring`.
  Expect 5/5 PASS unchanged. _Acceptance:_ R4.2 stays green.

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
