---
slug: cockpit-baseline-panel
status: proposed
owner: analyst
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

_architect fills this. Two decisions to resolve are pre-stated below._

### Decision D1 (architect to resolve) — loader-reads-CSV-live vs embed-as-constants

The realized BH metrics (§ Data contract) can reach the panel two ways:

- **(a) Live-load (Recommended)** — `baseline/loader.rs` reads the two
  committed CSVs into `EquitySeries` **and** parses/maps the
  characterization metrics into `core::BacktestMetrics` at boot.
  - *Pro:* data-driven; the panel is the single source of truth and
    re-runs of the characterization flow through automatically; exercises
    a real Loading/Error path; matches the `registry_read.rs` precedent
    (reads committed files at runtime).
  - *Con:* couples the cockpit to a **non-anchored** artifact path
    (`spec/runbooks/artifacts/passive-baseline-2026-06-08/`); the path is
    date-stamped and could move; needs a robust Error fallback (which R4
    requires anyway).
- **(b) Embed metrics as typed constants** — the six metric values are
  hand-entered as a `const BacktestMetrics` (the equity curve still loads
  from CSV, since a `const` 367-point vector is unwieldy).
  - *Pro:* simplest; no metrics-parse failure mode; the metrics are
    byte-stable + anchored, so embedding is safe.
  - *Con:* goes **stale** silently if the characterization is ever
    re-run; splits the data source (curve from file, metrics from code);
    a second source of truth to keep honest.

**Analyst recommendation: (a) live-load**, but **hybrid is acceptable
and may be the durable sweet spot**: live-load the *equity curve* from
CSV (R1 needs the 367 points anyway), and let the architect choose
whether the *six metric scalars* are parsed-from-the-`.md`-table or
embedded-as-`const`. The curve is the bulky data and must be file-driven;
the six scalars are the only thing in tension. Live-load keeps the panel
data-driven (the program's stated value: the cockpit reflects the shipped
result, not a hand-copied snapshot) at the cost of a path coupling that
the **mandatory Error state already isolates** — a missing/moved CSV
degrades to the honest "data isn't bundled in this build" copy, never a
crash. Embedding is the **if-budget-tightens fallback**: it removes the
metrics-parse path but commits to a stale-data follow-up the next time
the characterization changes.

**If budget tightens:** ship (b)-for-metrics + (a)-for-curve (embed the
six scalars, file-load the curve). This is the smaller blast radius and
still satisfies every acceptance criterion; it adds a "re-sync metrics
const if characterization re-runs" maintenance note.

The architect locks D1 in § Design and records the rationale in the
Changelog.

### Decision D2 (architect to resolve) — fixtures-smoke routing

Adding a `Screen::Baseline` route means the fixtures `cockpit` should be
able to paint it inside the ~7 s smoke window. The architect decides
whether the smoke **default-routes** to Baseline or merely makes it
**navigable** (and whether the smoke asserts the Error-state render,
since the CSVs may be absent in a minimal checkout). Either satisfies R7;
the constraint is first-frame render + no panic.

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

_developer ‖ ui-designer fill this._

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
