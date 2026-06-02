---
slug: carry-strategy
status: arch-done
owner: architect → developer
updated: 2026-06-02
---

# Tasks — carry-strategy (M-DEV build order)

> **Binding design:** [`feature.md` § Design](feature.md#design) (D-CARRY.0 …
> D-CARRY.8). **Determinism/anchoring:** ADR-0051 § D6 (SAME-paths) + the **§ D6.6
> real-mechanism amendment** (a 2nd series co-resampled under the shared index).
> **Decision-rule bands:** frozen
> [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0.
> **The reuse base is C3 + MR** — read
> [`param_robustness_sweep.rs`](../../crates/backtest/src/bin/param_robustness_sweep.rs),
> the MR tasks ([`../cross-sectional-mean-reversion-strategy/tasks.md`](../cross-sectional-mean-reversion-strategy/tasks.md)),
> [`bootstrap.rs`](../../crates/data/src/synth/bootstrap.rs) (the crux loop, line 265),
> [`realdata.rs`](../../crates/backtest/src/realdata.rs) (the loader to mirror), and
> [`montecarlo.rs`](../../crates/backtest/src/scenarios/montecarlo.rs) (run_path; equity push line 281) FIRST.

## The one-paragraph build

Carry is **3–5× MR**: it needs a NEW funding source loaded, aligned, **co-resampled
through the block bootstrap by the SAME `idx_seq` as the returns** (the crux, § D-CARRY.7
— TRACTABLE, ~15 lines at the existing loop), and a NEW per-bar funding-cashflow
accrual in `run_path`. Build order, all ADDITIVE/defaults-off so the 87 anchors hold
by construction: (1) the funding loader + REVISION pin, (2) the as-of forward-fill on
the real grid, (3) the shared-index resample → `GeneratedPath.funding_by_symbol`
(Option), (4) thread funding through `merge`→`funding_override`→`run_path`, (5) the
`ScoreSource::FundingCarry` signal + the load-bearing sign + the settlement-ring,
(6) the funding-cashflow accrual at the equity push, (7) the day-1 BOTH-axes gate +
the 4 mandatory falsifiers, (8) the LOCKED 6-cell carry-C3 sweep on BOTH 2023 + 2024,
(9) lock +1 anchor (87→88). **Wall-clock re-validation (6×200) is a gate before
anchoring.** **Remove the disposable frame-diagnostic flags from `param_robustness_sweep.rs`
(M-DEV-0) before anything else.**

## Non-negotiables (CLAUDE.md + the brief + the design)

- **`montecarlo::run_path` stays CONCRETE** — it takes a concrete `MomentumStrategy`;
  do NOT make it generic/`dyn`. Carry is a `ScoreSource`-on-config variation of
  `MomentumStrategy` + a parallel funding injection (D-CARRY.1; the 87 anchors hold).
- **`Bar` is NOT extended** — funding rides in parallel as `funding_by_symbol`
  (Option) / `funding_override` (Option). Touching `Bar` is REJECTED (D-CARRY.1, ADR § D6.6.3).
- **The 87 anchors stay byte-identical** — every funding seam is gated on an `Option`
  defaulting absent; `score_source` defaults `VolAdjustedReturn`. Momentum #86
  (`0dd989d9…`) + MR #87 (`a708112e…`) θ-surfaces MUST verify byte-identical after the build.
- **The SIGN is load-bearing** (R-CARRY.2): positive funding → longs pay shorts;
  harvest the PAID side. Framing (a) LONGS the most-**negative**-funding names.
  `carry_score` returns `−trailing_mean(funding)` so the most-negative name tops the
  unchanged descending `top_k_long`. **The R-CARRY.2 sign-assertion test ships day 1**
  and goes RED if the sign is flipped (a flip silently inverts carry into a funding-PAYER).
- **The funding cashflow MUST measurably move equity** (CLAUDE.md v3-vol-overlay
  non-negotiable) — R-CARRY.10b forces the cashflow to zero and asserts the equity
  collapses to the no-funding case. A computed-and-ignored cashflow = RED.
- **Money math stays `Decimal`** (ADR-0003) — the funding accrual + the
  realized-funding column are `Decimal`; only the stats layer crosses the f64 boundary.
- **No `.unwrap()` in library code; `cargo clippy -- -D warnings` + `cargo fmt` clean.**
- **NO argmax "best θ is ROBUST" claim** — reuse the C3 anti-cherry-pick renderer (FP-C3.5).
- **Re-verify the sign against the banked data** (brief § Scope) — a positive-funding
  period on a real symbol should correspond to a perp trading above its index — before
  locking the carry direction.

---

## M-DEV-0 — REMOVE the disposable frame-diagnostic flags (housekeeping, do FIRST)

> Left in `param_robustness_sweep.rs` by the frame-diagnostic (2026-05-31). The
> diagnostic note's § 5 said `git checkout` reverts them, but they are now committed
> (the diagnostic ran on the working tree). They MUST be removed as a clean edit.

- [x] Remove the `match_slippage_bps` CLI arg + doc (`param_robustness_sweep.rs:507-511`)
      and `match_taker_fee_bps` (`:513-517`).
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:501-505` (post-edit; was lines 507-517).
- [x] Remove the two params from `run_one_path_with_config` (`:1230-1232` — the
      `slippage_bps: u32, taker_fee_bps: u32` params + the DISPOSABLE-DIAGNOSTIC comment).
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:1218-1228`.
- [x] Restore the hardcoded `slippage_bps: 2, taker_fee_bps: 4` literal in the
      `TcnScenarioInput` construction — back to anchor-#86-reproducing state.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:1278-1280`.
- [x] Remove the two call-site args from the rayon loop.
      **file:line** `crates/backtest/src/bin/param_robustness_sweep.rs:1435-1446`.
- **Gate:** `bash scripts/verify_anchors.sh` → 87/87 PASS (all anchors including #86 + #87).
  **Test command:** `bash scripts/verify_anchors.sh`
  **Output:** `ANCHORS PASS  (87 / 87)`

## M-DEV-1 — `FundingDataSource` loader + REVISION pin (Sub-A; SMALL-MED ~0.5–1 d)

> Mirror `RealDataBarSource` (`realdata.rs`) on the funding root. Tiny data (240
> parquets, ~1,095 rows/sym-yr). Schema: `symbol` Utf8, `funding_time` Int64 ms,
> `funding_rate` Utf8 decimal-string (confirmed `fetch_binance_funding.rs:26-33` +
> on-disk REVISION.toml).

- [x] New module `crates/backtest/src/funding_data.rs` (sibling to `realdata.rs`):
      `FundingDataSource { funding_root: PathBuf, universe: Vec<Symbol> }` with a
      `load(span, scenario_name) -> Result<LoadedFunding, FundingDataError>`.
      **file:line** `crates/backtest/src/funding_data.rs:112-295`.
- [x] Verify `data/binance-funding/REVISION.toml` exactly as the OHLCV loader does:
      reuse `data::revision::{read_manifest_raw, file_sha256, compute_aggregate_sha}`;
      the expected aggregate SHA is `bf1ede44e57d797b57e5a4f2743f58027e4eba12d91e1ffaf883dcdd49365668`.
      Mismatch → `FundingDataError::RevisionMismatch` (mirror the OHLCV error enum).
      **file:line** `crates/backtest/src/funding_data.rs:155-206` (revision check) + constant at line 35.
- [x] Read the 3-column funding parquet via leaner polars `scan_parquet`; parse `funding_rate`
      via `rust_decimal::Decimal::from_str` (NOT f64). Output `Vec<FundingRow>` (leaner tuple — no
      `next_funding_ts`/`poll_ts`). `FundingRow` at line 88.
      **file:line** `crates/backtest/src/funding_data.rs:88-94` (struct) + `207-268` (parse loop).
- [x] Files-for-span helper: mirror `RealDataBarSource::files_for_span` (the funding
      layout is identical `<SYM>/<YEAR>/<MM>.parquet`).
      **file:line** `crates/backtest/src/funding_data.rs:299-341`.
- [x] Unit tests: REVISION-mismatch rejection; a known parquet parses to the expected
      rows (real-data integration test, `--include-ignored`); `funding_rate` decimal precision
      preserved (no f64 round-trip); out-of-span filter.
      **file:line** `crates/backtest/src/funding_data.rs:448-700` (tests module).
- **Gate:** `cargo test -p backtest --features "realdata candle" --lib funding_data` → 10 passed, 0 failed.
  `cargo clippy -p backtest --features "realdata candle" --lib -- -D warnings` → 0 errors.
  **Test command:** `cargo test -p backtest --features "realdata candle" --lib funding_data`
  **Output:** `test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 63 filtered out`

## M-DEV-2 — as-of forward-fill on the REAL grid (Sub-B; SMALL ~0.5 d)

> Deterministic step-function join. Computed ONCE on the real data, length `T−1`
> (matches the return series the bootstrap builds). NO look-ahead.

- [x] A pure function `funding_as_of(funding: &[(i64, Decimal)], bar_open_ts_ms: &[i64])
      -> Vec<Option<Decimal>>`: for each bar open ts, the funding rate from the last
      settlement **at or before** that ts. Before the first settlement → `None` (warm-up).
      Uses `partition_point` for O(log n) binary search per bar.
      **file:line** `crates/backtest/src/funding_data.rs:360-396`.
- [x] Build `funding_at_return[s][k]` via `build_funding_at_return(...)`: aligns to T-1
      return steps (slices `bar_ts[..T-1]`, the bars the returns depart FROM).
      Convention locked: bar k's open_ts → `funding_at_return[sym_i][k]`.
      **file:line** `crates/backtest/src/funding_data.rs:420-445`.
- [x] Unit test (the no-look-ahead falsifier, R-CARRY.6): shifting the funding series
      one settlement into the FUTURE changes the as-of result (proves the join is
      causal); a bar before the first settlement → `None`.
      **file:line** `crates/backtest/src/funding_data.rs:516-548` (`no_look_ahead_falsifier`).
- **Gate:** `cargo test -p backtest --features "realdata candle" --lib funding_data` → all green.
  **Test command:** `cargo test -p backtest --features "realdata candle" --lib funding_data`
  **Output:** `test result: ok. 10 passed; 0 failed; 1 ignored; 0 measured; 63 filtered out`

## M-DEV-3 — co-resample funding through the bootstrap by the SHARED index (Sub-C, THE CRUX; MED ~1.5–2.5 d)

> § D-CARRY.7 + ADR-0051 § D6.6. The ~15-line gather is easy; the THREADING is the
> bulk. ZERO new RNG draws — `idx_seq` is already a materialized `Vec<usize>`.

- [x] Add `funding_by_symbol: Option<Vec<Vec<Option<Decimal>>>>` to `GeneratedPath`
      (`crates/data/src/synth/mod.rs:34`), defaulting `None`. Updated struct doc with
      the co-resampling invariant and bar-0 convention.
      **file:line** `crates/data/src/synth/mod.rs:34-73`
- [x] Add `funding_at_return: Option<Vec<Vec<Option<Decimal>>>>` to
      `BlockBootstrapPathGen` + `with_funding(...)` builder method (returns `self`).
      When `None`, `generate` behaves byte-identically to the pre-carry code.
      **file:line** `crates/data/src/synth/bootstrap.rs:71-155`
- [x] In `generate`'s reconstruction loop (`bootstrap.rs:265`), gather
      `funding_at_return[sym_i][ret_idx]` into a per-symbol output vec by the
      **SAME `ret_idx`** — zero new RNG draws. Bar-0 carries `funding_at_return[sym_i][0]`
      as the sentinel. Emits `GeneratedPath.funding_by_symbol = Some(...)`.
      **file:line** `crates/data/src/synth/bootstrap.rs:266-392`
- [x] Determinism test: `funding_co_resample_same_seed_deterministic` — same seed
      twice → byte-identical `funding_by_symbol`.
      **file:line** `crates/data/src/synth/bootstrap.rs:895-940`
- [x] Index-alignment test (FP-C1.5 sibling, THE CRUX): `funding_index_aligned_co_movement`
      — uses an integer-tag funding source; verifies the resampled funding decodes to
      the same source index as the bar's log-return. 0 misaligned bars (strict).
      **file:line** `crates/data/src/synth/bootstrap.rs:941-1073`
- [x] Anchor-neutrality test: `funding_none_is_byte_identical_bars` — with
      `with_funding(None)`, bars are byte-identical to the base generator.
      **file:line** `crates/data/src/synth/bootstrap.rs:844-893`
- [x] Thread funding into the run input: added `funding_override: Option<BTreeMap<...>>`
      to `TcnScenarioInput` (`crates/backtest/src/cli_types.rs`). Default `None`
      everywhere — all existing construction sites updated (main.rs, engine.rs,
      param_robustness_sweep.rs, monte_carlo.rs, threshold_sweep.rs, all test files).
      **file:line** `crates/backtest/src/cli_types.rs:500-543`
- **Gate:** `cargo test -p data synth::bootstrap` → 15/15 PASS.
      `bash scripts/verify_anchors.sh` → 87/87 PASS.
      `cargo test -p backtest --features "candle realdata" --test montecarlo_e2e` → 9/9 PASS.
      **Test command:** `cargo test -p data synth::bootstrap`
      **Output:** `test result: ok. 15 passed; 0 failed; 0 ignored; 0 measured`

## M-DEV-4 — `ScoreSource::FundingCarry` + the sign + the settlement-ring (Signal, PROBLEM 1; MED ~1–1.5 d)

> **Stage 2 note:** The brief's M-DEV-4 "thread funding to run_path (seam (ii),
> D-CARRY.1)" was completed as the final bullet in M-DEV-3 above (the
> `funding_override: Option<BTreeMap<...>>` field on `TcnScenarioInput`). The
> remaining M-DEV-4 items (ScoreSource enum, sign logic, settlement-ring) are Stage 3.

> Sibling to MR's `Direction` (the proven serde-default pattern), BUT carry needs its
> OWN funding ring (counts SETTLEMENTS, not price bars).

- [x] Add `ScoreSource { VolAdjustedReturn (default), FundingCarry }` to
      `crates/strategy/src/cross_sectional/config.rs` (derive `Copy, PartialEq, Eq,
      Serialize, Deserialize, Default`; `#[serde(rename_all="snake_case")]`). Add
      `#[serde(default)] pub score_source: ScoreSource` to `CrossSectionalMomentumConfig`
      + `RawConfig`. Re-export from `mod.rs` + `lib.rs`. No new validation error
      (closed enum). Backward-compat unit test: omitting `score_source` → `VolAdjustedReturn`.
      **file:line** `config.rs:44` (enum + field) + `mod.rs`/`lib.rs` re-export; tests
      `m_dev5_score_source_funding_carry_parses` + `m_dev5_no_score_source_defaults_to_vol_adjusted_return`.
- [x] Append `;score_source={score_source:?}` to `compute_config_hash`
      (`momentum.rs:244`) so carry-vs-momentum at the same θ hashes differently (K3).
      Unit test the hash differs.
      **file:line** `momentum.rs:387`; test `m_dev5_config_hash_differs_by_score_source`.
- [x] Carry the funding lookup onto `MomentumStrategy` (a new
      `funding_by_symbol_ts: Option<BTreeMap<(Symbol, Timestamp), Decimal>>` field +
      a settlement-ring per symbol) via a `with_funding(...)` setter the harness calls
      after `from_config`. Default `None` → momentum/MR unchanged.
      **file:line** `momentum.rs:139` (`with_funding`) + the per-symbol `funding_rings`.
- [x] In `on_bar` (`momentum.rs:203`), fork the score on `score_source`:
      `VolAdjustedReturn` = the EXISTING `score_vol_adjusted_return` path (byte-identical);
      `FundingCarry` = `carry_score(&bar.symbol, bar.open_ts)` = **`−trailing_mean`** of
      the last L SETTLED funding rates at-or-before `open_ts` (the leading minus is the
      sign — § D-CARRY.1). A symbol with < L settlements seen → `None` (excluded from
      the rank, same as a warming-up momentum score). `Direction` stays `Momentum`
      (identity) for carry — the sign lives in `carry_score`, not in `Direction`.
      **file:line** `momentum.rs:321-322` (fork) + `momentum.rs:275` (`carry_score`; −mean at :305).
- [x] **R-CARRY.2 sign-assertion test (day-1 mandatory):** a synthetic universe with a
      known-POSITIVE-funding symbol and a known-NEGATIVE one + K=1; assert the carry
      strategy SELECTS (LONGS) the NEGATIVE-funding name (the paid side), and goes RED
      if the sign in `carry_score` is flipped.
      **file:line** `momentum.rs:729` (`r_carry2_sign_assertion_longs_negative_funding_name`)
      + `r_carry2_carry_score_negative_funding_outscores_positive`.
- [x] **No-look-ahead test (R-CARRY.6, strategy level):** the carry score at a bar uses
      only funding settled at-or-before its `open_ts` (re-assert at the strategy seam,
      complementing the M-DEV-2 join test).
      **file:line** `momentum.rs` `r_carry6_no_look_ahead_strategy_level`.
- **Gate:** `cargo test -p strategy cross_sectional` green; momentum behavior unchanged
      (`score_source` defaults `VolAdjustedReturn`).
      **Output:** `cargo test -p strategy --lib` → `test result: ok. 136 passed; 0 failed`.

## M-DEV-5 — the funding-cashflow accrual in `run_path` (Sub-D; SMALL-MED ~0.5–1 d)

> The mechanism that makes carry ≠ a price tilt. At the existing equity push
> (`montecarlo.rs:281`), gated on `funding_override` present. `Decimal` throughout.

- [x] In `run_path`, accept the funding lookup from `input.funding_override` (already
      threaded M-DEV-3). Pass it to the strategy (`with_funding`) AND keep a copy for the accrual.
      **file:line** `montecarlo.rs:138-139` (`funding_map_for_accrual` clone + `strategy.with_funding`).
- [x] Immediately BEFORE the per-bar equity push (`montecarlo.rs:276-281`): if funding
      is present AND the bar's `open_ts` is a funding-settlement boundary
      (`bar_index % 8 == 0` on the synthetic hourly grid — § D-CARRY.7; lock the exact
      convention incl. whether bar 0 settles), for each held LONG position accrue
      `cash += position_notional × (−funding_rate)` (framing (a): earns on
      negative-funding names, pays on positive). At-most-once per 8h block per position.
      `funding_rate` = the resampled `funding_by_symbol[s][k]` for this (symbol, ts).
      **file:line** `montecarlo.rs:283` (settlement-boundary accrual; `Decimal` money math throughout).
- [x] **R-CARRY.10b funding non-no-op test (day-1 mandatory, CLAUDE.md v3-vol-overlay
      analogue):** run a small synthetic carry path WITH the accrual and with the
      accrual forced to zero; assert the equity curves DIVERGE (the WITH case ≠ the
      zero case by ≥ ε). RED if the cashflow is computed-and-ignored. Pattern:
      `crates/strategy/tests/vol_targeting_overlay_end_to_end.rs`.
      **file:line** `montecarlo.rs` `r_carry10b_funding_cashflow_non_no_op` — **SINGLE-SYMBOL
      isolation**: the same symbol is selected with AND without funding, so the only difference
      is the cashflow (no 2-symbol alphabetical-tie-break confound). Asserts `diff > ε`
      (non-no-op) AND `equity_with > equity_zero` (longs EARN on the negative-funding name).
- [x] **Anchor-neutrality:** with `funding_override=None`, `run_path` equity is
      byte-identical to today (the accrual block is never entered) — re-assert via the
      existing `run_path` unit test + the anchor gate.
      **file:line** `montecarlo.rs` `run_path_funding_none_is_anchor_neutral` + the 87/87 anchor gate.
- **Gate:** `cargo test -p backtest montecarlo` green; `bash scripts/verify_anchors.sh` → 87/87.
      **Output:** `cargo test -p backtest --features "candle realdata" --lib` → `70 passed; 0 failed`;
      `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (87 / 87)`.

## M-DEV-6 — `--score-source carry` flag + `CARRY_TIER1_GRID` on the sweep bin

> Thread through the EXISTING `param_robustness_sweep` bin (one driver), exactly as MR
> did with `--direction`. The momentum #86 + MR #87 anchors MUST stay byte-identical.

- [ ] Add `--score-source {vol-adjusted-return,carry}` (default `vol-adjusted-return`)
      to the bin CLI. Add a `--funding-root` arg (default `data/binance-funding/`) +
      a `--funding-revision-sha` arg (default `bf1ede44…`), used only when `carry`.
- [ ] Add `CARRY_TIER1_GRID: &[ThetaCell]` const = the LOCKED 6 cells from
      § D-CARRY.2-LOCKED. **NB the `lookback` column is L SETTLEMENTS, not minutes** —
      either add an `l_settlements` field to `ThetaCell` (preferred — explicit) or
      document the reinterpretation; the dev's call, but the cell value hashed MUST be
      the L the design locked (9/3/21/9/9/3). Add a `CarryTier1` `GridKind` variant (or
      select grid by `(score_source, grid)`).
- [ ] When `score_source == carry`: build + REVISION-verify the funding via
      `FundingDataSource`, compute the as-of `funding_at_return`, pass it to
      `BlockBootstrapPathGen::with_funding`, and build the per-path `funding_override`
      in `run_one_path_with_config`. Scenario name
      `v1-carry-theta-surface-{year}-block-bootstrap-real-fy`; out-dir default
      `spec/carry-strategy/reports/`.
- [ ] Reuse `classify_verdict`, `derive_path_seed`, `run_buyhold_path`,
      `prepare_generator_params` VERBATIM. The buy-and-hold control row runs UNCHANGED
      (no funding — it is the +1.74/+1.10 bar carry must clear; the control is a price-
      only benchmark by design).
- [ ] Extend `render_surface_report` with ONE additive column — the per-cell
      **realized-funding-harvested** total (`Decimal`, fixed precision) — GATED to
      carry reports (`score_source == carry`) so the momentum/MR body-SHAs are
      byte-identical (the same gating MR used for its trade-count column, ADR § D6.5.4).
- **Gate:** `--score-source vol-adjusted-return --grid tier1` reproduces momentum #86
      byte-identical; `--direction reversion --grid mr-tier1` reproduces MR #87; the
      FP-C3.x two-run identity tests pass. `bash scripts/verify_anchors.sh` → 87/87.

## M-DEV-7 — the day-1 BOTH-axes gate + divergence falsifier (R-CARRY.9-10; e2e)

> CLAUDE.md non-negotiable: every overlay/sizing-modifier ships a baseline-divergence
> e2e from day 1. Carry's are the divergence + non-no-op + sign + look-ahead falsifiers.

- [ ] **R-CARRY.10a carry-vs-price divergence (the headline falsifier).** New e2e
      `crates/backtest/tests/carry_divergence_e2e.rs` (model on
      `crates/backtest/tests/mr_divergence_e2e.rs`): SAME small synthetic path +
      funding series, run a `carry` strategy and a `vol_adjusted_return` strategy;
      assert the selected-symbol sets DIFFER on ≥ 1 rebalance AND the equity curves
      diverge by ≥ 1 bp. Construct the universe so the highest-funding names are NOT
      the highest-momentum names → guaranteed selection divergence. Tests carry is a
      genuinely DIFFERENT return source, not a relabelled price bet.
- [ ] **Two-run byte-identity of the carry θ-surface body-SHA** (ADR § D6.6.5 / D6.4):
      run the small-N carry sweep twice at the same `ensemble_seed`; assert identical
      `report_body_hash`. Catches any unordered fold in the funding resample/gather or
      the carry renderer (model on `param_sweep_e2e.rs`).
- [ ] Confirm R-CARRY.10b (M-DEV-5), the R-CARRY.2 sign-assertion (M-DEV-4), and the
      no-look-ahead falsifier (M-DEV-2 + M-DEV-4) are all present and RED-on-revert.
- **Gate:** the full carry test file green; each falsifier verified RED when its
      guarded property is broken (divergence → no-op; sign → flipped; cashflow → zeroed;
      look-ahead → future-shifted). `cargo test -p backtest carry` + `-p strategy`.

## M-DEV-8 — wall-clock re-validation + the anchored carry-C3 sweep on BOTH 2023 + 2024

> Per the C3 lesson `wall-clock ≈ grid × N × per-path cost`. The funding gather is
> O(n_bars) per path (negligible), but the gate is mandatory before anchoring.

- [ ] **Wall-clock probe:** run the carry-C3 6-cell sweep at a reduced N (e.g. N=20)
      first; extrapolate to N=200; confirm ≲ ~25-30 min (C3 was ~20 min). If materially
      larger, STOP and flag the orchestrator (do not silently anchor a slow run).
      Emit a watch block for the full run (per the long-running-task recipe):
      ```
      watch -n 30 'ls -la spec/carry-strategy/reports/ 2>/dev/null; tail -5 /tmp/carry-c3.log 2>/dev/null'
      ```
- [ ] Run the LOCKED carry-C3 surface on **2023-FY** (the apples-to-apples anchor
      deliverable, #88): N=200, `ensemble_seed=0xC0FFEE`, `--score-source carry
      --grid carry-tier1 --year 2023`, generator `block-bootstrap-real`,
      `bootstrap_mode=shared-index`. Output → `spec/carry-strategy/reports/`.
- [ ] Run the LOCKED carry-C3 surface on **2024-FY** (the frame-diagnostic E1
      multi-regime day-1 gating read): same grid + N, `--year 2024`. Both surfaces are
      read against their respective buy-and-hold controls at M-TEST.
- [ ] Confirm both report headers print `generator: block-bootstrap-real` AND
      `bootstrap_mode: shared-index` (the pre-flight void-if-fail) + both revision SHAs
      (OHLCV `3a8b96c4…` + funding `bf1ede44…`) in the hashed body.
- [ ] Re-run `bash scripts/verify_anchors.sh` → 87/87 still PASS (the carry runs wrote
      only to the carry reports dir; #86/#87 untouched).
- **Gate (hand to tester):** both surfaces produced, deterministic (two-run identity),
      anti-cherry-pick renderer in force (no argmax winner), the realized-funding column
      present. Do NOT lock the anchor here — the TESTER locks #88 (M-TEST), per the MR
      precedent (the grid + N are locked at design time, § D-CARRY.2-LOCKED).

## M-TEST — verify on BOTH robustness axes vs the +1.74 / +1.10 buy-and-hold bar (tester)

- [ ] Verify the science gate: 87/87 anchors byte-identical (the funding path is
      additive/off for them); the 4 falsifiers RED-on-revert; two-run identity.
- [ ] Read the carry-C3 family verdict on BOTH 2023 (vs +1.74 BH) AND 2024 (vs +1.10
      BH, tail-negative) under the frozen § 0 decision rule. Apply the FP-C3.5
      anti-cherry-pick family-summary; any non-FRAGILE cell carries `→ C5 DEFLATION
      REQUIRED` (and IF non-FRAGILE, the C5 PBO/Deflated-Sharpe pass is genuinely owed).
- [ ] **Lock the +1 carry θ-surface anchor (87→88)** in `spec/anchors.toml`
      (scenario `v1-carry-theta-surface-2023-block-bootstrap-real-fy`); decide whether
      the 2024 surface warrants +1 (#89) per ADR § D6.6.4 (locking it is the durable
      choice). Extend `verify_anchors.sh`'s `mc-robustness-2026-06` handler to search
      `spec/carry-strategy/reports/`.
- [ ] Write the test report per the template; set the verdict (PASS / REGRESSION).

---

## Build-order summary

| # | Task | Size | Anchor-safe by | The crux? |
|---|---|---|---|---|
| 0 | Remove disposable diag flags | trivial | restores #86 baseline | — |
| 1 | Funding loader + REVISION pin | 0.5–1 d | new module, off-path | — |
| 2 | As-of forward-fill (real grid) | 0.5 d | pure fn, off-path | — |
| 3 | **Co-resample by shared `idx_seq`** | **1.5–2.5 d** | `Option` defaults absent | **YES (TRACTABLE)** |
| 4 | `ScoreSource::FundingCarry` + sign + ring | 1–1.5 d | serde-default enum | the sign |
| 5 | Funding-cashflow accrual | 0.5–1 d | gated on `Option` | the non-no-op |
| 6 | `--score-source` flag + grid + render col | (folded) | gated to carry reports | — |
| 7 | Day-1 gate + 4 falsifiers + 2-run | 0.5–1 d | tests only | — |
| 8 | Wall-clock + anchored carry-C3 (2023+2024) | run-time | writes only carry dir | — |
| — | **TOTAL (framing (a))** | **~4.5–7.5 d** | 87 anchors hold | — |

> **STOP-and-flag triggers for the dev** (per the M-T1 mandate): (a) the M-DEV-3
> anchor gate FAILS (a momentum/MR anchor moves) — the additive discipline is
> broken, do not work around it; (b) the M-DEV-8 wall-clock extrapolation is
> materially > ~30 min — re-scope N or the grid with the orchestrator before
> anchoring; (c) the sign re-verification against banked data (M-DEV-4) contradicts
> R-CARRY.2 — re-confirm the convention before locking the direction.
