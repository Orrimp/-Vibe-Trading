---
slug: lab-recipe-test-harness-v0.2.0-cross-surface-extension
version: 0.1.0
status: arch-done
owner: developer
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

> **M-T1 close (architect 2026-05-29).** M-OD locked Q1=(a) [all 4 +
> extras] + Q2=(a) [per-Recipe T-T4 falsification probe documented in
> each test file]. R3 picks **per-Recipe-specific mocks** (analyst
> default); ADR-0048 D1-D6 contract **carries forward verbatim** — one
> Changelog row appended, no new ADR. M-DEV decomposes into four
> dependency-ordered waves A→D (~1 week dev + 1 day tester). Frontmatter
> flips `owner: analyst → developer`, `status: draft → arch-done`.

### D-V0.2.0-1 — R3 mock pattern: per-Recipe-specific (NOT a single shared trait)

**Decision.** Each new test file ships its own bespoke mock struct +
trait impl, mirroring the v0.1.0 `MockLabYahooBarSource` shape. NO
single `MockChannel<T>` trait family.

**Rationale.**

1. **No shared surface to abstract over.** The four highest-urgency
   Recipes each take a structurally distinct input — `std::mpsc::Receiver`
   bridged via `spawn_blocking` (TrainingLog), `broadcast::Receiver`
   inside a two-arm `tokio::select!` (ActivityAuditAggregator),
   `tokio::time::interval` over a `Handle` (ServerTime + ToastDismiss),
   and `tokio::sync::broadcast::Receiver<TrailMirrorTick>` (TrailMirror).
   A single generic mock would need an associated type per dimension
   (channel kind, message type, clock source, lag policy) — the
   abstraction collapses to a marker trait with no methods.
2. **Independent ship / revert.** Per-Recipe files compile and fail
   independently. K2 (existing v0.1.0 harness breakage from trait
   collision) is structurally eliminated.
3. **Mirrors v0.1.0 precedent.** `MockLabYahooBarSource` is a private
   test-file struct, not a workspace-visible trait. Same shape repeated
   four times keeps the harness pattern legible to the next maintainer.
4. **Cost of the rejected alternative.** A shared trait `MockChannel<T>`
   would force `ActivityAuditAggregator`'s `tokio::select!` arm to be
   re-expressed as a single-channel poll, defeating the entire point of
   the boundary test (which is to assert the two-arm shape doesn't
   cross-consume).

### D-V0.2.0-2 — Per-Recipe mock surface inventory

| Recipe | Mock struct | Lives at | Mock surface | Production trait/seam |
|---|---|---|---|---|
| `TrainingLogRecipe` | `MockTrainingLogChannel` | `tests/training_log_recipe_stream.rs` | Wraps a real `std::sync::mpsc::sync_channel::<TrainingLogLine>(16)`. Mock holds the `Sender<TrainingLogLine>`; test drives `tx.send(line)` then `drop(tx)` (sender-drop signals stream EOF). Mock is the `Sender` half + a `take_rx()` accessor; the recipe under test takes the `Receiver` via `Arc<Mutex<Option<_>>>::take()` exactly as production does. | Existing `stream_impl(rx_opt)` helper at `crates/ui/src/lab/training_log.rs:103` — already extracted for test reachability. **No new production seam needed.** |
| `ActivityAuditAggregator` | `MockAuditTickBus` | `crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs` | Wraps a real `tokio::sync::broadcast::channel::<AuditTick<AuditEvent>>(16)`. Mock holds the `Sender`; test drives `tx.send(tick)` interleaved with `tokio::time::advance(Duration::from_millis(100))` to fire the `interval.tick()` arm. Asserts: after N interval boundaries, `rx.recv()` still yields the NEXT tick (channel not consumed/shadowed by the select). | Promote `Aggregator::new(rx, bus)` from `pub(crate)` to `pub` AND extract `Aggregator::run` body into `pub async fn run_aggregator_loop(rx, bus)` so the test file can drive the loop without `spawn_aggregator`'s `JoinHandle`-only return. |
| `ServerTimeRecipe` (S2) | (no struct mock — uses real production `subscription()` fn under `Cockpit::default()` introspection) | `tests/cockpit_subscription_server_time_always_batched.rs` | Construct `Cockpit::default()`; flip `current_screen` across all five `Screen::*` variants; assert via the subscription-batch list count that the recipe is unconditionally included (current contract — S2 here is a **regression guard against future screen-gating** of an always-on recipe, not asserting NEW gating). | Extract a `pub fn build_subscription_batch_descriptor(cockpit, bus, rt_handle, trail_mirror, lab_progress_rx, lab_progress_salt, training_log_rx, training_log_salt) -> SubscriptionBatchDescriptor` from `cockpit_live.rs::subscription()` — returns a structurally-introspectable enum-list (one variant per recipe) the test can match on. Production `subscription()` then calls `from_recipe(...)` on each variant. **Architect carry-over note**: this descriptor doubles as the seam ToastDismiss S2 also needs. |
| `ToastDismissRecipe` (S1+S2) | `MockClock` NOT used — `tokio::time::pause()` + `advance()` (already proven sufficient for this surface per ADR-0048 § Alternatives). S2 uses the `SubscriptionBatchDescriptor` introduced for ServerTime above. | `tests/toast_dismiss_recipe_stream.rs` (S1) + `tests/cockpit_subscription_toast_dismiss_always_batched.rs` (S2) | S1: drive `toast_dismiss_stream_impl(&rt_handle)` under `tokio::test(start_paused = true)`; `advance(500ms)` N times; assert N `ToastTick` messages with monotone `Instant`. S2: same descriptor pattern as ServerTime — always batched regardless of `current_screen` or `toast_queue.is_empty()`. | None new (uses extracted `toast_dismiss_stream_impl` and the `SubscriptionBatchDescriptor` seam). |
| `TrailMirrorRecipe` (S2 only) | `MockTrailMirrorHandle` | `tests/trail_mirror_subscription_handle_gating.rs` | Mock returns `TrailMirrorHandle::new(broadcast::channel(8).0)`. Test 1: `trail_mirror_handle = Some(_)` → batched. Test 2: `trail_mirror_handle = None` → omitted (yields `Subscription::none()` — the production fallback). Uses the `SubscriptionBatchDescriptor` seam. | None new. |
| `ActivityRecipe` (S1 only) | `MockActivityBus` (in-process `Arc<EventBus>`) | `tests/activity_recipe_stream.rs` | Construct a real `EventBus::new(BusConfig::default())`; subscribe via `activity_stream_impl(bus.activity().subscribe())`; drive `bus.activity().send(event)` × 3; assert 3 `Message::ActivityEventReceived` arrive in order. Tests Lagged + Closed paths via `tokio::time::sleep` + sender drop. | None new (uses extracted `activity_stream_impl` at `live.rs:720`). |
| `TrainingPoller` (deferred to Wave D, scope-confirmed by Q1=(a)) | `MockAuditLedger` = `audit::Ledger::in_memory()` (already exists). | `tests/training_poller_subscription.rs` | Bypass the `Recipe::stream()` iced surface (no extracted `stream_impl` exists yet) by replicating the inner `async_stream::stream!` body with the same 1 Hz poll + filter + `last_seen_ts` advance logic. Test asserts: write 3 rows for `run_id_A` + 2 rows for `run_id_B`; poll yields only the 3 `_A` rows in order; second poll yields zero (idempotent). Test 2: drop the in-memory ledger mid-poll → stream errors gracefully (logs `warn!`, continues; does NOT panic). | Extract `pub async fn training_poller_stream_impl(ledger, run_id, last_seen_ts) -> BoxStream<Message>` from `training_subscription.rs:108-144`. Production `Recipe::stream` delegates to it. **This IS a new production seam — analogous to v0.1.0's `LabYahooBarSource` trait extraction.** |

### D-V0.2.0-3 — Per-Recipe T-T4 falsification probe lines (Q2=(a) durable)

Each new test file's module docstring MUST include a `## T-T4 falsification
probe` section naming the EXACT source-line to comment out + the
expected FAIL assertion. Mirrors v0.1.0 `spawn_lab_run_yahoo_harness.rs`
docstring shape. Lines below are pinned at architect time (2026-05-29);
developer verifies line numbers haven't drifted before committing each
docstring.

| Wave | Recipe | File | Falsification line — comment out to FAIL | Expected FAILing assertion |
|---|---|---|---|---|
| A | `TrainingLogRecipe` (S1) | `crates/ui/src/lab/training_log.rs:124` (`yield Message::TrainingLogLine(...)`) | `tests/training_log_recipe_stream.rs::stream_yields_lines_in_order` — receiver yields 0 messages, expected 3 → `assert_eq!(messages.len(), 3)` FAILs with `left: 0, right: 3` |
| A | `TrainingLogRecipe` (S1, take-ownership probe) | `crates/ui/src/lab/training_log.rs:87` (`.take()`) — replace with `.as_ref().cloned()` semantic-break OR comment the whole `take()` and substitute `None` | `tests/training_log_recipe_stream.rs::recipe_takes_receiver_via_arc_mutex_option` — second invocation of `recipe.stream()` after first take yields >0 messages, expected 0 → `assert!(messages.is_empty())` FAILs |
| A | `TrainingLogRecipe` (S2) | `crates/ui/src/state.rs:2232` (`model.lab_state.training_inflight = None;` in `Message::TrainingExited` arm) | `tests/training_log_inflight_gating.rs::training_exited_clears_inflight` — `training_inflight` remains `Some(_)` after `TrainingExited`, expected `None` → `assert!(cockpit.lab_state.training_inflight.is_none())` FAILs |
| B | `ActivityAuditAggregator` (S1) | `crates/agent/src/activity_audit_aggregator.rs:134` (`self.counter.fetch_add(1, ...)` in `recv_result = Ok(_tick)` arm) | `crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs::recv_arm_increments_after_interval_fires` — after `advance(100ms)` (interval fires) + `tx.send(tick)`, the next interval boundary sees `counter == 0`, expected `counter >= 1` → emitted `ActivityEvent::Tick` has count 0 instead of 1 → `assert_eq!(received_tick_count, 1)` FAILs |
| B | `ActivityAuditAggregator` (S1, select-arm shadowing probe) | `crates/agent/src/activity_audit_aggregator.rs:131` (`recv_result = self.rx.recv() =>` arm guard) — substitute the arm body with `_ = futures::future::pending::<()>() =>` so the arm never fires | `crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs::recv_arm_survives_n_interval_boundaries` — after `advance(500ms)` (5 interval fires) + `tx.send(tick)`, counter stays 0 across N windows → `assert!(total_ticks_received >= 1)` FAILs |
| C | `ServerTimeRecipe` (S2 always-batched) | `crates/ui/src/bin/cockpit_live.rs:1571-1573` (`let time_sub = iced::advanced::subscription::from_recipe(ServerTimeRecipe { ... });`) — comment out the let binding AND remove `time_sub` from BOTH `Subscription::batch` calls | `tests/cockpit_subscription_server_time_always_batched.rs::server_time_recipe_in_every_screen_batch` — `SubscriptionBatchDescriptor::ServerTime` absent from the descriptor list across all 5 `Screen::*` variants → `assert!(descriptor.contains(SubscriptionVariant::ServerTime))` FAILs |
| C | `ToastDismissRecipe` (S1) | `crates/ui/src/live.rs:839` (`yield Message::ToastTick(Instant::now());`) | `tests/toast_dismiss_recipe_stream.rs::stream_yields_toast_tick_every_500ms` — under `tokio::time::pause()` + `advance(500ms)` × 3, receiver yields 0 messages, expected 3 → `assert_eq!(ticks.len(), 3)` FAILs |
| C | `ToastDismissRecipe` (S2 always-batched) | `crates/ui/src/bin/cockpit_live.rs:1623-1625` (`let toast_dismiss_sub = ...`) — comment out + remove from both batches | `tests/cockpit_subscription_toast_dismiss_always_batched.rs::toast_dismiss_in_every_screen_batch` — `SubscriptionBatchDescriptor::ToastDismiss` absent → `assert!(descriptor.contains(SubscriptionVariant::ToastDismiss))` FAILs |
| D | `TrailMirrorRecipe` (S2 handle-gated) | `crates/ui/src/bin/cockpit_live.rs:1579-1583` (the `.map(...).unwrap_or_else(iced::Subscription::none)` block) — replace with unconditional `iced::Subscription::none()` | `tests/trail_mirror_subscription_handle_gating.rs::trail_mirror_batched_when_handle_present` — descriptor lacks `TrailMirror` variant when handle is `Some(_)` → `assert!(descriptor.contains(SubscriptionVariant::TrailMirror))` FAILs |
| D | `ActivityRecipe` (S1) | `crates/ui/src/live.rs:726` (`yield Message::ActivityEventReceived(event);`) | `tests/activity_recipe_stream.rs::stream_yields_activity_events_in_send_order` — receiver yields 0 messages, expected 3 → `assert_eq!(events.len(), 3)` FAILs |
| D | `TrainingPoller` (S1+S2 combined) | `crates/ui/src/lab/training_subscription.rs:141` (`yield Message::TrainingEventsRefreshed(new_rows);`) | `tests/training_poller_subscription.rs::poller_yields_refresh_on_new_rows` — under `tokio::time::pause()` + `advance(1s)` × 2 after writing 3 rows, receiver yields 0 messages, expected 1 batch → `assert_eq!(refreshes.len(), 1)` FAILs |

### D-V0.2.0-4 — ADR-0048 carries forward (one Changelog row, no new ADR)

ADR-0048 D1-D6 contract maps to v0.2.0 verbatim:

- **D1 (pattern)** = combination of boundary-test + state-gating — same.
- **D2 (file locations)** = mechanically replicated per Recipe (one S1
  file + one S2 file each where both surfaces apply). Two new
  production seams (`training_poller_stream_impl` extraction at Wave D;
  `SubscriptionBatchDescriptor` extraction at Wave C) are API-additive,
  same shape as v0.1.0's `LabYahooBarSource` trait extraction — covered
  by the D2 "API-additive" clause.
- **D3 (regression categories A/B/C)** = same; per-Recipe falsification
  table at D-V0.2.0-3 above proves each new test catches at least one
  named category.
- **D4 (NOT-catches)** = same; harness is behaviour, not pixels.
- **D5 (invocation cadence)** = same; tests join the default suite,
  `#[cfg(feature = "live")]` where appropriate.
- **D6 (anchor-additivity)** = same; **71/71 PASS byte-identical
  pre/post** (R-NR contract). Zero file output from any new test.

**Verdict.** No new ADR required. ADR-0048 § Changelog gets a single
2026-05-29 row referencing this feature.md.

### D-V0.2.0-5 — Wave decomposition for developer (M-DEV)

Waves are dependency-ordered. Wave C extracts a seam (`SubscriptionBatchDescriptor`)
that Wave D (TrailMirror S2 + ToastDismiss S2 already covered) re-uses,
so Wave C MUST land before Wave D's TrailMirror test. Waves A and B are
independent and can run concurrently if dev bench available.

| Wave | Scope | Files added | Production seam delta | LoC budget | Sequential after |
|---|---|---|---|---|---|
| **A** (HIGHEST URGENCY — Bug #64 exact shape) | `TrainingLogRecipe` S1 + S2 | `tests/training_log_recipe_stream.rs` (S1, ~120 LoC) + `tests/training_log_inflight_gating.rs` (S2, ~80 LoC) | None (uses existing `stream_impl(rx_opt)` at `training_log.rs:103`) | ~200 LoC | — (start here) |
| **B** (Bug #64 `tokio::select!` shape) | `ActivityAuditAggregator` S1 boundary | `crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs` (~150 LoC) | Promote `Aggregator::new` to `pub`; extract `pub async fn run_aggregator_loop(rx, bus)` from `Aggregator::run` body. Production `spawn_aggregator` re-calls the extracted fn. | ~150 LoC + ~20 LoC src delta | — (parallel to A) |
| **C** (extract `SubscriptionBatchDescriptor` seam — enables D) | `ServerTimeRecipe` S2 + `ToastDismissRecipe` S1 + S2 | `tests/cockpit_subscription_server_time_always_batched.rs` (~60 LoC) + `tests/toast_dismiss_recipe_stream.rs` (~120 LoC) + `tests/cockpit_subscription_toast_dismiss_always_batched.rs` (~60 LoC) | **NEW** — Extract `pub fn build_subscription_batch_descriptor(...) -> SubscriptionBatchDescriptor` from `cockpit_live.rs::subscription()`. Descriptor is a `Vec<SubscriptionVariant>` enum where each variant carries the args the production `from_recipe(...)` call would receive. Production `subscription()` calls `build_subscription_batch_descriptor(...).into_iced_subscription()`. **API-additive — anchor-clean.** | ~240 LoC + ~80 LoC src delta | A or B (independent of A/B; sequential before D) |
| **D** (lower urgency, batch) | `TrailMirrorRecipe` S2 + `ActivityRecipe` S1 + `TrainingPoller` S1+S2 | `tests/trail_mirror_subscription_handle_gating.rs` (~80 LoC) + `tests/activity_recipe_stream.rs` (~120 LoC) + `tests/training_poller_subscription.rs` (~150 LoC) | Extract `pub async fn training_poller_stream_impl(ledger, run_id, last_seen_ts) -> BoxStream<Message>` from `training_subscription.rs:108-144`. Production `Recipe::stream` delegates to it. (TrailMirror + Activity already have extracted `stream_impl` helpers.) | ~350 LoC + ~30 LoC src delta | C (uses `SubscriptionBatchDescriptor` for TrailMirror gating) |

**Totals.** ~940 LoC new tests + ~130 LoC src deltas (3 API-additive
extractions: `run_aggregator_loop`, `build_subscription_batch_descriptor`,
`training_poller_stream_impl`). Under K4 budget (≤ 800 LoC tests was the
analyst-suggested ceiling; +18% reflects the descriptor-seam adding 2
small subscription-presence tests not in analyst's count — operator
already approved Q1=(a) durable, the descriptor is the load-bearing seam
that makes 4 of the 10 falsification probes mechanically possible).

**Wall-clock.** ~1 week dev as estimated; suggested split: A (1.5 days)
‖ B (1.5 days) → C (2 days, includes seam extraction + iced
`Subscription::batch` plumbing) → D (1.5 days). +1 day tester.

### D-V0.2.0-6 — R-NR contract (architect re-affirms)

- All v0.1.0 harness tests (`spawn_lab_run_yahoo_harness.rs` 3 tests +
  `lab_stop_button_gating.rs` 3 tests) stay byte-identical PASS.
- All Recipe stream tests already shipped (`lab_progress_recipe_stream.rs`,
  `server_time_recipe_stream.rs`, `trail_mirror_recipe_stream.rs`,
  `live_subscription{,_full_bus}.rs`, `cockpit_training_pressed_wiring.rs`)
  stay byte-identical PASS.
- **71/71 anchors PASS** byte-identical pre/post-merge. Zero file output
  from any new test (channel-only events + pure-state assertions only).
- **Zero new design tokens / zero `strings.rs` adds** — harness tests
  are non-user-visible.
- The three new production seams (Wave B `run_aggregator_loop` extract,
  Wave C `SubscriptionBatchDescriptor`, Wave D `training_poller_stream_impl`)
  are **API-additive only**; production call sites change only at the
  delegation point (call the new pub fn instead of inline body). No
  binary-path behaviour change; anchor preservation evidence: the
  existing `crates/backtest/tests/determinism.rs` row 70 SHA + the
  `cockpit_live_lab_run_smoke.rs` cockpit-smoke gate cover the
  production path's untouched behaviour.

### D-V0.2.0-7 — Open items routed to developer

- **DEV-CONFIRM-1**: Verify the four falsification probe LINE NUMBERS
  (training_log.rs:87, :124; state.rs:2232; activity_audit_aggregator.rs:131,
  :134; cockpit_live.rs:1571-1573, :1579-1583, :1623-1625; live.rs:726,
  :839; training_subscription.rs:141) before committing each test
  file's module docstring. If a line shifted between architect M-T1
  (2026-05-29) and dev start, update the docstring to the current
  position. **Do NOT comment out a different line "close enough" — the
  point of the probe is exactness.**
- **DEV-CONFIRM-2**: The `SubscriptionBatchDescriptor` extraction (Wave C)
  is the one design-impacting delta. Developer MAY propose an alternative
  test-introspection seam IF the descriptor extraction proves invasive
  (e.g. breaks the `cockpit_live.rs` orphan-rule shape) — route via short
  HANDOFF → architect ping rather than self-deciding the alternative.
  Acceptable fallbacks: (a) `#[cfg(test)] pub` on the inner subscription-
  vector before `Subscription::batch(vec)` is called; (b) macro-based
  expansion that emits both the production batch AND a `Vec<&'static str>`
  variant-name list visible to tests.
- **DEV-CONFIRM-3**: Wave D `TrainingPoller` test takes the option of
  EITHER (a) extracting `training_poller_stream_impl` and driving it,
  OR (b) replicating the inner loop body inline (zero src delta, mirrors
  the existing `last_seen_ts_advances_only_on_new_rows` pattern at
  `training_subscription.rs:187`). Architect default: (a) extraction is
  preferred for symmetry with the other Recipe `stream_impl` shapes, but
  (b) is acceptable if the extraction balloons LoC beyond ~30 src delta.

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
- 2026-05-29 (architect): M-T1 closed. M-OD locked Q1=(a) all 4 +
  extras + Q2=(a) per-Recipe T-T4 falsification probe DURABLE.
  D-V0.2.0-1 (per-Recipe mock pattern, NOT single trait) +
  D-V0.2.0-2 (per-Recipe mock surface inventory: `MockTrainingLogChannel`,
  `MockAuditTickBus`, `MockTrailMirrorHandle`, `MockActivityBus`,
  `MockAuditLedger`; `tokio::time::pause()` for clock-driven recipes) +
  D-V0.2.0-3 (per-Recipe falsification probe lines pinned across 11
  rows — 4 production files: `training_log.rs`, `state.rs`,
  `activity_audit_aggregator.rs`, `cockpit_live.rs`, `live.rs`,
  `training_subscription.rs`) + D-V0.2.0-4 (ADR-0048 D1-D6 carries
  forward verbatim — no new ADR, single Changelog row on ADR-0048) +
  D-V0.2.0-5 (Wave A→D dependency-ordered decomposition; A‖B parallel,
  C extracts `SubscriptionBatchDescriptor` seam, D depends on C) +
  D-V0.2.0-6 (R-NR re-affirmed: 71/71 byte-identical, zero design
  tokens, zero `strings.rs`, three API-additive production seams only)
  + D-V0.2.0-7 (DEV-CONFIRM-1/2/3 open items routed to developer).
  Frontmatter flipped `owner: analyst → developer`,
  `status: draft → arch-done`. Trace row arch column populated; state
  `proposed → arch-done`. HANDOFF → developer (M-DEV Wave A starts;
  TrainingLogRecipe is highest urgency — exact Bug #64 shape).
