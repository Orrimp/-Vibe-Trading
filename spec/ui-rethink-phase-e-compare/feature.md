---
slug: ui-rethink-phase-e-compare
status: shipped
owner: operator
updated: 2026-05-20
version: 0.1.0
predecessor: ui-rethink-phase-d-trail-followup v0.1.1
---

# UI rethink Phase E — Compare matrix (J3)

> Fifth concrete feature carved out of
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/archive/2026-Q2/ui-rethink-2026-05-17.md).
> Dev-note §6 Phase E (lines 1082-1096) is the **scope source-of-truth**;
> this brief is the **implementation contract**. Predecessor:
> [`ui-rethink-phase-d-trail-followup v0.1.1`](../ui-rethink-phase-d-trail-followup/feature.md)
> shipped 2026-05-20 (with Phase D v0.1.0 trail surface already live on
> top of mig 011). The Phase C 3-group sidebar already reserves a
> `Compare` entry under the Work zone
> (`SIDEBAR_GROUPS_PHASE_C` in `crates/ui/src/theme.rs:741-742`,
> currently a Phase A placeholder routed through `placeholder::view`
> with `strings::COMPARE_PLACEHOLDER`); Phase E makes the entry
> meaningful.

## Why

The operator job-story **J3 — "Compare strategies across pairs"**
(dev-note §J3, lines 340-390) is the navigation hinge of the UI
rethink. With 6 strategies (v0.sma, v0.5.composed, v1.momentum,
v1.5a.pairs, v2.llm, v2.5.tcn) × ≤10 pairs each in the v1 universe,
the operator already asks two reciprocal questions every session:

1. **"Which strategy works best for this pair?"** — fix a column,
   scan the rows.
2. **"Which pair works best for this strategy?"** — fix a row,
   scan the columns.

The Lab screen (Phase A/B) answers either of these one cell at a
time — pick a (strategy, pair, range) tuple, click Run, read the
KPIs. The dev-note's §3 hybrid IA recommendation explicitly names
the **Compare matrix as the bidirectional bridge** (lines 681-686):
"Same screen, two mental models." Without a matrix surface the
operator must mentally hold a 6×10 grid by running 60 Labs in
sequence, comparing Sharpe by memory. That is exactly the bottleneck
the dev-note calls out at lines 342-345.

Phase E lands a **read-only matrix view that reads cached backtest
report frontmatter** (no live recompute on screen-open) and seeds
the Lab on cell-click — a purely additive UI surface with **zero
anchor risk** by construction (no strategy/audit/exec/report-renderer
code touched; the matrix consumes the same report files the
22-anchor regression gate already locks).

Key design tension Phase E resolves:

- **Q8 from the dev-note's first cut** ("recompute foreground or
  background?", line 1154) — operator-decided **background**
  2026-05-17 in the §1141 addendum. Phase E's MINIMAL shape ships
  with **no new orchestration at all** (Q2c below): the cache is
  read-only; recompute happens by clicking through to Lab and
  pressing Run, leveraging the Phase B in-process backtest runner.
  Background polling is an explicit non-goal at v0.1.0 to keep the
  anchor surface flat. Operator can lift this in v0.2.0 if practice
  warrants.
- The 6×10 matrix legibility constraint (dev-note line 1094, "are
  the KPI cells legible at 6×10?") drives the layout — single-KPI
  cells (Sharpe by default, per Q3a below) with a 30-bar sparkline
  beneath, mirroring the dev-note's §J3 sketch at lines 363-374.
- **Empty cell ergonomics** — if a (strategy, pair) tuple has no
  cached report, the cell renders a per-cell "Run" affordance
  (Q4b below) that dispatches the same `LabRun` round-trip as the
  Lab Run button. One-click drill-down from the matrix to a
  populated cell.

## Requirements

### R1 — `screens::compare::view` (new screen body)

**R1.1** — New file `crates/ui/src/screens/compare.rs` exposing
`pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>`.
Replaces the current `placeholder::view(strings::COMPARE_PLACEHOLDER,
mode)` body wired at `crates/ui/src/shell.rs:96`.

**R1.2** — `screens::compare::view` body composition (top to bottom):
1. **Toolbar row** — date-range picker (reuses
   `Cockpit::lab_state.range` via a NEW
   `compare_screen_state.range` field — see R3.4 for the range-
   isolation rationale) + KPI-axis dropdown (single KPI at v0.1.0
   per Q3a; the multi-KPI heatmap is Q3d follow-up scope).
2. **The matrix widget** — `widgets::matrix::view(model, mode)`
   (R2 below).
3. **(Reserved for v0.2.0)** "Recompute all missing" toolbar
   button — explicitly out of scope at v0.1.0 per Q2c
   (operator-decide).

**R1.3** — `Cockpit::current_screen == Screen::Compare` routes to
`screens::compare::view` (replaces the Phase A placeholder route
in `shell.rs:96`).

**R1.4** — Default state when matrix has zero populated cells (cold
boot, no cached reports for any (strategy, pair) cell) — the matrix
still renders with every cell showing the per-cell "Run" affordance
(Q4b default). No empty-screen modal; cold operators land on a
fully clickable surface.

### R2 — `widgets::matrix` (new widget)

**R2.1** — New file `crates/ui/src/widgets/matrix.rs` exposing
`pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>`.

**R2.2** — Matrix axis orientation = **strategies as rows, pairs
as columns** (Q1a default — the dev-note §J3 sketch at lines 362-
374 uses this orientation; matches the operator's "of my 6
strategies × my 10 pairs" framing at line 342). 6 strategy rows ×
≤10 pair columns; the actual pair count is `Cockpit::universe`
length (XRP-first per Phase A Q7 default).

**R2.3** — Cell shape (per dev-note §J3 lines 350-354):
- **Populated cell**: single KPI value (default = Sharpe, Q3a) with
  Lumen color treatment (UP_500 / DOWN_500 / WARN_500 per sign);
  30-bar sparkline of `equity_curve` (truncated tail) beneath the
  KPI; right-aligned hairline-bordered `Button` with the
  `active_row` border tint on hover. Click → `Message::LabSeedCell
  { strategy, pair, range }` (R4 below).
- **Empty cell** (no cached report for this `(strategy, pair, range)`
  tuple): a centred **"Run"** label inside a Tier 2 button with the
  `ACCENT_500` hairline (Q4b default). Click → same
  `LabSeedCell` round-trip + auto-trigger `LabRun` (decision
  deferred to architect: separate `Message::CompareCellRun` or
  re-use `LabSeedCell` + `LabRun` chain like Phase C's
  `OpenStrategyInLab` precedent).

**R2.4** — Row / column headers:
- **Row header** (left edge) — strategy `id` text label; click →
  the same Lab-seed compound dispatch with `pair = None` (operator
  picks a pair in Lab via the pair-chip — matches dev-note §3 line
  713 "Row header click → Lab with strategy locked, pair-chip
  pickable").
- **Column header** (top edge) — pair `Symbol` text label (e.g.
  "BTCUSDT"); click → Lab-seed compound dispatch with
  `strategy = None` (operator picks a strategy in Lab via the
  strategy-chip — matches dev-note §3 line 714).

**R2.5** — Cell layout invariants — every cell must satisfy the
`layout_invariants` proptest gate (cockpit-smoke proxy, see Phase D
§3.6): no zero-dimension panics across 256 proptest cases of viewport
sizing. The matrix grid uses iced's `Column<Row>` primitive (NOT a
new `grid` widget), reusing existing layout primitives — H2 (R2.5)
holds by construction.

**R2.6** — Cell hover state — Lumen `BORDER_HAIRLINE` →
`active_row` border tint on cell hover (same primitive used by the
Phase C strategy-card hover state at
`widgets/strategy_card.rs`). NO tooltip at v0.1.0; tooltip detail
view is a Lumen-design-adoption follow-up.

### R3 — Report-cache lookup (the read-only shortcut)

**R3.1** — New module `crates/ui/src/compare/cache.rs` (sibling of
`crates/ui/src/lab/`). Exposes
`pub fn lookup_cell(strategy_id: &StrategyId, symbol: &Symbol,
range: DateRange) -> Option<CachedCell>` where `CachedCell` carries
the KPI snapshot (Sharpe + total_return + max_drawdown +
trade_count) read from a backtest report's frontmatter.

**R3.2** — Cache walk path — at module init (cold boot) the cache
**scans `spec/<strategy>/reports/backtest-*.md`** glob, parses the
leading YAML frontmatter (per `spec/anchors.toml`'s body-stripping
convention), and indexes by
`(strategy.id, scenario_name → (universe, year_range))`. The
scenario→pair-list mapping uses the **scenario name convention**
(e.g. `top10-2023-1h-momentum` → universe = top10 (10 pairs), year
= 2023, freq = 1h; `btc-2023-1m-sma-cross` → universe = BTCUSDT
only). Single-pair scenarios produce 1 cell; multi-symbol scenarios
produce ≤10 cells with identical aggregate KPIs (the report is
universe-aggregate, not per-pair — see Q6 below for the
per-pair-decomposition deferral).

**R3.3** — Cache hit semantics — `lookup_cell(...)` returns the
**most-recent** report for the matching tuple (by frontmatter
`generated:` timestamp); older reports are reachable from Trail
(Phase D) but not surfaced in the matrix.

**R3.4** — Range isolation — `Cockpit::compare_screen_state.range`
is a **separate field** from `Cockpit::lab_state.range`. Toggling
the Compare date-range picker MUST NOT mutate Lab state (and vice
versa). Cell-click R4 below copies the Compare-screen range into
Lab on seed.

**R3.5** — Cache invalidation — at v0.1.0, the cache is **cold-
boot-only**; running a backtest in Lab does NOT refresh the matrix
in-session. Operator must restart the cockpit to see the new cell
populated, OR navigate back to Compare and click the cell (which
re-reads via `lookup_cell` on view-render — cheap glob + parse,
H1 below quantifies). v0.2.0 adds an in-session subscription bridge
(Lab Run completion → matrix re-index for that single cell) but
that's out of scope here.

**R3.6** — No new external deps for YAML parsing — re-use
`serde_yaml` if already in workspace, else hand-parse the simple
key:value frontmatter (no nested structures — every existing report's
frontmatter is flat). Architect M-T1 to confirm the parser shape
(K3 below).

### R4 — Cell-click → Lab seeded

**R4.1** — New public `Message` variant `Message::OpenLabFromCompare {
strategy: StrategyId, pair: Option<(Venue, Symbol)>, range: DateRange }`
in `crates/ui/src/state.rs`. The compound dispatch expands to
`SwitchScreen(Screen::Lab)` + `SelectStrategy(strategy)` + (when
`pair` is `Some`) `LabSelectPair(venue, symbol)` + `LabRangeSelected(range)`.
Mirrors the Phase C `OpenStrategyInLab` precedent
(`state.rs:1376-1377` area) and the Phase D `OpenTrailFor` precedent.

**R4.2** — Cell click on a **populated cell** emits
`OpenLabFromCompare { strategy, pair: Some(..), range }` and DOES
NOT auto-trigger a Lab Run. The operator sees the seeded Lab with
the cached report already rendered (Phase A's `last_run_report`
shortcut already handles this when the tuple matches a cache hit).

**R4.3** — Cell click on an **empty cell** emits
`OpenLabFromCompare { strategy, pair: Some(..), range }` followed
by an auto-`LabRun` dispatch (Q4b — directly actionable). Architect
M-T1 to decide whether this is one compound message or two
sequential dispatches; the existing Phase A `LabRun` dispatch
pattern is synchronous so either shape works.

**R4.4** — Row-header click → `OpenLabFromCompare { strategy,
pair: None, range }` (strategy locked, pair-chip pickable).
Column-header click → there is no
`OpenLabFromCompare { strategy: None, pair: Some(..) }` because Lab
requires a strategy to function — instead the column-header click
**filters the matrix** to that single column (Q5b sub-decision —
defer to architect; the dev-note §3 line 714 framing ("Lab with
pair locked, strategy-chip pickable") implies a separate
"pair-first Lab" mode that we don't ship at v0.1.0). For v0.1.0
the column header is **non-interactive** (label only) and the
operator drills down via cell click.

### R5 — Sidebar wiring (already done in Phase C)

**R5.1** — `Screen::Compare` is already in
`SIDEBAR_GROUPS_PHASE_C[0][2]` (`crates/ui/src/theme.rs:742`).
The Phase A `placeholder::view` body at `shell.rs:96` is replaced
with `screens::compare::view` (R1.3). **No sidebar change**.

**R5.2** — String constants: `strings::COMPARE_PLACEHOLDER`
(`crates/ui/src/strings.rs:252`) becomes dead-code at Phase E ship
(architect/developer decision: deprecate-with-attribute for one
cycle or delete in lockstep; analyst recommends delete-in-lockstep
since no test harness references it — confirm in M-T1 grep).
`SIDEBAR_NAV_COMPARE` (`strings.rs:266`) stays as-is.

**R5.3** — Lab → Compare entry point — **no new button** (Q5a
default; the sidebar entry IS the entry point per dev-note §3 IA).
A "Compare" button inside Lab is Q5c (operator-decide) follow-up
scope.

### R6 — State plumbing

**R6.1** — New struct
`crates/ui/src/compare/state.rs::CompareScreenState`:
```rust
pub struct CompareScreenState {
    pub range: DateRange,                      // R3.4 isolation
    pub kpi_axis: CompareKpiAxis,              // R1.2 dropdown
    pub cache: HashMap<(StrategyId, Symbol, DateRange), CachedCell>,  // R3.1
    pub last_indexed: Option<chrono::DateTime<Utc>>,  // R3.5 cold-boot tag
}
```
`Cockpit::compare_screen_state: CompareScreenState` added at the
`state.rs:798` area (sibling of `lab_state`).

**R6.2** — Default values: `range = DateRange::Last90Days` (per
dev-note §J3 sketch line 365 "[Last 90d ▾]"); `kpi_axis =
CompareKpiAxis::Sharpe` (Q3a); `cache` empty until first view-render
(R3.2 cold-boot scan).

**R6.3** — `CompareKpiAxis` enum with variants `{ Sharpe,
Sortino, TotalReturn, MaxDrawdown, WinRate }` per dev-note §J3 line
359-360. v0.1.0 wires only `Sharpe` (the dropdown renders all 5
but selecting non-Sharpe falls back to Sharpe with a `tracing::warn!`
in dev builds — architect/developer to refine). Q3d (multi-KPI
heatmap) is v0.2.0 follow-up.

**R6.4** — Range options — same set as Lab
(`Last7Days / Last30Days / Last90Days / Last365Days / All`).
No new `DateRange` variant needed.

### R7 — Non-regression contract

**R7.1** — **22 body-SHA-256 anchors stay byte-identical**. Phase
E touches no strategy/audit/exec/report-renderer path; the matrix
consumes reports it does not generate.

**R7.2** — **Phase A/B/C/D-shipped surfaces byte-identical** —
specifically:
- Lab screen body (Phase A + B) — only adds the path to receive
  `OpenLabFromCompare` (a sibling of existing handlers); no Lab body
  render change.
- Live screen (Phase C) — unchanged.
- Strategy registry (Phase C) — unchanged.
- Settings (Phase C) — unchanged.
- Trail screen (Phase D) — unchanged.
- Sidebar (Phase C) — Compare entry already wired; only the body
  route swaps.

**R7.3** — **`cockpit-smoke` PASS 0 panics** — 6×10 matrix render
under the layout-invariants proptest (R2.5).

**R7.4** — **`cockpit-performance v1.0.0` idle-CPU floor ≤ 13.6 %**
preserved (Phase D+ baseline: 13.1 % floor + 0.5 % headroom).
Matrix render is on-demand (only when Compare screen active),
NOT a periodic widget — H3 hypothesis below quantifies. Cold-boot
cache scan (R3.2) happens once at Compare-screen first-open per
session; H1 quantifies the budget.

**R7.5** — **`spec-lint` Phase E contribution = 0** — baseline
from Phase D+ is 87 / 2 categories (or 91 / 2 if the
`trace.toml`-validator artefact persists into Phase E); Phase E
adds no new dead-link rows and no new trace-broken-path rows.

**R7.6** — **No new external crate deps; no new Lumen tokens; no
iced bump.** Vendored `iced_tiny_skia` fork stays untouched.

**R7.7** — **No backtest binary changes; no anchored renderer
touch.** The matrix reads existing report files; it does not write
report files, and it does not invoke the backtest engine directly
(invocation flows through Lab Run, which is Phase B contract).

### R8 — Public API surface added

**R8.1** — One new public `Message` variant
(`Message::OpenLabFromCompare`) — same precedent as Phase C
(`SwitchSettingsTab`) and Phase D (`OpenTrailFor`).

**R8.2** — One new enum (`CompareKpiAxis`) — pure data.

**R8.3** — One new struct (`CompareScreenState`) — pure data,
default-constructible.

**R8.4** — One new module (`crates/ui/src/compare/`) with
`state.rs` + `cache.rs` siblings (mirrors `crates/ui/src/lab/`
layout).

**R8.5** — Net-new file count: 4-5 (1 screen, 1 widget, 1-2 module
files, optional 1 helper). Architect M-T1 to lock exact count.

## Q-questions (operator-decide)

### Q1 — Matrix axis orientation

(a) **Strategies as rows, pairs as columns** — matches dev-note
    §J3 sketch (lines 362-374) and the operator's "6 strategies ×
    10 pairs" framing (line 342).
(b) Flipped (pairs as rows, strategies as columns).

**Analyst-recommended: (a)**. Rationale: there are typically more
pairs than strategies (10 vs 6); strategies in rows means the matrix
is wider than tall, which suits a horizontal cockpit; the dev-note
sketch already commits to this orientation.

### Q2 — Recompute orchestration cadence

(a) On-demand button + spinner — "Recompute all missing" in the
    toolbar; background task; spinner per cell.
(b) Background poll every N minutes — invisible refresh; KPI cells
    update silently.
(c) **Report-cache only with manual recompute via Lab** — no new
    orchestration; cell-click on empty cell seeds Lab + Runs
    (R4.3 above); seeded report → next Compare-screen open reads
    it from cache (R3.5).

**Analyst-recommended: (c)**. Rationale: simplest, no new
orchestration code, leverages existing Phase B Lab Run, anchor
risk zero. The dev-note's Q8-resolved-background framing (line
1154) is about **what cadence the recompute orchestration runs at
if we ship one**; (c) is "ship none at v0.1.0, lift in v0.2.0 if
practice warrants". Surface (a) / (b) for operator to override if
the no-orchestration framing feels too sparse on first sight.

### Q3 — Cell KPI to display

(a) **Sharpe** — single number, matches Lab Run's anchor metric
    and `BacktestKpis::sharpe` field.
(b) Total return — bigger numbers, easier to read at a glance but
    period-sensitive.
(c) Max drawdown — risk-first framing; useful but inverted (lower
    is better).
(d) Heatmap of multiple KPIs per cell — Sharpe color + drawdown
    color + return color; 3-stripe cell.

**Analyst-recommended: (a)**. Rationale: matches dev-note §J3
sketch (line 353 "Sharpe value, colored by sign"), matches Lab
Run anchor, single number per cell keeps 6×10 legible (H2 below).
The KPI-axis dropdown (R6.3) reserves the OPTION to switch later
without re-architecting; v0.1.0 wires Sharpe only.

### Q4 — Empty cell behavior

(a) `—` placeholder — passive; operator must navigate to Lab
    manually.
(b) **"Run" affordance per-cell** — directly actionable; cell
    click runs that single tuple via Lab Run round-trip (R4.3).
(c) "Run all missing" toolbar button — batch dispatch; ties to Q2(a).

**Analyst-recommended: (b)**. Rationale: directly actionable from
the matrix; reuses Phase B's Lab Run dispatch (no new code path);
keeps cold-boot first-impression hands-on instead of paralysed.
v0.2.0 can add (c) on top.

### Q5 — Compare screen entry point

(a) **Sidebar zone** — already reserved by Phase C IA
    (`SIDEBAR_GROUPS_PHASE_C` Work zone).
(b) Lab → Compare button — a "Compare strategies" affordance inside
    the Lab screen.
(c) Both — sidebar + Lab button.

**Analyst-recommended: (a)**. Rationale: sidebar entry already
exists; adding (b) is a Lab-body change which costs Phase A/B
surface-stability (R7.2). v0.2.0 can add (b) on top if the
operator's workflow finds sidebar-only friction.

### Q6 — Per-pair decomposition of multi-symbol scenarios (research gap)

The existing backtest reports for multi-symbol scenarios
(e.g. `top10-2023-1h-momentum`) carry **universe-aggregate KPIs**
(see `spec/v1-cross-sectional-momentum/reports/backtest-20260429-195148-top10-2023-1h-momentum.md`
frontmatter — Total return / Sharpe are over the full 10-symbol
universe, not per-pair). The matrix at v0.1.0 must show 10 cells
for v1.momentum × {ADA, AVAX, BNB, BTC, DOGE, DOT, ETH, LINK, SOL,
XRP}, but the cached report only has ONE aggregate KPI.

Options:

(a) **Render all 10 cells with the same aggregate KPI** — honest
    about the data shape; operator sees "v1 momentum scores 0.94
    aggregate Sharpe across this universe" repeated in every cell.
(b) **Render the strategy's row as a single merged cell** —
    visually represents the universe-aggregate honestly; breaks the
    matrix grid.
(c) **Per-pair backtest decomposition** — backtest engine emits
    per-pair P&L (a new emit channel); matrix shows true per-pair
    Sharpe. **NEW WORK** in `crates/backtest` — anchor-risky if it
    touches the report renderer.
(d) **Skip multi-symbol strategies in the matrix at v0.1.0** —
    show only single-pair strategies (v0.sma BTC, v0.5.composed BTC,
    v1.5a.pairs (ETH, BTC) merged). 4 rows × 1-2 pair cells.
    Honest about the limitation; matrix shrinks dramatically.

**Analyst-recommended: (a)** for v0.1.0 + **honest tooltip** "this
KPI is universe-aggregate, not per-pair (see Q6 follow-up)". Reason:
(c) is anchor-risky (backtest engine change); (d) shrinks the
matrix to ~4 cells which defeats the purpose; (b) breaks the grid.
Surface for operator-decide because the matrix UX is materially
different depending on the choice. v0.2.0 work item: ship per-pair
decomposition as a `crates/backtest` follow-up (`v25-tcn-per-pair-decomp`
sketch), then matrix becomes truly per-pair.

### Q7 — Strategy enumeration source

(a) **`Cockpit::strategies_config.strategies`** — the existing
    `Vec<StrategyConfigEntry>` (`state.rs:452`) populated from
    `config/strategies/*.toml`. Strategies that aren't registered
    in the config are not in the matrix.
(b) Filesystem scan of `spec/<strategy-version>/reports/` —
    discover strategies by which folders have backtest reports.
(c) Hardcoded `SIDEBAR_NAV_COMPARE`-adjacent registry of 6 strategy
    IDs.

**Analyst-recommended: (a)**. Rationale: single source of truth;
the strategy registry (Phase C) already enumerates these; if a
strategy doesn't exist in the registry it shouldn't appear in the
matrix. Architect M-T1 to confirm the StrategyId list at run time
matches dev-note §J3's enumerated `v0.sma / v0.5.composed / v1.momentum
/ v1.5a.pairs / v2.llm / v2.5.tcn` set.

### Q8 — Pair enumeration source

(a) **`Cockpit::universe` (XRP-first, Phase A Q7 default)** —
    matches the Lab pair-chip's universe.
(b) Per-strategy universe (each row shows only the pairs in that
    strategy's universe; cells outside the universe are blanked
    grey).
(c) Union of all strategies' universes.

**Analyst-recommended: (b)**. Rationale: honest about which cells
are even *legal* (v1.5a.pairs only runs on (ETH, BTC); rendering 10
empty cells for it would mislead the operator). Implementation:
strategy's `StrategyConfigEntry.params` carries the universe, OR
the strategy registry exposes a `universe()` getter. Architect
M-T1 to confirm. Blanked cells render as the Lumen `BORDER_HAIRLINE`
with a centred `—` label (passive — distinguishable from Q4b's
active "Run" affordance).

## K-risk register

### K1 — Matrix legibility at 6×10
**Risk:** 60 cells × ~80×60 px each = a 480×600 px grid + axis
labels. On the operator's typical screen (≥1440×900) this fits with
headroom; on a smaller laptop or in a split-window layout it may
require horizontal scroll.
**Severity:** LOW.
**Mitigation:** Layout-invariants proptest (R2.5) catches the
zero-dim edge cases; H2 below is the legibility hypothesis tester
can falsify by inspection. Fallback: a "compact mode" toggle in
the toolbar that drops the sparkline (saves ~30 px per row), Q-Future
follow-up if H2 falsified.

### K2 — Report-cache parser brittleness
**Risk:** The report frontmatter is hand-written YAML by the
backtest binary's report renderer; if any report has malformed
frontmatter the parser crashes and the entire matrix is empty.
**Severity:** LOW.
**Mitigation:** `lookup_cell` returns `None` on parse failure
(graceful empty cell, not panic); architect M-T1 to specify the
fail-soft contract. Add a `tracing::warn!` per malformed file so
the operator sees the offending path in logs. Existing reports
verified well-formed by spec-lint already.

### K3 — `serde_yaml` dependency status
**Risk:** If `serde_yaml` is not in the workspace, R3.6 requires
either adding a new external dep (violates R7.6) or hand-parsing
flat YAML.
**Severity:** LOW.
**Mitigation:** Architect M-T1 to grep
`cargo metadata --format-version 1 | grep serde_yaml` — if missing,
the hand-parser is ~20 LOC (every existing report's frontmatter is
flat key:value with at most one nested `strategy:` block). Hand
parser is the recommended path under R7.6.

### K4 — `OpenLabFromCompare` compound dispatch ordering
**Risk:** The compound message expands to `SwitchScreen + SelectStrategy
+ LabSelectPair + LabRangeSelected`. If `SelectStrategy` resets
`last_run_report` before `LabSelectPair` reads it, the seeded Lab
shows a blank result panel instead of the cached one.
**Severity:** LOW.
**Mitigation:** Identical pattern to Phase C's `OpenStrategyInLab`
and Phase D's `OpenTrailFor` — both proven by round-trip tests.
Phase E adds analogous tests:
`open_lab_from_compare_sets_lab_strategy_pair_and_range`. Synchronous
iced dispatch makes the compound atomic per-frame.

### K5 — Cache cold-boot cost
**Risk:** Globbing `spec/**/reports/backtest-*.md` and parsing N
frontmatters on Compare-screen first-open could stall the cockpit
for >100 ms if N is large.
**Severity:** LOW.
**Mitigation:** At 2026-05-20 N is ~80 backtest reports across the
spec tree (find indicates similar order); flat YAML parse + glob
scan is ~5-15 ms (H1 below quantifies). If H1 falsifies the budget,
move the cache scan to a background `tokio::spawn` at cockpit boot
(cheap, deterministic, no UI gating).

### K6 — Compare-range / Lab-range divergence (R3.4 confusion)
**Risk:** Operator toggles Compare range to "Last 7d", clicks a
cell, lands in Lab with the 7d range pre-filled — but then changes
Lab's range to 90d, runs, navigates back to Compare. Compare still
shows 7d data; the operator thinks the matrix is stale.
**Severity:** MEDIUM (subtle UX trap).
**Mitigation:** R3.4 explicitly isolates the two ranges. Q-Future
visual treatment: a small "(Compare range: 7d)" subtitle next to
the matrix toolbar so the operator always sees which range the
cells reflect. Surfaced honestly here for operator review at
M-FINAL.

### K7 — Q6 universe-aggregate vs. per-pair semantic confusion
**Risk:** The matrix shows v1.momentum × BTCUSDT = 0.94 Sharpe AND
v1.momentum × ETHUSDT = 0.94 Sharpe (because the underlying report
is universe-aggregate). The operator misreads this as "v1 momentum
scores 0.94 specifically on BTC AND 0.94 specifically on ETH" when
the reality is "v1 momentum scores 0.94 aggregate over the full
top10 universe" — a different statement.
**Severity:** MEDIUM (data-shape trap, not a code defect).
**Mitigation:** Q6 above is operator-decide; analyst-recommends
(a) plus a per-cell tooltip / footnote disambiguation. v0.2.0
ships per-pair decomposition (`v25-tcn-per-pair-decomp` follow-up).

### K8 — Q8 universe-aware cell blanking
**Risk:** v1.5a.pairs runs on (ETH, BTC) only. If Q8 default (b)
goes in and the matrix shows 8 blanked-grey cells for v1.5a.pairs,
the operator's eye reads "v1.5a.pairs is broken on 8 pairs" when
the truth is "v1.5a.pairs doesn't run on 8 pairs".
**Severity:** LOW.
**Mitigation:** Blanked cells render with a centred `—` and the
hairline border (passive, not error-flagged). Architect M-T1 to
confirm the visual distinction from Q4b's active "Run" affordance.
Optional: a "not in this strategy's universe" tooltip on hover.

## H-hypothesis register

### H1 — Report-cache hit-rate ≥ 30 % at first matrix open
**Claim:** On a freshly cloned repo at 2026-05-20 commit
`6e5b884`, the matrix at first-open hits the cache for ≥ 30 % of
cells (6 strategies × 10 pairs = 60 cells; ≥ 18 cells populated
from the existing report tree).
**Falsification:** Architect M-T1 enumerates the report tree;
counts (strategy, scenario) tuples in `spec/<strategy>/reports/`;
maps to (strategy, pair) cells via the scenario→universe mapping.
If < 18 cells populate at first-open, H1 is falsified and Q4b's
"Run" affordance dominates the cold-boot experience (acceptable
but worth knowing for operator messaging).
**Why this number:** the 22-anchor regression gate touches 22
scenarios over 6 strategy versions, plus the v1.momentum + v1.5a
+ tcn-overlay reports total ~80 backtest reports. A ~30 % hit-rate
is the "this surface is immediately useful" threshold; lower means
the matrix is mostly a "Run this cell" launchpad.

### H2 — Matrix layout legibility at 6×10
**Claim:** A 6-row × 10-column matrix with per-cell Sharpe (1
number) + 30-bar sparkline + hairline border renders as legible
(no overflow, no cell <40×60 px) on viewports ≥ 1280×720.
**Falsification:** Layout-invariants proptest (R2.5) catches zero-
dim panics; the legibility judgment is subjective and falls to
operator review at presenter deck. If the operator says "I can't
read the cells", H2 is falsified and the compact-mode fallback
(K1 mitigation) lifts.

### H3 — Idle-CPU floor preserved
**Claim:** Compare screen render is **on-demand** (no new
periodic widget, no new subscription); when Compare is not the
active screen, the matrix consumes zero CPU. Idle CPU floor stays
≤ 13.6 % (Phase D+ baseline).
**Falsification:** Tester runs cockpit-performance v1.0.0 with
Phase E applied and the Compare screen as the active screen for
60 s; if idle CPU > 14.6 % (13.6 % + 1 % budget for active matrix
render at 10 fps via ThrottledSpinner) H3 is falsified and the
matrix render path needs throttling. Static argument: no new
`tokio::time::interval`, no new subscription producer; the matrix
re-renders only on `Message` arrival (range change, KPI dropdown,
cell click) — same model as Phase C Live screen which already
hit the budget.

### H4 — Cold-boot cache scan budget < 50 ms p99
**Claim:** Globbing `spec/**/reports/backtest-*.md` + parsing each
file's frontmatter completes in < 50 ms p99 on the operator's
typical workstation at 2026-05-20 scale (~80 reports).
**Falsification:** Architect M-T1 micro-bench runs the scan path
against the live `spec/` tree; if p99 > 50 ms, K5 mitigation
(background `tokio::spawn` at cockpit boot) lifts. Acceptable
fallback — no UI gating either way.

### H5 — `OpenLabFromCompare` round-trip atomic
**Claim:** A single `OpenLabFromCompare { strategy, pair, range }`
message dispatch leaves the cockpit with `current_screen == Lab`,
`lab_state.strategy == Some(strategy)`, `lab_state.pair == Some(pair)`,
and `lab_state.range == range` after one `update()` tick.
**Falsification:** Unit test
`open_lab_from_compare_sets_lab_strategy_pair_and_range` (R8.1
test path) — failing assertion falsifies H5; identical pattern to
Phase C/D round-trip tests already established.

## Non-regression contract

1. **22 body-SHA-256 anchors stay byte-identical** (R7.1).
2. **Phase A/B/C/D-shipped surfaces byte-identical** (R7.2) —
   Lab body, Live, Strategy registry, Settings, Trail, sidebar 3-zone
   grouping all unchanged in body content.
3. **`cockpit-smoke` PASS 0 panics** under the new Compare screen
   active (R7.3).
4. **`cockpit-performance v1.0.0` idle-CPU floor ≤ 13.6 %** preserved
   (R7.4, H3).
5. **`spec-lint` Phase E contribution = 0** (R7.5).
6. **No new external crate deps; no new Lumen tokens; no iced bump**
   (R7.6). Hand-parse YAML if `serde_yaml` is missing (K3
   mitigation).
7. **No backtest binary changes; no anchored renderer touch** (R7.7).
8. **Backtest determinism preserved** — Phase E does not invoke the
   engine directly; Lab Run round-trip (Phase B contract) handles
   any recompute under R4.3.

## Acceptance criteria

### M0 — Analyst synthesis (this pass)
- [x] R1..R8 anchored to dev-note §6 Phase E + §J3 + §3 IA scope.
- [x] Q1-Q8 surfaced with analyst-recommended defaults.
- [x] K1-K8 risk register; K6 (range divergence) + K7 (universe-
      aggregate semantic) surfaced as the load-bearing UX traps.
- [x] H1-H5 falsifiable hypotheses.
- [x] Non-regression contract enumerated (8 items).
- [x] Trace row `REQ-UI-RETHINK-PHASE-E-001` to be opened in
      `draft` state by this pass.

### M-OD — Operator-decide (Q1-Q8)
- [ ] Q1 — axis orientation (analyst-recommended: a).
- [ ] Q2 — recompute cadence (analyst-recommended: c).
- [ ] Q3 — cell KPI (analyst-recommended: a).
- [ ] Q4 — empty cell behavior (analyst-recommended: b).
- [ ] Q5 — entry point (analyst-recommended: a).
- [ ] Q6 — multi-symbol universe-aggregate semantic
      (analyst-recommended: a + tooltip, ship per-pair-decomp in v0.2.0).
- [ ] Q7 — strategy enumeration source (analyst-recommended: a).
- [ ] Q8 — pair enumeration source (analyst-recommended: b — universe
      gating).

### M-T1 — Architect decomposition
- [ ] Architect resolves K3 (`serde_yaml` presence/absence) and
      locks the parser shape.
- [ ] Architect runs H1 enumeration: count cache-hit cells at
      first-open against the live `spec/` tree; record in `decomp.md`.
- [ ] Architect runs H4 micro-bench: cache scan p99; record in
      `decomp.md`.
- [ ] Architect decomposes R1-R8 into ordered T-D-N tasks per wave
      (suggested wave map: A = cache module; B = state plumbing;
      C = widgets::matrix; D = screens::compare + shell wiring;
      E = OpenLabFromCompare + Lab seed; F = snapshot baselines +
      cockpit-smoke; G = spec-lint sweep).
- [ ] Architect confirms net-new file count (R8.5).
- [ ] Architect closes Q6 sub-decision on universe-aggregate tooltip
      vs. footnote shape.

### M-FINAL — Tester sweep
- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D
      warnings` exit 0.
- [ ] `cargo test --workspace --lib` 100 % PASS.
- [ ] New snapshot baselines: `compare__cold_boot_all_empty`,
      `compare__steady_state_populated`,
      `compare__empty_cell_run_affordance`,
      `compare__column_header_hover`.
- [ ] `scripts/verify_anchors.sh` → 22/22 PASS — non-negotiable
      (R7.1).
- [ ] `cockpit-smoke` → 0 panic lines on Compare screen active
      (R7.3).
- [ ] Cockpit-performance v1.0.0 idle-CPU floor ≤ 13.6 % preserved
      under Compare screen active (R7.4, H3).
- [ ] Round-trip test
      `open_lab_from_compare_sets_lab_strategy_pair_and_range` PASS
      (H5).
- [ ] H1 cache-hit enumeration recorded in test report.
- [ ] H4 cold-boot cache scan benchmark p99 recorded in test
      report.
- [ ] Author
      `spec/ui-rethink-phase-e-compare/reports/test-final-<YYYY-MM-DD>.md`.

## Cost estimate

Per dev-note §6 Phase E (line 1096): **~2-3 weeks**. No cliffs;
independently shippable; independently reversible (revert the
`screens::compare::view` body + `Cockpit::compare_screen_state`
field; the placeholder route + sidebar reservation remain).

Anchor risk: **zero** (purely additive UI surface; no backtest
binary changes, no anchored renderer touch). 22-anchor regression
gate carry-forward H2 from Phase D+ predecessor.

## Trace

Trace row `REQ-UI-RETHINK-PHASE-E-001` to be opened in `draft`
state by this analyst pass. `arch`, `crates`, `tests`, `anchors`
columns to be filled by architect / developer / tester respectively.

## Implementation

Developer pass completed 2026-05-20. All 18 T-D-N rows ticked N1-N17
(N18 = tester handoff envelope); tester verifies at M-FINAL.

### Net-new files (5)

| File | Role |
|------|------|
| `crates/ui/src/compare/mod.rs` | Module root — re-exports `cache` + `state` |
| `crates/ui/src/compare/state.rs` | `CompareScreenState`, `CachedCell`, `CompareKpiAxis` |
| `crates/ui/src/compare/cache.rs` | `scan_spec_tree`, `lookup_cell`, `parse_frontmatter` + 5 unit tests |
| `crates/ui/src/widgets/matrix.rs` | `pub fn view(model, mode) -> Element` — Column×Row matrix widget |
| `crates/ui/src/screens/compare.rs` | `pub fn view(model, mode) -> Element` — toolbar + matrix body |

### Modified files (7)

| File | Change |
|------|--------|
| `crates/ui/src/lib.rs` | `pub mod compare;` |
| `crates/ui/src/lab/state.rs` | `PartialOrd, Ord` derives on `Preset` + `DateRange` (BTreeMap key req) |
| `crates/ui/src/state.rs` | `compare_screen_state` field on `Cockpit`; 3 Message variants; 3 update arms; 2 H5 tests |
| `crates/ui/src/strings.rs` | 5 new Phase E constants; `COMPARE_PLACEHOLDER` deprecated |
| `crates/ui/src/widgets/mod.rs` | `pub mod matrix;` |
| `crates/ui/src/screens/mod.rs` | `pub mod compare;` |
| `crates/ui/src/shell.rs` | `Screen::Compare` arm → `compare::view` (1-line swap) |

### Test files (3 new + 2 modified)

| File | Change |
|------|--------|
| `crates/ui/tests/fixtures/mod.rs` | 4 new compare cockpit fixture builders |
| `crates/ui/tests/visual_snapshots.rs` | `COMPARE_SLOTS`, `run_compare_slot()`, 4 `#[test]` fns |
| `crates/ui/tests/layout_invariants.rs` | `compare_screen_no_zero_dim` proptest (256 cases) + `build_compare_cockpit()` helper; pre-existing deprecated-Screen-variant lints fixed |

### Visual baselines (4 new PNGs)

| Baseline | Size | Fixture |
|----------|------|---------|
| `compare__cold_boot_all_empty.png` | 84,356 B | 2 strategies, empty cache — all Run affordances |
| `compare__steady_state_populated.png` | 109,613 B | 5 strategies, 24 cells populated, K7 subtitle visible |
| `compare__empty_cell_run_affordance.png` | 94,390 B | 2 strategies, 7/12 cells populated, 4 Run affordance cells |
| `compare__column_header_hover.png` | 84,356 B | Non-interactive headers confirmed (same as cold_boot) |

### Anchor gate

`scripts/verify_anchors.sh` → `ANCHORS PASS  (22 / 22)` post-implementation.
Phase E is purely additive UI surface; no anchored renderer touched (R7.7).

### Deviations from architecture

None. The implementation follows `decomp.md` Wave A-E exactly. The only
minor deviation: `build_kpi_chips` returns `Element<'static, Message>` (not
`'_`) because it has no borrowed arguments — the compiler required this
since v0.1.0 of the widget; the lifetime is technically more lenient than
`'_` and is fully correct per Rust lifetime elision rules.

## Changelog

- 2026-05-20 (developer): implementation complete — 18 T-D-N rows N1-N17
  ticked; 4 visual baselines written; 22 anchors verified; HANDOFF → tester.
- 2026-05-20 (analyst): initial brief — R1-R8, Q1-Q8, K1-K8, H1-H5,
  non-regression contract; predecessor
  `ui-rethink-phase-d-trail-followup v0.1.1`; scope anchored to
  dev-note §6 Phase E (lines 1082-1096) + §J3 (lines 340-390) + §3
  IA (lines 651-744). HANDOFF → operator-decide (Q1-Q8) → architect
  for M-T1 decomposition.
