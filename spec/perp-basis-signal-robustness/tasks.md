---
slug: perp-basis-signal-robustness
status: arch-done
owner: architect → developer
updated: 2026-06-05
---

# Tasks — perp-basis-signal-robustness (M-DEV staged build order)

> **Mirrors the carry `M-DEV` staging** (`spec/carry-strategy/feature.md#implementation`).
> The basis arm is the **cheaper sibling**: the 8h→1h forward-fill is an identity (basis
> is native 1h), there is NO cashflow accrual (the basis is a selection signal — D-BR.1),
> and the bootstrap co-resample is REUSED, not rebuilt (the basis rides the existing
> `funding_by_symbol` channel — D-BR.3). The headline new work is the **fee-sweep axis**
> (D-BR.LOAD) + the **basis loader** (D-BR.3).
>
> **Hard gate after every additive seam: `bash scripts/verify_anchors.sh` → 99/99 PASS.**
> Every basis seam is additive/defaults-off; the 99 anchors are byte-identical by
> construction (D-BR.8). If a seam drops 99/99, STOP — the seam is not anchor-neutral.
>
> **The SIGN is load-bearing (R-BR.2 / D-BR.1):** `basis_reversal_score =
> −trailing_mean(basis)`. A flip turns the reversal arm into a basis-MOMENTUM payer. The
> M-DEV-3 sign-assertion falsifier is the guard — it MUST go RED on a sign flip.
>
> **Decimal money throughout (ADR-0003).** No `f64` in the basis parse, the score, the
> rank, or the P&L. Strict no-look-ahead: the basis at the open of bar `t` is
> `basis_close[t-1]` (D-BR.5).
>
> **Files only — do NOT `git commit`/`git push` (the orchestrator commits). Do NOT touch
> `crates/ui/`, `data/yahoo/REVISION.toml`, or any anchored `spec/*/reports/` file.**

---

## Build feature gate

All basis-data + sweep work is `#[cfg(feature = "realdata")]` (pulls polars), exactly as
carry. The canonical build/test invocation:

```
cargo build  -p backtest --features "candle realdata" --bin param_robustness_sweep
cargo test   -p backtest --features "candle realdata" --lib basis_data
cargo clippy -p backtest -p strategy -p data --features "backtest/realdata backtest/candle" --all-targets -- -D warnings
cargo fmt --check
bash scripts/verify_anchors.sh            # 99/99 after every additive seam
```

---

## Stage 1 — Basis-data foundation

### M-DEV-0 — anchor-baseline floor (the pre-flight)

- **Goal:** confirm the 99-anchor floor BEFORE any change, so a later 99/99 regression is
  attributable to the seam under test.
- **Gate:** `bash scripts/verify_anchors.sh` → **99/99 PASS**. Record the count.
- **No files changed.** (If the working tree is dirty from a prior feature, note it — the
  basis seams must not perturb it.)

### M-DEV-1 — the basis loader (`basis_data.rs`, a near-mirror of `funding_data.rs`)

- **File (new):** `crates/backtest/src/basis_data.rs`, `#[cfg(feature = "realdata")]`,
  registered in `crates/backtest/src/lib.rs` (mirror the `funding_data` mod declaration).
- **Mirror `crates/backtest/src/funding_data.rs` exactly**, substituting the basis schema:
  - `pub const EXPECTED_BASIS_REVISION_SHA: &str =
    "aa72409aa0f856960385a823bc61be1b8274e84f658439b65e5d1b1b1a48f1cd";` (verified on disk,
    `data/binance-basis/REVISION.toml`).
  - `BasisRow { symbol: Symbol, open_time_ms: i64, basis_close: Decimal }` (basis twin of
    `FundingRow`).
  - `BasisDataError` enum (mirror `FundingDataError`: `RevisionMissing`, `RevisionParse`,
    `RevisionMismatch`, `Parquet`, `DecimalParse`, `Io`).
  - `BasisDataSource::load(span, scenario)` — the SAME 6-step load+verify+parse
    (`funding_data.rs:149-294`): REVISION existence, per-file SHA verify, aggregate SHA vs
    `EXPECTED_BASIS_REVISION_SHA`, polars `scan_parquet`, **read columns `open_time` (i64)
    + `basis_close` (str)**, `Decimal::from_str(basis_close)` (handle the leading `-` for
    negative basis), span filter, sort `(open_time_ms ASC, symbol ASC)`.
  - `files_for_span` — copy verbatim from `funding_data.rs:301` (`<SYM>/<YEAR>/<MM>.parquet`).
- **Tests (mirror `funding_data.rs` tests):** schema/parse round-trip, **signed/negative-
  basis parse** (`Decimal::from_str("-0.0012")`), `revision_mismatch_is_rejected`, decimal-
  precision-preserved, an `#[ignore]` real-data test (`--include-ignored`) asserting the
  10-symbol × 2023 tree loads ~8,760 rows/symbol and the SHA matches the locked constant.
- **Gate:** `cargo test -p backtest --features "candle realdata" --lib basis_data` →
  all green; `bash scripts/verify_anchors.sh` → **99/99** (new module is off-path).

### M-DEV-2 — the as-of join (`basis_as_of` + `build_basis_at_return`) — the basis-loader anchor-neutrality gate

- **File:** `crates/backtest/src/basis_data.rs` (append, mirror `funding_data.rs:351-435`).
- **`basis_as_of(basis: &[(i64, Decimal)], bar_open_ts_ms: &[i64]) -> Vec<Option<Decimal>>`**
  — the basis at the open of bar `t` is the `basis_close` of the most-recent bar whose
  `close_time ≤ t.open_ts` = **`basis_close[t-1]`** on the aligned 1h grid (D-BR.5). The
  `funding_as_of` `partition_point` binary-search structure works VERBATIM (the "settlements"
  here are the dense per-bar `basis_close` values keyed by `close_time`); `None` for bars
  before the first available basis (warm-up).
- **`build_basis_at_return(...)`** — the basis twin of `build_funding_at_return`: the
  `T-1`-length `basis_at_return[s][k]` array the bootstrap co-resamples (the basis in force
  at real return-step `k` = the as-of basis at source bar `k`'s open).
- **Tests:** step-function / one-bar-lag correctness, **no-look-ahead falsifier** (shift the
  basis series +1 bar into the future → different result → RED on revert — the basis twin of
  `funding_data.rs::no_look_ahead_falsifier`, re-asserting the spike's BS.4 leak-check),
  warm-up→None, empty→all-None, `build_basis_at_return` aligns to `T-1`.
- **Gate:** `cargo test -p backtest --features "candle realdata" --lib basis_data` → green;
  `bash scripts/verify_anchors.sh` → **99/99**.

---

## Stage 2 — The signal (`ScoreSource::BasisReversal`)

### M-DEV-3 — the `BasisReversal` ScoreSource arm + the SIGN + the sign-assertion falsifier

- **File:** `crates/strategy/src/cross_sectional/config.rs` — add `BasisReversal` to the
  `ScoreSource` enum (`config.rs:48`, a 3rd variant after `VolAdjustedReturn` (default) +
  `FundingCarry`), `#[serde(rename_all = "snake_case")]` → `"basis_reversal"`. The
  serde-default stays `VolAdjustedReturn` (anchor-neutral).
- **File:** `crates/strategy/src/cross_sectional/momentum.rs`:
  - Add `basis_reversal_score(&mut self, symbol, open_ts) -> Option<Decimal>` — the basis
    twin of `carry_score` (`momentum.rs:305`): a trailing ring of the last L **bars** of
    the injected basis lookup (reuse the `funding_map` field as the basis carrier — D-BR.3;
    reuse the `funding_rings` mechanism but the ring counts BARS not 8h settlements, so push
    on every bar's basis lookup). Return **`−mean`** (the R-BR.2 sign — the load-bearing
    minus, in ONE place). `None` until the ring holds ≥ L bars (warm-up).
  - Add a 3rd arm to the `score_source` match in `on_bar` (`momentum.rs:372`, alongside
    `VolAdjustedReturn` and `FundingCarry` — both byte-untouched): `ScoreSource::BasisReversal
    => { /* push close for history consistency */ self.basis_reversal_score(&bar.symbol,
    bar.open_ts) }`. `Direction::Momentum` (identity) stays — the sign is in the score.
  - Extend `all_warmed` (`momentum.rs:176`) with a `BasisReversal` arm (ring ≥ L bars,
    mirroring the `FundingCarry` arm).
  - **Doc-comment the reuse (MANDATORY — the single most-confusable point):** at the basis
    injection site, *"the basis arm reuses the `funding_by_symbol`/`funding_map` channel as
    a generic sidecar carrier — the value is the BASIS, not funding, and is consumed ONLY by
    `basis_reversal_score`, NEVER by the `run_path` accrual (which stays gated `None` for the
    basis arm — D-BR.1)."*
  - Extend `compute_config_hash` (`momentum.rs:431`) — `score_source` is already in the
    canonical string; the new `BasisReversal` discriminant hashes distinctly (K3). Add a
    config-hash test (`m_dev3_config_hash_differs_by_basis_reversal`) mirroring
    `m_dev5_config_hash_differs_by_score_source`.
- **The SIGN-assertion falsifier (R-BR.7 #2, R-BR.2 — MANDATORY, day-1).** In
  `momentum.rs` tests, mirror `r_carry2_sign_assertion_longs_negative_funding_name`
  (`momentum.rs:787`): a synthetic universe with a known-HIGH-basis name (BTCUSDT) + a
  known-LOW-basis name (ETHUSDT), K=1, L=1. Assert the arm selects the **LOW-basis** name
  (ETHUSDT — the reversal-favored leg) and `basis_reversal_score(low) > basis_reversal_score(high)`.
  **RED-on-mutation:** if `basis_reversal_score` returns `+mean` instead of `−mean`, the
  HIGH-basis name scores higher and is selected → a basis-MOMENTUM payer → the test fails
  exactly there. Add the explicit RED-on-revert assertion message naming the flip.
- **The no-look-ahead falsifier (R-BR.7 #5, strategy level).** Mirror
  `r_carry6_no_look_ahead_strategy_level` (`momentum.rs:925`): basis injected at ts=0 →
  score available at ts=0; basis injected only at ts=1 → `None` at ts=0 (the future basis
  must not leak).
- **Gate:** `cargo test -p strategy --lib cross_sectional` → green (sign + look-ahead +
  hash tests pass); `cargo test -p strategy` → no regression in the existing
  momentum/MR/carry/TS tests; `bash scripts/verify_anchors.sh` → **99/99** (config-only
  additive change — the enum default is unchanged).

---

## Stage 3 — The fee-sweep axis + the sweep-bin wiring

### M-DEV-4 — the `--taker-fee-bps` / `--slippage-bps` axis (D-BR.LOAD) — the fee-axis anchor-neutrality gate

- **File:** `crates/backtest/src/bin/param_robustness_sweep.rs`.
  - **`Args`:** add `--taker-fee-bps <u32>` (**default `4`**) + `--slippage-bps <u32>`
    (**default `2`**) — the legacy literals. Mirror the `--horizon` flag's defaulting
    discipline (`Args` ~line 1243, defaults `1h` → anchor-neutral).
  - **`run_one_path_with_config`** (`param_robustness_sweep.rs:2307`): add `taker_fee_bps:
    u32` + `slippage_bps: u32` params; **replace the hardcoded literals at lines
    2409-2410** (`slippage_bps: 2, taker_fee_bps: 4`) with the passed values. The caller
    (`main`) passes `args.taker_fee_bps`/`args.slippage_bps` (defaults `4`/`2` for every
    non-basis run → `MatchConfig` byte-identical → 99 anchors unchanged).
  - **`render_surface_report`** (`param_robustness_sweep.rs:1603`): add a `| taker_fee_bps
    | {n} |` (+ `| slippage_bps | {n} |`) body row, **GATED `score_source ==
    SweepScoreSource::BasisReversal`** (the same gating idiom as the `is_horizon_run` row at
    ~line 1751 — renders ONLY for the basis arm so every existing body-SHA is byte-identical).
    The fee level becomes part of the basis anchor's hashed identity (so {0,2,5,10} bps are
    four DISTINCT anchors).
- **Anchor-neutrality test (MANDATORY):** add a unit/integration assertion (or rely on the
  verify gate) that a momentum run with the default `--taker-fee-bps 4 --slippage-bps 2`
  produces the byte-identical body-SHA to the pre-change run. The `verify_anchors.sh` 99/99
  IS this gate.
- **Gate:** `cargo test -p backtest --features "candle realdata" --test param_sweep_e2e` →
  green (FP-C3.x identity tests); `bash scripts/verify_anchors.sh` → **99/99** (the fee
  defaults reproduce the literals AND the body row is gated off for non-basis runs — both
  neutrality conditions).

### M-DEV-5 — `SweepScoreSource::BasisReversal` + `BASIS_TIER1_GRID` + `GridKind::BasisTier1` + the basis path-gen

- **File:** `crates/backtest/src/bin/param_robustness_sweep.rs`.
  - **`SweepScoreSource`** (`param_robustness_sweep.rs:1089`): add `BasisReversal`
    (`#[value(name = "basis-reversal")]`); `to_strategy_score_source` →
    `strategy::ScoreSource::BasisReversal`; a `needs_basis()` helper (mirror
    `needs_funding`).
  - **`BASIS_TIER1_GRID`** (a new `&[ThetaCell]` const, mirror `CARRY_TIER1_GRID`
    `:531`): the 6 LOCKED cells from § D-BR.2-LOCKED — `lookback_minutes` = L in **bars**
    (60/24/168/60/60/24), `k_long` (3/3/3/5/1/5), `rebalance_minutes_override`
    (480/480/480/1440/480/480), `drift` 0.10. `GridKind::BasisTier1` variant
    (`:348`) + the `grid_for_kind` arm (`:376`).
  - **`basis_grid_def_string`** (a new fn, mirror `carry_grid_def_string` `:1533`):
    `g={} lookback_bars={} rebalance_minutes={} k_long={} drift={}` — the hashed body field
    for the basis anchor (K3). Wire it into `render_surface_report`'s grid-header branch
    (mirror the carry branch).
  - **`load_basis_path_gen`** (a new fn, mirror `load_carry_path_gen` `:2508` + its
    `#[cfg(not(feature="realdata"))]` stub `:2605`): load + REVISION-verify the basis via
    `BasisDataSource`, build `basis_at_return` via `build_basis_at_return`, return a
    `BlockBootstrapPathGen` with the basis attached via the existing `.with_funding(...)`
    builder (REUSE — the basis rides the `funding_at_return` channel, D-BR.3).
  - **`cell_config`** (`:1388`): no change needed (it already sets `score_source =
    score_source.to_strategy_score_source()` and `rebalance_minutes =
    cell.effective_rebalance(...)`).
  - **`run_one_path_with_config`** (`:2307`): the `is_carry` bool that gates the
    `funding_override` extraction (`:2371`) must ALSO fire for the basis arm — rename/extend
    to `inject_sidecar: bool` (true for carry OR basis) so the basis value is extracted from
    `generated_path.funding_by_symbol` into the strategy's `funding_map`. **CRITICAL: the
    basis arm does NOT pass `funding_override` to `run_path` for ACCRUAL.** The strategy gets
    the basis (via `with_funding` for the score), but the `run_path` accrual gate
    (`montecarlo.rs:322`) must stay `None` for the basis arm. The cleanest wiring: build the
    `(Symbol, ts) → basis` map and pass it to the strategy via `with_funding`, but pass
    `funding_override: None` into `TcnScenarioInput` so the accrual block is never entered.
    **Confirm the basis P&L is pure price-of-selection (no cashflow) — D-BR.1.**
  - **`main`** (`:2720`): add `let is_basis = args.score_source ==
    SweepScoreSource::BasisReversal;` and route to `load_basis_path_gen`; the
    scenario-name + effective-out-dir branches (`:2967` / `:3044`) gain a `BasisReversal`
    arm → `v1-basis-reversal-fee{NN}bps-theta-surface-{year}-block-bootstrap-real-fy`
    (`{NN}` = zero-padded `args.taker_fee_bps`) routed to
    `spec/perp-basis-signal-robustness/reports/`.
- **Gate:** a throwaway N=3 basis smoke renders a non-empty θ-surface with the basis grid +
  the fee row + the correct slug; `bash scripts/verify_anchors.sh` → **99/99** (the basis
  path is additive — momentum #86 + MR #87 + carry #88/#89 + TS #90/#91 + horizon byte-
  identical).

---

## Stage 4 — Day-1 falsifiers (the integration gate, BEFORE the anchored run)

### M-DEV-6 — the integration-level day-1 falsifiers (`basis_divergence_e2e.rs`)

- **File (new):** `crates/backtest/tests/basis_divergence_e2e.rs` (mirror
  `crates/backtest/tests/carry_divergence_e2e.rs`). Each falsifier GREEN-as-written AND
  RED-on-revert:
  1. **`r_br_baseline_equity_divergence` (R-BR.7 #3 — the CLAUDE.md non-negotiable).** The
     basis arm's output equity diverges from the un-tilted baseline (a `VolAdjustedReturn`
     or equal-weight run on the SAME path) by **≥ 1 bp** when the basis decision variable is
     non-trivial. Construct the synthetic universe so the low-basis names ≠ the
     high-price-momentum names → guaranteed selection divergence. Mirror
     `r_carry_10a_carry_vs_price_diverge`. **The CLAUDE.md overlay-divergence gate.**
  2. **`r_br_baseline_divergence_red_on_revert`** — two identical-signal strategies (both
     `VolAdjustedReturn`, no basis) produce Δ=0, proving #1 would FAIL if the basis were not
     load-bearing (the RED-on-revert proof). Mirror
     `r_carry_10a_red_on_revert_vol_adjusted_return_no_divergence`.
  3. **`r_br_basis_non_no_op` (R-BR.7 #4).** Force the basis signal to a CONSTANT (no
     cross-sectional dispersion — same basis for every name) → the arm's selection/equity
     **collapses to the baseline** (Δ < ε), proving the basis is load-bearing, not
     decorative (the carry R-CARRY.10b analogue, adapted: there is NO cashflow, so the
     non-no-op is on the SELECTION, not a cashflow accrual).
  4. **`r_br_sign_assertion_integration` (R-BR.2).** Correct-sign vs flipped-sign basis →
     different equity, proving the sign convention is active at the integration level.
     Mirror `r_carry_2_sign_assertion_integration`.
  5. **`r_br_no_look_ahead_integration` (R-BR.5).** Future-shifted basis → different equity
     from causal basis, proving the as-of join is causal. Mirror
     `r_carry_6_no_look_ahead_integration`.
- **NOTE — NO `r_br_cashflow_non_no_op` test exists** (and must NOT): the basis is a
  selection signal with no cashflow accrual (D-BR.1). The non-no-op guard is the SELECTION
  collapse (#3), not a cashflow collapse. The developer MUST confirm the `run_path` accrual
  block (`montecarlo.rs:322`) is never entered for the basis arm (the `funding_override`
  passed to `TcnScenarioInput` is `None`).
- **Gate:** `cargo test -p backtest --features "candle realdata" --test basis_divergence_e2e`
  → all green; each RED-on-revert verified by the developer (revert the guard locally, see
  RED, restore); `bash scripts/verify_anchors.sh` → **99/99**.

### M-DEV-7 — the two-run byte-identity falsifier (R-BR.7 #6)

- **In `basis_divergence_e2e.rs`** (or the sweep e2e): `basis_two_run_byte_identity` — run
  the small-N basis sweep twice at the same `ensemble_seed`; assert identical
  `report_body_hash` (catches any unordered fold in the basis co-resample or the renderer).
  Mirror `carry_two_run_byte_identity`. The basis co-resample inherits the ChaCha20
  determinism of `idx_seq` (ZERO new RNG — D-BR.8); `select_above_threshold`/`top_k_long`
  is a deterministic `BTreeMap`-ordered fold → two-run identity by construction.
- **Gate:** the N=3 basis smoke body-SHA is identical across two runs (binary-level
  confirmation); `bash scripts/verify_anchors.sh` → **99/99**.

---

## Stage 5 — The anchored fee × grid × regime run (the deliverable)

### M-DEV-8 — the 8 anchored basis-reversal θ × fee surfaces (2023 + 2024 × {0,2,5,10} bps)

- **Pre-flight wall-clock gate (the C3 lesson — MANDATORY).** Re-confirm the per-path cost
  on the canonical box at the N=3 smoke. Expected ~0.094 s/path (the carry M-DEV-7
  measurement); `per surface ≈ 1,200 × 0.094 ≈ 2 min`; `all 8 ≈ 16 min` (§ D-BR.WALLCLOCK).
  **If the smoke shows a material per-path regression**, fall back to the documented economy
  ({0, 5} bps × {2023, 2024} = 4 surfaces — the R-BR.LOAD minimum) and flag it.
- **Run the 8 surfaces** (or the 4-surface R-BR.LOAD minimum) via the sweep bin. Per fee
  level `FF ∈ {0,2,5,10}` and per year `YYYY ∈ {2023,2024}`:
  ```
  cargo run -p backtest --features "candle realdata" --bin param_robustness_sweep -- \
    --score-source basis-reversal --grid basis-tier1 \
    --taker-fee-bps FF --slippage-bps 2 \
    --year YYYY --paths 200 \
    --data-root data/binance/ --basis-root data/binance-basis/
  ```
  (Add a `--basis-root` flag defaulting `data/binance-basis/` + a `--basis-revision-sha`
  defaulting `aa72409a…`, mirroring carry's `--funding-root`/`--funding-revision-sha`.)
- **Watch recipe (for the >2-min run):**
  ```
  watch -n 30 'ls -1 spec/perp-basis-signal-robustness/reports/ | tail -10; \
    echo "---"; pgrep -fl param_robustness_sweep | head'
  ```
- **Per-surface body content** (per R-BR.LOAD + the surface plan): per-cell
  FRAGILE/MARGINAL/ROBUST + family verdict + per-cell `→ C5` flags + the **trades** column
  (turnover legibility — the reversal-fee story) + the **net-of-fee edge vs the BH control**
  + the BH control row (re-asserts +1.74 (2023) / +1.10 (2024)). Pre-flight headers print
  `generator: block-bootstrap-real` AND `bootstrap_mode: shared-index` (void-if-fail).
- **Gate:** all surfaces render; the BH control reproduces the +1.74/+1.10 bar; the basis
  surfaces are scored against the frozen § 0 weakest-link composite at the realistic (5 bps)
  fee level; `bash scripts/verify_anchors.sh` → **99/99** (the basis runs are additive — the
  99 existing anchors are byte-identical). **The basis anchors are NOT locked by the
  developer** — the tester locks them at M-TEST PASS.
- **Files written:** `spec/perp-basis-signal-robustness/reports/robustness-*-v1-basis-reversal-fee{NN}bps-theta-surface-{year}-block-bootstrap-real-fy.md` (up to 8).

### M-DEV-9 — the additive `verify_anchors.sh` handler (D-BR.9)

- **File:** `scripts/verify_anchors.sh`. Add an `elif [[ "$version" ==
  "perp-basis-signal-robustness" ]]` branch **after** the `horizon-retest-robustness` branch
  (line 162), searching `spec/perp-basis-signal-robustness/reports/` for
  `robustness-*-${scenario}.md` (mirror the horizon branch, lines 162-169). **Touch NO
  existing branch** → the 99 anchors resolve through their existing branches byte-identically.
- **Gate:** `bash scripts/verify_anchors.sh` → **99/99** (the new branch only fires for the
  not-yet-locked `perp-basis-signal-robustness` namespace).

---

## M-TEST — the tester's gate (handoff to tester after M-DEV-8/9)

The tester (not the developer) owns these. Listed here so the developer leaves the build in
a tester-ready state:

1. **The FEE-SWEEP result (R-BR.LOAD) — the headline.** The fee-sensitivity read is
   produced; the best cell's net-of-fee edge vs the BH control is reported at each fee
   level; the FRAGILE-on-fees verdict (if it dies at realistic fees) is explicit. The
   decision-grade output of the feature.
2. **The day-1 falsifiers RED-on-revert (R-BR.7):** sign-assertion / baseline-divergence
   e2e / basis-non-no-op / no-look-ahead each GREEN-as-written AND RED-on-revert.
3. **The 99 existing anchors byte-identical** (`verify_anchors.sh` 99/99) + the new basis
   anchors LOCKED (the tester adds the rows to `spec/anchors.toml` under the
   `perp-basis-signal-robustness` namespace — up to 8, minimum the {0,5}×{2023,2024} = 4).
4. **Two-run byte-identity** of the basis surface body-SHA.
5. **Pre-flight void-if-fail** — the basis report headers print `generator:
   block-bootstrap-real` AND `bootstrap_mode: shared-index`.
6. **The frozen § 0 composite verdict** read per
   [`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md)
   § 0 (weakest-link), at the realistic (5 bps) fee level, against the BH control.

---

## Definition of done (the feature ships when)

- M-DEV-0..9 complete; `bash scripts/verify_anchors.sh` → **99/99** at every stage.
- The 5 day-1 falsifiers (sign / baseline-divergence / basis-non-no-op / no-look-ahead /
  two-run identity) GREEN + RED-on-revert.
- The 8 (or ≥4) anchored fee × grid × regime surfaces produced + the fee-sensitivity read
  legible.
- `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- M-TEST gate cleared; the tester locks the basis anchors + reads the frozen § 0 verdict at
  5 bps vs the BH control.

---

## Changelog

- 2026-06-05 (architect, M-T1): authored tasks.md (M-DEV-0..9 + M-TEST) from the Design
  (D-BR.0..10). Staged build order mirrors carry: basis loader (M-DEV-1, near-mirror of
  `funding_data.rs`, pin `aa72409a…`) + as-of join (M-DEV-2, `basis_close[t-1]`, the 8h→1h
  forward-fill is an IDENTITY at 1h) → the `ScoreSource::BasisReversal` arm + the SIGN +
  the sign-assertion falsifier (M-DEV-3) → the `--taker-fee-bps`/`--slippage-bps` fee axis
  (M-DEV-4, defaults `4`/`2` → 99 anchors byte-identical; the fee site is the hardcoded
  literals at `param_robustness_sweep.rs:2409-2410`; the fee body-row gated to the basis
  arm) → the sweep wiring (M-DEV-5: `SweepScoreSource::BasisReversal` + `BASIS_TIER1_GRID` +
  `load_basis_path_gen` REUSING the `funding_by_symbol` co-resample channel; the basis
  threads to the score via `with_funding` but the `run_path` accrual stays gated `None` —
  NO cashflow, the basis is a selection signal) → the day-1 falsifiers (M-DEV-6/7, mirror
  `carry_divergence_e2e.rs`; NO cashflow-non-no-op test — the non-no-op guard is the
  SELECTION collapse) → the anchored 8-surface fee × grid × regime run (M-DEV-8, ~16 min,
  TRACTABLE) → the additive `verify_anchors.sh` handler (M-DEV-9). Each task carries a
  file:line target + the `verify_anchors.sh` 99/99 gate. M-DEV-2 + M-DEV-4 are the
  anchor-neutrality gates (the basis loader + the fee axis). The tester locks the basis
  anchors at M-TEST PASS (up to 8; {0,5}×{2023,2024}=4 the minimum). No code written.
