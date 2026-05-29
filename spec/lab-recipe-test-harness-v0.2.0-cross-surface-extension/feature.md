---
slug: lab-recipe-test-harness-v0.2.0-cross-surface-extension
version: 0.1.0
status: draft
owner: analyst
updated: 2026-05-29
predecessor: lab-recipe-test-harness v0.1.0
priority: P2
---

# Lab Recipe test harness v0.2.0 — cross-surface extension

> **P2 — durable, not cheap.** v0.1.0 proved the two-surface harness
> pattern catches the channel-survival + state-gating regression class
> Bug #64 surfaced. This brief extends the same pattern PREEMPTIVELY
> to every other vulnerable Recipe in `crates/ui/` and `crates/agent/`
> so the next regression in this class gets caught at `cargo test`
> time, not at operator visual-verify time.

## Why (durable contract — AGENT.md 2026-05-29)

v0.1.0 covers only `spawn_lab_run` (S1) + `lab_run_inflight` (S2). Four
other `Recipe` / aggregator surfaces have the SAME shape (mpsc/broadcast
+ `Arc<Mutex<Option<_>>>::take()` + state-gated predicate) and zero
boundary/gating pair. Per the new durable-over-quick contract, extend
the proven pattern PREEMPTIVELY (~1 week dev) before the next visual-
verify revert lands (~3-5 days deferred cleanup + 1-2 lossy windows).

## Requirements

### R1 — Recipe inventory (per-surface vulnerability map)

Every `impl Recipe` / Subscription-pattern in `crates/ui/` and
`crates/agent/`, with regression class exposure and current coverage:

| # | Recipe / surface | File:line | Channel shape | Regression class exposure | Existing coverage | v0.2.0 status |
|---|---|---|---|---|---|---|
| 1 | `LabProgressRecipe` | `crates/ui/src/lab/progress.rs:57` | tokio mpsc + `Arc<Mutex<Option<_>>>::take()` + per-run salt | A sentinel / B channel survival / C inflight predicate | v0.1.0 (S1 + S2 both done; `lab_progress_recipe_stream.rs`, `lab_stop_button_gating.rs`, `spawn_lab_run_yahoo_harness.rs`) | **DONE — carry forward, no edits** |
| 2 | `ServerTimeRecipe` | `crates/ui/src/bin/cockpit_live.rs:127` + `ui::live::server_time_stream_impl` | tokio interval (no channel) | timer-arm survival; runtime-context (K8) panic | `server_time_recipe_stream.rs` covers Surface 1 cadence + open-stream | **Surface 2 gating MISSING** — no test pins "is the recipe wired to update loop when the cockpit is in screen X?" |
| 3 | `ToastDismissRecipe` | `crates/ui/src/bin/cockpit_live.rs:166` + `ui::live::toast_dismiss_stream_impl` | tokio interval, always-on | timer-arm survival; runtime-context panic; **dismiss-flag gating** | **ZERO** — no boundary test, no gating test | **Both S1 + S2 MISSING** |
| 4 | `TrainingLogRecipe` | `crates/ui/src/lab/training_log.rs:65` | std-mpsc bridged via `spawn_blocking` + `Arc<Mutex<Option<_>>>::take()` + per-run salt | **EXACTLY Bug #64 shape** — `take()` ownership, salt-bump, subprocess channel | unit tests in module assert filter shape only; NO `stream_impl` boundary test; NO `training_log_inflight` gating test | **Both S1 + S2 MISSING — HIGHEST URGENCY** |
| 5 | `TrainingPoller` (subscription) | `crates/ui/src/lab/training_subscription.rs:82` | tokio interval + audit DB poll, run-id identity gate | timer-arm survival; idempotent-since-cursor; identity-hash-on-run-id | hash-identity unit test only; NO boundary stream test asserting "ticks fire AND emit `TrainingEventsRefreshed` when audit ledger gets a new row" | **Both S1 + S2 MISSING** |
| 6 | `BusRecipe` (11 channels) | `crates/ui/src/live.rs:114` | `broadcast::Receiver`, per-channel | lag-handling, close-handling, channel discriminant | `live_subscription.rs` + `live_subscription_full_bus.rs` cover Surface 1 per-channel | **DONE — carry forward** |
| 7 | `TrailMirrorRecipe` | `crates/ui/src/live.rs:551` | `broadcast::Receiver` | lag, close, eager-subscribe race | `trail_mirror_recipe_stream.rs` covers Surface 1 | **Surface 2 gating MISSING** — no test pins "Trail screen renders trail-ticks only when mirror is armed" |
| 8 | `ActivityRecipe` | `crates/ui/src/live.rs:691` | `broadcast::Receiver` | lag, close, publish-before-subscribe race | `activity_tape_event_storm.rs` drives broadcast (not recipe surface) | **Surface 1 stream_impl boundary test MISSING; S2 partial via storm test** |
| 9 | `ActivityAuditAggregator` (agent) | `crates/agent/src/activity_audit_aggregator.rs:128` | `broadcast::Receiver<AuditTick>` → `tokio::select!` (rx + 100 ms interval) | **EXACTLY Bug #64 `tokio::select!` shape** — two arms, channel-survival across interval | `activity_audit_aggregator.rs` + `activity_audit_aggregator_invariants.rs` cover counter math + lifecycle | **Surface 1 EXTRA test MISSING** — boundary test asserting `interval.tick()` arm never consumes / shadows `rx` |

**Scope decision matrix (per Recipe):**

| Recipe | S1 needed? | S2 needed? | Mock pattern |
|---|---|---|---|
| `TrainingLogRecipe` | YES | YES | `MockTrainingLogChannel`: closure over `std::mpsc::Sender<TrainingLogLine>` + sender-drop signal |
| `TrainingPoller` | YES | YES | `MockAuditLedger`: in-memory `Ledger::in_memory()` already exists; harness writes rows + asserts polling reads them |
| `ActivityAuditAggregator` | YES | (S2 covered by existing invariants test) | `MockAuditTickBus`: `broadcast::channel::<AuditTick>` injected directly — already used by existing tests; new test adds the **`tokio::select!`-arm-survival** assertion (channel still receives after interval fires) |
| `ToastDismissRecipe` | YES | YES | `MockClock` not needed — `tokio::time::pause()` + manual `advance()` already provides the same control; gating test mirrors `lab_stop_button_gating.rs` but on the `toasts` queue field |
| `ServerTimeRecipe` (S2 only) | (S1 done) | YES | gating test asserts: `cockpit.screen == Screen::Lab` does NOT silence the `ServerTimeTick` → status-bar update |
| `TrailMirrorRecipe` (S2 only) | (S1 done) | YES | gating test asserts: `Trail` screen renders incoming tick only when `model.trail_mirror_armed == true` |
| `ActivityRecipe` (S1 only) | YES | (S2 storm-test partial) | `activity_stream_impl_boundary.rs` mirrors `lab_progress_recipe_stream.rs` shape on `broadcast::Receiver<ActivityEvent>` |

### R2 — Harness pattern application contract

For each uncovered Recipe, the **same v0.1.0 T-T4 falsification protocol**
applies: tester comments out the single load-bearing line in source
(stop-flag clear / channel-take / select-arm bind), runs the harness,
asserts ≥ 1 test FAILs with a descriptive message, restores, verifies
3/3 (or N/N) PASS. Per-Recipe falsification probe stub MUST be
documented in each new test file's module doc-comment (mirrors
v0.1.0 `spawn_lab_run_yahoo_harness.rs` Test 1 docstring).

### R3 — Per-Recipe mock pattern (architect ratifies in M-T1)

Open architect question: **single shared mock pattern** (one
`MockChannel<T>` trait family for all four) OR **per-Recipe-specific
mocks** (the v0.1.0 `MockLabYahooBarSource` shape, distinct per
surface). Analyst default: **per-Recipe-specific** — same shape as
v0.1.0; avoids cross-surface coupling and lets each test file ship/
revert independently.

### R4 — Non-regression contract

- All v0.1.0 harness tests (3 in `spawn_lab_run_yahoo_harness.rs` + 3
  in `lab_stop_button_gating.rs`) stay byte-identical PASS.
- All existing Recipe stream tests (`lab_progress_recipe_stream.rs`,
  `server_time_recipe_stream.rs`, `trail_mirror_recipe_stream.rs`,
  `live_subscription{,_full_bus}.rs`, `cockpit_training_pressed_wiring.rs`)
  stay byte-identical PASS.
- `bash scripts/verify_anchors.sh` → **71/71 PASS** byte-identical
  pre/post-merge.

### R-NR — Zero new surface area

- **Zero new ADRs** — ADR-0048 D1-D6 carries forward; the per-Recipe
  application is a mechanical extension of the pattern, not a new
  decision.
- **Zero new design tokens** — harness tests are non-user-visible.
- **Zero new `strings.rs` entries** — same.
- **Zero anchor delta** — channel-only events / pure-state assertions,
  no file output.
- **Zero production-binary path changes** — all extensions are
  API-additive (mirror v0.1.0's `LabYahooBarSource` trait pattern OR
  use existing extracted `stream_impl` helpers).

## Falsifiers

- **K1 — A Recipe surfaces a regression class the v0.1.0 pattern does
  NOT cover** (e.g. multi-channel coordination on `ActivityAuditAggregator`
  `tokio::select!`; cross-Recipe race on `TrainingPoller` + `TrainingLogRecipe`
  during run lifecycle). If observed, document the gap in feature.md
  § Implementation and route back to analyst for a v0.3.0 follow-up
  scope question.
- **K2 — Existing v0.1.0 harness tests break under v0.2.0 additions**
  (e.g. trait-pattern collision between `LabYahooBarSource` and a new
  `MockAuditLedger` if developer chooses single-trait pattern). Routes
  back to architect for M-T1 re-ratification.
- **K3 — A Recipe is genuinely well-isolated and doesn't need a
  harness** (rare; document explicit reasoning). Candidates: `BusRecipe`
  per-channel cases already at 100 % via `live_subscription_full_bus.rs`;
  this brief excludes them from the work list and treats existing
  coverage as evidence.
- **K4 — Cargo build time inflation** from new tests above 10 % over
  v0.1.0 baseline. Mitigation: each new test file ≤ 100 LoC;
  `#[cfg(feature = "live")]` gating where the recipe is live-only.

## Hypotheses

- **H1 — Pattern application catches ≥ 1 new regression class per
  Recipe** (validates the durable investment). Most likely surface:
  `TrainingLogRecipe` — exact Bug #64 shape (subprocess `mpsc` channel
  + `take()` + per-run salt) currently has zero boundary coverage.
- **H2 — Per-Recipe LoC budget ≈ 80 LoC** (~50 Surface 1 + ~30 Surface
  2), matching v0.1.0 footprint per Recipe.
- **H3 — T-T4 falsification confirmable in < 5 min per Recipe** —
  same protocol as v0.1.0; tester runs once per Recipe.
- **H4 — Post-v0.2.0, harness coverage is "every UI Recipe /
  Subscription"** — durable contract honored; future Recipe additions
  inherit the pattern via copy + edit + falsify cycle.

## Operator decisions

- **Q1 — Scope.**
  (a) **[Recommended — DURABLE]** all four uncovered Recipes
  (`TrainingLogRecipe` + `TrainingPoller` + `ToastDismissRecipe` +
  `ActivityAuditAggregator`) PLUS Surface-2-only extension for
  `ServerTimeRecipe` + `TrailMirrorRecipe` and Surface-1-only for
  `ActivityRecipe`. **~1 week dev + 1 day tester.** Future Recipes
  inherit the pattern; closes the durable-coverage contract.
  (b) **[cheap fallback]** `TrainingLogRecipe` only (the next-most-
  vulnerable per Bug #64 retrospective: exact same `take()` + salt
  shape). **~2-3 days.** Defers `TrainingPoller` + `ToastDismissRecipe`
  + `ActivityAuditAggregator` to v0.3.0 (with the operator-visible
  deferral commitment + a v0.3.0 backlog row carrying the scope-list
  byte-identically). +3-5 days deferred cost.

- **Q2 — Falsification protocol.**
  (a) **[Recommended — DURABLE]** T-T4-style per-Recipe falsification
  probe documented in each test file's module doc-comment AND tester
  M-FINAL report § 4 records the FAIL → restore → PASS cycle per
  Recipe. Proves the harness genuinely catches per surface, mirrors the
  v0.1.0 lesson that "prove it or it's theater".
  (b) **[cheap fallback]** no per-Recipe falsification proof — assume
  the v0.1.0 protocol generalizes. **Rejected by analyst** — v0.1.0
  was literally created because a 415-PASS gate set still missed three
  live regressions; "the harness probably catches it" is not evidence.

**Cost framing.**
- Q1=(a) + Q2=(a) durable = ~1 week dev + 1 day tester. Anchors
  71/71 stable. Closes the cross-surface coverage gap.
- Q1=(b) + Q2=(b) cheap = ~2-3 days now + ~3-5 days deferred to
  v0.3.0 for ActivityAudit + TrainingPoller + ToastDismiss + ServerTime
  S2 + TrailMirror S2 + Activity S1. Net +0 days, but with 1-2 visual-
  verify revert windows in between (call it +1-3 days lossy).

## Verdict tree (pre-drawn)

| Q1 \ Q2 | Q2=(a) durable proof | Q2=(b) no proof |
|---|---|---|
| **Q1=(a) all 4 + extras** | **DURABLE — Recommended.** ~1 week dev + 1 day tester; closes the cross-surface gap; pattern catches every shape v0.1.0 was designed to catch. PASS → ship. | INCONSISTENT — durable scope without durable proof is the v0.1.0 anti-pattern. Reject. |
| **Q1=(b) Training only** | SOFT-PASS — covers the highest-urgency Recipe with proof, defers the rest with explicit v0.3.0 commit. Operator-acceptable if the v0.3.0 row lands before merge. | Cheap fallback — defers everything with no proof. FAIL by the v0.1.0 lesson; reject unless operator explicitly waives. |

## Design

_(architect M-T1 — ratify Q1+Q2; lock R3 mock-pattern; ADR-0048
Changelog amendment iff R3 changed the contract.)_

## Implementation

_(developer — per-Recipe waves A-F; T-T4 probe per wave.)_

## Verification

_(tester M-FINAL — per-Recipe T-T4 table; anchors 71/71 stable.)_

## Changelog

- 2026-05-29 (analyst): M0 brief authored; R1 inventory enumerates
  9 surfaces (4 uncovered); R2-R4 + R-NR locked; Q1+Q2 framed durable-
  recommended; 4-cell verdict tree pre-drawn. Trace row
  `REQ-LAB-RECIPE-TEST-HARNESS-V0-2-0-001` opened at `proposed`.
  HANDOFF → architect (M-T1 ratifies R3 mock-pattern + decomposes
  M-DEV per-Recipe waves).
