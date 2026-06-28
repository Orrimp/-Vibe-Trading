---
slug: advisor-forward-plan
status: in-progress
owner: architect
updated: 2026-06-21
---

# Tasks — Advisor forward buy/sell plan (F6)

Normative design: [feature.md § Design](feature.md#design) +
[ADR-0062](../../architecture/adr/0062-forward-plan-read-seam.md). The seam reuses
[ADR-0060 § D6](../../architecture/adr/0060-budget-aware-sizing-and-forward-paper-run-seam.md)
(the `ForwardCommand::Launch` / `paper_loop_supervisor` / `build_registry_for` /
`forward_rx` hot-swap) and the [ADR-0059](../../architecture/adr/0059-bakeoff-orchestrator-home-and-result-seam.md)
mirror discipline.

**Parallelism:** the **developer** track (D-tasks: trait surface + agent-side
plan production + the `core` struct + the anti-drift test) and the
**ui-designer** track (U-tasks: the plan surface + the buy-and-hold negative
control + the render PNG proof) run **in parallel** after T0. They meet at the
seam = the `core`-typed `agent::config::ForwardPlan` (D-tasks emit it; U-tasks
mirror + render it). Until D5 lands, the ui-designer works against a
`ui::fixtures` fake `ForwardPlan` (the `fake_bakeoff_report_mirror` precedent).

**Hard invariants (every task):**
- `cargo tree -p ui` UNCHANGED — no `strategy`/`exec`/`forecast`/`llm` edge added
  (the gate is T-DX.1 / T-UX.1).
- `scripts/verify_anchors.sh` → **119/119** before AND after (F6 writes no
  anchored report; run it as the first and last thing).
- **F5 stays BYTE-IDENTICAL** — no change to the `paper_loop_supervisor` /
  `spawn_trading_loop` lifecycle, no `horizon_days` self-terminate path. Headless
  `trading` bin + the soak harness pass `forward_plan_rx = None`.
- `Result<T, E>` in library code; no `.unwrap()` outside tests; no `println!`
  (use `tracing`); `cargo clippy -- -D warnings` clean.

---

## T0 — Shared baseline (do first, blocks both tracks)

- [ ] **T0.1** — Re-run `scripts/verify_anchors.sh`; confirm **119/119** as the
  pre-change baseline. — _acceptance: `ANCHORS PASS (119 / 119)` captured before
  any code lands._
- [ ] **T0.2** — Snapshot `cargo tree -p ui` to a scratch file as the
  layering-invariant baseline (the diff is the gate at T-DX.1 / T-UX.1). —
  _acceptance: baseline tree saved; it lists no `strategy`/`exec`/`forecast`/`llm`._

---

## Developer track (D) — trait surface + agent-side plan + the `core` struct

### D1 — `strategy::PlanDescribe` read-only trait (ADR-0062 § D1, § D2)

- [ ] **T-D1.1** — Add `crates/strategy/src/plan.rs`: the read-only **sibling**
  trait `PlanDescribe { fn describe_plan(&self, ctx: &PlanContext) -> StrategyPlan; }`
  (non-mutating `&self`), plus `PlanContext { last_close: Price, last_bar_ts:
  Timestamp, budget: Money<Usdt>, budget_cap: Money<Usdt> }` and the `strategy`-side
  `StrategyPlan { stance: PlanStance, latest_signal: Option<PlanSignal>, rule:
  PlanRuleShape, sizing: ProjectedSizing }` with closed enums `PlanStance{Flat,
  Long}` / `PlanSignal{Buy,Sell,Hold}` / `PlanRuleShape{SmaCross{fast_len,slow_len},
  MacdCross{fast,slow,signal}, RsiReversion{len,lower,upper}, BollingerReversion{
  len,k}, BuyAndHold}` / `ProjectedSizing{units: Quantity, capped: bool}`. **Do
  NOT** put `describe_plan` on the `Strategy` trait (ADR-0005 freeze) and **do
  NOT** emit any copy `String` (the engine emits structured data only). —
  _acceptance: `cargo check -p strategy` clean; the trait is a sibling, `Strategy`
  is untouched._
- [ ] **T-D1.2** — Implement `PlanDescribe` for `SmaCrossover`: expose the latest
  warmed `fast`/`slow` SMA values via read-only getters and derive the stance
  (`Long` iff `fast > slow + ε`, the SAME comparison `on_bar` uses), `latest_signal`
  (`Buy`/`Sell`/`Hold`), `rule = SmaCross{fast_len, slow_len}`, and `ProjectedSizing`
  = `{units: (budget/last_close).min(budget_cap/last_close), capped: budget_cap binds}`
  — **without** pushing `last_close` into the SMA (no state advance). —
  _acceptance: a unit test shows `describe_plan` does not change the indicator
  state (call it twice, identical result, no mutation) and the units honour the
  cap._
- [ ] **T-D1.3** — Implement `PlanDescribe` for the buy-and-hold degenerate case
  (`PlanRuleShape::BuyAndHold`): `stance = Long` (after the first buy),
  `latest_signal = None` (no re-evaluation), `sizing` = the FULL budget (capped).
  Implement it on whatever concrete type `build_registry_for("v0.buyhold")`
  resolves to (today the `SmaCrossover::new(1,2)` proxy — describe what the loop
  RUNS, ADR-0062 § D3). — _acceptance: a unit test asserts the buy-and-hold plan
  reads stance=Long / no sell trigger / full €200 units._
- [ ] **T-D1.4** — Implement `PlanDescribe` for the MACD/RSI/Bollinger ids **as
  they are resolved by `build_registry_for` today** — i.e. the `SmaCrossover`
  proxy (`runtime.rs:312-339`). The `describe_plan` for those ids therefore
  honestly returns `SmaCross{…}` (what the F5 loop runs), NOT a fabricated MACD/RSI
  rule. Leave a `// F5b:` note that when the dedicated forward-run ctors land,
  these gain their real `PlanRuleShape` with no F6 rework. — _acceptance: a unit
  test asserts the proxy ids describe the SMA rule the loop actually runs (the
  anti-drift guarantee at the source)._

### D2 — `agent`-side production + the `core`-typed `ForwardPlan` (ADR-0062 § D3, § D4)

- [ ] **T-D2.1** — Add `agent::config::ForwardPlan` (Clone + Debug) with the
  fields in [feature.md § Design → the `ForwardPlan` struct](feature.md#the-forwardplan-struct-the-core-typed-seam):
  `strategy: StrategyId, symbol: Symbol, stance: PlanStance, latest_signal:
  Option<PlanSignal>, rule: PlanRuleKind, last_close: Price, last_bar_ts:
  Timestamp, budget: Money<Usdt>, projected_units: Quantity, sizing_capped: bool,
  horizon_days: u16`. Define `PlanStance`/`PlanSignal`/`PlanRuleKind` as **closed
  `agent`-owned enums** (NOT re-exported from `strategy`). **Every field a `core`
  type / `agent` enum / primitive** — no `strategy`/`exec`/`forecast`/`llm` type.
  — _acceptance: `cargo check -p agent` clean; a doc-comment states the
  core-types-only invariant (mirrors `ForwardRunConfig`)._
- [ ] **T-D2.2** — Add the one-place map `strategy::StrategyPlan` →
  `agent::config::ForwardPlan` (the `BakeoffReportMirror::from_report` precedent —
  the ONLY place `StrategyPlan` is read on the `agent` side). Map `PlanRuleShape`
  → `PlanRuleKind`. — _acceptance: a unit test round-trips each `PlanRuleShape`
  variant → the matching `PlanRuleKind`._
- [ ] **T-D2.3** — Add the **agent→iced return path**: `RunHandles.forward_plan_rx:
  Option<...>`? **No** — the receiver lives on the iced side. Add a
  `mpsc::Sender<ForwardPlan>` to the `paper_loop_supervisor`'s retained context
  and a `mpsc::Receiver<ForwardPlan>` field on `RunHandles` named to make the
  direction unambiguous (e.g. `plan_tx` held by the supervisor; the cockpit holds
  the matching `Receiver`). Pattern: symmetric with the iced→agent `forward_rx`
  (ADR-0060 § D6). Headless bin + soak pass `None` → no plan produced (byte-identical).
  — _acceptance: the supervisor compiles with the new sender; `forward_rx = None`
  path is unchanged (T-D2.6 proves byte-identity)._
- [ ] **T-D2.4** — In the `paper_loop_supervisor` `ForwardCommand::Launch(cfg)`
  arm, AFTER `build_registry_for(Some(&cfg))` resolves the strategy for the
  hot-swap, **also** resolve its `&dyn PlanDescribe`, call `describe_plan(&PlanContext{
  last_close, last_bar_ts, budget: cfg.budget, budget_cap: <derived from cfg.budget> })`
  using the latest bar `(close, ts)` the loop is about to consume, map →
  `ForwardPlan` (T-D2.2), set `horizon_days` from the selection, and `send` it on
  `plan_tx`. This shares the ONE resolved registry with the F5 swap → consistency
  by construction (R7). — _acceptance: an `agent` integration test drives a
  `Launch(cfg)` and observes a `ForwardPlan` whose `strategy`/`symbol` match `cfg`
  and whose `rule` matches the resolved engine._
- [ ] **T-D2.5** — Export `ForwardPlan` + its enums from `agent::lib` (next to the
  `ForwardCommand`/`RunHandles`/`build_registry_for` re-exports). — _acceptance:
  `use agent::ForwardPlan;` resolves from a downstream crate._
- [ ] **T-D2.6** — **F5-byte-identity guard.** Add/extend an `agent` test proving
  that with `forward_plan_rx`/`plan_tx` wired but no `Launch` sent (and with
  `forward_rx = None`), the paper loop's behaviour is byte-identical to today (the
  supervisor produces no plan, runs the initial loop unchanged). — _acceptance:
  the existing paper-loop determinism guards still pass; no `horizon_days`
  self-terminate exists anywhere (grep proof)._

### D3 — the anti-drift consistency assertion (ADR-0062 § D8.2)

- [ ] **T-D3.1** — Add `crates/strategy/tests/plan_describe_matches_on_bar.rs` (or
  an `agent` test if it needs `build_registry_for`): for EACH candidate engine **as
  resolved by `build_registry_for`** (`v0.sma`, `v0.5.macd`, `v0.5.rsi`,
  `v0.5.bbands`, `v0.buyhold`), feed a fixture bar series, then assert
  `describe_plan(&ctx{last_close = last bar close, …}).stance` (and the rule
  family) **matches what `on_bar(last_bar)` actually decides** on the same bar.
  This is the honesty thesis as a falsifiable test — it would FAIL if the plan
  described rules the loop does not run. — _acceptance: the test PASSES for every
  resolved engine (including the SMA-proxy ids); deliberately mutating the
  `describe_plan` stance logic makes it FAIL (the negative control of the test
  itself)._

### D-exit gates (developer)

- [ ] **T-DX.1** — `cargo tree -p ui` UNCHANGED vs the T0.2 baseline (no
  `strategy`/`exec`/`forecast`/`llm` edge leaked through `ForwardPlan`). —
  _acceptance: empty diff._
- [ ] **T-DX.2** — `scripts/verify_anchors.sh` → **119/119**. — _acceptance:
  `ANCHORS PASS (119 / 119)`._
- [ ] **T-DX.3** — `cargo clippy --workspace -- -D warnings` clean; `cargo test
  -p strategy -p agent` green. — _acceptance: both clean._

---

## UI-designer track (U) — the plan surface + the negative control + the PNG proof

> Works against `ui::fixtures` fake `ForwardPlan` values until D2 lands. Owns the
> plain-language COPY + the not-a-prediction / not-advice disclaimers (no engine
> string crosses the seam). The central UX call is **OQ-D — make the conditional,
> reactive, rule-driven nature unmistakable** (IF/THEN framing, a dated "current
> stance" badge, sizing labelled "at the last close", disclaimers integral to the
> layout — NOT a forecast-looking timeline). IA placement is **OQ-F** (recommend a
> section appended to the Leaderboard screen, OR a distinct pre-launch step
> between Leaderboard and Live).

### U1 — the `ui`-side mirror + copy

- [ ] **T-U1.1** — Add the `ui`-side `ForwardPlanView` mirror of
  `agent::config::ForwardPlan` (closed `ui` enums + render-ready shape), built
  via a single `from_plan(&ForwardPlan)` — the `BakeoffReportMirror::from_report`
  precedent (the ONLY place the `agent` `ForwardPlan` is read in `ui`). —
  _acceptance: pure + total `from_plan`; a unit test maps each `PlanRuleKind` →
  the matching `ui` rule discriminant._
- [ ] **T-U1.2** — Add the plan COPY to `ui::strings`: the IF/THEN rule sentences
  per `PlanRuleKind` (e.g. SMA → "Buys when the fast SMA crosses above the slow
  SMA, sells on the reverse cross"; BuyAndHold → "Buy now, hold the whole horizon
  — no sell trigger"), the stance badge labels, the sizing line ("€200 ≈ 200 USDT
  (FX not modelled); on the next BUY it would deploy ~{units} at the last close
  ${last_close} — never more than €200"), and the **mandatory disclaimers** ("This
  is a conditional, rule-based plan — NOT a price prediction or implied return" +
  the standing not-financial-advice + simulated-budget line, product D5). —
  _acceptance: every `PlanRuleKind` + both stances have copy; the two disclaimers
  are present and not behind a fold._
- [ ] **T-U1.3** — Add `ui::fixtures::fake_forward_plan()` (active SMA pick: FLAT
  or LONG, SMA rule, €200 sizing) and `fake_forward_plan_buy_and_hold()` (the
  degenerate plan) — the `fake_bakeoff_report_mirror` / `_benchmark_wins`
  precedent. — _acceptance: both fixtures construct a populated `ForwardPlanView`._

### U2 — the screen surface (OQ-D / OQ-F)

- [ ] **T-U2.1** — Build the plan surface (`crates/ui/src/screens/` near
  `leaderboard.rs`) rendering `ForwardPlanView`: the dated **current-stance badge**
  (FLAT/LONG + the bar close + timestamp for honest staleness), the **IF/THEN
  standing rules**, the **projected €200 sizing** number, the **horizon** framing
  ("planned through {date}; rules in force + checked each bar"), and the
  disclaimers — presented so it reads as conditional, NOT a forecast (OQ-D). —
  _acceptance: the surface renders both fixtures; the buy-and-hold and active
  plans read as obviously the same KIND of object._
- [ ] **T-U2.2** — Wire the IA placement (OQ-F): append to the Leaderboard screen
  (or a pre-launch step before Live). On `BakeoffRunCompleted(Ok(mirror))` with a
  crowned row, the cockpit already sends `ForwardCommand::Launch` (ADR-0060 § D6);
  add the receive of the returning `ForwardPlan` (D2's channel) via an iced
  subscription/recipe (the Live-PnL-recipe pattern) and feed it to the surface. —
  _acceptance: the plan surface populates from the real returning `ForwardPlan`
  when the runtime is wired, and from the fixture in headless render tests._
- [ ] **T-U2.3** — Add the **empty / no-pick** state (no crowned row yet → the
  surface shows a "run a bake-off to see the plan" prompt, no plan table). —
  _acceptance: the empty state paints no plan rule text / no sizing number._

### U3 — the render-layer PNG proof (R9 / ADR-0062 § D8.1 — THE verification floor)

- [ ] **T-U3.1** — Add `crates/ui/tests/forward_plan_populated_render.rs`, modelled
  on `crates/ui/tests/leaderboard_populated_render.rs` (the `iced_test::screenshot`
  real-renderer + a `forward_plan_screen_program` test-support helper +
  `#![cfg(target_os = "macos")]` per ADR-0057 § D2). Three guards:
  - **(a) populated active-strategy plan** — assert the rendered PIXELS show the
    stance badge + the IF/THEN rule text + the €200 projected-sizing NUMBER paint
    (measurable foreground text + the sizing digits), writing the operator-facing
    PNG to `/tmp/forward_plan_populated_render.png`.
  - **(b) buy-and-hold degenerate plan = the NEGATIVE CONTROL** — the SAME harness
    with `fake_forward_plan_buy_and_hold()` paints the "buy now, hold, no sell
    trigger" plan and visibly DIFFERS from (a) (e.g. no sell-rule line), proving
    (a) is not a tautology.
  - **(c) empty / no-pick state** — paints no plan table (the second negative
    control; proves the populated guard discriminates against the empty prompt).
  — _acceptance: all three guards pass on macOS; the PNGs are eyeball-correct; a
  passing proxy (model state / text snap / no-panic boot) is explicitly NOT used._

### U-exit gates (ui-designer)

- [ ] **T-UX.1** — `cargo tree -p ui` UNCHANGED vs the T0.2 baseline. —
  _acceptance: empty diff (the surface reads `ForwardPlanView` only — no
  `strategy`/`exec`/`forecast`/`llm` edge)._
- [ ] **T-UX.2** — `scripts/verify_anchors.sh` → **119/119**. — _acceptance:
  `ANCHORS PASS (119 / 119)`._
- [ ] **T-UX.3** — `cargo clippy -p ui -- -D warnings` clean. — _acceptance: clean._

---

## Tester contract (handed to the tester — ADR-0062 § D8)

Stated explicitly so the tester does NOT expect the gates that do not apply here:

- **NO new anchored backtest scenario.** F6 is a read-only descriptive surface →
  `scripts/verify_anchors.sh` MUST stay **119/119** (the developer/ui-designer
  touch zero report files). The tester confirms 119/119.
- **The CLAUDE.md day-1 baseline-equity-divergence e2e gate is N/A.** That gate is
  for a strategy overlay or sizing modifier; it landed on F4 (ADR-0060 § D2). F6
  *describes* sizing — it sizes/runs nothing — so there is no equity path to
  diverge. Do not author or expect it here.
- **Verification FLOOR = the render-layer PNG** (`forward_plan_populated_render.rs`,
  T-U3.1): a populated active-strategy plan (stance + IF/THEN rules + €200 sizing)
  **plus the buy-and-hold degenerate plan as the negative control** (plus the
  no-pick empty tautology guard). Read the PNG — a passing proxy is not proof the
  screen draws (the Live-view-saga precedent).
- **Anti-drift consistency assertion** (`plan_describe_matches_on_bar.rs`, T-D3.1):
  the plan's described action MATCHES the engine's actual `on_bar` decision on the
  same bar, for each resolved engine — the honesty thesis as a falsifiable test.
- **Layering gate:** `cargo tree -p ui` unchanged.
- **F5-byte-identity:** confirm no change to the `paper_loop_supervisor` /
  `spawn_trading_loop` lifecycle and no `horizon_days` self-terminate path.

## Notes

- The biggest correctness subtlety is **what `build_registry_for` resolves to**:
  MACD/RSI/Bollinger/buy-hold are SMA proxies for the forward run today
  (`runtime.rs:288-348`). `describe_plan` MUST describe the resolved engine (the
  SMA proxy), not the Lab-time strategy — otherwise the plan asserts rules the F5
  loop does not run (the exact drift the OQ-A fallback was rejected for). T-D1.4 +
  T-D3.1 are the guards.
- When the F5b dedicated MACD/RSI/Bollinger forward-run ctors land (ADR-0060 OQ),
  `describe_plan` automatically describes the real engine and `PlanRuleShape`'s
  reserved variants light up — no F6 rework.
