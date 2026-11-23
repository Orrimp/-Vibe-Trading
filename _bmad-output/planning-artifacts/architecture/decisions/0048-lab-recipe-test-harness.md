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
  *(Corrected 2026-11-23, story 2-63.* This bullet was false when
  written: that file hashed no recipe then and does not now. The ids of
  the recipes `ui::live::subscription_batch` builds are pinned since Wave
  C2 (`subscription_batch_inclusion.rs`), and a swapped salt in the live
  bin's `SubscriptionCtx` literal is still checked by nothing. See
  § Changelog, the v0.3.0 row.)
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
`bug-log.md#64`). `evidence/anchors.toml` not touched.
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
  see [`docs/archive/pre-bmad-spec/v1/lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md)
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
  [`docs/archive/pre-bmad-spec/v1/visual-fail-html-reporter/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/visual-fail-html-reporter/feature.md)
  § Design D-VF-1..D-VF-6. Wave 1 sibling
  `ui-test-harness-viewport-matrix` inherits the
  `.claude/agents/tester.md` "Visual failures — HTML artifact emission"
  stanza (D-VF-4) without further amendment per trifecta-direction
  § Risk R1 mitigation.
- 2026-05-29 (architect, ui-contrast-asserter v0.1.0 M-T1 close):
  D1-D6 carry forward verbatim. The contrast asserter is a sibling
  boundary-test shape — `crates/ui/tests/contrast.rs` reads
  `crates/ui/src/theme.rs` `pub const` tokens via the public API and
  asserts WCAG 2.1 contrast invariants (Body ≥ 4.5, Equity ≥ 7.0,
  OptOut-with-mandatory-reason skip). D1 (Pattern d Combination
  Surface-1-API-via-public-symbols + Surface-2-state-machine-via-
  enum-class-dispatch) applies: Surface 1 = `contrast_ratio` pure-fn
  over the enumerated `PAIRS` table; Surface 2 = `ContrastClass`
  dispatch into Body / Equity / OptOut. D2 (file:line locations) =
  `crates/ui/tests/contrast.rs` NEW. D3 (scope = closed regression
  categories) = palette-refactor hex drift + token-storage-shape
  drift via `MIN_PAIRS=60` floor. D4 (visual regressions NOT caught)
  — contrast asserter is hex-token level, not pixel level, same
  layering as ADR-0048 D4. D5 (M-FINAL gate) = `cargo test -p ui
  --test contrast` in both WARN + gate modes per tester M-FINAL T-CONT-
  FINAL.1/2/3. D6 (anchor-additivity) = zero anchor delta; 75/75
  byte-identical (pure test-infra addition; no production code touch
  per R-NR.1). No new ADR. See
  [`docs/archive/pre-bmad-spec/v1/ui-contrast-asserter/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/ui-contrast-asserter/feature.md)
  § Design D-CONT-1..D-CONT-7. WARN-mode-default-then-gate-flip pattern
  (D-CONT-6) inherits the env-var-gated DURABLE shape from the Pick A
  test-infra trifecta; v0.2.0 promotion contract requires upstream
  hex tune for 2 GENUINE WCAG-AA defects surfaced by M-T1 dry-run
  (`FG_ON_ACCENT_light` 3.52 + `WARN_500_light` 2.96) — these are
  NOT opt-outs but the asserter's signal-of-record.
- 2026-05-29 (architect, ui-test-harness-viewport-matrix v0.1.0 M-T1
  close): D1-D6 carry forward verbatim. Matrix harness shape extends
  the bootstrap-Charts SLOTS table (1280×720 / 1920×1080 / 3360×1890)
  across 22 in-scope fixtures via the shared
  `crates/ui/tests/fixtures/viewport_matrix.rs::snapshot_widget_at_slot`
  helper — function-with-closure shape per Q2 (a) ratification (no
  proc-macro). Zero new D-clauses; D6 anchor-additivity re-verified
  (71/71 byte-identical PASS post-merge; empirical ~13 MB net repo
  growth in baseline PNGs per K2 dry-run, well below 100 MB ceiling —
  the analyst's 50-100 MB projection was conservative by ~10× because
  the in-scope `#[test] fn` count is 22 (not 30-40) and PNG deflate on
  the cockpit dark UI is more efficient than estimated). `.gitattributes`
  rule: plain `binary` (no `diff=exif` driver present on architect
  host; analyst's driver-availability contract triggered the (b)
  durable fallback). No new ADR. See
  [`docs/archive/pre-bmad-spec/v1/ui-test-harness-viewport-matrix/feature.md`](../../../../docs/archive/pre-bmad-spec/v1/ui-test-harness-viewport-matrix/feature.md)
  § Design D-VPM-1..D-VPM-7. `.claude/agents/tester.md` inheritance
  from sibling visual-fail-html-reporter § D-VF-4 (D-VPM-6) — no
  independent amendment.
- 2026-05-29 (developer, bug-64-d11-attempt-3-yahoo-run-runtime-context v0.1.0):
  ride-along — two new e2e tests extend the Surface 1 contract per ADR-0050 § D3
  (timer-fired-in-bounded-window test contract). `lab_runner_ticker_e2e.rs`
  asserts the `tokio::time::interval(250ms)` ticker fires ≥ 3 times in a 1 s
  preload window (D-R1.3 gate for D1 rt_handle.enter() invariant).
  `lab_runner_cancel_e2e.rs` asserts Stop dispatched within 100 ms causes
  preload loop exit within 500 ms (D-R2.3 gate for D2 CancellationToken
  arm in preload select!). Both tests replicate the production select! loop
  inline (same pattern as v0.1.0 spawn_lab_run_yahoo_harness.rs). D6
  anchor-additivity: zero anchor delta; 84/84 byte-identical (pure test-infra
  addition; production code changes in cancel.rs + runner.rs are scoped to
  non-backtest crates). See ADR-0050 for the full contract.
- 2026-11-23 (story 2-63 Close, lab-recipe-test-harness v0.3.0,
  `REQ-LAB-RECIPE-HARNESS-V0-3-001`): D1-D6 carry forward; no new ADR (the
  v0.2.0 D-V0.2.0-4 carry-forward precedent). Pattern (d) now covers the four
  Recipes added since v0.2.0 (`ForwardPlanRecipe`, `NarrationOutcomeRecipe`,
  `BakeoffProgressRecipe`, `SweepProgressRecipe`), and the v0.1/v0.2 surfaces
  were re-derived from the code (the story's AC1 inventory). Waves 0, A
  (`03ad88f4`), B (`074e907a`), D (`cbe90861`), E1a (`1a4d876c`), C1
  (`0ce426cf`), C2+E1b (`73bfde06`), E2 (`b417cf67`), E3 (`8eabf996747b`) and
  F (`72a42a8c97ef`) shipped; Wave G, the agent narration-producer seam, is
  deferred (story decision 4).
  **S1 and S2 each gain a refined form.** S1, executor fidelity: a recipe
  stream is built on a thread with no tokio context and drained on iced's
  own default executor, `futures::executor::ThreadPool`, the way iced runs
  it, not inside `#[tokio::test]`, whose implicit reactor hides a missing
  guard (`crates/ui/tests/recipe_off_runtime_poll.rs`; the ADR-0050
  § D3/D4 non-tokio-executor rule, carried to the live bin's recipe
  streams; `ServerTimeRecipe` and `ToastDismissRecipe` only through their
  seams). S2, real-batch introspection: inclusion is asserted on the
  function the live bin builds its batch with, `ui::live::subscription_batch`
  (from a test ctx), unit by unit through
  `iced::advanced::subscription::into_recipes` and `Subscription::units()`,
  not only on the descriptor function the v0.2 tests pinned
  (`crates/ui/tests/subscription_batch_inclusion.rs`).
  **Two seams, both under D2's API-additive clause** and shape-identical to
  the v0.2.0 Wave C/D extractions: Wave C1's `subscription_batch` family in
  `crates/ui/src/live.rs` (`SubscriptionCtx`, `SubscriptionBatchInputs`,
  `Held`, `build_subscription_batch_descriptor_from`; the 3-argument
  `build_subscription_batch_descriptor` keeps its signature and delegates),
  with `esc_closes_modal` in `crates/ui/src/widgets/journal_transaction_modal.rs`;
  and Wave F's `ui::lab::runner::run_yahoo_preload_arm`, the Yahoo twin of
  `run_binance_preload_arm`, which `spawn_lab_run` now calls once for every
  Yahoo source.
  **Surface 1 did not drive production until Wave F.**
  `spawn_lab_run_yahoo_harness.rs`, `lab_runner_ticker_e2e.rs` and
  `lab_runner_cancel_e2e.rs` called no runner function: they imported
  `LabRunConfig`, `LabDataSource` and (the harness) the source traits for
  its mock, and copied the preload loop (the story's inventory, row 16).
  The 2026-05-29 ride-along row's "Both tests replicate the production
  select! loop inline" is superseded: all three now call
  `run_yahoo_preload_arm` on a thread with no tokio context, and two
  task-driven tests run `spawn_lab_run` itself through
  `iced_test::runtime::task::into_stream`.
  Surface 1's (A) bound is 200 ms, not 50 ms (50 ms is fragile on the
  4-vCPU Windows leg; a delay of one 250 ms tick still fails 200 ms), and
  (B)'s `elapsed_ms` is non-decreasing, not strictly increasing (tokio's
  default `MissedTickBehavior::Burst` can fire two ticks with the same
  millisecond stamp). Surface 2's category C, "pure state unchanged" on
  `LabRunStopRequested`: `lab_stop_button_gating.rs` has pinned the flag
  from an armed run since v0.1.0. Since Wave E3,
  `lab_run_cancel.rs::lab_run_stop_requested_is_noop_on_model` also arms a
  run through `update` first and pins both `lab_run_inflight` and the
  stored `run_progress` (it used to start from `false`, so on its own it
  could not fail).
  **D4 corrected** (annotated in place): its salt-bump bullet was false when
  written. `lab_progress_recipe_stream.rs` hashed no recipe, at `a9710089`
  (this ADR's first commit) or at the story's base `2d2d95be`; the only
  test in `crates/` that hashed a recipe was `TrainingPoller`'s unit test
  `stops_when_training_completes`, which compares the hashes of two
  pollers built with different run ids. Since Wave C2,
  `recipe_ids_are_frame_stable`, `salted_recipe_ids_change_with_salt` and
  `recipe_ids_are_pairwise_distinct` (`subscription_batch_inclusion.rs`)
  pin the recipe ids. A swapped salt in the bin's `SubscriptionCtx` literal
  (`crates/ui/src/bin/cockpit_live.rs`) is still checked by nothing
  (C1-RES2, bug-log `#203`).
  **Non-vacuity (bug-log `#66`):** the AC3 sweep turned every skip-as-pass
  path it found in the harness files into a failure (the per-file record is
  in the story). A missing committed corpus or fixture fails, and each such
  guard Waves E2 and E3 added has a `#[should_panic]` twin that runs its
  body on an empty root; test files that compiled to zero tests on the
  default build were un-gated. Probes (Q2; a T-T4 table in each module
  doc): Waves A to E2 are recorded per wave in the story.
  Wave E3: 21/21 catalogue probes red at the designed assertion.
  Wave F: 15/15 F-P probes red at the designed assertion; F-V1 6/6 green.
  **D6:** zero anchor delta; `scripts/verify_anchors.sh` 119/119
  before and after.
  **Known residuals** (recorded in the story): the bin's `AppState` cannot be
  built in a test (K3 #6), so its `SubscriptionCtx` literal (C1-RES1,
  C1-RES2), its receiver lifecycle and the bin-side activity producers stay
  unpinned; a cancelled Yahoo preload ends `Failed("operator cancelled")`,
  not `Cancelled`, and `lab_runner_cancel_e2e.rs` pins it as it is;
  `lab_yahoo_empty_range_classification.rs` stays `#[tokio::test]` (its
  subject is the classification), so F-P3 would leave it green (reasoned,
  not measured: F-P3's run did not build it);
  `TrainingPoller` has no production caller (K1, analyst).
