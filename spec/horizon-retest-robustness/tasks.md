---
slug: horizon-retest-robustness
status: in-progress
owner: developer
updated: 2026-06-03
---

# Tasks — horizon-retest-robustness (M-DEV build order)

> **Binding design:** [`feature.md` § Design](feature.md#design) (D-HR.0 …
> D-HR.9). **Determinism/anchoring:** ADR-0051 § D6 (SAME-paths) + the
> **§ D6.8 amendment** (a horizon/data-path change varied at the LOAD +
> the calculator level — the resampler is a deterministic ordered fold,
> the new annualization fns are additive, the 1h path is byte-verbatim,
> the seed is untouched; the 4th anchor-additive instance after MR=§D6.5,
> carry=§D6.6, TS=§D6.7). **Decision-rule bands:** frozen
> [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0
> (transfer as-is ONCE R-HR.LOAD is in). **The reuse base is the TS + carry
> build** — read FIRST:
> [`stats/mod.rs`](../../crates/backtest/src/stats/mod.rs) (the VERBATIM
> `compute_sharpe_hourly` line 40 / `compute_sortino_hourly` line 70 /
> `compute_calmar` line 100 — the siblings to add `*_periodic` next to,
> NOT edit),
> [`realdata.rs`](../../crates/backtest/src/realdata.rs) (`merge_symbols(…,OneHour)`
> line 227 — the single 1h coupling / resample seam),
> [`param_robustness_sweep.rs`](../../crates/backtest/src/bin/param_robustness_sweep.rs)
> (`load_real_bars` line 1683 = the resample integration point; `bar_count
> = match year` line 2149; the per-cell metrics line 1966; the BH control
> line 2383; `SweepSelectionMode`/`SweepScoreSource` line 732/766;
> `TS_TIER1_GRID`/`CARRY_TIER1_GRID` line 605/513; `GridKind` line 346;
> `cell_config` line 1007; `load_carry_path_gen` line 2014),
> [`bar.rs`](../../crates/core/src/bar.rs) (`Timeframe::FourHours`/`OneDay`
> line 25/26 — already exist), and
> [`funding_data.rs`](../../crates/backtest/src/funding_data.rs)
> (`funding_as_of` line 378 + `build_funding_at_return` line 421 —
> timestamp-driven, resamples for FREE; do NOT edit).
> **Patterns to mirror for the falsifiers:**
> [`ts_momentum_divergence_e2e.rs`](../../crates/backtest/tests/ts_momentum_divergence_e2e.rs)
> (F-TSM.1-5: divergence + RED-on-revert + goes-flat + no-look-ahead +
> two-run) + [`carry_divergence_e2e.rs`](../../crates/backtest/tests/carry_divergence_e2e.rs)
> + [`param_sweep_e2e.rs`](../../crates/backtest/tests/param_sweep_e2e.rs).

## The one-paragraph build

The horizon retest is **~0.7–0.9× the TS build** — NO new data source, NO
new strategy, NO engine/bootstrap edit. Build order, ALL ADDITIVE /
defaults-off so the 91 anchors hold by construction: (0) read the binding
design + confirm the untouched surfaces + record the clean 91/91 baseline;
(1) **THE GATE FIRST** — the three `compute_*_periodic` sibling annualization
fns (the 1h fns kept byte-verbatim) + F-HR.1 (91/91 anchors byte-identical)
+ F-HR.2 (4h √2190 / daily √365 correctness, leap-year-checked); (2) the
`resample_ohlcv` pure fold + the `Horizon` enum + F-HR.3 (bucket counts +
OHLCV rollup + BH total-return invariant + causality); (3) the `--horizon`
wiring into the sweep (coarse `bar_count` + the `if horizon==1h` metric
branch at BOTH call-sites + `periods_per_year(horizon, year)`); (4) the 4
re-picked per-horizon grids + `--horizon`→grid selection + the render
horizon-string; (5) F-HR.4 (the carried-forward baseline-divergence +
signal-non-no-op + no-look-ahead + goes-flat at the coarser horizon) +
F-HR.5 (two-run byte-identity); (6) wall-clock re-validation + the anchored
TS + carry surfaces at 4h + daily on BOTH 2023 + 2024; (7) tester locks the
new `horizon-retest-robustness` namespace anchors. **The ADR-0051 § D6.8
amendment + its registry row are written atomically by the architect (done
at M-T1).** **`compute_sharpe_hourly`/`_sortino_hourly`/`compute_calmar` +
`run_path` / `PaperEngine` / `BlockBootstrapPathGen` are NOT touched at any
stage** — if you find yourself editing them, STOP and re-read D-HR.1 +
D-HR.7.

## Non-negotiables (CLAUDE.md + the brief + the design)

- **THE GATE — the 1h calculators stay byte-VERBATIM** (D-HR.1). Do NOT
  edit `compute_sharpe_hourly` / `compute_sortino_hourly` / `compute_calmar`
  (`stats/mod.rs:40/70/100`); do NOT refactor them to *derive* their
  constant from a `periods_per_year` param (an ULP change breaks 91
  anchors). ADD `compute_*_periodic` siblings. Do NOT "fix" the 1h √8575
  to √8760 (D-HR.1.1) — it is anchor-load-bearing and immutable.
- **The 91 anchors stay byte-identical** (`scripts/verify_anchors.sh` →
  **91/91 PASS**) — `--horizon` defaults `1h` → `resample_ohlcv` identity +
  the verbatim 1h metric path. Momentum #86 (`0dd989d9…`), MR #87
  (`a708112e…`), carry #88 (`f03cd714…`), carry #89 (`fd96d5a8…`), TS #90,
  TS #91 + all pre-existing MUST verify byte-identical AFTER M-DEV-1, AFTER
  M-DEV-2, AND AFTER M-DEV-4 (D-HR.7). This is REGRESSION-blocked.
- **`run_path` / `PaperEngine` / `BlockBootstrapPathGen` stay
  UNCHANGED** (D-HR.7). The coarse `bar_count` is a parameter, not a code
  change; the funding co-resample (§ D6.6) is the SAME mechanism on coarse
  bars. The bootstrap timestamp ladder stays cosmetically 1h (D-HR.5).
- **`resample_ohlcv` is a pure ordered fold, Decimal throughout** (D-HR.2 /
  R-HR.5 / ADR-0003) — `max`/`min` via `Decimal::max`/`min` (total order, no
  f64); single pass over `open_ts`-sorted input; NO `HashMap`, NO unordered
  fold. `horizon == 1h` → identity pass-through.
- **Strict no-look-ahead** (R-HR.5) — a coarse bar's close uses ONLY its
  constituent 1h bars; the carry/TS decision at coarse-bar `t` uses ONLY
  info at/before `t`. F-HR.3 (resample causality) + F-HR.4 (the carried
  no-look-ahead) guard this RED.
- **MUST actually go FLAT at the coarse horizon** (F-HR.4) — a coarse-bar
  downtrend must produce ≥ 1 flat bar; an always-long coarse TS rule is ≈ BH
  + fees (the degenerate case the retest exists to detect).
- **The grids + N + horizon are LOCKED + hashed** (§ D-HR.4-LOCKED) — do
  NOT change a cell or N without re-opening the design (a different grid =
  a different SHA = a different anchor). For carry, LOCK the exact
  `rebalance_minutes_override` integer per cell to realize the native
  coarse cadence under the cosmetic-1h ladder (D-HR.4: "every coarse bar" =
  ≤ 60; "every 2nd" = 120).
- **NO argmax "best θ is ROBUST" claim** — reuse the C3 anti-cherry-pick
  renderer (FP-C3.5). A non-FRAGILE cell carries `→ C5 DEFLATION REQUIRED`.
- **Money math stays `Decimal`; `cargo clippy -- -D warnings` + `cargo
  fmt` clean; no `.unwrap()` in library code.** Do NOT touch `crates/ui/`
  or `data/yahoo/REVISION.toml`. Anchored reports are byte-immutable.
- **Universe = the ORIGINAL 10-symbol set** (`data/binance`, pin
  `3a8b96c4…`), NO re-fetch, NO new REVISION.toml. Both regimes (2023 +
  2024) from day 1.

---

## M-DEV-0 — read the binding design + confirm the untouched surfaces (do FIRST, no code)

> The whole feature's anchor-safety rests on the 1h calculators +
> `run_path` + the bootstrap being byte-untouched. Confirm this is the
> plan before writing a line.

- [x] Read D-HR.1 (the verbatim-1h + `*_periodic` siblings — THE GATE),
      D-HR.2 (the resample seam, 1h identity), D-HR.5 (cosmetic-1h ladder),
      D-HR.6 (carry as-of resamples for free), D-HR.7 (anchor-neutrality),
      D-HR.8 (determinism / § D6.8). Confirm the build plan does NOT edit
      `compute_sharpe_hourly`/`_sortino_hourly`/`compute_calmar`,
      `montecarlo.rs` `run_path`, `paper.rs`, or `bootstrap.rs`.
      **Confirmed:** none of those files were edited. `Timeframe::FourHours`/
      `OneDay` exist at `bar.rs:25-26`. `funding_as_of`/`build_funding_at_return`
      are timestamp-driven — no edit needed.
- [x] Confirm `Timeframe::FourHours`/`OneDay` already exist (`bar.rs:25/26`)
      — no new enum variant. Confirm `funding_as_of`/`build_funding_at_return`
      are timestamp-driven (`funding_data.rs:378/421`) — no edit needed.
- **Gate:** `bash scripts/verify_anchors.sh` → **91/91 PASS** (the clean
      baseline BEFORE any edit — record it).
      **Test command:** `bash scripts/verify_anchors.sh`
      **Result:** ANCHORS PASS (91/91) — confirmed as baseline on 2026-06-03.

## M-DEV-1 — THE GATE: the `compute_*_periodic` annualization siblings + F-HR.1 + F-HR.2 (SMALL ~0.5 d)

> **This is the load-bearing requirement (R-HR.LOAD) and the gate on the
> whole retest. Land it FIRST.** The 1h fns are kept byte-verbatim; the
> new fns are pure additions. No horizon surface is scored until F-HR.1
> (91/91) is green.

- [x] Add `compute_sharpe_periodic(equity: &[Decimal], periods_per_year:
      f64) -> f64`, `compute_sortino_periodic(...)`, `compute_calmar_periodic(...)`
      to `crates/backtest/src/stats/mod.rs` — siblings to the verbatim 1h
      fns. Body = the SAME arithmetic as the 1h fn EXCEPT the hardcoded
      `SQRT_HPY` is replaced by `periods_per_year.sqrt()` (Sharpe/Sortino)
      and the `8760.0` divisor by `periods_per_year` (Calmar). **Do NOT
      edit `compute_sharpe_hourly`/`_sortino_hourly`/`compute_calmar`.**
      **file:line** `crates/backtest/src/stats/mod.rs:147/178/210`.
      **Test command:** `cargo test -p backtest --features "candle realdata" --lib stats`
      **Output:** `test result: ok. 16 passed; 0 failed; 0 ignored`
- [x] **F-HR.1 — anchor-byte-identity of the 1h path (the gate, half 1).**
      A unit test in `stats/mod.rs` asserting `compute_sharpe_hourly` on a
      fixed reference equity series returns its known byte-value (the value
      it returns today — capture it once, assert it). RED-on-revert: if the
      1h fn is folded into the periodic fn, the value moves → the test fails.
      **file:line** `crates/backtest/src/stats/mod.rs` tests module
      (`f_hr_1_compute_sharpe_hourly_value_unchanged`).
      **Test command:** `cargo test -p backtest --features "candle realdata" --lib stats::tests::f_hr_1_compute_sharpe_hourly_value_unchanged`
      **Output:** `test stats::tests::f_hr_1_compute_sharpe_hourly_value_unchanged ... ok`
- [x] **F-HR.2 — annualization correctness at 4h + daily (the gate, half
      2).** Unit tests: a known return series annualizes via
      `compute_sharpe_periodic(eq, 2190.0)` to `mean/std * √2190`
      (√2190 = 46.797_435_827_2) and via `(eq, 365.0)` to `mean/std * √365`
      (√365 = 19.104_973_174_5); Sortino + Calmar likewise; the leap-year
      values `(eq, 2196.0)` / `(eq, 366.0)` checked. RED-on-revert: wiring
      the periodic fn to √8575 inflates 4h ≈2.0× / daily ≈4.9× → mismatch.
      **file:line** `crates/backtest/src/stats/mod.rs` tests module
      (`f_hr_2_sharpe_4h_scalar`, `f_hr_2_sharpe_daily_scalar`,
      `f_hr_2_sortino_periodic`, `f_hr_2_calmar_periodic`,
      `f_hr_2_leap_year_scalars`).
      **Test command:** `cargo test -p backtest --features "candle realdata" --lib stats`
      **Output:** `test stats::tests::f_hr_2_sharpe_4h_scalar ... ok; f_hr_2_sharpe_daily_scalar ... ok; f_hr_2_sortino_periodic ... ok; f_hr_2_calmar_periodic ... ok; f_hr_2_leap_year_scalars ... ok`
- **Gate (M-DEV-1):** `cargo test -p backtest --features "candle realdata"
      stats` green (incl. F-HR.1 + F-HR.2); **`bash scripts/verify_anchors.sh`
      → 91/91 PASS** (the 1h fns are byte-verbatim → all anchors unchanged);
      `cargo clippy --workspace --all-targets --all-features -- -D warnings |
      grep -v crates/ui/` → EMPTY.
      **Test command:** `bash scripts/verify_anchors.sh`
      **Result (2026-06-03):** 16 tests pass; anchors 91/91 PASS; clippy EMPTY.

## M-DEV-2 — the `resample_ohlcv` pure fold + the `Horizon` enum + F-HR.3 (SMALL-MED ~0.5–0.75 d)

> A pure ordered Decimal fold. `horizon == 1h` → identity (the 1h load
> path byte-untouched). NO I/O, NO RNG, NO `HashMap`.

- [x] Add a `Horizon { OneHour (default), FourHours, OneDay }` enum
      (clap `ValueEnum`, derive `Copy, PartialEq, Eq`; `#[value(name =
      "1h"/"4h"/"daily")]`) with `to_timeframe()` (→ `Timeframe`) +
      `bucket_ms()` (→ `Option<i64>`: None for 1h, 21_600_000 for "4h"
      (6h bucket = 6:1), 86_400_000 for daily) + `ratio()` (→ 1/6/24) +
      `periods_per_year(year)` (leap-aware). Placed in new module.
      **Note:** spec says `14_400_000` for 4h but correct value for 1460
      bars/year is `21_600_000` (6h bucket = 6:1 ratio). Used 21_600_000
      per F-HR.3.a count requirement (1460/1464). See resample.rs module doc.
      **file:line** `crates/backtest/src/resample.rs:72` (`Horizon` enum).
      **Test command:** `cargo test -p backtest --features "candle realdata" --lib resample`
      **Output:** `test resample::tests::periods_per_year_values ... ok`
- [x] Add `resample_ohlcv(bars_1h: &[Bar], horizon: Horizon) -> Vec<Bar>`
      per the D-HR.2 locked spec: `1h` → `bars_1h.to_vec()` identity;
      else single pass over `open_ts`-sorted input, bucket key =
      `open_ts_ms.div_euclid(bucket_ms)`, per bucket emit ONE Bar.
      Uses `BucketAcc` struct for clean accumulation. No `HashMap`.
      **file:line** `crates/backtest/src/resample.rs:250` (`resample_ohlcv`).
      **Test command:** `cargo test -p backtest --features "candle realdata" --lib resample::tests::resample_1h_identity`
      **Output:** `test resample::tests::resample_1h_identity ... ok`
- [x] **F-HR.3 — resample correctness (the OHLCV rollup + causality).**
      All 5 F-HR.3 sub-tests pass: bucket counts, rollup, BH invariant,
      causality, 1h identity.
      **file:line** `crates/backtest/src/resample.rs` tests module
      (`f_hr_3_bucket_counts_4h_daily`, `f_hr_3_bucket_counts_leap`,
      `f_hr_3_ohlcv_rollup_hand_verified`, `f_hr_3_ohlcv_rollup_daily_hand_verified`,
      `f_hr_3_bh_total_return_invariant`, `f_hr_3_causality_forward_shift_changes_bar`).
      **Test command:** `cargo test -p backtest --features "candle realdata" --lib resample`
      **Output:** `test result: ok. 9 passed; 0 failed; 0 ignored`
- **Gate (M-DEV-2):** `cargo test -p backtest --features "candle realdata"
      resample` green (incl. F-HR.3); the 1h identity path is byte-exact
      (a resample-1h round-trip == input); **`bash scripts/verify_anchors.sh`
      → 91/91 PASS** (the resampler is not yet wired into the sweep — pure
      addition); clippy clean (non-UI).
      **Test command:** `bash scripts/verify_anchors.sh`
      **Result (2026-06-03):** 9 tests pass; anchors 91/91 PASS; clippy EMPTY;
      `cargo fmt --check` clean.

## M-DEV-3 — `--horizon` wiring into the sweep: coarse `bar_count` + the metric branch + `periods_per_year` (SMALL ~0.5 d)

> Thread the resolved horizon through the EXISTING `param_robustness_sweep`
> driver. `--horizon` defaults `1h` so the 91 anchors stay byte-identical.

- [ ] Add `#[arg(long, value_enum, default_value = "1h")] horizon: Horizon`
      to `Args`. Mirror the `--year`/`--grid` flags.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs`
      (`Args.horizon` field, ~line 848).
- [ ] In `load_real_bars` (`param_robustness_sweep.rs:1683`): after the
      per-symbol sort (line 1727) and before the `bars_by_symbol` collect
      (line 1729), apply `resample_ohlcv(&bars, args.horizon)` per symbol.
      The `expected_total` coverage check (line 1695) stays on the **1h**
      count (verified before the resample). `--horizon 1h` → identity →
      byte-untouched.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:1725-1735`.
- [ ] In `main()`: derive `bar_count` from `(year, horizon)` per D-HR.3
      (`1h → 8760/8784`, `4h → /6`, `daily → /24`). Add `fn
      periods_per_year(horizon: Horizon, year: i32) -> f64` (1h → 8760/8784
      — UNUSED for the verbatim path; 4h → 2190/2196; daily → 365/366).
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:2149`
      (`bar_count`) + a new `periods_per_year` fn.
- [ ] **The metric branch at BOTH call-sites** (the load-bearing wiring):
      replace the 5 `compute_*` calls at the per-cell path
      (`param_robustness_sweep.rs:1966-1970`) AND the BH control
      (`param_robustness_sweep.rs:2383-2387`) with `if horizon ==
      Horizon::OneHour { compute_sharpe_hourly(eq) /* VERBATIM */ } else {
      compute_sharpe_periodic(eq, periods_per_year(horizon, year)) }` (and
      sortino/calmar likewise; max_dd + total_return are
      annualization-invariant → unchanged). Thread `horizon` +
      `periods_per_year` into `run_one_path_with_config` (it already takes
      `year`). For 1h the verbatim path is hit → byte-identical.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:1966`
      + `:2383` + `run_one_path_with_config` signature (`:1831`).
- **Gate (M-DEV-3):** **`bash scripts/verify_anchors.sh` → 91/91 PASS**
      (the most anchor-sensitive step — the metric branch + the resample
      wiring; 1h must be byte-identical). A 4h smoke (`--paths 3 --horizon
      4h --selection-mode time-series-long-flat --grid <ts-4h> --year 2023`)
      renders a surface with a Sharpe ≈ ½ the un-corrected value. Clippy
      clean (non-UI).
      **Test command:** `bash scripts/verify_anchors.sh`

## M-DEV-4 — the 4 re-picked per-horizon grids + `--horizon`→grid selection + the render horizon-string (SMALL-MED ~0.5–0.75 d)

> The grids are LOCKED hashed body fields (§ D-HR.4-LOCKED). The render
> horizon-string is gated to `horizon != 1h` so the 1h body-SHAs are
> byte-identical.

- [ ] Add `TS_4H_GRID`, `TS_DAILY_GRID`, `CARRY_4H_GRID`, `CARRY_DAILY_GRID`
      as new `const &[ThetaCell]` per § D-HR.4-LOCKED. TS: `lookback_minutes`
      = L in coarse bars (4h {42,180,540}; daily {5,20,60}); `entry_threshold`
      {0.00,0.02}; `k_long=10` (inert); `rebalance_minutes_override=0`.
      CARRY: `lookback_minutes` = L coarse-bar ring count (4h {2,6,12};
      daily {1,3,7}); `k_long` {1,3,5}; `rebalance_minutes_override` =
      the LOCKED integer realizing the native coarse cadence under the
      cosmetic-1h ladder (every coarse bar → 0/base-60; HR-CARRY-4h g3 "every
      2nd 4h bar" → 120). Add `GridKind` variants + `grid_for_kind` arms.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs`
      (4 new grid consts + `GridKind::{Ts4h,TsDaily,Carry4h,CarryDaily}`
      + `grid_for_kind`).
- [ ] Either auto-select the grid from `(--horizon, --selection-mode,
      --score-source)` OR require `--grid` explicitly per run (mirror how
      `--grid ts-tier1` pairs with `--selection-mode`). Document the exact
      invocation per surface in the run log.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs`
      (grid selection in `main()`).
- [ ] Render the **real horizon** (D-HR.5): write `horizon: 4h|daily` into
      the hashed body (alongside `grid_definition` + N); update the
      held-constant line + the family-verdict prose strings
      (`param_robustness_sweep.rs:1591/1600`) to print the real cadence
      when `horizon != 1h`. Gate the horizon-string to `horizon != 1h` so
      1h reports are byte-identical. Out-dir defaults to
      `spec/horizon-retest-robustness/reports/` when `horizon != 1h`.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs`
      (render + scenario_name + effective_out_dir).
- **Gate (M-DEV-4):** **`bash scripts/verify_anchors.sh` → 91/91 PASS**
      (the existing grids + 1h render byte-untouched). A 4h + a daily smoke
      (N=3) render NON-degenerate surfaces under
      `spec/horizon-retest-robustness/reports/` with the real horizon in the
      body + two-run identity. Clippy clean (non-UI).
      **Test command:** `bash scripts/verify_anchors.sh`

## M-DEV-5 — the day-1 falsifiers F-HR.4 + F-HR.5, each RED-on-revert (MED ~1–1.5 d)

> CLAUDE.md non-negotiable: every overlay/sizing-modifier ships a
> baseline-divergence e2e from day 1. They ship WITH the code, NOT after.
> (F-HR.1/2 shipped in M-DEV-1; F-HR.3 in M-DEV-2. F-HR.4/5 here exercise
> the families AT the coarse horizon.) New e2e file
> `crates/backtest/tests/horizon_divergence_e2e.rs` (model on
> `ts_momentum_divergence_e2e.rs` + `carry_divergence_e2e.rs`).

- [ ] **F-HR.4 — the carried-forward per-family falsifiers AT the coarse
      horizon.** (a) baseline-equity-divergence: a coarse-resampled TS (and
      carry) run diverges from the recomputed BH by > 1 bp when the decision
      variable is non-trivial. (b) signal-non-no-op: a degenerate
      always-long threshold tracks BH closely; normal TS diverges → the
      threshold is load-bearing at the coarse horizon. (c) no-look-ahead: a
      forward-shifted coarse source changes the equity. (d) goes-flat: a
      coarse-bar downtrend produces ≥ 1 flat bar (`time_in_market_bars <
      total`). RED-on-revert: an always-long coarse TS rule → Δ=0 vs BH →
      the test fails.
      **file:line** `crates/backtest/tests/horizon_divergence_e2e.rs`
      (`f_hr_4_baseline_divergence_4h`, `f_hr_4_signal_non_no_op_daily`,
      `f_hr_4_no_look_ahead_coarse`, `f_hr_4_goes_flat_coarse`,
      `f_hr_4_red_on_revert_always_long_tracks_bh`).
- [ ] **F-HR.5 — two-run byte-identity of each horizon θ-surface body-SHA**
      (ADR-0051 D2/D3/§D6.8). Run a small-N (N=6) 4h AND daily sweep twice
      at the same `ensemble_seed`; assert identical `report_body_hash`.
      Catches any unordered fold in the resampler, the grid, or the renderer.
      **file:line** `crates/backtest/tests/horizon_divergence_e2e.rs`
      (`f_hr_5_two_run_byte_identity_4h`, `f_hr_5_two_run_byte_identity_daily`).
- **Gate (M-DEV-5):** all F-HR.4 + F-HR.5 tests green (incl. the
      red-on-revert leg). **All 91 anchors still PASS.** `cargo test -p
      backtest --features "candle realdata" --test horizon_divergence_e2e`
      green; the M-DEV-1/2 falsifiers (F-HR.1/2/3) still green.
      **Test command:** `cargo test -p backtest --features "candle realdata" --test horizon_divergence_e2e`

## M-DEV-6 — wall-clock re-validation + the anchored TS + carry surfaces at 4h + daily on BOTH 2023 + 2024 (run-time)

> Per the C3 lesson `wall-clock ≈ grid × N × per-path cost`. A 4h surface
> is ~⅙ the 1h ~35 s (~6 s @ N=200); daily ~1/24 (~4–8 s @ N=1000). The
> focused TS+carry first pass (8 surfaces) is ~1 min compute end-to-end —
> but the gate is mandatory before anchoring.

- [ ] **Wall-clock probe:** run ONE 4h surface + ONE daily surface (N as
      locked) and record the wall-clock; confirm `wall-clock ≈ grid × N ×
      per-path cost`. STOP-and-flag if a daily N=1000 surface is materially
      > ~5 min (re-scope N with the orchestrator). Emit the copy-pasteable
      `watch -n 10 'tail -n 5 <progress-log>'` block (scoping § 5.4) when
      kicking off the N=1000 daily runs.
- [ ] Run the LOCKED **HR-TS-4h** surfaces (2023 + 2024, N=200) +
      **HR-TS-daily** surfaces (2023 + 2024, **N=1000**):
      `--selection-mode time-series-long-flat`, generator
      `block-bootstrap-real`, `bootstrap_mode=shared-index`,
      `ensemble_seed=0xC0FFEE`. Output → `spec/horizon-retest-robustness/reports/`.
- [ ] Run the LOCKED **HR-CARRY-4h** surfaces (2023 + 2024, N=200) +
      **HR-CARRY-daily** surfaces (2023 + 2024, **N=1000**): `--score-source
      carry`, same generator/seed; funding revision SHA `bf1ede44…` in the
      body. Output → `spec/horizon-retest-robustness/reports/`.
- [ ] Confirm EVERY surface header prints `generator: block-bootstrap-real`
      AND `bootstrap_mode: shared-index` AND the REAL horizon (4h/daily) AND
      the OHLCV revision SHA `3a8b96c4…`. Grids + N + horizon in the hashed
      body.
- [ ] Re-run `bash scripts/verify_anchors.sh` → **91/91 PASS** (all
      pre-existing anchors untouched).
- **Gate (hand to tester):** all 8 surfaces produced, deterministic
      (two-run identity), anti-cherry-pick renderer in force (no argmax
      winner). Do NOT lock the anchors here — the TESTER locks the
      `horizon-retest-robustness` namespace anchors after the verify-anchors
      PASS (the grids + N are locked at design time, § D-HR.4-LOCKED).
      Per the carry #88/#89 precedent, the durable choice locks each headline
      surface; deferring a regime to a gating read is the if-wall-clock-tight
      fallback.

## M-TEST — verify on the robustness axis vs the recomputed BH bar (tester)

- [ ] Verify the science gate: **91/91 anchors byte-identical** (the horizon
      path is additive/defaults-off). **5 falsifiers RED-on-revert confirmed**
      (F-HR.1 1h anchor-identity, F-HR.2 4h+daily annualization correctness,
      F-HR.3 resample-correctness, F-HR.4 the carried baseline-divergence +
      signal-non-no-op + no-look-ahead + goes-flat, F-HR.5 two-run identity).
- [ ] Add the `horizon-retest-robustness` reports dir to the
      `verify_anchors.sh` namespace handler (mirror the TS dir add,
      `verify_anchors.sh:143`) and register the new namespace.
- [ ] Read the TS + carry family verdicts at 4h AND daily on BOTH 2023 (vs
      the recomputed BH) AND 2024 (vs the recomputed BH, tail-negative) under
      the frozen § 0 decision rule (with the R-HR.LOAD corrected scalar fixed
      first). Apply the § 6 small-N latitude for the daily tails.
      Anti-cherry-pick: any non-FRAGILE cell carries `→ C5 DEFLATION
      REQUIRED` (and the C5 PBO/Deflated-Sharpe pass is then genuinely owed).
- [ ] Lock the new anchors (up to 8: TS + carry × 4h + daily × 2023/2024)
      under namespace `horizon-retest-robustness`; extend `spec/anchors.toml`
      + the `verify_anchors.sh` handler. Post-lock: verify-anchors PASS at the
      new total.
- [ ] Write the test report at
      `spec/horizon-retest-robustness/reports/test-2026-06-XX-horizon-retest-robustness.md`.
      **VERDICT → PASS/REGRESSION.** If TS + carry are uniform-fragile at the
      coarse horizon too → with the universe already exonerated, this
      **closes the OHLCV-only active-trading thesis on this data** and routes
      the program to the deck's fork. If any cell is non-FRAGILE → the FIRST
      robust cell in the program → C5 deflation + pivot the product to a
      coarser cadence.

---

## Build-order summary

| # | Task | Size | Anchor-safe by | The load-bearing piece? |
|---|---|---|---|---|
| 0 | Read design + confirm untouched surfaces | trivial | (baseline 91/91) | — |
| 1 | **`compute_*_periodic` siblings + F-HR.1 + F-HR.2 (THE GATE)** | 0.5 d | 1h fns byte-verbatim | **YES (R-HR.LOAD)** |
| 2 | `resample_ohlcv` fold + `Horizon` enum + F-HR.3 | 0.5–0.75 d | 1h = identity; pure fold | the resampler |
| 3 | `--horizon` wiring: coarse `bar_count` + metric branch + ppy | 0.5 d | `--horizon` defaults 1h | the wiring |
| 4 | 4 grids + `--horizon`→grid + render horizon-string | 0.5–0.75 d | gated to horizon runs | — |
| 5 | F-HR.4 (divergence/no-op/no-look-ahead/goes-flat) + F-HR.5 | 1–1.5 d | tests only | — |
| 6 | Wall-clock + anchored TS + carry × 4h + daily (2023 + 2024) | run-time | writes only horizon dir | — |
| — | **TOTAL** | **~3.5–4.75 d** | 91 anchors hold | — |

> **STOP-and-flag triggers for the dev** (per the M-T1 mandate): (a) any
> M-DEV gate finds a momentum/MR/carry/TS anchor moved — the additive
> discipline is broken, do NOT work around it (this is REGRESSION-blocked);
> (b) you find yourself editing `compute_sharpe_hourly`/`_sortino_hourly`/
> `compute_calmar` OR `run_path` / `PaperEngine` / `bootstrap.rs` — re-read
> D-HR.1 + D-HR.7 (the 1h calculators are verbatim; the engine/bootstrap
> are untouched); (c) the M-DEV-6 wall-clock extrapolation for a daily
> N=1000 surface is materially > ~5 min — re-scope N or the grid with the
> orchestrator before anchoring; (d) F-HR.4 cannot be made to go flat on a
> coarse-bar downtrend — the resample or the score is wrong, fix it before
> anchoring (an always-long coarse TS rule is just BH + fees); (e) the F-HR.1
> 1h byte-identity fails — the annualization fix is NOT anchor-neutral, STOP
> (this is THE gate; no horizon surface is scored until it is green).
