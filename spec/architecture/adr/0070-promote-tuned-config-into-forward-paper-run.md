---
adr: 0070
title: Promote a robustness-gated Tune config into the forward €200 paper-trade — the param-override seam on ForwardRunConfig, honored by both forward resolvers
status: accepted
date: 2026-06-25
supersedes: none
superseded-by: none
---

# ADR-0070: Promotion wiring — carry a robust Tune config into the forward paper-trade

## Context

The gate-tied hyperparameter sweep editor ("Tune", feature `advisor-param-tuning`,
ADR-0069) ships the operator a per-cell robustness verdict: overfit configs read
**FRAGILE** and are promotion-blocked; non-fragile (Robust / Marginal) configs are
`promotable`. ADR-0069 § D5 explicitly deferred the carry-forward: *"promotion-into-
F4/F5 OUT OF SCOPE v0.1"*, and `crates/ui/src/screens/tune.rs:967-970` says the
"Use this config" affordance on a promotable row is *"a visual pill (a v0.2 wires the
carry-forward)"* with **no message attached**. This ADR is that v0.2: wire the
affordance so a promotable config carries the **tuned** strategy into the existing
forward €200 paper-trade (the F5 paper loop + the F6 forward plan).

Five facts shape this decision (all verified in code, 2026-06-25):

1. **`ForwardRunConfig` carries NO param override.** `crates/agent/src/config.rs:32-42`
   carries `strategy: StrategyId`, `symbol`, `budget`, `lookback` — the strategy is
   resolved from the *id*; params come from `Config` / disk, not from the carrier.

2. **Both forward resolvers resolve `id → strategy` from fixed sources.**
   `build_registry_for` (`runtime.rs:331-475`): SMA reads
   `cfg.strategies.sma_crossover.{fast,slow}_len` (`:348-349`); MACD/RSI/BBands load
   the FIXED disk TOML `btc_macd_trend` / `btc_rsi_reversion` / `btc_bbands_mean_revert`
   (`:370-411`). `build_forward_plan_from_registry` (`plan.rs:190-268`) does the SAME
   for the F6 plan describer (`:200-202`, `:231-236`). Neither honors any tuned param.

3. **Both resolvers run in the supervisor's `Launch` arm, each taking `&cfg` (the
   `ForwardRunConfig`).** `runtime.rs:1096` calls `build_registry_for(&supervisor_config,
   Some(&cfg))`; `runtime.rs:1193` calls `build_forward_plan_from_registry(&supervisor_config,
   &cfg, …)`. **A new field on `ForwardRunConfig` therefore reaches BOTH** with no
   signature change to either resolver.

4. **The sweep already proves the per-family override paths — but only inside its
   own `ScenarioConfig`.** SMA → typed `sma_fast_len/slow_len`; MACD/RSI/Bollinger →
   `composed_toml_override`, an **in-memory** string from the private generators
   `macd_toml/rsi_toml/bbands_toml` (`sweep.rs:570-611`), each **identity-guarded** by
   a round-trip through `ComposedStrategyConfig::from_str(&toml, stem)`
   (`sweep.rs:681/712/743`). The forward path has no equivalent — that asymmetry is
   the engine gap.

5. **The F6 plan already renders structured rules.** `build_forward_plan_from_registry`
   emits a `PlanRuleKind` (closed `agent` enum), which `ForwardPlanView::from_plan`
   mirrors to `PlanRuleView` → IF/THEN copy (`forward_plan/adapter.rs:87-118`).
   So once the override reaches the plan resolver, **the F6 screen describes the tuned
   rules faithfully and automatically** — no plan-body UI change.

The product thesis ("no active strategy robustly beats just holding") makes a silent
or unfaithful promotion an overfitting footgun. Promotion must (a) only ever carry
non-FRAGILE configs, (b) run the byte-identical strategy the gate scored, and (c)
frame the carry honestly ("you tuned this; robust on THIS window ≠ a guarantee; not
advice").

## Decision

**D1 — A single optional `param_override: Option<ForwardParamOverride>` field on
`ForwardRunConfig`, honored by BOTH forward resolvers; `None` is byte-identical to
today (anchor-safe).** `ForwardParamOverride` is an **agent-owned closed enum** (NOT
`backtest::SweptParams` — keeps the UI's `core`-types-only invariant on the
UI-constructed carrier) with one variant per sweep family: `Sma{fast_len,slow_len}`,
`Macd{fast,slow,signal}`, `Rsi{period,oversold}`, `Bollinger{period,k_tenths}` (k as
tenths — the existing `PlanRuleKind::BollingerReversion` convention, `config.rs:94`,
no `Decimal` field). `build_registry_for` and `build_forward_plan_from_registry`
each gain a `Some` branch that resolves from the override; the `None` path runs the
**existing** match arms unchanged. The existing crowned-pick forward path
(`param_override: None`) is therefore byte-for-byte preserved — the
`forward_run_engine_fidelity.rs` regression tests pass unchanged, anchors stay
119/119.

**D2 — Composed-family overrides reuse the sweep's TOML generators (promoted to
`pub`), validated by the SAME `from_str` identity guard — so what paper-trades ==
what the gate scored.** The private `macd_toml/rsi_toml/bbands_toml` (`sweep.rs:570-611`)
become `pub` (re-exported from `backtest`; `agent` already depends on `backtest` →
no new crate edge); the sweep keeps calling them (no behaviour change). Both forward
resolvers, for a composed override, build the in-memory TOML via the identical
generator and validate via `ComposedStrategyConfig::from_str(&toml, stem)` — the
exact round-trip the sweep used to score the cell. No new disk file is written
(in-memory string, like the sweep); the composed `source_path` is a synthetic label
(`"tuned:<stem>"`) so audit identity reads "tuned, not the shipped TOML". One source
of truth for the DSL — duplicating the format strings agent-side is **rejected**
(two copies drift; a one-char change breaks fidelity silently).

**D3 — The tuned config reaches F5 (paper run) AND F6 (plan) through the ONE
override, both built from the SAME generator — the identity guarantee is
structural.** In the supervisor `Launch` arm, `build_registry_for` (F5) and
`build_forward_plan_from_registry` (F6) both read `cfg.param_override` and build
from it; for composed families both go through the SAME `from_str(stem)` guard.
Therefore: strategy-that-paper-trades == strategy-the-gate-scored == rules-the-plan-
describes == byte-identical AST (ADR-0062 § D3 "the plan and the loop share the same
resolved strategy" — preserved, now under the override).

**D4 — `PromoteSweptConfig(PromoteParams)` is the UI message; the pure handler
preseeds the forward target + navigates; the binary layer dispatches the
`ForwardCommand::Launch` (the crowned-pick precedent).** The UI carries a **closed
`PromoteParams` enum** (UI-owned, four families, k as tenths) added to `SweepCellRow`
and populated at the ONE engine→UI boundary `SweepReportMirror::from_report` /
`cell_to_row` (`tune/state.rs:191`, from `cell.params: backtest::SweptParams`) — so
`from_report` stays the ONLY place `SweptParams` is read and no engine type reaches
`view`/`update`. The pure `update` arm (mirroring `open_strategy_in_lab`,
`state.rs:2385`) preseeds `pending_forward_promotion: Option<ForwardPromotion>` and
sets `current_screen = Screen::ForwardPlan` (the operator reviews the plan, then
their €200 begins). The binary layer (`cockpit_live.rs`, the
`BakeoffRunCompleted` crowned-launch block `1531-1593`) maps `PromoteParams →
agent::ForwardParamOverride` (the single UI→agent map, binary-side where `agent` is
already imported), builds `ForwardRunConfig` with `param_override: Some(..)`, reuses
the F7/ADR-0065 €200→USDT conversion, and `try_send`s `ForwardCommand::Launch`.
**Two boundaries, each crossed once:** `SweptParams → PromoteParams` (from_report)
and `PromoteParams → ForwardParamOverride` (binary). Carrying the closed
`PromoteParams` in the message (not a mirror index) keeps it self-contained (the
`Recommendation`-not-`String` discipline).

**D5 — Only `promotable` (non-FRAGILE) configs promote; the FRAGILE lock and the
frozen gate are untouched; Marginal stays promotable.** `promotable == !Fragile` is
already computed at the boundary (`tune/state.rs:193`); the wired affordance gates
`on_press` behind `promotable` — a FRAGILE row keeps its greyed locked label and
emits NO message. Promotion **reads** the verdict, never recomputes it; no gate band,
seed rule, or verdict changes (reaffirms ADR-0059 § D4 / ADR-0063 § D4 / ADR-0066 D3
BYTE-FROZEN gate). Marginal is promotable exactly as today.

**D6 — Honest provenance framing, distinct from the crowned-pick framing.** A
`TUNE_PROMOTE_CONFIRM_FMT` string (zero literals, via `crate::strings`) renders as
the forward-plan header when the active plan came from a promotion: *"You tuned this
{family} config ({params}). It survived resampling on {window} — that is not a
guarantee, and not advice. Paper-trading your €{budget}."* The crowned-pick header
keeps its "best of the bake-off" provenance; the promoted header says "you tuned
this" — different provenance, different words. The persistent not-advice footer
(`TUNE_DISCLAIMER`) stays; promotion adds framing, never removes a disclaimer.

**D7 — Day-1 divergence + fidelity gate (CLAUDE.md non-negotiable, the
v3-vol-overlay-noop precedent).** New `crates/agent/tests/forward_promotion_divergence.rs`
(mirroring `forward_run_engine_fidelity.rs`): (a) **divergence** — same id, `None`
vs `Some(tuned)` (SMA + ≥1 composed), SAME bars → ≥1 differing signal/fill
(FAILS-before if the override is ignored); (b) **fidelity** — the agent's generated
composed TOML byte-equals the sweep's `build_swept_config` TOML for identical params
and the resolved `id` == stem (the ADR-0069 § D3 identity guard, agent-side);
(c) **plan-reflects-tuned** — `build_forward_plan_from_registry` with an SMA override
emits the tuned `PlanRuleKind::SmaCross`, not the default 20/50.

**D8 — Anchor-safe + render-pixel verified.** The `None`-override path is
byte-identical (D1) → 119/119 anchors preserved (re-run `verify_anchors.sh` after the
engine change). The cockpit UI is verified at the rendered-PIXEL layer
(`spec/dev-notes/iced-ui-render-verification.md`, font-mutex serialized): Proof 1 —
a promotable row's "Use this config" renders as the ENABLED accent button vs the
FRAGILE row's greyed locked-label negative control; Proof 2 — the post-promote
forward-plan shows the TUNED IF/THEN rules (tuned params, not default) + the
promote-framing header, with the crowned-pick header as the negative control.

**D9 — No new dependency; `cargo tree -p ui` UNCHANGED.** `agent` already depends on
`backtest` (the generators) and `strategy` (the resolvers). `PromoteParams` is
UI-owned; the `PromoteParams → ForwardParamOverride` map lives binary-side where
`agent` is already imported — no new `ui` edge. Single-binary / SQLite untouched;
edition 2024; no stdlib-shadowing name.

## Alternatives considered

- **Carry `backtest::SweptParams` directly on `ForwardRunConfig` / in the UI
  message.** Rejected — the UI constructs `ForwardRunConfig` (`cockpit_live.rs`) and
  imports it via `agent`; a public `SweptParams` field would either pull a
  `backtest::SweptParams` type into the UI's view surface or violate the
  `core`-types-only carrier invariant. The agent-owned `ForwardParamOverride` +
  UI-owned `PromoteParams` keep both seams closed and crossed exactly once.

- **Write a tuned TOML to `config/strategies/` and reuse the existing disk-load
  path.** Rejected — mutates a tracked config dir at runtime (anchor/identity
  hazard), races the strategy file-watcher, and leaves orphan files. The sweep
  already proved the in-memory `from_str` path; reuse it (D2).

- **A new "promotion" forward subsystem / a second paper loop.** Rejected — the F5
  loop + F6 plan + the `ForwardCommand::Launch` lifecycle already exist and the
  crowned-pick path already launches through them. Promotion is a second trigger
  into the SAME machinery ("set the forward target = this tuned config + go"), not a
  parallel system.

- **Recompute / re-display a verdict at promote time.** Rejected — the gate is
  frozen and already ran; promotion reads `promotable`. Re-running would risk a
  second, divergent verdict and contradict the "gate is the single source of truth"
  invariant.

- **Duplicate the TOML format strings agent-side instead of promoting the
  generators to `pub`.** Rejected — two copies of the `signal` DSL drift; fidelity
  (D2) would break silently on a one-character edit. One source of truth.

## Consequences

- **Positive:** the operator can paper-trade their €200 with a strategy THEY tuned,
  reviewed via a faithful forward plan, with honest provenance copy. Fidelity is
  structural (one override → both resolvers → one generator → one identity guard).
  The FRAGILE lock + frozen gate are untouched. Zero new deps, anchors preserved.
- **Negative / watch:** `SweepCellRow` gains `promote_params` (every literal +
  fixture updated). The F6 plan at Launch shows "FLAT — pending first bar" (sentinel
  close, `runtime.rs:1175-1186`) until a real bar lands — the framing copy must not
  imply a held position. A promoted run and a crowned run drive the same loop; the
  plan header's provenance copy is the only live signal of which is running — it must
  always reflect the active provenance.
- **Follow-ons (not in scope):** a replay-preview window for the promoted run
  (`lookback` is still `None`, the crowned-pick MVP behaviour); a "promotion history"
  trail. Both deferred.

## Changelog

- 2026-06-25 — accepted. Initial promotion-wiring seam: `ForwardRunConfig.param_override`
  (agent-owned `ForwardParamOverride`) honored by `build_registry_for` +
  `build_forward_plan_from_registry`; sweep TOML generators promoted to `pub` for
  byte-identical composed fidelity (shared `from_str` identity guard); UI
  `PromoteSweptConfig(PromoteParams)` preseeds the forward launch (the crowned-pick
  precedent), two boundaries each crossed once; FRAGILE lock + frozen gate untouched
  (Marginal promotable); honest "you tuned this, not a bake-off winner" provenance
  framing; day-1 divergence + fidelity + plan-reflects-tuned gates; anchor-safe
  (119/119) + render-pixel proofs.
