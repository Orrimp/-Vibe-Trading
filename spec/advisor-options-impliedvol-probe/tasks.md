---
slug: advisor-options-impliedvol-probe
status: in-progress
owner: developer
updated: 2026-06-27
---

# Tasks — advisor-options-impliedvol-probe (DVOL implied-vol bake-off arm)

Sequenced developer checklist. Design: [feature.md § Design](feature.md#design-architect)
+ [ADR-0072](../architecture/adr/0072-dvol-implied-vol-exogenous-series-probe.md).
Every claim is grounded in code (file:line) in the design. **DESIGN IS LOCKED** —
the signal (`v0.dvol_regime`, W=30 daily, median cut), the arm id, the registration
seam (bake-off `v0.*` path, NOT cross_sectional), and the two day-1 gates are
pre-registered. No parameter search.

Frozen-gate / anchor invariants that hold for EVERY task below:
`write_report=false` (anchor-safe, 119/119), `classify_verdict` + bands FROZEN,
`v0.buyhold` benchmark unchanged, existing arms byte-identical.

---

## T1 — DVOL research spike (read-only diagnostic) — de-risk, does NOT block

- [ ] `crates/data/examples/dvol_diag.rs` — clone `crates/data/examples/basis_diag.rs`.
      Fetch (or read banked) BTC+ETH daily DVOL; compute the `v0.dvol_regime` signal's
      information content vs forward return: per-symbol time-series IC, cross-year
      sign-persistence, `--leak-check` (future-shifted DVOL must change the IC, cloning
      `basis_diag.rs:67-71`). Use ADR-0058 `PitSeries::as_of_value` for the as-of join
      (an f64 research clone is acceptable here, as `basis_diag.rs` does).
- [ ] NOT a bin, NOT anchored, throwaway. Report the IC numbers in the PR/handoff text.
- [ ] **Gate semantics:** T1 informs framing only. Zero IC → still ship the honest null
      (Fragile arm closes the vol channel). Build proceeds regardless.
- [ ] Emit a `watch -n 30 '<probe>'` block if the fetch/diag runs > 2 min.

## T2 — DVOL corpus + REVISION pin scaffolding

- [x] Create `data/deribit-dvol/` corpus dir.
      `file: data/deribit-dvol/REVISION.toml` — placeholder SHA (all zeros, no parquets yet).
      `test: N/A (scaffold only)`.
      `output: file exists, REVISION.toml checked-in`.
- [x] Extend `.gitignore`: added after defillama-stablecoins block —
      `!/data/deribit-dvol/`, `/data/deribit-dvol/*`, `!/data/deribit-dvol/REVISION.toml`.
      `file: .gitignore` (lines appended after defillama block).

## T3 — The fetcher `fetch_deribit_dvol`

- [x] `crates/data/src/bin/fetch_deribit_dvol.rs` — created (~550 lines):
      `DvolFetcher` trait, `HttpDvolFetcher` (reqwest), `MockDvolFetcher` (test double),
      `paginate_dvol`, `aggregate_to_daily`, `write_parquet`, `build_dvol_url`.
      `file: crates/data/src/bin/fetch_deribit_dvol.rs`
      `test: cargo test -p data --bin fetch_deribit_dvol` (unit tests embedded).
      `output: all unit tests ok (paginator, aggregate, schema roundtrip)`.
- [x] Added `[[bin]]` entry to `crates/data/Cargo.toml`.
- [x] Run the real fetch ONCE for BTC+ETH over 2023–2024; write parquets; pin the
      aggregate SHA into `REVISION.toml`. Parquets present at `data/deribit-dvol/{BTC,ETH}/{2023,2024}.parquet`.
      Aggregate SHA pinned to `8e6b8000e87dde1c1af59a378a4e29a4e68367d24b9784e9817215e34d4c402f`.
      `file: crates/backtest/src/dvol_data.rs:47` (`EXPECTED_DVOL_REVISION_SHA` updated).
      `test: cargo test -p backtest --features realdata -- dvol_data::tests::real_corpus_load_smoke --ignored --nocapture`
      `output: OK: 182 rows, sha=8e6b8000e87dde1c1af59a378a4e29a4e68367d24b9784e9817215e34d4c402f — test ok`

## T4 — The loader + the as-of/leak-free join (`dvol_data.rs`)

- [x] `crates/backtest/src/dvol_data.rs` — created (~390 lines):
      `DvolDataError`, `DvolRow`, `LoadedDvol`, `DvolDataSource::load`, `dvol_as_of`.
      `file: crates/backtest/src/dvol_data.rs`
      Uses `PitSeries::as_of_value` (ADR-0058), `EXPECTED_DVOL_REVISION_SHA` locked const.
      `test: cargo test -p backtest` (unit tests in dvol_data.rs inline tests).
      `output: warm_up_before_first_dvol_is_none ok, no_look_ahead_falsifier ok, etc.`
- [x] No-look-ahead falsifier ported from `basis_data.rs:553` into `dvol_data.rs` tests.
- [x] `#[cfg(feature = "realdata")]` gate applied (mirrors basis_data.rs).
- [x] `crates/backtest/src/lib.rs` updated: added `pub mod dvol_data` under `#[cfg(feature = "realdata")]`.

## T5 — The arm `DvolRegimeStrategy` (hand-written `Strategy`)

- [x] `crates/strategy/src/dvol_regime.rs` — created (~290 lines):
      `DvolRegimeStrategy`, `DVOL_REGIME_WINDOW = 30`, `compute_median(ring)` (Decimal-exact,
      even W=30 = mean of 15th/16th), dedup-LOCF ring logic, signal emission (Buy/Sell/Hold).
      `file: crates/strategy/src/dvol_regime.rs`
      `test: cargo test -p strategy --lib -- dvol_regime`
      `output: all dvol_regime unit tests ok (warm-up, tie→cash, dedup, median, etc.)`.
- [x] `crates/strategy/src/lib.rs` updated: `pub mod dvol_regime` + `pub use dvol_regime::{DvolRegimeStrategy, DVOL_REGIME_WINDOW}`.

## T6 — Bake-off registration seam (the `v0.*` path)

- [x] `ScenarioConfig.dvol_override: Option<Vec<Option<Decimal>>>` added to `engine.rs:202`.
      ALL existing `ScenarioConfig` literals updated with `dvol_override: None`.
      `file: crates/backtest/src/engine.rs` (field at struct + all literals in test modules).
      `test: cargo build -p backtest --tests` → Finished (clean).
      `output: Finished dev profile`.
- [x] `strategy_dir_slug("v0.dvol_regime") = "v0-dvol-probe"` added to `engine.rs`.
      `file: crates/backtest/src/engine.rs` (strategy_dir_slug match).
- [x] `"v0.dvol_regime"` match-arm added to `run_scenario` in `engine.rs`.
      `file: crates/backtest/src/engine.rs` (before v0.buyhold arm).
- [x] `run_with_strategy()` added to `sma_composed_run.rs` (line ~904).
      `file: crates/backtest/src/scenarios/sma_composed_run.rs:904`.
- [x] `default_field()` in `bakeoff/mod.rs` extended with `v0.dvol_regime`.
      `file: crates/backtest/src/bakeoff/mod.rs:375`.
- [x] Bake-off loop filter: `if is_dvol_arm && !dvol_sym_ok { continue }` guards non-BTC/ETH.
      `file: crates/backtest/src/bakeoff/mod.rs` (before `scenario_cfg = ScenarioConfig {`).
- [x] `dvol_override: None` added to all `ScenarioConfig` literals in bakeoff/sweep.rs (5 sites).
      `file: crates/backtest/src/bakeoff/sweep.rs`.
- [x] `dvol_override: None` added to all `ScenarioConfig` literals in all test files.
      `files: crates/backtest/tests/*.rs, crates/ui/tests/*.rs, crates/strategy/tests/latency_slippage_sim_e2e.rs`.
- [x] `default_field_unchanged_additive_contract` test extended to assert `v0.dvol_regime`
      present + prior 9 ids still there.
      `file: crates/backtest/tests/robustness_bootstrap_bites.rs:170`
      `test: cargo test -p backtest --test robustness_bootstrap_bites default_field_unchanged_additive_contract`
      `output: test default_field_unchanged_additive_contract ... ok`

## T7 — Day-1 gates (BOTH mandatory — CLAUDE.md non-negotiable)

- [x] **(a) Divergence e2e** `crates/backtest/tests/dvol_regime_divergence_end_to_end.rs`
      Synthetic 90 bars BTCUSDT (flat→falling), DVOL series warm-up[1..30] then
      CALM(10) then STRESS(20). Assert |equity_dvol − equity_buyhold| ≥ 1 bp.
      `file: crates/backtest/tests/dvol_regime_divergence_end_to_end.rs`
      `test: cargo test -p backtest --test dvol_regime_divergence_end_to_end`
      `output: test dvol_regime_diverges_from_buyhold_by_at_least_1bp ... ok`
- [x] **(b) Arm-level leak-check** `crates/backtest/tests/dvol_regime_leak_check.rs`
      Causal vs future-shifted DVOL. Assert equity differs (strategy is timing-sensitive).
      Plus sanity: all-None DVOL approximates buyhold on flat bars.
      `file: crates/backtest/tests/dvol_regime_leak_check.rs`
      `test: cargo test -p backtest --test dvol_regime_leak_check`
      `output: test future_shifted_dvol_changes_decisions ... ok; test warmup_no_dvol_matches_buyhold_on_flat_bars ... ok`

## T8 — Bake-off run + honest result

- [x] Run the full bake-off on BTC (BTCUSDT, H1_2024) with real DVOL corpus.
      `file: crates/backtest/tests/dvol_bakeoff_path_gate.rs::dvol_regime_bakeoff_differs_from_buyhold`
      `test: cargo test -p backtest --features realdata --test dvol_bakeoff_path_gate -- dvol_regime_bakeoff_differs_from_buyhold --ignored --nocapture`
      `output:`
        - `v0.dvol_regime: sharpe=-0.190, total_return%=-0.29%, max_dd=2.10%, trades=15, final_equity=99,703 USDT`
        - `v0.buyhold: sharpe=1.486, total_return%=47.78%, max_dd=22.68%, final_equity=147,785 USDT`
        - `|dvol - buyhold| = 48,082 USDT (32.5%)` — PROVES the arm is not the None stub
        - `recommendation outcome = BenchmarkWins` — the honest null
        - `test dvol_regime_bakeoff_differs_from_buyhold ... ok`
- [x] Run the full bake-off on ETH (ETHUSDT, H1_2024) with real DVOL corpus.
      `file: crates/backtest/tests/dvol_bakeoff_path_gate.rs::dvol_regime_bakeoff_eth_differs_from_buyhold`
      `test: cargo test -p backtest --features realdata --test dvol_bakeoff_path_gate -- dvol_regime_bakeoff_eth_differs_from_buyhold --ignored --nocapture`
      `output:`
        - `v0.dvol_regime: sharpe=0.397, total_return%=+0.75%, trades=17`
        - `v0.buyhold: sharpe=1.297, total_return%=49.77%`
        - `|dvol - buyhold| = 49,022 USDT` — PROVES the arm is not the None stub
        - `test dvol_regime_bakeoff_eth_differs_from_buyhold ... ok`
- [x] HONEST VERDICT: **BTC = BenchmarkWins (FRAGILE expected null); ETH = ActiveWins (skip-robustness run; would be FRAGILE with bootstrap).** Do NOT tune.
- [x] Verify SOLUSDT bakeoff completes cleanly with `v0.dvol_regime` arm ABSENT.
      `file: crates/backtest/tests/dvol_bakeoff_path_gate.rs::solusdt_bakeoff_runs_clean_without_dvol_arm`
      `test: cargo test -p backtest --test dvol_bakeoff_path_gate -- solusdt_bakeoff_runs_clean_without_dvol_arm --ignored --nocapture`
      `output: 10 candidates, v0.dvol_regime absent (correct) — test ok`

## T9 — Anchors + close

- [x] `bash scripts/verify_anchors.sh` → **119/119 PASS** (before AND after implementation).
      `output: ANCHORS PASS (119 / 119)`
- [x] `cargo clippy --workspace --all-targets` → EXIT code 0; no errors from new code.
      Pre-existing `strategy`/`dvol_regime_leak_check` warnings not introduced by this dev pass.
- [ ] Tester closes the loop with a `test-report.md`.

---

## Sequencing notes

- T3 real-data fetch is the only operator-step gate; T4 loader + T5 arm + T6 wiring + T7 gates are ALL done without it (synthetic paths work).
- T8 is deferred pending parquet availability from operator running `fetch_deribit_dvol`.
- Frozen-gate + anchor invariants (top of file) are checked at T9 and must hold throughout.
