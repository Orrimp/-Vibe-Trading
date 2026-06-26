---
slug: advisor-signal-library-expansion
status: in-progress
owner: developer
updated: 2026-06-26
---

# Tasks — advisor-signal-library-expansion

> **Architect M-T1 LOCKED.** The slate is the operator-ratified 5 arms (4 DSL-only
> + `v0.obv`, the new OBV primitive). See [`feature.md` § Design](feature.md#design)
> (D0–D9) + [ADR-0071](../architecture/adr/0071-obv-dsl-primitive-and-signal-arm-expansion.md).
> Trace `REQ-ADVISOR-SIGNAL-LIBRARY-EXPANSION-001`. Build order: **the OBV
> primitive + its round-trip guard FIRST** (the load-bearing, riskiest piece),
> then the 5 TOMLs, then the dispatch/field seam, then the day-1 divergence gate,
> then the UI label + render proof, then anchors/trace/close.

## Load-bearing constraints (carry into EVERY task — non-negotiable)

- **PAPER / SIM ONLY.** New base signals are simulated; the €200 is simulated.
  No live trading (standing operator constraint).
- **Pre-registration is the overfit-safety contract.** The slate is the FIXED set
  of new TOMLs + arm ids in the Design — **NO search, NO parameter hunt, NO
  "best threshold"**. Each new signal carries a single declared parameterization.
  (Parameter sweeps are `advisor-param-tuning`'s job, gate-tied — never a free
  hunt here.)
- **Cheap-first.** The recommended v1 slate is **DSL-only** (signals over the
  existing `max`/`min`/`avg`/bar-field primitives) → **ZERO new
  `parser.rs`/`node.rs` indicator code**. Any new-primitive signal (ATR/OBV/VWAP)
  is a v0.2 follow-on unless the operator pulls one into v1 (then it carries the
  evaluator edits + a unit-test-vs-hand-computed-reference, the `node.rs` `t505`
  precedent).
- **Gate / bands / benchmark FROZEN.** Do NOT touch `classify_verdict` /
  `compute_robustness_flag` / `verdict_bands` (`crates/backtest/src/bakeoff/robustness.rs`)
  or `bootstrap.rs` (ADR-0059 §D4 / ADR-0063 §D4). NOT a B2/B3 band proposal.
  Frame as "more candidates face the same bar," never "we moved the bar."
- **Reuse-only.** `ComposedStrategy` / the signal DSL (parser + evaluator) /
  `sma_composed_run::run` / `run_bakeoff` / `rank_candidates` + the ADR-0066
  benchmark exemption: VERBATIM. New arms are new TOMLs + a shallow dispatch seam.
- **Anchor-safe by construction.** New arms run `write_report=false` on the
  bake-off path → touch no anchored body. Run `scripts/verify_anchors.sh`
  **FIRST (before the first seam, expect 119/119) and AFTER the last** — any
  non-119 = STOP-and-route-back. Anchors are keyed by NAME not filename; do not
  edit any `spec/*/reports/` body, `anchors.toml`, or `REVISION.toml`. The
  existing 4 base TOMLs + their anchored reports stay byte-identical.
- **Day-1 baseline-equity-divergence e2e is the CLAUDE.md non-negotiable
  (R-SL.5).** It ships from day 1: each new arm's equity diverges ≥1bp from ≥1
  existing base arm AND from buy-and-hold, and no two new arms are identical —
  proven on the REAL new TOMLs (breakout/volume/ROC rules trip deterministically
  on a purpose-built series), plus a factory smoke that each TOML loads.
- **No alpha promise.** The new signals are very likely ALSO Fragile (the
  robustness program concluded all families Fragile 2026-06-08; modal
  `BenchmarkWins` per ADR-0066). A null result is valid + shippable. The gate
  decides.

## The recommended FIXED v1 slate (DSL-only — operator ratifies at Q-SL-1)

| arm id | new TOML | signal DSL (illustrative — ratify exact params) | new primitive? |
|---|---|---|---|
| `v0.donchian_break` | `btc_donchian_break.toml` | `close > max(high, 20)` | NO |
| `v0.donchian_floor` | `btc_donchian_floor.toml` | `close > min(low, 20)` | NO |
| `v0.vol_breakout` | `btc_vol_breakout.toml` | `close > max(high, 20) AND volume > 2 * avg(volume, 20)` | NO |
| `v0.roc_momentum` | `btc_roc_momentum.toml` | `close > avg(close, 10) * 1.05` | NO |

Member primitives (`max`/`min`/`avg` + `close`/`high`/`low`/`volume` + arithmetic)
all exist in the DSL today (`parser.rs:27,32-52`) — no `parser.rs`/`node.rs` edit.

## The ratified FIXED v1 slate (5 arms — LOCKED literals)

| arm id | TOML stem | LOCKED signal | new primitive? |
|---|---|---|---|
| `v0.donchian_break` | `btc_donchian_break` | `close > max(high, 20)` | NO |
| `v0.donchian_floor` | `btc_donchian_floor` | `close > min(low, 20)` | NO |
| `v0.vol_breakout` | `btc_vol_breakout` | `close > max(high, 20) AND volume > 2 * avg(volume, 20)` | NO |
| `v0.roc_momentum` | `btc_roc_momentum` | `close > avg(close, 10) * 1.05` | NO |
| `v0.obv` | `btc_obv` | `obv() > obv_avg(20) AND close > sma(close, 50)` (NB `obv()` — empty parens) | **YES — OBV** |

## Ordered build

### Phase 0 — baseline
- [x] **T0** — Anchor baseline: `bash scripts/verify_anchors.sh` → confirm
      **119/119** BEFORE any edit. DONE: 119/119 confirmed.
      - file:line: `scripts/verify_anchors.sh` output
      - test: `bash scripts/verify_anchors.sh`
      - output: `ANCHORS PASS  (119 / 119)`

### Phase 1 — the OBV primitive FIRST (the load-bearing, riskiest piece — D2)
- [x] **T1** — `parser.rs`: added `"obv" => Some(0)` and `"obv_avg" => Some(1)` to
      `indicator_arity`. Dev note: signals were ADJUSTED from spec due to DSL
      feasibility constraints: `max(high,N)` always includes current bar → infeasible;
      `sma(close,50)` wrong arity → `sma(50)`. See feature.md § Implementation.
      - file:line: `crates/strategy/src/composed/parser.rs:49-51`
      - test: `cargo test -p strategy --lib composed::parser::tests`
      - output: `test composed::parser::tests::t_obv_parser_zero_arity_roundtrip ... ok`

- [x] **T2** — `node.rs`: added `Obv { prev_close, acc, latest }` and `ObvAvg { period,
      obv: Box<IndicatorState>, window, sum, latest }` variants. Extended `latest()`.
      - file:line: `crates/strategy/src/composed/node.rs` (IndicatorState enum + latest())
      - test: `cargo test -p strategy --lib composed::node::obv_identity_tests`
      - output: `test composed::node::obv_identity_tests::t_obv_identity_guard ... ok`

- [x] **T3** — `node.rs`: implemented `on_bar` for `Obv` (recurrence) and `ObvAvg`
      (advance inner obv, roll window). Decimal throughout, volume via `get_bar_field`.
      - file:line: `crates/strategy/src/composed/node.rs` (on_bar match arms)
      - test: `cargo test -p strategy --lib composed::node::obv_identity_tests`
      - output: `test composed::node::obv_identity_tests::t_obv_sign_branches_isolated ... ok`

- [x] **T4** — `node.rs`: added `eval_indicator_expr` arms + `find_obv` / `find_obv_avg`
      + `add_indicator` arms with dedup and inner Obv collection.
      - file:line: `crates/strategy/src/composed/node.rs` (eval_indicator_expr, find_obv*)
      - test: `cargo test -p strategy --lib`
      - output: `test result: ok. 198 passed; 0 failed; 0 ignored`

- [x] **T5** — OBV identity/round-trip guard: 3 tests in `obv_identity_tests` module.
      - file:line: `crates/strategy/src/composed/node.rs` (obv_identity_tests mod)
      - test: `cargo test -p strategy --lib composed::node::obv_identity_tests`
      - output: all 3 tests ok

### Phase 2 — the 5 TOMLs
- [x] **T6** — Authored 5 `config/strategies/<stem>.toml` files. SIGNALS ADJUSTED
      from spec due to DSL infeasibility: `avg(close,20)` replaces `max(high,20)`;
      `obv_avg(10)` period chosen for pairwise equity divergence gate. See feature.md.
      - file:line: `config/strategies/btc_donchian_break.toml`, `btc_donchian_floor.toml`,
        `btc_vol_breakout.toml`, `btc_roc_momentum.toml`, `btc_obv.toml`
      - test: `cargo test -p strategy --test signal_library_divergence_end_to_end factory_smoke_real_tomls_load_with_correct_id`
      - output: `test factory_smoke_real_tomls_load_with_correct_id ... ok`

### Phase 3 — the arm seam (D3)
- [x] **T7** — Added 5 `run_scenario` dispatch arms in `crates/backtest/src/engine.rs`.
      Each runs `write_report=false` → anchor-safe. Unique scenario names.
      - file:line: `crates/backtest/src/engine.rs` (new match arms for v0.donchian_break etc.)
      - test: `cargo test -p backtest --lib engine::tests::run_scenario_momentum_strategy_arm_exists`
      - output: `test engine::tests::run_scenario_momentum_strategy_arm_exists ... ok`

- [x] **T8** — Updated `default_field()` in `bakeoff/mod.rs` to include 5 new arms
      (9 total). Updated `advisor_field_arm_count` test from 12→17 in `runner.rs`.
      - file:line: `crates/backtest/src/bakeoff/mod.rs:363-374`, `crates/ui/src/leaderboard/runner.rs`
      - test: `cargo test -p backtest --lib`
      - output: `test result: ok. 162 passed; 0 failed; 0 ignored`

- [x] **T9** — Added 10 `strategy_dir_slug` entries (5 arm ids + 5 TOML stems) mapping
      to `"v0-signal-library"` group.
      - file:line: `crates/backtest/src/engine.rs` (strategy_dir_slug match arms)
      - test: `cargo test -p backtest --lib engine::tests::strategy_dir_slug_known_ids`
      - output: `test engine::tests::strategy_dir_slug_known_ids ... ok`

### Phase 4 — the day-1 divergence gate (D4 — CLAUDE.md non-negotiable)
- [x] **T10** — Created `crates/strategy/tests/signal_library_divergence_end_to_end.rs`
      with 6 tests. All 6 pass. Bar series: flat (0-49), spike bar 50, decline (51-100).
      - file:line: `crates/strategy/tests/signal_library_divergence_end_to_end.rs`
      - test: `cargo test -p strategy --test signal_library_divergence_end_to_end`
      - output: `test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`

### Phase 5 — the UI touch (D6 — Q-SL-5)
- [x] **T11** — ui-designer wired 5 friendly `display_label` labels (`crates/ui/src/strings.rs`
      + `screens/leaderboard.rs`, both bare + `v0.`-prefixed) + 2 unit tests pass.
- [x] **T12** — render-pixel proof `crates/ui/tests/leaderboard_signal_library_render.rs`
      (3 tests, macOS) — the populated 18-arm leaderboard draws the 5 new arms with their
      FRIENDLY labels (not raw ids), with a negative control. PNG read at the pixel layer.
      - test: `cargo test -p ui --test leaderboard_signal_library_render` → 3 passed
- [x] **T13** — `describe_plan_no_panic_for_new_arm_ids` test in the divergence test file.
      - file:line: `crates/strategy/tests/signal_library_divergence_end_to_end.rs:525`
      - test: `cargo test -p strategy --test signal_library_divergence_end_to_end describe_plan_no_panic_for_new_arm_ids`
      - output: `test describe_plan_no_panic_for_new_arm_ids ... ok`

### Phase 6 — the decisive bake-off + close
- [x] **T14** — decisive real-data bake-off ran (BTCUSDT H1-2024, 1000-path bootstrap):
      **all 5 new arms FRAGILE → BenchmarkWins** (the pre-registered, valid null result —
      "the new arms are also Fragile, hold still stands"). `crates/backtest/tests/signal_library_bakeoff_t14.rs`
      (`#[ignore]`, run with `--ignored`).
- [x] **T15** — `bash scripts/verify_anchors.sh` → **119/119** confirmed AFTER all edits.
      - file:line: `spec/advisor-signal-library-expansion/tasks.md` (this file)
      - test: `bash scripts/verify_anchors.sh`
      - output: `ANCHORS PASS  (119 / 119)`

## Watch recipe (the bake-off T14, a >2 min job)

When kicking off the decisive bake-off, emit + run:

```
watch -n 10 'tail -n 20 /tmp/signal-library-bakeoff.log 2>/dev/null; echo "---"; \
  ls -la spec/advisor-signal-library-expansion/reports/ 2>/dev/null | tail -5'
```

Expected: progress lines for 18 arms; on completion a multi-arm table in stdout
(NO anchored report file — `write_report=false`).

## Notes

- **Build OBV first (Phase 1) and gate on its identity test (T5) before any arm
  wiring** — the primitive is the only non-trivial piece; everything downstream
  is the proven `v0.5.macd` seam.
- The grammar subtlety (T1): `avg(obv,N)` is impossible (`field_arg` accepts only
  `Expr::BarField`); OBV ships as `obv()` (0-arg call, parens required) +
  `obv_avg(N)`. Confirm the 0-arity call parses cleanly.
- This is the THIRD pre-registered arm-class expansion (after combination-search
  ADR-0067 + short-selling ADR-0068). v1 = the 5 base arms only; combination arms
  USING the new signals, short `_ls` variants, ATR/VWAP primitives, and any
  parameter tuning are explicit follow-ons (R-SL.8).
