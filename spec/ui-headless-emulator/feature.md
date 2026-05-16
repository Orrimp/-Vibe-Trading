---
slug: ui-headless-emulator
version: 0.1.0
status: shipped
owner: shipped
predecessor: ui-session-journal-iced-tester v0.1.0
updated: 2026-05-16
---

> **Status (2026-05-16):** v0.1 shipped in commit (TBD). All
> V1-V6 green. One impl-time correction: feature.md prescribed
> `iced::core::Size`; actual API is `iced::Size` (the `core`
> sub-crate is private). One-token fix.

# Headless emulator adapter (v0.1)

> Decomposed out of `ui-test-harness-ci` (~4d) per operator decision
> 2026-05-16: ship the headless-emulator portion standalone (~1d) so
> the unchecked "headless mode" cell from
> [`iced-014-feature-analysis-2026-05-15.md §4`](../dev-notes/iced-014-feature-analysis-2026-05-15.md#headless-mode)
> closes without waiting on `ui-test-harness-viewport-matrix` +
> `ui-test-harness-evaluator`. The CI workflow piece + cross-platform
> falsifier remain queued under `ui-test-harness-ci`.

## Why

Our existing integration tests
([cockpit_live_kill_button_writes_audit.rs](../../crates/ui/tests/cockpit_live_kill_button_writes_audit.rs),
[risk_telemetry_subscription.rs](../../crates/ui/tests/risk_telemetry_subscription.rs),
[live_subscription.rs](../../crates/ui/tests/live_subscription.rs))
drive `ui::state::update(&mut cockpit, msg)` directly — bypassing
iced's runtime entirely. That works for "given message → expected state
mutation" assertions but cannot exercise:

- **Subscription pump:** iced's subscription tree is built from
  `AppState::subscription(&self) -> Subscription<Message>`. The
  current direct-update tests never invoke it. A subscription that
  panics or fails to emit on real wall-clock progression is invisible.
- **Task graph:** `iced::Task::*` returns from `update()` are dropped
  on the floor by direct-update tests. Real cockpit message dispatch
  may chain through tasks; the deep-dive's §1.3 named "loading
  spinner stuck because subscription never completes" as a
  representative failure class direct-update misses.
- **Window-event integration:** keyboard / mouse / focus events
  arrive via iced's event loop. Without a runtime, none of these can
  be exercised.

iced 0.14 ships [`iced_test::emulator::Emulator`][emulator-docs] (PR
#2698) which runs the FULL Program tree (boot + update + subscriptions
+ tasks + view) without a window server. v0.1 of this feature ships a
smoke test proving the Emulator boots and ticks our cockpit; future
features (or a refactor inside `ui-test-harness-ci`) port existing
direct-update tests to Emulator-based ones.

[emulator-docs]: https://docs.rs/iced_test/0.14.0/iced_test/emulator/struct.Emulator.html

## Scope locked

- **D-HE-1** — v0.1 ships ONE smoke test
  (`crates/ui/tests/headless_emulator_smoke.rs`). It boots the cockpit
  via `program_from_cockpit(...)` (the same factory `visual_snapshots`
  uses), waits for `Event::Ready`, takes a screenshot, asserts
  dimensions match the floor viewport (1280×720). NO subscription
  refactoring of existing tests in this cycle.
- **D-HE-2** — `Mode::Zen` (the default — wait for ALL tasks). Slowest
  but most deterministic; matches the project's H1 byte-determinism
  posture.
- **D-HE-3** — No new cargo features. `iced_test = "=0.14.0"` is
  already a dev-dep (since the bootstrap); the Emulator surface is
  available immediately.
- **D-HE-4** — Test runs at the bootstrap's `floor` viewport
  (1280×720 @ 1.0×) only. Adding `typical` + `operator` slots is
  bundled into `ui-test-harness-ci` (which adds the viewport matrix
  to existing snapshot work).

### In scope (v0.1)

- One new test file: `crates/ui/tests/headless_emulator_smoke.rs`.
- Brief usage docstring inside the test pointing at this spec.
- This `feature.md` + `tasks.md`.
- Backlog entry transition queued → shipped on commit.

### Out of scope

- Refactoring existing direct-update tests (`cockpit_live_kill_button_writes_audit.rs`
  et al.) to use Emulator. Separate cycle if/when value justifies.
- CI workflow integration. Stays in `ui-test-harness-ci`.
- Cross-platform falsifier (revisits Q-D3-RELITIGATE). Stays in
  `ui-test-harness-ci`.
- Replacing `iced_test::screenshot(...)` calls in
  [`visual_snapshots.rs`](../../crates/ui/tests/visual_snapshots.rs)
  with Emulator-based screenshots — the free function works fine for
  pure-render snapshots; Emulator's value is for subscription-bound tests.
- Operator-decision overrides (Q-014-PIN, Q-COMET-EVAL stay locked).

## Design

### Test shape

[`crates/ui/tests/headless_emulator_smoke.rs`](../../crates/ui/tests/headless_emulator_smoke.rs)
(NEW) modeled on the bootstrap's `visual_snapshots.rs`:

```rust
#[test]
fn headless_emulator_boots_cockpit_and_renders() {
    use iced_test::emulator::{Emulator, Event, Mode};
    use iced_test::futures::futures::channel::mpsc;
    use iced_test::futures::futures::executor;
    use iced_test::futures::futures::stream::StreamExt;

    let cockpit = ui::test_support::charts_screen_cockpit();
    let program = ui::test_support::program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;

    let (tx, mut rx) = mpsc::channel(64);
    let mut emulator = Emulator::new(
        tx,
        &program,
        Mode::Zen,
        iced::core::Size::new(1280.0, 720.0),
    );

    // Drain events until Ready or a deadline (10 events / ~50 ms).
    executor::block_on(async {
        for _ in 0..10 {
            match rx.next().await {
                Some(Event::Ready) => break,
                Some(Event::Action(action)) => emulator.perform(&program, action),
                Some(Event::Failed(_)) | None => break,
            }
        }
    });

    let screenshot = emulator.screenshot(&program, &theme, 1.0);
    assert_eq!(screenshot.size.width, 1280);
    assert_eq!(screenshot.size.height, 720);
    assert!(!screenshot.rgba.is_empty(), "screenshot rgba must be non-empty");
}
```

The exact API call shapes will be verified by compile + iterate at
implementation time. The shape above is the spike-confirmed surface
of [`iced_test::emulator`](https://docs.rs/iced_test/0.14.0/iced_test/emulator/index.html).

### Why this is meaningfully different from the bootstrap's `screenshot()`

| | Bootstrap `iced_test::screenshot(...)` | New Emulator-based |
|---|---|---|
| Builds Program | yes | yes |
| Calls `boot()` | yes | yes |
| Calls `view()` | yes | yes |
| Pumps subscriptions | NO | YES |
| Pumps tasks | NO | YES |
| Receives events | NO | YES |
| Loop time control | `Duration::ZERO` (single-frame) | event-driven, can tick N frames |
| Mode | n/a | Zen / Patient / Immediate |

`iced_test::screenshot` is the right tool for "given seeded state,
render once". `Emulator` is the right tool for "boot the whole tree
and let it settle, then assert".

## Acceptance / verification (V-items)

| # | What | How |
|---|---|---|
| V1 | `cargo test -p ui --test headless_emulator_smoke` exits 0 | Compile + test gate |
| V2 | The test takes a 1280×720 screenshot via the Emulator (proves boot + view loop runs) | Inside the test (assert on screenshot.size) |
| V3 | `cargo test --workspace` stays green (no regression on the 1223-baseline) | Workspace test gate |
| V4 | `cargo build -p ui --features live --bin cockpit_live` (production build) succeeds and is unaffected | Compile gate |
| V5 | `cargo clippy -p ui --no-deps` adds zero new warnings | Lint gate |
| V6 | `cargo fmt --check` clean | Fmt gate |

## Dependencies

- `iced_test = "=0.14.0"` — already in dev-dependencies (bootstrap).
- Re-uses `ui::test_support::program_from_cockpit` and
  `charts_screen_cockpit`. No new deps, no new crates.

## Risks

| # | Risk | Mitigation |
|---|---|---|
| R-HE-1 | Emulator's executor hangs waiting for a subscription that never produces | Bounded event loop (`for _ in 0..10`) caps total ticks; falls through to screenshot on deadline |
| R-HE-2 | `Mode::Zen` produces non-deterministic ordering across runs | If observed: switch to `Mode::Patient` and re-baseline. Logged as a TODO in test if it surfaces |
| R-HE-3 | The `iced_test` re-exported `futures::futures::channel::mpsc` path is wrong | Compile-iterate; the spike confirmed iced_test re-exports `futures` so the path works |

## Out-files

**New:**
- `crates/ui/tests/headless_emulator_smoke.rs`
- `spec/ui-headless-emulator/feature.md` (this file)
- `spec/ui-headless-emulator/tasks.md`

**Modified:**
- `spec/backlog.md` — promote candidate from queued to shipped on V6 PASS

**Unchanged:**
- `crates/ui/Cargo.toml` (no new deps)
- All production code (test-only feature)

## Changelog

- 2026-05-16 (orchestrator): feature spec authored after operator
  picked "headless first, comet revisit later" to close the unchecked
  "headless mode" cell from `iced-014-feature-analysis-2026-05-15.md
  §4`. Decomposed out of `ui-test-harness-ci` to ship without waiting
  on the viewport-matrix + evaluator prereqs. Comet revisit deferred
  until iced 0.15.0 stable releases (currently `0.15.0-dev` master
  only); tracked in [`backlog.md ## Recent (shipped) ## Comet revisit
  trigger`](../backlog.md).
