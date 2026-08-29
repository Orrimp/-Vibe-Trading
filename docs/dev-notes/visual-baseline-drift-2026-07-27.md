# Visual-baseline drift — diagnosis (2026-07-27)

## Symptom

At clean HEAD on the canonical Apple-Silicon box, the byte-exact pixel gates
are broadly red: **48** failures in `visual_snapshots` + **6** in the trail
harness (all three viewports of every text-bearing screen state) + **8**
advisor-side render tests (`leaderboard_narration_render` 3,
`leaderboard_populated_render` 2, `leaderboard_scorecard_render` 2,
`leaderboard_short_arms_render` 1) — **62 total**. The 2026-07-25 Phase-5b
commit flagged "8 pre-existing render_snapshots baseline drifts (traced,
unrelated)"; the set has since widened to effectively *every glyph-bearing
baseline*.

## Evidence

1. **Not caused by recent changes:** stash A/B on 2026-07-27 (story-1-10
   review tree vs clean HEAD) produced **byte-identical failure sets** —
   same names, same counts (`3 passed; 48 failed` both sides).
2. **Glyph-localized:** the diff artifact
   `target/visual-diff/assistant_slot__open_stub__floor.png` shows changed
   pixels ONLY on text runs (sidebar nav labels + a top-right text element);
   every non-text region is pixel-identical. Same pattern class across the
   failing set.
3. **Structural checks survive:** harnesses asserting structure rather than
   glyph bytes (`lab_binance_render` chip/band/polyline checks,
   `live_equity_render`, contrast asserts) all pass — the screens draw
   correctly; only glyph rasterization moved.

## Cause (assessed)

The cockpit sets **no embedded default font**: body text resolves via
cosmic-text `PlatformFallback` against the per-OS system font DB (documented
in the cockpit-cross-platform survey / trace row). Baselines captured before
a macOS/toolchain update no longer byte-match the current rasterizer output.
This is exactly the instability ADR-0051 § D5 / ADR-0043 scoped around when
they pinned determinism to "the canonical box" — the pin assumed the box's
font stack was stationary. It isn't across OS updates (current: Darwin
25.5.0; capture-era version unrecorded — a gap this note now closes going
forward: **record `sw_vers` output whenever baselines are captured**).

## Remediation (ordered — durable first)

1. **Kill the class before re-baselining:** enable the embedded-font path
   (`fira-sans` feature exists but is NOT in defaults; wiring it as the
   cockpit default font makes glyph rasterization repo-deterministic,
   independent of the OS font DB). This is H1 of the cross-platform story
   (6-9) and would also unlock cross-OS baselines later.
2. **Then re-baseline ONCE** under the embedded font, with per-screen human
   eyeball approval per `docs/dev-notes/iced-ui-render-verification.md` — the
   task chip "Re-audit the 54 drifted macOS visual baselines" covers this.
   Re-baselining *without* step 1 just re-arms the same bomb for the next OS
   update.
3. Record the environment (`sw_vers`, toolchain) in the baseline commit
   message each re-baseline.

## Interim honesty rule

Until remediation lands, a red `visual_snapshots` run on this box is
**expected noise for glyph-bearing screens** — but that makes the gate blind:
any NEW visual regression must be caught by the structural harnesses + the
`target/visual-diff/*-actual.png` eyeball trail, and the stash-A/B technique
above is the way to attribute any new failure to a change vs the drift.

---

## Correction (2026-08-29) — remediation step 1 does not work on native

The diagnosis above is **confirmed**: re-measured at HEAD, `visual_snapshots`
is `3 passed; 48 failed`, the diff artifact is glyph-localized (changed pixels
only on sidebar nav labels and one top-right text run; every non-text region
pixel-identical), and structural harnesses still pass. Nothing here disputes
the cause.

**But step 1 as written cannot fix it, and would have cost a whole cycle.**

It says: *"enable the embedded-font path (`fira-sans` feature exists but is NOT
in defaults …)"*. Two things are wrong with that.

1. **The feature is already enabled.** `cargo tree -e features` resolves
   `iced_renderer feature "fira-sans"` -> `iced_graphics feature "fira-sans"`
   at HEAD, without `crates/ui/Cargo.toml` listing it.
2. **Enabling it changes nothing on native.** A/B measured 2026-08-29 —
   `assistant_slot__open_stub__typical` rendered with the feature explicitly on
   and explicitly off produced **byte-identical PNGs** (`29eea22ef8432708` both).

**Why**, at source:

- `iced_graphics/src/text.rs:125` — the feature LOADS `FiraSans-Regular.ttf`
  into the global cosmic-text `FontSystem`. It makes the font *available*.
- `iced_graphics/src/settings.rs:42` — the switch of `default_font` to
  `Font::with_name("Fira Sans")` is gated on
  **`cfg!(all(target_arch = "wasm32", feature = "fira-sans"))`**. WASM only.
- `iced_core/src/font.rs:19` — on native, `Font::DEFAULT` therefore stays
  `Family::SansSerif`, a GENERIC family that cosmic-text resolves through
  `PlatformFallback` against the OS font DB. Which is the bomb.

So the font is in the database and nothing asks for it by name.

### What the real fix has to do

Something must select the font explicitly. Two halves, and the second is the
hard one:

- **The app**: `iced::application(..).default_font(Font::with_name("Fira Sans"))`
  — the builder API exists (`iced-0.14.0/src/application.rs:234`). Easy.
- **The pixel gate**: no hook exists. The harness renders through
  `iced_test::screenshot(&program, &theme, viewport, scale, duration)`
  (`viewport_matrix.rs:133`), which takes **no font argument** and constructs
  `Emulator::new` with none either. Setting the font on the application builder
  does NOT reach the tests — so the gate would keep measuring the OS font while
  the shipped app used Fira Sans, which is *worse* than today: the two would
  disagree by construction.

Options, none free, and this is an operator call:
  (a) give the widgets an explicit font at the `ui` layer, so both paths agree;
  (b) vendor/patch `iced_test` to accept a default font — the repo already
      vendors `iced_tiny_skia`, so the pattern exists (and is operator-locked);
  (c) upstream a font parameter to `iced_test`.

### Consequence for the re-baseline

**Do not re-baseline yet.** The ordering rule in step 2 is right, but its
precondition is not met by simply flipping the feature — and re-baselining now
would re-arm exactly the bomb this note was written to defuse, while *appearing*
to have followed the documented remediation.

`crates/ui/Cargo.toml` now lists `fira-sans` explicitly with this correction
recorded at the dependency, because the next reader will otherwise repeat the
same experiment.
