//! Render-verifiable harness for the Live equity curve — the regression
//! guard that exercises the **real render path** (cockpit-live-equity-render-guard,
//! 2026-06-11).
//!
//! ## Why this file exists (the lesson)
//!
//! Every prior fix to the Live equity curve was verified at the **agent /
//! unit-test layer** (publishing a `PnlSnapshot`, or asserting
//! `live_equity_buffer.len()`), which passed — while the **actually rendered
//! curve** was broken and the operator kept seeing "no graph". The
//! `state.rs` headless tests assert the model `PanelState` transitions; the
//! `panel_snapshots.rs` `live_screen_summary` builds a TEXT summary — neither
//! rasterizes the canvas. So a bug that drops the curve's points (the bar-time
//! monotone-guard regression) sails through all of them.
//!
//! This harness closes that gap: it drives a realistic sequence of
//! `Message::PnlRefreshed` snapshots into the cockpit state (the production
//! `update` path), renders the **real Live screen** via `iced_test::screenshot`
//! (which runs `view` → `EquityCurveProgram::draw` → tiny_skia rasterization,
//! the same path the live `cockpit_live` binary paints), and inspects the
//! curve region of the RGBA buffer for the polyline's `ACCENT`-colored pixels.
//!
//! An empty / Loading / degenerate curve draws **zero** `ACCENT` pixels in the
//! curve band (it shows the muted "No equity data" body on a `PANEL`
//! background; gridlines are `BORDER_1`, never `ACCENT`). A real Ready curve
//! draws a visible `ACCENT` polyline. So `ACCENT`-pixel-count > threshold
//! ⟺ the curve actually rendered. THIS is the signal the prior tests lacked.
//!
//! ## bug-log #205 — the Lab's committed-evidence fallback (PHASE 4b)
//!
//! Vocabulary: **container depth 0** = `<root>/<slug>/reports/`, **container
//! depth 1** = `<root>/<container>/<slug>/reports/`. The Lab's cold path reads
//! `[lab-runs/, evidence/]` (ADR-0055 § D5), but its report walk read
//! container depth 0 only, while every committed report the Lab maps to sits at
//! container depth 1 (`evidence/v1/<slug>/reports/`) since `1395520629dc` — so
//! `evidence/` answered nothing. The walk now goes through
//! `ui::reports::layout` (ADR-0055 D8, container depth ≤ 1; within one root a
//! tie goes to container depth 0 first). PHASE 4b proves it on the same
//! overlay draw path `lab::view` uses:
//!
//! - **LR1** [`lab_curve_hydrated_from_container_depth_1_evidence_report_renders`]
//!   — a report at `<tmp>/evidence/v1/<slug>/reports/`, resolved by
//!   `route_equity_overlay` over the production-ordered roots, adds
//!   ≥ `OVERLAY_ACCENT_DELTA_MIN` ACCENT px. PNG
//!   `/tmp/lab_container_depth_1_evidence_overlay.png`.
//! - **LR2** [`lab_curve_from_container_depth_2_evidence_report_does_not_resolve`]
//!   — the same bytes one container deeper resolve to nothing
//!   (`route_equity_overlay` is `None`, `load_equity` is `ReportDirNotFound`).
//!   The routing assertion carries the negative control; there is no pixel
//!   assertion (a `None` overlay renders the bars-only baseline by
//!   construction). PNG `/tmp/lab_container_depth_2_evidence_overlay.png`.
//! - **LR3** [`lab_committed_start_end_only_report_over_bars_is_pixel_identical_to_bars_only`]
//!   — the real committed momentum report resolves through the production
//!   roots (its `.expect("… must resolve …")` is the #205 half); being
//!   start/end-only with the ordinal `(0, 1)` timestamps, it draws nothing over
//!   bars, so the frame is byte-identical to the bars-only frame (the bug-log
//!   #207 half). The #207 fix replaced the characterisation that first held
//!   this key (`lab_207_characterization_committed_start_end_only_report_draws_no_overlay`,
//!   which pinned the empty gutter; its curve-delta half was green on both
//!   sides of the fix, so it could not be the #207 gate). PNG
//!   `/tmp/lab_207_committed_overlay.png`.
//!
//! LR1 prints its ACCENT delta and threshold on stderr (`#205 pixel margin …`),
//! LR3 its count of pixels that differ from the bars-only frame (`#207 pixel
//! margin …`); run with `--nocapture`.
//!
//! ### T-T4 probe table (bug-log #205), Lab legs
//!
//! The same probes as the table in `reports_populated_curve_render.rs` (which
//! also holds the test keys and the Reports-leg sets), with the OLD → NEW text
//! repeated verbatim and the Lab-side sets: `E*` = `lab::equity_loader` lib
//! tests, `LR*` = this file, H3 = `lab_run_engine.rs`. Protocol: replace the
//! exactly-once OLD with NEW, `touch`, re-run (a `Compiling ui` line precedes
//! the red run), restore with `cp` + `touch`, `cmp` against the backup. `\|`
//! below is the Markdown escape of `|`. Paths are under `crates/ui/src/`.
//! "Measured" = applied and run on 2026-11-15; "REASONED" = derived from code.
//!
//! The LR3 cells were measured against the characterisation that held the key
//! until the #207 fix. Its replacement keeps the routing precondition
//! (`route_equity_overlay(…).expect("… must resolve …")`, `StartEndOnly`,
//! samples `(0, 1)`) and adds the frame identity, so each cell carries over:
//! under P1 and P4 the committed report no longer resolves and the new LR3 is
//! RED at that precondition (REASONED: the same `expect` the old LR3 failed
//! on); under every other probe the resolved report is unchanged — one
//! `v1-cross-sectional-momentum/reports/` dir exists under `evidence/`, and in
//! it only the `2024-h1` report scores for the momentum tuple — so the frame
//! identity holds (REASONED). Counts such as "18 of 18" are of this file
//! before the #207 tests were added.
//!
//! | Probe | File / symbol | OLD → NEW | Lab RED | Lab GREEN | Measured |
//! |---|---|---|---|---|---|
//! | P1 | `reports/layout.rs` `MAX_CONTAINER_DEPTH` | `pub const MAX_CONTAINER_DEPTH: usize = 1;` → `pub const MAX_CONTAINER_DEPTH: usize = 0;` | E1, E2b (2 ≠ 4), E3, E3b, E4, E4c, E5, LR1 (None), LR3 ("must resolve") | E1b, E2, E4b, LR2, the container-depth-0 Lab tests, H3 | measured |
//! | P2 | same | `pub const MAX_CONTAINER_DEPTH: usize = 1;` → `pub const MAX_CONTAINER_DEPTH: usize = 2;` | E1b (Ok), LR2 (resolves) | E1, E2, E2b, E3, E3b, E4, E4b, E4c, E5, LR1, LR3 | measured (E4b/E4c/E5 cells REASONED) |
//! | P3 | `reports/loader.rs` `discover_reports_in` | `    for rd in layout::report_dirs(root) {` → `    for rd in layout::report_dirs(root).into_iter().filter(\|rd\| rd.dir.parent().and_then(Path::parent) == Some(root)) {` | — | every E*, LR1-LR3, H3 | measured (E4b/E4c/E5 cells REASONED) |
//! | P4 | `lab/equity_loader.rs` `discover_reports` | `        .flat_map(\|d\| layout::backtest_files(&d))` → `        .filter(\|d\| d.parent().and_then(\|p\| p.parent()) == Some(report_root)).flat_map(\|d\| layout::backtest_files(&d))` | E1, E2b, E3, E3b, E4, E4c, LR1, LR3 | E1b, E2, E4b, E5, LR2, container-depth-0 Lab tests, H3 | measured (E4b/E4c/E5 cells REASONED) |
//! | P5 | `reports/layout.rs` `child_dirs` | `        .filter(\|p\| p.is_dir() && p.file_name() != Some(OsStr::new(REPORTS_DIR)))` → `        .filter(\|p\| p.is_dir())` | — | every E*, LR1-LR3 | measured |
//! | P6 | `lab/equity_loader.rs` `EquityCache::get_or_load_roots` | `    for root in roots {` → `    for root in roots.iter().rev() {` | `route_overlay_two_root_union_lab_runs_wins_collision`, E2 (4 ≠ 2), E2b (2 ≠ 4) | E1, E1b, E3, E3b, E4, E4b, E4c, E5, LR1-LR3 (lab-runs/ is absent there), single-root tests | measured (E4b/E4c/E5 cells REASONED) |
//! | P7 | `test_support.rs` `evidence_oracle::git_listed_files` | `            .args(["ls-files", "-z", "--cached", "--others", "--", "evidence"])` → `            .args(["ls-files", "-z", "--cached", "--others", "--", "evidence/v5-latency-slippage-sim-v0.2.0-anchor-migration"])` | E3 (0 momentum reports listed) | every other E*, LR1-LR3 | REASONED |
//! | P8 | `reports/loader.rs` `discover_reports_in` | `            .then_with(\|\| a.file_stem.cmp(&b.file_stem))` → `            .then_with(\|\| std::cmp::Ordering::Equal)` | — | every E*, LR1-LR3 | measured |
//! | P9 | `reports/loader.rs` `discover_reports_in` | `                slug: rd.slug.clone(),` → `                slug: smol_str::SmolStr::new(rd.dir.to_string_lossy()),` | — | every E*, LR1-LR3 | measured |
//! | existing pin | `lab/equity_loader.rs` `default_report_roots` | `    vec![default_lab_runs_root(), default_evidence_root()]` → `    vec![default_evidence_root(), default_lab_runs_root()]` | `default_roots_order_lab_runs_first` | the rest | measured |
//! | P10 | `lab/equity_loader.rs` `discover_reports` | `        .flat_map(\|d\| layout::backtest_files(&d))` → `        .flat_map(\|d\| layout::backtest_files(&d)).collect::<std::collections::BTreeSet<PathBuf>>().into_iter()` | E4b (`root-a0` resolves the container-depth-1 report: 5 points, not 3) | every other E*, LR1-LR3 (18 of 18 in this file), H3 | measured |
//! | P11 | `reports/layout.rs` `is_slug_component` | `        (Some(Component::Normal(c)), None) if c == OsStr::new(slug) && c != OsStr::new(REPORTS_DIR)` → `        (Some(Component::Normal(c)), None) if c != OsStr::new(REPORTS_DIR)` | — (LY3, a layout test, is the only red) | every E*, LR1-LR3 (18 of 18) | measured |
//! | P12 | `test_support.rs` `evidence_oracle::git_listed_files` | `            .args(["ls-files", "-z", "--cached", "--others", "--", "evidence"])` → `            .args(["ls-files", "-z", "--cached", "--", "evidence"])` | — (OR1, an oracle test, is the only red) | every E* (E3 reads the same listing on a checkout with 0 untracked files under `evidence/`); LR1-LR3 do not use the oracle | measured |
//! | O3 | `lab/equity_loader.rs` `discover_reports` | `    layout::report_dirs_for_slug(report_root, slug)` → `    layout::report_dirs_for_slug(report_root, slug).into_iter().take(1).collect::<Vec<_>>()` | E4c (left 4, right 3) | every other E*, LR1-LR3 (18 of 18) | measured (second falsifier) |
//! | O9 | `reports/loader.rs` `discover_reports_in` | `            .cmp(&b.slug)` → `            .cmp(&a.slug)` | — | every E*, LR1-LR3 | measured, lib (second falsifier) |
//! | O6 | `reports/layout.rs` `report_dirs` | `            if dir.is_dir() {` → `            if dir.exists() {` | — (`report_dirs_for_slug` keeps its own `is_dir` filter) | every E* | measured, lib (second falsifier); LR cells REASONED |
//! | O5b | `reports/loader.rs` `discover_reports_in` AND `test_support.rs` `evidence_oracle::git_listed_files` | `    for rd in layout::report_dirs(root) {` → `    for rd in layout::report_dirs(root).into_iter().filter(\|rd\| !rd.dir.starts_with(root.join("v3"))) {` AND `            .args(["ls-files", "-z", "--cached", "--others", "--", "evidence"])` → `            .args(["ls-files", "-z", "--cached", "--others", "--", "evidence/v5-latency-slippage-sim-v0.2.0-anchor-migration", "evidence/v1"])` | — | every E* (E3's momentum report is under `evidence/v1`) | measured, lib (second falsifier); LR cells REASONED |
//! | Q3 | `reports/layout.rs` `backtest_files` | `    out.sort_unstable();` → `    out.sort_unstable_by(\|a, b\| b.cmp(a));` | — (no Lab test's winning dir holds two tied reports; LY4, a layout test, is the red) | every E*, LR1-LR3 (18 of 18), H3 | measured (second falsifier, in the `sort_by` form) |
//! | Q6 | `reports/loader.rs` `discover_reports_in` | `            .then_with(\|\| a.path.cmp(&b.path))` → `            .then_with(\|\| std::cmp::Ordering::Equal)` | — (the Lab never calls `discover_reports_in`) | every E*, LR1-LR3 | REASONED |
//! | Q8 | `reports/layout.rs` `is_slug_component` | `        (Some(Component::Normal(c)), None) if c == OsStr::new(slug) && c != OsStr::new(REPORTS_DIR)` → `        (Some(Component::Normal(c)), None) if c == OsStr::new(slug)` | — (no Lab slug is `reports`) | every E*, LR1-LR3 | REASONED |
//!
//! ## bug-log #207 — the equity gutter is reserved iff the overlay draws (PHASE 4c)
//!
//! The chart widened its right gutter for the equity axis whenever an overlay
//! was PRESENT, but draws the overlay only when a sample lies inside the bar
//! window. A start/end-only report (ordinal `(0, 1)` timestamps) or a
//! persisted run from another period therefore left an empty gutter beside
//! the bars (and the Lab's volume histogram, which mirrors the gutter, was
//! narrowed with it). One windowed range now decides it:
//! `widgets::chart::bar_window_equity_range` (`compute_equity_range` over the
//! bar window `[first open, last close]`). `ChartProgram::draw` computes it
//! ONCE per frame (`ChartProgram::equity_range`) and that one value decides
//! the gutter and scales Pass 5 / the bar-less path; both hit-test arms of
//! `ChartProgram::update` (`ChartProgram::plot_rect`) and the histogram
//! alignment in `screens/lab.rs` (`widgets::chart::equity_axis_shown`, with
//! `active.is_some()`, exactly what the chart is given) apply the same rule.
//! The start/end-only series is not anchored in time: a report records
//! neither its first nor its last bar time.
//!
//! - **LR-207a** [`start_end_overlay_outside_bar_window_reserves_no_gutter`]
//!   — the committed start/end-only momentum report, copied byte-for-byte
//!   into a temp root, over the 60 bars: the frame is byte-identical to the
//!   bars-only frame. RED before the fix (25,778 px differed). Negative
//!   control in the same test: LR-207b's in-window series over the same bars
//!   changes the frame (> 0 px), so the identity cannot hold because the
//!   harness dropped the overlay. PNGs `/tmp/lab_207_bars_only.png`,
//!   `/tmp/lab_207_out_of_window.png`.
//! - **LR-207b** [`overlay_inside_bar_window_reserves_the_gutter`] — an
//!   in-window per-bar report moves the plot's right edge left by the derived
//!   gutter widening (`ui::test_support::EQUITY_GUTTER_WIDENING_PX` =
//!   `AXIS_GUTTER_EQUITY_PX` 56.0 − `AXIS_GUTTER_RIGHT_PX` 16.0 = 40 px),
//!   band `[34, 44]`. PNG `/tmp/lab_207_in_window.png`.
//! - **LR-207c** [`start_end_overlay_without_bars_still_draws`] — without bars
//!   the start/end-only segment still draws (≥ `OVERLAY_DREW_MIN` ACCENT px);
//!   negative control: no bars and no series draws none. PNG
//!   `/tmp/lab_207_no_bars_start_end.png`.
//! - **LR-207e** [`compare_only_overlay_without_bars_draws_its_curve`] — the
//!   Compare screen's one-slot case (slot 0 unresolved, slot 1 resolved): no
//!   bars, no primary, one compare series. Its `ACCENT_2` curve draws (≥
//!   `OVERLAY_DREW_MIN` px); negative control: no series draws none. A chart
//!   that dropped its compare series from the range (probe P7-6) hides it.
//!   PNG `/tmp/lab_207_no_bars_compare_only.png`.
//! - **LR3** (above) — LR-207a's identity on the operator path: the committed
//!   report resolved by `route_equity_overlay` over the production roots.
//! - **LR-207d** — `lab_buy_sell_overlay_render.rs`
//!   `lab_start_end_only_overlay_keeps_the_chart_full_width_and_aligned`
//!   (macOS): the full Lab screen with a pre-warmed equity cache; scene A
//!   (the committed start/end-only report) minus scene B (an in-window
//!   report) right-edge shift in `[34, 44]`; each seeded fill's volume-bar
//!   shift A − B equal to the DERIVED `i / (n − 1) · EQUITY_GUTTER_WIDENING_PX`
//!   (`x_for_index` geometry; 8.1 / 16.3 / 24.4 / 32.5 px for bars
//!   12 / 24 / 36 / 48 of 60) within `HISTO_SHIFT_TOL_PX` 4; and in both
//!   scenes exactly one triangle and one volume bar per seeded fill
//!   (`synthetic_fills_for`, 2 buys and 2 sells), each volume bar under its
//!   triangle (`ALIGN_TOL_PX`). The chart legend's `UP_500`/`DOWN_500`
//!   swatches are excluded by the legend's derived box (plot right edge −
//!   `space::M` − `LEGEND_CARD_WIDTH_PX`), which a positive control checks
//!   holds both swatches. PNGs (with `UI_DUMP_PNG`)
//!   `/tmp/lab_207_full_out_of_window.png`, `/tmp/lab_207_full_in_window.png`.
//! - **C1-C10** — `widgets::chart` lib tests: C1
//!   `equity_axis_hidden_when_no_sample_in_bar_window`, C2
//!   `equity_axis_shown_when_a_sample_is_in_bar_window` (its `one_inside`
//!   half pins TODAY's at-least-one-in-window-sample gate, which bug-log
//!   #211's fix raises to two), C3
//!   `equity_axis_shown_without_bars_for_any_sample`, C4
//!   `equity_axis_hidden_without_any_series`, C5
//!   `equity_axis_follows_the_window_for_compare_only_series` (C5a: an
//!   in-window compare series shows the axis; C5b: an out-of-window one does
//!   not), C6 `hit_test_uses_the_plot_rect_draw_paints_into` (a fill on bar 59
//!   is hit at its no-gutter centroid and missed at its gutter centroid under
//!   an out-of-window overlay, and the reverse under an in-window one, through
//!   both the click and the hover arm), C7
//!   `compare_only_program_reserves_the_gutter_iff_its_series_is_in_window`
//!   (a compare-only `ChartProgram` with bars: C7a in-window → axis shown and
//!   the equity plot rect; C7b out-of-window → neither), C8
//!   `bar_less_compare_only_program_shows_the_axis` (the Compare screen with
//!   slot 0 unresolved: no bars, compare only → the axis; negative half: no
//!   series → none), C9 `bar_window_is_first_open_to_last_close_inclusive` (a
//!   lone sample at the last close / first open → shown, at close + 1 ms /
//!   open − 1 ms → hidden, on bars that span a minute), C10
//!   `equity_range_spans_only_in_window_samples` (a mixed series' range equals
//!   the range derived from its in-window values and `RANGE_PAD_FRACTION`;
//!   the extreme moved INTO the window reaches the top).
//!
//! Every #207 pixel test prints `#207 pixel margin …` (its measured value and
//! threshold) on stderr; run with `--nocapture`.
//!
//! ### T-T4 probe table (bug-log #207)
//!
//! Same protocol as the #205 table above; paths under `crates/ui/src/`. Each
//! OLD occurs exactly once in its file. "measured" = applied and run on
//! 2026-11-16; "REASONED" = derived from the code. "lib" = `cargo test -p ui
//! --lib` (686 tests); "pixel" = `live_equity_render` (this file, 22) and
//! `lab_buy_sell_overlay_render.rs` (4). The compute-once change
//! (`ChartProgram::equity_range`) and the `active.is_some()` parity moved the
//! OLD of P7-1, P7-2 and P7-4; their pixel cells were measured on the earlier
//! OLD (P7-1 `        let has_equity = self.equity_axis_shown();`, P7-2
//! `    compute_equity_range(equity, compare, start, end).is_some()`, P7-4 the
//! same call without `active.is_some() &&`) and are REASONED to carry over.
//! The earlier probe that set Pass 5's own window to `(None, None)` survived
//! every test; it has no line left to mutate now (Pass 5 reads the range
//! `draw` computed once), and P7-8 is its analogue.
//!
//! | Probe | File / symbol | OLD → NEW | RED | GREEN | Measured |
//! |---|---|---|---|---|---|
//! | P7-1 draw leg | `widgets/chart.rs` `ChartProgram::draw` | `        let has_equity = equity_range.is_some();` → `        let has_equity = self.equity.is_some() \|\| !self.compare.is_empty();` | pixel: LR-207a and LR3 (25,778 px differ from the bars-only frame: the empty gutter), LR-207d (scene A's alignment: the chart reserves the gutter, the histogram does not) | lib 686 of 686 (C1-C10 test the gate and the range, not `draw`); LR-207b, LR-207c and the other 20 of 22 in this file, the other 3 of 4 in `lab_buy_sell_overlay_render.rs` | lib measured; pixel measured on the earlier OLD |
//! | P7-2 the shared window | `widgets/chart.rs` `bar_window_equity_range` | `    compute_equity_range(equity, compare, start, end)` → `    compute_equity_range(equity, compare, None, None)` (two unused-variable warnings, `start` and `end`) | lib: C1, C5 (C5b), C6 (its out-of-window half), C7 (C7b), C9 (its close + 1 ms half), C10; pixel: LR-207a, LR3, LR-207d (shift 0) | lib 680 of 686 (C2, C3, C4, C8); LR-207b, LR-207c, LR-207d's alignment (the histogram asks the same range) | lib measured; pixel measured on the earlier OLD |
//! | P7-3 hit-test leg | `widgets/chart.rs` `ChartProgram::plot_rect` | `        if self.equity_axis_shown() {` → `        if self.equity.is_some() \|\| !self.compare.is_empty() {` | lib: C6 (its out-of-window half), C7 (C7b: the plot rect) | lib 684 of 686; every pixel test (`draw` keeps its own rule) | lib measured; pixel measured (falsifier) |
//! | P7-4 histogram leg | `screens/lab.rs` `view` | `        active.is_some() && chart::equity_axis_shown(&bars, equity_overlay.as_ref(), &[]);` → `        equity_overlay.is_some();` | pixel: LR-207d only (scene A: `SELL[1]` volume bar x=1022 vs its triangle x=1042, residual 20 px > `ALIGN_TOL_PX` 16; the histogram reserves the gutter, the chart does not). Its derived histogram shift A − B is then 0 for every fill, 8.1 to 32.5 px off (REASONED; asserted after the alignment) | lib (682 of 682 measured before C7-C10 existed; REASONED 686 of 686 now: no lib test reaches `lab::view`'s histogram flag); this file (22 of 22); the other 3 of 4 in `lab_buy_sell_overlay_render.rs` | measured on the earlier OLD |
//! | P7-5 the window is live | `widgets/chart.rs` `bar_window_equity_range` | `    let (start, end) = bar_window_ms(bars);` → `    let (start, end) = (Some(i64::MAX), Some(i64::MAX));` (an unused-variable warning, `bars`) | lib: C2, C3, C5 (C5a), C6 (its in-window half), C7 (C7a), C8, C9 (its last-close half), C10 (no range at all); pixel: LR-207b (shift 0), LR-207c (the bar-less segment is gone), LR-207d (shift 0), LR1, T7(a) `lab_curve_hydrated_from_lab_runs_report_renders`, T7(b) `compare_two_run_overlay_renders_both_series`, `compare_screen_two_run_overlay_renders_both_series`, `compare_screen_single_run_overlay_draws_no_accent2` (the primary ACCENT curve is gone: 0 px < 120); REASONED, added since: LR-207a (its in-window negative control changes 0 px) | lib 678 of 686 (C1, C4); LR3, LR-207d's alignment | lib measured; pixel measured (falsifier, LR-207a REASONED) |
//! | P7-6 compare reaches the chart's range | `widgets/chart.rs` `ChartProgram::equity_range` | `        bar_window_equity_range(&self.bars, self.equity.as_ref(), &self.compare)` → `        bar_window_equity_range(&self.bars, self.equity.as_ref(), &[])` | lib: C7 (C7a), C8; pixel: LR-207e (the compare-only curve is gone), T7(b) `compare_two_run_overlay_renders_both_series` and `compare_screen_two_run_overlay_renders_both_series` (the second curve is scaled to the primary's range alone and leaves the plot) | lib 684 of 686 (C1-C6, C9, C10 pass a primary or call the free helper); every other pixel test in this file and `lab_buy_sell_overlay_render.rs` | measured (orchestrator's gate, lib + both pixel files); LR-207e added after the fixer's lib measurement (the earlier form of this probe was killed only by the corpus-gated `lab_binance_render.rs` `binance_sourced_equity_curve_rasterizes`, which skips without `data/binance`) |
//! | P7-7 the window ends at the last CLOSE | `widgets/chart.rs` `bar_window_ms` | `        bars.last().map(\|b\| b.close_ts.unix_millis()),` → `        bars.last().map(\|b\| b.open_ts.unix_millis()),` | lib: C9 (a sample at the last close is hidden) | lib 685 of 686 (the `bars_207` bars open and close at the same instant); pixel REASONED green: no fixture sample lies in (last open, last close] (`lab_runs_report_at` stops at bar 50) | lib measured; pixel REASONED |
//! | P7-8 the chart's range keeps the window | `widgets/chart.rs` `ChartProgram::equity_range` | `        bar_window_equity_range(&self.bars, self.equity.as_ref(), &self.compare)` → `        compute_equity_range(self.equity.as_ref(), &self.compare, None, None)` | lib: C6 (its out-of-window half), C7 (C7b), C10 (range (−395000, 10495000) vs the derived (99975, 100525)); pixel REASONED: LR-207a and LR3 (the out-of-window overlay reserves the gutter and strokes its clamped edge), LR-207d (scene A: the chart reserves the gutter, the histogram's free helper does not) | lib 683 of 686 (C1-C5, C8, C9 use the free helper or no bars) | lib measured; pixel REASONED |
//!
//! ## bug-log #206 — Compare's scan walks the ONE layout (PHASE 4d)
//!
//! Compare's `compare::cache::scan_one_root` walked `<root>/<x>/reports/`
//! only (container depth 0): on `evidence/` it considered 108 of the 197
//! in-contract reports git lists (108 at container depth 0, 89 at container
//! depth 1; CS5's tally on the unfixed scan, measured 2026-11-21). It now
//! walks `ui::reports::layout` (ADR-0055 D8, container depth ≤ 1); inside
//! one root the container-depth-0 report claims an identical filename first
//! and keeps a tuple on an equal `generated:`. No shipped build shows the
//! change: production renders no matrix (it never sets `strategies_config`);
//! the fixtures build's Compare screen body (toolbar, K7 subtitle, matrix,
//! overlay panel; `fake_strategies_config`, the config the `cockpit` bin
//! sets), rendered by a one-off harness, gives byte-identical PNGs before and
//! after the fix (measured 2026-11-21 over the real `[lab-runs/, evidence/]`:
//! the same 66 cell keys both times, 62 of them multi-symbol, the K7
//! subtitle's input; one winner moves, at `(sma_crossover, BTCUSDT)`, a row
//! the fixtures config does not have),
//! and no file under `evidence/` is a companion equity CSV, so no committed
//! report draws an overlay. The proof is hermetic:
//!
//! - **CR1** [`compare_screen_overlay_renders_runs_from_a_container_depth_1_root`]
//!   — the two-run scene at `<tmp>/evidence/v1/<slug>/reports/`, scanned by
//!   `scan_report_roots` over `<tmp>/evidence`, both runs selected on the
//!   real Compare screen: `ACCENT` and `ACCENT_2` each ≥ `OVERLAY_DREW_MIN`
//!   in the chart band. RED before the fix at its precondition (run A's cell
//!   had 0 points: nothing was scanned). PNG `/tmp/compare_206_depth1.png`.
//! - **CR2** [`compare_screen_overlay_draws_nothing_from_a_container_depth_2_root`]
//!   — the same scene at `<tmp>/evidence/v9/x/<slug>/reports/` (container
//!   depth 2): the files exist, neither cell is scanned, and neither colour
//!   reaches `OVERLAY_DREW_MIN`. PNG `/tmp/compare_206_depth2.png`.
//! - **CS1-CS5** — `compare::cache` lib tests: CS1
//!   `scan_admits_a_container_depth_1_report` (and its `source_report_path`
//!   keeps the container), CS2 `scan_skips_a_container_depth_2_report`, CS3
//!   `scan_tally_over_the_layout_tree_counts_exactly_its_listed_rows` (the
//!   #205 fixture TREE: discovered = its 6 listed rows; 5 distinct names
//!   `MalformedFrontmatter`, 1 `DuplicateFilename`; its `backtest-dir.md`
//!   directory is no candidate), CS4a
//!   `duplicate_filename_inside_a_root_goes_to_container_depth_0`, CS4b
//!   `equal_generated_inside_a_root_goes_to_container_depth_0` (CS4a and CS4b
//!   run once with a container sorting before the slug, `a0`, and once after
//!   it, `zz`), CS4c `equal_generated_inside_one_dir_goes_to_the_first_by_path`,
//!   CS5 `compare_scan_discovers_every_git_listed_report` (live: discovered ≥
//!   the git-listed in-contract reports; it prints the tally), CS6
//!   `scan_skips_an_uppercase_md_report` (a `….MD` name is no candidate; the
//!   same bytes named `….md` in another root are admitted), CS7
//!   `scan_report_roots_identical_filename_lab_runs_wins` (the later root's
//!   copy is strictly newer, so only the cross-root filename rule keeps the
//!   `lab-runs/` copy; tally 2 discovered, 1 admitted, 1 `DuplicateFilename`).
//! - **ST1, ST2** — `state` lib tests: `lab_run_completed_ok_resets_compare_index`
//!   and `lab_run_completed_err_resets_compare_index_too`. A completed Lab run
//!   clears Compare's cold-boot tag on Ok AND Err (#208's reason: a run that
//!   failed on its CSV write can leave a readable `.md`); `LabRunProgressDone`
//!   keeps it (the control). AD-10: no shipped build can show the reset. The
//!   one build that draws a matrix, the `fixtures` `cockpit`, completes every
//!   run with `Ok`, whose reset predates this fix; `cockpit_live`, the only
//!   bin that completes with `Err`, never sets `strategies_config`; and the
//!   scan reads `default_report_roots()`, with no root seam. So both tests set
//!   the tag by hand.
//!
//! **Before the fix** (measured 2026-11-21: HEAD `1f4ec79481d4`'s production
//! `compare/cache.rs`, everything above its `#[cfg(test)]`, under the new
//! tests module; `layout.rs` and `state.rs` stay fixed; the lib run): RED CS1 (discovered 0 ≠ 1), CS3 (4 ≠ 6: 3
//! `MalformedFrontmatter` and 1 `Unreadable`; REASONED, the three are the two
//! container-depth-0 rows and `reports/reports`, the one is the
//! `backtest-dir.md` directory), CS4a and CS4b (1 ≠ 2), CS5 (108 < 197), CS6
//! (discovered 1, admitted 1 ≠ 0, 0: the old scan matched the extension
//! case-insensitively); GREEN CS2, CS4c, CS7 and the other 693 lib tests. CR1
//! was RED at its precondition and CR2 GREEN (the fixer's run of this file
//! before the fix). ST1 and ST2 have no before-state in that splice: P6-9
//! below moves the reset back into the `Ok` arm, which turns ST2 RED and
//! leaves ST1 GREEN.
//!
//! CR1 and CR2 print `#206 pixel margin …` on stderr, CS5 its tally; run with
//! `--nocapture`.
//!
//! ### T-T4 probe table (bug-log #206)
//!
//! Same protocol as the #205 table above; paths under `crates/ui/src/`. Each
//! OLD occurs exactly once in its file. P6-3′'s OLD starts with a newline
//! (`\n`): the same line without it also occurs, indented deeper, in the
//! layout test helper `expected_dirs`, and both copies differ only in their
//! indentation, so no single-line OLD selects `report_dirs`'s sort. Test keys
//! LY*, D*, E*, R*, LR* are `reports_populated_curve_render.rs`'s legend.
//! Every row was measured on 2026-11-21: applied, run, restored, strictly one
//! probe at a time. Each probe ran the lib, this file and
//! `session_log_replays_66_a4.rs` (5 of 5 under every probe), plus "RPC" =
//! `reports_populated_curve_render.rs` (13 tests) for the probes that touch
//! `reports/layout.rs`. "lib" = `cargo test -p ui --lib` (702 tests); "this
//! file" = 25 tests.
//!
//! | Probe | File / symbol | OLD → NEW | RED | GREEN | Measured |
//! |---|---|---|---|---|---|
//! | P6-1 container depth 0 only | `compare/cache.rs` `scan_one_root` | `    for rd in layout::report_dirs(root) {` → `    for rd in layout::report_dirs(root).into_iter().filter(\|rd\| rd.container_depth == 0) {` | lib: CS1 (discovered 0 ≠ 1), CS3 (2 ≠ 6: 2 `MalformedFrontmatter`, the container-depth-0 rows only), CS4a and CS4b (1 ≠ 2, at their `a0` half), CS5 (108 < 197); pixel: CR1 (its precondition: 0 points) | lib 697 of 702 (CS2, CS4c, CS6, CS7, ST1, ST2 and every existing test); this file 24 of 25 (CR2 and every existing test) | measured |
//! | P6-2 the bound is 0 | `reports/layout.rs` `MAX_CONTAINER_DEPTH` | `pub const MAX_CONTAINER_DEPTH: usize = 1;` → `pub const MAX_CONTAINER_DEPTH: usize = 0;` | lib (22): P6-1's five, LY1, LY3, LY5, D1, D2, D3, E1, E2b, E3, E3b, E4, E4c, E5, #208's `cached_miss_is_not_re_read_until_clear_under_evidence` and `clear_replaces_a_stale_committed_hit_with_the_newer_lab_runs_report`, and #208's two `state` tests (`lab_run_completed_ok_clears_equity_cache_hits_and_misses`, `lab_run_completed_err_clears_equity_cache_too`); this file (3): CR1, LR1, LR3; RPC (7): R1-R5, R6, R7a | lib 680 of 702 (CS2, CS4c, CS6, CS7, ST1, ST2 and the rest); this file 22 of 25 (CR2, LR2 and the rest); RPC 6 of 13 | measured |
//! | P6-2′ the bound is 2 | same | `pub const MAX_CONTAINER_DEPTH: usize = 1;` → `pub const MAX_CONTAINER_DEPTH: usize = 2;` | lib (7): CS2 (discovered 1 ≠ 0), CS3 (7 ≠ 6: the TREE's container-depth-2 row), E1b, LY1, LY3, LY5, D1; this file (2): CR2 (its precondition: the cell is scanned), LR2; RPC (1): R7b | lib 695 of 702 (CS1, CS4a, CS4b, CS4c, CS5: git lists no container-depth-2 report and CS5 asserts ≥); this file 23 of 25 (CR1, LR1, LR3); RPC 12 of 13 | measured |
//! | P6-3 reversed dir order | `compare/cache.rs` `scan_one_root` | `    for rd in layout::report_dirs(root) {` → `    for rd in layout::report_dirs(root).into_iter().rev() {` | lib: CS4a, CS4b (the container-depth-1 copy is met first: 0.99, not 0.11) | lib 700 of 702 (CS3: its counts do not depend on which copy is first); this file 25 of 25 | measured |
//! | P6-3′ full-path dir order | `reports/layout.rs` `report_dirs` | `\n    out.sort_by(\|a, b\| (a.container_depth, &a.dir).cmp(&(b.container_depth, &b.dir)));` → `\n    out.sort_by(\|a, b\| a.dir.cmp(&b.dir));` | lib: CS4a, CS4b (at their `a0` half; the `zz` half alone would pass), LY1, LY3 | lib 698 of 702 (the Lab's `report_dirs_for_slug` and the Reports loader's `discover_reports_in` sort on their own); this file 25 of 25; RPC 13 of 13 | measured |
//! | P6-4 `reports` as a container | `reports/layout.rs` `child_dirs` | `        .filter(\|p\| p.is_dir() && p.file_name() != Some(OsStr::new(REPORTS_DIR)))` → `        .filter(\|p\| p.is_dir())` | lib: CS3 (8 ≠ 6: the TREE's `reports/reports` and `reports/inner/reports` rows), LY1, LY3, D1 | lib 698 of 702 (CS5: the probe can only add dirs, and it asserts ≥); this file 25 of 25; RPC 13 of 13 | measured |
//! | P6-5 reversed file order | `reports/layout.rs` `backtest_files` | `    out.sort_unstable();` → `    out.sort_unstable_by(\|a, b\| b.cmp(a));` | lib: CS4c (0.99, not 0.11), LY4 | lib 700 of 702 (CS4a and CS4b put their two reports in different dirs); this file 25 of 25; RPC 13 of 13 | measured |
//! | P6-6 an equal stamp replaces | `compare/cache.rs` `scan_one_root` | `                    cell.generated_at.as_str() <= existing.generated_at.as_str()` → `                    cell.generated_at.as_str() < existing.generated_at.as_str()` | lib: CS4b, CS4c (0.99, not 0.11) | lib 700 of 702 (CS4a: its duplicate is skipped before it is parsed); this file 25 of 25 | measured |
//! | P6-7 the scan lists its own files | `compare/cache.rs` `scan_one_root` | `        for report_path in layout::backtest_files(&rd.dir) {` → an inline `fs::read_dir(&rd.dir)` walk that keeps `backtest-*` files whose extension `eq_ignore_ascii_case("md")`, sorted (HEAD's file test) | lib: CS6 (discovered 1, admitted 1 ≠ 0, 0) | lib 701 of 702 (CS4c: the inline walk sorts by path too); this file 25 of 25 | measured |
//! | P6-8 roots forget each other's names | `compare/cache.rs` `scan_report_roots` | `        scan_one_root(root, &mut cache, &mut seen_filenames, &mut tally);` → `        scan_one_root(root, &mut cache, &mut BTreeSet::new(), &mut tally);` | lib: CS7 (admitted 2 ≠ 1, no `DuplicateFilename`) | lib 701 of 702 (CS4a: its duplicate is inside one root); this file 25 of 25 | measured |
//! | P6-9 the reset is `Ok`-only again | `state.rs` `Message::LabRunCompleted` arm | two replaces: drop `            model.compare_screen_state.last_indexed_at = None;\n` before `            // Bug #54`, and add it after `                    model.lab_state.last_run_notice = None;\n` at the end of the `Ok` arm | lib: ST2 | lib 701 of 702 (ST1: the `Ok` arm still resets); this file 25 of 25 | measured |
//!
//! ## bug-log #214 — a long Compare row label stays inside its column (PHASE 4e)
//!
//! `widgets/matrix.rs` gave every row label a fixed 120 px column: a 104 × 48
//! px content box at `text::SMALL`, the default word wrap, and no clip. An id
//! has no space to break at, so `btc_bbands_mean_revert` (142.3 px, MEASURED
//! from the embedded Inter's advance widths) stayed on one line and drew over
//! its first value cell's `-32.89`. The label now wraps per glyph
//! (`Wrapping::WordOrGlyph`) inside an inner container that clips to the
//! content box, and the column width is one constant, `ROW_LABEL_W`, which the
//! header row's corner cell shares. The tests derive their geometry from
//! `test_support::{COMPARE_ROW_LABEL_W, COMPARE_ROW_LABEL_PADDING,
//! COMPARE_CELL_H}` and the screen body's `space::M` padding: the label column
//! ends at x = 132, its content box ends at x = 124 and is 48 px tall, and a
//! `SMALL` line is 15.6 px. Each test renders the real Compare screen body with
//! `fake_strategies_config()` (the config the fixtures `cockpit` sets), its
//! `btc_bbands_mean_revert` row renamed, against the reference frame S, the
//! same scene with the row renamed `btc_bb` (one line):
//!
//! - **ML1** [`compare_matrix_long_row_label_leaves_the_value_columns_untouched`]
//!   — L (`btc_bbands_mean_revert`) and S differ in 0 px at x ≥ 132.
//!   Preconditions: `-32.89` drew at x ≥ 132 in both frames (≥
//!   `KPI_DREW_MIN` `DOWN_500`-class px; 46 and 46; it is the only `DOWN_500`
//!   text on screen, so a frame without that cell has 0); the long label drew
//!   (L adds ≥ 16 ink px over S inside the label column, one per added glyph;
//!   400). PNGs `/tmp/compare_214_long.png`, `/tmp/compare_214_short.png`.
//! - **ML2** [`compare_matrix_long_row_label_wraps_onto_a_second_line`] — the
//!   L-vs-S diff box is taller than one line: 26 px > 15.6.
//! - **ML3** [`compare_matrix_overlong_row_label_stays_inside_its_content_box`]
//!   — a 68-character id against S. Precondition, derived: iced's own
//!   paragraph at the content width and unbounded height lays the id out on 5
//!   lines (78 px), more than the 3 the 48 px box holds, so the clip has a
//!   line to cut (the fourth, laid out because its top is inside the buffer,
//!   its ink about 2 px below the box). Assertions: the diff box ends at or
//!   left of x = 124 (123); it is at most 48 px tall (41), measured from the
//!   first line's ink top, about 3 px below the box's top, so the label may
//!   reach at most 3 px into the 6 px bottom padding but never leaves its
//!   cell; and it is taller than two lines (31.2), so the widget drew the id
//!   wrapped. PNG `/tmp/compare_214_overlong.png`. `diff_bbox` returns `None`
//!   on an empty diff, and ML2 and ML3 fail on it.
//!
//! **Before the fix** (P214-0 below, measured 2026-11-24; its frames of the L
//! and 68-character scenes are byte-identical to those of HEAD `0fa64fbc`'s
//! row-header code): RED ML1 (151 px differ at x ≥ 132; 13 `FG_2`-class px
//! there in L, 0 in S), ML2 (one line: the diff box is 10 px tall), ML3 (the
//! diff box runs to x = 438 and is 13 px tall); GREEN this file's other 25
//! tests and the 12 `compare__*` byte baselines.
//!
//! Those 12 baselines in `visual_snapshots.rs` (macOS, ADR-0057) draw one-line
//! ids only (the widest, `top10_momentum`, is 101.1 px of 104), and they pass
//! on the fix with no file under `tests/visual-baselines/` moved: a one-line
//! label renders byte-identically.
//!
//! ML1-ML3 print `#214 pixel margin …` on stderr; run with `--nocapture`.
//!
//! ### T-T4 probe table (bug-log #214)
//!
//! Same protocol as the tables above. P214-0 to P214-5 edit
//! `crates/ui/src/widgets/matrix.rs` and P214-6 this file; every OLD occurs
//! exactly once in its file (P214-3's second edit and P214-4 carry the line
//! after `.width(Length::Fixed(ROW_LABEL_W))`, which occurs twice). Measured
//! 2026-11-24 after ML3's derived precondition landed: applied, run, both
//! files restored byte-identical (`cmp`), strictly one probe at a time. Every
//! probe ran this file (28 tests) and `cargo test -p ui --test
//! visual_snapshots compare__` (12 tests, "baselines").
//!
//! | Probe | OLD → NEW | RED | GREEN | Measured |
//! |---|---|---|---|---|
//! | P214-0 HEAD's behaviour | P214-1 and P214-2 together | ML1 (151 px at x ≥ 132), ML2 (10 px tall), ML3 (right edge 438, 13 px tall) | this file 25 of 28; baselines 12 of 12 | measured |
//! | P214-1 word wrap | `.wrapping(iced::widget::text::Wrapping::WordOrGlyph)` → `.wrapping(iced::widget::text::Wrapping::Word)` | ML2 (10 px tall), ML3 (10 px tall, not past two lines) | ML1 (the clip holds the one line: 0 px at x ≥ 132; ML3's right edge is 124 ≤ 124); this file 26 of 28; baselines 12 of 12 | measured |
//! | P214-2 no clip | `.clip(true),` → `.clip(false),` | ML3 (59 px tall > 48: a fourth line draws below the cell) | ML1, ML2 (26 px); this file 27 of 28; baselines 12 of 12 | measured |
//! | P214-3 the clip on the outer container | `.clip(true),` → `.clip(false),`, and `.width(Length::Fixed(ROW_LABEL_W))\n            .height(Length::Fixed(CELL_MIN_H))` → the same plus `\n            .clip(true)` | ML3 (51 px tall > 48: the fourth line draws into the bottom padding; a thin 3 px margin, since ML3's height runs from the first line's ink top, about 3 px below the box) | ML1, ML2; this file 27 of 28; baselines 12 of 12 | measured |
//! | P214-4 the corner cell drifts | `.width(Length::Fixed(ROW_LABEL_W))\n            .height(Length::Fixed(24.0))` → `.width(Length::Fixed(ROW_LABEL_W - 20.0))\n            .height(Length::Fixed(24.0))` | baselines 12 of 12 (the existing pin) | this file 28 of 28 | measured |
//! | P214-5 a wider column | `pub(crate) const ROW_LABEL_W: f32 = 120.0;` → `pub(crate) const ROW_LABEL_W: f32 = 160.0;` | ML2 (the 144 px box holds 142.3 px: 10 px tall), ML3 at its precondition (the 144 px box holds the id in 3 lines, 46.8 px, so there is nothing to clip; its bounds hold, 43 px, right edge 162 ≤ 164); baselines 12 of 12 | ML1 (0 px at x ≥ 172, derived); this file 26 of 28 | measured |
//! | P214-6 a 3-line id | `const OVERLONG_ID: &str = "btc_bbands_mean_revert_20_2_with_volume_filter_and_atr_stop_v2_tuned";` → `const OVERLONG_ID: &str = "btc_bbands_mean_revert_20_2_with_volume_filter_an";` (its first 49 characters) | ML3 at its precondition (3 lines, 46.8 px: no more than the box holds) | ML3's three bounds hold (41 px, right edge 123); this file 27 of 28; baselines 12 of 12 | measured |
//! | P214-6′ a 3-line id, no clip | P214-6 plus P214-2 | ML3 at its precondition alone | ML3's three bounds hold with no clip (41 px, right edge 123): without the precondition this pair is green, the gap the precondition closes; this file 27 of 28; baselines 12 of 12 | measured |

#![allow(clippy::expect_used, clippy::unwrap_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use rust_decimal::Decimal;
use rust_decimal_macros::dec;
use trading_core::{Money, PnlSnapshot, Timestamp, Usdt};

use ui::state::{Cockpit, Message, Screen, update};
use ui::test_support::program_from_cockpit;
use ui::theme::{ThemeMode, color};

// Floor viewport — the same 1280×720 the headless-emulator smoke uses.
const VIEW_W: u32 = 1280;
const VIEW_H: u32 = 720;
const SCALE: f32 = 1.0;

/// Process-wide UTC-forcing initialiser — thread-safe replacement for
/// `unsafe { std::env::set_var(CHART_FORCE_UTC_ENV, "1") }` in this binary.
/// All three render helpers call this before `iced_test::screenshot` so
/// time-axis labels are UTC regardless of host time zone.
static INIT_UTC: std::sync::Once = std::sync::Once::new();
fn force_utc_once() {
    INIT_UTC.call_once(ui::force_chart_utc_for_tests);
}

/// Curve-region crop window in physical pixels (scale 1.0 → logical == physical).
///
/// The Live screen layout (`screens/live.rs`) stacks
/// `headline → health_strip → equity_curve(240px) → kpi_row → caption →
/// bottom_row` inside the shell (sidebar LEFT = `SIDEBAR_WIDTH_PX` 180px). The
/// equity curve lands below the headline + health strip and is 240px tall.
///
/// We crop a generous band that (a) starts well RIGHT of the 180px sidebar (so
/// the sidebar's active-item `ACCENT` highlight can never leak into the count)
/// and (b) spans the vertical zone where the 240px curve canvas paints. The
/// `accent_pixel_stats` diagnostic below pins the actual `ACCENT` bounding box
/// so this window is verified empirically, not guessed.
const CROP_X: u32 = 220;
const CROP_W: u32 = 980; // 220..1200 — inside the 1280 frame, right of sidebar
const CROP_Y: u32 = 130;
const CROP_H: u32 = 290; // 130..420 — the equity-curve vertical zone (verified
// by diag_accent_bounding_box: a healthy curve's ACCENT bbox lands at
// y≈202..370, x≈271..1199, comfortably inside this window)

/// Per-channel tolerance for an `ACCENT` pixel match. tiny_skia anti-aliases
/// the polyline edges, so an exact byte match would only catch the line's
/// interior. A ±40 window over the 8-bit channels catches the AA ramp without
/// admitting unrelated colors (`PANEL` dark = (28,33,39) is ~150 away from
/// `ACCENT` dark = (111,182,174) on the green channel — nowhere near).
const CHANNEL_TOL: i32 = 40;

/// One realistic Live session: a monotone-non-decreasing wallclock-stamped
/// sequence of `(seconds_since_epoch, equity)` points, the shape the agent's
/// reconciler publishes once per bar. Rises, dips (so Max-DD is non-zero),
/// recovers — a curve with visible vertical movement.
fn session_points() -> Vec<(i64, Decimal)> {
    vec![
        (0, dec!(100000)),
        (60, dec!(100800)),
        (120, dec!(101500)),
        (180, dec!(101200)),
        (240, dec!(100400)), // dip → drawdown
        (300, dec!(101900)),
        (360, dec!(102600)),
        (420, dec!(103100)),
    ]
}

/// Build a `PnlSnapshot` with `as_of` = the given wallclock second and the
/// given total equity. Mirrors the production reconciler publish shape.
fn snap(secs: i64, equity: Decimal) -> PnlSnapshot {
    let as_of = Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(secs));
    PnlSnapshot {
        cash: Money::<Usdt>::from_decimal(equity),
        unrealized: Money::<Usdt>::from_decimal(dec!(0)),
        realized: Money::<Usdt>::from_decimal(dec!(0)),
        total_equity: Money::<Usdt>::from_decimal(equity),
        daily_return: Money::<Usdt>::from_decimal(dec!(0)),
        as_of,
        // This harness exercises the curve's RASTERIZATION over a monotone
        // wallclock sequence; `None` makes the x-coord fall back to `as_of`,
        // preserving the calibrated ACCENT bbox above. The bar_ts→x-axis
        // mapping is asserted at the unit level by
        // `live_equity_curve_plots_bar_ts_not_wallclock` (state.rs).
        bar_ts: None,
    }
}

/// Render the Live screen of the given cockpit and return its RGBA screenshot.
fn render_live(cockpit: Cockpit) -> iced::window::Screenshot {
    // Snapshot-determinism: render the bottom time-axis labels at UTC so this
    // harness is machine-independent (matches the visual-snapshot contract).
    // Thread-safe atomic flag — no env data race.
    force_utc_once();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO)
}

/// `(r, g, b)` 0-255 of an `ACCENT` (dark theme) pixel — the polyline color.
fn accent_rgb() -> (i32, i32, i32) {
    let c = color::ACCENT.current(ThemeMode::Dark);
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

/// Does `(r,g,b)` match `ACCENT` within `CHANNEL_TOL` on every channel?
fn is_accent(r: u8, g: u8, b: u8) -> bool {
    let (ar, ag, ab) = accent_rgb();
    (i32::from(r) - ar).abs() <= CHANNEL_TOL
        && (i32::from(g) - ag).abs() <= CHANNEL_TOL
        && (i32::from(b) - ab).abs() <= CHANNEL_TOL
}

/// Count `ACCENT`-colored pixels inside the curve crop window, and report the
/// bounding box of those pixels (for the diagnostic). The bounding box lets a
/// human verify the crop window actually frames the curve.
struct AccentStats {
    count: usize,
    min_x: u32,
    max_x: u32,
    min_y: u32,
    max_y: u32,
}

fn accent_pixel_stats(shot: &iced::window::Screenshot) -> AccentStats {
    let w = shot.size.width;
    let h = shot.size.height;
    let rgba: &[u8] = &shot.rgba;
    let mut count = 0usize;
    let (mut min_x, mut max_x, mut min_y, mut max_y) = (u32::MAX, 0u32, u32::MAX, 0u32);

    let x0 = CROP_X.min(w);
    let x1 = (CROP_X + CROP_W).min(w);
    let y0 = CROP_Y.min(h);
    let y1 = (CROP_Y + CROP_H).min(h);

    for y in y0..y1 {
        for x in x0..x1 {
            let idx = ((y * w + x) * 4) as usize;
            // Bounds-safety: the buffer is w*h*4; idx+2 is always < len here.
            if idx + 2 >= rgba.len() {
                continue;
            }
            if is_accent(rgba[idx], rgba[idx + 1], rgba[idx + 2]) {
                count += 1;
                min_x = min_x.min(x);
                max_x = max_x.max(x);
                min_y = min_y.min(y);
                max_y = max_y.max(y);
            }
        }
    }
    AccentStats {
        count,
        min_x,
        max_x,
        min_y,
        max_y,
    }
}

// ── Diagnostic: pin the ACCENT bounding box (run with --nocapture) ──────────

/// Diagnostic that renders a healthy Ready curve and prints the `ACCENT`
/// pixel bounding box + count, so the crop window above can be verified
/// empirically rather than guessed. Not an assertion gate — `--nocapture`
/// surfaces the numbers; kept as a `#[test]` so it compiles + runs in CI and
/// documents the observed geometry in the log.
#[test]
fn diag_accent_bounding_box() {
    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    for (secs, eq) in session_points() {
        update(&mut c, Message::PnlRefreshed(snap(secs, eq)));
    }
    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);
    eprintln!(
        "[diag] ACCENT pixels in crop ({CROP_X}..{},{CROP_Y}..{}): count={} bbox=({}..{}, {}..{})",
        CROP_X + CROP_W,
        CROP_Y + CROP_H,
        stats.count,
        stats.min_x,
        stats.max_x,
        stats.min_y,
        stats.max_y,
    );
    eprintln!(
        "[diag] frame size = {}x{}, ACCENT rgb (dark) = {:?}",
        shot.size.width,
        shot.size.height,
        accent_rgb()
    );
}

// ── PHASE 1 — the render-verifiable assertion (the regression guard) ────────

/// Minimum `ACCENT` pixels that constitute a "the curve actually drew" signal.
/// A healthy 8-point session draws ~1200 ACCENT pixels (see
/// `diag_accent_bounding_box`); a degenerate / empty curve draws 0. 200 is a
/// comfortable floor that no AA-noise or single-dot artifact can reach, while
/// staying far below the ~1200 a real polyline produces.
const CURVE_DREW_MIN_ACCENT: usize = 200;

/// Minimum horizontal extent (in px) of the `ACCENT` pixels — a real curve's
/// polyline spans most of the panel width (the healthy case spans ~947px). A
/// single-point dot or a vertical-only artifact would fail this. This is the
/// "sane x-span" assertion: the curve must traverse the time axis, not collapse
/// to one x.
const CURVE_X_SPAN_MIN: u32 = 400;

/// **AC1 (render proof).** Drive a realistic monotone wallclock-stamped
/// `PnlRefreshed` sequence through the production `update` path, render the
/// REAL Live screen, and assert the equity curve actually drew:
///   (i)   the model buffer holds every point (no drops),
///   (ii)  the curve `PanelState` is `Ready`,
///   (iii) the rendered polyline paints a non-trivial number of `ACCENT`
///         pixels (the curve is on-screen, not a blank "no graph" panel), and
///   (iv)  those pixels span a sane horizontal range (the curve traverses the
///         time axis — not a degenerate single-x dot).
///
/// This is the test that would have caught every "the unit test passed but the
/// operator sees no graph" miss this session: it asserts on rasterized pixels,
/// not on `PnlSnapshot` publishes or model state alone.
#[test]
fn live_equity_curve_actually_renders() {
    let points = session_points();
    let n = points.len();

    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    for (secs, eq) in &points {
        update(&mut c, Message::PnlRefreshed(snap(*secs, *eq)));
    }

    // (i) every monotone point landed in the buffer — none dropped.
    assert_eq!(
        c.live_equity_buffer.len(),
        n,
        "all {n} monotone points must be retained (none dropped by the guard)"
    );
    // (ii) the model says Ready.
    assert_eq!(
        c.live_equity_curve.variant_name(),
        "ready",
        "curve must be Ready after a multi-point session"
    );

    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);

    // (iii) the polyline rasterized — a real curve, not a blank panel.
    assert!(
        stats.count >= CURVE_DREW_MIN_ACCENT,
        "equity curve did NOT render: only {} ACCENT pixels in the curve band \
         (expected ≥ {CURVE_DREW_MIN_ACCENT}). This is the 'no graph' bug — the \
         model is Ready but the canvas painted no visible polyline.",
        stats.count,
    );
    // (iv) the curve spans a sane horizontal range (traverses the x/time axis).
    let x_span = stats.max_x.saturating_sub(stats.min_x);
    assert!(
        x_span >= CURVE_X_SPAN_MIN,
        "equity curve x-span is degenerate: ACCENT pixels span only {x_span}px \
         (expected ≥ {CURVE_X_SPAN_MIN}px). A real curve traverses the time axis; \
         a single-point dot or collapsed series would land here.",
    );
}

// ── PHASE 1 — PROVE THE HARNESS CATCHES THE BUG ─────────────────────────────

/// **The regression-guard proof (TODO §1 failing case).** Reconstruct the
/// reverted-I1 bar-time / monotone-guard scenario and assert the harness SEES
/// the empty/degenerate curve.
///
/// ## The bug being reproduced
///
/// I1 (reverted in `40f5de9`) stamped `PnlSnapshot.as_of = bar.close_ts` (the
/// 2023 data time) instead of `Timestamp::now()`. During a FAST replay the
/// agent first publishes a snapshot, then the next bar's `close_ts` can be
/// EARLIER on the wall axis than what the buffer already holds — and
/// `push_live_equity_point`'s monotone guard (`as_of < back.ts` ⇒ drop) drops
/// every such out-of-order point. The buffer never grows past its first point,
/// `EquitySeries::from_points` builds a 1-point series, and the rendered curve
/// collapses to a single dot → the operator's "no graph, no movement".
///
/// Here we feed exactly that pathology: a first point, then a run of snapshots
/// whose `as_of` goes strictly BACKWARDS (descending) — the signature of
/// bar-time stamping after a wallclock-anchored open during fast replay. The
/// guard drops them all.
///
/// ## What this proves
///
/// If a future change re-introduces the bar-time regression (or any change
/// that lets the curve's points get dropped / collapse), THIS test goes red
/// because the rendered curve loses its `ACCENT` polyline. The prior
/// model-only / text-summary tests would NOT catch it — that is the entire
/// reason this harness exists.
#[test]
fn harness_catches_dropped_points_empty_curve() {
    // The failing case: one good open point, then 6 snapshots whose `as_of`
    // marches BACKWARDS (descending), as bar-time stamping produces during a
    // fast replay against a wallclock-anchored first point.
    let dropped_seq: Vec<(i64, Decimal)> = vec![
        (1_000_000, dec!(100000)), // first point lands
        (700, dec!(100800)),       // as_of < back → dropped
        (650, dec!(101500)),       // dropped
        (600, dec!(101200)),       // dropped
        (550, dec!(100400)),       // dropped
        (500, dec!(101900)),       // dropped
        (450, dec!(102600)),       // dropped
    ];

    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    for (secs, eq) in &dropped_seq {
        update(&mut c, Message::PnlRefreshed(snap(*secs, *eq)));
    }

    // Model-level proof of the drop: only the first point survived the guard.
    assert_eq!(
        c.live_equity_buffer.len(),
        1,
        "the monotone guard must drop every backwards-`as_of` point, leaving \
         only the first (this is the bar-time regression's mechanism)"
    );

    // The render-level proof — THE POINT OF THIS HARNESS. A 1-point series is a
    // degenerate curve: the canvas paints (at most) a single dot, NOT a
    // traversing polyline. The harness sees the curve is effectively absent.
    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);

    assert!(
        stats.count < CURVE_DREW_MIN_ACCENT,
        "REGRESSION-GUARD SELF-CHECK FAILED: the dropped-points scenario should \
         render an EMPTY/degenerate curve, but the harness saw {} ACCENT pixels \
         (≥ {CURVE_DREW_MIN_ACCENT}). If this assertion ever fails it means the \
         harness can no longer distinguish a broken curve from a healthy one — \
         fix the harness before trusting it.",
        stats.count,
    );

    // And the x-span proof: with a single retained point there is no horizontal
    // traverse — the curve cannot span the time axis.
    let x_span = stats.max_x.saturating_sub(stats.min_x);
    assert!(
        x_span < CURVE_X_SPAN_MIN,
        "the degenerate 1-point curve must not span the time axis, but ACCENT \
         pixels spanned {x_span}px (≥ {CURVE_X_SPAN_MIN}px expected only for a \
         real multi-point curve)",
    );
}

/// **The contrast pair (belt-and-braces).** Render the SAME screen twice — once
/// healthy, once in the dropped-points failure mode — and assert the healthy
/// curve paints strictly MORE `ACCENT` than the broken one by a wide margin.
/// This pins the harness's discriminating power as a single relational fact
/// (healthy ≫ broken), independent of the absolute thresholds above, so the
/// guard can never silently degrade into "both look the same".
#[test]
fn healthy_curve_draws_far_more_than_broken() {
    // Healthy.
    let mut healthy = Cockpit::new();
    healthy.current_screen = Screen::Live;
    for (secs, eq) in session_points() {
        update(&mut healthy, Message::PnlRefreshed(snap(secs, eq)));
    }
    let healthy_shot = render_live(healthy);
    let healthy_accent = accent_pixel_stats(&healthy_shot).count;

    // Broken (the dropped-points pathology).
    let mut broken = Cockpit::new();
    broken.current_screen = Screen::Live;
    let backwards: Vec<(i64, Decimal)> = vec![
        (1_000_000, dec!(100000)),
        (700, dec!(100800)),
        (650, dec!(101500)),
        (600, dec!(101200)),
    ];
    for (secs, eq) in backwards {
        update(&mut broken, Message::PnlRefreshed(snap(secs, eq)));
    }
    let broken_shot = render_live(broken);
    let broken_accent = accent_pixel_stats(&broken_shot).count;

    assert!(
        healthy_accent > broken_accent * 4 + 100,
        "harness must strongly distinguish healthy ({healthy_accent} ACCENT px) \
         from broken ({broken_accent} ACCENT px); the healthy curve should draw \
         a polyline an order of magnitude larger than the broken curve's noise"
    );
}

// ── PHASE 2 — live-equity-history-durable: the HYDRATED-boot render gate ─────
//
// AC6 (THE gate, R5): a cockpit hydrated from a durable store tail (zero live
// snapshots) must rasterize a real ACCENT polyline. AC5: a live append AFTER
// the hydrate must still land and the curve still draw + grow — the A4 `as_of`
// delivery-guard reconciliation proven at the pixel layer. These use REALISTIC
// two-timestamp rows: 2023-era `bar_ts` (the plotted x-coord, an old data date)
// paired with PRIOR-SESSION `as_of` wallclock stamps (all in the past relative
// to a live `now()`) — exactly the guard-reconciliation scenario the architect
// flagged as the riskiest (a fresh `now()` live tick must out-rank the stale
// hydrated `as_of` max and append, not be dropped).

/// 2023-data-time base for the hydrate tail's `bar_ts` (the plotted x-coord):
/// 2023-01-15 12:30:00 UTC. The curve self-scales to its own x-range, so the
/// absolute epoch does not move the ACCENT bbox out of the crop band.
const HYDRATE_BAR_BASE: i64 = 1_673_789_400;

/// Prior-session wallclock base for the hydrate tail's `as_of` (the delivery
/// key): ~2025-04-30, comfortably in the past vs. a live `Timestamp::now()`.
const HYDRATE_AS_OF_BASE: i64 = 1_746_000_000;

/// Build one durable-tail row `(bar_ts, as_of, equity)` from a 2023 `bar_ts`
/// second and a prior-session `as_of` second — the exact tuple shape
/// `audit::query::equity_snapshot_tail` returns and `Message::PnlHydrated`
/// consumes.
fn hydrate_row(
    bar_secs: i64,
    as_of_secs: i64,
    equity: Decimal,
) -> (Timestamp, Timestamp, Money<Usdt>) {
    (
        Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(bar_secs)),
        Timestamp::new(time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(as_of_secs)),
        Money::<Usdt>::from_decimal(equity),
    )
}

/// A realistic ≥2-row hydrate tail mirroring `session_points`' equity SHAPE
/// (rises, dips for a non-zero drawdown, recovers — same vertical span) so the
/// rasterized ACCENT geometry is directly comparable to the live-path harness.
/// `bar_ts` and `as_of` advance one minute per row, in disjoint epoch ranges.
fn hydrate_tail() -> Vec<(Timestamp, Timestamp, Money<Usdt>)> {
    session_points()
        .into_iter()
        .enumerate()
        .map(|(i, (secs, eq))| {
            // `secs` (0..420) seeds the per-row offset for BOTH timestamps so the
            // tail is monotone in bar_ts and as_of, in their separate epochs.
            let _ = secs;
            let off = i as i64 * 60;
            hydrate_row(HYDRATE_BAR_BASE + off, HYDRATE_AS_OF_BASE + off, eq)
        })
        .collect()
}

/// **AC6 (THE gate).** A cockpit hydrated by ONE `Message::PnlHydrated` (a faked
/// ≥2-row durable tail) with **zero `PnlRefreshed`** must rasterize a real curve:
///   (i)   the buffer holds every hydrated row,
///   (ii)  the curve `PanelState` is `Ready`,
///   (iii) the rendered polyline paints ≥ `CURVE_DREW_MIN_ACCENT` ACCENT pixels
///         (the durable history is on-screen, not a blank "no graph" panel), and
///   (iv)  those pixels span a sane horizontal range (`≥ CURVE_X_SPAN_MIN`).
///
/// A model-Ready-but-blank-canvas regression fails here — this is the render
/// proof the restart-hydrated curve actually draws before the first new bar.
#[test]
fn hydrated_boot_curve_actually_renders() {
    let tail = hydrate_tail();
    let n = tail.len();

    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    // ONE batch hydrate through the production update path; NO live tick.
    update(&mut c, Message::PnlHydrated(tail));

    // (i) every hydrated row landed in the buffer.
    assert_eq!(
        c.live_equity_buffer.len(),
        n,
        "all {n} hydrated rows must seed the buffer (none dropped)"
    );
    // (ii) the model says Ready off the hydrate alone.
    assert_eq!(
        c.live_equity_curve.variant_name(),
        "ready",
        "curve must be Ready after a ≥2-row hydrate, before any live tick"
    );
    // (and the since-inception switch flipped — the caption is mode-correct).
    assert!(c.live_equity_hydrated);

    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);

    // (iii) the hydrated polyline rasterized — a real curve, not a blank panel.
    assert!(
        stats.count >= CURVE_DREW_MIN_ACCENT,
        "HYDRATED curve did NOT render: only {} ACCENT pixels in the curve band \
         (expected ≥ {CURVE_DREW_MIN_ACCENT}). The buffer is seeded + Ready but \
         the canvas painted no visible polyline — the restart-blank-curve bug.",
        stats.count,
    );
    // (iv) the curve spans a sane horizontal range (traverses the x/time axis).
    let x_span = stats.max_x.saturating_sub(stats.min_x);
    assert!(
        x_span >= CURVE_X_SPAN_MIN,
        "hydrated curve x-span is degenerate: ACCENT pixels span only {x_span}px \
         (expected ≥ {CURVE_X_SPAN_MIN}px). A real multi-row history traverses \
         the time axis.",
    );
}

/// **AC5 (the guard-reconciliation render proof).** Hydrate from a durable tail
/// whose `as_of` values are all in the PAST, THEN deliver ONE live
/// `PnlRefreshed(now())`. The fresh `now()` wallclock out-ranks the stale
/// hydrated `as_of` max, so the live point MUST append (not be dropped) — and
/// the curve must still draw AND grow. This is the A4 `as_of` delivery-guard
/// contract proven at the rasterized layer (the riskiest decision, the one the
/// model-layer AC5 also pins — here at the pixel layer the brief demands).
#[test]
fn live_append_after_hydrate_still_renders_and_grows() {
    let tail = hydrate_tail();
    let seeded = tail.len();

    let mut c = Cockpit::new();
    c.current_screen = Screen::Live;
    update(&mut c, Message::PnlHydrated(tail));
    assert_eq!(c.live_equity_buffer.len(), seeded);

    // Baseline geometry of the hydrated-only curve. We compare the horizontal
    // EXTENT (x-span) rather than the raw ACCENT count, because appending a new
    // peak rescales the curve's Y-axis (the auto-fit range grows), which can
    // shift the polyline's pixel count slightly either way — a rescale is NOT a
    // regression. The x-span is the rescale-invariant "grew" signal: a forward
    // bar extends the curve along the time axis.
    let before_x_span = {
        let s = accent_pixel_stats(&render_live(c.clone()));
        s.max_x.saturating_sub(s.min_x)
    };

    // A live snapshot: fresh `now()` wallclock `as_of` (≥ every prior-session
    // hydrated `as_of`) + a `bar_ts` one minute AFTER the 2023 tail's last bar,
    // so the curve extends forward in DATA time too.
    let next_bar = HYDRATE_BAR_BASE + (seeded as i64) * 60;
    let live = PnlSnapshot {
        cash: Money::<Usdt>::from_decimal(dec!(104000)),
        unrealized: Money::<Usdt>::from_decimal(dec!(0)),
        realized: Money::<Usdt>::from_decimal(dec!(0)),
        total_equity: Money::<Usdt>::from_decimal(dec!(104000)),
        daily_return: Money::<Usdt>::from_decimal(dec!(0)),
        as_of: Timestamp::now(),
        bar_ts: Some(Timestamp::new(
            time::OffsetDateTime::UNIX_EPOCH + time::Duration::seconds(next_bar),
        )),
    };
    update(&mut c, Message::PnlRefreshed(live));

    // Model proof: the live point landed (the guard did NOT drop it).
    assert_eq!(
        c.live_equity_buffer.len(),
        seeded + 1,
        "the first live snapshot after hydrate must append at the model layer"
    );

    // Render proof (i): the curve still draws a real polyline after the append.
    let shot = render_live(c);
    let stats = accent_pixel_stats(&shot);
    assert!(
        stats.count >= CURVE_DREW_MIN_ACCENT,
        "post-hydrate live-append curve did NOT render: only {} ACCENT pixels \
         (expected ≥ {CURVE_DREW_MIN_ACCENT}). The guard must not have dropped \
         the live point AND the curve must still rasterize.",
        stats.count,
    );
    let x_span = stats.max_x.saturating_sub(stats.min_x);
    // Render proof (ii): the curve still spans the time axis (not degenerate)…
    assert!(
        x_span >= CURVE_X_SPAN_MIN,
        "post-append curve x-span is degenerate ({x_span}px < {CURVE_X_SPAN_MIN}px)",
    );
    // …and it GREW — the forward bar extended the curve's horizontal extent at
    // least as wide as the hydrated-only baseline (a dropped live point or a
    // collapsed curve would shrink it). Rescale-invariant, unlike a raw count.
    assert!(
        x_span >= before_x_span,
        "the curve must extend (or hold) its x-span after the forward live \
         append: {x_span}px now vs. {before_x_span}px hydrated-only — a shrink \
         would mean the live point was lost or the series collapsed"
    );
}

// ── PHASE 3 — ADR-0053 paper-mode-equity-wiring Y-variation gate (AC6) ────────
//
// The existing `count`/`x_span` assertions PASS on a flat equity series:
// `equity_curve.rs:178` (the flat-line guard) renders a degenerate series
// as a **centered, full-width horizontal line** that draws plenty of ACCENT
// pixels and spans the full x-axis. So `CURVE_DREW_MIN_ACCENT` and
// `CURVE_X_SPAN_MIN` do NOT distinguish a flat (flat `initial_capital` bug)
// from a real moving curve.
//
// The ONLY valid discriminator is the ACCENT bounding-box HEIGHT:
// `AccentStats` already tracks `min_y`/`max_y`. A moving curve has a tall
// bbox; the flat-line guard's centered horizontal line has a ~1-2px bbox.
// `CURVE_Y_VAR_MIN` is the new threshold that separates them.

/// Minimum ACCENT bounding-box HEIGHT (max_y − min_y) that a MOVING equity
/// curve must produce. A flat/degenerate series is rendered by the
/// `equity_curve.rs` flat-line guard as a centered horizontal line, producing
/// a bbox height of ~1–2 px (AA ramp), well below this threshold.
///
/// Calibration (via `diag_accent_bounding_box`): a healthy 8-point session
/// with rises/dips produces an ACCENT bbox spanning ~168 px vertically.
/// Setting the floor at 30 px is comfortably ABOVE the ~1–2 px flat-line
/// noise and comfortably BELOW the ~168 px healthy-curve height — the gap
/// is ~2 orders of magnitude, so this threshold is stable across themes and
/// minor layout shifts.
const CURVE_Y_VAR_MIN: u32 = 30;

/// **AC6 — Y-variation gate: non-flat moving curve passes; flat initial_capital
/// series fails (the self-proving contrast pair, ADR-0053 A6 / D5).**
///
/// This is the render-layer half of the baseline-equity-divergence gate.
/// The existing `count`/`x_span` checks do NOT discriminate flat from moving
/// (the flat-line guard renders the flat series as a full-width centered
/// horizontal line that PASSES both); only the ACCENT bbox HEIGHT distinguishes.
///
/// **Non-flat half:** a moving paper equity series (with rises and dips) renders
/// a polyline whose ACCENT bbox height ≥ `CURVE_Y_VAR_MIN`. This is the "the
/// paper loop actually produced moving equity" proof at the pixel layer.
///
/// **Flat contrast half (the self-proving proof):** a flat `initial_capital`
/// series (all-equal values — the current paper-mode bug) renders a bbox height
/// < `CURVE_Y_VAR_MIN`. It still PASSES `count`/`x_span` (the flat-line guard
/// draws it as a full-width horizontal line) — proving Y-variation is the ONLY
/// valid discriminator for the "is equity moving?" question.
#[test]
fn y_variation_gate_moving_passes_flat_fails() {
    // ── Non-flat half ─────────────────────────────────────────────────────────
    // Use `session_points()` which has rises AND dips (same as the other tests),
    // producing a curve with real vertical extent.
    let mut moving_c = Cockpit::new();
    moving_c.current_screen = Screen::Live;
    for (secs, eq) in session_points() {
        update(&mut moving_c, Message::PnlRefreshed(snap(secs, eq)));
    }
    let moving_shot = render_live(moving_c);
    let moving_stats = accent_pixel_stats(&moving_shot);
    let moving_y_span = moving_stats.max_y.saturating_sub(moving_stats.min_y);

    // A moving curve must pass the Y-variation threshold.
    assert!(
        moving_y_span >= CURVE_Y_VAR_MIN,
        "Y-VARIATION GATE FAILED for MOVING curve: ACCENT bbox height = {}px < {}px (CURVE_Y_VAR_MIN). \
         A real paper equity session with rises and dips should have a tall polyline; \
         if this fails the curve is rendering as flat when it should not.",
        moving_y_span,
        CURVE_Y_VAR_MIN,
    );
    // Confirm count/x_span also pass (belt-and-braces).
    assert!(
        moving_stats.count >= CURVE_DREW_MIN_ACCENT,
        "moving curve count check: {} < {CURVE_DREW_MIN_ACCENT}",
        moving_stats.count,
    );
    let moving_x_span = moving_stats.max_x.saturating_sub(moving_stats.min_x);
    assert!(
        moving_x_span >= CURVE_X_SPAN_MIN,
        "moving curve x_span check: {moving_x_span} < {CURVE_X_SPAN_MIN}",
    );

    // ── Flat contrast half (self-proving proof) ───────────────────────────────
    // A flat series at constant `initial_capital` — the current paper-mode bug.
    // All points have the same equity value: 100_000 (never moves).
    let mut flat_c = Cockpit::new();
    flat_c.current_screen = Screen::Live;
    for secs in [0i64, 60, 120, 180, 240, 300, 360, 420] {
        update(
            &mut flat_c,
            Message::PnlRefreshed(snap(secs, dec!(100_000))),
        );
    }
    let flat_shot = render_live(flat_c);
    let flat_stats = accent_pixel_stats(&flat_shot);
    let flat_y_span = flat_stats.max_y.saturating_sub(flat_stats.min_y);

    // A flat curve FAILS the Y-variation threshold (the flat-line guard draws it
    // as a centered horizontal line — ~1-2 px bbox height).
    assert!(
        flat_y_span < CURVE_Y_VAR_MIN,
        "Y-VARIATION SELF-PROOF FAILED for FLAT curve: ACCENT bbox height = {}px >= {}px \
         (CURVE_Y_VAR_MIN). The flat-line guard should render the flat series as a centered \
         horizontal line (~1-2px bbox). If this fails the self-proving contrast pair is broken \
         and the Y-variation gate can no longer distinguish flat from moving.",
        flat_y_span,
        CURVE_Y_VAR_MIN,
    );
    // The flat curve STILL passes count/x_span — proving those are insufficient.
    // (The flat-line guard draws a full-width centered horizontal line.)
    assert!(
        flat_stats.count >= CURVE_DREW_MIN_ACCENT,
        "flat curve count check (must pass — flat-line guard draws a full-width line): \
         {} < {CURVE_DREW_MIN_ACCENT}",
        flat_stats.count,
    );
    let flat_x_span = flat_stats.max_x.saturating_sub(flat_stats.min_x);
    assert!(
        flat_x_span >= CURVE_X_SPAN_MIN,
        "flat curve x_span check (must pass — flat-line guard draws a full-width line): \
         {flat_x_span} < {CURVE_X_SPAN_MIN}",
    );

    // Final relational proof: the moving curve has a taller ACCENT bbox than the flat.
    assert!(
        moving_y_span > flat_y_span,
        "Y-variation discriminator broken: moving curve bbox height ({moving_y_span}px) \
         must be > flat curve bbox height ({flat_y_span}px)"
    );
}

// ── Render-crash regression (a bonus bug this harness caught) ────────────────

/// **Render-crash regression — flat / single-point series.**
/// cockpit-live-equity-render-guard (2026-06-11): building this harness
/// surfaced a real production crash. A `Ready` equity curve holding ONE point
/// (the first live bar after boot — the design renders the curve from ≥1
/// point), or any all-equal run, made the Y-range collapse so the rasterizer
/// got a NaN `Point.y` and panicked inside `lyon_path`
/// (`assertion failed: p.y.is_finite()`). That is the cockpit crashing the
/// instant the agent publishes its first P&L snapshot.
///
/// The fix (`widgets/equity_curve.rs`) guards the Y denominator and centers a
/// flat line. This test pins it: rendering a 1-point curve AND a multi-point
/// FLAT (all-equal) curve must NOT panic. Before the fix both panicked; the
/// model-level `state.rs` tests never exercised the rasterizer so they sailed
/// past it — the exact gap this render harness exists to close.
#[test]
fn flat_and_single_point_curves_render_without_panic() {
    // (a) Single point — Ready from one bar (the first-bar-after-boot case).
    let mut one = Cockpit::new();
    one.current_screen = Screen::Live;
    update(&mut one, Message::PnlRefreshed(snap(0, dec!(100000))));
    assert_eq!(one.live_equity_buffer.len(), 1);
    assert_eq!(one.live_equity_curve.variant_name(), "ready");
    // The assertion under test is simply: this does not panic.
    let _ = render_live(one);

    // (b) Multi-point FLAT line — equity never moves (a paused / no-fill
    // session). All-equal → the same Y-range collapse, different point count.
    let mut flat = Cockpit::new();
    flat.current_screen = Screen::Live;
    for secs in [0i64, 60, 120, 180, 240] {
        update(&mut flat, Message::PnlRefreshed(snap(secs, dec!(100000))));
    }
    assert_eq!(flat.live_equity_buffer.len(), 5);
    assert_eq!(flat.live_equity_curve.variant_name(), "ready");
    // A flat multi-point line renders a finite horizontal traverse — and must
    // not panic. (It draws ACCENT, but we only gate "no panic" here; the
    // value-rich assertions live in `live_equity_curve_actually_renders`.)
    let _ = render_live(flat);
}

// ── PHASE 4 — lab-run-save-compare: Lab repaint + Compare overlay (T7) ────────
//
// The render-layer proof that the lab-run-save-compare feature's two new
// surfaces actually rasterize (project law — MEMORY.md "verify UI at the render
// layer"; model-Ready is necessary but NOT sufficient):
//
//   T7(a) — a Lab equity curve HYDRATED FROM A `lab-runs/` REPORT (loaded by the
//           two-root loader from a tempdir `lab-runs/` root) rasterizes a
//           non-empty `ACCENT` polyline on the real `chart::view` overlay widget
//           the Lab screen renders.
//   T7(b) — a Compare OVERLAY OF TWO RUNS (both loaded from `lab-runs/` reports)
//           rasterizes BOTH series — the primary `ACCENT` curve AND a second
//           `ACCENT_2` curve — proving two distinct curves drew on one chart
//           (R5: KPIs + equity overlay of both series).
//
// Both feed `ui::widgets::chart::view(bars, …, equity, compare, …)` via
// `ui::test_support::chart_overlay_program`, the EXACT overlay draw path
// (`ChartProgram::draw` → tiny_skia) the Lab/Compare screens paint.

use std::io::Write;

use ui::lab::equity_loader::{LabTuple, load_equity};
use ui::lab::state::{DateRange as LabDateRange, LabDataSource, Preset};
use ui::test_support::{chart_overlay_program, lab_runs_report_at};

/// Number of synthetic bars under the overlay. `synthetic_candles` stamps bar
/// `i` at `(FIXED_EPOCH_SECS + i*60)` seconds, so the bar window in
/// milliseconds is `[BAR_BASE_MS, BAR_BASE_MS + (RENDER_BARS-1)*60_000]`. The
/// `lab-runs/` fixture equity timestamps MUST fall INSIDE this window
/// (`bar_ms()` below; `ui::test_support::lab_runs_report_at` stamps bars 0,
/// 10, …, 50) or `draw_equity_polyline` clamps them to one edge → a
/// degenerate vertical line; `compute_equity_range` would also drop them
/// (it windows to `[bars.first, bars.last]`).
const RENDER_BARS: usize = 60;

/// `FIXED_EPOCH_SECS` from `fixtures.rs` (Jan 2024) — the base second
/// `synthetic_candles` stamps bar 0 at. Mirrored here so the fixture equity
/// timestamps land inside the synthetic-bar window.
const FIXED_EPOCH_SECS: i64 = 1_705_320_000;
const BAR_BASE_MS: i64 = FIXED_EPOCH_SECS * 1000;

/// Equity-curve timestamp (ms) for the `n`-th fixture point — one bar-minute
/// apart, anchored at the bar window's base so the overlay traverses the
/// x-axis inside `[bars.first, bars.last]`.
fn bar_ms(n: i64) -> i64 {
    BAR_BASE_MS + n * 60_000
}

/// Write a `lab-runs/<slug>/reports/<fname>` fixture report and return the
/// `lab-runs/` root path. Mirrors the developer's write-root shape exactly.
fn write_lab_run(lab_runs_root: &std::path::Path, slug: &str, fname: &str, content: &str) {
    let reports = lab_runs_root.join(slug).join("reports");
    std::fs::create_dir_all(&reports).expect("create lab-runs reports dir");
    let mut f = std::fs::File::create(reports.join(fname)).expect("create lab-runs report");
    f.write_all(content.as_bytes())
        .expect("write lab-runs report");
}

/// Synthetic bars for the overlay x-axis (same builder + seed the fixtures
/// cockpit uses, so the bar window is deterministic).
fn render_bars() -> Vec<trading_core::Bar> {
    let venue = trading_core::Venue::Binance;
    let symbol = trading_core::Symbol::new("XRPUSDT");
    let seed = ui::fixtures::seed_for(venue, &symbol);
    ui::fixtures::synthetic_candles(seed, venue, symbol, RENDER_BARS)
}

/// `(r,g,b)` 0-255 of an `ACCENT_2` (dark theme) pixel — the SECOND overlay
/// curve's color (the `accent_palette()[0]` compare line). ACCENT dark =
/// (111,182,174) teal-300; ACCENT_2 dark = (166,213,207) teal-200 (lighter).
/// They share a hue, so a loose per-channel box would let AA pixels count as
/// both — instead [`classify_curve`] below uses a NEAREST-of-two classifier so
/// each curve pixel is attributed to exactly ONE curve (the proof that two
/// DISTINCT curves drew).
fn accent2_rgb() -> (i32, i32, i32) {
    let c = color::ACCENT_2.current(ThemeMode::Dark);
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

/// Squared Euclidean RGB distance.
fn dist2(p: (i32, i32, i32), q: (i32, i32, i32)) -> i32 {
    let dr = p.0 - q.0;
    let dg = p.1 - q.1;
    let db = p.2 - q.2;
    dr * dr + dg * dg + db * db
}

/// Which overlay curve (if any) a pixel belongs to. A pixel is attributed to
/// the NEAREST of {ACCENT, ACCENT_2} when it is within `CURVE_MATCH_R2` of that
/// color AND strictly closer to it than to the other (a clear margin). Pixels
/// near neither (background, gridlines, AA midpoints between the two) are
/// `Neither`. Because the classifier is winner-take-all, no pixel is ever
/// counted for BOTH curves — so two non-zero counts prove two distinct curves.
#[derive(PartialEq, Eq)]
enum Curve {
    Accent,
    Accent2,
    Neither,
}

/// Max squared distance for a pixel to be claimed by a curve color. The
/// ACCENT↔ACCENT_2 squared separation is 55²+31²+33² = 5075; a radius² of 900
/// (≈30/channel) is well inside half that separation, so the two acceptance
/// balls are disjoint and an AA pixel exactly between them is `Neither`.
const CURVE_MATCH_R2: i32 = 900;

fn classify_curve(r: u8, g: u8, b: u8) -> Curve {
    let p = (i32::from(r), i32::from(g), i32::from(b));
    let d_accent = dist2(p, accent_rgb());
    let d_accent2 = dist2(p, accent2_rgb());
    if d_accent <= CURVE_MATCH_R2 && d_accent < d_accent2 {
        Curve::Accent
    } else if d_accent2 <= CURVE_MATCH_R2 && d_accent2 < d_accent {
        Curve::Accent2
    } else {
        Curve::Neither
    }
}

/// Count `(accent, accent2)` curve pixels across the WHOLE frame via the
/// winner-take-all classifier. The bare overlay widget fills the frame, so no
/// sidebar/crop offset is needed (unlike the Live screen harness above).
fn count_curve_pixels(shot: &iced::window::Screenshot) -> (usize, usize) {
    let rgba: &[u8] = &shot.rgba;
    let (mut accent, mut accent2) = (0usize, 0usize);
    let mut i = 0;
    while i + 2 < rgba.len() {
        match classify_curve(rgba[i], rgba[i + 1], rgba[i + 2]) {
            Curve::Accent => accent += 1,
            Curve::Accent2 => accent2 += 1,
            Curve::Neither => {}
        }
        i += 4;
    }
    (accent, accent2)
}

/// Render the bare chart-overlay widget (bars + equity + compare) and return
/// its RGBA screenshot via the real `ChartProgram::draw` path.
fn render_overlay(
    bars: Vec<trading_core::Bar>,
    equity: Option<ui::lab::equity_loader::LabEquitySeries>,
    compare: Vec<ui::lab::equity_loader::LabEquitySeries>,
) -> iced::window::Screenshot {
    // Thread-safe atomic flag — no env data race, mirrors `render_live`.
    force_utc_once();
    let program = chart_overlay_program(bars, equity, compare);
    let theme = iced::Theme::Dark;
    iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO)
}

/// Floor for "the overlay polyline actually drew". The overlay stroke is
/// thinner (1.5 px) than the Live curve's, and there is no fill under the
/// compare line, so the per-curve pixel budget is smaller; 120 is a
/// comfortable floor above AA noise yet well below a real traversing
/// polyline's count on a 1280-wide frame.
const OVERLAY_DREW_MIN: usize = 120;

/// Diagnostic (run with `--nocapture`): print the per-curve pixel counts for a
/// single-run overlay and a two-run overlay so `OVERLAY_DREW_MIN` is calibrated
/// empirically, not guessed. Mirrors `diag_accent_bounding_box` above.
#[test]
fn diag_overlay_curve_pixel_counts() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = write_two_run_scene(&tmp);
    let (a, b) = load_two_run_scene(&lab_runs);

    let (bars_only_accent, bars_only_accent2) =
        count_curve_pixels(&render_overlay(render_bars(), None, vec![]));
    let (one_accent, one_accent2) =
        count_curve_pixels(&render_overlay(render_bars(), Some(a.clone()), vec![]));
    let (two_accent, two_accent2) =
        count_curve_pixels(&render_overlay(render_bars(), Some(a), vec![b]));
    eprintln!(
        "[diag] bars-only (no equity): ACCENT={bars_only_accent} ACCENT_2={bars_only_accent2}; \
         single-run overlay: ACCENT={one_accent} ACCENT_2={one_accent2}; \
         two-run overlay: ACCENT={two_accent} ACCENT_2={two_accent2}"
    );
}

/// Write a two-run `lab-runs/` scene: run A (momentum/XRPUSDT) at the ~100k
/// level + run B (sma/BTCUSDT) at the ~60k level — DISJOINT y-bands so the two
/// overlay polylines paint separate pixels. Returns the `lab-runs/` root.
fn write_two_run_scene(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    let lab_runs = tmp.path().join("lab-runs");
    write_lab_run(
        &lab_runs,
        "v1-cross-sectional-momentum",
        "backtest-20260601-120000-top10-2024-h1-momentum.md",
        &lab_runs_report_at("top10-2024-h1-momentum", "XRPUSDT", 100_000),
    );
    write_lab_run(
        &lab_runs,
        "v0-paper-sma",
        "backtest-20260601-130000-btc-2023-1m-sma-cross.md",
        &lab_runs_report_at("btc-2023-1m-sma-cross", "BTCUSDT", 60_000),
    );
    lab_runs
}

/// Load both runs of the two-run scene via the loader the production path uses.
fn load_two_run_scene(
    lab_runs: &std::path::Path,
) -> (
    ui::lab::equity_loader::LabEquitySeries,
    ui::lab::equity_loader::LabEquitySeries,
) {
    let tuple_a = LabTuple {
        strategy: smol_str::SmolStr::new("v1.momentum"),
        symbol: smol_str::SmolStr::new("XRPUSDT"),
        range: LabDateRange::Preset(Preset::H1_2024),
        source: LabDataSource::Synthetic,
    };
    let tuple_b = LabTuple {
        strategy: smol_str::SmolStr::new("v0.sma"),
        symbol: smol_str::SmolStr::new("BTCUSDT"),
        range: LabDateRange::Preset(Preset::Last90d),
        source: LabDataSource::Synthetic,
    };
    let a = load_equity(&tuple_a, lab_runs).expect("run A loads");
    let b = load_equity(&tuple_b, lab_runs).expect("run B loads");
    (a, b)
}

/// Minimum EXTRA `ACCENT` pixels the hydrated equity overlay must add over the
/// bars-only (price-line) baseline. Calibration (`diag_overlay_curve_pixel_counts`):
/// bars-only ≈ 2610 ACCENT (the price line, also `ACCENT`); +equity overlay
/// ≈ 3894 (+1284). 400 is a comfortable floor well above paint jitter yet far
/// below the ~1284 a real overlay polyline adds. The CONTRAST (with-equity ≫
/// bars-only) is what isolates the equity curve from the same-color price line.
const OVERLAY_ACCENT_DELTA_MIN: usize = 400;

/// **T7(a) — Lab repaint-from-`lab-runs/` render proof.** A `LabEquitySeries`
/// LOADED by the two-root loader from a tempdir `lab-runs/` report rasterizes a
/// non-empty `ACCENT` polyline on the real Lab overlay widget (`chart::view`).
/// This is the AC4 "the curve survives a restart by repainting from the
/// persisted report" guarantee, proven at the pixel layer.
///
/// The Lab chart's PRICE line is also `ACCENT`, so a raw ACCENT count cannot
/// tell "the equity curve drew" from "the price line drew". The proof is a
/// CONTRAST: render the SAME bars WITHOUT the equity overlay (price line only)
/// and WITH the hydrated overlay, and assert the overlay adds a non-trivial
/// band of EXTRA `ACCENT` pixels — i.e. the loaded-from-disk equity polyline is
/// genuinely on screen, additive to the price line.
#[test]
fn lab_curve_hydrated_from_lab_runs_report_renders() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = tmp.path().join("lab-runs");
    write_lab_run(
        &lab_runs,
        "v1-cross-sectional-momentum",
        "backtest-20260601-120000-top10-2024-h1-momentum.md",
        &lab_runs_report_at("top10-2024-h1-momentum", "XRPUSDT", 100_000),
    );

    // Load the series via the loader the production Lab cold path uses, pointed
    // at the `lab-runs/` root (the developer's write-root shape).
    let tuple = LabTuple {
        strategy: smol_str::SmolStr::new("v1.momentum"),
        symbol: smol_str::SmolStr::new("XRPUSDT"),
        range: LabDateRange::Preset(Preset::H1_2024),
        source: LabDataSource::Synthetic,
    };
    let series = load_equity(&tuple, &lab_runs).expect("series loads from lab-runs report");
    assert!(
        series.samples.len() >= 2,
        "hydrated series must have ≥2 points to traverse (got {})",
        series.samples.len()
    );

    let bars = render_bars();
    let (baseline_accent, _) = count_curve_pixels(&render_overlay(bars.clone(), None, vec![]));
    let (with_equity_accent, _) = count_curve_pixels(&render_overlay(bars, Some(series), vec![]));

    let delta = with_equity_accent.saturating_sub(baseline_accent);
    assert!(
        delta >= OVERLAY_ACCENT_DELTA_MIN,
        "Lab curve hydrated from a lab-runs/ report did NOT render: the equity overlay added \
         only {delta} ACCENT pixels over the price-line baseline ({baseline_accent} → \
         {with_equity_accent}; expected +≥{OVERLAY_ACCENT_DELTA_MIN}). The series parsed from \
         disk but the overlay canvas painted no visible polyline — the repaint-from-disk \
         render bug."
    );
}

/// **T7(b) — Compare two-run overlay render proof (the headline render gate for
/// R5).** TWO `LabEquitySeries`, BOTH loaded from `lab-runs/` reports, overlaid
/// on ONE chart (`equity` = run A drawn `ACCENT`; `compare[0]` = run B drawn
/// `ACCENT_2`) must rasterize BOTH curves. Asserts ACCENT pixels (run A) AND
/// ACCENT_2 pixels (run B) BOTH cross the "drew" floor — i.e. two DISTINCT
/// curves are on screen, not one. A single-curve regression (compare series
/// dropped, or both collapsed to the same color) fails here.
#[test]
fn compare_two_run_overlay_renders_both_series() {
    let tmp = tempfile::tempdir().expect("tempdir");
    // Run A (primary, ACCENT) at ~100k; run B (compare, ACCENT_2) at ~60k —
    // disjoint y-bands so the two polylines paint separate pixels.
    let lab_runs = write_two_run_scene(&tmp);
    let (series_a, series_b) = load_two_run_scene(&lab_runs);

    let shot = render_overlay(render_bars(), Some(series_a), vec![series_b]);

    let (accent, accent2) = count_curve_pixels(&shot);

    // Run A (primary, ACCENT) drew.
    assert!(
        accent >= OVERLAY_DREW_MIN,
        "Compare overlay: the PRIMARY (ACCENT) run did NOT render: only {accent} ACCENT \
         pixels (expected ≥ {OVERLAY_DREW_MIN})."
    );
    // Run B (compare, ACCENT_2) drew — the proof TWO distinct curves are on
    // screen. This is the assertion that fails if the second series is dropped
    // or both render in the same color.
    assert!(
        accent2 >= OVERLAY_DREW_MIN,
        "Compare overlay: the SECOND (ACCENT_2) run did NOT render: only {accent2} ACCENT_2 \
         pixels (expected ≥ {OVERLAY_DREW_MIN}). The two-run overlay must rasterize BOTH \
         series as distinct curves — a dropped compare series or a same-color collapse lands \
         here."
    );
}

/// **T7(b) contrast self-proof.** Render the SAME overlay scene WITHOUT the
/// compare series and assert NO `ACCENT_2` pixels appear — proving the
/// `ACCENT_2` count in `compare_two_run_overlay_renders_both_series` genuinely
/// comes from the second run, not from chrome/AA bleed. Belt-and-braces so the
/// two-curve discriminator can never silently degrade into "ACCENT_2 is always
/// present".
#[test]
fn single_run_overlay_draws_no_accent2() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = tmp.path().join("lab-runs");
    write_lab_run(
        &lab_runs,
        "v1-cross-sectional-momentum",
        "backtest-20260601-120000-top10-2024-h1-momentum.md",
        &lab_runs_report_at("top10-2024-h1-momentum", "XRPUSDT", 100_000),
    );
    let tuple = LabTuple {
        strategy: smol_str::SmolStr::new("v1.momentum"),
        symbol: smol_str::SmolStr::new("XRPUSDT"),
        range: LabDateRange::Preset(Preset::H1_2024),
        source: LabDataSource::Synthetic,
    };
    let series = load_equity(&tuple, &lab_runs).expect("series loads");

    // No compare series → ACCENT_2 must be absent.
    let shot = render_overlay(render_bars(), Some(series), vec![]);
    let (_accent, accent2) = count_curve_pixels(&shot);
    assert!(
        accent2 < OVERLAY_DREW_MIN,
        "single-run overlay must draw NO second (ACCENT_2) curve, but saw {accent2} ACCENT_2 \
         pixels (≥ {OVERLAY_DREW_MIN}). If this fails the two-curve discriminator is reading \
         chrome/AA bleed, not the actual compare series — fix the detector before trusting it."
    );
}

// ── PHASE 4b — bug-log #205: the Lab's committed-evidence fallback ──────────

use ui::lab::equity_loader::{
    EquityCache, EquityLoadError, Fidelity, default_evidence_root, route_equity_overlay,
};
use ui::lab::state::LabState;

/// The momentum tuple both fixtures and the committed report answer.
fn momentum_tuple() -> LabTuple {
    LabTuple {
        strategy: smol_str::SmolStr::new("v1.momentum"),
        symbol: smol_str::SmolStr::new("XRPUSDT"),
        range: LabDateRange::Preset(Preset::H1_2024),
        source: LabDataSource::Synthetic,
    }
}

/// The momentum fixture's file name (its scenario matches `momentum_tuple`).
const MOMENTUM_FIXTURE: &str = "backtest-20260601-120000-top10-2024-h1-momentum.md";

/// **LR1 — a committed-layout evidence report repaints the Lab curve (#205).**
/// The fixture sits at `<tmp>/evidence/v1/<slug>/reports/` — container depth
/// 1, the committed layout since `1395520629dc` — and is resolved by the
/// production cold path `route_equity_overlay` over the production-ordered
/// roots `[lab-runs/ (absent), evidence/]`. It must add a visible overlay
/// polyline on the real chart draw path.
#[test]
fn lab_curve_hydrated_from_container_depth_1_evidence_report_renders() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let evidence = tmp.path().join("evidence");
    write_lab_run(
        &evidence.join("v1"),
        "v1-cross-sectional-momentum",
        MOMENTUM_FIXTURE,
        &lab_runs_report_at("top10-2024-h1-momentum", "XRPUSDT", 100_000),
    );
    let lab_runs = tmp.path().join("lab-runs");
    assert!(
        !lab_runs.exists(),
        "lab-runs/ is absent: evidence/ must answer"
    );
    let roots = [lab_runs, evidence];

    let mut cache = EquityCache::new();
    let series = route_equity_overlay(&LabState::default(), &mut cache, &momentum_tuple(), &roots)
        .expect(
            "a report at evidence/v1/<slug>/reports must resolve through the Lab cold path (#205)",
        );
    assert_eq!(series.fidelity, Fidelity::PerBar);
    assert!(series.samples.len() >= 2);

    let bars = render_bars();
    let (baseline_accent, _) = count_curve_pixels(&render_overlay(bars.clone(), None, vec![]));
    let shot = render_overlay(bars, Some(series), vec![]);
    if let Some(img) =
        image::RgbaImage::from_raw(shot.size.width, shot.size.height, shot.rgba.to_vec())
    {
        let _ = img.save("/tmp/lab_container_depth_1_evidence_overlay.png");
    }
    let (with_equity_accent, _) = count_curve_pixels(&shot);
    let delta = with_equity_accent.saturating_sub(baseline_accent);
    eprintln!(
        "#205 pixel margin LR1: +{delta} ACCENT px ({baseline_accent} -> {with_equity_accent}; \
         threshold >= {OVERLAY_ACCENT_DELTA_MIN})"
    );
    assert!(
        delta >= OVERLAY_ACCENT_DELTA_MIN,
        "the container-depth-1 evidence report must paint the Lab overlay (+{delta} ACCENT px, \
         {baseline_accent} → {with_equity_accent}; expected +≥{OVERLAY_ACCENT_DELTA_MIN}) (#205). \
         PNG: /tmp/lab_container_depth_1_evidence_overlay.png"
    );
}

/// **LR2 — LR1's pair: an out-of-contract container depth resolves nothing.** The SAME
/// bytes at `<tmp>/evidence/v1/x/<slug>/reports/` (container depth 2) are not
/// read (ADR-0055 D8): `route_equity_overlay` is `None` and `load_equity` is
/// `ReportDirNotFound`. The routing assertion (`overlay.is_none()`) carries
/// the negative control; there is no pixel assertion, because a `None`
/// overlay renders the bars-only frame by construction (a pixel delta against
/// that same frame could not fail). The PNG is still written for reading.
#[test]
fn lab_curve_from_container_depth_2_evidence_report_does_not_resolve() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let evidence = tmp.path().join("evidence");
    write_lab_run(
        &evidence.join("v1").join("x"),
        "v1-cross-sectional-momentum",
        MOMENTUM_FIXTURE,
        &lab_runs_report_at("top10-2024-h1-momentum", "XRPUSDT", 100_000),
    );
    assert!(
        evidence
            .join("v1/x/v1-cross-sectional-momentum/reports")
            .join(MOMENTUM_FIXTURE)
            .is_file(),
        "the container-depth-2 fixture must exist, so the negative is not 'file missing'"
    );
    let roots = [tmp.path().join("lab-runs"), evidence.clone()];

    let mut cache = EquityCache::new();
    let overlay = route_equity_overlay(&LabState::default(), &mut cache, &momentum_tuple(), &roots);
    assert!(
        overlay.is_none(),
        "a report at container depth 2 must NOT resolve (ADR-0055 D8)"
    );
    let direct = load_equity(&momentum_tuple(), &evidence);
    assert!(
        matches!(direct, Err(EquityLoadError::ReportDirNotFound(_))),
        "load_equity must report ReportDirNotFound for an out-of-contract report; got {direct:?}"
    );

    // For reading only (see the doc): the bars-only frame the `None` overlay
    // routes to.
    let shot = render_overlay(render_bars(), overlay, vec![]);
    if let Some(img) =
        image::RgbaImage::from_raw(shot.size.width, shot.size.height, shot.rgba.to_vec())
    {
        let _ = img.save("/tmp/lab_container_depth_2_evidence_overlay.png");
    }
}

// ── PHASE 4c — bug-log #207: the equity gutter is reserved iff the overlay draws ──

/// The committed v1 momentum report (relative to `evidence/`). It is
/// start/end-only: no `## Equity curve` section and no companion CSV, so the
/// Lab loader synthesises two samples at the ordinal timestamps `0` and `1`.
const COMMITTED_START_END_ONLY: &str =
    "v1/v1-cross-sectional-momentum/reports/backtest-20260429-195243-top10-2024-h1-momentum.md";

/// Write `shot` to `path` (the file's PNG-dump convention: always written,
/// read by a human; a failed save never fails the test).
fn dump_png(shot: &iced::window::Screenshot, path: &str) {
    if let Some(img) =
        image::RgbaImage::from_raw(shot.size.width, shot.size.height, shot.rgba.to_vec())
    {
        let _ = img.save(path);
    }
}

/// Number of pixels whose RGBA differs between two same-size frames.
fn differing_pixels(a: &iced::window::Screenshot, b: &iced::window::Screenshot) -> usize {
    assert_eq!(
        a.size, b.size,
        "frames must have the same size to be compared"
    );
    a.rgba
        .chunks_exact(4)
        .zip(b.rgba.chunks_exact(4))
        .filter(|(p, q)| p != q)
        .count()
}

/// Copy the committed start/end-only report byte-for-byte into
/// `<tmp>/v1-cross-sectional-momentum/reports/` and load it with the Lab
/// loader. Asserts the preconditions every #207 test below stands on: the
/// copy is byte-identical, the series is `StartEndOnly` with two samples, and
/// both samples lie outside the `render_bars()` window (derived from the bars,
/// not from the subject).
fn committed_start_end_only_series(
    tmp: &tempfile::TempDir,
) -> ui::lab::equity_loader::LabEquitySeries {
    let src = default_evidence_root().join(COMMITTED_START_END_ONLY);
    assert!(
        src.is_file(),
        "the committed report must exist: {}",
        src.display()
    );
    let dst_dir = tmp
        .path()
        .join("v1-cross-sectional-momentum")
        .join("reports");
    std::fs::create_dir_all(&dst_dir).expect("create tmp reports dir");
    let file_name = src.file_name().expect("report file name");
    let dst = dst_dir.join(file_name);
    std::fs::copy(&src, &dst).expect("copy the committed report");
    assert_eq!(
        std::fs::read(&src).expect("read source"),
        std::fs::read(&dst).expect("read copy"),
        "the copy must be byte-identical to the committed report"
    );

    let series = load_equity(&momentum_tuple(), tmp.path()).expect("the copied report loads");
    assert_eq!(series.fidelity, Fidelity::StartEndOnly);
    assert_eq!(series.samples.len(), 2, "start/end-only = two samples");
    let bars = render_bars();
    let win_start = bars.first().expect("bars").open_ts.unix_millis();
    let win_end = bars.last().expect("bars").close_ts.unix_millis();
    assert!(
        series
            .samples
            .iter()
            .all(|(ts, _)| *ts < win_start || *ts > win_end),
        "both samples must lie outside the bar window [{win_start}, {win_end}]: {:?}",
        series.samples
    );
    series
}

/// The in-window per-bar series LR-207b draws (and LR-207a's negative control
/// uses): a `lab-runs/` report stamped on the `render_bars()` bars
/// (`ui::test_support::lab_runs_report_at`) under `<tmp>/lab-runs/`, loaded
/// with the Lab loader. Asserts it is `PerBar` and that every sample lies in
/// the bar window (derived from the bars, not from the subject).
fn in_window_lab_run_series(tmp: &tempfile::TempDir) -> ui::lab::equity_loader::LabEquitySeries {
    let lab_runs = tmp.path().join("lab-runs");
    write_lab_run(
        &lab_runs,
        "v1-cross-sectional-momentum",
        MOMENTUM_FIXTURE,
        &lab_runs_report_at("top10-2024-h1-momentum", "XRPUSDT", 100_000),
    );
    let series = load_equity(&momentum_tuple(), &lab_runs).expect("the in-window report loads");
    assert_eq!(series.fidelity, Fidelity::PerBar);
    let bars = render_bars();
    let win_start = bars.first().expect("bars").open_ts.unix_millis();
    let win_end = bars.last().expect("bars").close_ts.unix_millis();
    assert!(
        series
            .samples
            .iter()
            .all(|(ts, _)| (win_start..=win_end).contains(ts)),
        "precondition: every sample lies in the bar window [{win_start}, {win_end}]"
    );
    series
}

/// **LR-207a — a start/end-only overlay outside the bar window reserves no
/// gutter (#207).** The committed report's two samples miss the bar window, so
/// no equity curve and no equity axis can draw; the frame must be
/// byte-identical to the bars-only frame (no empty right gutter). Negative
/// control, in this test: LR-207b's in-window series over the same bars DOES
/// change the frame, so the identity cannot pass because `render_overlay`
/// dropped the overlay. PNGs `/tmp/lab_207_bars_only.png`,
/// `/tmp/lab_207_out_of_window.png`.
#[test]
fn start_end_overlay_outside_bar_window_reserves_no_gutter() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let series = committed_start_end_only_series(&tmp);
    let tmp_in = tempfile::tempdir().expect("tempdir");
    let in_window = in_window_lab_run_series(&tmp_in);

    let bars = render_bars();
    let bars_only = render_overlay(bars.clone(), None, vec![]);
    let with_series = render_overlay(bars.clone(), Some(series), vec![]);
    let with_in_window = render_overlay(bars, Some(in_window), vec![]);
    dump_png(&bars_only, "/tmp/lab_207_bars_only.png");
    dump_png(&with_series, "/tmp/lab_207_out_of_window.png");

    let differing = differing_pixels(&bars_only, &with_series);
    let control = differing_pixels(&bars_only, &with_in_window);
    eprintln!(
        "#207 pixel margin LR-207a: {differing} px differ from the bars-only frame \
         (threshold == 0); negative control: the in-window overlay changes {control} px \
         (threshold > 0)"
    );
    assert!(
        differing == 0,
        "#207: an overlay that draws nothing must not change the frame, but {differing} px \
         differ from the bars-only frame (the empty equity gutter). PNGs: \
         /tmp/lab_207_bars_only.png, /tmp/lab_207_out_of_window.png"
    );
    assert!(
        control > 0,
        "negative control: an in-window overlay must change the bars-only frame, but 0 px \
         differ, so this harness does not draw the overlay it is given"
    );
}

/// Rows of the bare 1280×720 overlay frame searched for the plot's right
/// edge: below the chart legend (a top-right inset that holds an `ACCENT`
/// "Price" line stub, rows ≈ 20..100) and above the time-axis gutter. The
/// legend moves with the plot rect, so it is excluded only to keep the
/// measurement on the price line. Calibrated by
/// `diag_207_right_edge_by_row_band`.
const PLOT_ROWS_BELOW_LEGEND: (u32, u32) = (120, 680);

/// Rightmost column holding a `Curve::Accent` pixel within rows `[y0, y1)`:
/// the price line's last vertex sits on the plot rect's right edge, and the
/// equity overlay (also `ACCENT`) ends before it, so this is the plot's right
/// edge in the frame.
fn rightmost_accent_column(shot: &iced::window::Screenshot, (y0, y1): (u32, u32)) -> Option<u32> {
    let w = shot.size.width;
    let rgba: &[u8] = &shot.rgba;
    (0..w).rev().find(|&x| {
        (y0..y1.min(shot.size.height)).any(|y| {
            let i = ((y * w + x) * 4) as usize;
            classify_curve(rgba[i], rgba[i + 1], rgba[i + 2]) == Curve::Accent
        })
    })
}

/// Expected right-edge shift when the equity gutter is reserved, DERIVED from
/// the widget constants (`ui::test_support::EQUITY_GUTTER_WIDENING_PX` =
/// `AXIS_GUTTER_EQUITY_PX` 56.0 − `AXIS_GUTTER_RIGHT_PX` 16.0 = 40 px), with
/// slack for the line's anti-aliased end column (the last segment's slope
/// differs between the two plot widths). A missing gutter (0) or a doubled one
/// (80) falls far outside `[40 − 6, 40 + 4]`.
fn gutter_shift_band() -> (i64, i64, i64) {
    let expected = ui::test_support::EQUITY_GUTTER_WIDENING_PX.round() as i64;
    (expected, expected - 6, expected + 4)
}

/// Diagnostic (run with `--nocapture`): the rightmost `ACCENT` column of the
/// bars-only frame per 40-row band, so `PLOT_ROWS_BELOW_LEGEND` is placed
/// below the legend's rows by measurement. Not an assertion gate.
#[test]
fn diag_207_right_edge_by_row_band() {
    let shot = render_overlay(render_bars(), None, vec![]);
    let bands: Vec<String> = (0..shot.size.height)
        .step_by(40)
        .map(|y0| {
            let edge = rightmost_accent_column(&shot, (y0, y0 + 40));
            format!("{y0}..{}: {edge:?}", y0 + 40)
        })
        .collect();
    eprintln!("[diag #207] bars-only rightmost ACCENT column by rows: {bands:?}");
    eprintln!(
        "[diag #207] PLOT_ROWS_BELOW_LEGEND {PLOT_ROWS_BELOW_LEGEND:?}: {:?}",
        rightmost_accent_column(&shot, PLOT_ROWS_BELOW_LEGEND)
    );
}

/// **LR-207b — an overlay inside the bar window reserves the gutter (#207's
/// other side).** A per-bar `lab-runs/` report stamped on the bars draws, so
/// the plot's right edge moves left by the derived gutter widening (LR-207a
/// pins the out-of-window shift at 0 by frame identity). PNG
/// `/tmp/lab_207_in_window.png`.
#[test]
fn overlay_inside_bar_window_reserves_the_gutter() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let series = in_window_lab_run_series(&tmp);
    let bars = render_bars();

    let bars_only = render_overlay(bars.clone(), None, vec![]);
    let with_series = render_overlay(bars, Some(series), vec![]);
    dump_png(&with_series, "/tmp/lab_207_in_window.png");

    let edge_plain =
        rightmost_accent_column(&bars_only, PLOT_ROWS_BELOW_LEGEND).expect("price line drawn");
    let edge_gutter =
        rightmost_accent_column(&with_series, PLOT_ROWS_BELOW_LEGEND).expect("price line drawn");
    let shift = i64::from(edge_plain) - i64::from(edge_gutter);
    let (expected, lo, hi) = gutter_shift_band();
    eprintln!(
        "#207 pixel margin LR-207b: right edge {edge_plain} -> {edge_gutter}, shift {shift} px \
         (expected {expected}, band [{lo}, {hi}])"
    );
    assert!(
        (lo..=hi).contains(&shift),
        "#207: an in-window overlay must reserve the equity gutter: the plot's right edge moved \
         {shift} px ({edge_plain} -> {edge_gutter}), expected {expected} within [{lo}, {hi}]. \
         PNG: /tmp/lab_207_in_window.png"
    );
}

/// **LR-207c — without bars the start/end-only series still draws.** The
/// bar-less path has no window, so the committed report's two samples span
/// the chart (the pre-existing straight segment): #207 must not darken it.
/// Negative control: no bars and no series draws no `ACCENT` curve. PNG
/// `/tmp/lab_207_no_bars_start_end.png`.
#[test]
fn start_end_overlay_without_bars_still_draws() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let series = committed_start_end_only_series(&tmp);

    let with_series = render_overlay(Vec::new(), Some(series), vec![]);
    let nothing = render_overlay(Vec::new(), None, vec![]);
    dump_png(&with_series, "/tmp/lab_207_no_bars_start_end.png");
    let (accent_with, _) = count_curve_pixels(&with_series);
    let (accent_none, _) = count_curve_pixels(&nothing);
    eprintln!(
        "#207 pixel margin LR-207c: {accent_with} ACCENT px with the series (threshold >= \
         {OVERLAY_DREW_MIN}); control {accent_none} without (threshold < {OVERLAY_DREW_MIN})"
    );
    assert!(
        accent_with >= OVERLAY_DREW_MIN,
        "#207: the bar-less path must still draw the start/end-only segment ({accent_with} \
         ACCENT px, expected >= {OVERLAY_DREW_MIN}). PNG: /tmp/lab_207_no_bars_start_end.png"
    );
    assert!(
        accent_none < OVERLAY_DREW_MIN,
        "negative control: no bars and no series must draw no curve ({accent_none} ACCENT px)"
    );
}

/// **LR-207e — a compare-only chart without bars draws its curve (#207).**
/// The Compare screen's one-slot case: slot 0 unresolved, slot 1 resolved,
/// so the chart gets no bars, no primary and one compare series. The gutter
/// gate must count compare series, or the bar-less path skips the only curve.
/// Negative control: the same chart with no series draws no `ACCENT_2`.
#[test]
fn compare_only_overlay_without_bars_draws_its_curve() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let series = committed_start_end_only_series(&tmp);

    let with_compare = render_overlay(Vec::new(), None, vec![series]);
    let nothing = render_overlay(Vec::new(), None, vec![]);
    dump_png(&with_compare, "/tmp/lab_207_no_bars_compare_only.png");
    let (_, accent2_with) = count_curve_pixels(&with_compare);
    let (_, accent2_none) = count_curve_pixels(&nothing);
    eprintln!(
        "#207 pixel margin LR-207e: {accent2_with} ACCENT_2 px with the compare series \
         (threshold >= {OVERLAY_DREW_MIN}); control {accent2_none} without (threshold < \
         {OVERLAY_DREW_MIN})"
    );
    assert!(
        accent2_with >= OVERLAY_DREW_MIN,
        "#207: a bar-less compare-only chart must draw its ACCENT_2 curve ({accent2_with} px, \
         expected >= {OVERLAY_DREW_MIN}). PNG: /tmp/lab_207_no_bars_compare_only.png"
    );
    assert!(
        accent2_none < OVERLAY_DREW_MIN,
        "negative control: no bars and no series must draw no ACCENT_2 curve ({accent2_none} px)"
    );
}

/// **LR3 — the operator path: the committed start/end-only report over bars
/// is pixel-identical to the bars-only frame (#205 + #207).** The REAL
/// committed v1 momentum report resolves through the production cold path
/// (`route_equity_overlay` over `[lab-runs/ (absent), evidence/]`, #205); it
/// is start/end-only with the ordinal `(0, 1)` timestamps, so over bars it
/// draws nothing and must change nothing (#207). The resolution is asserted
/// first, so the identity is never measured on a `None` overlay. This replaced
/// the #207 characterisation that held the LR3 key (which pinned the empty
/// gutter). PNG `/tmp/lab_207_committed_overlay.png`.
#[test]
fn lab_committed_start_end_only_report_over_bars_is_pixel_identical_to_bars_only() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = tmp.path().join("lab-runs");
    assert!(
        !lab_runs.exists(),
        "lab-runs/ is absent: evidence/ must answer"
    );
    let roots = [lab_runs, default_evidence_root()];
    let mut cache = EquityCache::new();
    let series = route_equity_overlay(&LabState::default(), &mut cache, &momentum_tuple(), &roots)
        .expect(
            "the committed v1 momentum report must resolve through the production roots (#205)",
        );
    assert_eq!(series.fidelity, Fidelity::StartEndOnly);
    let ts: Vec<i64> = series.samples.iter().map(|(t, _)| *t).collect();
    assert_eq!(
        ts,
        vec![0, 1],
        "start/end-only samples carry the ordinal (0, 1) timestamps"
    );

    let bars = render_bars();
    let bars_only = render_overlay(bars.clone(), None, vec![]);
    let shot = render_overlay(bars, Some(series), vec![]);
    dump_png(&shot, "/tmp/lab_207_committed_overlay.png");
    let differing = differing_pixels(&bars_only, &shot);
    eprintln!(
        "#207 pixel margin LR3: {differing} px differ from the bars-only frame (threshold == 0)"
    );
    assert!(
        differing == 0,
        "#207: the committed start/end-only report draws nothing over bars, so the frame must \
         equal the bars-only frame, but {differing} px differ (an empty equity gutter). PNG: \
         /tmp/lab_207_committed_overlay.png"
    );
}

// ── PHASE 5 — lab-compare-equity-overlay: REAL Compare-screen overlay (T3) ────
//
// The headline gate for THIS feature (R3 / AC1-AC2). PHASE 4 above proved the
// bare `chart::view` overlay widget rasterizes two series; here we prove the
// **real Compare-screen production path** does the same when driven by two
// persisted runs:
//
//   1. Write two `lab-runs/` reports + their COMPANION equity CSVs.
//   2. Scan them via `compare::cache::scan_report_roots` (the production
//      cold-boot path) — building `CachedCell`s whose new `equity_series_ts`
//      field is hydrated from the companion CSVs (the T1 wiring).
//   3. Install those cells in a `Cockpit`'s `compare_screen_state.cache`, select
//      both via `Message::CompareToggleOverlay` (the T2 ring), and render the
//      REAL `screens::compare::view` (its `overlay_panel` → `chart::view`).
//   4. Assert BOTH curves rasterize (ACCENT + ACCENT_2) in the chart band, and
//      a single-run contrast draws NO ACCENT_2.
//
// This exercises the EXACT screen code the operator sees — not a synthetic
// series, not the bare widget. The `CachedCell.equity_series_ts → LabEquitySeries
// → chart::view` thread is what this feature added; this test is its proof.

use reports::csv_artifacts::{EquitySample, write_equity_csv};
use ui::compare::state::{CachedCell, OverlaySlot};
use ui::lab::state::DateRange as CmpDateRange;
use ui::state::{StrategiesConfig, StrategyConfigEntry};
use ui::test_support::compare_screen_program;

/// Write a companion equity CSV (`<stem>-equity.csv`) beside a `lab-runs/`
/// report via the PRODUCTION `csv_artifacts::write_equity_csv` schema, carrying
/// a 6-point series that rises/dips/recovers shifted to `level` (so two runs
/// occupy disjoint y-bands → separate overlay pixels). Mirrors the offsets of
/// `lab_runs_report_at` so the two series have a comparable shape.
fn write_companion_equity_csv(
    lab_runs_root: &std::path::Path,
    slug: &str,
    md_fname: &str,
    level: i64,
) {
    let reports = lab_runs_root.join(slug).join("reports");
    std::fs::create_dir_all(&reports).expect("create lab-runs reports dir");
    let stem = md_fname.strip_suffix(".md").expect("md suffix");
    let csv_path = reports.join(format!("{stem}-equity.csv"));

    let p = |off: i64| -> Decimal { Decimal::from(level + off) };
    let offsets = [0i64, 3000, 1000, -3000, 4000, 8000];
    let samples: Vec<EquitySample> = offsets
        .iter()
        .enumerate()
        .map(|(i, off)| EquitySample {
            ts: Timestamp::new(
                time::OffsetDateTime::UNIX_EPOCH
                    + time::Duration::milliseconds(bar_ms(i as i64 * 10)),
            ),
            equity_total: p(*off),
            realized_pnl: Decimal::ZERO,
            unrealized_pnl: Decimal::ZERO,
            cash_balance: p(*off),
        })
        .collect();
    write_equity_csv(&csv_path, &samples).expect("write companion equity csv");
}

/// Build a Compare-screen `Cockpit` whose cache is hydrated by the PRODUCTION
/// `scan_report_roots` over the report root `root` holding a two-run scene
/// (reports + companion CSVs) — a `lab-runs/` root, or the `evidence/` root of
/// the bug-log #206 tests — with a populated `strategies_config` so the real
/// matrix renders.
fn compare_cockpit_from_two_run_scene(root: &std::path::Path) -> Cockpit {
    let (cache, _tally) = ui::compare::cache::scan_report_roots(&[root.to_path_buf()]);

    let mut c = Cockpit::new();
    c.current_screen = Screen::Compare;
    c.strategies_config = Some(StrategiesConfig {
        strategies: vec![
            StrategyConfigEntry {
                id: trading_core::StrategyId::new("top10_momentum_h1"),
                source_path: smol_str::SmolStr::new("config/strategies/top10_momentum_h1.toml"),
                params: vec![],
            },
            StrategyConfigEntry {
                id: trading_core::StrategyId::new("btc_sma_cross"),
                source_path: smol_str::SmolStr::new("config/strategies/btc_sma.toml"),
                params: vec![],
            },
        ],
    });
    c.compare_screen_state.cache = cache;
    c
}

/// The two overlay slots for the scene — run A (momentum/XRPUSDT, ACCENT) and
/// run B (sma/BTCUSDT, ACCENT_2). Keys match `scan_report_roots`' cell keys
/// (`strategy.id` from frontmatter, `DateRange::default()`).
fn scene_slots() -> (OverlaySlot, OverlaySlot) {
    let range = CmpDateRange::default();
    (
        (
            smol_str::SmolStr::new("top10_momentum_h1"),
            trading_core::Symbol::new("XRPUSDT"),
            range.clone(),
        ),
        (
            smol_str::SmolStr::new("btc_sma_cross"),
            trading_core::Symbol::new("BTCUSDT"),
            range,
        ),
    )
}

/// A `lab-runs/`-style report WITH a `strategy:` frontmatter block + a
/// `## Summary` KPI table — the shape `compare::cache::scan_report_roots`
/// requires (it skips reports without `strategy.id`). The per-bar series for
/// the overlay lives in the companion CSV, not the `.md` (the `.md`'s equity
/// section is a sparkline only). `{strategy_id}` keys the scanned `CachedCell`.
fn scan_report_with_strategy(
    scenario: &str,
    strategy_id: &str,
    strategy_kind: &str,
    level: i64,
) -> String {
    format!(
        r#"---
scenario: {scenario}
seed: 0xC0FFEE
generated: 2026-06-01T12:00:00Z
wall_clock_s: 0.0
data_source: synthetic
strategy:
  id: {strategy_id}
  kind: {strategy_kind}
  source: config/strategies/{strategy_id}.toml
---

# Backtest Report — {scenario}

## Summary

| Metric          | Value             |
|-----------------|-------------------|
| Scenario        | {scenario}        |
| Initial capital | ${level}.00 USDT  |
| Final equity    | ${level}.00 USDT  |
| Sharpe ratio    | **0.94**          |
| Total return    | **12.3 %**        |
| Max drawdown    | **-5.6 %**        |
| Trade count     | **42**            |
"#
    )
}

/// Write the full two-run scene: `.md` reports WITH strategy blocks (for the
/// production `scan_report_roots` KPI scan) + companion equity CSVs (for the
/// timestamped overlay series). Returns the `lab-runs/` root. Run A at the
/// ~100k level, run B at the ~60k level (disjoint y-bands).
fn write_two_run_scene_with_csvs(tmp: &tempfile::TempDir) -> std::path::PathBuf {
    let lab_runs = tmp.path().join("lab-runs");
    write_two_run_scene_at(&lab_runs);
    lab_runs
}

/// The two-run scene of [`write_two_run_scene_with_csvs`], the same bytes,
/// under any directory `root` (`<root>/<slug>/reports/` per run; bug-log #206
/// puts `root` one and two containers below an `evidence/` root). Returns the
/// two `.md` report paths, run A first.
fn write_two_run_scene_at(root: &std::path::Path) -> Vec<std::path::PathBuf> {
    // (slug, report name, scenario, strategy id, strategy kind, equity level)
    let runs = [
        // Run A — momentum/XRPUSDT @ 100k.
        (
            "v1-cross-sectional-momentum",
            "backtest-20260601-120000-top10-2024-h1-momentum.md",
            "top10-2024-h1-momentum",
            "top10_momentum_h1",
            "cross_sectional_momentum",
            100_000,
        ),
        // Run B — sma/BTCUSDT @ 60k.
        (
            "v0-paper-sma",
            "backtest-20260601-130000-btc-2023-1m-sma-cross.md",
            "btc-2023-1m-sma-cross",
            "btc_sma_cross",
            "sma_crossover",
            60_000,
        ),
    ];
    runs.iter()
        .map(|&(slug, fname, scenario, strategy_id, kind, level)| {
            write_lab_run(
                root,
                slug,
                fname,
                &scan_report_with_strategy(scenario, strategy_id, kind, level),
            );
            write_companion_equity_csv(root, slug, fname, level);
            root.join(slug).join("reports").join(fname)
        })
        .collect()
}

/// Crop band for the overlay CHART within the Compare screen body. The screen
/// stacks `toolbar → matrix(Fill) → overlay-panel(title + legend + 240px
/// chart)`; with `matrix = Length::Fill` the overlay panel sits at the BOTTOM.
/// We crop the lower ~250px where the 240px chart paints, excluding the matrix
/// cells' `+`/✓ chips above — so the curve-pixel classifier sees only the two
/// overlay polylines, never a selected cell's ACCENT/ACCENT_2 chip.
const CMP_CROP_Y0: u32 = VIEW_H.saturating_sub(250);

/// Count `(accent, accent2)` curve pixels inside the overlay chart band only
/// (rows `≥ CMP_CROP_Y0`) via the same winner-take-all classifier.
fn count_curve_pixels_chart_band(shot: &iced::window::Screenshot) -> (usize, usize) {
    let w = shot.size.width;
    let h = shot.size.height;
    let rgba: &[u8] = &shot.rgba;
    let (mut accent, mut accent2) = (0usize, 0usize);
    let y0 = CMP_CROP_Y0.min(h);
    for y in y0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            if idx + 2 >= rgba.len() {
                continue;
            }
            match classify_curve(rgba[idx], rgba[idx + 1], rgba[idx + 2]) {
                Curve::Accent => accent += 1,
                Curve::Accent2 => accent2 += 1,
                Curve::Neither => {}
            }
        }
    }
    (accent, accent2)
}

/// Render the real Compare screen body of `cockpit` and return its screenshot.
fn render_compare_screen(cockpit: Cockpit) -> iced::window::Screenshot {
    // Thread-safe atomic flag — no env data race, mirrors `render_live`.
    force_utc_once();
    let program = compare_screen_program(cockpit);
    let theme = iced::Theme::Dark;
    iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO)
}

/// Diagnostic (run with `--nocapture`): per-curve pixel counts in the overlay
/// chart band for the real Compare screen with two runs selected. Calibrates
/// the `OVERLAY_DREW_MIN` floor empirically against the cropped band.
#[test]
fn diag_compare_screen_overlay_pixel_counts() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = write_two_run_scene_with_csvs(&tmp);
    let mut c = compare_cockpit_from_two_run_scene(&lab_runs);
    let (slot_a, slot_b) = scene_slots();

    // Sanity: the cells exist AND carry hydrated timestamped series.
    let cell_a: &CachedCell = c
        .compare_screen_state
        .cache
        .get(&slot_a)
        .expect("run A cell present in scanned cache");
    let cell_b: &CachedCell = c
        .compare_screen_state
        .cache
        .get(&slot_b)
        .expect("run B cell present in scanned cache");
    eprintln!(
        "[diag] cell A series len={} ; cell B series len={}",
        cell_a.equity_series_ts.len(),
        cell_b.equity_series_ts.len()
    );

    // Select both through the production message path.
    update(&mut c, Message::CompareToggleOverlay(slot_a));
    update(&mut c, Message::CompareToggleOverlay(slot_b));

    let shot = render_compare_screen(c);
    let (accent, accent2) = count_curve_pixels_chart_band(&shot);
    eprintln!(
        "[diag] Compare-screen overlay chart band (y≥{CMP_CROP_Y0}): ACCENT={accent} ACCENT_2={accent2}"
    );
}

/// **T3 / AC1-AC2 (THE gate).** Two persisted runs selected in the REAL Compare
/// screen overlay BOTH rasterize — run A as the `ACCENT` primary curve, run B
/// as the `ACCENT_2` compare curve — when hydrated from companion-CSV-backed
/// `CachedCell`s through `screens::compare::view`'s `overlay_panel` → the
/// render-proven `chart::view` overlay. The `equity_series_ts` field (T1) +
/// the selection ring (T2) + the screen wiring are proven end-to-end here.
#[test]
fn compare_screen_two_run_overlay_renders_both_series() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = write_two_run_scene_with_csvs(&tmp);
    let mut c = compare_cockpit_from_two_run_scene(&lab_runs);
    let (slot_a, slot_b) = scene_slots();

    // Both cells carry a non-empty timestamped series (companion CSV hydrated).
    assert!(
        c.compare_screen_state
            .cache
            .get(&slot_a)
            .is_some_and(|cell| cell.equity_series_ts.len() >= 2),
        "run A cell must hydrate a ≥2-point timestamped series from its companion CSV"
    );
    assert!(
        c.compare_screen_state
            .cache
            .get(&slot_b)
            .is_some_and(|cell| cell.equity_series_ts.len() >= 2),
        "run B cell must hydrate a ≥2-point timestamped series from its companion CSV"
    );

    // Select both runs through the production update path (the T2 ring).
    update(&mut c, Message::CompareToggleOverlay(slot_a));
    update(&mut c, Message::CompareToggleOverlay(slot_b));
    assert_eq!(
        c.compare_screen_state.overlay_selection.len(),
        2,
        "both runs must be in the overlay selection ring"
    );

    let shot = render_compare_screen(c);
    let (accent, accent2) = count_curve_pixels_chart_band(&shot);

    // Run A (primary, ACCENT) drew on the real screen.
    assert!(
        accent >= OVERLAY_DREW_MIN,
        "Compare SCREEN overlay: the PRIMARY (ACCENT) run did NOT render: only {accent} ACCENT \
         pixels in the chart band (expected ≥ {OVERLAY_DREW_MIN}). The cell's hydrated \
         equity_series_ts did not reach chart::view through overlay_panel."
    );
    // Run B (compare, ACCENT_2) drew — two DISTINCT curves on one chart.
    assert!(
        accent2 >= OVERLAY_DREW_MIN,
        "Compare SCREEN overlay: the SECOND (ACCENT_2) run did NOT render: only {accent2} \
         ACCENT_2 pixels in the chart band (expected ≥ {OVERLAY_DREW_MIN}). The second selected \
         run's series was dropped or both curves collapsed to one color."
    );
}

/// **T3 contrast self-proof.** The SAME real Compare screen with only ONE run
/// selected draws the `ACCENT` primary curve but NO `ACCENT_2` — proving the
/// `ACCENT_2` count above genuinely comes from the second selected run, not
/// from the screen's chrome / matrix chips / AA bleed inside the chart band.
#[test]
fn compare_screen_single_run_overlay_draws_no_accent2() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = write_two_run_scene_with_csvs(&tmp);
    let mut c = compare_cockpit_from_two_run_scene(&lab_runs);
    let (slot_a, _slot_b) = scene_slots();

    // Select ONLY run A.
    update(&mut c, Message::CompareToggleOverlay(slot_a));
    assert_eq!(c.compare_screen_state.overlay_selection.len(), 1);

    let shot = render_compare_screen(c);
    let (accent, accent2) = count_curve_pixels_chart_band(&shot);

    // The primary curve drew…
    assert!(
        accent >= OVERLAY_DREW_MIN,
        "single-run Compare screen: the primary (ACCENT) curve must still render ({accent} \
         ACCENT pixels < {OVERLAY_DREW_MIN})"
    );
    // …and NO second curve.
    assert!(
        accent2 < OVERLAY_DREW_MIN,
        "single-run Compare screen overlay must draw NO second (ACCENT_2) curve, but saw \
         {accent2} ACCENT_2 pixels in the chart band (≥ {OVERLAY_DREW_MIN}). The two-curve \
         discriminator is reading chrome/chip/AA bleed — not the actual second run."
    );
}

// ── PHASE 4d — bug-log #206: Compare's scan walks container depth ≤ 1 ───────

/// CR1's PNG (read by a human; a failed save never fails the test).
const CR1_PNG: &str = "/tmp/compare_206_depth1.png";
/// CR2's PNG.
const CR2_PNG: &str = "/tmp/compare_206_depth2.png";

/// **CR1 — bug-log #206.** The two-run scene at `<tmp>/evidence/v1/<slug>/reports/`
/// (container depth 1, the committed layout since `1395520629dc`) is scanned
/// by the PRODUCTION `scan_report_roots` over the `evidence/` root, both runs
/// are selected on the real Compare screen, and both curves draw (`ACCENT` and
/// `ACCENT_2` each ≥ `OVERLAY_DREW_MIN` in the chart band). A cell missing
/// from the scan FAILS the precondition; it never skips. RED before the fix:
/// the scan walked `<root>/<x>/reports/` only, so the cache was empty.
#[test]
fn compare_screen_overlay_renders_runs_from_a_container_depth_1_root() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let evidence = tmp.path().join("evidence");
    write_two_run_scene_at(&evidence.join("v1"));
    let mut c = compare_cockpit_from_two_run_scene(&evidence);
    let (slot_a, slot_b) = scene_slots();
    for slot in [&slot_a, &slot_b] {
        let points = c
            .compare_screen_state
            .cache
            .get(slot)
            .map_or(0, |cell| cell.equity_series_ts.len());
        assert!(
            points >= 2,
            "the container-depth-1 run {slot:?} must scan into a cell with a >=2-point series \
             (got {points}) (#206)"
        );
    }
    update(&mut c, Message::CompareToggleOverlay(slot_a));
    update(&mut c, Message::CompareToggleOverlay(slot_b));
    let shot = render_compare_screen(c);
    dump_png(&shot, CR1_PNG);
    let (accent, accent2) = count_curve_pixels_chart_band(&shot);
    eprintln!(
        "#206 pixel margin CR1: ACCENT {accent} px, ACCENT_2 {accent2} px in the chart band \
         (threshold >= {OVERLAY_DREW_MIN} each)"
    );
    assert!(
        accent >= OVERLAY_DREW_MIN,
        "CR1: run A (ACCENT) from a container-depth-1 report drew only {accent} px (expected >= \
         {OVERLAY_DREW_MIN}) (#206). PNG: {CR1_PNG}"
    );
    assert!(
        accent2 >= OVERLAY_DREW_MIN,
        "CR1: run B (ACCENT_2) from a container-depth-1 report drew only {accent2} px (expected \
         >= {OVERLAY_DREW_MIN}) (#206). PNG: {CR1_PNG}"
    );
}

/// **CR2 — CR1's pair (bug-log #206).** The same scene one container deeper,
/// `<tmp>/evidence/v9/x/<slug>/reports/` (container depth 2, out of contract,
/// ADR-0055 D8): the files exist, the scan yields neither cell, and with both
/// slots selected neither curve draws (each colour < `OVERLAY_DREW_MIN`).
#[test]
fn compare_screen_overlay_draws_nothing_from_a_container_depth_2_root() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let evidence = tmp.path().join("evidence");
    for report in write_two_run_scene_at(&evidence.join("v9").join("x")) {
        assert!(
            report.is_file(),
            "the container-depth-2 fixture {} must exist, so the negative is not 'file missing'",
            report.display()
        );
    }
    let mut c = compare_cockpit_from_two_run_scene(&evidence);
    let (slot_a, slot_b) = scene_slots();
    for slot in [&slot_a, &slot_b] {
        assert!(
            !c.compare_screen_state.cache.contains_key(slot),
            "a container-depth-2 run {slot:?} must not scan into a cell (ADR-0055 D8)"
        );
    }
    update(&mut c, Message::CompareToggleOverlay(slot_a));
    update(&mut c, Message::CompareToggleOverlay(slot_b));
    let shot = render_compare_screen(c);
    dump_png(&shot, CR2_PNG);
    let (accent, accent2) = count_curve_pixels_chart_band(&shot);
    eprintln!(
        "#206 pixel margin CR2: ACCENT {accent} px, ACCENT_2 {accent2} px in the chart band \
         (threshold < {OVERLAY_DREW_MIN} each)"
    );
    assert!(
        accent < OVERLAY_DREW_MIN && accent2 < OVERLAY_DREW_MIN,
        "CR2: nothing may draw from a container-depth-2 report, but ACCENT {accent} px and \
         ACCENT_2 {accent2} px (threshold < {OVERLAY_DREW_MIN}). PNG: {CR2_PNG}"
    );
}

// ── PHASE 4e — bug-log #214: a long row label stays inside its column ───────

use ui::test_support::{COMPARE_CELL_H, COMPARE_ROW_LABEL_PADDING, COMPARE_ROW_LABEL_W};

/// The subject: `fake_strategies_config()`'s id that the `#206` AD-10 PNG
/// showed running over its `BTCUSDT` Sharpe (142.3 px at 12 px, MEASURED from
/// the embedded Inter's advance widths, against a 104 px content box).
const LONG_ID: &str = "btc_bbands_mean_revert";
/// The reference id: the same `btc` universe (so the same one-column matrix)
/// and one line wide.
const SHORT_ID: &str = "btc_bb";
/// ML3's id: 68 characters, more than the content box's three lines hold.
const OVERLONG_ID: &str = "btc_bbands_mean_revert_20_2_with_volume_filter_and_atr_stop_v2_tuned";
/// The renamed row's Sharpe: the sighting's value, and the only `DOWN_500`
/// value on screen (the other two rows are `UP_500`).
const LONG_ROW_SHARPE: f64 = -32.89;

/// ML1 and ML2's PNGs (read by a human; a failed save never fails the test).
const ML_LONG_PNG: &str = "/tmp/compare_214_long.png";
/// The reference frame S.
const ML_SHORT_PNG: &str = "/tmp/compare_214_short.png";
/// ML3's PNG.
const ML_OVERLONG_PNG: &str = "/tmp/compare_214_overlong.png";

/// New for #214. A floor, not a calibration: the fewest `DOWN_500`-class
/// pixels (by [`is_down_500`]) that prove `-32.89` drew in the value column.
/// The value is six glyphs; a frame without it holds no `DOWN_500` text at all
/// (the other cells are `UP_500` and the labels `FG_2`), so 10 is clear of
/// zero yet far below a drawn value (the measured count is printed as
/// `#214 pixel margin ML1`).
const KPI_DREW_MIN: usize = 10;

/// Max squared RGB distance for a pixel to count as a token colour: the same
/// ≈30-per-channel ball as [`CURVE_MATCH_R2`], so only near-full-coverage
/// text pixels match, never an antialiased blend with the `PANEL` background.
const TEXT_MATCH_R2: i32 = CURVE_MATCH_R2;

/// A token's dark-mode colour as 8-bit RGB.
fn token_rgb(token: ui::theme::color::ModeColor) -> (i32, i32, i32) {
    let c = token.current(ThemeMode::Dark);
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}

/// `true` for a pixel within [`TEXT_MATCH_R2`] of `DOWN_500` (clay), the
/// negative-Sharpe KPI colour (tolerance classifier, the `classify_curve`
/// pattern).
fn is_down_500(p: &[u8]) -> bool {
    let q = (i32::from(p[0]), i32::from(p[1]), i32::from(p[2]));
    dist2(q, token_rgb(color::DOWN_500)) <= TEXT_MATCH_R2
}

/// `true` for a pixel within [`TEXT_MATCH_R2`] of `FG_2`, the row label's
/// colour: the entry's negative control (label-colour pixels in a value cell).
fn is_fg_2(p: &[u8]) -> bool {
    let q = (i32::from(p[0]), i32::from(p[1]), i32::from(p[2]));
    dist2(q, token_rgb(color::FG_2)) <= TEXT_MATCH_R2
}

/// `true` for a pixel visibly off the `PANEL` background (farther than
/// [`TEXT_MATCH_R2`]): ink of some kind, never a background pixel.
fn is_ink(p: &[u8]) -> bool {
    let q = (i32::from(p[0]), i32::from(p[1]), i32::from(p[2]));
    dist2(q, token_rgb(color::PANEL)) > TEXT_MATCH_R2
}

/// The label column's right edge in physical px, `M + ROW_LABEL_W` (132): the
/// Compare screen body pads the matrix by `space::M` (`screens/compare.rs`),
/// and every value cell starts at or right of it.
fn label_column_right() -> u32 {
    ((ui::theme::space::M as f32 + COMPARE_ROW_LABEL_W) * SCALE) as u32
}

/// The label content box's right edge in physical px (124): the column's
/// right edge less the row header's horizontal padding.
fn content_box_right() -> u32 {
    label_column_right() - (f32::from(COMPARE_ROW_LABEL_PADDING[1]) * SCALE) as u32
}

/// The label content box's height in physical px (48): the row height less
/// the vertical padding on both sides, three `SMALL` lines.
fn content_box_h() -> f32 {
    (COMPARE_CELL_H - 2.0 * f32::from(COMPARE_ROW_LABEL_PADDING[0])) * SCALE
}

/// One `SMALL` line in physical px: iced's default line height at
/// `text::SMALL`, 1.3 × 12 = 15.6.
fn one_line_h() -> f32 {
    iced::widget::text::LineHeight::default()
        .to_absolute(iced::Pixels(ui::theme::text::SMALL as f32))
        .0
        * SCALE
}

/// The Compare screen with `fake_strategies_config()` (the subject) whose
/// `btc_bbands_mean_revert` row is renamed to `id`. Asserts the config still
/// holds that id first, so a renamed fixture fails here rather than testing
/// something else. One cached `BTCUSDT` cell per row (`has_series` false, so
/// no overlay chip): `btc_macd_trend` 1.42 and `btc_rsi_reversion` 0.87, both
/// `UP_500`, and the renamed row the sighting's `-32.89`, the only `DOWN_500`.
fn label_scene(id: &str) -> Cockpit {
    let mut config = ui::fixtures::fake_strategies_config();
    let entry = config
        .strategies
        .iter_mut()
        .find(|s| s.id.0.as_str() == LONG_ID)
        .expect("fake_strategies_config() must still hold btc_bbands_mean_revert, #214's subject");
    entry.id = trading_core::StrategyId::new(id);

    let mut c = Cockpit::new();
    c.current_screen = Screen::Compare;
    let range = c.compare_screen_state.range.clone();
    for (strategy, sharpe) in [
        ("btc_macd_trend", 1.42),
        ("btc_rsi_reversion", 0.87),
        (id, LONG_ROW_SHARPE),
    ] {
        c.compare_screen_state.cache.insert(
            (
                smol_str::SmolStr::new(strategy),
                trading_core::Symbol::new("BTCUSDT"),
                range.clone(),
            ),
            CachedCell {
                sharpe,
                total_return_pct: 0.0,
                max_drawdown_pct: 0.0,
                trade_count: 0,
                equity_curve_tail: Vec::new(),
                equity_series_ts: Vec::new(),
                source_report_path: smol_str::SmolStr::new(
                    "evidence/v1/x/reports/backtest-20260601-120000-x.md",
                ),
                generated_at: smol_str::SmolStr::new("2026-06-01T12:00:00Z"),
                is_multi_symbol: false,
            },
        );
    }
    c.strategies_config = Some(config);
    c
}

/// How tall `id` is in physical px with nothing to cut it: iced's own
/// paragraph (the one the `Text` widget lays out) at the row label's content
/// width (`ROW_LABEL_W` less both horizontal paddings, 104 px), its size
/// (`text::SMALL`), font (the embedded Inter), wrap (`WordOrGlyph`) and the
/// widget's other defaults, with an unbounded height.
fn unbounded_label_h(id: &str) -> f32 {
    use iced::advanced::text::Paragraph as _;
    let content_w = COMPARE_ROW_LABEL_W - 2.0 * f32::from(COMPARE_ROW_LABEL_PADDING[1]);
    let paragraph = iced::advanced::graphics::text::Paragraph::with_text(iced::advanced::Text {
        content: id,
        bounds: iced::Size::new(content_w, f32::INFINITY),
        size: iced::Pixels(ui::theme::text::SMALL as f32),
        line_height: iced::widget::text::LineHeight::default(),
        font: ui::theme::font::embedded(),
        align_x: iced::advanced::text::Alignment::Default,
        align_y: iced::alignment::Vertical::Top,
        shaping: iced::widget::text::Shaping::default(),
        wrapping: iced::widget::text::Wrapping::WordOrGlyph,
    });
    paragraph.min_bounds().height * SCALE
}

/// Count the pixels at `x0 <= x < x1` (all rows) for which `pred` holds.
fn count_in_columns(
    shot: &iced::window::Screenshot,
    (x0, x1): (u32, u32),
    pred: impl Fn(&[u8]) -> bool,
) -> usize {
    let w = shot.size.width;
    shot.rgba
        .chunks_exact(4)
        .enumerate()
        .filter(|(i, p)| {
            let x = (*i as u32) % w;
            x >= x0 && x < x1 && pred(p)
        })
        .count()
}

/// Count the pixels at `x0 <= x < x1` (all rows) where `a` and `b` differ and
/// `a`'s pixel satisfies `pred`.
fn count_differing_in_columns(
    a: &iced::window::Screenshot,
    b: &iced::window::Screenshot,
    (x0, x1): (u32, u32),
    pred: impl Fn(&[u8]) -> bool,
) -> usize {
    assert_eq!(
        a.size, b.size,
        "frames must have the same size to be compared"
    );
    let w = a.size.width;
    a.rgba
        .chunks_exact(4)
        .zip(b.rgba.chunks_exact(4))
        .enumerate()
        .filter(|(i, (p, q))| {
            let x = (*i as u32) % w;
            x >= x0 && x < x1 && p != q && pred(p)
        })
        .count()
}

/// A half-open pixel rectangle `[x0, x1) × [y0, y1)`.
#[derive(Debug, Clone, Copy)]
struct PxRect {
    x0: u32,
    y0: u32,
    x1: u32,
    y1: u32,
}

impl PxRect {
    fn height(self) -> u32 {
        self.y1 - self.y0
    }
}

/// The bounding box of the pixels that differ between two same-size frames,
/// or `None` when no pixel differs (the caller fails on `None`: an empty diff
/// must never satisfy an upper bound).
fn diff_bbox(a: &iced::window::Screenshot, b: &iced::window::Screenshot) -> Option<PxRect> {
    assert_eq!(
        a.size, b.size,
        "frames must have the same size to be compared"
    );
    let w = a.size.width;
    let mut rect: Option<PxRect> = None;
    for (i, (p, q)) in a
        .rgba
        .chunks_exact(4)
        .zip(b.rgba.chunks_exact(4))
        .enumerate()
    {
        if p == q {
            continue;
        }
        let (x, y) = ((i as u32) % w, (i as u32) / w);
        rect = Some(match rect {
            None => PxRect {
                x0: x,
                y0: y,
                x1: x + 1,
                y1: y + 1,
            },
            Some(r) => PxRect {
                x0: r.x0.min(x),
                y0: r.y0.min(y),
                x1: r.x1.max(x + 1),
                y1: r.y1.max(y + 1),
            },
        });
    }
    rect
}

/// **ML1 — bug-log #214 (the bug).** `btc_bbands_mean_revert` (frame L) and
/// the one-line `btc_bb` (frame S) render the same Compare screen except for
/// that row's label, so no pixel may differ at `x >= M + ROW_LABEL_W` (132):
/// the long label stays in its column and leaves the value cells untouched.
/// Preconditions, so the zero cannot be vacuous: `-32.89` drew in the value
/// column of both frames (≥ `KPI_DREW_MIN` `DOWN_500` pixels at x ≥ 132; it
/// is the only `DOWN_500` text on screen, so a frame without that cell has
/// 0); and the long label drew (L adds at least one ink pixel per glyph it has
/// beyond S, inside the label column). RED before the fix: the label ran to
/// x ≈ 162 and covered `-32.89`.
#[test]
fn compare_matrix_long_row_label_leaves_the_value_columns_untouched() {
    let shot_l = render_compare_screen(label_scene(LONG_ID));
    let shot_s = render_compare_screen(label_scene(SHORT_ID));
    dump_png(&shot_l, ML_LONG_PNG);
    dump_png(&shot_s, ML_SHORT_PNG);

    let col_right = label_column_right();
    let values = (col_right, shot_l.size.width);
    let label_col = (ui::theme::space::M, col_right);

    let down_l = count_in_columns(&shot_l, values, is_down_500);
    let down_s = count_in_columns(&shot_s, values, is_down_500);
    let added_glyphs = LONG_ID.chars().count() - SHORT_ID.chars().count();
    let label_added_ink = count_differing_in_columns(&shot_l, &shot_s, label_col, is_ink);
    let differing = count_differing_in_columns(&shot_l, &shot_s, values, |_| true);
    let fg2_l = count_in_columns(&shot_l, values, is_fg_2);
    let fg2_s = count_in_columns(&shot_s, values, is_fg_2);
    eprintln!(
        "#214 pixel margin ML1: {differing} px differ at x >= {col_right} (expected 0); FG_2-class \
         px there: L {fg2_l}, S {fg2_s}; DOWN_500 px there: L {down_l}, S {down_s} (floor \
         {KPI_DREW_MIN}); the long label's added ink in x {}..{col_right}: {label_added_ink} px \
         (floor {added_glyphs}, one per added glyph)",
        ui::theme::space::M
    );
    assert!(
        down_l >= KPI_DREW_MIN && down_s >= KPI_DREW_MIN,
        "ML1 precondition: -32.89 must draw in the value column of both frames (DOWN_500 px at x \
         >= {col_right}: L {down_l}, S {down_s}; floor {KPI_DREW_MIN}) (#214). PNGs: {ML_LONG_PNG}, \
         {ML_SHORT_PNG}"
    );
    assert!(
        label_added_ink >= added_glyphs,
        "ML1 precondition: the long label must draw in its column (L adds {label_added_ink} ink px \
         over S in x {}..{col_right}; floor {added_glyphs}) (#214). PNGs: {ML_LONG_PNG}, \
         {ML_SHORT_PNG}",
        ui::theme::space::M
    );
    assert_eq!(
        differing, 0,
        "ML1: the long row label changed {differing} px at x >= {col_right}, the value columns \
         ({fg2_l} FG_2-class px there in L, {fg2_s} in S) (#214). PNGs: {ML_LONG_PNG}, \
         {ML_SHORT_PNG}"
    );
}

/// **ML2 — bug-log #214 (the full id is on screen).** The same L and S: the
/// box around the pixels that differ (L's extra glyphs) is taller than one
/// `SMALL` line (15.6 px), so the id went on to a second line instead of
/// being cut at the column. One line's ink fits inside Inter's ascent plus
/// descent (14.5 px at 12 px), so a single line cannot pass.
#[test]
fn compare_matrix_long_row_label_wraps_onto_a_second_line() {
    let shot_l = render_compare_screen(label_scene(LONG_ID));
    let shot_s = render_compare_screen(label_scene(SHORT_ID));
    let line = one_line_h();
    let bbox = diff_bbox(&shot_l, &shot_s).expect(
        "ML2: btc_bbands_mean_revert and btc_bb must render differently: no pixel differs (#214)",
    );
    eprintln!(
        "#214 pixel margin ML2: the diff box {bbox:?} is {} px tall (expected > one line, {line:.1} \
         px)",
        bbox.height()
    );
    assert!(
        bbox.height() as f32 > line,
        "ML2: the long label's diff box {bbox:?} is {} px tall, not more than one {line:.1} px \
         line: the id did not wrap (#214). PNGs: {ML_LONG_PNG}, {ML_SHORT_PNG}",
        bbox.height()
    );
}

/// **ML3 — bug-log #214 (the clip backstop).** A 68-character id against S.
///
/// Precondition, derived rather than read off a frame: iced's own paragraph
/// ([`unbounded_label_h`]) lays the id out on more lines than the 48 px content
/// box holds (5 against 3), so the clip has at least one line to cut. Every
/// line whose top lies inside the box is laid out and drawn (cosmic-text drops
/// only lines whose top is below the 48 px buffer), so the fourth line, whose
/// top is at 46.8 px and whose ink starts about 2 px below the box, reaches
/// the clip; without the clip the diff box is 59 px tall (P214-2, measured).
///
/// Assertions: the diff box ends at or left of the content box's right edge
/// (124), and it is at most the box's 48 px tall. That height is measured from
/// the first line's ink top, about 3 px below the box's top, so the label may
/// reach at most 3 px into the 6 px bottom padding but never leaves its cell.
/// The box is also taller than two lines (31.2 px): the widget drew the id
/// wrapped, so an empty or one-line label cannot meet the two bounds.
#[test]
fn compare_matrix_overlong_row_label_stays_inside_its_content_box() {
    let (line, box_h) = (one_line_h(), content_box_h());
    let needed_h = unbounded_label_h(OVERLONG_ID);
    let lines_needed = (needed_h / line).round() as u32;
    let lines_held = (box_h / line).floor() as u32;
    let shot_o = render_compare_screen(label_scene(OVERLONG_ID));
    let shot_s = render_compare_screen(label_scene(SHORT_ID));
    dump_png(&shot_o, ML_OVERLONG_PNG);
    let (right, two_lines) = (content_box_right(), 2.0 * line);
    let bbox = diff_bbox(&shot_o, &shot_s)
        .expect("ML3: the overlong id and btc_bb must render differently: no pixel differs (#214)");
    eprintln!(
        "#214 pixel margin ML3: the id needs {lines_needed} lines ({needed_h:.1} px unbounded) \
         against the {lines_held} its {box_h} px content box holds (expected more); the diff box \
         {bbox:?} is {} px tall (expected > {two_lines:.1} and <= {box_h}), right edge {} \
         (expected <= {right})",
        bbox.height(),
        bbox.x1
    );
    assert!(
        lines_needed > lines_held,
        "ML3 precondition: iced lays the overlong id out on {lines_needed} line(s) \
         ({needed_h:.1} px), no more than the {lines_held} its {box_h} px content box holds, so \
         the clip has nothing to cut and this test cannot show it (#214)"
    );
    assert!(
        bbox.x1 <= right,
        "ML3: the overlong label drew to x = {} past its content box's right edge {right} (#214). \
         PNG: {ML_OVERLONG_PNG}",
        bbox.x1
    );
    assert!(
        bbox.height() as f32 <= box_h,
        "ML3: the overlong label's diff box {bbox:?} is {} px tall, past its {box_h} px content \
         box (#214). PNG: {ML_OVERLONG_PNG}",
        bbox.height()
    );
    assert!(
        bbox.height() as f32 > two_lines,
        "ML3: the overlong label's diff box {bbox:?} is only {} px tall, not past two lines \
         ({two_lines:.1} px): the widget did not draw the id wrapped past two lines (#214). PNG: \
         {ML_OVERLONG_PNG}",
        bbox.height()
    );
}
