---
slug: cockpit-toast-queue
mode: release
status: draft
audience: human-operator
updated: 2026-05-27
generated: 2026-05-27T00:00:00Z
predecessor: cockpit-training-pressed-wiring v0.1.0 (K5 follow-on, 2026-05-26)
verdict_source: spec/cockpit-toast-queue/reports/test-final-2026-05-27-cockpit-toast-queue.md
adr: spec/architecture/adr/0046-cockpit-toast-queue.md
commit: a723d24
---

# cockpit-toast-queue v0.1.0 — release

## TL;DR

The cockpit's "one toast wipes the previous one" behaviour is gone — multiple
notification cards now stack in the bottom-right corner (max 5, oldest drops),
each auto-fades after 5 seconds or can be closed individually with an `×`
button. Tester verdict **SOFT-PASS** (all automated gates green; the one
remaining hand-eyeball check is bundled into this deck for you to run).

## What changed

- **New widget** — `crates/ui/src/widgets/toast_tray.rs` (~187 lines).
  Renders a stacked column of Lumen cards pinned to the bottom-right corner,
  one card per active notification, sitting 28 px above the existing 24 px
  activity tape so the two surfaces never overlap.
- **State model rewritten** — `crates/ui/src/state.rs`. The old
  `toast_message: Option<SmolStr>` single-slot is now backed by a
  `VecDeque<ToastEntry>` (capacity 5, drop-oldest FIFO). Three new message
  arms (`ShowToastWithSeverity`, `DismissToastById`, `ToastTick(Instant)`),
  one shared `enqueue_toast` helper, and a back-compat `toast_message()`
  method so the predecessor's K5 test keeps compiling unchanged.
- **Auto-dismiss subscription** — `crates/ui/src/live.rs` +
  `crates/ui/src/bin/cockpit_live.rs`. A 6th iced `Recipe`
  (`ToastDismissRecipe`) ticks every 500 ms, sweeps cards older than 5 s,
  and runs alongside the existing time / bus / trail / progress / activity /
  training-log subscriptions.
- **Producer migration** — the two existing producers (Lab Compare cap-hit
  and Training spawn-failure) now route through
  `Message::ShowToastWithSeverity(...)`, picking `Warning` and `Danger`
  severities respectively. No new string literals; severity colours map to
  existing Lumen tokens (`FG_2`, `UP_500`, `INFO_400`, `DOWN_500`) — zero new
  design tokens.
- **Tests** — 4 inline unit tests in `state.rs` + 4 integration tests in
  `crates/ui/tests/cockpit_toast_queue.rs` covering enqueue / overflow /
  by-id dismiss / back-compat / rapid succession / auto-dismiss. All 397
  lib tests + 86 panel snapshots + 5 K5 regression tests stay green.

Public API additions only — no signature changes elsewhere. Zero touched
files in `crates/backtest`, `crates/strategy`, `crates/exec`, `crates/risk`,
`crates/reports`, `crates/forecast`, `crates/audit`, `crates/cost`, or
`crates/data`. All 69 anchored reports stay byte-identical.

## Why it matters

Yesterday's `cockpit-training-pressed-wiring v0.1.0` ship left a known foot-gun
in the toast subsystem: a single `Option<SmolStr>` slot that any producer
could clobber. The K5 regression test only passed because Training completion
emitted **no** toast — a silent no-op. Three queued features were each
preparing to add their own completion toasts (`audit-ledger-producer`,
`llm-producer`, and a likely `v5-latency-slippage-sim` divergence alert), so
the next ship was going to silently flatten the K5 contract: operator runs
Lab Run + Train back-to-back, both completions fire within a second, the
second toast wins, the first signal is lost forever.

This ship closes that surface cleanly. The error messages an operator now
expects to see are guaranteed to be visible, not just stored. The K5 contract
**upgrades** from "one toast is preserved" to "two completions in rapid
succession are both visible to the operator" — the test
`two_completions_in_rapid_succession_both_visible` in
`crates/ui/tests/cockpit_toast_queue.rs` pins it.

## Architecture call-outs

ADR-0046 (`spec/architecture/adr/0046-cockpit-toast-queue.md`) locked the
design before any code was written:

- **Storage** — `VecDeque<ToastEntry>` capped at `MAX_TOAST_QUEUE_LEN = 5`.
  Drop-oldest FIFO on overflow (newest event most likely to be operator-
  relevant). O(1) front-drop; zero new deps.
- **Ticker pattern** — single shared `ToastDismissRecipe` at 500 ms cadence
  (mirror of `ServerTimeRecipe`). Per-toast `Task::perform` rejected because
  manual `×` cancellation would need a handle per entry.
- **Clock injection** — `Message::ToastTick(Instant)` carries the "now"
  stamp in the payload rather than reading `Instant::now()` inside the arm.
  This is the testability trick that lets `auto_dismiss_after_timeout` send
  a synthetic future instant without faking a global clock.
- **Overlay** — `iced::widget::Stack` wraps the existing shell row with the
  toast tray as a higher z-order layer. The shell-grid invariant test
  (`shell_grid_phase_3_entries_are_six`) still passes — Stack is structurally
  invisible when the queue is empty (empty layer = no overlay).
- **Severity tokens** — `Info → FG_2`, `Success → UP_500`, `Warning → INFO_400`,
  `Danger → DOWN_500`. All four already existed in Lumen Phase 1; zero new
  tokens introduced (R-NR.4 honoured).

## Architecture deviation (honest disclosure)

ADR-0046 § T-AR-5 specified a back-compat **method** shim
`pub fn toast_message(&self) -> Option<&SmolStr>`. The implementation also
kept the original `pub toast_message: Option<SmolStr>` **field** alongside
the method, because the predecessor's K5 test writes the field directly
(`cockpit.toast_message = Some(...)`) and a method shim cannot provide a
write target.

The tester walked this deviation independently and labelled it **functionally
sound, non-blocking at v0.1.0**:

- Writes to the legacy field are dead stores relative to the new queue —
  the two storage paths are isolated by construction.
- The K5 test reads via the *method* shim, which correctly resolves to
  `toast_queue.front().map(|t| &t.message)`. Self-consistent.
- Cost: ~24 bytes per `Cockpit` instance. Negligible.
- Annotated `// MIGRATION: remove at v0.2.0` in source. The cleanup brief
  will hard-remove the field once `cockpit_training_pressed_wiring.rs` is
  migrated to the method-based API.

No ADR amendment was deemed necessary at v0.1.0; this is logged for
architect awareness and rolled into the v0.2.0 "what's next" slot below.

## What the operator can do now

- **Run the cockpit and trigger toasts** —
  `cargo run --release -p ui --bin cockpit_live` then exercise the visual
  smoke recipe in the next section. Multiple toasts will stack; each auto-
  fades after 5 s; `×` dismisses individually.
- **Tune the defaults if H1/H2/H3 falsify** — capacity, timeout, and
  placement are all single-constant flips in `crates/ui/src/state.rs`
  (lines 37-47). Cost of a wrong default is < 50 LoC.

## Live demo — automated gate replay

The four toast-queue integration tests, re-run by the presenter at
2026-05-27 18:45 local against commit `a723d24`:

```
$ cargo test -p ui --test cockpit_toast_queue
    Finished `test` profile [unoptimized + debuginfo] target(s) in 1.04s
     Running tests/cockpit_toast_queue.rs (target/debug/deps/cockpit_toast_queue-add20ea83fad1e68)

running 4 tests
test queue_displays_multiple ... ok
test two_completions_in_rapid_succession_both_visible ... ok
test overflow_drops_oldest_keeps_newest ... ok
test auto_dismiss_after_timeout ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s
```

The K5 regression suite from the predecessor feature, replayed to confirm the
back-compat shim still holds:

```
$ cargo test -p ui --test cockpit_training_pressed_wiring
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.80s
     Running tests/cockpit_training_pressed_wiring.rs

running 5 tests
test k5_toast_non_clobber_run_completed_then_training_completed ... ok
test spawn_failure_surfaces_toast ... ok
test training_pressed_dispatches_spawn ... ok
test double_press_is_inert ... ok
test training_completed_clears_inflight_and_drops_activity ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
```

Anchor gate (UI-only feature; zero delta expected):

```
$ bash scripts/verify_anchors.sh | tail -1
ANCHORS PASS  (69 / 69)
```

Raw stdout for each command saved at
`spec/cockpit-toast-queue/presentations/artifacts/cockpit-toast-queue-2026-05-27/`.

## Live demo — operator visual smoke recipe (BLOCKING for full PASS)

Per the AGENT.md human-verification recipe contract (commit `fe48fd7`),
the visual smoke for this feature is the operator's call. The recipe below
is the deferred T-D-N17 / T-T-7 — please run it before ticking approval.

### Command

```
cargo run --release -p ui --bin cockpit_live
```

(Run from the repository root.)

### Steps

1. Run the command from the repo root.
2. Wait for the binary to compile and the cockpit window to open.
3. Click the **Lab** tab in the sidebar.
4. Click **Run Training** at least 4 times in quick succession (try to
   keep clicks ≤ 100 ms apart). Each click triggers a Training-spawn
   path; if the second-and-later spawns fail because Training is already
   in-flight, that failure path surfaces a `Danger` toast via
   `Message::ShowToastWithSeverity` — which is exactly what we want to
   observe. (If your environment never produces the spawn-failure error,
   a Lab Compare cap-hit also enqueues a `Warning` toast; either trigger
   demonstrates the queue.)
5. Look at the bottom-right corner of the window.
6. Wait 5 seconds without interacting with the cockpit.
7. Trigger 1 more toast (repeat step 4 once), then click the `×` button
   on it.

### Expected timing

- First launch: ~3-5 minutes to compile in release mode (one-time cost).
- Subsequent launches: < 10 seconds.
- Each toast card lives for exactly 5 seconds from enqueue.
- The dismiss ticker fires at 500 ms cadence, so a card may linger up to
  ~5.5 s before sweeping.

### Expected result on success

- **After step 4:** a stack of up to 5 cards is visible in the bottom-
  right corner, *above* the 24 px activity tape (the tape stays anchored
  to the bottom edge and is not occluded). Each card carries a coloured
  left-edge tinge by severity — `Info` is neutral grey (`FG_2`),
  `Success` is green (`UP_500`), `Warning` is amber (`INFO_400`),
  `Danger` is red (`DOWN_500`).
- **After step 6:** cards auto-dismiss FIFO; the oldest disappears
  first; within 5 seconds of the last enqueue the stack is empty.
- **After step 7:** the targeted card disappears immediately when `×`
  is clicked; the remaining cards stay visible and continue their own
  5-second timers.

### What to do if it fails

- **No cards appear at all** → the `ToastDismissRecipe` subscription
  may not be wired into the running binary. Capture terminal stderr and
  grep for `toast_dismiss_stream`, `ActivityKind`, or `Recipe` errors.
- **Cards appear but never auto-dismiss** → the 500 ms ticker is not
  firing. Re-run with `RUST_LOG=ui=debug` and look for
  `ToastTick(Instant)` messages in the log; absence means the
  subscription is not active.
- **`×` button does nothing** → `Message::DismissToastById` arm wiring
  bug. Report the exact click behaviour (does the button visually press?
  does the cursor change?) verbatim.
- **A 6th enqueue grows the stack past 5 visible cards** → drop-oldest
  policy is broken. Report which card stayed and which was dropped.
- **Cards visually overlap the activity tape at the bottom edge** →
  Stack z-order or the 28 px bottom-padding is wrong. Describe the
  vertical layout (which surface is above which).

### Cleanup

- Close the cockpit window (Cmd+Q on macOS).

## Operator demo prompts — H2 / H3 falsifiers

These are open feedback questions framed for the demo. They are **not**
pre-ship blockers — the analyst defaults are reversible single-constant
flips:

- **H2 — Does the 5-second auto-dismiss feel right?** If a toast vanishes
  before you've registered the message, the default is wrong and we
  should revisit Q3 (operator options were 3 s / 5 s / 10 s / never).
- **H3 — Do 5 stacked toasts occlude anything critical?** Run a depth-5
  trial (steps 4 + 4 = 6 enqueues, the queue caps at 5) and check whether
  the stack ever blocks the Lab Run progress bar, the activity tape's
  "+N more" tail, or any panel chrome you actively rely on.

Both questions are operator-decide; no decision today is fine — the
shipped defaults stand until you say otherwise.

## Verification matrix

| Req  | Status   | Evidence                                                                                   |
|------|----------|--------------------------------------------------------------------------------------------|
| R1.1 | VERIFIED | `state.rs:886` field swapped to `VecDeque<ToastEntry>`; constants at `state.rs:37-47`.     |
| R1.2 | VERIFIED | `ToastEntry` + `ToastSeverity` at `state.rs:55-82`; unit test `toast_queue_enqueue_basic`. |
| R1.3 | VERIFIED | `toast_queue_overflow_drops_oldest` unit test + `overflow_drops_oldest_keeps_newest`.      |
| R1.4 | VERIFIED | `toast_queue_dismiss_by_id` unit test passes (4/4 in lib + 4/4 in integration).            |
| R1.5 | VERIFIED | `show_toast_msg_back_compat` unit test pins `Info` default; new variant compiles.          |
| R2.1 | VERIFIED | `widgets/toast_tray.rs` (187 LoC); panel_snapshots 86/86 PASS.                             |
| R2.2 | VERIFIED | Source review: only `color::FG_2 / UP_500 / INFO_400 / DOWN_500` referenced; zero new.     |
| R2.3 | VERIFIED | `shell.rs` Stack wrap; `shell_grid` 3/3 PASS; 28 px padding keeps tape uncovered.          |
| R2.4 | VERIFIED | `×` button emits `DismissToastById`; covered by `queue_displays_multiple` test path.       |
| R2.5 | DEFERRED | `auto_dismiss_after_timeout` integration test PASSES; live-runtime cadence is operator-VS. |
| R3.x | VERIFIED | `bin/cockpit_live.rs` spawn-failure migrated to `ShowToastWithSeverity(.., Danger)`.       |
| R4.1 | VERIFIED | `two_completions_in_rapid_succession_both_visible` (new stronger K5) PASSES.               |
| R4.2 | VERIFIED | Parent K5 test `k5_toast_non_clobber_run_completed_then_training_completed` PASSES.        |
| R5.1 | VERIFIED | `scripts/verify_anchors.sh` → `ANCHORS PASS (69 / 69)`.                                    |
| R5.5 | DEFERRED | `cockpit_smoke.sh` does not exist in repo; operator-run live smoke is the equivalent.      |
| R5.6 | VERIFIED | Workspace tests 818+ PASS; 2 failures pre-existing whitelisted (non-attributable).         |
| R5.7 | VERIFIED | spec-lint adds no new violation categories vs 2026-05-25 baseline.                         |
| R5.8 | VERIFIED | 69/69 anchors PASS (UI-only feature; zero delta).                                          |
| R5.9 | VERIFIED | `toast_message()` method shim at `state.rs:1258`; K5 test compiles unchanged.              |

## Numbers that matter

- **Tests added:** 4 inline unit + 4 integration = **8 new tests**, all PASS.
- **Tests still green:** **397 lib + 86 panel snapshots + 5 K5 regression +
  3 shell-grid + 4 toast-queue integration = 495 PASS** in the touched
  surface; 818+ across the workspace.
- **Anchors:** **69 / 69 PASS**, zero delta (UI-only feature, as expected
  per R5.1).
- **New Lumen tokens introduced:** **0** (severity colours reuse existing
  Phase 1 palette).
- **Lines added (developer accounting):** ~80 in `state.rs` + ~187 in
  `toast_tray.rs` + ~60 in `cockpit_live.rs` + ~120 in
  `tests/cockpit_toast_queue.rs` + supporting wiring ≈ **~500 LoC** plus
  tests.
- **Queue capacity:** **5** (`MAX_TOAST_QUEUE_LEN`). Auto-dismiss
  timeout: **5 s** (`TOAST_AUTODISMISS`). Card width: **320 px**.
- **Ticker cadence:** **500 ms** (so worst-case toast lifespan is
  ~5.5 s before sweep).
- **Pre-existing workspace failures:** **2**, both whitelisted and
  documented as non-attributable in
  `spec/v5-latency-slippage-sim-v0.2.0-anchor-migration/reports/`.
- **spec-lint delta:** +8 dead-link (other features), +2 missing-frontmatter
  (auto-resolved when status flipped to `shipped`), +1 shipped-no-tests
  (unrelated). **No new categories.**

## Open decisions

1. **Visual smoke pass / fail** — run the recipe above; report success
   per the "Expected result" block or per the failure-handling steps.
   This is the only blocking item between SOFT-PASS and full PASS.
2. **H2 — keep 5 s auto-dismiss?** Flag if it feels too short or too
   long after the demo; no decision needed today.
3. **H3 — depth-5 stack acceptable?** Flag if 5 cards ever block a
   surface you rely on; no decision needed today.
4. **v0.2.0 cleanup — when?** The `toast_message` field carries a
   `// MIGRATION: remove at v0.2.0` annotation. Spawn the cleanup brief
   now, queue it, or defer until another producer wants to write the
   field directly and forces our hand?

## What's next

- **v0.2.0 cleanup brief** — hard-remove the legacy `toast_message`
  field; migrate `cockpit_training_pressed_wiring.rs` to write via
  `Message::ShowToastWithSeverity` instead of the direct field write.
  Estimated cost: < 50 LoC, < 0.5 day. Spawn on operator request.
- **`cockpit-toast-queue v0.2.0` brief (conditional)** — only authored
  if the operator demo surfaces a desire for per-toast action buttons
  ("Retry", "View report") or persistent toasts (manual-dismiss only).
  Both are explicitly out-of-scope at v0.1.0 per
  `feature.md § Out of scope`.
- **`cockpit-toast-a11y` follow-on (deferred per K4)** — ARIA / screen-
  reader live-region announcements for the toast cards. Already noted
  in `feature.md`; no operator request today.
- **Producer audit** — once `cockpit-activity-audit-ledger-producer
  v0.1.0`, `cockpit-activity-llm-producer v0.1.0`, and the v5 latency-
  slippage divergence alert land, audit each for toast-spam risk per K3.
  The toast queue does NOT subscribe to the audit broadcast (by design);
  producers must respect the producer-side aggregation envelope.

## Approval

Operator: tick exactly one box below.

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes

_(operator fills if needed)_

## Verdict block

Mechanical gates run by the presenter at 2026-05-27 18:45 local against
this presentation file:

- `bash scripts/check_presentation.sh ...` → `PRESENTATION CHECK PASS  (/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/spec/cockpit-toast-queue/presentations/cockpit-toast-queue-2026-05-27.md — approval block UN-ticked)`
- `python3.14 scripts/spec_lint.py` → `spec-lint: FAIL (71 violations in 3 categories)` — tester PASS baseline was 73 in same 3 categories (dead-link / missing-frontmatter / shipped-no-tests); current count is LOWER (the +2 missing-frontmatter from this feature's `dev-complete` status resolved when status flipped to `shipped`). **No new categories vs tester PASS baseline.** R5.7 holds.

## Changelog

- 2026-05-27 (presenter): authored release deck for v0.1.0; embedded
  live test stdout + anchor gate + 6-section operator visual-smoke
  recipe per AGENT.md human-verification contract. Approval block
  shipped UN-ticked.
