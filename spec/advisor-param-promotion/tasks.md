---
slug: advisor-param-promotion
status: in-progress
owner: developer
updated: 2026-06-25
---

# Tasks — advisor-param-promotion (ADR-0070)

Ordered so the **engine seam lands first** (the extraction + override), then the
agent resolvers honor it, then the UI wires the affordance, then the gates.
`(dev)` = developer (engine/agent/binary), `(ui)` = ui-designer (view/copy),
`(both)` = coordinated. The day-1 divergence + fidelity tests (T6) are the
CLAUDE.md non-negotiable and must FAIL-before.

## Phase A — the engine extraction (shared TOML, the first dev task)

- [ ] **T1 (dev)** — Promote the sweep's composed-TOML generators to `pub`.
  `macd_toml` / `rsi_toml` / `bbands_toml` in
  `crates/backtest/src/bakeoff/sweep.rs:570-611` → `pub fn` (same bodies; the
  sweep keeps calling them). Re-export from `backtest` so `agent` can call them.
  _acceptance: `backtest::…::{macd_toml,rsi_toml,bbands_toml}` callable from
  `agent`; sweep behaviour unchanged (existing sweep tests pass)._

- [ ] **T2 (dev)** — Round-trip pin test for the generators (extends ADR-0069
  § D3): each generator's output parses via
  `ComposedStrategyConfig::from_str(&toml, stem)` and the parsed `id` == stem.
  _acceptance: a test asserts all three generators round-trip; it lives next to
  the generators so a future edit that breaks the DSL fails here._

## Phase B — `ForwardRunConfig` carries the override

- [ ] **T3 (dev)** — Add `ForwardParamOverride` (agent-owned closed enum:
  `Sma{fast_len,slow_len}` / `Macd{fast,slow,signal}` / `Rsi{period,oversold}` /
  `Bollinger{period,k_tenths}`) to `crates/agent/src/config.rs`, and add
  `param_override: Option<ForwardParamOverride>` to `ForwardRunConfig`
  (`config.rs:32-42`). Update every `ForwardRunConfig { .. }` literal (search:
  `cockpit_live.rs:1562`, `forward_run_engine_fidelity.rs:36`, plan tests) to
  add `param_override: None` (preserves the existing crowned-pick path,
  byte-identical / anchor-safe).
  _acceptance: workspace compiles; existing forward tests pass with
  `param_override: None`._

- [ ] **T4 (dev)** — `build_registry_for` honors the override.
  `crates/agent/src/runtime.rs:331-475` — when `fwd.param_override == Some(..)`,
  resolve from the override (SMA → `SmaCrossover::new(tuned)`; composed → shared
  generator → `from_str(stem)` → `ComposedStrategy::from_config`, synthetic
  `source_path` e.g. `"tuned:<stem>"`); when `None`, the existing match arms run
  **unchanged**. No new disk file.
  _acceptance: `None` path byte-identical (fidelity tests unchanged); `Some` path
  registers the tuned strategy._

- [ ] **T5 (dev)** — `build_forward_plan_from_registry` honors the SAME
  override. `crates/agent/src/plan.rs:190-268` — `Some` builds the describer from
  the override (SMA `SmaCrossover::new(tuned)`; composed `from_str` describer
  from the SAME generated TOML); `None` unchanged. Confirms F6 plan describes the
  tuned rules (the `PlanRuleKind` → `PlanRuleView` mirror does the rest).
  _acceptance: with an SMA override the emitted `PlanRuleKind::SmaCross` carries
  the tuned lens; `None` unchanged._

## Phase C — the day-1 gates (CLAUDE.md non-negotiable; must FAIL-before)

- [ ] **T6 (dev)** — Divergence + fidelity + plan-reflects-tuned tests
  (new `crates/agent/tests/forward_promotion_divergence.rs`, mirroring
  `forward_run_engine_fidelity.rs`):
  - **T6a divergence** — same id, `None` vs `Some(tuned)` (SMA + ≥1 composed),
    SAME bars → ≥1 differing signal/fill. FAILS-before.
  - **T6b fidelity** — agent's generated composed TOML byte-equals the sweep's
    `build_swept_config` TOML for identical params; resolved `id` == stem.
  - **T6c plan-reflects-tuned** — `build_forward_plan_from_registry` with an SMA
    override emits the tuned `PlanRuleKind::SmaCross`, not 20/50.
  _acceptance: all three pass; each demonstrably fails if the override is
  ignored (verify by temporarily stubbing the override path)._

## Phase D — the UI carrier + message (the ONE boundary)

- [ ] **T7 (ui)** — Add `PromoteParams` (UI-side closed enum mirroring the four
  families; `k` as `k_tenths: u32`) to `crates/ui/src/tune/state.rs`. Add
  `promote_params: PromoteParams` to `SweepCellRow`. Populate it in `cell_to_row`
  (`tune/state.rs:191`) from `cell.params` (`backtest::SweptParams`) — keeping
  `from_report` the ONLY place `SweptParams` is read. Update the `tune/state.rs`
  unit tests + `SweepCellRow` fixtures.
  _acceptance: every `SweepCellRow` carries structured params; the ONE-boundary
  invariant holds (no `SweptParams` in `view`/`update`)._

- [ ] **T8 (ui)** — Add `Message::PromoteSweptConfig(PromoteParams)` to
  `crate::state::Message` (near the other `Sweep*` variants, `state.rs:2266`).
  Add the `update` arm + `promote_swept_config(model, params)` helper (mirror
  `open_strategy_in_lab`, `state.rs:2385`): preseed
  `pending_forward_promotion: Option<ForwardPromotion>` (new `Cockpit` field),
  set `current_screen = Screen::ForwardPlan`, set the `promote_framing` flag.
  Pure (navigate + preseed; NO bin-layer Task).
  _acceptance: pure-state test — message sets the preseed + navigates; no engine
  type in `update`._

## Phase E — wire the affordance + the binary launch

- [ ] **T9 (ui)** — Wire `use_config_cell` (`crates/ui/src/screens/tune.rs:971-1019`):
  the promotable branch becomes a real `Button` with
  `on_press(Message::PromoteSweptConfig(cell.promote_params.clone()))`; the
  FRAGILE branch stays the greyed locked label (NO press — unchanged). Remove the
  "WIRING is out of scope" comment.
  _acceptance: promotable row = pressable accent button; fragile row = unchanged
  locked label._

- [ ] **T10 (dev)** — Binary launch dispatch. In `crates/ui/src/bin/cockpit_live.rs`
  (mirror the `BakeoffRunCompleted` crowned-launch block, `1531-1593`): when
  `pending_forward_promotion` is set, build `agent::ForwardRunConfig` from `core`
  types + `param_override: Some(promote_params_to_override(params))` (the single
  UI→agent map, binary-side), reuse the F7/ADR-0065 €200→USDT conversion,
  `try_send(ForwardCommand::Launch(fwd_cfg))`, emit `ForwardPaperTradeStarted`.
  Clear `pending_forward_promotion` after dispatch.
  _acceptance: pressing "Use this config" launches a forward run whose registry
  is the tuned strategy (observable via the divergent equity / the plan)._

## Phase F — honesty copy + render-pixel verification

- [ ] **T11 (ui)** — Framing copy in `crate::strings` (zero literals):
  `TUNE_PROMOTE_CONFIRM_FMT` ("You tuned this {family} config ({params}). It
  survived resampling on {window} — not a guarantee, not advice. Paper-trading
  your €{budget}."). Render it as the forward-plan header when `promote_framing`
  is set (distinct from the crowned-pick "best of the bake-off" provenance).
  Keep `TUNE_DISCLAIMER`.
  _acceptance: promoted forward plan shows the "you tuned this" framing; crowned
  forward plan shows its existing provenance; both keep the not-advice line._

- [ ] **T12 (ui)** — Render-pixel proofs (extend the Tune + forward-plan render
  harnesses; serialize the font mutex per
  `spec/dev-notes/iced-ui-render-verification.md`):
  - **Proof 1** — populated Tune `Ready`: a promotable row's "Use this config"
    renders as the ENABLED accent button; the FRAGILE row's locked label is the
    negative control (unchanged).
  - **Proof 2** — post-promote forward-plan: the TUNED IF/THEN rules (tuned
    params, e.g. "fast 10 / slow 20") + the promote-framing header are on screen;
    negative control = the crowned-pick header.
  _acceptance: both PNGs read correctly; the proofs would fail if the affordance
  were still a label or the plan described default params._

## Notes

- **Order rationale:** T1-T6 (engine + gates) before any UI so the divergence
  gate exists before the affordance is wired — the affordance cannot ship a
  silent no-op.
- **Anchor safety:** run `bash scripts/verify_anchors.sh` after Phase B/C →
  expect **119/119** (the `None`-override path is byte-identical).
- **Fidelity is structural:** F5 (registry) and F6 (plan) both build from the
  SAME override via the SAME generator + the SAME `from_str` identity guard, so
  "what paper-trades == what the gate scored == what the plan describes" holds by
  construction (ADR-0070 § D2/D3).
- **Marginal stays promotable** — only Fragile is locked. No change to
  `promotable`.
