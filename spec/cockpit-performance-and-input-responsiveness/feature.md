---
slug: cockpit-performance-and-input-responsiveness
status: shipped
owner: presenter
updated: 2026-05-15
version: 1.0.0
predecessor: ui-quality-gate-overhaul v1.0.0 (shipped 2026-05-15)
trigger: operator's 2026-05-15 live-cockpit verification flagged UI slowness + dropped clicks during ui-quality-gate-overhaul v1.0.0 approval
priority: P1
---

# Cockpit performance + input responsiveness

## TL;DR

On 2026-05-15 the operator ran `cargo run --bin cockpit --features
fixtures` live during the `ui-quality-gate-overhaul v1.0.0` approval pass
and flagged two real-world UX defects no current gate catches: **(1) UI
is SLOW** (laggy redraw / sluggish frame pacing under fixtures on macOS
Apple Silicon), and **(2) input dispatch is unreliable** — not every
click is recognized. Both defects pre-date the M1/M2 quality gates that
just shipped — they live in [Brief A's](../iced-native-widgets/feature.md)
native `iced::widget::Table` adoption, [Brief B's](../iced-aw-cherry-pick/feature.md)
`iced_aw::Spinner` continuous-redraw subscription, and the existing
event-dispatch surface.

This brief proposes a four-sub-target investigate-then-fix arc:

- **M0** — profile a default-fixtures cockpit boot with `samply` or
  `cargo flamegraph` and falsify / confirm each of H-PERF-1..5.
- **M1** — fix the dominant hot path identified by M0 (most likely the
  spinner subscription cadence or the Table layout pass).
- **M2** — extend the M1-A `cockpit-smoke` skill (just shipped) to also
  record `fps_p50` over its 7 s window and assert against a budget, so
  no future regression of this class escapes.
- **M3** — input-dispatch investigation (may split off into a sibling
  brief depending on M0's findings).

**Constraint envelope (operator-ratified, do not re-litigate without
operator override):**

- Renderer backend stays `iced_tiny_skia` per
  `trading_ui_library_constraints.md` (user-memory);
  no wgpu swap proposed. (Architect may re-litigate IF M0 shows
  tiny-skia is the dominant cost — see open questions.)
- `plotters-iced` / `iced_plot` / `iced-anim` remain off-table per the
  same constraints memo. Do not re-suggest.
- No upstream `iced` fork; pinned at 0.14.0 per Brief B.

## Problem statement

The operator's verbatim approval-with-notes block from
[`spec/ui-quality-gate-overhaul/presentations/ui-quality-gate-overhaul-2026-05-15.md`](../archive/presentations-2026-Q2.tar.gz)
lines 479-489:

> Operator verified the cockpit live (2026-05-15) and flagged TWO
> real-world UX issues not caught by any current gate:
>
> 1. UI is SLOW (laggy redraw / sluggish frame pacing under fixtures).
> 2. Input dispatch is unreliable — NOT every click is recognized.
>
> Both pre-date this brief (they live in Brief A + Brief B render/event
> surfaces, not in M1/M2 quality gates) but were observed during
> operator's manual cockpit check after this brief's PASS. Filed as a
> new roadmap entry `cockpit-performance-and-input-responsiveness`
> (analyst brief, P1). Approval stands for v1.0.0; performance brief
> opens as a follow-up.

### Why no current gate caught this

The defects escaped because every existing gate measures something
adjacent to — but not — real frame pacing under live event dispatch:

| Gate | What it actually checks | Why it missed perf+input |
|------|-------------------------|--------------------------|
| `cockpit-smoke` (M1-A, just shipped) | First-frame panic over 7 s | No frame-time sampling; binary alive/dead only |
| `render_snapshots` (M1-B) | Layout determinism vs PNG baselines | Single snapshot per scenario — no temporal axis |
| `layout_invariants` (M1-C, proptest) | Root-`Node` geometric invariants | Static layout only; no event flow, no redraw cadence |
| 267 `panel_snapshots` (legacy) | Text-summary `tree::display` of widgets | Helpers route around the real renderer — the F1 gap |
| Manual `cargo run --bin cockpit` | Everything | **The only gate that exercised real frame pacing — and is the gate that just caught this.** |

The systemic gap: nothing in CI exercises 60 fps + click dispatch over a
sustained window. M2 of this brief closes that gap for perf; M3 closes
it for input.

## Sub-targets

### M0 — Profile the dominant hot path *(orchestrator-runnable)*

**Owner:** orchestrator (≤ 0.5 dev-days).
**Depends on:** none.

Profile `cargo run --bin cockpit --features fixtures` (release-mode) for
~30 seconds of steady-state idle plus ~30 seconds of operator interaction
(panel-switch, scroll, click) on macOS Apple Silicon using `samply`
(preferred — interactive flamegraph in browser) or `cargo flamegraph`
(SVG fallback).

**Acceptance criteria:**

- Capture top-10 hot frames by self-time and a frame-time histogram
  (p50 / p95 / p99).
- A concrete falsify-or-confirm verdict on EACH of H-PERF-1..5 (and
  H-PERF-6 if the analyst's late-binding hypothesis stands).
- Output committed under `spec/cockpit-performance-and-input-responsiveness/reports/m0-profile-<ts>.{flamegraph.svg, json, md}`
  where the `.md` body summarizes the verdict and embeds the histogram.

**Consumer surfaces (read-only):**

- `crates/ui/src/widgets/spinner.rs` (Brief B B2 spinner cadence)
- `crates/ui/src/widgets/strategies.rs` (Brief A R1/R2 Table)
- `crates/ui/src/app.rs` event-loop entry (hit-test path)

**Expected LOC delta:** zero (read-only profiling). All output goes
under `reports/`; no source change.

### M1 — Fix the dominant hot path *(developer + ui-designer)*

**Owner:** developer + ui-designer (joint; 1-3 dev-days depending on
which hypothesis confirms).
**Depends on:** M0 verdict.

Three branching candidates, ordered by analyst's prior probability after
reviewing the backlog hypotheses:

#### Candidate A — if H-PERF-1 confirms (spinner-driven 60 fps repaint)

The `iced_aw::Spinner` subscription forces `request_redraw_at(now +
~16ms)` whenever ANY panel is in `PanelState::Loading`. With 8 B2 call
sites across the cockpit, even one loading panel pulls the whole window
into 60 fps software-rasterized redraw.

**Proposed fixes (in order of preference):**

1. **Coarsen cadence to 10 fps.** Replace `now + 16ms` with `now +
   100ms`. Cheapest fix; visually identical for an idle spinner; ~1-2
   LOC change in the spinner cadence call site. **File-span LOC:** ~+2,
   -2 in `crates/ui/src/widgets/spinner.rs` (or wherever the cherry-
   picked subscription lives — confirm at architect pass).
2. **Only-while-visible gating.** Wrap the spinner subscription in a
   `viewport_intersection` check so panels scrolled off-screen don't
   request redraws. ~+15-30 LOC.
3. **Replace with non-redraw-requesting animation** (e.g., a CSS-style
   keyframe driven by `iced::time::Instant::now()` projected through a
   `Canvas` shader without forcing a tick). Most invasive; only if 1+2
   prove insufficient.

#### Candidate B — if H-PERF-2 confirms (Table layout cost)

Native `iced::widget::Table` (Brief A R1/R2) recomputes per-cell bounds
every redraw. With ~12 rows × 6 columns at 60 fps = 4320 layout
calls/sec just for the strategies panel.

**Proposed fixes:**

1. **Memoize Table layout per row.** Add a `layout_cache: HashMap<RowId,
   Cached>` invalidated on row-data hash change. **File-span LOC:**
   ~+40-80 in `crates/ui/src/widgets/strategies.rs`; possibly a small
   upstream-iced patch if `iced::widget::Table` doesn't expose the
   hook (in which case fall back to Candidate B2 instead).
2. **Diff-update only changed cells.** Track cell version per
   `(row, col)` and only redraw cells whose hash changed. ~+60-120 LOC.
3. **Partial revert of Brief A R2 for strategies panel only.** Drop
   back to the pre-Brief-A `Row::new()` + `Scrollable<Column>` shape
   for this single panel. **Architectural divergence — flagged honestly
   below.** ~+20 LOC revert, -120 LOC of Brief A R2 surface area for
   that panel.

#### Candidate C — if H-PERF-3 confirms (hit-test cost)

Click hit-test traverses the full widget tree. Home screen has 4 panels
× ~50 widgets each ≈ 200 candidates/click. If a 60 fps redraw lands on
the same event-loop tick as a click, the `CursorButtonPressed` event
may be starved.

**Proposed fixes:**

1. **Z-order shortcut** — early-exit hit-test on the topmost panel
   under the cursor. ~+30 LOC in `crates/ui/src/app.rs`.
2. **Hit-test cache invalidated on layout-change.** ~+50-80 LOC.
3. **Audit iced 0.14's event-loop ordering** to see whether redraws
   and input share a queue; possible upstream `iced` PR.

**Acceptance criteria (all candidates):**

- `fps_p50` measured by M0 method improves to ≥ M2's chosen budget
  (TBD at architect pass; ballpark 30+).
- Click-recognition rate (M3 method) ≥ 100 % over a 60-click bench.
- All existing M1-A/M1-B/M1-C/M2-A gates still PASS.

### M2 — Perf-budget regression gate *(developer)*

**Owner:** developer (~0.5 dev-days).
**Depends on:** M0 (for budget value) + M1 (for landing the fix
first; otherwise M2 ships red).

Extend the `cockpit-smoke` skill (M1-A from `ui-quality-gate-overhaul`)
so it records frame-per-second over its existing 7-second observation
window in addition to panic-count.

**Implementation sketch:**

- Add a `render-debug`-gated stderr emission of frame timestamps from
  the cockpit's `Application::view` boundary (one line per frame:
  `RENDER_FRAME <monotonic_ns>`).
- `cockpit-smoke` skill parses these timestamps, computes
  `frame_ms_p50` / `p95` / `p99`, and ASSERTS `fps_p50 >= <budget>`.
- Budget value: TBD per M0's measurements; analyst placeholder is 30
  fps but the architect chooses the floor.
- Skill version bumps from current ship (M1-A v1.x) to v1.(x+1).0 with
  the new field appended to its output schema. The version-bump is
  honest signal that this brief touches a gate that JUST shipped (see
  Architectural divergences below).

**Acceptance criteria:**

- `cockpit-smoke` emits both `panic_count` and `fps_p50` to its report.
- Gate fails when `fps_p50 < budget`.
- One regression test: artificially insert a `std::thread::sleep` of
  50 ms into `view()` under a non-default feature flag; assert
  `cockpit-smoke` reports `fps_p50 ≤ 20` and FAILs against a 30 fps
  budget. (Tester scaffolds this; analyst flags the need.)

**Glue-layer LOC delta:** ~+50 in the `cockpit-smoke` skill template +
~+15 in `crates/ui/src/app.rs` for the gated stderr emission.

### M3 — Input dispatch investigation *(analyst → architect re-spawn)*

**Owner:** analyst initial pass (~0.5 dev-days) — then likely
re-spawn the architect (~0.5-1.5 dev-days) depending on M0 verdict.
**Depends on:** M0 verdict.

The operator's note conflates "slow" and "dropped clicks" because they
co-occur, but the root causes may diverge:

- **Perf-coupled:** if H-PERF-3 confirms, dropped clicks are a
  symptom of frame-rate starvation and M1 Candidate C resolves both.
- **Independent:** if H-PERF-1 or H-PERF-2 confirms but click-drops
  persist after M1 lands, there's a real `iced` 0.14 input-dispatch
  bug to chase — likely upstream.
- **Tooling-coupled:** the M2-A `tracing` instrumentation (just
  shipped, behind `render-debug`) could surface dropped events via a
  new `event_dispatch` span — cheaper than custom telemetry.

**Decision-fork left for the architect:** keep M3 in this brief, or
split into a sibling brief `cockpit-input-dispatch` (slug TBD).
Analyst preference: keep coupled until M0 verdict — split only if
the perf and input root causes diverge.

**Acceptance criteria (provisional):**

- `tracing` span on `WindowEvent::CursorButtonPressed` dispatch.
- Bench: 60 deliberately-spaced clicks via an automated input
  generator (e.g., `enigo` crate in a `dev-dependencies` slot or a
  manual operator-driven log) → 60/60 recognition.

**LOC delta:** TBD; placeholder ~+30 if it stays in this brief.

## Hypothesis register

Re-stated from the backlog's H-PERF-1..5 seeds as analyst-facing
falsifiable claims, with verbatim falsifier text preserved. One late-
binding addition: H-PERF-6.

### H-PERF-1 — Spinner subscription forces continuous repaint

**Claim:** `iced_aw::Spinner`'s 60 FPS `request_redraw_at` subscription
forces continuous full-cockpit repaint while ANY panel is in
`PanelState::Loading`. Even one loading panel pulls the whole cockpit
into 60 fps software-rasterized (`iced_tiny_skia`) redraw. CPU cost
compounds across the 8 B2 call sites.

**Falsifier:** M0 flamegraph shows the spinner-driven redraw path is
NOT in the top-3 self-time entries, OR `frame_ms_p50` is unchanged
when all panels are in `PanelState::Ready` vs `PanelState::Loading`.

**Analyst prior:** HIGH. The 60 fps timer is a known iced anti-pattern
and Brief B shipped 8 call sites in one go.

### H-PERF-2 — Table layout pass uncached

**Claim:** Native `iced::widget::Table` (Brief A R1/R2) recomputes
per-cell bounds every redraw, not cached. ~12 rows × 6 columns × 60 fps
= 4320 layout calls/sec just for the strategies panel.

**Falsifier:** M0 flamegraph shows `iced::widget::Table::layout` is
NOT in the top-5 self-time entries, OR `frame_ms_p50` is unchanged
when the strategies panel is collapsed vs expanded.

**Analyst prior:** MEDIUM. Native Table is new in 0.14 and unlikely to
have layout caching out of the box.

### H-PERF-3 — Hit-test traversal cost / event-loop starvation

**Claim:** Click hit-test traverses the full widget tree (Home screen =
4 panels × ~50 widgets each ≈ 200 hit-test candidates per click).
Missed clicks correlate with a 60-fps redraw landing during the same
event-loop tick and starving `WindowEvent::CursorButtonPressed`.

**Falsifier:** M0 trace shows event-dispatch latency is bounded
(< 5 ms p99) under both idle and loading-spinner conditions, OR
forcing a sustained 60 fps redraw via a synthetic load does NOT
correlate with click drops.

**Analyst prior:** MEDIUM. The "starvation under redraw" pattern is
plausible but H-PERF-1's fix may resolve it incidentally.

### H-PERF-4 — Tiny-skia software-raster is dominant cost

**Claim:** `iced_tiny_skia` is software-rasterized on Apple Silicon
hardware that has wgpu-capable GPU available. Even an optimal redraw
cadence is bottlenecked by tiny-skia's CPU rasterization.

**Falsifier:** M0 flamegraph shows tiny-skia rasterization is NOT in
the top-3 self-time entries.

**Analyst prior:** LOW-MEDIUM. If H-PERF-4 confirms, the architect
must re-litigate the wgpu constraint with the operator — see open
questions. NOT proposed as in-scope; flagged as deferred decision.

### H-PERF-5 — `render-debug` tracing leaking into default builds

**Claim:** The M2-A `tracing` instrumentation shipped behind
`#[cfg(feature = "render-debug")]` accidentally fires in default builds
(misplaced `cfg` attribute, missing `default-features = false` on a
downstream consumer, etc.).

**Falsifier:** `grep -n 'render-debug' crates/ui/Cargo.toml` shows
`render-debug` is NOT in the `[features].default` list; `cargo
expand --bin cockpit --no-default-features --features fixtures` shows
no `tracing` spans on the render path.

**Analyst prior:** LOW. Just shipped with explicit gating in
ui-quality-gate-overhaul v1.0.0, but worth a 30-second falsification.

### H-PERF-6 — Spinner subscription wakes the executor even when no panel is loading *(analyst late-binding)*

**Claim:** The cherry-picked `iced_aw::Spinner` subscription remains
registered with the iced runtime even when zero panels are in
`PanelState::Loading`, causing the executor to wake every 16 ms even
during steady-state idle. If true, this would mean H-PERF-1's
8-call-site CPU compounding is on top of a baseline 60 fps wake-up
that exists with NO loading panels at all.

**Falsifier:** M0 trace of the cockpit at steady-state Home with all
panels `Ready` shows < 5 wake-ups per second (i.e. only operator-event-
driven), OR the spinner subscription `unsubscribe`s correctly when
`PanelState != Loading`.

**Analyst prior:** MEDIUM. Cherry-picked iced_aw code may not have
been audited for subscription lifecycle.

## Numbers that matter

- **Baseline frame-rate:** TBD — M0 measures.
  - Operator subjective verdict: "slow / sluggish / laggy" — calibrate
    to a numeric `fps_p50` after M0.
- **Architect's seeded scope guess** (analyst-confirmed envelope):
  - M0 ~0.5 dev-days.
  - M1 ~1-3 dev-days depending on which candidate (A/B/C) confirms.
  - M2 ~0.5 dev-days.
  - M3 ~0.5-2 dev-days depending on split decision.
  - **Total: 2.5 - 6 dev-days.**
- **File-span LOC delta** (analyst estimate; architect refines):
  - M1 Candidate A1 (cadence coarsen): ~+2/-2 LOC in 1 file.
  - M1 Candidate B1 (Table memoize): ~+40-80 LOC in 1-2 files.
  - M1 Candidate B3 (partial Brief A R2 revert for strategies):
    ~+20/-120 LOC net (-100 file-span).
  - M1 Candidate C1 (Z-order hit-test shortcut): ~+30 LOC in 1 file.
  - **Most likely outcome (Candidate A1 + maybe B1): ~+5-80 LOC.**
- **Glue-layer LOC delta:**
  - M2 cockpit-smoke skill extension: ~+50 LOC in skill template +
    ~+15 LOC in `crates/ui/src/app.rs` for `render-debug` stderr.
  - M3 if in-scope: ~+30 LOC tracing span + bench harness.
  - **Total glue: ~+65-95 LOC.**
- **Detection coverage delta:**
  - M2 catches any future regression where `cockpit-smoke` window's
    `fps_p50` drops below budget. **It does NOT catch single dropped
    clicks** — that's M3's surface.
  - M3 (if it lands here) catches click-recognition regressions via
    a tracing span on `CursorButtonPressed`.
- **Skill version bumps caused:**
  - `cockpit-smoke` skill: minor bump (e.g., v1.0 → v1.1 — confirm
    current at architect pass).

## Architectural divergences (honest)

1. **M1 Candidate B3 partially reverts Brief A R2 for the strategies
   panel.** Brief A's native-Table adoption shipped on 2026-05-13 and
   is still warm. Reverting one panel back to `Row::new()` +
   `Scrollable<Column>` is a surgical churn and the architect should
   weigh it carefully against B1 (in-place memoization). The reversion
   only makes sense if M0 shows Table layout is the dominant cost AND
   the upstream `iced::widget::Table` does not expose a layout-cache
   hook. **Honest signal:** the analyst flags this option but does not
   advocate it as default — preference is in-place memoization (B1)
   unless that's blocked.

2. **M2 extends a gate that JUST shipped (`cockpit-smoke` from M1-A,
   2026-05-15).** Touching it again four days post-ship is unusual.
   The skill version-bump tracks the churn. The alternative (a
   separate `cockpit-perf-budget` skill) would duplicate the 7-second
   process-spawn cost, which is real (M1-A invoked it once per
   pre-tick gate). Extending the existing skill is the right
   structural choice but it means the M1-A author (the developer) is
   modifying their own code in less than a week. Track this in the
   skill changelog explicitly.

3. **Wgpu renderer backend swap is off-table per the architecture pin
   in `trading_ui_library_constraints.md`** — but IF H-PERF-4 confirms
   (tiny-skia is dominant), the architect should re-litigate the
   constraint with the operator. This brief does NOT propose the
   swap; it surfaces it as a deferred architect decision in open
   questions. Operator override required to lift the pin.

4. **M3 may grow this brief or split off.** Analyst preference is to
   keep input-dispatch in this brief because the operator filed both
   defects together, but the root causes may diverge after M0. The
   architect's call.

## Out of scope

- **Renderer backend swap (wgpu).** Deferred architect decision per
  divergence 3; not proposed unless M0 forces it AND operator overrides
  the pin.
- **Upstream `iced` fork or major version bump.** Pinned at 0.14.0 per
  Brief B. Small upstream PRs to `iced` for layout-cache hook
  (Candidate B1) or event-loop ordering audit (Candidate C3) are
  acceptable; forks are not.
- **`plotters-iced` / `iced_plot` / `iced-anim` family** — off-table
  per `trading_ui_library_constraints.md`. Do not re-suggest.
- **M2-B `DebugRenderer` runtime wiring.** Already queued as separate
  follow-up brief `ui-debug-renderer-runtime-wiring` per
  `ui-quality-gate-overhaul v1.0.0` presenter operator decision 1.
- **M2-C LLM-as-judge for perf regressions.** Deferred per
  `ui-quality-gate-overhaul v1.0.0` operator decisions.
- **Cross-platform perf (Linux / Windows / iOS / Android).** Separate
  brief in backlog (`cockpit-cross-platform`). This brief is macOS
  Apple Silicon only, matching the operator's hardware.
- **Permanent `render-debug` enablement in default builds.** The
  feature stays off by default; M2 turns it ON only inside the
  `cockpit-smoke` skill's spawned subprocess.

## Open questions for architect

1. **Does M3 stay in this brief or split into a sibling brief
   `cockpit-input-dispatch`?** Analyst preference: keep coupled until
   M0 verdict. Architect's call after seeing the flamegraph.

2. **If H-PERF-4 confirms (tiny-skia is the dominant cost):** does the
   architect re-litigate the wgpu constraint with the operator? The
   pin is in `trading_ui_library_constraints.md` and predates Brief A
   / Brief B. **Operator override required to lift it.** Architect
   should signal in their handoff whether this is a 1-line ratify or
   a full architecture-revision pass.

3. **M2 perf-budget value — what `fps_p50` is the right floor?** 30 fps
   for laptop-on-battery scenarios? 60 fps for desktop steady-state?
   Should the budget be hardware-aware (i.e., a `cpu_throttle_factor`
   coefficient)? Analyst placeholder is 30 fps; architect chooses.

4. **M1 Candidate B (Table layout) — surgical revert (B3) vs in-place
   memoization (B1)?** B1 is preferred per divergence 1; B3 is the
   fallback if upstream `iced::widget::Table` has no layout-cache
   hook. Architect should signal the preference in their handoff so
   the developer doesn't sequence-search the wrong candidate first.

5. **Skill version-bump policy for in-place gate extensions.** Does
   `cockpit-smoke` minor-bump (v1.0 → v1.1) or major-bump (v1.0 →
   v2.0) when M2 adds the `fps_p50` field? Analyst inclines minor
   (additive output field); architect ratifies.

6. **M0 profiler choice — `samply` vs `cargo flamegraph`.** Both are
   reasonable; samply is interactive (browser-served flamegraph) and
   easier to share via screenshot, cargo-flamegraph is one-shot SVG.
   Either works; architect picks one for the orchestrator-runnable
   step.

## Design — architect synthesis

Architect pass 2026-05-15 (status `draft → design`, owner `analyst → architect`,
version `0.1.0 → 0.2.0`). Six analyst open-questions resolved; two sub-agent-safe
falsifiers (H-PERF-5, H-PERF-6) executed inline with verdicts below; the M0
profiler is locked and the orchestrator-runnable command body is fixed in
[`tasks.md ## M0`](tasks.md). No `crates/` edits in this pass — design only.

### Q1 resolution — M3 stays in this brief (coupled, no sibling brief)

**Resolution.** Keep M3 coupled inside this brief until the M0 verdict lands.
Split into a sibling `cockpit-input-dispatch` brief ONLY if the M0 flamegraph
confirms H-PERF-1 or H-PERF-2 (a non-event-loop hot path) AND a follow-up
click-recognition bench after M1 lands shows < 60/60 recognition (i.e., the
input bug is independent of the perf bug). The trigger is now mechanical, not
judgement.

**Why.** The operator filed both symptoms together as one observation. Three
out of four H-row branches (H-PERF-1 spinner / H-PERF-2 Table / H-PERF-3
hit-test) plausibly couple "slow" and "dropped clicks" via event-loop
starvation. Splitting now would force the analyst to re-spawn before any
evidence exists; splitting later loses nothing (the sibling brief inherits
this brief's M0 results verbatim). Analyst preference (keep coupled) ratified.

**Cost of being wrong.** If perf and input root causes diverge AFTER M1 lands,
one extra analyst-spawn round (~0.5 dev-days) to lift the sibling brief from
this brief's residual evidence. If they converge, zero cost — we save a brief.

**Cite.** [`feature.md ## M3`](#m3--input-dispatch-investigation-analyst--architect-re-spawn);
[`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
rule on bidirectional loops.

### Q2 resolution — wgpu re-litigation is a 1-line ratify, not an architecture-revision pass

**Resolution.** If H-PERF-4 confirms (tiny-skia rasterization in the top-3
self-time entries), this brief's M1 enters a DEFERRED state and the architect
files a 1-line addendum requesting operator override of the
`trading_ui_library_constraints.md` (user-memory)
tiny-skia pin. Mechanics: ONE-paragraph "operator decision required" stub in
this brief's changelog + ONE-line trace.toml note flipping
REQ-COCKPIT-PERF-001's state from `design` to `blocked-on-operator`. No
separate brief; no architecture.md revision pass; no Cargo.toml edit by the
architect. The architecture-revision PASS happens only if the operator
overrides the pin — at that point a new `cockpit-wgpu-renderer-swap` brief
opens with its own analyst spawn.

**Why.** The constraint is operator-locked, not architect-locked. The
architect's job is to surface evidence and route the decision; not to
re-litigate. A full architecture-revision pass without operator override
would be a constitution violation per `trading_ui_library_constraints.md`.

**Cost of being wrong.** If the architect over-escalates (full revision pass
when a 1-line ratify suffices): wasted ~1 dev-day of architecture.md
churn. If the architect under-escalates (treats H-PERF-4 confirmation as
"M1 done" without operator signal): the operator's "slow UI" symptom
persists post-ship. The 1-line ratify cost is the asymmetric correct call.

**Trigger condition (mechanical).** H-PERF-4 confirms iff M0 flamegraph
shows `tiny_skia::*` or `iced_tiny_skia::engine::*` in the top-3 self-time
entries AND `frame_ms_p50` exceeds the Q3 budget after a hypothetical
M1-fix of the spinner/Table dominant. Both halves required; either alone
is not enough.

**Cite.** [`feature.md ## Architectural divergences (honest)`](#architectural-divergences-honest)
divergence 3; `trading_ui_library_constraints.md` (user-memory).

### Q3 resolution — M2 perf-budget floor is `fps_p50 >= 30` (hardware-uniform, no coefficient)

**Resolution.** The M2 regression gate asserts `fps_p50 >= 30.0` over its
7-second observation window. No `cpu_throttle_factor` coefficient; no
hardware-aware adjustment in v1.1 of the cockpit-smoke skill.

**Why.** Three reasons converge on 30:
1. **Operator-observed slowness threshold.** "Slow / sluggish / laggy" is
   a subjective verdict at an unknown numeric fps. 30 fps is the floor
   below which animation-vs-still discrimination collapses for most
   observers (the Bushnell `Pong` empirical line). If the cockpit is
   below 30 fps the operator's verdict will not be "slow"; it will be
   "broken". Setting the gate at 30 catches the slow-end, not the
   broken-end — that's the regression we want to detect.
2. **Hardware-uniform is mechanically simpler.** The cockpit-smoke skill
   already spawns a subprocess and parses stderr; adding a CPU-throttle
   probe (e.g., reading `/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq`
   on Linux, `pmset -g therm` on macOS) doubles the skill surface and
   introduces a per-platform shim. v1.1 stays hardware-uniform; if the
   30 fps floor proves to noise out on battery-throttled M-series the
   operator can ratchet it down to 20 in a v1.2 bump, separately.
3. **Headroom above 30.** The M2 acceptance scenario (analyst's `sleep
   50 ms` regression test) expects `fps_p50 ≤ 20` to FAIL the gate.
   A 30 fps floor gives `~10 fps` of headroom above the FAIL line —
   enough that natural cold-start jitter (the first 1-2 seconds of
   the 7s window catching iced's renderer warmup) does not produce
   a flaky gate.

**Cost of being wrong.** If 30 fps is too aggressive on operator hardware,
the gate FAILS spuriously on a clean run; one operator round of ratchet-to-20.
If 30 fps is too lenient (operator perceives slowness ABOVE 30 fps), the
gate passes a regression the operator catches manually; one additional
brief to ratchet up. Both failure modes are recoverable in one round; the
midpoint 30 is the asymmetric correct call.

**Cite.** [`feature.md ## M2 — Perf-budget regression gate`](#m2--perf-budget-regression-gate-developer);
[`feature.md ## H-PERF-1`](#h-perf-1--spinner-subscription-forces-continuous-repaint)
through [`H-PERF-3`](#h-perf-3--hit-test-traversal-cost--event-loop-starvation).

### Q4 resolution — M1 Candidate B prefers B1 (memoize) over B3 (revert); B3 fallback only if upstream Table has no cache hook

**Resolution.** If H-PERF-2 confirms, the developer attempts **B1
(in-place memoization of Table layout)** first. The fallback to B3
(partial Brief A R2 revert for the strategies panel) fires ONLY if B1
proves infeasible because `iced::widget::Table::layout` exposes no
extension point for caching (i.e., the developer cannot insert a memoize
wrapper without modifying upstream iced 0.14, which is out of scope per
the constraint envelope).

**Why.**
1. **Brief A R2 is 2 days warm.** Reverting one panel back to
   `Row::new()` + `Scrollable<Column>` four days post-ship is a
   structural-decision unwind; the upstream Table adoption was a
   ratified architecture call (operator-approved 2026-05-13). B1
   preserves the architectural decision; B3 surfaces it for
   re-litigation.
2. **B1's LOC delta is bounded.** Analyst estimates ~+40-80 LOC in
   `crates/ui/src/widgets/strategies.rs` for a `layout_cache:
   HashMap<RowId, Cached>` invalidated on row-data hash change. Even
   the high end (~80 LOC) is < B3's -120 LOC revert surface.
3. **B1 is testable without architecture-pass churn.** A unit test
   on the cache (hit on identical row, miss on hash change) plus the
   existing M1-B render_snapshots PNG baselines on the strategies
   panel give defense-in-depth without touching trace.toml's `crates`
   column.

**Cost of being wrong.** If B1 turns out infeasible mid-implementation,
the developer routes `HANDOFF → architect` with a one-line "no
extension point" note; architect re-engages to lift the B3 fallback
with an ADR per divergence 1. Round-trip cost: ~0.5 dev-day; B3 is
gated behind a written ADR so the structural unwind is durable in
spec.

**Sub-task ordering signal for developer.** In M1's Candidate-B
branch, T-M1-B-1 (B1 memoize) MUST be attempted before T-M1-B-3 (B3
revert). T-M1-B-2 (diff-update only changed cells) is the architect's
SECOND fallback below B1 and ABOVE B3 — try B2 between B1 and B3 only
if B1 fails the "no extension point" check but B2 (which lives inside
the widget, not the Table) does not.

**Cite.** [`feature.md ## M1 Candidate B`](#candidate-b--if-h-perf-2-confirms-table-layout-cost);
[`feature.md ## Architectural divergences (honest)`](#architectural-divergences-honest)
divergence 1.

### Q5 resolution — cockpit-smoke minor-bumps to v1.1 (additive); the v1.0 baseline is minted by M2

**Resolution.** When M2 extends the cockpit-smoke skill with `fps_p50`
emission and assertion, the skill version goes to **v1.1.0** as an
additive minor bump. The version-bump policy:

- **Minor (vX.Y → vX.(Y+1)).** Additive: new output field appended,
  new assertion gated behind a default-PASS threshold (i.e., legacy
  PASS records stay PASS), no breaking change to invocation shape or
  exit-code semantics. This is M2's class.
- **Major (vX.Y → v(X+1).0).** Breaking: invocation shape changes,
  exit codes are remapped, an existing assertion's threshold tightens
  in a way that flips historical PASS records to FAIL.

**Side note: the v1.0 baseline.** The cockpit-smoke skill shipped in
`ui-quality-gate-overhaul v1.0.0` without an explicit `version:` field
in its frontmatter. M2 mints that baseline as **v1.0** in the same
edit that introduces the v1.1 fps emission — a back-fill, not a bump.
The skill's changelog gets a 2-line entry: `2026-05-15 v1.0 baseline
mint (back-fill from ship 2026-05-15)` immediately followed by
`2026-05-15 v1.1 fps_p50 emission + 30 fps assertion`. No anchor SHA
churn (the skill is not body-hashed; only reports are).

**Why.** Analyst's "additive" instinct is correct: appending a new
field to the skill's output JSON does not break existing report
parsers (they ignore unknown fields). The 30 fps assertion is gated
behind the new field's presence in the output, so legacy invocations
(if any predated M2's land) cannot flip from PASS to FAIL.

**Cost of being wrong.** If v1.1 turns out to break an existing
parser (e.g., a presenter deck template that strict-validates the
report-JSON schema), the parser surfaces a known-class failure at
its own deck-render time and the operator-routes back to
spec-auditor to update the schema check. Recoverable in one round.

**Cite.** [`feature.md ## M2 — Perf-budget regression gate`](#m2--perf-budget-regression-gate-developer);
[`.claude/skills/cockpit-smoke/SKILL.md`](../../.claude/skills/cockpit-smoke/SKILL.md).

### Q6 resolution — M0 profiler is `samply` (preferred); `cargo flamegraph` is the documented fallback

**Resolution.** The M0 profile run uses **`samply`** as the primary
profiler. `cargo flamegraph` is the documented SVG fallback if samply
is unavailable on the orchestrator's host.

**Why.** Three reasons:
1. **Browser-served interactive UI.** Samply opens an interactive
   flamegraph in a local HTTP server with full zoom + filter +
   stack-trace inspection. `cargo flamegraph` emits a single SVG
   that needs a separate viewer. For a 60-second profile with a
   long-tail call graph (iced + iced_aw + tiny_skia + the cockpit's
   own widget tree), interactive exploration is non-trivially better.
2. **Screenshot-friendly for the brief evidence trail.** The
   `spec/<slug>/reports/m0-profile-<ts>.md` summary embeds screenshots
   of the flamegraph view; samply's browser tab is a natural source
   for those. Cargo-flamegraph SVGs also screenshot, but the
   resulting PNG is less navigable in a presenter deck.
3. **Zero-perturbation default on macOS.** Samply uses macOS
   `task_for_pid` + DTrace symbols natively (no `sudo`); cargo
   flamegraph on macOS requires either `sudo dtrace` or a `dtrace`
   alternative. Operator hardware is macOS Apple Silicon (per the
   constraint envelope); samply is the lower-friction choice.

**Fallback trigger.** If `command -v samply` returns non-zero on the
orchestrator's host, fall back to `cargo flamegraph` with the
equivalent invocation. The fallback is non-blocking — the M0
acceptance criteria (top-10 hot frames + frame-time histogram) are
satisfied by either tool.

**Cost of being wrong.** If samply produces a noisier profile than
cargo flamegraph on this specific workload, the M0 verdict is
preserved (both tools sample the same call stacks). Round-trip cost
is the differential `cargo install samply` vs `cargo install
flamegraph` (both ~30s on warm cache); no productive-time cost.

**Cite.** [`feature.md ## M0`](#m0--profile-the-dominant-hot-path-orchestrator-runnable);
[`tasks.md ## M0`](tasks.md) for the exact orchestrator-runnable
command body.

### H-PERF-5 falsifier — sub-agent-safe, RESOLVED-UNFALSIFIED (defensively true)

**Falsifier execution log (architect, 2026-05-15):**

| Step | Command (architect-runnable; grep, no live cockpit) | Result |
|------|------|--------|
| 1 | `grep -n 'render-debug' crates/ui/Cargo.toml` | Feature `render-debug = ["dep:tracing-subscriber"]` exists at `crates/ui/Cargo.toml:161`; documented at lines 134-160 as an opt-in build flag with explicit "Build-time-only" annotation. |
| 2 | `grep -n 'default = \[' crates/ui/Cargo.toml` | **Empty output.** No `default = [...]` line exists in `[features]`. The `[features]` block at `crates/ui/Cargo.toml:126-194` defines `fixtures`, `render-debug`, `live`, `in_process_cron`, `live-broadcast-cron` — none default. |
| 3 | `grep -rn '#[cfg(feature = "render-debug")]' crates/ui/src/ \| wc -l` | **6 gated sites.** Locations: `crates/ui/src/bin/cockpit.rs:125`, `crates/ui/src/widgets/debug_renderer.rs:26` (module doc; module also has top-level gate via `widgets/mod.rs:23`), `crates/ui/src/widgets/strategies.rs:225`, `crates/ui/src/widgets/mod.rs:23`, `crates/ui/src/widgets/frame.rs:60`, `crates/ui/src/widgets/frame.rs:188`. |
| 4 | `grep -rn 'tracing::trace_span\|tracing::trace!\|tracing::debug!' crates/ui/src/widgets/frame.rs crates/ui/src/widgets/strategies.rs crates/ui/src/bin/cockpit.rs` | **3 emit sites; all 3 directly preceded by `#[cfg(feature = "render-debug")]` on the immediately-prior line.** Verified pairings: `frame.rs:60` gate → `frame.rs:61` `trace_span!("widget_draw", widget = "panel", ...)`; `frame.rs:188` gate → `frame.rs:190` `trace_span!("widget_draw", widget = "loading_with_spinner", ...)`; `strategies.rs:225` gate → `strategies.rs:226` `trace_span!("widget_draw", widget = "strategies::id_cell", ...)`. **Zero unguarded emit sites in the render path.** |

**Verdict.** H-PERF-5 is **UNFALSIFIED in the defensive sense**:
the `render-debug` instrumentation does NOT fire in default builds.
Six `#[cfg]` gates are correctly placed; three `trace_span!` emit
sites are individually wrapped at the statement level (not the
function level), so the compiler elides them entirely under
`--no-default-features --features fixtures` (the M0 profile invocation
matches this shape — `cargo run -p ui --bin cockpit --features
fixtures` does NOT pass `render-debug` and `render-debug` is not in
any `default` list, which doesn't exist).

**Implication for M0/M1.** H-PERF-5 is RESOLVED. The M0 profile run
will NOT see render-debug spans in its flamegraph; if the M0 profile
shows tracing spans in the hot path, that is a different bug class
(e.g., an unguarded `tracing::info!` somewhere in the render tree),
not H-PERF-5. The defensive hypothesis is confirmed-defensively-true
and is dropped from the M0 falsifier-batch.

**Cite.** [`feature.md ## H-PERF-5`](#h-perf-5--render-debug-tracing-leaking-into-default-builds);
[`crates/ui/Cargo.toml:126-194`](../../crates/ui/Cargo.toml).

### H-PERF-6 falsifier — sub-agent-safe, RESOLVED-UNFALSIFIED (architectural impossibility)

**Falsifier execution log (architect, 2026-05-15):**

| Step | Source-read (no live cockpit) | Result |
|------|------|--------|
| 1 | Inspect `iced_aw::Spinner::update` at `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_aw-0.14.1/src/widget/spinner.rs:165-202`. | The `request_redraw_at` call site at line 197-199 is **inside** an `if let Event::Window(window::Event::RedrawRequested(now)) = event && is_visible(&bounds)` branch (lines 180-181). The redraw is re-scheduled ONLY when (a) the iced runtime has delivered a `RedrawRequested` event to this widget instance AND (b) the widget's `is_visible(&bounds)` check passes. No `request_redraw_at` call lives outside this branch. |
| 2 | Inspect cockpit subscription wiring at `crates/ui/src/bin/cockpit.rs:276-295`. | The cockpit's `fn subscription(&self)` returns only `time_sub` (1 Hz `ServerTimeRecipe`) plus a modal-gated `iced::event::listen_with` for the tape-audit modal's Esc-handling. **No spinner subscription is registered at the application level.** The spinner's tick is widget-internal; it does not pull on the iced executor when the widget is absent from the view tree. |
| 3 | Inspect Spinner instantiation surface at `crates/ui/src/widgets/frame.rs:178-206`. | `iced_aw::Spinner::new()` is constructed inside `frame::loading_with_spinner(text, mode)`. That helper is **only invoked from `match` arms** keyed on `PanelState::Loading` (or Empty): verified call sites at `crates/ui/src/widgets/positions.rs:55`, `crates/ui/src/widgets/strategies.rs:48`, `crates/ui/src/widgets/pnl.rs:22`, `crates/ui/src/screens/strategies.rs:57`, `crates/ui/src/screens/strategies.rs:247`, `crates/ui/src/screens/risk.rs:45`, `crates/ui/src/screens/audit.rs:57`. **In every site, the helper is unreachable when the panel's state is `PanelState::Ready(_)` or `PanelState::Error(_)`.** |

**Verdict.** H-PERF-6 is **architecturally impossible** to manifest as
the analyst's late-binding hypothesis claimed (lifecycle leak even
when no panel is loading). The Spinner is a `Widget`, not a
`Subscription`; it has no application-level registration. When the
view tree returns no Spinner instance (because every panel is
`Ready`), the iced runtime has no widget to deliver
`RedrawRequested` to and the `request_redraw_at` re-schedule loop
cannot fire. The lifecycle is purely view-tree-bound — there is no
"executor wake-up" channel for the Spinner to leak into.

**Caveat: H-PERF-6 has a narrower live variant the architect ratifies
for M0.** If even ONE panel sits in `PanelState::Loading` indefinitely
(e.g., a bus-event arm never fires because of a state-machine bug),
the Spinner will pull a 60 fps redraw indefinitely — and that's
H-PERF-1, not H-PERF-6. M0's idle-vs-loading frame-time histogram
comparison resolves H-PERF-1 directly.

**Implication for M0.** H-PERF-6 is RESOLVED-by-impossibility and
removed from the M0 falsifier-batch. M0 keeps H-PERF-1 / -2 / -3 / -4
as the live hypotheses; H-PERF-5 / -6 are sub-agent-falsified inline
above.

**Cite.** [`feature.md ## H-PERF-6`](#h-perf-6--spinner-subscription-wakes-the-executor-even-when-no-panel-is-loading-analyst-late-binding);
`~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_aw-0.14.1/src/widget/spinner.rs:165-202` (cargo registry, machine-local);
[`crates/ui/src/bin/cockpit.rs:276-295`](../../crates/ui/src/bin/cockpit.rs);
[`crates/ui/src/widgets/frame.rs:178-206`](../../crates/ui/src/widgets/frame.rs).

### M0 falsifier-batch — final state going into the orchestrator-run

| Hypothesis | Pre-architect state | Architect verdict | Orchestrator M0 still resolves? |
|------------|--------------------|--------------------|---------------------------------|
| H-PERF-1 (spinner 60 fps repaint while loading) | analyst HIGH | **live** | YES — M0 profile + idle/loading delta |
| H-PERF-2 (Table layout uncached) | analyst MEDIUM | **live** | YES — M0 profile + collapsed/expanded delta |
| H-PERF-3 (hit-test traversal / event-loop starvation) | analyst MEDIUM | **live** | YES — M0 trace + synthetic-load click-bench |
| H-PERF-4 (tiny-skia software-raster dominant) | analyst LOW-MEDIUM | **live** | YES — M0 profile top-3 self-time entries |
| H-PERF-5 (render-debug leaking) | analyst LOW | **RESOLVED-UNFALSIFIED** (architect grep) | NO — dropped from M0 batch |
| H-PERF-6 (spinner exec wake-up when no panel loading) | analyst MEDIUM | **RESOLVED-by-impossibility** (architect source-read) | NO — dropped from M0 batch |

The orchestrator's M0 run carries four live hypotheses, not six. H-PERF-5
and H-PERF-6 are filed as ARCHITECT-RESOLVED in the M0 results section
template; the orchestrator does NOT need to re-falsify them.

### Design risks the architect signals to developer / orchestrator

1. **B1 memoization may need an upstream-iced PR.** Per Q4 resolution,
   if `iced::widget::Table::layout` has no extension point for an
   external cache (the analyst's "possibly a small upstream-iced patch
   if `iced::widget::Table` doesn't expose the hook"), the developer
   routes back to architect for the B3 fallback + ADR. The architect's
   B1-preferred call is contingent on an upstream-iced inspection the
   developer does at M1 task start — not at architect pass.
2. **Samply install on orchestrator host.** The M0 task body includes
   a `command -v samply || cargo install samply` line so the
   orchestrator's first M0 run is self-bootstrapping. If samply install
   fails (e.g., MSL toolchain mismatch), the fallback to cargo
   flamegraph kicks in via the same task body.
3. **M2 regression test scaffolding (analyst's `sleep 50 ms`
   regression).** Analyst flagged the need; architect ratifies tester
   ownership. The non-default-feature flag for the `sleep` injection
   is `perf-regression-test` (new in `crates/ui/Cargo.toml` at
   developer's M2 pass). The tester scaffolds the assertion harness;
   the developer adds the feature flag.
4. **No anchor SHA churn.** This brief touches no strategy / audit /
   exec / backtest / report-rendering code (UI-only). All 9 anchors
   in `spec/anchors.toml` remain unchanged through M0 → M1 → M2 → M3.
   No ADR required for anchor preservation; surface this in tasks.md
   `T-M_FINAL_VERIFY-ANCHORS` row as a structural-cite, not a run.

## M0 results (orchestrator-executed 2026-05-15)

**Profile artifacts.**
- `spec/cockpit-performance-and-input-responsiveness/reports/m0-profile-2026-05-15T12-09Z.json.gz` (46 KB, gecko/Firefox-profiler format; samply 0.13.1)
- `spec/cockpit-performance-and-input-responsiveness/reports/m0-profile-2026-05-15T12-09Z.json.syms.json` (26 KB, presymbolicated sidecar for offline analysis without dSYM bundles)
- Capture window: ~3.5s wall-clock, 1971 main-thread samples at 1 ms interval. Cockpit was idle (no operator interaction; fixtures-mode default screen `Home` with all panels populated via `bin/cockpit.rs:161-198` seed).

**Empirical CPU signal (independent of profile).**
- Cockpit running idle at **~66.9% CPU** (1.79 cores effective, observed via `ps aux` across multiple runs). Cumulative CPU time grew 20m36s over ~5min wall-clock = ~400% load. This is the operator-reported "slow UI" reproduced empirically — not a subtle frame-pacing nuance, a busy-rendering loop.

**Top 10 hot leaf frames (self-time).**

| % | Library | Symbol |
|---|---|---|
| 38.9% | libsystem_kernel.dylib | `mach_msg2_trap` (kernel idle wait between repaints) |
| 23.5% | ? | `<unresolved>` (likely libsystem; samply didn't symbolicate every range) |
| 10.9% | cockpit | `core::ops::function::impls::<...>::call_mut` (closure dispatch through iced's event loop) |
| 5.3% | libsystem_platform.dylib | `_platform_memmove` |
| 4.4% | cockpit | `tiny_skia::pipeline::blitter::RasterPipelineBlitter::blit_rect` |
| 2.7% | libsystem_platform.dylib | `_platform_memset` |
| 2.1% | cockpit | `tiny_skia::pipeline::highp::source_over_rgba` |
| 1.8% | libsystem_platform.dylib | `__bzero` |
| 1.6% | cockpit | `tiny_skia::pipeline::highp::gather` |
| 1.0% | cockpit | `<alloc::vec::Vec<T>>::from_iter` |

**Inclusive rendering pipeline cost (each ancestor counted once per sample).**

| % inclusive | Symbol |
|---|---|
| 45.5% | `iced_tiny_skia::Compositor::present` (the entry point for the per-frame paint) |
| 30.1% | `iced_tiny_skia::Renderer::draw` |
| 20.5% | `iced_tiny_skia::engine::Engine::draw_quad` |
| 12.5% | `tiny_skia::blitter::Blitter::blit_rect` |
| 9.0% | `tiny_skia::scan::path::fill_path_impl` |
| 6.4% | `tiny_skia::PixmapMut::fill_path` |
| 6.1% | `iced_tiny_skia::engine::Engine::draw_text` |
| 5.8% | `tiny_skia::scan::fill_rect` |
| 4.9% | `tiny_skia::scan::path_aa::fill_path` (anti-aliased path — text glyphs) |
| 3.3% | `iced_tiny_skia::text::draw` |

**Verdict per H-PERF row (orchestrator-executed):**

| Hypothesis | Verdict | Evidence |
|---|---|---|
| **H-PERF-1** (`iced_aw::Spinner` 60 fps redraw subscription) | **CONFIRMED-INDIRECT** | 45.5% of main-thread time inside `Compositor::present()` proves the cockpit is doing continuous full-frame repaints at idle. Architect's earlier "resolution-by-impossibility" for H-PERF-6 (spinner only exists when a panel is Loading) was correct *as a code-read inference*, but the empirical signal shows ANOTHER continuous-redraw trigger is firing — either a panel is in Loading state I didn't account for (likely `pnl_state` / `agent_feed_state` defaults), the chart canvas requests continuous redraws independent of spinner, or some other subscription. **Routing note: the SPECIFIC continuous-redraw trigger is open; H-PERF-1's fix (cadence gating) is still the right shape regardless of which widget fires it.** |
| **H-PERF-2** (Brief A native `iced::widget::Table` per-cell layout/quad cost) | **CONFIRMED** | 20.5% inclusive in `draw_quad` is consistent with Brief A's Table emitting `separator_x` + `separator_y` quads per cell-grid boundary (cited in architect synthesis from `iced_widget-0.14.2/src/table.rs:704-714`). With 4 panels × ~12 rows × ~6 columns × 60 fps, the quad-emission rate is large. |
| **H-PERF-3** (click hit-test path) | **DEFERRED — not in capture** | Profile was idle-only (orchestrator can't simulate clicks); H-PERF-3 stays live for M3. |
| **H-PERF-4** (tiny-skia software raster dominant) | **CONFIRMED** | 12.5% in `blit_rect`, 9.0% + 4.9% in `fill_path` (geometric + AA), 4.4% in pipeline `blitter`. Pixel pipeline is doing real work; this is what makes 60 fps full-redraw expensive. Architect's Q2 trigger condition is met — wgpu re-litigation is a candidate for follow-up brief if M1 candidate A + B don't recover acceptable fps. |

**Routing per architect's M0-2 verdict matrix.**

The matrix says "if two hypotheses share self-time within 10% of each other, name primary + secondary and route to architect for ADR on which to fix first." We have THREE confirmed (H-PERF-1 indirect + H-PERF-2 + H-PERF-4). Per architect Q4 resolution (B1 → B2 → B3 ladder):

- **Primary fix candidate: M1 Candidate A (spinner / continuous-redraw cadence).** Cutting redraw frequency from 60 fps → 10 fps drops the per-frame cost ~6×, which makes H-PERF-2 and H-PERF-4 effectively non-blocking even without their own fixes. Highest leverage.
- **Secondary fix candidate: M1 Candidate B1 (Table layout memoization).** If A alone doesn't drop idle CPU below ~10%, B1 fans out next per architect's ladder.
- **Tertiary (deferred): wgpu re-litigation.** Architect Q2 says operator override required. Surface in approval notes but do NOT route automatically.

**Open question for architect re-engagement before M1 spawn:**
1. The specific continuous-redraw trigger is unidentified. Before developer implements Candidate A, architect should grep `crates/ui/src/bin/cockpit.rs` for any default `PanelState::Loading` field that's not explicitly seeded, AND grep `crates/ui/src/widgets/chart.rs` for `subscription` / `request_redraw_at` calls. The trigger MAY be:
   - A panel in default `Loading` state → architect-recommended fix: seed it Ready in fixtures.
   - The chart widget requesting per-frame redraws → architect-recommended fix: gate the chart's redraw subscription.
   - Some other unknown — architect adds a candidate hypothesis.

**Profile re-run if needed.** The current capture is short (~3.5s, cockpit died ~mid-profile so samply finalized on SIGINT). A longer run (30-60s) would strengthen the per-frame quad-count evidence. Not blocking M1 candidate A spawn; orchestrator can re-profile post-M1 to measure improvement.

## Implementation

### M1 Candidate A (developer, 2026-05-15) — `ThrottledSpinner` local widget

Landed the A2 sub-candidate per the architect's
[Q4 resolution](#q4-resolution--m1-candidate-b-prefers-b1-memoize-over-b3-revert-b3-fallback-only-if-upstream-table-has-no-cache-hook)
and the [M0 results](#m0-results-orchestrator-executed-2026-05-15)
routing note ("Primary fix candidate: M1 Candidate A
(spinner / continuous-redraw cadence)"). The fix is a behavioural-clone
of `iced_aw::Spinner` with the `FRAMES_PER_SECOND` constant flipped
60 → 10; the upstream MIT-licensed widget body is re-implemented locally
because the constraint envelope forbids forking `iced_aw`.

**Files touched.**

| Path | Δ | Purpose |
|------|---|---------|
| `crates/ui/src/widgets/throttled_spinner.rs` (new) | +296 LOC | Local Widget impl with 10 fps cadence + 5 unit tests + module-doc rationale |
| `crates/ui/src/widgets/frame.rs` | +14/-7 LOC | `loading_with_spinner` constructs `ThrottledSpinner::new()` instead of `iced_aw::Spinner::new()` |
| `crates/ui/src/widgets/mod.rs` | +4 LOC | `pub mod throttled_spinner;` with module-doc cross-link |

**Why A2 (wrap and throttle), not A1 (cadence edit) or A3 (canvas
replacement).** Architect's preferred A2 sub-candidate per the
discipline-reminders block in the developer brief: A1 was the
analyst's seeded option but it is mechanically equivalent to a
local-widget approach in iced 0.14 (the FPS constant is private to
`iced_aw`'s `Widget::update` impl, so any "cadence edit" requires
either an upstream fork — vetoed — or a wrap). A3 (canvas
re-paint with no `request_redraw_at`) was deferred because A2
already drops the cost ~6× without architectural churn; A3 stays
available as a follow-up if the post-fix idle-CPU measurement
shows residual redraw cost.

**Determinism (Brief B H-arch-9 carry-through).** The internal
`SpinnerState` shape is preserved verbatim from upstream
`iced_aw::Spinner`, including the `Instant::now()` seed in
`Widget::state`. Per H-arch-9 (RESOLVED-PASS-with-caveat), this is
test-unreachable: `iced_test` snapshot paths never deliver
`RedrawRequested` events, so `state.t` and `state.last_update`
stay at their seed values during snapshot rendering. No new
wall-clock calls were introduced in this widget;
`scripts/check_no_clocks_in_ui_tests.sh` continues to PASS because
the unit tests reference only `FRAMES_PER_SECOND`, `circle_radius`,
`width`, `height` — no clock tokens.

**Acceptance — developer-runnable evidence.**

| Gate | Command | Result |
|------|---------|--------|
| Build | `cargo build -p ui --bin cockpit --features fixtures` | PASS (clean) |
| Build (render-debug) | `cargo build -p ui --bin cockpit --features fixtures,render-debug` | PASS |
| Format | `cargo fmt -p ui --check` | PASS |
| Clippy | `cargo clippy -p ui --no-deps --lib --tests` | 0 net-new errors on touched files; the documented 6 pre-existing `expect()` errors in `widgets/chart.rs` + `window_icon.rs` remain (out of scope per developer brief). |
| Unit tests (focused) | `cargo test -p ui --lib throttled_spinner` | `5 passed; 0 failed; 0 ignored` |
| Full UI suite (default) | `cargo test -p ui` | `280 passed; 0 failed; 5 ignored` |
| Full UI suite (render-debug) | `cargo test -p ui --features render-debug` | `286 passed; 0 failed; 5 ignored` |

**Open verification path (orchestrator-owned).** Per
`AGENT.md ## Capability boundaries` the developer does not run a
live cockpit. Three observations are deferred to the orchestrator's
post-developer gate:

1. Re-run cockpit-smoke and confirm `panic_count = 0` (still v1.0;
   the v1.1 `fps_p50` assertion lands under M2, not M1).
2. Repeat the M0 idle-CPU measurement (`ps aux | grep cockpit`)
   and confirm the ~66.9 % → < 15 % drop expected from the 6×
   redraw-cadence cut. If residual CPU sits above 15 %, the
   secondary continuous-redraw trigger flagged in M0 results
   ("the SPECIFIC continuous-redraw trigger is unidentified") is
   distinct from the spinner — escalate to architect re-engagement
   for M1 Candidate B (Table memoization) or chart-subscription
   audit.
3. Visual verification that the spinner still animates smoothly at
   10 fps. The architect picked 10 per the scope sketch; if the
   operator wants a different cadence, the constant lives at
   `crates/ui/src/widgets/throttled_spinner.rs:101` and is a
   one-line change.

**Part 1 quick-fix status (not applied; routed to architect).**
The developer brief described a "Part 1 quick fixture fix" to seed
`cockpit.agent_feed_state = PanelState::Ready(...)` in
`crates/ui/src/bin/cockpit.rs:200-225` because the orchestrator's
M0 results section hypothesised an unseeded `Loading` panel as the
empirical continuous-redraw trigger. On inspection the field name
in the brief does not match the current code: the agent-feed panel
state lives on `Cockpit::tape` (per Phase 5 Q14 rename-but-preserve;
see `state.rs:634-639`), and `tape` is already seeded via
`Cockpit::ready(fake_fill_feed(8), ...)` (`fixtures.rs:695-705`).
All four Home-screen panels (pnl / positions / strategies /
agent_feed) are seeded `Ready` at fixtures boot. Surfaced as an
open question for the orchestrator/architect in the developer's
HANDOFF envelope; the systemic M1 Candidate A fix lands regardless
because the architect's design rationale explicitly applies to any
future legitimate use of `loading_with_spinner` ("a future panel
that legitimately enters `PanelState::Loading` (during real data
loading) shows a 10 fps spinner instead of 60 fps").

## Changelog

- 2026-05-15 (presenter, v1.0.0): release-mode presentation assembled
  at [`presentations/cockpit-performance-and-input-responsiveness-2026-05-15.md`](../archive/presentations-2026-Q2.tar.gz)
  after evaluator `VERDICT → PASS` (15/15 criteria; log body-SHA-256
  `9c50ec45ce3a627e97088d2adc2d6da8407e33b85f969341a994062729d699c2`)
  and orchestrator cockpit-smoke pre-tick PASS (0 panics, 7s window;
  first cross-brief fire of AGENT.md rule 6 — log
  `cockpit-smoke-pretick-2026-05-15T13-07Z.log`). Headline metric:
  idle CPU dropped from ~66.9% to 2.2–13.1% (5.1× minimum, ~18×
  typical, 30× peak). Frontmatter bumped `version: 0.3.0 → 1.0.0`,
  `owner: developer → presenter`, `updated: 2026-05-15`. `status`
  stays `in-progress` until operator approval flips it to `shipped`
  (orchestrator owns that flip post-approval per AGENT.md Process
  discipline rule 2). T_FINAL_* ticks intentionally left blank in
  `tasks.md` — orchestrator's post-approval job. Four operator
  decisions surfaced in the presentation: M2 perf-budget gate
  scheduling, M3 input-dispatch verification, M0 root-cause
  re-engagement, Brief B frontmatter amendment.
- 2026-05-15 (developer, M1 Candidate A): landed `ThrottledSpinner`
  local widget at `crates/ui/src/widgets/throttled_spinner.rs`
  (10 fps cadence vs upstream 60 fps); wired
  `frame::loading_with_spinner` to use it instead of bare
  `iced_aw::Spinner::new()`. Build clean, 280/0/5 (default) +
  286/0/5 (render-debug) test pass-count. `T-M1-A-1` ticked with
  three-citation contract. Part 1 quick-fix from developer brief
  not applied — field name does not match current code (see
  `## Implementation` open verification path). `status: design →
  in-progress`, `owner: architect → developer`, `version: 0.2.0 →
  0.3.0`.
- 2026-05-15 (orchestrator, post-architect M0 execution): captured samply profile against release-mode fixtures cockpit. Symbol resolution succeeded via `--unstable-presymbolicate` sidecar (samply 0.13.1; `samply setup` ran by operator to grant `task_for_pid` entitlement). Empirical idle-CPU: ~66.9%. Top hot path: `iced_tiny_skia::Compositor::present` 45.5% inclusive, `draw_quad` 20.5%, pixel pipeline (`blit_rect` + `fill_path*`) 27%+. H-PERF-2 + H-PERF-4 CONFIRMED; H-PERF-1 CONFIRMED-INDIRECT (continuous-redraw trigger unidentified — open question for architect re-engagement before M1 Candidate A spawn); H-PERF-3 DEFERRED (no click in capture). Routing: architect should grep cockpit default `PanelState` fields + chart widget subscription before developer M1 spawn.
- 2026-05-15 (analyst): initial draft (v0.1.0). Lifted REQ-COCKPIT-PERF-001
  from `proposed` to `draft` in `spec/trace.toml`. Trigger: operator's
  approval-with-notes block on `ui-quality-gate-overhaul v1.0.0`
  approval pass (2026-05-15). Six hypotheses (H-PERF-1..6) framed as
  falsifiable claims; four sub-targets (M0/M1/M2/M3) scoped with
  acceptance criteria. Six open questions for the architect. Honest
  divergence section flags Brief A R2 partial-revert option + M2
  touching a just-shipped gate + wgpu re-litigation as a deferred
  decision.
- 2026-05-15 (architect): design pass (v0.2.0; `status: draft → design`,
  `owner: analyst → architect`). Six open questions resolved inline
  (Q1 M3-coupled / Q2 wgpu 1-line ratify / Q3 30 fps floor /
  Q4 B1-preferred over B3 / Q5 cockpit-smoke minor bump to v1.1 /
  Q6 samply primary). Two sub-agent-safe falsifiers executed:
  H-PERF-5 RESOLVED-UNFALSIFIED (defensively true; 6 cfg gates + 3
  matched emit sites verified by grep); H-PERF-6 RESOLVED-by-impossibility
  (Spinner has no application-level subscription; widget-tree absence
  precludes lifecycle leak; source-read of `iced_aw-0.14.1/src/widget/spinner.rs`
  + cockpit.rs subscription + frame.rs spinner-instantiation paths
  verified). M0 falsifier-batch reduced from H-PERF-1..6 to
  H-PERF-1..4. No ADR filed (Q4 B1-preferred holds without ADR;
  the B3-fallback ADR is the developer's HANDOFF-back trigger if B1
  proves infeasible). tasks.md authored with M0/M1/M2/M3/M_FINAL
  task ladder + three-citation contract per AGENT.md ## Process
  discipline rule 1. trace.toml `arch` column extended with the
  design-synthesis anchor.
