---
slug: advisor-calibrate-stage
status: dev-done
owner: ui-designer
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

- [x] T0. `bash scripts/verify_anchors.sh` → confirm **119 / 119** BEFORE any edit.
      ✅ 119/119 confirmed pre-edit.
- [x] T1. Read the grounding seams named in `feature.md` § Verified ground truth:
      `crates/ui/src/state.rs` (`Screen` enum), `crates/ui/src/theme.rs:747`
      (`SIDEBAR_ENTRIES_PHASE_A`) + `:788` (`SIDEBAR_GROUPS_PHASE_C`) + the
      flatten-invariant test `:1631`, `crates/ui/src/shell.rs:43/78` (shell
      composition), `crates/ui/src/screens/leaderboard.rs` (F3 input + data-quality
      panel + ready-pane), `crates/ui/tests/leaderboard_populated_render.rs` (render
      harness pattern).

## Build — the spine stepper widget (Decision 1 + 2)

- [x] T2. Add a `SpineStage` enum (`Data | Calibrate | Analyze | Suggest`) — home it
      in `crates/ui/src/` alongside the stepper widget (proposed:
      `crates/ui/src/widgets/stage_stepper.rs`). Plain UI-owned type; derive
      `Copy, Clone, Debug, PartialEq, Eq`.
- [x] T3. Write the pure mapping `stage_for(screen: Screen, leaderboard_state: &…)
      -> Option<SpineStage>` per the `feature.md` § Design D2 table. The
      DATA/ANALYZE discriminator reads the EXISTING leaderboard panel substate
      (`PanelState::Empty` → `Data`, a `Ready` bake-off result → `Analyze`) — do NOT
      add a new state field. Return `None` for non-journey screens.
      ✅ `stage_for<T>(Screen, &PanelState<T>)` — generic over the panel payload so it
      reads the existing `LeaderboardScreenState::result` with no new field.
      `Loading`/`Error` on Leaderboard → `Data` (still the input surface).
- [x] T4. Build `stage_stepper::view(current: Option<SpineStage>, mode) ->
      Element` — a horizontal four-segment band (DATA · CALIBRATE · ANALYZE ·
      SUGGEST) with the current segment highlighted (accent token) and the rest in
      the neutral/foreground token. Compose existing widgets/tokens; no new theme
      token, no new dependency. Return a pixel-silent/elided element when `current`
      is `None`.
      ✅ Active = SOLID `ACCENT` chip + `FG_ON_ACCENT` + `●` marker; inactive =
      `PANEL_RAISED` + `FG_2`; `›` chevrons. `None` → 0-sized `Space`.
- [x] T5. Register the 4 verb-label string constants + register them in
      `crate::strings::all()` (`crates/ui/src/strings.rs`). No literal in the widget
      body.
      ✅ `SPINE_STAGE_{DATA,CALIBRATE,ANALYZE,SUGGEST}` + `CALIBRATE_SIDEBAR_LABEL`
      (5 total) registered; consistency gate (no inline strings/hex) GREEN.
- [x] T6. Wire the stepper into the shell: in `crates/ui/src/shell.rs`, push
      `stage_stepper::view(stage_for(model.current_screen, …), mode)` at the TOP of
      the `centre` Column (above `body`, mirroring the halted-banner placement). It
      must render consistently across screens (elided off the journey).
      ✅ `centre = Column[stepper, body, bar]`; `stage_for(model.current_screen,
      &model.leaderboard_screen_state.result)`.
- [~] T7. (Optional convenience) If a stepper segment is clickable… If click-nav adds
      risk, ship the stepper display-only for R3-3a and note it.
      ✅ CHOSE DISPLAY-ONLY (view-time decision, no ADR amendment): the sidebar (now
      with Calibrate) routes every stage; a clickable band risks "click DATA/ANALYZE
      both go to Leaderboard" ambiguity. Widget takes `Option<SpineStage>` → click-nav
      is a clean additive follow-up. Noted in feature.md § UI.

## Build — promote Tune to a sidebar "Calibrate" stage (Decision 3)

- [x] T8. In `crates/ui/src/theme.rs`: insert `Screen::Tune` into
      `SIDEBAR_ENTRIES_PHASE_A` between `Screen::Leaderboard` and
      `Screen::ForwardPlan`, AND into the **Work** sub-array of
      `SIDEBAR_GROUPS_PHASE_C` in the SAME position. **Both must move in
      lock-step** or the flatten-invariant test fails.
      ✅ Both constants edited in lock-step; flatten-invariant test GREEN.
- [x] T9. Give the sidebar entry the display label **"Calibrate"** (NOT "Tune") — add
      the sidebar label string constant, register it in `strings::all()`, and wire it
      wherever `sidebar_nav` resolves a `Screen` to its label. The enum variant STAYS
      `Screen::Tune` (source-compat); only the display string changes.
      ✅ `label_for(Screen::Tune) => CALIBRATE_SIDEBAR_LABEL`; enum unchanged. Sidebar
      snapshots updated (show `label=Calibrate screen=Tune` between Leaderboard/Plan).
- [x] T10. Confirm the existing `OpenTuneEditor` drill-down still works (preseeded
      family/coin/lookback from the Leaderboard) AND the new sidebar entry opens the
      Tune form unseeded. Both entry points route to `Screen::Tune` → `tune::view`.
      ✅ Both route to `Screen::Tune` → `tune::view` (unchanged body — I touched only
      the sidebar constants + the label resolver, not the `SwitchScreen`/`OpenTuneEditor`
      arms). Tester to confirm live navigation.

## Verification — render layer (Decision 5; CLAUDE.md non-negotiable)

- [x] T11. New render harness `crates/ui/tests/stage_stepper_render.rs`,
      `#![cfg(target_os = "macos")]` (ADR-0057), modelled on
      `leaderboard_populated_render.rs`. Write PNGs to `/tmp/` for operator eyeball.
      ✅ 4 tests, all PNGs read + eyeballed. Uses `program_from_cockpit` (full shell).
      - [x] T11a. `stepper_highlights_current_stage` — `current_screen = Tune`:
            CALIBRATE segment paints its ACCENT highlight (>200 band-teal px).
            → `/tmp/stage_stepper_calibrate.png`
      - [x] T11b. **Negative control** `stepper_highlight_moves_with_screen` —
            `current_screen = ForwardPlan`: SUGGEST highlighted; the ACCENT centroid
            moves RIGHT ≥ 2 segment widths vs the Tune frame (proves non-tautology).
            → `/tmp/stage_stepper_suggest.png`
      - [x] T11c. DATA/ANALYZE discriminator — Leaderboard+`Empty` → DATA;
            Leaderboard+`Ready` → ANALYZE; the highlight moves RIGHT on the SAME screen
            purely from the substate flip.
            → `/tmp/stage_stepper_data.png`, `/tmp/stage_stepper_analyze.png`
      - [x] T11d. (added) off-journey elision — `Screen::Lab`: ~no band teal.
            → `/tmp/stage_stepper_off_journey.png`
- [x] T12. Pure-function unit tests for `stage_for(...)` covering EVERY row of the D2
      table (incl. the `None` non-journey case). Home them next to the widget.
      ✅ 9 unit tests in `widgets::stage_stepper::tests` (incl. every non-journey +
      alias screen → `None`, and `Loading`/`Error` → `Data`).
- [x] T13. Confirm the sidebar flatten-invariant test
      `theme::layout::tests::sidebar_groups_phase_c__flatten_matches_phase_a` passes.
      ✅ GREEN (both constants moved in lock-step).

## Gates — run before HANDOFF → tester

- [x] T14. `cargo test -p ui --lib` green (existing + new unit tests). ✅ 597 passed.
- [x] T15. `cargo test -p ui --test stage_stepper_render` green on macOS; eyeball the
      `/tmp/` PNGs — the highlight visibly moves. ✅ 4 passed; PNGs read (the harness
      auto-enables `fixtures` under `#[cfg(test)]`, so no explicit `--features` needed).
- [x] T16. `cargo clippy -p ui --tests -- -D warnings` clean. ✅ (also `-p ui` clean.)
- [x] T17. `cargo fmt --check` clean. ✅
- [x] T18. `cargo tree -p ui` UNCHANGED vs `main` (no new dependency edge). ✅ No
      `Cargo.toml`/`Cargo.lock` change; widget composes existing primitives only.
- [x] T19. `bash scripts/verify_anchors.sh` → **119 / 119** AFTER (no anchor churn). ✅
- [x] T20. `python3 scripts/spec_lint.py` → PASS (0). ✅ `spec-lint: PASS (0 violations)`.

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
