---
slug: cockpit-reports-viewer
status: dev-done
owner: ui-designer
updated: 2026-06-17
version: 0.1.0
---

# Cockpit Reports viewer — browse + render committed backtest reports in-cockpit

> **Status: arch-done (greenlit + designed 2026-06-17).** The operator
> GREENLIT the build 2026-06-17. The prior "deferred" rationale (equity
> curves render empty for the current corpus) is the **accepted, honest
> behavior** per § Data contract — not a blocker. v0.1.0 ships the picker +
> KPI strip + markdown body over the 112 `backtest-*.md` corpus; the equity
> curve + drawdown band render their Empty-by-data state and light up
> automatically the day a report ships a companion CSV (D6 default = no
> demo). The architect has resolved D1–D6 and written the `## Design` +
> `tasks.md`; HANDOFF → ui-designer.

## Why

The offline `viewer` binary (`crates/ui/src/bin/viewer.rs`, Phase 4) already
renders any committed `spec/<feature>/reports/backtest-*.md` as a KPI strip +
equity curve + drawdown band + markdown body. But it is a **separate
CLI-launched process**: the operator must quit the cockpit, hand-type a report
path, and launch `viewer <path>`. None of that capability is reachable from
`cockpit_live`. The ui-designer's build-out audit
(`spec/dev-notes/cockpit-buildout-audit-2026-06-08.md` §2 rough-edge #4, §3
candidate **#2**) flags this as "the 'surface backtest results in the cockpit'
muscle is already written; it just lives in the wrong binary."

This feature is the audit's **ranked #2 candidate**, greenlit as the
general-case follow-on to the **#1 Baseline panel** (shipped 2026-06-08,
`spec/cockpit-baseline-panel/feature.md`). Where the Baseline panel surfaces
**one** hardcoded result, Reports surfaces **any** committed backtest report:
a navigable `Screen::Reports` with a left report picker + a right detail pane
that renders the selected report via the promoted viewer render logic.

**Scope:** M-sized UI feature. The render path already exists and is
snapshot-tested in the `viewer` bin; this is mostly (a) lifting the bin's
load/parse logic into a pure-`ui` loader module, (b) a file-discovery scan,
and (c) a list-detail screen mirroring the Baseline / Memory / Models
precedents. **No new widget, no new crate edge.**

## Requirements

A new **`Screen::Reports`** inside the cockpit shell — a list-detail screen
that browses committed backtest reports and renders the selected one, reusing
the `viewer`'s render logic and the existing `kpi_strip` / `equity_curve` /
`drawdown_band` widgets verbatim.

- **R1 — Report picker (list).** A left-hand list of discovered committed
  backtest reports, grouped/labelled by **feature slug** + the report's
  **filename/date** (the `backtest-<YYYYMMDD-HHMMSS>-<scenario>.md` stem
  carries both). Selecting an entry loads + renders it in the detail pane.
  The list reuses the established list-detail pattern (Memory / Models /
  `strategy_registry`). A typed `Message::ReportsSelect(<index-or-id>)` arm
  — **no raw `String`/`PathBuf` payload** as the message key (mirror the
  Baseline `BaselineSelectYear(BaselineYear)` typed-message discipline; the
  selection is an index or a small typed id into the discovered list, with
  the `PathBuf` held in state).

- **R2 — Render the selected report (detail).** On selection, the detail pane
  renders, top-to-bottom, reusing the `viewer`'s `App::view` composition
  (`bin/viewer.rs:101-124`) **verbatim** in widget terms:
  - **KPI strip** (`kpi_strip::view`) from the report's `## Summary` markdown
    table, parsed by `reports::parse::parse_from_report` (already corpus-wide
    robust — graceful `_present=false` on absent fields).
  - **Equity curve** (`equity_curve::view`) + **drawdown band**
    (`drawdown_band::view`) from the companion equity CSV **when present** —
    see § Data contract for the honest caveat that **no backtest report in
    the current corpus ships this companion**, so these render their built-in
    **Empty** state for every report today (R3).
  - **Markdown body** — the report body below the front-matter. v0.1.0 MAY
    reuse the `viewer` bin's minimal `body_render` heading pre-pass (the
    `# / ## / ###` → `text::H2/H3` mapping at `bin/viewer.rs:262-300`), or
    defer the body to a follow-on and ship KPI+curve+band only — **architect
    decides** (see § Open decisions D3); the picker + KPI strip are the
    load-bearing value.

- **R3 — Four panel states (no panic).** Each panel honors `PanelState<T>`,
  exactly as the Baseline panel (R4 there):
  - **Loading** — transiently while a selection's load runs (synchronous +
    cheap; a spinner only if a large body parse exceeds the ~100 ms latency
    rule, per audit §3 #2).
  - **Ready** — populated KPI strip (+ curve/band when a companion exists).
  - **Empty** — two distinct empty surfaces, both with helpful copy (no blank
    screen): (i) **no reports discovered** ("No backtest reports found in
    spec/ yet") for the list; (ii) **no equity companion** for the selected
    report → the curve/band render their built-in empty body (the common case
    today — R3 of § Data contract). Neither is an error.
  - **Error** — a selected report that is **missing on disk** (deleted between
    discovery and selection) or whose `## Summary` table is **malformed**
    (`parse_from_report` → `Err(NoSummaryHeading)`) → `PanelState::Error(<copy>)`
    on the KPI strip; **never panics**. The list-discovery scan likewise never
    panics on an unreadable dir (mirror `registry_read.rs` K2 — return an
    empty list + a `tracing` breadcrumb).

- **R4 — Reuse the viewer's loader (no parse duplication).** The screen MUST
  NOT re-implement the markdown/front-matter/CSV parse. The `viewer` bin's
  `load_report` + `parse_front_matter` + `strip_front_matter` +
  `load_equity_companion` (+ optionally `body_render`) currently live **in
  the bin** (`bin/viewer.rs`), not the lib (`crate::viewer` holds only the
  model/message/update). This feature lifts that load logic into a pure-`ui`
  module so **both** the `viewer` bin and the Reports screen call one
  implementation (§ Open decisions D2). The bin is then refactored to call the
  lifted fn (small, mechanical; its existing CLI/exit-code tests stay green).

- **R5 — Lumen consistency.** Theme tokens only (the reused widgets are
  already theme-correct; **target zero new tokens**). All copy in `strings.rs`
  (a `REPORTS_*` block — no hardcoded strings). Renders correctly in both
  `--theme dark` and `--theme light` for free.

- **R6 — Sidebar IA.** Add `Screen::Reports` to the sidebar. **Architect
  picks the group** (§ Open decisions D4): the natural homes are the **Work**
  group (next to `Baseline`, since both surface backtest results) or the
  **Library** group (next to `Models`/`Memory`, since it is a browse-a-corpus
  registry). Add to **both** `SIDEBAR_ENTRIES_PHASE_A` and
  `SIDEBAR_GROUPS_PHASE_C` in the same position (the flatten-invariant test is
  the lock-step guard, AC6) + a `sidebar_nav::label_for` arm. Label =
  `REPORTS_SIDEBAR_LABEL` = "Reports".

- **R7 — Smoke safety.** Discovery + load **never panic** (parse-miss → Error
  state, unreadable dir → empty list, mirroring `registry_read.rs` K2 +
  `baseline/loader.rs`). The fixtures `cockpit` smoke must paint the Reports
  route (first-frame render, no panic) within its window — covered by a
  dedicated headless render test mirroring `headless_emulator_paints_baseline_route`
  (D2 navigable-not-default-routed precedent, see § Open decisions D5).

### Out of scope (explicit)

- **Rendering the robustness θ-surface report family** (`robustness-sweep-*.md`,
  34 files) and the `test-*.md` family (37 files). These are **not** single
  equity-curve+KPI-strip reports — θ-surfaces are parameter grids with no
  single equity path. Scoping the picker to `backtest-*.md` (which IS the
  viewer's contract) keeps the render honest. See § Data contract.
- **A file-picker dialog / arbitrary-path open.** The picker browses the
  **discovered committed corpus** only — it does not open arbitrary
  filesystem paths (preserves the `viewer`'s read-only-on-`spec` posture; no
  write path, no path-traversal surface).
- **Refactoring the Baseline panel into Reports.** The Baseline panel stays a
  separate special-case screen for now (§ Relationship to Baseline). A future
  fold-in is noted, not done.
- **New widgets / new theme tokens.** If either is needed, that is a smell to
  challenge in review (R5 / AC7).
- **Deploy/export/run CTAs** in the detail pane — Reports is read-only, like
  the `viewer` (the viewer's "zero-button surface" Master Constraint carries
  over in spirit; the only interactions are pick-a-report + theme, both
  already in the cockpit shell).

## Data contract

A new pure-`ui` loader module (the lifted `viewer` load logic) — **no new
cross-crate edge**: `ui` already depends on `core` + `reports`. Precedents:
`crates/ui/src/baseline/loader.rs` (the just-shipped #1 loader) and
`crates/ui/src/models/registry_read.rs` (the corpus-scan K2 pattern).

### Browsable report set — scope (honest)

The corpus survey (2026-06-08) found:

| Report family | Count | In picker? | Why |
|---------------|-------|-----------|-----|
| `backtest-*.md` | **112** | **Yes** | The `viewer`'s contract: has a `## Summary` KPI table `parse_from_report` reads. |
| `robustness-sweep-*.md` | 34 | **No** | θ-parameter surfaces — no single equity path, no `## Summary` KPI strip the widgets render. |
| `test-*.md` (tester reports) | 37 | **No** | Tester verdict docs, not backtest results — different layout, no equity companion. |

→ **The picker scopes to `backtest-*.md` only.** That is the family the
`viewer` was built for and the only one whose `## Summary` table
`parse_from_report` parses. The other families are excluded by the discovery
glob, not silently dropped — see the table above; the architect may surface
the exclusion in copy ("Backtest reports" picker title) so it is not a
mystery.

### LOAD-BEARING CAVEAT — equity curve + drawdown band render Empty for the whole corpus today

The corpus survey found **zero** `backtest-*.md` reports with the
`<dir>/artifacts/<run_id>/equity-*.csv` companion that
`viewer::load_equity_companion` (`bin/viewer.rs:172`) scans for. Confirmed
exhaustively:

- `find spec -path '*/reports/artifacts/*' -name 'equity-*.csv'` → **0 files**.
- **No** `spec/*/reports/` directory even has an `artifacts/` subdir.
- The only `equity-*.csv` files anywhere under `spec/` live at
  `spec/real-mtm-unrealized-pnl/presentations/artifacts/.../equity-{7d,since-inception}.csv`
  — a **presentations** path, not a `reports/artifacts/<run_id>/` companion,
  and not discovered by the viewer's scan.

**Consequence:** for every report in the browsable set **as the corpus stands
today**, the detail pane renders:

- KPI strip — **Ready** (parsed from the `## Summary` markdown table). ✅
- Markdown body — **Ready** (if D3 ships the body). ✅
- Equity curve + drawdown band — **Empty** (no companion CSV). ➖

This is **not a defect** — it is the honest state of the corpus, and the
`viewer` bin exhibits exactly the same behavior on these reports today (it
falls to `PanelState::Empty` per `bin/viewer.rs:182`). The feature's value is
real and intact: **the operator can browse + read any backtest report's KPIs
and body in-cockpit instead of CLI-launching the viewer.** The equity-curve
slot is a *latent* capability that lights up automatically the day a report
ships with a companion CSV (e.g. a future v5 latency/slippage run that emits
the companion). The brief flags this so the architect/tester do **not** spend
effort chasing a "missing curve" that is simply absent data.

> **Architect note (tee'd to D6):** if the operator wants a *non-empty* curve
> demo in v0.1.0, the two real `equity-*.csv` files at the
> `real-mtm-unrealized-pnl` presentations path could seed a single
> proof-of-render example — but they are a different schema
> (`ts, equity_total_usdt, …`, the viewer's native schema, NOT the BH
> 3-column schema) and a different path family. Surfacing them is a scope
> decision, not a requirement. Default v0.1.0 ships the picker + KPI strip +
> body over the `backtest-*.md` corpus with curves Empty-by-data.

### Loader contract (lifted from `bin/viewer.rs`)

The lifted module reuses the viewer's **two-schema-aware** companion loader:

- **KPI strip** — `reports::parse::parse_from_report(path)` →
  `BacktestMetrics`. Graceful: absent `CAGR`/`win_rate` rows →
  `_present=false` (renders `—`); only `Err(NoSummaryHeading)` on a truly
  malformed body → `PanelState::Error`.
- **Equity companion** — `load_equity_companion(report_path)` scans
  `<dir>/artifacts/<run_id>/equity-*.csv` and parses via
  `reports::csv_artifacts::read_equity_csv` (the native
  `ts, equity_total_usdt, realized_pnl_usdt, unrealized_pnl_usdt,
  cash_balance_usdt` RFC3339-µs schema) → `EquitySeries::from_points` →
  `downsample(2000)`. Missing companion → `PanelState::Empty` (the common
  case, above). This is a **distinct** schema from the BH 3-column loader the
  Baseline panel uses — the two loaders coexist; Reports does NOT reuse the
  Baseline loader.
- **Discovery** — a `walkdir`-style scan of `spec/*/reports/backtest-*.md`,
  resolved workspace-relative (`CARGO_MANIFEST_DIR/../..`, exactly as
  `baseline/loader.rs::workspace_root` + `registry_read`). Sorted
  deterministically (by feature slug then filename) for stable list ordering
  + reproducible snapshots. Unreadable dir → empty list + breadcrumb, never
  panic.

### Target types (verified against the codebase)

- `core::BacktestMetrics` (`crates/core/src/equity_series.rs:167`) — the
  `kpi_strip` input. Six fixed cards (Total return / CAGR / Sharpe / Max DD /
  Win rate / Trades); no Sortino/Calmar slot (same contract the Baseline
  panel hit — A2 there).
- `core::EquitySeries::from_points` — computes drawdown per-point, so the
  drawdown band is free once a curve loads (same as Baseline).
- `ui::viewer::{ReportLoadResult, ReportFrontMatter, ViewerModel, ViewerMessage}`
  (`crate::viewer`) — the existing lib model. The Reports screen state may
  reuse `ReportLoadResult` as the per-selection payload, or define its own
  thin selection state holding the discovered `Vec<PathBuf>` + the active
  `ReportLoadResult` — architect's call (D1).

## Relationship to the Baseline panel

The just-shipped **Baseline panel** (`Screen::Baseline`,
`spec/cockpit-baseline-panel/feature.md`) and this **Reports** screen are
deliberately parallel:

- **Baseline = one pinned, special-case result** (the passive-BH headline),
  with embedded §7.1 KPI scalars (D1=c there) + a 2023/2024 toggle + an
  honest bounded-scope caption. Its data is the **BH 3-column CSV** at a
  runbook path.
- **Reports = the general case** — any committed `backtest-*.md`, KPIs parsed
  live from the markdown table, equity from the **native 5-column companion**
  (when present).

**Decision for now: keep them separate.** Do **NOT** refactor Baseline into
Reports in v0.1.0. They use **different loaders** (3-column BH vs 5-column
native companion), **different metric sources** (embedded `const` vs
live-parsed table), and the Baseline caption/toggle is bespoke. Folding them
prematurely would couple two things that are honestly different.

**Noted for a future version (not this one):** once Reports exists, Baseline
*could* become a pinned first entry in the Reports picker (a "★ Passive
baseline" row that routes to the bespoke Baseline body, or that the BH curve
is re-homed as a discoverable report). That is a v0.2+ consolidation, recorded
here only so the architect knows the eventual shape and does not lock a
decision that blocks it. **The Baseline panel is out of scope to modify.**

## Open decisions (for the architect)

- **D1 — Selection state shape.** Reuse `ui::viewer::ReportLoadResult` as the
  per-selection payload (smaller delta, the lib type already exists), or a
  thin bespoke `ReportsScreenState { discovered: PanelState<Vec<ReportEntry>>,
  selected: Option<usize>, loaded: PanelState<ReportLoadResult> }`?
  **Analyst leans bespoke** (it carries the discovered list + selection index
  + the load result cleanly, mirrors `BaselineScreenState` and the
  Memory/Models list-detail state), but the architect owns the final shape.

- **D2 — Lift the viewer loader into a shared pure-`ui` module
  (RECOMMENDED — durable).** Lift `load_report` + `parse_front_matter` +
  `strip_front_matter` + `load_equity_companion` (currently in
  `bin/viewer.rs`) into `crate::viewer` (the lib, sibling of the existing
  `ViewerModel`) or a new `crate::reports_loader` module, then refactor the
  `viewer` bin to call it. Both the bin and the Reports screen then share
  **one** parse implementation. This is the durable choice — it prevents two
  copies of the markdown/CSV parse drifting apart, and it is a precondition
  for R4. The cheaper-but-divergent fallback (copy the parse into a new
  module, leave the bin's copy untouched) spawns a v0.2 "de-dupe the viewer
  parse" cleanup brief and risks the two copies disagreeing on a malformed
  report — **avoid unless budget forces it**. The lift is mechanical (the
  functions are already pure + tested via the bin); the bin's CLI/exit-code
  tests (`bin/viewer.rs` `#[cfg(test)]`) stay green because the call sites are
  unchanged in behavior.

- **D3 — Body render in v0.1.0?** Ship the markdown body (reuse the viewer
  bin's `body_render` heading pre-pass, lifted alongside D2) for parity with
  the offline viewer, OR ship picker + KPI strip + curve/band only and defer
  the body to v0.2? **Analyst leans ship-the-body** (it is already written + a
  trivial lift, and "read the report in-cockpit" is the point), but it is the
  architect's proportionality call — the body is the one piece that is
  genuinely optional for the M-sized cut.

- **D4 — Sidebar group placement.** Work group (next to `Baseline`) or
  Library group (next to `Models`/`Memory`)? Both are defensible; the
  flatten-invariant lock-step (add to `SIDEBAR_ENTRIES_PHASE_A` +
  `SIDEBAR_GROUPS_PHASE_C` in the same position) is identical either way.
  **Analyst leans Library** (Reports is a browse-a-corpus registry, the same
  shape as Models/Memory; Baseline is a single-result dashboard, a different
  shape) — but the architect decides.

- **D5 — Smoke routing.** Mirror the Baseline D2 decision: **navigable, not
  default-routed**, with a dedicated `headless_emulator_paints_reports_route`
  test carrying the no-panic assertion (the smoke's default stays on `Live`
  for determinism). The discovery scan must degrade to an empty list (never
  panic) in a fixtures-only checkout where `spec/` reports may be absent —
  the empty-list state is then the deterministic smoke surface. Confirmed: the
  smoke harness uses **per-route dedicated tests**
  (`headless_emulator_paints_baseline_route`), not an all-screens visitor, so
  Reports needs its own.

- **D6 — Non-empty-curve demo (optional).** Per § Data contract, no
  `backtest-*.md` has an equity companion. If the operator wants a non-empty
  curve in v0.1.0, the architect decides whether to surface the two real
  `equity-*.csv` files at the `real-mtm-unrealized-pnl` presentations path as
  a proof-of-render example. **Default: do not** — ship over the
  `backtest-*.md` corpus with curves Empty-by-data; the slot lights up
  automatically when a report ships a companion. This is a nice-to-have, not a
  requirement.

## Acceptance criteria

Proportionate + testable. The tester closes the loop against these.

- **AC1 — Report picker discovers + lists the corpus.** With committed
  reports present, `Screen::Reports` lists the discovered `backtest-*.md`
  reports labelled by feature slug + filename/date, in deterministic order.
  The robustness-sweep / test-report families are **not** listed (R1 / § Data
  contract).
- **AC2 — Selection renders the report.** Selecting an entry renders its KPI
  strip (parsed from the `## Summary` table) + (if D3) its markdown body, in
  the detail pane. The equity curve + drawdown band render their **Empty**
  state for current-corpus reports (no companion CSV) — asserted as the
  expected state, not a failure (R2 / R3 / § Data contract).
- **AC3 — Four panel states behave, no panic** (R3). A test covers: empty
  list ("No backtest reports found" copy), a Ready selection (KPI strip
  populated), the Empty curve/band on a companion-less report, and the
  **Error** path — a malformed/`## Summary`-less report → `PanelState::Error`
  on the KPI strip, no panic. Discovery on an unreadable dir → empty list, no
  panic.
- **AC4 — Fixtures `cockpit` smoke passes** — first-frame render of the
  Reports route, **no panic**, within the existing smoke window
  (`headless_emulator_paints_reports_route`, mirroring the Baseline route
  test). Degrades to an empty list (never panics) in a fixtures-only checkout.
- **AC5 — Shared loader, no parse duplication** (R4 / D2). Review confirms the
  Reports screen and the `viewer` bin call **one** lifted load implementation;
  the bin's CLI/exit-code tests stay green after the refactor.
- **AC6 — Lumen-consistent** — `tests/consistency.rs` / `tests/contrast.rs` /
  `tests/layout_invariants.rs` stay green; **no hardcoded colors** (theme
  tokens only) and **no hardcoded strings** (all copy via `strings.rs`
  `REPORTS_*`). Renders in both themes. A panel-snapshot test (both themes) is
  added per the cockpit's 267-test snapshot convention, and the
  `SIDEBAR_GROUPS_PHASE_C` flatten-invariant test is updated to include
  `Screen::Reports`.
- **AC7 — No new crate edge, no new widget, no new theme token.** Review
  confirms the loader is pure-`ui` over `core` + `reports` + `std::fs`; the
  three render widgets (+ optional `body_render`) are reused verbatim.

### Lint convention (pre-existing tech-debt — do not fix-all)

The `crates/ui` crate carries ~140 pre-existing pedantic clippy lints. New
Reports code follows the crate's existing per-module allow-pattern (mirror
`screens/baseline.rs` / `screens/live.rs`); it introduces **no new warnings**
and does **not** attempt to clear the pre-existing 140.

## Fold-in cleanup (trivial — for the implementer)

The implementer will be editing `crates/ui/src/bin/cockpit.rs` (and likely
`cockpit_live.rs`) to add the `Screen::Reports` boot wiring. While there, fix
the one deprecated-alias line: **`cockpit.rs:185` sets
`cockpit.current_screen = Screen::Home`** — `Home` is the
`#[deprecated(since="0.2.0", note="use Screen::Live")]` alias. Change it to
`Screen::Live` (the live behavior is identical — `shell::screen_body` routes
`Screen::Live | Screen::Home` to the same `live::view`). One-liner; it removes
the last in-tree use of the deprecated `Home` alias in a binary boot path.
(Out of scope to chase the other deprecated aliases that exist only in
`#[cfg(test)]` harness code — `state.rs:3123,3217,3290`.)

## Design

_Architect-owned (2026-06-17). Shapes locked against the **actual** current
code (line numbers re-verified — the 2026-06-08 brief had drifted; see
"Drift corrections" below). No new crate edge, no new widget, no new theme
token — the brief's M-sized hold-the-line is RATIFIED. No ADR needed: every
decision reuses an in-tree precedent (Baseline panel, Models registry,
viewer bin); no cross-crate edge, no anchor mutation._

### Drift corrections (brief line numbers re-verified)

The brief's specific line citations have shifted since 2026-06-08. The
**substance** is unchanged; only the addresses moved. Verified current
locations the ui-designer should trust:

- **`cockpit.rs:185` `Screen::Home` fold-in fix — ALREADY DONE.** The brief's
  "Fold-in cleanup" §  asks the implementer to change `cockpit.rs:185` from
  `Screen::Home` to `Screen::Live`. **That line already reads
  `cockpit.current_screen = Screen::Live;`** (verified `crates/ui/src/bin/cockpit.rs:185`),
  and `grep "Screen::Home" crates/ui/src/bin/` returns **nothing** — both
  boot bins are already clean (`cockpit_live.rs:608` is also `Screen::Live`).
  **This fold-in is a NO-OP; do not re-apply it.** The only remaining
  `Screen::Home` uses are in `#[cfg(test)]` harness code (`state.rs:~4197`),
  which the brief explicitly scopes out.
- `bin/viewer.rs` load fns confirmed present with these exact signatures:
  `fn load_report(path: &Path) -> Result<ReportLoadResult, std::io::Error>`
  (`:136`), `fn load_equity_companion(report_path: &Path) -> Result<PanelState<EquitySeries>, String>`
  (`:172`), `fn parse_front_matter(raw: &str) -> ReportFrontMatter` (`:223`),
  `fn strip_front_matter(raw: &str) -> &str` (`:244`), and `mod body_render`
  with `pub fn view<'a>(markdown: &'a str, mode: ThemeMode) -> Element<'a, ViewerMessage>`
  (`:263-300`). `App::view` composition (KPI strip → curve → band → body) is
  `bin/viewer.rs:101-124`. All confirmed.
- `crate::viewer` lib (`crates/ui/src/viewer.rs`) holds `ReportFrontMatter`,
  `ReportLoadResult` (4 fields: `front_matter`, `metrics`, `equity`,
  `body_markdown`), `ViewerModel`, `ViewerMessage`, `update` — confirmed; the
  load fns are NOT here yet (they live in the bin), which is exactly why D2
  lifts them here.
- `reports::parse::parse_from_report(path) -> Result<BacktestMetrics, ParseError>`
  with `ParseError::NoSummaryHeading` (`crates/reports/src/parse.rs:43,33`);
  `reports::csv_artifacts::read_equity_csv(path) -> Result<Vec<EquitySample>, CsvError>`
  + the 5-col `EquitySample { ts, equity_total, realized_pnl, unrealized_pnl,
  cash_balance }` (`csv_artifacts.rs:52,109`); `core::BacktestMetrics`
  (`equity_series.rs:167`, 9 fields incl. `*_present` flags) +
  `EquitySeries::from_points` (`:68`) + `downsample` (`:130`). All confirmed.
- Sidebar: `SIDEBAR_ENTRIES_PHASE_A` (`theme.rs:747`) +
  `SIDEBAR_GROUPS_PHASE_C` (`theme.rs:773`) + the flatten-invariant test
  `sidebar_groups_phase_c__flatten_matches_phase_a` (`theme.rs:1607`);
  `sidebar_nav::label_for` (`sidebar_nav.rs:35`); `Screen` enum
  (`state.rs:113`, `Baseline` at `:142`); `shell::screen_body` exhaustive
  match (`shell.rs:138`, no catch-all → a new arm is required). Confirmed.

### D-resolutions

**D1 — Selection state shape → bespoke `ReportsScreenState` (RATIFY analyst
lean).** A new pure-data struct in `crate::reports::state`:

```rust
pub struct ReportsScreenState {
    /// Discovered corpus. `Loading` pre-boot, `Ready(list)` after the boot
    /// scan, `Empty` when the scan finds zero `backtest-*.md`, `Error` only
    /// on a catastrophic scan failure (not expected — the scan degrades to
    /// an empty list per K2, so `Empty` is the normal no-reports surface).
    pub discovered: PanelState<Vec<ReportEntry>>,
    /// Index into the discovered list of the active selection. `None` = no
    /// selection yet (detail pane shows a "pick a report" prompt-empty).
    pub selected: Option<usize>,
    /// The active selection's load result. `Loading` transiently, `Ready`
    /// once loaded. Each field of `ReportLoadResult` carries its own
    /// PanelState, so the KPI strip can be `Ready` while the curve is `Empty`.
    pub loaded: PanelState<ReportLoadResult>,
}

pub struct ReportEntry {
    pub slug: SmolStr,        // feature slug (parent-of-parent dir name)
    pub file_stem: SmolStr,   // backtest-<YYYYMMDD-HHMMSS>-<scenario>
    pub path: PathBuf,        // full path held in state — never the msg key
}
```

Reason for OVERRIDING "reuse `ReportLoadResult` directly as the screen
state": `ReportLoadResult` is a single load payload; it cannot carry the
discovered list + the selection index. The bespoke struct mirrors
`BaselineScreenState` (`baseline/state.rs:37`) and `ModelsScreenState`
(`models/state.rs:114`) one-for-one — the established list-detail shape. It
**reuses** `ReportLoadResult` (the lib type) verbatim as the `loaded`
payload, so we add no parallel load-result type. `ReportEntry` holds the
`PathBuf` in state so the selection message is a typed index, satisfying R1.

**D2 — Lift the viewer loader → new `crate::reports::loader` module (RATIFY
recommended/durable).** Lift `load_report`, `parse_front_matter`,
`strip_front_matter`, `load_equity_companion`, and `mod body_render` out of
`bin/viewer.rs` into a new sibling-of-`baseline` feature module
`crates/ui/src/reports/loader.rs` (with `body_render` promoted to a pub
sibling `crates/ui/src/reports/body_render.rs` so both the bin and the
screen call one impl). Then refactor `bin/viewer.rs` to `use
ui::reports::loader::{load_report, ...}` + `ui::reports::body_render::view`,
deleting its local copies.

- **Target = `crate::reports::loader`, NOT `crate::viewer`.** Reason for
  this OVERRIDE of the brief's "lift into `crate::viewer` (the lib) **or** a
  new `crate::reports_loader`": the new feature module is `crate::reports`
  (housing `reports::state` + `reports::loader` + `reports::body_render`,
  the Baseline `crate::baseline` precedent — `lib.rs:52`). Putting the lifted
  load fns under `crate::reports::loader` co-locates them with their primary
  new consumer and the screen state, exactly as `baseline::loader` sits with
  `baseline::state`. `crate::viewer` stays the **model/message** module
  (it is named for the bin and would be a confusing home for a now-shared
  loader). Note: module path is `crate::reports::*` (the UI feature module,
  distinct from the `reports` **crate** = `trading`'s report-writer crate;
  the `use reports::parse` crate import and the `crate::reports` module
  coexist unambiguously — Rust resolves `crate::reports` to the local module
  and bare `reports::` to the extern crate, as `bin/viewer.rs` already does).
- The lift is **mechanical** (the fns are already pure + exercised by the
  bin's `#[cfg(test)]`). The bin's CLI/exit-code tests
  (`cli_parser_*`, `cli_help_renders_without_lumen`, `parse_front_matter_*`)
  stay green; the `parse_front_matter_extracts_scenario` test moves with the
  fn (or the bin keeps a thin re-export call — implementer's mechanical
  choice, but the assertion must survive). The `viewer_read_only.rs`
  build-time grep (greps `bin/viewer.rs` for `File::create`/`write` against
  `spec/**`) stays green — the lift removes file-READ helpers, adds no write.
- This is the precondition for R4/AC5 (one parse impl, no drift). The
  cheaper copy-the-parse fallback is **rejected** — it spawns a v0.2 de-dupe
  brief and risks two copies disagreeing on a malformed report.

**D3 — Ship the markdown body in v0.1.0 (RATIFY analyst lean).** Reuse the
lifted `reports::body_render::view` heading pre-pass verbatim in the detail
pane. Reason: it is already written, the lift is part of D2 (zero marginal
cost), and "read the report in-cockpit" is half the feature's value (the
brief's "the picker + KPI strip are the load-bearing value" leaves the body
optional, but at zero cost there is no proportionality argument to defer it).
The body renders inside the existing `scrollable` the `body_render` fn
already returns.

**D4 — Sidebar placement → Library group, after `Models` (RATIFY analyst
lean).** Insert `Screen::Reports` into the **Library** group
(`Strategies · Memory · Models · Trail`) immediately **after `Models`**.
Reason for choosing Library over Work: Reports is a **browse-a-corpus
registry** — the identical shape to Models (`discover_*` scan → list →
detail) and Memory — whereas the Work group's `Baseline` is a single-pinned-
result dashboard (a different shape). Placing it after `Models` groups the
two "scan a committed corpus and render a selected item" screens adjacently.
Exact lock-step (the flatten-invariant test `theme.rs:1607` is the guard —
AC6):
- `SIDEBAR_ENTRIES_PHASE_A` (`theme.rs:747`): insert `Screen::Reports`
  between `Screen::Models` and `Screen::Trail`.
- `SIDEBAR_GROUPS_PHASE_C` (`theme.rs:773`): in the **library** sub-slice
  `&[Strategies, Memory, Models, Trail]`, insert `Reports` between `Models`
  and `Trail` → `&[Strategies, Memory, Models, Reports, Trail]`.
- The two edits MUST be the same relative position (Models < Reports <
  Trail in both) or the flatten test fails — that is the lock-step guard.

**D5 — Smoke routing → navigable, not default-routed (RATIFY; mirror
Baseline D2).** The cockpit default stays `Screen::Lab`/`Live` (deterministic
first frame). Reports is reachable only via the sidebar. Add a dedicated
`headless_emulator_paints_reports_route` test (mirror
`headless_emulator_paints_baseline_route`, `headless_emulator_smoke.rs:87`):
boot the fixtures cockpit, set `current_screen = Screen::Reports`, run the
boot discovery scan, drain to `Ready`, assert a non-empty 1280×720
first-frame screenshot, **no panic**. In a fixtures-only checkout where
`spec/` reports are absent, the scan degrades to an empty list (K2) and the
Empty list-state is the deterministic smoke surface — still a clean paint.

**D6 — Non-empty-curve demo → DO NOT ship (RATIFY default).** Ship over the
`backtest-*.md` corpus with the equity curve + drawdown band rendering their
**Empty** state (no companion CSV exists for any report — the honest,
accepted state per § Data contract). Do **not** surface the two
`real-mtm-unrealized-pnl` presentations-path `equity-*.csv` files as a
proof-of-render: they live on a `presentations/` path (not a
`reports/artifacts/<run_id>/` companion the loader scans), and seeding them
would require a bespoke off-corpus code path that contradicts AC7's
"read-only over the discovered committed corpus." The equity slot is a
latent capability that lights up automatically the day a `backtest-*.md`
ships a companion CSV. **This empty-by-data state is the feature, not a
defect — no effort is to be spent chasing the absent curve.**

### Loader-lift plan (exact)

New module tree under `crates/ui/src/reports/` (sibling of `baseline/`):

| File | Contents | Source |
|------|----------|--------|
| `reports/mod.rs` | `pub mod loader; pub mod state; pub mod body_render;` + re-exports (`pub use state::{ReportsScreenState, ReportEntry};`) | new (mirror `baseline/mod.rs`) |
| `reports/loader.rs` | **lifted** `load_report`, `parse_front_matter`, `strip_front_matter`, `load_equity_companion` (all made `pub`) + **new** `discover_reports() -> Vec<ReportEntry>` (all-slug `spec/*/reports/backtest-*.md` scan) + `workspace_root()` (copy the `baseline/loader.rs:234` helper) | `bin/viewer.rs:136-260` (lift) + new scan |
| `reports/body_render.rs` | **lifted** `mod body_render` → `pub fn view(markdown, mode) -> Element<ViewerMessage>` | `bin/viewer.rs:263-300` (lift) |
| `reports/state.rs` | `ReportsScreenState` + `ReportEntry` (D1) + a `load_selection(&mut self, idx)` helper that calls `loader::load_report` and stores the result | new (mirror `baseline/state.rs`) |

`crate::reports` is registered in `lib.rs` (a `pub mod reports;` line next to
`pub mod baseline;`, `lib.rs:52`).

**Discovery scan (new, K2 never-panic):**
`discover_reports()` resolves `workspace_root().join("spec")`, reads its
entries (each a feature-slug dir); for each slug dir it reads
`<slug>/reports/`, filters `backtest-*.md` (the established filter at
`lab/equity_loader.rs:261` — `starts_with("backtest-") && ends_with(".md")`),
and builds `ReportEntry { slug, file_stem, path }`. Sorted deterministically
by `(slug, file_stem)` for stable list ordering + reproducible snapshots.
Any unreadable dir → skip with a `tracing::debug!` breadcrumb, never panic
(mirror `registry_read.rs:92` + `baseline/loader.rs:77`). The
`robustness-sweep-*.md` / `test-*.md` families are excluded by the
`backtest-` filter (R1 / § Data contract). NOTE: this is a NEW top-level
scan — the existing `lab::equity_loader::discover_reports` is `private` +
**per-slug** (`spec_root, slug`), so it is not directly reusable; the new fn
walks all slugs. This is **not** parse duplication (R4/AC5 is about the
markdown/CSV **parse**, which IS shared via the lift) — discovery is a
distinct concern with no existing all-slug public API.

### `Screen::Reports` state shape + wiring touchpoints

- **`Screen` enum** (`state.rs:113`): add `Reports` variant after `Baseline`
  (`:142`), with a doc-comment mirroring Baseline's (navigable via Library
  group, not default-routed).
- **`Cockpit` struct** (`state.rs:~987`, three-touchpoint pattern — struct
  field + it derives `Debug`/`Clone` so no manual impl + `Default`/`new()`):
  add `pub reports_screen_state: crate::reports::ReportsScreenState`, default
  `ReportsScreenState::default()` (= `discovered: Loading`, `selected: None`,
  `loaded: Loading`) at both construction sites (`state.rs:~1207` and
  `~1321`, exactly as `baseline_screen_state` appears in both).
- **Boot-load** (mirror `baseline::load_into`): add
  `crate::reports::state::load_into(&mut Cockpit)` that runs the boot
  discovery scan → `model.reports_screen_state.discovered =
  PanelState::Ready(discover_reports())` (or `Empty` on zero results). Call
  it from both bins' boot paths next to the existing
  `ui::baseline::load_into(&mut cockpit)` (`cockpit.rs:238`,
  `cockpit_live.rs:618`). Discovery is filename-only (no per-file parse), so
  it is cheap + synchronous — the Baseline-style boot-load fits the
  "synchronous + cheap" + Loading→Ready contract (R3). Per-selection
  `load_report` (one-file parse) runs lazily on the select message arm.

### Typed message arm(s)

Two new `Message` variants in `state.rs` (the typed-message discipline —
mirror `BaselineSelectYear(BaselineYear)` at `state.rs:1951`):

```rust
/// Operator picked report at index `usize` in the discovered list. The
/// PathBuf lives in `reports_screen_state.discovered[idx].path` — the
/// message key is the typed index, NEVER a String/PathBuf payload (R1).
ReportsSelect(usize),
```

`update` arm (`state.rs`, near the `BaselineSelectYear` arm at `:2824`):

```rust
Message::ReportsSelect(idx) => {
    model.reports_screen_state.selected = Some(idx);
    // Synchronous one-file load (parse the ## Summary table + scan for the
    // companion CSV). Lifted loader; never panics (parse-miss → the loaded
    // field's metrics PanelState carries Error, not a panic).
    model.reports_screen_state.load_selection(idx);
}
```

`load_selection` looks up the `PathBuf` by index, calls
`loader::load_report(path)`, and stores `PanelState::Ready(result)` (or, if
the file vanished between discovery and selection, `PanelState::Error` — the
Error-on-detail surface, R3). Because the load is fast (<100 ms for these
small files), no async `Task` is needed — synchronous in the arm, matching
the Baseline precedent. (If a future large body parse exceeds the ~100 ms
rule, the `loaded` field's transient `Loading` state is the spinner hook —
not needed at v0.1.0.) **No boot hydrate message** is needed (boot discovery
is the synchronous `load_into`), so `ReportsSelect` is the only new arm.

### Screen body (`screens/reports.rs`)

`pub fn view(model: &Cockpit, mode: ThemeMode) -> crate::Element<'_>`,
registered in `screens/mod.rs` (`pub mod reports;`) and routed in
`shell::screen_body` (`shell.rs:138`) with a new arm
`Screen::Reports => reports::view(model, mode)`. Layout = the established
list-detail split (mirror Memory/Models toolbar+list + the Baseline detail
composition):

- **Left: report picker.** A scrollable `Column` of selectable rows, one per
  `ReportEntry`, labelled `"<slug> · <file_stem>"`. Each row is a `Button`
  with `.on_press(Message::ReportsSelect(idx))`, active-row styling mirroring
  the Baseline year-chip token discipline (active = `PANEL_RAISED` bg +
  `ACCENT`; inactive = `FG_3`/`BORDER_1`). Picker title =
  `REPORTS_PICKER_TITLE` ("Backtest reports" — surfaces the scope so the
  exclusion of robustness/test families is not a mystery, per § Data
  contract). Empty list → `REPORTS_EMPTY_LIST` copy ("No backtest reports
  found in spec/ yet").
- **Right: detail pane.** When `selected` is `None` →
  `REPORTS_SELECT_PROMPT` ("Select a report to view its results"). When
  `Ready(ReportLoadResult)` → the **verbatim** `bin/viewer.rs:101-124`
  composition: `kpi_strip::view(&loaded.metrics, mode)` →
  `equity_curve::view(&loaded.equity, mode)` →
  `drawdown_band::view(&loaded.equity, mode)` →
  `body_render::view(&loaded.body_markdown, mode)`, each bridged to
  `Message` via `.map(|_| Message::ChartMarkerHoverEnded)` (the never-fired
  no-op arm `screens/baseline.rs:79` uses). The three widgets + body_render
  take `&PanelState<…>`/`&str` borrowed from the model-stored
  `ReportLoadResult`, so the element gets the model's lifetime (the Baseline
  borrow pattern — `baseline/state.rs` doc-comment). KPI strip `Error`
  (malformed `## Summary`) renders the strip's muted Error body; curve/band
  `Empty` (no companion) render their built-in empty bodies — both expected,
  not failures (R3 / AC2).

### Strings (`strings.rs` — `REPORTS_*` block, mirror BASELINE block at `:1834`)

All copy via `crate::strings` (R5 / AC6 — no inline literals). New block +
registry-table entries (the `pub const NAME: &str` + the
`("NAME", NAME)` row in the registry table at `strings.rs:~1479`, exactly as
the BASELINE entries appear):

| Const | Value |
|-------|-------|
| `REPORTS_SIDEBAR_LABEL` | `"Reports"` |
| `REPORTS_PICKER_TITLE` | `"Backtest reports"` |
| `REPORTS_EMPTY_LIST` | `"No backtest reports found in spec/ yet."` |
| `REPORTS_SELECT_PROMPT` | `"Select a report to view its results."` |
| `REPORTS_LOAD_ERROR` | `"This report could not be read — it may have been moved or its summary table is malformed."` |

`sidebar_nav::label_for` (`sidebar_nav.rs:35`) gets a `Screen::Reports =>
REPORTS_SIDEBAR_LABEL` arm.

### Test plan (maps to AC1–AC7)

1. **`reports/loader.rs` unit tests** (mirror `baseline/loader.rs` tests):
   - `discover_reports` finds `backtest-*.md`, excludes
     `robustness-sweep-*.md` + `test-*.md` (AC1) — gated on `spec/` present,
     skip-if-absent like `committed_csvs_load_to_ready` does.
   - `discover_reports` on an unreadable/absent root → empty `Vec`, no panic
     (AC3 — K2). Deterministic sort assertion.
   - `load_report` on a fixture with a valid `## Summary` → metrics `Ready`;
     on a `## Summary`-less fixture → metrics `Error` (NoSummaryHeading), no
     panic (AC3); companion-less report → equity `Empty` (AC2).
   - The moved `parse_front_matter_extracts_scenario` assertion survives the
     lift (AC5).
2. **`reports/state.rs` unit tests:** `Default` = (`discovered: Loading`,
   `selected: None`, `loaded: Loading`); `ReportsSelect`/`load_selection`
   round-trip sets `selected` + transitions `loaded`; selecting a
   vanished-path index → `loaded: Error`, no panic (AC3).
3. **`bin/viewer.rs` CLI/exit-code tests stay green** after the refactor
   (AC5) — re-run `cargo test -p ui --bin viewer`.
4. **Panel-snapshot test, both themes** (`tests/panel_snapshots.rs`, mirror
   the `baseline_screen` mod at `:3248`): a textual-summary snapshot of the
   Reports screen body for Dark + Light — picker title + N rows (or empty
   copy) + the detail pane (a Ready selection's KPI lines + curve/band
   Empty-state + body line). Asserts copy resolves through `strings::REPORTS_*`
   and the one theme-dependent token (active-row accent) differs Dark vs
   Light (AC6). Use a deterministic fixture corpus (synthesize a tiny
   `ReportsScreenState` with hand-built `ReportEntry` + a `Ready`
   `ReportLoadResult`) so the snapshot is checkout-independent.
5. **`tests/consistency.rs` / `contrast.rs` / `layout_invariants.rs` stay
   green** (AC6) — no new tokens, no hardcoded hex/strings.
6. **Flatten-invariant test updated** (`theme.rs:1607`
   `sidebar_groups_phase_c__flatten_matches_phase_a`): passes automatically
   once both `SIDEBAR_ENTRIES_PHASE_A` and `SIDEBAR_GROUPS_PHASE_C` get
   `Reports` in the same relative position. Add a one-line assertion that
   `Reports` sits between `Models` and `Trail` in the flattened list (AC6).
7. **`headless_emulator_paints_reports_route`** (`tests/headless_emulator_smoke.rs`,
   mirror `:87`): boot fixtures cockpit → `Screen::Reports` →
   `reports::state::load_into` → drain to `Ready` → non-empty 1280×720
   screenshot, no panic; empty-list degrade in a fixtures-only checkout (AC4).
8. **Crate-edge / widget / token review** (AC7): loader is pure-`ui` over
   `core` + `reports` + `std::fs` (both already deps — `ui/Cargo.toml:77,86`);
   the three render widgets + `body_render` are reused verbatim; zero new
   theme tokens. Pre-existing ~140 pedantic clippy lints are NOT touched —
   new modules carry the per-module `#![allow(...)]` pattern that
   `screens/baseline.rs:32` uses (no new warnings).

## Backtest Scenarios

N/A — this is a **read-only UI feature** over already-committed reports. It
runs no new strategy and produces no new backtest. Per CLAUDE.md, the
baseline-equity-divergence e2e gate applies to **strategy overlays / sizing
modifiers** — this is neither (no overlay, no sizing math, no decision
variable). The verification floor is the panel-snapshot suite + the four-state
+ no-panic tests + the unchanged regression-anchor gate (no anchored file is
touched — Reports only **reads** committed reports).

## Implementation

_ui-designer (2026-06-17). Implemented the architect's resolved D1–D6
verbatim; no design re-litigation. All M-DEV + M-TEST tasks complete; gates
green (see § Verification gates below). No new crate edge, no new widget, no
new theme token (AC7 held)._

### Module map (what was built)

**New `crate::reports` feature module** (sibling of `crate::baseline`,
registered `pub mod reports;` in `lib.rs`):

| File | Contents |
|------|----------|
| `crates/ui/src/reports/mod.rs` | Re-exports `state::{ReportEntry, ReportsScreenState, load_into}`; doc-comment disambiguating `crate::reports` (the UI module) from the `reports` extern crate. |
| `crates/ui/src/reports/loader.rs` | **D2 lift** — `load_report`, `load_equity_companion`, `parse_front_matter`, `strip_front_matter` moved verbatim from `bin/viewer.rs` (all made `pub`), **plus the new all-slug `discover_reports() -> Vec<ReportEntry>`** scan of `spec/*/reports/backtest-*.md` + a `workspace_root()` helper (copied from `baseline/loader.rs`). The `is_backtest_report` filter (`starts_with("backtest-") && ends_with(".md")`) excludes the robustness-sweep / test-report families by construction. K2 never-panic (unreadable dir → skip + `tracing::debug!`; absent `spec/` → empty Vec). 7 unit tests incl. the **moved** `parse_front_matter_extracts_scenario` (AC5). |
| `crates/ui/src/reports/body_render.rs` | **D2 lift** — the `mod body_render` heading pre-pass (`# / ## / ###` → `text::H2/H3`) promoted to `pub fn view(markdown, mode) -> Element<ViewerMessage>`. |
| `crates/ui/src/reports/state.rs` | **D1** — `ReportsScreenState { discovered: PanelState<Vec<ReportEntry>>, selected: Option<usize>, loaded: PanelState<ReportLoadResult> }` (reuses the `ReportLoadResult` lib type verbatim as the `loaded` payload) + `ReportEntry { slug, file_stem, path }` (PathBuf held in state, never the msg key) + `load_selection(idx)` (synchronous, never-panics → `loaded: Error` on a vanished path) + `load_into(model)` boot scan. 4 unit tests. |
| `crates/ui/src/screens/reports.rs` | **M-DEV-7** — `pub fn view(model, mode)`: left picker (scrollable `Button` rows, `.on_press(Message::ReportsSelect(idx))`, Baseline chip-token active styling) + right detail pane (the **verbatim** `bin/viewer.rs` stack — `kpi_strip` → `equity_curve` → `drawdown_band` → `body_render`, each bridged `.map(|_| Message::ChartMarkerHoverEnded)`). Empty/prompt/error surfaces all carry `REPORTS_*` copy (no blank screen). Per-module clippy allow mirrors `screens/baseline.rs:32`. |

**Edits to existing files:**

- `bin/viewer.rs` — **refactored to call the lifted loader**: deleted the
  local `load_report` / `load_equity_companion` / `parse_front_matter` /
  `strip_front_matter` / `mod body_render`; now `use ui::reports::loader::*`
  + `ui::reports::body_render`. `App::view` / `main` call sites unchanged in
  behaviour. The `parse_front_matter_extracts_scenario` test moved to
  `reports/loader.rs`; the CLI/exit-code tests stayed in the bin. (AC5 — one
  shared parse implementation; the bin's `cli_parser_*` + the
  `viewer_read_only.rs` build-time grep both stay green.)
- `state.rs` — `Screen::Reports` variant (after `Baseline`);
  `pub reports_screen_state: crate::reports::ReportsScreenState` on `Cockpit`
  (struct field + `Debug` impl field + both construction sites);
  `Message::ReportsSelect(usize)` (typed index, no String/PathBuf payload);
  the `update` arm (sets `selected` + calls `load_selection`).
- `theme.rs` — `Screen::Reports` inserted into **both**
  `SIDEBAR_ENTRIES_PHASE_A` and `SIDEBAR_GROUPS_PHASE_C` between `Models` and
  `Trail` (D4, Library group); the `sidebar_groups_phase_c__flatten_matches_phase_a`
  test extended with the `Models < Reports < Trail` ordering assertion.
- `widgets/sidebar_nav.rs` — `Screen::Reports => REPORTS_SIDEBAR_LABEL` arm.
- `strings.rs` — `REPORTS_*` block (5 consts) + 5 registry-table rows.
- `screens/mod.rs` + `shell.rs` — `pub mod reports;` + the
  `Screen::Reports => reports::view(model, mode)` routing arm + `reports` in
  the `use crate::screens::{…}` list.
- `bin/cockpit.rs` + `bin/cockpit_live.rs` — `ui::reports::load_into(&mut
  cockpit)` boot scan next to `ui::baseline::load_into`. Default route stays
  `Screen::Live` (D5). The `cockpit.rs:185` `Screen::Home`→`Live` fold-in was
  **already done** (architect's drift correction — NO-OP, not re-applied).

### Tests added (M-TEST)

- `reports/loader.rs` `#[cfg(test)]` (7): discovery finds `backtest-*` /
  excludes sweep+test families, deterministic-sort, the `backtest-` filter,
  `load_report` valid→Ready / no-summary→`metrics: Error` / missing-file→`Err`
  / companion-less→`equity: Empty`, the moved `parse_front_matter` test.
- `reports/state.rs` `#[cfg(test)]` (4): `Default`, vanished-path→Error,
  out-of-range→Error, valid-fixture→Ready + `selected_entry`.
- `tests/panel_snapshots.rs` `reports_screen` mod (5): Ready dark + light
  (both-theme gate; active-row accent differs), empty-list, detail-error,
  accent-differs-by-theme. Snapshot baselines generated (see filenames below).
- `tests/headless_emulator_smoke.rs`: `headless_emulator_paints_reports_route`
  (mirror of the Baseline route test; degrades to empty list in a
  fixtures-only checkout; default smoke route stays `Live`).
- `theme.rs` flatten-invariant extended (the lock-step guard).

### New snapshot baselines (committed)

`crates/ui/tests/snapshots/`:
- `panel_snapshots__reports_screen__reports_snapshot__ready_dark.snap`
- `panel_snapshots__reports_screen__reports_snapshot__ready_light.snap`
- `panel_snapshots__reports_screen__reports_snapshot__empty_list_dark.snap`
- `panel_snapshots__reports_screen__reports_snapshot__detail_error_dark.snap`

**Regenerated visual baselines (sidebar-row shift — expected fallout of
adding a Library nav entry, identical to the Baseline panel's fan-out):** the
two `sidebar_nav` lib snapshots (`.../ui__widgets__sidebar_nav__tests__sidebar__phase_a_workflow_group.snap`,
`...phase_c_three_groups.snap`) + **48 full-shell visual-baseline PNGs**
under `crates/ui/tests/visual-baselines/` (the `visual_snapshots.rs` corpus +
the legacy `charts_screen_dark_*` + `strategies_ready_dark_*` PNGs). Verified
via the perceptual-diff harness that **every changed pixel is confined to the
left sidebar column** (the new `Reports` row + the downshifted `Trail`/`Settings`
rows) — the screen bodies are byte-identical. Regenerated by deleting the
affected baselines and re-running the harness twice (auto-write + determinism
re-check, both green).

### AC coverage

| AC | Covered by |
|----|-----------|
| AC1 picker discovers + lists corpus, excludes sweep/test | `discover_finds_backtest_excludes_other_families`, `discover_is_deterministically_sorted`, `is_backtest_report_filter` |
| AC2 selection renders KPI strip + body; curve/band Empty-by-data | `reports_snapshot__ready_{dark,light}`, `load_report_valid_summary_ready_no_companion_empty` |
| AC3 four states + no panic (empty list / Ready / Empty curve / Error / unreadable-dir) | `reports_snapshot__{empty_list,detail_error}_dark`, `load_report_no_summary_yields_metrics_error_no_panic`, `load_selection_vanished_path_yields_error_no_panic`, `load_selection_out_of_range_or_not_ready_yields_error`, `load_report_missing_file_is_err_no_panic` |
| AC4 fixtures smoke paints Reports route, no panic, empty-list degrade | `headless_emulator_paints_reports_route` |
| AC5 shared loader, bin tests stay green | the D2 lift (one impl); `cargo test -p ui --bin viewer` green; `viewer_read_only.rs` green; moved `parse_front_matter_extracts_scenario` |
| AC6 Lumen-consistent (no hex/strings, both themes, snapshot, flatten test) | `REPORTS_*` strings; `reports_active_accent_differs_by_theme`; `consistency.rs`/`contrast.rs`/`layout_invariants.rs` green; `sidebar_groups_phase_c__flatten_matches_phase_a` extended |
| AC7 no new crate edge / widget / theme token | loader is pure-`ui` over `core`+`reports`+`std::fs`; 3 widgets + `body_render` reused verbatim; zero new tokens |

### Verification gates (ui-designer local run — re-runnable)

- `cargo build -p ui --lib --bins` — **Finished, clean**.
- `cargo test -p ui` — **856 passed, 0 failed, 27 ignored**.
- `cargo clippy -p ui --lib --tests --bins -- -D warnings` (forced re-lint
  via `touch lib.rs`) — **clean (rc 0)**.
- `cargo fmt -p ui --check` — **clean (rc 0)**.

### Deviations / risks

- **D2 lift target.** Followed the architect's resolution exactly: the load
  fns went to `crate::reports::loader` (NOT `crate::viewer`), co-located with
  the screen state. `crate::reports::loader` imports `ReportEntry` from
  `crate::reports::state`; `state` imports `loader` — a benign intra-module
  cycle that compiles cleanly (both are submodules of the same crate).
- **`cargo clippy -p ui --all-targets`** surfaces **one PRE-EXISTING lint**,
  `crates/ui/benches/cockpit_render.rs:107` `empty line after doc comment`,
  in a bench file **this feature did not touch** (git-confirmed: empty diff).
  Per the brief's "do NOT fix-all the ~140 pre-existing pedantic lints" +
  scope-boundary discipline, it was left as-is rather than fixed. The
  feature's own code is `-D warnings`-clean on `--lib --tests --bins`. Flag
  to the tester: if the gate is `--all-targets`, this pre-existing bench lint
  needs a separate one-line cleanup (out of this feature's scope).
- **Transient cosmic-text proptest flake (NOT a Reports regression).** During
  full-suite runs, `tests/layout_invariants.rs` proptest fuzzers can discover
  a `cosmic-text-0.15.0/src/shape.rs:960` glyph-shaping panic that poisons the
  shared font-system mutex (→ cascade of `PoisonError` panics across unrelated
  widgets — `positions`, `compare`, `models`, etc.). The trigger is persisted
  to an **untracked** `crates/ui/tests/layout_invariants.proptest-regressions`
  cache, so it only re-fires once that cache exists. Deleting the untracked
  cache makes `layout_invariants` green (11/11, parallel + single-threaded).
  This is a pre-existing latent iced/cosmic-text issue, surfaced (not caused)
  by Reports' first full-suite run; it touches widgets Reports never modifies.
  The final 856/0 run was on a clean cache. Flag to the tester: if a future
  run trips it, `rm crates/ui/tests/layout_invariants.proptest-regressions`
  and re-run — do NOT attribute it to Reports.

## Verification

_tester links to reports here._

## Changelog

- 2026-06-08 (analyst): initial brief carved from the ui-designer build-out
  audit §3 candidate #2 (operator-greenlit as the #1-Baseline follow-on).
  Grounded in the actual `bin/viewer.rs` render path, the `crate::viewer` lib
  model, `reports::parse::parse_from_report`, the Baseline-panel precedent
  (`screens/baseline.rs`, `baseline/loader.rs`, the four `PanelState` states,
  the sidebar lock-step, the D2 navigable-not-default-routed + per-route
  headless-smoke test), and `models/registry_read.rs` (the K2 corpus-scan
  never-panic contract). **Load-bearing scope finding flagged:** the corpus
  survey found **zero** `backtest-*.md` reports with the
  `artifacts/<run_id>/equity-*.csv` companion the viewer scans for — so the
  equity curve + drawdown band render Empty-by-data for the entire corpus
  today (the `viewer` bin behaves identically). The picker is scoped to the
  112 `backtest-*.md` reports (the viewer's contract); the 34
  `robustness-sweep-*.md` θ-surfaces + 37 `test-*.md` verdict docs are
  excluded (no single equity path / no `## Summary` KPI strip). Tee'd D1
  (selection-state shape), D2 (lift the viewer loader into a shared pure-`ui`
  module — recommended/durable, prevents a v0.2 de-dupe brief), D3 (ship the
  body in v0.1.0?), D4 (sidebar group placement), D5 (smoke routing — mirror
  Baseline D2), D6 (optional non-empty-curve demo). Noted the Baseline
  relationship (keep separate now; possible v0.2+ fold-in). Flagged the
  trivial `cockpit.rs:185` `Screen::Home`→`Screen::Live` fold-in fix. Opened
  REQ-COCKPIT-REPORTS-001 (proposed). HANDOFF → architect.
- 2026-06-17 (architect): operator greenlit the build. Resolved D1–D6, wrote
  the `## Design` section + `tasks.md`, advanced status `candidate` →
  `arch-done`. **Shapes locked against the actual current code** (re-verified
  every load-bearing location — the 2026-06-08 brief line numbers had
  drifted). **D1** = bespoke `ReportsScreenState { discovered: PanelState<Vec<
  ReportEntry>>, selected: Option<usize>, loaded: PanelState<ReportLoadResult> }`
  (mirrors `BaselineScreenState`/`ModelsScreenState`; reuses the `ReportLoadResult`
  lib type as the payload). **D2** = lift `load_report` + `parse_front_matter`
  + `strip_front_matter` + `load_equity_companion` + `body_render` from
  `bin/viewer.rs` into a NEW `crate::reports::{loader, body_render}` feature
  module (the durable shared-parse choice; bin refactored to call it — bin
  CLI/exit-code tests stay green); target is `crate::reports`, not
  `crate::viewer` (co-locate with the screen state, the Baseline precedent).
  **D3** = ship the markdown body (zero marginal cost on top of the D2 lift).
  **D4** = sidebar **Library** group, after `Models` (browse-a-corpus shape =
  Models/Memory; lock-step `SIDEBAR_ENTRIES_PHASE_A` + `SIDEBAR_GROUPS_PHASE_C`
  + flatten-invariant test). **D5** = navigable, not default-routed; dedicated
  `headless_emulator_paints_reports_route` (mirror Baseline). **D6** = NO
  non-empty-curve demo — curves Empty-by-data (the accepted honest state).
  **Drift correction:** the brief's "Fold-in cleanup" `cockpit.rs:185`
  `Screen::Home`→`Screen::Live` is **already done** (both boot bins are clean;
  remaining `Screen::Home` uses are `#[cfg(test)]`-only) — that fold-in is a
  NO-OP, do not re-apply. No new crate edge / widget / theme token; no ADR
  (every decision reuses an in-tree precedent). `arch` column →
  REQ-COCKPIT-REPORTS-001 cites `## Design`. HANDOFF → ui-designer.
