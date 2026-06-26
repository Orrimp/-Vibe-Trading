---
adr: 0071
title: OBV DSL primitive (obv + obv_avg) and the pre-registered 5-arm signal-library expansion
status: accepted
date: 2026-06-26
supersedes: none
superseded-by: none
---

# ADR-0071: OBV DSL primitive + the pre-registered signal-library expansion

## Context

The single-coin advisor bakes off **four** base signals today — SMA crossover,
MACD trend, RSI reversion, Bollinger-band reversion — all price-only,
moving-average-or-band-family rules (feature `advisor-signal-library-expansion`,
the backlog's one product-aligned growth item, operator-approved 2026-06-26).
Two structural gaps: (a) whole signal axes a retail user expects — breakout /
channel, volume-flow, short-horizon momentum — are absent, so the honest
"we bake off *every* strategy on your coin" promise is only as wide as the menu;
(b) the combination feature (ADR-0067) draws vote ensembles from this pool, and
every member is price-derived — there is no structurally-decorrelated member to
pull the one legitimate Fragile→Robust lever (decorrelation).

This is the **third** pre-registered arm-class expansion scored by the existing
**byte-frozen** robustness gate (after combination-search ADR-0067 + short-selling
ADR-0068), and the cheapest. The operator **ratified a FIXED 5-arm slate**: four
DSL-only arms + **one genuinely-new indicator, On-Balance-Volume (OBV)** — the
operator chose to include exactly one new primitive (overriding the analyst's
"defer all primitives" lean; ATR-channel + VWAP-reversion stay deferred).

Six facts shape this decision (all verified in code via CodeGraph, 2026-06-26):

1. **The signal DSL is a FIXED enum of primitives, self-contained in
   `crates/strategy/src/composed/{node.rs,parser.rs,ast.rs}`** — it imports
   nothing from `crates/features` (the `features` streaming primitives serve only
   the cross-sectional/pairs strategies). A new DSL primitive lives in
   `node.rs`+`parser.rs`, **NOT** `features`.

2. **The cheap slate is expressible TODAY.** `max(high,N)` / `min(low,N)` /
   `avg(field,N)` + bar fields + scalar `*` + `AND` + `>` all exist
   (`parser.rs:34-49`, arith `parser.rs:309-312`). `close > max(high,20)`,
   `close > min(low,20)`, `close > max(high,20) AND volume > 2*avg(volume,20)`,
   and `close > avg(close,10)*1.05` need **zero** parser/evaluator code — only new
   TOMLs. The volume-surge clause is *already in production* in
   `btc_bbands_mean_revert.toml` (`volume > 1.5 * avg(volume, 20)`).

3. **OBV cannot reuse the existing `avg(...)`.** The rolling family `avg/max/min`
   takes a **bar field** as arg-0, matched by `field_arg` which accepts ONLY
   `Expr::BarField` (`node.rs:533-538`, `node.rs:924-930`). `obv` is not a bar
   field (`BAR_FIELDS`, `parser.rs:27`), so `avg(obv,N)` errors `UnknownParam`
   (`parser.rs:357`). OBV needs its own moving average.

4. **A new base signal becomes a bake-off arm via a shallow, proven seam.**
   `run_scenario` dispatches an id to `sma_composed_run::run` which loads
   `config/strategies/<stem>.toml` from disk (`sma_composed_run.rs:386`); the
   `"v0.5.macd"` arm (`engine.rs:1234`) is the exact precedent. The field is
   `BakeoffConfig::default_field()` (`bakeoff/mod.rs:355`); `advisor_field()`
   (`runner.rs:53`) concatenates it ∪ the ensembles and the cockpit auto-picks up
   new ids; the lockstep count is `advisor_field_arm_count` (`runner.rs:66`).

5. **`obv` would be the FIRST 0-arity indicator.** Every existing arity is ≥1
   (`parser.rs:34-49`). The parser routes an ident to `parse_indicator_call` only
   when followed by `(` (`parser.rs:348`), so the signal must spell it `obv()`
   (a bare `obv` errors). The empty-arg parse path reads correct
   (`parser.rs:374-396`) but is unexercised.

6. **`write_report=false` on the bake-off / Bootstrap advisor path**
   (`bakeoff/mod.rs:697`) → a new arm writes no anchored body
   (`maybe_write_report` no-ops, `engine.rs:702`) → `verify_anchors.sh` stays
   **119/119** by construction (confirmed at scoping). `describe_plan` already
   falls back to a generic `SmaCross` shape for unknown ids (`node.rs:1358`) → the
   F6 forward plan does not panic for the new arms.

## Decision

**D1 — Ship OBV as TWO minimal new indicators: `obv` (arity 0) + `obv_avg(N)`
(arity 1).** Each is a new `IndicatorState` enum variant (`node.rs:26`) mirroring
the existing `Sma` / `RollingAvg` shapes, with the 6-site evaluator surface added
twice: `indicator_arity` (`parser.rs:32`), `IndicatorState`, `latest()`
(`node.rs:119`), `on_bar` (`node.rs:150`), `eval_indicator_expr` (`node.rs:519`) +
a `find_*` lookup, and `add_indicator` (`node.rs:909`). No `ast.rs` `RuleAst`
variant (OBV is a plain value indicator, not comparison-sugar). Rejected reusing
`avg(obv,N)` (impossible per fact 3) and a single `obv()`-only primitive with a
degenerate `obv() > 0` signal (sign-of-OBV ≈ sign-of-drift; barely a strategy).

**D2 — OBV recurrence + warm-up are LOCKED + identity-guarded.**
`OBV_0 = 0` (seed `prev_close` on bar 0, like `Rsi`, `node.rs:251`; OBV is
`Some(0)` immediately); `OBV_t = OBV_{t-1} + sign(close_t − close_{t-1})·volume_t`
for t≥1, with `sign(x) ∈ {+1,0,−1}` and `volume_t = get_bar_field(bar,"volume")`
(`node.rs:1383`). All math is `rust_decimal::Decimal` (NEVER f64). `obv_avg(N)`
owns its inner `Obv` (a `Box<IndicatorState>`, the `MacdLine`-owns-EMA pattern,
`node.rs:43`), pushes `obv.latest()` each bar, and emits the mean once its window
is full (`RollingAvg` gating, `node.rs:382`); `None` during warm-up → the
comparison is `false` (the `eval_rule` `None`-guard, `node.rs:421`). A bare
`obv()` term and the inner `obv` inside `obv_avg` advance independently and agree
by construction (deterministic recurrence). The **OBV identity guard** (the
`t505` + ADR-0069 round-trip discipline) is REQUIRED: a `btc_obv` TOML round-trips
through `ComposedStrategyConfig::from_str`, and the `Obv` state reproduces a
hand-computed textbook OBV vector EXACTLY (Decimal, no tolerance) over a series
covering all three `sign` branches (up / down / flat), with `obv_avg(20)` equal
to the SMA of the reference OBV. The signal is spelled **`obv()` with empty
parens** (fact 5); a dedicated parser unit test for `obv()` is owed because it is
the first 0-arity indicator.

**D3 — The FIXED pre-registered 5-arm slate (operator-ratified, LOCKED literals,
NO search).** New composed TOMLs at `config/strategies/<stem>.toml`
(`kind="composed"`, `id==stem`, `size="fixed_fraction(0.1)"`):
`v0.donchian_break` = `btc_donchian_break` = `close > max(high, 20)`;
`v0.donchian_floor` = `btc_donchian_floor` = `close > min(low, 20)`;
`v0.vol_breakout` = `btc_vol_breakout` = `close > max(high, 20) AND volume > 2 *
avg(volume, 20)`; `v0.roc_momentum` = `btc_roc_momentum` = `close > avg(close,
10) * 1.05`; `v0.obv` = `btc_obv` = `obv() > obv_avg(20) AND close > sma(close,
50)`. The slate spans the breakout/channel + volume-flow axes the existing 4 do
not cover. Pre-registration (a code-declared slate chosen before results) is the
overfit defense — the same lock as ADR-0067 / ADR-0068.

**D4 — Each new arm is just another candidate scored by the BYTE-FROZEN gate.**
Per-arm via the `"v0.5.macd"` seam (fact 4): a `run_scenario` dispatch arm
(`composed_toml_override: None`; a UNIQUE non-anchored `scenario_name` per arm so
the unreachable write branch can never collide with an anchored body), the id in
`default_field()`, the `advisor_field_arm_count` test bump (13→18), and a
`strategy_dir_slug` entry (write-path correctness; unreachable on the bake-off
path). The robustness bands (`classify_verdict` / `verdict_bands` /
`compute_robustness_flag`, `bakeoff/robustness.rs`) and the buy-and-hold benchmark
(ADR-0066) are **byte-unchanged** — more candidates face the SAME bar. This is
**NOT** a B2/B3 band proposal.

**D5 — Anchor-safe by construction; no anchor-additive amendment owed.**
`write_report=false` → 119/119 held (run before the first seam + after the last).
The existing 4 base TOMLs + their anchored reports
(`btc-2023-1m-{sma-cross,macd-trend,rsi-reversion,bbands-mean-revert}`) stay
byte-identical — the 5 new arms are strictly additive new ids/files; the OBV
evaluator edits are ADDITIVE enum variants + match arms (no existing variant
changes shape). The 9 anchor SHAs in `spec/anchors.toml` are untouched. The
classifier byte-freeze keeps the block-bootstrap θ-surface anchors (which hash
the sweep-bin `classify_verdict`, not the bake-off) identical.

**D6 — Day-1 baseline-equity-divergence e2e (CLAUDE.md non-negotiable).** Each
new arm ships, from day 1, a divergence test on the REAL TOMLs (built via
`ComposedStrategyConfig::from_str` → `ComposedStrategy::from_config`,
`node.rs:1166`): terminal equity diverges ≥1 bp from ≥1 existing base arm AND from
buy-and-hold; no two new arms identical; a factory smoke that each TOML loads
(`id==stem`); FAIL-before/PASS-after. Modelled on
`combination_slate_divergence_end_to_end.rs`; stronger than the vote-ensemble case
because breakout/volume/ROC/OBV rules trip deterministically on a purpose-built
series. Note the `v0.donchian_floor` rule `close > min(low,20)` is a strict
SUPERSET of the second AND-clause of `btc_rsi_reversion` (`rsi(14) < 30 AND close
> min(low, 20)`) — they diverge (RSI fires strictly less often); the series is
built to make them visibly disagree.

**D7 — UI is label + render-pixel proof only; Tune + forward-plan are untouched.**
Five friendly `display_label` entries (`leaderboard.rs:957`) + 5 string constants
(`strings.rs`) so the rows read as strategies, not raw `v0.*` ids; a populated
leaderboard render-pixel guard at the ~18-row field with a negative control
(`#![cfg(target_os = "macos")]`, ADR-0057). The Tune editor (ADR-0069) sweeps the
4 EXISTING families only and does not enumerate `default_field()` → the new arms
do not surface there (out of v1). Forward-plan narration (`PlanRuleShape` per arm)
is a follow-on; the `SmaCross` fallback (fact 6) keeps it panic-free.

## Honest framing (load-bearing — not optional copy)

No alpha claim. The new base signals are **very likely ALSO Fragile** under the
frozen gate (the robustness program concluded 2026-06-08 that whole families are
uniformly Fragile on real crypto; the live advisor field is modal `BenchmarkWins`
per ADR-0066). The deliverable is honest **coverage** ("we tested breakout /
volume / momentum / OBV on your coin and here is how each scored vs holding") +
a richer **decorrelation menu** (the first structurally-decorrelated members for
the combination feature), **NOT a winner**. A **null result** ("every new base
signal is also Fragile; holding still stands") is the expected, valid, shippable
outcome — the gate decides, not the feature author. `BenchmarkWins` / `AllFragile`
reachability is UNCHANGED. Not-financial-advice + paper-only disclaimers stand.

## Consequences

- **Positive:** honest menu width (the product promise); a genuinely-orthogonal
  volume-flow member (OBV) for the combination feature; bounded blast radius (5
  TOMLs + 5 dispatch arms + 5 `default_field` ids + the 2-indicator OBV evaluator
  surface + the OBV identity test + the divergence e2e + the render guard); zero
  gate/benchmark change; anchor-safe by construction.
- **Negative / risk:** OBV is the one non-trivial piece — a new `IndicatorState`
  recurrence + the first 0-arity-call parse path; mitigated by the D2 identity
  guard + a dedicated `obv()` parser test, built FIRST and gated before any arm
  wiring. The field grows 13→18 arms ⇒ ≈+38% bake-off wall-clock (bootstrap
  ~linear in arm count), within budget on the on-demand determinate-progress path.
- **Follow-ons (explicitly OUT of v1, R-SL.8):** ATR-channel / VWAP-reversion
  primitives; combination arms USING the new signals; short-capable `_ls` variants
  (gated behind short-selling); any parameter sweep of a new signal (the
  gate-tied `advisor-param-tuning` job — never a free hunt here); per-arm
  forward-plan `PlanRuleShape` narration.

## Alternatives considered

- **OBV via `avg(obv,N)`** — rejected (impossible; `field_arg` accepts only
  `Expr::BarField`).
- **Single `obv()` primitive with `obv() > 0`** — rejected (degenerate signal).
- **OBV in `crates/features`** — rejected (the DSL is self-contained in `node.rs`;
  `features` is unrelated to this evaluator).
- **4 DSL-only arms, defer all primitives (analyst lean)** — superseded by the
  operator ratifying OBV into v1 for a decorrelated volume-flow member.
- **Loosening the robustness bands for the new asset/axis (B2/B3)** — rejected
  (operator-REJECTED; the gate is byte-frozen; this is "more candidates, same
  bar").

## References

- Feature: [`advisor-signal-library-expansion/feature.md`](../../advisor-signal-library-expansion/feature.md)
  (§ Design D0–D9), [`tasks.md`](../../advisor-signal-library-expansion/tasks.md).
- Trace: `REQ-ADVISOR-SIGNAL-LIBRARY-EXPANSION-001`.
- Leans on / relates to: ADR-0067 (pre-registered combination slate), ADR-0068
  (single-coin short-selling — sibling arm-class pattern), ADR-0066 (benchmark
  exempt from `AllFragile`), ADR-0069 (the identity/round-trip discipline for a
  generated/parsed config), ADR-0063/ADR-0059 (the frozen robustness gate),
  ADR-0010 (ComposedStrategy exit policy — signal-flip), ADR-0057 (render-pixel
  macOS-canonical guards).
