---
slug: lumen-phase-5-humancontrol-agentfeed
status: shipped
owner: architect
updated: 2026-05-07
version: 2.4.0
---

# Lumen design adoption — Phase 5: HumanControl + AgentFeed rename

> **Phase 5 of 6** in the
> [`lumen-design-adoption`](lumen-design-adoption.md) initiative. Master
> roadmap is the orientation; this brief is the **shippable feature**.
> Operator-locked constraints (no brand, no voice rewrite, sequential
> phases, Phase 6 reserved) are documented in the master file and apply
> here without re-litigation.
>
> **Status: active.** Was originally Phase 3 in the pre-2026-05-04
> roadmap; renumbered to Phase 5 at the 2026-05-04 master-roadmap
> revision. The 2026-05-04 stub (110 lines, queued status, scope outline
> only) is **superseded by this expansion**. The Why section is preserved
> verbatim and extended with the "first net-new operator-write paths"
> framing and the TD-1 tightening point; high-level scope is replaced by
> R-cluster-pointing summary; open questions are replaced by the
> architect Q-items below.

## Why

Phase 1 ships a kill-only operator surface; Phases 2 + 3 give the
operator a richer view of the system but no richer write surface.
Phase 4 ships the offline backtest viewer, also read-only. Phase 5
closes that gap by adopting the Lumen `HumanControl.jsx` pattern (kill
+ execution-mode toggle + per-strategy pause/override) and aligning
vocabulary by renaming the `tape` widget to `AgentFeed`.

### Phase 5 is the first net-new operator-write surface

Phases 1–4 were uniformly **read surfaces and design infrastructure**
(Phase 1 tokens; Phase 2 sidebar IA + chart over read-only audit;
Phase 3 detail screens over read-only backend data; Phase 4 offline
viewer + read-only equity query). The kill switch is the only
operator-write surface in the shipped tree and predates this
initiative
([`crates/ui/src/widgets/kill.rs:140–155`](../../crates/ui/src/widgets/kill.rs)).

**Phase 5 introduces three new operator-write paths:**

1. **Execution-mode toggle** — Observe (paper-only) / Supervised
   (per-decision approval) / Auto (within-envelope autonomy). Maps
   onto the v2 LLM strategy gate (master Phase 6 forward-compat).
   Phase 5 ships only the UI as a runtime flag on `Cockpit`. **Not
   destructive** — gates future trades, doesn't reverse past ones;
   no typed-confirm needed
   ([`HumanControl.jsx:11–32`](../design/project/ui_kits/desktop/HumanControl.jsx)).
2. **Pause-strategy** — operator pauses a single strategy without
   halting the agent. Per-strategy button. **Bounded destructive**
   (a paused strategy skips signals it would otherwise take); but
   reversible — the principles doc's "undo where possible" rule
   ([`spec/ui-design-principles.md:275–278`](../ui-design-principles.md))
   recommends single-click pause + single-click resume.
3. **Override-risk-veto** — operator overrides a risk-engine veto.
   **Maximally destructive** — the veto exists precisely to stop
   the order. The principles doc reserves `OVERRIDE` for this
   surface
   ([`spec/ui-design-principles.md:268`](../ui-design-principles.md)).

### Plus the AgentFeed rename — vocabulary alignment

[`crates/ui/src/widgets/tape.rs`](../../crates/ui/src/widgets/tape.rs)
(147 lines) renders the live fills panel. Lumen's
[`AgentFeed.jsx`](../design/project/ui_kits/desktop/AgentFeed.jsx)
is the operator vocabulary alignment for the same surface. **Visual
upgrade is deferred** — Phase 5 is module-level rename only; the
per-event sparkline / agent column / event-kind icons stay
out-of-scope so the rename ripple is isolated and reviewable.

### TD-1 tightening point — Phase 5 is load-bearing

The master roadmap's TD-1 row (true keyboard-focus ring) has been
restated four phases in a row. Verified at this kickoff:
[`crates/ui/Cargo.toml:69`](../../crates/ui/Cargo.toml) still pins
`iced = "=0.14.0"` (line was 52 in pre-Phase-4 architect notes; the
viewer-bin block shifted it to 69, pin unchanged). Neither named
upgrade trigger has fired.

The cost/benefit tightens materially at Phase 5: the kill switch
already has typed-confirm gating, but Phase 5 adds **three** new
operator-write surfaces. Override-risk-veto needs typed-confirm
(`OVERRIDE`); pause-strategy is single-click but is one of multiple
write buttons co-resident on the Strategies-detail screen, where
focus discrimination is exactly what the missing `Focused` variant
provides. **Phase 5 must resolve this**, not restate it. R13 + Q5
carry the closure; analyst recommendation rejects a fifth
restatement.

### What Phase 5 ships

- **`HumanControl` panel widget** at
  `crates/ui/src/widgets/human_control.rs` — execution-mode
  segmented control + daily-loss-limit / max-position / used-today
  P&L mirror rows + the existing kill button as the bottom action.
  Placement = Q1.
- **Per-strategy controls** on the Strategies-detail screen
  (Phase 3 surface) — pause button (single-click toggle per Q8) +
  override-risk-veto button (typed-confirm with the `OVERRIDE`
  phrase per the principles doc).
- **`Message` extensions** — `ExecutionModeSelected(Mode)`,
  `StrategyPauseToggled(StrategyId)`, plus the
  `OverrideRiskVeto*` family mirroring the kill-confirm flow.
- **`Cockpit` model extensions** — `execution_mode: ExecutionMode`
  (default Observe), `paused_strategies: HashSet<StrategyId>`,
  `override_risk_veto: OverrideRiskVetoState`, plus a placeholder
  `risk_veto_events: Vec<VetoEvent>` (Q13 — risk-engine emit
  upstream stubbed today).
- **Two new audit writers** (Q2 / Q3) — `strategy_paused` and
  `risk_veto_overridden` event_kinds, sibling of the existing
  `kill_switch_tripped` pattern
  ([`crates/audit/src/journal.rs:316–407`](../../crates/audit/src/journal.rs)).
  Schema unchanged (`strategy_events.kind` is `TEXT`).
- **Module rename** — `tape.rs` → `agent_feed.rs`; mod-decl + import
  ripple + the 9 `tape_*.snap` baseline filename renames (Q6).
- **TD-1 resolution** — R13 + Q5 close the four-phase deferral.

### What Phase 5 does NOT ship — load-bearing

- **No AgentFeed visual upgrade.** Per-event sparkline / agent
  column / event-kind icons deferred; rendered fills byte-
  identical to Phase 4 modulo snapshot filename rename.
- **No execution-mode persistence** (Q4 — runtime-only).
- **No risk-engine veto-emit upstream** in Phase 5. Phase 5 ships
  the operator-side override surface over a placeholder feed; the
  agent-side veto-emit is a Phase-N+ task (Q13).
- **No new audit migration.** The two new writers extend the
  `kind` enum at the application layer; the column is `TEXT`
  ([`crates/audit/migrations/002_strategy_events.sql`](../../crates/audit/migrations/002_strategy_events.sql)).
- **No anchor budget.** Audit writer additions are additive; no
  committed report body re-renders.
- **No `cockpit_live` chrome change** beyond HumanControl
  placement + the per-strategy buttons.

### Why now

Phase 4 shipped 2026-05-06 / approved 2026-05-06. Phase 5 inherits
Phase 1's typed-confirm modal pattern + kill widget; Phase 2's
sidebar (HumanControl placement Q1); Phase 3's `RiskTelemetry`
channel + thread-isolation rule
([`crates/agent/src/runtime.rs:1023–1090`](../../crates/agent/src/runtime.rs))
for the new pause/override bus channels; Phase 4's
`EquitySeries` if a HumanControl mini-card needs it. Two write
surfaces (pause + override) share audit-writer plumbing — landing
them together is the cheapest moment to extend
`StrategyEventKind`.

## Scope (high-level)

The full R-item list is below. High-level grouping:

- **R1–R3 HumanControl panel widget** — placement (Q1), execution-
  mode segmented control, mirrored limits + used-today P&L, kill as
  bottom action.
- **R4–R6 Pause-strategy control** — per-strategy button on the
  Strategies-detail screen + Home → Strategies-summary panel,
  single-click toggle (Q8), audit writer (Q2).
- **R7–R8 Override-risk-veto control** — per-veto button (Q9),
  typed-confirm flow with `OVERRIDE` phrase, audit writer (Q3).
- **R9–R10 Execution-mode toggle** — three-mode segmented control,
  runtime-only persistence (Q4).
- **R11–R12 `tape` → `AgentFeed` rename** — module rename, import
  ripple, snapshot filename rename (Q6), consistency-test fixture
  ripple.
- **R13 TD-1 resolution** — one of fold-iced-upgrade /
  custom-widget escape hatch / final-restatement-with-Phase-6-
  deadline; analyst recommends + architect ratifies at Q5.
- **R14 Cross-feature invariants preservation** — every prior
  shipped feature still passes.
- **R15 Anchor regression** — 11/11 byte-identical; new audit
  writers are additive.

## Anchor risk

**Zero by default.** Phase 5 is a UI rename + new HumanControl panel
+ new operator-write surfaces. The two new audit writers
(`strategy_paused`, `risk_veto_overridden`) are **additive** —
they extend the `strategy_events.kind` enum
([`crates/core/src/strategy_events.rs:99–113`](../../crates/core/src/strategy_events.rs))
with two new PascalCase variants and add two new sibling functions
in [`crates/audit/src/journal.rs`](../../crates/audit/src/journal.rs)
that follow the `kill_switch_tripped` pattern verbatim. No existing
row's body is altered; no committed report body re-renders; no
backtest scenario is affected. The 11/11 backtest body-SHA-256
anchor regression goal is preserved by construction.

If at any point in design or implementation a path is proposed that
*does* touch a committed report body, the architect must **stop and
surface as a Q-item** — that path is out of scope for Phase 5 and
must be re-litigated.

## Snapshot ripple

Two ripples land in Phase 5: the rename ripple (existing tape
baselines) and the net-new HumanControl + per-strategy-control
baselines.

**Rename ripple** (Q6 — rename rather than keep stale filenames):

| Before                                              | After                                                     |
|-----------------------------------------------------|-----------------------------------------------------------|
| `panel_snapshots__tape_loading.snap`                | `panel_snapshots__agent_feed_loading.snap`                |
| `panel_snapshots__tape_empty.snap`                  | `panel_snapshots__agent_feed_empty.snap`                  |
| `panel_snapshots__tape_error.snap`                  | `panel_snapshots__agent_feed_error.snap`                  |
| `panel_snapshots__tape_ready_three_fills.snap`      | `panel_snapshots__agent_feed_ready_three_fills.snap`      |
| `panel_snapshots__tape_paused.snap`                 | `panel_snapshots__agent_feed_paused.snap`                 |
| `panel_snapshots__tape_audit_modal_*.snap`          | `panel_snapshots__agent_feed_audit_modal_*.snap` (×4)     |

**Net-new** (Q11 estimate — ~8–12):

1. `panel_snapshots__human_control__observe_default.snap` —
   Observe mode active, kill idle, mirror fields populated.
2. `panel_snapshots__human_control__supervised_active.snap` —
   Supervised mode active.
3. `panel_snapshots__human_control__auto_active.snap` — Auto
   mode active.
4. `panel_snapshots__human_control__kill_armed.snap` — kill
   button hovered (typed-confirm gate per Phase 1 R7 inherits).
5. `panel_snapshots__strategies_screen__pause_button_idle.snap`
   — strategy row with pause button visible.
6. `panel_snapshots__strategies_screen__pause_button_paused.snap`
   — strategy row showing paused indicator.
7. `panel_snapshots__strategies_screen__override_button_idle.snap`
   — surfaced veto event with override button visible.
8. `panel_snapshots__strategies_screen__override_confirm_modal.snap`
   — `OVERRIDE` typed-confirm modal open, mismatch state.
9. `panel_snapshots__strategies_screen__override_confirm_modal_matched.snap`
   — `OVERRIDE` typed-confirm modal, phrase matched, confirm
   enabled.
10. `panel_snapshots__home_screen__with_human_control.snap` —
    Home screen reflecting Q1 placement decision (regenerates if
    Q1 picks Home-card placement; otherwise Home byte-identical).
11. `panel_snapshots__debug_screen__without_kill.snap` — Debug
    screen with kill removed (regenerates only if Q1 picks
    HumanControl-as-sidebar-entry, since kill migrates).

Phase 1 / 2 / 3 / 4 baselines stay byte-identical except the
9 tape rename ripples and (depending on Q1) up to 2 home/debug
re-renders. Single `cargo insta accept` pass at end of phase per
Phase 1 Q2 / Phase 2 V11 / Phase 3 V12 / Phase 4 V12 precedent.

## Requirements

Numbered, testable, derived from
[`spec/design/project/ui_kits/desktop/HumanControl.jsx`](../design/project/ui_kits/desktop/HumanControl.jsx),
[`spec/design/project/ui_kits/desktop/AgentFeed.jsx`](../design/project/ui_kits/desktop/AgentFeed.jsx),
the
[Phase 1 typed-confirm precedent](lumen-phase-1-foundation.md),
the
[Phase 3 RiskTelemetry channel](lumen-phase-3-detail-screens.md),
the existing
[`crates/ui/src/widgets/kill.rs`](../../crates/ui/src/widgets/kill.rs),
[`crates/ui/src/widgets/tape.rs`](../../crates/ui/src/widgets/tape.rs),
[`crates/ui/src/state.rs`](../../crates/ui/src/state.rs),
[`crates/audit/src/journal.rs`](../../crates/audit/src/journal.rs),
[`crates/core/src/strategy_events.rs`](../../crates/core/src/strategy_events.rs),
and
[`spec/architecture.md` § Frontend](../architecture.md). Each ends
with a one-line **acceptance** the tester can verify. Operator-locked
constraints inherited from the
[master roadmap](lumen-design-adoption.md) (no brand, no voice
rewrite, sequential phases, Q11–Q14) apply throughout.

### R1 — HumanControl panel widget scaffold

- **R1.1** New file `crates/ui/src/widgets/human_control.rs`. Public
  `pub fn view(model: &Cockpit) -> Element<'_, Message>` framed by
  `widgets::frame::panel` Tier-1 chrome; renders the four sub-blocks
  (R9 mode + R3 limits + R2 kill).
- **R1.2** Panel title `"You're in control"` and sub-title
  `"Human-in-the-loop"` per
  [`HumanControl.jsx:9`](../design/project/ui_kits/desktop/HumanControl.jsx)
  via new `PANEL_HUMAN_CONTROL_TITLE` / `PANEL_HUMAN_CONTROL_META`
  constants.
- **R1.3** Placement TBD at Q1; analyst recommends 7th sidebar
  entry. Widget code is placement-agnostic.
- **R1.4** Both bins consume; fixtures-mode renders with the Lumen
  reference's static placeholder limits so baselines stay
  deterministic.
- **R1.5** No new theme tokens — Phase 1 palette covers all
  surfaces.
- **Acceptance:** `panel_snapshots__human_control__observe_default.snap`
  PASS.

### R2 — Kill action as the panel's bottom button

- **R2.1** Phase 1 kill button moves into HumanControl as the
  bottom action per
  [`HumanControl.jsx:49–51`](../design/project/ui_kits/desktop/HumanControl.jsx)
  via direct `widgets::kill::view(model)` call — no flow
  duplication.
- **R2.2** Debug-screen kill placement preserved if Q1 picks
  sidebar-entry. If Q1 picks Home-card or footer, Debug-screen
  kill becomes redundant; architect resolves at Q1.
- **R2.3** Kill widget public surface unchanged
  ([`crates/ui/src/widgets/kill.rs:52, 227`](../../crates/ui/src/widgets/kill.rs)).
- **R2.4** `KILL_BUTTON_LABEL` copy preserved per Q12 / Master
  Constraint 2 (no voice rewrite). Lumen `"Halt all agents"`
  cited but not adopted.
- **Acceptance:** typed-confirm behaviour byte-identical to Phase
  1; `panel_snapshots__human_control__kill_armed.snap` PASS.

### R3 — Mirrored limits and used-today P&L

- **R3.1** Three label-value rows above the kill button per
  [`HumanControl.jsx:34–47`](../design/project/ui_kits/desktop/HumanControl.jsx):
  `Daily loss limit` / `Max position` / `Used today`.
- **R3.2** Daily-loss reads `risk_state.daily_loss_cap_pct`; max
  position derives from `risk_state.per_symbol_caps`; used-today
  reads from `Cockpit::pnl`. `RiskState` is the Phase 3
  `PanelState<RiskState>` mirror
  ([`crates/ui/src/state.rs:563`](../../crates/ui/src/state.rs)).
- **R3.3** Used-today coloured `UP_500` / `DOWN_500` by sign per
  P&L-colouring rule
  ([`spec/ui-design-principles.md:344–357`](../ui-design-principles.md)).
- **R3.4** Loading → three muted `—` dashes; Error →
  `frame::muted_body(strings::HUMAN_CONTROL_LIMITS_UNAVAILABLE)`.
- **R3.5** New `strings::*` constants are net-new copy, not edits
  to existing `ui::strings` — Master Constraint 2 not violated.
- **Acceptance:** three rows render with sentiment colours; loading
  + error baselines covered.

### R4 — Pause-strategy control: per-strategy button

- **R4.1** Per-row pause button on the Strategies-detail screen
  rows (Phase 3 R5); rendered via a new helper in
  `widgets::strategies` adding a trailing column.
- **R4.2** Label toggles per row state — `STRATEGY_PAUSE_LABEL`
  (`"Pause"`) running / `STRATEGY_RESUME_LABEL` (`"Resume"`)
  paused.
- **R4.3** Click emits `Message::StrategyPauseToggled(StrategyId)`;
  pure `update` flips `Cockpit::paused_strategies:
  HashSet<StrategyId>` set membership (sibling of `tape_paused`
  at [`state.rs:455`](../../crates/ui/src/state.rs)).
- **R4.4** Single-click both directions (Q8). No typed-confirm.
- **R4.5** Home → Strategies-summary panel reuses the same
  per-row button via the shared widget helper.
- **R4.6** Live mode adds bus channel `pause_strategy_tx:
  broadcast::Sender<(StrategyId, bool)>` (sibling of kill-switch
  closure pattern, [`architecture.md:319–324`](../architecture.md));
  agent runtime's registry subscribes and skips `on_bar` /
  `on_tick` for paused strategies. Fixtures mode is UI-only.
- **Acceptance:** click toggles label both directions; baselines
  cover both states.

### R5 — Pause-strategy persistence: audit writer

- **R5.1** New `audit::journal::strategy_paused(ledger,
  strategy_id, paused: bool, operator)` in
  [`crates/audit/src/journal.rs`](../../crates/audit/src/journal.rs).
  Sibling of `kill_switch_tripped` (line 316–407).
- **R5.2** New `StrategyEventKind::StrategyPaused` variant at
  [`strategy_events.rs:99`](../../crates/core/src/strategy_events.rs);
  no payload; pause direction encodes in `error_summary`
  (`"paused"` / `"resumed"`).
- **R5.3** Atomic dual-write — same SQL shape as
  `kill_switch_tripped` (memo + `strategy_events` row in one
  txn). Memo description: `"strategy:StrategyPaused:<id>:<paused|resumed>"`.
- **R5.4** No migration — `strategy_events.kind` is already
  `TEXT`
  ([002_strategy_events.sql](../../crates/audit/migrations/002_strategy_events.sql)).
- **R5.5** Live cockpit-side wiring — bus channel from R4.6
  drives a separate audit-write task on the agent runtime
  thread; fixtures mode skips. Pure `update` preserved.
- **Acceptance:** `cargo test -p audit
  journal::tests::strategy_paused_*` PASSES (4 tests:
  pauses-emit-PascalCase-kind, balanced-memo-row, atomic-dual-
  write, resumes-flip-error_summary).

### R6 — Pause-strategy resume semantics

- **R6.1** Resume = default state; single-click; no
  typed-confirm (Q8).
- **R6.2** Resume emits a separate `StrategyPaused` audit row
  with `error_summary = "resumed"` — `strategy_events` carries
  the full pause→resume timeline.
- **R6.3** Cold-start: `paused_strategies` is empty (session-
  scoped per Phase 3 Q5 precedent).
- **Acceptance:** pause-then-resume produces two audit rows
  with same `strategy_id`, opposite `error_summary` values.

### R7 — Override-risk-veto control: per-veto button

- **R7.1** Each surfaced veto event in
  `Cockpit::risk_veto_events: Vec<VetoEvent>` (R7.2) renders an
  `Override` button.
- **R7.2** New field, sibling of `risk_state` (Phase 3).
  `VetoEvent = (veto_id: SmolStr, ts, strategy_id, reason:
  SmolStr, blocked_signal: Signal)`. **Live upstream stubbed in
  Phase 5** (Q13) — fixtures populate; live emits empty `Vec`.
  Risk-engine veto-emit integration is Phase-N+ work; Phase 5
  ships the override surface over the placeholder so the typed-
  confirm flow + audit writer are testable.
- **R7.3** **Per-veto override** (Q9). One button per surfaced
  event, not per strategy. **Forward-only** — the veto is
  dismissed + audit row recorded; the agent does NOT re-emit the
  blocked signal.
- **R7.4** Click opens typed-confirm modal with `"OVERRIDE"`
  phrase per
  [principles doc table](../ui-design-principles.md#confirm-destructive-actions).
- **R7.5** New widget `crates/ui/src/widgets/override_risk_veto.rs`
  reusing the kill-confirm visual contract (sunken input,
  mismatch hint, confirm-disabled-until-matched) — separate
  widget keeps each destructive state machine local.
- **Acceptance:**
  `panel_snapshots__strategies_screen__override_confirm_modal{,_matched}.snap`
  PASS.

### R8 — Override-risk-veto persistence: audit writer

- **R8.1** New `audit::journal::risk_veto_overridden(ledger,
  veto_id, strategy_id, reason, operator)`. Sibling of
  `kill_switch_tripped`.
- **R8.2** New `StrategyEventKind::RiskVetoOverridden` variant
  ([`strategy_events.rs`](../../crates/core/src/strategy_events.rs)).
- **R8.3** Atomic dual-write per `kill_switch_tripped`. Memo
  desc: `"strategy:RiskVetoOverridden:<id>:<reason>"`;
  `error_code = "risk_veto_overridden"`; `error_summary` carries
  reason verbatim.
- **R8.4** No migration (R5.4).
- **R8.5** Cockpit wiring — `OverrideRiskVetoConfirmed` arm
  spawns audit-write task + clears the matching `VetoEvent` from
  `risk_veto_events`. Pure `update` preserved.
- **Acceptance:** `cargo test -p audit
  journal::tests::risk_veto_overridden_*` PASSES (3 tests:
  emits-PascalCase-kind, balanced-memo-row,
  reason-preserved-in-error_summary).

### R9 — Execution-mode toggle widget

- **R9.1** New `pub enum ExecutionMode { Observe, Supervised,
  Auto }` in `core::views`; `Default = Observe` (Q4 — safest
  cold-start).
- **R9.2** Three-option segmented control per
  [`HumanControl.jsx:14–32`](../design/project/ui_kits/desktop/HumanControl.jsx);
  active mode highlighted via Phase 1 active-row pattern.
- **R9.3** Click emits
  `Message::ExecutionModeSelected(ExecutionMode)`; pure `update`
  arm flips `Cockpit::execution_mode`.
- **R9.4** Three new mode-hint constants
  (`EXECUTION_MODE_{OBSERVE,SUPERVISED,AUTO}_HINT`) per
  [`HumanControl.jsx:27–31`](../design/project/ui_kits/desktop/HumanControl.jsx).
- **R9.5** No typed-confirm — mode gates *future* trades only;
  not destructive in the typed-confirm sense (master roadmap
  operator-impact bound).
- **Acceptance:** click flips active highlight + hint copy;
  three baselines.

### R10 — Execution-mode persistence

- **R10.1** Runtime-only for v1 (Q4). Cold-start = Observe. No
  `config/agent.toml` write.
- **R10.2** No audit writer — mode is prospective, not a
  decision.
- **R10.3** Live plumbing — bus channel `execution_mode_tx:
  broadcast::Sender<ExecutionMode>`; strategy registry's
  `on_bar` / `on_tick` consults active mode before forwarding
  signals. Phase 5 ships the channel + cockpit emit; executor-
  side gating is Phase-N+ (gated on v2 LLM). Observe short-
  circuits at the executor (already shipped paper-only).
- **R10.4** Fixtures cold-start = Observe; no consumer.
- **Acceptance:** toggle flips active highlight; restart returns
  to Observe.

### R11 — `tape` → `AgentFeed` module rename

- **R11.1** `git mv crates/ui/src/widgets/tape.rs
  crates/ui/src/widgets/agent_feed.rs`; module doc-comment
  retitles to "Live agent activity feed".
- **R11.2** [`widgets/mod.rs:28`](../../crates/ui/src/widgets/mod.rs)
  `pub mod tape;` → `pub mod agent_feed;`.
- **R11.3** Import-site ripple — `use crate::widgets::tape` →
  `agent_feed` across `state.rs`, `bin/cockpit.rs`,
  `bin/cockpit_live.rs`, `crates/ui/tests/`.
- **R11.4** **`Cockpit` field name preserved** — `pub tape:
  PanelState<...>` stays (Q14). Field rename would ripple
  through ~100+ tests for cosmetic value; module renames, state
  doesn't.
- **R11.5** Strings — `PANEL_TAPE_TITLE` →
  `PANEL_AGENT_FEED_TITLE` (`"Agent activity"` per
  [`AgentFeed.jsx:71`](../design/project/ui_kits/desktop/AgentFeed.jsx)).
  Net-new constant; not a Master-Constraint-2 voice-rewrite
  edit.
- **R11.6** No visual change beyond title copy.
- **Acceptance:** `cargo build -p ui` zero errors; consistency
  test scans `widgets/agent_feed.rs` clean.

### R12 — Snapshot baseline + consistency-test fixture rename

- **R12.1** Nine `panel_snapshots__tape_*.snap` files rename to
  `agent_feed_*` (5 panel-states + 4 audit-modal variants) per
  Q6.
- **R12.2** Consistency test globs `*.rs` under `src/widgets/`
  ([`tests/consistency.rs:24–33`](../../crates/ui/tests/consistency.rs))
  — rename auto-picks up; no test-code update needed.
- **R12.3** Title-string change (R11.5) is the only body-content
  diff; rest is filename-only.
- **R12.4** Single `cargo insta accept` pass at phase end.
- **Acceptance:** `git status` after accept shows 9 rename pairs
  + ~10 net-new + zero stale `tape_*.snap`.

### R13 — TD-1 resolution (load-bearing)

- **R13.1** Pre-implementation gate: at Q5 ratification, verify
  iced version one final time. If iced 0.15+ has shipped with
  `button::Status::Focused` + `text_input::Style.shadow`, take
  R13.2; else R13.3.
- **R13.2** **Path (a) — fold iced 0.15+ upgrade.** Bump
  `crates/ui/Cargo.toml:69`; sweep `widgets/kill.rs` (lines
  64–83 + 113–155 — three button styles + input),
  `widgets/journal_transaction_modal.rs` (one button), and add
  focus-ring wiring to new `human_control.rs` (R1) and
  `override_risk_veto.rs` (R7.5). Master roadmap scopes the
  follow-up as ~30 lines net. **All four destructive surfaces
  ship with true keyboard-focus rings.**
- **R13.3** **Path (b) — custom-widget escape hatch.** Project-
  local `iced::widget::Component` owns focus state via
  `Subscription` on `keyboard::Event::KeyPressed { key: Tab }`,
  emits `FocusChanged(WidgetId)` Message, re-renders with halo.
  Wire into every destructive button + input. Heavier path;
  Phase 5 is two phases past the master-roadmap promotion
  threshold ("if iced upstream stalls past Phase 3").
- **R13.4** **Path (c) — final restatement, hard Phase 6
  deadline.** Phase 6 is gated on v2 LLM (which may take
  quarters); analyst-recommended **NOT (c)** but listed for
  honesty.
- **R13.5** Tester verifies: (a) / (b) — focus halo on
  `Tab`-focused destructive button + input; (c) — fifth TD-1
  restatement row in master roadmap with explicit Phase 6
  deadline language; tester REJECTS a (c) entry that omits
  the deadline.
- **Acceptance:** Q5 ratifies; tester gate verifies the chosen
  state.

### R14 — Cross-feature invariants preservation

- **R14.1** `operator-success-reports` — latency-badge surface
  unchanged.
- **R14.2** `live-cockpit-unified` — `cockpit_live` launches
  unchanged; halted-banner triggers preserved; new
  `pause_strategy_tx` + `execution_mode_tx` channels are
  additive on `EventBus`.
- **R14.3** `real-mtm-unrealized-pnl` — P&L card unchanged
  (R3 reads but does not mutate).
- **R14.4** `per-symbol-position-accounts` — unchanged.
- **R14.5** `tape-row-audit-modal` — modal trigger / frame
  unchanged; widget renamed but modal trigger source path
  byte-identical (R11.4 preserves `Cockpit::tape` field name).
- **R14.6** `journal-tx-metadata` — unchanged.
- **R14.7** `v1.5b-multi-venue` — unchanged; new audit writers
  do not consume the venue column.
- **Acceptance:** 7/7 invariant table PASS.

### R15 — Anchor regression (11/11 byte-identical)

- **R15.1** `verify-anchors` PASS — 11/11 byte-identical.
- **R15.2** Two new audit writers are additive — new
  `StrategyEventKind` enum variants + sibling functions; no
  existing row body changes; no committed report body
  re-renders.
- **R15.3** No new backtest scenarios; no re-anchor budget.
- **R15.4** Any path proposed that re-renders a committed
  report body is **out of scope** — surface as Q-item.
- **Acceptance:** `verify-anchors` 11/11 PASS.

## Verification (V-items)

Numbered, each with a precise test command + expected output.

- **V1 — HumanControl renders default.** `cargo test -p ui
  human_control_observe_default_baseline` PASS; baseline
  carries four sub-blocks (mode + 3 limits + kill). (R1, R2, R3.)
- **V2 — Mode toggle round-trips.** `cargo test -p ui
  execution_mode_toggle_round_trips` PASS — Observe →
  Supervised → Auto → Observe; three mode baselines PASS.
  (R9, R10.)
- **V3 — Limits read from `RiskState` + `pnl`.** `cargo test -p
  ui human_control_limits_render` PASS — fixture
  `daily_loss_cap_pct = 5%` renders correctly; used-today
  reads `Cockpit::pnl` with sign colouring; loading + error
  baselines covered. (R3.)
- **V4 — Pause toggles state.** `cargo test -p ui
  pause_strategy_button_toggles` PASS; two baselines
  (`pause_button_{idle,paused}`). (R4, R6.)
- **V5 — Pause audit writer.** `cargo test -p audit
  journal::tests::strategy_paused_*` PASS (4 tests). (R5.)
- **V6 — Override typed-confirm flow.** `cargo test -p ui
  override_risk_veto_typed_confirm` PASS — pressed → modal
  open → phrase mismatch hint → "OVERRIDE" matches → confirm
  enabled → confirmed clears `VetoEvent`; two baselines
  (`override_confirm_modal{,_matched}`). (R7.)
- **V7 — Override audit writer.** `cargo test -p audit
  journal::tests::risk_veto_overridden_*` PASS (3 tests). (R8.)
- **V8 — Module rename.** `cargo build -p ui` zero errors;
  `cargo test -p ui --tests
  no_inline_user_visible_strings_in_widgets` PASS scanning
  `widgets/agent_feed.rs`; zero stale `tape_*.snap` files.
  (R11, R12.)
- **V9 — `StrategyEventKind` PascalCase.** `cargo test -p
  trading-core strategy_events::tests::pascal_case_for_new_variants`
  PASS — `StrategyPaused` / `RiskVetoOverridden` serialise
  per existing contract
  ([`strategy_events.rs:120–135`](../../crates/core/src/strategy_events.rs)).
  (R5.2, R8.2.)
- **V10 — TD-1 path verified.** Per Q5 ratification:
  - Path (a): `Cargo.toml` carries `iced = "=0.15.x"`;
    `cargo test -p ui keyboard_focus_ring_visible_on_*`
    PASS for all four destructive surfaces.
  - Path (b): `widgets/focus_component.rs` ships; `cargo
    test -p ui focus_component_*` PASS.
  - Path (c): master roadmap TD-1 row carries fifth
    restatement **with explicit Phase 6 deadline
    language**. Tester REJECTS (c) entry without deadline.
    (R13.)
- **V11 — Cross-feature invariants.** 7/7 PASS. (R14.)
- **V12 — Snapshot baselines.** Single `cargo insta accept`;
  ~9 rename pairs + ~8–12 net-new + ≤2 Q1-driven re-renders.
  (R12, R14.)
- **V13 — Anchor regression.** `verify-anchors` 11/11 PASS.
  (R15.)
- **V14 — `rust-validate` PASS.** `cargo fmt` / `clippy -D
  warnings` / `deny check` / `audit` all PASS. New widgets
  inherit lints; `ExecutionMode` derives `Debug + Clone +
  Copy + PartialEq + Eq` per `AgentMode`
  ([`state.rs:89–96`](../../crates/ui/src/state.rs)).
- **V15 — `rust-build` PASS.** `cargo build -p ui --bins` /
  `audit` / `trading-core` all PASS.

## Acceptance criteria

Phase 5 ships when all of the following hold:

- **HumanControl panel reachable** in both bins
  (`cockpit` fixtures + `cockpit_live`); placement per
  Q1 ratification. (R1, R2, R3.)
- **Execution-mode toggle round-trips** via the
  `Message::ExecutionModeSelected` handler; three modes
  reflected in the active highlight + hint copy. Cold-
  start on Observe; restart resets to Observe. (R9, R10.)
- **Per-strategy pause control** flips a single strategy's
  pause flag in/out of `paused_strategies`; single-click
  in both directions; audit writer emits `StrategyPaused`
  rows for both pause and resume. (R4, R5, R6.)
- **Per-veto override-risk-veto control** opens the
  typed-confirm modal with `OVERRIDE` phrase; confirm
  disabled until matched; on confirm, audit writer emits
  a `RiskVetoOverridden` row + clears the
  `VetoEvent` from `risk_veto_events`. (R7, R8.)
- **`tape` → `AgentFeed` module rename** clean — zero
  `widgets::tape` imports remain; consistency test
  reads `widgets/agent_feed.rs`; snapshot files
  rename per Q6. (R11, R12.)
- **TD-1 resolved** per Q5 ratification — paths (a) /
  (b) / (c). Path (c) requires explicit Phase 6
  deadline language. (R13.)
- **Cross-feature invariants PASS** (7/7) and **11/11
  anchor regression PASS** (byte-identical bodies).
  (R14, R15.)
- **`rust-validate` + `rust-build` PASS.** Single
  `cargo insta accept` pass for ~9 rename pairs +
  ~8–12 net-new baselines. (V12, V14, V15.)

## Open questions for architect

Q11–Q14 from the master roadmap are **operator-locked**
and not opened here:

- **Master Q11** — sidebar fixed-width: relevant if
  Phase 5 picks the 7th-sidebar-entry placement (Q1);
  fixed-width is preserved.
- **Master Q12** — chart data both modes: irrelevant
  to Phase 5 (HumanControl has no chart).
- **Master Q13** — extend `audit::query` for read
  additions: relevant if HumanControl needs a read
  surface for "recent operator writes" (Q-item Q15
  below — analyst recommends NOT adding this in Phase
  5).
- **Master Q14** — Phase 2/3 split kept: past gate.

The questions below are the genuinely-open design
choices that ratify at architect kickoff. Each ends
with a one-line **analyst recommendation** and one-
line alternatives considered.

### Q1 — HumanControl panel placement

**Question:** R1.3 — HumanControl lives as **(a)** a 7th sidebar
entry (`Home / Debug / Charts / Strategies / Risk / Audit /
Control`), **(b)** a Home-screen header card, or **(c)** a footer
panel above the status bar?

**Recommended: (a) — 7th sidebar entry.** Consistency with the
Phase 2/3 IA (every surface is a sidebar entry); panel is too tall
for footer (~6–8 rows); Home-card breaks the four-panel grid +
hides kill behind a click. Lumen reference describes HumanControl
as "always visible"
([`HumanControl.jsx:2`](../design/project/ui_kits/desktop/HumanControl.jsx)).

**Alternatives:** (b) Home-card — breaks grid; (c) footer — too
tall for status-bar-adjacent slot.

### Q2 — Pause-strategy persistence

**Question:** R4.6 / R5 — runtime-only on
`Cockpit::paused_strategies`, or new `strategy_paused` audit
writer?

**Recommended: new audit writer.** Operator decisions belong in
the ledger; sibling of `kill_switch_tripped`
([`crates/audit/src/journal.rs:316–407`](../../crates/audit/src/journal.rs)).
Pause-resume timeline reconstructible from `strategy_events`.

**Alternatives:** runtime-only — leaves no audit trail; violates
"audit ledger is the canonical why" rule
([`spec/ui-design-principles.md:282–284`](../ui-design-principles.md)).

### Q3 — Override-risk-veto persistence

**Question:** R7 / R8 — same as Q2 for the risk-veto override.

**Recommended: new `risk_veto_overridden` audit writer.**
Compliance — overriding a risk veto is the kind of decision
regulators and the operator's future self look back on. Reason
preserved verbatim in `error_summary`.

**Alternatives:** runtime-only — rejected, compliance-bounded.

### Q4 — Execution-mode persistence

**Question:** R10 — runtime-only flag (cold-start = Observe), or
persisted to `config/agent.toml` on change?

**Recommended: runtime-only for v1.** Shipped tree has zero
config-write surfaces (v0–v4 are config-driven); introducing one
for session ergonomics is out of bounds. Cold-start on **Observe**
is the safest default.

**Alternatives:** config-write — bisects "config-driven, no
UI-write-to-disk" non-goal; corruption-on-crash risk.

### Q5 — TD-1 resolution (load-bearing)

**Question:** R13 — path (a) iced 0.15+ fold-in / (b) custom-widget
escape hatch / (c) final restatement with hard Phase 6 deadline?

**Recommended:** Architect verifies iced version on disk at Q5
ratification (current pin
[`crates/ui/Cargo.toml:69`](../../crates/ui/Cargo.toml) is
`iced = "=0.14.0"`). **If iced 0.15+ has shipped**, commit to
**(a)** — fold-in. **If not**, commit to **(b)** — custom-widget
escape hatch. **(c) is analyst-rejected** — Phase 6 is gated on v2
LLM (which may take quarters); a fifth restatement-with-deadline
is operationally indefinite. Phase 5 ships three new write
surfaces; restating is no longer viable.

**Alternatives:** (c) — rejected.

### Q6 — `tape` → `AgentFeed` snapshot rename scope

**Question:** R12.1 — rename `panel_snapshots__tape_*.snap` →
`panel_snapshots__agent_feed_*.snap`, or keep filenames stable?

**Recommended: rename.** Snapshot filenames are operator-
greppable; stale-filename → new-module mismatch breaks the
greppability future operators rely on. The rename is one-time and
mechanical.

**Alternatives:** keep stable — greppability mismatch.

### Q7 — HumanControl panel field set

**Question:** R3 — full Lumen set (mode + 3 limits + kill), or
trimmed?

**Recommended: full Lumen set per
[`HumanControl.jsx:6–55`](../design/project/ui_kits/desktop/HumanControl.jsx).**
All three limit fields read from existing `Cockpit::risk_state` +
`Cockpit::pnl`; no new backend wiring needed.

**Alternatives:** trimmed (mode + kill only) — hides daily-loss-
limit context the principles doc calls out as load-bearing.

### Q8 — Pause-strategy resume semantics

**Question:** R4.4 / R6 — typed-confirm to resume a paused
strategy too, or single-click resume?

**Recommended: single-click resume.** Pausing requires conscious
action; resume returns to default state, principles-doc "undo
where physically possible" case
([`spec/ui-design-principles.md:275–278`](../ui-design-principles.md)).
Pause is bounded-destructive (skips future signals; doesn't
reverse past decisions) so the typed-confirm gate is even less
load-bearing on the pause side.

**Alternatives:** typed-confirm both sides — friction without
proportional safety value.

### Q9 — Override-risk-veto scope

**Question:** R7.3 — per-veto override (one button per surfaced
veto event) or per-strategy override (one flag-setting button)?

**Recommended: per-veto override.** Operator must consciously
override each veto; per-strategy is too broad — "disable risk-
engine for this strategy" is exactly what the engine exists to
prevent. Each override is its own typed-confirm flow per the
principles doc.

**Alternatives:** per-strategy — too broad; loses per-decision
audit trail.

### Q10 — Audit-writer test scope

**Question:** R5 / R8 — unit tests only, or unit + integration +
audit-row snapshot baseline?

**Recommended: all three.** Unit tests cover the dual-write
contract (sibling of `kill_switch_tripped` tests); integration
tests cover cockpit → bus → writer wiring; snapshot baseline
locks the audit row format
(`strategy_events__risk_veto_overridden_row.snap`).

**Alternatives:** unit only — leaves cockpit-side wiring
untested.

### Q11 — Snapshot baseline budget

**Question:** R12 / V12 — how many net-new baselines, single-pass
or staged refresh?

**Recommended: ~8–12 net-new + ~9 rename pairs; single `cargo
insta accept` at end of phase.** Phase 1 Q2 / Phase 2 V11 / Phase
3 V12 / Phase 4 V12 precedent. Phase 1–4 baselines byte-identical
except rename ripple + (Q1-dependent) up to 2 home/debug re-
renders.

**Alternatives:** staged review — three passes for tightly
coupled visual work is overhead without value.

### Q12 — Kill button copy in HumanControl

**Question:** R2.4 — adopt Lumen `"Halt all agents"`
([`HumanControl.jsx:50`](../design/project/ui_kits/desktop/HumanControl.jsx))
or preserve shipped `"Stop trading"`
([`crates/ui/src/strings.rs`](../../crates/ui/src/strings.rs)
`KILL_BUTTON_LABEL`)?

**Recommended: preserve `"Stop trading"`.** Master Constraint 2
(no voice rewrite) + principles doc "exact phrase not negotiable
mid-session"
([`spec/ui-design-principles.md:391–393`](../ui-design-principles.md)).

**Alternatives:** adopt Lumen copy — rejected by Master
Constraint 2.

### Q13 — Risk-engine veto-emit wiring

**Question:** R7.2 — wire the upstream risk-engine veto-emit in
Phase 5, or land the override surface over a placeholder feed?

**Recommended: placeholder feed.** Risk-engine veto-emit is its
own non-trivial backend task (`VetoEvent` shape; emit point in
strategy → risk → executor pipeline; persistence question).
Phase 5 ships the cockpit-side flow (typed-confirm + audit writer
+ clear-from-list) over `Vec<VetoEvent>` populated by fixtures.
The real risk-engine emit is Phase-N+ work; Phase 5's surface is
fully testable today because the typed-confirm modal is operator-
driven.

**Alternatives:** wire full pipeline — couples Phase 5 ship to a
larger backend refactor; Phase 5's in-scope is the operator-facing
override surface.

### Q14 — `Cockpit::tape` field rename

**Question:** R11.4 — rename the cockpit state field too
(`Cockpit::tape` → `Cockpit::agent_feed`)?

**Recommended: preserve `Cockpit::tape` field name.** Field is
referenced by every Phase 1–4 test fixture and the
[`tape-row-audit-modal`](tape-row-audit-modal.md) modal-trigger
import path; rename would ripple through ~100+ test sites for
cosmetic value. Phase 5 is module rename, not state-shape rename.
Mismatch documented in the widget's module doc-comment.

**Alternatives:** rename the field — disproportionate test
ripple; out of Phase 5 scope.

### Q15 — Audit-query reader for "recent operator writes"

**Question:** master Q13 (extend `audit::query` for read
additions) — does Phase 5 add a "recent operator actions" reader?

**Recommended: NO — defer.** New `StrategyPaused` /
`RiskVetoOverridden` rows are already queryable via Phase 3's
`recent_journal_filtered` (with `kind` filtering); Phase 3's
Audit screen is the canonical surface for `strategy_events`. A
dedicated "recent operator activity" panel is a separate future
brief.

**Alternatives:** add a reader — scope creep.

## Backlog updates

Effective on this brief's promotion (2026-05-06):

### Active

- **`lumen-phase-5-humancontrol-agentfeed`** — this brief,
  expanded from stub status (110-line, queued, scope outline
  only) to active. Status: `active`. Owner: analyst. Pipeline
  next stage: **architect**.

### Queue (unchanged from master roadmap)

- **`lumen-phase-6-assistant-slot`** — reserved, linked to v2
  LLM. No analyst spawn until v2 LLM is approved.

### Recent (shipped)

- **`lumen-phase-4-backtest-panel`** — shipped 2026-05-06.
- **`lumen-phase-3-detail-screens`** — shipped 2026-05-05 /
  approved 2026-05-06.
- **`lumen-phase-2-shell-ia-charts`** — shipped 2026-05-05.
- **`lumen-phase-1-foundation`** — shipped 2026-05-04.

### Stub supersede note

The 2026-05-04 stub of this brief (110 lines, queued status,
high-level scope only) is **superseded by this expansion**.
The Why section is preserved verbatim and extended with the
"first net-new operator-write paths" framing + the TD-1
tightening point; Scope (high-level) is replaced by the
R-cluster-pointing summary; Open questions are replaced by
the architect Q-items below; Acceptance criteria are extended
to trace each bullet to its R-cluster. Master roadmap
reference unchanged: see
[`lumen-design-adoption.md` Phase 5 section](lumen-design-adoption.md).

## Cross-phase technical-debt — TD-1 keyboard focus ring

**TD-1 status check at Phase 5 analyst kickoff (2026-05-06).**
Verified [`crates/ui/Cargo.toml:69`](../../crates/ui/Cargo.toml)
still pins `iced = "=0.14.0"` (line number shifted 52 → 69 in
Phase 4 when the viewer bin block landed; pin unchanged). Neither
named upgrade trigger has fired.

**Phase 5 is the tightening point.** Three new operator-write
surfaces (execution-mode toggle + pause-strategy + override-risk-
veto), the first phase since the initiative began to ship any.
Deferral restated four phases in a row; analyst recommendation
rejects a fifth restatement (Q5 / R13).

**R13 is the closure R-item.** Architect picks (a) iced 0.15+
fold-in if available or (b) custom-widget escape hatch if not.
The tester gate verifies the chosen path at V10.

**Operator-impact bound** unchanged from master roadmap: the
override-risk-veto destructive flow is `OVERRIDE`-typed-confirm
gated; focus halo is a secondary signal. Pause is single-click
(Q8); execution-mode is not destructive (Q4). The deviation
ships a bounded ergonomic gap, not a safety gap; Phase 5 closes
the ergonomic gap regardless.

## Changelog

- 2026-05-06 (architect, Phase 5 design): appended `## Design`. **15 /
  15 architect Q-items ratified, zero principled overrides.** Q1
  HumanControl placement = 7th sidebar entry (Lumen "always-visible"
  framing + Phase 2 / 3 IA consistency); Q2 / Q3 new audit writers
  `audit::journal::strategy_paused` + `audit::journal::risk_veto_overridden`
  (sibling of `kill_switch_tripped` at `crates/audit/src/journal.rs:316–407`);
  Q4 execution-mode runtime-only persistence (cold-start = `Observe`);
  **Q5 / TD-1 = path (b) custom-widget escape hatch** — verified at
  design pass `crates/ui/Cargo.toml:69` still pins `iced = "=0.14.0"`,
  iced 0.15+ has not landed, fold-in unavailable; restate-with-deadline
  rejected (Phase 6 v2-LLM gated, operationally indefinite); Phase 5 is
  the operator-write-surface sharpening point so a fifth restatement
  is no longer viable; commits to a new `crates/ui/src/widgets/focus_ring.rs`
  Subscription-driven `Component` wrapper around all four destructive
  surfaces (kill button + kill confirm input + override-risk-veto
  confirm + per-strategy pause + execution-mode segments). Q6 snapshot
  rename via `git mv` (preserves history; body diff = title-string
  only); Q7 full Lumen field set (mode + 3 limits + kill); Q8 single-
  click pause-resume; Q9 per-veto override (forward-only — agent does
  not re-emit blocked signal); Q10 unit + integration + audit-row
  snapshot baseline (all three); Q11 ~9 rename + ~10 net-new + 1 Q1-
  driven Debug regen, single `cargo insta accept` pass; Q12 preserve
  `"Stop trading"` (Master Constraint 2); **Q13 placeholder feed for
  risk-engine veto-emit; deferred upstream wiring tracked as new TD-2
  row** (architect flags for orchestrator to append to master
  roadmap's Cross-phase technical-debt section); Q14 preserve
  `Cockpit::tape` field name (rename ripples through ~100+ test sites
  for cosmetic value) — annotated via code-comment pointing at
  `widgets/agent_feed.rs`; Q15 NO new audit-query reader. Cockpit
  state diff specified — three new fields (`execution_mode`,
  `paused_strategies`, `override_risk_veto`, `risk_veto_events`),
  one new enum (`ExecutionMode`), one new state machine
  (`OverrideRiskVetoState`), six new `Message` variants
  (`ExecutionModeSelected`, `StrategyPauseToggled`, +4 `OverrideRiskVeto*`).
  HumanControl panel widget + per-strategy pause control + override-
  risk-veto control + execution-mode toggle + audit writer additions
  + `tape` → `AgentFeed` rename + TD-1 resolution (path b) +
  risk-engine veto-emit deferral all carry concrete contracts. Cross-
  feature invariants table re-stated (7 rows). **Zero anchor risk
  re-affirmed** — additive `StrategyEventKind` variants
  (`StrategyPaused`, `RiskVetoOverridden`); no schema migration
  (`kind` is `TEXT`); no committed report body re-renders. Snapshot
  ripple: 9 rename pairs (via `git mv`) + ~10 net-new + 1 Q1-driven
  Debug regen + 1 focus-ring net-new; single `cargo insta accept`
  pass per Phase 1 Q2 / Phase 2 V11 / Phase 3 V12 / Phase 4 V12
  precedent. Implementation parallelism map: T1901 foundation gate
  → fan-out across T1902 / T1903 / T1904 / T1909 / T1912 → T1905 /
  T1906 / T1911 share HumanControl skeleton + focus-ring → T1907 /
  T1908 / T1910 share audit writers + focus-ring → narrow at T1913
  snapshot accept → T1914–T1916 → T_FINAL. Task list at
  [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/tasks.md`](../tasks/lumen-phase-5-humancontrol-agentfeed.md)
  with 16 T19xx tasks + tester `T_FINAL_LUMEN_PHASE_5` gate. Brief
  status `active`; owner bumped `analyst` → `architect`. HANDOFF →
  developer ‖ ui-designer (developer takes T1901–T1916 implementation;
  ui-designer takes the visual-diff attestation sub-block at T1913 /
  T_FINAL after the developer's snapshot refresh pass).
- 2026-05-06 (analyst, Phase 5 kickoff expansion): expanded the
  2026-05-04 stub into the full analyst brief — 15 R-items in 7
  clusters (R1–R3 HumanControl panel; R4–R6 pause-strategy +
  audit writer; R7–R8 override-risk-veto + audit writer; R9–R10
  execution-mode toggle + persistence; R11–R12 `tape` →
  `AgentFeed` rename + snapshot ripple; R13 TD-1 resolution;
  R14–R15 invariants + anchors), 15 V-items, 8 acceptance
  criteria, 15 architect Q-items (placement, pause/override/mode
  persistence, TD-1 resolution, rename scope, panel field set,
  resume semantics, override scope, test scope, baseline budget,
  kill copy, risk-engine wiring, field rename, audit-query
  reader). Master Q11–Q14 inherited (Q11 fixed-width preserved
  if Q1 picks sidebar; Q12 / Q14 irrelevant or past gate; Q13
  recommended NOT-add-in-Phase-5 per Q15). **Phase 5 framed as
  the first phase to introduce net-new operator-write paths**
  (execution-mode toggle + pause-strategy + override-risk-veto).
  **TD-1 verified** — iced still pins `=0.14.0` at
  `crates/ui/Cargo.toml:69`; deferral restated four phases in a
  row; Phase 5 carries R13 + Q5 as the closure point; analyst
  recommends fold-in (a) if iced 0.15+ shipped, escape hatch (b)
  otherwise; restate-with-deadline (c) named for honesty but
  analyst-rejected. **Anchor risk zero by default** — UI rename
  + new widget + additive `StrategyEventKind` variants
  (`StrategyPaused`, `RiskVetoOverridden`) following the
  `kill_switch_tripped` sibling pattern at
  `crates/audit/src/journal.rs:316–407`; no schema migration
  (column is `TEXT`); no committed report body re-rendered.
  Snapshot ripple: ~9 rename pairs (`tape_*` → `agent_feed_*`) +
  ~8–12 net-new; single `cargo insta accept` pass per Phase 1 Q2
  / Phase 2 V11 / Phase 3 V12 / Phase 4 V12 precedent. Brief
  status `queued` → `active`; owner unchanged. HANDOFF →
  architect.
- 2026-05-04 (analyst, master-roadmap revision): stub created at
  the 6-phase roadmap revision. Replaces the Phase 3 sketch in
  the pre-revision master roadmap. Renumbered Phase 3 → Phase 5.
  Full brief expansion deferred to Phase 5 kickoff per master Q3.

## Design

_Architect-owned. Resolves Q1–Q15 — every recommendation lands as
**ratified** unless flagged "Architect override". The analyst sections
above are immutable; this section is the design contract the developer
reads alongside the task list at
[`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/tasks.md`](../tasks/lumen-phase-5-humancontrol-agentfeed.md)._

### Q-item resolutions

All 15 architect Q-items resolved. **15 / 15 ratified, zero deviations
from analyst recommendation on substance.** Q5 (TD-1) carries a
load-bearing concrete commitment grounded in the on-disk iced version
verification below. Each row cites the R-item(s) it ratifies. Phase 5
inherits more upstream primitives than any prior phase (Phase 1 tokens
+ kill widget + typed-confirm modal contract; Phase 2 sidebar IA + 7th
entry plumbing; Phase 3 `RiskTelemetry` channel + `PanelState` + the
in-binary `Task::perform` shim; Phase 4 `EquitySeries` mirror + the
sibling-of-existing audit-writer pattern), so the resolutions are
short and lean on "sibling of …" framing throughout.

| Q   | Question                                              | Resolution                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                                              | Ratifies        |
|-----|-------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------|
| Q1  | HumanControl panel placement                          | **(a) — 7th sidebar entry.** Ratified per analyst. Consistency with the Phase 2 / 3 IA (every cockpit surface is a sidebar entry) + Lumen's "always visible" framing. The Phase 2 R1.6 sidebar widget API is parameterised; absorbing a 7th entry is an additive `.entries.push(SidebarEntry { id: "control", … })` in the binary's sidebar build. Implication: the existing Debug-screen kill placement migrates into HumanControl as the bottom action (R2.1); Debug-screen kill row retires (one extra snapshot regenerates per R12.1's row 11 budget). Home-screen header card rejected (breaks the four-panel grid); footer rejected (panel is ~6–8 rows tall, exceeds status-bar-adjacent slot). | R1.3, R2.2      |
| Q2  | Pause-strategy persistence                            | **New `audit::journal::strategy_paused` writer.** Ratified per analyst. Operator decisions belong in the ledger; sibling of `kill_switch_tripped` ([`crates/audit/src/journal.rs:316–407`](../../crates/audit/src/journal.rs)). Pause / resume timeline is reconstructible from `strategy_events` rows. Runtime-only rejected — leaves no audit trail; bisects the principles-doc "audit ledger is the canonical why" rule. Exact signature in the **Audit writer additions** sub-section below.                                                                                  | R5.1–R5.5       |
| Q3  | Override-risk-veto persistence                        | **New `audit::journal::risk_veto_overridden` writer.** Ratified per analyst. Compliance-bounded — overriding a risk veto is exactly the kind of decision regulators and the operator's future self look back on. Reason preserved verbatim in `error_summary`. Exact signature in **Audit writer additions** below. Runtime-only rejected — compliance-bounded.                                                                                                                                                                                                                  | R8.1–R8.5       |
| Q4  | Execution-mode persistence                            | **Runtime-only for v1.** Ratified per analyst. Cold-start = `Observe` (safest default; v0–v4 are config-driven, so introducing a UI-write-to-disk surface for session ergonomics is out of bounds). No `config/agent.toml` write; no audit writer (mode is prospective, not a decision). `config/agent.toml` write rejected — bisects the "config-driven, no UI-write-to-disk" non-goal; corruption-on-crash risk.                                                                                                                                                              | R10.1–R10.4     |
| Q5  | TD-1 resolution (load-bearing)                        | **Path (b) — custom-widget escape hatch.** Verified at this design pass: `crates/ui/Cargo.toml:69` reads `iced = { version = "=0.14.0", default-features = false, features = ["tiny-skia", "thread-pool", "advanced", "canvas"] }`. iced 0.15+ has not landed; neither `button::Status::Focused` nor `text_input::Style.shadow` is available. Path (a) fold-in is therefore not on the table this phase. Path (c) restate-with-deadline is **rejected**: Phase 6 is gated on v2 LLM (which may take quarters); a fifth restatement is operationally indefinite, and Phase 5 is exactly the moment the cost/benefit tightened (three new operator-write surfaces). The architect commits to **(b)** — see the **TD-1 resolution** sub-section for the concrete `crates/ui/src/widgets/focus_ring.rs` shape. | R13.1, R13.3, R13.5 |
| Q6  | `tape` → `AgentFeed` snapshot rename scope            | **Rename, via `git mv`** (preserves snapshot-history greppability). Ratified per analyst. Snapshot filenames are operator-greppable; stale-filename → new-module mismatch breaks the convention. The 9 baselines (`panel_snapshots__tape_loading.snap`, `_empty.snap`, `_error.snap`, `_ready_three_fills.snap`, `_paused.snap`, `_audit_modal_{empty,error,loading,ready_paper_fill}.snap`) move via `git mv` to preserve git-history continuity; the body diff is title-string only (R11.5 — `PANEL_TAPE_TITLE` → `PANEL_AGENT_FEED_TITLE`). After the `git mv`, a single `cargo insta accept` pass at end of phase regenerates the body content for the title-string change. **Not** delete-then-regenerate (`cargo insta accept` over deleted baselines) — that path loses git history and bloats the diff for review. | R11.1, R12.1, R12.4 |
| Q7  | HumanControl panel field set                          | **Full Lumen set** (mode + 3 limits + kill). Ratified per analyst. All three limit fields read from existing `Cockpit::risk_state` + `Cockpit::pnl` (no new backend wiring). Trimmed (mode + kill only) rejected — hides the daily-loss-limit context the principles doc calls out as load-bearing.                                                                                                                                                                                                                                                                                  | R3.1–R3.5       |
| Q8  | Pause-strategy resume semantics                       | **Single-click resume.** Ratified per analyst. Pause is bounded-destructive (skips future signals; doesn't reverse past decisions); resume returns to default state — principles-doc "undo where physically possible" case. Typed-confirm both sides rejected — friction without proportional safety value.                                                                                                                                                                                                                                                                                                  | R4.4, R6.1      |
| Q9  | Override-risk-veto scope                              | **Per-veto override.** Ratified per analyst. Operator must consciously override each veto; per-strategy is too broad ("disable risk-engine for this strategy" is exactly what the engine exists to prevent). Forward-only — the veto is dismissed + audit row recorded; the agent does NOT re-emit the blocked signal. Each override is its own typed-confirm flow.                                                                                                                                                                                                                                | R7.3            |
| Q10 | Audit-writer test scope                               | **Unit + integration + audit-row snapshot baseline (all three).** Ratified per analyst. Unit tests cover the dual-write contract (sibling of `kill_switch_tripped` tests); integration tests cover cockpit → bus → writer wiring; snapshot baselines lock the audit row format (`strategy_events__strategy_paused_row.snap`, `strategy_events__risk_veto_overridden_row.snap`). Unit-only rejected — leaves cockpit-side wiring untested; cockpit ↔ writer is the load-bearing seam.                                                                                                          | R5, R8          |
| Q11 | Snapshot baseline budget                              | **~9 rename pairs + ~10 net-new (4 HumanControl + 3 Strategies-pause/override + 2 modal flow + 1 home/debug regen) + 1 Q1-driven Debug-screen regen; single `cargo insta accept` pass at end of phase.** Ratified per analyst (lower-end of the 8–12 range). Phase 1 Q2 / Phase 2 V11 / Phase 3 V12 / Phase 4 V12 single-pass precedent. Q1 picks (a) sidebar entry → kill migrates from Debug → Debug-screen baseline regenerates (1 row), Home stays byte-identical.                                                                                                                                                                                                          | R12, V12        |
| Q12 | Kill button copy in HumanControl                      | **Preserve `"Stop trading"`** (`KILL_BUTTON_LABEL`). Ratified per analyst. Master Constraint 2 (no voice rewrite) + principles-doc "exact phrase not negotiable mid-session". Lumen `"Halt all agents"` rejected.                                                                                                                                                                                                                                                                                                                                                                                                                          | R2.4            |
| Q13 | Risk-engine veto-emit wiring                          | **Placeholder feed in Phase 5; defer real upstream wiring.** Ratified per analyst. Phase 5 ships the cockpit-side override flow (typed-confirm + audit writer + clear-from-list) over `Vec<VetoEvent>` populated by fixtures; live emits empty `Vec`. The real risk-engine veto-emit wiring tracks as deferred task **`TD-2-risk-engine-veto-emit`** — see the **Risk-engine veto-emit deferral** sub-section. The deferred-task tracking row is flagged for the orchestrator to append to the master roadmap's "Cross-phase technical-debt items" section as a new TD-2 row. | R7.2            |
| Q14 | `Cockpit::tape` field rename                          | **Preserve `Cockpit::tape` field name.** Ratified per analyst. Field is referenced by every Phase 1–4 test fixture and the `tape-row-audit-modal` modal-trigger import path; renaming would ripple through ~100+ test sites for cosmetic value. Phase 5 is module rename, not state-shape rename. **Mismatch is documented** via a code-comment on the field annotation pointing at the `agent_feed.rs` module path — see **Cockpit state diff** below for the exact comment text.                                                                                                                                                              | R11.4           |
| Q15 | Audit-query reader for "recent operator writes"       | **NO — defer.** Ratified per analyst. New `StrategyPaused` / `RiskVetoOverridden` rows are queryable via Phase 3's `recent_journal_filtered` (with `kind` filtering); Phase 3's Audit screen is the canonical surface for `strategy_events`. A dedicated "recent operator activity" panel is a separate future brief; not Phase 5 scope.                                                                                                                                                                                                                                                                                                                                                | (none)          |

**No principled overrides.** Analyst recommendations are
operator-aligned, consistent with the master roadmap's operator-locked
Q11–Q14, the cross-feature invariant table, and the
zero-anchor-risk discipline; the architect ratifies all fifteen. The
load-bearing decision (Q5 / TD-1) commits to **path (b)** based on
the on-disk iced version verification and the operator-write surface
sharpening point.

### Cockpit state diff

The state diff `crates/ui/src/state.rs` receives in Phase 5 is
**three new fields + one new enum + four new `Message` variants**.
The `tape` field name is **preserved** per Q14 with an annotation
comment.

```rust
// ── crates/ui/src/state.rs — Phase 5 additions ─────────────────────────────

/// Phase 5 — execution mode (Q4 — runtime-only). Cold-start = Observe.
/// Mirrors AgentMode's derive set ([state.rs:89–96]).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Observe,
    Supervised,
    Auto,
}

impl Default for ExecutionMode {
    fn default() -> Self { Self::Observe }
}

/// Phase 5 — typed-confirm state for the override-risk-veto modal
/// (R7.4 / R7.5). Mirror of `KillState::Confirming { typed }`.
#[derive(Debug, Clone, Default)]
pub enum OverrideRiskVetoState {
    #[default]
    Idle,
    Confirming { veto_id: SmolStr, typed: String },
    Submitting { veto_id: SmolStr },
}

/// Phase 5 — surfaced risk-engine veto event (R7.2). Live upstream
/// stubbed (Q13); fixtures populate. Real wiring tracked as TD-2.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VetoEvent {
    pub veto_id: SmolStr,
    pub ts: Timestamp,
    pub strategy_id: StrategyId,
    pub reason: SmolStr,
    pub blocked_signal: Signal,
}

pub struct Cockpit {
    // … all existing Phase 1 / 2 / 3 / 4 fields …

    /// Live fills panel state. **Module renamed `tape.rs` → `agent_feed.rs`
    /// (Phase 5 R11). Field name preserved per Phase 5 Q14 — renaming
    /// the field would ripple through ~100+ test sites for cosmetic
    /// value. See `widgets::agent_feed` module doc-comment for the
    /// rename rationale.**
    pub tape: PanelState<VecDeque<FillView>>,

    // … existing tape_paused / tape_paused_buffer / tape_audit_modal stay …

    // ── Phase 5 — HumanControl ─────────────────────────────────────────
    /// Operator-selected execution mode (Q4 — runtime-only).
    pub execution_mode: ExecutionMode,
    /// Per-strategy pause set (R4.3). Sibling of `tape_paused: bool`
    /// at [`state.rs:455`](state.rs); single-click toggles membership.
    pub paused_strategies: HashSet<StrategyId>,
    /// Typed-confirm state for the per-veto override modal (R7).
    pub override_risk_veto: OverrideRiskVetoState,
    /// Surfaced risk-engine veto events (R7.2). Live upstream is the
    /// `default_risk_telemetry_stub` at
    /// [`crates/agent/src/runtime.rs:1023–1090`](runtime.rs); real
    /// wiring tracked as TD-2 (see Phase 5 Design / Risk-engine
    /// veto-emit deferral).
    pub risk_veto_events: Vec<VetoEvent>,
}

pub enum Message {
    // … all existing Phase 1 / 2 / 3 / 4 variants …

    // ── Phase 5 — HumanControl panel ───────────────────────────────────
    /// Operator clicked one of the three execution-mode segments.
    /// Pure assignment to `Cockpit::execution_mode`; live mode also
    /// emits on `execution_mode_tx` (R10.3).
    ExecutionModeSelected(ExecutionMode),

    // ── Phase 5 — Pause-strategy ───────────────────────────────────────
    /// Operator clicked the per-row pause/resume button (R4.3 / Q8 —
    /// single-click both directions). Pure update flips set membership;
    /// live mode also emits on `pause_strategy_tx` (R4.6) and spawns
    /// the audit-writer task (R5.5).
    StrategyPauseToggled(StrategyId),

    // ── Phase 5 — Override-risk-veto (kill-confirm modal mirror) ───────
    /// Operator pressed the `Override` button on a surfaced veto event.
    /// Opens the typed-confirm modal in `Confirming { veto_id, typed: "" }`.
    OverrideRiskVetoPressed(SmolStr),
    /// Operator typed into the OVERRIDE input. Pure update.
    OverrideRiskVetoTyped(String),
    /// Operator pressed cancel on the modal. Returns to `Idle`.
    OverrideRiskVetoCancelled,
    /// Operator pressed confirm with the phrase matched. Spawns the
    /// audit-writer task (R8.5) + clears the matching `VetoEvent`.
    OverrideRiskVetoConfirmed(SmolStr),
}
```

**`Default` impl extension:** `execution_mode: ExecutionMode::default()`,
`paused_strategies: HashSet::new()`, `override_risk_veto:
OverrideRiskVetoState::default()`, `risk_veto_events: Vec::new()`.
**Manual `Debug` impl extension:** four new `.field(...)` calls
mirroring the field set.

**Message-handler diffs** (pure assignment; live mode wires `_tx`
emission + audit-spawn in the binary):

```rust
Message::ExecutionModeSelected(mode) => {
    model.execution_mode = mode;
}
Message::StrategyPauseToggled(id) => {
    if !model.paused_strategies.remove(&id) {
        model.paused_strategies.insert(id);
    }
}
Message::OverrideRiskVetoPressed(veto_id) => {
    model.override_risk_veto = OverrideRiskVetoState::Confirming { veto_id, typed: String::new() };
}
Message::OverrideRiskVetoTyped(s) => {
    if let OverrideRiskVetoState::Confirming { typed, .. } = &mut model.override_risk_veto {
        *typed = s;
    }
}
Message::OverrideRiskVetoCancelled => {
    model.override_risk_veto = OverrideRiskVetoState::Idle;
}
Message::OverrideRiskVetoConfirmed(veto_id) => {
    model.risk_veto_events.retain(|v| v.veto_id != veto_id);
    model.override_risk_veto = OverrideRiskVetoState::Idle;
    // Live: binary's update arm spawns audit::journal::risk_veto_overridden(...) here.
}
```

### HumanControl panel widget contract

**File:** `crates/ui/src/widgets/human_control.rs` (new). **Public
API** (R1.1):

```rust
pub fn view(model: &Cockpit) -> Element<'_, Message>;
```

**Layout** (R1.1, R1.2 — Lumen `HumanControl.jsx:6–55` reference):
Tier-1 `widgets::frame::panel(PANEL_HUMAN_CONTROL_TITLE, body,
ThemeMode::Dark)` wrapper. Body = `Column::new().spacing(space::M)`
with four sub-blocks top-to-bottom:

1. **Mode segmented control** (R9.2 — see **Execution-mode toggle
   contract** below).
2. **Three mirror rows** (R3.1–R3.4 — `Daily loss limit` /
   `Max position` / `Used today`).
3. **Kill action** — direct `widgets::kill::view(model)` call (R2.1).
   No flow duplication; the existing kill widget is the bottom action
   verbatim. The kill widget's Tier-1 `panel` wrapper is **stripped**
   when called from HumanControl by passing through a new
   `widgets::kill::view_inner(model)` helper that returns the body
   only — the outer HumanControl `panel` is the chrome owner. (Public
   `widgets::kill::view` retains its current shape per R2.3 so the
   Debug-screen invariant doesn't shift if Q1 had picked another
   placement; `view_inner` is the body extraction the new caller
   uses.)

**Title constants** (R1.2): `PANEL_HUMAN_CONTROL_TITLE = "You're in
control"`, `PANEL_HUMAN_CONTROL_META = "Human-in-the-loop"`. Both
net-new to `crates/ui/src/strings.rs` (Constraint 2 unchanged —
additive, not voice rewrite).

**Mirror rows** (R3.1–R3.4): `widgets::human_control::limit_row(label,
value, sentiment)` private helper renders a `Row` with `text::SMALL`
`FG_3` muted label on the left, `text::BODY` value on the right
(coloured per `sentiment: Option<Color>`). Used-today row passes
`Some(UP_500)` / `Some(DOWN_500)` per `Cockpit::pnl` sign
(`color_for_delta` from `widgets::pnl`); the other two rows pass
`None` (`FG_1` neutral). Loading → three muted `—` dashes; Error
→ `frame::muted_body(strings::HUMAN_CONTROL_LIMITS_UNAVAILABLE)`.

**Placement** (Q1 / R1.3): rendered as the **7th sidebar entry**
`Screen::Control`. Sidebar build in both bins
(`crates/ui/src/bin/cockpit.rs` + `cockpit_live.rs`) gains an
additional `SidebarEntry { id: "control", label: "Control", screen:
Screen::Control }` after the existing 6. The Phase 2 R1.6 sidebar
widget API absorbs this additively — no widget code change.

### Pause-strategy control contract

**Per-row button** on the Strategies-detail screen (R4.1) + Home →
Strategies-summary (R4.5). New helper
`widgets::strategies::pause_button(id, paused: bool) -> Element<'_,
Message>` renders a `Button` labelled
`STRATEGY_PAUSE_LABEL` (`"Pause"`) when `paused == false`,
`STRATEGY_RESUME_LABEL` (`"Resume"`) when `paused == true`. Click
emits `Message::StrategyPauseToggled(id)`. **Single-click both
directions per Q8** — no typed-confirm gate.

**Audit writer call** (live mode only): the binary's
`StrategyPauseToggled(id)` arm spawns
`audit::journal::strategy_paused(ledger, id, paused, operator)` via
`Task::perform` + `tokio::runtime::Handle::spawn` — sibling of the
Phase 1 kill-confirm spawn pattern at `crates/ui/src/state.rs`
under `#[cfg(feature = "live")]`.

**Live bus wiring** (R4.6): new broadcast channel
`pause_strategy_tx: broadcast::Sender<(StrategyId, bool)>` lives on
`EventBus` (sibling of the kill-switch closure pattern documented
at architecture.md `Cockpit ← Arc<KillSwitch>` § 3160). Agent
runtime's strategy registry subscribes; per-strategy `on_bar` /
`on_tick` consults pause membership before forwarding signals.
Fixtures-mode is UI-only (no bus emit; pure visual round-trip).

### Override-risk-veto control contract

**Per-veto button** (R7.1, R7.3 — Q9 ratifies per-veto). Each entry
in `Cockpit::risk_veto_events` renders a row on the Strategies-
detail screen with veto reason text + `Override` button. Click
emits `Message::OverrideRiskVetoPressed(veto_id)`.

**Typed-confirm modal flow** (R7.4 / R7.5) — **mirror of kill-confirm
at `widgets/kill.rs:92–155`**:

- New module `crates/ui/src/widgets/override_risk_veto.rs`. Public:
  `pub fn modal_view(state: &OverrideRiskVetoState) ->
  Option<Element<'_, Message>>` (returns `None` when `Idle`; `Some`
  when `Confirming` or `Submitting`).
- Modal body: title `OVERRIDE_RISK_VETO_DIALOG_TITLE` ("Override risk
  veto"), explanatory body, `OVERRIDE` phrase via constant
  `OVERRIDE_RISK_VETO_PHRASE = "OVERRIDE"`, sunken `text_input` with
  `BORDER_2 → ACCENT` border-shift on focus (mirror of kill confirm
  input — focus-ring shadow lands via the new `widgets::focus_ring`
  per TD-1 path b).
- Confirm button **disabled until `typed == OVERRIDE_RISK_VETO_PHRASE`**;
  emits `Message::OverrideRiskVetoConfirmed(veto_id)`. Cancel emits
  `Message::OverrideRiskVetoCancelled` (always enabled).
- Mismatch hint copy: `OVERRIDE_RISK_VETO_PHRASE_MISMATCH_HINT`
  ("Type OVERRIDE exactly to enable confirm").

**Audit writer call** + clear-from-list: the binary's
`OverrideRiskVetoConfirmed(veto_id)` arm spawns
`audit::journal::risk_veto_overridden(ledger, veto_id, strategy_id,
reason, operator)` via the same closure pattern as kill-confirm; the
pure-update arm has already cleared the `VetoEvent` from
`risk_veto_events` so the visual state reflects immediately.

**Forward-only** per Q9: the agent does NOT re-emit the blocked
signal. The override is recorded; the trade does not happen.

### Execution-mode toggle contract

**Segmented control** (R9.2) rendered inside HumanControl. New helper
`widgets::human_control::mode_segment(active: ExecutionMode) ->
Element<'_, Message>` renders a `Row` with three `Button`s — one per
`ExecutionMode` variant. Active variant uses the Phase 1
**active-row pattern** (background `PANEL_RAISED`, border `ACCENT @
1px`); inactive variants use the default panel-button style.

**Per-mode hint copy** (R9.4) below the segment row, rendered via
`frame::muted_body(...)` against the active mode's hint constant:
`EXECUTION_MODE_OBSERVE_HINT = "Watch only — no orders sent."` /
`EXECUTION_MODE_SUPERVISED_HINT = "Each decision needs your approval."`
/ `EXECUTION_MODE_AUTO_HINT = "Within-envelope autonomy."` Phrasing
mirrored from `HumanControl.jsx:27–31` with the project's voice
discipline.

**Click handler:** emits `Message::ExecutionModeSelected(mode)`. Pure
update flips `Cockpit::execution_mode`. **No typed-confirm** (R9.5 /
Q4 — mode gates *future* trades only; not destructive).

**Runtime-only persistence** per Q4: cold-start = `Observe`; restart
returns to `Observe`; no `config/agent.toml` write; no audit writer.

**Live bus wiring** (R10.3): new broadcast channel `execution_mode_tx:
broadcast::Sender<ExecutionMode>` on `EventBus`. Strategy registry
consults active mode before forwarding signals. `Observe` short-
circuits at the executor (already shipped paper-only); `Supervised`
v1 v1 ships the channel surface only (per-decision approval UI is a
separate Phase-N+ deliverable, gated on v2 LLM); `Auto` is the
existing default behaviour. Fixtures-mode emits no bus event.

### Audit writer additions

**No SQL migration.** Confirmed at `crates/audit/migrations/002_strategy_events.sql`
— `kind` column is `TEXT`. Two new variants extend
`StrategyEventKind` at the application layer per Q-confirm.

**Enum extension** (`crates/core/src/strategy_events.rs:99–113`):

```rust
#[serde(rename_all = "PascalCase")]
pub enum StrategyEventKind {
    // … existing 9 variants …
    /// Phase 5 R5.2 — operator paused or resumed a strategy.
    /// Direction encoded in `error_summary` ("paused" | "resumed").
    StrategyPaused,
    /// Phase 5 R8.2 — operator overrode a risk-engine veto.
    /// Reason preserved verbatim in `error_summary`;
    /// `error_code = "risk_veto_overridden"`.
    RiskVetoOverridden,
}
```

Mandatory `Display` impl extensions + serde round-trip tests under
`strategy_events::tests::pascal_case_for_new_variants` (R5.2 / R8.2 /
V9).

**Writer signatures** (sibling of `kill_switch_tripped` at
`crates/audit/src/journal.rs:316–407`):

```rust
/// Phase 5 R5 — audit writer for operator pause / resume of a single
/// strategy. Atomic dual-write: memo row in `journal_transactions` +
/// `strategy_events` row in one transaction.
///
/// - `paused == true`  → `error_summary = "paused"`,
///   memo `"strategy:StrategyPaused:<id>:paused"`.
/// - `paused == false` → `error_summary = "resumed"`,
///   memo `"strategy:StrategyPaused:<id>:resumed"`.
///
/// Memo row uses `Rfc3339` second precision (preserved from
/// `kill_switch_tripped`); strategy_events row uses 6-digit
/// fractional-second format.
#[instrument(name = "ledger.strategy_paused", skip(ledger))]
pub async fn strategy_paused(
    ledger: &Ledger,
    strategy_id: &StrategyId,
    paused: bool,
    operator: &str,
) -> Result<(), LedgerError>;

/// Phase 5 R8 — audit writer for operator override of a risk-engine
/// veto. Atomic dual-write per `strategy_paused`. Memo desc:
/// `"strategy:RiskVetoOverridden:<veto_id>:<reason>"`. `error_code =
/// "risk_veto_overridden"`; `error_summary` carries `reason` verbatim.
#[instrument(name = "ledger.risk_veto_overridden", skip(ledger))]
pub async fn risk_veto_overridden(
    ledger: &Ledger,
    veto_id: &str,
    strategy_id: &StrategyId,
    reason: &str,
    operator: &str,
) -> Result<(), LedgerError>;
```

**Column projection on the `strategy_events` insert** (sibling of
`kill_switch_tripped`'s 11-column bind at `journal.rs:382–398`):

| Column          | `strategy_paused`                       | `risk_veto_overridden`              |
|-----------------|-----------------------------------------|-------------------------------------|
| `id`            | `Uuid::new_v4().to_string()`            | `Uuid::new_v4().to_string()`        |
| `ts`            | 6-digit fractional-second format        | same                                |
| `kind`          | `"StrategyPaused"`                      | `"RiskVetoOverridden"`              |
| `strategy_id`   | `Some(strategy_id.as_str())`            | `Some(strategy_id.as_str())`        |
| `old_hash`      | `None`                                  | `None`                              |
| `new_hash`      | `None`                                  | `None`                              |
| `source_path`   | `""`                                    | `""`                                |
| `operator`      | `operator`                              | `operator`                          |
| `error_code`    | `Some("strategy_paused")`               | `Some("risk_veto_overridden")`      |
| `error_summary` | `Some("paused")` / `Some("resumed")`    | `Some(reason)`                      |
| `venue`         | `None`                                  | `None`                              |

**Test scope (Q10 — unit + integration + audit-row snapshot baseline):**

- **Unit** in `crates/audit/src/journal.rs::tests`:
  - `strategy_paused_emits_pascal_case_kind`
  - `strategy_paused_balanced_memo_row`
  - `strategy_paused_atomic_dual_write`
  - `strategy_paused_resume_flips_error_summary`
  - `risk_veto_overridden_emits_pascal_case_kind`
  - `risk_veto_overridden_balanced_memo_row`
  - `risk_veto_overridden_reason_preserved_in_error_summary`
- **Integration** in `crates/audit/tests/strategy_paused.rs` and
  `crates/audit/tests/risk_veto_overridden.rs`: cockpit → bus → writer
  end-to-end fixture seeded against an in-memory ledger.
- **Snapshot baselines** in `crates/audit/tests/snapshots/`:
  `strategy_events__strategy_paused_row.snap`,
  `strategy_events__risk_veto_overridden_row.snap`. Lock the row
  format (sibling of any existing audit-row snapshot baseline; format
  fields: `kind`, `strategy_id`, `error_code`, `error_summary`).

### `tape` → `AgentFeed` rename

**Module rename via `git mv`** (preserves git history; reviewable
diff = title-string body change only):

```bash
git mv crates/ui/src/widgets/tape.rs crates/ui/src/widgets/agent_feed.rs
```

**Module doc-comment** retitles to `"Live agent activity feed"` and
adds a one-line note: `"Field on Cockpit is preserved as
'Cockpit::tape' per Phase 5 Q14 — see lumen-phase-5-humancontrol-
agentfeed.md / Cockpit state diff."`

**Mod-decl** at `crates/ui/src/widgets/mod.rs:28`:
`pub mod tape;` → `pub mod agent_feed;`.

**Import-site ripple** (R11.3): `use crate::widgets::tape` →
`use crate::widgets::agent_feed` across `state.rs`, `bin/cockpit.rs`,
`bin/cockpit_live.rs`, `crates/ui/tests/`. The **field name
`Cockpit::tape` stays** per Q14; the rename is module-path-only.

**String constant rename** (R11.5): `PANEL_TAPE_TITLE` →
`PANEL_AGENT_FEED_TITLE = "Agent activity"` per `AgentFeed.jsx:71`.
Net-new constant (the old constant retires same commit; this is the
Phase 5 rename closure, not a voice rewrite).

**Snapshot baselines** rename via `git mv` (Q6 — preserves history):

```bash
git mv crates/ui/tests/snapshots/panel_snapshots__tape_loading.snap \
       crates/ui/tests/snapshots/panel_snapshots__agent_feed_loading.snap
git mv crates/ui/tests/snapshots/panel_snapshots__tape_empty.snap \
       crates/ui/tests/snapshots/panel_snapshots__agent_feed_empty.snap
git mv crates/ui/tests/snapshots/panel_snapshots__tape_error.snap \
       crates/ui/tests/snapshots/panel_snapshots__agent_feed_error.snap
git mv crates/ui/tests/snapshots/panel_snapshots__tape_ready_three_fills.snap \
       crates/ui/tests/snapshots/panel_snapshots__agent_feed_ready_three_fills.snap
git mv crates/ui/tests/snapshots/panel_snapshots__tape_paused.snap \
       crates/ui/tests/snapshots/panel_snapshots__agent_feed_paused.snap
git mv crates/ui/tests/snapshots/panel_snapshots__tape_audit_modal_empty.snap \
       crates/ui/tests/snapshots/panel_snapshots__agent_feed_audit_modal_empty.snap
git mv crates/ui/tests/snapshots/panel_snapshots__tape_audit_modal_error.snap \
       crates/ui/tests/snapshots/panel_snapshots__agent_feed_audit_modal_error.snap
git mv crates/ui/tests/snapshots/panel_snapshots__tape_audit_modal_loading.snap \
       crates/ui/tests/snapshots/panel_snapshots__agent_feed_audit_modal_loading.snap
git mv crates/ui/tests/snapshots/panel_snapshots__tape_audit_modal_ready_paper_fill.snap \
       crates/ui/tests/snapshots/panel_snapshots__agent_feed_audit_modal_ready_paper_fill.snap
```

After the moves land, the body content of these 9 baselines diffs
only on the title-string change (R11.5). A single `cargo insta
accept` pass at end of phase regenerates the body content
in-place; the rename history is preserved by `git mv`.

**Consistency-test fixture** (R12.2): `crates/ui/tests/consistency.rs`
globs `*.rs` under `src/widgets/` (verified at lines 24–33 in the
brief reference); the rename auto-picks up — **no test-code update
needed**.

### TD-1 resolution

**Verification on disk (load-bearing).** `crates/ui/Cargo.toml:69`
reads:

```
iced = { version = "=0.14.0", default-features = false, features = ["tiny-skia", "thread-pool", "advanced", "canvas"] }
```

iced 0.15+ has not landed; neither `button::Status::Focused` nor
`text_input::Style.shadow` is available. **Path (a) fold-in is
unavailable.** Path (c) restate-with-deadline is rejected — Phase 6
is gated on v2 LLM (operationally indefinite); Phase 5 is the
sharpening point (three new operator-write surfaces). **The
architect commits to path (b) — custom-widget escape hatch.**

**Concrete plan: `crates/ui/src/widgets/focus_ring.rs` shape.**

- New module `crates/ui/src/widgets/focus_ring.rs`. Implements an
  `iced::widget::Component` (or, if `Component` proves heavyweight
  for this scope, a small `Element` wrapper that owns focus state
  via a parent-side `Subscription`).
- Owns focus state via a `Subscription` on
  `iced::keyboard::on_key_press` filtered to `Key::Named(Named::Tab)`
  + `Key::Named(Named::ArrowDown)` / `ArrowUp` for arrow-key
  navigation within a focus group.
- Emits a synthetic `Message::FocusChanged(WidgetId)` (`WidgetId =
  SmolStr`) on focus traversal. Cockpit's `update` arm assigns to
  a new `Cockpit::focused_widget: Option<WidgetId>` field (treated
  as Phase-5 internal state — no audit writer, no persistence).
- The focus group is a per-screen registration: each destructive
  control (kill button, kill confirm input, override-risk-veto
  button per veto, override-risk-veto confirm input, pause/resume
  button per strategy) registers its `WidgetId` with the focus
  group at `view` time. Tab order = registration order.
- Focus-ring rendering: when `focused_widget == Some(self.id)`, the
  wrapper draws a halo `Container` overlay using the existing
  `theme::focus::ring(mode)` token (3 px low-alpha accent) — same
  visual contract as the Phase 1 hover-state approximation, applied
  on `Focused` semantics instead of `Hovered`.

**Consumer sites** (the four destructive surfaces gating on focus):

1. `widgets::kill::view` — kill button + kill confirm input both
   wrap in `focus_ring::wrap("kill_button"/"kill_input", child)`.
2. `widgets::override_risk_veto::modal_view` — confirm input + cancel
   + confirm buttons all wrap in `focus_ring::wrap(...)`.
3. `widgets::strategies::pause_button` — wrap each per-strategy
   pause button.
4. `widgets::human_control::mode_segment` — wrap each of the three
   mode buttons (so Tab traversal lands on the segmented control
   coherently).

**Tester verification** (V10 — path (b)): `cargo test -p ui
focus_ring::tests::focus_traversal_*` PASS — synthetic `Tab`
keypress on a fixtures cockpit advances focus through the registered
widgets in order; `focus_ring::tests::focus_halo_renders_on_focused`
asserts the halo overlay lands on the focused widget snapshot
baseline (`panel_snapshots__focus_ring__focused_kill_button.snap`,
one new baseline; folds into the Q11 net-new budget).

**Rejection rationale for the alternatives.** **Path (a)** rejected
on the on-disk version verification — iced still pins `=0.14.0`;
Phase 5 cannot fold in an upstream upgrade that hasn't shipped.
**Path (c)** rejected on the operator-write-surface sharpening: a
fifth restatement of the deferral with a Phase 6 deadline is
operationally indefinite (Phase 6 is gated on v2 LLM, which may
take quarters); the analyst rejects it and the architect agrees.

**Anchor risk.** Zero — `focus_ring` is a UI module; no audit /
report / strategy path touched. The new focus baseline is additive.

### Risk-engine veto-emit deferral

**Phase 5 ships the operator-side surface over a placeholder feed.**
The cockpit-side `Vec<VetoEvent>` is populated by fixtures in
fixtures-mode and emits an empty `Vec` in live mode at Phase 5 ship.
The typed-confirm modal flow + the `risk_veto_overridden` audit
writer + the `clear-from-list` semantics are fully testable today
because the typed-confirm modal is operator-driven, not feed-driven.

**Real risk-engine veto-emit upstream is out of Phase 5 scope.** The
upstream wiring touches the strategy → risk → executor pipeline (where
in the chain the veto emits, what fields the veto carries, persistence
question for veto provenance) — non-trivial and orthogonal to Phase 5's
operator-facing override surface.

**Tracking row.** The deferred task is named **`TD-2 — Risk-engine
veto-emit upstream wiring`** (sibling of TD-1). The architect flags
this for the orchestrator to append a new row to the master roadmap's
`## Cross-phase technical-debt items` section under TD-1, with the
following content (architect does NOT edit the master roadmap directly
— this is a flag for the orchestrator to route to the analyst on
Phase 5 ship):

> **TD-2 — Risk-engine veto-emit upstream wiring (Phase 5 Q13
> deferral, ratified 2026-05-06)**
>
> **Origin:** Phase 5, Q13. Architect ratified the analyst's
> "placeholder feed" recommendation: Phase 5 ships the cockpit-side
> override flow (typed-confirm + audit writer + clear-from-list)
> over `Cockpit::risk_veto_events: Vec<VetoEvent>` populated by
> fixtures; live emits empty `Vec`.
>
> **Gap.** The agent runtime's `default_risk_telemetry_stub` at
> `crates/agent/src/runtime.rs:1023–1090` does not emit `VetoEvent`s
> upstream of the cockpit. The risk-engine veto path lives in
> `crates/risk/` but the emission point + the `VetoEvent` shape +
> the persistence question are not wired through to `EventBus`.
>
> **Phase 5 shipped state.** Operator-side override surface is
> fully testable in fixtures mode; live mode shows an empty
> override surface (no vetoes emitted, no overrides possible).
>
> **Promotion timing.** Earliest target is **Phase 6 (Assistant
> slot, v2-LLM-gated)** if v2 LLM lands first; otherwise a
> standalone backend brief with no Lumen-phase coupling.
>
> **Operator-impact bound.** Live operators do not lose safety —
> the risk engine still vetoes upstream of the executor (the
> existing veto path is functional); Phase 5's surface is the
> *operator override* of those vetoes, which is currently
> impossible-to-exercise live (no surfaced vetoes → no override
> button) but the safety primary is preserved.

### Cross-feature invariants

Phase 5 column from the master roadmap, re-stated with the design
note:

| Feature                         | Phase 5 invariant note                                                                                                | How preserved                                                                                                                                                                                                                                                                                                                                            |
|---------------------------------|-----------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `operator-success-reports`      | Latency-badge + report-rendering surfaces unchanged.                                                                  | Phase 5 is HumanControl + audit-writer additions + UI rename. No reports-side write path; latency-badge surface untouched. R14.1.                                                                                                                                                                                                                        |
| `live-cockpit-unified`          | `cockpit_live` launches unchanged; halted-banner triggers preserved; new bus channels are additive on `EventBus`.     | `pause_strategy_tx` + `execution_mode_tx` are additive `broadcast::Sender` channels on `EventBus`; existing Phase 1 / 2 / 3 / 4 channels untouched. Halted-banner shell-level wrap (Phase 2 R3.3) untouched. R14.2.                                                                                                                                       |
| `real-mtm-unrealized-pnl`       | P&L card unchanged (R3 reads but does not mutate).                                                                    | HumanControl R3 mirror of `Cockpit::pnl` is read-only via `widgets::pnl::color_for_delta`; helper signature unchanged; the P&L card on Home stays in place. R14.3.                                                                                                                                                                                       |
| `per-symbol-position-accounts`  | Positions widget unchanged.                                                                                           | No position contract change; new audit writers do not touch the position columns. R14.4.                                                                                                                                                                                                                                                                 |
| `tape-row-audit-modal`          | Modal trigger / frame unchanged; widget renamed but modal trigger source path byte-identical (R11.4 preserves field). | The `Cockpit::tape` field name stays per Q14; the modal trigger reads from the same field. The module path renames `tape` → `agent_feed` but the modal widget code (`widgets::journal_transaction_modal`) is untouched. R14.5.                                                                                                                            |
| `journal-tx-metadata`           | Modal continues to render `description` + `strategy_id`.                                                              | New audit writers populate `strategy_id` per the column projection table above; modal reader is unchanged. R14.6.                                                                                                                                                                                                                                        |
| `v1.5b-multi-venue`             | Unchanged; new audit writers do not consume the venue column.                                                         | Both `strategy_paused` and `risk_veto_overridden` writers bind `venue: None` per the column projection table; venue plumbing untouched. R14.7.                                                                                                                                                                                                           |

**Acceptance:** the tester's per-feature invariant table = 7 / 7 PASS.

### Anchor regression

**Zero anchor risk re-affirmed.** The design pass found no path
where Phase 5 touches committed report bodies:

- **The two new audit writers (`strategy_paused`,
  `risk_veto_overridden`) are additive** — new
  `StrategyEventKind` enum variants + sibling functions following
  the `kill_switch_tripped` pattern verbatim. No existing row's
  body is altered; `kind` column is `TEXT` so no schema migration.
- **No new backtest scenarios** — Phase 5 is operator-write
  surfaces, not strategy code; no new committed report renders.
- **The `tape` → `agent_feed` rename is module-path + snapshot-
  filename + title-string only** — no committed report references
  the `tape` widget module.
- **The custom-widget focus-ring (TD-1 path b)** is UI-only — no
  audit / report / strategy path touched.
- **HumanControl + per-strategy pause + override-risk-veto controls**
  are operator-write surfaces over the audit ledger; the writers
  are sibling-of-`kill_switch_tripped` per architecture.md's
  v1+ Q8 contract.

`verify-anchors` gate at the Phase 5 tester run must report 11 / 11
PASS with byte-identical bodies. The R16.3 grep gate from Phase 1
(`grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800"
spec/reports/`) remains zero — Phase 5 adds no new rendered prose
to any committed report.

If at any point in implementation a path is proposed that re-renders
a committed report body, the developer must **stop and route HANDOFF
→ analyst** — that path is out of scope for Phase 5 and must be
re-litigated.

### Implementation parallelism map

```
T1901 (foundation gate — ExecutionMode + OverrideRiskVetoState + VetoEvent +
       4 Cockpit field additions + 6 Message variants + Default/Debug ext)
  ├─ T1902 (StrategyEventKind ext + audit writers + unit + integration + 2 row baselines
  │         — parallel; audit + core crates, no ui dep)
  ├─ T1903 (tape → agent_feed rename — git mv module + 9 snapshot git mv
  │         + import-site ripple + title-string change — parallel; ui crate)
  ├─ T1904 (HumanControl widget skeleton + PANEL_HUMAN_CONTROL_TITLE/META —
  │         parallel after T1901; ui crate)
  ├─ T1909 (Override-risk-veto modal widget + OVERRIDE phrase constants —
  │         parallel after T1901; ui crate)
  └─ T1912 (TD-1 resolution — focus_ring widget skeleton — parallel after T1901;
            ui crate; Subscription wiring + Cockpit::focused_widget field)
        │
        ▼
   After T1904 + T1912 land:
        ├─ T1905 (HumanControl mirror rows — Daily-loss / Max-position / Used-today)
        ├─ T1906 (HumanControl integration into Cockpit — 7th sidebar entry +
        │         kill-as-bottom-action via widgets::kill::view_inner extraction)
        ├─ T1911 (Execution-mode segmented control + 3 hint constants +
        │         live bus channel execution_mode_tx)
        └─ T1909-followup (override modal focus_ring wrapping + audit-writer call)
                        │
                        ▼
              After T1902 + T1912 land:
                ├─ T1907 (Pause-strategy per-row button + pause_strategy_tx
                │         + audit-writer call wiring)
                └─ T1908 (Pause-strategy integration into Strategies-detail
                          + Home → Strategies-summary panel)
                                │
                                ▼
                T1910 (Override-risk-veto per-veto button on Strategies-detail
                       + VetoEvent fixture seeding)
                                │
                                ▼
                T1913 (snapshot refresh + ui-designer attestation sub-block — narrow point;
                       9 rename pairs + ~10 net-new + 1 Q1-driven Debug regen;
                       single `cargo insta accept`)
                                │
                                ▼
                T1914 (cross-feature invariants verify — 7 / 7)
                                │
                                ▼
                T1915 (anchor regression + R16.3 grep)
                                │
                                ▼
                T1916 (rust-validate + all 3 bins launch clean)
                                │
                                ▼
                T_FINAL_LUMEN_PHASE_5 (tester gate — VERDICT → presenter on PASS)
```

T1901 is the foundation gate (state additions). After T1901, **five**
tasks fan out in parallel: T1902 (audit writers — separate crate, no
UI dep), T1903 (rename — ripple is mechanical and reviewable in
isolation), T1904 (HumanControl skeleton), T1909 (override modal
skeleton), T1912 (focus_ring skeleton). T1905 / T1906 / T1911 share
T1904's HumanControl skeleton + T1912's focus-ring wrapper. T1907 /
T1908 / T1910 share T1902's audit writers + T1912's focus-ring
wrapper for the destructive button surfaces. T1913 (snapshot accept)
is the narrow point. T1914–T1916 close out before the tester gate.


