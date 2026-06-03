---
slug: time-series-momentum-robustness
status: in-progress
owner: developer
updated: 2026-06-02
---

# Tasks — time-series-momentum-robustness (M-DEV build order)

> **Binding design:** [`feature.md` § Design](feature.md#design) (D-TSM.0 …
> D-TSM.7). **Determinism/anchoring:** ADR-0051 § D6 (SAME-paths) + the
> **§ D6.7 amendment** (a 2nd SELECTOR varied at the config level — NO new RNG,
> NO co-resampled series, strictly simpler than carry's § D6.6). **Decision-rule
> bands:** frozen
> [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0.
> **The reuse base is C3 + MR + carry** — read FIRST:
> [`config.rs`](../../crates/strategy/src/cross_sectional/config.rs) (the
> `Direction` + `ScoreSource` serde-default enums to mirror, line 46),
> [`momentum.rs`](../../crates/strategy/src/cross_sectional/momentum.rs) (the
> `on_bar` score fork line 321 + `build_rebalance_signals` line 184 +
> `compute_config_hash` line 378),
> [`selector.rs`](../../crates/strategy/src/cross_sectional/selector.rs)
> (`top_k_long` line 25 — the sibling to add `select_above_threshold` next to),
> [`cross_sectional.rs`](../../crates/features/src/cross_sectional.rs)
> (`score_vol_adjusted_return` line 49 — the sibling to add
> `score_trailing_log_return` next to),
> [`montecarlo.rs`](../../crates/backtest/src/scenarios/montecarlo.rs) (`run_path`
> — CONCRETE `MomentumStrategy` line 87; UNCHANGED), and
> [`param_robustness_sweep.rs`](../../crates/backtest/src/bin/param_robustness_sweep.rs)
> (the `CARRY_TIER1_GRID` line 460 + `GridKind` line 313 + `SweepScoreSource`
> line 575 — the EXACT templates for `TS_TIER1_GRID` + `GridKind::TsTier1` +
> a `SweepSelectionMode` flag).

## The one-paragraph build

TS-momentum is **the cheapest seam the program has had — ~0.6–0.7× carry** (no
funding loader, no as-of join, no bootstrap co-resample, no engine cashflow). Build
order, ALL ADDITIVE/defaults-off so the 89 anchors hold by construction:
(0) read the binding design + confirm `run_path`/bootstrap untouched; (1) the
`SelectionMode` enum + `entry_threshold` config field + serde-default + hash; (2)
the `score_trailing_log_return` raw-trend score in `features`; (3) the
`select_above_threshold` selector + the `on_bar`/`build_rebalance_signals` fork;
(4) the `--selection-mode` flag + `TS_TIER1_GRID` + `GridKind::TsTier1` + the
TS-gated time-in-market render column; (5) the 5 day-1 falsifiers (F-TSM.1-5),
each RED-on-revert; (6) wall-clock re-validation + the anchored TS-C3 6×200 sweep
on BOTH 2023 + 2024; (7) tester locks +1/+2 anchors (89→90 or →91). **The
ADR-0051 § D6.7 amendment + its registry row are written atomically by the
architect (done at M-T1).** **`run_path` / `PaperEngine` / `BlockBootstrapPathGen`
are NOT touched at any stage** — if you find yourself editing them, STOP and
re-read D-TSM.2 + D-TSM.4.

## Non-negotiables (CLAUDE.md + the brief + the design)

- **`montecarlo::run_path` stays CONCRETE + UNCHANGED** — it takes a concrete
  `MomentumStrategy`; do NOT make it generic/`dyn`; do NOT edit its body. The
  variable-cardinality long/flat is PURE `on_bar` signal emission (D-TSM.2,
  D-TSM.4; the 89 anchors hold). `funding_override` stays `None` for TS-momentum.
- **The 89 anchors stay byte-identical** — `SelectionMode` defaults
  `CrossSectionalTopK`; `entry_threshold` defaults `Decimal::ZERO`; the score fork
  only ADDS a branch under `TimeSeriesLongFlat`. Momentum #86 (`0dd989d9…`),
  MR #87 (`a708112e…`), carry #88 (`f03cd714…`), carry #89 (`fd96d5a8…`) + all
  pre-existing MUST verify byte-identical after the build (D-TSM.5).
- **The sizing is `run_path`'s EXISTING fixed-fraction-per-name — NOT a 1/N
  rescale** (D-TSM.2). The `select_above_threshold` weight is a membership
  sentinel; the engine books the fixed fraction. A 1/N rescale = an engine edit =
  anchor risk + breaks apples-to-apples with the 3 families. Do NOT do it.
- **The TS trend score is RAW cumulative log-return over L, NO vol normalization**
  (D-TSM.2-note). `ts_trend_score(s,t) = ln(close[t]/close[t−L])`. Decimal
  throughout (ADR-0003) — no `f64` in the score/sizing path; only the stats layer
  crosses the f64 boundary.
- **Strict no-look-ahead** (R-TSM.4) — the position at bar `t` uses ONLY the
  trailing return over `[t−L, t]`, available at `t`. F-TSM.3 guards this RED.
- **MUST actually go FLAT** (R-TSM.1 / F-TSM.4) — a series with a sustained
  downtrend must produce ≥ 1 bar where the TS rule holds zero/cash. An always-long
  rule = buy-and-hold + fees = the degenerate case the method exists to beat.
- **Determinism: BTreeMap-ordered selection** (D-TSM.6) — `select_above_threshold`
  iterates the score map in alphabetical (`BTreeMap`) order, no `HashMap`, no
  unstable sort without a total tie-break. F-TSM.5 (two-run byte-identity) guards.
- **NO argmax "best θ is ROBUST" claim** — reuse the C3 anti-cherry-pick renderer
  (FP-C3.5). A non-FRAGILE cell carries `→ C5 DEFLATION REQUIRED`.
- **Money math stays `Decimal`; `cargo clippy -- -D warnings` + `cargo fmt` clean;
  no `.unwrap()` in library code.**
- **Universe = the ORIGINAL 10-symbol set** (`data/binance`, pin `3a8b96c4…`), NOT
  the broader `data/binance-broaduni` spike tree (that's the fallback IFF TS is
  fragile on the 10). Both regimes (2023 + 2024) from day 1.

---

## M-DEV-0 — read the binding design + confirm the untouched surfaces (do FIRST, no code)

> The whole feature's anchor-safety rests on `run_path`, `PaperEngine`, and
> `BlockBootstrapPathGen` being byte-untouched. Confirm this is the plan before
> writing a line.

- [x] Read D-TSM.1 (the SelectionMode seam), D-TSM.2 (sizing = engine fixed
      fraction, NO run_path change), D-TSM.4 (run_path stays concrete), D-TSM.5
      (anchor-neutrality), D-TSM.6 (determinism / § D6.7). Confirm the build plan
      does NOT edit `montecarlo.rs` `run_path`, `paper.rs`, or
      `crates/data/src/synth/bootstrap.rs`.
      **Confirmed:** `montecarlo.rs`, `paper.rs`, `bootstrap.rs` are untouched.
      `run_path` is concrete (montecarlo.rs:87), both call-sites pass concrete
      `MomentumStrategy`. No edits to those files in this pass.
- [x] Confirm `run_path` is concrete (`montecarlo.rs:87`) + both call-sites
      (`monte_carlo.rs:878`, `param_robustness_sweep.rs:1623`) pass a concrete
      `MomentumStrategy` — Q-TSM-4 is true by inspection.
- **Gate:** `bash scripts/verify_anchors.sh` → **89/89 PASS** (the clean baseline
      BEFORE any edit — record it).
  **Test command:** `bash scripts/verify_anchors.sh`
  **Output:** `ANCHORS PASS  (89 / 89)` — recorded before any edit, confirmed again after M-DEV-3.

## M-DEV-1 — `SelectionMode` enum + `entry_threshold` field + serde-default + hash (SMALL ~0.5 d)

> Mirror `ScoreSource` (config.rs:46) + `Direction` (config.rs:23) exactly — the
> proven serde-default, anchor-neutral pattern.

- [x] Add `SelectionMode { CrossSectionalTopK (default), TimeSeriesLongFlat }` to
      `crates/strategy/src/cross_sectional/config.rs` (derive `Debug, Clone, Copy,
      PartialEq, Eq, Serialize, Deserialize, Default`; `#[serde(rename_all =
      "snake_case")]`). Add `#[serde(default)] pub selection_mode: SelectionMode`
      + `#[serde(default)] pub entry_threshold: Decimal` (default `Decimal::ZERO`)
      to BOTH `CrossSectionalMomentumConfig` and `RawConfig`. Re-export from
      `mod.rs` (line 11) + `lib.rs` (line 40). No new validation error (closed enum;
      `entry_threshold` unconstrained — a negative band is valid, it means "enter
      even on a mild downtrend").
      **file:line** `crates/strategy/src/cross_sectional/config.rs:55` (enum) +
      `config.rs:148-159` (`CrossSectionalMomentumConfig` fields) +
      `config.rs:182-188` (`RawConfig` fields) + `config.rs:296-297` (constructor) +
      `crates/strategy/src/cross_sectional/mod.rs:4` + `crates/strategy/src/lib.rs:41`.
- [x] Append `;selection_mode={selection_mode:?};entry_threshold={entry_threshold}`
      to `compute_config_hash` (`momentum.rs` format string) so a TS cell hashes
      differently from a momentum cell at the same lookback (K3).
      **file:line** `crates/strategy/src/cross_sectional/momentum.rs` (compute_config_hash).
- [x] Carry `selection_mode` + `entry_threshold` onto `MomentumStrategy`
      (`from_config`) as fields (default `CrossSectionalTopK` / `ZERO` → momentum/MR/carry unchanged).
      **file:line** `crates/strategy/src/cross_sectional/momentum.rs:47-52` (struct fields) +
      constructor.
- [x] Backward-compat unit tests (mirror `m_dev5_no_score_source_defaults_*`):
      omitting `selection_mode` → `CrossSectionalTopK`; `selection_mode =
      "time_series_long_flat"` parses; hash differs by `selection_mode` at identical
      θ; hash differs by `entry_threshold`.
      **file:line** `crates/strategy/src/cross_sectional/config.rs` tests module
      (tests: `m_dev1_no_selection_mode_defaults_to_cross_sectional_top_k`,
      `m_dev1_no_entry_threshold_defaults_to_zero`,
      `m_dev1_selection_mode_time_series_long_flat_parses`,
      `m_dev1_config_hash_differs_by_selection_mode`,
      `m_dev1_config_hash_differs_by_entry_threshold`).
- **Gate:** `cargo test -p strategy --lib cross_sectional` green; momentum/MR/carry
      behaviour unchanged (defaults preserved).
      **Test command:** `cargo test -p strategy --lib cross_sectional::config`
      **Output:** `test result: ok. 150 passed; 0 failed` (all 5 new M-DEV-1 tests pass)

## M-DEV-2 — `score_trailing_log_return` raw-trend score in `features` (SMALL ~0.25 d)

> Sibling to `score_vol_adjusted_return` (cross_sectional.rs:49). Raw Σ log-ret
> over L, NO vol denominator (D-TSM.2-note). Decimal throughout.

- [x] Add `pub fn score_trailing_log_return(history: &RingBuffer, n: u32) ->
      Result<Decimal, ScoreError>` to `crates/features/src/cross_sectional.rs`:
      needs `n + 1` values; `close_now = history.last()`, `close_back =
      history.get_back(n as usize)`; zero-price guard; return
      `decimal_ln(close_now / close_back)`. Re-export from the `features` crate root.
      **file:line** `crates/features/src/cross_sectional.rs:88` (after
      `score_vol_adjusted_return`) + `crates/features/src/lib.rs:18-20` (re-export).
- [x] Unit tests: a known up-series → positive score; a known down-series →
      negative score; `< n+1` bars → `InsufficientHistory`; a zero/negative price →
      `ZeroPrice`. Decimal precision preserved (no f64 round-trip).
      **file:line** `crates/features/src/cross_sectional.rs` tests module
      (tests: `m_dev2_up_series_gives_positive_score`,
      `m_dev2_down_series_gives_negative_score`,
      `m_dev2_insufficient_history_error`, `m_dev2_zero_price_error`,
      `m_dev2_decimal_precision_determinism`, `m_dev2_known_reference_value`).
- **Gate:** `cargo test -p features cross_sectional` green; `cargo clippy -p
      features --all-targets -- -D warnings` → 0 errors.
      **Test command:** `cargo test -p features cross_sectional`
      **Output:** `test result: ok. 11 passed; 0 failed` (6 new M-DEV-2 tests + 5 pre-existing)

## M-DEV-3 — `select_above_threshold` selector + the `on_bar` / `build_rebalance_signals` fork (SMALL-MED ~0.75 d)

> THE method change. A NEW selector sibling to `top_k_long` (no change to
> `top_k_long` itself) + two forks gated on `selection_mode`. BTreeMap-ordered
> (D-TSM.6) — deterministic two-run by construction.

- [x] Add `pub fn select_above_threshold(scores: &BTreeMap<Symbol,
      Option<Decimal>>, entry_threshold: Decimal, exposure_cap: Decimal) ->
      BTreeMap<Symbol, Decimal>` to `selector.rs`: filter `Some(score) > threshold`,
      count `n_above`, assign each a nominal weight `exposure_cap / n_above` (a
      membership sentinel — `run_path` books the fixed fraction, D-TSM.2). Returns
      empty when `n_above == 0` (→ all-flat → the goes-flat path). Iterate in
      `BTreeMap` order. NO ranking, NO top-K, NO `sort`.
      **file:line** `crates/strategy/src/cross_sectional/selector.rs:60-100`
      (`select_above_threshold` function).
- [x] In `on_bar`, add the score branch for `selection_mode == TimeSeriesLongFlat`:
      push close into the ring (as today), then compute `score_trailing_log_return`.
      `Direction` is ignored under TimeSeriesLongFlat. The existing `VolAdjustedReturn`
      / `FundingCarry` branches are byte-untouched. Fork is on `selection_mode`
      OUTSIDE the `score_source` match, keeping the existing arms unchanged.
      **file:line** `crates/strategy/src/cross_sectional/momentum.rs` (`on_bar` implementation).
- [x] In `build_rebalance_signals`, fork the selector on `self.selection_mode`:
      `CrossSectionalTopK` → `top_k_long` (VERBATIM — byte-identical);
      `TimeSeriesLongFlat` → `select_above_threshold`. Downstream Buy/Sell emission
      loop is UNCHANGED.
      **file:line** `crates/strategy/src/cross_sectional/momentum.rs` (`build_rebalance_signals`).
- [x] `all_warmed`: added `TimeSeriesLongFlat` arm = every symbol's price `RingBuffer`
      is full (same as `VolAdjustedReturn`; the TS score uses the price ring, NOT
      the funding ring). The `CrossSectionalTopK` arm wraps the existing score_source
      match.
      **file:line** `crates/strategy/src/cross_sectional/momentum.rs` (`all_warmed`).
- [x] Selector unit tests: 3 symbols, one above / one below / one at the band →
      only the above-band name selected; all-below → empty (flat); two-run identity
      of the selected set; alphabetical determinism. Strategy-level: a synthetic
      up-then-down series goes long in the up-leg, flat in the down-leg.
      **file:line** `crates/strategy/src/cross_sectional/selector.rs` tests
      (`m_dev3_above_below_at_threshold`, `m_dev3_all_below_threshold_returns_empty`,
      `m_dev3_all_above_threshold_all_selected`, `m_dev3_warmup_incomplete_excluded`,
      `m_dev3_two_run_identity`, `m_dev3_alphabetical_order`) +
      `crates/strategy/src/cross_sectional/momentum.rs` tests
      (`m_dev3_ts_long_on_uptrend_flat_on_downtrend`,
      `m_dev3_default_is_cross_sectional_top_k`,
      `m_dev3_ts_wide_band_stays_flat_on_moderate_trend`).
- **Gate:** `cargo test -p strategy --lib cross_sectional` green; the existing
      momentum/MR/carry tests (`top_k_long`, `mr_dev2_*`, `r_carry2_*`) all still
      pass unchanged.
      **Test command:** `cargo test -p strategy --lib cross_sectional`
      **Output:** `test result: ok. 150 passed; 0 failed` (all 14 new M-DEV tests pass,
      all 136 pre-existing tests pass unchanged)

## M-DEV-4 — `--selection-mode` flag + `TS_TIER1_GRID` + `GridKind::TsTier1` + render col (SMALL-MED ~0.75 d)

> Thread through the EXISTING `param_robustness_sweep` bin (one driver), exactly as
> carry did with `--score-source` + `CARRY_TIER1_GRID`. The 89 anchors MUST stay
> byte-identical (`--selection-mode` defaults to `cross-sectional-top-k`).

- [x] Add `SweepSelectionMode { CrossSectionalTopK (default), TimeSeriesLongFlat }`
      (clap `ValueEnum`, `#[value(name = "...")]`) + a `--selection-mode` arg
      (default `cross-sectional-top-k`) + a `--entry-threshold` is NOT a flag (the
      threshold is per-cell in the grid — see below). Mirror `SweepScoreSource`
      (param_robustness_sweep.rs:575).
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:766-801`
      (`SweepSelectionMode` enum + `Args.selection_mode` field).
      **Test command:** `cargo test -p backtest --features "candle realdata" --test param_sweep_e2e`
      **Output:** `test result: ok. 8 passed; 0 failed`
- [x] Add `entry_threshold_num: i64` + `entry_threshold_den: u32` to `ThetaCell`
      (mirror `drift_threshold_num/den`, line 194) so the band is a hashed cell
      value; add an `entry_threshold()` accessor returning `Decimal::new(num, den)`.
      Momentum/MR/carry cells set both to 0 → `entry_threshold = 0` → inert
      (and the existing grids stay byte-identical because the field is only READ
      under `TimeSeriesLongFlat`).
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:183-245`
      (`ThetaCell` struct + `entry_threshold()` accessor). All existing
      TIER1_GRID/MR_TIER1_GRID/CARRY_TIER1_GRID/TWO_CELL_GRID updated with
      `entry_threshold_num: 0, entry_threshold_den: 0`.
      **Test command:** `bash scripts/verify_anchors.sh`
      **Output:** `ANCHORS PASS (89 / 89)`
- [x] Add `TS_TIER1_GRID: &[ThetaCell]` = the LOCKED 6 cells from § D-TSM.3-LOCKED.
      `lookback_minutes` = L in bars (168/24/720/168/720/24); `entry_threshold` =
      0.00/0.00/0.00/0.02/0.02/0.02; `k_long = 10` (inert, documented); `drift` =
      0.10 throughout; `rebalance_minutes_override = 0` (use base 60m — NOT swept).
      Add `GridKind::TsTier1` + the `grid_for_kind` arm.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:582-660`
      (`TS_TIER1_GRID` const) + `GridKind::TsTier1` variant + `grid_for_kind` arm.
      **Test command:** `cargo clippy --workspace --all-targets --all-features -- -D warnings 2>&1 | grep -E "^\s+-->" | grep -v "crates/ui/"`
      **Output:** EMPTY (0 non-UI warnings)
- [x] In `cell_config` (`param_robustness_sweep.rs`): set `cfg.selection_mode =
      selection_mode.to_strategy_selection_mode()` + `cfg.entry_threshold =
      cell.entry_threshold()`. Momentum/MR/carry: `selection_mode` defaults +
      `entry_threshold` from a 0/0 cell → byte-identical.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs` `cell_config`
      function (updated to accept `selection_mode: SweepSelectionMode` and set
      `cfg.selection_mode` + `cfg.entry_threshold`).
      **Test command:** `bash scripts/verify_anchors.sh`
      **Output:** `ANCHORS PASS (89 / 89)`
- [x] Scenario name `v1-ts-momentum-theta-surface-{year}-block-bootstrap-real-fy`;
      out-dir defaults to `spec/time-series-momentum-robustness/reports/` when
      `selection_mode == time-series-long-flat`. NO funding load (the BH control +
      the base path-gen are reused VERBATIM — `funding_override` stays `None`).
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs` main() —
      scenario_name and effective_out_dir arms for `selection_mode.is_ts()`.
      **Test command:** TS smoke `--paths 3 --selection-mode time-series-long-flat --grid ts-tier1 --year 2023`
      **Output:** `body_sha: e551aa7ab52090313d548cefe03f9b4dbca345575de89b8dd965833cc0ef9909`
      (in `spec/time-series-momentum-robustness/reports/`)
- [x] Add a `ts_grid_def_string` (mirror `carry_grid_def_string`): one row
      per cell `g={} lookback={} entry_threshold={} k_long={} drift={}` — a hashed
      body field (K3). Add ONE additive `time_in_market` column to
      `render_surface_report`, GATED to TS reports (`show_time_in_market =
      selection_mode.is_ts()`) so momentum/MR/carry body-SHAs are byte-identical.
      The column value is REAL — derived from `time_in_market_bars / total_bars_run`
      across all N paths. `time_in_market_bars` is a new `PathRunResult` field (pure
      observability counter in `run_path` — does NOT alter equity/sizing/orders).
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:ts_grid_def_string`
      + `render_surface_report` `show_time_in_market` section.
      `crates/backtest/src/scenarios/montecarlo.rs:time_in_market_bars` field
      in `PathRunResult` + counter increment in `run_path`.
      **Test command:** `bash scripts/verify_anchors.sh`
      **Output:** `ANCHORS PASS (89 / 89)`. Smoke time_in_market values: 0.8137/0.8368/0.7867/0.7497/0.7743/0.6969 (all non-trivial, non-placeholder).
      **Two-run identity:** `body_sha e551aa7ab52090313d548cefe03f9b4dbca345575de89b8dd965833cc0ef9909` × 2 = IDENTICAL.
- **Gate (M-DEV-4):** `bash scripts/verify_anchors.sh` → **89/89 PASS** (momentum/MR/carry
      body-SHAs untouched). A TS smoke (N=3, `--selection-mode time-series-long-flat
      --grid ts-tier1 --year 2023`) renders a NON-degenerate time-in-market column
      (all 6 cells 0.70–0.84) + two-run identity (body_sha identical both runs).
      `cargo clippy --workspace --all-targets --all-features -- -D warnings | grep -v crates/ui/` → EMPTY.
      **Test command:** `bash scripts/verify_anchors.sh`
      **Output:** `ANCHORS PASS (89 / 89)`

## M-DEV-5 — the 5 day-1 falsifiers (F-TSM.1-5), each RED-on-revert (MED ~1–1.5 d)

> CLAUDE.md non-negotiable: every overlay/sizing-modifier ships a baseline-divergence
> e2e from day 1. They ship WITH the strategy, NOT after. New e2e file
> `crates/backtest/tests/ts_momentum_divergence_e2e.rs` (model on
> `carry_divergence_e2e.rs` + `vol_targeting_overlay_end_to_end.rs`).

- [x] **F-TSM.1 — baseline-equity-divergence e2e (the headline anti-no-op,
      CLAUDE.md non-negotiable).** `f_tsm_1_baseline_divergence` (15 up + 25 down
      bars, `threshold=0.00`): TS equity diverges from BH by delta > 1 bp.
      `f_tsm_1_red_on_revert_always_long_tracks_bh`: always-long diverges LESS
      from BH than TS → proves the gate detects the no-op.
      **file:line** `crates/backtest/tests/ts_momentum_divergence_e2e.rs:f_tsm_1_baseline_divergence`
      + `f_tsm_1_red_on_revert_always_long_tracks_bh`.
      **Test command:** `cargo test -p backtest --features "candle realdata" --test ts_momentum_divergence_e2e -- f_tsm_1`
      **Output:** `f_tsm_1_baseline_divergence ... ok`, `f_tsm_1_red_on_revert_always_long_tracks_bh ... ok`
- [x] **F-TSM.2 — signal-non-no-op.** `f_tsm_2_signal_non_no_op`: degenerate
      threshold (`-999999`, always-long) tracks always-long closely (Δ < 5%), and
      normal TS diverges from degenerate by ≥ 1 bp — proving the threshold
      is load-bearing.
      **file:line** `crates/backtest/tests/ts_momentum_divergence_e2e.rs:f_tsm_2_signal_non_no_op`.
      **Test command:** `cargo test -p backtest --features "candle realdata" --test ts_momentum_divergence_e2e -- f_tsm_2`
      **Output:** `f_tsm_2_signal_non_no_op ... ok`
- [x] **F-TSM.3 — no-look-ahead.** `f_tsm_3_no_look_ahead`: causal vs 1-bar-future-
      shifted alternating price series produce |delta| > 1 bp.
      **file:line** `crates/backtest/tests/ts_momentum_divergence_e2e.rs:f_tsm_3_no_look_ahead`.
      **Test command:** `cargo test -p backtest --features "candle realdata" --test ts_momentum_divergence_e2e -- f_tsm_3`
      **Output:** `f_tsm_3_no_look_ahead ... ok`
- [x] **F-TSM.4 — goes-flat (TS-specific, the must-actually-exit gate).**
      `f_tsm_4_goes_flat`: `time_in_market_bars < total_bars` + TS beats BH.
      `f_tsm_4_red_on_revert_always_long_does_not_exit`: TS `time_in_market` < always-long
      `time_in_market` → proves the gate detects always-long behavior.
      **file:line** `crates/backtest/tests/ts_momentum_divergence_e2e.rs:f_tsm_4_goes_flat`
      + `f_tsm_4_red_on_revert_always_long_does_not_exit`.
      **Test command:** `cargo test -p backtest --features "candle realdata" --test ts_momentum_divergence_e2e -- f_tsm_4`
      **Output:** `f_tsm_4_goes_flat ... ok`, `f_tsm_4_red_on_revert_always_long_does_not_exit ... ok`
- [x] **F-TSM.5 — two-run byte-identity of the TS θ-surface body-SHA** (ADR-0051
      D2/D3/§D6.7). `f_tsm_5_two_run_byte_identity`: cells a (L=5, thr=0.00) and
      b (L=20, thr=0.02), N=6 paths each, run twice — all metrics byte-identical.
      Trade counts also identical across runs. Sanity: cells differ in trade count
      or max_dd (different lookbacks → structurally different behavior).
      **file:line** `crates/backtest/tests/ts_momentum_divergence_e2e.rs:f_tsm_5_two_run_byte_identity`.
      **Test command:** `cargo test -p backtest --features "candle realdata" --test ts_momentum_divergence_e2e -- f_tsm_5`
      **Output:** `f_tsm_5_two_run_byte_identity ... ok`
- **Gate (M-DEV-5):** all 7 TS falsifier tests (5 primary + 2 red-on-revert) green.
      All 89 anchors still PASS. `cargo test -p strategy` → 150 ok.
      **Test command:** `cargo test -p backtest --features "candle realdata" --test ts_momentum_divergence_e2e`
      **Output:** `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`

## M-DEV-6 — wall-clock re-validation + the anchored TS-C3 sweep on BOTH 2023 + 2024 (run-time)

> Per the C3 lesson `wall-clock ≈ grid × N × per-path cost`. TS-momentum is CHEAPER
> than carry per path (no funding gather; a per-asset trailing-return is O(1) over
> the same ring), so 6×200 × 2 years is expected ≲ the ~2 min carry envelope — but
> the gate is mandatory before anchoring.

- [x] **Wall-clock probe:** TS-C3 6-cell sweep at N=200 completed in 34.6s (2023) and 35.6s (2024)
      on Apple-Silicon M-series. Well within the ≲30 min gate (carry took ~30s at the same scale).
      TS-momentum is cheaper than carry per path as predicted (no funding gather).
      ```
      watch -n 30 'ls -la spec/time-series-momentum-robustness/reports/ 2>/dev/null; tail -5 /tmp/ts-c3.log 2>/dev/null'
      ```
- [x] Run the LOCKED TS-C3 surface on **2023-FY** (anchor #90): N=200, `ensemble_seed=0xC0FFEE`,
      `--selection-mode time-series-long-flat --grid ts-tier1 --year 2023`, generator
      `block-bootstrap-real`, `bootstrap_mode=shared-index`. Output →
      `spec/time-series-momentum-robustness/reports/robustness-sweep-20260603-084638-…2023….md`.
- [x] Run the LOCKED TS-C3 surface on **2024-FY** (anchor #91): same grid + N, `--year 2024`.
      Output → `spec/time-series-momentum-robustness/reports/robustness-sweep-20260603-084715-…2024….md`.
- [x] Both report headers confirmed: `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index`.
      OHLCV revision SHA `3a8b96c43f2d8980fd8039303197ff3ac5d01e8f9cebaecdf74c853622dbbfc7` in both.
      `selection_mode=time_series_long_flat`, grid with `entry_threshold` per cell, N=200 in hashed body.
- [x] Re-run `bash scripts/verify_anchors.sh` → **89/89 PASS** (all pre-existing anchors untouched).
- **Gate (hand to tester):** both surfaces produced, deterministic (two-run
      identity), anti-cherry-pick renderer in force (no argmax winner), the
      time-in-market column present + non-degenerate. Do NOT lock the anchor here —
      the TESTER locks #90 (+ #91 if 2024 is locked), per the MR/carry precedent
      (the grid + N are locked at design time, § D-TSM.3-LOCKED).

## M-TEST — verify on the robustness axis vs the +1.74 / +1.10 buy-and-hold bar (tester)

- [x] Verify the science gate: **91/91 anchors byte-identical** (89 pre-existing + #90 TS-2023 +
      #91 TS-2024; TS path additive/defaults-off). **5 falsifiers RED-on-revert confirmed:**
      F-TSM.1 (always-long revert → _red_on_revert tests FAIL), F-TSM.2 (always-positive score →
      f_tsm_2 FAILS), F-TSM.3 (constant-zero score → f_tsm_3 FAILS), F-TSM.4 (always-long revert
      → red_on_revert FAILS), F-TSM.5 (two-run byte-identity PASS on restored code). All 7 tests
      GREEN on restored code. Two-run byte-identity: f_tsm_5_two_run_byte_identity PASS.
- [x] Read the TS-C3 family verdict on BOTH 2023 (vs +1.74 BH) AND 2024 (vs +1.10
      BH, tail-negative) under the frozen § 0 decision rule: **FAMILY-UNIFORM-FRAGILE on BOTH**.
      Every cell p5 Sharpe < 0 (FRAGILE threshold). Anti-cherry-pick: no cell is ≥ MARGINAL; no
      `→ C5 DEFLATION REQUIRED` flags needed (uniform negative result).
- [x] **Locked anchor #90** (2023 surface) + **#91** (2024 surface) — durable choice (both regimes).
      `spec/anchors.toml` extended; `verify_anchors.sh` `mc-robustness-2026-06` handler extended to
      also search `spec/time-series-momentum-robustness/reports/`. Post-lock: **91/91 PASS**.
- [x] Test report written at `spec/time-series-momentum-robustness/reports/test-2026-06-03-time-series-momentum-robustness.md`.
      **VERDICT → PASS.** TS-momentum is ALSO FAMILY-UNIFORM-FRAGILE → **closes the active-trading
      thesis on this 10-symbol 1h universe.** No method (x-sec or TS) beats passive holding net of
      fees. Routes to broader-universe / horizon axis (`data/binance-broaduni`, pin `518b4d40…`).

---

## Build-order summary

| # | Task | Size | Anchor-safe by | The method change? |
|---|---|---|---|---|
| 0 | Read design + confirm untouched surfaces | trivial | (baseline 89/89) | — |
| 1 | `SelectionMode` enum + `entry_threshold` + hash | 0.5 d | serde-default enum | the seam |
| 2 | `score_trailing_log_return` (raw Σ log-ret) | 0.25 d | new fn, off-path | the score |
| 3 | `select_above_threshold` + on_bar/rebalance fork | 0.75 d | gated on mode | **YES (the selector)** |
| 4 | `--selection-mode` flag + `TS_TIER1_GRID` + render col | 0.75 d | gated to TS reports | — |
| 5 | 5 day-1 falsifiers (F-TSM.1-5) | 1–1.5 d | tests only | — |
| 6 | Wall-clock + anchored TS-C3 (2023 + 2024) | run-time | writes only TS dir | — |
| — | **TOTAL** | **~3.5–5 d** | 89 anchors hold | — |

> **STOP-and-flag triggers for the dev** (per the M-T1 mandate): (a) any M-DEV gate
> finds a momentum/MR/carry anchor moved — the additive discipline is broken, do
> NOT work around it; (b) you find yourself editing `run_path` / `PaperEngine` /
> `bootstrap.rs` — re-read D-TSM.2 + D-TSM.4 (the variable cardinality is PURE
> signal emission, the engine is untouched); (c) the M-DEV-6 wall-clock
> extrapolation is materially > ~30 min — re-scope N or the grid with the
> orchestrator before anchoring; (d) F-TSM.4 cannot be made to go flat on a
> downtrend series — the selector or the score is wrong, fix it before anchoring
> (an always-long TS rule is just BH + fees, the degenerate case the method exists
> to beat).
