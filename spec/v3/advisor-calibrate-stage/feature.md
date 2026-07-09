---
slug: advisor-calibrate-stage
status: dev-done
owner: ui-designer
updated: 2026-07-09
version: 3.0.0
---

# advisor-calibrate-stage — the named "Calibrate" stage + the spine stepper (R3-3a)

> **One-line:** promote today's `Screen::Tune` (a Lab drill-down) to a first-class,
> sidebar-visible **"Calibrate"** stage, and add a **DATA → CALIBRATE → ANALYZE →
> SUGGEST** stepper across the existing advisor screens so a first-time viewer can
> see the spine. Build the **visible** stepper only; **defer** the
> `agent::AdvisorStage` context-carrier again (D7 — "until the need is felt").

This is the single real build of the v3 close-out phase. It is a **UI / information-
architecture** feature: no engine code, no gate touch, no new anchors. Owner is the
**ui-designer** (with the developer if the Rust wiring warrants a second lane).

## Why (the gap this closes)

The v2-analysis flagged: "the cockpit stops at Return/Sharpe/MaxDD + a headline… no
explicit data→train→analyze→suggest spine ties the screens." v2 closed the *content*
gap (every stage now has honest content — DATA-quality panel, gate-tied sweep,
scorecard, forward plan). What remains is **product-visible IA**: the spine is real
but *implicit*. Two seams were approved-but-deferred at v2 scoping (D7,
`spec/v2/v2-architecture.md` §6.0):

1. The **visible cross-stage stepper** — there is no "you are here" affordance tying
   the four screens into one journey.
2. **Naming the Calibrate stage** — `Screen::Tune` is still a Lab drill-down, not a
   first-class named stage.

R3-3a builds both. It does **not** build the context-carrier (see § Non-goals).

## Verified ground truth (read the code before designing)

Grounded in the current `crates/ui/` tree (2026-07-09), not spec prose:

- **`Screen` enum** — `crates/ui/src/state.rs` (`pub enum Screen`, ~line 114). The
  advisor journey screens are `Leaderboard`, `Tune`, `ForwardPlan` (plus `Baseline`,
  `Live`). `Tune` exists and renders (`Screen::Tune => tune::view` at
  `crates/ui/src/shell.rs:161`).
- **`Screen::Tune` is drill-down-only.** It is **NOT** in `SIDEBAR_ENTRIES_PHASE_A`
  or `SIDEBAR_GROUPS_PHASE_C` (`crates/ui/src/theme.rs:747`, `:788`). It is reached
  ONLY via `Message::OpenTuneEditor { family, coin, lookback }`
  (`crates/ui/src/state.rs`, handled ~line 3480) — a per-row "Tune…" affordance off
  the Leaderboard (the `InspectStrategyFromLeaderboard` precedent).
- **The four spine verbs do NOT map 1:1 to four screens.** This is the crux:
  - **DATA** — the F3 guided input (coin + budget + lookback) **and** the P1-7
    data-quality panel both live INSIDE `Screen::Leaderboard`
    (`crates/ui/src/screens/leaderboard.rs:155` guided input; `:459`,`:781`
    `data_quality_block`).
  - **CALIBRATE** — `Screen::Tune` (the gate-tied sweep, ADR-0069).
  - **ANALYZE** — the bake-off ranking + scorecard, ALSO `Screen::Leaderboard`
    (the ready-pane table + recommendation + Risk-story + scorecard blocks).
  - **SUGGEST** — `Screen::ForwardPlan` (F6).
  → **DATA and ANALYZE share `Screen::Leaderboard`.** So the stepper is an
  **orientation affordance** ("you are here" across the journey), NOT a strict 1:1
  router. See § Design for how the current stage is resolved.
- **Shell composition (where the stepper renders).** `shell::view`
  (`crates/ui/src/shell.rs:43`) builds `Row[ sidebar | centre | right_rail ]`, where
  `centre = Column[ body, status_bar ]` (`shell.rs:78`). The stepper is a NEW
  horizontal band pushed at the **top of `centre`, above `body`**, so it spans every
  screen consistently (the halted-banner precedent in the same file).
- **Sidebar IA invariant (load-bearing).** `SIDEBAR_GROUPS_PHASE_C` flattened must
  equal `SIDEBAR_ENTRIES_PHASE_A` — guarded by
  `theme::layout::tests::sidebar_groups_phase_c__flatten_matches_phase_a`
  (`crates/ui/src/theme.rs:1631`). Adding `Tune` to the sidebar **must update BOTH
  constants in lock-step** or this test fails. This is the primary structural gate.
- **Render-harness precedent.** `crates/ui/tests/leaderboard_populated_render.rs`
  (`#![cfg(target_os = "macos")]`, populated fixture + negative control, PNG to
  `/tmp/`) is the exact pattern the stepper's render proof follows (ADR-0057 macOS
  gate).

## Non-goals (explicit deferrals — do NOT build these)

- **The `agent::AdvisorStage` context-carrier.** D7 deferred it "until the need is
  felt," and that is **still true** (2026-07-09, re-confirmed). The stepper reads
  `current_screen` (a UI concern); it does NOT thread coin+budget+window+crown
  through a cross-stage agent-side context object. Building the carrier now would add
  a `ui`↔`agent` coupling for zero present benefit. **Deferred again.**
- **No crown-eligibility veto / gate change.** Out of scope entirely (that is R3-3b,
  which *documents* the report-only decision; the veto stays unbuilt).
- **No PBO plumbing on the Tune surface.** (Deferred v2 item R2; not part of R3-3a.)
- **No new screen bodies.** `Tune`'s existing `tune::view` is reused verbatim; only
  its *promotion* (sidebar entry + stepper) is new. DATA/ANALYZE keep their
  `Leaderboard` home.

## Design

### Decision 1 — the stepper is an orientation band, not a router (ADR-0083 D1)

A new `stage_stepper` widget renders a horizontal four-segment band —
**DATA · CALIBRATE · ANALYZE · SUGGEST** — pushed at the top of the shell's `centre`
Column (`shell.rs`, above `body`). Each segment shows the verb + a "you are here"
highlight. It is **display + navigation orientation**, not authoritative routing:
clicking a segment MAY navigate to that stage's primary screen (a convenience), but
the spine's real state lives in the existing screens. The band is pixel-silent on
screens outside the advisor journey (renders the same neutral band, or is elided on
non-journey screens — a `view`-time decision, see D3).

### Decision 2 — map `current_screen` → the highlighted stage (ADR-0083 D2)

Because DATA and ANALYZE share `Screen::Leaderboard`, the mapping is a small pure
function `stage_for(screen, &leaderboard_substate) -> Option<SpineStage>`:

| `current_screen` | Highlighted stage | Note |
|---|---|---|
| `Leaderboard` (no result yet / input focus) | **DATA** | Before a bake-off has run — the operator is filling F3 input / reading the data-quality panel. |
| `Leaderboard` (a `Ready` bake-off result present) | **ANALYZE** | After the run — the ranked table + scorecard are the analysis. |
| `Tune` | **CALIBRATE** | The named stage. |
| `ForwardPlan` | **SUGGEST** | The forward plan. |
| any other screen | `None` | Not on the advisor journey → band elided (D3). |

The DATA/ANALYZE discriminator reads the **already-present** leaderboard panel state
(`PanelState::Empty` vs `Ready`) — NO new state field. Keep the function pure and
unit-tested; keep the render test as the real proof.

> **Alternative rejected:** split `Screen::Leaderboard` into two screens (a DATA
> screen + an ANALYZE screen) so the mapping is 1:1. Rejected — it fractures a
> cohesive screen the operator uses as one surface (input at top, result below), adds
> a nav hop mid-journey, and churns every leaderboard test/harness for pure taxonomy.
> The substate discriminator is cheaper and honest. (ADR-0083 § Alternatives.)

### Decision 3 — promote `Tune` to a sidebar-visible "Calibrate" stage (ADR-0083 D4)

`Screen::Tune` gains a **sidebar entry labelled "Calibrate"** (the enum variant name
stays `Tune` for source-compat; only the display string is "Calibrate"). It is
inserted into the **Work** group between `Leaderboard` and `ForwardPlan` (the spine
order DATA/ANALYZE → CALIBRATE → SUGGEST places Calibrate after the leaderboard it
tunes and before the forward plan). **Both** `SIDEBAR_ENTRIES_PHASE_A` and
`SIDEBAR_GROUPS_PHASE_C` are updated in lock-step (the flatten-invariant test is the
guard). The existing `OpenTuneEditor` drill-down still works (a preseeded entry from
the Leaderboard); the sidebar entry is an ADDITIONAL, unseeded entry point (opens the
Tune form in its default/last state).

> Placement note: the sidebar order becomes Lab · Live · Compare · Baseline ·
> Leaderboard · **Calibrate** · ForwardPlan · Strategies · … The advisor sub-journey
> (Leaderboard → Calibrate → ForwardPlan) reads top-to-bottom in the Work group,
> matching the stepper's left-to-right order.

### Decision 4 — strings + tokens (ADR-0083 D5)

Zero hardcoded literals / hex (CLAUDE.md UI rule): the four verb labels + the
"Calibrate" sidebar label are new `crate::strings` constants registered in
`strings::all()`; colours are `crate::theme` tokens (reuse the accent/foreground
tokens the sidebar + scorecard blocks already use — no new theme token). The stepper
widget composes existing primitives; no new dependency (`cargo tree -p ui`
unchanged).

### Decision 5 — verification floor (ADR-0083 D6; CLAUDE.md non-negotiable)

The stepper MUST be proven at the **rendered-pixel layer** with a populated state +
a negative control (per `spec/dev-notes/iced-ui-render-verification.md`):

1. **`stepper_highlights_current_stage`** — render the shell (or the stepper widget
   in a harnessed shell) with `current_screen = Tune`; assert the **CALIBRATE**
   segment paints its highlight (accent hue present at the expected band region) AND
   the other three segments paint their non-highlighted style.
2. **Negative control `stepper_highlight_moves_with_screen`** — same harness with
   `current_screen = ForwardPlan`; assert the **SUGGEST** segment is highlighted and
   CALIBRATE is NOT (proves guard 1 is not a tautology — the highlight genuinely
   tracks the screen).
3. **DATA/ANALYZE discriminator** — a leaderboard-hosted render with
   `PanelState::Empty` highlights **DATA**; with a `Ready` bake-off result highlights
   **ANALYZE** (the substate mapping renders correctly).
4. Pure-function unit tests for `stage_for(...)` covering every row of the D2 table.

macOS-gated (`#![cfg(target_os = "macos")]`, ADR-0057). PNG written to `/tmp/` for
operator eyeballing. A passing model state / text snapshot / no-panic boot is
explicitly NOT sufficient.

### Anchor + gate safety

- **Anchors 119/119 by construction.** This is UI-only (`write_report=false` advisor
  paths are untouched; no anchored CLI path reads any new UI state). Zero anchors
  added. Run `bash scripts/verify_anchors.sh` before AND after → 119/119 both.
- **FROZEN gate untouched.** No `crates/backtest/src/bakeoff/{robustness,rank}.rs`
  edit. This feature does not read or change any verdict.
- **Sidebar flatten-invariant** stays green (both constants edited together).

## Acceptance criteria

- A visible **DATA → CALIBRATE → ANALYZE → SUGGEST** stepper renders at the top of
  every advisor-journey screen, highlighting the current stage.
- `Screen::Tune` appears in the sidebar labelled **"Calibrate"** in the Work group,
  between Leaderboard and ForwardPlan; the flatten-invariant test passes.
- The DATA/ANALYZE discriminator works (Leaderboard empty → DATA; Leaderboard ready
  → ANALYZE) via the existing panel substate, no new state field.
- The `agent::AdvisorStage` context-carrier is **NOT** built (deferred again).
- Render-layer proof (populated + negative control, macOS-gated) is green and the
  PNGs visibly show the correct highlight moving with the screen.
- `bash scripts/verify_anchors.sh` → 119/119 before AND after.
- `cargo clippy -p ui --tests -- -D warnings` clean; `cargo fmt --check` clean;
  `cargo tree -p ui` unchanged (no new dependency).
- ADR-0083 accepted + registered atomically in
  `spec/architecture/adr/README.md`.

## Risks

- **DATA/ANALYZE-on-one-screen ambiguity.** The substate discriminator is a small
  heuristic; if it feels wrong in the rendered walk, the fallback is to highlight a
  merged "DATA·ANALYZE" or default to ANALYZE whenever on Leaderboard. Decide at the
  render-review, not before. (Flagged for the ui-designer.)
- **Sidebar flatten-invariant** is easy to trip — the two constants MUST move
  together. The test catches it; call it out in tasks.
- **Font-mutex / cosmic-text** render-test flake (the known param_sweep_render
  caution) — follow the existing macOS-gated harness pattern and coarse hue
  thresholds.
- **Scope creep toward the context-carrier.** If a task starts threading coin/budget
  through `agent`, STOP — that is the deferred non-goal.

## Trace

`REQ-V3-R3-3A-CALIBRATE-STAGE-001` in [`spec/trace.toml`](../../trace.toml), state
`arch-done` (design-complete; NOT shipped — honoring ADR-0082). Arch refs:
`spec/v3/README.md`, this feature, `spec/architecture/adr/0083-calibrate-stage-and-spine-stepper.md`.

## UI

### Wireframe (the band, rendered)

```
┌──────────┬──────────────────────────────────────────────────────────────────┐
│ Lab      │  Data › [● Calibrate] › Analyze › Suggest        ← spine stepper   │
│ Live     │ ─────────────────────────────────────────────────────────────────│
│ Compare  │  Tune parameters                                                  │
│ Baseline │  Sweep a strategy's parameters and see how each config holds up…  │
│ Leaderb… │  ┌ Choose a parameter grid ─────────────────────────────────────┐│
│ Calibrate│  │ [SMA crossover] MACD  RSI  Bollinger bands                    ││
│ Plan     │  │ …                                                            ││
│ ──────   │  └──────────────────────────────────────────────────────────────┘│
│ Strateg… │                                                                   │
│ …        │                                        [status bar]               │
└──────────┴──────────────────────────────────────────────────────────────────┘
```

The band is the FIRST child of the shell `centre` Column (above `body`, below
nothing) — it spans every advisor-journey screen. The active segment paints a
SOLID `ACCENT` chip with `FG_ON_ACCENT` text + a leading `●` marker (shape
signal, not colour-only); the rest are `PANEL_RAISED` chips with `FG_2` text,
`›` chevrons between. Off the journey the band is elided (a 0-sized `Space`).

The four rendered states (all pixel-verified — read the PNGs):

| screen / substate            | highlight   | PNG                              |
|------------------------------|-------------|----------------------------------|
| `Tune`                       | ● Calibrate | `/tmp/stage_stepper_calibrate.png` |
| `ForwardPlan`                | ● Suggest   | `/tmp/stage_stepper_suggest.png`   |
| `Leaderboard` + `Empty`      | ● Data      | `/tmp/stage_stepper_data.png`      |
| `Leaderboard` + `Ready`      | ● Analyze   | `/tmp/stage_stepper_analyze.png`   |
| `Lab` (off-journey)          | (elided)    | `/tmp/stage_stepper_off_journey.png` |

### New screens / panels / widgets

- **`crates/ui/src/widgets/stage_stepper.rs`** (new) — `SpineStage` enum
  (`Data|Calibrate|Analyze|Suggest`), the pure `stage_for(screen,
  &PanelState<T>) -> Option<SpineStage>` mapping, and `view(Option<SpineStage>,
  mode)`. Registered `pub mod` in `widgets/mod.rs` + a gallery cell
  (`stage_stepper :: calibrate_highlighted`) in `gallery/routes.rs`.
- **`Screen::Tune` promoted** to a sidebar-visible **"Calibrate"** entry (Work
  group, between Leaderboard and ForwardPlan) — the enum variant is unchanged;
  `sidebar_nav::label_for(Screen::Tune)` now resolves `CALIBRATE_SIDEBAR_LABEL`.

### New strings (`ui::strings`)

`CALIBRATE_SIDEBAR_LABEL`, `SPINE_STAGE_DATA`, `SPINE_STAGE_CALIBRATE`,
`SPINE_STAGE_ANALYZE`, `SPINE_STAGE_SUGGEST` — all registered in `strings::all()`.
(`TUNE_SIDEBAR_LABEL` retained but superseded as the sidebar-row label.)

### New theme tokens

**Zero.** The band composes existing tokens only (`ACCENT`, `FG_ON_ACCENT`,
`PANEL`, `PANEL_RAISED`, `FG_2`, `FG_3`, `BORDER_1`, `space::{XS,S,M,L}`,
`radius::R2`, `text::SMALL`). No new dependency (`cargo tree -p ui` unchanged).

### Accessibility notes

- **Keyboard**: the band is display-only for R3-3a (no click-nav — see the
  view-time decision below), so it introduces no new focus stops; every stage is
  still reachable via the sidebar (Calibrate now included) + the existing
  `OpenTuneEditor` drill-down.
- **Colour is never the only signal**: the active segment carries a leading `●`
  marker + the chevron flow, so "you are here" is legible without hue (satisfies
  the contrast/second-signal minimum). Text-on-accent uses `FG_ON_ACCENT`
  (theme-verified ≥ 4.5:1).
- **Both themes**: colours resolve via `ModeColor::current(mode)` — the band
  renders correctly under `--theme dark` and `--theme light` (all tokens are
  dual-mode).

### View-time decision at the pixel layer (ADR-0083 sanctioned fallback)

The DATA/ANALYZE-share-`Leaderboard` highlight (D2) rendered **correctly and
unambiguously** at the render review (see `/tmp/stage_stepper_data.png` vs
`/tmp/stage_stepper_analyze.png` — the highlight moves DATA→ANALYZE purely on
the substate flip, same screen). So the sanctioned fallback (merged
"DATA·ANALYZE" segment / default-to-ANALYZE) was **NOT needed** — the substate
discriminator is honest and legible as-is.

Two intentional scope calls, both `view`-time (no ADR amendment):
- **Display-only stepper** (T7 optional). The band does NOT click-navigate for
  R3-3a — the sidebar (now with Calibrate) already routes every stage, and a
  clickable band risks a confusing "click DATA/ANALYZE both go to Leaderboard"
  ambiguity for zero present benefit. Deferred cleanly; the widget takes only
  `Option<SpineStage>` so adding click-nav later is additive.
- **`Loading`/`Error` on Leaderboard → DATA** (not a 5th state). The operator is
  still on the input surface with no ranked table to analyse, so DATA is the
  honest highlight.

## Changelog

- 2026-07-09 (architect): design pass complete (`status: arch-done`). Grounded R3-3a
  in the real `crates/ui/` seams (Screen enum, sidebar constants + flatten-invariant,
  shell composition, DATA/ANALYZE-share-Leaderboard IA finding, render-harness
  precedent). Decided the stepper is an orientation band (not a router) and Tune is
  promoted to a sidebar "Calibrate" stage; deferred the `AdvisorStage` carrier again
  (D7). Recorded the design in ADR-0083. Handoff to ui-designer.
- 2026-07-09 (ui-designer): built + render-verified (`status: dev-done`). Shipped the
  `stage_stepper` widget (`SpineStage` + pure `stage_for` + `view`), wired it at the
  top of the shell `centre` Column, promoted `Screen::Tune` to a sidebar "Calibrate"
  entry (both `SIDEBAR_ENTRIES_PHASE_A` + `SIDEBAR_GROUPS_PHASE_C` Work group in
  lock-step — flatten-invariant green), added 5 `strings` constants + a gallery cell.
  Render proof `crates/ui/tests/stage_stepper_render.rs` (macOS-gated): CALIBRATE
  positive + ForwardPlan→SUGGEST negative control + the DATA/ANALYZE substate
  discriminator + off-journey elision — all four PNGs read + eyeballed. 9 `stage_for`
  unit tests cover every D2 row incl. `None`. Gates green: `cargo build -p ui`,
  `cargo test -p ui --lib` (597), `cargo test -p ui --test stage_stepper_render` (4),
  `cargo clippy -p ui [--tests] -- -D warnings`, `cargo fmt --check`, consistency
  (no inline strings/hex), `cargo tree -p ui` unchanged, `verify_anchors.sh` 119/119,
  `spec_lint.py` PASS. `AdvisorStage` carrier NOT built (D3/D7 deferred). Handoff to
  tester.
