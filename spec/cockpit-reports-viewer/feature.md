---
slug: cockpit-reports-viewer
status: proposed
owner: analyst
updated: 2026-06-08
version: 0.1.0
---

# Cockpit Reports viewer — browse + render committed backtest reports in-cockpit

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

_Architect-owned. Resolve D1–D6, the sidebar IA insertion, and the loader-lift
shape against the actual crate edges, then hand to developer ‖ ui-designer._

## Backtest Scenarios

N/A — this is a **read-only UI feature** over already-committed reports. It
runs no new strategy and produces no new backtest. Per CLAUDE.md, the
baseline-equity-divergence e2e gate applies to **strategy overlays / sizing
modifiers** — this is neither (no overlay, no sizing math, no decision
variable). The verification floor is the panel-snapshot suite + the four-state
+ no-panic tests + the unchanged regression-anchor gate (no anchored file is
touched — Reports only **reads** committed reports).

## Implementation

_developer ‖ ui-designer fill this._

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
