# Story 2.63: lab-recipe-test-harness-v0.3-extension

Status: in-progress

<!-- Analyst-drafted 2026-07-29 (Mary). Operator-DECIDED build: PRD §13 Q2 answer, 2026-07-27
     ("schedule"). Promotes the standing backlog row (backlog.md § Deferred by decision, tracked
     as this story slug) from backlog to ready-for-dev. TEST-INFRA, not product feature surface. -->

## Story

As the operator of the Honest Advisor,
I want the v0.3 cross-surface extension of the lab recipe/subscription test harness — the ADR-0048 pattern applied to every Recipe added since v0.2.0, with every recipe test asserting EXECUTION,
so that the v0.2 durable contract ("harness coverage is every UI Recipe/Subscription"; H4: future Recipes inherit the pattern) holds again on today's code, and no recipe guard can pass vacuously (the bug-log #66 class: a test that exists and passes is not a test that runs).

## Acceptance Criteria

1. **Fresh R1-style inventory:** re-derive the per-surface vulnerability map (mirror v0.2's R1 table) over today's `impl Recipe`/subscription set in `crates/ui/` + `crates/agent/`; it MUST cover at minimum the four post-v0.2 recipes verified uncovered 2026-07-29 — `ForwardPlanRecipe` (`crates/ui/src/live.rs:889`, zero test references), `NarrationOutcomeRecipe` (`:963`, zero), `SweepProgressRecipe` (`:1128`, zero), `BakeoffProgressRecipe` (`:1052`, partial via `tests/bakeoff_progress_relay.rs`) — and classify each as S1 (boundary `stream_impl` test) and/or S2 (gating/inclusion test) needed, with K3 well-isolated exclusions reasoned in writing.
2. **Pattern-(d) coverage lands per the inventory:** each in-scope recipe gets its S1/S2 tests via the per-Recipe-specific mock pattern (ADR-0048 D1-D6 carry forward; the extracted `forward_plan_stream_impl`/`narration_outcome_stream_impl` seams already exist — reuse; new production seams are API-additive extractions only, shape-identical to v0.2's Wave C/D precedent).
3. **Non-vacuity — every recipe test asserts EXECUTION (bug-log #66 precedent, 2026-07-27 hardening theme):** no skip-as-pass anywhere in the harness suite (new AND the surviving v0.1/v0.2 files): fixture/corpus paths resolve from the workspace root (`CARGO_MANIFEST_DIR`-derived, never cwd-relative), skip is legal ONLY on genuine probe-absence, any `Err` with the fixture present FAILS loudly, and each test proves it ran (event/count assertions that cannot be satisfied by an early return). A sweep records the audit result per existing harness file.
4. **Falsification proof (v0.2 Q2=(a) durable protocol):** every new test file carries the T-T4 probe line in its module doc-comment and the tester report records one FAIL → restore → PASS cycle per recipe — no probe, no PASS ("prove it or it's theater").
5. **Batch-inclusion + floor:** `build_subscription_batch_descriptor` (`crates/ui/src/live.rs:1361-1387`, the Wave C seam) gains/asserts variants for the in-scope recipes' inclusion gating; zero production behavior change beyond API-additive seams; `verify_anchors` 119/119 before AND after; `python3 scripts/spec_lint.py` PASS; clippy clean; build-time inflation ≤ 10% (K4 carry-over).

## Tasks / Subtasks

- [x] Inventory pass (AC 1) — output as a table in this story's Dev Agent Record before coding. *(2026-11-10, below.)*
- [ ] Wave-decomposed implementation per inventory (mirror v0.2's Wave A-D shape; one tester verdict per wave).
  - [x] Wave 0: baselines + TrainingLog confirmation. *(2026-11-10; confirmed defect became bug-log `#196`, fixed in `dcce75e2`.)*
  - [x] Wave A: post-v0.2 S1, test-only. *(2026-11-11; 27/27 probes red, see "Wave A results".)*
  - [x] Wave B: executor fidelity, test-only. *(2026-11-12; 40/41 probes red, one survivor B-P41 routed to E1, see "Wave B results".)*
  - [ ] C1 · C2 · D · E1 · E2 · F (G deferred, decision 4).
- [ ] #66 non-vacuity sweep over the existing harness suite (AC 3) — fix or document every skip path.
- [ ] ADR-0048 § Changelog row for v0.3.0 (no new ADR expected — D-V0.2.0-4 precedent) unless a genuinely new pattern class emerges (K1 route: back to analyst).
- [ ] Gates: anchors 119/119, spec-lint, clippy, fmt; T-T4 probe evidence in the tester report.

## Dev Notes

- **What v0.2 covered (so v0.3 doesn't re-litigate):** v0.2.0 shipped the FULL Q1=(a) scope 2026-05-30 — all four waves (TrainingLog S1+S2, ActivityAuditAggregator select-arm survival, SubscriptionBatchDescriptor seam + ServerTime/ToastDismiss, TrailMirror S2 + Activity S1 + TrainingPoller S1+S2). Nothing was deferred FROM v0.2; "v0.3.0+" was reserved (brief § Hypotheses H4 + falsifier K1 routing) for exactly this: applying the contract to recipes that did not exist yet. The four advisor-era recipes above (F5 forward-plan, F9 narration, F1/F2 bakeoff progress, F8 sweep progress) are that accrued surface.
- **Motivating precedent (cite in every skip-path fix): bug-log #66** — the ui real-data guard tests were vacuous since day 1 (cwd-relative corpus root; any-`Err`→skip), the anchored report's "ran for real" claim held only for the backtest twins, and reviving ONE chain surfaced three real product bugs within the hour. Moral, per the entry: skip paths need positive proof of execution. That is AC 3's bar, suite-wide.
- **Do-not-build register check (mandatory): PASS.** Test-infra only — no alpha surface (Group A untouched), no scope change (B), no data feeds (C), no execution machinery (D), no gate/anchor edit (E; pure test additions, the v0.2 D6 anchor-additivity precedent held byte-identical). The backlog row was explicitly re-confirmed "still wanted" at the v3 close-out (2026-07-09) and CHANGELOG § Deferred carries it as the one genuinely-open forward build item.
- Feature-completeness note: this story does NOT reopen feature development; it is the hardening/infra lane the 2026-07-27 theme names. `#[cfg(feature = "live")]` gating where a recipe is live-only (K4 carry-over).

### References

- Trace: `REQ-LAB-RECIPE-HARNESS-V0-3-001` (state=`in-progress` since 2026-11-10)
- Epic: `_bmad-output/planning-artifacts/epics.md` § Epic 2 (Cockpit & UI (Lumen shell, Live, Lab, charts, quality gates))
- Decision record: PRD §13 Q2 (operator answer 2026-07-27); `_bmad-output/planning-artifacts/backlog.md` § Deferred by decision; ADR-0048 (+ v0.2.0 Changelog row); predecessor stories `2-41-lab-recipe-test-harness`, `2-42-lab-recipe-test-harness-v0-2-0-cross-surface-extension`; `docs/dev-notes/bug-log.md` #66.

## Dev Agent Record

### Agent Model Used

Claude Opus 5.5 (orchestrator). Inventory by a read-only workflow: one agent per post-v0.2 recipe, one re-check of the v0.1/v0.2 recipes, two AC3 audit agents, then a synthesis + completeness critic that re-read the cited lines at HEAD `2d2d95be`. No cargo was run during the inventory.

### Debug Log References

- Inventory workflow run `wf_23e8261b-a57` (2026-11-10): 8 agents, 0 errors.
- Wave A workflow run `wf_95e5c67f-b2a` (2026-11-11): implement → independent falsify → fix (12 vacuity findings) → re-probe; 3 agents, 0 errors.

### Completion Notes List

**2026-11-10: dev started (orchestrator, under the standing decide-and-proceed authority).** The inventory is below, and it is much larger than the story assumed:

- **AC1's "zero test references" is true only for the Recipe structs.** `forward_plan_stream_impl` and `narration_outcome_stream_impl` already have seam tests (`forward_narration_relay.rs`). Only `SweepProgressRecipe` is genuinely untested.
- **The v0.2 descriptor tests pin a function that no real batch is checked against.** No test executes a subscription arm. iced 0.14's `into_recipes` / `units()` make this introspectable, contrary to the v0.2 docs.
- **Every existing recipe test runs under `#[tokio::test]`, but production polls on iced's ThreadPool with no tokio context.** The executor-fidelity blind spot is covered by Wave B.
- **Two HIGH-severity AC3 findings.**
  - `cockpit_live_lab_run_smoke.rs::smoke_yahoo_cache_hit` skips on every CI run. Its corpus (`data/yahoo/`) is gitignored: the exact bug-log `#66` shape.
  - The three ADR-0048 v0.1 Surface 1 files (`spawn_lab_run_yahoo_harness.rs`, `lab_runner_ticker_e2e.rs`, `lab_runner_cancel_e2e.rs`) never call production code. They copy the loop.

**Decisions recorded here:**

1. **`TrainingPoller` gets no tests (K1 → analyst).** It has no production caller; `scripts/callers.sh` finds only its own unit test. Wiring it or retiring it is a scope decision, so it is routed to the analyst, not built around.
2. **The suspected `TrainingLogRecipe` off-runtime panic (`training_log.rs:115`, `spawn_blocking` inside an iced-polled stream) is NOT fixed here.** AC5 forbids production behaviour change beyond additive seams. Wave 0 confirms or refutes it with a throwaway test. If it is confirmed, it gets a bug-log entry and its own fix story, and its off-runtime test lands with that fix.
3. **Narration is `FellBack`-only in production.** `llm_provider` is `None` in both branches, `crates/agent/src/runtime.rs:1613-1625`, `TODO(F9-v0.3)`, tracked nowhere. That is disclosed through the bug-log, not tested around. A green harness here must not be read as "LLM narration works".
4. **Wave G (the agent narration-producer seam) is deferred.** No AC requires it, so it is recorded as a residual.
5. **Seam shape for C1: the `Held` enum + `SubscriptionBatchInputs`, not a struct of 7 bools.** `#![warn(clippy::pedantic)]` (`crates/ui/src/lib.rs:24`) plus CI's `-D warnings` would fail `struct_excessive_bools`. The existing 3-arg descriptor fn keeps its signature and DELEGATES, and a literal-order pin keeps that from becoming self-referential (bug-log `#161`/`#187` form).

**Later outcomes of decisions 2 and 3. The decisions above stay as written.**
- **Decision 2 was confirmed and fixed outside this story.** Wave 0's throwaway off-runtime test panicked. That became bug-log `#196`, fixed in `dcce75e2`: the stream is now a std-thread bridge with no tokio reactor. Its off-runtime test landed with that fix, as the decision required: `recipe_stream_drains_without_a_tokio_runtime` in `crates/ui/src/lab/training_log.rs`. Wave B therefore keeps "§6 without TrainingLog".
- **Decision 3 is no longer true of production.** The operator ruled "make the claim true" (bug-log `#197`), and `bab164f0` plumbs the boot-built provider into the narration task. Explain now reaches the LLM when `llm.enabled`. The pin is `crates/agent/tests/narration_task_uses_boot_provider.rs`. The ui harness here still tests only the relay; it never calls a provider.

**2026-11-10: Wave 0 results (baselines; nothing committed from the wave itself).**
- **K4 baseline:** `cargo test -p ui --no-run` took **342 s** wall on a warm target and produced **119** test executables (same box). AC5's 10% ceiling is therefore about **376 s**. The Close wave re-measures on the same box.
- **Anchors:** 119/119 before any Wave A edit, and spec_lint PASS.
- **TrainingLog:** confirmed. See the decision 2 outcome above.

**2026-11-11: Wave A results (post-v0.2 S1, test-only).**
- **Files:** `crates/ui/tests/forward_narration_relay.rs` (11 tests), `crates/ui/tests/bakeoff_progress_relay.rs` (6), and the NEW `crates/ui/tests/sweep_progress_relay.rs` (4). There are 21 tests across the three files, and **no production edit**: `git diff -- crates/ui/src crates/agent/src` is empty.
- **Round 1, independent falsifier.** An agent that wrote none of the tests applied all 13 designated probes, P1–P13, one at a time. Every probe turned a test red, and every restore was byte-identical (`cmp`).
  - P3's red fired early, at a `SendError` unwrap, not at its designed assertion.
  - The vacuity review raised **12 findings**: tautological counters, by-construction preconditions, an accepting timeout, self-computed arithmetic, and a test named for an enqueue it never exercised.
  - Probe P4b was planned but never run.
- **Round 2, fix plus re-probe.** All 12 findings were fixed in the same three files. The notable changes:
  - An `assert_pending` helper (`now_or_never()` must be `Pending`). It distinguishes an ENDED stream from a live one, and catches an extra buffered item.
  - Seeded stale state before every "cleared" or "untouched" precondition.
  - A two-sided request-facts test over `fake_bakeoff_report_mirror` vs `…_benchmark_wins`, which differ in every dimension. It was renamed from `explain_action_enqueues_request_with_faithful_facts` to `narration_request_from_mirror_carries_the_on_screen_facts`.
  - Checked sends throughout.
- **The re-probe ran 27 probes:** the 13 originals, P4b, and 13 new ones (P14–P26), each aimed at a newly added assertion.
  - **27/27 went red at their designed assertions.**
  - All 27 restores were byte-identical (`cp` + `touch`, so cargo rebuilds).
  - The SHA-256 of `live.rs`, `state.rs`, `leaderboard/state.rs` and `tune/screen_state.rs` was unchanged at the end.
  - The same agent wrote and probed the round-2 tests, which is why the orchestrator re-checked below.
- **Measured vacuity of the predecessor:** the previous `forward_narration_relay.rs` stays **11/11 green** under P19 (`winner_id: candidate_ids[0].clone()`). Its single-fixture request test could not catch a positional winner.
- **Orchestrator re-check, independent of both agents:**
  - The three targets are green after a fresh `Compiling ui` (6 + 11 + 4).
  - `cargo fmt -p ui -- --check` exits 0.
  - `cargo clippy -p ui --test forward_narration_relay --test bakeoff_progress_relay --test sweep_progress_relay -- -D warnings` is clean after a forced re-lint (`touch`; `Checking ui` was shown).
  - Anchors are **119/119** and spec_lint PASS after the wave.
  - I ran two probes with my own harness (exact-once replace, run, restore, `cmp`):
    - **P19**: red at `forward_narration_relay.rs:710`, "BenchmarkWins mirror: the crowned winner id is carried", `v0.sma` ≠ `v0.buyhold`.
    - **P25′**, my own *additive* variant: the SweepProgress arm writes the tune **and also** the leaderboard, a subtler cross-talk than the agent's replacement. It went red at `sweep_progress_relay.rs:224`, the cross-talk control itself, `(3,12,…)` ≠ `(2,7,"v0.5.macd")`.
    - Both restores were byte-identical.
- **Residuals the wave cannot close, stated in the module docs:**
  - The Explain enqueue (`try_send` in `crates/ui/src/bin/cockpit_live.rs`) is unreachable from an integration test. That is the K3 #6 `AppState` builder residual.
  - Recipe salt identity is unpinned here. It belongs to §5, Wave C2.
- **Probe numbering:** P1–P26 are taken by the three Wave A files. Later waves prefix their probes with the wave (`B-P1`, `C1-P1`, …) so no two tables share an id.
- **Evidence (scratch, not committed):** `/private/tmp/claude-502/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/362d2a09-04ba-4ea6-a7c1-07605f6e187a/scratchpad/wave-a/` holds `round1-P*.log`, `round2/summary.txt` and `round2/P*.log` (per-probe red list, failure text, restore check), and `orch/` (the orchestrator's two probes and clippy log).

**2026-11-11 to 11-12: Wave B (executor fidelity, test-only). DONE; see "Wave B results" below. Decisions:**
- **Executor.** The harness polls on iced's own default executor, `<iced::executor::Default as iced::Executor>::new()`.
  - Its return type is pinned to `futures::executor::ThreadPool`, so it only type-checks while iced's default IS that pool (`iced_futures-0.14.0/src/backend/native/thread_pool.rs:4`).
  - After round 2, `Recipe::stream()` runs on an event-loop stand-in thread, and only the drain goes to the pool. That is where iced 0.14 puts them: `iced_winit` `runtime.track` → `iced_futures` `tracker.rs` `recipe.stream(..)` synchronously, then `executor.spawn`.
- **Round 1, independent falsifier (`wf_74575068-523`).** All 11 probes went red.
  - **B-P1…B-P9 went red ONLY in the new file, and all 8 `#[tokio::test]` sibling targets stayed green.** These are reactor-dependent calls in a stream body (`tokio::time::timeout` around a recv) and deleted enter guards. Nine production regressions the pre-Wave-B harness cannot see: the blind spot this wave exists for, now measured.
  - B-P10 and B-P11, leaked `EnterGuard`s, also turned siblings red: `server_time_recipe_stream` went 4/4 red, plus one `forward_narration_relay` test, via tokio's out-of-order guard-drop panic.
  - The falsifier also raised 5 vacuity findings. They were fixed in round 2, which added B-P12…B-P16.
- **Round 2 was stopped deliberately at B-P5.** B-P1…B-P4 had re-measured red, only in the new file, siblings green.
  - Each probe cost about 12 min, because 9 test binaries were relinked each time.
  - Meanwhile an independent read-only review (`wf_0b3be12f-0e4`: 3 lenses, each finding checked by 2 skeptics) left 7 verified weaknesses (below). Round 3 rewrites the file for them, so finishing round 2 would have measured a revision that is never committed.
  - The `live.rs` it left mutated was restored from the agent's snapshot and shown byte-equal to `git show HEAD:crates/ui/src/live.rs`.
- **Review verdict on fidelity: faithful, not stricter.**
  - `cockpit_live.rs` holds no tokio `EnterGuard` on iced's thread: `fn main` has no `#[tokio::main]`, `drop(bootstrap_rt)` runs before `iced::application(..)`, and the agent runtime lives only in `rt.block_on` on the `agent-runtime` side thread.
  - `Executor::enter` is the no-op trait default for the thread pool.
  - So production `stream()` calls really do run with no tokio context.
- **The 7 surviving review findings, fixed in round 3:**
  - R1 (medium): a buffer-until-close recipe passed every relay test. The only wake proven was the close wake.
  - R2 (medium): the take-once and eager-subscribe checks ran after the first poll.
  - R3: TrainingLog was excluded, but its own test builds and polls on one thread.
  - R4: the 11 BusRecipe streams were never exercised, and the headline claimed "every recipe".
  - R5: server-time freshness spanned about 2 s of wall clock.
  - R6: the 5 s hang guard on the expected-panic shapes may be too short for backtrace symbolization on Windows.
  - R7: stamps were bounded only from below.
- **Re-probe scope decision.** The final, fresh re-probe runs B-P1…B-P11 against the new target only.
  - Those probes' sibling outcomes are an invariant already measured in round 1: neither the 8 sibling files nor those production probe texts changed since.
  - New probes (B-P12 upward) run against the new target plus all siblings.
- **Rounds 3 and 4 (`wf_0a2326a7-62d`).** The fix agent resolved R1–R7 and declared B-P17…B-P27 (11 → 14 tests). A fresh falsifier then applied all 27 probes to that revision (SHA `3d7a7f31…`): **27/27 went red at their designed assertion**, with every restore byte-identical and rebuilt. The 5× flake check was green.
  - Siblings, now measured: B-P16 turns `toast_dismiss_recipe_stream` red 3/3; B-P17 turns `bakeoff_progress_relay` red 4/6. **B-P14 (a 10× faster server-time interval) leaves `server_time_recipe_stream` GREEN.** Its `…_emits_at_1_hz_cadence` test asserts only that the stamps do not decrease.
  - Two survivors, measured: **X-1** (server-time interval 1 s → 2 s) and **X-2** (toast 500 ms → 1000 ms) left the new file 14/14 green. Cadence was pinned only from below.
  - Round 4 added a `(k+1)`-period upper bound (B-P28, B-P29), symmetric freshness checks for both tick streams, a cancellable `Drain` guard so a failing test no longer prints a misleading "runtime is being shutdown" panic, and a read-on for the activity eager-subscribe test (14 → 15 tests, SHA `52bab6eb…`). Its re-falsifier measured B-P28 and B-P29 red, and each also turned its tick sibling red.
- **Gap that decides round 5.** Round 4 changed the shared harness (the `Drain` guard, the tick helpers, the activity read-on) after B-P1…B-P27 were measured. **No probe except B-P28/29 has been measured on the file that would be committed.** Round 5 therefore re-applies every probe to the final revision.
- **Round 4's re-falsifier left 9 findings. Decisions (2026-11-12):**
  1. *Slow side pinned only to `[P, 1.5P)`.* **Split the labour instead of tightening the slack.** A tighter real-time bound trades load tolerance on the 4-vCPU Windows leg for drifts that no unit slip produces. Exact periods belong to the paused-clock siblings:
     - `toast_dismiss_recipe_stream` already pins the period exactly (`advance(500ms)` → one tick).
     - `server_time_recipe_stream::…_emits_at_1_hz_cadence` is made to do the same: paused clock, consecutive tick gaps equal to the period. That also fixes the sibling vacuity B-P14 measured, so the test's name stops lying.
     - The new file keeps the coarse real-time window, and its doc states that window exactly instead of "pinned from both sides".
  2. *Timing bounds are exposed to load from the deliberately panicking self-test (backtrace symbolization under `RUST_BACKTRACE=1`).* **Serialize them.** One `static` `RwLock`: the two tick tests take it for writing, every other test for reading. The tick tests then never share the binary with a panic in flight. The cost is about 1 s of binary time.
  3. *Self-test shape (g) passes on an expired wait.* **Make it two-sided.** After the 1 s silence, drop the sender. The buffered items must then arrive in order, followed by END. That proves `next_item` sees items, and that the silence was buffering rather than a broken reader.
  4. *Teardown-order claim stated as universal.* Doc fix only.
  5. *Harness-shape asserts reported as a production panic.* Give harness self-checks a `HARNESS:` payload prefix, have `interpret()` report them as a harness defect, and add a self-test for the label.
  6. *`expect_waiting` cannot tell a stale `Waiting` from a fresh one.* At each turn, drain the relay's queue: an `Item` before its own publish is named **cross-talk**. After each turn, sweep the other relays for `Item`s.
  7. *Staleness under `P/2` passes.* Document it as a design limit. No realistic edit produces it, and tightening it would cost load tolerance.
  8. *By-construction premises.* Already labelled as harness self-checks. No action.
  9. *The T-T4 table is stale for B-P28/29.* Update the rows to measured.
  - B-P20…B-P22's `--lib` claims (the training-log unit test) are measured in round 5. `cargo test -p ui --lib lab::training_log` is allowed for those three probes only.
- **Round 5 sibling policy.** The sibling files are unchanged since they were measured, except `server_time_recipe_stream`, which decision 1 edits. Probes therefore run against the new target only, plus `server_time_recipe_stream` for every server-time probe (B-P1, B-P10, B-P13, B-P14, B-P27, B-P28) and plus `--lib lab::training_log` for B-P20…B-P22.
- **Rounds 5 and 6 (`wf_f002d295-ed8`).**
  - Round 5 resolved the 9 findings as decided above and added B-P30…B-P34.
  - Its read-only review and its falsifier left 12 more:
    - One review finding was rated high: the harness skipped `Tracker::update`'s per-recipe hash and same-id skip. A `BusRecipe::hash` that ignores the channel therefore passed every test, although iced would stream only Fills.
    - Two production-fact counts were mislabelled "HARNESS DEFECT".
    - Several doc claims were false.
    - Three sibling vacuities were measured.
  - Round 6 fixed them:
    - test-thread asserts on iced-id identity and recipe count;
    - per-recipe context sampling;
    - bounded lock and teardown waits whose expiry fails the test;
    - a grace period for a panic report that arrives late;
    - two new self-tests (16 → 18 tests);
    - in the server-time cadence test, a two-sided freshness check plus 3600 paused ticks of liveness;
    - new probes B-P35…B-P38.
  - A fresh falsifier then re-applied **every** probe to that file (SHA `bc9133fb…`, sibling `d132975d…`).
- **Correction to decision 1.** Its premise, that "`toast_dismiss_recipe_stream` already pins the period exactly", was false.
  - That sibling accepts a period in (375 ms, 500 ms].
  - Its `toast_tick_instants_are_monotone` check cannot fail: on a paused clock it compares real `Instant`s about 24 µs apart, measured under B-P31.
  - Toast's exact period and liveness therefore go to Wave E1, which owns that file.
- **Correction to decision 2.** The cost is not "about 1 s". The binary went from about 2.0 s (no lock) to 4.07 s (round 5's lock), then 5.02 s (round 6's two self-tests), then 6.08 s (the close-out's third writer, below).

**2026-11-12: Wave B results (executor fidelity, test-only). DONE.**
- **Files.**
  - NEW `crates/ui/tests/recipe_off_runtime_poll.rs` (18 tests).
  - `crates/ui/tests/server_time_recipe_stream.rs`: only `server_time_stream_impl_emits_at_1_hz_cadence` was rewritten. It runs on a paused clock: every gap is exactly 1 s, over 3600 ticks; each stamp is fresh, checked from both sides; and the stream is still live at the end.
  - **No production edit:** `git diff -- crates/ui/src crates/agent/src` is empty.
- **Final falsifier sweep (round 6) on the file as probed:**
  - **40 of 41 probes went red at their designed assertion.** That is 38 designated (B-P1…B-P38) plus two of the falsifier's own: B-P39 (server time at 1.6 s) and B-P40 (toast at 800 ms), the first measurement of the `k = 2` upper cadence bound.
  - B-P30, B-P31 and B-P38 are green in this file by design, because 1.4 × P sits inside its documented `[P, 1.5P)` window and a ticker that stops after 4 ticks outlives its 2 ticks. Each was red in its paused-clock sibling, at the designed assertion.
  - Every restore was byte-identical and rebuilt (`Compiling ui`).
  - The flake check was 5× green (18/18 and 4/4).
  - Production equalled HEAD at the end.
- **The blind spot, measured.**
  - B-P1…B-P9 (a reactor-dependent call in a stream body, or a deleted enter guard) are red **only** here. All 8 `#[tokio::test]` sibling targets stayed green (rounds 1 and 2).
  - B-P21 and B-P22 (a leaked `EnterGuard` in `TrainingLogRecipe::stream`) are caught only by this file. The lib test that bug-log `#196` cites, `recipe_stream_drains_without_a_tokio_runtime`, was measured **green** under both.
- **One measured survivor, routed: B-P41.**
  - The mutation: `toast_dismiss_stream_impl`'s `loop {` → `for _ in 0..4 {`. This file stays 18/18 green and `toast_dismiss_recipe_stream` 3/3. Nothing pins the toast ticker past its 4th tick.
  - **Routed to Wave E1**, which owns `toast_dismiss_recipe_stream.rs`. The fix is a paused-clock liveness test in the style of `server_time_stream_impl_emits_at_1_hz_cadence`, which measured B-P38 (the server-time analogue) red.
  - B-P41 becomes an E1 designated probe.
- **Close-out edit after the sweep (`wf_7005929c-184`): documentation plus one lock mode.**
  - The T-T4 table's last column now holds the round-6 measurements, and rows B-P39…B-P41 were added.
  - `a_late_panic_report_is_reported_as_the_panic` moved from the read lock to the write lock. It has a real-time bound of its own: the test thread must start a 100 ms wait within 900 ms, and the falsifier reasoned that load could turn it falsely red. No production probe targets it.
  - An independent read-only checker audited every table cell against the probe logs. It found 10 wording errors and no code change beyond that one line. A fixer verified each error against the logs and corrected it.
  - Final SHA `8db233d1…` (4082 lines).
- **Orchestrator re-check, independent of every agent, on the final file:**
  - `cargo fmt -p ui -- --check` exits 0.
  - `cargo test -p ui --test recipe_off_runtime_poll --test server_time_recipe_stream` passed 4 times, with `Compiling ui` on the first run: 18/18 in 6.07–6.08 s and 4/4 in 3.20 s.
  - `cargo clippy -p ui --test recipe_off_runtime_poll --test server_time_recipe_stream -- -D warnings` is clean after `touch` (`Checking ui` shown).
  - I applied two probes with my own harness (exact-once replace, run, restore with `cp`, compare with `git show HEAD:`):
    - **B-P35** went red at `bus_recipes_relay_off_runtime`'s "IDENTITY: bus recipe #2 (Positions) has the same iced id as bus recipe #1 (Fills)". That is the review's high finding, closed.
    - **B-P1** went red at `server_time_stream_ticks_off_runtime`'s "`Recipe::stream` PANICKED on the event-loop stand-in thread (no tokio context)".
    - Both restores are byte-equal to HEAD.
  - Anchors are **119/119**, spec_lint PASS, and the bug-log schema check PASS.
- **Routed, not fixed here:**
  - **To E1, which owns that file:**
    - The real-time server-time siblings (T-ST-1a/1c/1d) accept any interval under 1.5 s. That is by decision 1.
    - T-ST-1d's doc claims it builds a real `ServerTimeRecipe`. E1 deletes T-ST-1d.
    - The vacuous toast monotonicity check.
  - **Recorded in the file:**
    - Diagnosis quality, not vacuity: the bus cross-talk label names the foreign channel only when that channel publishes first. A relay rewired to a later channel fails as "hung", and is still red.
    - A design limit: a stamp stale by less than P/2 passes.
- **Evidence (scratch, not committed):** `/private/tmp/claude-502/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/362d2a09-04ba-4ea6-a7c1-07605f6e187a/scratchpad/wave-b/` holds:
  - `r5x2-B-P*.{summary,log,diff}`, `r5x2-flake.out`, `r5x2-rwlock-measure.out` and `r5x2-final.log`;
  - `closeout/` (the edit log, the diff, the pre-edit snapshots);
  - `orch/` (my green, clippy, flake and probe logs).

### Inventory (AC1), audit (AC3) and wave plan, re-verified at HEAD `2d2d95be`

### Inventory (AC1)

I re-verified this inventory at HEAD `2d2d95be` by reading the source. Unless a cell says "per agent, not re-read", every file:line below was re-read by me.

**Scope.** `impl Recipe for` appears 13 times:
- `crates/ui/src/live.rs`: 7 (lines 119, 577, 718, 894, 968, 1065, 1141)
- `bin/cockpit_live.rs`: 2 (lines 133, 172)
- `bin/cockpit.rs`: 1 (line 78)
- `lab/`: 3 (progress.rs:57, training_log.rs:65, training_subscription.rs:83)

There are also 2 `iced::event::listen_with` sites, 1 inert subscription fn, and 4 producer loops that feed the recipes.

The descriptor fn is at live.rs:1383-1403; the story's AC5 cites :1361-1387, but :1361 is the type alias. AC1's line numbers (:889, :963, :1052, :1128) are the struct lines; the impls follow at :894, :968, :1065 and :1141.

| # | recipe | definition | gate | seam | S1 status | S2 status | batch-inclusion status | action |
|---|---|---|---|---|---|---|---|---|
| 1 | **ForwardPlanRecipe** | struct live.rs:889, impl :894; hash = TypeId + `0xF6_F6u16` (:900-902), no salt; delegates at :916 | `live` (live.rs:37; default features Cargo.toml:303; both CI test legs ci.yml:781, :791) | `forward_plan_stream_impl` live.rs:931 (exists) | **PARTIAL.** Seam tests in forward_narration_relay.rs: :73 sends one plan only, so `while`→`if` at :936 survives. :121 sends only a drop, so an empty stream passes. :134 is the None control. `ForwardPlanRecipe` has 0 test refs (grep), so `Recipe::stream` never runs. No off-runtime poll. | **MISSING.** Loading (state.rs:2808) → Ready (:3997) is never driven. promote_swept_config.rs:22 is `#![cfg(feature="fixtures")]`, so it builds 0 tests on the macOS leg. | **OUTSIDE descriptor.** cockpit_live.rs:2590-2597, condition `plan_rx.is_some()` (outer Option, Some from :832, never reset). 0 tests. | S1 + S2. Joins shared seam C1. |
| 2 | **NarrationOutcomeRecipe** | struct :963, impl :968; hash TypeId + `0xF9_F9u16` (:976); delegates at :988 | `live` | `narration_outcome_stream_impl` :997 (exists) | **PARTIAL.** forward_narration_relay.rs:150 and :186 send one outcome each (`while`→`if` at :1002 survives). No `Recipe::stream`. Tokio-context only. | **MISSING.** The guard at state.rs:3880 (`Ready result && !is_requested`) is never driven through `update`. | **OUTSIDE.** cockpit_live.rs:2603-2610 on `narration_outcome_rx.is_some()` (Some from :840). | S1 + S2. Note: production only ever sends `FellBack`, because `llm_provider` is `None` in both branches (crates/agent/src/runtime.rs:1613-1625). |
| 3 | **BakeoffProgressRecipe** | struct :1052, impl :1065; hash TypeId (:1071) + salt (:1073); delegates at :1086 | `live` | `bakeoff_progress_stream_impl` :1102 | **PARTIAL-STRONG.** bakeoff_progress_relay.rs has 5 tests, including a real `Recipe::stream` at :190-237. Gaps: no count/order assertion (`Err(_) => break` at :145); no off-runtime poll; salt identity is never hashed (0 hits for `Hasher::default` in tests). | **MISSING.** The `begin_run` clear (leaderboard/state.rs:1264) is pinned only by a vacuous precondition (relay.rs:80-85, starts from a fresh `Cockpit`). The `finish_run` clear (:1284) is unpinned. | **OUTSIDE.** cockpit_live.rs:2619-2627. rx/salt set at :2412-2417 (bump :2414), cleared at :1965. | S1 top-up + S2. |
| 4 | **SweepProgressRecipe** | struct :1128, impl :1141; hash TypeId (:1147) + salt (:1148); delegates at :1161. A 1:1 copy of #3. | `live` | `sweep_progress_stream_impl` :1171 | **MISSING.** 0 test refs to the recipe, the seam, or `Message::SweepProgress`. | **MISSING.** tune/screen_state.rs:1085 and :1097 assert `progress.is_none()` from Default, which is vacuous. | **OUTSIDE.** :2634-2642. Set :2453-2458, cleared :1974. | Full S1 + S2. |
| 5 | **BusRecipe ×11** | private struct live.rs:114, impl :119; hash TypeId + Channel (:126); dispatch match :136-150; batched by `ui::live::subscription` :65-81 | `live` | per-channel `stream_*` fns (pub) | **PARTIAL.** The `stream_*` fns are covered (live_subscription.rs, live_subscription_full_bus.rs, risk_telemetry_subscription.rs). `stream_market_health` (:405) has 0 tests. The `BusRecipe::stream` dispatch and hash never execute. | **PROXY-only.** No test references `SubscriptionVariant::Bus`. | descriptor :1389 → arm cockpit_live.rs:2510 | v0.2's K3 "fully covered" exclusion no longer holds. Add a dispatch test and a count test. |
| 6 | **TrailMirrorRecipe** | private struct :573, impl :577 (`0xD0_05` :586); pub ctor `trail_mirror_subscription` :567; eager subscribe :592 | `live` | `trail_mirror_stream_impl` :609 | **PARTIAL.** Yield is covered. The close and None tests accept a timeout (trail_mirror_recipe_stream.rs:103, :132-136, :268). The "end_to_end" test calls the seam, not the Recipe (:225-234). The eager subscribe and the Lagged arm never execute. | **PROXY.** Descriptor pure fn only (trail_mirror_subscription_handle_gating.rs:83, :108). The bin-arm half of its probe (:22-33) can never fail. | descriptor :1391-1393 → arm :2521-2528 | Fix S1; add a real-batch S2. |
| 7 | **ActivityRecipe** | pub struct :714, impl :718 (`0xAC71`); eager subscribe :732 | `live` | `activity_stream_impl` :744 | **PARTIAL.** Yield is valid. The lag test subscribes after the sends (:153), so it never lags, and it asserts nothing (`let _ = got_late;` :180). The close test accepts a timeout (:220-228). The `live.rs` unit `activity_recipe_handles_lag` is valid per agent (not re-read). | **MISSING.** | always-on :1397 → arm :2549-2555 | Fix S1; add S2. |
| 8 | **ServerTimeRecipe (cockpit_live)** | private, cockpit_live.rs:127 / impl :133; hash TypeId only | `live` (bin `required-features` Cargo.toml:37-40) | `server_time_stream_impl` live.rs:804; guard :809 is load-bearing (interval construction) | **PARTIAL.** Yield is valid. Cadence is unpinned (only `t2 >= t1`). The K8 guard is unfalsifiable because all 4 tests are `#[tokio::test]`. T-ST-1d (server_time_recipe_stream.rs:178-206) duplicates T-ST-1a and never builds the Recipe. | **PROXY.** Descriptor only. The screen loop (:77-91) never passes the screen to the fn (live.rs:1383-1387 takes 3 bools). | always-on :1390 → arm :2511-2520 | Fix S1; real-batch S2; move into `ui::live` (C1). |
| 9 | **ToastDismissRecipe** | private, cockpit_live.rs:166 / impl :172 | `live` | `toast_dismiss_stream_impl` live.rs:848; guard :854 is load-bearing | **GOOD** for yield and cadence (paused time, exact counts). The K8 guard is unfalsifiable. The Recipe wrapper is untested. | **PROXY.** C4-T7 makes two identical calls (cockpit_subscription_toast_dismiss_always_batched.rs:138, :152). The screen loop at :88 is decorative. | always-on :1401 → arm :2573-2580 | Off-runtime test; real-batch S2; move into `ui::live` (C1). |
| 10 | **LabProgressRecipe** | lab/progress.rs:50 / impl :57; salt :64 | `live` | `stream_impl` :97 | **GOOD-WEAK.** The real `Recipe::stream` test (lab_progress_recipe_stream.rs:199-210) is valid. :83 and :116-120 accept a timeout. :151 asserts neither order nor count. The salt id is untested. The producer side is tautological (row 16). | **PARTIAL.** Lifecycle via lab_stop_button_gating.rs is valid. No inclusion test exists. | descriptor :1394-1396 → arm :2530-2547. rx/salt set :2350-2351, cleared :1942 and :1950. | Fix S1; add an identity test; real-batch S2. |
| 11 | **TrainingLogRecipe** | lab/training_log.rs:58 / impl :65; salt :72 | `live` | `stream_impl` :104 | **PARTIAL.** Yield is valid. The P2 `.take()` probe is fake: training_log_recipe_harness.rs:227-233 takes from its own closure. The Recipe is never called. | **PARTIAL.** State gating is valid but `#![cfg(unix)]` (training_log_state_gating.rs:55). No inclusion test. | :1398-1400 → arm :2557-2571 | Fix S1 and S2. **Likely production defect, unverified:** `tokio::task::spawn_blocking` (:115) runs inside a stream body that iced polls on a ThreadPool with no tokio context (Cargo.toml:145, no `tokio` feature). lab_runner_preload_callthrough_e2e.rs:254-293 proves exactly that topology panics. Route out of this story (see critic). |
| 12 | **TrainingPoller** | lab/training_subscription.rs:75 / impl :83; built only by `training_events_subscription` :57 | `live` | `training_poller_stream_impl` :153 | Tests exist (training_poller_subscription.rs), but D3-T2 is vacuous. | `none_run_id_returns_no_subscription` (:212-231) asserts nothing. | **NOT WIRED.** `scripts/callers.sh` finds 1 call site, its own unit test at :222. No variant exists; cockpit_live never builds it. | **K1 → analyst** (wire or retire). Do not invest tests in dead code. |
| 13 | **ServerTimeRecipe (cockpit.rs, fixtures bin)** | cockpit.rs:76 / impl :78; OS thread + blocking `rx.recv()` inside `unfold` (:91-110) | `fixtures` bin (Cargo.toml:17-20); built only on the Windows leg | none | none | none | always-on, cockpit.rs:349 | **K3 exclusion** (reasoned in Proposed tests §9). |
| 14 | **Modal-Esc `listen_with` ×2** | inline closures, cockpit_live.rs:2646-2656 and cockpit.rs:351-363 | live bin / fixtures bin | none (closure) | none | Consumer state only (tape_row_click_opens_modal.rs, per agent) | **OUTSIDE.** Gated on `tape_audit_modal.is_some()`. | Extract the mapping fn + add a variant (C1). |
| 15 | `widgets::focus_ring::subscription` | focus_ring.rs:106, returns `none()`; 0 callers | always | n/a | `subscription_none_is_inert` (:178) checks the type only | n/a | not batched | **K3**; optional 1-line pin. |
| 16 | `spawn_lab_run` Yahoo preload (LabProgress producer; ADR-0048 v0.1 Surface 1) | runner.rs:1253. The override branch :1405-1446 has no ticker and no cancel; the ticker/cancel `select!` lives only in the `#[cfg(feature="yahoo")]` branch (:1449-1640). | `live` | none for Yahoo. The Binance twin `run_binance_preload_arm` :1186 is the precedent. | **TAUTOLOGICAL.** spawn_lab_run_yahoo_harness.rs, lab_runner_ticker_e2e.rs and lab_runner_cancel_e2e.rs never call production. They import only `LabRunConfig` and a mock, and copy the loop (yahoo harness :153). | n/a | Production passes `None` (cockpit_live.rs:2380) | New seam `run_yahoo_preload_arm` (Wave F). |
| 17 | ActivityAuditAggregator (agent) | `run_aggregator_loop` crates/agent/src/activity_audit_aggregator.rs:167 | none | exists (v0.2 Wave B) | **PARTIAL.** Tests 1-2 only assert loop exit (select_arm_survival.rs:191-200, :256-265). Test 3 is valid (:321). | n/a | feeds row 7 | 1 test + docstring fix. The P-B1 text at :156-161 disagrees with the test file (:31-50). |
| 18 | Agent narration task (agent, producer for row 2) | inline `set.spawn` loop, runtime.rs:1627-1653 | none | none | none | n/a | n/a | Optional Wave G seam `narration_task_loop` (v0.2 Wave B shape). |
| 19 | Agent forward-plan supervisor send (producer for row 1) | inline `try_send` at runtime.rs:1474-1514, inside the supervisor | none | none | none. Every `RunHandles` in tests passes `plan_tx: None` (runtime.rs:3346; agent/tests/*:83, :106, :143). | n/a | n/a | **K3/K1 note.** Not additively extractable. |

### Proposed tests (AC2, AC4, AC5)

#### 0. Shared seam (Wave C1): how I resolved the agents' disagreement

The five agents proposed three incompatible seam shapes:
- Agent 1: `_with(.., ReturnChannels)` with enums.
- Agents 2-4: a `SubscriptionBatchInputs` of **7 `bool`s**, passed by reference in one proposal and by value in another.
- Agent 5: `subscription_for_variant(v, &ctx)`.

**The 7-bool struct fails CI.** The crate sets `#![warn(clippy::pedantic)]` (crates/ui/src/lib.rs:24). There is no clippy.toml. CI runs `cargo clippy --workspace -- -D warnings` (ci.yml:519). A struct with more than 3 bools trips `struct_excessive_bools`. The repo already takes the enum route for exactly this lint (state.rs:1282-1285).

**Decided shape.** Everything is additive in `crates/ui/src/live.rs`, which is live-gated at the module level:

```rust
pub enum SubscriptionVariant { /*existing 7*/ ForwardPlan, NarrationOutcome, BakeoffProgress, SweepProgress, ModalEsc } // appended
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)] pub enum Held { Yes, #[default] No }   // + From<bool>
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SubscriptionBatchInputs { pub trail: Held, pub lab_progress: Held, pub training_log: Held,
  pub forward_plan: Held, pub narration_outcome: Held, pub bakeoff_progress: Held, pub sweep_progress: Held, pub modal_esc: Held }
#[must_use] pub fn build_subscription_batch_descriptor_from(inputs: SubscriptionBatchInputs) -> SubscriptionBatchDescriptor // by value (Copy; avoids trivially_copy_pass_by_ref)
// existing 3-arg fn: SAME signature, body becomes `_from(SubscriptionBatchInputs { trail: has_trail.into(), .., ..Default::default() })`
pub struct SubscriptionCtx<'a> { pub rt_handle: &'a Handle, pub bus: &'a Arc<EventBus>, pub trail: Option<&'a TrailMirrorHandle>,
  pub lab_progress: Option<(&'a LabProgressCell, u64)>, pub training_log: Option<(&'a TrainingLogCell, u64)>,
  pub forward_plan: Option<&'a PlanCell>, pub narration_outcome: Option<&'a NarrationCell>,
  pub bakeoff_progress: Option<(&'a ProgressCell, u64)>, pub sweep_progress: Option<(&'a ProgressCell, u64)>, pub tape_audit_modal_open: bool }
impl SubscriptionCtx<'_> { #[must_use] pub fn inputs(&self) -> SubscriptionBatchInputs } // each flag = OUTER Option .is_some()
#[must_use] pub fn subscription_batch(ctx: &SubscriptionCtx<'_>) -> iced::Subscription<Message> // descriptor_from(ctx.inputs()) → exhaustive match (no `_ =>`) → batch
pub struct ServerTimeRecipe { pub rt_handle }  pub struct ToastDismissRecipe { pub rt_handle }   // MOVED from cockpit_live.rs:127-188
```

Two pieces live outside `live.rs`:
- `pub fn esc_closes_modal(event: iced::Event, _: iced::event::Status, _: iced::window::Id) -> Option<Message>` goes in `crates/ui/src/widgets/journal_transaction_modal.rs`. That file is always compiled, which matters because the fixtures bin is checked without `live` (ci.yml:546). `listen_with` takes a plain `fn` pointer (iced_futures-0.14.0/src/event.rs:26-28), so both bins can pass this fn directly.
- In the bin, `subscription()` shrinks to building a `SubscriptionCtx` from `self` fields and calling `subscription_batch`. The four tail pushes and the inline Esc closure (cockpit_live.rs:2583-2656) are deleted.

**Why this shape:**
1. Inclusion flags are derived from the same Option the arm consumes. This closes the "bin passes a literal `false`" residual that agents 1-4 had declared unclosable.
2. Tests call **the function production calls**. That closes the false claim at live.rs:1313-1315 ("If the conversion and the descriptor diverge, T-D-C2 / T-D-C4 catch it"); today no test executes an arm.
3. The 3-arg fn **must delegate** to `_from`. Otherwise the 8 existing call sites (server_time :80, :101; toast :89, :109, :138, :152; trail :83, :108) would pin a function production no longer calls, which is the bug-log #190 wiring form.

**Additivity.** No signature changes. The only non-additive edge is new variants on an exhaustive pub enum. Its only exhaustive matcher is in-crate, and no crate depends on `ui`. Production behaviour is unchanged:
- The same recipes are built under the same conditions, with the same hashes, except that the ServerTime/Toast TypeId path changes. iced's tracker keys by in-process hash, and order is irrelevant (iced_futures-0.14.0/src/subscription/tracker.rs:73-85, :126).
- The descriptor order appends `[ForwardPlan, NarrationOutcome, BakeoffProgress, SweepProgress, ModalEsc]` after `ToastDismiss`, which is today's push order.
- No file output, so D6 and the anchors are untouched.

**Introspection is available.** These are public in iced 0.14 and unused in the repo today:
- `iced::advanced::subscription::{into_recipes, Hasher}` (iced-0.14.0/src/advanced.rs:4-6)
- `Subscription::units()` (iced_futures-0.14.0/src/subscription.rs:404)

`batch` flattens recipes (:212-221). The "cannot be introspected" premise in the v0.2 docs (trail_mirror_subscription_handle_gating.rs:12-13) is false.

**Comment fixes in the same diff:**
- live.rs:1301-1315: false divergence claim.
- cockpit_live.rs:2583-2588: "not a descriptor-tested recipe".
- cockpit_live.rs:119-125: says the EnterGuard stays alive. live.rs:807-811 drops it before `Box::pin`.

**Residual.** The two progress cells share one type, so a swapped field in the bin's ctx literal compiles. Accept this and document it.

All new and extended files are `#![cfg(feature = "live")]` only, never `fixtures`. Each carries a T-T4 probe table in its module doc that cites **symbol + quoted line text**, not just the line number (line-number rot is listed in AC3). Timeouts are always `.expect(..)`, never an accepting `Err(_) => break`.

#### 1. ForwardPlanRecipe

| file | fn | S | asserts | falsification probe |
|---|---|---|---|---|
| forward_narration_relay.rs (extend) | `forward_plan_relay_yields_one_message_per_plan_in_order` | S1 | Send plans `v0.sma` then `v0.5.rsi`, drop tx, `timeout(2s, collect)`. Exactly 2 `ForwardPlanReceived` in send order, then the stream ends. | **AC4 designated:** live.rs:936 `while let Some(plan) = rx.recv().await {` → `if let …` (the existing :73 test stays green) |
| same (rewrite in place) | `forward_plan_relay_terminates_on_sender_drop` | S1 | Send 1 plan, then drop. Exactly 1 message, then `None` within 2s. | :936 → `while let Some(plan) = None::<agent::ForwardPlan> {` (today's version stays green, which is its vacuity) |
| same | `forward_plan_recipe_stream_takes_receiver_once_and_relays` | S1 | `Box::new(ForwardPlanRecipe{rt_handle: Handle::current(), rx: Arc::clone(&cell)}).stream(Box::pin(futures::stream::empty()))`. One plan gives `ForwardPlanReceived` with strategy and symbol carried, and `cell.lock().unwrap().is_none()` afterwards. A second recipe on the same Arc gives `None` within 1s. | live.rs:916 → `forward_plan_stream_impl(None)` |
| recipe_off_runtime_poll.rs (§6) | `forward_plan_recipe_drains_off_runtime` | S1 | See §6. | :936 → `while let Ok(Some(plan)) = tokio::time::timeout(Duration::from_secs(60), rx.recv()).await {` |
| subscription_batch_inclusion.rs (§5) | matrix + `forward_plan_stays_batched_after_receiver_taken` + `batched_forward_plan_recipe_relays` + identity tests | S2 | See §5. | §5 |
| advisor_recipe_state_gating.rs (NEW) | `relayed_plan_resolves_promotion_loading_to_ready` | S2 | `Cockpit::new()`, then `update(PromoteSweptConfig(..))` with params as built in promote_swept_config.rs. Precondition: `plan == PanelState::Loading` (state.rs:2808). Relay one plan through `forward_plan_stream_impl`, then `update`. Result is `Ready(v)` with `v.strategy == "v0.sma"`. | state.rs:3997 → `if !matches!(model.forward_plan_screen_state.plan, PanelState::Loading) { …set_plan(view); }` |
| same | `relayed_second_plan_replaces_the_first` | S2 | Relay A then B. Final state is `Ready(B)`. | :3997 guarded on `!matches!(…, PanelState::Ready(_))` |

#### 2. NarrationOutcomeRecipe

| file | fn | S | asserts | probe |
|---|---|---|---|---|
| forward_narration_relay.rs | `narration_outcome_stream_resolves_two_bakeoff_runs_through_one_receiver` | S1 | One stream for the whole test.<br>Run 1: Requested, Completed(Ok(`fake_bakeoff_report_mirror()`)), Explain → InFlight; send `Ready(p1)` → `Ready(p1)`.<br>Run 2: Requested → `NotRequested` (reset at leaderboard/state.rs:1260); Completed, Explain → InFlight; send `FellBack` → `FellBack`.<br>The message counter must equal 2. Then drop tx; `None` within 2s. | **AC4 designated:** live.rs:1002 `while`→`if` (run 2 gets `None`). A second probe, `futures::future::pending::<()>().await` after the loop, also fails it (final `None` times out). |
| same | `narration_outcome_recipe_stream_takes_receiver_once_and_relays` | S1 | Full `Recipe::stream`. `Ready("p")` arrives, then the cell is `None`. A second recipe on the same Arc gives `None` within 1s. | live.rs:988 → `narration_outcome_stream_impl(None)` |
| same (edit existing) | add `assert_eq!(…narration, NarrationState::InFlight)` before `update` in :150 and :186; prove NotRequested→InFlight via `BakeoffNarrationRequested` | S1 | The precondition is pinned rather than assumed. | drop `st.begin_narration()` at state.rs:3881 |
| recipe_off_runtime_poll.rs | `narration_outcome_recipe_drains_off_runtime` | S1 | Exactly `[Ready(p), FellBack]` in order. | :1002 wrapped in `tokio::time::timeout` |
| advisor_recipe_state_gating.rs | `explain_request_ignored_until_a_ready_result_is_on_screen` | S2 | Result `Empty` + Explain → `NotRequested`. Result Ready + Explain → `InFlight` (two-sided over the result dimension). | state.rs:3880 drop `matches!(st.result, PanelState::Ready(_)) &&` |
| same | `explain_rerequest_does_not_clobber_a_resolved_narration` | S2 | Apply a `Ready(p)` obtained **from** `narration_outcome_stream_impl` over a real channel. A second Explain leaves `Ready(p)`. | :3880 drop `&& !st.narration.is_requested()` |
| §5 | matrix + `batched_narration_outcome_recipe_relays` + identity | S2 | | |

#### 3. BakeoffProgressRecipe

| file | fn | S | asserts | probe |
|---|---|---|---|---|
| bakeoff_progress_relay.rs (extend) | `bakeoff_progress_relay_forwards_every_event_in_order_then_ends` | S1 | `try_send` done=0,1,2 (well under capacity 8), drop tx. Collected `done == [0,1,2]`, ids in send order, end via `Ok(None)`. A timeout panics. | live.rs:1107 body: insert `if progress.done == 0 { continue; }` (all 5 existing tests stay green) |
| same (edit) | `bakeoff_progress_relay_latest_event_wins_on_state` | S1 | `:145 Err(_) => break` → `panic!`. | stream never ends (`pending` after the loop) |
| same (edit) | `bakeoff_progress_relay_yields_message_and_populates_state` | S1 | Seed `progress = Some(stale)` **before** `begin_run()` so the :81-85 precondition stops being vacuous. | leaderboard/state.rs:1264 delete `self.progress = None;` |
| recipe_off_runtime_poll.rs | `bakeoff_progress_recipe_drains_off_runtime` | S1 | One `BakeoffProgress{done:4,total:7}`, then `None`, and the cell is `None`. | :1107 wrapped in `tokio::time::timeout` |
| advisor_recipe_state_gating.rs | `bakeoff_run_requested_clears_prior_runs_progress` | S2 | Run → progress 6/7 → Completed(Err). Seed a late stale progress. Requested → `running && progress.is_none()`. | leaderboard/state.rs:1264 delete clear |
| same | `bakeoff_progress_cleared_on_completion` | S2 | Requested, Progress{3,7} → done==3. Completed(Err) → `progress.is_none() && !running`. | :1284 delete clear |
| same | `second_bakeoff_request_mid_run_keeps_progress` | S2 | Mid-run Requested keeps progress done 3, `running`, and `Loading`. | state.rs:3851 replace the guard with an unconditional `begin_run()` |
| §5 | salt identity, collision, `batched_bakeoff_progress_recipe_relays` | S2 | | **AC4 designated:** live.rs:1073 delete `self.salt.hash(state);` |

#### 4. SweepProgressRecipe (NEW `crates/ui/tests/sweep_progress_relay.rs`)

| fn | S | asserts | probe |
|---|---|---|---|
| `sweep_progress_relay_yields_sweep_message_and_populates_tune_state` | S1 | `{done:3,total:12,current_id:"fast=15, slow=40"}` gives `Message::SweepProgress` with fields compared one by one (`BakeoffProgress` has no `PartialEq`, progress.rs:40). After `SweepRunRequested` + `update`, the tune progress equals the fed values **and** `leaderboard_screen_state.progress.is_none()` (cross-talk control). | **AC4 designated:** live.rs:1177 `yield Message::SweepProgress(progress);` → `yield Message::BakeoffProgress(progress);` |
| `sweep_progress_relay_forwards_every_event_in_order_then_ends` | S1 | 5 events done=0..4 (≤ 8, since `try_send` is lossy, progress.rs:72-76), drop. Exactly `[0,1,2,3,4]`, then `None`. | :1176 `while`→`if` |
| `sweep_progress_relay_none_receiver_yields_nothing` | S1 | `sweep_progress_stream_impl(None)` gives `None` within 1s. This is the negative half of the pair. | insert `if rx_opt.is_none() { futures::future::pending::<()>().await; }` at the top of the `stream!` body |
| `sweep_progress_recipe_stream_takes_receiver_once` | S1 | Real `Recipe::stream`, salt 1. done==5 arrives as `SweepProgress`; the cell is `None` afterwards. | live.rs:1161 → `bakeoff_progress_stream_impl(rx_opt)` (copy-paste sibling) |
| recipe_off_runtime_poll.rs `sweep_progress_recipe_drains_off_runtime` | S1 | as §6 | :1176 timeout wrap |
| advisor_recipe_state_gating.rs `sweep_lifecycle_running_and_progress_transitions` | S2 | For `Ok(fake_sweep_report_mirror())` and `Err("boom")`: Requested → running, Loading, None. 3× Progress → running stays and done == latest. Precondition `is_some()`. Completed → `!running`, `None`, Ready/Error. | tune/screen_state.rs:814 delete clear |
| same `sweep_rerun_clears_late_stale_progress` | S2 | A late Progress after Completed keeps `!running` (the bar is hidden, screens/tune.rs:628-630 per agent). Then Requested gives `running && progress.is_none()`. | screen_state.rs:800 delete clear |
| same `sweep_double_run_request_mid_run_keeps_progress` | S2 | A mid-run Requested keeps done==4 and `running`. | state.rs:3964 `if model.tune_screen_state.can_run() {` → `if true {` |

#### 5. Shared S2 file: NEW `crates/ui/tests/subscription_batch_inclusion.rs` (Wave C1/C2)

This file folds in and **replaces** cockpit_subscription_{server_time,toast_dismiss}_always_batched.rs and trail_mirror_subscription_handle_gating.rs. That is a net K4 change of +1 test binary, not +4.

| fn | asserts | probe(s) |
|---|---|---|
| `legacy_three_arg_descriptor_is_pinned_to_its_literal_order` | For all 8 `(a,b,c)`, compare against an **explicit literal** Vec `[Bus, ServerTime, (Trail), (LabProgress), Activity, (TrainingLog), ToastDismiss]` and check that none of the 5 new variants is present. Agents 1 and 4 proposed `== _from(default)` instead; that is self-referential once the wrapper delegates (the bug-log #161/#187 form). | wrapper sets `forward_plan: Held::Yes` |
| `descriptor_membership_keys_each_variant_on_its_own_input` | Over all 2^8 `SubscriptionBatchInputs`: each optional variant is present iff its own field is `Yes`. The 4 always-on variants are always present. Each variant appears at most once. Order equals the canonical literal order filtered. This replaces the decorative per-`Screen` loops. | (a) delete the `SweepProgress` push → red; (b) push it unconditionally → red; (c) key the `BakeoffProgress` push on `inputs.sweep_progress` → red; (d) `if has_lab_progress {` → `if true {` in the legacy body (live.rs:1394) → red |
| `ctx_inputs_key_on_outer_option_not_inner_receiver` | Cells whose inner Option was already `take()`-n still give `Held::Yes` for forward_plan and narration. This guards the "missed frame drops the only Receiver" hazard (tracker.rs:126 `retain`). | `inputs()`: `forward_plan` → `.filter(\|c\| c.lock().map(\|g\| g.is_some()).unwrap_or(false))` |
| `subscription_batch_units_match_descriptor` | For ctx = all-None, each single-Some (8 cases), and all-Some: `subscription_batch(&ctx).units() == 11 + (descriptor_from(ctx.inputs()).len() - 1)`. Bus expands to 11, derived from the 11 `Channel` variants listed literally in the test. This is the descriptor⇔conversion agreement no test has today. | the `SweepProgress` arm in `subscription_batch` → `iced::Subscription::none()`; live.rs:77 delete the MarketHealth line |
| `batched_<recipe>_recipe_relays`: one each for forward_plan, narration_outcome, bakeoff_progress (salt 7), sweep_progress (salt 7), lab_progress, training_log, trail, activity | `into_recipes(subscription_batch(&ctx))`. Find the unit whose id equals a reference recipe's id (same salt). `.stream(empty)`. Feed one event. The expected `Message` arrives within 2s. A `None` ctx field gives no unit with that id. This proves the batched unit is **live**, not merely present. | arm builds with `salt: 0` (id not found); arm builds the sibling recipe type |
| `forward_plan_stays_batched_after_receiver_taken` | After the first batch's ForwardPlan unit was streamed (inner cell `None`), the next `subscription_batch(&ctx)` still contains the same id. | the probe from `ctx_inputs_key_on_outer_option…` |
| `recipe_ids_are_frame_stable` | Two instances (Arc clones) of each static-id recipe give equal ids: Bus×11 via `into_recipes(ui::live::subscription(bus))` twice, Trail via `trail_mirror_subscription`, Activity, ServerTime, Toast, ForwardPlan, Narration. Same for salted recipes at equal salt. Ids are computed as `let mut h = Hasher::default(); Recipe::hash(&*r, &mut h); std::hash::Hasher::finish(&h)` (the same computation as tracker.rs:74-79). | live.rs:902 append a deterministic per-instance `static AtomicU64` counter `.hash(state)`; live.rs:1073 add `std::ptr::from_ref(self).hash(state)` |
| `salted_recipe_ids_change_with_salt` | For LabProgress, TrainingLog, Bakeoff, Sweep: salt 1 ≠ salt 2. | delete progress.rs:64, training_log.rs:72, live.rs:1073, live.rs:1148 (one probe each) |
| `recipe_ids_are_pairwise_distinct` | All ids distinct: 11 Bus channels + 10 others, salted at 1. Both progress salts start at 0 and bump to 1 (cockpit_live.rs:818, :825, :2414, :2455), which is where a collision would bite. | live.rs:1147 `TypeId::of::<Self>()` → `TypeId::of::<BakeoffProgressRecipe>()`; delete live.rs:126 (Bus channel hash) |
| `escape_keypress_maps_to_modal_closed` / `non_escape_key_maps_to_none` | Pure `esc_closes_modal` with `iced::window::Id::unique()`. Escape gives `TapeAuditModalClosed`; Enter and a non-keyboard event give `None`. | `Named::Escape` → `Named::Enter` |

#### 6. NEW `crates/ui/tests/recipe_off_runtime_poll.rs` (executor-fidelity S1; Wave B)

This file closes the blind spot shared by every existing recipe test: all of them run under `#[tokio::test]`, while production polls on iced's ThreadPool executor, which has no tokio context.

**Pattern** (precedent lab_runner_preload_callthrough_e2e.rs:176-293):
- A plain `#[test]` builds its own multi-thread runtime and never enters it on the harness thread.
- The recipe is built **and** `.stream(empty)`-ed **and** drained with `futures::executor::block_on` on a fresh `std::thread`.
- The producer uses `try_send`.
- The result comes back over std mpsc with `recv_timeout(5s)`. A panic in the thread shows up as `Disconnected`, so the test fails instead of hanging.

| fn | asserts | probe |
|---|---|---|
| `harness_thread_has_no_tokio_context` | `catch_unwind(tokio::runtime::Handle::current).is_err()` on a fresh thread. This negative control establishes the premise. | n/a (premise check) |
| `forward_plan_…`, `narration_outcome_…`, `bakeoff_progress_…`, `sweep_progress_…`, `lab_progress_…` (progress then `LabRunProgressDone`), `trail_mirror_…`, `activity_…` `_recipe_drains_off_runtime` | The exact message list, then `None`. For take-recipes the cell is `None` afterwards. | wrap that recipe's `rx.recv()` in `tokio::time::timeout(..)`. It panics off-runtime ("no reactor running", unverified by execution) while every `#[tokio::test]` sibling stays green. |
| `server_time_stream_ticks_off_runtime` | `server_time_stream_impl(&handle)` called on the bare thread gives the first `ServerTimeTick` within 3s. | **AC4 designated:** delete live.rs:809 `let _guard = rt_handle.enter();`. This makes the K8 claim in server_time_recipe_stream.rs falsifiable for the first time. |
| `toast_dismiss_stream_ticks_off_runtime` | First `ToastTick` within 2s. | **AC4 designated:** delete live.rs:854 |
| ~~`training_log_recipe_drains_off_runtime`~~ | **Expected RED** today (training_log.rs:115). Wave 0 confirms it with a throwaway copy. Do NOT land it in 2-63 as a gate; route it (critic). | n/a |

#### 7. v0.2 recipes (AC1 re-derivation + AC3)

| recipe | file | fn | S | asserts | probe |
|---|---|---|---|---|---|
| Bus | live_subscription_full_bus.rs (extend, no new binary) | `bus_recipe_stream_routes_each_channel_to_its_message` | S1 | Stream each of the 11 recipes from `into_recipes(ui::live::subscription(bus))`. Publish one event per channel. The multiset of yielded variants equals the 11 expected (incl. `MarketHealthUpdated`, `RiskStateRefreshed`). The publish API for MarketHealth/strategy channels is unverified. | **AC4 designated:** live.rs:141 `Channel::Pnl => Box::pin(stream_pnl(&bus))` → `stream_ticks` |
| Bus | same | `market_health_stream_yields_update_then_ends` | S1 | First item is `MarketHealthUpdated`; strict `Ok(None)` after bus drop (assumes `stream_market_health` breaks on Closed, live.rs:405-423 per agent). | live.rs:412 suppress the yield |
| Trail | trail_mirror_recipe_stream.rs (rewrite T-TM-1d) | `trail_mirror_recipe_stream_end_to_end` | S1 | `into_recipes(trail_mirror_subscription(handle))`: len 1. Call `.stream(empty)`. `tick_tx.send(TrailUpdated(id))` **before** the first poll; the id arrives. Drop all senders; strict `Ok(None)`. | **AC4 designated:** live.rs:592 subscribe to a fresh `broadcast::channel(16).1` |
| Trail | same | `trail_mirror_stream_continues_after_lag` | S1 | Capacity 2, subscribe first, send 4, then a marker. The marker arrives. | Lagged arm (live.rs:616-620) → `break` |
| Trail | same (edit T-TM-1a/1b/1c) | strict `Ok(None)` on close; T-TM-1c `assert_eq!(received_ids, ids)`; add the pair `trail_mirror_stream_stays_open_while_sender_alive` (expects `Err(Elapsed)`) | S1 | | :624 `break` → `std::future::pending::<()>().await` |
| Activity | activity_recipe_stream.rs | `activity_recipe_stream_subscribes_eagerly` | S1 | `Box::new(ActivityRecipe{bus}).stream(empty)`, then `bus.activity().start(..)` before the first poll. Start arrives within 2s. | live.rs:732 subscribe moved into the stream body |
| Activity | same (replace :141-181) | `stream_survives_lagged_and_delivers_retained_events` | S1 | rx created first (capacity 2); send 1..=4 → ids `[3,4]`; then 99 arrives. | **AC4 designated:** Lagged arm (live.rs:751-759) → `break` |
| Activity | same (replace :193-230) | `stream_terminates_on_sender_close` (strict) + `stream_yields_buffered_event_then_ends_on_close` | S1 | strict `Ok(None)` within 500 ms; buffered id 7, then `None` | Closed `break` (live.rs:760-763) → `pending().await`. Do NOT use `continue` as the probe: recv on a closed channel is Ready immediately, so the loop spins and the test hangs. |
| ServerTime | server_time_recipe_stream.rs (replace T-ST-1d) | `server_time_recipe_stream_yields_tick` | S1 | `Box::new(ui::live::ServerTimeRecipe{rt_handle}).stream(empty)` gives a tick within 1.5s. Needs C1. | moved `stream()` → `Box::pin(futures::stream::empty())` |
| ServerTime | same | `server_time_period_is_one_second` | S1 | `start_paused`. After the skip tick, `advance(999ms)` gives `now_or_never() == None`, and `advance(1ms)` gives exactly 1. Two-sided on the period. | live.rs:810 `from_secs(1)` → `from_millis(100)` |
| Toast | toast_dismiss_recipe_stream.rs | `toast_dismiss_recipe_stream_yields_tick` (needs C1); tighten C3-T3 (:248-251) to require the 4th tick | S1 | | moved `stream()` → empty |
| LabProgress | lab_progress_recipe_stream.rs | `lab_progress_second_stream_on_same_cell_ends_immediately`; fix :83 and :116-120 to `matches!(…, Ok(None))`; fix :125-162 to `assert_eq!(received_bars, vec![128,256,384])` with `None => panic!` | S1 | | **AC4 designated:** progress.rs:64 delete the salt hash (via §5 `salted_recipe_ids_change_with_salt`); progress.rs:107 suppress `LabRunProgressDone` (the None arm panics) |
| TrainingLog | training_log_recipe_harness.rs | `training_log_recipe_stream_takes_receiver_once` (real `TrainingLogRecipe`, pub fields training_log.rs:58-63, under `#[tokio::test]`): the cell is `None` right after `stream()`; 3 lines then `None`; salt-2 recipe on the same Arc gives `Ok(None)` within 200 ms | S1 | | **AC4 designated:** replace training_log.rs:83-88 with `let rx_opt: Option<std::sync::mpsc::Receiver<TrainingLogLine>> = None;` (a real P2, unlike the current fake) |
| Modal-Esc | §5 + matrix (`ModalEsc` iff `tape_audit_modal_open`) | | S1/S2 | | `Named::Escape` → `Named::Enter` |

#### 8. Producer loops

| recipe fed | file | fn | asserts | probe |
|---|---|---|---|---|
| LabProgress (Wave F) | spawn_lab_run_yahoo_harness.rs (rewrite) + seam | `yahoo_arm_emits_sentinel_before_preload_resolves` | New API-additive seam `#[cfg(feature="live")] pub async fn run_yahoo_preload_arm(rt, source: Box<dyn LabYahooBarSource>, cfg, &mut scenario_cfg, &progress_tx, &cancel, activity: Option<&ActivitySender>) -> Result<(), SmolStr>`, extracted from runner.rs:1479-1640. Both the override branch and the yahoo branch call it, which removes the ticker-less divergent mock branch (:1405-1446). Production passes `None` (cockpit_live.rs:2380), so the shipped path is behaviour-identical. The `ActivityKind` import at runner.rs:48-50 becomes live-only. The mock returns non-empty bars (spawn_lab_run_binance_harness.rs precedent). First event `Progress{0,1,0}` within 50 ms; `Ok`; `bars_override` len N; `calls == 1`. | **AC4 designated:** move the sentinel `try_send` after the `select!` loop |
| | same | `yahoo_arm_ticker_emits_during_preload_with_increasing_elapsed` | 1000 ms mock: ≥ 3 events with `elapsed_ms` > 0, strictly increasing | delete the `_ = ticker.tick() =>` arm (runner.rs:1598) |
| | same | `yahoo_arm_ticker_stops_after_preload_resolves` | sentinel seen; after return, a marker `Progress{7,7,7}` is the next item, and nothing else arrives within 600 ms | keep ticking in a detached task |
| | same | `yahoo_arm_empty_bars_returns_no_data_notice` | `Err`, classified as Notice; `bars_override` stays `None` | drop `classify_preload_result` |
| | lab_runner_ticker_e2e.rs (rewrite) | `ticker_fires_at_least_3_times_without_caller_reactor` | plain `#[test]`, `block_on(run_yahoo_preload_arm(..))` on a thread without a reactor; ≥ 3 increasing events | delete runner.rs:1500 `let _guard = rt.enter();` |
| | lab_runner_cancel_e2e.rs (rewrite) | `stop_during_preload_returns_cancelled_within_500ms`, `stop_during_preload_aborts_preload_task` (mock drop-guard sets `dropped`), `cancel_before_preload_start_is_instant` (`calls == 1`) | the Err text equals the production literal; `dropped == true` within 200 ms | delete the `cancel.cancelled()` arm (:1588); delete `fetch_join.abort();` (:1589) |
| Activity (agent) | crates/agent/tests/activity_audit_aggregator_select_arm_survival.rs | `recv_arm_counts_ticks_after_n_idle_windows` | 5 idle windows, then 3 ticks, then `advance(100ms)`: a Start whose count reflects 3. Also fix the source docstring (activity_audit_aggregator.rs:156-161) to match the `pending` probe. | comment out the recv-arm `fetch_add` |
| Narration (Wave G, optional) | NEW crates/agent/tests/narration_task_loop.rs + seam `pub async fn narration_task_loop(req_rx, outcome_tx, llm_provider, cancel)`, extracted from runtime.rs:1627-1653 | `narration_task_loop_answers_every_request`, `_exits_on_cancel_within_bound`, `_exits_when_request_channel_closes` | 2 requests give 2 `FellBack`; cancel makes the join complete within 1s; dropping the sender makes the join complete within 1s | add `break;` after the send block; delete the `narration_cancel.cancelled() => break` arm (:1632); let-else `break` → `match std::future::pending::<std::convert::Infallible>().await {}` |

#### 9. K3 exclusions (reasoned)

1. **The `rt_handle.enter()` guard around `take()`** in ForwardPlan/Narration/Bakeoff/Sweep/LabProgress/TrainingLog `stream()` (e.g. live.rs:910-915). A std `Mutex` plus `Option::take` needs no tokio context, so removing the guard is unobservable. The real hazard is covered by §6. By contrast, the guards inside `server_time_stream_impl`/`toast_dismiss_stream_impl` **are** load-bearing and are tested.
2. **Static discriminants** `0xF6F6`/`0xF9F9`/`0xD005`/`0xAC71`. `TypeId` already separates the types, so a single-constant probe changes nothing. They are covered collectively by `recipe_ids_are_pairwise_distinct`.
3. **Mutex-poison recovery.** Production code never locks these cells anywhere except inside `stream()`.
4. **cockpit.rs fixtures `ServerTimeRecipe`.**
   - It is a demo bin, not the shipped product (Cargo.toml:17-20).
   - CI builds it only on the Windows leg (ci.yml:791).
   - Its blocking `recv` is a documented, accepted design (cockpit.rs:104-106).
   - Record the design smell; no test.
5. **`focus_ring::subscription`.** Inert and unwired. Optional: replace the type-only test at :178 with `assert!(into_recipes(subscription()).is_empty())`.
6. **Bin-side receiver lifecycle**: salt bumps and rx set/clear at cockpit_live.rs:2350-2351, 2412-2417, 2453-2458, 1942, 1950, 1965, 1974, plus the ctx field literal.
   - `AppState` cannot be built from tests (bus, ledger, kill switch). The in-bin tests deliberately avoid it (:2688+).
   - The pure-state lifecycle tests on the same messages act as a proxy.
   - The residual goes to cockpit-smoke (orchestrator-only).
7. **The agent forward-plan `try_send`** (runtime.rs:1474-1514) is inline in the supervisor and has no additive extraction.
8. **Backtest emitters** are outside crate scope: bakeoff/mod.rs:1236-1242 is covered by `t_prog_1`; sweep.rs:979-985 is untested repo-wide, per agent.
9. **Pixels** (ADR-0048 D4) and the lossy capacity-8 policy.

### Non-vacuity audit (AC3)

Every `#![cfg(feature = "live")]` gate is a **legitimate compile gate**, not a skip-as-pass: `live` is a default feature, and both CI test legs run with defaults (ci.yml:781, :791). Only one file has a #66-shape cwd-relative skip.

| file | tests | skip paths (legit?) | cwd-relative paths | swallowed Errs | proves execution | verdict | fixes |
|---|---|---|---|---|---|---|---|
| ui/tests/cockpit_live_lab_run_smoke.rs | 5 | **:400-419 eprintln + `return` when the parquet is missing. NOT legit:** data/yahoo/ is gitignored (only REVISION.toml is tracked), so it skips on every CI run | :402, :408, :429-433 | :418 `return` | no (for `smoke_yahoo_cache_hit`) | **fix-needed (HIGH, exact #66 shape)** | Root at `Path::new(env!("CARGO_MANIFEST_DIR")).join("../../data/yahoo-sample")`; 12 BTC-USD 2024 parquets are tracked. Delete :400-419 and :428-434; keep `.expect` at :443. Assert `bars.len() >= 170` and `progress_events.iter().any(\|p\| p.current_bar > 0)`. Add `#[cfg(feature="yahoo")]` to that test. Fix doc drift: :32/:391 say `#[ignore]` but none exists. Rename `smoke_empty_selection_no_engine_call` and drop its tautological gate assert (:561-566). The sample's 95% coverage for H1_2024 is unverified. |
| ui/tests/spawn_lab_run_yahoo_harness.rs | 3 | live gate (legit) | — | :161, :227, :318 `let _ = mock.preload(..)` | **no.** It never calls production; it sends the sentinel itself (:153-159) | **fix-needed (HIGH, tautology)** | Wave F seam + 4 tests (§8) |
| ui/tests/lab_runner_ticker_e2e.rs | 1 | live (legit) | — | — | **no.** Loop copied inline (:90-110); imports only `LabRunConfig` | **fix-needed (HIGH)** | Wave F rewrite (§8); stale `runner.rs:735-739` cite |
| ui/tests/lab_runner_cancel_e2e.rs | 2 | live (legit) | — | — | **no.** Asserts the test's own literal | **fix-needed (HIGH)** | Wave F rewrite (§8) |
| ui/tests/activity_recipe_stream.rs | 3 | live (legit) | — | :173 `Ok(None)\|Err(_) => break`; :180 `let _ = got_late;`; :220, :223-228 timeout accepted | test 1 only | **fix-needed** | §7 rewrites; probe cite "approx. live.rs:726" → the yield at :750 |
| ui/tests/trail_mirror_recipe_stream.rs | 4 | live (legit) | — | :103, :132-136, :268 `Err(_) => {}`; :168/:175 `None => break` + prefix check | yield only | **fix-needed** | §7; T-TM-1d is mislabeled (:225-234) |
| ui/tests/lab_progress_recipe_stream.rs | 4 | live (legit) | — | :83, :116-120 timeout accepted; :151 `None => break` | partly (the :199 real-Recipe test is valid) | **fix-needed** | §7; add a T-T4 line (none exists) |
| ui/tests/server_time_recipe_stream.rs | 4 | live (legit) | — | :154 (acceptable: ticks already asserted) | yes, for yield | **fix-needed** | T-ST-1d duplicates T-ST-1a; the K8 claim is unfalsifiable; cadence unpinned → §6/§7 |
| ui/tests/training_log_recipe_harness.rs | 3 | live (legit) | — | — | yes, for yield; **P2 fake** (:227-233) | **fix-needed** | §7; P1 cite :124 → :125; the probe table names a nonexistent `stream_yields_lines_in_order` (:42) |
| ui/tests/training_log_state_gating.rs | 3 | live (legit); **`#![cfg(unix)]` :55, platform-scoped.** It runs on the canonical macOS leg but yields 0 tests on Windows for a platform-neutral subject (state.rs arms). Not #66-class. | — | — | yes | **fix-needed (low)** | Port off `cfg(unix)` using the `cfg!(unix)` `sleep`/`timeout` precedent (cockpit_training_pressed_wiring.rs:91-95), or document it. Stale cites state.rs:2232/2225 → 3631/3624 (per agent). |
| ui/tests/training_poller_subscription.rs | 3 | live (legit) | — | :219-220 `Ok(None)\|Err(_) => break` with no positive assert | D3-T1/T3 yes; **D3-T2 no** | **K1-route** (the subject is unwired) | If kept: phase-2 rewind positive control; `units()` 0/1 pair; remove the header's "S1 + S2" claim |
| ui/tests/cockpit_audit_aggregator_boot.rs | 2 | none | — | :125 `let _ = timeout(..)`; :144-148 `is_ok()` checks only the timeout layer; :151-155 `is_err()` also accepts Closed/Lagged | test 1 yes | **fix-needed (low)** | `.expect(..).expect(..)` on the join; `matches!(…, Err(TryRecvError::Empty))`; optional End-on-close test (End emission unverified) |
| ui/tests/bakeoff_progress_relay.rs | 5 | live (legit) | — | :145 `Err(_) => break` | yes | **clean, with tightenings** | :145 → `panic!`; make the :80-85 precondition non-vacuous; add a T-T4 line; §3 |
| ui/tests/forward_narration_relay.rs | 7 | live (legit) | — | — | yes (positives); :121 and :134 valid only as pair halves | **clean, with tightenings** | Strengthen :121 (§1); preconditions (§2); add a T-T4 table (none at :1-23); `assert!(!facts.candidate_kpi_strings.is_empty())` before the :291 loop |
| ui/tests/cockpit_subscription_toast_dismiss_always_batched.rs | 3 | live (legit) | — | — | mechanically yes; the per-screen dimension is decorative (:88), and C4-T7 repeats a call (:138/:152) | **superseded** | Fold into §5 and delete |
| ui/tests/cockpit_subscription_server_time_always_batched.rs | 2 | live (legit) | — | — | same; loop at :77-91 | **superseded** | Fold into §5 and delete |
| ui/tests/trail_mirror_subscription_handle_gating.rs | 2 | live (legit) | — | — | yes, for the descriptor; the bin-arm probe (:22-33) cannot fail | **superseded** | Fold into §5 and delete |
| ui/tests/toast_dismiss_recipe_stream.rs | 3 | live (legit) | — | :248-251 accepts `Empty` (non-fatal) | yes (exact counts, paused time) | **clean** | Optional: require the 4th tick. Its probe cites by symbol (:32-42), which is the model for AC4. |
| ui/tests/lab_stop_button_gating.rs | 4 | none | Production `default_cache_root()` is `PathBuf::from("data/yahoo")` (lab/cache_state.rs:109-111), reached via state.rs:3475/3562; not asserted on | — | yes | **clean** | Stale doc refs (per agent: screens/lab.rs:419→486, state.rs:2179→3578); log the production cwd path separately |
| ui/tests/live_subscription.rs | 6 | live (legit) | — | — | yes | **clean** | Optional: Closed-arm test (`TapeError` then `None`) |
| ui/tests/live_subscription_full_bus.rs | 2 | live (legit) | — | — | yes | **clean** | Host the §7 Bus dispatch test; optionally tighten `_ => {}` (:207-223) (per agent) |
| ui/tests/risk_telemetry_subscription.rs | 1 | live (legit) | — | — | yes | **not-a-harness-file (clean)** | Covered by §5/§7 |
| ui/tests/activity_tape_event_storm.rs | 1 | live (legit) | — | — | perf of a raw broadcast; imports no `ui` symbol | **not-a-harness-file** | Run it alone (load-sensitive); optional routing through `activity_stream_impl` |
| ui/tests/headless_emulator_smoke.rs | 4 | none | — (loaders are CARGO_MANIFEST_DIR-rooted, per agent) | Ready-wait falls through (:96-113 etc.); degraded loads accepted | renders, yes; "subscription pump" unproven | **not-a-harness-file** | Optional `drain_until_ready` helper and Ready asserts |
| ui/tests/cockpit_training_pressed_wiring.rs (not audited by the agents; spot-check) | 5 | live (legit) | `/dev/null`, `/tmp` are absolute | none found by grep | It tests a **test-local copy** of `AppState::update` (`TrainingSimState`, :39-53), so edits to cockpit_live.rs:1883-1931 are invisible | **fix-needed (document)** | Same residual as K3 #6. Rename and document; no additive seam. |
| ui/tests/spawn_lab_run_binance_harness.rs (spot-check) | 3 | live (legit) | — | none found | yes: drives production `run_binance_preload_arm` (:9, :40) | **clean** | The precedent for Wave F |
| ui/tests/lab_runner_preload_callthrough_e2e.rs (spot-check) | 2 | live (legit) | — | `let _ =` at :282 is intentional (inside `catch_unwind`) | yes (has a negative control :254-293) | **clean** | Pattern source for §6 |
| ui/tests/promote_swept_config.rs (adjacent) | 3 | **`#![cfg(feature="fixtures")]` :22 is unnecessary:** `ui::fixtures` is always compiled (lib.rs:116-120), so the gate gives 0 tests on the macOS leg | — | — | yes (on Windows only) | **fix-needed (low)** | Drop the gate |
| agent/tests/activity_audit_aggregator_select_arm_survival.rs (spot-check) | 3 | none | — | :312 `let _ = timeout(..)` (followed by a real assert) | tests 1-2 only exit-on-close; test 3 yes | **fix-needed (low)** | §8 + docstring |
| ui/src/lab/training_subscription.rs `mod tests` | — | — | — | — | `none_run_id_returns_no_subscription` asserts nothing (:228-230 "can't inspect", which is false) | **K1** (with the poller) | `units() == 0` if kept |
| ui/src/widgets/focus_ring.rs:178 | 1 | — | — | — | type-only | **K3** | Optional `into_recipes` empty pin |

### Wave plan

The waves mirror v0.2's A-D shape with the same rules:
- One dev agent owns `crates/ui` per wave, and never two agents in one crate.
- One tester verdict per wave, recording FAIL → restore → PASS for each designated probe.
- Cargo is scoped: prefix `SDKROOT=/Library/Developer/CommandLineTools/SDKs/MacOSX26.sdk`, run `cargo test -p ui --test <files>`, and finish with `cargo clippy -p ui --tests -- -D warnings`. CI clippy does not lint tests (ci.yml:519 has no `--tests`).
- Any job over 2 minutes ships a `watch` block.
- The final full `cargo test -p ui` runs alone.

| wave | scope | files | production code? | depends on | designated probes |
|---|---|---|---|---|---|
| **0** | Baselines and confirmation; nothing committed | none. Measure the K4 baseline: wall time of `cargo test -p ui --no-run` and the test-binary count, same box, warm target. Run `scripts/verify_anchors.sh` (expect 119/119) and `python3 scripts/spec_lint.py`. Use a throwaway off-runtime TrainingLog test to confirm or refute the training_log.rs:115 defect, then delete it. | no | — | the TrainingLog result goes to the orchestrator for a bug-log entry |
| **A** | Post-v0.2 S1, test-only | forward_narration_relay.rs, bakeoff_progress_relay.rs (extend); NEW sweep_progress_relay.rs (§1-§4 S1 rows) | no | — | live.rs:936; :1002; :1107 skip; :1177 variant swap; :1161 delegate swap |
| **B** | Executor fidelity, test-only | NEW recipe_off_runtime_poll.rs (§6, without TrainingLog) | no | — (uses existing pub seams) | live.rs:809, :854 guard deletion; one timeout-wrap per recipe |
| **C1** | **Shared seam** + descriptor S2 | live.rs (variants, `Held`, `SubscriptionBatchInputs`, `_from`, wrapper delegation, `SubscriptionCtx`, `subscription_batch`, moved ServerTime/Toast recipes, comment fixes); journal_transaction_modal.rs `esc_closes_modal`; cockpit_live.rs `subscription()` rewired (delete tail pushes + bin-private recipes); cockpit.rs uses `esc_closes_modal`; NEW subscription_batch_inclusion.rs (legacy pin, matrix, ctx-inputs, units agreement, modal fn tests); delete the 3 superseded v0.2 descriptor files | **YES**, API-additive (Wave C shape) | — | matrix probes (a)-(d); Sweep arm → `none()`; wrapper `forward_plan: Yes`; `inputs()` inner-filter |
| **C2** | Real-batch S2 + identity, test-only | subscription_batch_inclusion.rs: `batched_*_recipe_relays` ×8, `forward_plan_stays_batched…`, three identity tests | no | C1 | live.rs:1073, :1148, progress.rs:64, training_log.rs:72 salt deletions; :1147 TypeId swap; :126 Bus channel hash; :902 per-instance counter |
| **D** | State-machine S2, test-only | NEW advisor_recipe_state_gating.rs (§1-§4 S2 rows); drop the `fixtures` gate on promote_swept_config.rs | no | — | state.rs:3997, :3880 (×2), :3851, :3964; leaderboard/state.rs:1264, :1284; tune/screen_state.rs:800, :814 |
| **E1** | AC3 sweep of v0.2 recipe-stream files, test-only | activity_recipe_stream.rs, trail_mirror_recipe_stream.rs, lab_progress_recipe_stream.rs, server_time_recipe_stream.rs, toast_dismiss_recipe_stream.rs, training_log_recipe_harness.rs, live_subscription_full_bus.rs (Bus dispatch); probe-citation rot → symbol + quoted line | no | C1 (moved ServerTime/Toast recipes) | live.rs:592; :751-759; :141; :810; training_log.rs:83-88 take-removal; progress.rs:107; **B-P41** (Wave B's measured survivor: `toast_dismiss_stream_impl` `loop {` → `for _ in 0..4 {`; needs a paused-clock toast liveness test) |
| **E2** | AC3 sweep of the remaining files, test-only (+1 agent test) | cockpit_live_lab_run_smoke.rs (#66 fix), cockpit_audit_aggregator_boot.rs, training_log_state_gating.rs, forward_narration_relay.rs leftovers, cockpit_training_pressed_wiring.rs doc, agent select_arm_survival.rs (+ source docstring) | docstring only (agent) | — | smoke: point at the sample corpus and confirm the test now **runs** (non-zero bars; no SKIPPED line in `--nocapture`); aggregator `fetch_add` |
| **F** | Surface 1 de-tautologization | runner.rs `run_yahoo_preload_arm` extraction + rewrites of spawn_lab_run_yahoo_harness.rs, lab_runner_ticker_e2e.rs, lab_runner_cancel_e2e.rs | **YES**, extraction (the Binance-arm precedent). The test-only override branch gains a ticker and cancel; the shipped path is identical (passes `None`, cockpit_live.rs:2380). | — (runner.rs only) | sentinel moved after the loop; delete the ticker arm; delete runner.rs:1500 guard; delete the cancel arm; delete `abort()` |
| **G** (optional) | Agent narration producer | agent runtime.rs `narration_task_loop` extraction + NEW agent test | **YES** (agent crate) | — | §8 |
| **Close** | Artifacts and gates | ADR-0048 § Changelog v0.3.0 row (+ correct the D4 bullet, see critic); story Dev Agent Record (this inventory, corrected lines); trace/CHANGELOG triad at `done`; anchors 119/119 **after**; spec_lint; fmt; K4 re-measure (≤ 10% vs Wave 0); full `cargo test -p ui` alone; read CI once after push | — | all | — |

Waves A, B and D touch no production code and are independent; they may run before C1 but must be tested serially. C1 is the single risky shared edit, and its owner holds live.rs:1300-1403 plus cockpit_live.rs:2487-2659 exclusively. Net new ui test binaries: +4 new − 3 deleted = **+1**, plus 1 optional agent binary.

### Completeness critic

**Coverage of the recipe set.** All 13 `impl Recipe` are inventoried. Extras found that no story AC names:
- the two `listen_with` Esc listeners
- `focus_ring`
- `spawn_lab_run`
- three agent producer loops

**Recipes no agent covered:** none. The backtest-side emitters (bakeoff/mod.rs:1236-1242, sweep.rs:979-985) are outside the ui/agent scope, but the sweep emitter is untested repo-wide (every caller passes `SweepProgressSender::disabled()`, per agent) and should be routed as a backtest finding.

**Production defects found. Do not fix them inside 2-63:** AC5 forbids production behaviour change beyond additive seams.
1. **TrainingLogRecipe:** `tokio::task::spawn_blocking` inside an iced-polled stream (training_log.rs:115). Strongly inferred to panic, by the same topology proven at lab_runner_preload_callthrough_e2e.rs:254-293. Unverified for this recipe; Wave 0 confirms. If confirmed: new bug-log entry plus its own fix story; its off-runtime test lands with that fix.
2. **Narration is FellBack-only in production:** `llm_provider` is `None` in both branches (runtime.rs:1613-1625), tagged `TODO(F9-v0.3)`. grep finds it tracked nowhere in docs/, _bmad-output/ or CHANGELOG.md. Candidate bug-log disclosure. A green harness must not be read as "LLM narration works".
3. **TrainingPoller has no production caller.** Wire or retire is a K1 analyst decision.
4. `default_cache_root()` is cwd-relative in production (cache_state.rs:110); route separately.

**Contradictions between agents, and how I resolved them:**
- **Seam shape.** Seven bools fails CI pedantic; by-ref vs by-value was inconsistent; per-variant constructors had differing signatures. Resolved in §0.
- **Legacy-equality test.** `build(a,b,c) == _from(default)` is tautological once the wrapper delegates. It must pin a literal.
- **Call-site count.** Agent 4 says "5 call sites in 3 files"; the actual count is 8 test call sites + 1 bin (grep).
- **Probe line errors.** Agent 5's "delete live.rs:78" names a comment line; the MarketHealth line is :77 (RiskTelemetry :79). The LabProgress salt is progress.rs:64, not :63.
- **ActivityAuditAggregator.** Agent 5 rates its coverage "still valid", yet notes that tests 1-2 assert only exit. I classify it as partial.
- **training_log_state_gating `cfg(unix)`.** Audit 2 calls it a non-legit skip. It runs on the canonical macOS leg, so I rate it platform-scoped, low severity.
- **Story AC1 "zero test references" for ForwardPlan/Narration.** True for the Recipe structs. Seam-level S1 exists (forward_narration_relay.rs, 7 tests). The Dev Agent Record should say so precisely.

**Claims I could not confirm by reading:**
- that `tokio::time::timeout` panics off-runtime (the basis of every §6 probe)
- that tokio mpsc `recv` works under `futures::executor::block_on` (implied by production polling on iced's ThreadPool)
- that `data/yahoo-sample` passes `load_cached`'s 95% coverage for H1_2024
- the bus publish APIs for MarketHealth and the strategy channels
- whether `iced::advanced::subscription::Event::Interaction` is constructible, if one wants to drive the `listen_with` stream rather than test the pure fn
- the End-on-close emission of the aggregator
- `can_run()` returning true for a default Tune form
- whether dev-dependency `iced_test` unifies iced's `tokio` feature (§6 sidesteps iced's executor either way)
- the files not re-read: lab_runner_ticker/cancel bodies beyond the spot-check, headless_emulator_smoke, live_subscription*, activity_audit_aggregator{,_invariants}.rs, `live.rs mod tests`, `lab/training_log.rs mod tests`, tape_row_click_opens_modal.rs, cockpit_toast_queue.rs (grep showed no vacuity patterns)
- None of the proposed tests were compiled or run.

**Story ACs and tasks that need explicit handling:**
- **AC5** cites live.rs:1361-1387. Correct it to 1383-1403 in the Dev Agent Record.
- **AC5's "clippy clean"** must include `--tests` locally, because CI does not lint test targets.
- **AC5's K4 "≤ 10%"** has no baseline anywhere, so Wave 0 must create one. Disk is tight (target/ is about 25 GB per memory).
- **AC4 requires a probe cycle per recipe.** The tester report must list the K3/K1 recipes (cockpit.rs ServerTime, focus_ring, TrainingPoller pending K1, agent forward-plan send) as explicitly unprobed, with reasons, or the "no probe, no PASS" rule reads as violated.
- **ADR-0048 D4 is inaccurate.** It claims salt-bump/reconstruction bugs are "already covered by `lab_progress_recipe_stream_end_to_end`", but no test anywhere hashes a recipe (`Hasher::default`: 0 hits).
  - The v0.3.0 changelog row must correct that bullet and record the two refined S1/S2 forms: executor-fidelity (inherited from ADR-0050 §D3/D4) and real-batch introspection via `into_recipes`/`units`.
  - Whether amending a D-row counts as "a genuinely new pattern class" (the story's K1 route back to the analyst) is a call for the architect. My recommendation: a changelog amendment, no new ADR.
- **The ADR-0082 triad.** At `done`, story Status, `REQ-LAB-RECIPE-HARNESS-V0-3-001` (`scoped` → shipped) and the CHANGELOG line move together, or spec_lint fails.
- **Residual no wave closes:** the bin-side receiver lifecycle and the field mapping (K3 #6), and cockpit_training_pressed_wiring.rs replaying `AppState`. Both need an `AppState` builder seam, which is not additive. Record them as known residuals.


### File List

- Wave A (2026-11-11): `crates/ui/tests/forward_narration_relay.rs` (modified), `crates/ui/tests/bakeoff_progress_relay.rs` (modified), `crates/ui/tests/sweep_progress_relay.rs` (NEW). No production file.
- Wave B (2026-11-12): `crates/ui/tests/recipe_off_runtime_poll.rs` (NEW), `crates/ui/tests/server_time_recipe_stream.rs` (modified: `server_time_stream_impl_emits_at_1_hz_cadence` only). No production file.
