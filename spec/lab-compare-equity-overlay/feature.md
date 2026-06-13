---
slug: lab-compare-equity-overlay
status: arch-done
owner: orchestrator
updated: 2026-06-13
version: 0.1.0
trace: REQ-LAB-COMPARE-OVERLAY-001
---

# Compare screen — two-run equity overlay

## Changelog

- 2026-06-13 (orchestrator): scoped as the operator-greenlit follow-on to the
  shipped `lab-run-save-compare` (the deferred R5 "equity overlay" half). The
  design is settled — this is a focused UI-only build, so it skips a fresh
  analyst/architect cycle and goes straight to ui-designer → tester. Pipeline
  origin: `lab-run-save-compare` feature.md changelog + ADR-0055 § R5.

## Why

`lab-run-save-compare` shipped run → save → compare-**KPIs** + real per-run
curve repaint. The one deferred piece of its asked scope is the **two-run
equity OVERLAY**: two selected Lab runs' equity curves drawn on ONE chart so the
operator can visually compare strategies/params on real data. The overlay
**widget** is already render-proven
(`crates/ui/tests/live_equity_render.rs::compare_two_run_overlay_renders_both_series`);
what's missing is wiring it into the live Compare screen.

## What was blocking it (now unblocked)

The Compare matrix's `CachedCell` stored only `equity_curve_tail: Vec<f64>` — a
bare tail with **no timestamps** — so it could not feed the timestamped
`chart::view` overlay. As of `lab-run-save-compare`, the loader
(`equity_loader::load_companion_equity_csv`) provides a full, **timestamped**,
PerBar `LabEquitySeries` from the companion equity CSV. So the data now exists;
this feature threads it to the overlay.

## Requirements

- **R1 — CachedCell carries a timestamped series.** Add a timestamped
  equity-series field to `CachedCell` (`crates/ui/src/compare/cache.rs`),
  populated from the CSV-backed `LabEquitySeries` the loader returns (PerBar
  fidelity; fall back gracefully when a cell is start-end-only).
- **R2 — Two-run overlay in the Compare screen.** The operator selects **two**
  persisted runs and their equity curves render overlaid on one chart, in two
  distinguishable accent colors (`ACCENT` + `ACCENT_2`, per the proven widget).
  The **selection UX is the ui-designer's call** within the Compare screen's
  existing interaction patterns (Q1 below) — keep it minimal and honest.
- **R3 — Render-layer verification (mandatory).** Per project law, prove the
  overlay at the rasterized layer: extend the render harness so the overlay
  hydrates from **two real companion-CSV-backed `lab-runs/` fixtures** and
  asserts BOTH series draw (the `compare_two_run_overlay_renders_both_series`
  pattern, now fed by real CSV cells, not a synthetic series).

## Acceptance criteria

- **AC1** — Two persisted runs overlay on one chart; both polylines rasterize in
  distinct accent colors (winner-take-all ACCENT / ACCENT_2 classifier).
- **AC2** — Render proof in `live_equity_render.rs` drives the overlay from two
  CSV-backed `lab-runs/` fixtures through the production path and asserts both
  series present (and a single-run contrast draws no ACCENT_2).
- **AC3** — `cargo test -p ui --lib` + `--features fixtures` + `--test
  live_equity_render` + `--test panel_snapshots` green; `cargo test -p ui
  --features live --test lab_run_engine` (H3) STILL passes (no loader
  regression). `bash scripts/verify_anchors.sh` → 119/119 (UI-only, no engine
  change — a tripwire). `cargo clippy -p ui --tests` zero new; fmt clean.

## Open questions

- **Q1 (ui-designer's call) — the two-run selection UX.** How does the operator
  pick which two runs to overlay? Candidates: (a) click two cells/columns in the
  existing Compare matrix [recommended — reuses the matrix the feature already
  has]; (b) two dropdowns above the chart. Pick the one that fits the existing
  Compare screen with the least new surface; document the choice.

## Out of scope / law

- No engine/backtest change (UI-only); no new anchors (AC3 tripwire).
- **NO live trading** — real-data backtesting comparison only.
- `Decimal`/`Money<Usdt>` never f64; no inline hex / strings via `strings.rs`
  (`spec/ui-design-principles.md`); no `.unwrap()`/`.expect()` in `crates/ui`
  lib code; render-layer verification is the gate (not unit tests alone).
- Baseline-equity-divergence gate **N/A** (read-only visualization of already-
  persisted backtest series — no strategy overlay or sizing decision).

## Effort

**S**, UI-only, single ui-designer track (design settled). The only genuine
decision is Q1 (selection UX). Render proof is the close-out gate.
