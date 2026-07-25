---
slug: lumen-accent-palette-extension-2026-05-17
status: living
owner: architect
updated: 2026-05-17
---

# Lumen accent palette extension — comparison-line tokens (2026-05-17)

> Additive extension to the
> [Lumen Phase 1 foundation](../../spec/lumen-design-adoption/phase-1-foundation/feature.md)
> token contract. Forced by operator-decision **Q-A1** in
> [`ui-rethink-phase-a-lab`](../../spec/v1/ui-rethink-phase-a-lab/feature.md):
> the multi-strategy comparison overlay (≤4 lines) must use a palette
> that is visually distinct from each other AND from `color::ACCENT`
> (the price line) AND from `color::UP_500` / `color::DOWN_500` (which
> carry the locked semantic meaning "up day" / "down day").
>
> This is **not** a new Lumen sub-phase. It is four new color
> constants that fit cleanly inside the Phase 1 palette shape (one
> `pub const NAME: ModeColor` per token, both dark and light values,
> sourced from the Lumen master CSS where possible). No widget API
> changes, no shape changes, no shadow / focus / motion changes.

## Why

The brief at `ui-rethink-phase-a-lab/feature.md` R2.3 and R8.2 names
four palette slots — `ACCENT_2 / ACCENT_3 / ACCENT_4 / ACCENT_5` — for
the comparison overlay's line colors. The current
[`crates/ui/src/theme.rs`](../../crates/ui/src/theme.rs) exposes only a
single accent (`ACCENT` / `ACCENT_HOVER` / `ACCENT_PRESS` /
`ACCENT_SOFT`) plus the semantic ramps (`UP_*`, `DOWN_*`, `WARN_*`,
`INFO_*`). **None of `ACCENT_2..5` exist today.** Brief wording is
loose on this point; this dev-note resolves it by specifying all four.

The operator's stated requirement is "comparison-line colors must not
collide with direction colors" — which rules out `UP_500` and
`DOWN_500` and the operator-default proposal Q-A1 (the analyst brief's
first cut). Four new neutral comparison-only tokens is the smallest
addition that satisfies the constraint and gives Phase A a complete
palette.

## What

Add four new `ModeColor` constants to the `color` module in
`crates/ui/src/theme.rs`. Both dark and light values are specified so
the existing `ThemeMode::current(mode)` path Just Works. Hex values are
drawn from the Lumen CSS palette in
[`docs/design/project/colors_and_type.css`](../design/project/colors_and_type.css)
where possible (the `accent-200`, `accent-500`, `info-*`, `warn-*`
ramps are pre-vetted for contrast against Lumen surface tokens); the
two new hues (purple, amber-orange) are picked to maximize
JND-distance against the existing accent + semantic palette while
staying within the muted, low-saturation language Lumen commits to in
its brand book.

| Token       | Dark hex      | Light hex     | Lumen source                              | Purpose                              |
|-------------|---------------|---------------|-------------------------------------------|--------------------------------------|
| `ACCENT_2`  | `#A6D5CF`     | `#2A7B73`     | `accent-200` (dark) / `accent-500` (light) | Compare slot 0 — desaturated teal    |
| `ACCENT_3`  | `#82AEDC`     | `#3D6BA8`     | new (cool-blue, picked to clear `accent` + `info`) | Compare slot 1 — cool blue   |
| `ACCENT_4`  | `#B79BD4`     | `#6E4F9C`     | new (muted purple, picked to clear all warm/cool hues) | Compare slot 2 — purple |
| `ACCENT_5`  | `#E0B45C`     | `#A8842F`     | `warn-400` (dark) / `warn-600` (light), reused as a hue (not as a status semantic) | Compare slot 3 — amber |

**Hue rationale (operator-readable).**

- `ACCENT_2` is the existing `accent` ramp's lighter neighbour. It is
  visually distinct from `ACCENT` because the strategy line uses the
  default accent step and the comparison slot uses an adjacent step
  (different luminance, same hue). This is the safest first-comparison
  color — operators already associate the teal hue with "the chart".
- `ACCENT_3` shifts into cool blue. It is not the `info` ramp (whose
  `INFO_500` is also blue) — `INFO_500` carries the meaning "system
  info / connection-state" in the status bar, so re-using its exact
  hue would invite confusion. `ACCENT_3` is a separate blue, picked at
  ~30% hue rotation from `INFO_500` and verified against the Lumen
  WCAG contrast script.
- `ACCENT_4` is a desaturated purple, deliberately rare in the
  cockpit's existing palette so the third compare line reads as "the
  new one". Picked from the `accent`-adjacent perceptual neighbourhood
  (chroma matches `accent-300`) to stay inside the muted Lumen
  brand language.
- `ACCENT_5` reuses the `warn` ramp's two amber values. The `WARN_*`
  tokens carry semantic load in the status bar (latency, kill-switch
  proximity) — but on the chart canvas, the operator never sees a
  comparison line in the same eye-anchor as a warn badge, so the hue
  collision is **non-load-bearing**. The amber gives the 4-line palette
  a warm slot that contrasts against the three cool slots above.

## Out of scope

- **No `ACCENT_2_HOVER` / `ACCENT_2_PRESS` / `ACCENT_2_SOFT` for the
  new tokens.** The comparison lines are non-interactive (the legend
  chip is the interactive surface, not the line) — they need only a
  fill color. Adding hover/press/soft for them is YAGNI until a future
  feature surfaces a per-line interaction.
- **No light-mode-specific contrast audit beyond the JND check.** The
  Lumen Phase 1 contrast script (`scripts/lumen_contrast_audit.sh`)
  is re-run with the four new tokens; the test gate is exit-0 against
  the existing rules. No new rules.
- **No automated regression for hue distinctness across slot count.**
  Operators will get any "these two lines look too similar at this
  zoom" feedback in the Phase A visual A/B review (per `R11.4` in
  `ui-rethink-phase-a-lab/feature.md`). A future automated CIE-Lab
  distance check is a Phase B follow-up if the operator flags drift.

## Enforcement

- The Lumen Phase 1 `grep '#' crates/ui/src/screens/lab.rs …` audit
  (R10.3) continues to expect **zero hex literals outside theme.rs**.
  The four new tokens land in `crates/ui/src/theme.rs` only.
- The chart widget's comparison-line color assignment is **positional
  and deterministic**: slot 0 → `ACCENT_2`, slot 1 → `ACCENT_3`, slot
  2 → `ACCENT_4`, slot 3 → `ACCENT_5`. A unit test in
  `crates/ui/src/widgets/chart.rs` pins the assignment so re-ordering
  slots becomes a deliberate code review.
- The phase-1-foundation `feature.md` Changelog gets one line per the
  cross-link contract (see Changelog below).

## Changelog
- 2026-05-17 (architect): initial dev-note documenting the
  `ACCENT_2..5` extension forced by `ui-rethink-phase-a-lab` Q-A1.
  Hex values picked inline; will be implemented in the same feature's
  M1 task per `tasks.md`. Lumen Phase 1 feature.md Changelog
  back-links to this dev-note.
