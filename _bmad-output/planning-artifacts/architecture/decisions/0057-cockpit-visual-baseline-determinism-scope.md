---
adr: 0057
title: Cockpit visual-baseline determinism scope is the macOS canonical box
status: accepted
date: 2026-06-15
supersedes: none
superseded-by: none
related:
  - "ADR-0043 simulated-latency-and-slippage (§ D5 determinism scope = Apple-Silicon canonical box)"
  - "ADR-0051 monte-carlo-determinism (§ D5 cross-platform byte-identity NOT contracted)"
  - "ADR-0023 iced is the single UI stack"
  - "ADR-0038 spec-anchor byte-immutable reports"
---

# ADR-0057: Cockpit visual-baseline determinism scope is the macOS canonical box

## Context

The `cockpit-cross-platform` feature (v0.1.0) makes the cockpit build and run on
Linux and Windows. That raises a render-determinism question the existing
determinism ADRs answered only for the *backtest* lane: the 56 PNG visual
baselines in `crates/ui/tests/visual-baselines/` were captured on macOS.

Fully traced (analyst M0, architect-verified): the cockpit sets **no** iced
default font (no `.font()` / `default_font` / `include_bytes!` in
`crates/ui/src/bin/`); the embedded `FiraSans-Regular.ttf` is gated on
`#[cfg(feature = "fira-sans")]` (`iced_graphics-0.14.0/src/text.rs:121-127`) and
`fira-sans` is **not** enabled (`iced = { default-features = false, features =
["tiny-skia","thread-pool","advanced","canvas"] }`). Therefore all body text
resolves through cosmic-text's `PlatformFallback` against the **per-OS system
font database** (`cosmic-text-0.15.0/src/font/system.rs:400`
`db.load_system_fonts()`). Consequence: glyph shaping + rasterization differ
across OSes, so the 56 baselines will not match pixel-for-pixel on Linux/Windows.

ADR-0043 § D5 and ADR-0051 § D5 already declare the determinism scope as the
"Apple-Silicon canonical box" and state cross-platform byte-identity is
explicitly NOT contracted — but they read, by their wording, as scoping the
**Monte-Carlo anchors and backtest report bodies**. They do not name the **UI
render snapshot gate** as a third byte-comparable artifact class. A future
contributor running `cargo test -p ui` on Linux, or a naive CI re-org, could
re-baseline the visual tests on Linux and silently fork the gate. This ADR closes
that gap by stating the scope explicitly for the UI snapshot gate and locking the
enforcement mechanism in source.

## Decision

**D1 — The cockpit visual-baseline determinism scope is the macOS canonical box.**
The 56 PNG baselines in `crates/ui/tests/visual-baselines/` are **macOS-canonical**.
Linux and Windows render body text via cosmic-text `PlatformFallback` against the
per-OS system font database and are therefore **NOT byte-gated** against those
baselines. This extends the ADR-0043 § D5 / ADR-0051 § D5 determinism scope to a
third artifact class — the UI render snapshot gate — verbatim in spirit.

**D2 — Enforcement lives in source, not CI YAML.** The four snapshot integration-
test files — `crates/ui/tests/{visual_snapshots.rs, render_snapshots.rs,
panel_snapshots.rs, gallery_snapshots.rs}` — each carry a **file-level inner
attribute `#![cfg(target_os = "macos")]`** (alongside their existing `#![allow(…)]`
inner attributes). On Linux/Windows the entire file and its `#[path =
"fixtures/mod.rs"] mod fixtures;` private copy compile to nothing: the tests are
**skipped, never re-baselined**. The macOS leg compiles them in and runs all 56
byte-identical. The source gate is the single source of truth — CI legs need **no**
test-name filter to exclude the visual tests off-macOS, which removes the
"CI-filter drifts out of sync with the test set" failure mode.

**D3 — Cross-platform visual regression is a v0.2 follow-on, not silent breakage.**
A Linux (or Windows) visual-regression capability requires its **own** canonical
box and its **own** baseline set, and is gated on hypothesis H1 of
`cockpit-cross-platform` (enable `fira-sans` + set a pinned `default_font` so
renders become font-deterministic across OSes). Any such feature MUST supersede or
amend this ADR — it may not re-baseline the macOS set onto another OS.

## Alternatives considered

- **Fold the scope into ADR-0051's § Changelog** — rejected: ADR-0051's Changelog
  is the Monte-Carlo robustness lane's running ledger (D6.5-D6.10 are all
  MC-strategy amendments); a UI-render-determinism scope mis-files there. A
  standalone ADR is the correct, citable home.
- **CI-job-filter only (no source gate)** — rejected: the "these tests are
  macOS-only" truth would live only in YAML, so a local `cargo test -p ui` on a
  contributor's Linux box goes red confusingly, and the filter can drift out of
  sync with the test set. Source-gating puts the contract where the failure
  happens.
- **Per-`#[test]` `#[cfg(target_os = "macos")]` gating** — rejected: each file
  holds dozens of `<fixture>__<slot>` test fns; gating each attribute is exactly
  the bookkeeping-drift surface the project keeps paying for. One file-level inner
  attribute per file is the single-point gate.
- **Enable `fira-sans` + pin a default font now to make one baseline set serve all
  OSes (H1)** — rejected for v0.1: re-capturing on macOS with `fira-sans` enabled
  would change the 56 baselines, a *baseline migration* that violates the v0.1
  byte-identical contract (R-NR.1). Correctly deferred to v0.2.

## Consequences

- **What breaks if violated.** Re-baselining the visual tests on Linux/Windows
  forks the gate: the macOS canonical baselines and the Linux/Windows captures
  diverge by font, and "green CI" would no longer mean "matches the canonical
  render." The source gate makes this structurally hard — you would have to delete
  the `#![cfg(target_os = "macos")]` line to even compile the tests off-macOS.
- **What enforces it.** The `#![cfg(target_os = "macos")]` inner attribute in the
  four named files (mechanically: the tests do not exist as compilation units off
  macOS). The macOS CI leg (`.github/workflows/ci.yml`, ADR feature D8) runs the
  full visual suite byte-identical as the canonical gate; the
  `cockpit-cross-platform` tester's 4-cell verdict tree treats any macOS baseline
  byte-change as REGRESSION (R-NR.1).
- **Anchor safety.** This ADR touches no `crates/backtest`/`exec`/`cost` bytes; the
  119 backtest anchors are unaffected (R-NR.2). It modifies no byte-immutable
  report file (ADR-0038 § D6 safe).

## Changelog
- 2026-06-15 (architect): initial accept. Authored alongside `cockpit-cross-platform`
  M-T1 (feature.md § Design D6). Elevates the ADR-0043 § D5 / ADR-0051 § D5
  determinism scope to the UI render snapshot gate; locks the source-gating
  mechanism (Q2=(a)).
