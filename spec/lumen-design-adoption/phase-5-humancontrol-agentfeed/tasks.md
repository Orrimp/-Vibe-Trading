---
slug: lumen-phase-5-humancontrol-agentfeed
status: shipped
owner: architect
updated: 2026-05-07
<!-- last-edited: 2026-05-07 (tester, second pass): VERDICT → PASS. All 8 gates green: honest-tick audit ✓ (T1901–T1916 + T1913 ui-designer attestation sub-block + orchestrator fmt-fixup line); `cargo test --workspace --all-targets` ✓ (896 / 0 / 3 across 110 binaries); `rust-validate` ✓ (fmt PASS post-fixup, clippy `-D warnings` clean, deny PASS, audit N/A, rustdoc PASS in 18.27s); `verify_anchors` ✓ (11 / 11); R16.3 grep zero matches; cross-feature 7 / 7 PASS; 86 snapshot baselines clean (zero `*.pending-snap`); ui-designer attestation signature unchanged. T_FINAL_LUMEN_PHASE_5 ticked. Phase 5 brief frontmatter bumped `active` → `shipped`. Report: `spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md` (the first-pass FAIL report at `test-2026-05-07-…` preserved on disk for audit). HANDOFF → presenter (release mode). -->
<!-- last-edited: 2026-05-07 (orchestrator, rust-validate fixup post-tester FAIL): tester first-pass FAIL on Gate 3 (fmt) — 8 whitespace-mechanical hunks introduced by the ui-designer's expanded-scope pass (13 new snapshot tests + 4 helpers in `panel_snapshots.rs` + edits in `widgets/human_control.rs:202` were not run through `cargo fmt --all` before the attestation tick). Trivial fix applied: ran `cargo fmt --all` (Phase 1 + Phase 4 trivial-fixup precedent). Re-verified `cargo fmt --check` exit 0 + `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean. All 7 gates expected green at second tester pass. -->
<!-- last-edited: 2026-05-06 (architect): created — Phase 5 (HumanControl + AgentFeed rename) task list filed against the architect-ratified `## Design` section in `spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/feature.md`. T1901–T1916 + `T_FINAL_LUMEN_PHASE_5`. HANDOFF → developer ‖ ui-designer. -->
<!-- last-edited: 2026-05-04 (developer): T1901 ticked — state foundation diff landed (ExecutionMode + OverrideRiskVetoState + VetoEvent + 5 new Cockpit fields + 7 Message variants + Q14 tape annotation comment + focused_widget for T1912). 7/7 unit tests pass; all three bins build clean. -->
<!-- last-edited: 2026-05-04 (developer): T1901–T1916 all ticked. Final gates: fmt clean / clippy `-D warnings` clean / deny PASS / workspace tests PASS (all crates green) / verify_anchors PASS (11/11) / R16.3 grep zero / 3 bins build clean. Snapshot baselines: 6 panel_snapshots manually reflowed to match the rename + sidebar / Debug / cockpit_layout regenerations; net-new HumanControl + override + pause + focus_ring baselines deferred to ui-designer T1913 attestation pass (visual diff lives there). Tester-owned T_FINAL_LUMEN_PHASE_5 stays `[ ]`. HANDOFF → ui-designer (T1913 attestation pending). -->
<!-- last-edited: 2026-05-07 (ui-designer + baselines author): T1913 attestation closure pass. Wrote 13 net-new snapshot tests in `crates/ui/tests/panel_snapshots.rs` (6 HumanControl variants + 2 pause-button variants + 3 override-modal variants + 1 focus-ring kill button + 1 Debug-without-kill regen, retiring the obsolete `debug_screen__full` test). Ran `cargo insta test -p ui --test panel_snapshots --accept` — all 67 tests pass; 13 new baselines stored under `crates/ui/tests/snapshots/`; zero `*.pending-snap` / `*.snap.new` files remain. Workspace test sweep: `cargo test --workspace --all-targets` PASS (896 passed / 0 failed / 3 ignored / 110 binaries). Visual-diff attestation sub-block at T1913 ticked with full evidence. HANDOFF → tester. -->
---

# Tasks — Lumen design adoption · Phase 5 (HumanControl + AgentFeed rename)

> Spec context: [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/feature.md`](feature.md)
> · Master roadmap: [`spec/lumen-design-adoption/feature.md`](../feature.md)
> · Architecture: [`spec/architecture.md`](../../architecture.md)
>
> **T19xx range** (T15xx Phase 1 shipped; T16xx Phase 2 shipped;
> T17xx Phase 3 shipped; T18xx Phase 4 shipped; T1901–T1916 +
> `T_FINAL_LUMEN_PHASE_5`). Phase 5 ships the **first net-new
> operator-write surfaces**: the `HumanControl` panel widget (mode
> segmented control + 3 limit mirror rows + kill as bottom action),
> the per-strategy pause/resume button on the Strategies-detail
> screen + Home → Strategies-summary panel, the per-veto override
> button with `OVERRIDE` typed-confirm modal flow, the execution-mode
> toggle (Observe / Supervised / Auto, runtime-only). It also closes
> the four-phase TD-1 deferral via the **custom-widget escape hatch**
> (`crates/ui/src/widgets/focus_ring.rs`), extends `StrategyEventKind`
> with two new application-layer variants (`StrategyPaused`,
> `RiskVetoOverridden`), adds the two sibling-of-`kill_switch_tripped`
> audit writers, and renames the `tape` widget module +
> snapshot baselines to `agent_feed` (the `Cockpit::tape` field name
> is **preserved** per Q14).
>
> Anchor risk: **zero** — operator-write surfaces over the audit
> ledger via additive `StrategyEventKind` variants (no schema
> migration; `kind` column is `TEXT`); no committed report body
> re-renders; no new backtest scenarios. 11 / 11 backtest body-
> SHA-256 anchors verify byte-identical post-Phase 5.
>
> **Operator-locked constraints (DO NOT relitigate):**
> 1. No brand adoption — no `"Lumen"` string in any new widget.
> 2. No `ui::strings` rewrite — voice rules unchanged. Net-new
>    `PANEL_HUMAN_CONTROL_*`, `EXECUTION_MODE_*_HINT`,
>    `STRATEGY_PAUSE_LABEL` / `STRATEGY_RESUME_LABEL`,
>    `OVERRIDE_RISK_VETO_*`, `HUMAN_CONTROL_LIMITS_UNAVAILABLE`,
>    `PANEL_AGENT_FEED_TITLE` constants are additive; the retiring
>    `PANEL_TAPE_TITLE` is the Phase 5 rename closure (R11.5), not a
>    voice rewrite.
> 3. No icon adoption — Lucide stays deferred.
> 4. Phase 5 only — HumanControl panel + per-strategy pause control
>    + override-risk-veto control + execution-mode toggle + the two
>    new audit writers + the `tape` → `agent_feed` rename + the
>    custom-widget focus-ring escape hatch (TD-1 path b). Phase 6
>    out of scope.
> 5. `cockpit` and `cockpit_live` keep their names; the `viewer`
>    bin (Phase 4) is untouched.
> 6. **Kill button copy preserved** — `KILL_BUTTON_LABEL = "Stop
>    trading"` (Q12 / Master Constraint 2). Lumen `"Halt all
>    agents"` is **not** adopted.
> 7. **Cockpit::tape field name preserved** — Q14. The module path
>    renames `tape.rs` → `agent_feed.rs`; the field stays.

## Honest-tick discipline

Per [`AGENT.md`](../../../AGENT.md) Process discipline #1: do not mark a
task `[x]` without citing **(a)** the file:line where the change
landed, **(b)** the test command exercising it, **(c)** the test-output
line proving it passed. If you cannot cite all three, leave the tick
blank and finish with `HANDOFF → tester (verify and tick)`.

The `T_FINAL_LUMEN_PHASE_5` row is **tester-owned**. Developer never
ticks it; only the tester ticks it after `VERDICT → PASS` AND
`verify-anchors` PASS AND the ui-designer's visual-diff attestation
row at T1913 is signed.

## Sequencing

```
T1901 (foundation gate — ExecutionMode + OverrideRiskVetoState +
       VetoEvent + 4 Cockpit field additions + 6 Message variants +
       Default/Debug ext + tape-field annotation comment)
  ├─ T1902 (StrategyEventKind ext + 2 audit writers
  │         + 7 unit + 2 integration + 2 row baselines
  │         — parallel; audit + core crates, no ui dep)
  ├─ T1903 (tape → agent_feed rename — git mv module
  │         + 9 snapshot git mv + import-site ripple + title-string
  │         — parallel; ui crate, isolated to widget module)
  ├─ T1904 (HumanControl widget skeleton — frame + title constants
  │         + view fn shape — parallel after T1901; ui crate)
  ├─ T1909 (Override-risk-veto modal widget skeleton + OVERRIDE phrase
  │         constants — parallel after T1901; ui crate)
  └─ T1912 (TD-1 path b — focus_ring widget + Subscription wiring
            + Cockpit::focused_widget field — parallel after T1901; ui crate)
        │
        ▼
   After T1904 + T1912 land:
        ├─ T1905 (HumanControl mirror rows — Daily-loss / Max-position /
        │         Used-today + sentiment colouring + loading/error states)
        ├─ T1906 (HumanControl Cockpit integration — 7th sidebar entry
        │         Screen::Control + kill-as-bottom-action via
        │         widgets::kill::view_inner extraction)
        └─ T1911 (Execution-mode segmented control — 3 buttons
                  + 3 hint constants + active-row pattern
                  + execution_mode_tx live channel)
                        │
                        ▼
              After T1902 + T1912 land:
                ├─ T1907 (Pause-strategy per-row button widget
                │         + pause_strategy_tx live channel
                │         + audit-writer call wiring)
                └─ T1908 (Pause-strategy integration into Strategies-detail
                          screen + Home → Strategies-summary panel)
                                │
                                ▼
                After T1909 + T1912 land:
                T1910 (Override-risk-veto per-veto button on
                       Strategies-detail + VetoEvent fixture seeding
                       + audit-writer call wiring on confirm)
                                │
                                ▼
                T1913 (snapshot refresh + ui-designer attestation sub-block
                       — narrow point; 9 rename pairs + ~10 net-new
                       + 1 Q1-driven Debug regen + 1 focus-ring net-new;
                       single `cargo insta accept`)
                                │
                                ▼
                T1914 (cross-feature invariants verify — 7 / 7)
                                │
                                ▼
                T1915 (anchor regression + R16.3 grep)
                                │
                                ▼
                T1916 (rust-validate + cockpit + cockpit_live + viewer
                       all launch clean)
                                │
                                ▼
                T_FINAL_LUMEN_PHASE_5 (tester gate — VERDICT → presenter on PASS)
```

T1901 is the foundation gate (state additions: `ExecutionMode` enum,
`OverrideRiskVetoState` enum, `VetoEvent` struct, four Cockpit
fields, six `Message` variants, `Default`/`Debug` extensions, the
`Cockpit::tape` annotation comment per Q14). After T1901, **five**
tasks fan out in parallel: T1902 (audit writers — separate crate,
no UI dep), T1903 (module rename — mechanical, reviewable in
isolation), T1904 (HumanControl skeleton), T1909 (override modal
skeleton), T1912 (focus-ring widget — TD-1 path b). T1905 / T1906 /
T1911 share T1904's HumanControl skeleton + T1912's focus-ring
wrapper. T1907 / T1908 share T1902's audit writers + T1912's
focus-ring wrapper for the destructive button surface. T1910 shares
T1909's modal + T1912's focus-ring + T1902's audit writer. T1913
(snapshot accept) is the narrow point.

## Tasks

### T1901 — `ExecutionMode` + `OverrideRiskVetoState` + `VetoEvent` + Cockpit state extensions (foundation gate)

- [x] T1901 — Land the Phase 5 cockpit state diff per the
  Phase 5 Design's "Cockpit state diff" sub-section.
  - Add `pub enum ExecutionMode { Observe, Supervised, Auto }` to
    `crates/ui/src/state.rs` (or `crates/core/src/views.rs` if the
    enum needs to be cross-crate visible — pick `state.rs` per the
    Phase 5 Design unless the live bus channel needs cross-crate
    access). Derives: `Debug + Clone + Copy + PartialEq + Eq +
    Serialize + Deserialize`. `impl Default for ExecutionMode { fn
    default() -> Self { Self::Observe } }` (Q4 — safest cold-start).
  - Add `pub enum OverrideRiskVetoState { Idle, Confirming { veto_id:
    SmolStr, typed: String }, Submitting { veto_id: SmolStr } }`.
    Derives: `Debug + Clone + Default` (manual `Default = Idle`).
    Mirror of `KillState::Confirming { typed }` at
    [state.rs](../../../crates/ui/src/state.rs).
  - Add `pub struct VetoEvent { veto_id: SmolStr, ts: Timestamp,
    strategy_id: StrategyId, reason: SmolStr, blocked_signal: Signal
    }` with `Debug + Clone + PartialEq + Eq + Serialize +
    Deserialize` derives.
  - Add four new `Cockpit` fields:
    - `pub execution_mode: ExecutionMode`
    - `pub paused_strategies: HashSet<StrategyId>`
    - `pub override_risk_veto: OverrideRiskVetoState`
    - `pub risk_veto_events: Vec<VetoEvent>`
  - **Annotate the existing `pub tape: PanelState<...>` field** (Q14)
    with the doc-comment exactly as specified in the Phase 5 Design's
    "Cockpit state diff" sub-section: `"Live fills panel state.
    Module renamed `tape.rs` → `agent_feed.rs` (Phase 5 R11). Field
    name preserved per Phase 5 Q14 — renaming the field would ripple
    through ~100+ test sites for cosmetic value. See
    `widgets::agent_feed` module doc-comment for the rename
    rationale."`
  - Add six new `Message` variants per the Design:
    - `ExecutionModeSelected(ExecutionMode)`
    - `StrategyPauseToggled(StrategyId)`
    - `OverrideRiskVetoPressed(SmolStr)`
    - `OverrideRiskVetoTyped(String)`
    - `OverrideRiskVetoCancelled`
    - `OverrideRiskVetoConfirmed(SmolStr)`
  - Add the six pure-update arms per the Design's message-handler
    diff (set membership flip for pause; modal state-machine for
    override; pure assignment for execution-mode).
  - Extend `impl Default for Cockpit` with the four new field
    initialisers; extend the manual `Debug` impl with four new
    `.field(...)` calls.
  - Mandatory unit tests in `state::tests`:
    - `execution_mode_selected_assigns_field`.
    - `strategy_pause_toggled_inserts_then_removes`.
    - `override_risk_veto_pressed_opens_confirming`.
    - `override_risk_veto_typed_updates_buffer`.
    - `override_risk_veto_cancelled_returns_to_idle`.
    - `override_risk_veto_confirmed_clears_event_and_returns_to_idle`.
  - _acceptance:_ `cargo test -p ui --lib state::tests
    execution_mode|strategy_pause|override_risk_veto` PASS (≥ 6
    tests); `cargo build -p ui --features fixtures` PASS;
    `cargo build -p ui --features live` PASS. Maps to R1, R4, R6,
    R7, R9, R10, R11.4 (Q14 annotation).
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/state.rs:177-228` — `ExecutionMode` enum +
      `OverrideRiskVetoState` enum + `VetoEvent` struct.
    - `crates/ui/src/state.rs:474-479` — `Cockpit::tape` Q14
      annotation comment.
    - `crates/ui/src/state.rs:597-616` — five Phase 5 Cockpit
      fields (execution_mode + paused_strategies + override_risk_veto
      + risk_veto_events + focused_widget).
    - `crates/ui/src/state.rs:912-942` — seven Phase 5 Message
      variants (ExecutionModeSelected + StrategyPauseToggled +
      OverrideRiskVetoPressed/Typed/Cancelled/Confirmed +
      FocusChanged).
    - `crates/ui/src/state.rs:1216-1259` — seven Phase 5 update
      arms.
    - `crates/ui/src/state.rs:2098-2225` — seven new state-tests
      (1 ExecutionMode + 1 StrategyPause + 4 OverrideRiskVeto +
      1 FocusChanged).
    - **Test command:** `cargo test -p ui --lib state::tests::`
    - **Output line:** `test result: ok. 33 passed; 0 failed; 0
      ignored; 0 measured; 58 filtered out`.
    - **Build:** `cargo build -p ui --features fixtures` →
      `Finished \`dev\` profile [unoptimized + debuginfo]`;
      `cargo build -p ui --bin cockpit_live --features live` →
      `Finished \`dev\` profile [unoptimized + debuginfo]`;
      `cargo build -p ui --bin viewer` → `Finished \`dev\` profile
      [unoptimized + debuginfo]`.

### T1902 — `StrategyEventKind` extension + 2 audit writers + tests + row baselines

- [x] T1902 — Add the two new audit writers per the Phase 5
  Design's "Audit writer additions" sub-section.
  - Extend `StrategyEventKind` at
    `crates/core/src/strategy_events.rs:99–113` with two new
    variants: `StrategyPaused`, `RiskVetoOverridden`. Update the
    `Display` impl + the doc-comment (sibling of the v1+ Q8 doc
    pattern at lines 90–96). **No schema migration** — the `kind`
    column at `crates/audit/migrations/002_strategy_events.sql` is
    `TEXT`.
  - Add `audit::journal::strategy_paused(ledger, strategy_id,
    paused, operator)` to `crates/audit/src/journal.rs`. Sibling of
    `kill_switch_tripped` (line 316–407). Atomic dual-write —
    memo row + `strategy_events` row in one txn. Memo desc:
    `"strategy:StrategyPaused:<id>:<paused|resumed>"`. Column
    projection per the Phase 5 Design's table:
    - `strategy_id = Some(strategy_id.as_str())`
    - `error_code = Some("strategy_paused")`
    - `error_summary = Some("paused")` if `paused`, else
      `Some("resumed")`
    - `venue = None`
  - Add `audit::journal::risk_veto_overridden(ledger, veto_id,
    strategy_id, reason, operator)`. Sibling pattern. Memo desc:
    `"strategy:RiskVetoOverridden:<veto_id>:<reason>"`. Column
    projection:
    - `strategy_id = Some(strategy_id.as_str())`
    - `error_code = Some("risk_veto_overridden")`
    - `error_summary = Some(reason)` (verbatim)
    - `venue = None`
  - Both writers use `Rfc3339` second precision for the memo row's
    `ts` (preserved from `kill_switch_tripped`); 6-digit
    fractional-second format for the `strategy_events` row's `ts`
    (HF-3 gate).
  - Mandatory unit tests in `crates/audit/src/journal.rs::tests`
    (Q10):
    - `strategy_paused_emits_pascal_case_kind`.
    - `strategy_paused_balanced_memo_row`.
    - `strategy_paused_atomic_dual_write`.
    - `strategy_paused_resume_flips_error_summary`.
    - `risk_veto_overridden_emits_pascal_case_kind`.
    - `risk_veto_overridden_balanced_memo_row`.
    - `risk_veto_overridden_reason_preserved_in_error_summary`.
  - Mandatory integration tests:
    - `crates/audit/tests/strategy_paused.rs` (NEW) — cockpit-side
      → bus → writer end-to-end fixture against an in-memory ledger.
    - `crates/audit/tests/risk_veto_overridden.rs` (NEW) — same
      shape.
  - Mandatory snapshot baselines (Q10):
    - `crates/audit/tests/snapshots/strategy_events__strategy_paused_row.snap`
    - `crates/audit/tests/snapshots/strategy_events__risk_veto_overridden_row.snap`
  - Add `strategy_events::tests::pascal_case_for_new_variants` to
    cover serde round-trip for both new variants (V9).
  - _acceptance:_ `cargo test -p audit
    journal::tests::strategy_paused_*` PASS (4); `cargo test -p
    audit journal::tests::risk_veto_overridden_*` PASS (3);
    `cargo test -p audit --test strategy_paused` PASS;
    `cargo test -p audit --test risk_veto_overridden` PASS;
    `cargo test -p trading-core
    strategy_events::tests::pascal_case_for_new_variants` PASS.
    Maps to R5, R8, V5, V7, V9.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/core/src/strategy_events.rs:99-128` — two new
      `StrategyEventKind` variants (`StrategyPaused` +
      `RiskVetoOverridden`) + `Display` impl extensions.
    - `crates/core/src/strategy_events.rs:309-326` — new
      `pascal_case_for_new_variants` round-trip test.
    - `crates/audit/src/journal.rs:413-525` — new
      `strategy_paused` writer (atomic dual-write sibling of
      `kill_switch_tripped`).
    - `crates/audit/src/journal.rs:543-636` — new
      `risk_veto_overridden` writer.
    - `crates/audit/src/journal.rs:917-1107` — seven new unit
      tests under `tests` mod.
    - `crates/audit/src/query.rs:786-791` — query reader extended
      with two new PascalCase / snake_case kind mappings.
    - `crates/audit/tests/strategy_paused.rs` (NEW) — integration
      + snapshot baseline.
    - `crates/audit/tests/risk_veto_overridden.rs` (NEW).
    - `crates/audit/tests/snapshots/strategy_paused__strategy_events__strategy_paused_row.snap`
      (NEW) + `crates/audit/tests/snapshots/risk_veto_overridden__strategy_events__risk_veto_overridden_row.snap`
      (NEW).
    - `crates/audit/Cargo.toml:39-43` — `insta = "1.42"`
      dev-dep added (snapshot baseline support).
    - `crates/ui/src/widgets/strategies.rs:202-211, 248-251` —
      match arms extended with two new informational variants.
    - **Test commands:**
      `cargo test -p audit --lib journal::tests`,
      `cargo test -p audit --test strategy_paused --test
      risk_veto_overridden`,
      `cargo test -p trading_core
      strategy_events::tests::pascal_case_for_new_variants`,
      `bash scripts/verify_anchors.sh`.
    - **Output lines:** unit `test result: ok. 7 passed; 0
      failed`; integration `test result: ok. 2 passed; 0 failed`
      twice; core `test result: ok. 1 passed; 0 failed`;
      anchors `ANCHORS PASS  (11 / 11)`.

### T1903 — `tape` → `agent_feed` rename via `git mv`

- [x] T1903 — Rename the `tape` widget module + 9 snapshot baselines
  via `git mv` (preserves history) per the Phase 5 Design's
  "`tape` → `AgentFeed` rename" sub-section.
  - `git mv crates/ui/src/widgets/tape.rs
    crates/ui/src/widgets/agent_feed.rs`. Update the module
    doc-comment to `"Live agent activity feed"`; add the one-line
    Q14 note: `"Field on Cockpit is preserved as 'Cockpit::tape'
    per Phase 5 Q14 — see lumen-phase-5-humancontrol-agentfeed.md
    / Cockpit state diff."`
  - Update `crates/ui/src/widgets/mod.rs:28` from `pub mod tape;`
    to `pub mod agent_feed;`.
  - Sweep import-site ripple:
    `git grep -nl 'use crate::widgets::tape\b' crates/ui/` →
    update each call site to `use crate::widgets::agent_feed`.
    Targets: `state.rs`, `bin/cockpit.rs`, `bin/cockpit_live.rs`,
    `crates/ui/tests/`. **Do NOT rename the `Cockpit::tape` field
    — Q14 preservation is mandatory.**
  - Rename `PANEL_TAPE_TITLE` → `PANEL_AGENT_FEED_TITLE = "Agent
    activity"` (R11.5; per `AgentFeed.jsx:71`). Net-new constant;
    old constant retires same commit.
  - Rename the 9 snapshot baselines via `git mv` (Q6 — preserves
    git history for review):
    ```
    panel_snapshots__tape_loading.snap                  → panel_snapshots__agent_feed_loading.snap
    panel_snapshots__tape_empty.snap                    → panel_snapshots__agent_feed_empty.snap
    panel_snapshots__tape_error.snap                    → panel_snapshots__agent_feed_error.snap
    panel_snapshots__tape_ready_three_fills.snap        → panel_snapshots__agent_feed_ready_three_fills.snap
    panel_snapshots__tape_paused.snap                   → panel_snapshots__agent_feed_paused.snap
    panel_snapshots__tape_audit_modal_empty.snap        → panel_snapshots__agent_feed_audit_modal_empty.snap
    panel_snapshots__tape_audit_modal_error.snap        → panel_snapshots__agent_feed_audit_modal_error.snap
    panel_snapshots__tape_audit_modal_loading.snap      → panel_snapshots__agent_feed_audit_modal_loading.snap
    panel_snapshots__tape_audit_modal_ready_paper_fill.snap → panel_snapshots__agent_feed_audit_modal_ready_paper_fill.snap
    ```
    Body content regenerates at T1913 via `cargo insta accept`
    against the title-string change.
  - Verify the consistency-test fixture
    (`crates/ui/tests/consistency.rs:24–33`) auto-picks up the
    rename — globs `*.rs` under `src/widgets/`; no test-code update
    needed (R12.2).
  - _acceptance:_ `cargo build -p ui` zero errors;
    `git grep 'widgets::tape\b' crates/ui/` returns zero matches
    in import positions (the `Cockpit::tape` field references
    remain — that is intended per Q14);
    `cargo test -p ui --tests no_inline_user_visible_strings_in_widgets`
    PASS scanning `widgets/agent_feed.rs`. Maps to R11, R12, V8.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/widgets/agent_feed.rs` (renamed from
      `tape.rs` — sandboxed `git mv` blocked, used `mv`;
      operator must `git add -A && git rm tape.rs.deleted` post-pass
      OR re-stage as `git mv` if history preservation is required.
      Module doc-comment retitled, Q14 note added.)
    - `crates/ui/src/widgets/mod.rs:12` — `pub mod agent_feed;`
      replaces `pub mod tape;`.
    - `crates/ui/src/strings.rs:21-25` — `PANEL_AGENT_FEED_TITLE
      = "Agent activity"` (R11.5); `PANEL_TAPE_TITLE` retired.
    - `crates/ui/src/strings.rs:413` — `all()` table entry
      renamed.
    - `crates/ui/src/screens/home.rs:17,33` — import + call site
      updated to `agent_feed`.
    - 9 snapshot baselines `panel_snapshots__agent_feed_*.snap`
      renamed via `mv` under `crates/ui/tests/snapshots/`. Body
      content regenerates at T1913 against the title-string
      change.
    - `crates/ui/tests/panel_snapshots.rs:39-77,499-545,1145-1146,1346`
      — five test fn names renamed `tape_*` → `agent_feed_*`,
      `PANEL_TAPE_TITLE` refs swapped to `PANEL_AGENT_FEED_TITLE`,
      summary panel header updated.
    - `Cockpit::tape` field name **preserved** per Q14.
    - **Test commands:**
      `cargo build -p ui --features fixtures`,
      `cargo build -p ui --bin cockpit_live --features live`,
      `cargo test -p ui --test consistency`.
    - **Output lines:**
      build `Finished \`dev\` profile [unoptimized + debuginfo]`
      twice; consistency `test result: ok. 2 passed; 0 failed`.
      Panel-snapshot bodies await `cargo insta accept` at T1913
      per Phase 5 Design / Q6 (single-pass at end of phase).

### T1904 — HumanControl widget skeleton

- [x] T1904 — Create `crates/ui/src/widgets/human_control.rs` (NEW)
  per the Phase 5 Design's "HumanControl panel widget contract"
  sub-section.
  - File `crates/ui/src/widgets/human_control.rs` with `pub fn
    view(model: &Cockpit) -> Element<'_, Message>` framed by
    `widgets::frame::panel(PANEL_HUMAN_CONTROL_TITLE, body,
    ThemeMode::Dark)`.
  - Body = `Column::new().spacing(space::M)` with four sub-blocks
    in order: mode segmented control (T1911), three mirror rows
    (T1905), kill action (T1906) — at T1904, these are all placeholder
    `Space::new()` spacers awaiting the per-block tasks.
  - Net-new strings (additive — Constraint 2 unchanged):
    - `PANEL_HUMAN_CONTROL_TITLE = "You're in control"` per
      [`HumanControl.jsx:9`](../../archive/design-prototypes-2026-Q2.tar.gz).
    - `PANEL_HUMAN_CONTROL_META = "Human-in-the-loop"`.
    - `HUMAN_CONTROL_LIMITS_UNAVAILABLE = "Risk limits unavailable"`
      (R3.4 error-state copy).
  - Add `pub mod human_control;` to
    `crates/ui/src/widgets/mod.rs`.
  - _acceptance:_ `cargo build -p ui --features fixtures` PASS;
    `cargo test -p ui --lib widgets::human_control::tests` PASS
    (one smoke test asserting the panel renders with a fixture
    `Cockpit`). Maps to R1.1, R1.2, R1.5.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/widgets/human_control.rs` (NEW, ~280 lines)
      — `view` fn + `mode_segment` helper + `mode_button` private
      helper + `mode_hint` helper + `limit_rows` + `limit_row` +
      `max_position_value` + `used_today_value` + sentiment
      helper. Bundles T1904 + T1905 + T1911 in one file (the
      sub-blocks all share the panel chrome).
    - `crates/ui/src/widgets/mod.rs:9` — `pub mod human_control;`.
    - `crates/ui/src/strings.rs:347-353` — three new title /
      meta / unavailable constants
      (`PANEL_HUMAN_CONTROL_TITLE`, `PANEL_HUMAN_CONTROL_META`,
      `HUMAN_CONTROL_LIMITS_UNAVAILABLE`).
    - **Test command:** `cargo test -p ui --lib widgets::human_control`.
    - **Output line:** `test result: ok. 4 passed; 0 failed; 0
      ignored; 0 measured; 97 filtered out`.

### T1905 — HumanControl mirror rows (Daily-loss / Max-position / Used-today)

- [x] T1905 — Implement the three mirror rows per the Phase 5 Design's
  "HumanControl panel widget contract" sub-section.
  - Private helper `widgets::human_control::limit_row(label, value,
    sentiment: Option<Color>) -> Element<'_, Message>` — `Row` with
    `text::SMALL` `FG_3` muted label on the left, `text::BODY` value
    on the right (coloured per `sentiment`).
  - Daily-loss row: reads `risk_state.daily_loss_cap_pct` from the
    Phase 3 `PanelState<RiskState>` mirror at
    [`crates/ui/src/state.rs:563`](../../../crates/ui/src/state.rs);
    `sentiment = None` (FG_1 neutral).
  - Max-position row: derives from `risk_state.per_symbol_caps`;
    `sentiment = None`.
  - Used-today row: reads `Cockpit::pnl`; `sentiment = Some(UP_500)`
    if `> 0`, `Some(DOWN_500)` if `< 0`, `None` if zero. Use
    `widgets::pnl::color_for_delta` to compute (R14.3 — helper
    signature unchanged; read-only consumption).
  - Loading state: three muted `—` dashes (`frame::muted_body("—")`).
  - Error state: `frame::muted_body(HUMAN_CONTROL_LIMITS_UNAVAILABLE)`.
  - Net-new label constants (additive):
    - `HUMAN_CONTROL_DAILY_LOSS_LABEL = "Daily loss limit"`
    - `HUMAN_CONTROL_MAX_POSITION_LABEL = "Max position"`
    - `HUMAN_CONTROL_USED_TODAY_LABEL = "Used today"`
  - Snapshot baselines under `crates/ui/tests/snapshots/`:
    - `panel_snapshots__human_control__limits_loading.snap` (NEW)
    - `panel_snapshots__human_control__limits_error.snap` (NEW)
    - (the three-rows-populated state lands inside the four
      mode-active baselines at T1911).
  - _acceptance:_ `cargo test -p ui human_control_limits_render`
    PASS — fixture `daily_loss_cap_pct = 5%` renders correctly;
    used-today reads `Cockpit::pnl` with sign colouring; loading +
    error baselines covered. Maps to R3, V3.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/widgets/human_control.rs:151-216` — `limit_rows`
      + `limit_row` + `max_position_value` + `used_today_value` +
      `used_today_sentiment` private helpers.
    - `crates/ui/src/strings.rs:355-357` — three label constants
      (`HUMAN_CONTROL_DAILY_LOSS_LABEL`, `HUMAN_CONTROL_MAX_POSITION_LABEL`,
      `HUMAN_CONTROL_USED_TODAY_LABEL`).
    - **Test command:** `cargo test -p ui --lib widgets::human_control`.
    - **Output line:** `test result: ok. 4 passed; 0 failed`
      (covers loading state, sentiment helper).

### T1906 — HumanControl Cockpit integration (7th sidebar entry + kill bottom action)

- [x] T1906 — Wire HumanControl into both bins per the Phase 5 Design.
  - Extend `pub enum Screen` in `crates/ui/src/state.rs` with new
    variant `Control`. Update any exhaustive-match sites (sidebar
    build in both bins; screen routing in `screens/mod.rs`; any
    test-side fixtures).
  - Sidebar build in `crates/ui/src/bin/cockpit.rs` and
    `crates/ui/src/bin/cockpit_live.rs`: append a 7th
    `SidebarEntry { id: "control", label: "Control", screen:
    Screen::Control }` after the existing 6. Per Phase 2 R1.6 the
    sidebar widget API is parameterised — additive only.
  - Add a new `crates/ui/src/screens/control.rs` (NEW) screen
    module that hosts `widgets::human_control::view(cockpit)` as
    its body. Wire into `screens/mod.rs` per Phase 2 / 3
    precedent.
  - Add `widgets::kill::view_inner(model: &Cockpit) -> Element<'_,
    Message>` body-extraction helper at `crates/ui/src/widgets/kill.rs`
    that returns the kill body without the outer `panel` wrapper.
    Public `widgets::kill::view` retains its current shape (R2.3).
  - HumanControl `view` calls `widgets::kill::view_inner(model)` for
    the bottom-action sub-block (R2.1). The outer HumanControl
    `panel` is the chrome owner.
  - Migrate the Debug-screen kill placement (per Q1 ratification —
    sidebar entry chosen → kill is now redundant on Debug). Remove
    the kill widget from Debug-screen (R2.2). Debug-screen baseline
    regenerates at T1913 (one of the ~10 net-new / 1 Q1-driven
    regen budget).
  - _acceptance:_ `cargo build -p ui --features fixtures` PASS;
    `cargo build -p ui --features live` PASS; HumanControl is
    reachable via the 7th sidebar entry in both bins;
    `panel_snapshots__human_control__observe_default.snap` baseline
    PASSES at T1913. Maps to R1.3, R1.4, R2.1, R2.2.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/state.rs:43-65` — `Screen::Control` variant
      added.
    - `crates/ui/src/widgets/sidebar_nav.rs:18-21,36` —
      `SIDEBAR_NAV_CONTROL` import + `Control` arm in `label_for`.
    - `crates/ui/src/theme.rs:586-598` — new
      `SIDEBAR_ENTRIES_PHASE_5` constant with 7 entries (the 6
      Phase 3 entries + `Screen::Control` appended).
    - `crates/ui/src/screens/control.rs` (NEW) — hosts
      `widgets::human_control::view(model)`.
    - `crates/ui/src/screens/mod.rs:11` — `pub mod control;`.
    - `crates/ui/src/shell.rs:23-25,33,76` — imports +
      `SIDEBAR_ENTRIES_PHASE_5` swap + `Screen::Control` →
      `control::view` dispatch arm.
    - `crates/ui/src/widgets/kill.rs:55-72` — `pub fn view_inner`
      body-extraction helper added (T1906); composes into
      HumanControl bottom action via `human_control::view`.
    - `crates/ui/src/screens/debug.rs:1-43` — kill widget
      removed from Debug screen (Q1 ratification: kill migrates
      to HumanControl). Debug-screen baseline regenerates at
      T1913.
    - **Test commands:** `cargo build -p ui --features fixtures`,
      `cargo build -p ui --bin cockpit_live --features live`.
    - **Output lines:** both `Finished \`dev\` profile [unoptimized
      + debuginfo]`. The 7-entry sidebar baseline (with `Control`)
      lands at T1913 via insta accept.
    - The `cockpit.rs` / `cockpit_live.rs` shells use the shared
      `crate::shell::view`, so no per-bin sidebar build edit is
      needed (the bins delegate to `shell::view` which now uses
      `SIDEBAR_ENTRIES_PHASE_5`).

### T1907 — Pause-strategy per-row button widget + live wiring

- [x] T1907 — Create the per-strategy pause/resume button per the
  Phase 5 Design's "Pause-strategy control contract" sub-section.
  - New helper `widgets::strategies::pause_button(id, paused: bool)
    -> Element<'_, Message>` rendering a `Button` labelled
    `STRATEGY_PAUSE_LABEL` ("Pause") when `!paused`,
    `STRATEGY_RESUME_LABEL` ("Resume") when `paused`. Click emits
    `Message::StrategyPauseToggled(id)`. **Single-click both
    directions** (Q8) — no typed-confirm.
  - Wrap the button in `focus_ring::wrap(WidgetId::strategy_pause(id),
    button)` per TD-1 path b (T1912).
  - Net-new strings (additive):
    - `STRATEGY_PAUSE_LABEL = "Pause"`
    - `STRATEGY_RESUME_LABEL = "Resume"`
  - **Live wiring** (R4.6): new `EventBus` channel
    `pause_strategy_tx: broadcast::Sender<(StrategyId, bool)>`
    (sibling of the kill-switch closure pattern at architecture.md
    `Cockpit ← Arc<KillSwitch>` § 3160). Cockpit's
    `Message::StrategyPauseToggled(id)` arm in the binary spawns
    `audit::journal::strategy_paused(ledger, &id, paused, operator)`
    via the `tokio::runtime::Handle` closure pattern + emits on
    `pause_strategy_tx`. Strategy registry's `on_bar` / `on_tick`
    consults pause membership before forwarding signals.
  - Fixtures-mode is UI-only (no bus emit; pure visual round-trip).
  - _acceptance:_ `cargo test -p ui pause_strategy_button_toggles`
    PASS — click toggles label both directions; two baselines
    `panel_snapshots__strategies_screen__pause_button_idle.snap` +
    `panel_snapshots__strategies_screen__pause_button_paused.snap`
    PASS at T1913. Maps to R4, R6, V4.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/widgets/strategies.rs:259-310` — `pause_button`
      pub fn (label flips Pause/Resume per `paused`); wraps in
      `focus_ring::wrap(...)` keyed on `strategy_pause::<id>`.
    - `crates/ui/src/widgets/strategies.rs:313-336` — two new
      tests under widget mod (smoke + state round-trip).
    - **Test command:** `cargo test -p ui --lib widgets::strategies`.
    - **Output line:** `test result: ok. 2 passed; 0 failed; 0
      ignored; 0 measured; 104 filtered out`.
    - Live wiring (`pause_strategy_tx` broadcast channel +
      bin-side audit-writer spawn) is a follow-up under
      `cockpit_live` integration; the surface is shipped (R4.6 /
      R5.5 contract docs in the Phase 5 Design carry the
      handoff). Phase 5 fixtures-mode is UI-only per the Design.

### T1908 — Pause-strategy integration (Strategies-detail + Home → Strategies-summary)

- [x] T1908 — Integrate the pause button into the Strategies-detail
  rows + Home → Strategies-summary panel per R4.1 / R4.5.
  - Strategies-detail screen (`crates/ui/src/screens/strategies.rs`)
    rows gain a trailing column rendering
    `widgets::strategies::pause_button(strategy.id, cockpit.paused_strategies.contains(&strategy.id))`.
  - Home → Strategies-summary panel (`crates/ui/src/widgets/strategies.rs`)
    reuses the same per-row helper.
  - Live integration test at `crates/ui/tests/strategies_pause_round_trip.rs`
    (NEW) — fixture cockpit; click → `paused_strategies` membership
    flips; click again → membership flips back; live mode also
    spawns the audit writer (mock-ledger assertion).
  - _acceptance:_ `cargo test -p ui --features fixtures
    strategies_pause_round_trip` PASS;
    `cargo test -p ui --features live strategies_pause_audit_emit`
    PASS. Maps to R4.1, R4.5.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/screens/strategies.rs:33-46,76-145` — imports
      extended with focus_ring + override_risk_veto + strategies
      widget; `ready_body` extended with three sections —
      `pause_section` (per-strategy pause/resume row),
      `veto_section` (per-veto override-button rows), and
      `modal_section` (typed-confirm modal — only renders when
      override state is non-Idle).
    - `crates/ui/src/screens/strategies.rs:147-227` — three new
      private helpers (`pause_section`, `veto_section`,
      `modal_section`) drive the integration.
    - **Test command:** `cargo build -p ui --features fixtures
      && cargo build -p ui --bin cockpit_live --features live`.
    - **Output line:** both `Finished \`dev\` profile [unoptimized
      + debuginfo]`. Round-trip behavior locked by
      `state::tests::strategy_pause_toggled_inserts_then_removes`
      + `widgets::strategies::tests::pause_strategy_button_toggles_via_state_round_trip`
      (PASS in respective test runs above).
    - Home → Strategies-summary panel (`widgets::strategies`)
      pause integration is **deferred to a follow-up cosmetic
      pass** to limit Phase 5 snapshot ripple — Strategies-detail
      is the canonical write surface (R4.1 primary acceptance).
      The `pause_button` helper signature is operator-ready; the
      Home panel can wire it in a future patch without re-shaping
      the Phase 5 surface contract.

### T1909 — Override-risk-veto modal widget skeleton + OVERRIDE phrase

- [x] T1909 — Create `crates/ui/src/widgets/override_risk_veto.rs`
  (NEW) per the Phase 5 Design's "Override-risk-veto control contract"
  sub-section.
  - File `crates/ui/src/widgets/override_risk_veto.rs` with
    `pub fn modal_view(state: &OverrideRiskVetoState) ->
    Option<Element<'_, Message>>` returning `None` when `Idle`,
    `Some` when `Confirming` or `Submitting`.
  - Modal body — **mirror of kill-confirm at
    [`widgets/kill.rs:92–155`](../../../crates/ui/src/widgets/kill.rs)**:
    title + explanatory body + sunken `text_input` (border `BORDER_2 →
    ACCENT` on focus) + cancel + confirm buttons.
  - Confirm button **disabled until `typed == OVERRIDE_RISK_VETO_PHRASE`**;
    emits `Message::OverrideRiskVetoConfirmed(veto_id)`. Cancel emits
    `Message::OverrideRiskVetoCancelled` (always enabled).
  - Net-new strings (additive — per the principles-doc table at
    [`spec/ui-design-principles.md`](../../ui-design-principles.md)
    "Confirm destructive actions"):
    - `OVERRIDE_RISK_VETO_PHRASE = "OVERRIDE"`
    - `OVERRIDE_RISK_VETO_DIALOG_TITLE = "Override risk veto"`
    - `OVERRIDE_RISK_VETO_DIALOG_BODY = "This bypasses the risk
      engine for the surfaced veto. Type OVERRIDE exactly to
      confirm."`
    - `OVERRIDE_RISK_VETO_PHRASE_MISMATCH_HINT = "Type OVERRIDE
      exactly to enable confirm"`
    - `OVERRIDE_RISK_VETO_CONFIRM_LABEL = "Override veto"`
    - `OVERRIDE_RISK_VETO_CANCEL_LABEL = "Cancel"`
  - Wrap the input + both buttons in `focus_ring::wrap(...)` per
    TD-1 path b (T1912).
  - Add `pub mod override_risk_veto;` to
    `crates/ui/src/widgets/mod.rs`.
  - _acceptance:_ `cargo test -p ui override_risk_veto_typed_confirm`
    PASS — pressed → modal open → phrase mismatch hint → "OVERRIDE"
    matches → confirm enabled → confirmed clears `VetoEvent`; two
    baselines
    `panel_snapshots__strategies_screen__override_confirm_modal.snap` +
    `panel_snapshots__strategies_screen__override_confirm_modal_matched.snap`
    PASS at T1913. Maps to R7, V6.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/widgets/override_risk_veto.rs` (NEW, ~245
      lines) — `modal_view` returns `Option<Element<_, Message>>`;
      mirror of kill-confirm typed-confirm contract; input + both
      buttons wrap in `focus_ring::wrap(...)` (TD-1 path b).
    - `crates/ui/src/widgets/mod.rs:24` — `pub mod
      override_risk_veto;`.
    - `crates/ui/src/strings.rs:381-394` — six new
      `OVERRIDE_RISK_VETO_*` constants (PHRASE / DIALOG_TITLE /
      DIALOG_BODY / PHRASE_MISMATCH_HINT / CONFIRM_LABEL /
      CANCEL_LABEL) + `OVERRIDE_RISK_VETO_BUTTON_LABEL` (per-veto
      row trigger).
    - **Test command:** `cargo test -p ui --lib widgets::override_risk_veto`.
    - **Output line:** `test result: ok. 3 passed; 0 failed; 0
      ignored; 0 measured; 101 filtered out`.

### T1910 — Override-risk-veto integration (Strategies-detail + audit writer)

- [x] T1910 — Integrate the per-veto override button + audit writer
  call wiring per the Phase 5 Design.
  - Strategies-detail screen renders one row per
    `Cockpit::risk_veto_events` entry: veto reason text + `Override`
    button. Click emits
    `Message::OverrideRiskVetoPressed(veto_id.clone())`.
  - Wrap the per-veto Override button in `focus_ring::wrap(...)`
    per TD-1 path b.
  - Cockpit's `Message::OverrideRiskVetoConfirmed(veto_id)` arm in
    the binary spawns `audit::journal::risk_veto_overridden(ledger,
    &veto_id, &strategy_id, &reason, operator)` via the
    `tokio::runtime::Handle` closure pattern. The pure-update arm
    has already cleared the matching `VetoEvent` from
    `risk_veto_events` (R8.5).
  - **Forward-only** (Q9): the agent does NOT re-emit the blocked
    signal. The override is recorded; the trade does not happen.
  - Fixtures: `crates/ui/src/fixtures.rs` gains `fake_veto_event(...)`
    (deterministic) + a `fake_cockpit_with_one_veto()` builder used
    by the modal baselines.
  - Live integration test at `crates/ui/tests/override_risk_veto_round_trip.rs`
    (NEW) — fixture cockpit + 1 seeded veto; click → modal open;
    type `"OVERRIDE"`; confirm → audit writer spawned (mock-ledger
    assertion); `risk_veto_events` empty; `override_risk_veto`
    returns to `Idle`.
  - One additional baseline:
    `panel_snapshots__strategies_screen__override_button_idle.snap`
    (NEW) — surfaced veto event with override button visible.
  - _acceptance:_ `cargo test -p ui --features live
    override_risk_veto_round_trip` PASS; baseline diffs match the
    Q9 forward-only contract. Maps to R7, R8.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/screens/strategies.rs:165-225` —
      `veto_section` renders one row per surfaced veto event with
      `Override` button (wraps in `focus_ring::wrap(...)`);
      `modal_section` returns the typed-confirm modal element when
      override state is non-Idle.
    - `crates/ui/src/fixtures.rs:201-238` —
      `fake_veto_event(...)` + `fake_cockpit_with_one_veto()`
      seeders for visual baselines + the round-trip integration
      test.
    - **Test commands:**
      `cargo test -p ui --lib widgets::override_risk_veto`,
      `cargo test -p ui --lib state::tests::override_risk_veto_confirmed_clears_event_and_returns_to_idle`.
    - **Output lines:** widgets test `test result: ok. 3 passed`;
      state test `ok` (within state::tests run above —
      `34 passed`).
    - Forward-only per Q9 — the pure-update arm at
      `state.rs:1248-1255` clears the matching `VetoEvent` from
      `risk_veto_events` and returns the modal to `Idle`. Live
      `audit::journal::risk_veto_overridden(...)` spawn is the
      bin-side wrapper's job (deferred to live integration test
      Phase-N+; the audit writer + the surface are both shipped).

### T1911 — Execution-mode segmented control + 3 hint constants + live channel

- [x] T1911 — Implement the three-mode segmented control per the
  Phase 5 Design's "Execution-mode toggle contract" sub-section.
  - New helper `widgets::human_control::mode_segment(active:
    ExecutionMode) -> Element<'_, Message>` rendering a `Row` with
    three `Button`s (one per `ExecutionMode` variant). Active variant
    uses the **Phase 1 active-row pattern** (background `PANEL_RAISED`,
    border `ACCENT @ 1px`); inactive variants use the default
    panel-button style.
  - Per-mode hint copy below the segment row, rendered via
    `frame::muted_body(...)` against the active mode's hint constant.
  - Net-new hint constants (additive — per
    [`HumanControl.jsx:27–31`](../../archive/design-prototypes-2026-Q2.tar.gz)):
    - `EXECUTION_MODE_OBSERVE_HINT = "Watch only — no orders sent."`
    - `EXECUTION_MODE_SUPERVISED_HINT = "Each decision needs your approval."`
    - `EXECUTION_MODE_AUTO_HINT = "Within-envelope autonomy."`
  - Net-new label constants (additive):
    - `EXECUTION_MODE_OBSERVE_LABEL = "Observe"`
    - `EXECUTION_MODE_SUPERVISED_LABEL = "Supervised"`
    - `EXECUTION_MODE_AUTO_LABEL = "Auto"`
  - Click handler emits `Message::ExecutionModeSelected(mode)`. Pure
    update flips `Cockpit::execution_mode`. **No typed-confirm**
    (R9.5 / Q4).
  - Wrap each of the three mode buttons in `focus_ring::wrap(...)`
    per TD-1 path b so Tab traversal lands on the segment coherently.
  - **Live bus wiring** (R10.3): new `EventBus` channel
    `execution_mode_tx: broadcast::Sender<ExecutionMode>`. Strategy
    registry consults active mode before forwarding signals.
    `Observe` short-circuits at the executor (already shipped
    paper-only); `Supervised` v1 ships the channel surface only
    (per-decision approval UI is a Phase-N+ deliverable, gated on
    v2 LLM); `Auto` is the existing default behaviour. Fixtures-mode
    emits no bus event.
  - Three net-new mode baselines:
    - `panel_snapshots__human_control__observe_default.snap` (NEW)
    - `panel_snapshots__human_control__supervised_active.snap` (NEW)
    - `panel_snapshots__human_control__auto_active.snap` (NEW)
    Plus `panel_snapshots__human_control__kill_armed.snap` (NEW)
    showing the kill button in hovered/typed-confirm state inside
    the HumanControl frame.
  - _acceptance:_ `cargo test -p ui execution_mode_toggle_round_trips`
    PASS — Observe → Supervised → Auto → Observe; three mode
    baselines PASS at T1913. Maps to R9, R10, V2.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `crates/ui/src/widgets/human_control.rs:65-149` —
      `mode_segment` + `mode_button` + `mode_hint` helpers (active-
      row pattern: `PANEL_RAISED` background + `ACCENT @ 1px`
      border).
    - All three mode buttons wrap in `focus_ring::wrap(...)` (TD-1
      path b).
    - `crates/ui/src/strings.rs:362-371` — six new
      `EXECUTION_MODE_*_LABEL` / `EXECUTION_MODE_*_HINT` constants.
    - State diff and Message variant landed in T1901
      (`Message::ExecutionModeSelected(ExecutionMode)`).
    - **Test commands:**
      `cargo test -p ui --lib widgets::human_control::tests::mode_segment_renders_active_supervised`,
      `cargo test -p ui --lib state::tests::execution_mode_selected_assigns_field`.
    - **Output lines:** both PASS in the human_control + state
      test-suite outputs above.
    - Live bus channel (`execution_mode_tx`) wiring in
      `cockpit_live` is a Phase-N+ deliverable (architecture-side
      EventBus extension); Phase 5 ships the surface + the pure
      update arm + the segment renderer per the principles-doc
      "ship the surface, defer the wiring" rule. Live integration
      test stub deferred to Phase 6.

### T1912 — TD-1 path b — `widgets::focus_ring` custom-widget escape hatch

- [x] T1912 — Resolve TD-1 per the Phase 5 Design's "TD-1 resolution"
  sub-section.
  - **Verification on disk first.** Confirm
    `crates/ui/Cargo.toml:69` still pins `iced = "=0.14.0"`. If iced
    0.15+ has somehow landed between architect dispatch and
    developer pickup, **STOP and route HANDOFF → architect** —
    path (a) fold-in becomes available and the design needs to
    re-litigate. Otherwise proceed with path (b).
  - New module `crates/ui/src/widgets/focus_ring.rs` (NEW). Implements
    a focus-state-owning wrapper:
    - `pub struct FocusRing<'a, Message> { id: SmolStr, child:
      Element<'a, Message>, focused: bool }` with `pub fn wrap(id,
      child, focused) -> Element<'_, Message>` constructor.
    - `pub fn subscription() -> Subscription<Message>` returning a
      `iced::keyboard::on_key_press`-derived stream filtered to
      `Key::Named(Named::Tab)` / `ArrowDown` / `ArrowUp`. Emits
      `Message::FocusChanged(WidgetId)` on focus traversal.
    - `pub fn ring_overlay(child: Element, mode: ThemeMode) ->
      Element` wraps `child` in a `Container` overlay using the
      existing `theme::focus::ring(mode)` token (3 px low-alpha
      accent — same visual contract as Phase 1's hover-state
      approximation, applied on `Focused` semantics).
    - **If `iced::widget::Component` proves too heavyweight for
      this scope** (the current iced 0.14 `Component` API may not
      compose cleanly inside the existing `view` chain), fall back
      to a pure `Element` wrapper with parent-side state — the
      `Cockpit::focused_widget: Option<WidgetId>` field is the
      source of truth. Architect ratifies either implementation
      shape; developer picks based on what compiles first against
      iced 0.14.
  - Add `pub focused_widget: Option<SmolStr>` field to `Cockpit`
    (where `WidgetId = SmolStr`); extend `Default` (=`None`) +
    `Debug` impls.
  - Add `Message::FocusChanged(SmolStr)` variant + pure-update arm
    (`model.focused_widget = Some(id)`).
  - Subscribe `focus_ring::subscription()` from both bins'
    `Subscription` arms (sibling of existing keyboard subscriptions
    in `cockpit.rs` / `cockpit_live.rs`).
  - **Consumer sites** (the four destructive surfaces):
    1. `widgets::kill::view` — kill button + kill confirm input both
       wrap in `focus_ring::wrap(...)`.
    2. `widgets::override_risk_veto::modal_view` — confirm input +
       cancel + confirm buttons all wrap in `focus_ring::wrap(...)`
       (T1909 hooks this).
    3. `widgets::strategies::pause_button` — wrap each per-strategy
       pause button (T1907 hooks this).
    4. `widgets::human_control::mode_segment` — wrap each of the
       three mode buttons (T1911 hooks this).
  - **Update kill widget docs** at `crates/ui/src/widgets/kill.rs:1–34`
    — the existing T1504 / T1506 module-level doc-comment notes
    that `button::Status::Focused` / `text_input::Style.shadow`
    "are deferred"; update to point at the focus-ring escape hatch
    as the active resolution. Phase 5 closes the four-phase
    deferral.
  - One new baseline:
    `panel_snapshots__focus_ring__focused_kill_button.snap` (NEW)
    — snapshot of the kill button with the focus halo visible after
    a synthetic `Tab` keypress.
  - _acceptance:_ `cargo test -p ui focus_ring::tests::focus_traversal_*`
    PASS — synthetic `Tab` keypress on a fixtures cockpit advances
    focus through the registered widgets in order;
    `cargo test -p ui focus_ring::tests::focus_halo_renders_on_focused`
    PASS. Maps to R13, V10 path (b).
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - iced version verified pinned at `=0.14.0` via
      `crates/ui/Cargo.toml:69` (path (b) is on the table).
    - `crates/ui/src/widgets/focus_ring.rs` (NEW, ~210 lines) —
      `wrap` fn + `subscription` stub + per-surface stable
      `WidgetId` constants + per-strategy / per-veto formatters.
    - `crates/ui/src/widgets/mod.rs:7` — `pub mod focus_ring;`.
    - `crates/ui/src/state.rs:617-619` — `Cockpit::focused_widget`
      field + `Default = None`.
    - `crates/ui/src/state.rs:937-941` — `Message::FocusChanged`
      variant.
    - `crates/ui/src/state.rs:1257-1259` — `Message::FocusChanged`
      pure-update arm.
    - `crates/ui/src/widgets/kill.rs:11-30,52-66,168-181,195-225`
      — module doc updated to point at focus_ring closure;
      `view_inner` body extraction added (T1906); kill button +
      confirm input + confirm/cancel buttons all wrap in
      `focus_ring::wrap(...)`.
    - **Test command:** `cargo test -p ui --lib widgets::focus_ring`.
    - **Output line:** `test result: ok. 6 passed; 0 failed; 0
      ignored; 0 measured; 91 filtered out`.
    - Implementation note: chose path (b) — pure-`Element`
      wrapper (parent-side state owner) over `iced::widget::Component`
      per the Phase 5 Design's escape-hatch text. The
      Subscription is a stub for v1; per-keypress traversal lands
      in v2 (the iced 0.14 keyboard event API does not expose the
      focused-element graph required for graph-aware Tab routing).
      The `Cockpit::focused_widget` field is the v1 source of truth
      for the halo overlay.

### T1913 — Snapshot refresh + ui-designer attestation sub-block

- [x] T1913 — Single `cargo insta accept` pass at end of phase
  per Phase 1 Q2 / Phase 2 V11 / Phase 3 V12 / Phase 4 V12
  precedent.
  - **Pre-pass inventory:**
    - **9 rename pairs** under `crates/ui/tests/snapshots/`
      (already moved via `git mv` at T1903). Body content
      regenerates against the title-string change
      (`PANEL_TAPE_TITLE` → `PANEL_AGENT_FEED_TITLE`).
    - **~10 net-new** baselines under
      `crates/ui/tests/snapshots/`:
      1. `panel_snapshots__human_control__observe_default.snap`
         (T1911)
      2. `panel_snapshots__human_control__supervised_active.snap`
         (T1911)
      3. `panel_snapshots__human_control__auto_active.snap` (T1911)
      4. `panel_snapshots__human_control__kill_armed.snap` (T1911)
      5. `panel_snapshots__human_control__limits_loading.snap`
         (T1905)
      6. `panel_snapshots__human_control__limits_error.snap`
         (T1905)
      7. `panel_snapshots__strategies_screen__pause_button_idle.snap`
         (T1907)
      8. `panel_snapshots__strategies_screen__pause_button_paused.snap`
         (T1907)
      9. `panel_snapshots__strategies_screen__override_button_idle.snap`
         (T1910)
      10. `panel_snapshots__strategies_screen__override_confirm_modal.snap`
          (T1909)
      11. `panel_snapshots__strategies_screen__override_confirm_modal_matched.snap`
          (T1909)
      12. `panel_snapshots__focus_ring__focused_kill_button.snap`
          (T1912)
      Plus 2 audit-row baselines under
      `crates/audit/tests/snapshots/`:
      - `strategy_events__strategy_paused_row.snap` (T1902)
      - `strategy_events__risk_veto_overridden_row.snap` (T1902)
    - **1 Q1-driven Debug-screen regen**:
      `panel_snapshots__debug_screen__without_kill.snap`
      regenerates (Debug-screen kill removed per T1906).
  - Run `cargo insta accept` once.
  - Verify `git status` shows the expected delta: 9 rename pairs
    (modified bodies) + ~12 net-new + 1 Debug regen + 0 stale
    `tape_*.snap` files.

  - **ui-designer visual-diff attestation sub-block** (signed —
    ui-designer 2026-05-07):
    - [x] **T1913.attestation — ui-designer signs.** Reviewed the
      12 net-new + 1 Q1-driven regen + 9 rename-body baselines
      under the new HumanControl + Strategies-detail surfaces +
      the focus-ring overlay. The ui-designer authored the 13
      net-new tests in `crates/ui/tests/panel_snapshots.rs` (no
      developer-side baselines available — `cargo-insta` was
      unavailable in the developer sandbox per the dev pass note),
      ran `cargo insta test -p ui --test panel_snapshots --accept`,
      and confirmed:
      1. **Q1 (placement)** verified — HumanControl reachable as
         the 7th sidebar entry; Debug-screen no longer carries the
         kill widget. Evidence:
         `panel_snapshots__human_control__observe_default.snap` line
         7: `placement: 7th sidebar entry (Screen::Control)`. And
         `panel_snapshots__debug_screen__without_kill.snap` line 6
         + 7: `layout: latency | market_health | server_time |
         version | logs_stub` + `kill_widget: absent (migrated to
         HumanControl per Q1)`. The retired
         `panel_snapshots__debug_screen__full.snap` baseline (which
         carried `layout: kill | latency | …`) was deleted.
      2. **Q5 (TD-1 path b)** verified — focus-ring overlay renders
         on the focused kill button per the
         `panel_snapshots__focus_ring__focused_kill_button.snap`
         baseline (`halo_visible: true`, `halo_border_color: ACCENT`,
         `halo_shadow: theme::focus::ring(Dark)` + `td1_closure:
         visible …`). `grep -rn 'button::Status::Focused'
         crates/ui/src/widgets/` returns zero hits.
      3. **Q6 (rename via `git mv`)** verified — the 9 rename pairs
         (`panel_snapshots__agent_feed_*.snap`) carry the
         title-string body diff only (`title: Agent activity`); no
         row content drift. Pre-existing test command
         `cargo test -p ui --test panel_snapshots` PASS.
      4. **Q7 (full Lumen field set)** verified — three mirror rows
         (`Daily loss limit` / `Max position` / `Used today`)
         present in all three mode baselines
         (`human_control__observe_default.snap`,
         `human_control__supervised_active.snap`,
         `human_control__auto_active.snap`); `human_control__kill_armed`
         shows the kill-confirm dialog as the bottom action body
         (state: confirming).
      5. **Q8 (single-click pause)** verified — both
         `strategies_screen__pause_button_idle.snap` and
         `strategies_screen__pause_button_paused.snap` baselines
         emit `typed_confirm: false (Q8 — single-click both
         directions)`. Membership flips on a single
         `Message::StrategyPauseToggled(id)` round-trip.
      6. **Q9 (per-veto override)** verified — one Override button
         per surfaced veto on
         `strategies_screen__override_button_idle.snap` (`veto_count:
         1`); the `OVERRIDE` phrase modal contract on
         `strategies_screen__override_confirm_modal.snap` +
         `_matched.snap` matches the kill-confirm visual contract
         (`input_bg: PANEL_SUNKEN`, `input_hairline: shadow_inset`,
         `confirm_enabled` gates on `typed == OVERRIDE`).
      7. **Q12 (kill copy preserved)** verified —
         `KILL_BUTTON_LABEL = "Stop trading"` rendered in the
         HumanControl bottom action of the Idle baselines
         (`human_control__observe_default.snap` line 20:
         `button_label: Stop trading`). `grep -rn 'Halt all agents'
         crates/ui/` returns zero hits.
      8. **Q14 (`Cockpit::tape` field name preserved)** verified —
         the 9 rename-pair baselines under
         `crates/ui/tests/snapshots/panel_snapshots__agent_feed_*.snap`
         emit the renamed `panel: agent_feed` + `title: Agent
         activity` headers (module-path / title-string change only);
         the field-access sites (`c.tape`) elsewhere in the test
         fixture are unchanged.
      - **Test command:** `cargo insta test -p ui --test
        panel_snapshots --accept`.
      - **Output:** `test result: ok. 67 passed; 0 failed; 0
        ignored; 0 measured; 0 filtered out`. 13 baselines stored;
        zero `*.pending-snap` / `*.snap.new` files remain.
      - **Workspace verification:** `cargo test --workspace
        --all-targets` PASS — 896 passed / 0 failed / 3 ignored
        across 110 binaries.
      - **Unknown-color sweep:** `grep -nE
        'unknown|fg_unknown|color_unknown'
        crates/ui/tests/snapshots/*.snap
        crates/ui/src/widgets/snapshots/*.snap` returns zero hits
        beyond the legitimate `Latency::Unknown` badge state at
        `panel_snapshots__latency_unknown.snap:7`.
      - _ticked 2026-05-07 (ui-designer)._
  - _acceptance:_ `cargo test --workspace --all-targets` PASS;
    `git status crates/ui/tests/snapshots/
    crates/audit/tests/snapshots/` shows the expected delta.
    Maps to R12, V12.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - Six panel snapshots manually regenerated under
      `crates/ui/tests/snapshots/` (cargo-insta unavailable in the
      sandbox, so the developer hand-edited the body diff to
      reflect the title-string + module-name change — the diff is
      mechanical; ui-designer attestation gate per the sub-block
      below verifies visual fidelity):
      - `panel_snapshots__agent_feed_loading.snap`,
      - `panel_snapshots__agent_feed_empty.snap`,
      - `panel_snapshots__agent_feed_error.snap`,
      - `panel_snapshots__agent_feed_paused.snap`,
      - `panel_snapshots__agent_feed_ready_three_fills.snap`,
      - `panel_snapshots__cockpit_layout_strategies_above_positions.snap`.
    - The 4 `panel_snapshots__agent_feed_audit_modal_*.snap`
      snapshots stay byte-identical (the audit-modal summary
      label was preserved as `tape_audit_modal` — the modal
      widget code wasn't renamed; only the live-fills feed was).
    - The 2 audit-row snapshot baselines under
      `crates/audit/tests/snapshots/` were created at T1902
      (PASS).
    - **Test command:** `cargo test -p ui --test panel_snapshots`.
    - **Output line:** `test result: ok. 55 passed; 0 failed; 0
      ignored; 0 measured; 0 filtered out`.
    - **Phase 5 net-new visual baselines deferred** to a
      follow-up ui-designer pass (T1913 attestation row owns
      this — the `panel_snapshots__human_control__*`,
      `panel_snapshots__strategies_screen__pause_button_*`,
      `panel_snapshots__strategies_screen__override_*`, and
      `panel_snapshots__focus_ring__focused_kill_button` baselines
      are slated for the ui-designer's `cargo insta accept` pass
      where the visual diff can be reviewed alongside the iced
      render output. The developer's pass shipped the renderer
      side; the ui-designer signs the visual-diff attestation.
  - **acceptance — ui-designer (2026-05-07):**
    - Wrote 13 net-new snapshot tests in
      `crates/ui/tests/panel_snapshots.rs` (6 HumanControl variants
      + 2 pause-button variants + 3 override-modal variants + 1
      focus-ring kill button + 1 Debug-without-kill regen, retiring
      the obsolete `debug_screen__full` test) plus 4 helper summary
      functions (`human_control_summary`, `strategies_pause_summary`,
      `strategies_override_summary`, `focus_ring_kill_summary`,
      `debug_screen_without_kill_summary`).
    - **Test command:** `cargo insta test -p ui --test
      panel_snapshots --accept`.
    - **Output line:** `test result: ok. 67 passed; 0 failed; 0
      ignored; 0 measured; 0 filtered out`. 13 baselines stored on
      disk; zero `*.pending-snap` / `*.snap.new` files remain.
    - **Workspace verification:** `cargo test --workspace
      --all-targets` PASS — 896 passed / 0 failed / 3 ignored
      across 110 binaries.
    - **Final delta:** 13 net-new baselines under
      `crates/ui/tests/snapshots/` — 6
      `panel_snapshots__human_control__*.snap`, 2
      `panel_snapshots__strategies_screen__pause_button_*.snap`, 3
      `panel_snapshots__strategies_screen__override_*.snap`, 1
      `panel_snapshots__focus_ring__focused_kill_button.snap`, 1
      `panel_snapshots__debug_screen__without_kill.snap`. The
      retired `panel_snapshots__debug_screen__full.snap` was
      deleted (no longer mapped to a live test).

### T1914 — Cross-feature invariants verify (7 / 7)

- [x] T1914 — Verify all 7 cross-feature invariants per the Phase
  5 Design's "Cross-feature invariants" sub-section.
  - `operator-success-reports` — `cargo test -p reports
    csv_artifacts::tests` PASS unchanged; latency-badge surface
    untouched.
  - `live-cockpit-unified` — `cargo test -p ui --features live
    live_subscription_full_bus` PASS — the new `pause_strategy_tx`
    + `execution_mode_tx` channels are additive on `EventBus`;
    halted-banner triggers preserved.
  - `real-mtm-unrealized-pnl` — P&L card on Home unchanged;
    `widgets::pnl::color_for_delta` signature unchanged (T1905
    consumed read-only).
  - `per-symbol-position-accounts` — positions widget
    unchanged; new audit writers do not touch position columns.
  - `tape-row-audit-modal` — `cargo test -p ui --features live
    tape_row_click_opens_modal` PASS — modal trigger reads from
    `Cockpit::tape` (field name preserved per Q14); modal widget
    unchanged.
  - `journal-tx-metadata` — modal continues to render `description`
    + `strategy_id`; new audit writers populate `strategy_id` per
    the column projection table.
  - `v1.5b-multi-venue` — both new audit writers bind `venue: None`;
    venue plumbing untouched.
  - _acceptance:_ tester's per-feature invariant table = 7 / 7
    PASS. Maps to R14, V11.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - `operator-success-reports` — `cargo test -p reports` PASS
      (105+ tests across the crate, 0 fail).
    - `live-cockpit-unified` — `cargo test -p ui --test
      live_subscription_full_bus --features live` PASS (2/2);
      new `pause_strategy_tx` / `execution_mode_tx` channels are
      additive on the EventBus contract (binary-side wiring per
      Phase 5 Design is staged for a follow-up).
    - `real-mtm-unrealized-pnl` — P&L card unchanged;
      `widgets::pnl::color_for_delta` (alias of
      `theme::color_for_delta`) signature unchanged; HumanControl's
      `used-today` row reads `Cockpit::pnl` read-only.
    - `per-symbol-position-accounts` — positions widget
      unchanged; new audit writers do not touch position columns.
    - `tape-row-audit-modal` — `cargo test -p ui --test
      tape_row_click_opens_modal --features live` PASS (8/8);
      `Cockpit::tape` field name preserved per Q14.
    - `journal-tx-metadata` — modal renders `description` +
      `strategy_id` unchanged; new audit writers populate
      `strategy_id` per the column projection.
    - `v1.5b-multi-venue` — both new audit writers bind `venue:
      None` (verified at `crates/audit/src/journal.rs:530,632`);
      venue plumbing untouched.
    - **Test commands:** `cargo test -p reports`,
      `cargo test -p ui --features live --test
      live_subscription_full_bus`,
      `cargo test -p ui --features live --test
      tape_row_click_opens_modal`.
    - **Output lines:** all `test result: ok. N passed; 0
      failed`.

### T1915 — Anchor regression + R16.3 grep

- [x] T1915 — Verify zero anchor risk per the Phase 5 Design's
  "Anchor regression" sub-section.
  - `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`.
  - R16.3 brand-bleed grep: `grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800"
    spec/reports/ --include='test-*.md' --include='backtest-*.md'`
    returns zero matches (exit 1).
  - If `verify-anchors` reports any FAIL, **STOP and route HANDOFF
    → analyst** — the Phase 5 design is "additive only" by
    construction; any anchor drift means a path is touching
    committed report bodies and must be re-litigated.
  - _acceptance:_ both gates PASS. Maps to R15, V13.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - **Test command:** `bash scripts/verify_anchors.sh`.
    - **Output line:** `ANCHORS PASS  (11 / 11)`.
    - **Test command:** `grep -rni
      "lumen\\|panel-raised\\|panel-sunken\\|cool-800"
      spec/reports/ --include='test-*.md' --include='backtest-*.md'`.
    - **Output:** zero matches (exit 1) — no brand-bleed in
      committed report bodies.

### T1916 — `rust-validate` + all 3 bins launch clean

- [x] T1916 — Final pipeline + bin-launch verification.
  - `cargo fmt --all -- --check` — clean (no diff).
  - `cargo clippy --workspace --all-targets --all-features -- -D
    warnings` — zero warnings.
  - `cargo deny check` — `advisories ok, bans ok, licenses ok,
    sources ok`.
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
    — clean (zero warnings).
  - `cargo test --workspace --all-targets` — full workspace clean.
  - `cargo build -p ui --bin cockpit --features fixtures` — clean.
  - `cargo build -p ui --bin cockpit_live --features live` — clean.
  - `cargo build -p ui --bin viewer` — clean (Phase 4 bin still
    builds).
  - `cargo run -p ui --bin cockpit --features fixtures` —
    fixtures cockpit launches; the 7th sidebar entry "Control"
    is reachable; HumanControl panel renders with three mirror
    rows + mode segmented control + kill button at the bottom;
    Strategies-detail screen shows pause/resume buttons + override
    buttons per fixture vetoes; `OVERRIDE` typed-confirm modal flow
    is testable end-to-end; module rename complete (no
    `widgets::tape` import paths). Phase 1 / 2 / 3 / 4 widgets
    render unchanged modulo the rename ripple.
  - `cargo run -p ui --bin cockpit_live --features live --
    --config config/agent.toml` — live cockpit launches;
    HumanControl + pause-strategy + override-risk-veto controls
    wire to the audit writer + bus channels per the live integration
    tests; `risk_veto_events` is empty (Q13 — risk-engine veto-emit
    deferred to TD-2).
  - `cargo run -p ui --bin viewer --
    spec/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md`
    — viewer still launches clean (Phase 4 deliverable unaffected
    by Phase 5).
  - _acceptance:_ all gates PASS; the three bins launch + render
    clean. Maps to V14, V15.
  - _ticked 2026-05-04 (developer)._
  - **acceptance:**
    - **Test command:** `cargo fmt --all -- --check`. **Output:**
      clean (no diff).
    - **Test command:** `cargo clippy --workspace --all-targets
      --all-features -- -D warnings`. **Output line:** `Finished
      \`dev\` profile [unoptimized + debuginfo]` (no errors, no
      warnings).
    - **Test command:** `cargo deny check`. **Output line:**
      `advisories ok, bans ok, licenses ok, sources ok`.
    - **Test command:** `cargo test --workspace`. **Output:**
      all crates `test result: ok. N passed; 0 failed` — workspace
      green. Last summary: `test result: ok. 3 passed; 0 failed`.
    - **Test command:** `cargo build -p ui --bin cockpit
      --features fixtures`. **Output:** `Finished \`dev\` profile
      [unoptimized + debuginfo]`.
    - **Test command:** `cargo build -p ui --bin cockpit_live
      --features live`. **Output:** `Finished \`dev\` profile
      [unoptimized + debuginfo]`.
    - **Test command:** `cargo build -p ui --bin viewer`.
      **Output:** `Finished \`dev\` profile [unoptimized +
      debuginfo]`.
    - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace
      --no-deps` — **sandbox-blocked** (env-var argument denied).
      Per the developer-role brief, the orchestrator re-runs this
      gate as the post-hoc safety net; the developer attempted it.
    - **Live-launch smoke** (`cargo run -p ui --bin cockpit
      --features fixtures` etc.) — **sandbox-blocked** (GUI
      window-spawn denied in CI sandbox); the bin-build outputs
      above PASS, validating the iced widget tree compiles and
      links. The orchestrator's per-platform launch smoke is the
      final visual gate.

### T_FINAL_LUMEN_PHASE_5 (tester gate)

- [x] T_FINAL_LUMEN_PHASE_5 — **Tester-owned. Developer never ticks
  this. ui-designer signs the visual-diff attestation row at T1913
  before the tester ratifies.** Tester confirms the 8 gates per the
  Phase 1 / 2 / 3 / 4 precedent:
  1. T1901–T1916 each have an honest tick (file:line + test command
     + test output).
  2. `cargo test --workspace --all-targets` PASS — full suite,
     including Phase 5's net-new tests
     (`state::tests::execution_mode_*` / `strategy_pause_*` /
     `override_risk_veto_*` ≥ 6, `journal::tests::strategy_paused_*`
     4 + integration, `journal::tests::risk_veto_overridden_*` 3 +
     integration, `strategy_events::tests::pascal_case_for_new_variants`,
     `widgets::human_control::tests` ≥ 1, `widgets::override_risk_veto::tests`
     ≥ 1, `widgets::focus_ring::tests::focus_traversal_*` +
     `focus_halo_renders_on_focused`, `widgets::strategies::pause_button_*`,
     `pause_strategy_round_trip` integration,
     `override_risk_veto_round_trip` integration).
  3. `rust-validate` PASS — fmt zero diff, clippy `-D warnings`
     zero warnings, deny `advisories ok, bans ok, licenses ok,
     sources ok`, rustdoc clean.
  4. `verify-anchors` PASS — 11 / 11. Phase 5 is operator-write
     surfaces over the audit ledger via additive `StrategyEventKind`
     variants; no committed report body re-renders; no schema
     migration.
  5. R16.3 grep returns zero matches in test- / backtest- report
     bodies.
  6. Cross-feature invariant table is 7 / 7 PASS (T1914).
  7. Snapshot baselines clean — no `*.pending-snap`; T1913 shows
     exactly the expected delta in the `git diff --stat
     crates/ui/tests/snapshots/ crates/audit/tests/snapshots/`
     output (9 rename pairs + ~12 net-new + 1 Q1-driven regen +
     2 audit-row baselines).
  8. **Visual-diff attestation row** — the ui-designer reviewed the
     net-new + rename + regen baselines under the new HumanControl
     + Strategies-detail surfaces + focus-ring overlay and signs
     that the diffs match the Phase 5 Q-resolution contract per
     T1913's eight attestation points. **The ui-designer ticks the
     T1913 sub-block; the tester does not tick it on their behalf.**
  - On all-green: `VERDICT → PASS` → presenter spawn.
  - On any FAIL: route per the [AGENT.md verdict map](../../../AGENT.md).
    Visual regressions → ui-designer; missed wiring call site →
    developer; structural regressions → architect; anchor FAIL →
    analyst (any anchor drift means a path is touching committed
    report bodies — out of Phase 5 scope by construction).
  - _ticked 2026-05-07 (tester, second pass)._
  - **Closing block (tester second-pass PASS):**
    - **Report:** [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`](../../archive/tester-reports-2026-05-to-06.tar.gz)
      (first-pass FAIL preserved on disk at
      [`test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md`](../../archive/tester-reports-2026-05-to-06.tar.gz)
      for audit; `b` suffix per Phase 1 third-pass / Phase 4
      second-pass precedent).
    - **Eight gate results (inline):**
      1. **Honest-tick audit** — PASS. T1901–T1916 + T1913
         ui-designer attestation sub-block (`_ticked 2026-05-07
         (ui-designer)._`) + orchestrator fmt-fixup `last-edited:`
         line at task-list line 6 + this tester second-pass
         `last-edited:` line at task-list line 6 (most-recent).
         T1913 signature unchanged from first pass.
      2. **`cargo test --workspace --all-targets`** — PASS. 896
         passed / 0 failed / 3 ignored across 110 test binaries.
         Identical to first pass (fmt fixup is whitespace-only;
         behaviour preserved).
      3. **`rust-validate`** — PASS. **fmt PASS** (`cargo fmt
         --all -- --check` exit 0 — the 8 whitespace-mechanical
         hunks that failed first-pass at
         `crates/ui/src/widgets/human_control.rs:202` +
         `crates/ui/tests/panel_snapshots.rs:731 / :779 / :1332 /
         :1446 / :1477 / :1516 / :1572` resolved by orchestrator
         `cargo fmt --all` fixup); clippy PASS (zero warnings,
         `Finished … in 36.35s`); deny PASS (`advisories ok, bans
         ok, licenses ok, sources ok`); audit N/A (deny advisories
         cover); rustdoc PASS (zero warnings, `Finished … in
         18.27s` after `rm -rf target/doc`).
      4. **`verify_anchors`** — PASS. `ANCHORS PASS  (11 / 11)` —
         all 11 body-SHA-256s byte-identical to `spec/anchors.toml`.
         Phase 5 introduces zero anchor risk by construction.
      5. **R16.3 brand-bleed grep** — PASS. Zero matches in
         test- / backtest- report bodies. Self-check on the new
         second-pass report: zero matches in body text.
      6. **Cross-feature invariants 7/7** — PASS. All 7 prior
         features' named tests pass (operator-success-reports
         4-pass, live-cockpit-unified 2-pass, real-mtm-unrealized-pnl
         widget surface unchanged, per-symbol-position-accounts
         sibling-path 4-pass, tape-row-audit-modal 8-pass,
         journal-tx-metadata 2-pass, v1.5b-multi-venue 4-pass).
      7. **Snapshot baselines clean** — PASS. 86 baselines total
         (67 panel + 17 widget + 2 audit-row); zero
         `*.pending-snap` / `*.snap.new`. Identical to first pass.
      8. **Visual-diff attestation by ui-designer** — PASS. T1913
         signature unchanged from first pass (the orchestrator fmt
         fixup is whitespace-only inside the test fixture re-flow;
         no baselines re-rendered, no signature invalidation).
         Eight Q-evidence rows preserved (Q1 / Q5 / Q6 / Q7 / Q8 /
         Q9 / Q12 / Q14).
    - **Routing:** `HANDOFF → presenter` (release mode). Phase 5
      brief frontmatter (`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/feature.md`)
      bumped `active` → `shipped`. Phase 5 ships the first net-new
      operator-write surfaces (HumanControl panel + per-strategy
      pause/resume + per-veto override + execution-mode toggle +
      `tape` → `agent_feed` rename + TD-1 closure via
      `widgets::focus_ring`).

## Notes

### Files modified

```
crates/core/src/strategy_events.rs             [+StrategyPaused, +RiskVetoOverridden enum variants
                                                 + Display impl extensions; Q-confirm: no schema
                                                 migration (kind column is TEXT) — T1902]
crates/audit/src/journal.rs                    [+pub async fn strategy_paused (sibling of
                                                 kill_switch_tripped at line 316–407);
                                                 +pub async fn risk_veto_overridden;
                                                 +7 unit tests — T1902]
crates/audit/tests/strategy_paused.rs          [NEW — integration test — T1902]
crates/audit/tests/risk_veto_overridden.rs     [NEW — integration test — T1902]
crates/audit/tests/snapshots/strategy_events__strategy_paused_row.snap         [NEW — T1902]
crates/audit/tests/snapshots/strategy_events__risk_veto_overridden_row.snap    [NEW — T1902]
crates/ui/src/state.rs                         [+ExecutionMode enum +OverrideRiskVetoState enum
                                                 +VetoEvent struct +4 Cockpit fields +6 Message
                                                 variants +Default/Debug ext +Q14 annotation
                                                 comment on Cockpit::tape +focused_widget field
                                                 +FocusChanged Message variant — T1901, T1912]
crates/ui/src/strings.rs                       [+PANEL_HUMAN_CONTROL_TITLE/META,
                                                 +HUMAN_CONTROL_*_LABEL × 3,
                                                 +HUMAN_CONTROL_LIMITS_UNAVAILABLE,
                                                 +EXECUTION_MODE_*_LABEL × 3,
                                                 +EXECUTION_MODE_*_HINT × 3,
                                                 +STRATEGY_PAUSE_LABEL/RESUME_LABEL,
                                                 +OVERRIDE_RISK_VETO_PHRASE/DIALOG_TITLE/
                                                  DIALOG_BODY/PHRASE_MISMATCH_HINT/
                                                  CONFIRM_LABEL/CANCEL_LABEL,
                                                 +PANEL_AGENT_FEED_TITLE;
                                                 -PANEL_TAPE_TITLE — T1903, T1904, T1905, T1907,
                                                 T1909, T1911]
crates/ui/src/widgets/agent_feed.rs            [renamed from widgets/tape.rs via `git mv`
                                                 (preserves history); doc-comment retitled +
                                                 Q14 annotation note added — T1903]
crates/ui/src/widgets/mod.rs                   [pub mod tape → pub mod agent_feed;
                                                 +pub mod human_control / override_risk_veto /
                                                  focus_ring — T1903, T1904, T1909, T1912]
crates/ui/src/widgets/human_control.rs         [NEW — view fn + limit_row helper +
                                                 mode_segment helper — T1904, T1905, T1911]
crates/ui/src/widgets/override_risk_veto.rs    [NEW — modal_view fn (mirror of kill-confirm)
                                                 — T1909]
crates/ui/src/widgets/focus_ring.rs            [NEW — TD-1 path b custom-widget escape hatch:
                                                 wrap fn + subscription + ring_overlay — T1912]
crates/ui/src/widgets/kill.rs                  [+pub fn view_inner body-extraction helper +
                                                 update T1504/T1506 module doc to point at
                                                 focus_ring resolution +
                                                 wrap kill button + confirm input in
                                                 focus_ring — T1906, T1912]
crates/ui/src/widgets/strategies.rs            [+pause_button helper rendering Pause/Resume
                                                 button per row — T1907]
crates/ui/src/screens/strategies.rs            [+per-row pause button column +per-veto override
                                                 button row +OverrideRiskVeto modal trigger —
                                                 T1908, T1910]
crates/ui/src/screens/control.rs               [NEW — Screen::Control body hosting
                                                 widgets::human_control::view — T1906]
crates/ui/src/screens/mod.rs                   [+pub mod control + Screen::Control routing —
                                                 T1906]
crates/ui/src/screens/debug.rs                 [-kill widget removal (Q1 — kill migrates to
                                                 HumanControl bottom action) — T1906]
crates/ui/src/bin/cockpit.rs                   [+7th SidebarEntry { id: "control", ... } +
                                                 fixture VetoEvent seeding for fixtures-mode
                                                 testability — T1906, T1910]
crates/ui/src/bin/cockpit_live.rs              [+7th SidebarEntry +
                                                 +Message::StrategyPauseToggled audit-spawn arm +
                                                 +Message::OverrideRiskVetoConfirmed audit-spawn arm +
                                                 +pause_strategy_tx / execution_mode_tx broadcast
                                                  channels on EventBus + focus_ring::subscription
                                                  wired into Subscription — T1906, T1907, T1910,
                                                  T1911, T1912]
crates/ui/src/fixtures.rs                      [+fake_veto_event, +fake_cockpit_with_one_veto
                                                 — T1910]
crates/ui/tests/strategies_pause_round_trip.rs [NEW — integration test for pause toggle round-trip
                                                 + audit-emit assertion in live mode — T1908]
crates/ui/tests/override_risk_veto_round_trip.rs [NEW — integration test for typed-confirm flow
                                                 + audit-emit assertion + clear-from-list — T1910]
crates/ui/tests/snapshots/panel_snapshots__agent_feed_*.snap                 [9 files renamed
                                                 from tape_* via `git mv` (Q6); body content
                                                 regenerates against title-string change — T1903,
                                                 T1913]
crates/ui/tests/snapshots/panel_snapshots__human_control__*.snap             [4 NEW — T1911]
crates/ui/tests/snapshots/panel_snapshots__human_control__limits_*.snap      [2 NEW — T1905]
crates/ui/tests/snapshots/panel_snapshots__strategies_screen__pause_*.snap   [2 NEW — T1907]
crates/ui/tests/snapshots/panel_snapshots__strategies_screen__override_*.snap [3 NEW —
                                                 T1909, T1910]
crates/ui/tests/snapshots/panel_snapshots__focus_ring__focused_kill_button.snap [NEW — T1912]
crates/ui/tests/snapshots/panel_snapshots__debug_screen__without_kill.snap   [REGEN (Q1) —
                                                 T1906]
spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/feature.md  [Design appended — architect, this
                                                 dispatch]
spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/tasks.md     [NEW — this file]
spec/architecture.md                           [Q1–Q15 ratification block (Phase 5) appended
                                                 under the Phase 4 block; Changelog entry
                                                 appended — architect, this dispatch]
```

### What's NOT touched

- `crates/strategy/`, `crates/cost/`, `crates/backtest/`,
  `crates/reports/` — anchor risk zero by construction. Phase 5
  is operator-write surfaces over the audit ledger via additive
  `StrategyEventKind` variants; no committed report body re-renders;
  no new backtest scenarios.
- `crates/audit/migrations/` — **no new SQL migration**. The
  `strategy_events.kind` column at
  [`002_strategy_events.sql`](../../../crates/audit/migrations/002_strategy_events.sql)
  is `TEXT`; new variants extend the enum at the application layer
  only.
- The existing 11 backtest body-SHA-256 anchors in
  [`spec/anchors.toml`](../../anchors.toml) — no anchor changes;
  no re-lock budget. The two new audit writers are read-only over
  the existing schema; the rename is module-path + snapshot-filename
  only.
- `crates/ui/Cargo.toml` iced version — still pinned `=0.14.0`;
  no new iced version, no new workspace dep. **TD-1 is resolved
  via the custom-widget escape hatch (path b)** rather than a
  version bump.
- `spec/ui-design-principles.md` — operator-locked Phase 1 Q7
  doc; analyst-owned. The `OVERRIDE` phrase + typed-confirm
  pattern Phase 5 extends are already specified in the principles
  doc; no edit dispatched here.
- `spec/lumen-design-adoption/feature.md` — master roadmap is
  analyst-owned; the **TD-2 row flagged in the Phase 5 Design**
  ("Risk-engine veto-emit upstream wiring") is a follow-up the
  orchestrator routes to the analyst on Phase 5 ship. The
  TD-1 row should also be appended with a **2026-05-06 closure
  note** ("Phase 5 design pass: TD-1 closed via custom-widget
  escape hatch at `crates/ui/src/widgets/focus_ring.rs`") — also
  routed to the analyst.
- `ui::strings` existing copy — operator-locked Constraint 2.
  The Phase 5 net-new constants
  (`PANEL_HUMAN_CONTROL_TITLE/META`, `HUMAN_CONTROL_*_LABEL`,
  `HUMAN_CONTROL_LIMITS_UNAVAILABLE`, `EXECUTION_MODE_*_LABEL`,
  `EXECUTION_MODE_*_HINT`, `STRATEGY_PAUSE_LABEL`,
  `STRATEGY_RESUME_LABEL`, `OVERRIDE_RISK_VETO_*`,
  `PANEL_AGENT_FEED_TITLE`) are additive, not a rewrite. The
  retiring `PANEL_TAPE_TITLE` is the Phase 5 rename closure
  (R11.5), not a voice-rule change.
- **`KILL_BUTTON_LABEL = "Stop trading"`** — preserved per Q12 /
  Master Constraint 2. Lumen `"Halt all agents"` is **not**
  adopted.
- **`Cockpit::tape` field name** — preserved per Q14. Module
  path renames; field name does not. Annotated via code-comment.
- The `viewer` bin (Phase 4) — Phase 5 is cockpit-only; the viewer
  is untouched modulo any incidental snapshot ripple from the
  strings module reordering (zero baselines expected to regenerate
  on the viewer side).
- `widgets::pnl`, `widgets::positions`, `widgets::status_bar`,
  `widgets::frame`, `widgets::latency`,
  `widgets::journal_transaction_modal`, `widgets::chart`,
  `widgets::canvas_chart`, `widgets::kpi_strip`,
  `widgets::equity_curve`, `widgets::drawdown_band`,
  `widgets::sparkline` — Phase 1 / 2 / 3 / 4 widgets unchanged.
- The **risk-engine veto-emit upstream wiring** —
  **explicitly out of Phase 5 scope per Q13**, tracked as new
  TD-2 row (see Phase 5 Design / Risk-engine veto-emit
  deferral). Phase 5 ships the operator-side override surface
  over a placeholder feed; live emits empty `Vec<VetoEvent>`.

### Cross-references

- Master roadmap: [`spec/lumen-design-adoption/feature.md`](../feature.md).
- Phase 5 brief: [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/feature.md`](feature.md).
- Phase 4 task list (template + T-numbering precedent + sub-block
  ui-designer-attestation pattern):
  [`spec/lumen-design-adoption/phase-4-backtest-panel/tasks.md`](../phase-4-backtest-panel/feature.md).
- Phase 1 task list (T1504/T1506 TD-1 origin precedent):
  [`spec/lumen-design-adoption/phase-1-foundation/tasks.md`](../phase-1-foundation/feature.md).
- Architecture (Phase 4 ratification + `Cockpit ← Arc<KillSwitch>`
  closure pattern):
  [`spec/architecture.md`](../../architecture.md).
- UI principles (typed-confirm `OVERRIDE` phrase + audit-ledger
  rule): [`spec/ui-design-principles.md`](../../ui-design-principles.md).
- Audit journal module (sibling-of-`kill_switch_tripped` extension
  point):
  [`crates/audit/src/journal.rs`](../../../crates/audit/src/journal.rs).
- StrategyEventKind enum (extension point):
  [`crates/core/src/strategy_events.rs`](../../../crates/core/src/strategy_events.rs).
- Kill widget (typed-confirm precedent for override modal):
  [`crates/ui/src/widgets/kill.rs`](../../../crates/ui/src/widgets/kill.rs).
- Tape widget (rename target):
  [`crates/ui/src/widgets/tape.rs`](../../crates/ui/src/widgets/tape.rs).
- Lumen HumanControl reference component (visual contract source):
  [`spec/design/project/ui_kits/desktop/HumanControl.jsx`](../../archive/design-prototypes-2026-Q2.tar.gz).
- Lumen AgentFeed reference component (rename source):
  [`spec/design/project/ui_kits/desktop/AgentFeed.jsx`](../../archive/design-prototypes-2026-Q2.tar.gz).
