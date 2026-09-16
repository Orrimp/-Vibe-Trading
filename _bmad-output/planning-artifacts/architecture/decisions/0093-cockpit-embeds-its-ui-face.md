---
adr: 0093
title: The cockpit embeds its own UI face
status: accepted
date: 2026-09-16
supersedes: none
superseded-by: none
---

# ADR-0093: The cockpit embeds its own UI face

## Context

The cockpit named no font. `Font::DEFAULT` is the generic `SansSerif`, which cosmic-text
maps to "Open Sans" (`cosmic-text-0.15.0/src/font/system.rs:159`) — a face neither macOS
nor the CI runners ship — so every glyph resolved through the OS font database's
fallback chain. The 56 byte-exact visual baselines were therefore a property of the
machine that captured them:

- they went red across an OS update on the canonical box — 62 failures, glyph-localised,
  every non-text region pixel-identical
  (`docs/dev-notes/visual-baseline-drift-2026-07-27.md`);
- they can never be shared across OSes, which is why ADR-0057 § D2 scopes the pixel
  gates to macOS.

The 2026-07-27 remediation said "enable the embedded-font path (`fira-sans`)". Measured
2026-08-29, that does nothing on native: the feature loads Fira Sans into the font
database, but the `default_font` auto-switch is gated on
`cfg!(all(target_arch = "wasm32", feature = "fira-sans"))`
(`iced_graphics/src/settings.rs:42`), so toggling it produced byte-identical snapshots.
Something has to NAME a face.

Naming one is not sufficient either:

- `iced_test`'s `Emulator` — what every pixel gate renders through — builds its renderer
  from `Program::settings().default_font` and never loads `settings.fonts`
  (`iced_test-0.14.0/src/emulator.rs`), so `.font(bytes)` on the application does not
  reach the gates;
- a canvas `Text` carries a concrete `font` field and does not inherit the renderer
  default;
- cosmic-text falls back PER GLYPH, so a face missing `⚠` puts an OS font back into an
  otherwise deterministic frame for that one character. Fira Sans lacks eight of the
  glyphs this UI draws — `✓ ✗ ⚠ ★ ● ⓘ ▸ ▾` — five of which carry the non-colour
  pass / weak-evidence / best signals in ADR-0085's verbatim copy.

## Decision

**D1 — The `ui` crate embeds Inter Regular** (`crates/ui/assets/fonts/Inter-Regular.ttf`,
303 KiB, SIL OFL 1.1, licence alongside the file; copied from the `cosmic-text` crate
already in the build graph) and draws everything with it. Inter is the Lumen stack's own
UI face (`theme::font::FONT_SANS`) and covers every glyph the cockpit draws.

**D2 — The operator's no-bundled-fonts lock is lifted for this one face** (2026-09-16).
It read: "every kilobyte of font is a kilobyte not spent on faster bar rendering." The
binary gets *smaller*: iced's `fira-sans` feature (431 KiB) is dropped in the same
change, a net −128 KiB. `JetBrains Mono` stays unbundled; `FONT_MONO` remains a
documentation stack.

**D3 — One call loads and names it.** `theme::font::embedded()` loads the face into the
global font database exactly once per process and returns `theme::font::UI`; every
`iced::application(..)` passes it to `.default_font(..)`. All 15 canvas texts name
`theme::font::UI` directly, because they do not inherit the renderer default.

**D4 — The contract is enforced by tests, not by discipline.**
`crates/ui/tests/embedded_font_contract.rs` runs on every platform and asserts: the face
is loaded and named; the renderer default reaches the widgets (pixel-compared against
text that names the face, with a control proving the comparison is not vacuous); every
`iced::application` chain under `src/` and `tests/` selects it; every `CanvasText`
literal names it; and no string literal under `src/` contains a glyph the face lacks.

The coverage check reads the face's character map straight from the embedded bytes with
`ttf-parser` (a dev-dependency, already in the lock via `fontdb`). The obvious source —
cosmic-text's `Font::unicode_codepoints()` — is a lazily-filled cache of what has been
shaped so far, not a cmap: asked cold it reported `—` and `→` as absent from a face that
carries both, which would have failed the gate on two glyphs the UI draws constantly.
The three glyphs Inter does not have were replaced in the Lab screen (`ⓘ` → `Note:`,
`▸`/`▾` → `▶`/`▼`).

**D5 — The byte-compare harness refuses an un-fonted render.** Every baseline entry
point asserts `Program::settings().default_font == theme::font::UI` BEFORE rendering, so
a baseline drawn through the OS font database cannot even be captured.

## Alternatives considered

- **Keep Fira Sans and replace the eight glyphs it lacks** — rejected: it amends
  ADR-0085's verbatim copy and weakens the non-colour signals (`✓` → `√`, `⚠` → `!`) to
  save 303 KiB that dropping `fira-sans` more than refunds.
- **Keep Fira Sans and let the symbols fall back to the OS** — rejected: it leaves the
  drift armed on exactly the screens that carry the honesty glyphs.
- **Vendor or patch `iced_test` to accept a font** — rejected: the repo already carries
  one operator-locked iced fork, and the font belongs to the ui layer, not the harness.
- **Remap the generic `sans-serif` family in the font database** — rejected: it fixes
  the default globally but leaves canvas text and any explicit `Font` untouched, and it
  is invisible at the call sites.

## Consequences

- **Every glyph in the cockpit changes shape once.** The 56 byte-exact baselines are
  re-captured in a single pass with per-screen operator approval, per the drift note's
  ordering rule, and that commit message records `sw_vers` + toolchain.
- Cross-OS baselines become *possible* for the first time — the face is no longer a
  property of the runner. ADR-0057 § D2 still scopes the gates to macOS; widening it is
  a separate, measured decision, since identical tiny-skia rasterisation across
  architectures is unproven here.
- `iced_test`'s `Simulator` default ("Fira Sans" when a program leaves `default_font` at
  `Font::DEFAULT`) no longer resolves. No test in this repo uses `Simulator`.
- A new glyph that the face lacks now fails a test instead of drifting silently, and the
  failure names the file, line and codepoint.
- **Measured limitation — text the cockpit LOADS is outside this contract.** The Reports
  screen and the `viewer` binary render report `body_markdown` read from `evidence/`, which
  a source scan cannot see. Across the 321 committed evidence documents, 11 glyphs Inter
  cannot draw appear in 21 files: overwhelmingly the block-element sparkline rows
  `reports::render::equity_curve` emits (`▁`..`█`, 480 occurrences in 4 files), plus
  scattered set/logic notation (`∧ ∈ ⊥ ≡ ≫ ∪ ≪`), one `ⓘ` and one `✅`. Those still fall
  back per glyph to an OS font on that screen. No byte-exact baseline is a Reports screen
  and the ui's own report fixtures are free of them, so no gate depends on the fallback
  today — but a Reports baseline would drift until the sparkline is DRAWN rather than
  typed. Recorded as bug-log #100.
