---
slug: cockpit-buildout-audit-2026-06-08
status: draft
owner: ui-designer
updated: 2026-06-08
---

# Cockpit build-out audit + ranked candidates — 2026-06-08

> **Scoping pass, not implementation.** The research program concluded
> (active ≤ passive; passive baseline shipped + operationalized). The
> operator chose to build out the cockpit / UI next, but both queued UI
> items are gated (Lumen Phase 6 Assistant depended on the now-moot v2
> LLM *strategy*; cockpit cross-platform waits on external demand). This
> note audits the current `ui` crate and proposes ungated, grounded
> build-out candidates. The operator picks #1 (or greenlights another),
> then the normal analyst → architect → (developer ‖ ui-designer) →
> tester pipeline implements it.
>
> Authored by the ui-designer. Every observation below is grounded in
> the actual code, cited by path:line.

---

## 1. Cockpit audit — current surfaces

The `ui` crate is **large and mature** — far past the "early cockpit"
framing. It ships **four binaries** and a screen-routed shell.

### 1.1 Binaries

| Binary | Path | Purpose | State |
|--------|------|---------|-------|
| `cockpit_live` | `crates/ui/src/bin/cockpit_live.rs` | Unified agent + iced cockpit (default operator entry; `--features live`) | Shipped |
| `cockpit` | `crates/ui/src/bin/cockpit.rs` | Fixtures-only smoke cockpit (`--features fixtures`) | Shipped; builds clean (59 s, exit 0, 1 intentional deprecation warning) |
| `viewer` | `crates/ui/src/bin/viewer.rs` | Read-only offline backtest-report viewer (CLI-arg-driven) | Shipped (Phase 4) |
| `ui-gallery` | `crates/ui/src/bin/ui_gallery.rs` | Widget gallery for visual regression | Shipped |

### 1.2 Shell + screens (screen-routed IA)

`shell::view` (`crates/ui/src/shell.rs`) composes
`Row[sidebar | (body + status_bar) | right-rail]` wrapped in a toast-tray
`Stack`. Sidebar IA (`theme.rs` `SIDEBAR_GROUPS_PHASE_C:769`) groups eight
screens:

- **Work:** Lab, Live, Compare
- **Library:** Strategies, Memory, Models, Trail
- **Chrome:** Settings

Screen body states (`screens/`):

| Screen | Module | State / human-friendliness note |
|--------|--------|--------------------------------|
| **Lab** | `screens/lab.rs` (56 KB) | **Richest surface.** Pair/strategy/date chips → status strip → chart canvas → position curve → volume histogram. In-process backtest **Run** button (`widgets/run_button.rs`), run-vs-run **delta badge** (`widgets/run_delta_badge.rs`), training panel. This is the de-facto "research surface" the operator actually drives. |
| **Live** | `screens/live.rs` | System-health strip + equity curve + KPI strip + positions/agent-feed. **GAP: the equity curve and KPI strip are hard-wired to `PanelState::Loading` (`live.rs:58` and `live.rs:66`)** — they render the "no data" empty state *permanently*. Comment says "No live feed yet… real wiring in Phase F." |
| **Compare** | `screens/compare.rs` | Matrix of cached strategy KPIs across date ranges + KPI axes. Toolbar wired; only the Sharpe axis is fully live (R6.3). |
| **Strategies** | `screens/strategy_registry.rs` | List-of-cards registry. |
| **Memory** | `screens/memory.rs` | Lesson-card list + side-drawer. |
| **Models** | `screens/models.rs` | Checkpoint registry, reads committed `*.metadata.json` (`models/registry_read.rs`). |
| **Trail** | `screens/trail.rs` | Audit-ledger journal (list mode) + upstream node stack (trail mode). Click-through to decision trail. |
| **Settings** | `screens/settings.rs` | Three-tab rollup (Risk / Control / Debug). |

### 1.3 Right-rail Assistant slot — wired but unreachable

`assistant/` is **structurally complete**: open/close state
(`AssistantState`), three modes (`Offline` / `ReasoningTrace` / `Live`),
a full reasoning-trace render with cited-lesson lookups (`assistant/view.rs`).
But:

- `is_open` defaults to `false` (`assistant/state.rs:121`); the shell
  collapses the rail to `Length::Fixed(0.0)` when closed (`shell.rs:61-65`).
- **There is no UI affordance anywhere that flips `is_open`.** I grepped:
  no toggle button in the sidebar, status bar, or any screen. The slot
  can only open via a programmatic state mutation that nothing emits.
- `ReasoningTrace` mode is gated on the **now-moot v2/v3 LLM strategy**
  (`assistant/state.rs:8-16`). With the strategy dead, the only reachable
  mode is `Offline` ("Assistant offline" placeholder) — and even that is
  unreachable because nothing opens the rail.

**Net:** the Assistant slot is dead weight today — a fully-built panel
with no door handle and no live data source.

### 1.4 Widgets + design system

48 widgets under `widgets/`. Relevant to the candidates:
`equity_curve.rs`, `drawdown_band.rs`, `kpi_strip.rs`, `run_delta_badge.rs`,
`sparkline.rs`. Design-system enforcement is strong and tested:
`tests/consistency.rs`, `tests/contrast.rs`, `tests/layout_invariants.rs`,
and a 267-test panel-snapshot suite. `theme.rs` (63 KB) is the executable
token contract; `strings.rs` (86 KB) holds all copy. **The consistency
bar is high — any new feature must clear it.**

---

## 2. Top rough edges / gaps (human-friendliness lens)

Ranked by operator impact:

1. **The Live dashboard shows a permanently-empty equity curve + KPI
   strip.** (`live.rs:58,66`) An operator opening "Live" — the second
   sidebar entry, the natural "how are we doing" screen — sees a "no
   equity data" placeholder and six `—` dashes. This violates the
   "no blank screens — write what the user should do next" rule. The
   placeholder doesn't even say *why* it's empty or what to do. **This
   is the single highest-leverage gap** because the slots already exist;
   they just have no data behind them.

2. **The shipped passive baseline is invisible in the operator tool.**
   We just operationalized the BH baseline (equity curves + full metrics
   at `spec/runbooks/artifacts/passive-baseline-2026-06-08/`). The
   *headline result of the entire research program* — "passive BH:
   +1.84 Sharpe / +196% in 2023, +0.89 / +91% in 2024" — lives only in a
   markdown file. The cockpit, which has `equity_curve` + `kpi_strip`
   widgets purpose-built to render exactly this shape, shows none of it.
   Research → cockpit is disconnected.

3. **The Assistant slot is built but has no door handle** (§1.3). Either
   wire an affordance + give it a non-LLM purpose, or stop carrying it.

4. **The `viewer` bin is excellent but siloed.** It renders KPI strip +
   equity curve + drawdown band + markdown body for any committed
   `backtest-*.md` (`bin/viewer.rs`) — but it's a separate process the
   operator must launch from a CLI with a hand-typed report path. None
   of its capability is reachable from inside `cockpit_live`. The
   "surface backtest results in the cockpit" muscle is *already written*;
   it just lives in the wrong binary.

5. **Minor:** the Compare screen advertises five KPI-axis chips but only
   Sharpe is fully wired (`compare.rs:158` / R6.3). Not a blank-screen
   issue, but a "button that does less than it looks like" affordance.

---

## 3. Ranked build-out candidates (ungated)

### #1 — Baseline panel: surface the passive BH result in the cockpit (RECOMMENDED)

**What:** A "Baseline" view inside `cockpit_live` that renders the shipped
passive buy-hold result the way the research program reports it — equity
curve (2023 + 2024) + the full KPI strip (Sharpe, Sortino-as-CAGR-slot,
MaxDD, total return) + a plain-language characterization. Reuses the
existing `equity_curve` / `kpi_strip` / `drawdown_band` widgets verbatim.

**Why high-value now:**
- Connects research → cockpit: makes the *shipped, operationalized*
  result visible in the operator-facing tool (the program's literal
  deliverable).
- Directly fixes rough-edge #1 and #2: gives the empty Live-style slots
  real data, against a result we *know* is correct and anchored.
- Lowest implementation risk: every rendering widget already exists and
  is snapshot-tested; the `viewer` bin already proves the
  load→`EquitySeries`→`kpi_strip` path end-to-end. The only genuinely
  new code is a small data-loader (mirrors `models/registry_read.rs`).
- Ungated: depends on nothing moot. The BH artifacts are committed and
  byte-stable (119/119 anchors pass).

**Data-plumbing dependency (must be in scope — flagged):** The cockpit
**cannot read the BH baseline CSV as-is.** Two concrete mismatches:
1. **Column schema differs.** BH CSV is
   `bar_index, timestamp_utc, equity_usd` with timestamps like
   `2023-01-01T00:00Z` (`bh-equity-curve-2023.csv`). The `viewer`'s
   `reports::csv_artifacts::read_equity_csv` expects
   `ts, equity_total_usdt, realized_pnl_usdt, unrealized_pnl_usdt,
   cash_balance_usdt` (RFC3339 µs). So the viewer's loader will not parse
   the BH file.
2. **Location differs.** BH CSVs live at a *non-anchored runbook* path
   (`spec/runbooks/artifacts/passive-baseline-2026-06-08/`), not the
   `spec/<feature>/reports/artifacts/<run_id>/equity-*.csv` layout the
   viewer scans (`bin/viewer.rs:172` `load_equity_companion`).

   → **Scope decision for the architect:** the cleanest plumbing is a
   small `ui` baseline-loader module that (a) reads the two BH CSVs with
   their own 3-column schema into `EquitySeries::from_points`, and (b)
   carries a hand-entered/parsed `BacktestMetrics` from the
   characterization table (the metrics row is in the committed
   `.md`; total_return_pct/sharpe/max_drawdown_pct all map cleanly to
   `core::BacktestMetrics`). The metrics are byte-stable and anchored, so
   embedding them as a typed `const`/loader against the committed file is
   safe. **No new cross-crate edge** — `ui` already depends on `reports`
   + `core`; the loader is pure-`ui` reading committed files, exactly the
   `models/registry_read.rs` precedent (which reads committed
   `*.metadata.json`). Do **not** touch the anchored reports or the
   `REVISION.toml`; the BH CSVs are non-anchored data files (safe to read).

**Implementation sketch (which `ui` modules/views):**
- New `crates/ui/src/baseline/` module (sibling of `models/`, `memory/`):
  - `baseline/state.rs` — `BaselineScreenState { year_2023:
    PanelState<EquitySeries>, year_2024: PanelState<EquitySeries>,
    metrics_2023: PanelState<BacktestMetrics>, metrics_2024: …,
    active_year: BaselineYear }` + a `BaselineYear` toggle enum.
  - `baseline/loader.rs` — reads the two committed BH CSVs (3-column
    schema) → `EquitySeries`; builds `BacktestMetrics` from the
    characterization table. Robust-by-default (serde/parse miss →
    `PanelState::Error` with a "couldn't read baseline data" string),
    mirroring `models/registry_read.rs` K2.
- New `crates/ui/src/screens/baseline.rs` — `view(&Cockpit, ThemeMode)`:
  year toggle chips (2023 | 2024, default 2024 = most recent) →
  `kpi_strip::view` → `equity_curve::view` → `drawdown_band::view` →
  a short plain-language caption ("Equal-weight buy-and-hold across 10
  large-cap pairs, no rebalancing. This is the bar every active strategy
  was measured against.").
- `state.rs` — add `Screen::Baseline` variant + `baseline_screen_state`
  field (three-touchpoint pattern: enum + Default + Debug) + a
  `Message::BaselineSelectYear(BaselineYear)` arm (typed, no String
  payload).
- `shell.rs` `screen_body` — add the `Screen::Baseline => baseline::view`
  arm.
- `theme.rs` — add `Screen::Baseline` to `SIDEBAR_GROUPS_PHASE_C` (Work
  group, after Compare) + the flatten-invariant test updates with it.
- `strings.rs` — new `BASELINE_*` copy block (see §4).

**Lumen + human-friendliness angle:**
- Reuses `equity_curve` / `kpi_strip` / `drawdown_band` → zero new
  widgets, zero new tokens (the design-system goal: additions near zero).
- Explicit `Loading` / `Empty` / `Error` states inherited from
  `PanelState` + the widgets' built-in empty rendering — no blank screen.
- Plain-language caption demystifies "buy-and-hold baseline" (the
  show-the-why rule). Sharpe/MaxDD get the existing one-line treatment.
- P&L coloring: equity-up renders in `UP_500`, drawdown band in
  `DOWN_500` — the existing operator mental model carries over.
- Renders in both `--theme dark` and `--theme light` for free (the
  widgets are already theme-correct).

**Rough size:** **S–M.** ~1 new module (2 files) + 1 screen + ~5
touchpoints in existing files + a strings block. No new widget, no new
token, no new crate edge. The loader is the only non-trivial logic and it
has a direct precedent. Highest value / lowest risk of the three.

**Cockpit-smoke implications:** Adding a `Screen::Baseline` route means
the fixtures `cockpit` should default-route or be navigable to it so the
7 s smoke window paints it. The loader must degrade to `PanelState::Error`
(never panic) when the committed CSV is absent in a fixtures-only checkout
— this keeps the smoke gate green and exercises the empty/error path. A
panel-snapshot test + a both-themes render test bring it up to the
crate's consistency bar.

---

### #2 — In-cockpit report viewer (promote the `viewer` into a cockpit screen)

**What:** Surface the existing offline `viewer` capability *inside*
`cockpit_live` as a "Reports" screen: a left list of discovered
`spec/*/reports/backtest-*.md` files + the right pane rendering the
selected one's KPI strip + equity curve + drawdown band + markdown body
(exactly what `bin/viewer.rs` already does, minus the separate process).

**Why high-value:** Generalizes #1 — instead of just the BH baseline, the
operator can browse *any* committed backtest report from inside the
cockpit. The render path is already written and snapshot-tested in the
`viewer` bin; this is mostly a re-host + a file-discovery list.

**Implementation sketch:** New `screens/reports.rs` + a
`reports_screen_state` (discovered report paths via a `walkdir`-style scan
mirroring `models/registry_read.rs`; selected-report `ReportLoadResult`).
Reuses `ViewerModel`'s load logic (lift `load_report` out of `bin/viewer.rs`
into the lib so both the bin and the screen call it). Message:
`Message::ReportsSelect(path)`.

**Lumen + human-friendliness angle:** Same widget reuse as #1. Adds a
list-detail pattern (already used by Memory/Models). Needs a real
**empty state** ("No backtest reports found in spec/ yet") and a careful
**loading state** (report parse is synchronous + cheap, but a large
markdown body should still show a spinner if > 100 ms per the latency
rule).

**Rough size:** **M.** More surface than #1 (file discovery, list-detail,
lifting `load_report` into the lib, a selection model). Higher value
ceiling (any report, not just BH) but more moving parts and more states
to get right. **Recommended as the v0.2 follow-on to #1** — #1 proves the
baseline-specific path; #2 generalizes it.

---

### #3 — Repurpose the Assistant slot as a "Help / Explain" panel (pure-UI)

**What:** Give the built-but-unreachable right-rail (§1.3) a door handle
(a sidebar/status-bar toggle) and a non-LLM purpose: a context-sensitive
**Help / Explain** panel that defines the terms on the current screen
(Sharpe, MaxDD, drawdown, Calmar, "buy-and-hold baseline") and links to
the relevant decision trail. Static, dictionary-style content from
`strings.rs` — no LLM, no `llm` dep, no moot strategy.

**Why high-value (and the honest caveat):** It satisfies the
"show-the-why / plain-language" rules and reclaims a dead panel. **But**
per the ui-designer pushback policy, I will not recommend building it
*just to fill a reserved slot* — the value is real only if the operator
actually wants an always-available glossary/explain rail. The tooltips
that the design principles already call for (`ui-design-principles.md:248`)
cover most of this need more cheaply. **Lower priority than #1/#2.**

**Implementation sketch:** Wire a `Message::ToggleAssistant` to a
status-bar button; add an `AssistantMode::Help` arm to `assistant/view.rs`
that renders a per-`current_screen` glossary `Column` from new
`HELP_*` strings. Keep the `Offline` byte-identity guard intact (the
snapshot baseline is locked — `assistant/view.rs:107`).

**Rough size:** **S** (mechanically) but **questionable value** — flag to
the operator as "only if you want a persistent explain rail; otherwise
prefer per-term tooltips."

---

## 4. #1 detail — strings, states, tokens, accessibility

(Promoted here so #1 is implementable next without re-discovery.)

**New screens / panels / widgets:**
- New screen: `Screen::Baseline` (`screens/baseline.rs`).
- New module: `baseline/` (`state.rs`, `loader.rs`).
- **New widgets: none** (reuses `equity_curve`, `kpi_strip`,
  `drawdown_band`, plus chip buttons in the established Compare/Lab chip
  pattern).

**New strings (`ui::strings`, `BASELINE_*` block):**
- `BASELINE_SIDEBAR_LABEL` = "Baseline"
- `BASELINE_HEADLINE` = "Passive baseline"
- `BASELINE_CAPTION` = "Equal-weight buy-and-hold across 10 large-cap
  pairs, bought once at year-open, never rebalanced. This is the bar
  every active strategy was measured against — and none beat it."
- `BASELINE_YEAR_2023_LABEL` = "2023"
- `BASELINE_YEAR_2024_LABEL` = "2024"
- `BASELINE_DATA_UNAVAILABLE` = "Baseline data isn't bundled in this
  build. Equity CSVs live at spec/runbooks/artifacts/." (error-state copy
  — tells the operator what to do next, per the no-blank-screen rule)
- `BASELINE_SHARPE_TOOLTIP` / `BASELINE_MAXDD_TOOLTIP` — one-line
  plain-language definitions (reuse existing KPI tooltips if present).

**New theme tokens:** **target zero.** Equity-up `UP_500`, drawdown
`DOWN_500`, panel chrome, type scale — all exist. (Any token addition
here is a smell and should be challenged in review.)

**Explicit states (per panel):**
- **Loading:** while the loader runs at boot — `kpi_strip` + `equity_curve`
  render their built-in Loading placeholders.
- **Ready:** the populated 2023/2024 curve + metrics.
- **Empty:** never expected (data is committed) — but if the CSV parses to
  zero points, the widgets render their empty state, not a blank.
- **Error:** CSV missing/malformed → `PanelState::Error(BASELINE_DATA_UNAVAILABLE)`;
  the widgets render the muted error body. **This is the path the
  fixtures-only `cockpit` smoke build will hit** (the CSVs may not ship in
  a minimal checkout), so it must be exercised and snapshot-tested.

**Accessibility notes:**
- Keyboard: the year toggle chips are `Button`s (focusable + Enter-
  activatable, same as Compare's range chips).
- Focus order: year toggle → (curve/strip are non-interactive read-only).
- Contrast: inherited from `theme` (already ≥ 4.5:1, verified by
  `tests/contrast.rs`); equity green is never the *only* signal — the
  axis labels + numeric values carry the meaning too.
- Color-blind safety: P&L sign is paired with the numeric value and the
  KPI labels, not color alone.

**Wireframe (ascii):**

```text
┌─ Baseline ─────────────────────────────────────────────────────────┐
│  Passive baseline                                  [ 2023 ][ 2024◀ ] │
│  Equal-weight buy-and-hold across 10 large-cap pairs, never          │
│  rebalanced. The bar every active strategy was measured against.     │
│                                                                      │
│  ┌── KPI strip ───────────────────────────────────────────────────┐ │
│  │  Total return   CAGR/Calmar   Sharpe    Max DD     Win rate  …  │ │
│  │  +91.0%         +1.85         +0.89     48.95%      —            │ │
│  └────────────────────────────────────────────────────────────────┘ │
│  ┌── Equity curve (Money<Usdt>) ──────────────────────────────────┐ │
│  │            ╱╲      ╱╲╱╲                                         │ │
│  │      ╱╲╱╲╱    ╲╱╲╱      ╲╱╲___                                 │ │
│  │  ___╱                                                          │ │
│  └────────────────────────────────────────────────────────────────┘ │
│  ┌── Drawdown band ───────────────────────────────────────────────┐ │
│  │  ▁▂▁▃▅▃▂▁▁▂▄▆▄▂▁  (DOWN_500 below zero)                        │ │
│  └────────────────────────────────────────────────────────────────┘ │
└──────────────────────────────────────────────────────────────────────┘
```

---

## 5. Recommendation to the operator

**Build #1 (Baseline panel) next.** It is ungated, lowest-risk (every
render widget already exists and is snapshot-tested), and directly closes
the two highest-impact gaps (empty Live slots + invisible shipped result).
The one real dependency — the BH-CSV schema/location mismatch — is small,
has a direct precedent (`models/registry_read.rs`), and adds no new crate
edge. **#2 (in-cockpit report viewer) is the natural v0.2 follow-on**
(generalizes #1 to any report). **#3 (Assistant Help rail) only if the
operator wants a persistent explain rail** — otherwise prefer per-term
tooltips and retire or defer the slot.

Next pipeline step: analyst carves a feature brief for #1 from this
note's §3/§4, architect resolves the loader-vs-embedded-metrics plumbing
decision and the sidebar IA insertion, then developer ‖ ui-designer
implement against the existing widgets.

---

## Changelog

- 2026-06-08 (ui-designer): initial audit + ranked candidates. Grounded in
  the `ui` crate at the wind-down commit; fixtures `cockpit` builds clean
  (exit 0). Flagged the Live empty-slot gap (`live.rs:58,66`), the
  unreachable Assistant slot (no `is_open` affordance), and the BH-CSV
  schema/location mismatch as the #1 data-plumbing dependency.
