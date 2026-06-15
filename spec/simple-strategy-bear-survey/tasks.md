---
slug: simple-strategy-bear-survey
version: 0.1.0
status: in-progress
owner: analyst
updated: 2026-06-15
---

# Tasks — simple-strategy-bear-survey v0.1.0

Ordered task list. Analyst drafts; architect refines (M-T1 lock / candidate
predicate + cap / seam); developer checks boxes; tester verifies; analyst closes
the loop. The whole feature is **thin two-stage reuse glue** over shipped infra —
see [`feature.md`](feature.md) § The two-stage method + § References for the
file:line map (the survey supplies Stage 1, the overfit-guard supplies Stage 2).
The only design decisions of substance are **Q-BS.1 (the seam)** and **Q-BS.2
(the candidate predicate + cap)**; everything else is pre-registered (frozen § 0
rule, N=500, ADR-0051 seeds — all transfer AS-IS).

## Architect (design lock) — DONE 2026-06-15 (see feature.md § Design D-BS.1–.4)

- [x] **T-BS.1** — Q-BS.1 seam **LOCKED → option (a)**: ONE new combined `#[ignore]`
      harness `crates/backtest/tests/realdata_simple_strategy_bear_survey.rs`,
      Stage-1 + candidate-handoff + Stage-2 in one file; the two shipped harnesses
      stay byte-untouched. COPY (not extract) the survey's + overfit-guard's
      test-local primitives with corpus root → `data/binance-2122/` and years →
      2021/22 — extract rejected (would risk both shipped harnesses' behavior for
      no decision benefit). (b) and (c) rejected with rationale. Exact harness
      shape (`Stage1Cell` struct, `select_candidates`, `ensemble_seed_for`,
      Stage-2 loop) specified in § Design D-BS.1. _seam keeps both shipped
      `data/binance` reads intact._
- [x] **T-BS.2** — Q-BS.2 predicate + cap **LOCKED + PRE-REGISTERED/FROZEN**
      (§ Design D-BS.2, with a pre-registration notice): apparent winner iff
      **`buy_and_hold_pct < 0` AND `strat_ret_pct − buy_and_hold_pct ≥ 10.0` pp**.
      **Cap N≤16**, top-16 by margin DESC, deterministic tie-break
      `(margin DESC, symbol ASC, year ASC, strat_idx ASC)`. X=10 justified vs the
      2024 motivating margins (≈13/26 pp); cap=16 gives 16×500 ≈ ~140 s (under the
      ~20 ceiling, over the 2-min watch threshold). Plus one fixed out-of-predicate
      up-market contrast cell (SMA on highest-2021-B&H symbol) for AC-BS.6. The
      harness prints the predicate string + threshold + selected/dropped cells.
- [x] **T-BS.3** — Frozen knobs **confirmed AS-IS** (§ Design D-BS.3): N=500,
      `BlockLengthPolicy::Auto`, ADR-0051 D1 seeds (per-ensemble seed via
      `ensemble_seed_for(strat_idx, candidate_rank)` from the deterministic
      candidate order; CONSTANT engine `SEED` per path). § 0 bands NOT recalibrated.
      Q-BS.5: run `Auto` first, WARN + surface as a finding if L≤1 — do NOT pre-empt.
- [x] **T-BS.4** — **UN-ANCHORED** confirmed (§ Design D-BS.4): `#[ignore]`, no
      `spec/*/reports/`, no `anchors.toml` row. Reopen-path hook recorded
      (ROBUST-survivor → operator greenlight → ADR-0051 § D6, NOT a null).
      Baseline-divergence e2e gate **N/A on substance** (no overlay/sizing-modifier;
      AC-BS.5 determinism + AC-BS.6 negative-control are the tripwires). **NO new
      ADR** (reuses ADR-0051 D1 + § 0 rule + ADR-0056 unchanged; touches no anchor
      SHA); no architecture.md change. `arch` trace fill → ADR-0051 + § 0 rule
      note + ADR-0056 (orchestrator applies).

## Developer (build — after architect lock; build per § Design)

> **READ [`feature.md`](feature.md) § Design + the overfit-guard harness first.**
> The reducer is **`DistributionSummary::from_path_metrics(&[PathMetrics])`**
> (`stats/mod.rs`), NOT `EnsembleSummary` (overfit-guard D-OG.0). Assemble
> `PathMetrics` per path from `RunReport.equity_series` via the shipped
> `compute_*` helpers. The Stage-2 ensemble loop, seed derivation, and § 0 scoring
> are copy-adapt from `realdata_simple_strategy_overfit_guard.rs`; the Stage-1
> point survey is copy-adapt from `realdata_simple_strategy_survey.rs` — both with
> the corpus root changed to `data/binance-2122/` and the years to 2021 + 2022.

- [x] **T-BS.5 (Stage-1 harness + loader)** — Create the combined `#[ignore]` harness at
      the locked path **`crates/backtest/tests/realdata_simple_strategy_bear_survey.rs`**
      (D-BS.1). COPY `workspace_root`/`load_year_bars`/`buy_and_hold_pct` from the survey,
      with the corpus root → **`data/binance-2122/`** (`ReplayFeed::new(root.join("data/binance-2122"), true)`)
      and these UTC year boundaries: **2021** `[1_609_459_200_000, 1_640_995_200_000)`,
      **2022** `[1_640_995_200_000, 1_672_531_200_000)`. Build a `Stage1Cell { sym,
      year_label, year_bounds, strat_idx, strat_id, strat_label, bh_pct, strat_ret_pct,
      trade_count, n_bars }` per (10 symbols × {2021,2022} × 4 strats) via the survey's
      `run_strategy` (absolute-equity return, `(fin−init)/init·100`) + `buy_and_hold_pct`.
      Print the full 80-cell table. **SKIP-guard**: if
      `data/binance-2122/BTCUSDT/2022/01.parquet` is absent → print SKIP + return. Thin
      cells (`bars.len() < 100`) print `(only N bars)` + skip (no `Stage1Cell` pushed).
      _acceptance: AC-BS.1 + AC-BS.4 — full Stage-1 table prints on a corpus-present box;
      clean SKIP + green default suite when absent._
- [x] **T-BS.6 (candidate selection — PRE-REGISTERED predicate)** — Implement
      `select_candidates(&[Stage1Cell]) -> Vec<&Stage1Cell>` to the **FROZEN** D-BS.2 rule:
      keep cells where **`bh_pct < 0` AND `strat_ret_pct − bh_pct ≥ dec!(10.0)`**; sort the
      qualifiers by `(margin DESC, symbol ASC, year ASC [2021<2022], strat_idx ASC
      [SMA<MACD<RSI<BBands])`; truncate to **16**. **Print the predicate string + threshold
      value + every qualifier with its margin, flagging which were kept vs dropped by the
      cap.** Do NOT change the predicate/threshold/cap — they are pre-registered (if a
      change is unavoidable, STOP and escalate per the D-BS.2 pre-registration notice; do
      not silently tune). _acceptance: AC-BS.2 — selected set printed with the exact
      predicate + threshold; count ≤ 16; selection byte-identical across two runs._
- [x] **T-BS.7 (Stage-2 bootstrap ensemble)** — COPY `run_one_path` /
      `path_metrics_from_report` / `run_ensemble` from the overfit-guard. For each
      candidate (and the contrast cell), build ONE
      `BlockBootstrapPathGen::new(vec![(sym, real_bars)], BlockLengthPolicy::Auto)`, loop
      `j in 0..500` with `path_seed_j = ensemble_seed.wrapping_add((j as u64).wrapping_mul(0x9E37_79B9))`,
      `generate(&[(sym, start_price)], source_len, path_seed_j)`, run `run_one_path` on
      `path.bars_by_symbol[0].clone()` keeping the **`RunReport`** (not `.kpis`), build
      `PathMetrics` per path from `equity_series` via `compute_sharpe_hourly`/`…sortino`/
      `…calmar`/`…max_drawdown_f64`/`…total_return` (index-order, do NOT sort), then
      `DistributionSummary::from_path_metrics(&metrics)`. Derive the **DISTINCT** per-ensemble
      seed via `ensemble_seed_for(strat_idx, candidate_rank)` =
      `0x00C0_FFEE_0000_0000 + (strat_idx as u64)*0x100 + candidate_rank as u64` where
      `candidate_rank` is the cell's index in the deterministic-sorted candidate list
      (D-BS.3); give the up-market contrast cell its own reserved rank above the cap (e.g.
      `0x00C0_FFEE_0000_0000 + strat_idx*0x100 + 0xF0`) so it never collides. All equity
      `Decimal`/`Money<Usdt>` to the `compute_*` boundary (R-BS.7). WARN if
      `selected_block_length ≤ 1` (Q-BS.5 — log + surface as a finding, do NOT pin `Fixed`).
      _acceptance: AC-BS.3 — each candidate returns a populated 500-path
      `DistributionSummary`; seeds distinct per (strategy × cell)._
- [x] **T-BS.8 (score + print + negative control + up-market contrast)** — COPY
      `score_verdict` from the overfit-guard; score each ensemble against the frozen § 0
      bands AS-IS (`sharpe.p5 < 0 ⇒ FRAGILE`; `prob_loss > 0.35 ⇒ FRAGILE`;
      `max_dd_tail_p95 > 0.70 ⇒ FRAGILE`; ROBUST iff `p5 ≥ 0.5 AND prob_loss ≤ 0.15 AND
      dd_p95 ≤ 0.50`; else MARGINAL; composite = worst band). Print one § 0 row per
      candidate: cell label, strategy, N, sharpe p5/p25/p50/p75/p95, prob_loss, P(sharpe>0),
      dd_p50/dd_p95, verdict. **Up-market contrast (D-BS.2 / AC-BS.6):** after Stage 1,
      deterministically pick the symbol with the **highest 2021 full-year B&H** from the
      `Stage1Cell` table and score **SMA** on that symbol's 2021 cell as a fixed
      out-of-predicate ensemble, clearly labelled `(up-market contrast)` (does NOT count
      against the cap; mirrors the overfit-guard's AVAX·2023 SMA control). Do NOT
      re-derive/soften the bands (R-BS.5). The module doc-comment states the UN-ANCHORED
      rationale + the **N/A baseline-divergence gate** (D-OG.6 analogue) + the
      pre-registered predicate. _acceptance: AC-BS.3 + AC-BS.6 — § 0 columns + composite
      verdict per candidate; any selected RSI/BBands scores FRAGILE/MARGINAL not ROBUST;
      the up-market contrast cell is scored and labelled._
- [x] **T-BS.9 (determinism + watch recipe + clippy + lint)** — `#[tokio::test] #[ignore]`.
      Confirm two consecutive `--release --ignored --nocapture` runs are byte-identical
      (AC-BS.5). **Emit a copy-pasteable `watch -n N '<probe>'` block** when kicking off the
      Stage-2 release run (R-BS.11 — likely > 2 min on a market-wide-bear candidate count).
      `cargo clippy --tests -p backtest -- -D warnings` clean; no `.unwrap()` outside tests.
      `scripts/spec_lint.py` = 70, zero new. Confirm `git status` shows no change to any
      `REVISION.toml` or `anchors.toml` (AC-BS.8). _acceptance: AC-BS.5 + AC-BS.8 + AC-BS.9._

## Tester (verify)

- [x] **T-BS.10** — On a corpus-present box: run the harness; confirm the full Stage-1 table
      prints (AC-BS.1), the candidate set + predicate print (AC-BS.2), and each candidate's
      Stage-2 § 0 summary + verdict prints (AC-BS.3). Capture the `--nocapture` stdout for
      the finding. _acceptance: all three sections present; exit 0._
      _verified 2026-06-15 (tester): 80-cell Stage-1 table printed; 40 qualifiers → 16 candidates
      explicit with predicate; all 16 Stage-2 rows + contrast row printed. Exit 0. PASS._
- [x] **T-BS.11** — Determinism: diff two consecutive `--release --ignored --nocapture` runs
      → byte-identical (AC-BS.5), including an identical candidate set. Corpus-absent:
      confirm clean SKIP + green default suite (AC-BS.4). _acceptance: empty diff; default
      suite green with the harness ignored._
      _verified 2026-06-15 (tester): diff <(grep '^|' /tmp/bear-A.log) <(grep '^|' /tmp/bear-B.log)
      = EMPTY. Default suite: 8 passed, 0 failed, harness ignored. PASS._
- [x] **T-BS.12** — Negative-control + contrast check: any selected RSI/BBands candidate
      scores FRAGILE/MARGINAL (NOT ROBUST); the up-market-contrast cell scores as expected
      (AC-BS.6). A mean-reverter coming back ROBUST is a RED flag → escalate, do not pass.
      _acceptance: no mean-reverter ROBUST; harness discriminates._
      _verified 2026-06-15 (tester): 9 RSI/BBands candidates in top-16, all FRAGILE (highest
      p5 = −0.888). Up-market contrast SOLUSDT·2021 SMA: p5=+0.439, MARGINAL — clearly
      discriminates from all-negative-p5 bear candidates. No mean-reverter ROBUST. PASS._
- [x] **T-BS.13** — `scripts/spec_lint.py` = 70 zero-new (R-BS.13 / AC-BS.9); `cargo clippy
      --tests -p backtest -- -D warnings` clean; `git status` shows no `REVISION.toml` /
      `anchors.toml` / `spec/*/reports/` change (AC-BS.8); `scripts/verify_anchors.sh` green.
      _acceptance: lint 70, clippy clean, corpus + anchors untouched._
      _verified 2026-06-15 (tester): spec-lint 70 (zero new); clippy clean; git status shows
      only pre-existing M data/yahoo/REVISION.toml (not this feature); verify_anchors.sh
      ANCHORS PASS (119/119). PASS._

## Analyst (close the loop)

- [x] **T-BS.14** — After the tester's run, author the `findings`-status dev-note
      `spec/dev-notes/analysis-<date>-simple-strategy-bear-survey.md` with the ACTUAL
      per-candidate p5 Sharpe + prob-of-loss numbers, stating whether **ANY** simple strategy
      shows a **path-robust edge** on the 2021-22 bear sample. Apply the **§ Scope cap**
      (per-symbol-year claim; hourly/default-params/10-large-caps/2-bear-years; null firms
      ship-passive but does NOT prove no strategy can ever work; a ROBUST survivor REOPENs
      the question — flag the high-value tail loudly). Fold into the passive-baseline thesis
      (firms on a null — amend the runbook; flags the v0.2.0 reopen on a ROBUST survivor).
      Revise the survey / overfit-guard cross-references with a one-line pointer. (AC-BS.7)
      _acceptance: dev-note states the headline with numbers, scope-capped; passive-baseline
      runbook amended; spec_lint stays 70._

## Watch recipe (long-running Stage-2 ensemble run)

The Stage-2 `N=500 × N_candidates` bootstrap run is likely a > 2 min job
(market-wide-bear candidate count). Background + watch:

```
cargo test -p backtest --test realdata_simple_strategy_bear_survey \
    --release -- --ignored --nocapture > /tmp/bear-survey-run.log 2>&1 &
watch -n 10 'tail -30 /tmp/bear-survey-run.log'
```

(Harness name locked at T-BS.1: `realdata_simple_strategy_bear_survey`. With the
cap=16, Stage 2 is ≈ 140 s — over the 2-min threshold, so this `watch` block is
mandatory per R-BS.11.)

## Notes

- **Frozen, reused AS-IS:** the § 0 decision rule
  ([`robustness-decision-rule-2026-05-30.md`](../dev-notes/robustness-decision-rule-2026-05-30.md) § 0),
  N=500, `BlockLengthPolicy::Auto`, ADR-0051 D1 seeds. None are re-derived here.
- **Corpus (read-only):** `data/binance-2122/`, pin `4f390622` — MUST NOT be
  mutated; no `REVISION.toml` re-emit (R-BS.12 / AC-BS.8).
- **Precedents this clones:** the Stage-1 shape from
  [`realdata_simple_strategy_survey.rs`](../../crates/backtest/tests/realdata_simple_strategy_survey.rs);
  the Stage-2 shape (ensemble loop, seeds, reducer, § 0 scoring, negative control)
  from [`realdata_simple_strategy_overfit_guard.rs`](../../crates/backtest/tests/realdata_simple_strategy_overfit_guard.rs).

## Changelog

- 2026-06-15 (analyst): T-BS.14 ticked. Authored the `findings` dev-note
  `spec/dev-notes/analysis-2026-06-15-simple-strategy-bear-survey.md` with the
  confirmed numbers (Stage 1: 40 apparent winners, all 2022, top SOL·2022 RSI
  +97.0 pp; Stage 2: all 16 candidates FRAGILE incl. the +97 pp winner at p5
  −0.888; up-market contrast SOL·2021 SMA MARGINAL p5 +0.439). Headline: the
  2021-22 deep bear FIRMS ship-passive — apparent winners are path-luck, not a
  robust edge; the high-value ROBUST-survivor tail did NOT materialise so the
  reopen path is not triggered and the question stays closed. Scope-capped.
  Amended the passive-baseline runbook § Real-data validation with a dated
  2026-06-15 bear-survey note. spec-lint stays 70 (zero new). AC-BS.7 satisfied.
- 2026-06-15 (tester): T-BS.10–.13 verified and ticked. VERDICT → PASS.
  Determinism: empty diff (AC-BS.5). Discrimination: SOLUSDT·2021 SMA p5=+0.439 MARGINAL
  vs all 16 bear candidates FRAGILE p5<0; 9/16 candidates RSI/BBands all FRAGILE — no
  mean-reverter ROBUST (AC-BS.6). Frozen predicate confirmed AS WRITTEN (40→16).
  Clippy clean; spec-lint 70 zero-new; verify_anchors.sh 119/119; shipped harnesses
  byte-untouched. No doctest failures. HANDOFF → analyst (T-BS.14).
  Report: spec/simple-strategy-bear-survey/reports/test-2026-06-15-1200-simple-strategy-bear-survey.md
- 2026-06-15 (developer): T-BS.5–.9 ticked. Harness created and run (153s release).
  Stage-1: 80 cells, 40 qualifying, top-16 bootstrapped. All 16 FRAGILE.
  Contrast cell (SOLUSDT·2021 SMA) MARGINAL. Determinism verified (two identical
  table outputs). Clippy clean. spec-lint 70. HANDOFF → tester.
- 2026-06-15 (architect): M-T1 lock. Checked off T-BS.1–.4 with the decisions
  (seam → option (a) one new combined harness; predicate FROZEN at `B&H<0 AND
  margin≥10pp`, cap N≤16 by-margin with deterministic tie-break; N=500 + Auto +
  ADR-0051 D1 seeds AS-IS; UN-ANCHORED, no new ADR, baseline-divergence gate N/A).
  Refined developer tasks T-BS.5–.8 to bind the locked harness path, the exact
  2021/22 UTC year boundaries (`1_609_459_200_000`/`1_640_995_200_000`/`1_672_531_200_000`),
  the `Stage1Cell` struct + frozen `select_candidates` predicate + cap, the
  `ensemble_seed_for(strat_idx, candidate_rank)` derivation, and the fixed
  out-of-predicate up-market contrast cell (SMA on highest-2021-B&H symbol).
  Locked the watch-recipe harness name. Status → in-progress, owner → developer.
  HANDOFF → developer.
- 2026-06-15 (analyst): v0.1.0 draft task list. 14 tasks across
  architect/developer/tester/analyst. The two substantive design decisions are
  **T-BS.1 (the Stage-1→Stage-2 seam — one new combined harness vs parameterize the
  2 existing `data/binance`-hardcoded harnesses)** and **T-BS.2 (the candidate
  predicate + cap, pre-registered before the run)**. Everything else is two-stage
  reuse glue over the survey (Stage 1) + overfit-guard (Stage 2) shapes + the frozen
  § 0 scoring. Flagged the Stage-2 watch recipe. HANDOFF → architect.
