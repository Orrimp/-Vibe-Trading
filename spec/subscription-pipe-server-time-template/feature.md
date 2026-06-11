---
slug: subscription-pipe-server-time-template
version: 0.1.0
status: draft
owner: analyst
priority: P2
updated: 2026-05-26
predecessor: cockpit-activity-status-bar v0.1.0 (shipped 2026-05-26)
parent: testing-framework-audit-2026-05-25 § R1 (subscription-pipe canonical layer)
---

# subscription-pipe-server-time-template — close the Wave 1 carve-out

> **Wave-1 follow-on.** The subscription-pipe test class template
> landed for `LabProgressRecipe` and `TrailMirrorRecipe` on
> 2026-05-25/26 (see
> [`crates/ui/tests/lab_progress_recipe_stream.rs`](../../crates/ui/tests/lab_progress_recipe_stream.rs)
> and
> [`crates/ui/tests/trail_mirror_recipe_stream.rs`](../../crates/ui/tests/trail_mirror_recipe_stream.rs)).
> The operator's 2026-05-26 carve-out decision excluded
> `ServerTimeRecipe` from that wave because
> `cockpit-activity-status-bar v0.1.0` was concurrently touching
> [`crates/ui/src/bin/cockpit_live.rs`](../../crates/ui/src/bin/cockpit_live.rs)
> (where `ServerTimeRecipe` lives at lines 129-174). With
> cockpit-activity v0.1.0 SHIPPED 2026-05-26 (see backlog Recent),
> the carve-out closes. This brief picks up the third and final
> open `Recipe` impl in the workspace's canonical UI subscription
> set.

## Why now

`ServerTimeRecipe` is the structurally simplest Recipe in the
workspace — a `tokio::time::interval(1 s)` ticker that emits
`Message::ServerTimeTick(Timestamp::now())`. But it shares the
same K8 pattern (runtime-context entry via `rt_handle.enter()`
before `Box::pin` to avoid the `EnterGuard !Send` leak) that the
two Wave-1 sibling recipes use. The 2026-05-23 P1 bug fix
captured in lines 110-128 of `cockpit_live.rs` is exactly the
class of wiring bug the
[testing-framework-audit-2026-05-25.md § R1](../dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md)
recommendation argued must be locked down with an end-to-end
test:

> "find every `Recipe` impl + every `subscription::*` function and
> confirm at least one end-to-end test exercises it"

The architect's audit explicitly named ServerTimeRecipe as a
recipe needing this treatment ("Pair with a `spec-lint` rule
`subscription-missing-e2e` that fails when a new Recipe impl
lands without a matching test file"). Closing the carve-out now
brings the workspace to **3/3 of the canonical UI Recipe impls
covered** by the subscription-pipe test class template — the
template is complete.

## Predecessor template (read first)

| Recipe | Helper fn | Test file | Test count |
|--------|-----------|-----------|------------|
| `LabProgressRecipe` | `ui::lab::progress::stream_impl` | [`crates/ui/tests/lab_progress_recipe_stream.rs`](../../crates/ui/tests/lab_progress_recipe_stream.rs) | 4 |
| `TrailMirrorRecipe` | `ui::live::trail_mirror_stream_impl` | [`crates/ui/tests/trail_mirror_recipe_stream.rs`](../../crates/ui/tests/trail_mirror_recipe_stream.rs) | 4 |
| `ServerTimeRecipe` | `stream_impl` (TO BE EXTRACTED — this brief) | `crates/ui/tests/server_time_recipe_stream.rs` (NEW — this brief) | 4-5 |

The refactor is mechanically identical to the precedent
`LabProgressRecipe` pattern:

```rust
// Before (today, cockpit_live.rs:144-173) — body inline in Recipe::stream
impl Recipe for ServerTimeRecipe {
    fn stream(self: Box<Self>, _input: EventStream)
        -> BoxStream<'static, Message>
    {
        let mut interval = {
            let _guard = self.rt_handle.enter();
            tokio::time::interval(Duration::from_secs(1))
        };
        Box::pin(async_stream::stream! { /* ... */ })
    }
}

// After (this brief)
impl Recipe for ServerTimeRecipe {
    fn stream(self: Box<Self>, _input: EventStream)
        -> BoxStream<'static, Message>
    {
        stream_impl(self.rt_handle)
    }
}

pub fn stream_impl(rt_handle: tokio::runtime::Handle)
    -> BoxStream<'static, Message>
{
    let mut interval = {
        let _guard = rt_handle.enter();
        tokio::time::interval(Duration::from_secs(1))
    };
    Box::pin(async_stream::stream! { /* same body */ })
}
```

The `EnterGuard` is dropped before `Box::pin` (preserves the
2026-05-23 K8 fix — `BoxStream<'static, T>` must stay `Send`).
The integration test (`server_time_recipe_stream_end_to_end`)
exercises `Recipe::stream()` proper to assert the delegation
preserves identity semantics (per H1).

## R1-R5 — Requirements

### R1 — Refactor `ServerTimeRecipe::stream` to delegate to `stream_impl`

Extract the body of `Recipe::stream` into a new free function
`stream_impl(rt_handle: tokio::runtime::Handle) -> BoxStream<'static, Message>`.
The function lives **outside the `ServerTimeRecipe` struct** so
integration tests can drive it directly without an `EventStream`
or a running iced application — mirrors the
`ui::lab::progress::stream_impl` precedent.

**Architect M-T1 decides** between two structural options:

- **(a)** Keep `ServerTimeRecipe` + `stream_impl` inline in
  `crates/ui/src/bin/cockpit_live.rs` and mark `stream_impl` `pub`
  (or `pub(crate)`) so the integration test imports it via the bin
  target. **Analyst-recommended.** Matches the file layout today;
  zero new module surface; integration tests at
  `crates/ui/tests/server_time_recipe_stream.rs` can `use cockpit_live::stream_impl`
  the same way Wave 1 imports `ui::live::trail_mirror_stream_impl`.
- **(b)** Move `ServerTimeRecipe` + `stream_impl` into a new
  `crates/ui/src/live/server_time.rs` module and re-export from
  `crates/ui/src/live/mod.rs`. Cleaner public-API shape (parallels
  `ui::live::trail_mirror_*`) but adds module-surface churn that
  is out of scope for a pure test-coverage follow-on.

**Architect decision-gate**: Option (a) at v0.1.0 unless the
architect surfaces a load-bearing reason to widen the refactor.
Either way, no behavior change.

### R2 — NEW test file `crates/ui/tests/server_time_recipe_stream.rs`

Mirror the `lab_progress_recipe_stream.rs` shape. **Four to five
tests** in the file:

| Test ID | What it pins |
|---------|--------------|
| T-ST-1a | **Happy path** — `stream_impl(rt_handle)` yields the first `Message::ServerTimeTick(Timestamp::now())` within ~1.5 s after the first interval tick (the body deliberately skips the immediate tick at line 167 of `cockpit_live.rs`). |
| T-ST-1b | **Tick monotonicity** — consecutive `ServerTimeTick` payloads are non-decreasing in `Timestamp` value. (`now()` may equal across cores under skew; assert `>=` not `>`.) |
| T-ST-1c | **Stream remains open** — after N=3 ticks, the stream is still open (not `None`) — `ServerTimeRecipe` never terminates by design; this is the inverse of the `LabProgressRecipe::stream_impl(None)` smoking-gun test. The "silent termination" failure mode for `ServerTimeRecipe` is "the stream ends and the status-bar clock freezes" — pin against it. |
| T-ST-1d | **Full `Recipe::stream()` end-to-end** — construct `ServerTimeRecipe { rt_handle: Handle::current() }`, call `Box::new(recipe).stream(no_op_event_stream)`, assert the first yielded message arrives within 1.5 s and is a `ServerTimeTick`. Exercises the exact code path `cockpit_live.rs::subscription()` uses (lines 1410-1412). |
| T-ST-1e (optional) | **Lag handling** — slow consumer (don't poll the stream for 3 s, then poll once). The `tokio::time::interval` default behavior is `MissedTickBehavior::Burst` — assert no panic, no `Lagged` enum (interval doesn't carry one). Documents the buffer behavior so a future change of `MissedTickBehavior` is gated. **Optional** because the default behavior is well-documented upstream; include only if architect Q surfaces value. |

Test count: **4 minimum, 5 with optional T-ST-1e**. The workspace
test count delta target is **+4 to +5**.

### R3 — `subscription-missing-e2e` spec-lint rule (CREATE-OR-UPDATE)

Verify via grep at M-T1 whether Wave 1 landed a
`subscription-missing-e2e` rule in
[`.claude/skills/spec-lint/SKILL.md`](../../.claude/skills/spec-lint/SKILL.md)
or [`scripts/spec_lint.py`](../../scripts/spec_lint.py). Analyst's
sweep at 2026-05-26 found **zero matches** for either keyword in
both files; the rule appears NOT to have shipped in Wave 1.

Two paths under R3:

- **R3.a (likely)** — Wave 1 did NOT ship the rule. v0.1.0 of THIS
  brief does NOT ship it either; it is out of scope. Add a
  forward-list line to feature.md § Out of scope citing the
  testing-framework-audit § R1 recommendation. Defer to a later
  brief once a 4th Recipe ships (i.e. the value of mechanical
  enforcement clears the cost of authoring the AST walker).
- **R3.b (unlikely)** — Wave 1 DID ship the rule. Update its
  allow-list / table-of-known-Recipes so `ServerTimeRecipe` is
  marked as covered by `server_time_recipe_stream.rs`. ~2 LoC
  Python edit.

**Default**: R3.a. Architect M-T1 confirms the grep result before
choosing.

### R4 — Non-regression contract

- **R4.1** — 34/34 anchors stay byte-identical
  (`scripts/verify_anchors.sh` PASS at M-FINAL). ZERO new
  anchors. Zero scenario-body changes — this brief is pure
  refactor + test addition, no strategy / backtest / exec / risk
  / audit / reports touch.
- **R4.2** — `cockpit_live.rs::subscription()` (lines 1401-1469)
  batch behavior is byte-identical. The `time_sub` recipe binding
  at line 1410 constructs `ServerTimeRecipe { rt_handle: ... }`
  exactly as today; the only delta is what happens *inside*
  `Recipe::stream` once iced calls it. No batch-order change. No
  hash-identity change (R4.3 below covers the hash specifically).
- **R4.3** — iced subscription identity hash unchanged. The
  `Recipe::hash` impl at lines 138-142 hashes only
  `TypeId::of::<Self>()` (`ServerTimeRecipe`); it does NOT depend
  on `stream`'s body. Refactoring the body cannot change the
  hash. Pinned by a sanity-assertion in the new test file (T-ST-1d
  can incidentally verify identity by hashing the recipe and
  comparing against a recorded byte vector — optional).
- **R4.4** — Cockpit-live boot smoke. Running
  `cargo run -p ui --features live --bin cockpit_live` for ~5 s
  (orchestrator-driven; sub-agents cannot launch the binary per
  AGENT.md capability boundary) MUST show the status-bar clock
  advancing every second. Architect M-T1 includes this as a
  manual orchestrator-side gate in the verdict tree; tester
  M-FINAL stops at the integration tests + workspace gates.
- **R4.5** — Wave 1's two existing test files stay byte-identical
  (no edits to `lab_progress_recipe_stream.rs` /
  `trail_mirror_recipe_stream.rs`).
- **R4.6** — `cockpit-activity-status-bar v0.1.0` ActivityRecipe
  test surface untouched (R4 is "ServerTimeRecipe only").

### R5 — Workspace test count delta

**+4 to +5 tests**. Verified by tester M-FINAL via
`cargo test --workspace --all-targets -- --nocapture` summary
table delta vs the baseline captured at
`spec/cockpit-activity-status-bar/reports/test-final-2026-05-26.md`
(or whichever is the most recent green M-FINAL workspace count).
No test removals. No `#[ignore]` additions.

## K1-K2 — Risk register

### K1 — Tokio runtime context leak via the helper extraction

**Risk**: the K8 pattern (entering the tokio runtime context
inside the helper before `Box::pin`, dropping the guard before
the future is constructed) is subtle. A naive extraction that
moves the `let _guard = ...` outside the helper, or that holds
the guard across `Box::pin`, would either re-introduce the P1
panic ("no reactor running") OR introduce a `!Send` constraint
on the returned `BoxStream<'static, _>`.

**Mitigation**: the precedent `lab::progress::stream_impl` shape
is the reference — guard scope is a `{ ... }` block ending
before `Box::pin(...)`. T-ST-1a (happy-path) is the falsifier:
if the guard leaks, the integration test panics on first
`stream.next().await` with the "no reactor running" message. T-ST-1d
(full `Recipe::stream()`) exercises both helper construction
AND iced's `BoxStream<'static, _>` consumption — the
`Send` constraint is checked at compile time.

### K2 — iced subscription identity hash changes

**Risk**: if architect Q surfaces a need to add a `salt` field
to `ServerTimeRecipe` (mirroring `LabProgressRecipe`'s salt-bump
per-run pattern), the `Recipe::hash` impl would change. This
would alter the iced subscription identity — iced would treat
the recipe as a new subscription on every render and call
`stream()` repeatedly. The status-bar clock would either freeze
(if `tokio::time::interval` is recreated each call but iced
de-duplicates by hash and reuses an old one) or re-emit at every
hash change (likely fine but uncertain).

**Mitigation**: **DO NOT add a salt field at v0.1.0**.
`ServerTimeRecipe` is process-lifetime (always-on, never
gated), not per-run. The current `TypeId::of::<Self>()`-only
hash is correct. R4.3 pins this. K2 is a documented forward-
risk only if a future brief wants to make the clock cadence
operator-configurable.

## H1 — Hypothesis

### H1 — The refactor is behavior-preserving

**Claim** (95% confidence): extracting the body of `Recipe::stream`
into a `stream_impl(rt_handle) -> BoxStream<...>` free function
and having `Recipe::stream` delegate to it is byte-identical at
the message-stream level. The `ServerTimeTick(Timestamp::now())`
emit cadence does not change. The `EnterGuard` lifetime does not
change (still scoped to a `{ ... }` block ending before
`Box::pin`).

**Justification**:

- **Precedent**. The same refactor was done for
  `LabProgressRecipe` (Bug #63 R8 fix, 2026-05-25) and
  `TrailMirrorRecipe` (Wave 1, 2026-05-26). Both refactors
  preserved their respective recipes' message-stream semantics
  exactly. No regression reports against either.
- **Mechanical shape**. The only body inside `Recipe::stream`
  for `ServerTimeRecipe` is (i) construct interval inside guard,
  (ii) drop guard, (iii) `Box::pin(async_stream::stream! { ... })`.
  All three steps move verbatim into the helper.
- **Falsifier**. T-ST-1a + T-ST-1d together would fail if H1 is
  wrong. T-ST-1a runs the helper directly; T-ST-1d runs the
  full `Recipe::stream` path. Both must yield a `ServerTimeTick`
  within 1.5 s; both must produce monotonically non-decreasing
  payloads. A divergence between (a) and (d) — e.g. (a) emits
  but (d) does not — would mean the refactor introduced a
  delegation-layer regression.

**Risk if H1 is wrong**: the integration test fails at M-FINAL,
the architect reverts the refactor, the test file stays in tree
asserting against the original inline implementation (which
requires `EventStream` plumbing — costlier but still feasible
via `iced::advanced::subscription::Event` empty stream as Wave 1
demonstrates). Cost upside: ~0.5 day.

## Q — Open operator decisions

**NONE.** Q1 = 0 by construction. This brief is a refactor +
test addition; no operator decision is required.

- The fix shape (extract `stream_impl`, mirror Wave 1) is
  determined by precedent.
- The test count (4-5) is determined by the Wave 1 sibling
  templates.
- R3 (spec-lint rule) is conditional on Wave 1 having shipped
  the rule — architect M-T1 confirms via grep, default to R3.a
  (defer).
- The architect's R1 (a) vs (b) (where to put `stream_impl`) is
  an internal-architecture decision the architect owns; no
  operator routing needed.

Standing Autoapprove applies trivially: there is nothing to
auto-approve, because there is nothing for the operator to
choose.

## Out of scope at v0.1.0

- Authoring the `subscription-missing-e2e` spec-lint rule from
  scratch (R3.a). Forward-listed; defer to a later brief.
- Moving `ServerTimeRecipe` to a new `crates/ui/src/live/server_time.rs`
  module (R1 option (b)). Architect M-T1 may revisit if the bin
  vs lib import shape proves awkward; default (a) is in tree
  today.
- Making the clock cadence operator-configurable (K2 forward-
  risk). Process-lifetime 1 Hz is the contract.
- Salting the `ServerTimeRecipe` hash (K2). Out of scope.
- Touching `cockpit_live.rs::subscription()` batch composition
  (lines 1401-1469). Out of scope.
- ActivityRecipe / LabProgressRecipe / TrailMirrorRecipe tests.
  Wave 1 owns those; this brief only extends with the 4-5 new
  ServerTimeRecipe tests.

## Non-regression contract — explicit (M-FINAL gate)

| ID | Assertion | How verified |
|----|-----------|--------------|
| NR-1 | 34/34 anchors PASS byte-identically. | `scripts/verify_anchors.sh` → `ANCHORS PASS (34 / 34)`. |
| NR-2 | Workspace test count delta = +4 or +5. | `cargo test --workspace --all-targets` summary table vs the baseline. |
| NR-3 | `lab_progress_recipe_stream.rs` + `trail_mirror_recipe_stream.rs` byte-identical (Wave 1 untouched). | `git diff` on those files = empty at M-FINAL. |
| NR-4 | `cockpit_live.rs::subscription()` batch composition byte-identical (lines 1401-1469). | `git diff` on those lines = empty (only the `Recipe::stream` body delegation changes; the subscription-construction call site is the same). |
| NR-5 | `Recipe::hash` impl byte-identical (lines 138-142). | `git diff` on those lines = empty. |
| NR-6 | `cargo clippy --workspace --all-targets --all-features -- -D warnings` PASS. | `rust-validate` skill. |
| NR-7 | `cargo fmt --all -- --check` PASS. | `rust-validate` skill. |
| NR-8 | spec-lint contribution = 0 new errors / 0 new warnings on this brief's slug. | `uv run scripts/spec_lint.py` baseline-comparison. |

## Verdict routing tree (presenter inherits)

- **R-O1** — refactor + tests land cleanly; all 4-5 new tests
  PASS; NR-1..NR-8 green; cockpit-live boot smoke shows clock
  advancing → **SHIP**.
- **R-O2** — refactor lands but one of T-ST-1a/T-ST-1d fails →
  architect reverts the `stream_impl` extraction; tests stay
  in tree against the original inline path (uses
  `iced::advanced::subscription::Event` empty stream like Wave
  1's `_event_stream` line); cost rises ~0.5 day; route to
  re-ship.
- **R-O3** — refactor lands; tests PASS; cockpit-live boot
  smoke shows clock NOT advancing → P1 regression; architect
  re-spawn; H1 was wrong; tester writes incident note. Low
  probability per the Wave 1 precedent.

## Cost framing

**~0.5 day end-to-end wall-clock.** Breakdown:

| Stage | Owner | Cost |
|-------|-------|------|
| M0 (this brief) | analyst | done (~30 min, this pass) |
| M-OD | operator | NONE (Q1 = 0) |
| M-T1 | architect | ~30 min (R1 (a) vs (b) decision; R3 grep; minor) |
| M-DEV — Wave A | developer | ~2 h (refactor + author 4-5 tests + run gates) |
| M-FINAL | tester | ~30 min (workspace gates + count delta) |
| Presenter | presenter | ~30 min (deck — small enough to skip optional) |

No LLM costs. Pure source patch + test authoring.

## Cross-references

- **Predecessor (Wave 1 templates)** —
  [`crates/ui/tests/lab_progress_recipe_stream.rs`](../../crates/ui/tests/lab_progress_recipe_stream.rs)
  (4 tests) +
  [`crates/ui/tests/trail_mirror_recipe_stream.rs`](../../crates/ui/tests/trail_mirror_recipe_stream.rs)
  (4 tests). Shape this brief mirrors verbatim.
- **Architecture audit** —
  [`spec/dev-notes/testing-framework-audit-2026-05-25.md § R1`](../dev-notes/archive/2026-Q2/testing-framework-audit-2026-05-25.md)
  ("Promote channel-recipe-state-widget end-to-end to a
  first-class layer with a `subscription-pipe` skill"). This
  brief closes one of three identified Recipe surfaces (the
  other two shipped Wave 1).
- **K8 pattern reference** —
  [`crates/ui/src/lab/progress.rs`](../../crates/ui/src/lab/progress.rs)
  lines 70-83 (the canonical "enter runtime, drop guard, then
  Box::pin" pattern).
- **Refactor target** —
  [`crates/ui/src/bin/cockpit_live.rs`](../../crates/ui/src/bin/cockpit_live.rs)
  lines 129-174 (`ServerTimeRecipe`).
- **Carve-out context** — operator decision 2026-05-26 to defer
  ServerTimeRecipe from Wave 1 because cockpit-activity-status-bar
  v0.1.0 had pending edits in `cockpit_live.rs`. Now SHIPPED
  (backlog Recent section).

## Changelog

- 2026-05-26 (analyst, M0) — v0.1.0 brief authored as a Wave 1
  follow-on closing the ServerTimeRecipe carve-out. R1-R5 + K1-K2
  + H1 + non-regression contract + verdict tree + cost framing.
  ZERO operator-decide Qs (Q1 = 0). Standing Autoapprove applies
  trivially. HANDOFF → developer (architect M-T1 small enough to
  skip — architect may invoke if R1 (a) vs (b) needs review).
- 2026-05-26 (developer, M-DEV) — Wave A complete. R1 option (b)
  chosen: `server_time_stream_impl` lives in `crates/ui/src/live.rs:780`
  (library), not inline in the bin — cleaner import path for tests.
  `Recipe::stream` in `cockpit_live.rs:150` delegates via
  `ui::live::server_time_stream_impl(&self.rt_handle)`. 4 tests authored
  and all pass. 34/34 anchors confirmed byte-identical. HANDOFF → tester.

## Implementation

**Developer M-DEV — 2026-05-26 (Wave A)**

### R1 — Refactor

- **`crates/ui/src/live.rs:780`** — `pub fn server_time_stream_impl(rt_handle: &tokio::runtime::Handle) -> BoxStream<'static, Message>` extracted. K8 EnterGuard pattern preserved: guard created and dropped in a `{ ... }` block ending before `Box::pin(...)`. `#[must_use]` attribute applied per clippy `must_use_candidate` lint.
- **`crates/ui/src/bin/cockpit_live.rs:150`** — `Recipe::stream` body collapsed to single delegation call: `ui::live::server_time_stream_impl(&self.rt_handle)`. `Recipe::hash` at lines 137-141 byte-identical (no touch). Unused `use trading_core::Timestamp;` import removed (line was previously 85; Timestamp usages now all fully qualified).
- **Architecture choice**: R1 option (b) — helper in `crates/ui/src/live.rs` (library, importable as `ui::live::server_time_stream_impl`), mirroring `trail_mirror_stream_impl` and `activity_stream_impl` in the same file. R1 option (a) (inline in bin) was skipped because bin items are not importable from integration tests.
- **Diff size**: ~25 LoC added to `live.rs`; ~12 LoC removed from `cockpit_live.rs` (stream body replaced by 1-line delegation + comments); 1 unused import removed.

### R2 — New test file

- **`crates/ui/tests/server_time_recipe_stream.rs`** — 4 tests created under `#![cfg(feature = "live")]` gate:
  - `server_time_stream_impl_yields_tick` (T-ST-1a) — happy path, first tick within 1.5 s
  - `server_time_stream_impl_emits_at_1_hz_cadence` (T-ST-1b) — monotonicity: t2 >= t1
  - `server_time_stream_impl_stream_remains_open` (T-ST-1c) — stream alive after 3 ticks
  - `server_time_stream_impl_recipe_path_end_to_end` (T-ST-1d) — full Recipe delegation path
  - T-ST-1e (optional lag handling) deferred per spec.

### R3 — spec-lint rule

- R3.a path taken. `subscription-missing-e2e` rule not present in Wave 1 (grep confirmed zero matches). No edits to spec-lint files. Deferred to a later brief per spec.

### Gates confirmed locally

| Gate | Result |
|------|--------|
| `cargo test -p ui --features live --test server_time_recipe_stream` | `4 passed; 0 failed; finished in 3.21s` |
| `cargo fmt --all -- --check` | PASS (exit 0) |
| `bash scripts/verify_anchors.sh` | `ANCHORS PASS (34 / 34)` |
| Workspace test count delta | +4 (tester to confirm exact baseline delta at M-FINAL) |
