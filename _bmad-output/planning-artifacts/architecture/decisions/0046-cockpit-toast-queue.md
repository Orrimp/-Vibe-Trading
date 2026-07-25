---
adr: 0046
title: Cockpit toast subsystem — bounded VecDeque queue with shared time-driven dismissal
status: accepted
date: 2026-05-27
supersedes: none
superseded-by: none
---

# ADR-0046: Cockpit toast subsystem — bounded VecDeque queue with shared time-driven dismissal

## Context

The cockpit ships today (post `cockpit-training-pressed-wiring v0.1.0`) with
a single-slot REPLACE toast surface: `AppState.toast_message: Option<SmolStr>`
at [`crates/ui/src/state.rs:816`](../../../../crates/ui/src/state.rs) with
`Message::ShowToast(msg) → model.toast_message = Some(msg)` unconditionally
clobbering any prior toast (`state.rs:2056-2061`). The parent feature's K5
non-clobber assertion only passes because Training completion currently emits
no toast — a silent no-op. Two existing producers and three queued producers
(`cockpit-activity-audit-ledger-producer`, `cockpit-activity-llm-producer`,
`v5-latency-slippage-sim` v0.2.0 divergence toast) will land completion
toasts in the next two cycles; with REPLACE semantics the second toast of any
back-to-back pair silently overwrites the first, dropping operator-visible
signal. The surface is fragile by construction and gets worse with every new
producer.

Additional context: no view-side render of toasts exists today (the slot is
set-only). v0.1.0 must ship both the queue model AND the view. Anchor risk
is zero — UI-only crate touches, no `crates/backtest/`, `crates/strategy/`,
`crates/exec/`, `crates/risk/`, `crates/reports/`, `crates/forecast/`,
`crates/audit/`, `crates/cost/`, or `crates/data/` files mutated.

## Decision

**Storage**: replace `toast_message: Option<SmolStr>` with
`toast_queue: VecDeque<ToastEntry>` capped at `MAX_TOAST_QUEUE_LEN = 5`. The
storage type is `std::collections::VecDeque` (zero new deps, O(1) front/back
ops, already used by `training_events`). Overflow policy is **drop-oldest
FIFO ring** (`pop_front` when `len() == cap` before `push_back`). Severity is
not priority-aware in v0.1.0 — chronological order is the operator's mental
model for "what just completed."

**ToastEntry** is a `Clone + Debug + PartialEq` struct holding `id: u64`
(per-`AppState` monotonic via `Cell<u64>` — matches the
`training_log_recipe_salt` precedent), `message: SmolStr`,
`severity: ToastSeverity` (4-variant enum: `Info | Success | Warning | Danger`),
and `created_at: Instant`.

**Render policy**: stacked vertical Lumen cards in the bottom-right corner,
above the 24 px activity tape, max 5 simultaneously visible (cap matches the
queue depth — no "+N more" overflow chip; the FIFO drop guarantees stack
height never exceeds 5). Each card carries a severity-tinted left border
(`Info → color::FG_2` / `Success → color::UP_500` /
`Warning → color::INFO_400` / `Danger → color::DOWN_500` — all existing Lumen
tokens; **zero new tokens introduced** per R-NR.4), the message text, and a
manual-dismiss `×` button emitting `Message::DismissToastById(u64)`. The
shell composes the tray over the cockpit body via `iced::widget::Stack` (same
primitive `widgets/journal_transaction_modal.rs` already uses).

**Dismissal**: dual path — per-toast auto-timeout (`TOAST_AUTODISMISS =
Duration::from_secs(5)`, evaluated against `Instant`) **plus** manual `×`.
Auto-timeout is driven by a single shared `ToastDismissTicker` recipe
mirroring `ServerTimeRecipe`'s cadence pattern (tokio interval inside the
agent runtime handle, 500 ms tick), emitting `Message::ToastTick(Instant)`.
The `update` arm sweeps `toast_queue` and removes entries whose
`created_at.elapsed() >= TOAST_AUTODISMISS`. Always-on cost is negligible
(the activity-tape already runs a 100 ms tick).

**Clock injection (K5 mitigation)**: `Message::ToastTick(Instant)` carries
the "now" instant as a payload rather than calling `Instant::now()` inside
the update arm. Tests construct `ToastTick(fake_now)` directly; no
`AppState` clock-field is needed — the message-carrying-instant pattern is
test-friendly without widening `AppState`'s footprint.

**Back-compat**: `pub fn toast_message(&self) -> Option<&SmolStr>` shim on
`AppState` returns `self.toast_queue.front().map(|t| &t.message)` so the
parent feature's K5 test (`cockpit_training_pressed_wiring.rs` lines 125,
196-197, 323-368) compiles unchanged. Doc-comment annotates
`// MIGRATION: remove at v0.2.0 cleanup`.

**Message surface**: three variants (one new constructor + one new dismiss-
by-id + the existing `ShowToast` retained for back-compat):

```rust
ShowToast(SmolStr),                            // existing — maps to Info severity
ShowToastWithSeverity(SmolStr, ToastSeverity), // NEW — typed-severity constructor
DismissToast,                                  // existing — dismisses FRONT entry
DismissToastById(u64),                         // NEW — per-card × button target
ToastTick(Instant),                            // NEW — auto-dismiss sweep trigger
```

## Alternatives considered

- **`SmallVec<[ToastEntry; 5]>`** — rejected because `VecDeque` gives O(1)
  front-drop for FIFO overflow without `remove(0)` shift; the `SmallVec` heap
  spill only matters if cap grew >5 and it doesn't (Q2 locked at 5).
- **`BinaryHeap<(Priority, ToastEntry)>`** — rejected because operator mental
  model for completion toasts is chronological ("what just happened"), not
  prioritized; severity-shuffled order surprises rather than helps. Defer
  to v0.2.0 if K2 falsifies.
- **Carousel single-visible auto-advance (Q1=(b))** — rejected per analyst
  default (Q1=(a) stacked); operator can miss an entry if not glancing at
  the right moment, defeating the K5 non-clobber purpose.
- **Per-toast `iced::Task::perform(sleep)` dismissal (T-AR-2 option ii)** —
  rejected because each timer adds state and complicates manual-dismiss
  cancellation (cancelling a per-toast Task on `×` click adds a handle to
  every entry); the shared 500 ms ticker is uniform and idle-cheap.
- **Subscribe the toast queue to the activity broadcast** — rejected (K3):
  the audit-ledger producer fans out at thousands/sec; passive subscription
  would overwhelm the queue. Toast emission stays a deliberate producer
  call. Activity bus stays untouched per R5.3.
- **Hard-rename `toast_message` field (no shim)** — rejected to keep the
  parent feature's K5 test green at v0.1.0; the shim is < 5 LOC and removal
  is a one-liner at v0.2.0.

## Consequences

- **Anchor-additive zero.** No backtest / strategy / exec / audit / reports
  crate touched; `scripts/verify_anchors.sh` stays 34/34 PASS.
- **Single source of truth for "what just completed."** New producers route
  through `Message::ShowToastWithSeverity`; queue size + drop-oldest policy
  is enforced in one place (`state.rs::update`).
- **Test contract.** The 4 unit tests in `state.rs::tests` lock enqueue,
  drop-oldest, dismiss-by-id, and `ShowToast → Info` back-compat. The 4
  integration tests in `crates/ui/tests/cockpit_toast_queue.rs` lock
  multi-display, auto-dismiss, K5 rapid-succession non-clobber, and
  overflow drop-oldest semantics.
- **Parent K5 graduation.** The single-slot K5 contract graduates to a
  stronger multi-visible assertion. The original test stays green via the
  shim; the new contract lives in the new integration test file.
- **Future producer discipline.** Producers MUST call
  `Message::ShowToastWithSeverity(_, _)` and MUST NOT bypass into the
  storage slot directly. The R5.3 contract (no broadcast subscription) is
  enforced by the absence of a subscriber, not by code; a violation
  surfaces as queue overload in operator demos.
- **Severity palette frozen at 4.** Adding `Trace` / `Critical` requires an
  ADR amendment plus producer + view mapping updates. v0.1.0 ships with the
  Lumen-token mapping `Info=FG_2 / Success=UP_500 / Warning=INFO_400 /
  Danger=DOWN_500` — no new tokens.
- **A11y gap explicit.** Stacked toast cards lack ARIA live-region; deferred
  to `cockpit-toast-a11y` follow-on per K4. Documented; not silently
  ignored.

## Changelog

- 2026-05-27 (architect): initial accept. Q1=(a) stacked / Q2=(b) cap 5 /
  Q3=(b) 5s / Q4=(a) above tape — all analyst defaults retained.
