---
slug: cockpit-toast-queue
version: 0.1.0
status: shipped
owner: shipped
updated: 2026-05-27
predecessor: cockpit-training-pressed-wiring v0.1.0
priority: P2
---

# Cockpit toast subsystem — bounded queue replacing single-slot REPLACE semantic

> **Spawned from `cockpit-training-pressed-wiring v0.1.0 M-DEV K5
> follow-on`.** The architect's M-T1 T-AR-2 K5 decision
> (`spec/cockpit-training-pressed-wiring/tasks.md ## T-AR-2`) inspected
> the existing toast subsystem at
> [`crates/ui/src/state.rs:2056-2061`](../../crates/ui/src/state.rs)
> and confirmed it is single-slot REPLACE: `Message::ShowToast(msg) →
> model.toast_message = Some(msg)`; the storage slot at
> `state.rs:816` is `toast_message: Option<SmolStr>`. At v0.1.0 of the
> parent feature this was acceptable (the K5 non-clobber assertion at
> `crates/ui/tests/cockpit_training_pressed_wiring.rs::k5_toast_non_clobber_run_completed_then_training_completed`
> succeeds only because Training completion currently emits NO
> auto-toast — silent no-op). The deeper question — _what should the
> cockpit DO when multiple actions complete in quick succession_ —
> was explicitly deferred:
>
> > Multi-toast queueing deferred to follow-on `cockpit-toast-queue`.
> > — T-AR-2, 2026-05-26
>
> This brief picks up that deferral.

## Why now

### State today (2026-05-27)

- **Single-slot REPLACE**: `AppState.toast_message: Option<SmolStr>`
  (`state.rs:816`). Setting via `Message::ShowToast(SmolStr)`
  unconditionally overwrites; clearing via `Message::DismissToast`
  sets `None` (`state.rs:2056-2061`).
- **Two producers today**:
  1. Lab Compare cap-hit (`state.rs:1983` —
     `crate::strings::LAB_COMPARE_CAP_HIT`).
  2. Training spawn-failure (`bin/cockpit_live.rs:1101` —
     `"Training failed to launch: {e}"`).
- **No view-side render path exists**. Toast bytes are stored in
  `AppState` but `grep -rn toast` across `crates/ui/src/screens/`,
  `crates/ui/src/shell.rs`, and `crates/ui/src/widgets/` returns
  zero render call-sites. The toast subsystem is currently
  **set-only** — the operator never SEES a toast today; producers
  fire blind into a slot nothing reads. This brief includes the
  view-side render path as part of v0.1.0 surface.

### The K5 silent-clobber scenario

The architect's T-AR-2 logic ("Training completion does NOT
auto-set a toast at v0.1.0; therefore the silent no-op is safe")
holds **only as long as no producer emits a completion toast**.
Three queued briefs each plan to add completion toasts:

- `cockpit-activity-audit-ledger-producer v0.1.0` (Active) — Q2=(a)
  redacted "Audit: N writes" tape entries with idle-end → toast on
  failure.
- `cockpit-activity-llm-producer v0.1.0` (Active) — surfaces
  per-LLM-call completion as activity events; failure-mode toasts
  are the natural escalation.
- `v5-latency-slippage-sim v0.1.0` (Active, in-progress) —
  emits `AuditEvent::SimulatedExecMetrics` per scenario; a
  divergence-from-baseline toast is a likely v0.2.0 follow-on.

Once any two of these land, the K5 assertion silently regresses:
operator runs Lab Run + Train back-to-back; both completion events
fire within seconds; the second toast wins; the operator misses
the first signal entirely. The current single-slot REPLACE is
**fragile by construction** — every new producer added to the
codebase widens the clobber surface.

### Why now (not v0.2.0 of the parent)

- **Cheap fix while the surface is small.** Only 2 producer
  call-sites today; ~3-5 by Q3. Bounded queue ships in ~1-2 days;
  retrofitting after 8+ producers is significantly more work.
- **Anchor risk zero.** UI-only; no backtest / strategy / exec /
  audit / data crate touches; the 34 locked anchors stay byte-
  identical.
- **Unblocks audit-ledger + LLM producer briefs.** Both will want
  to surface non-clobbering completion toasts; landing the queue
  first removes a coordination gate.
- **No view-render today means we can author the view at the same
  time as the queue** — no anchored screenshot regression, no
  Lumen token retrofit (we pick the design clean).

## Requirements

Numbered, testable, derived from the K5 framing above + the
existing `cockpit-activity-status-bar v0.1.0` Lumen + iced
precedents.

### R1 — Bounded queue replaces `Option<SmolStr>`

- **R1.1** Storage slot at `state.rs:816` widens from
  `toast_message: Option<SmolStr>` to a bounded queue type.
  Architect locks the exact shape at M-T1; analyst-recommended
  default: `toast_queue: VecDeque<ToastEntry>` with `MAX_TOAST_QUEUE_LEN`
  capacity (analyst-recommended **5**; see Q2).
- **R1.2** New `ToastEntry` struct:
  ```rust
  pub struct ToastEntry {
      pub id: u64,                 // monotonic; tap from a Cell counter
      pub message: SmolStr,
      pub severity: ToastSeverity, // Info / Success / Warning / Danger
      pub created_at: Instant,     // for auto-dismiss timeout (Q3)
  }
  pub enum ToastSeverity { Info, Success, Warning, Danger }
  ```
- **R1.3** `Message::ShowToast(SmolStr)` arm at `state.rs:2056`
  rewrites to enqueue. Overflow policy: drop OLDEST (FIFO ring) —
  rationale: newest toast is most likely the operator-relevant one.
- **R1.4** `Message::DismissToast` arm at `state.rs:2059` rewrites
  to dismiss the FRONT entry. New `Message::DismissToastById(u64)`
  for arbitrary dismiss (needed by stacked display Q1=(a)).
- **R1.5** New constructor `Message::ShowToastWithSeverity(SmolStr,
  ToastSeverity)` so producers can tag color without parsing the
  string. The original `ShowToast` arm maps to `Info` severity by
  default (backwards-compatible with the 2 existing call sites).
- **Acceptance:** Unit tests in `crates/ui/src/state.rs::tests`:
  - `toast_queue_enqueue_basic` — enqueue 3 distinct, queue has 3.
  - `toast_queue_overflow_drops_oldest` — enqueue 6 with cap 5, front entry dropped.
  - `toast_queue_dismiss_by_id` — enqueue 3, dismiss middle, queue has 2.
  - `show_toast_msg_back_compat` — `ShowToast` enqueues with `Info` severity.

### R2 — View-side render path (NEW surface)

- **R2.1** New widget `crates/ui/src/widgets/toast_tray.rs` (~120
  LOC). Renders a stacked vertical list of ≤ `MAX_TOAST_QUEUE_LEN`
  toast entries in the bottom-right corner of the shell. Each
  entry is a Lumen card with severity-tinted left border + message
  text + manual-dismiss × button.
- **R2.2** Lumen token reuse — NO new tokens introduced
  (`R-NR.4`). Mapping:
  - `Info`     → `color::FG_2` border
  - `Success`  → `color::SUCCESS` (existing)
  - `Warning`  → `color::WARNING` (existing)
  - `Danger`   → `color::DANGER` (existing)
- **R2.3** Wire into `crates/ui/src/shell.rs` as a top-layer
  overlay (`iced::widget::stack` or equivalent); placement
  decision is Q4 operator-decide. Analyst-recommended **above** the
  24 px bottom status bar (the activity tape) so the tape +
  toasts coexist visually distinct (analyst-recommended Q4=(a)).
- **R2.4** Manual-dismiss × button on each entry emits
  `Message::DismissToastById(id)`.
- **R2.5** Auto-dismiss timeout — new iced `Subscription` recipe
  `ToastDismissTicker` (1 Hz cadence; or `time::every(Duration::from_millis(500))`
  if architect prefers). At each tick, walks
  `toast_queue` and removes entries where `created_at.elapsed() >
  TOAST_AUTODISMISS`. Analyst-recommended timeout **5s** (Q3).
- **Acceptance:**
  - Manual cockpit run: trigger 4 toasts in succession, observe
    all 4 stacked in bottom-right; each auto-fades at 5s; manual
    × dismisses individual entries.
  - Integration test
    `crates/ui/tests/cockpit_toast_queue.rs::queue_displays_multiple`
    constructs `AppState`, dispatches 3 `ShowToast` messages, asserts
    `toast_queue.len() == 3` + (no view-test today; visual smoke at
    M-FINAL).
  - Integration test
    `cockpit_toast_queue.rs::auto_dismiss_after_timeout` —
    inject 1 toast, fast-forward `Instant` (or use a fake clock),
    dispatch the ticker tick, assert queue is empty.

### R3 — Producer migration

Audit existing call-sites and migrate to the new API; surface any
that should now carry a severity tag.

- **R3.1** Migrate `crates/ui/src/state.rs:1983` (Lab Compare
  cap-hit) — was `Some(SmolStr::new(LAB_COMPARE_CAP_HIT))`, now
  `enqueue(ToastEntry { severity: Warning, .. })`.
- **R3.2** Migrate `crates/ui/src/bin/cockpit_live.rs:1101`
  (Training spawn-failure) — was
  `Some(SmolStr::new(format!("Training failed to launch: {e}")))`,
  now `enqueue(ToastEntry { severity: Danger, .. })`.
- **R3.3** Both migrations are field-mechanical; no behavior
  change at the producer level.
- **Acceptance:** existing tests at
  `cockpit_training_pressed_wiring.rs::spawn_failure_surfaces_toast`
  and the Lab Compare cap-hit pass at the new API (assertion shape
  changes from `toast_message.is_some()` to
  `!toast_queue.is_empty()`).

### R4 — K5 non-clobber assertion graduates

- **R4.1** The current K5 non-clobber assertion at
  `cockpit_training_pressed_wiring.rs::k5_toast_non_clobber_run_completed_then_training_completed`
  succeeds because Training completion emits NO toast. With the
  queue in place, the assertion graduates to a stronger contract:
  *"Two completion toasts emitted in rapid succession are BOTH
  visible to the operator (queue length 2, both entries
  reachable)."* The new test is
  `cockpit_toast_queue.rs::two_completions_in_rapid_succession_both_visible`.
- **R4.2** The original K5 assertion in the parent feature stays
  green (back-compat) — the `cockpit.toast_message` shim alias
  resolves to `cockpit.toast_queue.front().map(|t| t.message.clone())`
  for the duration of the migration; can be removed at the
  v0.2.0 cleanup pass.
- **Acceptance:**
  - The parent feature's K5 test passes UNCHANGED.
  - The new stronger contract test PASSES.

### R5 — Non-regression contract

- **R5.1** **All 34 anchors stay byte-identical.** Zero touched
  files in `crates/backtest/`, `crates/strategy/`, `crates/exec/`,
  `crates/risk/`, `crates/reports/`, `crates/forecast/`,
  `crates/audit/`, `crates/cost/`, `crates/data/`.
- **R5.2** **No new audit migration.** No persistence; toasts are
  ephemeral session-only state.
- **R5.3** **No bus channel changes.** The activity bus shipped at
  `cockpit-activity-status-bar v0.1.0` is untouched. Toasts are a
  PARALLEL surface (the queue itself does not subscribe to the
  activity broadcast; the dispatcher and the toast queue are
  intentionally separate — see Q4).
- **R5.4** **No new Lumen tokens** (R2.2).
- **R5.5** **`cockpit-smoke` 0 panics.**
- **R5.6** **818+ workspace tests stay green.**
- **R5.7** **`spec-lint`** introduces no new violation categories.
- **R5.8** **`scripts/verify_anchors.sh`** exits 0 with 34/34 PASS.
- **R5.9** **Back-compat alias:** the `toast_message` field
  rename is a SOFT migration — analyst-recommended that
  architect adds a `pub fn toast_message(&self) -> Option<&SmolStr>`
  helper on `AppState` returning the queue front, so existing
  tests / downstream readers compile unchanged. Hard removal
  deferred to a v0.2.0 cleanup pass.

## Hypothesis register

- **H1** — *Operators want to see SUCCESS/COMPLETION toasts from
  the last 3-5 actions, not just the most recent.* **Falsifier**:
  in operator demo at M-PRESENTER, operator reports "the queue is
  noisy — I only want the most recent." Mitigation: queue capacity
  is configurable; default 5 is easy to flip down to 1 to recover
  the old REPLACE semantic. **Status at analyst pass**: assumed
  TRUE based on the K5 scenario (operator runs Lab Run + Train
  back-to-back).
- **H2** — *Auto-timeout at 5s is acceptable; the operator's
  glance cadence at the bottom-right corner is ~3-7s while doing
  other work.* **Falsifier**: operator reports "the toast
  disappeared before I saw it" at M-PRESENTER. Mitigation: 5s
  is the analyst default; operator can flip to 10s or "never" via
  Q3 re-decision; manual × dismiss always available.
- **H3** — *Stacked vertical display in bottom-right does not
  occlude critical operator workflows.* **Falsifier**: operator
  reports the toast tray overlaps the Lab Run progress bar or the
  activity tape's "+N more" tail at high queue depth. Mitigation:
  queue capacity (5) + bottom-right placement matches macOS /
  GitHub Desktop / Linear precedent — analyst-recommended Q1=(a)
  stacked is the dominant cross-app pattern.
- **H4** — *The activity-tape (cockpit-activity-status-bar v0.1.0)
  and the toast queue serve DIFFERENT purposes* — tape =
  "what is currently happening" (running, throttled to 100 ms);
  queue = "what just COMPLETED / FAILED" (terminal events,
  attention-grabbing). **Falsifier**: operator says "these are
  redundant; pick one." Mitigation: K2 risk register entry below.

## Risk register

- **K1** — **Visual clutter at high queue depth.** If a backtest
  produces 5+ rapid completion toasts, the bottom-right corner
  becomes a wall of cards that blocks the activity tape's
  "+N more" affordance. **Mitigation**: cap default at 5; FIFO
  ring drop-oldest behavior caps the max stack height; M-PRESENTER
  visual smoke at depth-3 + depth-5 manually validated. Q2
  decision is operator-overridable.
- **K2** — **Activity tape / toast queue surface overlap.** Both
  surface "what's happening" but at different cadences (tape =
  100 ms in-flight throttle, queue = terminal events). Operators
  may not understand which surface to look at. **Mitigation**:
  H4 framing — tape is for IN-FLIGHT, queue is for COMPLETED /
  FAILED. The two surfaces SHOULD coexist; we document the
  distinction in the M-PRESENTER deck. If H4 falsifies, fallback
  is to merge — but that's a v0.2.0 re-architecture.
- **K3** — **Toast spam after `cockpit-activity-audit-ledger-producer
  v0.1.0` ships.** Per that brief's K3, the audit-ledger writer
  fans out at thousands/sec during a fast backtest. If any of
  those audit writes generates a toast (e.g. a failure toast on
  ledger-write-error), the queue is overwhelmed in seconds.
  **Mitigation**: producer-side aggregation envelope (same 100 ms
  pattern the audit-ledger brief uses for its `ActivityKind`).
  Toast queue itself does NOT subscribe to the audit broadcast —
  intentionally. Toast emission must be a deliberate producer
  call. This brief documents the contract; the producers respect
  it.
- **K4** — **Accessibility (screen-reader friendliness).** Stacked
  toast cards are notoriously poor for assistive tech (the entire
  visual hierarchy is gone). **Mitigation**: at v0.1.0 we
  document this gap in feature.md and defer the ARIA / live-region
  wire-up to a future `cockpit-toast-a11y` follow-on. Toast
  message text is already plain-string SmolStr — no semantic loss,
  just no live-region announcement.
- **K5** — **`Instant`-based auto-dismiss is non-deterministic in
  tests.** **Mitigation**: architect locks the time source at
  M-T1 — analyst-recommended pattern is a `dyn Fn() -> Instant`
  injection point on `AppState` so tests can fake the clock
  (matches the `time_source` pattern already in
  `crates/agent/src/clock.rs`). Existing precedent in tree.
- **K6** — **Back-compat shim drift.** The `toast_message()`
  helper alias (R5.9) is easy to forget about; a future refactor
  might delete it before the v0.2.0 cleanup. **Mitigation**:
  doc-comment with `// MIGRATION: remove at v0.2.0` annotation;
  the cleanup brief makes its removal an explicit task.
- **K7** — **Severity-color coupling drift.** R1.5 introduces
  `ToastSeverity::Info/Success/Warning/Danger`, but adding new
  severities later (e.g. `Trace`) requires both the enum AND the
  view-side color mapping AND every producer call-site. **Mitigation**:
  severity is bounded to 4 at v0.1.0; new variants deferred to a
  future brief.

## Open questions for the operator

All four are standing-Autoapprove-eligible at analyst-recommended
defaults — the cost of a wrong default is < 50 LOC to flip.

- **Q1 — Display strategy.**
  - (a) **Stacked vertical cards in bottom-right corner** ← **ANALYST DEFAULT**
    — most common pattern in modern productivity apps (macOS
    Notifications, GitHub Desktop, Linear, Slack). Operator
    cognitive load is low because the pattern is familiar.
  - (b) Carousel auto-advance — one toast visible at a time,
    auto-advances every 3-5s. Lower visual clutter but
    operator might miss an entry if not glancing at the right
    moment.
  - (c) Dropdown menu — single bell icon in shell header; click
    to expand all toasts. Hides the surface by default; OK for
    rare-event UX, wrong for the K5 multi-completion scenario.
  - (d) Toast tray — fixed-position bottom-right with a max-2-visible
    + "+N more" tail mirroring the activity tape's pattern.
    Consistent with the existing tape surface but introduces a
    duplicate "+N more" footer that may confuse operators
    (which "+N more" is which?).

  **Trade-off:** (a) is the load-bearing pattern — bottom-right
  stacked is what every operator already expects from desktop
  notifications. The other 3 are evaluator's-distillation
  alternatives — none is wrong, but (a) is the lowest-friction
  choice.

- **Q2 — Queue capacity.**
  - (a) 3 — minimum for the K5 scenario (Lab + Train +
    audit-failure simultaneously).
  - (b) **5** ← **ANALYST DEFAULT** — covers K5 + a margin for
    rapid-fire failure cascades.
  - (c) 10 — over-large; visual clutter risk K1 amplifies.
  - (d) Unbounded — bug-by-construction; will eventually OOM on
    pathological producers.

  **Trade-off:** 5 is the sweet spot between "covers realistic
  burst" and "doesn't tile the corner of the screen."

- **Q3 — Auto-dismiss timeout.**
  - (a) 3s — too fast; operator glance cadence is 3-7s.
  - (b) **5s** ← **ANALYST DEFAULT** — matches macOS
    Notifications default; aligns with H2.
  - (c) 10s — generous; may feel sticky if the operator dismisses
    manually anyway.
  - (d) Never (manual-dismiss only) — bug-prone (operator
    forgets; queue stays full).

  **Trade-off:** 5s is the default for ~every notification system
  in modern OS; H2 is testable at M-PRESENTER demo.

- **Q4 — Placement relative to activity tape.**
  - (a) **Above the activity tape** ← **ANALYST DEFAULT** — the
    24 px tape stays anchored to the bottom edge; toasts float
    above it. Toasts (completed/failed events) sit "stacked on
    top of" the live activity stream. Reads naturally
    chronologically: tape = NOW, toasts = JUST-FINISHED above.
  - (b) Below the activity tape — pushes the tape up off the
    bottom edge; breaks the
    [`lumen-phase-1-foundation R13 24 px bottom`](../lumen-design-adoption/phase-1-foundation/feature.md)
    contract. Hard veto.
  - (c) Merged with the activity tape — same surface, terminal
    events get a special hold style. Discussed in K2; defer to
    v0.2.0 if H4 falsifies.
  - (d) Top-right of the shell — common in some apps (browsers,
    macOS Big Sur+); but the cockpit's top-right is reserved for
    Lumen Phase F right-rail Assistant slot
    (`spec/ui-rethink-phase-f-memory-models-assistant`), so this
    would conflict.

  **Trade-off:** (a) honors the existing tape contract + tracks
  the dominant desktop pattern. (b) violates a shipped Lumen
  constraint. (c) is a v0.2.0 re-architecture. (d) collides with
  Phase F.

## Out of scope

- **Toast persistence / history view.** Toasts are ephemeral
  session-only. The history of completed/failed events lives in
  the audit ledger (already on disk) + the activity tape's
  in-memory ring. A "review past toasts" UI is a future
  `cockpit-toast-history` follow-on (no operator request today).
- **Screen-reader / ARIA live-region announcements.** Deferred to
  `cockpit-toast-a11y` follow-on per K4.
- **Rich-content toasts** (icons, action buttons, inline
  progress). v0.1.0 = plain text + severity tint + manual ×.
- **Per-producer toast preferences.** No user-facing
  "mute Training toasts" setting at v0.1.0; deferred.
- **Action buttons on toasts** ("Cancel run", "View report"). The
  toast is a notification, not a control surface; clickable
  surfaces live in their owning panel.

## Backtest scenarios

**None.** This is a UI-only feature. It does not touch the
backtest engine, scenario producers, or any anchored report. The
34 locked anchors stay byte-identical (R5.1). Verified at
M-FINAL by `scripts/verify_anchors.sh`.

## Cost estimate

**~2-3 days end-to-end wall-clock**, distributed:

- M0 analyst: ~0.5 day (this pass).
- M-OD operator-decide: ~30 min (Q1-Q4, all standing-Autoapprove
  at analyst defaults).
- M-T1 architect: ~0.5 day — lock storage shape; pick the iced
  recipe pattern for the auto-dismiss ticker; resolve K5 clock
  injection.
- M-DEV developer: ~1-2 days — Wave A queue+message arms (R1,
  ~80 LOC + 4 unit tests); Wave B widget+shell wiring (R2, ~150
  LOC); Wave C producer migration + new integration test file
  (~120 LOC); Wave D auto-dismiss ticker recipe (~60 LOC).
- M-FINAL tester: ~0.5 day — anchor sweep, cockpit-smoke,
  workspace test sweep, manual visual smoke at depth-3 + depth-5.
- M-PRESENTER: ~0.5 day — deck + 4-cell verdict tree (SHIP /
  SHIP-with-Q1-flip / HOLD-on-K2-falsification / RE-ARCH).

## Cross-references

- **Predecessor**: [`cockpit-training-pressed-wiring v0.1.0`](../cockpit-training-pressed-wiring/feature.md)
  — K5 deferral note at T-AR-2 (2026-05-26); this brief picks
  that thread up.
- **Sibling surface** (status-strip bottom row): [`cockpit-activity-status-bar v0.1.0`](../cockpit-activity-status-bar/feature.md)
  — 24 px bottom bar contract that Q4=(a) sits above.
- **High-frequency producer risk** (K3): [`cockpit-activity-audit-ledger-producer v0.1.0`](../cockpit-activity-audit-ledger-producer/feature.md)
  — producer-side aggregation envelope precedent the toast queue
  inherits.
- **Future producer**: [`cockpit-activity-llm-producer v0.1.0`](../cockpit-activity-llm-producer/feature.md)
  — Active queued; will likely surface per-call completion toasts.
- **Lumen contract**: [`lumen-phase-1-foundation R13`](../lumen-design-adoption/phase-1-foundation/feature.md)
  — the 24 px bottom status bar; Q4=(b) hard-vetoed because it
  violates this.

## Design

_architect 2026-05-27 — locked at M-T1; see [ADR-0046](../architecture/adr/0046-cockpit-toast-queue.md)._

### Storage & types

- **Field**: `AppState.toast_queue: VecDeque<ToastEntry>` replaces
  `toast_message: Option<SmolStr>` at `state.rs:816`. The two
  constructors at `state.rs:1055` + `state.rs:1159` initialize
  `VecDeque::with_capacity(MAX_TOAST_QUEUE_LEN)`. `Debug` impl at
  `state.rs:1000` updates the field name.
- **ID counter**: `AppState.toast_next_id: Cell<u64>` (per-instance;
  no cross-instance contention; matches `training_log_recipe_salt`
  precedent).
- **Const**: `pub const MAX_TOAST_QUEUE_LEN: usize = 5;` and
  `pub const TOAST_AUTODISMISS: Duration = Duration::from_secs(5);`
  both colocated in `state.rs` near the `AppState` type.
- **`ToastEntry`**:
  ```rust
  #[derive(Clone, Debug, PartialEq)]
  pub struct ToastEntry {
      pub id: u64,
      pub message: SmolStr,
      pub severity: ToastSeverity,
      pub created_at: Instant,
  }

  #[derive(Clone, Copy, Debug, PartialEq, Eq)]
  pub enum ToastSeverity { Info, Success, Warning, Danger }
  ```

### Message arms (`state.rs:1466-1468` region + new variants)

- `Message::ShowToast(SmolStr)` retained → `enqueue(ToastEntry {
  severity: Info, .. })`. Back-compat for 2 existing producers + the
  parent feature's K5 test.
- `Message::ShowToastWithSeverity(SmolStr, ToastSeverity)` NEW.
  Both enqueue paths share `enqueue_toast(&mut self, msg, sev)`
  helper that allocates a new `id`, stamps `Instant::now()`, drops
  the front entry if `len == cap`, and pushes back.
- `Message::DismissToast` retained → `pop_front()`.
- `Message::DismissToastById(u64)` NEW → `retain(|t| t.id != id)`.
- `Message::ToastTick(Instant)` NEW → walks `toast_queue` and
  `retain(|t| now.duration_since(t.created_at) < TOAST_AUTODISMISS)`.
  **Important**: `Instant` is carried in the message payload, NOT
  read inside the arm — this is the K5 clock-injection point. Tests
  pass a synthetic instant.

### View (`crates/ui/src/widgets/toast_tray.rs` — NEW, ~150 LOC)

- Public entry: `pub fn view<'a>(queue: &'a VecDeque<ToastEntry>,
  mode: ThemeMode) -> Element<'a, Message>`. Returns
  `Container::new(Space::new(0, 0))` when `queue.is_empty()` — the
  Stack layer is structurally present but pixel-empty (mirrors the
  `journal_transaction_modal` empty-layer pattern; an empty Stack
  layer is byte-identical to no overlay).
- Layout: outer `Container` pinned to bottom-right of its parent
  cell via `align_x(Right) + align_y(Bottom)` chrome; inner
  `Column<ToastCard>` reversed (newest at bottom of the stack —
  matches macOS Notifications direction). Per-card width fixed at
  `TOAST_CARD_WIDTH_PX = 320.0` to avoid the cards growing with
  variable message length.
- Each card: `Row[severity-tinted-border | message text | × button]`
  inside a `Container` with `radius::SM` corner, `space::SM`
  internal padding, `color::PANEL_RAISED` background, severity
  border on the left edge (4 px). Mapping (zero new Lumen tokens):
  - `Info`     → `color::FG_2`
  - `Success`  → `color::UP_500`
  - `Warning`  → `color::INFO_400`
  - `Danger`   → `color::DOWN_500`
- `×` button: text glyph "×" only (no icon font); on press emits
  `Message::DismissToastById(entry.id)`. The button styling reuses
  existing button chrome (`button::text` if available; else the
  status-bar's plain `button` style).

### Shell wiring (`crates/ui/src/shell.rs:88-97` region)

- Wrap the existing `Container::new(shell_row)` body in
  `iced::widget::Stack` with two layers (lowest to highest z-order):
  1. The current `shell_row` Container (untouched chrome).
  2. `toast_tray::view(&model.toast_queue, mode)` — only renders
     visible content when the queue is non-empty.
- The Stack-wrapped outer Container retains the existing style
  closure (background `color::CANVAS`, `text_color: color::FG_1`)
  so the shell-grid invariant test stays green.
- Placement: the tray's `align_y(Bottom)` chrome stacks the cards
  upward FROM the bottom edge, but with a 28 px bottom offset
  (`padding::bottom(28)`) so the 24 px activity tape stays
  uncovered. This is Q4=(a) "above the activity tape."

### Auto-dismiss ticker (`crates/ui/src/bin/cockpit_live.rs` —
recipe + subscription wiring)

- NEW recipe `ToastDismissRecipe` colocated with `ServerTimeRecipe`
  at the top of `cockpit_live.rs`. Same `rt_handle` injection
  pattern; emits `Message::ToastTick(Instant::now())` every 500 ms
  via `tokio::time::interval` + `tokio_stream::wrappers::IntervalStream`.
  Stream body extracted to `ui::live::toast_dismiss_stream_impl` for
  test reachability (mirrors `server_time_stream_impl` precedent).
- Wire into `cockpit_live.rs::subscription()` as the 6th batched
  recipe (alongside `bus_sub`, `time_sub`, `trail_sub`,
  `progress_sub`, `activity_sub`, `training_log_sub`). Active
  unconditionally — the 500 ms idle cost is negligible vs the
  100 ms activity-tape tick.
- The `update` wrapper in `cockpit_live.rs` routes `ToastTick` to
  the pure `state.rs::update`. No binary-side state needed.

### Back-compat shim

- `pub fn toast_message(&self) -> Option<&SmolStr>` on `AppState`,
  doc-commented:
  ```rust
  /// MIGRATION: remove at v0.2.0 cleanup. Existed for one cycle to
  /// keep cockpit-training-pressed-wiring v0.1.0 K5 test compiling
  /// unchanged.
  ```
- Behavior: `self.toast_queue.front().map(|t| &t.message)`.

### Producer migration

- **Lab Compare cap-hit** (`state.rs:1983`):
  was `model.toast_message = Some(SmolStr::new(LAB_COMPARE_CAP_HIT));`
  becomes (handled at the `Message::ShowToastWithSeverity(.., Warning)`
  arm — emit it instead of mutating the slot inline OR call the
  `enqueue_toast` helper directly with severity = `Warning`).
- **Training spawn-failure** (`bin/cockpit_live.rs:1143`):
  was `self.cockpit.toast_message = Some(...)` direct mutation;
  becomes `self.update(Message::ShowToastWithSeverity(msg, Danger))`
  routed through the normal message path.

### Test plan delta

The 4 unit tests + 4 integration tests in feature.md R1/R2 acceptance
are honored as-is by tasks.md Wave-A T-D-N4 and Wave-C T-D-N9. The
`auto_dismiss_after_timeout` test sends a synthetic
`Message::ToastTick(future_instant)` rather than wiring a fake clock
field — simpler test ergonomics.

### File scope (developer-visible touch list)

- `crates/ui/src/state.rs` — replace field, add types/messages/arms,
  add back-compat shim. (~80 LOC + 4 unit tests inline.)
- `crates/ui/src/widgets/toast_tray.rs` — NEW file (~150 LOC).
- `crates/ui/src/widgets/mod.rs` — add `pub mod toast_tray;`.
- `crates/ui/src/shell.rs` — wrap shell body in `Stack`, push
  `toast_tray::view` overlay.
- `crates/ui/src/bin/cockpit_live.rs` — add `ToastDismissRecipe`,
  wire into `subscription()`, migrate the training spawn-failure
  producer call. (~60 LOC.)
- `crates/ui/src/live.rs` — add `pub fn toast_dismiss_stream_impl`
  (mirror of `server_time_stream_impl`).
- `crates/ui/tests/cockpit_toast_queue.rs` — NEW integration test
  file with 4 tests (~120 LOC).
- `crates/ui/tests/cockpit_training_pressed_wiring.rs` — UNCHANGED.
  The R5.9 back-compat shim is the explicit contract that keeps
  this file byte-stable.

**Zero overlap with the in-flight `lab-yahoo-realdata v0.1.1`
developer** (which touches `crates/data/`, `crates/strategy/`,
`spec/anchors.toml`, `spec/lab-yahoo-realdata/`). UI crate is
ours; data/strategy/anchors are theirs.

## Changelog

- 2026-05-27 (analyst): authored v0.1.0 draft. R1-R5 + H1-H4 +
  K1-K7 + Q1-Q4 closed; analyst-recommended defaults locked on
  all four operator questions. Anchor risk zero by construction.
  Cost ~2-3 days. HANDOFF → architect for M-T1 decomposition.
- 2026-05-27 (architect): M-T1 close. ADR-0046 authored locking
  `VecDeque<ToastEntry>` capped at 5, drop-oldest FIFO,
  `Message::ToastTick(Instant)` clock injection via message
  payload, stacked Lumen cards in bottom-right above the 24 px
  activity tape, severity → existing Lumen token mapping (zero new
  tokens). All 4 analyst Q-defaults retained. Frontmatter flipped
  to `owner: developer`. HANDOFF → developer for Wave A-D
  execution.
- 2026-05-27 (developer): M-DEV complete. Waves A-E shipped. See
  Implementation section below. HANDOFF → tester.

## Implementation

### Waves A-E delivered 2026-05-27

**Wave A — state types + message arms**

- `crates/ui/src/state.rs:37-47` — constants: `MAX_TOAST_QUEUE_LEN=5`,
  `TOAST_AUTODISMISS=Duration::from_secs(5)`, `TOAST_CARD_WIDTH_PX=320.0`,
  `type ToastId=u64`.
- `crates/ui/src/state.rs:55-82` — `ToastSeverity` enum (Info/Success/Warning/Danger),
  `ToastEntry` struct (id, message, severity, created_at).
- `crates/ui/src/state.rs:886+891` — `Cockpit` struct: `toast_queue: VecDeque<ToastEntry>`
  and `toast_next_id: Cell<u64>` added alongside kept `toast_message: Option<SmolStr>`
  (back-compat field — see deviation note below).
- `crates/ui/src/state.rs:1133+1239` — constructors initialize queue with
  `VecDeque::with_capacity(MAX_TOAST_QUEUE_LEN)`.
- `crates/ui/src/state.rs:1258` — back-compat method shim
  `pub fn toast_message(&self) -> Option<&SmolStr>`.
- `crates/ui/src/state.rs:1572-1580` — three new Message variants.
- `crates/ui/src/state.rs:1733` — private `fn enqueue_toast(...)` helper.
- `crates/ui/src/state.rs:2122+2201-2220` — 5 update arms wired.
- `crates/ui/src/state.rs:4171` — 4 unit tests.

**Wave B — toast_tray widget + shell wiring**

- `crates/ui/src/widgets/toast_tray.rs` (NEW, ~187 LoC) — `pub fn view(...)`,
  `fn toast_card(...)`, `fn severity_color(...)`.
- `crates/ui/src/widgets/mod.rs:112` — `pub mod toast_tray;`.
- `crates/ui/src/shell.rs` — `Stack::new()` wraps shell body + toast_tray overlay.
- `crates/ui/src/strings.rs` — `TOAST_DISMISS_BUTTON = "×"` (U+00D7) added.
- `crates/ui/src/gallery/routes.rs` — 2 gallery cells + EXPECTED_WIDGETS entry.
- `crates/ui/src/gallery/mod.rs` — `GALLERY_LOGICAL_HEIGHT` updated to 18040.

**Wave C — integration tests + producer migration**

- `crates/ui/tests/cockpit_toast_queue.rs` (NEW) — 4 integration tests.
- `crates/ui/src/bin/cockpit_live.rs` — training spawn-failure routed through
  `Message::ShowToastWithSeverity(...)`.

**Wave D — ToastDismissRecipe subscription**

- `crates/ui/src/live.rs:824` — `pub fn toast_dismiss_stream_impl(...)`.
- `crates/ui/src/bin/cockpit_live.rs:154-186` — `ToastDismissRecipe` + `Recipe` impl.
- `crates/ui/src/bin/cockpit_live.rs:1622-1651` — wired as 6th sub in both
  modal-open and modal-closed subscription batches.

**Architecture deviation (back-compat field kept)**

ADR-0046 § T-AR-5 specified a back-compat METHOD `pub fn toast_message(&self)`.
The implementation also KEEPS the `pub toast_message: Option<SmolStr>` FIELD
because `cockpit_training_pressed_wiring.rs` directly WRITES to it
(`cockpit.toast_message = Some(...)`), which a method shim cannot support.
Keeping both field and method is idiomatic Rust (field vs method syntax is distinct).
The field is annotated `// MIGRATION: remove at v0.2.0`. Cost: +4 bytes per Cockpit
instance. Risk: zero — write to field does NOT affect the queue; tests remain
self-consistent. Flagged for architect awareness; no ADR update required at v0.1.0.

**Test results**

- `cargo test -p ui --lib`: 397/397 PASS
- `cargo test -p ui --test cockpit_toast_queue`: 4/4 PASS
- `cargo test -p ui --test cockpit_training_pressed_wiring`: 5/5 PASS (K5 regression clean)
- `cargo test -p ui --test shell_grid`: 3/3 PASS
- `cargo test -p ui --test panel_snapshots`: 86/86 PASS
- `scripts/verify_anchors.sh`: 69/69 PASS (all anchors unaffected — UI-only change)
