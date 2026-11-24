//! Render-layer proof (project law `feedback_verify_ui_at_render_layer` +
//! `spec/dev-notes/iced-ui-render-verification.md`): the Lab screen's per-bar
//! buy/sell **volume bars** line up vertically UNDER the price chart's buy/sell
//! **triangle markers** for the SAME fills.
//!
//! ## The bug (lab-buy-sell-overlay-align, 2026-06-24)
//!
//! The operator: *"the Buy and Sell overlays are not over the triangles but the
//! overlay is in the bottom of the view."* The "overlay at the bottom" is the
//! `volume_histogram` strip (green buy bars up, clay sell bars down). It painted
//! into `canvas_chart::inner_rect` (symmetric `space::S` gutter, left origin 8)
//! while the chart paints into `chart_inner_rect` (a 48-px LEFT price-axis gutter
//! → left origin 56, plus a right gutter). So bar `i`'s triangle and bar `i`'s
//! volume bar landed at DIFFERENT x — confirmed at the pixel layer: the leftmost
//! fill was ~27 px off in a 1280-px render. The fix gives the histogram the
//! chart's HORIZONTAL plot geometry + the chart's `x_for_index` per-bar mapping
//! (`volume_histogram::view_aligned`), so each volume bar sits under its
//! triangle.
//!
//! ## What this proves
//!
//! Rasterizes the REAL full Lab body (`shell::view` → `lab::view` → the real
//! `chart` + `volume_histogram` widgets) via `iced_test::screenshot`, then
//! measures the x-centroid of each buy/sell triangle (chart band) and each
//! buy/sell volume bar (histogram band) and asserts they align within a tight
//! tolerance — a RED-before / GREEN-after guard. NEGATIVE control: a long-only
//! (buys-only) seed paints buy bars but no sell bars in the histogram band.
//!
//! ## bug-log #208 — a cached miss stays a miss until a Lab run completes
//!
//! `lab::equity_loader::EquityCache` caches each tuple's outcome, a hit (the
//! series) or a miss (the error), so a tuple that resolves nothing costs one
//! walk of the report roots, not one per `view`. `state::update` empties the
//! whole cache on every `Message::LabRunCompleted`, Ok and Err (the engine
//! writes the `.md` before its companion CSV). `lab::view` reads the roots
//! from `Cockpit::lab_report_roots`, resolved once at construction. Tests:
//!
//! - **(a)** `lab::equity_loader::tests::cached_miss_is_not_re_read_until_clear`
//!   — after a miss, a matching report is written: the same cache returns the
//!   same error, a fresh cache resolves the report (the control), and after
//!   `clear()` the original cache resolves it. RED at `141f9bd7`, before the
//!   fix (the second lookup resolved the report).
//! - **(d)** `lab::equity_loader::tests::clear_replaces_a_stale_committed_hit_with_the_newer_lab_runs_report`
//!   — a cached committed hit (2 points) is returned as is (the same `Arc`)
//!   after a `lab-runs/` report (5 points) is written; after `clear()` the
//!   `lab-runs/` report wins.
//! - **(b)** `state::tests::lab_run_completed_ok_clears_equity_cache_hits_and_misses`
//!   and **(c)** `state::tests::lab_run_completed_err_clears_equity_cache_too`
//!   — through `state::update`: a committed hit (first equity 100,000) and a
//!   miss both survive `LabRunProgressDone` (the control) after newer
//!   `lab-runs/` reports (200,000) are written; `LabRunCompleted(Ok)` (b) or
//!   `LabRunCompleted(Err)` (c) drops both.
//! - **seam** `state::tests::cockpit_lab_report_roots_are_the_production_union`
//!   — `Cockpit::default()`, `new()` and `ready(…)` hold
//!   `default_report_roots()`.
//! - **LR-208** [`lab_cached_miss_hides_a_new_report_until_a_lab_run_completes_then_draws_it`]
//!   — the full Lab screen: F0 (the miss, seeded through
//!   `ui::screens::lab::view`) and F1 (after an in-window report is written)
//!   are byte-identical; after `ui::state::update(LabRunCompleted(Ok))`, F2
//!   draws the report and the plot's right edge moves left by the derived
//!   `EQUITY_GUTTER_WIDENING_PX` (40 px) within `[34, 44]`. It prints `#208
//!   pixel margin …`. PNGs (with `UI_DUMP_PNG`) `/tmp/lab_208_F0.png`,
//!   `/tmp/lab_208_F1.png`, `/tmp/lab_208_F2.png`.
//!
//! ### T-T4 probe table (bug-log #208)
//!
//! The protocol of the #205 and #207 tables in `live_equity_render.rs`; paths
//! under `crates/ui/src/`; each OLD occurs exactly once in its file. "lib" =
//! `cargo test -p ui --lib` (691 tests when Q1–Q6 were measured; 692 since
//! `cached_miss_is_not_re_read_until_clear_under_evidence`); "pixel" = this
//! file (5, then 6 with LR-208e) and `live_equity_render.rs` (23). "measured"
//! = applied and run on 2026-11-16 (lib and both pixel files); "REASONED" =
//! derived from the code. (a-ev) = `cached_miss_is_not_re_read_until_clear_under_evidence`;
//! LR-208e = `lab_cached_miss_under_evidence_hides_a_new_report_until_a_lab_run_completes`.
//! Rows Q1–Q6 were measured before (a-ev) and LR-208e existed; their cells for
//! those two are REASONED until the orchestrator's gate measures them.
//!
//! | Probe | File / symbol | OLD → NEW | RED | GREEN | Measured |
//! |---|---|---|---|---|---|
//! | Q1 no negative cache | `lab/equity_loader.rs` `EquityCache::get_or_load_roots` | `        self.by_tuple.insert(tuple.clone(), resolved.clone());` → `        if resolved.is_ok() { self.by_tuple.insert(tuple.clone(), resolved.clone()); }` | lib: (a) (its second lookup resolves the report), (b) and (c) (before the run completes, the miss resolves); pixel: LR-208 (F0 vs F1: 19,025 px differ; then shift 0); (a-ev) and LR-208e (REASONED) | lib 688 of 691, among them (d), `cache_hit_returns_same_arc`, `cache_clear_re_reads`, the `route_overlay_*` tests and the seam test; pixel: the other 4 of 5 here (LR-207d among them), 23 of 23 in `live_equity_render.rs` | measured |
//! | Q2 clear unwired | `state.rs` `update`, the `Message::LabRunCompleted` arm | `            model.equity_cache.get_mut().clear();` → `            let _ = &model.equity_cache;` | lib: (b), (c) (after the run, the hit still reads 100,000); pixel: LR-208 (F0 vs F1 0 px; right-edge shift 0); LR-208e (REASONED) | lib 689 of 691, among them (a), (d) and `compare_cold_boot_index_gating`; pixel: the other 4 of 5 here, 23 of 23 in `live_equity_render.rs` | measured |
//! | Q3 clear keeps misses | `lab/equity_loader.rs` `EquityCache::clear` | `        self.by_tuple.clear();` → `        self.by_tuple.retain(\|_, c\| c.is_err());` | (a); (b) and (c) at the miss after the run; LR-208 (shift 0); (a-ev), LR-208e (REASONED) | (d), `cache_clear_re_reads` | measured by the falsifier (lib 688 of 691, this file 4 of 5, 23 of 23) |
//! | Q4 clear keeps hits | same | `        self.by_tuple.clear();` → `        self.by_tuple.retain(\|_, c\| c.is_ok());` | (d), `cache_clear_re_reads`; (b) and (c) at the hit after the run | (a), (a-ev), LR-208, LR-208e (REASONED) | measured by the falsifier (lib 687 of 691, this file 5 of 5, 23 of 23) |
//! | Q5 Ok-only clear | `state.rs` `update`, the `Message::LabRunCompleted` arm | `            model.equity_cache.get_mut().clear();` → `            if outcome.is_ok() { model.equity_cache.get_mut().clear(); }` | (c) only | (b), LR-208, LR-208e, the rest of lib | measured by the falsifier (lib 690 of 691, this file 5 of 5, 23 of 23) |
//! | Q6 root seam bypassed | `screens/lab.rs` `view` | `        let roots = &model.lab_report_roots;` → `        let roots = &crate::lab::equity_loader::default_report_roots();` | LR-208, LR-208e (REASONED) (the real roots hold no in-window report for its tuple: F2 draws what F1 drew, shift 0) | every lib test ((b) and (c) read `Cockpit::lab_report_roots` directly), LR-207d (its pre-warmed cache is keyed by tuple, not root), the rest of both pixel files | measured by the falsifier (lib 691 of 691, this file 4 of 5, 23 of 23) |
//! | Q7 a cached miss re-walks the last root | `lab/equity_loader.rs` `EquityCache::get_or_load_roots` | `        if let Some(cached) = self.by_tuple.get(tuple) {` → `        if let Some(cached) = self.by_tuple.get(tuple).filter(\|c\| c.is_ok() \|\| roots.last().is_none_or(\|l\| load_equity(tuple, l).is_err())) {` | (a-ev) (its second lookup resolves the report under `evidence/`); LR-208e (F0 vs F1 differ) | (a), (b), (c), (d), LR-208 (their late reports sit under `lab-runs/`) | REASONED (the falsifier's multi-line form survived before (a-ev) and LR-208e existed) |
//! | Q8 clear only the Lab's own tuple | `lab/equity_loader.rs` `EquityCache::clear` AND `state.rs` the `Message::LabRunCompleted` arm (two edits) | `    pub fn clear(&mut self) {` → `    pub fn remove(&mut self, t: &LabTuple) { self.by_tuple.remove(t); } pub fn clear(&mut self) {` AND `            model.equity_cache.get_mut().clear();` → `            if let (Some(s), Some((v, sym))) = (&model.lab_state.strategy, &model.lab_state.pair) { model.equity_cache.get_mut().remove(&crate::lab::equity_loader::LabTuple::new(s, *v, sym, model.lab_state.range.clone(), model.lab_state.data_source)); } else { model.equity_cache.get_mut().clear(); }` | (b), (c) (the Lab shows a third tuple when the run completes, so T_hit keeps its stale hit) | (a), (a-ev), (d), LR-208, LR-208e (the run's tuple is the missed tuple) | REASONED (the falsifier's form survived before (b)/(c) set a third tuple) |
//! | Q9 seam empty in `boot` | `state.rs` `Cockpit::boot` | `            session_log: open_session_log(),` → `            session_log: open_session_log(), lab_report_roots: Vec::new(),` | `cockpit_lab_report_roots_are_the_production_union` (its `boot` half) | every other test | REASONED (survived before the seam test covered `boot`) |
//! | Q10 view passes only the first root | `screens/lab.rs` `view` | `        let roots = &model.lab_report_roots;` → `        let roots: &[std::path::PathBuf] = model.lab_report_roots.get(..1).unwrap_or(&[]);` | LR-208e (F2 cannot read `evidence/`: shift 0) | every lib test, LR-208 (its report is under `lab-runs/`, the first root), LR-207d | REASONED (survived before LR-208e existed) |
//!
//! ## bug-log #211 — an overlay with one sample in the window is no overlay
//!
//! The chart now strokes only a series' in-window samples, and a series with
//! fewer than two of them (`widgets::chart::MIN_DRAWN_SAMPLES`) neither scales
//! the axis nor reserves the gutter; the volume histogram follows through
//! `chart::equity_axis_shown`. The full write-up, the lib tests and the probe
//! table are in `live_equity_render.rs` ("bug-log #211").
//!
//! - **LR-211d** [`lab_h1_2024_run_over_the_seeded_hour_matches_the_no_overlay_lab`]
//!   — the LR-207d Lab (v1 momentum on BTCUSDT, `H1_2024` since #212) with its
//!   equity cache pre-warmed from an `H1_2024` run as it is persisted (scene
//!   B's report plus a companion `-equity.csv` of the first 4,344 hourly
//!   samples from 2024-01-01, `ui::test_support::h1_2024_hourly_samples`; a
//!   real momentum run has 10 · 4,344 + 1, `#226`), so exactly one
//!   sample (12:00) is inside the seeded bar window. Scene C must be
//!   byte-identical to scene A (the committed start/end-only report, which
//!   draws no overlay): the chart keeps its full width and so does the
//!   histogram. Control: scene B (in-window) differs from A. Measured
//!   2026-11-24: before the fix C differed from A in 17,437 px (the gutter, a
//!   1.1 USDT axis and a stroke up the plot's left edge); after it, 0; B vs A
//!   19,025 px both times. It prints `#211 pixel margin …`. PNGs (with
//!   `UI_DUMP_PNG`) `/tmp/lab_211_full_no_overlay.png` (A),
//!   `/tmp/lab_211_full_in_window.png` (B), `/tmp/lab_211_full_h1_2024.png`
//!   (C). Probes (`live_equity_render.rs`' P11 table): P11-1, P11-3 and P11-7
//!   turn it red; the #207 probes P7-2, P7-5 and P7-8 too.
//!
//! ## bug-log #212 — the Lab draws only a report of its own tuple
//!
//! The Lab's report lookup (`lab::equity_loader`) took no strategy, accepted
//! every coin a report did not name in a one-entry `## Universe`, let
//! `Last30d`, `Last90d` and `Custom` accept every report of the slug, and kept
//! a second, shorter copy of the engine's slug map. A report now answers a
//! tuple only through its strategy (`backtest::engine::report_strategy_id` of
//! its `id:`), its coin (Summary `| Symbol |` row, else `## Universe`, else a
//! scenario ticker) and its scenario's span token, and the slug is the
//! engine's `strategy_dir_slug` (ADR-0055 D9). Tests:
//!
//! - **Lib** (`lab::equity_loader`): M1
//!   `a_coin_named_only_in_the_summary_row_matches_that_coin_only`, M2
//!   `a_multi_symbol_report_matches_only_a_coin_in_its_universe`, M3
//!   `a_report_naming_no_coin_matches_no_coin`, M4
//!   `a_report_matches_only_its_own_strategy`, M5
//!   `a_report_without_a_strategy_id_matches_no_strategy`, M6
//!   `range_tokens_match_exactly`, M7 (E4c, re-derived on `H1_2024`)
//!   `a_strictly_better_report_in_the_second_report_dir_wins`, M8
//!   `every_lab_chip_reads_the_dir_the_engine_writes`, and
//!   `strategy_slug_mapping`. `backtest::engine`: `strategy_dir_slug_known_ids`,
//!   `report_strategy_id_is_what_each_writer_emits` (E-ID),
//!   `cross_sectional_lab_scenario_names_carry_range_and_source`.
//! - **End to end** (`lab_run_engine.rs`, both CI legs):
//!   `inner::every_lab_chip_run_is_read_back_from_its_write_root` (E2E), and
//!   H3, which stays green only because the cross-sectional names carry their
//!   range.
//! - **LR-212a** [`lab_draws_its_tuples_strategy_not_a_sibling_in_its_dir`]
//!   and **LR-212b** [`lab_draws_no_report_of_another_coin`] (below). Both
//!   render the full Lab through `lab::view` with `Cockpit::lab_report_roots`
//!   on temp roots and no pre-warm. LR-207d, LR-208, LR-208e and LR-211d set
//!   the range to `H1_2024`: their reports are `2024-h1` runs, which no longer
//!   answer the default `Last90d`. Their margins are unchanged.
//!
//! ### T-T4 probe table (bug-log #212)
//!
//! The #208 protocol: one edit, each OLD exactly once in its file, applied to
//! the real file, `touch`, run, then restored from a `cp -p` copy, `cmp` and
//! `touch`. Paths: `ui/` = `crates/ui/src/`, `engine.rs` =
//! `crates/backtest/src/engine.rs`. Each probe ran `cargo test -p ui --lib`
//! (725), `--no-fail-fast --test lab_run_engine --test
//! lab_buy_sell_overlay_render --test live_equity_render --test
//! lab_binance_persist_compare` (2, 9, 31, 2), and for the `engine.rs` probes
//! `cargo test -p backtest --lib` (264). RED is the measured failing set,
//! compared by name; everything else stayed green. Measured 2026-11-24.
//!
//! | Probe | File / symbol | OLD → NEW | RED (measured) |
//! |---|---|---|---|
//! | R1 | `ui/lab/equity_loader.rs` `Coins::covers` | `            Coins::Unknown => false,` → `            Coins::Unknown => true,` | M3 |
//! | R2 | `ui/lab/equity_loader.rs` `parse_report_meta` | `    let coins = if let Some(sym) = summary_cell(body, "Symbol") {` → `    let coins = if let Some(sym) = None::<String> {` | M1, M3 (its control half), M6 (H1_2024 and H2_2024 × `btc-2024-sma-cross`: superset → none; the `btcusdt-…` cells keep their coin through the scenario fallback), LR-212b (its control: shift 0). Narrower than designed: an engine-shaped name (`btcusdt-…`) carries the ticker, so the scenario fallback still finds the coin for M4, M5, M8, E2E and LR-212a |
//! | R3 | `ui/lab/equity_loader.rs` `Coins::covers` | `            Coins::Universe(list) => list.iter().any(\|c\| c.eq_ignore_ascii_case(sym)),` → `            Coins::Universe(_) => true,` | M2, E2E (the pairs run answers XRPUSDT, outside its 4-coin universe) |
//! | R4 | `ui/lab/equity_loader.rs` `report_matches` | `    if want.is_none() \|\| want != have {` → `    if false {` | M4, M5, LR-212a (F_both vs F_macd 4,338 px, the F_swap figure: it draws the bbands curve; F_bb vs F_none 18,811 px) |
//! | R5 | `ui/lab/equity_loader.rs` `report_matches` | `        .and_then(backtest::engine::report_strategy_id);` → `        .map_or(want, backtest::engine::report_strategy_id);` | M5 |
//! | R6 | `ui/lab/equity_loader.rs` `range_fit` | `        (DateRange::Custom { start_raw, end_raw }, Span::Custom(start, end)) => {` → the same line preceded by `        (DateRange::Preset(Preset::Last90d), Span::Year(_)) => Some(RangeFit::Superset),` | M6 (Last90d × `btc-2024-…` and × `btc-2023-1m-…`: superset, not none) |
//! | R7 | `ui/lab/equity_loader.rs` `range_fit` | `        \| (DateRange::Preset(Preset::H1_2024), Span::Half(2024, 1))` → `        \| (DateRange::Preset(Preset::H1_2024), Span::Half(2024, _))` | M6 (H1_2024 × `…-2024-h2-…`: exact, not none) |
//! | R13 | `ui/lab/equity_loader.rs` `range_fit` | the two lines `        \| (DateRange::Preset(Preset::H1_2024), Span::Half(2024, 1))` + `        \| (DateRange::Preset(Preset::H2_2024), Span::Half(2024, 2)) => Some(RangeFit::Exact),` → `Span::Half(_, 1)` / `Span::Half(_, 2)` | M6 (H1_2024 × `…-2023-h1-…` and H2_2024 × `…-2025-h2-…`: exact, not none). Added after review: before M6 held a 2023 H1 and a 2025 H2 scenario, this probe left every test green. On the `#212` tree R13 ran `cargo test -p ui --lib` only (724 of 725); its four targets were measured on the `#212` + `#209` tree (2, 10, 31, 2, all green) |
//! | R8 | `ui/lab/equity_loader.rs` `strategy_slug` | `    SmolStr::new(backtest::engine::strategy_dir_slug(id))` → `    if id == "v0.5.rsi" { return SmolStr::new(id); }` + the same line | M8, `strategy_slug_mapping`, M4 (rsi: `ReportDirNotFound`, not `NoReport`), E2E |
//! | R9 | `engine.rs` the `"v1.momentum"` dispatch arm | `                scenario_name: lab_universe_scenario_name("v1.momentum", &cfg),` → `                scenario_name: "v1.momentum".to_string(),` | H3, E2E (momentum reads nothing back) |
//! | R10 | `engine.rs` `report_strategy_id` | `        "v0.5.macd" \| "macd_trend" \| "btc_macd_trend" \| "macd_trend_h1" => Some("btc_macd_trend"),` → `… => Some("macd_trend"),` | E-ID, E2E (the written `id:` is `btc_macd_trend`); M4 stays green, as designed: both sides normalise |
//! | R11 | `engine.rs` `report_strategy_id` | the same OLD → `        "v0.5.macd" \| "macd_trend" \| "btc_macd_trend" \| "macd_trend_h1" \| "btc_bbands_mean_revert" => Some("btc_macd_trend"),` | E-ID, M4, M8 (`v0.5.bbands`), E2E, LR-212a (4,338 px) |
//! | R12 | `engine.rs` `report_strategy_id` | the two TCN arms → the design's first form, `"v2.5.tcn" \| "v2.5.tcn_overlay" \| "tcn_overlay_momentum" \| "v2.5.tcn.weights" \| "v2.5.tcn_overlay_weights" => Some("tcn_overlay_momentum"),` | E-ID, M8 (`v2.5.tcn`, `v2.5.tcn.weights`), E2E (`v2.5.tcn`) |
//!
//! Every probe left `live_equity_render.rs` (31) and
//! `lab_binance_persist_compare.rs` (2) green: their fixtures are migrated to
//! well-formed reports of their own tuple. Red-first at HEAD is REASONED per
//! component, not measured as one run: HEAD's predicate is R4 (no strategy)
//! plus R1/R3 (any coin when the Universe is not one entry) plus a range rule
//! wider than R6, and its slug map is R8 for five chips. PNGs (with
//! `UI_DUMP_PNG`) of the fixed tree: `/tmp/lab_212a_{both,macd,bbands,none,swap}.png`,
//! `/tmp/lab_212b_{eth,none,btc}.png`.
//!
//! ## bug-log #209 — the Lab draws the newest run of its tuple
//!
//! The Lab's cold path kept the first root with a match (`lab-runs/` won every
//! collision) and, inside a root, the first strictly-best score in D8 order,
//! so the OLDEST stamp won a tie and `generated:` was never read. It now keeps
//! the union's greatest key (tier, range score, `generated:`); an identical
//! filename keeps its first copy (`lab-runs/`, then container depth 0) through
//! one filename set over the walk, as Compare does, and an equal key goes to
//! the root searched first, then to the first report in D8 order (ADR-0055
//! § D5 as amended). A report that cannot be read falls through to the next
//! candidate of its source tier (a broken tagged report never to an untagged
//! one); in the Lab a companion CSV that is present but does not read, or is
//! empty, makes its report unreadable. Tests: the `lab::equity_loader` lib tests N1-A
//! `a_newer_generated_in_the_larger_path_wins`, N1-B
//! `a_newer_generated_in_the_smaller_path_wins`, N2
//! `equal_score_and_generated_inside_one_dir_goes_to_the_first_path`, N3
//! `an_exact_range_beats_a_newer_superset`, N4
//! `a_report_without_generated_ranks_oldest`, X1
//! `a_newer_evidence_report_beats_an_older_lab_runs_one`, X2
//! `an_equal_generated_across_roots_goes_to_lab_runs`, X3
//! `a_tagged_older_evidence_report_beats_an_untagged_newer_lab_runs_one`, X4
//! `an_exact_evidence_report_beats_a_newer_lab_runs_superset`; re-derived on an
//! equal `generated:` E2, E2b, `route_overlay_two_root_union_lab_runs_wins_collision`,
//! E4 `equal_score_and_generated_tie_at_one_container_depth_goes_to_the_first_dir`,
//! E4b `equal_generated_tie_across_container_depths_goes_to_container_depth_0`
//! (+ `a_strictly_newer_generated_at_container_depth_1_wins`); the
//! identical-filename pins, mirroring Compare's, F1
//! `an_identical_filename_across_roots_goes_to_lab_runs` and F2
//! `an_identical_filename_inside_a_root_goes_to_container_depth_0` (two-sided:
//! `a0/` and `zz/` straddle the slug), and F4
//! `a_non_matching_first_copy_still_claims_its_file_name` (four first copies,
//! one per check that can reject it: read, parse, match, source); the
//! fall-through F3 `a_broken_newer_report_falls_through_to_the_next_candidate`
//! (across roots), F5
//! `a_broken_newest_report_falls_through_to_an_older_one_in_its_root` (inside
//! one root), F7 `a_broken_exact_report_falls_through_to_a_superset_with_its_badge`
//! (inside the tier, with the `narrowed_from` badge) and F8
//! `a_run_whose_csv_is_cut_short_falls_through_to_an_older_one` (a cut or empty
//! companion CSV); the tier boundary F6
//! `a_broken_tagged_report_never_yields_to_an_untagged_one`; and **LR-209**
//! [`lab_draws_the_newest_run_of_its_tuple`] (below).
//!
//! ### T-T4 probe table (bug-log #209)
//!
//! The #212 protocol above, on `ui/lab/equity_loader.rs`; each probe ran
//! `cargo test -p ui --lib` (743) and `--no-fail-fast --test lab_run_engine
//! --test lab_buy_sell_overlay_render --test live_equity_render --test
//! lab_binance_persist_compare` (2, 10, 31, 2). Measured 2026-11-24, every
//! probe again on the final code after review round 3 (the tier boundary, the
//! strict companion-CSV read, F4's four variants, F6-F8). F1-F8 "pair" = the
//! test's two-sided half (distinct names, a readable report, no tagged report,
//! an intact or absent CSV); F4, F6 and F8 run every variant and name each
//! that fails.
//!
//! | Probe | `ui/lab/equity_loader.rs` symbol | OLD → NEW | RED (measured) |
//! |---|---|---|---|
//! | S1 drop `generated:` | `Candidate::key` | `            generated: self.generated,` → `            generated: None,` | X1, N1-A, N4, `a_strictly_newer_generated_at_container_depth_1_wins`, F1, F2, F3, F5 (each at its pair), F8 (its intact and absent halves: the older `a` comes first in D8 order), LR-209 (F_both vs F_new 4,338 px: it draws the older run) |
//! | S2 oldest `generated:` wins | `Candidate::key` | the same OLD → `generated: self.generated.map(\|t\| time::OffsetDateTime::UNIX_EPOCH - (t - time::OffsetDateTime::UNIX_EPOCH)),` (mirrored about the epoch) | X1, N1-A, N1-B, `a_strictly_newer_generated_at_container_depth_1_wins`, `clear_replaces_a_stale_committed_hit_with_the_newer_lab_runs_report`, F1, F2, F3, F5 (each at its pair), F8 (its intact and absent halves), LR-209 (4,338 px) |
//! | S3a last of equals in a root | `matching_candidates` | `        out.push(Candidate {` → `        out.insert(0, Candidate {` (a root's candidates in reverse D8 order) | E4, E4b, N2 |
//! | S3b equal key to the later candidate | `resolve_roots` | `    cands.sort_by(\|a, b\| b.1.key().cmp(&a.1.key()));` → `    cands.sort_by(\|a, b\| a.1.key().cmp(&b.1.key()));` + `    cands.reverse();` | X2, E2, E2b, `route_overlay_two_root_union_lab_runs_wins_collision`, and, since the sort also orders a root's own candidates, E4, E4b, N2. (The #208 `state::tests::lab_run_completed_{ok,err}_…` stay green: their two reports share one file name, so the set skips the `evidence/` copy before any sort.) |
//! | S4 the first root wins again | `resolve_roots` | `            Ok(found) => cands.extend(found.into_iter().map(\|c\| (i, c))),` → the same plus `break;` | X1, X3, X4, F1 and F3 (each at its pair), F6 (one root: `NoReport`, not `ReportDirNotFound`, because `evidence/` is never walked; reverse cross-root: the untagged `lab-runs/` report is drawn) |
//! | S5 no tier | `Candidate::key` | `            tier: self.tier,` → `            tier: Tier::Tagged,` | X3, `legacy_untagged_report_only_matches_when_no_tagged_exists`, F6 (all four broken cells: the newer untagged report sorts first and is drawn) |
//! | S6 `generated:` before the range | `CandidateKey` (field order) | `    range_score: u32,` + `    generated: Option<time::OffsetDateTime>,` → the two lines swapped | N3, X4, F7 (its pair: the newer superset beats the readable exact report) |
//! | S7 an undated report is "now" | `parse_report_meta` | `        .and_then(\|v\| {` (before the RFC 3339 parse) → `        .map_or(Some(time::OffsetDateTime::now_utc()), \|v\| {` | N4 |
//! | S8 (P6′) roots reversed | `resolve_roots` | `    for (i, root) in roots.iter().enumerate() {` → `    for (i, root) in roots.iter().enumerate().rev() {` | X2, E2, E2b, `route_overlay_two_root_union_lab_runs_wins_collision`, F1, F4 (all four variants), and the #208 `state::tests::lab_run_completed_{ok,err}_…` (with the roots reversed the `evidence/` copy of a shared file name is walked first) |
//! | S9 no filename set | `matching_candidates` | the three lines `        if !path.file_name().is_some_and(\|n\| seen.insert(n.to_owned())) {` … `        }` → nothing | F1, F2, F4 (all four variants) |
//! | S10 one set per root | `resolve_roots` | `        match matching_candidates(tuple, root, &mut seen) {` → `… &mut HashSet::new()) {` | F1, F4 (all four variants); F2, inside one root, stays green |
//! | S11 no fall-through | `resolve_roots` | `    for (i, c) in cands.iter().take_while(\|(_, c)\| Some(c.tier) == top_tier) {` → `    for (i, c) in cands.iter().take(1) {` | F3, F5, F7 and F8 (each at its broken half; F8 at its cut and empty halves). F3's union returns the broken report's `ParseError`; F5's, F7's and F8's return the absent `evidence/` root's `ReportDirNotFound`, the last root's error, which masks the read error of `lab-runs/` (`ParseError`; for F8's empty CSV `EmptySeries`) |
//! | S12 a name claimed only after `report_matches` | `matching_candidates` | the three-line check moved from above the read to just below `let MatchQuality::Match { fit } = report_matches(…) else { continue; };` | F4 (its read, parse and match variants) |
//! | S12b a name claimed only by a candidate | `matching_candidates` | the three-line check moved to just above `out.push(Candidate {`, below the source check | F4 (all four variants) |
//! | S12c a name claimed only after the parse | `matching_candidates` | the three-line check moved to just below `let Some(meta) = parse_report_meta(&content) else { continue; };` | F4 (its read and parse variants) |
//! | S12d a name claimed only after the read | `matching_candidates` | the three-line check moved to just below `let Ok(content) = std::fs::read_to_string(path) else { continue; };` | F4 (its read variant) |
//! | S13 one candidate per root | `resolve_roots` | `            Ok(found) => cands.extend(found.into_iter().map(\|c\| (i, c))),` → `… found.into_iter().rev().max_by(\|a, b\| a.key().cmp(&b.key())).map(…)` (the first best of each root, the round-1 form) | F5, F7, F8 (each at its broken half; F8 at its cut and empty halves); F3, across roots, stays green |
//! | S14 no tier boundary | `resolve_roots` | `    for (i, c) in cands.iter().take_while(\|(_, c)\| Some(c.tier) == top_tier) {` → `    for (i, c) in &cands {` | F6 (all four broken cells: the untagged report is drawn) |
//! | S15 the reads stop at the range score too | `resolve_roots` | the same OLD → `    for (i, c) in cands.iter().take_while(\|(_, c)\| Some(c.tier) == top_tier && c.fit == cands[0].1.fit) {` | F7 (its broken half: a miss, not the superset) |
//! | S16 the CSV error ignored | `materialize` | `    let companion = read_companion_equity_csv(cand_path)?;` → `    let companion = read_companion_equity_csv(cand_path).unwrap_or(None);` | F8 (its cut and empty halves: the newer run is drawn as a start/end pair) |
//! | S17 an absent CSV is an error | `read_companion_equity_csv` | `        return Ok(None);` in the `try_exists` branch → `        return Err(unreadable(String::new()));` | F8 (its absent half) and every test whose report has no companion CSV: lib 46 of 743 (44 in `lab::equity_loader`, E3 `integration_load_real_v1_report` among them, and the two #208 `state` tests), this file 8 of 10 (LR-212a, LR-212b and LR-209 among them), `live_equity_render.rs` 13 of 31 |
//!
//! Every probe but S17 left `lab_run_engine.rs` (2), `live_equity_render.rs`
//! (31) and `lab_binance_persist_compare.rs` (2) green, and LR-212a/LR-212b
//! with them; S17 left `lab_run_engine.rs` and
//! `lab_binance_persist_compare.rs` green. S3b replaces the design's
//! "non-stable sort" form, which cannot reorder two equal keys. HEAD's rule is
//! S1 + S9 + S13 with a first-root-that-reads cross-root rule (S4 differs only
//! in stopping before the read): no `generated:`, no filename set, one
//! first-best candidate per root; it also lets a broken tagged report yield
//! to an untagged one in the next root and ignores an unreadable companion
//! CSV. Red-first is REASONED per probe. The `#205` P6 rows in
//! `live_equity_render.rs` and `reports_populated_curve_render.rs` gain a P6′
//! row (S8), because P6's OLD is gone. PNGs (with `UI_DUMP_PNG`):
//! `/tmp/lab_209_{both,new,old,none}.png`.

#![cfg(target_os = "macos")]
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::cast_precision_loss)]
#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use std::time::Duration;

use trading_core::{Side, Symbol, Venue};
use ui::state::{Cockpit, PanelState};
use ui::test_support::{charts_screen_cockpit, program_from_cockpit};
use ui::theme::layout::LEGEND_CARD_WIDTH_PX;
use ui::theme::{ThemeMode, color, space};

const VIEW_W: u32 = 1280;
const VIEW_H: u32 = 980;
const SCALE: f32 = 1.0;

// Vertical bands (from the 1280×980 render of the full Lab body): the price
// chart's marker zone and the per-bar-volume strip. Generous so font/AA jitter
// in either widget never drops the band.
const CHART_BAND_Y: (u32, u32) = (545, 650);
const HISTO_BAND_Y: (u32, u32) = (815, 890);

/// The chart legend's column span `[left, right]` in a Lab frame, DERIVED from
/// the frame's plot right edge and the legend's layout constants. The legend
/// card ("Buy"/"Sell" triangle swatches in `UP_500`/`DOWN_500`, the colours of
/// the fill triangles) is anchored top-right INSIDE the plot rect:
/// `space::M` in from the plot's right edge and `LEGEND_CARD_WIDTH_PX` wide
/// (`chart_legend::compute_card_rect_dynamic`, no compare rows in the Lab).
/// So it moves left with the plot's right edge when the chart reserves the
/// equity gutter, and it is excluded from the chart-band clusters by this
/// geometry, not by a fixed column cutoff (in a gutter frame the swatches sat
/// left of the old fixed cutoff and were counted as a third triangle).
fn legend_x_span(plot_right: u32) -> (u32, u32) {
    let right = plot_right as f32 - space::M as f32;
    let left = right - LEGEND_CARD_WIDTH_PX;
    (left.floor() as u32, right.ceil() as u32)
}

fn rgb_of(c: iced::Color) -> (i32, i32, i32) {
    (
        (c.r * 255.0).round() as i32,
        (c.g * 255.0).round() as i32,
        (c.b * 255.0).round() as i32,
    )
}
fn close(a: (i32, i32, i32), b: (i32, i32, i32), tol: i32) -> bool {
    (a.0 - b.0).abs() <= tol && (a.1 - b.1).abs() <= tol && (a.2 - b.2).abs() <= tol
}

/// Whether column `x` holds a `target`-coloured pixel within rows `[y0, y1)`.
fn column_hits(rgba: &[u8], w: u32, x: u32, (y0, y1): (u32, u32), target: (i32, i32, i32)) -> bool {
    (y0..y1).any(|y| {
        let idx = ((y * w + x) * 4) as usize;
        let p = (
            i32::from(rgba[idx]),
            i32::from(rgba[idx + 1]),
            i32::from(rgba[idx + 2]),
        );
        close(p, target, 10)
    })
}

/// x-centroids of horizontally-separated clusters of `target` color within the
/// row band `[y0,y1)`, columns split into clusters by gaps > 12 px. Columns in
/// the inclusive span `exclude` (the chart legend) are skipped.
fn cluster_xs(
    rgba: &[u8],
    w: u32,
    rows: (u32, u32),
    target: (i32, i32, i32),
    exclude: Option<(u32, u32)>,
) -> Vec<u32> {
    let cols: Vec<u32> = (0..w)
        .filter(|x| !exclude.is_some_and(|(lo, hi)| (lo..=hi).contains(x)))
        .filter(|&x| column_hits(rgba, w, x, rows, target))
        .collect();
    let mut clusters: Vec<u32> = Vec::new();
    let mut run: Vec<u32> = Vec::new();
    for &x in &cols {
        if run.last().is_some_and(|&last| x - last > 12) {
            clusters.push(run.iter().sum::<u32>() / run.len() as u32);
            run.clear();
        }
        run.push(x);
    }
    if !run.is_empty() {
        clusters.push(run.iter().sum::<u32>() / run.len() as u32);
    }
    clusters
}

fn render(cockpit: Cockpit) -> (u32, Vec<u8>) {
    ui::widgets::chart::force_chart_utc_for_tests();
    let program = program_from_cockpit(cockpit);
    let theme = iced::Theme::Dark;
    let shot = iced_test::screenshot(&program, &theme, (VIEW_W, VIEW_H), SCALE, Duration::ZERO);
    (shot.size.width, shot.rgba.to_vec())
}

/// Max |triangle_x − histo_x| over the paired buy/sell clusters. ≤ this many
/// logical px counts as "the volume bar sits under its triangle". Before the
/// fix the leftmost fill was ~27 px off; after, all residuals are < 16 px
/// (only the sub-bar-width timestamp-vs-index difference remains).
const ALIGN_TOL_PX: i32 = 16;

#[test]
fn buy_sell_volume_bars_align_under_triangles() {
    // POSITIVE: the standard populated Lab scene (60 bars, 4 alternating
    // buy/sell fills for BTCUSDT) — the exact state the operator sees.
    let cockpit = charts_screen_cockpit();
    let fills = seeded_fills(&cockpit);
    let (w, rgba) = render(cockpit);

    if std::env::var_os("UI_DUMP_PNG").is_some()
        && let Some(img) = image::RgbaImage::from_raw(w, VIEW_H, rgba.clone())
    {
        let _ = img.save("/tmp/lab_buy_sell_overlay_aligned.png");
    }

    let frame = measure_lab_frame("the standard Lab scene", w, &rgba);
    assert_volume_bars_under_triangles("the standard Lab scene", &frame, &fills);
}

/// The fills the Lab draws, DERIVED from the scene's own seed — the
/// cockpit's `chart_markers` (`charts_screen_cockpit` seeds them with
/// `ui::fixtures::synthetic_fills_for`) for the active BTCUSDT pair, over the
/// seeded BTCUSDT bars: per side, the index of the bar each fill falls in
/// (the first bar with `open <= ts <= close`, `compute_volume_bins`' rule),
/// left to right.
struct SeededFills {
    buy_bars: Vec<usize>,
    sell_bars: Vec<usize>,
    bar_count: usize,
}

fn seeded_fills(cockpit: &Cockpit) -> SeededFills {
    let symbol = Symbol::new("BTCUSDT");
    let bars = btc_bars();
    let PanelState::Ready(markers) = &cockpit.chart_markers else {
        panic!("the scene must seed chart markers")
    };
    let (mut buy_bars, mut sell_bars) = (Vec::new(), Vec::new());
    for fill in markers.iter().filter(|f| f.symbol == symbol) {
        let ts = fill.venue_ts.unix_millis();
        let idx = bars
            .iter()
            .position(|b| b.open_ts.unix_millis() <= ts && ts <= b.close_ts.unix_millis())
            .expect("every seeded fill lies on a seeded bar");
        match fill.side {
            Side::Buy => buy_bars.push(idx),
            Side::Sell => sell_bars.push(idx),
        }
    }
    buy_bars.sort_unstable();
    sell_bars.sort_unstable();
    for (label, idxs) in [("buy", &buy_bars), ("sell", &sell_bars)] {
        assert!(
            !idxs.is_empty() && idxs.windows(2).all(|p| p[0] < p[1]),
            "precondition: each side has fills, one per bar, so one triangle and one volume \
             bar per fill ({label} bars {idxs:?})"
        );
    }
    SeededFills {
        buy_bars,
        sell_bars,
        bar_count: bars.len(),
    }
}

/// One rendered Lab frame's measured marker geometry.
struct LabFrame {
    /// The plot rect's right edge: the price line's last vertex column.
    plot_right: u32,
    /// The legend's derived column span, excluded from the chart band.
    legend: (u32, u32),
    tri_buy: Vec<u32>,
    tri_sell: Vec<u32>,
    his_buy: Vec<u32>,
    his_sell: Vec<u32>,
}

/// Measure the x-centroid of each buy/sell triangle (chart band, legend
/// excluded by `legend_x_span`) and each buy/sell volume bar (histogram band,
/// which holds no legend). Positive control on the exclusion: the derived
/// legend span must hold the legend's `UP_500` and `DOWN_500` swatches within
/// the chart band, so the box excluded is the legend, wherever the gutter
/// put it.
fn measure_lab_frame(scene: &str, w: u32, rgba: &[u8]) -> LabFrame {
    let up = rgb_of(color::UP_500.current(ThemeMode::Dark));
    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));
    let plot_right = rightmost_accent_column(rgba, w, CHART_CANVAS_ROWS)
        .unwrap_or_else(|| panic!("{scene}: the price line must draw"));
    let legend = legend_x_span(plot_right);
    for (name, target) in [("Buy", up), ("Sell", down)] {
        assert!(
            (legend.0..=legend.1).any(|x| column_hits(rgba, w, x, CHART_BAND_Y, target)),
            "{scene}: the derived legend span {legend:?} (plot right edge {plot_right}) must \
             hold the legend's {name} swatch in rows {CHART_BAND_Y:?}"
        );
    }
    LabFrame {
        plot_right,
        legend,
        tri_buy: cluster_xs(rgba, w, CHART_BAND_Y, up, Some(legend)),
        tri_sell: cluster_xs(rgba, w, CHART_BAND_Y, down, Some(legend)),
        his_buy: cluster_xs(rgba, w, HISTO_BAND_Y, up, None),
        his_sell: cluster_xs(rgba, w, HISTO_BAND_Y, down, None),
    }
}

/// Largest |triangle x − volume-bar x| over the left-to-right pairs (no
/// assertion; for the margin line).
fn worst_residual(frame: &LabFrame) -> i32 {
    [
        (&frame.tri_buy, &frame.his_buy),
        (&frame.tri_sell, &frame.his_sell),
    ]
    .iter()
    .flat_map(|(tri, his)| tri.iter().zip(his.iter()))
    .map(|(&tx, &hx)| (tx as i32 - hx as i32).abs())
    .max()
    .unwrap_or(0)
}

/// Assert both widgets painted exactly the seeded fills — as many triangles
/// and as many volume bars per side as `fills` holds (the proof is
/// meaningless if either is blank, the Reports-curve trap; and a count above
/// the fills means something else, such as a legend swatch, was counted) —
/// and that each volume bar sits under its triangle within `ALIGN_TOL_PX`.
/// `scene` names the frame in the failure message. Returns the largest
/// residual.
fn assert_volume_bars_under_triangles(scene: &str, frame: &LabFrame, fills: &SeededFills) -> i32 {
    let LabFrame {
        plot_right,
        legend,
        tri_buy,
        tri_sell,
        his_buy,
        his_sell,
    } = frame;
    let want = (fills.buy_bars.len(), fills.sell_bars.len());
    assert_eq!(
        (tri_buy.len(), tri_sell.len()),
        want,
        "{scene}: the chart must paint one triangle per seeded fill, (buys, sells) = {want:?} \
         (buy={tri_buy:?}, sell={tri_sell:?}; legend {legend:?} excluded, plot right edge \
         {plot_right})"
    );
    assert_eq!(
        (his_buy.len(), his_sell.len()),
        want,
        "{scene}: the histogram must paint one volume bar per seeded fill, (buys, sells) = \
         {want:?} (buy={his_buy:?}, sell={his_sell:?})"
    );

    // Each histogram bar sits under its triangle (paired left-to-right).
    for (label, tri, his) in [("BUY", tri_buy, his_buy), ("SELL", tri_sell, his_sell)] {
        for (i, (&tx, &hx)) in tri.iter().zip(his.iter()).enumerate() {
            let delta = (tx as i32 - hx as i32).abs();
            assert!(
                delta <= ALIGN_TOL_PX,
                "{scene}: {label}[{i}] volume bar (x={hx}) is NOT under its triangle (x={tx}): \
                 |delta|={delta} px > {ALIGN_TOL_PX} px tolerance. The buy/sell volume \
                 overlay is detached from the triangles — lab-buy-sell-overlay-align \
                 regression. (chart buys={tri_buy:?} sells={tri_sell:?}; \
                 histo buys={his_buy:?} sells={his_sell:?})"
            );
        }
    }
    worst_residual(frame)
}

/// NEGATIVE control — a long-only (buys-only) scene paints buy volume bars but
/// NO sell volume bars in the histogram band. Proves the classifier above
/// discriminates buy vs sell (it isn't matching the same pixels for both), so
/// the alignment assertion is meaningful.
#[test]
fn long_only_paints_no_sell_volume_bars() {
    let mut cockpit = charts_screen_cockpit();
    // Replace the alternating buy/sell markers with buys only, spread across
    // the same bar window so they render along the x-axis.
    let sym = Symbol::new("BTCUSDT");
    let base = ui::fixtures::synthetic_fills_for(Venue::Binance, &sym, 4);
    let buys_only: Vec<_> = base
        .into_iter()
        .map(|mut f| {
            f.side = Side::Buy;
            f
        })
        .collect();
    // `lab::view` derives the volume bins from `chart_markers` at compose time,
    // so the seeded buys-only markers flow straight through to the histogram.
    cockpit.chart_markers = PanelState::Ready(buys_only);

    let (w, rgba) = render(cockpit);
    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));
    let his_sell = cluster_xs(&rgba, w, HISTO_BAND_Y, down, None);
    assert!(
        his_sell.is_empty(),
        "long-only scene must paint NO sell (DOWN_500) volume bars in the \
         histogram band, got clusters at {his_sell:?}"
    );
}

// ── bug-log #207 — the full Lab: the equity gutter is reserved iff it draws ──
//
// LR-207d: the operator's screen. The Lab is pointed at the v1 momentum
// strategy on BTCUSDT over `H1_2024` (Synthetic; the range is `H1_2024`, not
// the default `Last90d`, since bug-log #212: both scene reports are `2024-h1`
// runs, and a report answers only its own range) and its equity cache is
// pre-warmed from a temp root (`EquityCache::get_or_load`), so `lab::view`'s
// `route_equity_overlay` is a cache hit and never reads the real
// `lab-runs/`/`evidence/` roots: the frame depends on the fixture only.
//   A — the committed start/end-only momentum report (copied byte-for-byte):
//       its ordinal `0`/`1` samples miss the seeded bar window.
//   B — an in-window per-bar report (`ui::test_support::lab_runs_report_at`).
// A must keep the chart full width (no empty gutter) with the histogram still
// under the triangles; B reserves the gutter. The right-edge shift A − B is
// derived from the gutter constants.

use std::path::Path;

use trading_core::StrategyId;
use ui::lab::equity_loader::{Fidelity, LabTuple, default_evidence_root};
use ui::lab::state::{DateRange, Preset};
use ui::test_support::{EQUITY_GUTTER_WIDENING_PX, lab_runs_report_at};

/// The committed start/end-only momentum report, relative to `evidence/`.
const COMMITTED_START_END_ONLY: &str =
    "v1/v1-cross-sectional-momentum/reports/backtest-20260429-195243-top10-2024-h1-momentum.md";

/// Rows of the 1280×980 Lab frame searched for the chart's right edge: the
/// chart canvas (rows ≈ 563..695, below the KPI strip and above the
/// position-size strip), calibrated by `diag_207_lab_bands` and read off the
/// PNGs. The legend is excluded by geometry, not by rows: here it overlaps
/// the plot's rows, but it is inset from the plot's right edge and its `ACCENT`
/// "Price" stub sits at its left, while the price line's last vertex sits ON
/// the right edge, right of the legend box.
const CHART_CANVAS_ROWS: (u32, u32) = (560, 700);

/// The Lab scene at a strategy: `charts_screen_cockpit` (60 seeded BTCUSDT
/// bars, 4 alternating fills at bars 12/24/36/48) pointed at v1 momentum on
/// BTCUSDT over `H1_2024`, its equity cache pre-warmed from `root`. Returns
/// the cockpit and the cached series. The range is `H1_2024`, not the default
/// `Last90d`, because both scene reports are `2024-h1` runs and since bug-log
/// #212 a report answers only its own range.
fn lab_207_scene(root: &Path) -> (Cockpit, ui::lab::equity_loader::LabEquitySeries) {
    let mut cockpit = charts_screen_cockpit();
    let strategy = StrategyId::new("v1.momentum");
    let symbol = Symbol::new("BTCUSDT");
    cockpit.lab_state.strategy = Some(strategy.clone());
    cockpit.lab_state.pair = Some((Venue::Binance, symbol.clone()));
    cockpit.lab_state.range = DateRange::Preset(Preset::H1_2024);
    // The tuple exactly as `lab::view` builds it from `lab_state`.
    let tuple = LabTuple::new(
        &strategy,
        Venue::Binance,
        &symbol,
        cockpit.lab_state.range.clone(),
        cockpit.lab_state.data_source,
    );
    let series = cockpit
        .equity_cache
        .borrow_mut()
        .get_or_load(&tuple, root)
        .expect("the fixture report resolves for the Lab tuple")
        .as_ref()
        .clone();
    (cockpit, series)
}

/// The 60 seeded BTCUSDT bars, from the same builder (and seed and count)
/// `charts_screen_cockpit` seeds the chart with.
fn btc_bars() -> Vec<trading_core::Bar> {
    let symbol = Symbol::new("BTCUSDT");
    let seed = ui::fixtures::seed_for(Venue::Binance, &symbol);
    ui::fixtures::synthetic_candles(seed, Venue::Binance, symbol, 60)
}

/// The seeded BTCUSDT bar window `[first open, last close]` in ms.
fn btc_bar_window_ms() -> (i64, i64) {
    let bars = btc_bars();
    (
        bars.first().expect("bars").open_ts.unix_millis(),
        bars.last().expect("bars").close_ts.unix_millis(),
    )
}

/// Write the scene-A root: the committed report copied byte-for-byte.
fn write_committed_root(root: &Path) {
    let src = default_evidence_root().join(COMMITTED_START_END_ONLY);
    assert!(
        src.is_file(),
        "the committed report must exist: {}",
        src.display()
    );
    let dir = root.join("v1-cross-sectional-momentum").join("reports");
    std::fs::create_dir_all(&dir).expect("create reports dir");
    let dst = dir.join(src.file_name().expect("file name"));
    std::fs::copy(&src, &dst).expect("copy the committed report");
    assert_eq!(
        std::fs::read(&src).expect("read source"),
        std::fs::read(&dst).expect("read copy"),
        "the copy must be byte-identical"
    );
}

/// Write the scene-B root: an in-window per-bar report for BTCUSDT.
fn write_in_window_root(root: &Path) {
    let dir = root.join("v1-cross-sectional-momentum").join("reports");
    std::fs::create_dir_all(&dir).expect("create reports dir");
    std::fs::write(
        dir.join("backtest-20260601-120000-top10-2024-h1-momentum.md"),
        lab_runs_report_at(
            "top10_momentum_h1",
            "top10-2024-h1-momentum",
            "BTCUSDT",
            100_000,
        ),
    )
    .expect("write the in-window report");
}

/// Rightmost column within rows `[y0, y1)` holding an `ACCENT` pixel (the
/// price line's last vertex sits on the plot's right edge; an in-window equity
/// curve, also `ACCENT`, ends before it).
fn rightmost_accent_column(rgba: &[u8], w: u32, (y0, y1): (u32, u32)) -> Option<u32> {
    let accent = rgb_of(color::ACCENT.current(ThemeMode::Dark));
    (0..w).rev().find(|&x| {
        (y0..y1).any(|y| {
            let idx = ((y * w + x) * 4) as usize;
            let p = (
                i32::from(rgba[idx]),
                i32::from(rgba[idx + 1]),
                i32::from(rgba[idx + 2]),
            );
            close(p, accent, 10)
        })
    })
}

/// Write `rgba` to `path` when `UI_DUMP_PNG` is set (this file's convention).
fn dump_png(w: u32, rgba: &[u8], path: &str) {
    if std::env::var_os("UI_DUMP_PNG").is_some()
        && let Some(img) = image::RgbaImage::from_raw(w, VIEW_H, rgba.to_vec())
    {
        let _ = img.save(path);
    }
}

/// Diagnostic (run with `--nocapture`): the row extents of the UP/DOWN marker
/// colours and the rightmost `ACCENT` column per 20-row band for scenes A and
/// B, so `CHART_BAND_Y`, `HISTO_BAND_Y` and `CHART_CANVAS_ROWS` are
/// placed by measurement. Not an assertion gate.
#[test]
fn diag_207_lab_bands() {
    let up = rgb_of(color::UP_500.current(ThemeMode::Dark));
    let down = rgb_of(color::DOWN_500.current(ThemeMode::Dark));
    for (name, write) in [
        ("A committed", write_committed_root as fn(&Path)),
        ("B in-window", write_in_window_root as fn(&Path)),
    ] {
        let tmp = tempfile::tempdir().expect("tempdir");
        write(tmp.path());
        let (cockpit, _) = lab_207_scene(tmp.path());
        let (w, rgba) = render(cockpit);
        let legend = rightmost_accent_column(&rgba, w, CHART_CANVAS_ROWS).map(legend_x_span);
        eprintln!("[diag #207] {name}: derived legend span {legend:?}");
        let rows_with = |target: (i32, i32, i32)| -> Vec<u32> {
            (0..VIEW_H)
                .filter(|&y| {
                    (0..w)
                        .filter(|x| !legend.is_some_and(|(lo, hi)| (lo..=hi).contains(x)))
                        .any(|x| {
                            let idx = ((y * w + x) * 4) as usize;
                            let p = (
                                i32::from(rgba[idx]),
                                i32::from(rgba[idx + 1]),
                                i32::from(rgba[idx + 2]),
                            );
                            close(p, target, 10)
                        })
                })
                .collect()
        };
        let edges: Vec<String> = (0..VIEW_H)
            .step_by(20)
            .filter_map(|y0| {
                rightmost_accent_column(&rgba, w, (y0, (y0 + 20).min(VIEW_H)))
                    .map(|x| format!("{y0}:{x}"))
            })
            .collect();
        eprintln!("[diag #207] {name}: UP rows {:?}", rows_with(up));
        eprintln!("[diag #207] {name}: DOWN rows {:?}", rows_with(down));
        eprintln!("[diag #207] {name}: rightmost ACCENT by 20-row band {edges:?}");
        eprintln!(
            "[diag #207] {name}: CHART_CANVAS_ROWS {CHART_CANVAS_ROWS:?} -> {:?}",
            rightmost_accent_column(&rgba, w, CHART_CANVAS_ROWS)
        );
    }
}

/// **LR-207d — the operator's Lab screen (bug-log #207).** Scene A (the
/// committed start/end-only report, out of the bar window) keeps the chart
/// full width and the volume bars under their triangles; scene B (an
/// in-window per-bar report) reserves the equity gutter. The chart's right
/// edge in A minus B is the derived gutter widening
/// (`EQUITY_GUTTER_WIDENING_PX` = `AXIS_GUTTER_EQUITY_PX` 56.0 −
/// `AXIS_GUTTER_RIGHT_PX` 16.0 = 40 px) within `[40 − 6, 40 + 4]`. Under the
/// pre-#207 rule A reserves the gutter too (shift 0). When only one of the
/// chart and the histogram follows the rule, the plot widths differ by 40 px
/// and the right-hand fills drift from their volume bars (the `SELL` fill at
/// bar 48 by 20 px > `ALIGN_TOL_PX`, measured with the histogram leg reverted).
///
/// The histogram's own shift is pinned too, DERIVED from the geometry: the
/// histogram centres volume bar `i` of `n` at `chart::x_for_index(i, n, plot)`
/// = `plot.x + i / (n − 1) · plot.width`, and B's plot rect has the same left
/// edge (`AXIS_GUTTER_PRICE_PX` in both) and is `EQUITY_GUTTER_WIDENING_PX`
/// narrower. So the volume bar of a fill on bar `i` sits
/// `i / (n − 1) · EQUITY_GUTTER_WIDENING_PX` further right in A than in B:
/// for the seeded fills on bars 12, 24, 36 and 48 of 60, 8.1, 16.3, 24.4 and
/// 32.5 px, each within `HISTO_SHIFT_TOL_PX`. A histogram that ignores the
/// rule (shift 0) misses every one by more than the tolerance, independent of
/// the triangle/volume residual.
/// PNGs (with `UI_DUMP_PNG`): `/tmp/lab_207_full_out_of_window.png`,
/// `/tmp/lab_207_full_in_window.png`.
#[test]
fn lab_start_end_only_overlay_keeps_the_chart_full_width_and_aligned() {
    let (win_start, win_end) = btc_bar_window_ms();

    let tmp_a = tempfile::tempdir().expect("tempdir");
    write_committed_root(tmp_a.path());
    let (cockpit_a, series_a) = lab_207_scene(tmp_a.path());
    assert_eq!(series_a.fidelity, Fidelity::StartEndOnly);
    assert!(
        series_a
            .samples
            .iter()
            .all(|(ts, _)| *ts < win_start || *ts > win_end),
        "precondition A: every sample misses the bar window [{win_start}, {win_end}]"
    );

    let tmp_b = tempfile::tempdir().expect("tempdir");
    write_in_window_root(tmp_b.path());
    let (cockpit_b, series_b) = lab_207_scene(tmp_b.path());
    assert_eq!(series_b.fidelity, Fidelity::PerBar);
    assert!(
        series_b
            .samples
            .iter()
            .all(|(ts, _)| (win_start..=win_end).contains(ts)),
        "precondition B: every sample lies in the bar window [{win_start}, {win_end}]"
    );

    let fills = seeded_fills(&cockpit_a);
    let fills_b = seeded_fills(&cockpit_b);
    assert_eq!(
        (&fills.buy_bars, &fills.sell_bars),
        (&fills_b.buy_bars, &fills_b.sell_bars),
        "precondition: both scenes seed the same fills"
    );

    let (w_a, rgba_a) = render(cockpit_a);
    let (w_b, rgba_b) = render(cockpit_b);
    dump_png(w_a, &rgba_a, "/tmp/lab_207_full_out_of_window.png");
    dump_png(w_b, &rgba_b, "/tmp/lab_207_full_in_window.png");

    let frame_a = measure_lab_frame("#207 scene A (out of window)", w_a, &rgba_a);
    let frame_b = measure_lab_frame("#207 scene B (in window)", w_b, &rgba_b);

    // Chart: the plot's right edge moves by the derived gutter widening.
    let (edge_a, edge_b) = (frame_a.plot_right, frame_b.plot_right);
    let shift = i64::from(edge_a) - i64::from(edge_b);
    let expected = EQUITY_GUTTER_WIDENING_PX.round() as i64;
    let (lo, hi) = (expected - 6, expected + 4);

    // Histogram: each fill's volume bar moves by its derived share of it.
    let span = (fills.bar_count - 1) as f32;
    let histo_shifts: Vec<(String, f32, i64)> = [
        ("BUY", &fills.buy_bars, &frame_a.his_buy, &frame_b.his_buy),
        (
            "SELL",
            &fills.sell_bars,
            &frame_a.his_sell,
            &frame_b.his_sell,
        ),
    ]
    .into_iter()
    .flat_map(|(label, bars, his_a, his_b)| {
        bars.iter()
            .zip(his_a.iter().zip(his_b.iter()))
            .enumerate()
            .map(move |(i, (&bar, (&xa, &xb)))| {
                let want = bar as f32 / span * EQUITY_GUTTER_WIDENING_PX;
                (
                    format!("{label}[{i}] bar {bar}"),
                    want,
                    i64::from(xa) - i64::from(xb),
                )
            })
    })
    .collect();
    let histo_worst = histo_shifts
        .iter()
        .map(|(_, want, got)| (*got as f32 - want).abs())
        .fold(0.0_f32, f32::max);

    eprintln!(
        "#207 pixel margin LR-207d: right edge A {edge_a} - B {edge_b} = {shift} px (expected \
         {expected}, band [{lo}, {hi}]); worst triangle/volume residual A {} px, B {} px \
         (threshold <= {ALIGN_TOL_PX})",
        worst_residual(&frame_a),
        worst_residual(&frame_b)
    );
    eprintln!(
        "#207 pixel margin LR-207d histogram shift A - B: {histo_shifts:?} (label, derived, \
         measured); worst |measured - derived| {histo_worst:.1} px (threshold <= \
         {HISTO_SHIFT_TOL_PX})"
    );

    let worst_a =
        assert_volume_bars_under_triangles("#207 scene A (out of window)", &frame_a, &fills);
    let worst_b = assert_volume_bars_under_triangles("#207 scene B (in window)", &frame_b, &fills);
    assert_eq!(
        histo_shifts.len(),
        fills.buy_bars.len() + fills.sell_bars.len(),
        "every seeded fill's volume bar has a measured A - B shift"
    );
    assert!(
        histo_worst <= HISTO_SHIFT_TOL_PX,
        "#207: the histogram must follow the chart's gutter: each volume bar's A - B shift must \
         be its derived i / (n - 1) * {EQUITY_GUTTER_WIDENING_PX} px within \
         {HISTO_SHIFT_TOL_PX} px, got {histo_shifts:?} (label, derived, measured)"
    );
    assert!(
        (lo..=hi).contains(&shift),
        "#207: the out-of-window overlay must leave the chart full width and the in-window one \
         must reserve the gutter: right edge A {edge_a}, B {edge_b}, shift {shift} px, expected \
         {expected} within [{lo}, {hi}] (worst residuals A {worst_a}, B {worst_b})"
    );
}

/// Tolerance on LR-207d's derived per-bar histogram shift (logical px): the
/// volume-bar centroids are integer column means, and the shifts derived from
/// the gutter constants are 8.1 to 32.5 px.
const HISTO_SHIFT_TOL_PX: f32 = 4.0;

// ── bug-log #208 — a cached miss hides a new report until a run completes ──

/// The completed-run summary `ui::state::update` receives: no data (the pure
/// update builds no run mirror, so `last_run_report` stays `None`, the state
/// after the mirror is cleared, and the overlay comes from the cache).
fn empty_run_summary() -> ui::lab::runner::RunSummary {
    ui::lab::runner::RunSummary {
        strategy_id: smol_str::SmolStr::new("v1.momentum"),
        symbol: smol_str::SmolStr::new("BTCUSDT"),
        report_path: None,
        equity_series: Vec::new(),
        fills: Vec::new(),
        kpis: backtest::BacktestKpis::default(),
        bars: std::sync::Arc::new(Vec::new()),
        position_curve: Vec::new(),
    }
}

/// How many pixels differ between two same-size RGBA frames.
fn differing_pixels(a: &[u8], b: &[u8]) -> usize {
    a.chunks_exact(4)
        .zip(b.chunks_exact(4))
        .filter(|(p, q)| p != q)
        .count()
}

/// **LR-208 — a cached miss hides a new report until a Lab run completes,
/// then the report draws (bug-log #208).** The LR-207d Lab scene (v1
/// momentum on BTCUSDT), its `Cockpit::lab_report_roots` pointed at two
/// absent temp roots, and NO pre-warm: the miss is seeded on the cockpit
/// itself through the real `ui::screens::lab::view`, and every render clones
/// the cockpit, cache included (`program_from_cockpit`).
///   F0 — the cached miss: no overlay, the chart full width.
///   F1 — after an in-window report (`write_in_window_root`, LR-207d's
///        scene B) is written under the temp `lab-runs/`: byte-identical to
///        F0, the miss is not re-read. A fresh cache over the same roots
///        resolves the report (the control: the file is there and matches).
///   F2 — after `ui::state::update(LabRunCompleted(Ok))`: the report draws,
///        so the chart reserves the equity gutter and the plot's right edge
///        moves left by the derived `EQUITY_GUTTER_WIDENING_PX` (40 px) within
///        LR-207d's band `[40 − 6, 40 + 4]`. Only the right edge is compared:
///        the completed run also refreshes the cache-status badges.
/// PNGs (with `UI_DUMP_PNG`): `/tmp/lab_208_F0.png`, `/tmp/lab_208_F1.png`,
/// `/tmp/lab_208_F2.png`.
#[test]
fn lab_cached_miss_hides_a_new_report_until_a_lab_run_completes_then_draws_it() {
    assert_cached_miss_hides_then_draws_208(false, "lab_208", "LR-208");
}

/// **LR-208e — LR-208 with the report under the temp `evidence/` root at
/// container depth 1** (`evidence/v1/v1-cross-sectional-momentum/reports/`).
/// F1 == F0 pins that a cached miss re-walks NO root (not even the last,
/// which `#205` widened); F2's shift pins that `lab::view` passes the whole
/// seam (`Cockpit::lab_report_roots`), `evidence/` included. PNGs (with
/// `UI_DUMP_PNG`): `/tmp/lab_208_evidence_F{0,1,2}.png`.
#[test]
fn lab_cached_miss_under_evidence_hides_a_new_report_until_a_lab_run_completes() {
    assert_cached_miss_hides_then_draws_208(true, "lab_208_evidence", "LR-208e");
}

fn assert_cached_miss_hides_then_draws_208(under_evidence: bool, png: &str, key: &str) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let lab_runs = tmp.path().join("lab-runs");
    let report_root = if under_evidence {
        tmp.path().join("evidence").join("v1")
    } else {
        lab_runs.clone()
    };
    let mut cockpit = charts_screen_cockpit();
    let strategy = StrategyId::new("v1.momentum");
    let symbol = Symbol::new("BTCUSDT");
    cockpit.lab_state.strategy = Some(strategy.clone());
    cockpit.lab_state.pair = Some((Venue::Binance, symbol.clone()));
    // The written report is a `2024-h1` run (bug-log #212).
    cockpit.lab_state.range = DateRange::Preset(Preset::H1_2024);
    cockpit.lab_report_roots = vec![lab_runs.clone(), tmp.path().join("evidence")];
    assert!(
        cockpit.lab_state.last_run_report.is_none(),
        "precondition: no run mirror, so the overlay comes from the equity cache"
    );
    // The tuple exactly as `lab::view` builds it from `lab_state`.
    let tuple = LabTuple::new(
        &strategy,
        Venue::Binance,
        &symbol,
        cockpit.lab_state.range.clone(),
        cockpit.lab_state.data_source,
    );

    drop(ui::screens::lab::view(&cockpit, ThemeMode::Dark));
    let (w0, f0) = render(cockpit.clone());
    dump_png(w0, &f0, &format!("/tmp/{png}_F0.png"));

    write_in_window_root(&report_root);
    let control = ui::lab::equity_loader::EquityCache::new()
        .get_or_load_roots(&tuple, &cockpit.lab_report_roots)
        .expect("control: a fresh cache resolves the written report");
    let (win_start, win_end) = btc_bar_window_ms();
    assert!(
        control
            .samples
            .iter()
            .all(|(ts, _)| (win_start..=win_end).contains(ts)),
        "control: every sample of the written report lies in the bar window"
    );
    let (w1, f1) = render(cockpit.clone());
    dump_png(w1, &f1, &format!("/tmp/{png}_F1.png"));

    ui::state::update(
        &mut cockpit,
        ui::state::Message::LabRunCompleted(Ok(empty_run_summary())),
    );
    let (w2, f2) = render(cockpit.clone());
    dump_png(w2, &f2, &format!("/tmp/{png}_F2.png"));

    let frame_1 = measure_lab_frame("#208 F1 (cached miss, new report on disk)", w1, &f1);
    let frame_2 = measure_lab_frame("#208 F2 (after LabRunCompleted)", w2, &f2);
    let differ_01 = differing_pixels(&f0, &f1);
    let (edge_1, edge_2) = (frame_1.plot_right, frame_2.plot_right);
    let shift = i64::from(edge_1) - i64::from(edge_2);
    let expected = EQUITY_GUTTER_WIDENING_PX.round() as i64;
    let (lo, hi) = (expected - 6, expected + 4);
    eprintln!(
        "#208 pixel margin {key}: F0 vs F1 {differ_01} px differ (threshold == 0); right edge \
         F1 {edge_1} - F2 {edge_2} = {shift} px (expected {expected}, band [{lo}, {hi}])"
    );

    assert_eq!(
        [w0, w1, w2],
        [VIEW_W; 3],
        "the three frames must have the requested width"
    );
    assert_eq!(
        differ_01, 0,
        "#208: while the miss is cached, a new report on disk must not change the frame \
         (F0 vs F1: {differ_01} px differ)"
    );
    assert!(
        (lo..=hi).contains(&shift),
        "#208: after LabRunCompleted the new report must draw and reserve the equity gutter: \
         right edge F1 {edge_1}, F2 {edge_2}, shift {shift} px, expected {expected} within \
         [{lo}, {hi}]"
    );
}

// ── bug-log #211 — the full Lab: an H1_2024 run over the seeded hour ────────

use ui::test_support::{H1_2024_HOURS, h1_2024_hourly_samples, write_companion_equity_csv_for_md};

/// Write the scene-C root: an `H1_2024` Lab run as it is persisted, the
/// scene-B report plus its companion `-equity.csv` holding the first 4,344 of
/// the run's hourly samples (`h1_2024_hourly_samples`; a real top-10 momentum
/// run has 10 · 4,344 + 1, `#226`), which the loader prefers over the
/// report's own table.
fn write_h1_2024_root(root: &Path) {
    write_in_window_root(root);
    let md = root
        .join("v1-cross-sectional-momentum")
        .join("reports")
        .join("backtest-20260601-120000-top10-2024-h1-momentum.md");
    write_companion_equity_csv_for_md(&md, &h1_2024_hourly_samples(100_000))
        .expect("write the hourly companion CSV");
}

/// **LR-211d — the operator's Lab screen with an H1_2024 run over the seeded
/// hour (bug-log #211).** Scene C: the LR-207d Lab (v1 momentum on BTCUSDT,
/// `H1_2024` in every scene, so the range chip is the same) with its
/// equity cache pre-warmed from `write_h1_2024_root`: hourly samples from
/// 2024-01-01, exactly one of them (12:00) inside the seeded bar window.
/// One point strokes no line, so C draws no overlay and must be
/// byte-identical to scene A (the committed start/end-only report, which
/// draws none): no gutter on the chart, and the volume histogram (which asks
/// `chart::equity_axis_shown`) keeps the full width. Control: scene B (the
/// in-window report) differs from A, so the identity cannot pass because the
/// screen drops every overlay. PNGs (with `UI_DUMP_PNG`):
/// `/tmp/lab_211_full_no_overlay.png` (A), `/tmp/lab_211_full_in_window.png`
/// (B), `/tmp/lab_211_full_h1_2024.png` (C).
#[test]
fn lab_h1_2024_run_over_the_seeded_hour_matches_the_no_overlay_lab() {
    let window = btc_bar_window_ms();
    let inside = |series: &ui::lab::equity_loader::LabEquitySeries| {
        series
            .samples
            .iter()
            .filter(|(ts, _)| (window.0..=window.1).contains(ts))
            .count()
    };

    let tmp_a = tempfile::tempdir().expect("tempdir");
    write_committed_root(tmp_a.path());
    let (cockpit_a, series_a) = lab_207_scene(tmp_a.path());
    assert_eq!(series_a.fidelity, Fidelity::StartEndOnly);
    assert_eq!(
        inside(&series_a),
        0,
        "precondition A: no sample in {window:?}"
    );

    let tmp_b = tempfile::tempdir().expect("tempdir");
    write_in_window_root(tmp_b.path());
    let (cockpit_b, series_b) = lab_207_scene(tmp_b.path());
    assert_eq!(series_b.fidelity, Fidelity::PerBar);
    assert_eq!(
        inside(&series_b),
        series_b.samples.len(),
        "precondition B: every sample in {window:?}"
    );

    let tmp_c = tempfile::tempdir().expect("tempdir");
    write_h1_2024_root(tmp_c.path());
    let (cockpit_c, series_c) = lab_207_scene(tmp_c.path());
    assert_eq!(series_c.fidelity, Fidelity::PerBar);
    assert_eq!(
        series_c.samples.len(),
        H1_2024_HOURS,
        "precondition C: the loader read the hourly companion CSV"
    );
    assert_eq!(
        inside(&series_c),
        1,
        "precondition C: exactly one hourly sample in {window:?}"
    );

    let (w_a, rgba_a) = render(cockpit_a);
    let (w_b, rgba_b) = render(cockpit_b);
    let (w_c, rgba_c) = render(cockpit_c);
    dump_png(w_a, &rgba_a, "/tmp/lab_211_full_no_overlay.png");
    dump_png(w_b, &rgba_b, "/tmp/lab_211_full_in_window.png");
    dump_png(w_c, &rgba_c, "/tmp/lab_211_full_h1_2024.png");

    let differ_ac = differing_pixels(&rgba_a, &rgba_c);
    let differ_ab = differing_pixels(&rgba_a, &rgba_b);
    eprintln!(
        "#211 pixel margin LR-211d: scene C (the H1_2024 run) differs from scene A (no overlay) \
         in {differ_ac} px (threshold == 0); control: scene B (in-window) differs from A in \
         {differ_ab} px (threshold > 0)"
    );
    assert_eq!(
        [w_a, w_b, w_c],
        [VIEW_W; 3],
        "the three frames must have the requested width"
    );
    assert_eq!(
        differ_ac, 0,
        "#211: an H1_2024 run with one sample in the seeded hour draws no overlay, so the Lab \
         must equal the no-overlay Lab, but {differ_ac} px differ"
    );
    assert!(
        differ_ab > 0,
        "control: an in-window run must change the Lab, but 0 px differ, so this harness \
         draws no overlay"
    );
}

// ── bug-log #212 — the Lab draws only a report of its own tuple ─────────────

use ui::test_support::{LabReportFixture, lab_report};

/// The full Lab over `(strategy, BTCUSDT, H1_2024, Synthetic)`, reading the
/// report roots `[<root>/lab-runs, <root>/evidence]` through the real
/// `lab::view` (no pre-warm: every render clones a cockpit with an empty
/// equity cache, so the frame is what the cold path resolves from disk).
fn lab_reading(strategy: &str, root: &Path) -> Cockpit {
    let mut cockpit = charts_screen_cockpit();
    cockpit.lab_state.strategy = Some(StrategyId::new(strategy));
    cockpit.lab_state.pair = Some((Venue::Binance, Symbol::new("BTCUSDT")));
    cockpit.lab_state.range = DateRange::Preset(Preset::H1_2024);
    cockpit.lab_report_roots = vec![root.join("lab-runs"), root.join("evidence")];
    assert!(
        cockpit.lab_state.last_run_report.is_none(),
        "precondition: no run mirror, so the overlay comes from the report roots"
    );
    cockpit
}

/// Write `f` as `<root>/lab-runs/<slug>/reports/<name>`.
fn write_lab_run(root: &Path, slug: &str, name: &str, f: &LabReportFixture<'_>) {
    let dir = root.join("lab-runs").join(slug).join("reports");
    std::fs::create_dir_all(&dir).expect("create reports dir");
    std::fs::write(dir.join(name), lab_report(f)).expect("write the report");
}

/// An `H1_2024` synthetic engine-shaped single-coin report: the coin in the
/// Summary `| Symbol |` row only, the curve in the seeded bar window.
fn h1_run<'a>(
    strategy_id: &'a str,
    scenario: &'a str,
    coin: &'a str,
    generated: &'a str,
    falling: bool,
) -> LabReportFixture<'a> {
    LabReportFixture {
        strategy_id: Some(strategy_id),
        scenario,
        symbol_row: Some(coin),
        universe: &[],
        generated,
        data_source: "synthetic",
        level: 100_000,
        falling,
    }
}

/// The least number of pixels by which two Lab frames that draw different
/// curves for one tuple (rising vs falling) must differ, so a 0-px identity
/// against one of them cannot pass by drawing the other, and by which a frame
/// that draws a run must differ from the no-overlay frame. Measured
/// 2026-11-24: LR-212a's and LR-209's controls differ by far more (printed as
/// `#212 pixel margin …` / `#209 pixel margin …`).
const CURVE_SWAP_MIN_PX: usize = 1_000;

/// The v05-composed-strategies pair of LR-212a: (file name, writer id,
/// scenario, `generated:`, falling). The bbands run sits at the SMALLER stamp
/// and carries the NEWER `generated:`, so both a first-in-path-order and a
/// newest-`generated:` reader that ignores the strategy draw it.
const LR212A_RUNS: [(&str, &str, &str, &str, bool); 2] = [
    (
        "backtest-20260601-120000000-btcusdt-2024-h1-bbands-mean-revert-synthetic.md",
        "btc_bbands_mean_revert",
        "btcusdt-2024-h1-bbands-mean-revert-synthetic",
        "2026-06-02T12:00:00Z",
        true,
    ),
    (
        "backtest-20260601-130000000-btcusdt-2024-h1-macd-trend-synthetic.md",
        "btc_macd_trend",
        "btcusdt-2024-h1-macd-trend-synthetic",
        "2026-06-01T12:00:00Z",
        false,
    ),
];

/// **LR-212a — the Lab draws its tuple's strategy, not a sibling in the same
/// dir (bug-log #212).** Tuple `(v0.5.macd, BTCUSDT, H1_2024, Synthetic)`;
/// `lab-runs/v05-composed-strategies/reports/` (the dir the engine writes
/// macd, rsi and bbands runs to) holds a falling bbands run and a rising macd
/// run, equal in coin, range and source (`LR212A_RUNS`).
///   F_both  — both runs: must equal F_macd byte for byte.
///   F_macd  — the macd run alone.
///   F_bb    — the bbands run alone: must equal F_none (it answers no macd
///             tuple).
///   F_none  — empty roots: no overlay. F_macd must differ from it by
///             ≥ `CURVE_SWAP_MIN_PX`: the macd run does draw (#187).
///   F_swap  — the control: the bbands bytes relabelled `btc_macd_trend`,
///             i.e. what a strategy-blind reader draws. It must differ from
///             F_macd by ≥ `CURVE_SWAP_MIN_PX`, so the identity cannot pass
///             by drawing the bbands curve.
/// PNGs (with `UI_DUMP_PNG`): `/tmp/lab_212a_{both,macd,bbands,none,swap}.png`.
#[test]
fn lab_draws_its_tuples_strategy_not_a_sibling_in_its_dir() {
    let slug = backtest::engine::strategy_dir_slug("v0.5.macd");
    assert_eq!(
        slug,
        backtest::engine::strategy_dir_slug("v0.5.bbands"),
        "precondition: the engine writes macd and bbands runs to one dir"
    );
    assert!(
        LR212A_RUNS[0].0 < LR212A_RUNS[1].0,
        "precondition: the bbands run has the smaller stamp"
    );
    let write = |root: &Path, idx: &[usize], relabel: Option<&str>| {
        for &i in idx {
            let (name, id, scenario, generated, falling) = LR212A_RUNS[i];
            let id = relabel.unwrap_or(id);
            write_lab_run(
                root,
                slug,
                name,
                &h1_run(id, scenario, "BTCUSDT", generated, falling),
            );
        }
    };
    let tmp = tempfile::tempdir().expect("tempdir");
    let roots: Vec<(&str, std::path::PathBuf)> = ["both", "macd", "bbands", "none", "swap"]
        .into_iter()
        .map(|k| (k, tmp.path().join(k)))
        .collect();
    write(&roots[0].1, &[0, 1], None);
    write(&roots[1].1, &[1], None);
    write(&roots[2].1, &[0], None);
    write(&roots[4].1, &[0], Some("btc_macd_trend"));

    let frames: Vec<(u32, Vec<u8>)> = roots
        .iter()
        .map(|(k, root)| {
            let (w, rgba) = render(lab_reading("v0.5.macd", root));
            dump_png(w, &rgba, &format!("/tmp/lab_212a_{k}.png"));
            (w, rgba)
        })
        .collect();
    let [both, macd, bbands, none, swap] = [0, 1, 2, 3, 4].map(|i| &frames[i].1);
    assert!(
        frames.iter().all(|(w, _)| *w == VIEW_W),
        "every frame must have the requested width"
    );
    let differ_both = differing_pixels(both, macd);
    let differ_bb = differing_pixels(bbands, none);
    let differ_swap = differing_pixels(swap, macd);
    let differ_macd_none = differing_pixels(macd, none);
    eprintln!(
        "#212 pixel margin LR-212a: F_both vs F_macd {differ_both} px (threshold == 0); F_bb vs \
         F_none {differ_bb} px (threshold == 0); control F_swap vs F_macd {differ_swap} px \
         (threshold >= {CURVE_SWAP_MIN_PX}); control F_macd vs F_none {differ_macd_none} px \
         (threshold >= {CURVE_SWAP_MIN_PX})"
    );
    assert!(
        differ_swap >= CURVE_SWAP_MIN_PX,
        "control: the bbands curve must look different from the macd curve, but only \
         {differ_swap} px differ"
    );
    assert!(
        differ_macd_none >= CURVE_SWAP_MIN_PX,
        "control: the macd run alone must draw, but only {differ_macd_none} px differ from the \
         no-overlay frame"
    );
    assert_eq!(
        differ_bb, 0,
        "#212: a bbands run answers no macd tuple, so the Lab must draw nothing, but \
         {differ_bb} px differ from the no-overlay frame"
    );
    assert_eq!(
        differ_both, 0,
        "#212: beside a bbands run the macd tuple must still draw its own run, but {differ_both} \
         px differ"
    );
}

/// **LR-212b — the Lab draws no report of another coin (bug-log #212).**
/// Tuple `(v0.sma, BTCUSDT, H1_2024, Synthetic)`. The only report is an ETH
/// run whose coin is named only in its Summary `| Symbol |` row (the shape of
/// every engine-written single-symbol report; its scenario `eth-2024-h1-…`
/// names no `…USDT` ticker). Its frame must be byte-identical to the
/// empty-root frame. Control: the BTC twin (the same report with `BTCUSDT` in
/// the row) draws, so the chart reserves the equity gutter and the plot's
/// right edge moves left by the derived `EQUITY_GUTTER_WIDENING_PX` (40 px)
/// within `[34, 44]` (`measure_lab_frame`). PNGs (with `UI_DUMP_PNG`):
/// `/tmp/lab_212b_{eth,none,btc}.png`.
#[test]
fn lab_draws_no_report_of_another_coin() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (eth, none, btc) = (
        tmp.path().join("eth"),
        tmp.path().join("none"),
        tmp.path().join("btc"),
    );
    let slug = backtest::engine::strategy_dir_slug("v0.sma");
    for (root, coin, scenario) in [
        (&eth, "ETHUSDT", "eth-2024-h1-sma-cross"),
        (&btc, "BTCUSDT", "btc-2024-h1-sma-cross"),
    ] {
        assert!(
            !scenario.to_ascii_uppercase().contains("USDT"),
            "precondition: the scenario names no ticker, only the Summary row does"
        );
        write_lab_run(
            root,
            slug,
            &format!("backtest-20260601-120000000-{scenario}.md"),
            &h1_run(
                "sma_crossover",
                scenario,
                coin,
                "2026-06-01T12:00:00Z",
                false,
            ),
        );
    }
    let (w_eth, f_eth) = render(lab_reading("v0.sma", &eth));
    let (w_none, f_none) = render(lab_reading("v0.sma", &none));
    let (w_btc, f_btc) = render(lab_reading("v0.sma", &btc));
    dump_png(w_eth, &f_eth, "/tmp/lab_212b_eth.png");
    dump_png(w_none, &f_none, "/tmp/lab_212b_none.png");
    dump_png(w_btc, &f_btc, "/tmp/lab_212b_btc.png");
    assert_eq!([w_eth, w_none, w_btc], [VIEW_W; 3], "frame widths");

    let differ = differing_pixels(&f_eth, &f_none);
    let edge_none = measure_lab_frame("#212b no report", w_none, &f_none).plot_right;
    let edge_btc = measure_lab_frame("#212b BTC twin", w_btc, &f_btc).plot_right;
    let shift = i64::from(edge_none) - i64::from(edge_btc);
    let expected = EQUITY_GUTTER_WIDENING_PX.round() as i64;
    let (lo, hi) = (expected - 6, expected + 4);
    eprintln!(
        "#212 pixel margin LR-212b: F_eth vs F_none {differ} px (threshold == 0); control: right \
         edge F_none {edge_none} - F_btc {edge_btc} = {shift} px (expected {expected}, band \
         [{lo}, {hi}])"
    );
    assert!(
        (lo..=hi).contains(&shift),
        "control: the BTC twin must draw and reserve the equity gutter: shift {shift} px, \
         expected {expected} within [{lo}, {hi}]"
    );
    assert_eq!(
        differ, 0,
        "#212: an ETH run answers no BTCUSDT tuple, so the Lab must draw nothing, but {differ} \
         px differ from the no-overlay frame"
    );
}

// ── bug-log #209 — the Lab draws the newest run of its tuple ───────────────

/// The two runs of LR-209, both of `(v0.sma, BTCUSDT, H1_2024, Synthetic)`
/// in `lab-runs/v0-paper-sma/reports/`: (file name, `generated:`, falling).
/// The OLDER run (falling) sits at the smaller stamp, so a first-in-path-order
/// reader draws it.
const LR209_RUNS: [(&str, &str, bool); 2] = [
    (
        "backtest-20260601-120000000-btcusdt-2024-h1-sma-cross-synthetic.md",
        "2026-06-01T12:00:00Z",
        true,
    ),
    (
        "backtest-20260602-120000000-btcusdt-2024-h1-sma-cross-synthetic.md",
        "2026-06-02T12:00:00Z",
        false,
    ),
];

/// **LR-209 — the Lab draws the newest run of its tuple (bug-log #209,
/// ADR-0055 § D5).** Tuple `(v0.sma, BTCUSDT, H1_2024, Synthetic)`; two runs
/// of it in `lab-runs/v0-paper-sma/reports/` (`LR209_RUNS`): the older one
/// falling, at the smaller stamp, the newer one rising.
///   F_both  — both runs: must equal F_new byte for byte.
///   F_new   — the newer run alone.
///   F_old   — the older run alone: the control. It must differ from F_new by
///             ≥ `CURVE_SWAP_MIN_PX`, so the identity cannot pass by drawing
///             the older curve.
///   F_none  — an empty root: no overlay. F_new must differ from it by
///             ≥ `CURVE_SWAP_MIN_PX`: the newer run does draw (#187), so the
///             identity cannot pass with F_both and F_new both blank.
/// PNGs (with `UI_DUMP_PNG`): `/tmp/lab_209_{both,new,old,none}.png`.
#[test]
fn lab_draws_the_newest_run_of_its_tuple() {
    assert!(
        LR209_RUNS[0].0 < LR209_RUNS[1].0 && LR209_RUNS[0].1 < LR209_RUNS[1].1,
        "precondition: the older run has the smaller stamp"
    );
    let slug = backtest::engine::strategy_dir_slug("v0.sma");
    let tmp = tempfile::tempdir().expect("tempdir");
    let scenes: [(&str, &[usize]); 4] = [
        ("both", &[0, 1]),
        ("new", &[1]),
        ("old", &[0]),
        ("none", &[]),
    ];
    let frames: Vec<Vec<u8>> = scenes
        .iter()
        .map(|(k, idx)| {
            let root = tmp.path().join(k);
            for &i in *idx {
                let (name, generated, falling) = LR209_RUNS[i];
                write_lab_run(
                    &root,
                    slug,
                    name,
                    &h1_run(
                        "sma_crossover",
                        "btcusdt-2024-h1-sma-cross-synthetic",
                        "BTCUSDT",
                        generated,
                        falling,
                    ),
                );
            }
            let (w, rgba) = render(lab_reading("v0.sma", &root));
            assert_eq!(w, VIEW_W, "{k}: frame width");
            dump_png(w, &rgba, &format!("/tmp/lab_209_{k}.png"));
            rgba
        })
        .collect();
    let differ_both = differing_pixels(&frames[0], &frames[1]);
    let differ_old = differing_pixels(&frames[2], &frames[1]);
    let differ_new_none = differing_pixels(&frames[1], &frames[3]);
    eprintln!(
        "#209 pixel margin LR-209: F_both vs F_new {differ_both} px (threshold == 0); control \
         F_old vs F_new {differ_old} px (threshold >= {CURVE_SWAP_MIN_PX}); control F_new vs \
         F_none {differ_new_none} px (threshold >= {CURVE_SWAP_MIN_PX})"
    );
    assert!(
        differ_old >= CURVE_SWAP_MIN_PX,
        "control: the older curve must look different from the newer one, but only \
         {differ_old} px differ"
    );
    assert!(
        differ_new_none >= CURVE_SWAP_MIN_PX,
        "control: the newer run must draw, but only {differ_new_none} px differ from the \
         no-overlay frame"
    );
    assert_eq!(
        differ_both, 0,
        "#209: with two runs of the tuple the Lab must draw the newer one, but {differ_both} px \
         differ"
    );
}
