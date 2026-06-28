---
slug: cockpit-reports-viewer
status: arch-done
owner: architect
updated: 2026-06-17
---

# Tasks — cockpit-reports-viewer v0.1.0

Architect-sequenced (2026-06-17) for the **ui-designer** to execute directly.
Shapes locked in [`feature.md` § Design](feature.md). M-sized, read-only UI
feature inside `crates/ui`: **no new crate edge, no new widget, no new theme
token** (AC7). All decisions resolved (D1 bespoke state, D2 lift loader to
`crate::reports`, D3 ship body, D4 Library group after Models, D5 navigable +
dedicated smoke, D6 no demo).

Verify-before-build (per project memory): the brief's line numbers were
re-verified in § Design "Drift corrections" — trust those, and treat the
`cockpit.rs:185` fold-in as **already done** (NO-OP, do not re-apply).

Run `cargo fmt` + `cargo clippy -p ui -- -D warnings` (new code only — the
~140 pre-existing pedantic lints stay; use the per-module `#![allow(...)]`
pattern from `screens/baseline.rs:32`). Gate awareness: each new module is a
sibling of `baseline/`; mirror its shape and tests.

## Sequencing overview

D2 lift first (it is the precondition for the screen + R4/AC5), then state,
then wiring (enum + sidebar lock-step), then the screen body + strings, then
tests. Each task is small and independently checkable.

---

## M-DEV — implementation

### M-DEV-1 — Lift the viewer loader into `crate::reports::loader` (D2 / R4 / AC5)

- [x] Create `crates/ui/src/reports/mod.rs` (mirror `baseline/mod.rs`):
      `pub mod loader; pub mod state; pub mod body_render;` + `pub use
      state::{ReportsScreenState, ReportEntry};`.
- [x] Register the module in `lib.rs`: add `pub mod reports;` next to
      `pub mod baseline;` (`lib.rs:52`), with a doc-comment mirroring the
      baseline one.
- [x] Create `crates/ui/src/reports/loader.rs`. **Move** verbatim from
      `bin/viewer.rs` (making each `pub`): `load_report` (`:136`),
      `load_equity_companion` (`:172`), `parse_front_matter` (`:223`),
      `strip_front_matter` (`:244`). Copy the `workspace_root()` helper from
      `baseline/loader.rs:234` (`CARGO_MANIFEST_DIR/../..`).
- [x] Create `crates/ui/src/reports/body_render.rs`: **move** `mod
      body_render` (`bin/viewer.rs:263-300`) → `pub fn view<'a>(markdown:
      &'a str, mode: ThemeMode) -> iced::Element<'a, ViewerMessage>`.
- [x] Refactor `bin/viewer.rs`: replace the moved fns with
      `use ui::reports::loader::{load_report, parse_front_matter,
      strip_front_matter, load_equity_companion};` and
      `use ui::reports::body_render;`; delete the local copies. The bin's
      `App::view` and `main` call sites are unchanged in behavior.
- [x] Move the `parse_front_matter_extracts_scenario` test
      (`bin/viewer.rs:343`) into `reports/loader.rs` `#[cfg(test)]` (its
      assertion must survive — AC5). The bin keeps `cli_parser_*` +
      `cli_help_renders_without_lumen`.
- [x] `cargo build -p ui` + `cargo test -p ui --bin viewer` green; the
      `viewer_read_only.rs` build-time grep stays green (no write path added).

### M-DEV-2 — All-slug discovery scan (R1 / R7 / AC1 / AC3)

- [x] In `reports/loader.rs`, add `pub fn discover_reports() -> Vec<ReportEntry>`:
      resolve `workspace_root().join("spec")`; for each slug subdir, read
      `<slug>/reports/` and filter `backtest-*.md` using the established
      filter (`starts_with("backtest-") && ends_with(".md")`, per
      `lab/equity_loader.rs:261`); build `ReportEntry { slug, file_stem, path }`.
- [x] Sort the result deterministically by `(slug, file_stem)` (stable list
      order + reproducible snapshots).
- [x] K2 never-panic: any unreadable dir → skip with a `tracing::debug!`
      breadcrumb, never panic (mirror `registry_read.rs:92` +
      `baseline/loader.rs:77`). Absent `spec/` → empty `Vec`.
- [x] Confirm the `robustness-sweep-*.md` + `test-*.md` families are
      excluded by the `backtest-` filter (not silently dropped — they never
      match).

### M-DEV-3 — `ReportsScreenState` + `ReportEntry` (D1)

- [x] Create `crates/ui/src/reports/state.rs` (mirror `baseline/state.rs`):
      ```rust
      pub struct ReportEntry { pub slug: SmolStr, pub file_stem: SmolStr, pub path: PathBuf }
      pub struct ReportsScreenState {
          pub discovered: PanelState<Vec<ReportEntry>>,
          pub selected: Option<usize>,
          pub loaded: PanelState<ReportLoadResult>,
      }
      ```
      Reuse `crate::viewer::ReportLoadResult` verbatim as the `loaded` payload.
- [x] `impl Default`: `discovered: Loading`, `selected: None`, `loaded: Loading`.
- [x] Add `pub fn load_selection(&mut self, idx: usize)`: look up the
      `PathBuf` by index in `discovered`; call `loader::load_report(path)`;
      store `PanelState::Ready(result)` (or `PanelState::Error` if the file
      vanished between discovery + selection — never panic, R3).
- [x] Add `pub fn load_into(model: &mut crate::state::Cockpit)`: run the
      boot discovery scan → `model.reports_screen_state.discovered =
      PanelState::Ready(loader::discover_reports())` (or `Empty` on zero
      results). Synchronous; never panics (mirror `baseline::state::load_into`).

### M-DEV-4 — `Screen::Reports` enum + `Cockpit` field + message + update arm

- [x] `state.rs:113` `Screen` enum: add `Reports` variant after `Baseline`
      (`:142`), doc-comment mirroring Baseline's (navigable via Library
      group, not default-routed). **Do not** make it `Default`.
- [x] `Cockpit` struct (`state.rs:~987`): add
      `pub reports_screen_state: crate::reports::ReportsScreenState`. Set
      `ReportsScreenState::default()` at BOTH construction sites where
      `baseline_screen_state` appears (`state.rs:~1207` and `~1321`).
- [x] `Message` enum (`state.rs:~1951`, near `BaselineSelectYear`): add
      `ReportsSelect(usize)` — typed index, NEVER a String/PathBuf payload
      (R1). Doc-comment noting the PathBuf lives in state.
- [x] `update` (`state.rs:~2824`, near the `BaselineSelectYear` arm): add
      ```rust
      Message::ReportsSelect(idx) => {
          model.reports_screen_state.selected = Some(idx);
          model.reports_screen_state.load_selection(idx);
      }
      ```
      No boot-hydrate message (boot discovery is the synchronous `load_into`).

### M-DEV-5 — Sidebar IA: Library group, lock-step (D4 / R6 / AC6)

- [x] `SIDEBAR_ENTRIES_PHASE_A` (`theme.rs:747`): insert `Screen::Reports`
      between `Screen::Models` and `Screen::Trail`.
- [x] `SIDEBAR_GROUPS_PHASE_C` (`theme.rs:773`): in the **library** sub-slice
      `&[Strategies, Memory, Models, Trail]`, insert `Reports` between
      `Models` and `Trail` → `&[Strategies, Memory, Models, Reports, Trail]`.
      Update the group doc-comment.
- [x] `sidebar_nav::label_for` (`sidebar_nav.rs:35`): add
      `Screen::Reports => REPORTS_SIDEBAR_LABEL` arm.
- [x] Confirm the two sidebar edits put `Reports` in the SAME relative
      position (Models < Reports < Trail) — the flatten-invariant test is the
      guard (M-TEST-3).

### M-DEV-6 — Strings: `REPORTS_*` block (R5 / AC6)

- [x] `strings.rs` (after the BASELINE block, `:1834`): add the five
      `pub const` defs from § Design (REPORTS_SIDEBAR_LABEL,
      REPORTS_PICKER_TITLE, REPORTS_EMPTY_LIST, REPORTS_SELECT_PROMPT,
      REPORTS_LOAD_ERROR), each with a doc-comment.
- [x] Add the five `("NAME", NAME)` rows to the strings registry table
      (`strings.rs:~1479`, where the BASELINE entries sit).

### M-DEV-7 — Reports screen body (`screens/reports.rs`) (R2 / R3 / AC2)

- [x] Create `crates/ui/src/screens/reports.rs` with
      `pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_>`;
      carry the per-module `#![allow(...)]` pattern from `screens/baseline.rs:32`.
- [x] Register `pub mod reports;` in `screens/mod.rs` (doc-comment mirroring
      the baseline entry).
- [x] Route in `shell::screen_body` (`shell.rs:138`): add
      `Screen::Reports => reports::view(model, mode)` and add `reports` to
      the `use crate::screens::{...}` list (`shell.rs:29`).
- [x] **Left picker:** scrollable `Column` of `Button` rows, one per
      `ReportEntry`, label `"<slug> · <file_stem>"`,
      `.on_press(Message::ReportsSelect(idx))`; active-row styling reusing
      the Baseline chip token discipline (active = `PANEL_RAISED`+`ACCENT`,
      inactive = `FG_3`/`BORDER_1` — `screens/baseline.rs:117`). Title =
      `REPORTS_PICKER_TITLE`. Empty `discovered` list → `REPORTS_EMPTY_LIST`.
- [x] **Right detail:** `selected == None` → `REPORTS_SELECT_PROMPT`.
      `loaded == Ready(r)` → the verbatim `bin/viewer.rs:101-124` stack:
      `kpi_strip::view(&r.metrics, mode)` → `equity_curve::view(&r.equity,
      mode)` → `drawdown_band::view(&r.equity, mode)` →
      `body_render::view(&r.body_markdown, mode)`, each bridged with
      `.map(|_| Message::ChartMarkerHoverEnded)` (`screens/baseline.rs:79`).
      `loaded == Error` → `REPORTS_LOAD_ERROR` copy.
- [x] Confirm: curve/band render their built-in Empty body for
      companion-less reports (expected — R3/AC2, not a failure); KPI-strip
      Error (malformed `## Summary`) renders the strip's muted body, no panic.

### M-DEV-8 — Boot-load wiring in both bins (R3 / AC4)

- [x] `cockpit.rs` (next to `ui::baseline::load_into(&mut cockpit)` at
      `:238`): add `ui::reports::state::load_into(&mut cockpit);`.
- [x] `cockpit_live.rs` (next to the baseline load_into at `:618`): add
      `ui::reports::state::load_into(&mut cockpit);`.
- [x] Confirm the default screen stays `Screen::Live` in both bins (D5 — do
      NOT change the default route; the `cockpit.rs:185` `Screen::Home`
      fold-in is ALREADY done — NO-OP).

---

## M-TEST — verification (tester closes the loop against AC1–AC7)

### M-TEST-1 — Loader unit tests (`reports/loader.rs` `#[cfg(test)]`) (AC1/AC3/AC5)

- [x] `discover_reports` finds `backtest-*.md`, excludes
      `robustness-sweep-*.md` + `test-*.md` (skip-if-`spec/`-absent, like
      `baseline/loader.rs` `committed_csvs_load_to_ready`).
- [x] `discover_reports` on an unreadable/absent root → empty `Vec`, no panic
      (K2); deterministic-sort assertion.
- [x] `load_report` on a valid-`## Summary` fixture → metrics `Ready`; on a
      `## Summary`-less fixture → metrics `Error(NoSummaryHeading)`, no panic;
      companion-less report → equity `Empty`.
- [x] `parse_front_matter_extracts_scenario` (moved from the bin) still passes.

### M-TEST-2 — State unit tests (`reports/state.rs` `#[cfg(test)]`) (AC3)

- [x] `Default` = (`discovered: Loading`, `selected: None`, `loaded: Loading`).
- [x] `load_selection(idx)` sets `selected = Some(idx)` + transitions
      `loaded`; a vanished-path index → `loaded: Error`, no panic.

### M-TEST-3 — Sidebar flatten-invariant (`theme.rs:1607`) (AC6)

- [x] `sidebar_groups_phase_c__flatten_matches_phase_a` passes (auto-green
      once both consts get `Reports` in the same relative position).
- [x] Add a one-line assertion that `Reports` sits between `Models` and
      `Trail` in the flattened list.

### M-TEST-4 — Panel snapshot, both themes (`tests/panel_snapshots.rs`) (AC2/AC6)

- [x] New `reports_screen` mod (mirror `baseline_screen` at `:3248`):
      textual-summary snapshot for Dark + Light of the Reports body — picker
      title + N rows (or empty copy) + the detail pane (a Ready selection's
      KPI lines + curve/band Empty-state + body line).
- [x] Use a deterministic fixture `ReportsScreenState` (hand-built
      `ReportEntry` list + a `Ready` `ReportLoadResult`) so the snapshot is
      checkout-independent.
- [x] Assert all copy resolves through `strings::REPORTS_*` (no hardcoded
      strings) and the active-row accent token differs Dark vs Light.

### M-TEST-5 — Headless smoke route (`tests/headless_emulator_smoke.rs`) (AC4)

- [x] `headless_emulator_paints_reports_route` (mirror
      `headless_emulator_paints_baseline_route` at `:87`): boot fixtures
      cockpit → `Screen::Reports` → `ui::reports::state::load_into` → drain
      to `Ready` → non-empty 1280×720 first-frame screenshot, no panic.
- [x] Empty-list degrade verified for a fixtures-only checkout (the scan
      returns `Empty`/empty `Vec`, still a clean paint).

### M-TEST-6 — Lumen + crate-edge review (AC6/AC7)

- [x] `tests/consistency.rs` / `tests/contrast.rs` /
      `tests/layout_invariants.rs` stay green.
- [x] Review: loader is pure-`ui` over `core` + `reports` + `std::fs` (both
      already deps); three render widgets + `body_render` reused verbatim;
      **zero new theme tokens, zero new widgets, no new crate edge**.
- [x] `cargo clippy -p ui -- -D warnings` introduces no new warnings (the
      pre-existing ~140 pedantic lints are untouched).

### M-TEST-7 — Regression floor (read-only feature) (AC7)

- [x] `scripts/verify_anchors.sh` green — Reports only READS committed
      reports; no anchored `spec/*/reports/` file is touched, no new backtest
      anchor.
- [x] Full `cargo test -p ui` green (the existing snapshot suite + new tests).
- [x] HANDOFF → tester emits the `## Verification` link + the test report.

---

## Done-definition

All M-DEV + M-TEST boxes checked; `cargo test -p ui` green; AC1–AC7 satisfied;
no new crate edge / widget / theme token; the bin's CLI tests stay green after
the D2 lift. Then tester `VERDICT → PASS` → presenter.
