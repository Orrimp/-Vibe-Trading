---
slug: simple-strategy-overfit-guard
version: 0.1.0
status: in-progress
owner: developer
updated: 2026-06-15
---

# Tasks — simple-strategy-overfit-guard v0.1.0

Ordered task list. Architect refines (M-T locks / ADR), developer checks boxes.
The whole feature is **thin reuse glue** over shipped infra — see
[`feature.md`](feature.md) § 1 for the file:line map. The only design decision of
substance is **Q-OG.2 (the seam)**; everything else is pre-registered.

## Architect (design lock) — DONE 2026-06-15

- [x] **T-OG.1** — Method ratified: **block-bootstrap-on-real-data** ([`feature.md`](feature.md) § Design D-OG.2).
      CPCV/DSR rejected — no selection step exists in the untuned survey + they need new infra.
- [x] **T-OG.2** — Seam resolved: **option (a) — dedicated `#[ignore]` harness** at
      `crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs` (D-OG.1). Keeps `bin/monte_carlo.rs`
      + `scenarios/montecarlo.rs::run_path` byte-untouched (zero § 0 anchor risk); also the cheaper-to-get-right
      path. **Seam verified end-to-end:** `run_scenario` → `RunReport.equity_series` (engine.rs:271);
      single-symbol `BlockBootstrapPathGen` is a directly-tested path (bootstrap.rs `fp_c1_2/3/4`,
      `auto_block_length_is_some`). **Correction D-OG.0: the reducer is `DistributionSummary::from_path_metrics(&[PathMetrics])`,
      NOT the brief's nonexistent `EnsembleSummary`** — harness assembles `PathMetrics` per path via the shipped `compute_*` helpers.
- [x] **T-OG.3** — N=500 + `BlockLengthPolicy::Auto` + ADR-0051 D1 seeds transfer AS-IS (D-OG.3). Single-symbol
      universe confirmed non-edge (4 bootstrap unit tests exercise 1-entry universes). **Caveat locked:**
      `ScenarioConfig.seed` stays the CONSTANT `0xC0FFEE…` `[u8;32]` for every path (ADR-0051 D1 orthogonality);
      per-path variation lives ONLY in the bootstrap `path_seed_j`, not the engine seed.
- [x] **T-OG.4** — UN-ANCHORED confirmed (D-OG.4): `#[ignore]`, no report file, **no `anchors.toml` row**.
      Escalation hook (trend-following product greenlight → ADR-0051 § D6) recorded for the deck. No new ADR
      (ADR-0051 + § 0 ruler govern); no architecture.md change.

## Developer (build — architect-locked, build per § Design D-OG.0–6)

> **READ [`feature.md`](feature.md) § Design first.** The single substantive correction
> to the brief is **D-OG.0**: the reducer is `DistributionSummary::from_path_metrics(&[PathMetrics])`
> (`stats/mod.rs:365`), NOT `EnsembleSummary`. You assemble `PathMetrics` per path yourself
> from `RunReport.equity_series` using the shipped `compute_*` helpers. The exact 5-step seam
> + the harness signature sketch are in D-OG.1.

- [x] **T-OG.5** — Create `crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs` (option a).
      **Copy `load_year_bars` verbatim** from
      [`realdata_simple_strategy_survey.rs:55`](../../crates/backtest/tests/realdata_simple_strategy_survey.rs).
      Add `run_one_path(sym, strat, path_bars) -> Option<RunReport>`: call `run_scenario` with
      `bars_override: Some(path_bars)`, `seed: SEED` (the CONSTANT `[u8;32]`), keep the **`RunReport`**
      (do NOT `.map(|r| r.kpis)`). Add `path_metrics_from_report(report) -> PathMetrics`: maps
      `report.equity_series` to `Vec<Decimal>` (`.amount()` per `Money<Usdt>`) and builds `PathMetrics`
      via `compute_sharpe_hourly`/`compute_sortino_hourly`/`compute_calmar`/`compute_max_drawdown_f64`/
      `compute_total_return` + `report.kpis.final_equity/initial_equity.amount()`.
      - **file**: `crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs:160` (`load_year_bars`), `:183` (`run_one_path`), `:205` (`path_metrics_from_report`)
      - **test**: `cargo clippy --tests -p backtest -- -D warnings`
      - **output**: `Finished dev profile [unoptimized + debuginfo]` — 0 errors, 0 warnings

- [x] **T-OG.6** — Add `run_ensemble(root, sym, (start_ms,end_ms), strat, ensemble_seed) -> Option<DistributionSummary>`:
      load real year bars ONCE, build ONE `BlockBootstrapPathGen::new(vec![(sym, bars)], BlockLengthPolicy::Auto)`,
      loop `j in 0..500` with `path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))`,
      `generate(&[(sym, start_price)], source_len, path_seed_j)`, run `run_one_path` on `path.bars_by_symbol[0].clone()`,
      collect `PathMetrics` **in index order**, then `DistributionSummary::from_path_metrics(&metrics)`. Drive
      the matrix: {sma, macd, rsi, bbands} x {AVAX·2024, DOT·2024} = 8 ensembles + AVAX·2023 SMA contrast (R-OG.5).
      Use a DISTINCT `ensemble_seed` per (strategy x cell). All equity `Decimal`/`Money<Usdt>` until the `compute_*` boundary (R-OG.6).
      - **file**: `crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs:237` (`run_ensemble`)
      - **test**: `cargo test -p backtest --test realdata_simple_strategy_overfit_guard --release -- --ignored --nocapture`
      - **output**: all 9 ensembles returned populated `DistributionSummary` (500 paths each); `test result: ok. 1 passed; 0 failed; finished in 79.39s`

- [x] **T-OG.7** — Score each `DistributionSummary` against the frozen § 0 bands AS-IS:
      `sharpe.p5 < 0 => FRAGILE`; `prob_loss > 0.35 => FRAGILE`; `max_dd_tail_p95 > 0.70 => FRAGILE`;
      ROBUST iff `sharpe.p5 >= 0.5 AND prob_loss <= 0.15 AND max_dd_tail_p95 <= 0.50`; else MARGINAL.
      Composite = worst band (§ 0 step 3). Print one line per ensemble: N, sharpe p5/p25/p50/p75/p95,
      prob_loss, prob_sharpe_gt_0, max_dd_tail_p50, max_dd_tail_p95, verdict. Do NOT re-derive/soften bands (R-OG.4).
      - **file**: `crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs:220` (`score_verdict`), `:338` (print loop in test body)
      - **test**: same run as T-OG.6
      - **output**: all 9 ensembles printed with full § 0 columns + composite verdict (see per-cell table in handoff)

- [x] **T-OG.8** — `#[tokio::test] #[ignore]`; SKIP-clean (print `SKIP`, return) when
      `data/binance/AVAXUSDT/2024/01.parquet` absent (mirror survey line 112 with a down-market symbol). No report
      file, no `anchors.toml` row (R-OG.8). Module doc-comment states: UN-ANCHORED rationale (§ 3) AND that the
      CLAUDE.md baseline-divergence e2e gate is **N/A — analysis tooling, no overlay/sizing-modifier** (D-OG.6);
      the determinism (AC-OG.3) + negative-control (AC-OG.4) checks are the miscalibration tripwire.
      - **file**: `crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs:330` (`#[ignore]`), `:336` (SKIP guard), module doc-comment lines 1-61 (UN-ANCHORED + N/A gate)
      - **test**: `cargo test -p backtest` (default suite, no --ignored)
      - **output**: `test result: ok. 82 passed; 0 failed; 5 ignored` — harness not run in default suite; `scripts/spec_lint.py` = 70 findings (unchanged from baseline)

> **Watch recipe (emit when kicking off the N=500 x 9 run, > 2 min):**
> `cargo test -p backtest --test realdata_simple_strategy_overfit_guard -- --ignored --nocapture > /tmp/og-run.log 2>&1 &`
> then `watch -n 10 'tail -20 /tmp/og-run.log'`.

## Tester (verify)

- [ ] **T-OG.9** — On a corpus-present machine: run the harness, confirm all 8 down-market ensembles + the
      up-market contrast print full § 0 summaries (AC-OG.1). Capture the `--nocapture` stdout for the finding.
- [ ] **T-OG.10** — Determinism: diff two consecutive `--nocapture` runs → byte-identical summaries (AC-OG.3 /
      R-OG.7). Corpus-absent: confirm clean SKIP, green default suite (AC-OG.2).
- [ ] **T-OG.11** — Negative-control check: RSI + BBands on the down-market cells score FRAGILE/MARGINAL, NOT
      ROBUST (AC-OG.4) — proves the test discriminates and isn't blessing no-edge churn.
- [ ] **T-OG.12** — `scripts/spec_lint.py` <= 70 zero-new (AC-OG.6 / R-OG.10); `cargo clippy -- -D warnings`
      clean; no `.unwrap()` outside tests.

## Analyst (close the loop)

- [ ] **T-OG.13** — After the tester's run, author the `findings`-status dev-note
      `spec/dev-notes/analysis-<date>-simple-strategy-overfit-guard.md` with the ACTUAL p5 Sharpe +
      prob-of-loss numbers, stating whether the down-market trend-following hedge is **path-robust (real)**
      or **fragile (one lucky path)**, capped at the § 6 small-N latitude (claim scoped to AVAX-2024 /
      DOT-2024 individually, not down-markets in general — Q-OG.5). Fold into the passive-baseline thesis.
      (AC-OG.5)

## Watch recipe (long-running ensemble run)

The N=500 x 9-ensemble bootstrap run is a >2 min job. Background + watch:

```
cargo test -p backtest --test realdata_simple_strategy_overfit_guard \
    -- --ignored --nocapture > /tmp/og-run.log 2>&1 &
watch -n 10 'tail -20 /tmp/og-run.log'
```

## Changelog

- 2026-06-15 (developer): implemented T-OG.5–8. Created
  `crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs`. Harness compiles + passes
  `cargo clippy --tests -p backtest -- -D warnings` clean (0 errors, 0 warnings). Default suite
  stays green (82 passed, 0 failed). Full N=500 x 9-ensemble run completed in 79.39s.
  All 9 ensembles scored FRAGILE under the frozen § 0 rule (all cells have sharpe.p5 < 0).
  Negative control confirmed: RSI/BBands FRAGILE (AC-OG.4). spec_lint.py = 70 (unchanged).
  HANDOFF -> tester.
- 2026-06-15 (architect): checked off T-OG.1-.4 with the locked decisions; rewrote T-OG.5-.8 to the
  verified seam. **Corrected the reducer reference: `DistributionSummary::from_path_metrics(&[PathMetrics])`,
  NOT `EnsembleSummary`** — added the explicit per-path-metrics step (`RunReport.equity_series` → `compute_*`
  helpers → `PathMetrics`) the draft glossed. Pinned the harness path
  (`crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs`), the constant-engine-seed caveat, the
  § 0 threshold mapping, and per-task acceptance criteria. Named the concrete watch recipe. HANDOFF -> developer.
- 2026-06-15 (analyst): v0.1.0 draft task list. 13 tasks across architect/developer/tester/analyst.
  The substantive design decision is T-OG.2 (the seam — ComposedStrategy ids vs `monte_carlo.rs`).
  Everything else is reuse glue + the pre-registered § 0 scoring.
