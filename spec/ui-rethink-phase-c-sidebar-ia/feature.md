---
slug: ui-rethink-phase-c-sidebar-ia
status: shipped
owner: operator
updated: 2026-05-20
version: 0.1.0
predecessor: ui-rethink-phase-b-lab-run v0.2.0
---

# UI rethink Phase C — Sidebar IA flip + Live + Strategy registry + Settings rollup

> Third concrete feature carved out of
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/ui-rethink-2026-05-17.md).
> Dev-note §6 Phase C is the **scope source-of-truth**; this brief is
> the **implementation contract**. Predecessor:
> [`ui-rethink-phase-b-lab-run v0.2.0`](../ui-rethink-phase-b-lab-run/feature.md)
> shipped 2026-05-19. Lab + chart + Phase A/B groundwork stay intact;
> this phase replaces the surrounding shell.

## Why

Phase A renamed/added the `Screen::*` variants (Lab, Live, Compare,
Memory, Models, Trail, Settings) and switched the shell to
`SIDEBAR_ENTRIES_PHASE_A` (8 entries; legacy 7-entry shape retired from
the live UI). What Phase A did **not** do:

- Phase A's `screen_body` match (`crates/ui/src/shell.rs:78-94`) currently
  routes `Screen::Settings | Screen::Risk | Screen::Debug | Screen::Control`
  to `placeholder::view(strings::SETTINGS_PLACEHOLDER)`. The real
  `screens::risk::view`, `screens::debug::view`, and `screens::control::view`
  bodies are **dead code** (last reached only via the deprecated nav,
  which no longer renders). Phase C revives them as Settings sub-tabs.
- `Live` is routed to the **legacy 2×2 Home grid** (`home.rs:24-42`:
  `pnl + positions / strategies + agent_feed`). Dev-note §J6 specifies
  a different shape: full-width **equity curve**, KPI strip, open
  positions, recent activity, system-health row. Phase C lands the
  §J6 shape under `screens::live::view`.
- `Strategies` still renders the Phase 3 panel-style detail screen
  (`screens::strategies::view`: chip row + params block + recent events
  + pause/veto rows). Dev-note §4 (keep/change) prescribes a **list-of-
  cards registry** (J6 — what's available; what's tested; what's
  shipped) with status, last backtest anchor, last live-run timestamp,
  click-to-Lab. The runtime/panel content (params, pause, veto) is
  out-of-scope for Phase C — it migrates into Lab in Phase A+ or stays
  reachable via `Screen::Strategies` legacy alias (TBD per Q5 below).
- The sidebar has no **visual grouping** — `widgets::sidebar_nav::view`
  renders the 8 entries as one flat `Column` with uniform spacing.
  Dev-note §3 specifies a three-group structure (work / library /
  chrome) with operator-visible separators. Phase C lands the grouping.
- Six `Screen::*` variants are `#[deprecated]` (Home, Charts, Audit,
  Risk, Debug, Control) but actively referenced by **8 test files**
  (`tests/audit_filter_chip_emits_filter_changed.rs`,
  `tests/audit_row_opens_modal.rs`,
  `tests/chart_markers_from_audit_query.rs`,
  `tests/home_strategies_row_cross_link.rs`, `tests/panel_snapshots.rs`,
  `tests/render_snapshots.rs`, `tests/layout_invariants.rs`,
  `src/test_support.rs`). Phase C keeps the compat shim for one cycle;
  Phase D prunes per Q1.

## Scope (dev-note §6 Phase C — operator-locked)

1. **Sidebar IA visual grouping** — `widgets::sidebar_nav::view` learns to
   render three groups (work / library / chrome) with a divider between
   each. Group composition matches dev-note §3 lines 720-732:
   - **Work zone** (lines 721-724): Lab · Live · Compare
   - **Library zone** (lines 725-729): Strategies · Memory · Models · Trail
   - **Chrome zone** (lines 730-732): Settings (assistant slot reserved
     for Phase F — not introduced in Phase C)
   Entries themselves are unchanged from `SIDEBAR_ENTRIES_PHASE_A` (no
   adds, no removes) — only their visual relationship changes.
2. **`Live` screen body** — new `screens::live::view` replaces the
   home-grid pass-through. Dev-note §J6 sketch (lines 528-542):
   - Top: system-health row (latency, market-health, server-time skew,
     kill-threshold gauge, version badge) — collapses the current
     `screens::debug::view` system rows into one strip.
   - Mid: full-width **equity curve** (existing `widgets::equity_curve`)
     against today's paper-session running equity.
   - KPI strip (existing `widgets::kpi_strip`) — realized P&L,
     unrealized P&L, trade count, win rate, LLM spend.
   - 2-column row: open positions (`widgets::positions`) /
     recent activity (`widgets::agent_feed`).
   - LLM spend tile sources from `crates/llm` budget tracker if
     wired; otherwise falls back to placeholder per Q4 below.
3. **`Strategy registry` screen body** — new `screens::strategy_registry::view`
   reachable via `Screen::Strategies`. List-of-cards layout, one card
   per registered strategy. Each card surfaces:
   - Strategy ID + display name + status pill
     (`shipped` / `candidate` / `archived`)
   - Universe (symbols list, truncated)
   - Last backtest anchor (scenario name + version + sha256 prefix
     from `spec/anchors.toml`)
   - Last live-run timestamp (`Cockpit::strategies_recent_events` —
     newest `Run` event per `strategy_id`)
   - Primary action: **"Open in Lab"** button →
     `Message::SwitchScreen(Screen::Lab)` followed by
     `Message::SelectStrategy(id)` (mirrors the
     `home_strategies_row_cross_link` compound chain — Q11b precedent).
   The old detail-panel content (params block, chip row, pause rows,
   veto rows, sparkline) is **out of scope at Phase C**; per Q5
   default it migrates into Lab as a side-drawer in a follow-up
   (`lumen-design-adoption` track). For one cycle, the legacy detail
   view stays reachable via `Screen::Strategies` (no double-route —
   the registry replaces the screen body wholesale).
4. **`Settings` rollup** — new `screens::settings::view` with three
   sub-tabs surfacing the existing screen bodies unchanged:
   - **Risk** — `screens::risk::view` (per-venue exposure, daily loss,
     kill-threshold proximity). Source-of-truth content; no behaviour
     change.
   - **Control** — `screens::control::view` (`HumanControl` panel:
     execution-mode toggle + kill). Source-of-truth content; no
     behaviour change.
   - **Debug** — `screens::debug::view` (latency, per-venue market
     health, server time, version, logs placeholder). Source-of-truth
     content; **note** dev-note §J6 also pulls system-health rows into
     `Live`'s top strip — both surfaces show the same data (Live is
     glanceable, Debug is detail-table).
   Active sub-tab is operator-locked at `Risk` by default per Q2;
   no persistence across cockpit sessions in Phase C (operator
   preferences are a Lumen-design-adoption follow-up).
5. **Compat shim** — keep all six `#[deprecated]` `Screen::*` variants
   (`Home`, `Charts`, `Audit`, `Risk`, `Debug`, `Control`) for one
   cycle (Q1 default: Phase D removes them). Shell match-arm routing
   is unchanged from Phase A:
   - `Screen::Home` → `screens::live::view` (post-Phase-C body)
   - `Screen::Charts` → `screens::lab::view`
   - `Screen::Audit` → `screens::audit::view` (Trail, Phase D will
     replace)
   - `Screen::Risk` / `Screen::Debug` / `Screen::Control` →
     `screens::settings::view` with the corresponding sub-tab
     preselected (new wiring — Phase A routed them all to the same
     placeholder).

## Out of scope

- **Phase D (Trail view), Phase E (Compare matrix), Phase F (Memory +
  Models + Assistant slot).** Each is its own brief. The Settings
  rollup does **not** include the right-rail Assistant slot (that
  stays a Phase F deliverable per dev-note §6 Phase F lines 1106-1108).
- **Sidebar visual restyle beyond the IA grouping** — no Lumen-token
  changes, no new fonts, no animation overhaul. Visual polish lives
  in the existing `lumen-design-adoption` umbrella.
- **Removing the deprecated `Screen::*` variants.** Phase C keeps them
  as compat shims for one cycle (Q1 default: Phase D prunes).
- **New widgets for `Live` / `Strategy registry` / `Settings`** beyond
  what's listed in scope §2-4. Each reuses existing widgets (`equity_curve`,
  `kpi_strip`, `positions`, `agent_feed`, `latency`, `pnl`) where possible.
  Net-new widgets: (a) `widgets::strategy_card` for the registry list,
  (b) `widgets::settings_tabs` for the three-tab chrome on Settings,
  (c) `widgets::sidebar_divider` for the three-group separator. All
  three are Tier-2 surface compositions of existing Lumen primitives —
  no new tokens.
- **Operator persistence for active Settings sub-tab** (Q2 deferred —
  Lumen-design-adoption follow-up).
- **Migrating params/pause/veto from the legacy Strategies detail screen
  into Lab as a side-drawer.** Tracked separately under the Lab umbrella;
  Phase C leaves the detail panel reachable via the deprecated
  `Screen::Strategies` route IF Q5 lands "keep detail panel" (default:
  remove — registry is the single source).
- **Migrating the Live system-health strip from `screens::debug::view`**.
  Both surfaces show the same data in Phase C (Live = glanceable strip;
  Settings → Debug tab = detail table). The intentional duplication is
  documented; Phase D+ may consolidate.
- **LLM spend tile wiring**. If `crates/llm` budget tracker is not
  already exposing a cockpit-friendly `Cockpit::llm_spend_today` field,
  the tile shows a placeholder. Wiring is a backend ticket (Q4 default:
  defer — placeholder is fine for Phase C).

## Requirements

> R-rows numbered to support trace.toml; mapped to dev-note §6 Phase C
> + §3 + §J6. Each row is testable and falsifiable.

### R1 — Sidebar three-group IA

- **R1.1** `widgets::sidebar_nav::view` accepts an additional parameter
  describing the three-group composition (slice of slices, or a single
  flat slice + group-boundary indices — architect picks the shape).
- **R1.2** Renders work / library / chrome groups in scan order with a
  visible divider between each. Divider style operator-decide per Q3
  (default: 1-px `BORDER_1` hairline, full-width inside the sidebar
  panel — matches the sidebar's right-edge hairline).
- **R1.3** Default boot sidebar shows: `Lab · Live · Compare ─ Strategies ·
  Memory · Models · Trail ─ Settings` (8 entries; 2 dividers).
- **R1.4** Active-row left-rule accent (T1507) still works on every
  entry regardless of group.
- **R1.5** Sidebar snapshot test
  (`sidebar_nav__phase_c_three_groups`) pins the grouped layout.
- **R1.6** `sidebar__phase_a_workflow_group` snapshot test stays
  byte-identical OR is migrated under a new name with Phase A snapshot
  preserved as `.snap.disabled` for one cycle (tester decides).

### R2 — `Live` screen body

- **R2.1** New module `crates/ui/src/screens/live.rs`.
- **R2.2** Layout matches dev-note §J6 lines 528-542 sketch: system-
  health strip on top, equity curve full-width, KPI strip, 2-column
  positions/activity row at bottom.
- **R2.3** Shell `screen_body` match routes `Screen::Live` to
  `live::view`; `Screen::Home` (deprecated) routes to the same body
  (compat shim).
- **R2.4** The existing `home::view` is **retained** for one cycle
  (no source-file deletion) — `tests/home_strategies_row_cross_link.rs`
  + `tests/render_snapshots.rs` continue to exercise it through
  `Screen::Home` until Phase D pruning. Q1 default ratification.
- **R2.5** Per-day LLM spend tile sources from the LLM budget tracker
  if available; otherwise shows `PLACEHOLDER_NONE` per Q4 default.
- **R2.6** Visual snapshot
  (`live_snapshot__steady_state` under
  `tests/panel_snapshots.rs` or a dedicated `tests/live_snapshots.rs`).

### R3 — `Strategy registry` screen body

- **R3.1** New module `crates/ui/src/screens/strategy_registry.rs`.
- **R3.2** List-of-cards layout, one card per registered strategy from
  `Cockpit::strategies` (`PanelState::Ready(rows)`).
- **R3.3** Each card surfaces: ID, status pill, universe (truncated),
  last backtest anchor (scenario + version + sha256 prefix), last live
  run timestamp.
- **R3.4** Primary action button **"Open in Lab"** emits a compound
  message chain:
  `Message::SwitchScreen(Screen::Lab)` + `Message::SelectStrategy(id)`.
  Architect picks the dispatch pattern (single new `Message::OpenInLab(id)`
  variant OR `Task::batch` of existing two — `home_strategies_row_cross_link`
  precedent uses the two-message chain via `Task::done`).
- **R3.5** Shell `screen_body` match routes `Screen::Strategies` to
  `strategy_registry::view`. The legacy panel-style
  `screens::strategies::view` is retained as a source file for one
  cycle (Phase D prunes; tracked under Q5).
- **R3.6** Empty-state per Q3 default: render
  `widgets::frame::muted_body("No strategies registered. Run a backtest
  in Lab to register one.")`. No CTA button at Phase C.
- **R3.7** Loading/Error states surface via `PanelState` mirror
  (existing `loading_with_spinner` / `frame::error_body`).
- **R3.8** Visual snapshots:
  `strategy_registry_snapshot__empty`,
  `strategy_registry_snapshot__three_strategies`.

### R4 — `Settings` rollup

- **R4.1** New module `crates/ui/src/screens/settings.rs` with
  `Screen::Settings` routing.
- **R4.2** Three-tab chrome at top of body — tab labels: `Risk` ·
  `Control` · `Debug`. Tab order operator-decide per Q2 (default:
  Risk · Control · Debug — matches dev-note §3 ordering: Risk most-
  consulted, Control safety-action, Debug ops chrome).
- **R4.3** Active tab determined by **either** (a) explicit operator
  click (new `Message::SwitchSettingsTab(SettingsTab)` variant —
  architect-decide), OR (b) shell deep-link via deprecated `Screen::Risk`
  / `Screen::Control` / `Screen::Debug` route — the shell pre-selects
  the matching tab when routed through the deprecated alias.
- **R4.4** Tab body renders the existing screen view unchanged:
  - `SettingsTab::Risk` → `screens::risk::view(model, mode)`
  - `SettingsTab::Control` → `screens::control::view(model, mode)`
  - `SettingsTab::Debug` → `screens::debug::view(model, mode)`
- **R4.5** No tab-state persistence across cockpit boots in Phase C
  (default tab = `Risk` on every boot per Q2 default).
- **R4.6** Visual snapshots:
  `settings_snapshot__risk_tab_active`,
  `settings_snapshot__control_tab_active`,
  `settings_snapshot__debug_tab_active`.

### R5 — Compat shim contract (one cycle)

- **R5.1** Six `#[deprecated]` `Screen::*` variants stay: `Home`,
  `Charts`, `Audit`, `Risk`, `Debug`, `Control`.
- **R5.2** Shell `screen_body` match routes each deprecated variant to
  its successor body:
  - `Home` → `live::view`
  - `Charts` → `lab::view` (already wired Phase A)
  - `Audit` → `audit::view` (already wired Phase A; Phase D replaces
    with Trail)
  - `Risk` → `settings::view` **pre-selected to Risk tab**
  - `Debug` → `settings::view` **pre-selected to Debug tab**
  - `Control` → `settings::view` **pre-selected to Control tab**
- **R5.3** Test files that reference deprecated variants continue to
  compile and pass without code changes:
  - `tests/audit_filter_chip_emits_filter_changed.rs`
  - `tests/audit_row_opens_modal.rs`
  - `tests/chart_markers_from_audit_query.rs`
  - `tests/home_strategies_row_cross_link.rs`
  - `tests/panel_snapshots.rs` (5 deprecated-variant assertions)
  - `tests/render_snapshots.rs`
  - `tests/layout_invariants.rs`
- **R5.4** Per Q1 default, Phase D prunes the shim (variants → removed;
  test-files migrate to non-deprecated names in the same cycle).
- **R5.5** Sidebar widget does **not** render any deprecated variant
  as a nav entry. The deprecated routes are reachable only via direct
  `Message::SwitchScreen(...)` dispatch from test harness or compat
  links (already Phase A behaviour; Phase C preserves).

### R6 — Net-new widgets

- **R6.1** `widgets::strategy_card` — Tier-2 surface card composing
  text, status pill, anchor display, timestamp, and `Open in Lab`
  button. Reuses `frame::panel`, status-pill style from
  `widgets::positions`, existing button style. **No new tokens.**
- **R6.2** `widgets::settings_tabs` — three-tab chrome strip. Active
  tab carries bottom-edge accent rule (existing T1609 chip pattern
  reused). **No new tokens.**
- **R6.3** Sidebar divider — either inline in `sidebar_nav::view` or a
  thin `widgets::sidebar_divider` helper (architect-decide). 1-px
  `BORDER_1` hairline matching the sidebar's right edge. **No new
  tokens.**
- **R6.4** Net-new widget count: 3 (`strategy_card`, `settings_tabs`,
  sidebar divider). All three are Tier-2 compositions of existing
  Lumen primitives. **Zero new tokens; zero new external deps.**

### R7 — String table

- **R7.1** All new copy goes through `crate::strings`. No string
  literals in widget/screen bodies (existing `cockpit-strings-table`
  contract).
- **R7.2** New constants required (analyst-suggested names; architect
  may rename):
  - `LIVE_HEADLINE`, `LIVE_SYSTEM_HEALTH_LABEL`, `LIVE_LLM_SPEND_LABEL`,
    `LIVE_LLM_SPEND_PLACEHOLDER`
  - `STRATEGY_REGISTRY_PANEL_TITLE`, `STRATEGY_REGISTRY_EMPTY`,
    `STRATEGY_REGISTRY_OPEN_IN_LAB_LABEL`, `STRATEGY_REGISTRY_STATUS_SHIPPED`,
    `STRATEGY_REGISTRY_STATUS_CANDIDATE`, `STRATEGY_REGISTRY_STATUS_ARCHIVED`,
    `STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX`,
    `STRATEGY_REGISTRY_LAST_RUN_PREFIX`,
    `STRATEGY_REGISTRY_UNIVERSE_PREFIX`
  - `SETTINGS_TAB_RISK`, `SETTINGS_TAB_CONTROL`, `SETTINGS_TAB_DEBUG`
  - The existing `SETTINGS_PLACEHOLDER` constant becomes unused
    (Phase C wires the real body) — analyst leaves it deprecated for
    one cycle alongside the variants; Phase D prunes.

### R8 — `Cockpit` state additions

- **R8.1** `Cockpit::settings_active_tab: SettingsTab` (new enum,
  `Default::Risk`).
- **R8.2** Optional `Cockpit::llm_spend_today: Option<Decimal>` field
  IF Q4 lands "wire now"; otherwise no state change (placeholder
  copy is enough).
- **R8.3** Strategy status (shipped/candidate/archived) currently has
  no on-`Cockpit` field. Two options for architect:
  - (a) infer from `Cockpit::strategies_config` metadata (add `status`
    field to `StrategyConfigEntry` if absent),
  - (b) hardcode "shipped" for every registered row at Phase C; status
    discrimination is a follow-up.
  Default per Q5: option (b) — Phase C ships a uniform "shipped"
  pill; status discrimination is Phase D registry-content work.

### R9 — `Message` additions

- **R9.1** Optional new variant `Message::SwitchSettingsTab(SettingsTab)`
  if R4.3(a) is chosen.
- **R9.2** Optional new variant `Message::OpenInLab(StrategyId)` if
  R3.4 architect picks the single-message form.
- **R9.3** Otherwise no `Message` additions — existing `SwitchScreen`
  + `SelectStrategy` chain via `Task::done` covers R3.4.

### R10 — Non-regression contract

> Inherits the predecessor's contract; Phase C adds zero new strategy /
> audit / exec / report paths so anchor risk is **zero by construction**
> (dev-note §6 Phase C: *"Anchor risk: zero"*).

- **R10.1** **22 body-SHA-256 anchors stay byte-identical**
  (`spec/anchors.toml`). Verified by `scripts/verify_anchors.sh`
  exit 0 — non-negotiable.
- **R10.2** Lab + chart + Train panel + Run button + `run_delta_badge`
  + `engine::run_scenario` + `lab::runner::spawn_lab_run` all stay
  byte-identical. Phase B contract carries forward.
- **R10.3** `cockpit-smoke` PASS — 0 panic lines in 8 s window
  (cockpit-performance v1.0.0 R10.4 pattern inherited).
- **R10.4** `cockpit-performance-and-input-responsiveness v1.0.0`
  **idle-CPU floor ≤ 13.1%** stays under budget. Phase C adds no new
  `tokio::time::interval` redraws, no new subscriptions per compositor
  frame, no new periodic widgets. The `Live` equity curve reuses the
  existing widget (no new render loop). Tab switches inside Settings
  do not redraw the whole shell — only the body.
- **R10.5** `cargo fmt --check` + `cargo clippy --workspace --
  -D warnings` exit 0.
- **R10.6** `cargo test --workspace --lib` 100 % PASS.
- **R10.7** All Phase A snapshot baselines (sidebar, render, panel,
  layout, gallery, visual) stay green OR are migrated under
  named-versioned baselines with the old baseline preserved as
  `.snap.disabled` for one cycle.
- **R10.8** `spec-lint` Phase C contribution = 0 (current baseline 87
  carry-forward).
- **R10.9** **No new external crate deps**, no new Lumen tokens, no
  iced version bump.
- **R10.10** Zero string literals; zero hex colours in net-new code
  (cockpit-strings-table + theme-token contracts inherited).

## Open questions for operator (Q1-Q5)

> **OPERATOR DECIDED 2026-05-20 via "Autoapprove all" directive — all
> 5 Qs resolved to analyst-recommended defaults:**
> Q1 = **Q1a one cycle** (Phase D prunes the `#[deprecated]` variants);
> Q2 = **Q2a Risk · Control · Debug ordering; default tab = Risk**;
> Q3 = **Q3a hairline 1px `BORDER_1` divider** (operator-friendly,
> low visual noise);
> Q4 = **Q4b placeholder LLM tile now** (real wiring lands in Phase F
> alongside Memory + Models);
> Q5 = **Q5a delete legacy `strategies::view` wholesale** — Strategy
> registry is the single source; params / pause / veto detail-panel
> migrates into Lab as a side-drawer follow-up (`lumen-design-adoption`
> track). Architect proceeds against these defaults.

> Original framing — these five questions needed operator answers
> before architect could spawn. Analyst-recommended defaults are
> listed; operator may override.

### Q1 — Compat-shim retirement window

**Question.** How long do the six deprecated `Screen::*` variants
(`Home`, `Charts`, `Audit`, `Risk`, `Debug`, `Control`) stay alive?

- **Q1a — One cycle (Phase D prunes).** Phase D is the next phase per
  the dev-note's phase ordering; pruning at that boundary is the
  shortest defensible window. Test files migrate in the same Phase D
  ticket.
- **Q1b — Two cycles (Phase F prunes).** Defers the prune until the
  whole IA stack lands (Trail + Compare + Memory + Models + Assistant
  slot). Reduces churn on the test harness if Phase D adds further
  Screen variants (likely — Trail will replace `Audit`).
- **Q1c — Permanent compat (never prune).** Keep the deprecated
  variants for external callers. Not recommended — the `#[deprecated]`
  attribute already documents the migration; pruning is a no-op for
  call-site code if migration is done.

**Analyst default: Q1a (one cycle, Phase D prunes).** Rationale:
the test-file migration cost is low (8 files, ~30 `Screen::*` references
total per the grep audit). Two-cycle deferral compounds with Phase D's
own new variants. Cleanest cut at Phase D.

**Risk if wrong:** wrong direction here forces a re-migration in
Phase D or Phase F. K6 risk register entry covers test-harness
migration.

### Q2 — Settings tab default + ordering

**Question.** Which sub-tab opens by default when the operator clicks
`Settings`? In what scan order do the three tabs render?

- **Q2a — Risk · Control · Debug.** Risk is the most-consulted
  surface for operator daily use (exposure caps, daily loss, kill
  proximity); Control is the safety panel (mode toggle + kill);
  Debug is ops chrome. Default sub-tab = Risk.
- **Q2b — Control · Risk · Debug.** Control-first emphasises the
  "human-in-the-loop" framing — the operator is the loop.
- **Q2c — Debug · Risk · Control.** Ops-first; matches the
  alphabetical / Phase 5 sidebar order.

**Analyst default: Q2a (Risk · Control · Debug; default tab = Risk).**
Rationale: dev-note §3 lists Risk first in the chrome rollup; Risk is
the panel the operator opens reactively most often (per dev-note §1
audit "Risk … lives as a separate screen the operator only opens
reactively"). Putting it first matches its consultation frequency.

**Risk if wrong:** trivial — operator can override at any time; a
follow-up Lumen ticket can add persistence.

### Q3 — Sidebar group divider visual

**Question.** How does the sidebar render the three-group separation?

- **Q3a — Hairline divider (1 px `BORDER_1`).** Matches the sidebar's
  right-edge hairline; visually subtle; no new tokens. Default.
- **Q3b — Section header (group label above each group).** Denser;
  requires three new strings (`SIDEBAR_GROUP_WORK`,
  `SIDEBAR_GROUP_LIBRARY`, `SIDEBAR_GROUP_CHROME`); arguably
  redundant since entry labels themselves describe the group.
- **Q3c — Extra vertical spacing only (no rule).** Cleanest
  minimalism; possibly too subtle for new operators.

**Analyst default: Q3a (hairline).** Rationale: dev-note §3 lines
725, 730 use `─────` (a horizontal rule) in the IA sketch — the
hairline is the literal visual equivalent. No new tokens; consistent
with the existing right-edge sidebar hairline; matches the established
Lumen visual vocabulary (panel separators).

**Risk if wrong:** trivial — divider style is a 1-line widget change;
follow-up ticket if operator wants a different presentation.

### Q4 — `Live` LLM spend tile wiring

**Question.** Does the `Live` screen's LLM spend tile read from the
`crates/llm` budget tracker now, or show a placeholder?

- **Q4a — Wire now.** Adds `Cockpit::llm_spend_today: Option<Decimal>`
  field + the subscription/poll wiring. Touches outside crates/ui
  (small cost in `crates/llm`'s public surface).
- **Q4b — Placeholder now, wire later.** Tile shows `—` or
  `STRATEGIES_SPARKLINE_LOADING`-style copy; spend field stays
  unwired. Cleanest scope cut. Default.
- **Q4c — Skip the tile entirely (Phase C scope omits LLM spend).**
  Drops it from the KPI strip; operator gets it back when v2 LLM
  ships and the right-rail Assistant slot wakes (Phase F).

**Analyst default: Q4b (placeholder now).** Rationale: dev-note §J6
lists LLM spend at lines 519, 535 as a v2-onwards target. Today's
cockpit doesn't run v2 LLM; the data isn't meaningful yet. Placeholder
preserves the surface for Phase F.

**Risk if wrong:** trivial — tile copy is a 1-line strings.rs edit;
wiring is a 1-day backend ticket either way.

### Q5 — Legacy `screens::strategies::view` disposition

**Question.** What happens to the Phase 3 detail-panel `strategies::view`
(chip row + params + pause + veto rows) after Phase C ships the
registry?

- **Q5a — Delete the detail panel; registry is the single source.**
  Cleanest. But the params/pause/veto content has no replacement in
  Phase C — it disappears from the operator-visible surface until a
  follow-up migrates it into Lab.
- **Q5b — Keep the detail panel as a side-drawer in the registry.**
  Click a `strategy_card` → drawer slides in with params/pause/veto.
  Adds a net-new widget (`strategy_drawer`) and ~1 week of work.
- **Q5c — Keep the detail panel reachable via a "Details" button on
  each registry card** (opens an inline expansion, no new screen).
  Light-touch — but adds visual complexity to the card.
- **Q5d — Keep the legacy `strategies::view` reachable via the
  deprecated `Screen::Strategies` route AND ship the registry as
  `Screen::StrategyRegistry`** (new variant). Two parallel screens
  for one cycle; Phase D prunes the legacy. Mirrors the Q1 compat
  pattern.

**Analyst default: Q5a (delete the detail panel; registry is the
single source).** Rationale: dev-note §4 keep/change (line 757)
explicitly says *"params block migrates into Lab as a side-drawer"* —
the Lab umbrella owns that migration, not Phase C. Phase C's job is
to land the registry; the detail panel's content reappears in Lab as
a follow-up. Operator must accept a temporary regression in pause /
veto reachability — these flows are also reachable from the agent
feed / risk surfaces.

**Risk if wrong:** medium — pause and veto are operational safety
controls. K2 risk register covers this. If operator picks Q5b or Q5c,
Phase C scope grows by ~1 week.

## Hypothesis register (H1-H5)

> Falsifiable claims the tester can verify post-implementation.

- **H1 — Anchor risk is zero by construction.** Phase C touches no
  strategy / audit / exec / report code paths. `scripts/verify_anchors.sh`
  exits 0; all 22 sha256 prefixes match. **Falsifier:** any anchor
  diff in the test report.
- **H2 — Idle-CPU stays ≤ 13.1 %.** No new subscriptions, no new
  `tokio::time::interval`, no new periodic widgets. **Falsifier:**
  cockpit-performance v1.0.0 idle-CPU sample > 13.1 % across the
  three-run median.
- **H3 — Settings tab switch is < 10 ms wall-clock.** Tab switch is a
  pure body re-render (no new fetch, no I/O). **Falsifier:** any
  perceptible UI hitch during tab switching in cockpit-smoke.
- **H4 — Operator muscle memory transfers within one session.** With
  the deprecated nav already gone since Phase A (8-entry sidebar with
  Lab as default), the Phase C grouping is a visual layering, not an
  IA reshuffle. **Falsifier:** operator review post-ship reports
  reduced findability (subjective — captured in presenter deck).
- **H5 — Existing snapshot baselines that touch Risk / Debug / Control
  panel bodies stay byte-identical.** The Settings rollup wraps the
  bodies unchanged. **Falsifier:** panel_snapshots Risk / Debug /
  Control diff. (Note: the *wrapper* changes — the body content does
  not — so wrapper-level snapshots will diff; body-level snapshots
  will not.)

## Risk register (K1-K6)

> Risks the dev-note didn't enumerate; ordered by likelihood × impact.

### K1 — Operator muscle-memory disruption

**Risk.** Operator expects to find "Home" in the nav and finds "Live"
instead. Phase A already renamed the variant + sidebar entry, but
Phase C is the first cycle that drops a *visually different shape*
on the operator (3 groups instead of flat 8).

**Mitigation.**
- Compat shim (R5) routes `Screen::Home` to `live::view` transparently.
- Default boot screen is `Lab` (unchanged from Phase A; per `Screen::default()`
  in state.rs:100).
- Dev-note §3 lines 720-732 explicitly designed the three-group
  structure with muscle memory in mind ("The top group is the
  everyday workflow … muscle memory").
- Presenter deck (M-PRES) should call out the rename + IA flip
  prominently so operator-approval sweep catches surprise.

**Severity if mitigation fails:** low (operator can re-learn within
one session; the variant rename happened in Phase A without complaint).

### K2 — Settings rollup hides currently-discoverable Risk / Control / Debug

**Risk.** Operator opens "Risk" expecting a top-level screen; finds
"Settings" instead. One extra click + tab-pick to get to the same
data.

**Mitigation.**
- Compat shim R5.2: `Screen::Risk` / `Screen::Debug` / `Screen::Control`
  routes to `settings::view` **pre-selected to the matching tab** —
  bookmark/deep-link paths still work.
- System-health row in the new `Live` screen (R2.2) surfaces the
  most-consulted Debug data (latency, market-health, kill proximity)
  at the daily-tick level — no Settings drill needed for glance.
- Status-bar kill dot (already shipped in cockpit-strings-table or
  T1609 — verify with architect) keeps the kill action one-click from
  any screen.

**Severity if mitigation fails:** medium (Risk surfacing during fast
markets is operationally-critical; one extra click could matter at the
worst possible moment). **Counter:** the kill action stays in the
status bar; only the *display* of risk metrics migrates to Settings.

### K3 — Strategy registry empty-state UX

**Risk.** Fresh cockpit boots with no strategies in the registry.
Operator sees a blank screen with no obvious next action.

**Mitigation.**
- R3.6 default: `muted_body("No strategies registered. Run a backtest
  in Lab to register one.")` — points the operator at the next action
  (open Lab).
- Cockpit defaults already ship `Cockpit::strategies` populated from
  `strategies.toml`; truly-empty registry is rare in practice.
- Q3 default (Q3a hairline) keeps the sidebar visible — operator can
  navigate elsewhere immediately.

**Severity if mitigation fails:** low (only affects fresh installs;
the muted body copy is one strings.rs entry to fix if operator
disapproves).

### K4 — Sidebar visual grouping clarity

**Risk.** The hairline divider per Q3a default is too subtle; operator
doesn't perceive the three groups and the IA flip lands as a no-op.

**Mitigation.**
- A/B-testable in cockpit-smoke / visual snapshot — tester can pin
  the rendered divider style.
- Q3b (section header) is a 1-day fallback if Q3a tests poorly.
- Presenter deck should screenshot the new sidebar prominently so
  operator-approval sweep catches the issue.

**Severity if mitigation fails:** low (visual polish iteration; a
follow-up ticket can land a header or thicker divider).

### K5 — Live screen reuses Home widgets — rename risk

**Risk.** `screens::live::view` wires existing widgets (`equity_curve`,
`positions`, `agent_feed`, `pnl`, `kpi_strip`). Some of these were
authored when the screen was called "Home" — their internal copy /
labels may reference "Home" or assume the 2×2 grid layout.

**Investigation findings:**
- `home.rs:24-42` is a thin compositor — it doesn't bake "Home"
  copy into widget bodies. All widget copy already lives in
  `strings.rs`.
- `widgets::pnl`, `widgets::positions`, `widgets::agent_feed`,
  `widgets::strategies` are independent of screen name.
- `widgets::equity_curve` and `widgets::kpi_strip` are widely reused
  (Lab, viewer, gallery).

**Mitigation.** R2.4 keeps the legacy `home::view` source file
intact for one cycle — if a widget hides a layout assumption,
`Screen::Home` (deprecated alias) still routes to the legacy 2×2
grid as a fallback. Phase D prunes once Live is operator-confirmed.

**Severity if mitigation fails:** low (Live can fall back to
re-using `home::view` body verbatim if a widget hides a regression).

### K6 — Deprecated `Message::SwitchScreen` variants need careful test migration

**Risk.** Eight test files reference deprecated `Screen::*` variants
(see R5.3). Phase D's prune ticket must migrate all of them in
lockstep — easy to miss a file.

**Mitigation.**
- Phase C's tasks.md M-FINAL gate includes an explicit "deprecated-
  variant usage census" task — grep the workspace at ship and pin
  the count (currently ~77 references across the crates/ui tree).
  Phase D's prune ticket starts from the census number as the
  migration scope.
- The `#[deprecated]` attribute already emits a `warn` on use; if
  Phase C accidentally adds new references, the compile log surfaces
  them (`cargo clippy --workspace -- -D warnings` would block).

**Severity if mitigation fails:** low (Phase D ticket — the migration
is mechanical search-and-replace; failure mode is delayed ship, not
broken behaviour).

## Non-regression contract (R10 expanded)

> Reiterated from R10 for tester audit convenience.

1. **22 body-SHA-256 anchors stay byte-identical** (R10.1, dev-note
   §6 Phase C "Anchor risk: zero" — `spec/anchors.toml` 22 entries,
   `scripts/verify_anchors.sh` exit 0).
2. **Lab + chart + Phase A/B surface unchanged** (R10.2) — Lab screen,
   chart widget, Train sub-panel, Run button, `run_delta_badge`,
   `engine::run_scenario` dispatch, `EquityCache`, `LabRunRequested`/
   `LabRunCompleted` all byte-identical.
3. **`cockpit-smoke` PASS** — 0 panic lines in 8 s window (R10.3,
   inherited pattern from cockpit-performance v1.0.0 R10.4).
4. **`cockpit-performance v1.0.0` idle-CPU ≤ 13.1 %** (R10.4) — three-
   run median across `cargo run --bin cockpit` with no-data feeds.
5. **`cargo fmt --check` + `cargo clippy --workspace -- -D warnings`
   exit 0** (R10.5).
6. **`cargo test --workspace --lib`** 100 % PASS (R10.6).
7. **Snapshot baselines** — Phase A baselines stay green OR migrate
   under named-versioned files with `.snap.disabled` carry-over for
   one cycle (R10.7).
8. **`spec-lint`** Phase C contribution = 0 (R10.8, baseline 87 carry-
   forward).
9. **Zero new external deps; zero new Lumen tokens; no iced bump**
   (R10.9).
10. **Zero string literals; zero hex colours** in net-new code
    (R10.10, existing contracts).

## Acceptance criteria per milestone

> Architect decomposes M-T1+ once Q1-Q5 land. Analyst pins the M0 +
> M-FINAL gates here.

### M0 — Analyst pass (this brief)

- [x] R1-R10 anchored to dev-note §6 Phase C + §3 + §J6.
- [x] Q1-Q5 surfaced with analyst-recommended defaults.
- [x] K1-K6 risk register populated.
- [x] H1-H5 hypothesis register populated.
- [x] Non-regression contract reiterated.
- [x] `tasks.md` M0 + M-FINAL gates refined.
- [ ] Operator answers Q1-Q5. **← blocker before architect spawn.**

### M-T1+ — Architect decomposition (operator unblocks)

To be populated by architect post-operator-answers.

### M-FINAL — Tester sweep + ship

- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D warnings`
  exit 0.
- [ ] `cargo test --workspace --lib` 100 % PASS.
- [ ] `cargo test -p ui --test render_snapshots --test visual_snapshots
  --test panel_snapshots --test layout_invariants --test
  home_strategies_row_cross_link --test audit_filter_chip_emits_filter_changed
  --test audit_row_opens_modal --test chart_markers_from_audit_query`
  — visual baselines for Live / Strategy registry / Settings land;
  Phase A baselines for sidebar migrate cleanly; deprecated-variant
  test files stay green via compat shim.
- [ ] `scripts/verify_anchors.sh` → ANCHORS PASS (22 / 22) —
  non-negotiable.
- [ ] `cockpit-smoke` → 0 panic lines in 8 s window.
- [ ] Cockpit-performance v1.0.0 idle-CPU ≤ 13.1 % verified post-flip
  (three-run median).
- [ ] Deprecated-variant usage census appended to test report
  (grep count + per-file breakdown — Phase D prune budget).
- [ ] `scripts/spec_lint.py` → Phase C contribution = 0.
- [ ] Author `spec/ui-rethink-phase-c-sidebar-ia/reports/test-final-<YYYY-MM-DD>.md`
  per `.claude/skills/rust-test/templates/test-report.md`.
- [ ] Presenter deck `spec/ui-rethink-phase-c-sidebar-ia/presentations/
  ui-rethink-phase-c-sidebar-ia-<YYYY-MM-DD>.md` for operator
  approval.

## Trace

Trace row `REQ-UI-RETHINK-PHASE-C-001` opened in `proposed` state by
orchestrator promotion pass (2026-05-20). Analyst pass leaves it
`proposed` — architect transitions to `accepted` after operator
unblocks Q1-Q5.

## Design

> Architect pass 2026-05-20. Operator unblocked Q1-Q5 via "Autoapprove
> all" → analyst defaults; this Design section locks shapes the developer
> needs before opening an editor. Anchor risk remains zero by
> construction.

### A1 — Sidebar divider: inline in `sidebar_nav::view`, no new widget

The divider is a 6-line `Container { width=Fill, height=Fixed(1.0),
background=BORDER_1 }` — identical construction to the existing
right-edge hairline (`crates/ui/src/widgets/sidebar_nav.rs:116-122`).
A separate `widgets::sidebar_divider.rs` would add a net-new module,
import surface, and `mod.rs` entry for **one** call site (the sidebar
itself). Inline keeps cyclomatic complexity in one file and matches the
analyst's "recommended (cleaner)" framing.

**Net-new file count: 5** (3 screens + 2 widgets — `strategy_card`,
`settings_tabs`). Down from the analyst's optional 6th.

### A2 — Group composition: `&[&[Screen]]` slice-of-slices

New const in `crates/ui/src/theme.rs` next to `SIDEBAR_ENTRIES_PHASE_A`
(line 719):

```rust
/// Phase C — three-group sidebar IA. `flatten()` over this slice must
/// equal `SIDEBAR_ENTRIES_PHASE_A` (asserted by
/// `sidebar_groups_phase_c__flatten_matches_phase_a` test).
pub const SIDEBAR_GROUPS_PHASE_C: &[&[Screen]] = &[
    &[Screen::Lab, Screen::Live, Screen::Compare],            // work
    &[Screen::Strategies, Screen::Memory, Screen::Models, Screen::Trail], // library
    &[Screen::Settings],                                       // chrome
];
```

`sidebar_nav::view` accepts `groups: &[&[Screen]]` as a new parameter
(additive). The existing `entries: &[Screen]` parameter is **kept** for
one cycle as deprecated input (call sites that pass it auto-route to a
single-group rendering for backwards-compat). Phase D removes the flat
parameter once all call sites migrate.

Alternative considered: flat slice + group-boundary indices. Rejected —
boundary-index encoding is error-prone and requires runtime bounds
checks; slice-of-slices is the iced-native shape.

### A3 — Public `Message` surface: one new variant only

```rust
/// Wave D — Settings sub-tab switch. Pure assignment; no I/O.
SwitchSettingsTab(SettingsTab),
```

R3.4 "Open in Lab" uses the existing two-message chain
(`SwitchScreen(Screen::Lab)` + `SelectStrategy(id)`) dispatched from the
binary's `Task::done` wrapper — same pattern as
`home_strategies_row_cross_link.rs:48-56`. **No** `Message::OpenInLab`
variant. The chain lives in the bin layer (`bin/cockpit.rs`,
`bin/cockpit_live.rs`), not in `update`.

Alternative considered: `Message::OpenInLab(StrategyId)`. Rejected — adds
a second variant for a flow the precedent already covers in one message
chain; constraint pins one max.

### A4 — Deep-link tab pre-selection: extend the existing `SwitchScreen` arm

R5.2 requires `Screen::Risk` / `Screen::Debug` / `Screen::Control` deep
links to land the operator on the Settings screen with the matching tab
pre-selected. Implementation: extend `update`'s `SwitchScreen` arm
(`crates/ui/src/state.rs:1520-1522`):

```rust
Message::SwitchScreen(s) => {
    model.current_screen = s;
    // R5.2 deep-link: deprecated Risk/Debug/Control aliases pre-select
    // the matching Settings tab on the way through.
    #[allow(deprecated)]
    match s {
        Screen::Risk    => model.settings_active_tab = SettingsTab::Risk,
        Screen::Control => model.settings_active_tab = SettingsTab::Control,
        Screen::Debug   => model.settings_active_tab = SettingsTab::Debug,
        _ => {}
    }
}
```

Side-effect colocated with the routing decision; no new message variant
for the deep-link path. The shell match continues to route
`Settings | Risk | Debug | Control` → `settings::view` — the body reads
`model.settings_active_tab` to pick which sub-body to render.

Alternative considered: a separate `Message::OpenSettingsTab(SettingsTab)`
variant emitted from the bin's `SwitchScreen` interceptor. Rejected —
splits one logical operator action across two messages and makes the
single-message `cargo test --workspace --lib` round-trip harder.

### A5 — `SettingsTab` enum shape

New enum in `crates/ui/src/state.rs` adjacent to `Screen` (insert after
the `Screen` enum at line 95):

```rust
/// Phase C — Settings rollup sub-tab selector. Renders the three
/// existing screen bodies (Risk / Control / Debug) unchanged inside
/// `screens::settings::view`. Cold-start default `Risk` per Q2a.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    /// Risk / limits (most-consulted) — `screens::risk::view`.
    #[default]
    Risk,
    /// HumanControl (mode toggle + kill) — `screens::control::view`.
    Control,
    /// Operations chrome (latency, market health, server time, logs) —
    /// `screens::debug::view`.
    Debug,
}
```

`Default::Risk` per Q2a. `Cockpit::settings_active_tab: SettingsTab`
field added at line 745 (immediately after `current_screen`) so the
struct stays grouped by feature.

### A6 — Strategy status: literal "shipped" for every row at Phase C

Per Q5a / R8.3b, the registry card status pill renders
`STRATEGY_REGISTRY_STATUS_SHIPPED` for every `StrategyRow`. No
`Cockpit::strategies_config.entries[*].status` field added; no
`StrategyStatus::*` extension. Status discrimination
(shipped/candidate/archived) is Phase D registry-content work.

Implication: `STRATEGY_REGISTRY_STATUS_CANDIDATE` and
`STRATEGY_REGISTRY_STATUS_ARCHIVED` constants from R7.2 still ship
(unused) so Phase D's registry-content ticket has the copy ready —
deprecation-attribute pattern follows `SETTINGS_PLACEHOLDER` precedent.

### A7 — Live equity-curve: PanelState::Loading placeholder (no new state)

`screens::live::view` renders the equity curve via the existing
`widgets::equity_curve::view(series, mode)` but feeds it
`&PanelState::Loading` — the widget's existing
`empty_with_label(VIEWER_NO_EQUITY_DATA, mode)` arm
(`equity_curve.rs:47`) is the placeholder. No new `Cockpit` field;
no new subscription; no new periodic redraw. Phase F (or a sibling
backend ticket) wires the live paper-session equity feed.

**Message-type adapter.** `widgets::equity_curve::view` and
`widgets::kpi_strip::view` return `Element<'_, ViewerMessage>` (verified
at `equity_curve.rs:45` / `kpi_strip.rs:46`). The Live screen returns
`Element<'_, Message>` — the gallery precedent at
`crates/ui/src/gallery/routes.rs:533, 657` shows the
`.map(|_| Message::ServerTimeTick(...))` adapter pattern. Live uses
the same `.map(|_| <no-op msg>)` to bridge. **No new `Message` variant
needed for the bridge.**

### A8 — KPI strip: same Loading-placeholder treatment

`widgets::kpi_strip::view(metrics, mode)` is fed
`&PanelState::Loading` for the same reason — there is no
`Cockpit::today_metrics: PanelState<BacktestMetrics>` field. The
widget's `unavailable_strip(mode)` arm
(`kpi_strip.rs:48-49`) is the placeholder surface. Wires up the day
the paper-session metrics aggregator lands (Phase F sibling).

### A9 — Live LLM-spend tile: text-only placeholder

Per Q4b, the tile is a `Text::new(LIVE_LLM_SPEND_PLACEHOLDER)` cell
sitting alongside the KPI strip. No `Cockpit::llm_spend_today` field;
no LLM-budget-tracker subscription. Phase F wires the real spend
source. Cost: one `pub const LIVE_LLM_SPEND_PLACEHOLDER: &str = "—";`
in `strings.rs` (the `PLACEHOLDER_NONE` constant at line 719 may be
reused if the analyst-suggested name is too narrow — developer picks).

### A10 — `widgets::settings_tabs` shape

Three-tab chrome strip. Reuses `widgets::frame::active_chip`
(`frame.rs:238`) — same T1609 bottom-edge accent rule the
`screens::strategies::view` chip row uses. Signature:

```rust
pub fn view(active: SettingsTab, mode: ThemeMode) -> Element<'_, Message>
```

Renders three buttons in a `Row` with `space::M` spacing; each carries
`Message::SwitchSettingsTab(<tab>)`. **No new Lumen tokens.**
Snapshot tests pin each active state.

### A11 — `widgets::strategy_card` shape

Tier-2 surface card composing existing primitives. Signature:

```rust
pub fn view<'a>(
    row: &'a StrategyRow,
    config: Option<&'a StrategyConfigEntry>,  // universe / params lookup
    last_anchor: Option<(&'a str, &'a str)>,  // (scenario_name, sha256_prefix)
    last_run_ts: Option<Timestamp>,
    mode: ThemeMode,
) -> Element<'a, Message>
```

Composition (top → bottom inside `frame::panel`):
- Header row: ID + display name + status pill (`STRATEGY_REGISTRY_STATUS_SHIPPED`).
- Universe line: `STRATEGY_REGISTRY_UNIVERSE_PREFIX` + truncated symbols.
- Anchor line: `STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX` + scenario + sha7.
- Run line: `STRATEGY_REGISTRY_LAST_RUN_PREFIX` + relative timestamp
  (`Cockpit::strategies_recent_events` newest `Run` event for this
  `strategy_id`).
- Footer button: `STRATEGY_REGISTRY_OPEN_IN_LAB_LABEL` →
  `Message::SelectStrategy(row.id.clone())` (the bin layer chains
  `Message::SwitchScreen(Screen::Lab)` via `Task::done` — A3 precedent).

**No new Lumen tokens.** Last-anchor lookup is a constant-time map
read keyed by `StrategyId` from `spec/anchors.toml` data already
loaded at boot (or `None` if no anchor recorded — render
`PLACEHOLDER_NONE`).

### A12 — Compat-shim discipline (R5)

Six `#[deprecated]` `Screen::*` variants stay for one cycle (Q1a).
Phase D's prune ticket starts from the deprecated-variant census
(currently ~30 references in 8 test files + ~47 elsewhere per analyst
M0 audit — total ~77 workspace-wide). Phase C's M-FINAL gate writes
the census number into the test report so Phase D has a hard target.

**No new `#[deprecated]` markers added at Phase C.** The
`SETTINGS_PLACEHOLDER` constant (`strings.rs:258`) is the one existing
deprecation that Phase C's body wiring obsoletes — it gains a
`#[deprecated(since = "0.3.0", note = "Settings now renders the rollup body")]`
attribute in Wave E. Phase D removes the constant entirely.

### Net-new file inventory (final, post-A1)

1. `crates/ui/src/screens/live.rs` (Wave B)
2. `crates/ui/src/screens/strategy_registry.rs` (Wave C)
3. `crates/ui/src/screens/settings.rs` (Wave D)
4. `crates/ui/src/widgets/strategy_card.rs` (Wave C)
5. `crates/ui/src/widgets/settings_tabs.rs` (Wave D)

**No** `crates/ui/src/widgets/sidebar_divider.rs` (inlined per A1).

### Existing files modified

- `crates/ui/src/widgets/sidebar_nav.rs` — Wave A: `view` learns
  `groups: &[&[Screen]]` parameter; inline divider rendering between
  groups. Existing `entries: &[Screen]` parameter retained for compat.
- `crates/ui/src/shell.rs` — Wave B/C/D: `screen_body` match arms
  rewritten (lines 82-95 in current source); `Screen::Settings` /
  `Risk` / `Debug` / `Control` route to `settings::view`;
  `Screen::Live` / `Home` route to `live::view`;
  `Screen::Strategies` routes to `strategy_registry::view`.
- `crates/ui/src/screens/mod.rs` — Wave B/C/D: three new `pub mod`
  declarations (`live`, `strategy_registry`, `settings`).
- `crates/ui/src/state.rs` — Wave E: `SettingsTab` enum at line 96
  (after `Screen`); `Cockpit::settings_active_tab` field at line 745
  (after `current_screen`); `Message::SwitchSettingsTab` variant at
  line ~1146 (next to `SwitchScreen`); `update` arm at line 1520
  extended with deep-link pre-selection per A4.
- `crates/ui/src/strings.rs` — Wave E: ~15 new constants per R7.2;
  `SETTINGS_PLACEHOLDER` gains `#[deprecated]` attribute.
- `crates/ui/src/theme.rs` — Wave A: `SIDEBAR_GROUPS_PHASE_C` const at
  line 729 (after `SIDEBAR_ENTRIES_PHASE_A`).

### Sequencing constraints

- **Wave E lands first** (state + strings table changes). Waves A-D
  reference those symbols.
- **Wave A** (sidebar) is independent of B/C/D — can land in parallel
  with E.
- **Wave B/C/D** depend on Wave E only (specifically the
  `SettingsTab` enum + `Cockpit::settings_active_tab` field +
  `Message::SwitchSettingsTab` variant + new strings).
- **Wave F** (test-migration audit) is the last gate — runs after all
  code waves to capture the post-flip deprecated-variant census.

### Compat / deprecation gates

- `cargo clippy --workspace -- -D warnings` exit 0 — Phase C **must
  not** introduce new `Screen::Home/Charts/Audit/Risk/Debug/Control`
  references in net-new code. The deprecation warning is the canary.
  Test files keep their existing refs (R5.3 explicit exception).
- `#[allow(deprecated)]` is acceptable in `update` (already present at
  line 76 of `shell.rs` and required for the deep-link pre-select
  match arm) — but **not** in net-new code.

### Snapshot baseline plan

| Snapshot                                          | Owner test file              | Source row |
|---------------------------------------------------|------------------------------|------------|
| `sidebar_nav__phase_c_three_groups`               | `widgets/sidebar_nav.rs#mod tests` | R1.5 |
| `sidebar__phase_a_workflow_group` (kept or `.snap.disabled`) | `widgets/sidebar_nav.rs#mod tests` | R1.6 |
| `live_snapshot__steady_state`                     | `tests/panel_snapshots.rs` (new mod block) | R2.6 |
| `strategy_registry_snapshot__empty`               | `tests/panel_snapshots.rs` (new mod block) | R3.8 |
| `strategy_registry_snapshot__three_strategies`    | `tests/panel_snapshots.rs` (new mod block) | R3.8 |
| `settings_snapshot__risk_tab_active`              | `tests/panel_snapshots.rs` (new mod block) | R4.6 |
| `settings_snapshot__control_tab_active`           | `tests/panel_snapshots.rs` (new mod block) | R4.6 |
| `settings_snapshot__debug_tab_active`             | `tests/panel_snapshots.rs` (new mod block) | R4.6 |

Tester decides whether `sidebar__phase_a_workflow_group` migrates to
`.snap.disabled` or stays green (R1.6). The Phase A flat-group test
should stay green because Wave A is **additive** — the sidebar still
accepts the flat `entries: &[Screen]` parameter and renders identically
when the `groups` parameter is omitted.

## Changelog

- 2026-05-20 (orchestrator): promoted from dev-note §6 Phase C to
  proposed feature. Predecessor verified at
  `ui-rethink-phase-b-lab-run v0.2.0`. Status `proposed`; awaiting
  analyst pass.
- 2026-05-20 (analyst): M0 pass — R1-R10 anchored; Q1-Q5 refined
  with analyst defaults; K1-K6 risk register and H1-H5 hypothesis
  register populated; tasks.md M0 + M-FINAL gates refined. Five Qs
  blocked on operator before architect spawn.
- 2026-05-20 (operator): "Autoapprove all" — Q1a / Q2a / Q3a / Q4b /
  Q5a locked. Architect unblocked.
- 2026-05-20 (architect): M-T1 decomposition — Design § A1-A12 added
  with file:line anchors; 18 `T-D-N` rows seeded across Waves A-F in
  tasks.md; net-new file count locked at 5 (sidebar divider inlined per
  A1); one new public Message variant only (`SwitchSettingsTab`); deep-
  link tab pre-selection colocated in the existing `SwitchScreen` arm
  per A4. Trace row → `accepted`. Status frontmatter → `accepted` /
  owner → architect.
- 2026-05-20 (developer): All 25 T-D-N rows implemented and ticked.
  5 net-new files: `screens/live.rs`, `screens/strategy_registry.rs`,
  `screens/settings.rs`, `widgets/strategy_card.rs`,
  `widgets/settings_tabs.rs`. 6 files modified: `state.rs` (SettingsTab
  enum + Cockpit field + Message variant + update arms + 5 unit tests),
  `strings.rs` (15 Phase C constants + deprecated canary),
  `theme.rs` (SIDEBAR_GROUPS_PHASE_C + test), `shell.rs` (screen_body
  rewrite + sidebar_nav groups param), `widgets/sidebar_nav.rs` (group
  rendering + dividers + snapshot test), `gallery/routes.rs` (4 new
  cells + EXPECTED_WIDGETS updated). Gate results: `cargo fmt --check`
  exit 0; `cargo clippy --workspace -- -D warnings` exit 0;
  `cargo test --workspace` 0 failed; `scripts/verify_anchors.sh`
  ANCHORS PASS 22/22; cockpit-smoke 0 panic lines in 8 s window.
  Deviation from spec: `switch_screen_is_pure` test updated to account
  for the intentional `settings_active_tab` side-effect on deprecated
  Screen aliases (D § A4), restoring `settings_active_tab` alongside
  `current_screen` before the byte-equality assertion. Trace row
  state → `shipped`.
