---
slug: cockpit-baseline-panel
status: in-progress
owner: ui-designer
updated: 2026-06-08
version: 0.1.0
---

# Cockpit Baseline panel — surface the shipped passive BH result

## Why

The research program concluded: **active ≤ passive in the reachable
universe, this sample**. The headline deliverable — the passive
buy-and-hold (BH) baseline — was characterized and operationalized
(`spec/runbooks/artifacts/passive-baseline-2026-06-08/`), but it lives
**only in markdown**. The operator-facing cockpit, which already ships
`equity_curve` / `kpi_strip` / `drawdown_band` widgets purpose-built for
exactly this shape, surfaces none of it. Research → cockpit is
disconnected.

This feature is the ui-designer's **#1 ranked build-out candidate**
(`spec/dev-notes/cockpit-buildout-audit-2026-06-08.md` §3.1, operator
greenlit 2026-06-08: "build out the cockpit/UI"). It is the lowest-risk
of the three candidates: every render widget already exists and is
snapshot-tested; the only new logic is a small CSV loader with a direct
precedent (`crates/ui/src/models/registry_read.rs`).

**Scope:** S–M UI feature. One new `ui` module (2 files) + one screen +
~5 touchpoints in existing files + a strings block. No new widget, no
new theme token, no new crate edge.

## Requirements

A new **`Screen::Baseline`** inside the cockpit shell that renders the
shipped passive-BH result, reusing existing widgets verbatim.

- **R1 — Render the BH result.** The screen shows, for a selected year
  (2023 | 2024, default **2024** = most recent):
  - **Equity curve** (`equity_curve::view`) from the realized single-path
    BH curve (`bh-equity-curve-{2023,2024}.csv`).
  - **Drawdown band** (`drawdown_band::view`) derived from the same
    `EquitySeries` (drawdown is already computed per-point by
    `EquitySeries::from_points`).
  - **KPI strip** (`kpi_strip::view`) showing the six existing cards:
    Total return / CAGR / Sharpe / Max DD / Win rate / Trades.
- **R2 — Year toggle.** Two chip buttons (`2023` | `2024`) in the
  established Compare/Lab chip pattern, focusable + Enter-activatable.
  Default selection = **2024**. A typed `Message::BaselineSelectYear(
  BaselineYear)` arm (no `String` payload).
- **R3 — Honest caption.** A plain-language caption below the headline
  that conveys the **bounded** scope truthfully:
  - States the construction: equal-weight buy-and-hold across 10
    large-cap pairs, bought once at year-open, never rebalanced.
  - States the honest finding: **"passive baseline; active ≤ passive in
    the reachable universe, this sample."** **MUST NOT** claim "passive
    is optimal" or "none can beat it" (overclaims the bounded result —
    see § Adjustments to the audit, A3).
- **R4 — Four panel states.** Each panel honors `PanelState<T>`:
  - **Loading** — at boot while the loader runs; widgets render their
    built-in skeleton.
  - **Ready** — populated curve + band + metrics.
  - **Empty** — CSV parses to zero points (not expected; data is
    committed) → widgets render their empty body, never a blank.
  - **Error** — CSV missing/malformed → `PanelState::Error(
    BASELINE_DATA_UNAVAILABLE)`; widgets render the muted error body.
    **This is the path the fixtures-only `cockpit` smoke hits** (CSVs may
    not ship in a minimal checkout) — it must be exercised + snapshot-tested.
- **R5 — Lumen consistency.** Theme tokens only (equity-up `UP_500`,
  drawdown `DOWN_500`, panel chrome, type scale — all exist; **target
  zero new tokens**). All copy in `strings.rs` (`BASELINE_*` block — no
  hardcoded strings). Renders correctly in both `--theme dark` and
  `--theme light` for free (widgets are already theme-correct).
- **R6 — Sidebar IA.** Add `Screen::Baseline` to the **Work** group in
  `SIDEBAR_GROUPS_PHASE_C` (after Compare). Label = `BASELINE_SIDEBAR_LABEL`
  = "Baseline".
- **R7 — Smoke safety.** The loader **never panics** (parse-miss →
  `Error` state, mirroring `registry_read.rs` K2). The fixtures `cockpit`
  smoke must be able to paint the Baseline screen within its render
  window (first-frame render, no panic).

### Out of scope (explicit)

- Candidate **#2** (in-cockpit report viewer) — the v0.2 follow-on.
- Candidate **#3** (Assistant Help rail).
- Gated **Phase 6** Assistant / cockpit cross-platform.
- **New widgets / new theme tokens** — if either is needed, that is a
  smell and must be challenged in review (R5).
- **Sortino / Calmar KPI cards** — the existing `kpi_strip` renders six
  fixed cards (Total return / CAGR / Sharpe / Max DD / Win rate /
  Trades); it has no Sortino/Calmar slot. The characterization DOES carry
  Sortino (2.51 / 1.20) and Calmar (5.68 / 1.85), but surfacing them as
  KPI cards would require a widget change → **out of scope** (see A2).
  They MAY appear as caption text at the architect's discretion.

## Design

_Architect-owned (2026-06-08). D1 + D2 resolved below against the actual
crate edges. No ADR: every decision here is pure-`ui`, additive, and
within the existing `viewer`/`registry_read` precedent — nothing
architecturally load-bearing (no new crate edge, no anchor change, no
budget shift). Decisions are recorded inline + in the Changelog._

### Crate edges (verified — corrects one brief assumption)

The brief's "ui depends on core + reports, NOT backtest" is **slightly
off**: `crates/ui/Cargo.toml:70` does declare `backtest = { path =
"../backtest" }` — added by ADR-0030 for the **Lab Run button only**
(`engine::run_scenario`). It is the one sanctioned non-`live` cross-crate
edge. **This feature adds NO new edge** and does not use the `backtest`
dep: the baseline loader is pure-`ui` over `core` + `std::fs`, exactly
the `models/registry_read.rs` precedent. AC7 holds.

Relevant verified facts:

- `core::EquitySeries::from_points` (`equity_series.rs:68`) computes
  `peak`/`trough`/per-point `drawdown_pct`/`max_drawdown_pct` in one O(N)
  `Decimal` walk — **drawdown band (R1) is free** once the curve loads.
- `core::BacktestMetrics` (`equity_series.rs:167`) is a **pure-data
  struct** — no compute methods, no `sortino`/`calmar` field. Its only
  behaviour is the `all_absent()` zero-sentinel.
- **There is NO Sharpe / CAGR / total-return math anywhere in
  `crates/core`** (exhaustive grep: only the struct field declarations).
  `compute_sharpe_hourly` / `compute_calmar` / `compute_max_drawdown_f64`
  live **only in `crates/backtest`** (`param_robustness_sweep.rs`,
  `stats/mod.rs`, `examples/passive_baseline_equity.rs`) and
  `crates/forecast`. The Lab KPI strip itself renders Sharpe as `—` with
  the standing comment "engine not yet computing" (`kpi_strip.rs:273`).
- `core::Timestamp` (`time.rs`) wraps `time::OffsetDateTime`; its serde is
  `rfc3339`. There is **no `FromStr`/parse helper** — the loader parses
  the timestamp itself via the `time` crate (already a `ui` dep).

### Decision D1 — RESOLVED: (c) embed the six KPI scalars as a typed `const`; file-load the curve

**Option (a) "compute from the loaded curve" is INFEASIBLE here, and
worse, semantically wrong** — so it is rejected on the merits, not on
budget:

1. *Infeasible without a new edge.* Sharpe/CAGR/total-return are not in
   `core`. Computing them in-`ui` means either (i) adding a new
   annualized-Sharpe-from-equity-points module to `core` (a non-trivial
   methodology decision — annualization base, return-series convention —
   that this S–M read-only panel has no business introducing), or (ii)
   reaching into `backtest`'s `compute_sharpe_hourly`, which **violates
   AC7** (the `backtest` dep is ADR-0030-scoped to the Lab Run button).
2. *Semantically wrong even if feasible.* The published §7.1 Sharpe
   (`+1.8417`/`+0.8925`) was computed over the **full 8,759 / 8,784 hourly
   bars** with annualization `sqrt(8760)`. The committed CSV is
   **daily-sampled (stride 24, ~366 points)**. Recomputing Sharpe — or
   even MaxDD — from the 366 daily points would produce a **different
   number** than the operator reads in the characterization. The KPI strip
   must match the *published* result, not a re-derivation. So even the
   one metric `from_points` gives for free (MaxDD) is taken from the
   `const`, not the loaded daily curve, for cross-surface consistency.

**Option (b) "parse the `.md` table" is rejected** — fragile markdown
parsing (the §7.1 table is hand-formatted prose, not a stable wire
schema; `reports::parse::parse_from_report` targets the *backtest*-report
KPI block, a different layout) for zero benefit over (c): the six scalars
are byte-stable and anchored, so a parse buys no freshness a `const`
lacks, and adds a parse failure mode.

**Chosen: (c) hybrid — curve from CSV, six scalars + caption Sortino/Calmar
as a typed `const`.**

- The **equity curve** (R1) is file-driven: `baseline/loader.rs` reads the
  ~366-point CSV → `EquitySeries::from_points`. The curve is the bulky
  data; it must live in the file. This also gives the real Loading/Error
  path R4/R7 require, and the drawdown band for free.
- The **six KPI scalars + caption Sortino/Calmar** are a
  `const`/`fn`-built `BacktestMetrics` per year, sourced from
  characterization §7.1 (the **realized single-path** row, NOT bootstrap
  p50 — the panel draws the realized curve, so the strip must match the
  line). Values locked below.
- **MaxDD source nuance:** the KPI-strip `max_drawdown_pct` comes from the
  `const` (§7.1 intraday 34.57% / 48.95%), so the headline number matches
  the characterization. The drawdown *band* renders from the loaded daily
  curve's per-point `drawdown_pct` (visual shape only) — a band whose
  visual trough may sit a hair shallower than the const headline because
  it is daily-sampled. This is acceptable and expected (the band is a
  shape, the card is the number); the loader does not try to reconcile
  them.

**Re-sync contract (the one cost of (c)).** Because the scalars are
embedded, they go stale if the characterization is ever re-run with
different numbers. Mitigation: (1) a `// RE-SYNC:` doc-comment block on
the `const` block in `baseline/loader.rs` naming the source
(`passive-baseline-characterization.md §7.1`) and the exact values; (2) a
unit test `baseline_metrics_match_characterization` that asserts the six
embedded scalars equal the documented §7.1 values (so a silent edit trips
a test); (3) a one-line note in the `passive-baseline.md` runbook pointing
back here. This is a documented maintenance task, not a hidden
foot-gun. If the characterization is ever re-run, the failing test is the
re-sync trigger.

**Locked metric values** (characterization §7.1 realized row → the six
`kpi_strip` cards; `win_rate` + `trades` are not meaningful for
buy-once-hold):

| Field | 2023 | 2024 | Notes |
|-------|------|------|-------|
| `total_return_pct` | `196.22` | `91.04` | sentiment-coloured (positive → `UP_500`) |
| `cagr_pct` (`cagr_present=true`) | `196.22` | `91.04` | 1-yr horizon ⇒ CAGR ≈ total return (see caption nuance below) |
| `sharpe` (`sharpe_present=true`) | `1.8417` | `0.8925` | `format_sharpe` renders 4-dp |
| `max_drawdown_pct` | `34.57` | `48.95` | always `DOWN_500` with minus prefix |
| `win_rate_pct` | — (`win_rate_present=false`) | — | renders `—`; do NOT fabricate |
| `trades` | `0` | `0` | buy-once-hold |

Caption-only (no KPI slot — A2): **Sortino** `2.5126` / `1.2047`,
**Calmar** `5.677` / `1.853`.

> **CAGR honesty note.** §7.1 publishes `TotalReturn%` but not a separate
> `CAGR%`. For a single full-year hold the annualized growth rate equals
> the total return (the period IS one year), so setting `cagr_pct =
> total_return_pct` is correct, not a fabrication. The `const` doc-comment
> states this derivation explicitly so a future reader does not mistake it
> for a copied-from-source value. (Characterization §7.1 footnote derives
> Calmar as `CAGR/maxDD`; with CAGR=196.22% and maxDD=34.57%,
> 1.9622/0.3457 ≈ 5.677 — the published Calmar — which independently
> confirms CAGR≈total-return for this horizon.)

### Decision D2 — RESOLVED: navigable (not default-routed); a dedicated headless Error-state render test carries the smoke assertion

The fixtures `cockpit` smoke (`crates/ui/src/bin/cockpit.rs`) boots to
`Screen::Home` (→ `Live`) and paints the full fixture steady-state; the
gate is first-frame render + no panic (`headless_emulator_smoke.rs`).

**Chosen: make `Screen::Baseline` navigable, do NOT change the smoke's
default screen.**

- *Navigable* = registered in `SIDEBAR_GROUPS_PHASE_C` (Work group, after
  Compare) + routed in `shell::screen_body`. The operator (and any
  screen-visiting test) can reach it; the existing smoke's default-screen
  snapshot baseline is **untouched** (re-routing the default would churn
  the locked first-frame baseline for no smoke-coverage gain).
- *Determinism of the smoke gate.* Default-routing to Baseline would make
  the smoke's first frame depend on CSV presence in the checkout — a
  flakiness source the brief explicitly warns against. Keeping the default
  on `Live` keeps the smoke deterministic regardless of whether the
  runbook CSVs are present.
- *Error-state assertion lives in a dedicated headless test*, not the
  smoke: `baseline_error_state_renders_without_panic` constructs the
  Baseline screen state with the loader pointed at a **missing** path,
  asserts both panels land in `PanelState::Error(BASELINE_DATA_UNAVAILABLE)`
  and that `screens::baseline::view` renders in both themes without panic
  (AC2/AC3). This is the deterministic equivalent of "the smoke paints the
  Error path" — it pins the exact behaviour the fixtures-only checkout
  hits, without coupling the global smoke gate to data files.
- *Optional belt-and-braces (developer discretion, cheap):* if the smoke
  harness already visits every sidebar screen, Baseline is covered for
  free by being in `SIDEBAR_GROUPS_PHASE_C`; no extra work. If it does
  not, the dedicated test above is sufficient — do not expand the smoke.

Net: R7 / AC3 are satisfied by the navigable route + the dedicated
Error-state headless test; the cockpit-smoke gate stays first-frame +
no-panic + deterministic.

### `baseline/loader.rs` contract

Pure-`ui` module, no new crate edge. Mirrors `viewer::load_equity_companion`
(`bin/viewer.rs:172`) and the `registry_read.rs` K2 never-panic contract.

**Input** — committed CSV, schema `bar_index,timestamp_utc,equity_usd`:

```
bar_index,timestamp_utc,equity_usd
0,2024-01-01T00:00Z,100000.00
25,2024-01-02T00:00Z,105017.03
...
8784,2024-12-31T23:00Z,191040.25
```

- `timestamp_utc` is **minute-precision Zulu** (`2024-01-01T00:00Z`), NOT
  RFC3339 µs. The standard `time::OffsetDateTime::parse(s, &Rfc3339)` path
  used by `reports::csv_artifacts` **will reject** this (no seconds). The
  loader uses an explicit `format_description` accepting
  `[year]-[month]-[day]T[hour]:[minute]Z` (or normalises by appending
  `:00` before an `Rfc3339` parse — implementer's choice; the former is
  cleaner). A parse unit test pins the `…T00:00Z` shape.
- `equity_usd` → `rust_decimal::Decimal` → `Money::<Usdt>::from_decimal`.
  Never `f64`.
- `bar_index` is **informational — ignored**. Row order in the file is
  oldest-first (which `from_points` requires); the loader preserves file
  order and does not sort on `bar_index`.

**Signatures** (shape; final names the implementer's):

```rust
/// Load one year's BH equity curve from its committed CSV.
/// Mirrors viewer::load_equity_companion — synchronous, never panics.
/// Ok(Ready(series)) on success; Ok(Empty) on zero data rows;
/// Err(BASELINE_DATA_UNAVAILABLE) on missing file / parse miss.
pub fn load_baseline_curve(path: &Path) -> PanelState<EquitySeries>;

/// The realized §7.1 metrics for a year, embedded (D1=c).
/// RE-SYNC: values mirror passive-baseline-characterization.md §7.1.
pub fn baseline_metrics(year: BaselineYear) -> BacktestMetrics;
```

**Error cases → states** (R4):

| Condition | Returned state | Copy |
|-----------|----------------|------|
| File missing / unreadable | `PanelState::Error(BASELINE_DATA_UNAVAILABLE)` | "Baseline data isn't bundled in this build. …" |
| CSV header/row parse miss (bad timestamp, non-decimal equity) | `PanelState::Error(BASELINE_DATA_UNAVAILABLE)` | same |
| `from_points` → `Empty`/`NonMonotone` err | `PanelState::Error(BASELINE_DATA_UNAVAILABLE)` | same |
| Zero data rows (header only) | `PanelState::Empty` | widget empty body |
| OK | `PanelState::Ready(EquitySeries)` | populated |

The metrics half **never errors** (it is a `const` map): `baseline_metrics`
always returns a `Ready`-able `BacktestMetrics`. So a missing CSV yields
**curve+band in Error, KPI strip still populated from the const** — an
honest degrade (the numbers are known; only the drawn line is absent).
The implementer may, at discretion, also force the KPI strip to its
unavailable state when the curve errors (for visual coherence) — but the
default (strip stays populated) is acceptable and arguably more useful.

**Path resolution.** The CSV path is resolved relative to the workspace
root (`CARGO_MANIFEST_DIR`/`../../spec/runbooks/artifacts/passive-baseline-2026-06-08/bh-equity-curve-{year}.csv`),
matching how `registry_read` resolves `crates/forecast/checkpoints/...`.
The exact base-path helper is the implementer's call; isolate it in one
function so the Error-state test can point it at a bogus path. **Do not
hardcode an absolute path.**

### `Screen::Baseline` integration

Three-touchpoint pattern (matches every existing screen):

1. **`state.rs`** — add `Screen::Baseline` variant (after `Strategies`,
   before the deprecated aliases). Add a `baseline_screen_state:
   BaselineScreenState` field to `Cockpit` (enum + `Default` + `Debug`
   touchpoints). Add the typed message arm
   `Message::BaselineSelectYear(BaselineYear)` — **no `String` payload**
   (R2). `BaselineYear` is a 2-variant `enum { Y2023, Y2024 }`,
   `Default = Y2024`.
2. **`baseline/state.rs`** — `BaselineScreenState { curve_2023:
   PanelState<EquitySeries>, curve_2024: PanelState<EquitySeries>,
   active_year: BaselineYear }`. Metrics are not stored (pulled from the
   `const` `baseline_metrics(active_year)` at view time). The two curves
   are loaded once at boot (or lazily on first Baseline visit — boot-load
   is simpler and matches the fixtures pattern). `Default` =
   `active_year: Y2024`, both curves `Loading`.
3. **`shell::screen_body`** — add `Screen::Baseline => baseline::view(model,
   mode)` arm.

**`update` wiring** (`state.rs`): `Message::BaselineSelectYear(y)` sets
`baseline_screen_state.active_year = y`. Curve loading happens at boot via
a helper the bins call (mirroring how `cockpit.rs` boot pre-seeds
fixtures), or on first visit — either is fine; boot-load keeps `update`
trivial and is the recommended path.

**`screens/baseline.rs` `view(&Cockpit, ThemeMode)`** composition
(top→bottom), reusing widgets **verbatim**:

```
headline (BASELINE_HEADLINE, text::H2)            year chips [2023][2024◀]
caption (BASELINE_CAPTION, plain language, honest bounded scope — R3/A3)
kpi_strip::view(&Ready(baseline_metrics(active_year)), mode).map(bridge)
equity_curve::view(&curve[active_year], mode).map(bridge)
drawdown_band::view(&curve[active_year], mode).map(bridge)
[optional] caption-only Sortino/Calmar line (BASELINE_RISK_DETAIL, FG_3)
```

- The three widgets return `Element<'_, ViewerMessage>`; bridge to the
  screen's `Message` with `.map(|_| Message::…)` exactly as
  `screens/live.rs:62,67` does (`.map(|_| Message::ChartMarkerHoverEnded)`
  — a never-fired no-op bridge, since these panels emit no interactions
  for Baseline). Reuse the same harmless arm.
- **Year chips** use the established Compare/Lab chip pattern
  (`screens/compare.rs::build_range_chips`): `Button` + active/inactive
  token styling, `on_press(Message::BaselineSelectYear(y))`,
  focusable + Enter-activatable (R2 / accessibility). Active chip =
  `ACCENT` + `PANEL_RAISED` bg; inactive = `FG_3` + `BORDER_1`.

**Sidebar IA** (R6) — `theme.rs`:

- Add `Screen::Baseline` to `SIDEBAR_GROUPS_PHASE_C` **Work** group, after
  `Compare`: `&[Screen::Lab, Screen::Live, Screen::Compare, Screen::Baseline]`.
- The flatten-invariant test
  (`sidebar_groups_phase_c__flatten_matches_phase_a`) compares the
  flattened groups against `SIDEBAR_ENTRIES_PHASE_A` — so **add
  `Screen::Baseline` to `SIDEBAR_ENTRIES_PHASE_A` too**, in the same
  position (after `Compare`, before `Strategies`). Both consts must stay
  in lock-step or the test fails (this is the intended guard, AC6).
- Sidebar label = `BASELINE_SIDEBAR_LABEL` = "Baseline".

### Lumen / strings / lint

- **Zero new theme tokens** (R5): equity-up `UP_500`, drawdown `DOWN_500`,
  panel chrome, type scale all exist. Any token addition is a review smell.
- **All copy via `strings.rs` `BASELINE_*`** (R5): see § Strings. No
  hardcoded strings, no hex colours. Renders in both themes for free.
- **Lint:** new Baseline code follows the `ui` crate's existing
  per-module `#![allow(...)]` convention (e.g. the screen module mirrors
  `screens/live.rs`'s `#![allow(clippy::cast_possible_truncation,
  clippy::needless_pass_by_value)]`). It introduces **no new warnings** and
  does **not** touch the pre-existing ~140 pedantic lints — out of scope.

### Determinism / money / no-overclaim guardrails

- Money math is `rust_decimal::Decimal` + `Money<Usdt>` throughout — no
  `f64` in the loader or the metrics const (CLAUDE.md non-negotiable).
- No RNG, no clock reads, no timestamps-in-body — this is a read-only
  panel over committed data; nothing hashed, no anchor touched.
- The caption is **binding** (R3/A3): it states the honest bounded finding
  "passive baseline; active ≤ passive in the reachable universe, this
  sample" and **MUST NOT** claim "optimal"/"unbeatable"/"none beat it".
  Asserted by a string-content test (AC5).

### Decisions D1/D2 — one-line summary

- **D1 = (c)** embed the six §7.1 realized scalars (+ caption
  Sortino/Calmar) as a typed `const`; file-load the curve. Chosen because
  (a) is infeasible (no Sharpe/CAGR/return math in `core`) **and**
  semantically wrong (published Sharpe is hourly, the CSV is daily-sampled
  → re-derivation ≠ published number); (b) is fragile parsing for no gain.
  Cost = a documented re-sync test + runbook note.
- **D2 = navigable, not default-routed**; the Error-state assertion lives
  in a dedicated headless render test (`baseline_error_state_renders_without_panic`)
  so the global cockpit-smoke gate stays first-frame + no-panic +
  deterministic regardless of CSV presence.

## Data contract

A new pure-`ui` loader module — **no new cross-crate edge** (`ui` already
depends on `core` + `reports`). Precedent: `crates/ui/src/models/registry_read.rs`.

### Source files (non-anchored — safe to read)

| File | Rows | Columns |
|------|------|---------|
| `spec/runbooks/artifacts/passive-baseline-2026-06-08/bh-equity-curve-2023.csv` | 367 (366 data + header) | `bar_index, timestamp_utc, equity_usd` |
| `spec/runbooks/artifacts/passive-baseline-2026-06-08/bh-equity-curve-2024.csv` | 368 (367 data + header) | `bar_index, timestamp_utc, equity_usd` |

- Timestamps: `2024-01-01T00:00Z` form (minute-precision Zulu, **not**
  RFC3339 microseconds). The loader's date parse must accept this shape.
- `equity_usd`: plain decimal dollars (e.g. `100000.00`), starting at
  `$100,000.00`.

### Schema/location mismatch (flagged by the ui-designer — confirmed)

The cockpit's existing `viewer` loader **cannot parse these CSVs as-is**:

1. **Column schema differs.** The `viewer`'s
   `reports::csv_artifacts::read_equity_csv` expects `ts,
   equity_total_usdt, realized_pnl_usdt, unrealized_pnl_usdt,
   cash_balance_usdt` (RFC3339 µs). The BH CSV is a **3-column**
   `bar_index, timestamp_utc, equity_usd` schema → the viewer loader will
   not parse it.
2. **Location differs.** BH CSVs live at a **non-anchored runbook** path,
   not the `spec/<feature>/reports/artifacts/<run_id>/equity-*.csv`
   layout the viewer scans (`bin/viewer.rs` `load_equity_companion`).

→ Therefore a **new** 3-column loader is required (not a viewer reuse).
It reads `(timestamp_utc, equity_usd)` per row → `(Timestamp,
Money<Usdt>)` → `EquitySeries::from_points(...)`. The `bar_index` column
is informational (the loader may ignore it; ordering comes from the
file's oldest-first row order, which `from_points` requires).

### Target types (verified against the codebase)

- **`core::EquitySeries`** (`crates/core/src/equity_series.rs:68`):
  `pub fn from_points(points: Vec<(Timestamp, Money<Usdt>)>) -> Result<Self,
  EquitySeriesError>`. Computes `peak`, `trough`, per-point `drawdown_pct`,
  and series `max_drawdown_pct` — so the **drawdown band (R1) is free**
  once the curve loads.
- **`core::BacktestMetrics`** (`crates/core/src/equity_series.rs:167`)
  has fields: `total_return_pct, cagr_pct, cagr_present, sharpe,
  sharpe_present, max_drawdown_pct, win_rate_pct, win_rate_present,
  trades`. **No `sortino`/`calmar` field** (see A2).

### Realized BH metrics to surface (from characterization §7.1)

These map onto the six KPI cards the existing `kpi_strip` renders:

| KPI card | 2023 | 2024 | `BacktestMetrics` field |
|----------|------|------|-------------------------|
| Total return | **+196.22%** | **+91.04%** | `total_return_pct` |
| CAGR | +196.22% (1-yr ≈ total) | +91.04% | `cagr_pct` (`cagr_present = true`) |
| Sharpe | **+1.8417** | **+0.8925** | `sharpe` (`sharpe_present = true`) |
| Max DD | **34.57%** | **48.95%** | `max_drawdown_pct` |
| Win rate | — (`win_rate_present = false`) | — | `win_rate_pct` |
| Trades | 0 (buy-once-hold) | 0 | `trades` |

- Use the **realized single-path** metrics (§7.1), not the bootstrap p50
  — the equity curve the panel draws IS the realized path, so the KPI
  strip must match the line the operator sees (consistency). The
  characterization §7.3 documents the realized-vs-bootstrap gap; the
  panel surfaces the realized numbers only.
- `win_rate` and `trades` have no meaningful value for buy-once-hold:
  set `win_rate_present = false` (renders `—`) and `trades = 0`. **Do not
  fabricate** a win rate.
- **Sortino (2.51 / 1.20) and Calmar (5.68 / 1.85)** are in the
  characterization but have no KPI card → caption-text only, at architect
  discretion (A2). Do not invent KPI cards for them in v0.1.0.

## Acceptance criteria

Proportionate + testable. The tester closes the loop against these.

- **AC1 — Baseline screen renders the BH result.** With committed CSVs
  present, the `Screen::Baseline` body shows the year-2024 equity curve +
  drawdown band + the six-card KPI strip populated with the §Data
  contract values (Total return +91.04%, Sharpe +0.89, Max DD 48.95%).
  Toggling to 2023 swaps to the 2023 curve + metrics (+196.22%, +1.84,
  34.57%).
- **AC2 — Four panel states behave** (R4). A snapshot/unit test covers
  Loading, Ready, and **Error** (CSV-absent → `BASELINE_DATA_UNAVAILABLE`
  copy, no panic). Empty is covered if cheaply reachable (zero-point
  parse); otherwise documented as not-expected.
- **AC3 — Fixtures `cockpit` smoke passes** — first-frame render of the
  Baseline route, **no panic**, within the existing smoke window. The
  loader degrades to `Error` (never panics) when the CSV is absent in a
  fixtures-only checkout.
- **AC4 — Lumen-consistent** — `tests/consistency.rs` / `tests/contrast.rs`
  / `tests/layout_invariants.rs` stay green; **no hardcoded colors**
  (theme tokens only) and **no hardcoded strings** (all copy via
  `strings.rs` `BASELINE_*`). Renders in both themes.
- **AC5 — Honest caption** — the rendered caption conveys the bounded
  scope ("passive baseline; active ≤ passive in the reachable universe,
  this sample") and **does not** contain an "optimal" / "unbeatable"
  overclaim (R3 / A3). Asserted by a string-content test on
  `BASELINE_CAPTION`.
- **AC6 — Panel-snapshot test added** per the cockpit's 267-test
  panel-snapshot convention (a Baseline-screen snapshot in both themes),
  and the `SIDEBAR_GROUPS_PHASE_C` flatten-invariant test updated to
  include `Screen::Baseline`.
- **AC7 — No new crate edge, no new widget, no new theme token.** Review
  confirms `baseline/` is pure-`ui` reading committed files; the three
  render widgets are reused verbatim.

### Lint convention (pre-existing tech-debt — do not fix-all)

The `crates/ui` crate carries ~140 pre-existing pedantic clippy lints.
New Baseline code follows the **ui crate's existing lint convention**
(match the surrounding modules' allow-pattern); it does **not** introduce
new warnings, and it does **not** attempt to clear the pre-existing 140.
That is out of scope for this feature.

## Strings (`BASELINE_*` block — for the developer/ui-designer)

Per audit §4. Final wording is the ui-designer's call, but the honest-scope
constraint (R3/A3) is binding:

- `BASELINE_SIDEBAR_LABEL` = "Baseline"
- `BASELINE_HEADLINE` = "Passive baseline"
- `BASELINE_CAPTION` — equal-weight buy-and-hold across 10 large-cap
  pairs, bought once at year-open, never rebalanced; **honest finding:**
  "passive baseline; active ≤ passive in the reachable universe, this
  sample." (NOT "passive is optimal".)
- `BASELINE_YEAR_2023_LABEL` = "2023" / `BASELINE_YEAR_2024_LABEL` = "2024"
- `BASELINE_DATA_UNAVAILABLE` = error-state copy that tells the operator
  what to do next (e.g. "Baseline data isn't bundled in this build.
  Equity CSVs live at spec/runbooks/artifacts/.").
- `BASELINE_RISK_DETAIL` (architect addition, D1=c caption-only A2) =
  the optional Sortino/Calmar caption line, e.g. "Sortino 2.51 / Calmar
  5.68 (2023) · Sortino 1.20 / Calmar 1.85 (2024)." — rendered `FG_3`
  below the band. Surfaces the §7.1 metrics that have no KPI card. The
  ui-designer may template the per-year values or fold them into the
  caption; keep them out of the six fixed KPI cards (A2).
- KPI tooltips: reuse the existing `KPI_*` tooltips if present; add
  `BASELINE_*_TOOLTIP` only if a Baseline-specific gloss is needed.

## Adjustments to the ui-designer audit

The audit (`spec/dev-notes/cockpit-buildout-audit-2026-06-08.md`) is
~90% directly usable. Three corrections, grounded in the actual code:

- **A1 — KPI strip renders six FIXED cards, not the wireframe's labels.**
  The audit §4 wireframe shows "CAGR/Calmar" and a "Win rate" column with
  loose framing. The real `kpi_strip` (`crates/ui/src/widgets/kpi_strip.rs:3`)
  renders **Total return / CAGR / Sharpe / Max DD / Win rate / Trades** —
  CAGR (not Calmar) is the second card. Requirements (R1) use the actual
  widget contract.
- **A2 — No Sortino/Calmar KPI slot exists.** `core::BacktestMetrics` has
  no `sortino`/`calmar` field, and `kpi_strip` renders neither. The audit
  §3.1 wireframe implied a "Sortino-as-CAGR-slot" — that conflates two
  metrics. Resolution: surface CAGR in the CAGR card; Sortino/Calmar are
  caption-text-only (architect discretion). Pulling them into KPI cards
  is a widget change → out of scope.
- **A3 — Caption must NOT overclaim.** The audit's draft caption ended
  "…and none beat it." That overstates the **bounded** result (active ≤
  passive *in the reachable universe, this sample*) into a universal
  claim. Per the operator brief and the program's terminal verdict, the
  caption is corrected to the honest bounded form (R3). This is binding,
  not stylistic.

Everything else in the audit (the `Screen::Baseline` three-touchpoint
pattern, the `baseline/{state,loader}.rs` module shape, the
`registry_read.rs` K2 precedent, the four panel states, the four-state
smoke implication, the accessibility notes, the both-themes render) is
adopted as-is.

## Backtest Scenarios

N/A — this is a read-only UI feature. It surfaces an **already-shipped**,
already-anchored backtest result; it runs no new strategy and produces no
new backtest. The data it reads is the non-anchored realized-curve CSV
output of the existing characterization (`crates/backtest/examples/
passive_baseline_equity.rs`). Per CLAUDE.md, the baseline-equity-divergence
e2e gate applies to **strategy overlays / sizing modifiers**, which this
is not — there is no overlay, no sizing math, no decision variable.

## Implementation

_ui-designer solo (2026-06-08) — the dev‖ui split was collapsed; the loader
is a small pure-`ui` module._ Files created / changed below; T1–T10 all
landed (see `tasks.md`).

**New files:**

- `crates/ui/src/baseline/mod.rs` — feature module (re-exports).
- `crates/ui/src/baseline/loader.rs` — CSV → `EquitySeries` loader (T2) +
  embedded §7.1 metrics `const` (T3) + 13 unit tests (incl. the re-sync
  trip).
- `crates/ui/src/baseline/state.rs` — `BaselineScreenState` + boot-load
  helper (T4).
- `crates/ui/src/screens/baseline.rs` — the screen `view` (T5).
- `crates/ui/tests/baseline_error_state.rs` — Error-state headless render
  test (T8).

**Changed files:**

- `crates/ui/src/state.rs` — `Screen::Baseline` variant, `BaselineYear`
  enum (`Default = Y2024`), `baseline_screen_state` field (struct +
  `Default` + `ready` + `Debug`), `Message::BaselineSelectYear(BaselineYear)`
  + its pure `update` arm (T1).
- `crates/ui/src/theme.rs` — `Screen::Baseline` added to
  `SIDEBAR_ENTRIES_PHASE_A` + `SIDEBAR_GROUPS_PHASE_C` Work group (T7).
- `crates/ui/src/widgets/sidebar_nav.rs` — `label_for` arm (T7).
- `crates/ui/src/strings.rs` — `BASELINE_*` block + `all()` registry (T6).
- `crates/ui/src/shell.rs` — `Screen::Baseline => baseline::view` route
  (T5).
- `crates/ui/src/screens/mod.rs`, `crates/ui/src/lib.rs` — module decls.
- `crates/ui/src/bin/cockpit.rs`, `crates/ui/src/bin/cockpit_live.rs` —
  `baseline::load_into(&mut cockpit)` at boot (D2 — default screen
  unchanged).
- `crates/ui/tests/panel_snapshots.rs` — `mod baseline_screen` (T9) + the
  AC5 no-overclaim caption test.
- `crates/ui/tests/headless_emulator_smoke.rs` —
  `headless_emulator_paints_baseline_route` (AC3 belt-and-braces).

### Architect-design-vs-code corrections (flagged for the record)

1. **Timestamp parse.** The design specified `OffsetDateTime::parse(s, …)`
   with a `[year]-[month]-[day]T[hour]:[minute]Z` description. That returns
   `TryFromParsed(InsufficientInformation)` because `time` treats the
   trailing `Z` as a **literal** char, not an offset directive — so
   `OffsetDateTime` has no offset to bind. **Resolved:** parse to
   `PrimitiveDateTime` with the same description, then `.assume_utc()` (the
   `_utc` column + `Z` suffix make UTC exact). Also `time` 0.3.47 deprecated
   `FormatItem` → used `BorrowedFormatItem`. A unit test pins both the shape
   and the `Rfc3339`-rejects falsification.
2. **Metrics lifetime.** `kpi_strip::view` ties its returned `Element<'a>`
   to the input ref's lifetime, so a function-local `Ready(baseline_metrics())`
   can't outlive the returned screen element (E0515). **Resolved** the same
   way the `viewer` binary does (`bin/viewer.rs:102` borrows
   `&self.model.metrics`): `BaselineScreenState` carries `metrics_2023 /
   metrics_2024` materialized from the `const` at boot. The const remains
   the single source of truth; the re-sync test still guards it. (A slight
   relaxation of D1/T4's literal "metrics are NOT stored" — the *intent*
   "use the const, don't recompute/parse" is fully honored.)

The sidebar lock-step (`SIDEBAR_GROUPS_PHASE_C` flatten == `SIDEBAR_ENTRIES_PHASE_A`)
matched the code exactly — the flatten-invariant test passed once Baseline
was added to both consts in the same position.

## UI

_ui-designer (2026-06-08)._

### Wireframe (Baseline screen body)

```
┌──────────────────────────────────────────────────────────────────────┐
│ Passive baseline                                      [ 2023 ][ 2024◀] │  ← headline (H2) + year chips
│ Equal-weight buy-and-hold across 10 large-cap pairs, bought once at    │  ← caption (BODY, FG_2):
│ year-open and never rebalanced. Passive baseline; active ≤ passive     │    honest bounded scope,
│ in the reachable universe, this sample.                                │    NO overclaim (R3/A3)
│ ┌────────┬────────┬────────┬────────┬────────┬────────┐               │
│ │ Total  │ CAGR   │ Sharpe │ Max DD │ Win    │ Trades │               │  ← kpi_strip (6 FIXED cards)
│ │ return │        │        │        │ rate   │        │               │    from §7.1 const:
│ │ 91.04% │ 91.04% │ 0.8925 │−48.95% │   —    │   0    │               │    2024 default shown
│ └────────┴────────┴────────┴────────┴────────┴────────┘               │
│ ╱╲      realized BH equity curve (ACCENT line, UP_500 fill) ╱╲    ╱    │  ← equity_curve (~240px)
│╱  ╲╱╲╱╲╱                                              ╲╱╲╱  ╲╱╲╱       │
│ ▁▂▃▅▇█▆▄▃▂  drawdown band (DOWN_500, inverted Y)  ▁▂▃▄▅▆▇          │  ← drawdown_band (~100px)
│ Sortino 2.51 / Calmar 5.68 (2023) · Sortino 1.20 / Calmar 1.85 (2024) │  ← risk_detail (SMALL, FG_3)
└──────────────────────────────────────────────────────────────────────┘
```

Toggling `[2023]` swaps the curve + band + the six KPI values + the active
chip styling (`ACCENT` text on `PANEL_RAISED` bg). The caption +
risk-detail line are year-agnostic.

### New screens / panels / widgets

- **New screen:** `Screen::Baseline` (`screens::baseline::view`). Navigable
  from the Work sidebar group (after Compare); **not** default-routed (D2).
- **New widgets:** none. The three render widgets (`equity_curve`,
  `drawdown_band`, `kpi_strip`) are reused **verbatim** (AC7). The year
  chips reuse the Compare/Lab `Button`-chip pattern inline (not a new
  widget).

### New strings (`ui::strings`)

`BASELINE_SIDEBAR_LABEL` ("Baseline"), `BASELINE_HEADLINE` ("Passive
baseline"), `BASELINE_CAPTION` (honest bounded — see R3/A3),
`BASELINE_YEAR_2023_LABEL` ("2023"), `BASELINE_YEAR_2024_LABEL` ("2024"),
`BASELINE_DATA_UNAVAILABLE` (error-state copy with the artifacts path),
`BASELINE_RISK_DETAIL` (Sortino/Calmar caption line). All seven registered
in `strings::all()`. **Zero inline string literals** in the screen.

### New theme tokens

**Zero** (AC7 / R5). All chrome uses existing tokens: `FG_1`/`FG_2`/`FG_3`,
`ACCENT`, `PANEL_RAISED`, `BORDER_1`, `UP_500`/`DOWN_500` (via the reused
widgets), `text::{H2, BODY, SMALL}`, `space::{XS, S, M, L}`, `radius::R1`.
**Zero inline hex.**

### Accessibility notes

- **Keyboard:** the two year chips are iced `Button`s with `on_press` —
  Tab-reachable and Enter/Space-activatable (R2). The sidebar Baseline row
  is likewise a focusable nav button.
- **Color is never the only signal:** the active year chip pairs the
  `ACCENT` text colour with a `PANEL_RAISED` background + `ACCENT` border
  (shape), so the selection is distinguishable without colour. KPI
  sentiment colours pair with the sign-prefixed value text (`91.04%` vs
  `−48.95%`).
- **Contrast:** verified by `tests/contrast.rs` (green) — every token pair
  the screen uses is in the WCAG-asserted PAIRS table; no new pair
  introduced.
- **Both themes:** renders in `--theme dark` and `--theme light` for free
  (widgets are theme-correct); the both-theme snapshots + the both-theme
  headless render test prove it.
- **No blank screens:** all four `PanelState` arms are honoured — `Loading`
  (boot), `Ready` (populated), `Empty` (zero-row CSV), `Error` (CSV absent
  → helpful `BASELINE_DATA_UNAVAILABLE` copy + path, KPI strip still
  populated from the const = honest degrade).

## Verification

_tester links to reports here._

## Changelog

- 2026-06-08 (analyst): initial brief carved from the ui-designer
  build-out audit §3.1/§4 (candidate #1, operator-greenlit). Formalized
  into testable AC1–AC7. Flagged three audit corrections (A1 KPI six-card
  contract, A2 no Sortino/Calmar slot, A3 honest-caption no-overclaim),
  all grounded in the actual `kpi_strip` widget + `core::BacktestMetrics`
  struct. Teed up D1 (loader-live-vs-embed; analyst recommends live-load
  curve + architect-choice metrics, embed as if-budget-tightens fallback)
  and D2 (fixtures-smoke routing) for the architect. Opened REQ-COCKPIT-BASELINE-001
  (proposed). HANDOFF → architect.
- 2026-06-08 (architect): § Design authored; D1/D2 resolved; tasks.md
  written; trace REQ-COCKPIT-BASELINE-001 → arch-done. **D1 = (c)**
  embed the six §7.1 *realized* KPI scalars (+ caption Sortino/Calmar)
  as a typed `const`, file-load the curve. Decided on crate-edge facts,
  not budget: option (a) "compute from curve" is **infeasible** — there
  is NO Sharpe/CAGR/total-return math anywhere in `crates/core`
  (`compute_sharpe_hourly`/`compute_calmar` live only in `backtest` +
  `forecast`; the Lab strip renders Sharpe as `—` "engine not yet
  computing"), so (a) would need either a new `core` math module or an
  AC7-violating `backtest` reach — AND it is **semantically wrong**: the
  published Sharpe (+1.8417/+0.8925) is computed over 8,759/8,784 *hourly*
  bars, but the committed CSV is daily-sampled (~366 pts), so any
  re-derivation ≠ the number the operator reads. Even the free MaxDD from
  `from_points` is taken from the const for cross-surface consistency.
  Option (b) parse-the-`.md` rejected as fragile for zero freshness gain.
  Cost of (c) = a documented re-sync: a `// RE-SYNC:` const block + a
  `baseline_metrics_match_characterization` unit test + a runbook note.
  **D2 = navigable, not default-routed**: register `Screen::Baseline` in
  the sidebar (Work, after Compare) + route it, but keep the smoke's
  default screen on `Live` so the first-frame smoke gate stays
  deterministic regardless of CSV presence; the Error-state assertion
  lives in a dedicated headless test (`baseline_error_state_renders_without_panic`).
  Confirmed crate-edge surprise: `ui → backtest` **already exists**
  (ADR-0030, Lab Run button only); this feature adds NO new edge and does
  not use it. Added `BASELINE_RISK_DETAIL` to the strings block for the
  caption-only Sortino/Calmar (A2). No ADR — all decisions are pure-`ui`,
  additive, within the `viewer`/`registry_read` precedent. HANDOFF →
  developer ‖ ui-designer.
- 2026-06-08 (ui-designer, solo): T1–T10 all implemented (the dev‖ui split
  was collapsed — the loader is a small pure-`ui` module). § Implementation
  + § UI authored. All 7 ACs pass: AC1 (Baseline renders BH; 2024 default,
  toggle→2023 swaps curve+metrics), AC2 (four states incl. Error-no-panic),
  AC3 (fixtures cockpit headless smoke green — first-frame paint of the
  Baseline route + the default route, no panic), AC4 (consistency/contrast/
  layout_invariants green; zero new tokens; all copy via `strings`; both
  themes), AC5 (caption honest-bounded, no-overclaim string test), AC6
  (panel snapshots both themes + sidebar flatten-invariant updated), AC7
  (no new crate edge/widget/token — `baseline/` is pure-`ui` over `core` +
  `std::fs`). `cargo build -p ui` clean; new code clippy/fmt-clean per the
  crate convention; `verify_anchors` 119/119 (no anchored file touched).
  **Two architect-design-vs-code corrections** recorded in § Implementation:
  (1) the timestamp parse needs `PrimitiveDateTime` + `.assume_utc()` (not
  `OffsetDateTime::parse`, which `InsufficientInformation`-fails on the
  literal `Z`); (2) the KPI metrics are materialized on `BaselineScreenState`
  from the const (viewer-precedent lifetime fix) rather than a function-local.
  The sidebar lock-step matched the code exactly. HANDOFF → tester.
