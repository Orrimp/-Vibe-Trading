---
slug: ui-rethink-phase-c-sidebar-ia
status: draft
owner: analyst
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

## M-T1+ — Architect decomposition (awaits operator Q1-Q5)

To be populated by architect once operator unblocks Q1-Q5. Expected
shape per R1-R6:

- **Wave A — Sidebar grouping (R1).** `sidebar_nav::view` learns
  three-group composition + divider rendering. Snapshot baseline
  migration.
- **Wave B — `screens::live::view` (R2).** New module per §J6 sketch.
  Shell wiring for `Screen::Live` (active) + `Screen::Home` (compat).
  Snapshot baseline.
- **Wave C — `screens::strategy_registry::view` (R3).** New module
  + new `widgets::strategy_card`. Shell wiring for `Screen::Strategies`.
  Snapshot baselines for empty + populated states.
- **Wave D — `screens::settings::view` (R4) + `widgets::settings_tabs`.**
  Three-tab chrome wrapping `risk::view` / `control::view` /
  `debug::view` unchanged. New `Message::SwitchSettingsTab` variant
  per R9.1. Shell wiring for `Screen::Settings` + deprecated `Risk`/
  `Debug`/`Control` deep-link pre-selection (R5.2). Snapshot baselines.
- **Wave E — String table + state additions (R7, R8, R9).** New
  strings constants; `Cockpit::settings_active_tab` field with
  `SettingsTab` enum; (conditional) `OpenInLab` Message variant.
- **Wave F — Test migration audit.** Deprecated-variant usage census;
  any new deprecation warnings from R8/R9 wiring resolved.

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
