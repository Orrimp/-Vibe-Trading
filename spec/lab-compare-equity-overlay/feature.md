---
slug: lab-compare-equity-overlay
status: shipped
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

## UI

_Authored by ui-designer 2026-06-13 (T1–T3 implementation)._

### Q1 resolution — the two-run selection UX

**Chosen: a per-cell overlay-select chip on the existing Compare matrix**
(candidate (a), reusing the matrix). Each populated cell gains a compact
`+` / `✓` chip in its top-right corner. The chip — not the KPI text — drives
overlay selection:

- The KPI text stays the **primary** click → `OpenLabFromCompare` (drill into
  Lab), **unchanged** from v0.1.0. The H5 round-trip
  (`state.rs::open_lab_from_compare_*`) is untouched and stays green.
- The `+` chip → `Message::CompareToggleOverlay(slot)` adds the run to a 2-slot
  selection ring (`CompareScreenState::overlay_selection`); clicking a selected
  cell's `✓` removes it; a third add rotates out the oldest (bounded at 2).
- The chip renders **only** when the cell carries a timestamped series
  (`equity_series_ts` non-empty) — a curve with no companion CSV cannot be
  overlaid, so no dead button is shown.

Why this over two dropdowns (candidate (b)): it adds the least new surface (one
message, one state field, one chip + one panel), reuses the `+`-add metaphor the
Lab compare chip already established, and keeps the matrix the single
interaction surface. The overlay chart renders **below** the matrix and is
always present (empty-state prompt → chart) so the operator never meets a blank
panel.

### Wireframe (Compare screen body)

```
┌─ Range [30d][90d][H1'24][H2'24]   KPI [Sharpe]… ────────────────┐
│ (universe-aggregate subtitle, when any multi-symbol cell)        │
│ ┌───────── matrix ──────────────────────────────────────────┐  │
│ │            XRP    BTC    ETH   …                            │  │
│ │ top10_…  [0.94 +][1.10 ✓]…   ← ✓ = ACCENT (slot 0)        │  │
│ │ btc_sma  [1.42 ✓]  —    —    ← ✓ = ACCENT_2 (slot 1)      │  │
│ └────────────────────────────────────────────────────────────┘  │
│ Equity overlay                                                   │
│ ● Run A: top10_momentum_h1 · XRP   ● Run B: btc_sma_cross · BTC  │
│ ┌──────────── chart::view (no bars; standalone equity) ───────┐ │
│ │   ╱╲   ACCENT (Run A)        self-scales x from series ts    │ │
│ │  ╱  ╲╱‾‾  ACCENT_2 (Run B)                                   │ │
│ └──────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────┘
```

### New screens / panels / widgets

- **No new screen.** The two-run equity overlay is a new **panel** appended to
  the existing `screens::compare::view` body (`overlay_panel`, `overlay_legend`,
  `resolve_slot_series`).
- **No new widget.** Reuses the render-proven `widgets::chart::view` overlay
  (the no-bars standalone equity path, `chart.rs:480`) — `equity` = slot 0
  (`ACCENT`), `compare[0]` = slot 1 (`ACCENT_2`, via `accent_palette()[0]`).
- **New matrix affordance:** `matrix::overlay_select_chip` (the per-cell
  `+`/`✓` selection chip).

### New strings (`ui::strings`)

- `COMPARE_CELL_OVERLAY_ADD` (`+`), `COMPARE_CELL_OVERLAY_SELECTED` (`✓`),
  `COMPARE_CELL_OVERLAY_HINT` (chip tooltip).
- `COMPARE_OVERLAY_TITLE`, `COMPARE_OVERLAY_EMPTY` (empty-state prompt),
  `COMPARE_OVERLAY_LEGEND_PRIMARY` / `COMPARE_OVERLAY_LEGEND_COMPARE` (legend),
  `COMPARE_OVERLAY_NO_SERIES` (selected-but-no-companion-CSV note).

### New theme tokens

**Zero.** All colours come from existing tokens (`ACCENT`, `ACCENT_2`, `FG_*`,
`PANEL`, `PANEL_RAISED`, `BORDER_1`, `OVERLAY`, `WARN_500`); spacing/radius from
the existing scale. (Near-zero token additions is the design-system health
signal — met.)

### Accessibility notes

- **Keyboard:** the overlay chip is an `iced::Button` (focusable, Enter/Space
  activates) like every other matrix control; the date-range / KPI chips are
  unchanged.
- **Colour is never the only signal:** selection state pairs the **glyph**
  (`+` unselected vs. `✓` selected) with the colour, and the legend names each
  run's `strategy · pair` next to its colour swatch — so a colour-blind operator
  reads selection from the glyph + label, not hue alone.
- **Contrast:** `ACCENT` / `ACCENT_2` on `PANEL` are the verified Lumen overlay
  tokens (both dark + light variants defined); no new colour to re-verify.
- **No blank screens:** the overlay panel always renders — empty-state prompt
  ("Pick up to two runs with the + …") until a run is selected, a `WARN_500`
  note when a selected run has no saved curve, then the chart.
- **Both themes:** all colours are `ModeColor.current(mode)`; no hardcoded
  `ThemeMode::Dark` in screen/widget code (only the render-test harness locks
  Dark for snapshot determinism).

### Render proof (the gate)

`crates/ui/tests/live_equity_render.rs` PHASE 5 drives the **real Compare-screen
path** (`screens::compare::view` via `test_support::compare_screen_program`),
hydrated from **two real companion-CSV-backed `lab-runs/` fixtures** scanned by
the production `compare::cache::scan_report_roots`:

- `compare_screen_two_run_overlay_renders_both_series` — both `ACCENT` (Run A)
  **and** `ACCENT_2` (Run B) rasterize in the overlay chart band (winner-take-all
  classifier; observed ACCENT≈1351, ACCENT_2≈841, floor 120).
- `compare_screen_single_run_overlay_draws_no_accent2` — a single selected run
  draws `ACCENT` but **no** `ACCENT_2` (contrast self-proof).
- `diag_compare_screen_overlay_pixel_counts` — calibration diagnostic.

## Changelog

- 2026-06-13 (ui-designer): implemented T1–T3. T1 — `CachedCell.equity_series_ts`
  (timestamped per-bar series) hydrated from the report's companion equity CSV
  via `equity_loader::load_companion_equity_csv` (now `pub(crate)`), graceful
  empty fallback for start-end-only cells. T2 — `overlay_selection` 2-slot ring +
  `CompareToggleOverlay` message + per-cell `+`/`✓` chip (Q1); overlay panel
  wires the render-proven `chart::view` (`ACCENT`/`ACCENT_2`). T3 — render proof
  on the REAL Compare-screen path from two companion-CSV `lab-runs/` fixtures.
  All gates green; H3 still passes; anchors 119/119; no new theme tokens.
