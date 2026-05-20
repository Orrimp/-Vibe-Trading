---
slug: ui-rethink-phase-c-sidebar-ia
status: accepted
owner: architect
updated: 2026-05-20
---

# Tasks — UI rethink Phase C (Sidebar IA flip + Live + Strategy registry + Settings rollup)

> Analyst pass landed 2026-05-20. M0 closed; M-T1+ awaits architect
> after operator answers Q1-Q5 from feature.md. M-FINAL gates pinned
> from feature.md R10 + analyst K-risks + H-hypotheses.

## M0 — Analyst synthesis  ✅ landed 2026-05-20

- [x] Read dev-note §6 Phase C (scope source-of-truth) + §3 (three-group
  sidebar IA) + §J6 (Live screen contract).
- [x] Survey existing sidebar code path: `crates/ui/src/widgets/sidebar_nav.rs`,
  `crates/ui/src/shell.rs::screen_body`, the `Screen` enum at
  `crates/ui/src/state.rs:53-95`, `SIDEBAR_ENTRIES_PHASE_A` constant at
  `crates/ui/src/theme.rs:719-728`.
- [x] Read the current `home::view` (`screens/home.rs` — 2×2 grid; to be
  replaced by §J6 layout in `screens/live.rs`) and `strategies::view`
  (`screens/strategies.rs` — panel-style; to be replaced by list-of-cards
  registry in `screens/strategy_registry.rs`).
- [x] Confirm `screens::risk::view`, `screens::control::view`,
  `screens::debug::view` are real bodies (not placeholders) currently
  routed to `placeholder::view(SETTINGS_PLACEHOLDER)` by the Phase A
  shell — Settings rollup wraps them unchanged.
- [x] Lock R1-R10 requirements in feature.md.
- [x] Refine Q1-Q5 (compat-shim window, Settings tab order, sidebar
  divider, LLM tile wiring, legacy strategies::view disposition) with
  analyst-recommended defaults.
- [x] Populate K1-K6 risk register and H1-H5 hypothesis register in
  feature.md.

**Blocker before M-T1:** operator answers Q1-Q5 from feature.md
"Open questions for operator" section.

## M-T1 — Architect decomposition  ✅ landed 2026-05-20

> Operator unblocked Q1-Q5 via "Autoapprove all" → analyst defaults.
> Architectural shapes locked in `feature.md` § Design A1-A12. Sidebar
> divider **inlined** per A1 (5 net-new files, not 6). One new public
> `Message` variant only per A3 (`SwitchSettingsTab`). Deep-link tab
> pre-selection colocated in the existing `SwitchScreen` arm per A4.

### Sequencing

```
Wave E (state + strings)  ──┐
                            ├──→  Wave B  ┐
                            ├──→  Wave C  ├──→  Wave F
                            └──→  Wave D  ┘
Wave A (sidebar)  ─────────────────────────→  Wave F
```

Wave E lands first because B/C/D import `SettingsTab` / new strings.
Wave A is independent of E and can land in parallel.
Wave F is the M-FINAL pre-flight (census + warnings sweep).

### Wave E — State + strings table  *(land first)*

- [x] **T-D-N01** — Add `SettingsTab` enum + `Default::Risk` to
  `crates/ui/src/state.rs` immediately after `Screen` enum
  (insert at line 96, after the `}` on line 95). Shape per Design § A5.
  Verify: `cargo build -p ui` exits 0; `assert_eq!(SettingsTab::default(), SettingsTab::Risk)`.
  **file:line** `crates/ui/src/state.rs:107-117` | **cmd** `cargo test -p ui --lib state::tests::settings_tab_default_is_risk` | **output** `test state::tests::settings_tab_default_is_risk ... ok`
- [x] **T-D-N02** — Add `Cockpit::settings_active_tab: SettingsTab`
  field at `crates/ui/src/state.rs:745` (immediately after
  `current_screen: Screen` on line 742). Update the
  `impl Default for Cockpit` block at line 919 with
  `settings_active_tab: SettingsTab::default(),`. Verify:
  `cargo build -p ui` exits 0.
  **file:line** `crates/ui/src/state.rs:763` (field) + `state.rs:~941` (Default) | **cmd** `cargo build -p ui` | **output** `Finished`
- [x] **T-D-N03** — Add `Message::SwitchSettingsTab(SettingsTab)`
  variant at `crates/ui/src/state.rs:~1146` (next to `SwitchScreen`
  at line 1145). Add the `update` arm next to the existing
  `SwitchScreen` arm at line 1520: `Message::SwitchSettingsTab(t) => { model.settings_active_tab = t; }`.
  Verify: `cargo test -p ui --lib state::tests::` 100 % PASS.
  **file:line** `crates/ui/src/state.rs` Message variant + update arm | **cmd** `cargo test -p ui --lib -- state::tests::switch_settings_tab_assigns_field` | **output** `test state::tests::switch_settings_tab_assigns_field ... ok`
- [x] **T-D-N04** — Extend `Message::SwitchScreen(s)` arm at
  `crates/ui/src/state.rs:1520-1522` with the R5.2 deep-link
  pre-selection match per Design § A4. Wrap the new match in
  `#[allow(deprecated)]` (the deprecated `Screen::Risk/Debug/Control`
  variants trigger the warning). Add a unit test in the existing
  `mod tests` block (around line 2443) named
  `switch_screen_to_risk_alias_preselects_risk_tab` proving that
  `SwitchScreen(Screen::Risk)` sets both `current_screen = Risk` AND
  `settings_active_tab = SettingsTab::Risk`. Add sibling tests for
  Debug and Control.
  Cmd: `cargo test -p ui --lib state::tests::switch_screen_to_risk_alias_preselects_risk_tab`
  (and Debug/Control siblings).
  **file:line** `crates/ui/src/state.rs:1552-1560` (deep-link match) | **cmd** `cargo test -p ui --lib -- state::tests::switch_screen_to_risk_alias_preselects_risk_tab` | **output** `test state::tests::switch_screen_to_risk_alias_preselects_risk_tab ... ok`
- [x] **T-D-N05** — Add new strings constants to
  `crates/ui/src/strings.rs` per feature.md R7.2. Group them in a new
  `// ── Phase C — Live / Strategy registry / Settings ──` block
  appended after the existing Phase 5 block (around line 720).
  Constants: `LIVE_HEADLINE`, `LIVE_SYSTEM_HEALTH_LABEL`,
  `LIVE_LLM_SPEND_LABEL`, `LIVE_LLM_SPEND_PLACEHOLDER`,
  `STRATEGY_REGISTRY_PANEL_TITLE`, `STRATEGY_REGISTRY_EMPTY`,
  `STRATEGY_REGISTRY_OPEN_IN_LAB_LABEL`,
  `STRATEGY_REGISTRY_STATUS_SHIPPED`,
  `STRATEGY_REGISTRY_STATUS_CANDIDATE` (unused at Phase C, ships for
  Phase D per Design § A6), `STRATEGY_REGISTRY_STATUS_ARCHIVED`
  (unused, Phase D), `STRATEGY_REGISTRY_LAST_ANCHOR_PREFIX`,
  `STRATEGY_REGISTRY_LAST_RUN_PREFIX`,
  `STRATEGY_REGISTRY_UNIVERSE_PREFIX`, `SETTINGS_TAB_RISK`,
  `SETTINGS_TAB_CONTROL`, `SETTINGS_TAB_DEBUG`.
  Verify: `cargo build -p ui` exits 0; no string literals in widget /
  screen bodies in Waves A-D (zero-string-literals contract R10.10).
  **file:line** `crates/ui/src/strings.rs` Phase C block | **cmd** `cargo build -p ui` | **output** `Finished`
- [x] **T-D-N06** — Attach `#[deprecated(since = "0.3.0", note = "Settings now renders the rollup body — Phase D removes")]`
  to `pub const SETTINGS_PLACEHOLDER` at `crates/ui/src/strings.rs:258`
  per Design § A12. The `shell.rs` placeholder route on line 89 stops
  referencing it once Wave D lands; this attribute is the warning
  canary for Phase D pruning. Verify: `cargo clippy --workspace -- -D warnings`
  exits 0 (note the route at `shell.rs:89` is rewritten in Wave D so
  no `#[allow(deprecated)]` is needed at Phase C — but if any other
  callers exist they MUST be migrated to the live route first).
  **file:line** `crates/ui/src/strings.rs` `#[deprecated]` on `SETTINGS_PLACEHOLDER` | **cmd** `cargo clippy --workspace -- -D warnings` | **output** exit 0

### Wave A — Sidebar grouping (R1)  *(parallel with E)*

- [x] **T-D-N07** — Add `SIDEBAR_GROUPS_PHASE_C: &[&[Screen]]` const
  to `crates/ui/src/theme.rs` immediately after `SIDEBAR_ENTRIES_PHASE_A`
  (insert at line 729, after the closing `];` on line 728). Shape per
  Design § A2. Add a `#[cfg(test)] mod tests` test
  `sidebar_groups_phase_c__flatten_matches_phase_a` proving
  `SIDEBAR_GROUPS_PHASE_C.iter().flat_map(|g| g.iter()).copied().collect::<Vec<_>>() == SIDEBAR_ENTRIES_PHASE_A.to_vec()`.
  Cmd: `cargo test -p ui --lib theme::layout::tests::sidebar_groups_phase_c__flatten_matches_phase_a`.
  **file:line** `crates/ui/src/theme.rs:741-749` | **cmd** `cargo test -p ui --lib -- theme::layout::tests::sidebar_groups_phase_c__flatten_matches_phase_a` | **output** `test theme::layout::tests::sidebar_groups_phase_c__flatten_matches_phase_a ... ok`
- [x] **T-D-N08** — Extend `widgets::sidebar_nav::view` signature at
  `crates/ui/src/widgets/sidebar_nav.rs:70` to accept a new
  `groups: &[&[Screen]]` parameter **before** `mode`. Update the
  inner `for screen in entries` loop (line 75) to iterate the slice-
  of-slices: outer loop over groups, inner loop over each group's
  screens, inserting a 1-px `BORDER_1` divider (inline per Design § A1)
  between groups. The divider uses the same `Container { background: BORDER_1 }`
  trick the right-edge hairline already uses on lines 116-122.
  Width: `Length::Fill` minus 2 × `space::M` horizontal padding;
  Height: `Length::Fixed(1.0)`.
  **Backwards compat:** keep the existing `entries: &[Screen]`
  parameter; if callers pass `groups = &[entries]` the rendering is
  identical to today (single-group, no divider). Phase D removes the
  `entries` parameter.
  Verify: `cargo build -p ui` exits 0.
  **file:line** `crates/ui/src/widgets/sidebar_nav.rs:73-116` | **cmd** `cargo build -p ui` | **output** `Finished`
- [x] **T-D-N09** — Update `shell::view` at
  `crates/ui/src/shell.rs:39` to pass `SIDEBAR_GROUPS_PHASE_C`:
  `sidebar_nav::view(model.current_screen, SIDEBAR_ENTRIES_PHASE_A, SIDEBAR_GROUPS_PHASE_C, mode)`.
  Add the new import on line 31.
  Verify: `cargo run -p ui --bin cockpit -- --fixtures --frames 1 --exit-after 8` — sidebar renders 3 groups with 2 dividers (visual check via cockpit-smoke).
  **file:line** `crates/ui/src/shell.rs` sidebar_nav call | **cmd** `cargo run -p ui --bin cockpit --features fixtures -- --frames 1 --exit-after 8` | **output** `Finished ... Running` (0 panics)
- [x] **T-D-N10** — Add the Wave A snapshot baseline. Append a new
  `#[test] fn sidebar_nav__phase_c_three_groups()` to
  `crates/ui/src/widgets/sidebar_nav.rs` `mod tests` (after the
  existing `sidebar__phase_a_workflow_group` test on line 220). The
  test calls a new `sidebar_grouped_summary(current, groups)` helper
  (sibling of `sidebar_summary` on line 154) that emits one
  `--- group N ---` line per group followed by the entries; the
  divider position is implicit in the group boundaries.
  `assert_snapshot!("sidebar_nav__phase_c_three_groups", summary);`.
  Cmd: `cargo test -p ui --lib widgets::sidebar_nav::tests::sidebar_nav__phase_c_three_groups -- --nocapture`
  then `cargo insta accept` (developer reviews the snapshot before
  accepting). Expected output structure per Design § A2 (3 groups,
  Lab/Live/Compare in group 0; Strategies/Memory/Models/Trail in 1;
  Settings in 2).
  **file:line** `crates/ui/src/widgets/sidebar_nav.rs` tests block | **cmd** `cargo test -p ui --lib -- widgets::sidebar_nav::tests::sidebar_nav__phase_c_three_groups` | **output** `test widgets::sidebar_nav::tests::sidebar_nav__phase_c_three_groups ... ok`
- [x] **T-D-N11** — Decide fate of the existing
  `sidebar__phase_a_workflow_group` snapshot
  (`crates/ui/src/widgets/snapshots/ui__widgets__sidebar_nav__tests__sidebar__phase_a_workflow_group.snap`
  if present, else inline). Per R1.6, the test stays green because
  the `entries` parameter still renders flat when called as before.
  **No** `.snap.disabled` migration required at architect-pass.
  Verify: `cargo test -p ui --lib widgets::sidebar_nav::tests::sidebar__phase_a_workflow_group` exits 0.
  **file:line** `crates/ui/src/widgets/sidebar_nav.rs` existing test | **cmd** `cargo test -p ui --lib -- widgets::sidebar_nav::tests::sidebar__phase_a_workflow_group` | **output** `test widgets::sidebar_nav::tests::sidebar__phase_a_workflow_group ... ok`

### Wave B — `screens::live::view` (R2)

- [x] **T-D-N12** — Create `crates/ui/src/screens/live.rs`. Layout per
  feature.md R2.2 / dev-note §J6 lines 528-542:
  1. **Top:** system-health row — `widgets::latency::view(model)` +
     compact market-health summary + server-time skew badge +
     kill-threshold inline gauge. Copy: pulls from existing
     `screens::debug::view` helpers; expose them by promoting the
     private `market_health_section` / `server_time_row` / `version_row`
     functions in `screens/debug.rs` to `pub(crate)` so Live can call
     them directly (no duplication, R10.10).
  2. **Mid:** equity curve via
     `widgets::equity_curve::view(&PanelState::<EquitySeries>::Loading, mode).map(|_| Message::ServerTimeTick(Timestamp::default()))`
     per Design § A7 message-type adapter. The placeholder renders the
     `VIEWER_NO_EQUITY_DATA` body until a future ticket wires real
     paper-session equity.
  3. **KPI strip** via
     `widgets::kpi_strip::view(&PanelState::<BacktestMetrics>::Loading, mode).map(|_| Message::ServerTimeTick(Timestamp::default()))`
     — same placeholder pattern.
  4. **LLM-spend tile:** single `Text::new(LIVE_LLM_SPEND_PLACEHOLDER)`
     cell adjacent to the KPI strip (or as a sibling row beneath if
     layout pressures require).
  5. **2-column row:** `widgets::positions::view(model)` LEFT,
     `widgets::agent_feed::view(model)` RIGHT.
  Add `pub mod live;` to `crates/ui/src/screens/mod.rs` between
  `lab` (line 17) and `risk` (line 18).
  Verify: `cargo build -p ui` exits 0; no clippy `-D warnings` fires;
  zero string literals (R10.10).
  **file:line** `crates/ui/src/screens/live.rs` (new file) | **cmd** `cargo build -p ui` | **output** `Finished`
- [x] **T-D-N13** — Rewrite the Live / Home shell match arm at
  `crates/ui/src/shell.rs:83` from `home::view(model, mode)` to
  `live::view(model, mode)`. Add `use crate::screens::live;` to the
  existing `use crate::screens::{audit, home, lab, strategies};` line
  (line 28). **Keep** the `home::view` source file intact per R2.4 —
  do not delete or modify `screens/home.rs`; Phase D prunes.
  Verify: `cargo build -p ui` exits 0; `cargo test -p ui --test home_strategies_row_cross_link`
  100 % PASS (compat shim — `Screen::Home` still routes through
  `live::view` because both `Live | Home` are in the same arm).
  **file:line** `crates/ui/src/shell.rs` `Screen::Live | Screen::Home` arm | **cmd** `cargo test --workspace` | **output** all `test result: ok`
- [x] **T-D-N14** — Add the Live snapshot baseline to
  `crates/ui/tests/panel_snapshots.rs`. New mod block `mod live_screen`
  with `#[test] fn live_snapshot__steady_state()`. Cockpit fixture: a
  `Cockpit::with_fixtures_for(Screen::Live)` factory (extend
  `crates/ui/src/test_support.rs` if needed; otherwise reuse the
  default boot + manual `current_screen = Screen::Live` pattern from
  `panel_snapshots.rs:676-773`). Summary helper emits:
  `screen: Live`, `system_health: <row>`, `equity_curve: VIEWER_NO_EQUITY_DATA placeholder`,
  `kpi_strip: unavailable_strip`, `llm_spend_tile: LIVE_LLM_SPEND_PLACEHOLDER`,
  `bottom_left: positions`, `bottom_right: agent_feed`.
  `assert_snapshot!("live_snapshot__steady_state", summary);`.
  Cmd: `cargo test -p ui --test panel_snapshots -- live_screen::live_snapshot__steady_state`
  then `cargo insta accept`.
  **file:line** `crates/ui/tests/panel_snapshots.rs` `live_screen` mod | **cmd** `cargo test -p ui --test panel_snapshots -- live_screen::live_snapshot__steady_state` | **output** `test live_screen::live_snapshot__steady_state ... ok`

### Wave C — `screens::strategy_registry::view` (R3) + `widgets::strategy_card`

- [x] **T-D-N15** — Create `crates/ui/src/widgets/strategy_card.rs`
  per Design § A11. Signature:
  ```rust
  pub fn view<'a>(
      row: &'a StrategyRow,
      config: Option<&'a StrategyConfigEntry>,
      last_anchor: Option<(&'a str, &'a str)>,
      last_run_ts: Option<Timestamp>,
      mode: ThemeMode,
  ) -> Element<'a, Message>
  ```
  Reuse `widgets::frame::panel(title, body, mode)` for the card
  chrome; reuse the `frame::active_chip` pattern for the status pill
  (same T1609 visual). The "Open in Lab" button carries
  `Message::SelectStrategy(row.id.clone())` (per Design § A3 —
  bin layer chains `SwitchScreen(Screen::Lab)` via `Task::done`).
  Add `pub mod strategy_card;` to `crates/ui/src/widgets/mod.rs`.
  Verify: `cargo build -p ui` exits 0; zero string literals; zero hex
  colours.
  **file:line** `crates/ui/src/widgets/strategy_card.rs` (new file) | **cmd** `cargo build -p ui` | **output** `Finished`
- [x] **T-D-N16** — Create `crates/ui/src/screens/strategy_registry.rs`.
  Layout: `Column` of `widgets::strategy_card::view(…)` calls, one per
  `StrategyRow` in `Cockpit::strategies` (`PanelState::Ready(rows)`).
  States:
  - `Loading` → `frame::loading_with_spinner(STRATEGIES_LOADING, mode)`.
  - `Empty` → `frame::muted_body(STRATEGY_REGISTRY_EMPTY)` per R3.6
    (Q3a-derived copy: "No strategies registered. Run a backtest in
    Lab to register one.").
  - `Error(e)` → `frame::error_body(STRATEGIES_ERROR_PREFIX, e)`.
  - `Ready(rows)` → vertical card stack.
  Panel chrome: wrap in `widgets::frame::panel(STRATEGY_REGISTRY_PANEL_TITLE, …, mode)`.
  Anchor lookup: read from `spec/anchors.toml` mirror if exposed via
  any existing `Cockpit::*` field (analyst audit found none — pass
  `last_anchor = None` for all rows at Phase C; the strategy_card
  renders `PLACEHOLDER_NONE`). Last-run timestamp: scan
  `Cockpit::strategies_recent_events` for the newest entry with
  matching `strategy_id` (existing field — `state.rs:698`).
  Add `pub mod strategy_registry;` to `crates/ui/src/screens/mod.rs`.
  Verify: `cargo build -p ui` exits 0.
  **file:line** `crates/ui/src/screens/strategy_registry.rs` (new file) | **cmd** `cargo build -p ui` | **output** `Finished`
- [x] **T-D-N17** — Rewrite the Strategies shell match arm at
  `crates/ui/src/shell.rs:93` from
  `Screen::Strategies => strategies::view(model, mode),` to
  `Screen::Strategies => strategy_registry::view(model, mode),`.
  Drop the `strategies` import from line 28 (no longer used by shell)
  but **keep** `screens/strategies.rs` as a source file per R3.5
  (Phase D prunes per Q5a). Add a top-of-file `#[allow(dead_code)]`
  attribute to `screens/strategies.rs` so the dead-code warning
  doesn't block clippy.
  Verify: `cargo build -p ui` exits 0; `cargo clippy --workspace -- -D warnings` exits 0.
  **file:line** `crates/ui/src/shell.rs` `Screen::Strategies` arm | **cmd** `cargo clippy --workspace -- -D warnings` | **output** exit 0
- [x] **T-D-N18** — Add the two registry snapshot baselines to
  `crates/ui/tests/panel_snapshots.rs`. New mod block
  `mod strategy_registry_screen`:
  - `#[test] fn strategy_registry_snapshot__empty()` — cockpit with
    `strategies: PanelState::Empty`; expect the muted-body
    `STRATEGY_REGISTRY_EMPTY` copy.
  - `#[test] fn strategy_registry_snapshot__three_strategies()` —
    cockpit with three `StrategyRow` entries populated via
    `test_support::sample_strategy_rows()` (extend if absent);
    snapshot pins three card summaries in scan order.
  Both `assert_snapshot!(...)` then `cargo insta accept`.
  Cmd: `cargo test -p ui --test panel_snapshots -- strategy_registry_screen`.
  **file:line** `crates/ui/tests/panel_snapshots.rs` `strategy_registry_screen` mod | **cmd** `cargo test -p ui --test panel_snapshots -- strategy_registry_screen` | **output** `2 passed`

### Wave D — `screens::settings::view` (R4) + `widgets::settings_tabs`

- [x] **T-D-N19** — Create `crates/ui/src/widgets/settings_tabs.rs`
  per Design § A10. Signature
  `pub fn view(active: SettingsTab, mode: ThemeMode) -> Element<'_, Message>`.
  Renders three `Button` cells in a `Row` with `space::M` spacing;
  each button is wrapped in `widgets::frame::active_chip(content, is_active, mode)`
  (existing helper at `frame.rs:238`) for the T1609 bottom-edge accent
  rule on the active tab. Each carries
  `Message::SwitchSettingsTab(SettingsTab::{Risk,Control,Debug})` on
  press. Copy via `SETTINGS_TAB_RISK` / `SETTINGS_TAB_CONTROL` /
  `SETTINGS_TAB_DEBUG`. Tab order: Risk · Control · Debug per Q2a.
  Add `pub mod settings_tabs;` to `crates/ui/src/widgets/mod.rs`.
  Verify: `cargo build -p ui` exits 0.
  **file:line** `crates/ui/src/widgets/settings_tabs.rs` (new file) | **cmd** `cargo build -p ui` | **output** `Finished`
- [x] **T-D-N20** — Create `crates/ui/src/screens/settings.rs`.
  Composition: `Column[ widgets::settings_tabs::view(model.settings_active_tab, mode), <tab body> ]`
  where `<tab body>` is a match on `model.settings_active_tab`:
  - `SettingsTab::Risk` → `screens::risk::view(model, mode)`
  - `SettingsTab::Control` → `screens::control::view(model, mode)`
  - `SettingsTab::Debug` → `screens::debug::view(model, mode)`
  No tab body modification — the existing screen bodies render
  unchanged per R4.4 (H5 guard).
  Add `pub mod settings;` to `crates/ui/src/screens/mod.rs`.
  Verify: `cargo build -p ui` exits 0; `cargo test -p ui --test panel_snapshots`
  Risk/Debug/Control body snapshots stay byte-identical (H5).
  **file:line** `crates/ui/src/screens/settings.rs` (new file) | **cmd** `cargo build -p ui` | **output** `Finished`
- [x] **T-D-N21** — Rewrite the Settings shell match arm at
  `crates/ui/src/shell.rs:88-90` from the placeholder body
  `placeholder::view(strings::SETTINGS_PLACEHOLDER, mode)` to
  `settings::view(model, mode)`. The arm still covers
  `Screen::Settings | Screen::Risk | Screen::Debug | Screen::Control`
  — the deep-link pre-selection (T-D-N04) ensures the right tab is
  active for the deprecated aliases. Drop the
  `use crate::strings;` import if no other shell-level reference
  remains (verify with `cargo build`).
  Verify: `cargo build -p ui` exits 0; `cargo run -p ui --bin cockpit -- --fixtures` and manually navigate `Screen::Risk` → Settings opens on Risk tab.
  **file:line** `crates/ui/src/shell.rs` `Screen::Settings | Screen::Risk | Screen::Debug | Screen::Control` arm | **cmd** `cargo build -p ui` | **output** `Finished`
- [x] **T-D-N22** — Add the three Settings snapshot baselines to
  `crates/ui/tests/panel_snapshots.rs`. New mod block
  `mod settings_screen`:
  - `#[test] fn settings_snapshot__risk_tab_active()` —
    `cockpit.settings_active_tab = SettingsTab::Risk`; render
    `settings::view`; snapshot pins tab-row + Risk body.
  - `#[test] fn settings_snapshot__control_tab_active()` — sibling.
  - `#[test] fn settings_snapshot__debug_tab_active()` — sibling.
  All `assert_snapshot!(...)` then `cargo insta accept`.
  Cmd: `cargo test -p ui --test panel_snapshots -- settings_screen`.
  **file:line** `crates/ui/tests/panel_snapshots.rs` `settings_screen` mod | **cmd** `cargo test -p ui --test panel_snapshots -- settings_screen` | **output** `3 passed`

### Wave F — Test migration audit + warnings sweep

- [x] **T-D-N23** — Run the deprecated-variant census across the
  workspace post-flip. Cmd:
  `git grep -nE 'Screen::(Home|Charts|Audit|Risk|Debug|Control)' -- crates/ui | sort | uniq -c -w 0`
  Expected baseline pre-Phase-C (per analyst M0): 8 test files, ~77
  total references workspace-wide. Phase C must NOT increase this
  count — any new `Screen::Home/Charts/Audit/Risk/Debug/Control`
  reference in **net-new** code (`crates/ui/src/screens/live.rs`,
  `strategy_registry.rs`, `settings.rs`, `widgets/strategy_card.rs`,
  `widgets/settings_tabs.rs`) is a regression. Capture the post-flip
  number into the M-FINAL test report (K6 mitigation; Phase D prune
  budget).
  **file:line** N/A (grep census) | **cmd** `git grep -nE 'Screen::(Home|Charts|Audit|Risk|Debug|Control)' -- '*.rs'` | **output** 0 new-file hits; net-new files (live.rs, strategy_registry.rs, settings.rs, strategy_card.rs, settings_tabs.rs) have zero deprecated-Screen references
- [x] **T-D-N24** — Run `cargo clippy --workspace -- -D warnings` and
  fix any new `deprecated` warnings introduced by Wave E (`update`
  arm deep-link match) by **scoping** `#[allow(deprecated)]` to the
  smallest possible block (already present at `state.rs:1520` arm
  level — extend coverage rather than widen scope). No `#[allow(deprecated)]`
  in net-new files per feature.md M-FINAL Build+Lint gate.
  **file:line** N/A (lint sweep) | **cmd** `cargo clippy --workspace -- -D warnings` | **output** exit 0 (0 errors, 0 warnings)
- [x] **T-D-N25** — Verify the cockpit-smoke watch recipe:
  ```
  watch -n 2 'pgrep -af cockpit | head -3; \
              tail -3 /tmp/cockpit-smoke.log 2>/dev/null || echo no log yet'
  ```
  Run `cargo run -p ui --bin cockpit -- --fixtures --frames 1 --exit-after 8 2>&1 | tee /tmp/cockpit-smoke.log`;
  verify 0 panic lines in the 8 s window (R10.3). Settings tab
  switches subjectively < 10 ms (H3 — qualitative; report in
  M-FINAL).
  **file:line** N/A (smoke run) | **cmd** `cargo run -p ui --bin cockpit --features fixtures -- --frames 1 --exit-after 8` | **output** `Finished ... Running` (0 panic lines in 8 s window)

## M-FINAL — Tester sweep

> Pinned from feature.md R10 + Acceptance criteria.

### Build + lint

- [ ] `cargo fmt --check` exit 0.
- [ ] `cargo clippy --workspace -- -D warnings` exit 0 (R10.5).
- [ ] No new `#[allow(deprecated)]` in net-new Phase C code (Wave E
  state additions should not require it).

### Tests

- [ ] `cargo test --workspace --lib` 100 % PASS (R10.6).
- [ ] `cargo test -p ui --test render_snapshots` — Phase A baselines
  + new Live / Strategy registry / Settings additions land green.
- [ ] `cargo test -p ui --test visual_snapshots` green.
- [ ] `cargo test -p ui --test panel_snapshots` — Risk / Audit / Debug
  panel-body baselines stay byte-identical (H5 falsifier).
- [ ] `cargo test -p ui --test layout_invariants` green —
  `Screen::Charts` / `Home` / `Audit` arms still pass via compat
  shim (R5.3).
- [ ] `cargo test -p ui --test home_strategies_row_cross_link` green —
  `Screen::Home` → cross-link to `Screen::Strategies` (now Strategy
  registry) still works (R5.3).
- [ ] `cargo test -p ui --test audit_filter_chip_emits_filter_changed
  --test audit_row_opens_modal --test chart_markers_from_audit_query`
  green — deprecated `Screen::Audit` / `Screen::Charts` routes stay
  alive (R5.3).
- [ ] New snapshot baselines committed:
  - `sidebar_nav__phase_c_three_groups` (R1.5)
  - `live_snapshot__steady_state` (R2.6)
  - `strategy_registry_snapshot__empty` (R3.8)
  - `strategy_registry_snapshot__three_strategies` (R3.8)
  - `settings_snapshot__risk_tab_active` (R4.6)
  - `settings_snapshot__control_tab_active` (R4.6)
  - `settings_snapshot__debug_tab_active` (R4.6)

### Anchors (non-negotiable)

- [ ] `scripts/verify_anchors.sh` → ANCHORS PASS (22 / 22) — H1
  falsifier; R10.1.

### Runtime + performance

- [ ] `cockpit-smoke` PASS — 0 panic lines in 8 s window (R10.3).
- [ ] Cockpit-performance v1.0.0 idle-CPU floor ≤ 13.1 % verified
  post-flip (three-run median) — H2 falsifier; R10.4.
- [ ] Settings tab switch < 10 ms wall-clock (H3 falsifier) — measured
  via cockpit-smoke instrumentation if available, else qualitative
  in test report.

### Spec hygiene

- [ ] `scripts/spec_lint.py` → Phase C contribution = 0 (R10.8;
  baseline 87 carry-forward).
- [ ] Deprecated-variant usage census — grep
  `Screen::(Home|Charts|Audit|Risk|Debug|Control)` count + per-file
  breakdown appended to the test report (K6 mitigation; Phase D
  prune budget). Current baseline: 8 test files; ~77 references
  workspace-wide per analyst's M0 audit.
- [ ] Author
  `spec/ui-rethink-phase-c-sidebar-ia/reports/test-final-<YYYY-MM-DD>.md`
  per `.claude/skills/rust-test/templates/test-report.md`.

### Presenter

- [ ] Presenter assembles
  `spec/ui-rethink-phase-c-sidebar-ia/presentations/ui-rethink-phase-c-sidebar-ia-<YYYY-MM-DD>.md`
  per `.claude/agents/presenter.md`. K1 (muscle memory) and K2
  (Settings rollup discoverability) must surface prominently in
  the operator-approval section.
- [ ] Screenshots of net-new sidebar grouping, Live screen, Strategy
  registry, and Settings tabs included.

## Notes

- **Predecessor.** `ui-rethink-phase-b-lab-run v0.2.0` shipped
  2026-05-19 (commit `2112d69` and forward). Lab + chart + Train
  panel + Run button stay byte-identical (R10.2).
- **Estimated cost** (per dev-note §6 Phase C): ~2-3 weeks. Anchor
  risk: zero by construction.
- **Compat shim plan** documented in feature.md R5 + Q1. Default:
  one cycle (Phase D prunes).
- **Net-new file inventory:**
  - `crates/ui/src/screens/live.rs` (Wave B)
  - `crates/ui/src/screens/strategy_registry.rs` (Wave C)
  - `crates/ui/src/screens/settings.rs` (Wave D)
  - `crates/ui/src/widgets/strategy_card.rs` (Wave C)
  - `crates/ui/src/widgets/settings_tabs.rs` (Wave D)
  - Optional: `crates/ui/src/widgets/sidebar_divider.rs` (Wave A —
    architect may inline in `sidebar_nav::view`).
- **Existing files modified:**
  - `crates/ui/src/widgets/sidebar_nav.rs` (Wave A: group rendering)
  - `crates/ui/src/shell.rs` (Wave B/C/D/E: screen_body match arms)
  - `crates/ui/src/screens/mod.rs` (Wave B/C/D: module exports)
  - `crates/ui/src/state.rs` (Wave E: Cockpit field + Message
    variants if R9.1/R9.2 land)
  - `crates/ui/src/strings.rs` (Wave E: new constants per R7.2)
  - `crates/ui/src/theme.rs` (Wave A: optional group-boundary
    metadata next to `SIDEBAR_ENTRIES_PHASE_A`)
