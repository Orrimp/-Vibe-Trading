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
  - [x] Wave D: advisor state machine S2, test-only. *(2026-11-13; D-P1…D-P28 red at their designed assertion; production finding routed as bug-log `#200`, see "Wave D results".)*
  - [x] Wave E1a: AC3 sweep of the seven v0.2 recipe-stream files, test-only. *(2026-11-13; 28/28 listed probes red, the two falsifier survivors fixed or recorded, 8 orchestrator probes red incl. E1-P34…E1-P39; Wave B's survivor B-P41 killed; see "Wave E1a results".)*
  - [x] Wave C1: the shared subscription seam, production + tests. *(2026-11-14, `0ce426cf`; C1-P1…C1-P12 red, the falsifier's two survivors fixed test-only and re-measured red as C1-P13/C1-P15, C1-RES1/RES2 named survivors; bug-log `#203` filed; see "Wave C1 results".)*
  - [x] Wave C2 (+ E1b): real-batch relays and recipe identity, test-only. *(2026-11-14, `73bfde06`; C2-P1…C2-P27 and E1-P21…E1-P23 red at their designed assertion over two falsifier passes; the falsifier's two survivors OWN-1/OWN-3 fixed test-only and adopted as C2-P21/C2-P23; see "Wave C2 (+E1b) results".)*
  - [ ] E2 · E3 · F (G deferred, decision 4).
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

**2026-11-12 to 11-13: Wave D results (advisor recipe state machine, S2, test-only). DONE.**
- **Files.**
  - NEW `crates/ui/tests/advisor_recipe_state_gating.rs`: 10 tests, driven only through `ui::state::update` plus the `forward_plan_stream_impl` / `narration_outcome_stream_impl` seams.
  - `crates/ui/tests/promote_swept_config.rs`: un-gated. Its `#![cfg(feature = "fixtures")]` meant CI's default `cargo test -p ui` ran **0** of its tests (measured: `running 0 tests`). It now runs 4. Its tests now go through `update` only; before, they wrote state straight into the model's fields.
  - **No production edit:** `git diff -- crates/ui/src crates/agent/src` is empty.
- **Rounds (`wf_63b56ffc-609`, then `wf_672f98f1-bbe`).** Each round had an independent falsifier, and round 1 also had a parallel read-only reviewer.
  - **Round 1:** the implementer's 10 tests plus D-P11…D-P15; 18 probes, of which 17 went red; 3 review findings.
  - **Round 2:** fixed those (D-P16…D-P23). Its fresh falsifier measured all 23 designated probes red, and its own extras R2-X1…X3 survived.
  - **Round 3:** decoupled the fixtures behind R2-X1 and R2-X2, both reachable through `update` (D-P24, D-P25), and measured R2-X3 (below). Its fresh falsifier measured **25/25 designated red at the designed assertion**, with 5× flake green and production equal to HEAD. Two of its three extras survived, R3-X1 and R3-X2.
- **Orchestrator fix for R3-X1/R3-X2 (decision: two assertions do not need another 5-hour round).** Both mutations are a refused mid-run Run press that rewrites the result (a press that doubles as Cancel and lands `Error`). They passed because round 2 had removed the mid-run "still `Loading`" checks on a "holds by construction" argument. That argument covers only refuse versus accept, and story rows §3 and §4 require `Loading`.
  - Restored as **D-P26** (`second_bakeoff_request_mid_run_keeps_progress`) and **D-P27** (`sweep_double_run_request_mid_run_keeps_progress`). Both are placed AFTER the bar and run checks, so every earlier probe still fails at its own assertion.
  - R3-X3 (an Explain guard that forgets `FellBack`) was red at the second `D-P4` site, which no listed probe reached. It is adopted as **D-P28**.
- **Orchestrator re-check on the final files** (gating `90e796d4…`, promote `aeb2c34c…`):
  - fmt clean.
  - `cargo clippy -p ui --test advisor_recipe_state_gating --test promote_swept_config -- -D warnings` clean after `touch` (`Checking ui`).
  - Green 10/10 and 4/4 after `Compiling ui`.
  - Five probes applied with my own harness (exact-once replace, `--no-fail-fast`, restore with `cp`, compare with `git show HEAD:`). Every restore was byte-equal to HEAD.
    - **D-P26:** red at `:910`, "D-P26: a refused Run press mid-run must leave the in-flight run's Loading result".
    - **D-P27:** red at `:1235`, "D-P27: … mid-sweep …".
    - **D-P28:** red at `:745`, "D-P4: a re-press of Explain must not clobber the resolved FellBack".
    - **D-P7:** still red at its own `:901`, "keep the live bar".
    - **D-P12:** still red at its own `:1226`.
- **Production finding, measured and routed: bug-log `#200`.** The Lab's "Tune…" button re-opens Tune on another coin without clearing the landed grid. Promote then pairs the old coin's grid parameters and window with the new `tune_coin`. That was R2-X3, measured in round 3 through `update`: `coin=SOLUSDT` with an ETHUSDT grid's params and "2024 H1". `promote_swept_config.rs` records it as a Known residual and pins neither behaviour (AC5).
- **Deviations, recorded.**
  - `relayed_plan_resolves_promotion_loading_to_ready` cannot tell the grid's window from the Tune lookback, because they are coupled there. `promote_swept_config.rs` (D-P24) separates them.
  - Three preconditions read cold defaults that no message can seed (`pending_forward_promotion` is written only by `promote_swept_config` and cleared only by the bin; SMA is the default family). Both module docs say so.
  - The false production doc on `LeaderboardScreenState::set_progress` is recorded, not edited (production code).
- **Evidence (scratch):** `/private/tmp/claude-502/-Users-Vitaliy-Schreibmann-Projects-Privat-trading-trading/362d2a09-04ba-4ea6-a7c1-07605f6e187a/scratchpad/wave-d/`
  - `r1-*`, `r2-*`, `r3-*` probe logs and diffs;
  - `r3f-flake-*.log`;
  - `r3-r2x3-measure.log`;
  - `orch/` (my probes, clippy and green logs).

**2026-11-12 to 11-13: production bugs the waves found, fixed between waves (decide-and-proceed).** These are not story deliverables. Each is its own fix commit plus a docs commit, following the `#196`/`#197` precedent: no story, no trace row, no CHANGELOG line.
- **`#198`, fixed in `dfe9e72a`** (ADR-0064 D12). Found by the E2 brief. The Explain facts gave the LLM the KPI fraction as a percent: `0.18%` for an on-screen `+18.37%`.
  - Consequence for E2: retire E2's anchor `fmt_pct2 (no ×100)`, and E2-P11's note now reads `'18.37%'`.
  - `live.rs` kept its 1,658 lines, so the briefs anchored below `:1301` stay valid.
- **`#199`, fixed in `d2fa6bba`** (ADR-0064 D13). Found while fixing `#198`. The frozen P3 tokenizer never returned on `top-3` or `2023-2024`.
- **`#201`, fixed in `cded86b5`** (ADR-0064 D14). Found by the `#199` review. P1 and P2 panicked on an em dash at a window edge, and one panic ended Explain for the session.
  - The containment helper sits after `panic_message`, so `runtime.rs:1627-1653` (rows 18 and "Narration (Wave G)") did not move.
  - Row 19's `runtime.rs:3346` was repointed to `:3375`. It was already stale by 2.
- **Filed open:**
  - `#200` (Wave D, above).
  - `#202`: an Explain answer is not tied to its request, so A's answer can land under bake-off B, and a refused send leaves the panel `InFlight`. It needs an ADR-0064 amendment and gets its own change after this story's waves.
- **Bug-log ids for the remaining waves:** C1's "gate that cannot fail" entry is now `#203` (E5: ids strictly increase). E2's entry goes to the next free id after C1's.

**2026-11-13: Wave B gate, run by the orchestrator on the committed Wave B (`074e907a`) before E1a.** The results are mechanical and feed the E1a prompt (`scratchpad/e1a/wave-b-gate.md`):
- **(a)** `bus_recipes_relay_off_runtime` is present. The dispatch-multiset fallback is skipped, and E1-P18 is measured against Wave B's test.
- **(b)** `activity_recipe_subscribes_before_its_first_poll` is present. The activity fallback and E1-P33 are skipped.
- **(c)** The paused cadence test reads `const TICKS: usize = 3_600;`, not the 4 the brief's scratch had. So E1-P11 freezes after 3,600 ticks, and T-ST-1c reads 7,200 paused ticks.
- **(d)** `assert_taken(&built, "training log")` is present, so E1-P14 and E1-P15 are expected red there too.
- **E1 is split:** E1a now, and E1b (2 tests, E1-P21…P23) after C1, folded into C2's tester pass. E1-P26 stays retired.

**2026-11-13: Wave E1a results (AC3 sweep of the seven v0.2 recipe-stream files, test-only). DONE.**
- **Files** (test-only; `git diff -- crates/ui/src crates/agent/src` is empty). Tests after E1a, by file:
  - `activity_recipe_stream.rs`: 5;
  - `trail_mirror_recipe_stream.rs`: 6;
  - `lab_progress_recipe_stream.rs`: 5;
  - `server_time_recipe_stream.rs`: 3;
  - `toast_dismiss_recipe_stream.rs`: 3;
  - `training_log_recipe_harness.rs`: 4;
  - `live_subscription_full_bus.rs`: 3.

  Every file now carries a Wave A-shaped module doc: a non-vacuity contract, a T-T4 table citing symbol + quoted line, and a coverage table. The v0.2 line-number probe blocks are gone.
- **The workflow (`wf_f2fc2790-557`).** Seven devs ran, one per file with serial cargo, then four serial falsifiers over the 28 listed probes, plus a read-only reviewer in parallel.
  - **28/28 listed probes red**, each restored byte-identically, with production SHAs equal before and after and each group green at the end.
  - The only strict-set mismatches (E1-P4, E1-P8, E1-P27, E1-P28) were against the brief's probe list (`_probes_E1a.json`), whose sibling sets were incomplete. Every dev's module-doc row had the full set.
  - **Mid-run correction by the orchestrator, before the server_time group started:** E1-P11's mutation was still the pre-gate `0..4`; Wave B gate (c) makes it `0..3600`. The falsifier measured exactly the designed red, with T-ST-1b GREEN.
- **The falsifiers' own probes: 8, of which 2 survived.**
  - **AT-OWN-1 SURVIVED** (activity): a relay that ends itself after 100 ms of quiet passed 5/5, because every liveness check was one poll at t≈0.
    - **Fixed by the orchestrator:** `stream_stays_open_while_sender_alive` (activity) and `trail_mirror_stream_stays_open_while_sender_alive` (trail) now run on a paused clock, and a second `assert_pending` follows `advance(3600 s)`.
    - Adopted as designed probes **E1-P34** (activity) and **E1-P35** (trail). Both are MEASURED red at that new assertion only.
  - **LT-OWN-2 SURVIVED** (training log): a bridge that drops its sender and then blocks forever, orphaning one OS thread per run, passes everything. The test doc had claimed "no orphan `training-log-bridge` thread"; that is now reworded to what the END proves (the sender was dropped). Recorded as a residual: joining the bridge needs a `JoinHandle` seam in production.
  - Killed: AT-OWN-2, LT-OWN-1, ST-OWN-2, FB-OWN-1, FB-OWN-2, and **ST-OWN-1 = Wave B's B-P41**. ST-OWN-1 is red at "toast: the stream is still live after tick 4", so **Wave B's survivor is closed**.
- **Orchestrator fixes after the workflow, test-only:**
  - **`live_subscription_full_bus.rs`.** The falsifier found that the tape, P&L, latency and bar checks pinned only the variant (`Ready(_)`, `Known { .. }`, `is_some()`). They now compare content derived from the fixtures:
    - the first fill's side, price and stamp;
    - the first P&L snapshot's figures;
    - `Known { ms: 1000 }`, since the tick fixture now arrives 1 s after its venue stamp;
    - the first bar's close.

    Adopted as **E1-P38** (`fill_to_view` side) and **E1-P39** (`stream_pnl` zeroes `cash`), both MEASURED red at those assertions. Cruft removed: the `FundingObs` dead-code anchor and the `SmolStr` / `synthetic_fill` "silence" lines.
  - **Doc claims corrected to the measurement:**
    - trail E1-P6's Wave B sibling is red at `expect_parked`, not at its receiver count;
    - B-P12 now reddens T-TM-1d (reasoned);
    - B-P21 and B-P22 are MEASURED red in `training_log_recipe_stream_takes_receiver_once`, at tokio's own "`EnterGuard` values dropped out of order", not at a designed assertion. Wave B's rows calling this file a GREEN sibling are stale;
    - Wave B siblings MEASURED red, added to the rows: under E1-P12, E1-P25 (`toast_dismiss_stream_ticks_off_runtime`) and E1-P24 (`server_time_stream_ticks_off_runtime`);
    - full_bus's "outside this target" paragraph and the E1-P18 row are now measured. E1-P18 fails at the Pnl wait, never at CROSS-TALK;
    - the activity lib-sibling note is now measured.
  - **Toast "exactly one" and presence sides**, which no listed probe reached: **E1-P36** (a duplicated yield) and **E1-P37** (Wave B's B-P31, 700 ms, re-run) are MEASURED red at "toast period 1: exactly one ToastTick at 500 ms".
- **Orchestrator hand probes (8):** B-P21, B-P22, E1-P34…E1-P39. All red, each with exact-once apply, `Compiling ui` in the log, and a restore with `cp`, `touch` and `cmp`. Post-SHAs equal pre-SHAs. Evidence: `scratchpad/e1a/hand/`.
- **Review findings (F0…F5), disposition:**
  - F0 (medium): fixed. B-P21 and B-P22 are measured, and the doc is corrected.
  - F1 (medium): the probe list was fixed mid-run, and B-P41 was measured as ST-OWN-1.
  - F2 (low): fixed.
  - F3 (low): stale Wave B sibling columns, recorded below.
  - F4 (low): **ACCEPTED.** The server_time dev rewrote two doc sentences of Wave B's T-ST-1b, whose code is byte-identical. The T-ST-1c rewrite made the old sentences false.
  - F5 (low): carried into E1b. **E1b's new asserts must use the exact texts E1a's rows already quote:**
    - "toast recipe: `ToastDismissRecipe::stream` returns a live stream" (E1-P22);
    - "toast recipe: the first ToastTick arrives within 750 ms (paused clock)" (E1-P23);
    - `server_time_recipe_stream_yields_tick` (E1-P21).

    Otherwise E1b updates those rows in the same edit.
- **Stale since E1a, not E1a's files (F3).** `recipe_off_runtime_poll.rs`'s committed sibling columns describe the pre-E1a revisions of E1a's files:
  - B-P12 (trail GREEN);
  - B-P18 (lab_progress GREEN; now measured red under E1-P10);
  - B-P21 and B-P22 (training_log GREEN; now measured red);
  - B-P38 ("its other three tests … T-ST-1c accepts a timeout");
  - B-P28 and B-P39 ("0 of 4 … current revision");
  - B-P41 ("3/3 GREEN");
  - the module doc's `toast_tick_instants_are_monotone` and T-ST-1d, both deleted.

  **Decision:** E1b's dev corrects them, as the next wave in the E1 family. Each gets "(pre-E1a revision)" or the measured current state.
- **Story corrections, recorded, not rewritten (planning text).** The §7 rows below are superseded by the E1a module docs:
  - Activity: `activity_recipe_stream_subscribes_eagerly` → Wave B's B-P19.
  - ServerTime: `server_time_period_is_one_second` → T-ST-1b, with E1-P24 as the guard.
  - TrainingLog: stale line cites; it is now E1-P14 by symbol.
  - Bus: `market_health_stream_relays_into_cockpit_state_then_ends`, with E1-P19, E1-P20 and E1-P32. `bus_recipe_stream_routes_…` was not added, per gate (a).
  - LabProgress: the test is `lab_progress_second_recipe_on_a_taken_cell_ends_without_yielding`, its salt probe is deferred to C2, and progress.rs:107 = E1-P29.
  - The AC3 audit's "clean / acceptable / optional" verdicts on server_time, toast and full_bus are falsified by the rewrite.
  - The std-thread bridge (`training-log-bridge`, since `dcce75e2`) replaces the `spawn_blocking` wording at :74, :594 and inventory row 11.
  - Wave B decision 1's "already pins the period exactly" is now TRUE, measured by E1-P12, E1-P36 and E1-P37.
- **Residuals, recorded:**
  - LT-OWN-2 (above).
  - In training_log, the salt-2 half adds no take-state power: an empty cell and a dead receiver both end at once with nothing (reasoned).
  - "Every sender, not any sender" is unpinned in activity (tokio's semantics guarantee it).
  - T-ST-1c's `OPEN_TICKS = 7_200` is tied to Wave B's fn-local `TICKS = 3_600` by a comment only.
  - The toast `for k in 5..=TICKS` loop and T-ST-1b's drift assert have never been measured red. They use the helpers that were measured red at earlier ticks.
  - The kill-reason expected value is derived from `HaltReason`'s Display, deliberately. `"manual_operator"` itself is core's (`halt.rs:34`).
  - The 150 ms sticky-trip window can read green on a loaded box, never red.
- **Orchestrator gate on the final files:** **fmt clean; `cargo test -p ui` over the 7 targets: 29/29 green after `Compiling ui` (1004 s wall, mostly compile); `cargo test -p ui --lib lab::training_log` 3/3; `cargo clippy -p ui --tests -- -D warnings` clean after a touch (`Checking ui`, 383 s).** Final SHA-256 prefixes: activity `a9e46964`, trail `b889379e`, lab_progress `7134d970`, server_time `b9e03da8`, toast `7b3a219f`, training_log `0bac705f`, full_bus `c62403db`.
- **Gate finding, fixed in its own commit `f5c89e19`.** The story's own wave-plan gate, `cargo clippy -p ui --tests -- -D warnings`, was RED at HEAD since `dcce75e2` (`#196`): `training_log.rs`'s test module allowed `unwrap_used` but not `expect_used`, and `:246` calls `.expect(..)`. CI's clippy does not lint tests, and every wave since (the E1a devs included) scoped clippy to its own `--test` target, which never lints the lib's test module, so nothing saw it. **Close-wave item:** decide whether CI's clippy step gains `--tests` (cost: one more full ui test-target check per push).


**2026-11-14: Wave C1 results (the shared subscription seam, production + tests). DONE.**
- **Landed in `0ce426cf`.** Bug-log `#203` was filed `open` there and is `partial-fix` citing it since the follow-up docs commit. It goes to `fixed` with C2.
- **What landed (one diff; API-additive).**
  - **`crates/ui/src/live.rs`:**
    - five new `SubscriptionVariant`s (`ForwardPlan`, `NarrationOutcome`, `BakeoffProgress`, `SweepProgress`, `ModalEsc`), with `Held` and `SubscriptionBatchInputs`;
    - `build_subscription_batch_descriptor_from`, to which the 3-arg fn delegates;
    - `#[derive(Clone, Copy)] SubscriptionCtx` with `inputs()`;
    - `subscription_batch` and the private, exhaustive `variant_subscription`;
    - `ServerTimeRecipe` and `ToastDismissRecipe` moved in from the bin as `pub`;
    - the false "T-D-C2 / T-D-C4 catch it" seam comment rewritten.
  - **`crates/ui/src/widgets/journal_transaction_modal.rs`:** `pub fn esc_closes_modal`, which both bins batch.
  - **`crates/ui/src/bin/cockpit_live.rs`:** `subscription()` is now the 40-line `SubscriptionCtx` literal, and the bin-private recipes are deleted.
  - **`crates/ui/src/bin/cockpit.rs`:** `listen_with(journal_transaction_modal::esc_closes_modal)`.
  - **NEW `crates/ui/tests/subscription_batch_inclusion.rs`**, with 7 tests.
  - **Deleted:** the three v0.2 files `cockpit_subscription_server_time_always_batched.rs`, `cockpit_subscription_toast_dismiss_always_batched.rs` and `trail_mirror_subscription_handle_gating.rs`. Their `trace.toml` lines are commented out, following the `:2263` precedent.
  - **Bug-log `#203`** is filed, see below.
- **The workflow (`wf_8469a45c-804`).** One dev first; then a serial falsifier and two read-only reviewers (equivalence, tests+docs) worked from snapshots. The dev's STEP 0 re-validated the brief at HEAD `1a4d876c`: BAD 0, and the shift map was unchanged.
- **Falsifier.**
  - **C1-P1…C1-P12 red** at their designed rows, each restored byte-identically with `Compiling ui`.
  - **C1-P2's red set was one test larger than claimed:** `ctx_inputs_key_on_outer_option_not_inner_receiver` also went red, at "the taken-receiver ctx's descriptor lists SweepProgress". The table row now names it.
  - **C1-RES1 and C1-RES2 left every target green**, `cargo test -p ui --bin cockpit_live` included. Both are named survivors of K3 #6.
- **The falsifier's own probes: 3, of which 2 survived. Both survivors are fixed by the orchestrator, test-only.**
  - **OWN-1 SURVIVED.** A take-once arm (bake-off) was keyed on its INNER receiver. The flag test checked only `inputs()` and the descriptor, and the units test used only held cells.
    - In production, the bake-off bar would freeze one frame after its recipe took the receiver.
    - **Fixed:** `ctx_inputs_key_on_outer_option_not_inner_receiver` now asserts `subscription_batch(..).units()` for a TAKEN cell: 15 for each of the six take-once fields alone, 20 for all six.
    - Adopted as **C1-P13**.
  - **OWN-3m SURVIVED.** "Any captured event closes the modal" passed, because the negatives ran only at `Status::Ignored`.
    - **Fixed:** each negative now runs at both statuses.
    - Adopted as **C1-P15**.
  - **OWN-2** was killed, and is tabled as **C1-P14**.
  - **Vacuity finding (low), fixed:** the 128/256 derived-count loop held by construction once the per-mask assert passed. It is now a comment. In its place, an oracle self-check pins the hand-written table: each input bit keys exactly one variant.
- **Review findings, disposition.**
  - **(medium, both reviewers) The salt residual.** Neither C1-RES2 nor the walk could detect a salt swap. The bin clears each take-once cell when its run ends (`self.training_log_rx = None;`, and likewise for lab, bake-off and sweep). iced then drops the id on that frame, so the next run streams afresh whatever its salt is.
    - **Decision:** a swapped salt is recorded as checked by NOTHING, the walk included. The RES2 row, the test module doc, bug-log `#203` and the runbook all say so. The walk's "second run checks the salt" claims and its salt diagnosis lines are deleted.
  - **(medium) "An arm building the sibling recipe" survives C1.** `units()` counts recipes and does not check what they are.
    - **Decision:** `#203` is filed `open`, is flipped to `partial-fix` citing the C1 commit, and goes to `fixed` only with C2. C2's brief already plans `batched_*_recipe_relays` asserting the emitted `Message` VARIANT (e.g. `SweepProgress`, not `BakeoffProgress`) and `recipe_ids_are_pairwise_distinct`.
  - **(low) Docs.** The seam and bin comments said "every test calls `subscription_batch`", but only the units tests do; fixed. `SubscriptionVariant`'s doc now notes that `Bus` is an 11-unit batch. The modal's view doc no longer says Esc routes to the caller's close message. The test's non-vacuity paragraph is corrected: not every assert is in a loop, and the agreement half is a deliberate subject cross-check, guarded by the literal half.
  - **(low) The fixtures `cockpit` bin's `listen_with(esc_closes_modal)` wiring** is now listed under "What this file does not pin". That bin needs `fixtures`, and `cockpit-smoke` presses nothing.
  - **(low) Forward references.** The live.rs headers cited "ADR-0048 v0.3.0", which does not exist until Close. They now say "story 2-63" and "ADR-0048 row at its Close". `#203` is filed in this commit, so its citations resolve.
  - **(low) Line cites shifted by C1** (the bin's lines 190..2482 moved by −88; 101..189 were deleted).
    - **Repointed:** bug-log `#202`, which is open and gets implemented next: `:1800-1810` → `:1712-1722` and `:525` → `:437`.
    - **Left as dated history, by decision:**
      - ADR-0050's Context cite `cockpit_live.rs:104-126`, a 2026-05-29 narrative; the rule it names now lives in the `ServerTimeRecipe` doc in `ui::live`;
      - bug-log `#199` and the old entry at `:912`;
      - the 2026-08-15 audits (`reachability-map`, `claims-ledger`, `feature-reachability-audit`);
      - `review-rubric-walker.md`;
      - the done story 3-19.

      Each says what was true at its date.
    - `runner.rs:1496` stays routed to Wave F.
  - **(low) Texts still calling the recipes "bin-private" or naming a deleted target.** These are in `recipe_off_runtime_poll.rs` (:21-27, :412, :537, :631-636, :2963, :3023), `server_time_recipe_stream.rs:8-9`, `toast_dismiss_recipe_stream.rs:14-15`, `lab/progress.rs:24`, `lab/training_subscription.rs:32`, and the `trace.toml` comment at :1741.
    - **Decision:** E1b's dev sweeps all of them, alongside the F3 sibling-column fix.
  - **(low) The human recipe's diagnosis cargo step** now has a timing note and a `watch` block.
- **The operator walk** is durable now: [`docs/runbooks/cockpit-live-subscription-walk.md`](../../docs/runbooks/cockpit-live-subscription-walk.md). It has 10 steps and covers the `trail:` field the scratch draft missed.
  - It catches C1-RES1's class: a `None` passed for a held cell, or a field wired to the wrong cell.
  - It cannot catch C1-RES2.
  - It is the ONLY check of the bin literal, and it is not yet walked.
- **Story corrections, recorded, not rewritten (planning text).**
  - **K3 #6 (§9) said the residual "goes to cockpit-smoke".** That is false: `cockpit-smoke` boots only the fixtures `cockpit` bin and presses nothing. The residual is wider than §0 says: the salts are interchangeable and any `Option` can be `None`.
  - **§0 "Why this shape" #1 overclaims.** It closes flag⇔arm divergence, not the field-mapping residual (C1-RES1).
  - **§0's sketch `#[must_use] pub fn subscription_batch`** shipped without `#[must_use]`. iced's `Subscription` is already `#[must_use]`, so adding it trips `clippy::double_must_use`.
  - **§5's test names and coverage are out of date.**
    - The test is `non_escape_events_map_to_none`.
    - The outer-Option test covers all 6 take-once cells.
    - The units test has a literal half (14/15/22).
    - §5 probe (d) is retargeted as C1-P5.
  - **Line cites are pre-C1 and are superseded by symbol.**
    - The AC5 / critic / inventory cites of live.rs:1361-1403 are replaced post-C1 by the 3-arg wrapper `build_subscription_batch_descriptor`, `build_subscription_batch_descriptor_from`, `subscription_batch` and `variant_subscription`.
    - The inventory rows citing bin code C1 moved or deleted: §7 rows, the Esc arm, the recipe structs, and the deleted test files' lines. These are "pre-C1 (HEAD `1a4d876c`)".
  - **Wave F's cite "Production passes `None` (cockpit_live.rs:2380)"** is a `2d2d95be` number. It is `:2392` at `1a4d876c` and `:2304` after C1. Cite it by the quoted `None,` argument of `spawn_lab_run(`.
  - **The salt and lifecycle cites at story §5 and K3 #6** are `2d2d95be` numbers (+12 at HEAD, then −88). Cite them by quoted text: `bakeoff_progress_recipe_salt: 0,` and `self.bakeoff_progress_rx = None;`.
- **For C2's dev.** C2 builds on the CURRENT test file, not the brief's C1 skeleton. Three things changed since the skeleton:
  - the outer-Option test now carries the taken-cell unit rows;
  - the count loop is gone;
  - the negatives run at both statuses.

  C2's `forward_plan_stays_batched_after_receiver_taken` still adds frame-stability and relay, which the units rows do not.
- **Orchestrator gate on the final files:** **`scratchpad/c1/gate/gate.sh`, run alone, every step `Compiling ui`/`Checking ui`:** `cargo fmt -p ui -- --check` clean; `cargo clippy -p ui --tests -- -D warnings` green (365 s); `cargo clippy -p ui --bin cockpit --features fixtures -- -D warnings` green; `--test subscription_batch_inclusion` 7/7. **The three hand probes, re-applied by the orchestrator, each red at its designed row:** OWN-1 → "units for a ctx holding a TAKEN bakeoff_progress cell" (14 vs 15); OWN-2 → "Esc press (status Captured) closes the modal: None"; OWN-3m → "Enter press (status Captured) must not close the modal: Some(TapeAuditModalClosed)". Each was restored sha-identical, and the target is 7/7 again after. `cargo check -p ui --no-default-features --features fixtures` green; both bins build. **Full `cargo test -p ui --no-fail-fast`: 1320 passed, 0 failed, 32 ignored over 122 result lines** (6 h 14 min wall; nearly all of it is macOS's first-launch check of each freshly linked test binary, about 2-3 min apiece, not test time). **cockpit-smoke PASS** with startup proven: the 7 s window opened only after the process had used 1.05 s of CPU, it was alive at the window's end, and the log has 0 panic lines. Anchors 119/119; spec-lint 0 violations; ADR registry clean; bug-log schema PASS (`#203` open).

**2026-11-14: Wave C2 (+E1b) results (real-batch relays and recipe identity, test-only). DONE.**
- **Landed in `73bfde06`.** Bug-log `#203` is `fixed` since the follow-up docs commit, citing it and `0ce426cf`.
- **What landed (test-only).** The only `crates/ui/src` edits are doc-comment lines in `lab/progress.rs`, `lab/training_log.rs` and `lab/training_subscription.rs` (`comment_only_diff.py`: 0 non-comment lines). `live.rs`, both bins, the modal, `evidence/` and `vendor/` are byte-identical to `fad6fe25`.
  - **`crates/ui/tests/subscription_batch_inclusion.rs`: 7 → 25 tests.**
    - **12 relays**, one per unit kind: `batched_{forward_plan,narration_outcome,bakeoff_progress,sweep_progress,lab_progress,training_log,trail,activity,bus,server_time,toast_dismiss}_recipe_relays` and `batched_modal_esc_relays_escape_only`.
      - Each takes its unit out of `subscription_batch(..)` by a reference id built from an independent recipe, streams it, and asserts the `Message` VARIANT.
      - Nine also assert a distinctive payload. ServerTime and Toast cannot (their payload is a `now()` stamp), and ModalEsc has none.
      - The seven relays whose sender the test owns relay a SECOND item while the sender lives and then end on its drop.
    - **`batched_bakeoff_and_sweep_units_drain_own_cells_{equal,distinct}_salts`.** Both `BakeoffProgressCell`-typed cells are held at once, at salts 7/7 and 7/8. Each unit must empty ITS cell and relay ITS sender's value.
    - **`forward_plan_stays_batched_after_receiver_taken`**, for the plan and narration cells.
    - **Identity:** `recipe_ids_are_frame_stable`, `salted_recipe_ids_change_with_salt` (the exact differing set {13, 15, 19, 20}) and `recipe_ids_are_pairwise_distinct` (22 distinct ids at salt 0 and at salt 1).
    - **T-T4 rows C2-P1…C2-P27.**
  - **E1b:** `server_time_recipe_stream_yields_tick` and `toast_dismiss_recipe_stream_yields_tick`. Each is the real `Recipe::stream` of a recipe C1 moved into `ui::live`, on a paused clock (rows E1-P21…E1-P23).
  - **Doc sweeps** (E1a's F3 and C1's "bin-private" finding): `recipe_off_runtime_poll.rs`'s module doc and sibling columns, the `server_time_recipe_stream.rs` and `toast_dismiss_recipe_stream.rs` headers, the three `lab/` doc lines, and the `trace.toml` comment near `:1741`.
- **The workflows.**
  - **`wf_a82b74dd-7bb`:** one dev, then a serial falsifier and two read-only reviewers (docs, tests), all on snapshot `c2/final`.
  - **`wf_be1acf7b-3e7`:** one fixer applied the orchestrator's fix list (T1–T7, D1–D12, new rows C2-P21…C2-P27). A falsifier and a read-only checker followed, on snapshot `c2/final2`.
  - **The orchestrator** applied the checker's seven lows and the MEASURED labels itself. These were doc lines only, checked comment-only against `c2/final2` (111 lines, 0 non-comment).
- **Falsifier, pass 1:** C2-P1…C2-P20 and E1-P21…E1-P23 were **23/23 red** at the designed assertion. Each OLD was found once, and each file was restored SHA-identical with `Compiling ui`. Of its own probes, OWN-2 (an arm-level filter on a captured Esc) was killed. **OWN-1 and OWN-3 SURVIVED**, 23/23 green:
  - **OWN-1:** the Sweep arm drains the bake-off cell whenever that cell is held. No fixture streamed with both cells held. This state is reachable in production. `cockpit_live` sets each cell on its own run request and clears each only on its own completion (`self.bakeoff_progress_rx = None;`, `self.sweep_progress_rx = None;`). A bake-off and a sweep in flight together would therefore race for one take-once receiver.
  - **OWN-3:** a single-shot plan body. The relays sent one item and dropped the sender, so nothing checked that the stream stays open after an item. `forward_narration_relay` already killed it, so this was a coverage-row overclaim, not a suite gap.
  - **Fixed test-only** by T1 (the both-cells pair) and T2 (the second item), and adopted as C2-P21 and C2-P23.
- **Falsifier, pass 2:**
  - It ran C2-P21…C2-P27, C2-P13 at its new label, C2-P19 and C2-P20 in their E1b targets as well, and B-P34. **11/11 red** at the designed assertion.
  - OWN-1's and OWN-3's mutants are byte-identical to C2-P21's and C2-P23's. Both are now killed inside this target.
  - The three unmutated runs were green: 25/25, 4/4, 4/4.
  - **One under-claim:** C2-P24 also reddens `bakeoff_progress_relay_latest_event_wins_on_state`. Its row now names that red as MEASURED.
- **Review findings, disposition.**
  - **(medium) The activity/bus "another bus must not be heard" checks could never fail on the bus dimension**, because the other-bus event has zero receivers. **Fixed (T3):**
    - they are relabelled as liveness checks, and the inert publishes are removed;
    - the doc says the positive half pins the ctx-bus wiring and that no check claims two-sided bus isolation;
    - C2-P13 was re-measured red at the new label.
  - **(low) Each sibling swap was probed in one direction only.** **Added C2-P26** (the Bakeoff arm builds `SweepProgressRecipe`) **and C2-P27** (the ServerTime arm builds `ToastDismissRecipe`). Both are MEASURED red.
  - **(low) The training-log relay crosses threads** (its `training-log-bridge` thread). It now waits up to `RELAY_XTHREAD`, 10 s (T4).
  - **(low) The boot negative checked only the salt-7 id.** The four salted relays also check the boot salt's id, 0 (T5). That check is recorded as a one-sided extra negative, backed by C1's 14-unit boot count.
  - **(low) The bus-only half of `recipe_ids_are_frame_stable` was built from temporaries.** It is dropped (T6). The 22-unit half covers the bus ids with both batches alive.
  - **(low) The bus relay depends on the `BUS_CHANNELS` order.** Its positive label now says so (T7). Outside the file, `recipe_off_runtime_poll`'s `BUS_BATCH` table goes red on a reorder.
  - **(low, D1–D12) Docs.** These covered:
    - the MEASURED labels and the probe-overlap lists;
    - "A reference id is never read off the unit under test, except `ModalEsc`'s";
    - the two stale "Constructed by `cockpit_live.rs::subscription()`" lines in `lab/progress.rs` and `lab/training_log.rs`;
    - E1b's state under B-P34 in `server_time_recipe_stream.rs`;
    - the B-P35/B-P36/B-P37 sibling notes.
  - **(low) The checker's seven lows on pass 2,** applied by the orchestrator:
    - the "only fixture holding both cells" wording;
    - the `--test bakeoff_progress_relay` target;
    - the cross-file probe overlaps (C2-P23/C2-P25 are `forward_narration_relay`'s P1/P4);
    - five more sibling notes in `recipe_off_runtime_poll.rs` (B-P12, B-P17, B-P18, B-P29, B-P40, REASONED) and its paragraph;
    - T-ST-1c's "up to 1.5 s of paused time";
    - the boot-salt check's one-sidedness;
    - the snapshot gap, checked by SHA instead (`trace.toml` and `training_subscription.rs` equal `c2/final`).
- **Dev deviations, accepted.**
  - **The ModalEsc relay is wider than the brief.** It sends two Esc presses, at both statuses, and the non-closing kinds at both statuses. OWN-2 shows the widening earns its keep: the brief's single-Esc unit would have passed.
  - **Label-prefixed assert texts.** The rows quote them verbatim.
  - **The identity tests' 22-unit premise reddens extra tests** under C2-P14 and C2-P7. The rows say so.
  - **The E1b texts** are E1a's quoted texts plus a suffix: ": it ENDED at its first poll" or ": it yielded with no time passing". E1-P22's row was updated in the same edit, as F5 allows.
  - **`lab/progress.rs` corrected a false claim:** that `ServerTimeRecipe` uses the take-once channel pattern. It holds no receiver.
- **Bug-log `#203`, the open part.**
  - C2-P12 (the Sweep arm builds `BakeoffProgressRecipe`) and C2-P26 (its reverse) are MEASURED red.
  - C2-P19 and C2-P27 cover the ServerTime↔Toast swap, body and arm.
  - It goes to `fixed` in the follow-up commit.
- **Story corrections, recorded, not rewritten (planning text).**
  - **Wave plan row C2 and §5** say "`batched_*_recipe_relays` ×8". There are 12 relays, plus the both-cells pair.
  - **§5 `recipe_ids_are_pairwise_distinct`** says "11 Bus + 10 others". The batch is 22 units, with ModalEsc the 11th other, and the test asserts it at salt 0 and at salt 1.
  - **§5 `recipe_ids_are_frame_stable`** compares two whole all-Some batches, index by index, with both alive (`std::ptr::addr_eq` premise).
  - **§5 `salted_recipe_ids_change_with_salt`** asserts the EXACT differing set {13, 15, 19, 20}, so the other 18 ids must not move.
  - **Wave plan row C2's `training_log.rs:72`** is `:78` at `fad6fe25`. Cite it by text: `self.salt.hash(state);` in `<TrainingLogRecipe as Recipe>::hash`.
  - **§5 `forward_plan_stays_batched…`** covers the plan AND the narration cell.
  - **K3 #2 (§9) is false.** `recipe_ids_are_pairwise_distinct` does NOT cover the static discriminants: every recipe hashes `TypeId::of::<Self>()` first, so deleting any discriminant leaves every test green (by reasoning). Recorded as unobservable through iced ids, not as a gap to fill.
  - **§0's "Timeouts are always `.expect(..)`"** holds for positive claims only. `assert_silent` accepts a timeout as the PASS of a liveness check.
  - **E1a F5:** the shipped E1b texts carry the suffixes above.
- **Residuals, recorded.**
  - Nothing automated checks the bin's `SubscriptionCtx` literal (C1-RES1, C1-RES2, a swapped salt). The walk runbook catches C1-RES1's class only, and it is not yet walked.
  - A ModalEsc arm-level filter on mouse, window or touch events would pass unit B, which gets three event kinds.
  - Bus isolation is not a fault class of `subscription_batch`, and it is not pinned.
  - **Still REASONED:**
    - the both-cells tests' extra reds under C2-P5, C2-P8, C2-P11, C2-P12, C1-P2 and C1-P6;
    - C2-P18's relabelled text;
    - the boot-salt checks passing under C2-P1…C2-P4, C2-P11, C2-P15 and C2-P16;
    - the five new sibling notes in `recipe_off_runtime_poll.rs`.
- **Orchestrator gate on the final files:** **`scratchpad/c2/gate2/gate.sh`, run alone.**
  - **Scope checks.** The three test files vs `c2/final2`: 111 changed lines, 0 non-comment. `crates/ui/src` vs HEAD: +9 −5, 0 non-comment. `live.rs`, the bins, the modal, `evidence/` and `vendor/` are unchanged.
  - **Lints and tests.** `cargo fmt -p ui -- --check` is clean. `cargo clippy -p ui --tests -- -D warnings` is green (`Checking ui`, 366 s). The four targets after `Compiling ui` (674 s): `recipe_off_runtime_poll` 18/18, `server_time_recipe_stream` 4/4, `subscription_batch_inclusion` 25/25, `toast_dismiss_recipe_stream` 4/4.
  - **Two hand probes, re-applied by the orchestrator.**
    - **C2-P21:** 23 passed, 2 failed, red at "both cells, salts 7/7: the sweep unit took the SWEEP cell's receiver" and at "both cells, salts 7/8: sweep unit: exactly one batched unit carries the reference id" (0 vs 1).
    - **C2-P26:** 21 passed, 4 failed, red at the three "…: exactly one batched unit carries the reference id" asserts (bake-off unit at 7/7 and at 7/8, `bakeoff_progress`; 0 vs 1 each) and at "all 22 units of the all-Some batch have distinct ids" (21).
    - Each was restored equal to HEAD. The target was 25/25 again after `Compiling ui`, and 25/25 on a second run.
  - **Repo gates.** Anchors 119/119, spec-lint 0 violations, the ADR registry clean, and the bug-log schema PASS.
  - **No full `cargo test -p ui` this wave.** The change is test files plus doc-comment lines, and `clippy --tests` checks every test target. The full suite (6 h+ wall here) runs at Close.


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
| 19 | Agent forward-plan supervisor send (producer for row 1) | inline `try_send` at runtime.rs:1474-1514, inside the supervisor | none | none | none. Every `RunHandles` in tests passes `plan_tx: None` (runtime.rs:3375; agent/tests/*:83, :106, :143). | n/a | n/a | **K3/K1 note.** Not additively extractable. |

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
| Narration (Wave G, optional) | NEW crates/agent/tests/narration_task_loop.rs + seam `pub async fn narration_task_loop(req_rx, outcome_tx, llm_provider, cancel)`, extracted from runtime.rs:1627-1653 (it must keep calling `narrate_containing_panics`, bug-log `#201`) | `narration_task_loop_answers_every_request`, `_exits_on_cancel_within_bound`, `_exits_when_request_channel_closes` | 2 requests give 2 `FellBack`; cancel makes the join complete within 1s; dropping the sender makes the join complete within 1s | add `break;` after the send block; delete the `narration_cancel.cancelled() => break` arm (:1632); let-else `break` → `match std::future::pending::<std::convert::Infallible>().await {}` |

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
| **E1** | AC3 sweep of v0.2 recipe-stream files, test-only | activity_recipe_stream.rs, trail_mirror_recipe_stream.rs, lab_progress_recipe_stream.rs, server_time_recipe_stream.rs, toast_dismiss_recipe_stream.rs, training_log_recipe_harness.rs, live_subscription_full_bus.rs (Bus dispatch); probe-citation rot → symbol + quoted line | no | **Split 2026-11-13:** E1a depends on Wave B's commit (DONE, see "Wave E1a results"); E1b (2 tests, E1-P21…P23) depends on C1 (moved ServerTime/Toast recipes) and folds into C2's tester pass (DONE with C2, see "Wave C2 (+E1b) results") | live.rs:592; :751-759; :141; :810; training_log.rs:83-88 take-removal; progress.rs:107; **B-P41** (Wave B's measured survivor: `toast_dismiss_stream_impl` `loop {` → `for _ in 0..4 {`; needs a paused-clock toast liveness test) |
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
- Wave D (2026-11-13): `crates/ui/tests/advisor_recipe_state_gating.rs` (NEW), `crates/ui/tests/promote_swept_config.rs` (modified: un-gated, tests routed through `update`). No production file.
- Wave E1a (2026-11-13): `crates/ui/tests/{activity_recipe_stream,trail_mirror_recipe_stream,lab_progress_recipe_stream,server_time_recipe_stream,toast_dismiss_recipe_stream,training_log_recipe_harness,live_subscription_full_bus}.rs` (modified). No production file. The gate finding's one-line test-module lint fix to `crates/ui/src/lab/training_log.rs` is its own commit, `f5c89e19`.
- Wave C1 (2026-11-14): production `crates/ui/src/live.rs`, `crates/ui/src/widgets/journal_transaction_modal.rs`, `crates/ui/src/bin/cockpit_live.rs`, `crates/ui/src/bin/cockpit.rs` (modified); `crates/ui/tests/subscription_batch_inclusion.rs` (NEW); `crates/ui/tests/cockpit_subscription_server_time_always_batched.rs`, `crates/ui/tests/cockpit_subscription_toast_dismiss_always_batched.rs`, `crates/ui/tests/trail_mirror_subscription_handle_gating.rs` (DELETED, folded into the new file); `_bmad-output/planning-artifacts/trace.toml` (the 3 deleted paths commented out); `docs/dev-notes/bug-log.md` (`#203` filed; `#202`'s two line cites repointed); `docs/runbooks/cockpit-live-subscription-walk.md` (NEW).
- Wave C2 + E1b (2026-11-14): `crates/ui/tests/{subscription_batch_inclusion,server_time_recipe_stream,toast_dismiss_recipe_stream,recipe_off_runtime_poll}.rs` (modified); doc-comment lines only in `crates/ui/src/lab/{progress,training_log,training_subscription}.rs`; `_bmad-output/planning-artifacts/trace.toml` (one comment). No production behaviour change.
