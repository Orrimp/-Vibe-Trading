---
adr: 0070
title: Promote a robustness-gated Tune config into the forward €200 paper-trade — the param-override seam on ForwardRunConfig, honored by both forward resolvers
status: accepted
date: 2026-06-25
supersedes: none
superseded-by: none
updated: 2026-11-28
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
`Recommendation`-not-`String` discipline). *(Amended 2026-11-24, bug-log `#213`: the
`PromoteParams → ForwardParamOverride` map and both forward launch configs now live in
`ui::live` (`promote_params_to_override`, `crowned_forward_run_config`,
`promoted_forward_run_config`), and the binary calls them, so a test can pin what each launch
carries. The boundary is still crossed once. See D11.)*

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
guarantee, and not advice. Paper-trading your €200 in {coin}."* *(Amended 2026-11-23,
bug-log `#200`: `{coin}` is the promotion's coin, which D10 takes off the landed grid, so
a cross-coin promotion is never silent; the €200 literal is the code's, not a `{budget}`
placeholder.)* The crowned-pick header keeps its "best of the bake-off" provenance; the
promoted header says "you tuned this" — different provenance, different words. The
persistent not-advice footer (`TUNE_DISCLAIMER`) stays; promotion adds framing, never
removes a disclaimer. *(Amended 2026-11-28, bug-log `#222`: the header is the headline and
caption as well as the strip. A promotion draws `FORWARD_PLAN_PROMOTED_HEADLINE` ("Your tuned
config") over `FORWARD_PLAN_PROMOTED_CAPTION` ("Watching your tuned config as new bars arrive —
not the bake-off’s crowned pick, so no confidence check applies to it. Not a prediction or a
guarantee of future edge."); a crowned plan keeps "Confidence check" and its caption.
`screens::forward_plan::header_copy` reads `pending_forward_promotion` and nothing else, the
field the strip reads, so the header and the strip never disagree. The header and the plan on
screen can: the binary clears the field when it sends a crowned launch, not when that launch's
plan lands, and a late plan can land under a fresh promotion; both orderings are filed as
bug-log `#244` (open). The loading line is neutral ("Reading the
strategy’s standing rules…"), since only a promotion flips the plan to `Loading`
(`state::promote_swept_config`). Pinned by U1
(`screens::forward_plan::tests::header_copy_follows_the_promotion_not_the_bakeoff`), W1
(`forward_plan_provenance.rs::promoted_copy_says_tuned_not_crowned`) and LR-222
(`forward_plan_populated_render.rs::promoted_header_is_not_the_crowned_header_on_one_bakeoff`,
the crowned header on the same bake-off its negative control).)*

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
(`docs/dev-notes/iced-ui-render-verification.md`, font-mutex serialized): Proof 1 —
a promotable row's "Use this config" renders as the ENABLED accent button vs the
FRAGILE row's greyed locked-label negative control; Proof 2 — the post-promote
forward-plan shows the TUNED IF/THEN rules (tuned params, not default) + the
promote-framing header, with the crowned-pick header as the negative control.

**D9 — No new dependency; `cargo tree -p ui` UNCHANGED.** `agent` already depends on
`backtest` (the generators) and `strategy` (the resolvers). `PromoteParams` is
UI-owned; the `PromoteParams → ForwardParamOverride` map lives binary-side where
`agent` is already imported — no new `ui` edge. Single-binary / SQLite untouched;
edition 2024; no stdlib-shadowing name. *(Amended 2026-11-24, bug-log `#213`: the map moved
from the binary into `ui::live`. That module is `#![cfg(feature = "live")]` and already names
`agent` types, so there is still no new edge and `cargo tree -p ui` is unchanged.)*

**D10 — A promotion names the landed grid (amended 2026-11-23, bug-log `#200`).** Its
coin, like its window and params, comes off the `Ready` mirror
(`TuneScreenState::landed_coin`); `Cockpit::tune_coin` is the NEXT sweep's coin, read only
when no grid is on screen. A grid stays on screen after "Tune…" on another coin (the Lab's
`tune_entry_button` sends the Lab pair), and a sweep in flight lands its grid after the coin
moved; "Use this config" then paper-trades the grid's coin, which the Tune footer and the
D6 strip name. Rejected: clearing the grid on re-open (it misses the sweep that lands
after, and destroys a grid); disabling Promote on a mismatch (the reducer stays wrong).

**D11 — A promotion carries no bake-off confidence (amended 2026-11-24, bug-log `#213`).**
The forward plan's "How much to trust this pick" block, and the plan export's TRUST section,
show `ForwardPlan.confidence`: the crowning bake-off's ADR-0075 scorecard, projected as
`ScorecardSummary`. Its `n_candidates` and `min_btl_years` (from the slate's N_eff) describe
the bake-off's slate, and its deflated Sharpe and `crown_clears_dsr` deflate the slate's best
Sharpe by the slate's N_eff and Sharpe variance, using the crown's return skew and kurtosis.
A promoted config is not that crown, even on the same coin and family, and the sweep cells it
was chosen from are not in the count. So `ForwardRunConfig.confidence` is `None` for every
promotion (`ui::live::promoted_forward_run_config`, which reads only
`pending_forward_promotion`), and the D6 strip carries the promoted plan's honesty framing. A crowned launch
(`ui::live::crowned_forward_run_config`) carries the scorecard of the bake-off whose
completion crowned it, unchanged. Before this, the binary copied the scorecard of whatever
bake-off was `Ready` on the leaderboard into the promoted launch, whatever its coin and crown
(its comment said "same bake-off, same crown"), and with no bake-off on the leaderboard it
copied none. Pinned by `crates/ui/tests/forward_launch_config.rs` (L1–L5; L5 keeps the binary
from building a launch config of its own) and LR-213 in `forward_plan_confidence_render.rs`.
Rejected: keeping the scorecard when the coin and family match (it still describes another
search and another strategy's returns, and undercounts the trials). Rejected: scoring the
sweep (its trial count and the tuned cell's deflated Sharpe): a new scorecard surface, the Tune slot ADR-0075 D1 reserves for
PBO/CSCV, so a feature and not a fix. Known divergences, filed as bug-log `#222` and `#223`:
the forward plan's headline and caption ("Confidence check", "Watching the crowned strategy …
a confidence check on that pick") still draw over a promoted plan, which now has no confidence
block under them (`#222`); and "Export this plan" on a promoted plan still takes its coin,
window, verdict (including the scorecard-derived credibility line, whose "see below" hint now
has no TRUST section to point to) and filename from the leaderboard's bake-off (`#223`).
*(Resolved 2026-11-28, except the window between the crowned launch and its plan landing (and
the reverse race, a late plan landing under a fresh promotion), filed as `#244` (open): `#222`
by the D6 amendment, `#223` by ADR-0088 § D5 as amended (a promoted plan offers no export, and
the export's weak-evidence hint prints only over its TRUST section, ADR-0088 § D3 as amended).
In `#244`'s windows the header can mislabel the plan on screen; the export is refused there.)*

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

- 2026-11-28 (developer, bug-log `#222`, `#223`): **D6** the header follows the provenance: a
  promotion draws "Your tuned config" and its own caption over the strip, a crowned plan keeps
  "Confidence check"; `screens::forward_plan::header_copy` reads `pending_forward_promotion`
  only; the loading line no longer says "crowned". **D11** the two known divergences the
  2026-11-24 entry filed are resolved, `#222` by D6 and `#223` by ADR-0088 § D3/§ D5 as amended
  (a promoted plan offers no export), except the window between a crowned launch and its plan
  landing and the reverse late-plan race, filed as `#244` (open). D1–D5, D7–D10, the FRAGILE lock, the frozen gate and the
  scorecard maths unchanged; anchor-neutral (119/119).
- 2026-11-24 (developer, bug-log `#213`): **D11** a promotion carries no bake-off confidence:
  `ForwardRunConfig.confidence` is `None` for every promoted launch, so the forward plan's
  "How much to trust this pick" block and the export's TRUST section appear for a crowned plan
  only. **D4/D9** the `PromoteParams → ForwardParamOverride` map and both launch configs moved
  from `cockpit_live.rs` into `ui::live` (cfg `live`; no new edge, `cargo tree -p ui`
  unchanged). Known divergences filed as bug-log `#222` (the header copy over a promoted plan)
  and `#223` (a promoted plan's export). D1–D3, D5–D8 and D10, the FRAGILE lock, the frozen gate and the scorecard maths
  unchanged; anchor-neutral (119/119).
- 2026-11-23 (developer, bug-log `#200`): **D10** a promotion's coin now comes off the
  landed grid (`TuneScreenState::landed_coin`), not `Cockpit::tune_coin`, which "Tune…"
  moves for the next sweep; the Tune footer names the same coin. **D6** the strip gains
  `{coin}`, and its quoted copy now matches the code (`€200`, not `€{budget}`). D1–D5 and
  D7–D9, the FRAGILE lock and the frozen gate unchanged; anchor-neutral (119/119).
- 2026-06-25 — accepted. Initial promotion-wiring seam: `ForwardRunConfig.param_override`
  (agent-owned `ForwardParamOverride`) honored by `build_registry_for` +
  `build_forward_plan_from_registry`; sweep TOML generators promoted to `pub` for
  byte-identical composed fidelity (shared `from_str` identity guard); UI
  `PromoteSweptConfig(PromoteParams)` preseeds the forward launch (the crowned-pick
  precedent), two boundaries each crossed once; FRAGILE lock + frozen gate untouched
  (Marginal promotable); honest "you tuned this, not a bake-off winner" provenance
  framing; day-1 divergence + fidelity + plan-reflects-tuned gates; anchor-safe
  (119/119) + render-pixel proofs.
