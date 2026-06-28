---
slug: cockpit-app-bundle
status: candidate
owner: pending-analyst
updated: 2026-05-11
version: 0.1.0
---

# cockpit-app-bundle — macOS `.app` packaging for dock + cmd-tab + Spotlight icons (candidate)

> **Stub feature file.** Candidate follow-up captured during the
> `chart-buy-sell-emphasis` M6.2 visual-verification pass after the
> operator confirmed that `iced::window::Settings::icon` does **not**
> drive the macOS dock icon (only the iced-internal title-bar glyph).
> Not promoted; no analyst spawn. Holds the file-system slot and
> points back to the M6.2 entry so the next reader has the breadcrumb.

## Status

- **Not active.** Sits as a candidate awaiting operator promotion.
- **Awaits analyst spawn.** Analyst takes ownership when the operator
  decides macOS-native packaging is worth the build-system + CI
  complexity.
- **Surfaced by:**
  [`spec/chart-buy-sell-emphasis/tasks.md`](../v1/chart-buy-sell-emphasis/tasks.md)
  §  M6.2 / T2031 — operator visual-verification pass on commit
  `9bb5786` confirmed the dock + cmd-tab + Spotlight + Finder icons
  still show the generic iced/Cargo placeholder despite the title-bar
  icon plumbing being correct (`crates/ui/src/window_icon.rs`'s
  `lumen_window_icon` returns `Some(_)` and every bin's
  `iced::window::Settings::icon` is populated; `window_icon_set_on_all_bins`
  test stays green).
- **Blocks on:** nothing technical; pure operator-priority call. The
  fix is well-trodden ground (`cargo bundle` exists, every Tauri/egui
  shop solves this) — the question is whether shipping a `.dmg` /
  `.app` matters more than ad-hoc `cargo run` for the operator's
  current workflow.

## Why this is a separate feature

`iced::window::Settings::icon` plumbs into iced's own window-chrome
icon path — title-bar glyph on Windows + Linux compositors, mostly
ignored on macOS (some macOS configurations show it in the title bar,
many don't). The **macOS dock**, **cmd-tab application switcher**,
**Spotlight result icon**, and **Finder file icon** all come from an
`.app` bundle's `Info.plist` (`CFBundleIconFile` → `.icns` resource).
A bare `cargo run --bin cockpit` produces a Mach-O executable, not an
`.app` bundle — there is **no `Info.plist` for macOS to read**, so
the OS falls back to the generic placeholder.

This is a packaging concern, not a UI-widget concern. It does not
touch `crates/ui/src/`; it touches the build pipeline + a new
`crates/ui/macos/` (or per-bin) bundle layout. Keeping it in a
separate feature folder prevents conflating "iced window-icon
plumbing" (shipped, correct, in `window_icon.rs`) with "macOS-native
app packaging" (this candidate).

## Open questions for the analyst at promotion

1. **Tool choice.** [`cargo-bundle`](https://crates.io/crates/cargo-bundle)
   vs. hand-written `Info.plist` + `iconutil` + `sips` script invoked
   from `build.rs` or a `xtask`. `cargo-bundle` is mature (Tauri uses
   a fork) but adds a build-time crate dep + cargo subcommand. A
   hand-rolled script is zero-dep but rebuilds the bundling logic
   that's already a solved problem.
2. **Per-bin or one-bin.** There are three bins —  `cockpit`,
   `cockpit_live`, `viewer`. Operator's working binary is
   `cockpit_live` (the live wired path; the standalone `cockpit` is
   fixtures-only). Does the operator want a single bundled
   `Lumen.app` for `cockpit_live` only, or three separate `.app`s
   (lumen-cockpit, lumen-cockpit-live, lumen-viewer)?
3. **Icon rasterisation pipeline.** The brand mark is at
   [`spec/design/project/assets/brand/lumen-mark.svg`](../design/project/assets/brand/lumen-mark.svg).
   `.icns` needs 6+ sizes (16, 32, 64, 128, 256, 512, 1024 px).
   Pre-rasterise once into the repo (the way `assets/lumen-mark-64x64.rgba`
   handles the iced path), or rasterise on every build (slower but
   single source-of-truth)?
4. **Signing + notarisation.** Operator-private builds don't need
   notarisation; if the bundle is ever shared, Apple's gatekeeper
   demands a Developer ID signature + notarisation pass. Out-of-scope
   unless the operator promotes a distribution path.
5. **CI integration.** The current GitHub-Actions workflow builds +
   tests only. Bundling adds a macOS-only step that has to be gated
   behind `runs-on: macos-latest`. Worth it, or is local-only
   bundling enough?
6. **Linux + Windows surface.** Linux compositors (Wayland +
   GNOME/KDE) typically read app icons from `.desktop` files in
   `~/.local/share/applications/`; Windows reads from the embedded
   `.exe` resource section. Does this feature also produce those, or
   is it macOS-only? (Probably macOS-only — operator's primary
   workstation is macOS per the M6.2 feedback.)
7. **Determinism contract.** Anchors don't apply (no body-rendering
   change), but does the bundle build need to be reproducible
   bit-for-bit so CI never silently churns it? Probably yes — the
   `.icns` rasterisation is the only nondeterministic surface and
   pre-rasterising once into the repo (Q3 option A) sidesteps it.

## What changes when this is promoted

1. `status: candidate` flips to `status: in-progress`.
2. `owner: pending-analyst` flips to `owner: analyst`.
3. Analyst expands this stub into a full feature brief — answering
   the seven open questions above plus surfacing any new ones.
4. Architect resolves the bundle layout (per-bin or single bundle),
   the build pipeline (cargo-bundle or xtask), and the CI gate (skip
   on Linux runners or matrix-on-macOS).
5. Developer ships the bundle plumbing; ui-designer reviews the
   rasterised icon glyphs for parity with `spec/design/`.
6. Tester re-runs every cockpit V-gate against the bundled binary —
   the bundled path MUST keep all anchors green (the bundle is a
   wrapper, not a code change) — plus a new V-gate that confirms
   the bundled `.app`'s `Info.plist` carries the right
   `CFBundleIdentifier`, `CFBundleIconFile`, and that the `.icns` is
   non-empty.

## Cross-references

- [`crates/ui/src/window_icon.rs`](../../crates/ui/src/window_icon.rs)
  — module-level note (T2031) documents the limitation in-source so
  the next reader hits the breadcrumb without reading this stub.
- [`spec/chart-buy-sell-emphasis/tasks.md`](../v1/chart-buy-sell-emphasis/tasks.md)
  § M6.2 / T2031 — the operator-feedback breadcrumb that surfaced
  this candidate.
- [`spec/design/project/assets/brand/lumen-mark.svg`](../design/project/assets/brand/lumen-mark.svg)
  — the brand mark to rasterise into `.icns`.
- [`crates/ui/assets/lumen-mark-64x64.rgba`](../../crates/ui/assets/lumen-mark-64x64.rgba)
  — the pre-rasterised 64×64 RGBA blob already shipped for the
  iced-side icon path (precedent for "pre-rasterise once, ship in
  repo").
- [`spec/v25-dl-forecast-overlay/feature.md`](../v1/v25-dl-forecast-overlay/feature.md)
  — sibling `status: draft` stub (v2.5 DL forecaster brief).

## Changelog

- 2026-05-11 (ui-designer, M6.2 / T2031): stub created during the
  chart-buy-sell-emphasis M6.2 follow-up pass after the operator's
  visual-verification pass on commit `9bb5786` confirmed the macOS
  dock-icon gap that the iced-level plumbing cannot fix. The
  `window_icon.rs` module-level note (T2031 acceptance grep) points
  here. No analyst spawn; promotion gated on operator priority call.
