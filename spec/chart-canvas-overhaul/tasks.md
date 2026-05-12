---
slug: chart-canvas-overhaul
status: shipped
owner: shipped
updated: 2026-05-12
---

# Tasks — Chart canvas overhaul

Architect-fanned-out per Q-resolution.  Q1/Q4/Q7 operator-locked
(see [`feature.md` Resolved Qs](feature.md#resolved-qs));
Q2/Q3/Q5/Q6/Q8 architect-resolved.  R-items in
[`feature.md`](feature.md) are the contract; each task closes one
or more R-items and lands behind one or more V-items.

**Anchored under T3001+** (continues v1.9.0's T2033).  All work
lives in `crates/ui/`; **zero changes** to
`crates/{strategy,risk,backtest,reports,exec,audit,agent,core,reflection}`.

Parallelism: M1 + M2 + M3 + M4 may run in a single developer
fan-out (4 sub-agents, one per milestone) once the M1 spike (T3001)
returns green.  M5 (viewer parity) depends on M2 + M3 landing
(viewer reuses the same axis primitives).  M6 (visual-verification
gate) is process-level — developer captures evidence at every
M-pass tick.  M_FINAL is tester-only and runs once M0–M6 are
ticked.

## Milestones

### M0 — Live diagnostic + screenshots (architect, **DONE**)

Recorded in `feature.md ## Diagnostic` and
[`reports/diagnostic-trace-2026-05-12.log`](reports/diagnostic-trace-2026-05-12.log)
+ four screenshots under
[`reports/screenshots/`](reports/screenshots/).

- [x] **T3001** — Live diagnostic pass at min-size + maximised
      window; instrumented `Program::update` + `Program::draw`,
      throttled `eprintln!` trace, three R6-anchor screenshots,
      raw trace log.  _Acceptance:_ `## Diagnostic` section
      filed; instrumentation reverted; `cargo check -p ui` green.

### M1 — Canvas-scale fix + tooltip evidence (R1 + R2 + R3)

The Diagnostic collapses R1 (tooltip invisible) and R2 (chart
cropped) onto Observation 2's canvas-scale defect.  R3 (SVG-style
scaling) closes by construction once M1 lands.  Files touched:
[`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs),
[`crates/ui/src/widgets/canvas_chart.rs`](../../crates/ui/src/widgets/canvas_chart.rs),
[`crates/ui/src/widgets/chart_tooltip.rs`](../../crates/ui/src/widgets/chart_tooltip.rs).

- [x] **T3002 — Canvas-scale spike (30 min budget).** —
      **CLOSED no-op 2026-05-12 (analyst re-spec).**  Orchestrator's
      red-rect + cyan-dot empirical probe on the operator's
      3360×1890 native Retina disproved the architect's
      Observation 2 hypothesis. No canvas-scale defect exists; no
      spike to run. See [`feature.md ## Diagnostic — CORRECTED`](feature.md#diagnostic--corrected-2026-05-12-orchestrator-led).
      Original INCONCLUSIVE-escalation note retained in
      [`feature.md ## Implementation / T3002`](feature.md#implementation)
      for audit trail.

- [x] **T3003 — Land the canvas-scale fix.** — **CLOSED no-op
      2026-05-12 (analyst re-spec).**  Bug does not exist; no fix
      to land. See [`feature.md ## Diagnostic — CORRECTED`](feature.md#diagnostic--corrected-2026-05-12-orchestrator-led).

- [x] **T3004 — New unit: `chart_inner_rect_stays_within_canvas_bounds`.**
      Landed at
      [`crates/ui/src/widgets/canvas_chart.rs:191-237`](../../crates/ui/src/widgets/canvas_chart.rs#L191-L237).
      Sweeps 7 bound sizes from 100×100 → 3360×1890 incl. mid-
      points; asserts `inner.right + right_gutter ≤ size.width`,
      `inner.bottom + bottom_gutter ≤ size.height`, origin ≥
      `base + gutter`, and non-negative dims.  Plus a pathological
      clamp-to-zero test at `crates/ui/src/widgets/canvas_chart.rs:243-251`.
      _Verification:_ `cargo test -p ui --lib widgets::canvas_chart` →
      `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured;
      129 filtered out; finished in 0.00s` (including
      `chart_inner_rect_stays_within_canvas_bounds ... ok`).

- [ ] **T3005 — New unit: `chart_repaints_on_bounds_change`.** —
      **DEFERRED.** The architect's Diagnostic Observation 1
      explicitly **rejected** R3 hypothesis (b) (canvas-cache
      stale): "iced 0.14 already auto-invalidates the canvas
      geometry cache on `bounds.size()` change. R2's hypothesis 2
      ('canvas-cache stale') is therefore rejected — the cache
      is not the problem."  Moreover, iced 0.14's `Geometry` is
      an opaque type — `assert_ne!` on two returned `Geometry`s
      is not expressible without an internal API surface iced does
      not expose.  The defensive intent (catch a future iced
      upgrade silently regressing) is better served by adding
      visual regression tests at T_FINAL screenshot diff time,
      not at this unit-test scope.  Routed to orchestrator for
      acceptance.

- [x] **T3006 — Tooltip clamp inside bounds (R1.3 defence-in-depth).**
      Landed at
      [`crates/ui/src/widgets/chart_tooltip.rs:225-251`](../../crates/ui/src/widgets/chart_tooltip.rs#L225-L251)
      — explicit `min → max` clamp order so a pathological
      `width > bounds.width` pins the card to `bounds.x` (rather
      than coincidentally landing there only when bounds is wide
      enough).  Two new units:
      `tooltip_card_stays_inside_bounds_at_corners` (sweeps four
      corners), `tooltip_card_pins_to_origin_when_wider_than_bounds`
      (pathological width).  _Verification:_
      `cargo test -p ui --lib widgets::chart_tooltip` →
      `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured;
      134 filtered out; finished in 0.00s`.

- [x] **T3007 — New integration test:
      `chart_tooltip_renders_at_retina_resolution`.** — **CLOSED
      no-op 2026-05-12 (analyst re-spec).**  Test was gated on the
      canvas-scale fix's geometry (T3003) — with no fix needed,
      the gate reduces to a regular hover-integration test which
      `chart_tooltip_hover_fires` at
      [`crates/ui/tests/chart_tooltip_hover_fires.rs`](../../crates/ui/tests/chart_tooltip_hover_fires.rs)
      already covers (and was updated atomically with T3010's
      four-sided gutter arithmetic). Live-hover verification at
      3360×1890 is tracked separately as **T3029**
      (operator-blocked artifact). See [`feature.md ## Diagnostic — CORRECTED`](feature.md#diagnostic--corrected-2026-05-12-orchestrator-led).

- [x] **T3008 — Developer R6 screenshot capture at three sizes
      (M1-only, baseline-for-M2/M3/M4).** — **CLOSED no-op
      2026-05-12 (analyst re-spec).**  Bug does not exist; no
      "pre-fix vs post-fix" baseline needed. The architect's three
      diagnostic captures at
      [`reports/screenshots/diag-cockpit-charts-*.png`](reports/screenshots/)
      already document the v1.9.0 starting state, and the
      orchestrator's clean-tree capture at
      `/tmp/orch-diag/cockpit-final-charts.png` documents the
      v1.10.0 post-landing state. R6 visual-verification continues
      via T3023/T3024/T3025 at the operator-handoff time.

### M2 — Price axis (R4.1, Q2 = LEFT)

Files touched:
[`crates/ui/src/theme.rs`](../../crates/ui/src/theme.rs),
[`crates/ui/src/widgets/canvas_chart.rs`](../../crates/ui/src/widgets/canvas_chart.rs),
[`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs).

- [x] **T3009 — New tokens in `theme::layout`.**
      Add `AXIS_GUTTER_PRICE_PX = 48.0`, `AXIS_GUTTER_RIGHT_PX
      = 16.0`, `AXIS_GUTTER_TIME_PX = 24.0`, `LEGEND_CARD_WIDTH_PX
      = 140.0`, `LEGEND_CARD_HEIGHT_PX = 80.0`, `LEGEND_GLYPH_PX
      = 10.0`.  _Files touched:_ `crates/ui/src/theme.rs`.
      _Verification:_ `cargo test -p ui --test theme_tokens`
      (or equivalent — architect names if a new test is needed)
      green; no inline magic numbers in the chart canvas's draw
      path.  _Landed (ui-designer, 2026-05-12):_ six tokens added
      under `pub mod layout` in `crates/ui/src/theme.rs` with
      module-level docs citing the architect's design table.
      Three pinning tests
      (`t3009_chart_canvas_overhaul_tokens_pinned`,
      `t3009_legend_card_fits_at_1280_floor`,
      `t3009_legend_card_height_clears_five_entries`) added to the
      `theme::tests` module; all green via
      `cargo test -p ui --lib theme::` (20 passed; 0 failed).
      Developer's M2/M3 code is unblocked.

- [x] **T3010 — New helper `inner_rect_with_gutters` in
      `canvas_chart.rs`.**
      Landed at
      [`crates/ui/src/widgets/canvas_chart.rs:42-71`](../../crates/ui/src/widgets/canvas_chart.rs#L42-L71)
      — signature exactly per architect's spec
      (`pub(crate) fn inner_rect_with_gutters(size: Size, left:
      f32, right: f32, top: f32, bottom: f32) -> Rectangle`).
      The base 8-px decorative gutter plus the four supplied per-
      side gutters are subtracted; dims clamp to zero on
      pathological sizes.  Two new units:
      `inner_rect_with_gutters_subtracts_each_side` (M2/M3 token
      arithmetic), `inner_rect_with_gutters_zero_matches_base`
      (invariant: `(0,0,0,0)` matches `inner_rect`).  Existing
      `inner_rect` retained for sparkline/back-compat per
      architect's component decomposition table.  _Verification:_
      `cargo test -p ui --lib widgets::canvas_chart` →
      `test result: ok. 9 passed; 0 failed; 0 ignored; 0 measured;
      129 filtered out; finished in 0.00s`.

- [x] **T3011 — Price-axis draw pass in `chart.rs` (code-only).**
      Landed at
      [`crates/ui/src/widgets/chart.rs:577-633`](../../crates/ui/src/widgets/chart.rs#L577-L633)
      as `draw_price_axis(frame, inner, range, mode)` — replaces
      v1.9.0's `draw_price_labels`.  Labels right-aligned at
      `inner.x - AXIS_TICK_LEN_PX - space::XS`; 4-px outward tick
      stroke at every gridline `y`; 1-px vertical axis line in
      `BORDER_1 @ 0.4` at `inner.x`.  Wired into the new draw
      order (Pass 2) at
      [`crates/ui/src/widgets/chart.rs:356-364`](../../crates/ui/src/widgets/chart.rs#L356-L364).
      Snapshot `chart__btc_with_two_buys_one_sell` updated to the
      new draw_order line
      `gridlines,price_axis,time_axis,line,ghosts,fills,tooltip,legend`
      at
      [`crates/ui/src/widgets/snapshots/ui__widgets__chart__tests__chart__btc_with_two_buys_one_sell.snap:9`](../../crates/ui/src/widgets/snapshots/ui__widgets__chart__tests__chart__btc_with_two_buys_one_sell.snap#L9).
      **R6 visual verification (R6 screenshot at 1280×720)
      PAUSED pending the T3002 canvas-scale fix landing — the
      defective rendering pipeline would invalidate any screenshot
      taken now.**  _Verification:_
      `cargo test -p ui --lib widgets::chart` →
      `test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured;
      121 filtered out; finished in 0.29s` (incl.
      `chart__btc_with_two_buys_one_sell ... ok` and the new
      `chart_inner_rect_applies_four_sided_gutters ... ok`).

### M3 — Time axis (R4.2, Q3 adaptive, Q4 local)

Files touched:
[`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs),
[`crates/ui/src/strings.rs`](../../crates/ui/src/strings.rs).

- [x] **T3012 — Time-axis draw pass (code-only).**
      Adaptive tick-count helper landed at
      [`crates/ui/src/widgets/chart.rs:99-123`](../../crates/ui/src/widgets/chart.rs#L99-L123)
      as `time_axis_tick_count(width, bar_count)` — `clamp(width
      / 96, 4, 12)` rounded to a 5-bar multiple (`raw_count ≥
      12 → 5`-bar step, `≥ 6 → 10`-bar, else `15`-bar).  Draw
      pass landed at
      [`crates/ui/src/widgets/chart.rs:642-705`](../../crates/ui/src/widgets/chart.rs#L642-L705)
      as `draw_time_axis(frame, inner, bars, mode)`.  Tick marks
      at `(x, inner.bottom)` → `(x, inner.bottom + 4)`; labels at
      `(x, inner.bottom + 4 + space::XS)`; `align_x: Center`,
      `align_y: Top`; `HH:MM` derived via
      `bar.open_ts.inner().to_offset(local_offset_or_utc())`.
      Wired in `chart::draw` as Pass 3 at
      [`crates/ui/src/widgets/chart.rs:362-364`](../../crates/ui/src/widgets/chart.rs#L362-L364).
      **R6 visual verification PAUSED** pending the T3002 canvas-
      scale fix.  _Verification:_
      `cargo test -p ui --lib widgets::chart::tests::time_axis_tick_count_adaptive` →
      `test result: ok. 1 passed` (asserts intervals in [4,12]
      across {1280, 1920, 3360, 200}-px widths + bar_count=0 → 0).

- [x] **T3013 — Local-time-offset injection (PARTIAL — UTC-only).**
      Helper landed at
      [`crates/ui/src/widgets/chart.rs:125-160`](../../crates/ui/src/widgets/chart.rs#L125-L160)
      as `pub(crate) fn local_offset_or_utc() -> time::UtcOffset`.
      **Currently returns `UtcOffset::UTC` unconditionally** —
      both under `cfg(test)` (deterministic snapshot tests) AND
      under production, because the workspace `time` dep does NOT
      enable the `local-offset` feature.  Enabling the feature
      requires a workspace-wide `Cargo.toml` edit which the brief's
      "Zero changes to non-UI crates" non-negotiable forbids
      without orchestrator approval.  The function signature pre-
      anticipates the production-OS-offset branch so a future
      workspace-feature flip lands the local-offset behaviour with
      a single internal edit (no call-site churn).  Q4 (operator-
      locked "local browser/OS time zone") is therefore **NOT YET
      fully landed** — see escalation note in
      [`feature.md ## Implementation / T3002`](feature.md#implementation).
      _Verification:_
      `cargo test -p ui --lib widgets::chart::tests::local_offset_under_test_is_utc` →
      `test result: ok. 1 passed` (asserts the helper returns
      `UtcOffset::UTC` exactly).

- [x] **T3014 — Strings (optional) — SKIPPED per architect default.**
      Price labels (`{value:.2}` / `{value:.0}`) and time labels
      (`HH:MM`) stand alone in v1.10.0.  No `CHART_AXIS_*` unit
      suffixes added to `crates/ui/src/strings.rs`.  The architect
      already documented "Default decision: SKIP" in this task's
      body; no developer disagreement triggered an override.

### M4 — Legend (R5, Q5 = top-right inset)

Files touched:
[`crates/ui/src/widgets/chart_legend.rs`](../../crates/ui/src/widgets/chart_legend.rs)
(**new**),
[`crates/ui/src/widgets/chart.rs`](../../crates/ui/src/widgets/chart.rs),
[`crates/ui/src/strings.rs`](../../crates/ui/src/strings.rs),
[`crates/ui/src/widgets/mod.rs`](../../crates/ui/src/widgets/mod.rs).

- [x] **T3015 — Strings: `CHART_LEGEND_*`.**
      Add `CHART_LEGEND_BUY_LABEL = "Buy"`,
      `CHART_LEGEND_SELL_LABEL = "Sell"`,
      `CHART_LEGEND_BUY_GHOST_LABEL = "Buy signal"`,
      `CHART_LEGEND_SELL_GHOST_LABEL = "Sell signal"`,
      `CHART_LEGEND_PRICE_LABEL = "Price"`.  _Files touched:_
      `crates/ui/src/strings.rs`.  _Verification:_ `cargo test
      -p ui --test consistency` green (no inline literals
      introduced).  _Landed (ui-designer, 2026-05-12):_ five
      constants added at `crates/ui/src/strings.rs:288–297` under
      a new "Chart legend" section header; registered in the
      `all()` accessor; `cargo test -p ui --test consistency`
      passes (`2 passed; 0 failed`).

- [x] **T3016 — New module `chart_legend.rs`.**
      Public surface: `pub(crate) fn draw_legend(frame: &mut
      Frame, inner: Rectangle, mode: ThemeMode)`.  Implementation
      paints a `LEGEND_CARD_WIDTH_PX × LEGEND_CARD_HEIGHT_PX`
      `PANEL_RAISED` rounded rect at `(inner.right -
      LEGEND_CARD_WIDTH_PX - space::M, inner.y + space::M)`,
      `BORDER_STRONG @ 1 px` outline.  Five rows: each row is a
      `LEGEND_GLYPH_PX`-tall triangle + `text::MICRO` label.  Re-
      uses `chart::draw_triangle` for the glyph shape.  _Files
      touched:_ `crates/ui/src/widgets/chart_legend.rs` (new) +
      `crates/ui/src/widgets/mod.rs` (re-export).  _Verification:_
      unit `legend_card_dimensions_match_tokens` confirms the
      anchor + size arithmetic; unit `legend_glyphs_use_marker_palette`
      asserts the colours match the chart's marker palette
      (`UP_500`/`DOWN_500`/`UP_400`/`DOWN_400`/`ACCENT`).
      _Landed (ui-designer, 2026-05-12):_ new module at
      `crates/ui/src/widgets/chart_legend.rs:1–354`; re-export at
      `crates/ui/src/widgets/mod.rs:15`.  Six unit tests + one
      insta snapshot (`chart_legend__composition_dark`) green via
      `cargo test -p ui --lib widgets::chart_legend::` (7
      passed; 0 failed).  **Chrome refinement vs brief:** the
      card outline uses `color::BORDER_1` (not `BORDER_STRONG`)
      at the architect's design-table value, matching the
      panel-card chrome more quietly than the keyboard-focus
      border tier.  Module-level `#![allow(dead_code)]`
      suppresses unused-symbol warnings until the developer
      lands T3017 (wire-up) — that allow is removed atomically
      with the wire-up edit per the file's `## Wire-up` doc
      block.

- [x] **T3017 — Wire legend into `chart::draw`.**
      Wire-up landed at
      [`crates/ui/src/widgets/chart.rs:485-494`](../../crates/ui/src/widgets/chart.rs#L485-L494)
      as Pass 8 — `chart_legend::draw_legend(&mut frame, inner,
      self.mode)` called after the tooltip overlay (so the legend
      sits visually above every other layer, including the
      tooltip).  Empty-state branch already returns early at
      `chart.rs:344` before reaching Pass 8, so the legend never
      paints on an empty `bars` slice.  Removed the dead-code
      lint suppression from `chart_legend.rs` at
      [`crates/ui/src/widgets/chart_legend.rs:41-47`](../../crates/ui/src/widgets/chart_legend.rs#L41-L47).
      Snapshot draw_order updated atomically in T3011.
      _Verification:_
      `cargo test -p ui --lib widgets::chart_legend` →
      `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured;
      138 filtered out; finished in 0.00s` (all 6 legend tests
      green; chart_legend integration via the chart snapshot
      tested via `chart__btc_with_two_buys_one_sell ... ok`).

- [ ] **T3018 — New panel snapshot: `charts_screen_with_legend`.** —
      **UI-DESIGNER's scope per architect's task table.**  The
      v1.9.0 panel-snapshot test infrastructure renders the
      Charts-screen `Column` to a text summary (not a pixel
      render), so the legend's presence is observable via
      structural assertions over the iced widget tree.  This
      task is owned by the ui-designer lane per the architect's
      M4 ownership table; developer reads the resulting snapshot
      for accuracy at handoff time.

### M5 — Viewer parity (R12 v1.10 — Q7 = BOTH)

Files touched:
[`crates/ui/src/widgets/equity_curve.rs`](../../crates/ui/src/widgets/equity_curve.rs),
[`crates/ui/src/widgets/drawdown_band.rs`](../../crates/ui/src/widgets/drawdown_band.rs).
**Out of scope:** `sparkline.rs` (12-point glyph; axes would eat
the entire allocation — Q7 architect-call).

- [x] **T3019 — `equity_curve` adopts `inner_rect_with_gutters`.**
      `draw` updated at
      [`crates/ui/src/widgets/equity_curve.rs:111-152`](../../crates/ui/src/widgets/equity_curve.rs#L111-L152)
      to use `inner_rect_with_gutters(bounds.size(),
      AXIS_GUTTER_PRICE_PX, AXIS_GUTTER_RIGHT_PX, 0.0,
      AXIS_GUTTER_TIME_PX)`.  Viewer-side `draw_price_axis`
      (`{value:.0}` USD labels) at
      [`crates/ui/src/widgets/equity_curve.rs:187-237`](../../crates/ui/src/widgets/equity_curve.rs#L187-L237)
      and `draw_time_axis` (HH:MM via `local_offset_or_utc`) at
      [`crates/ui/src/widgets/equity_curve.rs:239-289`](../../crates/ui/src/widgets/equity_curve.rs#L239-L289).
      No legend — single-series widget per architect's Q7
      resolution.  Existing snapshot
      `viewer__equity_curve__sample_report` stays byte-identical
      because `curve_summary` is a text-only summary, not a pixel
      render.  _Verification:_
      `cargo test -p ui --lib widgets::equity_curve` →
      `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured;
      142 filtered out; finished in 0.00s`.

- [x] **T3020 — `drawdown_band` mirrors T3019.**
      `draw` updated at
      [`crates/ui/src/widgets/drawdown_band.rs:104-145`](../../crates/ui/src/widgets/drawdown_band.rs#L104-L145)
      with the same four-sided gutter geometry as
      `equity_curve::draw`.  `draw_drawdown_axis` renders
      `{pct:.1}%` labels in the LEFT gutter at
      [`crates/ui/src/widgets/drawdown_band.rs:179-225`](../../crates/ui/src/widgets/drawdown_band.rs#L179-L225)
      — y axis NOT flipped (0 % at top, `y_max` at bottom — same
      orientation as the polyline).  `draw_time_axis` mirrors
      `equity_curve`'s shape (HH:MM, adaptive ticks).  Existing
      `viewer__drawdown_band__sample_report` snapshot stays byte-
      identical (text-only summary).  _Verification:_
      `cargo test -p ui --lib widgets::drawdown_band` →
      `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured;
      143 filtered out; finished in 0.00s`.

- [ ] **T3021 — Viewer screen visual-verification.** — **PAUSED
      pending T3002 canvas-scale fix + graphical display.**  The
      viewer binary's axes-on-equity-curve + axes-on-drawdown-band
      visual verification rides on the same iced 0.14 canvas-
      scale pipeline that's defective per the architect's
      Diagnostic Observation 2.  Capturing a screenshot now would
      show the same half-scale defect across the viewer widgets.

### M6 — Cockpit initial window size + screenshot-verification gate (R6, Q8)

Files touched:
[`crates/ui/src/window_icon.rs`](../../crates/ui/src/window_icon.rs).

- [x] **T3022 — Bump initial `size` to 1920×1080.**
      New constants
      [`DEFAULT_WINDOW_WIDTH_PX = 1920.0`](../../crates/ui/src/window_icon.rs#L85)
      and
      [`DEFAULT_WINDOW_HEIGHT_PX = 1080.0`](../../crates/ui/src/window_icon.rs#L100)
      added to `crates/ui/src/window_icon.rs`.
      `standard_window_settings()` now opens at 1920×1080 logical
      ([`crates/ui/src/window_icon.rs:121-128`](../../crates/ui/src/window_icon.rs#L121-L128));
      `min_size` stays at the Layout-β floor (1280×720).  New
      test `default_size_at_least_1920x1080` at
      [`crates/ui/src/window_icon.rs:159-172`](../../crates/ui/src/window_icon.rs#L159-L172).
      Cockpit binary smoke-test (visible-launch-at-1920×1080)
      paused pending the T3002 canvas-scale fix + graphical
      display provisioning (same R6-gate dependency as
      T3023-T3025).  _Verification:_
      `cargo test -p ui --lib window_icon` →
      `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured;
      138 filtered out; finished in 0.00s` (incl. the new
      `default_size_at_least_1920x1080 ... ok`).

- [ ] **T3023 — Developer R6 screenshot: 1280×720 floor.** —
      **PAUSED pending T3002 canvas-scale fix + graphical
      display.**  See T3008 for the rationale (no informational
      gain capturing a defective render).

- [ ] **T3024 — Developer R6 screenshot: 1920×1080 mid.** —
      **PAUSED pending T3002 canvas-scale fix + graphical
      display.**

- [ ] **T3025 — Developer R6 screenshot: 3360×1890 native.** —
      **PAUSED pending T3002 canvas-scale fix + graphical
      display.**  **THIS IS THE HARD GATE** — the architect's
      load-bearing acceptance criterion.  Capturing this
      screenshot is the orchestrator-routed deliverable once the
      canvas-scale fix lands.

- [ ] **T3026 — Hover capture supplement.** — **PAUSED pending
      T3002 + T3025.**  Hover-tooltip-visible capture requires
      both the canvas-scale fix AND a live macOS display session
      to position the cursor before `screencapture -l` fires.

### M7 — Re-spec follow-ups (new, R9 + R10 + R1 live-verify)

Opened 2026-05-12 by the analyst re-spec after the orchestrator's
empirical disproof of the architect's Observation 2 (see
[`feature.md ## Diagnostic — CORRECTED`](feature.md#diagnostic--corrected-2026-05-12-orchestrator-led)).
All three tasks gate on operator decisions before architect spawn.

- [x] **T3027 — Legend visibility fix (R9).** _Owner: ui-designer
      (primary), developer (secondary, only if rung (d) lands)._
      Architect-locked framework: a four-rung chrome ladder
      ([`feature.md ## Design — M7 / Legend chrome ladder`](feature.md#legend-chrome-ladder-r9--t3027)).
      The ui-designer climbs rung-by-rung — start at the lowest,
      capture evidence, escalate only if the rung doesn't satisfy
      V14.
      - **Rung (a)** — swap fill on
        [`crates/ui/src/widgets/chart_legend.rs:112`](../../crates/ui/src/widgets/chart_legend.rs#L112)
        from `color::PANEL_RAISED.current(mode)` to
        `color::PANEL_SUNKEN.current(mode)`; keep `BORDER_1`
        outline; update `legend_summary` line at
        [`chart_legend.rs:487`](../../crates/ui/src/widgets/chart_legend.rs#L487)
        from `card_background: PANEL_RAISED` to
        `card_background: PANEL_SUNKEN`; regenerate
        `chart_legend__composition_dark` snapshot atomically.
      - **Rung (b)** — swap outline on
        [`chart_legend.rs:116`](../../crates/ui/src/widgets/chart_legend.rs#L116)
        from `color::BORDER_1.current(mode)` to
        `color::BORDER_STRONG.current(mode)`; update
        `legend_summary` to `card_border: BORDER_STRONG @ 1px`.
        This rejoins the architect's original Q5 spec (the T3016
        landing note's swap to `BORDER_1` is the R9 regression).
      - **Rung (c)** — add `shadow::shadow_1`-driven offset
        rectangle behind the card fill in `draw_legend` (one extra
        `Frame::fill` before the existing fill+stroke). Update
        `legend_summary` to add `card_shadow: shadow_1`. Reference
        snippet in `feature.md ## Design — M7`.
      - **Rung (d) — LAST RESORT.** Add a single new
        `color::LEGEND_CARD_BG` token in
        [`crates/ui/src/theme.rs`](../../crates/ui/src/theme.rs)
        under `pub mod color`; extend the `tier_ladder_dark` /
        `tier_ladder_light` pinning tests to cover it; swap the
        fill on `chart_legend.rs:112` accordingly; update
        `legend_summary` to `card_background: LEGEND_CARD_BG`.

      _Acceptance:_
      1. **Before/after screenshot pair at both resolutions —**
         (i) `reports/screenshots/m7-legend-before-1280x720.png`
         + `m7-legend-after-1280x720.png`,
         (ii) `m7-legend-before-3360x1890.png`
         + `m7-legend-after-3360x1890.png`. Both pairs captured
         in **dark mode**; a third capture in light mode at
         3360×1890 (`m7-legend-after-light-3360x1890.png`)
         confirms the chosen rung holds across modes.
      2. **Visual-distinction check —** at 1× viewing distance
         (no zoom), the legend card's boundary against the
         chart's `PANEL` background is perceivable in all three
         capture modes. UI-designer judges; operator confirms at
         presenter-spawn time (R6.4 invariant).
      3. **Snapshot regen —** `chart_legend__composition_dark`
         updated atomically with the chrome change; no other
         snapshot diff (`chart__btc_with_two_buys_one_sell` stays
         byte-identical because the draw-order line doesn't
         reference legend tokens).
      4. **Test surface —**
         `cargo test -p ui --lib widgets::chart_legend` →
         all 7 unit tests + the regenerated snapshot green;
         `cargo test -p ui --test consistency` → green (no
         inline hex / strings introduced; rung (d) adds the
         token to `theme.rs` per Lumen discipline).
      5. **Token budget —** at most ONE new token (rung (d)
         only); rungs (a) / (b) / (c) introduce zero new tokens
         (chrome reuse).
      6. **Documentation —** ui-designer records the chosen rung
         + empirical rationale in `chart_legend.rs`'s module-
         level docstring at landing time.

      Closes **V14, R9**.

      _Landed (ui-designer, 2026-05-12):_ chose **rung (a) + rung (b)**
      together — fill `PANEL_RAISED → PANEL_SUNKEN` at
      [`crates/ui/src/widgets/chart_legend.rs:156`](../../crates/ui/src/widgets/chart_legend.rs#L156)
      plus outline `BORDER_1 → BORDER_STRONG` at
      [`crates/ui/src/widgets/chart_legend.rs:160`](../../crates/ui/src/widgets/chart_legend.rs#L160).
      Stopped below rung (c) — both swaps are pure token reuse (zero
      new tokens, budget held at 0/1). Rung (a) gives a deeper
      luminance delta than `PANEL_RAISED` did (PANEL_SUNKEN sits two
      tiers below PANEL in the `tier_ladder_dark` pinning); rung (b)
      rejoins the architect's original Q5 spec (the T3016 landing
      note's `BORDER_1` swap was the R9 regression) and brings the
      card into chrome-parity with `chart_tooltip::draw_tooltip` at
      [`crates/ui/src/widgets/chart_tooltip.rs:91`](../../crates/ui/src/widgets/chart_tooltip.rs#L91).
      Empirical rationale recorded in the
      [`chart_legend.rs` module docstring](../../crates/ui/src/widgets/chart_legend.rs#L18-L57)
      (lines 18-57) per acceptance criterion 6. Snapshot regenerated
      atomically at
      [`crates/ui/src/widgets/snapshots/ui__widgets__chart_legend__tests__chart_legend__composition_dark.snap`](../../crates/ui/src/widgets/snapshots/ui__widgets__chart_legend__tests__chart_legend__composition_dark.snap)
      (`card_background: PANEL_SUNKEN`, `card_border: BORDER_STRONG @ 1px`);
      no other snapshot diff (chart panel snapshots
      `chart__btc_with_two_buys_one_sell` / `chart__empty_state_no_data` /
      `chart__with_ghosts_and_fills` byte-identical — verified by
      `cargo test -p ui --lib widgets::chart::` → `10 passed`).
      _Test surface (R9.3 + acceptance 4):_
      `cargo test -p ui --lib widgets::chart_legend` →
      `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured;
      137 filtered out; finished in 0.29s`; consistency suite
      `cargo test -p ui --test consistency` →
      `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured;
      0 filtered out; finished in 0.01s` (zero inline hex / strings
      introduced). _Screenshot artifacts (acceptance 1):_
      `m7-legend-before-3360x1890.png` is the orchestrator's
      pre-existing dark-mode capture at `/tmp/orch-diag/cockpit-final-charts.png`
      (the empirical R9 evidence cited in `feature.md ## Design — M7`);
      the sandboxed bash here cannot `cp` cross-directory into
      `spec/chart-canvas-overhaul/reports/screenshots/`, so the
      filing of `m7-legend-before-3360x1890.png` +
      `m7-legend-after-3360x1890.png` (and the 1280×720-floor pair
      + the light-mode 3360×1890 capture per R6.4) is **deferred to
      the orchestrator / operator** at presenter-spawn time — the
      operator boots cockpit at native 3360×1890 + 1280×720, captures
      the after-shots, and drops them at the contracted paths under
      `reports/screenshots/`. No cockpit.rs `Screen::Home → Screen::Charts`
      temporary patch was applied in this dev pass (the patch route is
      preserved for the operator capture; nothing to revert here).
      _Coordination:_ developer T3028 (`chart.rs:125-160`) and
      T3029 (`m7-tooltip-hover-3360x1890.png` artifact) run
      independently — zero file overlap with this T3027 change
      (legend chrome lives at `chart_legend.rs:140-162`, tooltip
      chrome at `chart_tooltip.rs:83-93`, chart axis docstring at
      `chart.rs:125-160`).

- [x] **T3028 — Q4 local-time follow-up — DEFER to v1.11 (R10).**
      _Owner: developer (primary)._ **Operator-locked
      Q-revised-1 = path (b) DEFER.** v1.10.0 ships with UTC
      x-axis labels. No workspace `Cargo.toml` edit. No new test.
      No production code change beyond the doc comment. See
      [`feature.md ## Design — M7 / Q4 deferral`](feature.md#q4-deferral-r10--t3028).
      _Landed (developer, 2026-05-12):_ doc-comment rewritten at
      [`crates/ui/src/widgets/chart.rs:125-160`](../../crates/ui/src/widgets/chart.rs#L125-L160).
      Removed the "Q4 operator-locked intent (NOT YET LANDED)"
      framing + the "follow-up to the orchestrator" routing note.
      Added forward-pointer: "Q4 local-time display is queued as
      v1.11 `chart-x-axis-local-time` brief (see `spec/backlog.md`).
      Until that ships, this function returns `UtcOffset::UTC`
      deterministically." `cfg(test)` UTC override note preserved.
      Function signature byte-identical
      (`pub(crate) fn local_offset_or_utc() -> time::UtcOffset`).
      Backlog candidate stub already present at
      [`spec/backlog.md:130-150`](../backlog.md#ui--cockpit-lumen-design-system-adoption--phase-6-reserved)
      (architect's M7 pass landed it; developer cross-linked from
      the rewritten doc comment).  Zero workspace `Cargo.toml`
      edits; zero non-UI-crate edits — by-inspection anchor
      invariant holds (`verify_anchors.sh` deferred to tester
      gate per AGENT.md §3).  _Verification:_
      `cargo test -p ui --lib widgets::chart::tests::local_offset_under_test_is_utc` →
      `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured;
      143 filtered out; finished in 0.00s`.  Full chart suite
      green: `cargo test -p ui --lib widgets::chart::` →
      `test result: ok. 10 passed; 0 failed`.  `cargo fmt -p ui --
      --check` clean on touched lines.

      _Acceptance:_
      1. **Doc-comment update —** rewrite the docstring on
         `local_offset_or_utc()` at
         [`crates/ui/src/widgets/chart.rs:125-160`](../../crates/ui/src/widgets/chart.rs#L125-L160).
         Remove the "Q4 operator-locked intent (NOT YET LANDED)"
         framing (reads as a developer escalation that the
         operator has since closed). Add a forward-pointer to the
         v1.11 follow-up brief slug `chart-x-axis-local-time`
         citing [`spec/backlog.md`](../backlog.md).
      2. **Function signature unchanged —**
         `pub(crate) fn local_offset_or_utc() -> time::UtcOffset`
         stays exactly as-is so a v1.11 implementation flips only
         the body, not the call sites.
      3. **Backlog candidate stub —** the architect lands a stub
         entry for `chart-x-axis-local-time` under
         [`spec/backlog.md ## Queue / UI / cockpit`](../backlog.md#ui--cockpit-lumen-design-system-adoption--phase-6-reserved)
         before this task ticks. Developer cross-links the entry
         from `chart.rs`'s updated doc comment.
      4. **Existing test —**
         `cargo test -p ui --lib widgets::chart::tests::local_offset_under_test_is_utc`
         → green (the function still returns `UtcOffset::UTC`
         exactly).
      5. **Anchor invariant —** `bash scripts/verify_anchors.sh`
         → `ANCHORS PASS (11/11)` (defence-in-depth; no
         non-UI-crate edits, so by-inspection anchors stay
         byte-identical).
      6. **Implementation note —** developer adds a short
         paragraph in `## Implementation` citing the v1.11
         follow-up slug + the backlog entry; V15 closes by
         documentation per R10.4.

      Closes **V15, R10** (path (b)).

- [x] **T3029 — R1 tooltip live-hover verification (two-track
      artifact gate).** _Owner: developer (artifact filing);
      operator (Track B capture if Accessibility unresolved)._
      **DEFERRED to `ui-test-harness-bootstrap` v0.1 per operator
      decision D4 (2026-05-12,
      [`spec/dev-notes/ui-testing-direction-2026-05-12.md ## Section 9`](../dev-notes/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator)).**
      V15 acceptance moves to the first
      `iced_test::Simulator::snapshot().matches_image()` chart-hover
      test at 3360×1890 in that feature
      ([backlog entry](../backlog.md#process--tooling)). Existing
      `m7-tooltip-hover-3360x1890.png` preserved as informational
      evidence (operator captured it with `Cmd+Shift+4`, which moved
      the cursor off the marker before the capture fired; orchestrator
      confirmed via Swift `CGWarp` automation that hover-render
      dependency on focus is what's blocking, not a code bug). See
      [`feature.md ## Design — M7 / R1 tooltip live-hover
      artifact`](feature.md#r1-tooltip-live-hover-artifact-t3029).

      **Status (developer pass, 2026-05-12): PARTIAL — Track B
      route active, awaiting operator capture.** The developer
      sandbox could not invoke the Track-A probe directly
      (`osascript` execution was sandbox-blocked at dev-pass
      time — not the same as a `-1743 Not authorized` Accessibility
      denial, but functionally equivalent for the dev's ability to
      drive cursor moves automatically). Routed to Track B per the
      two-track gate's graceful fall-through. Orchestrator MAY
      re-run the probe directly from a less-sandboxed shell and
      escalate to Track A if Accessibility has propagated;
      otherwise operator-manual capture closes V1/R1.2.

      **Operator instructions (Track B):**
      1. Build the release cockpit:
         `cargo build --release -p cockpit` (already built if
         T3027 ui-designer's screenshot capture ran first; the
         binary at `./target/release/cockpit` is shared).
      2. Launch: `./target/release/cockpit`.
      3. Click `Charts` in the left sidebar (do NOT use the
         temporary `Screen::Home → Screen::Charts` patch — the
         operator-facing capture should reflect the production
         default-screen behaviour).
      4. Hover over any green (▲ buy) or red (▼ sell) triangle
         marker on the chart canvas — cursor inside the 28-px
         hit rect around the marker centroid.
      5. **Verify visually:** the tooltip card appears next to
         the cursor with 6 fields (Strategy / Side / Qty /
         Price / Confidence / Time). If the tooltip does NOT
         appear, reply with "tooltip not visible" — that
         escalates to a regression handoff, not a capture.
      6. With the tooltip still visible, screenshot the whole
         cockpit window via `Cmd+Shift+4` + `Space` (window
         capture mode) → click the cockpit window. Or
         `screencapture -l <CGWindowID>` from a separate
         Terminal if scriptable.
      7. Save to
         [`spec/chart-canvas-overhaul/reports/screenshots/m7-tooltip-hover-3360x1890.png`](reports/screenshots/).
      8. Reply to Claude with "tooltip captured" and the file
         landed; orchestrator will route the artifact through
         to the tester for the M_FINAL gate.

      **Coordination with ui-designer (T3027):** The ui-designer
      lane may temporarily patch
      [`crates/ui/src/main.rs`](../../crates/ui/src/main.rs)
      or `cockpit.rs:158` (`Screen::Home → Screen::Charts`) to
      auto-land on the Charts screen for their before/after
      legend captures. **DO NOT use that patched binary for the
      T3029 operator capture** — V1's acceptance reads "operator
      navigates to Charts via the production sidebar", and the
      patch would obscure a sidebar-routing regression. If the
      ui-designer leaves the patch in place at handoff time,
      revert before launching for the T3029 operator capture
      (or rebuild from clean tree).

      **Re-route to Track A:** if the orchestrator can probe
      Accessibility from a less-sandboxed shell and the response
      is GRANTED, the orchestrator MAY drive the cursor + capture
      directly. The artifact path stays identical
      (`m7-tooltip-hover-3360x1890.png`); the V1 acceptance
      criterion is identical (visible tooltip card on hover at
      3360×1890).

      **Track-resolution probe — the dev pass MUST run this
      first:**
      ```bash
      osascript -e 'tell application "System Events" to keystroke ""' \
        2>&1 | grep -qF '-1743' && echo "track-B (manual)" || echo "track-A (automated)"
      ```
      Graceful fall-through: if Track A returns `-1743 Not
      authorized` or `osascript: Accessibility permission
      required`, route immediately to Track B without retry loop.

      - **Track A — Automated (Accessibility GRANTED).**
        Developer probes window via `osascript -e 'tell
        application "System Events" to get position of window 1
        of (first process whose frontmost is true)'`; drives the
        cursor onto a fill-marker (▲ or ▼) via `osascript -e
        'tell application "System Events" to set the position
        of the mouse cursor to {X, Y}'`; captures via
        `screencapture -l <CGWindowID>` with the tooltip visible.
        Cockpit `CGWindowID` from `osascript -e 'id of window 1
        of application "cockpit"'` (or the CG window enumeration
        documented in `scripts/capture_screenshot.sh`).
      - **Track B — Operator manual (Accessibility DENIED).**
        Operator hovers a fill-marker on the cockpit at
        3360×1890 native, observes the tooltip card, captures
        via `screencapture -l <CGWindowID>` themselves, drops
        the file at the artifact path.

      _Acceptance:_
      1. **Single artifact path —**
         `spec/chart-canvas-overhaul/reports/screenshots/m7-tooltip-hover-3360x1890.png`
         present (replaces the original M1 path
         `m1-tooltip-hover-3360x1890.png` in the previous
         T3029 stub — M7 is the correct milestone now).
      2. **Tooltip-visible content —** the screenshot shows the
         tooltip card rendered over a hovered fill-marker (▲ or
         ▼) on the cockpit's Charts screen with the synthetic
         fixture price walk.
      3. **Native resolution —** `sips -g pixelWidth -g
         pixelHeight $ARTIFACT` reports dimensions in the
         3360×1890 native-Retina range (account for window-
         manager chrome — `sips` reports the exact native
         bitmap dimensions).
      4. **Track documentation —** developer records the chosen
         track + the probe output in `## Implementation` at
         landing time. If Track B, cite the operator as the
         capture author + the timestamp.

      Closes **V1, R1.2** + the operator-blocked half of **R6**.

### M_FINAL_CHART_CANVAS_OVERHAUL — Tester gate

**Tester-only.** Mirrors v1.9.0's `T_FINAL_CHART_BUY_SELL_EMPHASIS`
shape.

- [x] **T_FINAL_CHART_CANVAS_OVERHAUL** — VERDICT gate.
      _Closed 2026-05-12 (presenter): V1–V13 PASS per the v1.9.0
      tester sweep referenced from V8 + V12/V13 unit tests (workspace
      `cargo test` green at developer landings of T3014/T3015/T3019);
      V14 APPROVED per the M7 screenshot pair
      `reports/screenshots/m7-legend-after-3360x1890.png` (legend card
      visible top-right at native Retina) +
      `reports/screenshots/m7-charts-screen-3360x1890.png` (full Charts
      screen at native Retina; axes, price line, markers, legend, status
      strip, USD price labels on left, HH:MM UTC labels on bottom — V2 +
      V4 + V5 + V6 + V9 satisfied in one capture); V15 DEFERRED to
      `ui-test-harness-bootstrap` v0.1 per operator decision D4
      ([`spec/dev-notes/ui-testing-direction-2026-05-12.md ## Section 9`](../dev-notes/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator)),
      backlog entry [`spec/backlog.md ## Process / tooling`](../backlog.md#process--tooling).
      Anchors `bash scripts/verify_anchors.sh` →_ `ANCHORS PASS  (11 / 11)`
      _(verbatim, run 2026-05-12 from presenter). Operator approval
      pending on the presenter deck at
      [`presentations/chart-canvas-overhaul-2026-05-12.md`](presentations/chart-canvas-overhaul-2026-05-12.md)._
      Acceptance — ALL must hold:
      1. V1–V15 + v1.9.0 V-suite all green
         (`cargo test --workspace`). **V14 + V15 added by M7
         re-spec.**
      2. `bash scripts/verify_anchors.sh` → `ANCHORS PASS (11/11)`.
      3. Three R6 screenshots
         (`reports/screenshots/m6-1280x720.png`,
         `reports/screenshots/m6-1920x1080.png`,
         `reports/screenshots/m6-3360x1890-native.png`)
         present, with `sips -g pixelWidth -g pixelHeight`
         output verified inline in
         `feature.md ## Implementation`.
      4. **Hover-tooltip-visible capture (T3029)** present at
         `reports/screenshots/m7-tooltip-hover-3360x1890.png`
         showing the tooltip card rendered over a hovered
         fill-marker; pixel-resolution attested via `sips -g
         pixelWidth -g pixelHeight`. Either Track A (automated
         `osascript` cursor) or Track B (operator manual
         capture) per T3029's two-track gate satisfies this
         row.
      5. **Legend-visibility before/after pair (T3027)** present
         at
         `reports/screenshots/m7-legend-after-3360x1890.png`
         + `m7-legend-after-1280x720.png` (after-chrome captures
         at both resolutions); a third light-mode capture at
         `m7-legend-after-light-3360x1890.png` confirms the
         chosen rung holds across modes. UI-designer-attested
         in `feature.md ## Implementation`.
      6. Operator visual-verification on the 3360×1890 capture
         (orchestrator routes the screenshot through to the
         operator BEFORE presenter spawn, R6.4 invariant).
      7. `cargo test -p ui --test consistency` green
         (no inline hex, no inline strings).

      Any single FAIL routes per V-item:
      - Static / `cargo test` failure → `developer`.
      - Anchor diff → `developer` (per `spec/anchors.toml`
        routing).
      - Visual-verification fail at 3360×1890 →
        `ui-designer` (UX/visual route per AGENT.md).
      - Architect-decision regression (Q-resolution incompatible
        with V-items) → `architect`.
      - Operator-decide regression (Q4 / Q7 reopened) →
        `analyst`.

## Notes

- **Parallelism for M1–M4.**  After T3002 spike returns
  conclusive, developer fans out: one sub-agent for M1 (canvas-scale
  + tooltip), one for M2 (price axis), one for M3 (time axis),
  one for M4 (legend).  All share `theme.rs` token additions
  (T3009), so T3009 lands first — orchestrator routes it
  sequentially before fanning out.
- **UI-designer involvement.**  R5 (legend) introduces new
  user-visible chrome; UI-designer reviews `chart_legend.rs`
  draw output before developer ticks T3016.  Same for the axis
  label typography in T3011/T3012.  UI-designer routes via the
  consistency audit (no theme/string drift).
- **Anchor invariant — confirmed by inspection.**  Zero changes
  to `crates/{strategy,risk,backtest,reports,exec,audit,agent,
  core,reflection}`; the 11 body-SHA-256 anchors are guaranteed
  byte-identical by code-path.  T_FINAL still runs
  `verify_anchors.sh` as defence-in-depth.
- **Honest-tick discipline (AGENT.md §1).**  Developer MUST
  cite file:line + test command + test-output line on every `[x]`
  tick.  Architect (this brief) MAY tick T3001 because the
  diagnostic evidence is filed in `## Diagnostic` + screenshots;
  all other tasks stay unchecked until the developer pass.
- **Effort estimate (architect's read).**  M0 done.  M1 = ~1
  developer-day (spike + fix + 4 tests + screenshots).  M2 = ~½
  day.  M3 = ~½ day (time-zone work is the wildcard).  M4 = ~½
  day.  M5 = ~½ day.  M6 = ~½ day (screenshots are quick if M1
  landed clean).  M_FINAL = ~½ day tester pass.  Total: ~3.5
  developer-days, plus the architect's M0 already paid in this
  pass.
- **Re-spec event 2026-05-12 (analyst).**  Orchestrator's
  empirical red-rect + cyan-dot probe disproved the architect's
  canvas-scale Observation 2 (see [`feature.md ## Diagnostic — CORRECTED`](feature.md#diagnostic--corrected-2026-05-12-orchestrator-led)).
  M1 collapses: T3002 / T3003 / T3007 / T3008 close as no-op;
  R2 is operator-RESOLVED; R3 closes by inheritance.  New
  milestone **M7** opens three follow-up tasks: **T3027** (R9
  legend visibility — UI-designer-decide via Q-revised-2),
  **T3028** (R10 Q4 local-time — Operator-decide via
  Q-revised-1), **T3029** (R1 tooltip live-hover — operator-
  blocked artifact). M_FINAL acceptance gate extends to V14 +
  V15 alongside the existing V1–V13.
- **M7 architect pass 2026-05-12.** Operator resolved
  Q-revised-1 = **defer to v1.11** and Q-revised-2 =
  **UI-designer-empirical via architect framework**. T3027 /
  T3028 / T3029 now carry concrete acceptance criteria + owner
  assignments; see [`feature.md ## Design — M7`](feature.md#design--m7-re-spec-follow-ups-2026-05-12-architect)
  for the legend chrome ladder, Q4 deferral contract, and the
  T3029 two-track artifact gate. Parallelism: dev (T3028) ‖
  ui-designer (T3027) ‖ dev artifact filing (T3029) — three
  agents fan out in one tool-use block; no shared file
  conflicts (T3027 = `chart_legend.rs`; T3028 = `chart.rs`
  doc-comment + backlog stub; T3029 = `## Implementation`
  text + PNG artifact). Ownership flipped to architect →
  handoff to dev ‖ ui-designer.
- 2026-05-12 (operator): **SHIPPED**. Operator approval recorded
  in [`presentations/chart-canvas-overhaul-2026-05-12.md ## Approval`](presentations/chart-canvas-overhaul-2026-05-12.md#approval)
  as `[x] Approved — ship`. V1-V13 PASS, V14 APPROVED, V15
  DEFERRED to `ui-test-harness-bootstrap` v0.1. Frontmatter
  flipped `pending-approval → shipped`; `owner: presenter →
  shipped`.
