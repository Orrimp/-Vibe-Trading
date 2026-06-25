---
slug: advisor-param-promotion
status: draft
owner: architect
version: 0.1.0
updated: 2026-06-25
---

# Promotion wiring — carry a Tune-editor ROBUST config into the forward €200 paper-trade

## Why

The gate-tied hyperparameter sweep editor ("Tune", feature
[`advisor-param-tuning`](../advisor-param-tuning/feature.md), ADR-0069) is
shipped: the operator sweeps a strategy's params (SMA / MACD / RSI / Bollinger),
and **each** config is scored through the **frozen** robustness gate — overfit
configs render **FRAGILE** and are promotion-blocked; non-fragile
(Robust / Marginal) configs are `promotable`.

Today the **"Use this config"** affordance on a promotable row is a **visual
label only** — it carries no message (`crates/ui/src/screens/tune.rs:967-970`
says so verbatim: *"Promotion WIRING is out of scope for v0.1 … The enabled
affordance is a visual pill (a v0.2 wires the carry-forward)"*). ADR-0069's
open questions flagged exactly this as the deferred **v0.2 promotion follow-on**.
This feature is that follow-on: clicking "Use this config" on a promotable row
carries that **tuned** config into the existing **forward €200 paper-trade** (F5
paper loop + F6 forward plan), so the operator can watch a strategy **they
tuned** paper-trade their budget.

## Honesty constraints (NON-NEGOTIABLE — this product exists to avoid overfitting footguns)

1. **Only `promotable` (non-FRAGILE) configs promote.** The FRAGILE lock stays
   exactly as-is. A fragile config can NEVER be promoted — the gate's verdict is
   final. `promotable == !matches!(verdict, Fragile)` is already computed at the
   ONE boundary (`crates/ui/src/tune/state.rs:193`); promotion **reads** that
   flag, never recomputes a verdict. **Marginal is promotable, same as today**
   (only Fragile is locked) — confirmed.
2. **Byte-identical fidelity.** The promoted forward run must execute the
   **byte-identical tuned strategy the sweep scored** — the same identity-guard
   discipline ADR-0069 § D3 uses (the in-memory composed TOML must round-trip
   through `ComposedStrategyConfig::from_str` and produce the same AST as the
   sweep bootstrapped). A promoted config that silently runs different params is
   a fidelity bug and is gated against (T-D8 fidelity test).
3. **Honest framing.** The operator is carrying a config **THEY tuned** — not a
   bake-off winner. The promote confirmation + the forward plan header say so
   plainly: *"You tuned this. It survived resampling on THIS window. That is not
   a guarantee, and not advice."* No implication that robust-on-one-window ⇒ a
   sure thing. Consistent with the existing not-advice disclaimers.
4. **The gate stays FROZEN.** Promotion is a pure routing of an
   already-scored config; it touches no gate band, no seed rule, no verdict.

---

## Design

### 0. Where the seam is today (ground truth)

The forward paper-trade is driven by `ForwardRunConfig` (carried UI→agent on
`ForwardCommand::Launch`) and two agent-side resolvers that BOTH map a
`StrategyId` → concrete strategy:

| Seam | File:line | What it does today | The gap |
|------|-----------|--------------------|---------|
| `ForwardRunConfig` | `crates/agent/src/config.rs:32-42` | carries `strategy: StrategyId`, `symbol`, `budget`, `lookback` | **carries NO param override** — params come from `Config`/disk |
| `build_registry_for` | `crates/agent/src/runtime.rs:331-475` | matches `id`; SMA reads `cfg.strategies.sma_crossover.{fast,slow}_len`; MACD/RSI/BBands load FIXED disk TOML (`btc_macd_trend` etc.) | **ignores any tuned params** |
| `build_forward_plan_from_registry` | `crates/agent/src/plan.rs:190-268` | same id→strategy map for the F6 plan describer; same `cfg.strategies` / fixed disk TOML | **same gap** — plan describes DEFAULT rules |
| supervisor `Launch` arm | `crates/agent/src/runtime.rs:1058-1257` | on `ForwardCommand::Launch(cfg)` calls `build_registry_for(&supervisor_config, Some(&cfg))` (1096) then `build_forward_plan_from_registry(&supervisor_config, &cfg, …)` (1193) | both take `&cfg` (the `ForwardRunConfig`) — **so a field on `ForwardRunConfig` reaches BOTH** |
| UI launch dispatch | `crates/ui/src/bin/cockpit_live.rs:1531-1593` | on `BakeoffRunCompleted(Ok(mirror))` with a crowned row, builds `ForwardRunConfig` from `core` types + `try_send(ForwardCommand::Launch(fwd_cfg))` | the **precedent** to mirror for promotion |
| "Use this config" | `crates/ui/src/screens/tune.rs:971-1019` | `Container` pill (NO `Button`, NO `on_press`) on promotable; greyed locked label on fragile | the visual-only affordance to **wire** |

The sweep ALREADY proves the param-override engine paths exist — but only inside
the sweep's `ScenarioConfig`:

- **SMA** → `ScenarioConfig.sma_fast_len / sma_slow_len` (typed override).
- **MACD / RSI / Bollinger** → `ScenarioConfig.composed_toml_override` — an
  **in-memory** TOML string from `macd_toml(fast,slow,signal)` /
  `rsi_toml(period,oversold)` / `bbands_toml(period,k)` (`sweep.rs:570-611`),
  each **identity-guarded** by a round-trip through
  `ComposedStrategyConfig::from_str(&toml, stem)` (`sweep.rs:681/712/743`).

The forward path (`build_registry_for` / `build_forward_plan_from_registry`)
has **no equivalent override** — that asymmetry is the engine gap this feature
closes. **Crucially, `build_registry_for` and `build_forward_plan_from_registry`
must stay BYTE-IDENTICAL when no override is present** (the untuned crowned-pick
forward path — anchor-safe, and the existing
`forward_run_engine_fidelity.rs` tests must keep passing unchanged).

### 1. The forward-run seam — `ForwardRunConfig` carries a tuned override

**Does `ForwardRunConfig` carry an override today? NO.** Add ONE optional field:

```rust
// crates/agent/src/config.rs — ForwardRunConfig
pub struct ForwardRunConfig {
    pub strategy: trading_core::StrategyId,
    pub symbol:   trading_core::Symbol,
    pub budget:   trading_core::Money<trading_core::Usdt>,
    pub lookback: Option<backtest::engine::DateRange>,
    /// NEW (ADR-0070 § D1) — a tuned-param override carried from the Tune
    /// editor's "Use this config". `None` = the existing crowned-pick path
    /// (params come from `Config` / disk TOML — byte-identical, anchor-safe).
    pub param_override: Option<ForwardParamOverride>,
}
```

`ForwardParamOverride` is an **agent-owned closed enum** (NOT
`backtest::SweptParams` — keeps the `core`-types-only invariant on the UI side;
the UI imports `ForwardRunConfig` via `agent` and must not gain a `backtest`
`SweptParams` edge on a public field that the UI constructs). It mirrors the
four sweep families one-for-one:

```rust
// crates/agent/src/config.rs (NEW)
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ForwardParamOverride {
    Sma { fast_len: u32, slow_len: u32 },
    Macd { fast: u32, slow: u32, signal: u32 },
    Rsi { period: u32, oversold: u32 },
    Bollinger { period: u32, k_tenths: u32 }, // k encoded as tenths (20 = 2.0σ)
}
```

> `k_tenths` follows the existing `PlanRuleKind::BollingerReversion` convention
> (`config.rs:94`) — keeps the enum `Clone + Eq` with no `Decimal` field, and
> the agent converts `k_tenths → Decimal` exactly where it builds the TOML
> (`Decimal::from(k_tenths) / dec!(10)`).

**`build_registry_for` change (signature UNCHANGED; body honors the override
FIRST):**

```rust
pub fn build_registry_for(
    cfg: &Config,
    forward: Option<&crate::config::ForwardRunConfig>,
) -> anyhow::Result<Arc<strategy::StrategyRegistry>>
```

New body shape — when `fwd.param_override` is `Some`, resolve from the override
INSTEAD of `cfg.strategies` / disk TOML; when `None`, the existing match arms
run **byte-identically** (anchor-safe):

- `ForwardParamOverride::Sma { fast_len, slow_len }` →
  `strategy::SmaCrossover::new(fast_len as usize, slow_len as usize)` (mirrors
  the existing SMA arm but with the tuned lens instead of
  `cfg.strategies.sma_crossover`).
- `ForwardParamOverride::Macd { .. } / Rsi { .. } / Bollinger { .. }` → build
  the **in-memory TOML** via the SAME generators the sweep uses, then
  `ComposedStrategyConfig::from_str(&toml, stem)` (the identity guard) →
  `ComposedStrategy::from_config(cfg, source_path)`. **Reuse the sweep's TOML
  generators** — do NOT duplicate the DSL. See § "Shared TOML generators (the
  first dev task)" below.

**No new disk file is written** — the composed override is an in-memory string,
exactly like the sweep. `source_path` is a synthetic label (e.g.
`"tuned:btc_macd_trend"`) so audit identity reads as "tuned, not the shipped
TOML" (mirrors the `load_composed_strategy_from_toml` source-path convention,
`runtime.rs:488-491`).

**`build_forward_plan_from_registry` change (F6 plan — same override, same
resolution).** Signature unchanged (it already takes `&fwd`). Body: when
`fwd.param_override` is `Some`, build the describer from the override (SMA →
`SmaCrossover::new(tuned)`; composed → `from_str` describer using the SAME
generated TOML) so the plan's `PlanRuleKind` reflects the **tuned** params, not
the default. When `None`, byte-identical to today. Because
`ForwardPlanView::from_plan` already mirrors `PlanRuleKind` → `PlanRuleView`
→ IF/THEN copy (`crates/ui/src/forward_plan/adapter.rs:87-118`), **the F6 plan
screen renders the tuned rules faithfully and automatically** once the override
reaches this function — no UI change to the plan body is needed.

> Both resolvers building from the SAME override + SAME generators is the F6/F5
> fidelity guarantee (ADR-0062 § D3 "the plan and the loop share the same
> resolved strategy" — preserved, now under the override).

### Shared TOML generators (the FIRST dev task — the engine extraction)

The sweep's `macd_toml` / `rsi_toml` / `bbands_toml` are **private `fn`s** in
`crates/backtest/src/bakeoff/sweep.rs:570-611`. The agent's resolvers need the
identical strings (byte-identical fidelity). Extraction:

- **Promote the three generators to `pub`** in a small shared module
  `backtest::bakeoff::sweep` (or a sibling `composed_toml` module re-exported
  from `backtest`) with the SAME bodies. The sweep keeps calling them (no
  behaviour change); the agent now calls them too. `agent` already depends on
  `backtest`, so no new crate edge.
- Signatures: `pub fn macd_toml(fast: u32, slow: u32, signal: u32) -> String`,
  `pub fn rsi_toml(period: u32, oversold: u32) -> String`,
  `pub fn bbands_toml(period: u32, k: Decimal) -> String`.
- A round-trip test (already implied by ADR-0069 § D3) guards that each
  generator's output parses via `from_str` — keep it; add an agent-side mirror
  asserting the agent's resolved composed strategy id matches the stem.

> Alternative considered: duplicate the format strings agent-side. **Rejected** —
> two copies of the DSL drift; a one-character change to a generator would break
> fidelity silently. One source of truth.

### 2. The UI seam — `PromoteSweptConfig` → preseed forward + launch

**The carrier (UI-side, closed).** `SweepCellRow` (`tune/state.rs:91-114`) today
carries only `params_label: SmolStr` — a **display** string, no structured
params. Promotion needs structured params. Two sub-decisions:

- **(a) Add a structured `promote_params` to `SweepCellRow`** — a **UI-side
  closed enum** `PromoteParams` mirroring the four families, populated at the
  ONE boundary (`cell_to_row`, `tune/state.rs:191`) from `cell.params`
  (`backtest::SweptParams`). This keeps **the ONE engine→UI boundary intact**:
  `SweepReportMirror::from_report` stays the ONLY place `backtest::SweptParams`
  is read. `PromoteParams` is `Clone + Eq` (`k` as `k_tenths: u32`).
  > `from_report` is "the ONE boundary" per `tune/state.rs:1-20` — `from_report`
  > → `cell_to_row` reads `cell.params` and maps to `PromoteParams`. No engine
  > type leaks into `view`/`update`.
- **(b) The family/coin/window.** Family is implicit in the `PromoteParams`
  variant. Coin is `model.tune_coin` (`state.rs:1080`, already UI-side). Window:
  the sweep's range is echoed as `range_label` on the mirror; the forward run is
  real-time-only today (`lookback: None`), so promotion carries **no replay
  window** into F5 — it carries the **strategy identity + tuned params** (the
  honesty copy names the window the sweep scored, from `range_label`, for the
  "robust on THIS window" framing). This matches the crowned-pick path, which
  also launches real-time with `lookback: None`.

**The message.** Add to `crate::state::Message`:

```rust
/// advisor-param-promotion (ADR-0070) — "Use this config" on a PROMOTABLE Tune
/// row. Carries the structured tuned params + the family for that row. Pure
/// state transition (preseed + navigate); the binary-layer Launch dispatch
/// fires off the resulting preseeded forward target (the BakeoffRunCompleted
/// crowned-launch precedent).
PromoteSweptConfig(crate::tune::state::PromoteParams),
```

> Carry the **closed `PromoteParams`** (not an index into the mirror) so the
> message is self-contained and the handler need not re-look-up the row — robust
> against a mirror that changed under the operator (the `Recommendation`-not-
> -`String` discipline: structured data crosses, not a lookup key).

**The pure state transition (`update`).** A new arm mirroring the
`open_strategy_in_lab` navigate+preseed precedent (`state.rs:2385-2410`):

```rust
Message::PromoteSweptConfig(params) => {
    // Pure: preseed the forward-launch target + navigate to the forward
    // plan/launch surface. The actual ForwardCommand::Launch is dispatched
    // binary-side (cockpit_live.rs) off this preseeded target — the
    // BakeoffRunCompleted crowned-launch precedent. The gate verdict is NOT
    // re-read here; only PROMOTABLE rows emit this message (the view gates it).
    promote_swept_config(model, params);
}
```

`promote_swept_config(model, params)` (a shared helper):
1. Translate `PromoteParams` → the `StrategyId` the forward path dispatches on
   (Sma → `"v0.5.sma"`; Macd → `"v0.5.macd"`; Rsi → `"v0.5.rsi"`; Bollinger →
   `"v0.5.bbands"` — the SAME ids `build_registry_for` matches).
2. Store a **preseeded forward target** on the model: a new
   `pending_forward_promotion: Option<ForwardPromotion>` field where
   `ForwardPromotion { strategy_id, coin, params: PromoteParams }` (UI-side
   struct). This is the preseed the binary reads, exactly as the crowned-pick
   path reads `mirror.crowned_row()`.
3. Navigate to the forward-plan screen (`current_screen = Screen::ForwardPlan`)
   so the operator **reviews the plan, then sees their €200 begin** — and set
   the promote-confirmation framing flag (honesty copy, § 4).

**The ONE engine→UI boundary** is `SweepReportMirror::from_report` /
`cell_to_row` (mapping `backtest::SweptParams` → `PromoteParams`). The
**UI→agent boundary** for promotion is the existing `cockpit_live.rs` launch
dispatch — promotion maps `PromoteParams` → `agent::ForwardParamOverride` THERE
(the binary layer, which already imports `agent`), so **no `agent` engine type
crosses into `view`/`update`**. Two seams, each crossed exactly once.

**The binary-layer Launch dispatch (`cockpit_live.rs`).** Mirror the
`BakeoffRunCompleted` crowned-launch block (`cockpit_live.rs:1531-1593`):
- On a `pending_forward_promotion` being set (detected the same way
  `forward_paper_budget` is computed from `&msg`), build
  `agent::ForwardRunConfig` from `core` types **+** `param_override:
  Some(promote_params_to_override(params))` where
  `promote_params_to_override: PromoteParams → agent::ForwardParamOverride`
  (the single UI→agent map, binary-side).
- Reuse the SAME budget/FX conversion the crowned path uses (F7 / ADR-0065 —
  €200 → USDT). The promoted run is still the operator's €200.
- `try_send(ForwardCommand::Launch(fwd_cfg))` on `forward_tx`, emit
  `ForwardPaperTradeStarted(budget)` to paint the Live P/L frame — identical
  lifecycle to the crowned path.

### 3. How the tuned config reaches BOTH F6 (plan) and F5 (paper run) — the identity guard

One override (`ForwardRunConfig.param_override`) reaches both resolvers in the
supervisor's `Launch` arm:
- **F5 (paper run):** `build_registry_for(&supervisor_config, Some(&cfg))`
  (`runtime.rs:1096`) builds the registry from the override.
- **F6 (plan):** `build_forward_plan_from_registry(&supervisor_config, &cfg, …)`
  (`runtime.rs:1193`) builds the describer from the SAME override.

**The identity guard (fidelity).** For composed families, BOTH resolvers build
the in-memory TOML via the SAME shared generator and validate via
`ComposedStrategyConfig::from_str(&toml, stem)` — the identical round-trip the
sweep used to score the cell (`sweep.rs:681/712/743`). Therefore the strategy
that paper-trades == the strategy the gate scored == the plan's described rules
== byte-identical AST. The T-D8 fidelity test asserts this (§ 5).

### 4. Honesty / framing copy

All copy via `crate::strings` (zero literals), consistent with the existing
not-advice disclaimers (`TUNE_DISCLAIMER`, `TUNE_FRAGILE_PROMOTE_NOTE`).

- **Promote confirmation (the preseeded forward-plan header, on a promoted
  run).** A `TUNE_PROMOTE_CONFIRM_FMT` string, e.g.:
  > *"You tuned this {family} config ({params}). It survived resampling on
  > {window} — that is not a guarantee, and not advice. Paper-trading your €{budget}."*
  Filled from `PromoteParams.label()`, `range_label`, and the budget. Sits as a
  header strip on the forward-plan screen when the active plan came from a
  promotion (`promote_framing` flag).
- **Where it appears:** (a) the forward-plan screen header (the operator's
  review-before-watch moment), reusing the F6 plan-header slot; (b) optionally a
  one-line inline confirmation note adjacent to the now-enabled "Use this config"
  button on hover/press — but the load-bearing copy is the plan header so the
  operator reads it before the €200 starts.
- **The "you tuned this, not a bake-off winner" distinction** is explicit: the
  crowned-pick forward header (existing) says "best of the bake-off"; the
  promoted header says "you tuned this." Different provenance, different words.
- **The persistent footer (`TUNE_DISCLAIMER`) stays.** Promotion adds framing,
  never removes a disclaimer.

### 5. The day-1 wiring/divergence gate + fidelity test (CLAUDE.md non-negotiable)

Per CLAUDE.md (the v3-vol-overlay-noop precedent): promotion must NOT be a silent
no-op. Two engine-layer tests (mirror `forward_run_engine_fidelity.rs`):

- **T-D8a — divergence (proves the tuned params REACH the loop).** Build two
  `ForwardRunConfig`s for the SAME strategy id: one with
  `param_override: None` (default), one with `param_override:
  Some(Sma { fast_len, slow_len })` (tuned, **different** from the default
  `cfg.strategies.sma_crossover`). Feed the SAME bar sequence to both registries
  (via `build_registry_for`). **Assert ≥1 differing signal / divergent fill**
  (the existing behavioural-divergence pattern, `forward_run_engine_fidelity.rs`
  lines 14-17). FAILS-before (override ignored → identical output). Repeat for at
  least one composed family (MACD: default `btc_macd_trend` TOML vs a tuned
  `macd(fast',slow',signal')`).
- **T-D8b — fidelity (proves the promoted run == the scored config).** For each
  composed family, assert the agent-resolved strategy (from
  `build_registry_for` with a composed override) has the SAME `id` /
  round-trips through the SAME `from_str(stem)` as the sweep's
  `build_swept_config` produced for the identical params — i.e. the agent's
  generated TOML **byte-equals** the sweep's generated TOML for identical params
  (shared generator ⇒ trivially true, but the test PINS it so a future
  divergence fails). Reuses the ADR-0069 § D3 identity-guard pattern.
- **T-D8c — the plan reflects the tuned params.** Call
  `build_forward_plan_from_registry` with an SMA override `{fast_len: F, slow_len:
  S}` (F,S ≠ default) and assert the emitted `PlanRuleKind::SmaCross { fast_len,
  slow_len }` carries F, S (not the default 20/50). FAILS-before (plan describes
  default).

### 6. Testability + render verification

Per CLAUDE.md the cockpit UI is verified at the **rendered-PIXEL** layer
(`iced_test::Emulator::screenshot`; the font-mutex deadlock hazard is real —
follow `spec/dev-notes/iced-ui-render-verification.md`, serialize the font
mutex). Two pixel proofs + the FAIL-before logic tests:

- **Pixel proof 1 — the affordance is an ENABLED BUTTON, not a label.** Render
  the populated Tune `Ready` state (the existing `fake_sweep_report_mirror`
  fixture with a Robust/Marginal/Fragile mix). Assert at the pixel layer that a
  **promotable** row's "Use this config" reads as the enabled accent **button**
  (it now carries `on_press(Message::PromoteSweptConfig(..))`) vs the FRAGILE
  row's greyed **locked label** (the negative control — unchanged, still no
  press). This extends the existing Tune render harness; the fragile-locked
  control already has a baseline.
- **Pixel proof 2 — the forward flow preseeded with the tuned config.** After a
  simulated `PromoteSweptConfig`, render the forward-plan screen and assert the
  **tuned** IF/THEN rules + the promote-framing header are on screen (the
  tuned params, not the default — e.g. "fast 10 / slow 20", not "20 / 50"), with
  the not-advice framing visible. Negative control: the crowned-pick forward
  plan header (different provenance copy).
- **FAIL-before logic tests:** `PromoteSweptConfig` sets
  `pending_forward_promotion` + navigates to `Screen::ForwardPlan` (pure-state
  test); `promote_params_to_override` maps each family correctly; `cell_to_row`
  populates `promote_params` for every cell (incl. fragile — the data is present;
  only the *affordance* is gated). A **fragile row emits NO message** (the view
  gates `on_press` behind `promotable`) — pure-view assertion.

### 7. Scope discipline

Minimal. Promotion = **"set the forward target = this tuned config + go"**:
- REUSE the existing F5 paper loop + F6 plan machinery wholesale (no new forward
  subsystem).
- REUSE the sweep's param-construction (the shared TOML generators) — no new DSL.
- REUSE the crowned-pick launch dispatch precedent (`cockpit_live.rs`) — promotion
  is a second trigger into the SAME `ForwardCommand::Launch`.
- The ONLY new engine surface is `ForwardRunConfig.param_override` +
  `ForwardParamOverride` + the two resolver branches + the shared generator
  `pub`. The ONLY new UI surface is `PromoteParams` on the row, the
  `PromoteSweptConfig` message + handler, the binary launch trigger, and the
  framing copy.
- **Marginal stays promotable** (only Fragile is locked) — confirmed; no change
  to `promotable`.

### Crate-compatibility checklist

No new dependency. `agent` already depends on `backtest` (the generators) and
`strategy` (the resolvers). `ui` already depends on `backtest` + `agent`
(`cargo tree -p ui` UNCHANGED — `PromoteParams` is UI-owned; the UI→agent map
lives binary-side where `agent` is already imported). Single-binary / SQLite —
untouched. Edition 2024 — no new code patterns. No stdlib-shadowing crate name.

---

## Risks / unknowns

- **R1 — `SweepCellRow` gains `promote_params`.** Touching the mirror means the
  `tune/state.rs` unit tests + any `SweepCellRow` fixtures must be updated. Low
  risk (additive field), but every `SweepCellRow { .. }` literal must add it.
- **R2 — supervisor sentinel-close at Launch.** The F6 plan at Launch uses a
  sentinel close (`runtime.rs:1175-1186`); the tuned plan will read "FLAT —
  pending first bar" until a real bar lands (same as the crowned path). Honest,
  but the promote-framing copy must not imply a position exists yet.
- **R3 — anchor safety.** The `None`-override path through both resolvers MUST
  be byte-identical to today. The existing `forward_run_engine_fidelity.rs`
  tests are the regression guard; they must pass unchanged. Verify 119/119
  anchors after the engine change.
- **R4 — font-mutex deadlock** on the render tests (macOS). Follow the dev-note;
  serialize the screenshot harness.
- **R5 — provenance confusion.** A promoted run and a crowned run both drive the
  same forward loop; the framing copy is the ONLY signal of which one is live.
  The plan header must always reflect the active provenance (promote vs crown).

## Verification

_tester links to reports here_

## Changelog

- 2026-06-25 (architect): initial design + ADR-0070 — wire promotable Tune
  configs into the forward €200 paper-trade. `ForwardRunConfig.param_override`
  (agent-owned `ForwardParamOverride` enum) honored by `build_registry_for` +
  `build_forward_plan_from_registry`; UI `PromoteSweptConfig` message preseeds
  the forward launch (the crowned-pick precedent); shared sweep TOML generators
  promoted to `pub` for byte-identical fidelity; day-1 divergence + fidelity +
  plan-reflects-tuned tests; render-pixel proofs (enabled button vs fragile
  lock; preseeded tuned forward plan). Anchors 119/119.
