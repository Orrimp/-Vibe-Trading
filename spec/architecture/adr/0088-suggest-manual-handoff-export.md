---
adr: 0088
title: SUGGEST → manual hand-off export — a deterministic offline serialiser of the crowned plan
status: accepted
date: 2026-07-10
supersedes: none
superseded-by: none
---

# ADR-0088: SUGGEST → manual hand-off export

## Context

The advisor journey (DATA → CALIBRATE → ANALYZE → SUGGEST) ends at a crowned
plan rendered *inside the running cockpit* (`Screen::ForwardPlan`, F6). There is
no way to take the plan away. The 2026-07-09 orchestrator critique named this the
sharpest product gap: the journey stops at a screen, so a retail user who decides
to act must hand-transcribe rules off a live UI — the exact moment the honesty
scaffolding (the weak-evidence band, the survivorship caveat, the unbounded-loss
short disclaimer) silently drops away. This is remediation-plan **P5**, feature
`advisor-handoff-export`.

The operator **ratified the draft wording AS DRAFTED** (DECISION-P5-WORDING,
2026-07-10): the § Draft wording (Variant A + short-crowned Variant B) in
`spec/v3/advisor-handoff-export/feature.md` is the **binding serialiser
contract**. This ADR designs the *mechanism* that emits it; it does not rewrite
the words. The export is the closest surface in the product to the not-advice
line, so it must be **useful** (portable) without ever becoming **advice**
(prescriptive) — it *describes* a computed plan, addressed to no one, prescribing
nothing.

Hard constraints (P5 non-goals, load-bearing): **NO order placement / NO venue
API** (register B-2 live-trading stays a settled dead-end); **NO new engine
computation** (no backtest/sizing/robustness/ranking re-run); **NO LLM in the
export path** (deterministic + offline; the F9 narration embeds only if already
generated + faithfulness-passed for this run); **NO new numbers** (verbatim from
the mirror); **NO prescriptive voice** in the content.

## Decision

**D1 — Home + layering (Q-HE-4).** `ui` owns the serialiser, in a new module
`crates/ui/src/export/plan_export.rs`. `ui` never depends on
`strategy`/`exec`/`models`/`llm`; the serialiser is a pure function over
**pure-`ui` mirror types only**, so `cargo tree -p ui` is unchanged.

**D2 — Signature over the VIEW, not the live-gated engine plan (refines the
brief).** The serialiser is:

```rust
// crates/ui/src/export/plan_export.rs — pure, DEFAULT ui build (no `live` feature)
pub fn serialize_plan_export(
    plan: &crate::forward_plan::ForwardPlanView,      // the pure-ui mirror
    report: &crate::leaderboard::BakeoffReportMirror, // carries outcome, scorecard,
                                                      //   data_quality, coin, range_label, run_seed
    narration: &crate::leaderboard::NarrationState,   // F9, embed only if Ready
    fx: Option<&crate::state::FxNote>,                // the €→USDT budget line, as the screen threads it
) -> String;

pub fn export_filename(report: &crate::leaderboard::BakeoffReportMirror) -> String;
```

It takes **`ForwardPlanView`** (the pure-`ui` mirror the SUGGEST screen already
renders), NOT `agent::config::ForwardPlan`. Grounds: the `from_plan` adapter that
reads the engine plan is `#[cfg(feature = "live")]`-gated, so a serialiser over
the raw plan would be live-gated and **untestable in the default `ui` build**;
`ForwardPlanView` is unit-constructible from fixtures and already carries the
short predicates (`is_short_capable`/`is_always_short`/`is_buy_and_hold`), the
`ConfidenceSummaryView`, and the pre-formatted `as_of_label`. `DataQualityView`,
`ScorecardView`, and the `OutcomeKind` are all reachable **through**
`BakeoffReportMirror`, so they are not separate arguments.

**D3 — Structural fidelity: mirror the screen's branch tree.** The serialiser
walks the SAME predicate tree that `screens/forward_plan.rs::view` +
`screens/leaderboard.rs::recommendation_block` walk, reusing the SAME
`crate::strings` constants, the SAME `crate::widgets::num` formatters, and the
SAME `crown_credibility` resolver — emitting text instead of widgets. This is the
structural guarantee that the export can never drift more prescriptive than, or
numerically inconsistent with, the surface it mirrors (R-HE.3 verbatim
numbers/copy). To reuse the resolver without duplicating logic, `crown_credibility`
+ `CrownCredibility` move (behaviour-preserving) from private in
`screens/leaderboard.rs` to `pub(crate)` in `crates/ui/src/leaderboard/state.rs`
(their data home); the render helper `crown_credibility_element` stays in the
screen.

**D4 — Format (Q-HE-1): markdown `.md`.** It preserves the co-located-caveat
structure the not-advice boundary depends on (headings + callouts) and stays
readable as raw text. A `.txt` variant is a trivial follow-on.

**D5 — Trigger + empty guard (Q-HE-2, Q-HE-6).** An "Export this plan" button
(new const `PLAN_EXPORT_BUTTON`) on the SUGGEST/`Screen::ForwardPlan` header — the
journey terminus. It renders ONLY when `forward_plan_screen_state.plan` is
`PanelState::Ready`; in `Empty`/`Loading`/`Error` it is absent (no empty-export
artifact). A `Message::ExportPlan` handler assembles the inputs from
`model.forward_plan_screen_state.plan` (Ready) + `model.leaderboard_screen_state`
(the mirror + narration) + `model.forward_fx`, calls `serialize_plan_export`, and
writes the file.

**D6 — Determinism + one-per-plan + no wall-clock (Q-HE-3, R-HE.1).** One
artifact per crowned plan (per-plan ≡ per-run today). The export path reads NO
wall-clock: it echoes the pre-formatted `plan.as_of_label` (already on the view)
and stamps the run identity with the **seed + last-bar**, NOT `now`. Determinism
is a property of the pure function over its inputs. Filename is deterministic:
`plan-{coin}-{window-slug}-{seed8}.md` (e.g. `plan-BTCUSDT-2024-h1-a1b2c3d4.md`),
where `window-slug` slugifies `range_label` and `seed8` is the first 8 hex chars
of the run seed. Same run ⇒ same name ⇒ idempotent overwrite; a different seed ⇒
a different suffix ⇒ no collision.

**D7 — Two mirror echo fields (value-echo, not computation).** The ratified
provenance footer names a **run seed** and an explicit window that are dropped at
the `from_report` mirror boundary today. `BakeoffReportMirror` gains ONE additive
echo field `run_seed: [u8; 32]`, populated in `from_report` from the
already-existing `report.request.seed`. This is a value echo inside `ui`, zero
engine computation, no `crates/backtest` change. The window line reuses the
existing `range_label` (the sanctioned window display already on the mirror); the
template's ISO dates are illustrative per the feature's § How-to-read. The seed
renders as lowercase hex in the provenance footer.

**D8 — Artifact home + anchor safety (R-HE.11).** The file lands under a
git-ignored workspace-root `plan-exports/` directory — the ADR-0055 `lab-runs/`
precedent: outside every `spec/**` anchor glob, so `verify_anchors.sh` stays
119/119 **by construction** (the git boundary is the guarantee, not reviewer
vigilance). `.gitignore` gains `/plan-exports/`. No `spec/*/reports/*.md` file is
created or read; no anchored scenario is touched; the 9 `anchors.toml` SHAs are
untouched; the FROZEN gate (`bakeoff/{robustness,rank,scorecard}.rs`) is
byte-untouched.

**D9 — Verification floor (Q-HE-5) + R-HE.12.** Golden-text serialiser tests
(default `ui` build, over fixtures) for the four variants — BenchmarkWins /
ActiveWins+WeakEvidence / ActiveWins+Passes / short-crowned — asserting the EXACT
ratified text, with negative controls (BenchmarkWins emits NO credibility badge;
the short-crowned fixture emits `SHORT_UNBOUNDED_LOSS_DISCLAIMER`) + a
byte-determinism test (same inputs ⇒ identical output) + a filename-determinism
test. The export button, a **new visible control on a journey screen**, gets a
**rendered-pixel proof** (macOS-gated, CLAUDE.md non-negotiable): it paints in
`Ready` and is ABSENT in `Empty` (the Q-HE-6 negative control). The CLAUDE.md
baseline-equity-divergence e2e is **N/A, recorded not skipped** (R-HE.12): P5
introduces no overlay/sizing-modifier/decision-variable and computes no equity —
a divergence gate would assert the opposite of the design goal (the P1/P3
precedent); the determinism + golden tests are the substitute gates.

## Alternatives considered

- **Serialise `agent::config::ForwardPlan` directly (the brief's literal
  signature)** — rejected: the `from_plan` adapter is `#[cfg(feature="live")]`,
  so the serialiser + its golden tests could not run in the default `ui` build.
  The `ForwardPlanView` mirror is the testable, layering-clean input.
- **Duplicate the credibility decision table in the serialiser** — rejected:
  two copies drift. Move the pure `crown_credibility` resolver to a shared
  `pub(crate)` home so screen and export read one source of truth.
- **Write to a user-chosen path via a file dialog (`rfd`)** — rejected for MVP:
  adds a dependency + defeats byte-determinism. A deterministic filename under a
  git-ignored dir is anchor-safe and reproducible; a "choose location" is an
  additive follow-on.
- **`.txt` output** — rejected as the primary: loses the co-located-caveat
  heading structure the not-advice boundary leans on. Trivial follow-on.
- **Add explicit `window_start/end` ISO echo fields to the mirror** — deferred:
  `range_label` is the already-sanctioned window display; ISO precision is an
  optional upgrade, not required by the ratified wording (its dates are
  illustrative).
- **A new anchored report** — rejected by construction: the export is a
  user-triggered artifact, never a `spec/*/reports/*.md` file.

## Consequences

- The export can only ever restate gates the product already passed; it is
  downstream of every credibility check and adds no number. If a future edit
  lets the serialiser compute a value, or read a clock, or place an order, it
  violates D2/D6/the register B-2 dead-end — caught by the determinism test and
  the "no `crates/backtest` diff" tester check.
- Anchor safety is by construction (D8): `verify_anchors.sh` MUST read 119/119
  before AND after; the git-ignored `plan-exports/` dir keeps the artifact out
  of every anchor glob. Enforced by the tester's before/after run.
- The ratified wording is the immutable contract: the ~27 new `crate::strings`
  consts (D3/the feature § Design) carry the § Draft wording text VERBATIM; a
  drift is a golden-text test failure.
- `crown_credibility` moving to `pub(crate)` in `leaderboard/state.rs` is
  behaviour-preserving; the screen's existing `crown_credibility_render.rs`
  proof still covers the render side.
- Adding `run_seed` to `BakeoffReportMirror` touches ~6 `fake_bakeoff_report_mirror*`
  fixtures (mechanical) + the one `from_report` line; no engine type crosses the
  seam (`[u8;32]` is `core`/std).
- PAPER/SIM ONLY. No live path ships. `ci.yml.deferred` untouched.

## Changelog
- 2026-07-10 (architect): initial accept — remediation-plan P5, feature
  `advisor-handoff-export`. Took 0088 (0087 was the last registered ADR, the
  sibling P4 `advisor-lot-realism`; 0088 verified free on disk + in the README
  Registry at authoring). Registered atomically (README Registry row +
  frontmatter `updated:`, same edit pass).
