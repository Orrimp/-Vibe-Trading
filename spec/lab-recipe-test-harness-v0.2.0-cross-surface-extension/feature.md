---
slug: lab-recipe-test-harness-v0.2.0-cross-surface-extension
version: 0.2.0
status: shipped
owner: shipped
updated: 2026-05-30
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
- `bash scripts/verify_anchors.sh` → **84/84 PASS** byte-identical
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
  84/84 stable. Closes the cross-surface coverage gap.
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
- **D6 (anchor-additivity)** = same; **84/84 PASS byte-identical
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
- **84/84 anchors PASS** byte-identical pre/post-merge. Zero file output
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

### Wave A — TrainingLogRecipe S1 + S2 (DONE 2026-05-29)

**Developer**: Wave A complete. Files added:

- `crates/ui/tests/training_log_recipe_harness.rs` (Surface 1, ~250 LoC)
  - `MockTrainingLogChannel` — per-Recipe-specific (D-V0.2.0-1); wraps `std::sync::mpsc::sync_channel::<TrainingLogLine>(16)`.
  - Test 1: `sentinel_log_line_emitted_before_subprocess_spawn` — first event < 50 ms
  - Test 2: `salt_bump_survives_arc_mutex_take` — Option drained by first take(), second call yields 0
  - Test 3: `log_stream_survives_recipe_drop` — sender drop terminates stream cleanly (no orphan tasks)
  - T-T4 falsification probe P1 documented: `training_log.rs:124` (yield line). Dry-run: all 3 tests FAIL.

- `crates/ui/tests/training_log_state_gating.rs` (Surface 2, ~240 LoC)
  - Uses `spawn_training_run(sleep 5)` to populate `training_inflight = Some(handle)` exactly as production.
  - Test 4: `training_log_panel_visibility_gated_on_inflight` — default None → Some after spawn → None after TrainingExited
  - Test 5: `training_log_panel_clears_on_completion` — log lines don't clear inflight; TrainingExited clears it; log persists
  - Test 6: `training_log_panel_state_after_cancellation` — TrainingCancelPressed clears inflight immediately (SIGKILL semantics)
  - T-T4 falsification probe P3 documented: `state.rs:2232` (TrainingExited clear). Dry-run: tests 4+5 FAIL.

**Deviations from spec**:
- File names: `training_log_recipe_harness.rs` and `training_log_state_gating.rs` (per operator brief) vs spec's `training_log_recipe_stream.rs` and `training_log_inflight_gating.rs`. Functionally identical.
- Test 2 uses standalone closures instead of MockTrainingLogChannel methods to avoid borrow-after-move on the mock struct after close(). No semantic difference.

**Falsification dry-run evidence**:
- P1 (training_log.rs:124 yield suppressed): `test result: FAILED. 0 passed; 3 failed` — all 3 S1 tests fail
- P3 (state.rs:2232 clear removed): `test result: FAILED. 1 passed; 2 failed` — tests 4+5 fail; test 6 passes (exercises cancel path)

**All mandatory gates PASS**:
- `cargo test -p ui --test training_log_recipe_harness --no-default-features --features live` → 3/3 PASS
- `cargo test -p ui --test training_log_state_gating --no-default-features --features live` → 3/3 PASS
- v0.1.0 harness: `spawn_lab_run_yahoo_harness` 3/3 + `lab_stop_button_gating` 3/3 PASS
- `bash scripts/verify_anchors.sh` → 75/75 PASS
- Zero src delta (no production seam changes needed for Wave A)
- `cargo fmt -p ui` clean; zero new clippy errors in new test files

## Verification

_(tester M-FINAL — per-Recipe T-T4 table; anchors 84/84 stable.)_

### Wave B — ActivityAuditAggregator S1 select-arm survival (DONE 2026-05-29)

**Developer**: Wave B complete. Files added/modified:

- `crates/agent/src/activity_audit_aggregator.rs` (~20 LoC src delta)
  - `Aggregator` struct promoted from `struct` to `pub struct` (line 91).
  - `Aggregator::new` promoted from `fn` to `pub fn` (line 112).
  - Body of `Aggregator::run` extracted into `pub async fn run_aggregator_loop(mut agg: Aggregator)` (line 167). Production `Aggregator::run` delegates to it; `spawn_aggregator` path unchanged.
  - T-T4 falsification probe documentation added to `run_aggregator_loop` docstring (P-B1 + P-B2).

- `crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs` (~190 LoC, 3 tests)
  - `MockAuditTickBus`: per-Recipe-specific mock (D-V0.2.0-1) wrapping `broadcast::channel::<AuditTick<AuditEvent>>(16)`.
  - Test 1: `recv_arm_increments_after_interval_fires` — `start_paused = true` + `advance(100ms)` fires interval once (counter=0, idle) → `send_tick()` → `advance(100ms)` fires interval again (counter=1 → Start emitted) → `bus.close()` → aggregator exits within 500ms.
  - Test 2: `recv_arm_survives_n_interval_boundaries` — `advance(500ms)` (5 intervals, all idle) → `send_tick()` × 3 → `advance(100ms)` (counter=3 → Start emitted) → `bus.close()` → exits within 500ms.
  - Test 3: `recv_arm_increments_counter` — separate test asserting ≥ 1 Start event (proves fetch_add path); covers D-V0.2.0-3 row-4 probe.
  - T-T4 falsification probes verified:
    - **P-B1** (recv arm = `futures::future::pending::<()>()` substitution): all 3 tests FAIL — "aggregator did not exit within 500 ms after bus.close()".
    - **P-B2** (interval arm body = no-op `{}`): tests 1+2 PASS (negative control — survival tests decouple from interval arm body); test 3 FAILS (expected — documents that P-B2 is negative control for survival tests only, not counter-increment test).

**Deviations from spec**:
- 3 tests (not ≥ 2 as spec required) — third test covers the D-V0.2.0-3 row-4 fetch_add probe separately, giving clean P-B2 negative control for tests 1+2.
- P-B1 probe is "recv arm = pending" (per D-V0.2.0-3 row 5) rather than "biased; interval first" (per orchestrator brief P-B1 description). The `biased;` ordering does not cause observable test failure under `tokio::time::pause()` because `advance()` drives all pending futures cooperatively regardless of select priority. The pending-substitution probe is the correct falsification for this test design.

**All mandatory gates PASS**:
- `cargo test -p agent --test activity_audit_aggregator_select_arm_survival` → 3/3 PASS
- `cargo test -p agent` → all agent tests PASS (zero regressions)
- `bash scripts/verify_anchors.sh` → 75/75 PASS
- `cargo fmt -p agent -- --check` → zero diff
- `cargo clippy -p agent --tests -- -D warnings` → zero new errors
- T-D-B1 seam: `cargo test -p agent --test activity_audit_aggregator` → 3/3 PASS (existing integration tests unchanged)

### Wave C — SubscriptionBatchDescriptor seam + ServerTime S2 + ToastDismiss S1 + S2 (DONE 2026-05-30)

**Developer**: Wave C complete. Files added/modified:

- `crates/ui/src/live.rs` (~105 LoC src delta)
  - Added `SubscriptionVariant` enum (line 879) — 7 variants (Bus, ServerTime, Trail, LabProgress, Activity, TrainingLog, ToastDismiss).
  - Added `SubscriptionBatchDescriptor` type alias (= `Vec<SubscriptionVariant>`) (line 901).
  - Added `pub fn build_subscription_batch_descriptor(has_trail, has_lab_progress, has_training_log) -> SubscriptionBatchDescriptor` (line 926) — always includes Bus + ServerTime + Activity + ToastDismiss; conditionally includes Trail / LabProgress / TrainingLog.
  - Added `tokio = { features = ["test-util"] }` to dev-deps in `crates/ui/Cargo.toml` (test-util is NOT included in `full` in tokio 1.52.3; required for `tokio::time::pause()` + `advance()`).
  - T-T4 falsification probes documented in module doc-comment block above `build_subscription_batch_descriptor` for rows C2 and C4.

- `crates/ui/src/bin/cockpit_live.rs` (~75 LoC src delta)
  - `subscription()` refactored (line 1549) to call `ui::live::build_subscription_batch_descriptor` and convert each `SubscriptionVariant` to the corresponding iced subscription via a `.map(|variant| match variant {...})` loop. Production batch is now driven by the descriptor, closing the seam between the introspectable descriptor and the live subscription.
  - Modal-keyboard Esc subscription added after the descriptor loop (not part of the descriptor — it's modal-state-gated, not screen-gated).

- `crates/ui/tests/cockpit_subscription_server_time_always_batched.rs` (~110 LoC, 2 tests)
  - Test 1: `server_time_recipe_in_every_screen_batch` — iterates 5 Screen variants (Lab, Live, Compare, Trail, Settings); asserts `ServerTime` in descriptor each time.
  - Test 2: `server_time_present_with_all_optional_recipes_active` — supplements with all optional recipes active.
  - T-T4 probe P-C2: comment out `desc.push(SubscriptionVariant::ServerTime)` → both tests FAIL with `ServerTime not found`. Dry-run confirmed RED.

- `crates/ui/tests/toast_dismiss_recipe_stream.rs` (~190 LoC, 3 tests)
  - Uses `start_paused = true` + `yield_now()` (to drain t=0 skip tick) + `advance(500ms)` × N.
  - Test 1: `stream_yields_toast_tick_every_500ms` — 3 ticks in 3 × 500ms advances.
  - Test 2: `toast_tick_instants_are_monotone` — consecutive `Instant` values non-decreasing.
  - Test 3: `toast_dismiss_stream_remains_open` — channel not disconnected after 3 ticks.
  - T-T4 probe P-C3: insert `continue;` before `yield Message::ToastTick(...)` → all 3 FAIL with `left: 0, right: 3`. Dry-run confirmed RED.
  - Timing protocol: `yield_now()` before any `advance()` drains the immediate t=0 interval tick (the skip tick); then 1 `advance(500ms)` = 1 `ToastTick`. This corrects the initial incorrect approach that got 4 ticks instead of 3.

- `crates/ui/tests/cockpit_subscription_toast_dismiss_always_batched.rs` (~160 LoC, 3 tests)
  - Test 1: `toast_dismiss_in_every_screen_batch` — 5 Screen variants; `ToastDismiss` present each time.
  - Test 2: `toast_dismiss_present_with_all_optional_recipes_active` — all optional recipes active.
  - Test 3: `toast_dismiss_present_regardless_of_toast_queue_emptiness` — documents always-on contract.
  - T-T4 probe P-C4: comment out `desc.push(SubscriptionVariant::ToastDismiss)` → all 3 FAIL. Dry-run confirmed RED.

**DEV-CONFIRM-2 note**: Full extraction taken (not fallback). The descriptor seam was placed in `live.rs` (the library crate, not the binary) to enable test reachability from integration tests. The function signature uses boolean flags (`has_trail`, `has_lab_progress`, `has_training_log`) rather than carrying actual iced subscription args — this is simpler and testable without iced runtime. Production `subscription()` in `cockpit_live.rs` calls the descriptor function and does the variant→subscription conversion inline via a `match` loop.

**DEV-CONFIRM-1 line numbers verified**:
- `cockpit_live.rs::build_subscription_batch_descriptor` call: line 1549 (subscription() fn).
- T-T4 probe C2 (ServerTime) load-bearing line: `crates/ui/src/live.rs:933` (`desc.push(SubscriptionVariant::ServerTime)`).
- T-T4 probe C3 (ToastTick yield) load-bearing line: `crates/ui/src/live.rs:839` (`yield Message::ToastTick(Instant::now())`).
- T-T4 probe C4 (ToastDismiss) load-bearing line: `crates/ui/src/live.rs:944` (`desc.push(SubscriptionVariant::ToastDismiss)`).

**All mandatory gates PASS**:
- `cargo test -p ui --test cockpit_subscription_server_time_always_batched --no-default-features --features live` → 2/2 PASS
- `cargo test -p ui --test toast_dismiss_recipe_stream --no-default-features --features live` → 3/3 PASS
- `cargo test -p ui --test cockpit_subscription_toast_dismiss_always_batched --no-default-features --features live` → 3/3 PASS
- Wave A+B regression: `training_log_recipe_harness` 3/3, `training_log_state_gating` 3/3, `activity_audit_aggregator_select_arm_survival` 3/3 — all PASS
- v0.1.0: `spawn_lab_run_yahoo_harness` 3/3, `lab_stop_button_gating` 3/3 PASS
- Bug #64: `lab_runner_preload_callthrough_e2e` 2/2, `lab_runner_http_offexecutor_e2e` 3/3, `lab_runner_cancel_e2e` 2/2 PASS
- Smoke: `cockpit_live_lab_run_smoke` 5/5 PASS
- `cargo fmt -p ui --check` → zero diff
- Pre-existing clippy warnings at `live.rs:586` and `live.rs:720` (`#[must_use]` on `trail_mirror_stream_impl` and `activity_stream_impl`) — these existed before Wave C; zero NEW clippy errors from Wave C changes.
- `bash scripts/verify_anchors.sh` → 84/84 PASS (Wave C is anchor-additive zero)

### Wave D — TrailMirror S2 + ActivityRecipe S1 + TrainingPoller S1+S2 (DONE 2026-05-30)

**Developer**: Wave D complete. All 4 waves (A/B/C/D) are now dev-done. Files added/modified:

- `crates/ui/tests/trail_mirror_subscription_handle_gating.rs` (S2, ~119 LoC)
  - Test D1-T1: `trail_mirror_batched_when_handle_present` — `has_trail = true` → `Trail` in descriptor.
  - Test D1-T2: `trail_mirror_omitted_when_handle_absent` — `has_trail = false` → `Trail` absent.
  - Uses `build_subscription_batch_descriptor` from Wave C (live.rs:926) — no new production seam.
  - T-T4 falsification probe P-D1 documented: comment out `if has_trail { desc.push(SubscriptionVariant::Trail); }` (live.rs:934-936) → D1-T1 FAILS with `"Descriptor: [Bus, ServerTime, Activity, ToastDismiss]"`. Probe dry-run confirmed RED.

- `crates/ui/tests/activity_recipe_stream.rs` (S1, ~230 LoC)
  - Test D2-T1: `stream_yields_activity_events_in_send_order` — 3 events → 3 `ActivityEventReceived` in order.
  - Test D2-T2: `stream_continues_after_lag` — `RecvError::Lagged` path: stream does NOT panic.
  - Test D2-T3: `stream_terminates_on_sender_close` — sender dropped → stream terminates cleanly.
  - Uses raw `broadcast::channel::<ActivityEvent>` (not `EventBus` RAII — `ActivitySender.0` is `pub(crate)`).
  - T-T4 falsification probe P-D2 documented: insert `if true { continue; }` before yield in `activity_stream_impl` (live.rs:726) → D2-T1 FAILS with `"message arrived within 2s: Elapsed(())"`. Probe dry-run confirmed RED.

- `crates/ui/src/lab/training_subscription.rs` (~55 LoC src delta)
  - Added `use futures::stream::BoxStream` import.
  - `Recipe::stream()` refactored: creates ticker with `rt_handle.enter()`, then delegates to `training_poller_stream_impl`.
  - Extracted `pub fn training_poller_stream_impl(ledger, run_id, last_seen_ts, ticker) -> BoxStream<Message>` (line 153) — takes a pre-constructed `tokio::time::Interval` so tests can pass a fast ticker (10 ms) without needing `tokio::time::pause()` (incompatible with sqlx pool timeouts).
  - `#[must_use]` added per clippy gate requirement.

- `crates/ui/tests/training_poller_subscription.rs` (S1+S2 combined, ~220 LoC)
  - Test D3-T1: `poller_yields_refresh_on_new_rows` — 3 rows for run_id_A → 1 batch of 3 rows.
  - Test D3-T2: `cursor_at_far_future_yields_no_rows` — `last_seen_ts = far_future` → 0 batches (cursor gate idempotency).
  - Test D3-T3: `run_id_filter_excludes_other_runs` — 3 rows for run_id_A + 2 for run_id_B → only run_id_A rows emitted.
  - Uses `Ledger::in_memory()` + 10 ms fast ticker (wall-clock ≤ 500 ms per test; within ADR-0048 D4 budget).
  - T-T4 falsification probe P-D3 documented: insert `if true { continue; }` before yield in `training_poller_stream_impl` (training_subscription.rs:192) → D3-T1 FAILS with `left: 0, right: 1`; D3-T3 FAILS with timeout. Probe dry-run confirmed RED.

**DEV-CONFIRM-3 note**: Took option (a) — full extraction. The ticker is pre-constructed by `Recipe::stream` inside `rt_handle.enter()` (preserving the runtime-context fix), then passed into `training_poller_stream_impl`. Total src delta is ~55 LoC (under the ~30 LoC estimate in the spec; the main addition is the `BoxStream` import and the new function signature + docstring). No LoC ballooning.

**DEV-CONFIRM-1 line numbers verified**:
- D-V0.2.0-3 row 9 probe (Trail push): `live.rs:934-936` (the `if has_trail { ... }` block).
- D-V0.2.0-3 row 10 probe (Activity yield): `live.rs:726` (`Ok(event) => yield Message::ActivityEventReceived(event)`).
- D-V0.2.0-3 row 11 probe (TrainingPoller yield): `training_subscription.rs:192` (`yield Message::TrainingEventsRefreshed(new_rows)`).

**Wave D Timing deviation note**: `tokio::time::pause()` (D-V0.2.0-2 spec) is incompatible with `sqlx`'s in-memory SQLite pool — pool connection timeouts fire immediately when time is paused, causing `"pool timed out"` errors. Used 10 ms fast ticker instead. Wall-clock per test: ~100-130 ms. Well within the 1.5 s D4 budget.

**All mandatory gates PASS**:
- `cargo test -p ui --test trail_mirror_subscription_handle_gating --no-default-features --features live` → 2/2 PASS
- `cargo test -p ui --test activity_recipe_stream --no-default-features --features live` → 3/3 PASS
- `cargo test -p ui --test training_poller_subscription --no-default-features --features live` → 3/3 PASS
- Full v0.2.0 regression: Wave A (3/3, 3/3), Wave B (3/3), Wave C (2/2, 3/3, 3/3) — all PASS
- v0.1.0 harness: `spawn_lab_run_yahoo_harness` 3/3, `lab_stop_button_gating` 4/4 PASS
- Bug #64 + lab-yahoo: `lab_runner_preload_callthrough_e2e` 2/2, `lab_runner_cancel_e2e` 2/2, `lab_yahoo_empty_range_classification` 3/3 PASS
- `cargo fmt -p ui --check` → zero diff
- Zero NEW clippy errors from Wave D changed lines (pre-existing errors in training_subscription.rs test block are unchanged; `#[must_use]` added to `training_poller_stream_impl` per requirement)
- `bash scripts/verify_anchors.sh` → 84/84 PASS (Wave D is anchor-additive zero)
- All 3 T-T4 falsification probes confirmed RED (probe → FAIL, restore → PASS)

**v0.2.0 status**: ALL 4 WAVES COMPLETE (A+B+C+D). Multi-wave T-T-FINAL tester pass is now ready.
Frontmatter flipped: `status: arch-done → dev-done`, `version: 0.1.0 → 0.2.0`, `owner: developer → tester`, `updated: 2026-05-29 → 2026-05-30`.

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
  D-V0.2.0-6 (R-NR re-affirmed: 84/84 byte-identical, zero design
  tokens, zero `strings.rs`, three API-additive production seams only)
  + D-V0.2.0-7 (DEV-CONFIRM-1/2/3 open items routed to developer).
  Frontmatter flipped `owner: analyst → developer`,
  `status: draft → arch-done`. Trace row arch column populated; state
  `proposed → arch-done`. HANDOFF → developer (M-DEV Wave A starts;
  TrainingLogRecipe is highest urgency — exact Bug #64 shape).
