---
adr: 0048
title: Lab Recipe/Subscription test harness — boundary-test spawn_lab_run + Stop-gating state-machine pair
status: accepted
date: 2026-05-28
supersedes: none
superseded-by: none
---

# ADR-0048: Lab Recipe/Subscription test harness — boundary-test `spawn_lab_run` + Stop-gating state-machine pair

## Context

Bug #64 attempt 1 (D.1.1 sentinel-ticker + D.2.1 post-completion linger,
commit `5f9f920`) shipped with 415 ui lib tests PASS, 70/70 anchors, K5
5/5 — every existing gate green. Operator visual-verify against a real
cold-cache Yahoo run surfaced **three live regressions** (revert at
commit `05937e4`):

1. **No label visible** — the pre-existing `"0 / N bars · Xs"` label
   stopped rendering during preload; suggests the new `tokio::select!`
   refactor dropped the sentinel emit OR consumed the channel.
2. **Progress bar stuck at ~30 % indeterminate** — the iced fallback
   that Bug #64's original sentinel-emit at `runner.rs:617-621` was
   specifically designed to eliminate reappeared. Implies the
   sentinel never reached `progress_tx.send(...)`.
3. **Stop button does nothing after Run** — Stop's view-gating predicate
   `model.lab_run_inflight` (`screens/lab.rs:419`) stayed false because
   D.2.1's linger logic broke the `LabRunRequested → inflight = true`
   transition OR `LabRunCompleted` failed to flip it back.

The dev's 4 new `LabState` invariant tests proved pure-state correctness
locally but did NOT catch any of the three. Three coverage gaps:

- **No test drives `spawn_lab_run(...)` end-to-end** with mocked Yahoo
  source + real `mpsc::channel` to assert sentinel emission *before*
  `preload_yahoo_bars().await`. `cockpit_live_lab_run_smoke.rs` calls
  `run_scenario` directly, skipping the spawn-task body entirely.
- **No test exercises `tokio::select!` channel-survival.** The D.1.1
  ticker added a `select!` between `preload_future` and `interval.tick()`;
  if the ticker arm consumes `progress_tx` and shadows it (closure-move
  bug), the preload-success arm runs but its `progress_tx.try_send(...)`
  never fires.
- **No test asserts Stop button enablement.** `screens/lab.rs:419`
  reads `model.lab_run_inflight`; there's no test that drives
  `LabRunRequested → LabRunCompleted` lifecycle and asserts the
  predicate transitions correctly.

`accesskit` / `kittest` were researched earlier this session and
determined not viable on iced 0.14 — out of scope here.

## Decision

**Pattern (d) Combination: boundary-test for spawn_lab_run + Stop-gating
state-machine pair.** Two tightly-scoped test surfaces, one new file
each, exercising the three failure-mode categories with minimal scaffolding.

### Surface 1 — `crates/ui/tests/spawn_lab_run_yahoo_harness.rs` (NEW)

Drive `spawn_lab_run(...)` directly inside `tokio::test` with a mocked
Yahoo bar source + real `mpsc::channel`, assert on the receiver stream.

**Mock injection point**: extract `pub trait YahooBarSource` over
`runner::preload_yahoo_bars`; production keeps the existing parquet+http
impl. `MockYahooBarSource` knobs: `sleep_duration` (default 500 ms)
+ `bars` (default 30 deterministic Yahoo bars).

**Test assertions**:
- **(A) Sentinel emission**: first event on `progress_rx` is
  `Progress { 0, 1, 0 }`, arriving within 50 ms wall-clock (i.e. BEFORE
  the mocked `sleep(500ms)`).
- **(B) Channel survival across `tokio::select!`**: receiver yields ≥ 2
  events with `total_bars == 1` and strictly-increasing `elapsed_ms`
  during the 500 ms preload window (ticker arm fires AND channel
  survives); receiver yields events with `total_bars > 1` post-preload
  (channel not consumed by ticker arm); final yield is
  `LabRunCompleted(Ok(_))` or channel-close → `LabRunProgressDone`.

### Surface 2 — `crates/ui/tests/lab_stop_button_gating.rs` (NEW)

Assert the Stop button view-gating predicate transitions correctly
across the Lab-run message lifecycle (catches Bug #64 regression #3).

**Pattern**: K5 shape (`cockpit_training_pressed_wiring.rs`): construct
`Cockpit::default()`, dispatch `update(...)`, assert on
`model.lab_run_inflight` (the predicate `screens/lab.rs:419` reads).

**Test assertions** (category C):
- Default → `lab_run_inflight == false`.
- After `LabRunRequested` → `true`.
- After 5 × `LabRunProgress(p)` → stays `true` (linger / partial-state
  changes must not flip it).
- After `LabRunCompleted(Ok|Err)` → `false`.
- After `LabRunStopRequested` mid-run → pure state unchanged per
  `state.rs:2179`; binary-side completion clears the flag.

Optional second test asserts the rendered Stop element is present iff
`lab_run_inflight == true` via `tests/fixtures/mod.rs` view introspection.
If fragile, fall back to predicate-only assertion.

### Why pattern (d)

- **(a) boundary-only** — catches A + B, misses C. Bug #64 regression #3
  would not have been caught.
- **(b) state+channel pair-test** — catches all three but requires iced
  `Subscription` runtime scaffolding inside `tokio::test` (~150 LoC).
- **(c) cockpit-smoke extension** — orchestrator-only per AGENT.md
  capability boundaries; gates post-PASS; Stop-gating hard to extract
  from stderr cleanly.
- **(d) combination** — two surfaces, one new test file each, ~200 LoC
  total + one trait. Surface 1 in tokio-runtime layer (no iced); Surface 2
  reuses K5 pattern.

## Alternatives considered

- **Extend `cockpit_live_lab_run_smoke.rs`** instead of new files —
  rejected because that file already runs `run_scenario` directly to
  cover the post-preload engine path; mixing in `spawn_lab_run`
  boundary tests dilutes its purpose. Two file-level intents stays
  cleaner per `crates/ui/tests/` conventions.
- **Mock `tokio::time::Interval`** to control ticker cadence
  deterministically — rejected at v0.1.0 because wall-clock sleep on
  the mock source is sufficient to prove ≥ 2 ticker emits at 250 ms
  cadence in a 500 ms window. Defer interval virtualization to v0.2.0
  if flake rate > 1 % across 100 CI runs.
- **`accesskit` / `kittest`** — already ruled out (not viable on iced 0.14).
- **Property-based test (`proptest`) on `Progress` sequence ordering** —
  rejected as out-of-scope at v0.1.0; the deterministic 5-event
  assertion is sufficient to catch the regression classes documented.

## Consequences

### D1 — Harness pattern picked

Pattern **(d) Combination**: boundary-test (`spawn_lab_run` + mock Yahoo
source) AND Stop-gating state-machine test.

### D2 — Harness file:line locations

- `crates/ui/tests/spawn_lab_run_yahoo_harness.rs` (NEW, ~120 LoC,
  feature-gated `#[cfg(feature = "live")]`).
- `crates/ui/tests/lab_stop_button_gating.rs` (NEW, ~80 LoC).
- `crates/ui/src/lab/runner.rs` — introduce `pub trait YahooBarSource`
  + extract `preload_yahoo_bars` to take `&dyn YahooBarSource`.
  Production wiring unchanged at the call sites (default impl).

### D3 — Regression categories caught

- **A) Sentinel emission**: Surface 1 asserts the sentinel
  `Progress { current_bar: 0, total_bars: 1, elapsed_ms: 0 }` arrives
  on `progress_rx` BEFORE the mock preload resolves.
- **B) Channel survival across `tokio::select!`**: Surface 1 asserts
  ≥ 2 ticker emits during the 500 ms mock preload window AND engine
  events arrive post-preload — proves channel not consumed/closed by
  the `select!` ticker arm.
- **C) Predicate-gated UI elements**: Surface 2 asserts
  `model.lab_run_inflight` transitions across the full Lab-run message
  lifecycle, including the linger / partial-state cases that broke
  Bug #64 attempt 1 regression #3.

### D4 — What this harness will NOT catch

- **Visual UX artifacts** (label tick smoothness, color drift, layout
  shift) — route to `cockpit-smoke` + M1-B snapshot suite. Harness
  asserts behaviour, not appearance.
- **Multi-frame iced repaint coalescing** — Surface 1 reads
  `progress_rx` synchronously (hypothesis D.2 in the dev-note).
- **Real Yahoo network failure modes** — mock bypasses HTTP/parquet;
  Bug #63 60 s timeout covered by existing `runner.rs` unit tests.
- **iced `Subscription` reconstruction bugs** (salt-bump regressions)
  — already covered by `lab_progress_recipe_stream.rs`'s
  `lab_progress_recipe_stream_end_to_end`.
- **Cross-sectional scenario channel-survival** — v0.1.0 covers SMA
  single-symbol; cross-sectional deferred to v0.2.0.

### D5 — Invocation cadence

- **Per-feature M-FINAL gate** for features touching
  `crates/ui/src/lab/runner.rs` OR `state.rs` Lab arms (`LabRunRequested`,
  `LabRunProgress`, `LabRunCompleted`, `LabRunStopRequested`,
  `LabRunProgressDone`). Tester runs both new test files.
- **Workspace test gate** (`cargo test --workspace`) — both tests join
  the default suite. Surface 1 is feature-gated so non-`live` builds
  skip cleanly.
- **No nightly cron / orchestrator pre-tick** — harness is fast
  (≤ 5 s combined, deterministic); cockpit-smoke stays separate.

### D6 — Anchor-additivity contract

Harness emits ZERO file output. `progress_tx → progress_rx` events
are channel-only (same property Bug #64's original ship relied on per
`bug-log.md#64`). `spec/anchors.toml` not touched.
`scripts/verify_anchors.sh` stays **70/70 PASS** post-merge.
`YahooBarSource` extraction is API-additive — anchor preservation
proven by the existing `crates/backtest/tests/determinism.rs` row 70
SHA assertion.

## Changelog

- 2026-05-28 (architect): ADR authored; pattern (d) selected; D1-D6 locked.
- 2026-05-29 (architect, lab-recipe-test-harness-v0.2.0-cross-surface-extension
  M-T1 close): D1-D6 **carry forward verbatim** to v0.2.0 — pattern (d)
  is the contract for all 9 R1 Recipe surfaces, not just v0.1.0's
  `spawn_lab_run` + `lab_run_inflight` pair. v0.2.0 extends the harness
  PREEMPTIVELY to 7 additional surfaces (`TrainingLogRecipe` S1+S2,
  `ActivityAuditAggregator` S1, `ServerTimeRecipe` S2, `ToastDismissRecipe`
  S1+S2, `TrailMirrorRecipe` S2, `ActivityRecipe` S1, `TrainingPoller`
  S1+S2) via the same per-Recipe-specific mock pattern v0.1.0's
  `MockLabYahooBarSource` proved (R3 = per-Recipe, NOT a shared trait —
  see [`spec/lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md`](../../lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md)
  § D-V0.2.0-1). Three new API-additive production seams accompany the
  extension: `run_aggregator_loop` extraction from `Aggregator::run`
  (Wave B), `build_subscription_batch_descriptor` extraction from
  `cockpit_live.rs::subscription()` (Wave C), `training_poller_stream_impl`
  extraction from `TrainingPoller::stream` (Wave D). All three are
  shape-identical to v0.1.0's `LabYahooBarSource` trait extraction —
  covered by the existing D2 "API-additive" clause; D6 anchor-additivity
  (71/71 byte-identical) re-verified post-v0.2.0 merge. No new ADR; no
  D1-D6 row revised. Q2 = per-Recipe T-T4 falsification probe in each
  test file docstring (mandatory; mirrors v0.1.0 `spawn_lab_run_yahoo_harness.rs`
  shape) — see § D-V0.2.0-3 for the 11-row probe-line table.
- 2026-05-29 (architect, visual-fail-html-reporter v0.1.0 M-T1 close):
  forensic-artifact emission pattern from D6 anchor-additivity extended
  to include `target/visual-diff/<test>-<ts>.html` alongside the existing
  `<test>.png` + `<test>-actual.png` triple on visual-assertion FAIL
  only. PASS path byte-identical; 71/71 anchors unaffected (helper
  produces zero output on PASS). No D1-D6 row revised. See
  [`spec/visual-fail-html-reporter/feature.md`](../../visual-fail-html-reporter/feature.md)
  § Design D-VF-1..D-VF-6. Wave 1 sibling
  `ui-test-harness-viewport-matrix` inherits the
  `.claude/agents/tester.md` "Visual failures — HTML artifact emission"
  stanza (D-VF-4) without further amendment per trifecta-direction
  § Risk R1 mitigation.
