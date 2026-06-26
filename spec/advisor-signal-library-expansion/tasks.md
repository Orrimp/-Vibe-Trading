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
- [ ] **T0** — Anchor baseline: `bash scripts/verify_anchors.sh` → confirm
      **119/119** BEFORE any edit (anchors keyed by NAME not filename; non-119 =
      STOP-and-route-back). [DONE at scoping 2026-06-26 — re-run at the start of dev.]

### Phase 1 — the OBV primitive FIRST (the load-bearing, riskiest piece — D2)
- [ ] **T1** — `parser.rs`: add `"obv" => Some(0)` and `"obv_avg" => Some(1)` to
      `indicator_arity` (`parser.rs:32`). **0-arity is NOVEL** (every existing
      indicator is ≥1-arity) — the LOCKED spelling is `obv()` (empty parens; a
      bare `obv` would error `UnknownParam` because call-detection peeks for `(`,
      `parser.rs:348`). The empty-arg parse reads correct (`expect(LParen)` → loop
      sees `RParen` → 0 args → arity `0==0` → `expect(RParen)`, `parser.rs:374-396`)
      but is UNEXERCISED — **add a dedicated parser unit test for `obv()` and
      `obv_avg(20)`** (D2.2).
- [ ] **T2** — `node.rs`: add the two `IndicatorState` variants — `Obv { prev_close,
      acc, latest }` and `ObvAvg { period, obv: Box<IndicatorState>, window, sum,
      latest }` (`node.rs:26`). Extend `latest()` (`node.rs:119`) with both arms.
- [ ] **T3** — `node.rs`: implement `on_bar` (`node.rs:150`) for both:
      `Obv` = the recurrence `OBV_t = OBV_{t-1} + sign(close_t − close_{t-1})·volume_t`,
      `OBV_0 = Some(0)` (seed `prev_close` on bar 0 like `Rsi`, `node.rs:251`);
      `ObvAvg` = advance the inner `obv`, push `obv.latest()`, roll the window/sum
      and emit the mean once `window.len() == period` (clone `RollingAvg`,
      `node.rs:369`). Use `rust_decimal::Decimal` throughout (NO f64); `volume`
      via `get_bar_field(bar, "volume")` (`node.rs:1383`).
- [ ] **T4** — `node.rs`: `eval_indicator_expr` (`node.rs:519`) add `"obv" =>
      find_obv(...)` + `"obv_avg" => find_obv_avg(..., period)`; add the
      `find_obv` / `find_obv_avg` lookups (`node.rs:631`, the `find_rolling`
      pattern); `add_indicator` (`node.rs:909`) push `Obv` (dedup: at most one)
      + `ObvAvg{period}` (dedup by period) **AND ensure the inner `Obv` is also
      collected** so a lone `obv_avg(20)` works without a bare `obv()` term.
- [ ] **T5** — **The OBV identity/round-trip guard (D2.1 — architect-required for
      a new primitive).** In `node.rs` `#[cfg(test)]` next to `t505`
      (`node.rs:1599`): (a) a `btc_obv` TOML string round-trips via
      `ComposedStrategyConfig::from_str(toml, "btc_obv")`, id == stem,
      `build_indicators` yields `Obv` + `ObvAvg{20}` + `Sma{50}`; (b) a hand-built
      ~12-bar series with KNOWN up/down/**flat** closes + volumes, a hand-computed
      reference OBV vector, assert `Obv.latest()` after each `on_bar` == reference
      EXACTLY (Decimal, no tolerance), covering all 3 `sign` branches; (c)
      `ObvAvg{20}.latest()` once warm == the mean of the last 20 reference OBV
      values; (d) warm-up: `Obv.latest() == Some(0)` at bar 0, `ObvAvg{20}.latest()
      == None` until 20 pushes. **This must pass before any arm wiring.**

### Phase 2 — the 5 TOMLs
- [ ] **T6** — Author the 5 `config/strategies/<stem>.toml` (D1 shape:
      `kind="composed"`, `id==stem`, `symbol="BTCUSDT"`, `stage="research"`,
      `size="fixed_fraction(0.1)"`, the LOCKED signal). The `btc_obv` signal uses
      `obv()` (with parens — see T1). Each loads via `from_file` with `id==stem`.

### Phase 3 — the arm seam (D3)
- [ ] **T7** — Add the 5 `run_scenario` dispatch arms in
      `crates/backtest/src/engine.rs` (pattern-copy the `"v0.5.macd"` arm
      `engine.rs:1234-1309`): the match id, `strategy_id: "<stem>".to_string()`,
      `composed_toml_override: None`, and a **UNIQUE non-anchored `scenario_name`**
      per arm (e.g. `"btc-2023-1m-donchian-break"` … `"btc-2023-1m-obv"`) so the
      (unreachable, `write_report=false`) write branch can never collide with an
      anchored body.
- [ ] **T8** — Add the 5 ids to `BakeoffConfig::default_field()`
      (`bakeoff/mod.rs:355`). Bump the `advisor_field_arm_count` covering test
      13 → **18** (`runner.rs:66`; single-sourced — a test update, not a contract
      change).
- [ ] **T9** — Add the 5 `strategy_dir_slug` entries (`engine.rs:657`) for
      write-path correctness (unreachable on the bake-off path; consistent with
      the existing `v0.5.*` arms sharing one slug — use a new `"v0-signal-library"`
      group or reuse `"v05-composed-strategies"`).

### Phase 4 — the day-1 divergence gate (D4 — CLAUDE.md non-negotiable)
- [ ] **T10** — `crates/strategy/tests/signal_library_divergence_end_to_end.rs`
      (mirror `combination_slate_divergence_end_to_end.rs`, the `run_strategy_equity`
      harness). Build the 5 NEW + the 4 EXISTING arms as REAL `ComposedStrategy`
      from their TOML strings (`from_str` → `from_config`). Purpose-built Decimal
      series (a ramp printing a new 20-bar high + a ≥2× volume spike on the
      breakout bar + sustained up-closes with rising volume for OBV + a pullback).
      Assert: (1) each new arm ≥1bp from ≥1 existing arm; (2) each ≥1bp from
      buy-and-hold; (3) the 5 new arms pairwise distinct; (4) FAIL-before/PASS-after
      documented; (5) factory smoke — each real TOML loads, id==stem. **Build the
      series so RSI and `donchian_floor` visibly disagree** (price holds the 20-bar
      floor while RSI never dips < 30) — the `donchian_floor ⊂ btc_rsi_reversion`
      overlap (D4) is handled, not a blocker.

### Phase 5 — the UI touch (D6 — Q-SL-5)
- [ ] **T11** — `crates/ui/src/strings.rs`: add the 5 label constants
      (`strings.rs:2549` pattern). `crates/ui/src/screens/leaderboard.rs`
      `display_label` (`leaderboard.rs:957`): map the 5 ids to the friendly labels
      (own the words UI-side — the combination-search lesson).
- [ ] **T12** — `crates/ui/tests/leaderboard_signal_library_render.rs` (mirror
      `leaderboard_short_arms_render.rs`, `#![cfg(target_os = "macos")]`): render
      the REAL leaderboard HEADLESS with an ~18-row fixture; assert the 5 new rows
      paint their friendly labels + KPIs + (likely) Fragile badge; a NEGATIVE
      CONTROL (13-arm field) proves the guard discriminates. PNG to
      `/tmp/leaderboard_signal_library_render.png`. (Per verify-UI-at-render-layer
      — a model-`Ready` state / text `.snap` / no-panic boot is NOT proof.)
- [ ] **T13** — Assert the new arms do NOT break Tune or forward-plan: a small
      test (or extend an existing one) that `describe_plan` returns the `SmaCross`
      fallback (no panic) for each new id (`node.rs:1358`), and that the Tune
      editor still builds + does not offer the new families as tune-able (out of
      v1 — sweeps only SMA/MACD/RSI/Bollinger).

### Phase 6 — the decisive bake-off + close
- [ ] **T14** — The decisive real-data bake-off (BTCUSDT H1-2024, `BinanceCache`,
      frozen `RobustnessMode::Bootstrap { paths: 1000, seed: <LAB_DEFAULT_SEED
      low-8> }`) over the full `advisor_field()` with the 5 new arms. Record the
      pre-registered prediction (most/all Fragile → modal `BenchmarkWins`; `v0.obv`
      / `v0.vol_breakout` predicted most-decorrelated; `v0.roc_momentum` predicted
      most-correlated) + the realized 18-arm table (per-new-arm robustness flag +
      p5/p50 Sharpe + total-return + max-DD + trade_count). **A null all-Fragile
      result is a valid + expected + shippable PASS — the gate decides.**
      `write_report=false` → NO anchored body, NO `anchors.toml` SHA touched.
- [ ] **T15** — `bash scripts/verify_anchors.sh` → re-confirm **119/119** AFTER
      the last edit. Fill the trace `crates` + `tests` columns
      (`spec/trace.toml`); tester fills `anchors` (expected: still 119/119, none
      added). HANDOFF → tester.

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
