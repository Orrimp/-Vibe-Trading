---
title: Pick A — Test infra trifecta strategic direction
date: 2026-05-29
authors: [analyst]
status: direction
tags: [strategy, process, tooling, route-c, test-infra, bundle, trifecta]
related:
  - docs/dev-notes/process-tooling-survey-2026-05-29.md
  - docs/dev-notes/post-v3-strategy-direction-2026-05-29.md
  - docs/dev-notes/weekly-retro-2026-05-27-to-2026-05-29.md
  - spec/lab-recipe-test-harness/feature.md
  - spec/lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md
  - spec/visual-fail-html-reporter/feature.md
  - spec/ui-test-harness-viewport-matrix/feature.md
  - _bmad-output/planning-artifacts/architecture/decisions/0048-lab-recipe-test-harness.md
  - spec/backlog.md
---

# Pick A — Test infra trifecta strategic direction

> **Strategic dev-note, NOT a feature brief.** Frames the bundle
> rationale, sequencing, acceptance, risks, and v0.3.0+ harness
> candidate list. Two feature briefs (`visual-fail-html-reporter`,
> `ui-test-harness-viewport-matrix`) get authored alongside and
> promoted Queue → Active under this direction. The third pillar
> (`lab-recipe-test-harness v0.3.0+`) stays Queue-side until v0.2.0
> Wave A lands.

## § Why bundle these three

Per [`process-tooling-survey-2026-05-29.md § Pick A`](process-tooling-survey-2026-05-29.md#pick-a--test-infra-trifecta-harness-v030--visual-fail-html-16--viewport-matrix-5):
**Visual-fail HTML reporter (#16) + viewport matrix (#5) + harness
v0.3.0+ extension** all target the SAME failure class: _"default-FAIL
visual + state oracles that survive future iced upgrades."_ The
durable-over-quick contract (AGENT.md 2026-05-28) says: ship the bundle
as one strategic direction so the tester contract gets amended ONCE,
the `.gitattributes` PNG rule lands ONCE, and the operator-facing
failure-artifact surface (HTML + viewport PNG triples + lab-recipe
falsification logs) is consistent across all three.

**Empirical precedent for "ship one wave, not three."** The shipped
harness v0.1.0 (2026-05-28) paid off **same-week** with the Bug #64
D.2.1 catch (per [`weekly-retro § 5`](weekly-retro-2026-05-27-to-2026-05-29.md#5-bug-64-d11--d21-attempt-and-revert--harness--re-attempt--wont-fix)).
Splitting the trifecta into three sequential cycles costs ~3× the
operator decision latency (three separate analyst → architect → tester
loops) for ~0× extra durability — every brief inherits ADR-0048 +
ADR-0042 + ADR-0044, no new ADRs surface, the M-T1 fast-skip is
guaranteed across all three.

**The anti-pattern to avoid.** Ship visual-fail-HTML alone first
(~1 day, "easy win"); discover when viewport-matrix later lands that
the HTML reporter's three-PNG triple layout assumed Charts-only; rework
the reporter to handle the panel/modal/status-bar matrix — that's a
v0.1.1 retrofit the bundle avoids. Same for the harness v0.3.0+ Recipe
extension — if the visual-fail reporter ships before the next harness
wave, the reporter HTML format has no Recipe-shaped slot (no
`falsification-probe-line` field), and v0.4.0 retrofits the format.

The bundle framing is **NOT "ship all three at once"** — it's "design
all three under one direction so the contracts mesh." Sequencing
below.

## § Sequencing (which goes first; which parallelizes; gates)

**Constraint anchor**: `lab-recipe-test-harness v0.2.0` Wave A (background
agent `a54c75856bc45efae` extending TrainingLogRecipe) is **in-flight at
the time this direction lands**. The v0.2.0 ship is the gate for any
v0.3.0+ Recipe-extension analyst spawn.

```
                     ┌────────────────────────────────┐
                     │ harness v0.2.0 Waves A→D       │
                     │ (in-flight; not touched here)  │
                     └─────────────┬──────────────────┘
                                   │ v0.2.0 SHIPS (verdict → PASS)
                                   │
                  ┌────────────────┴────────────────┐
                  │                                 │
                  ▼ (analyst spawn now)             ▼ (analyst spawn now)
        ┌────────────────────┐         ┌──────────────────────────┐
        │ visual-fail-html-  │         │ ui-test-harness-viewport-│
        │ reporter v0.1.0    │   ‖     │ matrix v0.1.0            │
        │ ~1 dev day         │         │ ~3-4 dev days            │
        └─────────┬──────────┘         └──────────┬───────────────┘
                  │                               │
                  │  (both ship; tester contract amended)
                  ▼                               ▼
        ┌──────────────────────────────────────────────────┐
        │ harness v0.3.0+ — analyst spawn AFTER v0.2.0     │
        │ ships; Recipe candidate list (this dev-note)     │
        └──────────────────────────────────────────────────┘
```

### Wave 1 (NOW, parallel to v0.2.0)

The two promoted features run in parallel — they have **independent
file-scope** per the AGENT.md § Parallelism rules conflict matrix:

| Pair | Same file? | Same Cargo.toml? | Same artifact? | Same operator-decide Q? | Verdict |
|------|------------|-------------------|----------------|---------------------------|---------|
| `visual-fail-html-reporter` × `ui-test-harness-viewport-matrix` | NO (different test files; reporter is a helper module) | NO new `Cargo.toml` deltas for reporter; matrix may add `image-compare` already present | NO (different baseline PNGs) | NO (no shared operator-decide gates) | **PARALLEL SAFE** |
| Either × `harness v0.2.0` Wave A (in-flight) | NO (Recipe tests live in different files) | NO | NO (Recipe tests emit zero file output per ADR-0048 § D6) | NO | **PARALLEL SAFE** |

So architect M-T1 passes can spawn concurrently for both promoted
briefs as soon as M0 lands. The orchestrator should kick architect on
both in the same tool-use block.

### Wave 2 (AFTER v0.2.0 ships)

Analyst spawn for `lab-recipe-test-harness v0.3.0+` extension. **Do
not start** until v0.2.0 verdict-PASS lands. Three reasons:

1. **Scope discovery depends on v0.2.0 lessons.** Wave A → D will
   surface whether the per-Recipe-specific mock pattern (D-V0.2.0-1)
   actually generalizes to non-iced surfaces (e.g. lesson-card persistence,
   backtest progress) or whether v0.3.0 needs a different shape.
2. **Falsification probe table (D-V0.2.0-3) is the input to v0.3.0+ scope
   pick.** If any v0.2.0 wave surfaces a regression class the existing
   pattern doesn't catch (per K1 falsifier in the v0.2.0 brief), that
   becomes v0.3.0's first Recipe.
3. **ADR-0048 § Changelog needs the v0.2.0 row before v0.3.0 amends
   again.** Architect M-T1 contract requires "one amendment per ship."

## § Acceptance — what "test infra trifecta v1.0 SHIPPED" means

The bundle is **SHIPPED** when ALL of the following hold:

1. **visual-fail-html-reporter v0.1.0 SHIPPED** (operator-approved presentation;
   trace row state = `passed`). Tester contract amended with the
   visual-fail HTML stanza at `.claude/agents/tester.md`. ≥ 1
   integration test in the workspace deliberately fails to emit a
   sample `spec/<slug>/reports/visual-fail-<ts>.html`, manually verified
   by operator that the three-PNG triple + assertion text + file:line
   renders correctly in Safari/Chrome.
2. **ui-test-harness-viewport-matrix v0.1.0 SHIPPED** (operator-approved; trace row state =
   `passed`). All widget tests under `crates/ui/tests/` whose existing
   bootstrap baselines are Charts-only get extended to the three
   viewport slots (1280×720 / 1920×1080 / 3360×1890). New baseline PNGs
   under `crates/ui/tests/visual-baselines/<widget>_<viewport>.png` are
   committed. `.gitattributes` rule for `*.png binary diff=exif`
   committed so PNG diffs stay reviewable (`git log -p` doesn't dump
   binary garbage).
3. **lab-recipe-test-harness v0.2.0 SHIPPED** (already on track via in-flight
   Wave A; this trifecta does not block on it but counts it as part of
   the bundle since the v0.2.0 brief was already promoted under Pick A
   framing).
4. **Tester contract single amendment**. `.claude/agents/tester.md`
   gets ONE amendment covering the visual-fail HTML emission protocol
   (replaces ad-hoc prose-only failure description on visual snapshots)
   and a stanza noting the viewport matrix is the default for any
   widget test added after the bundle ships. Architect M-T1 ratifies
   both stanzas in a single review pass.
5. **No new ADRs.** ADR-0048 carries forward verbatim for any Recipe
   work; the bootstrap ADR (architect-locked at bootstrap M-T1 2026-05-12
   per ui-test-harness-bootstrap feature.md § D5) carries forward for
   visual baselines. Each feature only amends a § Changelog row.

**Counter-example — not SHIPPED**: any one of the three at FAIL,
SOFT-PASS-with-deferred-rework, or unticked tester contract amendments.
The bundle does NOT ship partial.

## § Risks

### R1 — Cross-feature contract drift on `.claude/agents/tester.md`

Two briefs amending the same agent contract file in close succession
risks merge conflicts and silent divergence in the visual-fail emission
protocol. **Mitigation**: tester.md amendment is **owned by the FIRST
brief to ship** (likely `visual-fail-html-reporter` since it's ~1 day
vs viewport-matrix's ~3-4 days). The viewport-matrix brief explicitly
inherits the stanza without amendment; its M-T1 review confirms the
stanza covers the matrix case.

**Falsifier**: viewport-matrix's M-T1 finds the visual-fail HTML stanza
incomplete for the matrix case (e.g. needs per-viewport triple instead
of per-test triple). Route: architect amends the stanza in the viewport-
matrix's own M-T1, no separate brief needed — same file, one extra row.

### R2 — Harness v0.3.0+ scope-creep if v0.2.0 ships late

If v0.2.0 Wave A surfaces K1 (a Recipe regression class the pattern
doesn't catch), v0.2.0 ships SOFT-PASS with a v0.3.0 follow-on
commitment, and v0.3.0 inherits both (a) the existing Recipe candidate
list AND (b) the K1 gap. **Mitigation**: this dev-note's v0.3.0+
candidate list (§ next section) explicitly carves out the "K1 gap from
v0.2.0" slot so the analyst spawn AFTER v0.2.0 ships has a pre-known
scope-discovery delta.

**Falsifier**: v0.2.0 ships clean PASS (no K1 trip). Then v0.3.0+ scope
is exactly the candidate list below, no gap-filling.

### R3 — PNG baseline review friction

Viewport matrix adds N baseline PNGs per widget test (likely 30+ widget
tests × 3 viewports = 90+ new committed PNGs in v0.1.0). Operator review
on the first commit is significant. **Mitigation**: viewport-matrix v0.1.0
ships Charts + status bar only first (≤ 15 PNGs); Phase 2 adds panels +
modals + agent feed once Charts evidence proves the pattern. **DURABLE
CARVE-OUT NOTE**: the operator may push back that this phasing
violates "ship one correct thing" — fallback durable scope is all
widget tests at once with the operator carving review time. Defer the
phasing decision to feature.md Q-PHASING (operator-decide; analyst
recommends DURABLE = all at once per AGENT.md 2026-05-28).

### R4 — Visual-fail HTML reporter inline-PNG payload size

A single visual-fail HTML with three PNG triples at 3360×1890 × 2.0
scale → ~5-10 MB per HTML. Across N visual fail reports stored in
`spec/<slug>/reports/`, repo size grows fast. **Mitigation**: reporter
emits the HTML to `target/visual-diff/<test_name>-<ts>.html` by
default (gitignored), with an opt-in `EMIT_VISUAL_FAIL_TO_SPEC=1`
env var that promotes to `spec/<slug>/reports/`. Operator picks
spec-persist only when the failure is investigation-worthy. Tester
contract amendment names the env var.

### R5 — Trifecta directional debt if presenter blocked

If presenter deck approval for visual-fail-html-reporter takes ≥ 1 week
(e.g. operator on vacation), the viewport-matrix can ship without it
(matrix tests still pass/fail via existing prose-only path), but the
bundle is partial-shipped. **Mitigation**: explicit fallback path — the
viewport-matrix presenter deck calls out the visual-fail-html stanza
as the inheritance pre-req; if the visual-fail HTML reporter hasn't
landed by viewport-matrix presenter time, the deck recommends
sequencing the operator approval visual-fail-HTML first.

## § What "harness v0.3.0+" candidate list looks like

**Sketch only.** Do NOT promote until v0.2.0 Wave A→D ships. Analyst
spawn AFTER v0.2.0's tester emits VERDICT → PASS. Three candidate
Recipes worth investing in (ranked by per-cycle benefit):

### Candidate 1 — `LessonCardRecipe` (K4 byte-identity coverage)

**Why.** Lessons in `crates/reflection/src/lesson_cards/` are
operator-visible artifacts under `spec/<slug>/lessons/` — they
materialize during Lab runs and are the operator's primary "what did
the model learn" surface. Per the K4 RegimeTag deletion regression
class (the v3-regime-classifier 2026-05-22 retire), lessons currently
have **no test that asserts byte-identity across two consecutive
runs** with the same seed. A lesson-card mutation (e.g. a refactor
that changes lesson-card JSON field ordering) would silently break
operator workflow without any gate firing.

**Shape.** Boundary test that drives the lesson-card store with a
mocked `LessonStore` + asserts (a) write-then-read round-trip
byte-identical; (b) lesson IDs deterministic across runs; (c) lesson
content hashes stable per seed. ~80-100 LoC per Recipe.

**Falsification probe** (per the D-V0.2.0-3 pattern): comment out the
deterministic lesson-ID derivation in `lesson_cards/store.rs`; assert
the round-trip test FAILs with a "lesson ID drifted across runs"
message.

### Candidate 2 — `BacktestProgressRecipe` (backtest UI tracking)

**Why.** When operator triggers a backtest from cockpit, progress
flows from `crates/backtest/src/scenarios/runner.rs` through an mpsc
channel up to the Lab progress bar widget. Per the Bug #64 exact-shape
analysis, this channel has the same vulnerability pattern as
`LabProgressRecipe` (which v0.1.0 + v0.2.0 cover): `tokio::select!` arm
that can shadow channel-receive after a refactor. The backtest progress
path currently has NO boundary test asserting the channel survives
multi-scenario runs.

**Shape.** Boundary test with `MockBacktestProgressBus` driving a
2-scenario sequential run; assert (a) progress emits per scenario without
gap; (b) `BacktestProgressCompleted` arrives after final scenario; (c)
channel survives across scenario boundaries. ~120 LoC.

**Falsification probe**: comment out the per-scenario progress emit in
`runner.rs` and assert the boundary test FAILs with "progress emit gap
between scenarios".

### Candidate 3 — `TrailMirrorRecipeS1` boundary test (extends v0.2.0's S2-only coverage)

**Why.** v0.2.0 Wave D covers TrailMirrorRecipe **Surface 2 only**
(handle-gated subscription presence). Surface 1 (the actual
`broadcast::Receiver<TrailMirrorTick>` stream-impl boundary —
lag-handling, close-handling, eager-subscribe race) ships at v0.1.0
via `trail_mirror_recipe_stream.rs` but does NOT include the new
"select-arm-survival" assertion pattern v0.2.0 Wave B introduced for
ActivityAuditAggregator. v0.3.0+ adds the symmetric assertion: under
interleaved `advance(N)` + `tx.send(tick)`, the channel survives N
boundaries.

**Shape.** Single new test in existing
`trail_mirror_recipe_stream.rs` (no new test file); ~30-50 LoC.
Lowest cost of the three candidates but also lowest per-cycle benefit
(only catches a specific refactor pattern).

**Falsification probe**: introduce a synthetic `tokio::select!`
wrapper around the existing TrailMirror stream-impl, comment one arm,
assert the new test FAILs.

### Honorable mentions (NOT in v0.3.0 candidate list — Month-2+)

- `AuditWriteAggregator` — agent-side write batcher; same `tokio::select!`
  shape as ActivityAuditAggregator but on the write path. Lower urgency
  (write path is operator-visible-but-not-load-bearing on regression).
- `ChartRedrawRecipe` (if it lands; currently the chart redraws via
  iced canvas state, not an explicit recipe). Speculative until chart-
  canvas-overhaul v2.0+ activates an explicit recipe pattern.
- `LiveOrderbookRecipe` — when v15b multi-venue activates, the
  per-venue orderbook subscription becomes a high-leverage Recipe.
  Defer until v15b promotion.

### Wall-clock estimate for v0.3.0+ candidate trio

- Candidate 1 (LessonCard): ~2 dev days + 0.5 tester day
- Candidate 2 (BacktestProgress): ~2-3 dev days + 0.5 tester day
- Candidate 3 (TrailMirror S1 extension): ~0.5 dev days + 0.25 tester day
- Combined: ~5-6 dev days + 1.25 tester days ≈ ~1.5 weeks wall-clock

Same shape as v0.2.0 — per-Recipe-specific mocks, per-Recipe T-T4
falsification probe in module docstring, zero new ADRs (ADR-0048
carries forward), zero anchor delta.

## § Operator-decide questions

**Only one operator-decide question at the strategic level.** Per the
durable contract, both Wave-1 promoted features (visual-fail-html-reporter,
ui-test-harness-viewport-matrix) bias toward DURABLE at every internal
gate — they're cheap enough that the durable choice has no realistic
fallback worth surfacing.

### Q-TRIFECTA-1 — bundle ordering vs sequential ship

**Q.** Do we ship the trifecta as one strategic-direction bundle (Wave
1 parallel = visual-fail-HTML + viewport-matrix; Wave 2 sequential =
harness v0.3.0+ after v0.2.0), OR ship visual-fail-HTML alone first
and decide on the rest after that operator-approval cycle?

**(Recommended — DURABLE) Option A: Wave 1 parallel bundle.** Spawn
analyst for both promoted features now (orchestrator can in fact do
this in the same tool-use block as the v0.2.0 in-flight Wave A); both
ship under shared tester contract amendment; v0.3.0+ candidate list
takes shape after v0.2.0 ships.

**Cost.** ~5-7 dev days total (combined); ~1.5-2 weeks wall-clock
through to bundle SHIPPED.

**Rationale.** Two briefs in parallel = ONE operator approval cycle
for the tester contract amendment (visual-fail HTML stanza). One
cycle covers both because viewport-matrix's M-T1 inherits the stanza
without amendment (per Risk R1 mitigation). Sequencing instead = TWO
operator approval cycles + R1 amendment-drift risk. The bundle is
strictly cheaper and strictly more durable.

**Option B (cheap fallback).** Ship visual-fail-HTML alone first
(~1 dev day + 1 operator approval cycle); spawn viewport-matrix
analyst only AFTER the HTML reporter ships. ~+3-5 days deferred cost
+ 1 extra operator approval cycle + R1 amendment-drift risk.
Rejected at analyst-level per AGENT.md 2026-05-28 — when the cheap
option doesn't strictly dominate, the durable option is recommended.

**Default**: A (Recommended DURABLE) per AGENT.md 2026-05-28.

---

**No other operator-decide questions at the strategic level.** Each
promoted feature's own brief has its own internal operator-decides
(see `visual-fail-html-reporter/feature.md` and
`ui-test-harness-viewport-matrix/feature.md`), but the bundle-level
choice is just "all three together" vs "ship one at a time" — and the
recommended pick is bundle.

## § Cross-references

- [`process-tooling-survey-2026-05-29.md`](process-tooling-survey-2026-05-29.md) — Top-5 ranking (Pick A)
- [`weekly-retro-2026-05-27-to-2026-05-29.md`](weekly-retro-2026-05-27-to-2026-05-29.md) — § 5 harness v0.1.0 same-week catch
- [`spec/lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md`](../lab-recipe-test-harness-v0.2.0-cross-surface-extension/feature.md) — in-flight
- [`spec/visual-fail-html-reporter/feature.md`](../visual-fail-html-reporter/feature.md) — Wave 1 promoted (this dev-note's spawn)
- [`spec/ui-test-harness-viewport-matrix/feature.md`](../ui-test-harness-viewport-matrix/feature.md) — Wave 1 promoted (this dev-note's spawn)
- [`_bmad-output/planning-artifacts/architecture/decisions/0048-lab-recipe-test-harness.md`](../architecture/adr/0048-lab-recipe-test-harness.md) — parent ADR (D1-D6 carries forward across all three)
- [`spec/ui-test-harness-bootstrap/feature.md`](../ui-test-harness-bootstrap/feature.md) — bootstrap predecessor for viewport-matrix
- [`docs/dev-notes/post-v3-strategy-direction-2026-05-29.md`](post-v3-strategy-direction-2026-05-29.md) — Route C compounder argument

## Closing

Pick A's durable framing is **"bundle the three under one strategic
direction; ship Wave 1 parallel now, Wave 2 after v0.2.0 lands."** The
operator decides nothing at the strategic level beyond Q-TRIFECTA-1
(Recommended A = bundle). Each promoted feature's brief carries the
per-Recipe operator-decides as usual. The v0.3.0+ Recipe candidate
list above pre-positions the next analyst spawn so when v0.2.0 ships,
the orchestrator has a known scope sketch to brief.

## Changelog

- 2026-05-29 (analyst): direction authored under Route C Pick A framing
  per `process-tooling-survey-2026-05-29.md` architect recommendation.
  Two Wave 1 features promoted via parallel feature.md + tasks.md
  authoring (visual-fail-html-reporter, ui-test-harness-viewport-matrix).
  v0.3.0+ harness extension Recipe candidate list sketched (LessonCard,
  BacktestProgress, TrailMirror S1) for analyst spawn AFTER v0.2.0
  ships. One strategic-level operator-decide Q-TRIFECTA-1 (Recommended
  DURABLE = bundle) surfaced.
