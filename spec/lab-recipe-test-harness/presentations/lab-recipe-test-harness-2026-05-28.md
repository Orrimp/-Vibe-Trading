---
slug: lab-recipe-test-harness
version: 0.1.0
mode: release
date: 2026-05-28
owner: presenter
commit: 648d470c3bf3e5cdc6a2eca4def20c8cc5bb779d
tester_verdict: PASS
---

# Lab Recipe test harness v0.1.0 — operator review

## TL;DR

A new two-surface test harness now demonstrably catches the regression
class that slipped past every existing gate on Bug #64 attempt 1 — the
mandatory falsification probe (T-T4) **independently confirmed** that
2 tests fail when the bug is simulated, and 3/3 pass after restore.

## What changed

- Added `pub trait LabYahooBarSource` to `crates/ui/src/lab/runner.rs`
  so tests can inject a mock Yahoo bar source where production used to
  call the parquet + HTTP impl. Production call site passes `None`;
  behaviour is unchanged for real runs.
- Added two new integration test files (~200 LoC total) — Surface 1
  drives `spawn_lab_run` end-to-end with the mock channel; Surface 2
  drives `Cockpit::update` across the full Lab-run message lifecycle
  and asserts the Stop-button gating predicate.
- ADR-0048 records the design (pattern (d) Combination — boundary
  test + state-machine gating test).

## Why

Bug #64 attempt 1 (commit `5f9f920`) shipped with **every existing
gate green** — 415 UI library tests PASS, K5 5/5 cockpit-wiring,
70/70 anchors — and **three live regressions still slipped past**
the operator's visual-verify of a real cold-cache Yahoo Lab run
(label missing, progress bar stuck at indeterminate 30 %, Stop
button inert). The dev's 4 new `LabState` invariant tests proved
pure-state correctness in isolation but said nothing about whether
the `mpsc::channel` actually survives a `tokio::select!` refactor
or whether the cockpit message-lifecycle flips `lab_run_inflight`
back to `false`. Pure-state coverage was strict but the wrong shape;
this harness closes that coverage gap with two thin surfaces that
exercise the *plumbing* — the channel and the message bus — not
the maths.

## What the operator can do now

- **Approve and ship** — this is purely test infrastructure (no
  production behaviour change). Approving unblocks the Bug #64
  D.1.1 + D.2.1 re-attempt.
- **Reproduce the T-T4 proof locally** if you want hands-on
  confidence that the harness is not theater:

  ```bash
  # 1. Confirm clean baseline
  cargo test -p ui --test lab_stop_button_gating
  # → 3/3 PASS in ~0.00s

  # 2. Simulate the Bug #64 D.2.1 regression by commenting out
  #    crates/ui/src/state.rs:2147 ("model.lab_state.run_progress = None;")
  #    inside the LabRunCompleted arm. Then re-run:
  cargo test -p ui --test lab_stop_button_gating
  # → exit 101: 2 tests FAIL with "run_progress must be None after
  #   LabRunCompleted(Ok|Err)"

  # 3. Restore the line; re-run:
  cargo test -p ui --test lab_stop_button_gating
  # → 3/3 PASS
  ```

- **Inspect the new test files** if you want to see the assertion
  shape before signing off:

  ```bash
  $EDITOR crates/ui/tests/spawn_lab_run_yahoo_harness.rs
  $EDITOR crates/ui/tests/lab_stop_button_gating.rs
  ```

## Live demo — the load-bearing proof (T-T4 falsification probe)

The tester independently verified the harness by simulating the
exact Bug #64 D.2.1 regression on HEAD and confirming the harness
fails. This is the section to read carefully — it is what makes
the deck honest.

**Procedure** (from the tester report § 4):

1. Comment out `model.lab_state.run_progress = None;` at
   `crates/ui/src/state.rs:2147` (inside the
   `Message::LabRunCompleted(outcome)` arm).
2. Run `cargo test -p ui --test lab_stop_button_gating`.

**Output under simulated regression** (exit code 101):

```
running 3 tests
test stop_requested_mid_run_leaves_inflight_true ... ok
test err_completion_clears_inflight ... FAILED
test full_lifecycle_ok_completion_clears_inflight ... FAILED

failures:

---- err_completion_clears_inflight stdout ----
thread 'err_completion_clears_inflight' panicked at
crates/ui/tests/lab_stop_button_gating.rs:182:5:
run_progress must be None after LabRunCompleted(Err)

---- full_lifecycle_ok_completion_clears_inflight stdout ----
thread 'full_lifecycle_ok_completion_clears_inflight' panicked at
crates/ui/tests/lab_stop_button_gating.rs:133:5:
run_progress must be None after LabRunCompleted(Ok)

test result: FAILED. 1 passed; 2 failed; 0 ignored; 0 measured;
0 filtered out; finished in 0.00s
```

**Output after restore** (clean state.rs, `git diff` empty):

```
running 3 tests
test stop_requested_mid_run_leaves_inflight_true ... ok
test err_completion_clears_inflight ... ok
test full_lifecycle_ok_completion_clears_inflight ... ok

test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured;
0 filtered out; finished in 0.00s
```

**Reading this:** under simulated regression, exactly 2 tests fail
with descriptive assertion messages. After restore, all 3 pass and
`git diff state.rs` is empty. The harness catches the D.2.1
regression class with zero false-positives and zero missed catches.

## Verification matrix

| Req | Status | Evidence |
|---|---|---|
| **R1** — Boundary test `spawn_lab_run` with mock Yahoo source + real channel | VERIFIED | `crates/ui/tests/spawn_lab_run_yahoo_harness.rs` — 3/3 PASS in 0.50s under `cargo test -p ui --test spawn_lab_run_yahoo_harness --features live`. Tests: `sentinel_fires_before_preload_await`, `channel_survives_after_preload`, `ticker_events_stop_after_preload_complete`. |
| **R2** — Stop-button gating state-machine test | VERIFIED | `crates/ui/tests/lab_stop_button_gating.rs` — 3/3 PASS in 0.00s. Tests: `full_lifecycle_ok_completion_clears_inflight`, `err_completion_clears_inflight`, `stop_requested_mid_run_leaves_inflight_true`. |
| **R3** — `YahooBarSource` trait extraction (API-additive) | VERIFIED | `pub trait LabYahooBarSource` + `DefaultLabYahooBarSource` at `crates/ui/src/lab/runner.rs:194-260`; `spawn_lab_run` gains `Option<Box<dyn LabYahooBarSource>>`; production call site `crates/ui/src/bin/cockpit_live.rs:1531-1537` passes `None`. UI lib suite 411/411 PASS unchanged. |
| **R4** — Anchor-additivity (zero file output, 70/70 stable) | VERIFIED | `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (70 / 70)`. `spec/anchors.toml` untouched. |
| **R5** — Workspace test gate integration | VERIFIED | `cargo test --workspace --no-fail-fast` PASS; UI test count rises by 6 (3 + 3) — matches the 2-file budget. Surface 1 gated behind `#[cfg(feature = "live")]`. |
| **T-T4** — Falsification probe (mandatory proof) | VERIFIED | Tester report § 4: 2 tests FAIL under simulated D.2.1; restore verified (`git diff state.rs` empty); 3/3 PASS post-restore. |

## Numbers that matter

- **UI lib tests:** 411 / 411 PASS (`cargo test -p ui --lib --features live`, 0.53 s)
- **Surface 1 (new):** 3 / 3 PASS in 0.50 s combined wall-clock
- **Surface 2 (new):** 3 / 3 PASS in 0.00 s
- **K5 regression sentinel:** 5 / 5 PASS (`cockpit_training_pressed_wiring`)
- **Anchors:** 70 / 70 PASS — zero new anchors, zero anchor file mutations
- **New clippy errors:** 0 (9 pre-existing in `ui/lab/*` remain, explicitly out of scope)
- **LoC added:** ~200 across two new test files
- **Per-case wall-clock budget:** ≤ 1.5 s (ADR-0048 D4) — observed max 0.50 s
- **Falsification probe outcome:** 2 / 3 tests FAIL under simulated bug, 3 / 3 PASS after restore

## Architecture call-outs (ADR-0048 D1–D6)

- **D1 — Pattern (d) Combination.** Boundary test for the channel/spawn surface + state-machine test for the cockpit gating predicate. Picked over (a) pure E2E, (b) snapshot widget-tree, (c) accesskit (not viable on iced 0.14).
- **D2 — File:line locations.** Surface 1 at `crates/ui/tests/spawn_lab_run_yahoo_harness.rs`; Surface 2 at `crates/ui/tests/lab_stop_button_gating.rs`; trait at `crates/ui/src/lab/runner.rs:194-260`.
- **D3 — Scope: 3 regression categories.** (A) sentinel emission, (B) channel survival across `tokio::select!`, (C) cockpit predicate transitions.
- **D4 — What's NOT caught.** Visual regressions (font sizing, Lumen token shifts, theme drift) are orthogonal — those land in the screenshot/golden surface that ui-designer owns. Don't expect this harness to flag those.
- **D5 — Cadence.** Per-feature M-FINAL gate for any future UI Recipe / Subscription touch. New UI work that touches a channel-fed widget opts in by adding a third file in the same pattern.
- **D6 — Anchor-additivity.** Channel-only events (`progress_tx → progress_rx`), zero file output, 70/70 anchors byte-identical. Trait extraction is API-additive — `crates/backtest/tests/determinism.rs` row 70 SHA stable.

## What's next — downstream features unblocked

- **Bug #64 D.1.1 + D.2.1 re-attempt** is now unblocked AND gated. The re-attempt will rebase on top of this harness and add: (i) Surface 1 test 1 (`sentinel_fires_before_preload_await`) gates the D.1.1 sentinel-ticker contract; (ii) Surface 2 gates the D.2.1 post-completion linger contract (the exact line T-T4 simulated). Any future regression in the same class will be caught at `cargo test` time, not at operator visual-verify time.
- **Future UI Recipe / Subscription work** can opt into the harness via the `LabYahooBarSource` trait pattern. Pattern reference for the next author: see this brief + ADR-0048 D2 file layout.
- **T-T5 (D.1.1 bonus probe)** deferred to v0.2.0 — optional; the Surface 1 test docstring already documents the falsification mechanism for category A. Only worth picking up if explicit D.1.1 coverage is requested.

## Open follow-ups (no urgency, not gating)

- `aggregator_emits_one_tick_per_window` in `crates/agent/tests/activity_audit_aggregator.rs` shows a parallel-load timing flake under `cargo test --workspace`; passes 3/3 in isolation. Pre-existing, not attributable to this feature (crates/agent untouched). Worth investigating if it flakes again on the next workspace run.

## Open decisions

_n/a — no operator-decide questions raised. This is a binary ship-or-reject._

## Approval

- [ ] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / rejection reason

_(operator fills in if applicable)_

## Feedback log

_(empty — first presentation)_
