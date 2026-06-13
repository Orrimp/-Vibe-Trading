---
slug: lab-compare-equity-overlay
status: shipped
owner: orchestrator
updated: 2026-06-13
version: 0.1.0
---

# Tasks — lab-compare-equity-overlay

Focused UI-only follow-on (design settled in [feature.md](feature.md) +
ADR-0055 § R5). Single ui-designer track → tester. No engine change.

## UI track (ui-designer)

- [x] **T1 — `CachedCell` timestamped series (R1).** Add a timestamped
  equity-series field to `CachedCell` (`crates/ui/src/compare/cache.rs`),
  populated from the CSV-backed `LabEquitySeries`
  (`equity_loader::load_companion_equity_csv` → PerBar). Graceful fallback for
  start-end-only cells. _Gate: `cargo test -p ui --lib`._
- [x] **T2 — Two-run selection + overlay wiring (R2 / Q1).** Decide the
  selection UX (Q1 — recommend reusing the Compare matrix cells/columns), wire
  the render-proven overlay widget so two selected runs draw on one chart in
  ACCENT / ACCENT_2. _Gate: `cargo test -p ui --features fixtures`._
- [x] **T3 — Render proof (R3 / AC1-AC2 — THE gate).** Extend
  `crates/ui/tests/live_equity_render.rs`: the overlay hydrates from TWO real
  companion-CSV-backed `lab-runs/` fixtures through the production path; assert
  both series rasterize (ACCENT + ACCENT_2 each ≥ threshold) and a single-run
  contrast draws no ACCENT_2. _Gate: `cargo test -p ui --test
  live_equity_render`._

## Tester wave

- [x] **T4 — Close-out (AC3).** Full ui suites green (`--lib`, `--features
  fixtures`, `live_equity_render`, `panel_snapshots`) + H3 still passes
  (`--features live --test lab_run_engine` — no loader regression);
  `verify_anchors.sh` 119/119 (UI-only tripwire); `cargo clippy -p ui --tests`
  zero new; fmt clean. Report + `VERDICT → PASS`.

## `[[req]]` row for `spec/trace.toml` (orchestrator applies)

```toml
[[req]]
id          = "REQ-LAB-COMPARE-OVERLAY-001"
title       = "Compare screen two-run equity overlay — the deferred R5 half of lab-run-save-compare: two selected persisted Lab runs' equity curves render overlaid on one chart (ACCENT + ACCENT_2) from CSV-backed timestamped series. UI-only follow-on (CachedCell timestamped field + selection UX + wiring the render-proven overlay widget). Render-layer gate. No engine change, no new anchors (119/119 tripwire), no live trading."
feature     = "lab-compare-equity-overlay"
product     = "spec/product.md"
crates      = ["crates/ui"]
arch        = [
  "spec/lab-compare-equity-overlay/feature.md",
  "spec/lab-compare-equity-overlay/tasks.md",
  "spec/architecture/adr/0055-lab-run-persistence-topology-and-anchor-safety.md",
]
anchors     = []   # UI-only; no backtest scenario; 119/119 unchanged (AC3 tripwire).
state       = "arch-done"
```
