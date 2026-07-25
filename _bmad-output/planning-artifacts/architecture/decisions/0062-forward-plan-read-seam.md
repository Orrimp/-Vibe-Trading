---
adr: 0062
title: Forward-plan read seam — read-only stance/rules trait + core-typed ForwardPlan mirror
status: accepted
date: 2026-06-21
supersedes: none
superseded-by: none
---

# ADR-0062: Forward-plan read seam — read-only stance/rules trait + `core`-typed `ForwardPlan` mirror

## Context

The single-coin investment-advisor MVP (product pivot 2026-06-19) journey step 4
(feature `advisor-forward-plan`, roadmap **F6**) sits **between** the crowned
bake-off pick (F1–F3, ADR-0059) and the forward paper-trade (F4–F5, ADR-0060).
It is a **read-only, conditional, reactive decision plan — NOT a price forecast**
([feature.md](../../../../docs/archive/pre-bmad-spec/v1/advisor-forward-plan/feature.md) § The honest definition). For
the crowned strategy the surface shows:

1. **Current stance** — the latest-bar signal (`FLAT` / `LONG`, plus the most
   recent `BUY` / `SELL` / `HOLD`) as of the most recent available bar, with the
   bar's close price + timestamp.
2. **Standing rules in plain language** — the entry/exit conditions, *the same
   rules the F5 paper-trade then executes* ("buys when the fast SMA crosses above
   the slow SMA, sells on the reverse cross"), stated **qualitatively** (not a
   precise numeric trigger ladder).
3. **Budget-aware projected €200 next-BUY sizing** — `units ≈ budget / last_close`,
   **capped by the same F4 `budget_cap`** the F5 loop enforces.

over a configurable horizon (default 7 days, range 1–30 — operator-locked).

Three structural facts, **verified against code** (not theorised), force a seam
decision before the developer starts:

- **The `Strategy` trait has no read-only stance/rules accessor.**
  `crates/strategy/src/traits.rs:8` exposes only the **reactive**
  `on_bar(&mut self, &Bar) -> Vec<Signal>` / `on_tick` + `quantity_scale(&self)`.
  `on_bar` **mutates** indicator state (`self.fast.push(close)` in
  `sma_crossover.rs:39`; `ComposedStrategy::on_bar` advances every indicator then
  evaluates the rule tree, `composed/node.rs`). There is no way to ask "what is
  your current signal given the latest bar **without** mutating you" or "describe
  your standing rules" today.
- **Signal evidence is non-uniform.** `SmaCrossover` populates
  `fast_ma/slow_ma` in `Signal.evidence` (`sma_crossover.rs:57`), but
  `ComposedStrategy` (MACD/RSI/Bollinger) emits `SignalEvidence::empty()`
  (`composed/node.rs:1211`). So the plan's "rules + stance" cannot be read off
  existing signals uniformly.
- **The F5 forward run currently *proxies* every non-SMA strategy through
  `SmaCrossover`.** `agent::runtime::build_registry_for` (`runtime.rs:288`)
  resolves `v0.5.macd` / `v0.5.rsi` / `v0.5.bbands` / `v0.buyhold` to an
  `SmaCrossover` instance (logged as a proxy) until the dedicated forward-run
  ctors land (ADR-0060 OQ / F5b). **This is load-bearing for honesty**: the
  engine the F5 loop *actually runs* for a crowned MACD pick is, today, an SMA
  proxy — so the plan must describe **what the forward registry resolves to**,
  not the Lab-time strategy, or it would assert rules the loop does not run.

The crowned `(strategy, coin, budget)` selection already crosses the `ui` seam as
`core` types: `LeaderRow { strategy: SmolStr, is_benchmark, … }` (the crowned id),
`BakeoffReportMirror.coin`, `LeaderboardScreenState::budget_eur()`. The F5 launch
(ADR-0060 § D6) sends `ForwardCommand::Launch(ForwardRunConfig{strategy, symbol,
budget, …})` over an mpsc (`forward_rx`) into the `paper_loop_supervisor`, which
calls `build_registry_for(Some(&cfg))` and hot-swaps the trading loop. F6 must
reuse **that exact selection and that exact resolved registry** so the plan and
the paper-trade are provably the same engine (R7, the honesty thesis).

The standing layering invariant holds: the `ui` crate (lib + every binary) never
depends on `strategy` / `exec` / `forecast` / `llm` (ADR-0023 / ADR-0041); the
gate is `cargo tree -p ui` unchanged.

## Decision

**D1 — The read surface is a NEW read-only trait `PlanDescribe`, a SIBLING of
`Strategy`, NOT a method on `Strategy`.** Add to `crates/strategy`:

```rust
// crates/strategy/src/plan.rs  (sketch — developer owns the final shape)
pub trait PlanDescribe {
    /// Describe the standing stance + rules + next-action sizing WITHOUT
    /// mutating indicator state. Pure read (`&self`).
    fn describe_plan(&self, ctx: &PlanContext) -> StrategyPlan;
}

pub struct PlanContext {
    pub last_close: Price,        // core type — the latest bar's close
    pub last_bar_ts: Timestamp,   // core type — for the honest-staleness label
    pub budget: Money<Usdt>,      // core type — €200 ≈ 200 USDT (product § D4)
    pub budget_cap: Money<Usdt>,  // core type — the SAME F4 cap the F5 loop enforces
}
```

`StrategyPlan` is a **`strategy`-side struct** (it may name `core` types freely;
`strategy` already depends on `core`) carrying a closed `PlanStance` enum
(`Flat` / `Long`), an optional latest `PlanSignal` (`Buy` / `Sell` / `Hold`), a
closed `PlanRuleShape` enum that **names the rule family, not free text** (e.g.
`SmaCross { fast_len, slow_len }`, `MacdCross { fast, slow, signal }`,
`RsiReversion { len, lower, upper }`, `BollingerReversion { len, k }`,
`BuyAndHold`), and a `ProjectedSizing { units: Quantity, capped: bool }`. The
**plain-language copy is NOT in `StrategyPlan`** — the engine emits *structured
rule data*, the `ui` owns the words (the ADR-0059 `Recommendation`-not-a-`String`
precedent: no engine string crosses the seam, the copy + the mandatory
not-advice/not-a-prediction disclaimers live in `ui::strings`).

*Why a sibling trait, not a `Strategy` method:* the F6 candidate set is the five
bake-off arms + buy-and-hold. Putting `describe_plan` on `Strategy` would force
**every** impl — `VolTargetingOverlay`, `Pairs`, `CrossSectionalMomentum`,
`RegimeDispatcher`, the TCN/PatchTST overlays — to implement a method they will
never be asked for in the advisor flow, for no benefit, and would couple an
unrelated v0.5 trait surface to an MVP read concern. A sibling trait keeps the
blast radius to exactly the engines F6 describes and gives F8 (ensemble) +
any precise-trigger follow-up a clean home (they `impl PlanDescribe`). ADR-0005
froze the *`Strategy`* trait shape "does NOT change for v0.5/v1+" — a sibling
trait honours that freeze.

**D2 — The stance-now half is a NON-MUTATING evaluation, NOT a clone-and-step of
the live engine.** `describe_plan` derives the current stance from the engine's
own already-warmed indicator values (read-only getters on the concrete struct —
e.g. `SmaCrossover` exposes its latest `fast`/`slow` SMA values; the composed
engine exposes its latest MACD/RSI/Bollinger readings) **against `last_close`**,
applying the *same* comparison `on_bar` would apply — without pushing
`last_close` into the indicator (no `push`, no state advance). The plan is a
**snapshot of the standing decision**, read at the bar the supervisor last saw.
Rejected: a `Clone`-the-strategy-and-`on_bar` path — it requires `Strategy: Clone`
(a trait-shape change ADR-0005 forbids), doubles indicator memory, and risks a
divergent warm-up. The non-mutating read is sufficient because the plan describes
*standing* behaviour, not a hypothetical next bar.

**D3 — The plan is produced AGENT-SIDE, at the `ForwardCommand::Launch` boundary,
from the SAME resolved registry the F5 swap uses (consistency by construction).**
In `agent::runtime::paper_loop_supervisor`, the `Launch(cfg)` arm already calls
`build_registry_for(Some(&cfg))` to get the strategy for the hot-swap (ADR-0060
§ D6.2). F6 adds **one read on that same registry**: resolve the selected
`StrategyId` to its `&dyn PlanDescribe`, call `describe_plan(&PlanContext{…})`
with `cfg.budget`, the F4 `budget_cap` derived from `cfg.budget`, and the latest
bar's `(close, ts)` from the feed/`btc_closes_seed` the loop is about to consume,
then **mirror `StrategyPlan` → the `core`-typed `ForwardPlan`** (D4) and send it
back to the iced thread. Because the plan and the paper-trade are read from the
**one resolved registry entry** (including the SMA-proxy resolution for non-SMA
ids today), the plan **cannot describe rules the loop does not run** — drift is
structurally impossible, not merely tested. This is the answer to OQ-B(a) and the
direct discharge of R7.

**D4 — The plan reaches `ui` as a `core`-typed `ForwardPlan` over a SECOND mpsc,
symmetric with the `forward_rx` launch channel.** `ForwardPlan` is a new public
type in `agent::config` (alongside `ForwardRunConfig`), **`Clone + Debug`, every
field a `core` type or `Decimal`/primitive** — NO `strategy` / `exec` / `forecast`
/ `llm` types:

```rust
// crates/agent/src/config.rs  (sketch — developer owns the final shape)
#[derive(Debug, Clone)]
pub struct ForwardPlan {
    pub strategy: trading_core::StrategyId,   // the resolved forward-run id
    pub symbol:   trading_core::Symbol,       // the coin
    pub stance:   PlanStance,                 // FLAT | LONG  (core-free closed enum, defined here)
    pub latest_signal: Option<PlanSignal>,    // BUY | SELL | HOLD (closed enum)
    pub rule: PlanRuleKind,                   // a CLOSED enum the ui maps to copy (no engine String)
    pub last_close: trading_core::Price,      // the projection price
    pub last_bar_ts: trading_core::Timestamp, // honest-staleness label
    pub budget: trading_core::Money<trading_core::Usdt>,   // €200 ≈ 200 USDT
    pub projected_units: trading_core::Quantity,           // units ≈ budget/last_close, capped
    pub sizing_capped: bool,                  // true iff the F4 budget_cap bound the units
    pub horizon_days: u16,                    // DISPLAY-ONLY framing (D6) — does NOT terminate F5
}
```

`PlanStance` / `PlanSignal` / `PlanRuleKind` are **closed enums defined in
`agent::config`** (NOT re-exported from `strategy`), so `ui` matches on an
`agent`-owned discriminant and never gains a `strategy` edge — the exact
`LeaderRow` / `RobustnessLabel` / `OutcomeKind` mirror discipline (`ui` mirrors
`backtest::BakeoffReport` into `BakeoffReportMirror` via closed `ui`-side enums).
The `agent` boundary maps `strategy::PlanRuleShape` → `agent::config::PlanRuleKind`
in one place (the supervisor), the same way `BakeoffReportMirror::from_report`
is the one place an engine `BakeoffReport` is read. `ui` then mirrors `ForwardPlan`
→ a `ui`-side `ForwardPlanView` (closed `ui` enums + `ui`-owned copy) for render,
identical to `BakeoffReportMirror`. `cargo tree -p ui` stays unchanged.

The transport is a new `mpsc::Receiver<ForwardPlan>` on `RunHandles`
(`forward_plan_rx` — the **agent→iced** return path, symmetric with the
**iced→agent** `forward_rx`). The supervisor holds the `Sender`; on each
`Launch` it sends the freshly-built `ForwardPlan`. The iced thread receives it via
an iced subscription/recipe (the same way Live receives PnL) and renders the plan
surface. Headless `trading` bin + the soak harness pass `forward_plan_rx = None`
(byte-identical to today).

**D5 — The buy-and-hold degenerate case is a first-class `PlanRuleKind::BuyAndHold`
arm, not a special-cased absence.** When the crowned id is `v0.buyhold`,
`describe_plan` returns `stance = Long` (after the first buy), `latest_signal =
None` (no re-evaluation), `rule = BuyAndHold`, `projected_units = budget/last_close`
(the full €200), `sizing_capped` per the cap. The `ui` maps `BuyAndHold` to the
"buy now, hold the whole horizon, no sell trigger, deploy the full €200" copy
(feature.md § buy-and-hold special case). This makes the degenerate plan render as
**obviously the same kind of object** as an active plan (R5 + the ui-designer's
OQ-D), and it is the render-layer **negative control** (D8).

**D6 — The horizon is DISPLAY-ONLY metadata; F5 stays BYTE-IDENTICAL (OQ-C
resolved, operator-locked OPEN-ENDED).** `horizon_days` travels only as a field on
`ForwardPlan` (and may be echoed into `ForwardRunConfig` purely to compute the
"planned through <date>" label). It does **NOT** add a `horizon_days`
self-terminate to the forward run. The F5 `paper_loop_supervisor` / `spawn_trading_loop`
lifecycle is **unchanged** — the forward run remains OPEN-ENDED exactly as ADR-0060
§ D6 ships it; the "coming N days" is the **window over which the standing rules
are in force and checked each bar** (the planning frame), not a stop condition.
Rejected: a `horizon_days` that bounds the run — it would change the F5 lifecycle
(ADR-0060 § D6 determinism + the soak's open-ended assumption), and the operator
explicitly locked OPEN-ENDED. The horizon is a label, not a clock.

**D7 — Anchor-neutral + the day-1 equity-divergence e2e is N/A (both stated for the
tester).** F6 is a **read-only descriptive surface**: it places no orders, runs no
new backtest, writes no `docs/archive/pre-bmad-spec/*/reports/` body, and changes no `evidence/anchors.toml`
SHA. `scripts/verify_anchors.sh` stays **119/119 by construction** (the gate is run
before + after). The CLAUDE.md day-1 **baseline-equity-divergence e2e gate is N/A**:
that gate is for a *strategy overlay or sizing modifier* (it landed on F4 /
ADR-0060 § D2, the `budget_cap`); F6 *describes* sizing, it does not *size or run*
anything, so there is no equity path to diverge. The anti-drift guarantee is
instead made testable by D8's consistency assertion.

**D8 — The verification contract (handed to the tester):**
1. **Render-layer PNG (the floor, per the CLAUDE.md cockpit rule).** A **populated**
   active-strategy plan (e.g. an SMA pick showing the FLAT/LONG stance badge, the
   IF/THEN standing rules, and the €200 projected-sizing number) **plus the
   buy-and-hold degenerate plan as the NEGATIVE CONTROL** — asserting the plan
   text + the projected-sizing number actually **paint** (not a no-panic boot, not
   a text `.snap`). Harness: the existing `iced_test::screenshot` real-renderer
   pattern at `crates/ui/tests/leaderboard_populated_render.rs` (the
   `leaderboard_screen_program` + `PanelState::Empty` negative-control idiom),
   macOS-canonical per ADR-0057 § D2. A third "no pick yet" empty state is the
   tautology guard.
2. **The anti-drift consistency assertion (the honesty guarantee, made testable).**
   A `strategy`-crate (or `agent`-crate) unit/integration test that, for each
   candidate engine **as resolved by `build_registry_for`**, asserts the
   `describe_plan` stance + rule-family on a given `(last_close, prior bars)`
   **matches the engine's actual `on_bar` decision on the same bar** — i.e. the
   plan describes exactly what the loop runs. This is the testable form of "the
   plan cannot drift from the engine," covering the SMA-proxy resolution honestly.
3. **`cargo tree -p ui` unchanged** (the layering gate — no `strategy`/`exec`/
   `forecast`/`llm` edge added by `ForwardPlan`).

## Alternatives considered

- **(OQ-A fallback) Derive the rule text UI-side from the `ComposedStrategy` TOML
  AST** — **rejected as the primary path.** The AST (`Stage`, `cross_above`/
  `cross_below`, thresholds, `source_path` in `composed/config.rs`/`parser.rs`) is
  structurally available, and a `ui`-side derivation is cheaper (no trait change).
  But it has **two disqualifying drift vectors for this product**: (i) it creates a
  *second source of truth* for the rules — the AST the UI reads vs the engine the
  loop runs — and the product's entire credibility rests on "the plan is exactly
  what the engine will do"; (ii) **it would be actively wrong today**, because
  `build_registry_for` proxies MACD/RSI/Bollinger/buy-hold through `SmaCrossover`
  for the forward run — so a UI that read the *MACD AST* would describe a MACD
  cross while the F5 loop actually runs an SMA cross. The feature brief's
  if-budget-tightens annotation requires an explicit proof that the fallback
  *cannot* drift to be acceptable; the SMA-proxy fact is a concrete proof it
  **can and does** drift. The drift risk is disqualifying; D1+D3 source the plan
  from the resolved engine itself, so consistency holds by construction.
- **`describe_plan` as a method on the `Strategy` trait** — rejected (D1): forces
  every overlay/pairs/cross-sectional/forecast impl to implement a method the
  advisor flow never calls them for, and mutates the ADR-0005-frozen `Strategy`
  shape. A sibling `PlanDescribe` trait scopes the change to the F6 candidates.
- **Clone-the-strategy-and-`on_bar` for the stance** — rejected (D2): needs
  `Strategy: Clone` (an ADR-0005-forbidden shape change), doubles indicator state,
  and risks divergent warm-up. A non-mutating read of the already-warmed indicators
  is sufficient for a *standing-stance* snapshot.
- **Produce the plan UI-side from the crowned `LeaderRow` alone** — rejected: the
  `LeaderRow` carries KPIs + the id, not the stance or the standing rules, and a
  UI-side reconstruction reintroduces the drift vector. The plan must come from the
  resolved engine (D3).
- **Plan over a fresh `BakeoffReport`-style read at panel-open (OQ-B(b))** —
  rejected: a separate read decoupled from the `Launch` selection can use a
  *different* registry resolution than the one the loop swapped to (e.g. if the id
  set or proxy mapping changes), reopening drift. Binding the plan to the `Launch`
  boundary (D3) shares one selection + one resolved registry → R7 by construction.
- **`horizon_days` bounds the F5 run (self-terminate)** — rejected (D6): the
  operator locked OPEN-ENDED; a self-terminate changes the F5 lifecycle (ADR-0060
  § D6) + the soak's open-ended assumption. The horizon is a display frame.
- **`ForwardPlan` carries a pre-rendered copy `String` from the engine** — rejected:
  couples the engine to UI copy, blocks deterministic snapshotting + the not-advice
  /not-a-prediction disclaimer ownership, and breaks the closed-enum mirror
  discipline (ADR-0059 § D2 precedent). The engine emits structured rule data; the
  `ui` owns the words.
- **Reuse `forward_rx` for the return path** — rejected: `forward_rx` is the
  iced→agent *command* channel; the plan is an agent→iced *result*. A separate
  `forward_plan_rx` keeps the directions clean and mirrors the launch channel
  symmetrically (D4).

## Consequences

- **Honesty thesis discharged by construction (R7).** The plan is read from the
  *same* resolved `build_registry_for(Some(&cfg))` entry the F5 hot-swap runs, at
  the *same* `Launch` boundary, so the plan and the paper-trade are provably one
  engine — including today's SMA-proxy resolution, which the plan now describes
  honestly instead of asserting Lab-time rules the loop does not run. The
  consistency is *tested* (D8.2) but holds even before the test.
- **Layering invariant held (R8).** `ForwardPlan` + its enums are `agent`-owned and
  `core`-typed; the `strategy`→`agent` map lives in the supervisor; `ui` mirrors as
  it mirrors `BakeoffReport`. Gate: `cargo tree -p ui` unchanged (D8.3).
- **F5 byte-identical (D6 / OQ-C).** No `horizon_days` self-terminate; the
  `paper_loop_supervisor` / `spawn_trading_loop` lifecycle is untouched; the soak's
  open-ended forward run is unaffected. `forward_plan_rx = None` (headless + soak)
  is byte-identical to today.
- **Anchor-neutral + equity-divergence-N/A (D7).** No anchored report written, no
  `anchors.toml` SHA changed → `verify_anchors.sh` 119/119 by construction (run
  before+after). The CLAUDE.md sizing-modifier e2e gate does not apply (F6 sizes
  nothing — it describes); this is stated so the tester does not expect it.
- **Reuse / future-proofing.** `PlanDescribe` is the seam F8 (ensemble) and any
  precise-numeric-trigger follow-up reuse — the trigger-ladder upgrade has a home
  (a richer `PlanRuleShape` variant) without touching `ui` (the closed enum widens;
  the mirror maps the new variant). When the dedicated MACD/RSI/Bollinger
  forward-run ctors land (ADR-0060 F5b), the SMA-proxy resolution disappears and
  `describe_plan` automatically describes the real engine — no F6 rework, because
  the plan was always sourced from `build_registry_for`'s resolution.
- **No new dependency, no new crate.** `PlanDescribe` lives in `strategy` (which
  owns the engines); `ForwardPlan` lives in `agent` (which owns the supervisor +
  `ForwardRunConfig`); `ui` mirrors. No `crates/advisor`, no new edge.
- **This ADR does not add, remove, or mutate any of the 9 anchor SHAs in
  `evidence/anchors.toml`** — F6 produces no anchored artifact; the
  anchor-mutation-requires-an-ADR rule is untriggered.
- **Open (none gate the build):** when F5b lands the real per-strategy forward
  ctors, the `describe_plan` rule families for MACD/RSI/Bollinger become
  first-class (today they honestly describe the SMA proxy); the closed
  `PlanRuleKind` already reserves the variants so this is additive.

## Changelog

- 2026-06-21 (architect): initial accept. Homes the **forward-plan read seam** for
  feature `advisor-forward-plan` (F6). **D1** a NEW read-only sibling trait
  `strategy::PlanDescribe::describe_plan(&self, &PlanContext) -> StrategyPlan`
  (non-mutating; scoped to the F6 candidate engines, NOT a `Strategy` method —
  honours the ADR-0005 trait freeze; engine emits structured rule data, not copy,
  per the ADR-0059 `Recommendation`-not-`String` precedent). **D2** stance-now is a
  non-mutating read of already-warmed indicators (rejects Clone-and-`on_bar`).
  **D3** the plan is produced AGENT-SIDE at the `ForwardCommand::Launch` boundary
  from the SAME `build_registry_for(Some(&cfg))` registry the F5 hot-swap runs
  (ADR-0060 § D6.2) → plan↔engine consistency by construction (R7), honestly
  describing today's SMA-proxy resolution. **D4** the plan crosses to `ui` as a
  `core`-typed `agent::config::ForwardPlan` (`Clone + Debug`, closed `agent`-owned
  `PlanStance`/`PlanSignal`/`PlanRuleKind` enums, no `strategy`/`exec`/`forecast`/
  `llm` types) over a SECOND mpsc `RunHandles.forward_plan_rx` (agent→iced,
  symmetric with `forward_rx`); `ui` mirrors it like `BakeoffReportMirror`. **D5**
  buy-and-hold is a first-class `PlanRuleKind::BuyAndHold` arm (buy-now/hold/no-sell/
  full-€200) + the render negative control. **D6** the horizon is DISPLAY-ONLY
  metadata (`horizon_days`); F5 stays BYTE-IDENTICAL (OQ-C, operator-locked
  OPEN-ENDED — no self-terminate). **D7** anchor-neutral (119/119 by construction)
  + the CLAUDE.md day-1 equity-divergence e2e is N/A (F6 describes, it does not
  size/run — that gate landed on F4). **D8** verification = render-layer PNG
  (populated plan + buy-and-hold negative control, `leaderboard_populated_render.rs`
  pattern) + the anti-drift consistency assertion (`describe_plan` matches `on_bar`
  on the same bar for each resolved engine) + `cargo tree -p ui` unchanged.
  Rejected the OQ-A fallback (UI-side TOML-AST rule derivation) with a concrete
  drift proof (the `build_registry_for` SMA-proxy makes a MACD-AST-derived plan
  actively wrong vs the SMA the loop runs). Feature: `advisor-forward-plan`.
  Leans on ADR-0060 (§ D6 launch seam), ADR-0059 (`BakeoffReport` mirror + the
  structured-data-not-`String` discipline), ADR-0005 (`Strategy` trait freeze),
  ADR-0023/0041 (ui layering), ADR-0057 § D2 (macOS render-pixel canonicality).
