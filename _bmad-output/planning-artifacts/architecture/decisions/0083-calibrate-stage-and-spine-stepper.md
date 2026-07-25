---
adr: 0083
title: Named "Calibrate" stage + the DATA→CALIBRATE→ANALYZE→SUGGEST spine stepper
status: accepted
date: 2026-07-09
supersedes: none
superseded-by: none
---

# ADR-0083: Named "Calibrate" stage + the DATA→CALIBRATE→ANALYZE→SUGGEST spine stepper

## Context

The advisor's workflow spine — DATA → CALIBRATE → ANALYZE → SUGGEST — is
*functionally* complete after v2 (every stage has honest content) but
**product-visibly implicit**: there is no "you are here" affordance tying the four
screens together, and the CALIBRATE stage is unnamed. v2 scoping (D7,
`spec/v2/v2-architecture.md` §6.0) **approved** promoting today's `Screen::Tune` to a
named "Calibrate" stage and adding the visible stepper, but **deferred** the
`agent::AdvisorStage` context-carrier "until the need is felt." The v3 close-out
phase (R3-3a, `docs/dev-notes/post-v2-scoping-2026-07-09.md` §3) builds the visible
IA only.

Grounding the design in the real `crates/ui/` tree surfaced the decisive constraint:
the four spine verbs do **not** map 1:1 to four screens. `Screen::Tune` is a Lab
drill-down reached only via `Message::OpenTuneEditor` (it is NOT in the sidebar
constants). And **DATA and ANALYZE both live inside `Screen::Leaderboard`** — the F3
guided input + the P1-7 data-quality panel (DATA) and the bake-off ranking +
scorecard (ANALYZE) are the top and bottom of one screen. A naive "stepper = router"
design is therefore wrong.

## Decision

**D1 — The stepper is an orientation band, not a router.** Add a `stage_stepper`
widget rendering a horizontal four-segment band (DATA · CALIBRATE · ANALYZE ·
SUGGEST) at the top of the shell's `centre` Column (`crates/ui/src/shell.rs`, above
`body`, mirroring the halted-banner placement) so it spans every advisor-journey
screen. It shows a "you are here" highlight and MAY offer click-navigation as a
convenience, but the spine's authoritative state stays in the existing screens. It is
elided (pixel-silent) on non-journey screens.

**D2 — Resolve the current stage from `current_screen` + the leaderboard substate.**
A pure `stage_for(screen, &leaderboard_state) -> Option<SpineStage>`:
`Leaderboard`+`Empty` → DATA; `Leaderboard`+`Ready` → ANALYZE; `Tune` → CALIBRATE;
`ForwardPlan` → SUGGEST; anything else → `None`. The DATA/ANALYZE discriminator reads
the EXISTING `PanelState` — no new state field. Pure + unit-tested; the render test
is the load-bearing proof.

**D3 — DEFER the `agent::AdvisorStage` context-carrier again.** The stepper reads a
UI concern (`current_screen`); it does NOT thread coin+budget+window+crown through an
agent-side context object. D7's deferral still holds (2026-07-09) — building the
carrier now adds `ui`↔`agent` coupling for zero present benefit.

**D4 — Promote `Screen::Tune` to a sidebar-visible "Calibrate" stage.** Insert
`Screen::Tune` into BOTH `SIDEBAR_ENTRIES_PHASE_A` and the Work sub-array of
`SIDEBAR_GROUPS_PHASE_C` (`crates/ui/src/theme.rs`), in lock-step, between
`Leaderboard` and `ForwardPlan`. The **display label is "Calibrate"**; the enum
variant stays `Screen::Tune` (source-compat). The existing `OpenTuneEditor`
drill-down still works; the sidebar entry is an ADDITIONAL unseeded entry point.

**D5 — Zero literals / hex; no new dependency.** Verb labels + the "Calibrate" label
are `crate::strings` constants registered in `strings::all()`; colours are
`crate::theme` tokens (reuse existing accent/foreground — no new token). No new crate
(`cargo tree -p ui` unchanged).

**D6 — Verification is at the rendered-pixel layer (CLAUDE.md non-negotiable).** A
new `#![cfg(target_os = "macos")]` render harness
(`crates/ui/tests/stage_stepper_render.rs`, the `leaderboard_populated_render.rs`
pattern) proves: (a) `current_screen = Tune` highlights CALIBRATE; (b) the negative
control `current_screen = ForwardPlan` highlights SUGGEST and NOT CALIBRATE; (c) the
DATA/ANALYZE discriminator renders correctly. A passing model state / text snapshot /
no-panic boot is NOT sufficient.

**Anchor + gate safety.** UI-only, `write_report=false` advisor paths untouched → no
anchored CLI path reads any new UI state → **anchors 119/119 by construction**. The
FROZEN gate (`bakeoff/{robustness,rank}.rs`) is not read or changed. The sidebar
flatten-invariant test stays green (both constants moved together).

## Alternatives considered

- **Stepper as a strict 1:1 router (split `Screen::Leaderboard` into a DATA screen +
  an ANALYZE screen)** — rejected: fractures a cohesive single-surface screen (input
  at top, result below), adds a nav hop mid-journey, and churns every leaderboard
  test/harness for pure taxonomy. The D2 substate discriminator is cheaper and
  honest.
- **Build the `agent::AdvisorStage` context-carrier now (full D7)** — rejected/
  deferred: adds `ui`↔`agent` coupling for zero present benefit; "until the need is
  felt" is still true. The stepper needs only `current_screen`.
- **A new dedicated `Screen::Calibrate` variant (rename Tune)** — rejected: churns
  every `Screen::Tune` match/test/harness for a display change achievable with a
  label string; the variant name is not user-visible.
- **Put the stepper inside each screen body instead of the shell** — rejected:
  duplicates the band N times and risks per-screen drift; the shell `centre` Column
  is the single consistent home (the halted-banner precedent).

## Consequences

- The advisor spine becomes product-visible: a first-time viewer sees DATA →
  CALIBRATE → ANALYZE → SUGGEST and where they are. CALIBRATE is a named,
  sidebar-reachable stage.
- The sidebar flatten-invariant (`sidebar_groups_phase_c__flatten_matches_phase_a`,
  `crates/ui/src/theme.rs`) now guards the Tune entry too — editing only one of the
  two constants breaks the build. That test is the mechanical enforcement of D4.
- The render harness `stage_stepper_render.rs` is the mechanical enforcement of D6;
  the `stage_for` unit tests enforce D2. If either regresses, the stepper is
  presumed broken.
- If the DATA/ANALYZE substate discriminator feels wrong at the render-review, the
  sanctioned fallback is a merged "DATA·ANALYZE" highlight or default-to-ANALYZE on
  Leaderboard — a `view`-time change, no ADR amendment needed (D2 names the seam).
- The `agent::AdvisorStage` carrier remains deferred; if a future feature needs
  carried journey context (e.g. a "resume where you left off" flow), it supersedes
  this ADR's D3.

## Changelog
- 2026-07-09 (architect): initial accept. Scoped as R3-3a of the v3 close-out phase
  (feature `advisor-calibrate-stage`). Formalizes v2 D7 (promote Tune, defer the
  carrier). Grounded in the real `crates/ui/` seams. Registered atomically in
  `_bmad-output/planning-artifacts/architecture/decisions/README.md`.
