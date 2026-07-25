---
slug: lab-compare-equity-overlay
mode: release
status: draft
audience: human-operator
updated: 2026-06-13
generated: 2026-06-13T09:23:00Z
---

# Compare screen — two-run equity overlay — release

## TL;DR

You can now pick two saved Lab backtests and see both equity curves drawn on one chart, in two distinct colours, so you compare strategies on real data at a glance — this closes out the original "compare = KPIs + equity overlay" ask.

## What changed

- **Pick two runs, see two curves overlaid.** On the **Compare** screen, every populated cell now has a small `+` chip in its top-right corner. Click the `+` on two cells and both runs' equity curves draw together on one chart below the matrix — Run A in the primary accent colour, Run B in the second accent. Click a selected cell's `✓` to drop it; selecting a third rotates out the oldest (capped at two).
- **The chip only shows where a curve actually exists.** The `+` appears only on cells that carry a saved equity series (a real persisted run with a companion CSV). Cells without one show no chip — no dead buttons. In fixtures/demo mode, cells generally have no series, so you exercise this with real saved runs.
- **Nothing else moved.** The KPI number in each cell is still the primary click and still drills into Lab, exactly as before. This is a UI-only, additive feature: no engine, backtest, or strategy code changed, and no new regression anchors were added.

## Why

`lab-run-save-compare` (the previous ship) delivered run → save → compare-**KPIs** plus per-run real equity curves, but deferred one half of its asked scope: the **two-run equity OVERLAY** — two runs on one chart for a real visual side-by-side. The overlay drawing widget was already render-proven; the missing piece was a data plumbing + selection-UX wiring job. The block was that the Compare cache stored only a bare equity tail with no timestamps, so it could not feed the timestamped overlay chart. `lab-run-save-compare` already added a timestamped, CSV-backed series loader; this feature threads that series into the cache cell and into the overlay. See [feature.md](../feature.md) § Why and § What was blocking it.

## What you can do now

| Action | Command |
|--------|---------|
| Open the cockpit (real persisted runs, Live build) | `cargo run -p ui --release --bin cockpit_live --features live` |
| Open the cockpit (fixtures build — matrix renders; overlay chips need real runs) | `cargo run -p ui --release --bin cockpit` |
| Re-run the render gate that proves both curves rasterize | `cargo test -p ui --test live_equity_render` |
| Confirm no engine drift (anchor tripwire) | `bash scripts/verify_anchors.sh` |

In the cockpit: go to the **Compare** screen, click the `+` chip in the top-right of two populated cells, and read the **"Equity overlay"** panel below the matrix — both curves draw with a colour legend naming each run's `strategy · pair`.

## Live demo

This is a headless build environment, so the ground-truth demo is the render-layer gate: it drives the **real** `screens::compare::view` path, hydrates two runs from real companion-CSV-backed `lab-runs/` fixtures through the production scan path, rasterizes the screen, and counts coloured pixels in the overlay chart band. This is the actual proof both curves draw (not a unit test on the math).

```
$ cargo test -p ui --test live_equity_render -- --nocapture --test-threads=1

running 15 tests
test compare_screen_single_run_overlay_draws_no_accent2 ... ok
test compare_screen_two_run_overlay_renders_both_series ... ok
test compare_two_run_overlay_renders_both_series ... ok
test diag_compare_screen_overlay_pixel_counts ... [diag] cell A series len=6 ; cell B series len=6
[diag] Compare-screen overlay chart band (y≥470): ACCENT=1351 ACCENT_2=841
ok
test flat_and_single_point_curves_render_without_panic ... ok
test harness_catches_dropped_points_empty_curve ... ok
test healthy_curve_draws_far_more_than_broken ... ok
test hydrated_boot_curve_actually_renders ... ok
test lab_curve_hydrated_from_lab_runs_report_renders ... ok
test live_append_after_hydrate_still_renders_and_grows ... ok
test live_equity_curve_actually_renders ... ok
test single_run_overlay_draws_no_accent2 ... ok
test y_variation_gate_moving_passes_flat_fails ... ok

test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 1.57s
```

Notice the `diag_compare_screen_overlay_pixel_counts` line: two real runs each yield a 6-point equity series, and in the chart band the primary curve drew **ACCENT = 1351** pixels and the second curve drew **ACCENT_2 = 841** pixels — both far above the 120-pixel floor, so both curves provably render in distinct colours. The `compare_screen_single_run_overlay_draws_no_accent2` test is the contrast self-proof: one selected run draws ACCENT but no ACCENT_2. (Full stdout: [artifacts/lab-compare-equity-overlay-2026-06-13/live_equity_render-stdout.txt](artifacts/lab-compare-equity-overlay-2026-06-13/live_equity_render-stdout.txt).)

## Screenshots

_No screenshot is committed for this feature, and the build environment is headless so a faithful one cannot be auto-captured. Manual-capture recipe below — the render-gate pixel counts above are the binding visual proof; the screenshot is a nice-to-have for the deck, not a gate._

**Manual capture instruction (optional — for your own eyeball confirmation):**

- **Command:** `cargo run -p ui --release --bin cockpit_live --features live`
- **Steps:** (1) Run two *different* Lab backtests so two runs persist to `lab-runs/` — pick a different strategy / pair / date-range each time (e.g. `top10_momentum` on XRP over H1'24, then `btc_sma_cross` on BTC over 90d). (2) Open the **Compare** screen. (3) Click the `+` chip in the top-right corner of one populated cell — it turns to a `✓` in the primary accent. (4) Click the `+` on a second cell — `✓` in the second accent. (5) Read the **"Equity overlay"** panel below the matrix: both curves drawn, legend naming `Run A: <strategy> · <pair>` and `Run B: <strategy> · <pair>`.
- **Timing:** each backtest is a few seconds to a minute depending on range; the overlay repaints instantly on chip click.
- **Expected result:** two distinctly-coloured equity curves on one chart, a two-entry colour legend, no blank panel at any point.
- **Failure diagnosis:** if a cell shows **no** `+` chip, that run has no saved equity series (older run, or fixtures cell) — run a fresh backtest. If a selected run's curve is missing, a `WARN`-coloured "no saved curve" note appears under the title explaining why.
- **Cleanup:** none — the overlay is read-only; deselect with `✓` or just close the cockpit. No files written.

To persist a screenshot into the deck later, save it under `spec/lab-compare-equity-overlay/reports/screenshots/` and reference it here.

## Verification

| AC | Description | Status | Evidence |
|----|-------------|--------|----------|
| AC1 | Two persisted runs overlay on one chart; both polylines rasterize in distinct accent colours | VERIFIED | `live_equity_render.rs::compare_screen_two_run_overlay_renders_both_series` — chart-band pixels **ACCENT=1351, ACCENT_2=841** (floor 120), re-run live above |
| AC2 | Render proof drives the overlay from two real CSV-backed `lab-runs/` fixtures through the production path; single-run contrast draws no ACCENT_2 | VERIFIED | `compare_screen_two_run_overlay_renders_both_series` + `compare_screen_single_run_overlay_draws_no_accent2`, both hydrate via production `scan_report_roots`; both cell series len=6 |
| AC3 | Full UI suites green; H3 still passes (no loader regression); 119/119 anchors; no new clippy in lib; fmt clean | VERIFIED | Suites 626/626 (re-run); H3 1/1; `verify_anchors.sh` 119/119 (re-run live below); fmt clean; zero new lib clippy |
| — | Baseline-equity-divergence gate | N/A | Read-only visualization of already-persisted backtest series — no strategy overlay or sizing decision (per feature.md § Out of scope / law) |

Tester verdict: **PASS** (after baseline rebase — see Open decisions). Report: [reports/test-2026-06-13-lab-compare-equity-overlay.md](../../../../evidence/v1/lab-compare-equity-overlay/reports/test-2026-06-13-lab-compare-equity-overlay.md).

## Numbers that matter

- **Tests:** 626 passed, 0 failed (re-verification run). Breakdown — `--lib` 456, `--features fixtures` 51, `--test live_equity_render` 15, `--test panel_snapshots` 103, H3 (`--features live --test lab_run_engine`) 1.
- **Overlay render proof (pixels in chart band, y ≥ 470):** primary curve **ACCENT = 1351**, second curve **ACCENT_2 = 841**; floor = 120. Single-run contrast: ACCENT present, ACCENT_2 below floor.
- **H3 no-regression:** `h3_in_memory_equals_cached_disk` PASS — the loader visibility change (`pub` → `pub(crate)`) did not break the in-memory-equals-disk seam.
- **Anchors:** 119 / 119 PASS — re-run for this deck (UI-only tripwire; none added or changed).
- **New theme tokens:** 0. New strings: 9, all via `strings.rs` (no inline hex/literals).
- **Spec-lint:** 70 violations in 2 categories (65 dead-link + 5 trace-broken-path) — all pre-existing debt, one *fewer* than the 2026-06-12 audit baseline of 71. No new category, none attributable to this feature.
- **Perf:** no hot-path change — the series is hydrated at scan time (`CachedCell.equity_series_ts`), not at paint time. No benchmark.

Anchor gate, re-run live for this deck:

```
$ bash scripts/verify_anchors.sh
...
ANCHORS PASS  (119 / 119)
```

## Open decisions

This was the last build piece of the Lab strategy-checking arc; there are no design questions left for you to settle. One thing to be aware of before you tick:

1. **The tester PASS followed a visual-baseline rebase — confirm you're comfortable with the recorded story.** The tester's *first* pass returned **FAIL** on 12 stale `compare__*` visual baselines (4 Compare fixtures × 3 viewport slots, dated Jun 8). That is the visual-regression gate doing exactly its job: appending the new "Equity overlay" panel below the matrix changed how the Compare screen looks, so the gate correctly demanded the new look be re-accepted rather than silently absorbed. The orchestrator deleted the 12 stale baselines, the harness auto-wrote new ones from the current render, and the orchestrator visually verified `compare__steady_state_populated__typical.png` shows the matrix intact plus the correct empty-state "Equity overlay" panel. The tester then re-verified → **PASS (626/626)**. Visual baselines are mutable by design (unlike byte-immutable anchored reports), so rebasing for an intentional UI change is the standard workflow. **Nothing for you to do here — this note exists so the FAIL→PASS in the report is not a surprise.** If you'd rather eyeball the rebased render yourself before approving, run the manual-capture recipe above.

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

### Notes / feedback
2026-06-13 — operator: **Approved — ship** (in-chat). Completes the "compare = KPIs + equity overlay" ask.

## Changelog
- 2026-06-13 (presenter): initial release deck. Render proof re-run live (15/15, ACCENT=1351/ACCENT_2=841); anchors re-run (119/119); spec-lint at baseline (70, no regression). Documents the FAIL→PASS baseline-rebase as the visual gate working as intended.
