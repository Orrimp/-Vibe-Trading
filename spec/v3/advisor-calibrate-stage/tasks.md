---
slug: advisor-calibrate-stage
status: arch-done
owner: architect
updated: 2026-07-09
version: 3.0.0
---

# Tasks — advisor-calibrate-stage (R3-3a)

Ordered checklist for the **ui-designer** (+ developer for the Rust wiring). Every
file path is absolute-from-repo-root. Design + rationale: [`feature.md`](feature.md);
ADR-0083.

**Guardrails for every task:** no `crates/backtest/src/bakeoff/{robustness,rank}.rs`
edit; no `spec/**/reports/` edit; no new dependency (`cargo tree -p ui` must not
change); zero hardcoded string literals / hex colours in UI (use `crate::strings` +
`crate::theme`); do NOT build the `agent::AdvisorStage` context-carrier.

## Pre-flight

- [ ] T0. `bash scripts/verify_anchors.sh` → confirm **119 / 119** BEFORE any edit.
- [ ] T1. Read the grounding seams named in `feature.md` § Verified ground truth:
      `crates/ui/src/state.rs` (`Screen` enum), `crates/ui/src/theme.rs:747`
      (`SIDEBAR_ENTRIES_PHASE_A`) + `:788` (`SIDEBAR_GROUPS_PHASE_C`) + the
      flatten-invariant test `:1631`, `crates/ui/src/shell.rs:43/78` (shell
      composition), `crates/ui/src/screens/leaderboard.rs` (F3 input + data-quality
      panel + ready-pane), `crates/ui/tests/leaderboard_populated_render.rs` (render
      harness pattern).

## Build — the spine stepper widget (Decision 1 + 2)

- [ ] T2. Add a `SpineStage` enum (`Data | Calibrate | Analyze | Suggest`) — home it
      in `crates/ui/src/` alongside the stepper widget (proposed:
      `crates/ui/src/widgets/stage_stepper.rs`). Plain UI-owned type; derive
      `Copy, Clone, Debug, PartialEq, Eq`.
- [ ] T3. Write the pure mapping `stage_for(screen: Screen, leaderboard_state: &…)
      -> Option<SpineStage>` per the `feature.md` § Design D2 table. The
      DATA/ANALYZE discriminator reads the EXISTING leaderboard panel substate
      (`PanelState::Empty` → `Data`, a `Ready` bake-off result → `Analyze`) — do NOT
      add a new state field. Return `None` for non-journey screens.
- [ ] T4. Build `stage_stepper::view(current: Option<SpineStage>, mode) ->
      Element` — a horizontal four-segment band (DATA · CALIBRATE · ANALYZE ·
      SUGGEST) with the current segment highlighted (accent token) and the rest in
      the neutral/foreground token. Compose existing widgets/tokens; no new theme
      token, no new dependency. Return a pixel-silent/elided element when `current`
      is `None`.
- [ ] T5. Register the 4 verb-label string constants + register them in
      `crate::strings::all()` (`crates/ui/src/strings.rs`). No literal in the widget
      body.
- [ ] T6. Wire the stepper into the shell: in `crates/ui/src/shell.rs`, push
      `stage_stepper::view(stage_for(model.current_screen, …), mode)` at the TOP of
      the `centre` Column (above `body`, mirroring the halted-banner placement). It
      must render consistently across screens (elided off the journey).
- [ ] T7. (Optional convenience) If a stepper segment is clickable, emit the existing
      `SwitchScreen`/nav message to that stage's primary screen. Do NOT invent a new
      routing mechanism; do NOT thread coin/budget/window (that is the deferred
      carrier). If click-nav adds risk, ship the stepper display-only for R3-3a and
      note it.

## Build — promote Tune to a sidebar "Calibrate" stage (Decision 3)

- [ ] T8. In `crates/ui/src/theme.rs`: insert `Screen::Tune` into
      `SIDEBAR_ENTRIES_PHASE_A` (`:747`) between `Screen::Leaderboard` and
      `Screen::ForwardPlan`, AND into the **Work** sub-array of
      `SIDEBAR_GROUPS_PHASE_C` (`:788`) in the SAME position. **Both must move in
      lock-step** or the flatten-invariant test fails.
- [ ] T9. Give the sidebar entry the display label **"Calibrate"** (NOT "Tune") — add
      the `LEADERBOARD`/sidebar label string constant, register it in
      `strings::all()`, and wire it wherever `sidebar_nav` resolves a `Screen` to its
      label (`crates/ui/src/widgets/sidebar_nav.rs`). The enum variant STAYS
      `Screen::Tune` (source-compat); only the display string changes.
- [ ] T10. Confirm the existing `OpenTuneEditor` drill-down still works (preseeded
      family/coin/lookback from the Leaderboard) AND the new sidebar entry opens the
      Tune form unseeded (default/last state). Both entry points route to
      `Screen::Tune` → `tune::view` (unchanged body).

## Verification — render layer (Decision 5; CLAUDE.md non-negotiable)

- [ ] T11. New render harness `crates/ui/tests/stage_stepper_render.rs`,
      `#![cfg(target_os = "macos")]` (ADR-0057), modelled on
      `leaderboard_populated_render.rs`. Write PNGs to `/tmp/` for operator eyeball.
      - [ ] T11a. `stepper_highlights_current_stage` — shell with
            `current_screen = Tune`: assert the CALIBRATE segment paints its accent
            highlight AND the other three paint the non-highlighted style.
      - [ ] T11b. **Negative control** `stepper_highlight_moves_with_screen` — same
            harness, `current_screen = ForwardPlan`: assert SUGGEST is highlighted
            and CALIBRATE is NOT (proves T11a is not a tautology).
      - [ ] T11c. DATA/ANALYZE discriminator — Leaderboard with `PanelState::Empty`
            highlights DATA; with a `Ready` bake-off result highlights ANALYZE.
- [ ] T12. Pure-function unit tests for `stage_for(...)` covering EVERY row of the D2
      table (incl. the `None` non-journey case). Home them next to the widget.
- [ ] T13. Confirm the sidebar flatten-invariant test
      `theme::layout::tests::sidebar_groups_phase_c__flatten_matches_phase_a` passes
      (it will fail if T8 moved only one constant).

## Gates — run before HANDOFF → tester

- [ ] T14. `cargo test -p ui --lib` green (existing + new unit tests).
- [ ] T15. `cargo test -p ui --test stage_stepper_render --features fixtures`
      (or the crate's render-test feature) green on macOS; eyeball the `/tmp/` PNGs —
      the highlight must visibly move from CALIBRATE to SUGGEST between T11a/T11b.
- [ ] T16. `cargo clippy -p ui --tests -- -D warnings` clean.
- [ ] T17. `cargo fmt --check` clean.
- [ ] T18. `cargo tree -p ui` UNCHANGED vs `main` (no new dependency edge).
- [ ] T19. `bash scripts/verify_anchors.sh` → **119 / 119** AFTER (no anchor churn).
- [ ] T20. `python3 scripts/spec_lint.py` → PASS (0) — the feature folder + trace row
      resolve cleanly under `spec/v3/`.

## For the tester to independently verify

The tester MUST re-run T14–T20 from a fresh checkout of the branch (do not trust the
developer's session), AND read the rendered PNGs (T11) themselves — the render-layer
proof is the whole point (MEMORY.md "verify UI at the render layer"). Confirm the
stepper highlight tracks `current_screen` and the "Calibrate" sidebar entry is
present + navigates.

## Definition of done

- Stepper renders + highlights correctly (render-verified, both PNGs eyeballed).
- "Calibrate" sidebar entry present; flatten-invariant green.
- `AdvisorStage` carrier NOT built (deferred).
- All gates T14–T20 green; ADR-0083 accepted + registered.
- `feature.md` flips to the next lifecycle status by the OWNER as the pipeline
  advances (arch-done → dev-done → tester-done → shipped); trace row mirrors it
  (ADR-0082).
