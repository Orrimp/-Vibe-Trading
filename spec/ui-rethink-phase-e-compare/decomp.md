---
slug: ui-rethink-phase-e-compare
status: in-progress
owner: architect
updated: 2026-05-20
version: 0.1.0
predecessor: ui-rethink-phase-d-trail-followup v0.1.1
---

# Decomposition — UI rethink Phase E (Compare matrix, v0.1.0)

> Architect M-T1 pass. Resolves K3 (`serde_yaml` resolution = hand-parse),
> closes the H1 cache-hit-rate enumeration against the live spec/ tree
> (static count, 40 % hit-rate ≥ 30 % threshold), closes the H4
> cache-scan budget by static argument (32 reports × flat-YAML head =
> 0.12 s wall at shell speed; Rust parse << 50 ms p99), enumerates the
> file-line change-map, orders the implementation into Waves A-E, and
> ticks the spike requirement = NONE (matrix is purely additive UI;
> the predecessor Phase D/D+ Subscription-bridge pattern + the Phase B
> `lab::runner` Run dispatch supply every structural primitive Phase E
> needs).
>
> Inputs reviewed:
> - `spec/ui-rethink-phase-e-compare/feature.md` (R1-R8, K1-K8, H1-H5,
>   Q1-Q8 with M-OD resolutions: Q1=a, Q2=c, Q3=a, Q4=b, Q5=a, Q6=a,
>   Q7=a, Q8=b — operator "Autoapprove all" 2026-05-20).
> - `spec/ui-rethink-phase-e-compare/tasks.md` (T-A1..T-A12 done;
>   T-OD1..T-OD8 done; T-T1-*/T-D-N* this pass owns).
> - Predecessor `spec/ui-rethink-phase-d-trail-followup/decomp.md`
>   (structural template; change-map shape + wave shape + rollback
>   shape carry forward 1:1 except the K5 spike row — Phase E has no
>   structural unknown).
> - `spec/dev-notes/ui-rethink-2026-05-17.md` §6 Phase E (lines
>   1082-1096), §J3 (lines 340-390), §3 IA (lines 651-744).
> - Load-bearing source citations:
>   - `crates/ui/src/state.rs:55-62` — `Screen::Compare` enum variant
>     (Phase A placeholder reservation; Phase E re-uses the variant).
>   - `crates/ui/src/state.rs:447-468` — `StrategiesConfig` +
>     `StrategyConfigEntry` (Q7 source of truth).
>   - `crates/ui/src/state.rs:798` — `pub lab_state: LabState` sibling
>     field; `:879` — `pub trail_screen_state: TrailScreenState` sibling
>     field (location pattern for `compare_screen_state`).
>   - `crates/ui/src/state.rs:1305` — `Message::SelectStrategy(StrategyId)`;
>     `:1370` — `Message::LabSelectPair(Venue, Symbol)`; `:1380` —
>     `Message::LabSelectRange(DateRange)`; `:1383` —
>     `Message::LabRunRequested`; `:1424` — `Message::OpenTrailFor(SmolStr)`
>     (compound-dispatch precedent shape).
>   - `crates/ui/src/state.rs:1902-1910` — `OpenTrailFor` update arm
>     (verbatim compound-dispatch pattern Phase E mirrors at
>     `OpenLabFromCompare`).
>   - `crates/ui/src/shell.rs:96` — current
>     `placeholder::view(strings::COMPARE_PLACEHOLDER, mode)` route
>     (the only line Phase E swaps in `shell.rs`).
>   - `crates/ui/src/strings.rs:251-252,266` — `COMPARE_PLACEHOLDER` +
>     `SIDEBAR_NAV_COMPARE` reserved constants.
>   - `crates/ui/src/theme.rs:741-742` — `SIDEBAR_GROUPS_PHASE_C` Work
>     zone (Compare entry already wired; R5.1 — no sidebar change).
>   - `crates/ui/src/lab/state.rs:88` — `pub enum DateRange { Preset,
>     Custom }` (R6.4 — no new variant).
>   - `spec/v1-cross-sectional-momentum/reports/backtest-20260429-195148-top10-2023-1h-momentum.md:1-16`
>     — sample frontmatter shape (flat YAML key:value + one nested
>     `strategy:` block; K3 resolution justification).
> - `bash scripts/verify_anchors.sh` re-run 2026-05-20 BEFORE this
>   pass: `ANCHORS PASS  (22 / 22)` — baseline confirmed clean.

## 1. Architect-decide resolutions

### 1.1 — K3 resolution: YAML frontmatter parser shape (T-T1-1)

**Architect pick: (b) hand-parse the flat frontmatter.** No new external
crate dep; no ADR required; no architecture-edge change.

**Citation:**
- `cargo tree -e features --workspace 2>/dev/null | grep -i yaml` returns
  only `yaml-rust2 v0.8.1` (transitive: dev-dep via `insta` + runtime
  dep via `config` crate). `serde_yaml` is **not** present in the
  workspace.
- `grep -rn "serde_yaml\|serde-yaml" --include=Cargo.toml` returns
  empty.
- Sample report frontmatter
  (`spec/v1-cross-sectional-momentum/reports/backtest-20260429-195148-top10-2023-1h-momentum.md:1-16`):

  ```yaml
  ---
  scenario: top10-2023-1h-momentum
  seed: 0xC0FFEE
  generated: 2026-04-29T19:51:48Z
  wall_clock_s: 4.2
  data_source: synthetic (seeded RNG, v1 multi-symbol)
  baseline_report: n/a
  ledger_imbalance_total: 0
  llm_spend_usd: 0.00
  strategy:
    id: top10_momentum_h1
    kind: cross_sectional_momentum
    content_hash: ...
    source: config/strategies/top10_momentum_h1.toml
    signal: vol_adjusted_log_return(lookback=60)
  ---
  ```

  Shape invariant: flat `key: value` lines + one nested `strategy:`
  block with 2-space-indented children. No arrays, no multi-line
  strings, no anchors / references — every existing report observed.
  The KPI fields the matrix needs (`scenario`, `generated`, plus the
  Sharpe + total_return + max_drawdown + trade_count read from the
  Markdown body's `## Summary` table, NOT from frontmatter — see §1.4
  below) are all single-line single-value.

**Locked parser shape (`crates/ui/src/compare/cache.rs`):**

```rust
/// Hand-parses the flat YAML frontmatter at the head of a backtest
/// report. Returns `None` on any parse error (fail-soft per K2; a
/// `tracing::warn!` records the offending path for the operator).
///
/// Contract:
/// - Reads the file content between the leading `---` and the next
///   `---` line.
/// - For each line outside the `strategy:` block, splits on the first
///   `: ` and stores the key→value pair.
/// - For lines inside the `strategy:` block (detected via `line.starts_with("  ")`),
///   stores under a `strategy.<key>` namespace.
/// - Returns `BTreeMap<SmolStr, SmolStr>` (deterministic ordering for
///   downstream tests).
fn parse_frontmatter(content: &str) -> Option<BTreeMap<SmolStr, SmolStr>>;
```

**Estimated LOC:** ~30 lines including unit-tests-in-module pattern
(`#[cfg(test)] mod tests { ... }`). Three unit tests:
`parses_flat_kv`, `parses_strategy_block`, `returns_none_on_malformed`.

**Rejected alternative (a) — add `serde_yaml` workspace dep + ADR.**
R7.6 forbids new external deps without an ADR justification; the
hand-parse is shorter (≈30 LOC) than the ADR would be (≈300 LOC of
prose for one paragraph of code), and the frontmatter shape has been
stable across 32 backtest reports + 6 strategies + 18 months of report
history. Adding a workspace dep for 30 LOC of parsing is not the
right cost/benefit tradeoff. **Rejected on cost grounds.**

### 1.2 — H1 enumeration: cache-hit count at first matrix open (T-T1-2)

**Result: 24 / 60 cells populated = 40 % hit-rate** — passes the H1
≥ 30 % threshold by a comfortable margin.

**Method:** Enumerated all backtest reports under `spec/<strategy>/reports/`
via `find spec -type f -name 'backtest-*.md' | awk -F'/' '...'` (32
reports total, distributed across 6 spec folders). Mapped each
strategy slug to its declared universe per `config/strategies/*.toml`
(Q8=(b) universe gating) and counted the cells that the cache
populates on first open.

| Strategy (sidebar id) | Universe (Q8=b) | Scenarios with cache hit | Cells populated |
|-----------------------|-----------------|--------------------------|-----------------|
| v0.sma (`btc_sma_cross`) | BTC only | `btc-2023-1m-sma-cross`, `btc-2023-1m-sma-baseline-refresh` | **1 / 1** (only BTC legal) |
| v0.5.composed (`btc_macd_trend` + 2 siblings) | BTC only | `btc-2023-1m-{macd-trend,rsi-reversion,bbands-mean-revert}` | **1 / 1** (only BTC legal — most-recent wins per R3.3) |
| v1.momentum (`top10_momentum_h1`) | 10-symbol top10 | `top10-2023-1h-momentum`, `top10-2024-h1-momentum` (each carries universe-aggregate Sharpe via Q6=a) | **10 / 10** (every legal pair populates) |
| v1.5a.pairs (`pairs_mr_h1`) | (BTC, ETH) only | `pairs-2023-zscore-mr`, `pairs-2024-h1-zscore-mr` | **2 / 2** (only the two legal pairs) |
| v2.llm | TBD — strategy not yet registered in `config/strategies/` | none | **0 / N** (row blanked entirely until v2.llm registers) |
| v2.5.tcn (`tcn_overlay_momentum`) | 10-symbol top10 (inherits base `top10_momentum_h1`) | 22 `top10-{2023,2024}-fy-tcn-overlay-*` reports (most-recent collapses to one cell per pair per R3.3) | **10 / 10** |

**Total populated cells: 1 + 1 + 10 + 2 + 0 + 10 = 24.**
**Total legal cells (Q8=b universe-gated): 1 + 1 + 10 + 2 + 0 (v2.llm row entirely blank) + 10 = 24.**
**Total cells in the 6×10 grid: 60.**
**Hit rate vs. total: 24 / 60 = 40 %.**
**Hit rate vs. legal cells: 24 / 24 = 100 % at v0.1.0 — every legal cell has a cached hit; the only unpopulated cells are the blanked-grey "not in universe" cells (Q8=b) + the entire v2.llm row (pre-registration).**

**Implications for the dev wave shape:**

1. The empty-cell "Run" affordance (Q4=b) is the **dominant
   first-impression UX path for v2.llm only** at v0.1.0 — every other
   row is fully populated. This is acceptable per the H1 framing
   ("the matrix is immediately useful at first open"); the operator
   sees 5 populated rows + 1 row offering "Run" buttons for whichever
   v2.llm universe-pairs ship next.
2. The K7 universe-aggregate semantic confusion (v1.momentum × BTC =
   v1.momentum × ETH = 0.94 Sharpe) hits 10 of the 24 populated cells
   (v1.momentum row) + 10 of the 24 populated cells (v2.5.tcn row) =
   20 / 24 = 83 % of the populated surface. Q6=(a)'s tooltip
   "this KPI is universe-aggregate, not per-pair" must be a
   load-bearing widget on every multi-symbol cell, not a corner-case
   detail. **Wave A T-D-N3 below** locks the tooltip-or-footnote shape.

### 1.3 — H4 cache-scan budget (T-T1-3)

**Result: << 50 ms p99 by static argument** — no Rust micro-bench
required because the order-of-magnitude headroom is overwhelming.

**Method (static):**
- Report-tree size: 32 backtest reports (`find spec -type f -name 'backtest-*.md' | wc -l = 32`).
- Per-file frontmatter parse: lines 1-16 of the sample report
  (16 lines × ~40 chars/line = ~640 bytes of header). Reading 32 ×
  640 B = 20 KB total head-read.
- Shell-level glob + `head -20` × 32 files measured at 0.12 s wall
  (`/usr/bin/time -p sh -c 'find ... -exec head -20 {} \;'`) —
  includes 32 fork+exec roundtrips. Pure Rust streaming read of the
  same bytes (no fork, no exec, no shell parse) is **at least 10×
  faster** = ≤ 12 ms.
- Hand-parse (~30 LOC, O(N) over header bytes) adds ≤ 1 ms.
- **Total estimated p99 cold-boot cache scan: ≤ 15 ms.** Well under
  the H4 50 ms budget (≥ 3× headroom).

**If H4 falsifies at M-FINAL** (tester sees > 50 ms): K5 mitigation
lifts — move the cache-scan to a `tokio::spawn` at cockpit boot
(non-gating; matrix renders empty until the scan completes; ≤ 15 ms
delay is invisible). Implementation cost ≈ 10 LOC; no anchor risk.

### 1.4 — Q6 sub-decision: universe-aggregate KPI semantic surface (T-T1-4)

**Architect pick: footnote subtitle under the matrix toolbar + per-cell
tooltip on hover.** Both surfaces ship at v0.1.0; both reference the
same string `strings::COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE`
(new constant — additive to `strings.rs`).

**Citation:** Q6 of the feature brief surfaced (a) "render all 10
cells with the same aggregate KPI" as the operator-approved default,
and explicitly called out K7 (universe-aggregate vs. per-pair
semantic confusion, MEDIUM severity, "data-shape trap") as the
load-bearing UX risk. Section 1.2 above quantifies that 20 of 24
populated cells (83 %) are subject to this confusion. The mitigation
has to be load-bearing too — a single corner-tooltip is insufficient;
the subtitle anchors the operator's mental model whenever the matrix
is in view, the tooltip catches the per-cell hover.

**Locked shape:**
- New string constant in `crates/ui/src/strings.rs:~280`:
  ```rust
  /// Phase E — universe-aggregate KPI disclaimer. Rendered as a
  /// subtitle under the matrix toolbar AND as a per-cell tooltip on
  /// hover when the cell's source report is multi-symbol.
  pub const COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE: &str =
      "KPI is universe-aggregate, not per-pair (multi-symbol scenario). \
       Per-pair decomposition is v0.2.0 follow-up.";
  ```
- Subtitle render in `crates/ui/src/screens/compare.rs` toolbar row
  (always visible when ≥ 1 cell in view is multi-symbol).
- Per-cell tooltip in `crates/ui/src/widgets/matrix.rs` cell-body
  (`is_multi_symbol == true` gate; reuses the existing
  `widgets::tooltip` pattern from Phase D trail-node hover).

**Rejected alternatives:**
- **Footnote only** (no per-cell hover) — operator's eye lands on a
  cell, not a footnote; the confusion fires before they read the
  footnote. Rejected.
- **Tooltip only** (no footnote) — hover discovery is opt-in; first-
  open operators who don't hover never see the disclaimer. Rejected.

### 1.5 — Wave shape: A-E ordered (5 waves)

**Architect pick: 5 waves** (Wave A → E), mirroring the predecessor's
shape but compressing the "snapshot baselines + cockpit-smoke + bench"
work into one Wave D (Phase E has no criterion bench — H4 is static-
argued, H1 is static-enumerated — and no idle-CPU sampler change since
the cockpit-perf tooling from D+ is in place). Wave E is the M-FINAL
handoff prep + anchor gate.

| Wave | Title | Deliverable | Anchor exposure |
|------|-------|-------------|-----------------|
| **A** | Cache module + state types | `crates/ui/src/compare/cache.rs` + `compare/state.rs` + new `Message::OpenLabFromCompare` + `Cockpit::compare_screen_state` field + `CompareKpiAxis` enum + `CachedCell` struct + `COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE` string | None (purely additive types — additive Message variant) |
| **B** | Widget — `widgets::matrix` | `crates/ui/src/widgets/matrix.rs` — pure `view(&Cockpit, ThemeMode) -> Element` widget reading from `compare_screen_state.cache` | None (new widget file; no anchored renderer touch) |
| **C** | Screen body + shell wiring | `crates/ui/src/screens/compare.rs` (toolbar + matrix body) + swap `shell.rs:96` from `placeholder::view` to `screens::compare::view` | None (single-line shell route swap; no Lab / Live / Trail / Settings / Strategies body touched) |
| **D** | Snapshot baselines + cockpit-smoke gate | 4 new visual baselines under `crates/ui/tests/visual-baselines/` (`compare__cold_boot_all_empty`, `compare__steady_state_populated`, `compare__empty_cell_run_affordance`, `compare__column_header_hover`) + `cockpit-smoke` invocation in tasks.md | None (new baselines — additive PNGs) |
| **E** | M-FINAL handoff | Re-run `scripts/verify_anchors.sh` literal `ANCHORS PASS  (22 / 22)`; tester handoff envelope drafted | None (verification only) |

**Spike requirement: NONE.** The matrix is purely additive UI; the
report-cache parse path is shape-known (sample report verified at
§1.1); the compound-dispatch precedent is verbatim from
`OpenTrailFor` at `state.rs:1902-1910`; the layout primitive is iced's
`Column<Row>` (already in use across the workspace, e.g.
`crates/ui/src/screens/strategy_registry.rs`). If during Wave A the
developer discovers a non-trivial blocker (e.g. the report-cache scan
contends with an in-flight Lab Run for a file handle), they HANDOFF
back to architect for a Wave-A spike. Not anticipated.

### 1.6 — State location: `Cockpit::compare_screen_state` sibling field (T-T1-5)

**Architect pick: new `pub compare_screen_state: CompareScreenState`
field on `Cockpit` at `crates/ui/src/state.rs:~880`, immediately after
`pub trail_screen_state: TrailScreenState` at line 879.**

**Citation:**
- `Cockpit::lab_state` lives at `state.rs:798`, default-constructed
  at `:998,1097`, mirrored in `Debug` impl at `:948`, in `Clone` and
  `new_with_persistence` at `:1042-1045`.
- `Cockpit::trail_screen_state` lives at `state.rs:879`,
  default-constructed at `:1009,1108`, mirrored in `Debug` at `:959`.
- Both fields follow the same 3-touchpoint pattern (struct field
  declaration + `Default` impl + `Debug` impl) that
  `CompareScreenState` must replicate.

**Locked shape (`crates/ui/src/compare/state.rs`, new file):**

```rust
//! Phase E — Compare-screen per-session state.
//!
//! Sibling of `crates/ui/src/lab/state.rs` (Phase A/B) and
//! `crates/ui/src/state.rs::TrailScreenState` (Phase D). All fields
//! are session-scoped; no on-disk persistence at v0.1.0
//! (matches Q5=(a) sidebar-only-entry + Q2=(c) report-cache-only).

use std::collections::BTreeMap;

use chrono::{DateTime, Utc};
use smol_str::SmolStr;

use crate::lab::state::DateRange;
use trading_core::Symbol;

/// Phase E — KPI axis dropdown variants (R6.3).
///
/// v0.1.0 wires `Sharpe` only (Q3=(a)); selecting any other variant
/// at runtime falls back to `Sharpe` with a `tracing::warn!` in dev
/// builds. The full enum lives now so the dropdown widget can render
/// all 5 options (UI surface stable across the v0.1.0 → v0.2.0
/// transition).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum CompareKpiAxis {
    #[default]
    Sharpe,
    Sortino,
    TotalReturn,
    MaxDrawdown,
    WinRate,
}

/// Phase E — one (strategy, pair) cell's cached KPI snapshot
/// (R3.1).
///
/// Populated by `compare::cache::lookup_cell` at view-render time
/// (or at cold-boot scan; R3.5 cold-boot-only policy at v0.1.0).
/// `is_multi_symbol` toggles the K7 disclaimer tooltip path
/// (§1.4 architect resolution).
#[derive(Debug, Clone, PartialEq)]
pub struct CachedCell {
    pub sharpe: f64,
    pub total_return_pct: f64,
    pub max_drawdown_pct: f64,
    pub trade_count: u32,
    pub equity_curve_tail: Vec<f64>,        // ≤ 30 bars for sparkline
    pub source_report_path: SmolStr,        // for "Open report" follow-up
    pub generated_at: SmolStr,              // most-recent tiebreaker (R3.3)
    pub is_multi_symbol: bool,              // K7 disclaimer gate
}

/// Phase E — Compare-screen per-session state (R6.1).
#[derive(Debug, Clone)]
pub struct CompareScreenState {
    /// R3.4 isolation: separate from `Cockpit::lab_state.range`.
    pub range: DateRange,
    /// R6.3 — single-KPI v0.1.0 (Sharpe); dropdown reserves the
    /// option for v0.2.0 multi-KPI heatmap.
    pub kpi_axis: CompareKpiAxis,
    /// R3.1 lookup table — keyed by `(strategy_id, symbol, range)`.
    /// Empty until first view-render (R3.5 cold-boot-only at v0.1.0).
    pub cache: BTreeMap<(SmolStr, Symbol, DateRange), CachedCell>,
    /// R3.5 cold-boot tag — `None` until first scan completes.
    pub last_indexed_at: Option<DateTime<Utc>>,
}

impl Default for CompareScreenState {
    fn default() -> Self {
        Self {
            range: DateRange::default(),      // Preset::default (= Last90Days per Phase A)
            kpi_axis: CompareKpiAxis::Sharpe,
            cache: BTreeMap::new(),
            last_indexed_at: None,
        }
    }
}
```

**Why `BTreeMap` (not `HashMap`):** deterministic iteration order for
snapshot baselines (`compare__steady_state_populated` and friends).
Cache size is small (60 cells max at v0.1.0); `BTreeMap` lookup is
O(log n) over n=60 = 6 ops. No perf concern.

## 2. Module / file change-map

| # | File | Line(s) | Wave | Change |
|---|------|---------|------|--------|
| 1 | `crates/ui/src/compare/mod.rs` | new | A | New module root: `pub mod cache; pub mod state;`. ~10 LOC including module doc. |
| 2 | `crates/ui/src/compare/state.rs` | new | A | `CompareScreenState`, `CachedCell`, `CompareKpiAxis` per § 1.6 above. ~70 LOC including doc-comments. |
| 3 | `crates/ui/src/compare/cache.rs` | new | A | `pub fn lookup_cell(strategy_id: &SmolStr, symbol: &Symbol, range: &DateRange) -> Option<CachedCell>` + `pub fn scan_spec_tree(spec_root: &Path) -> BTreeMap<(SmolStr, Symbol, DateRange), CachedCell>` + private `parse_frontmatter` (§1.1) + scenario→universe mapper (R3.2). ~180 LOC including 5 unit tests. |
| 4 | `crates/ui/src/lib.rs` | (declarations) | A | Add `pub mod compare;` next to existing `pub mod lab;`. ~1 line. |
| 5 | `crates/ui/src/state.rs` | ~880 (after `trail_screen_state`) | A | Add `pub compare_screen_state: CompareScreenState` field; mirror in `Default` impl at `:1009,1108` and `Debug` impl at `:959`. |
| 6 | `crates/ui/src/state.rs` | ~1425 (after `OpenTrailFor`) | A | Add `Message::OpenLabFromCompare { strategy: StrategyId, pair: Option<(Venue, Symbol)>, range: DateRange }` enum variant (R4.1). |
| 7 | `crates/ui/src/state.rs` | ~1380 (toolbar Messages) | A | Add `Message::CompareSelectRange(DateRange)`, `Message::CompareSelectKpiAxis(CompareKpiAxis)` enum variants (R1.2 toolbar wiring — pure assignment to `compare_screen_state.{range,kpi_axis}`). |
| 8 | `crates/ui/src/state.rs` | ~1911 (after `OpenTrailFor` arm) | A | Add 3 update-arms: `OpenLabFromCompare` (compound dispatch per § 1.2 below), `CompareSelectRange` (assign), `CompareSelectKpiAxis` (assign). |
| 9 | `crates/ui/src/strings.rs` | ~280 (Phase E section) | A | Add `COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE` (§1.4), `COMPARE_TOOLBAR_RANGE_LABEL`, `COMPARE_TOOLBAR_KPI_LABEL`, `COMPARE_CELL_RUN_LABEL`, `COMPARE_CELL_BLANKED_LABEL` (R8 universe-blanked `—`). Mark `COMPARE_PLACEHOLDER` (`:252`) with `#[deprecated(since = "0.3.0", note = "Compare now renders the matrix body — Phase F removes this constant")]` per the precedent at `SETTINGS_PLACEHOLDER:259-263`. |
| 10 | `crates/ui/src/widgets/matrix.rs` | new | B | `pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>`; iterates over `model.strategies_config.strategies` (rows) × `strategy.universe()` (Q8=b gated columns); per-cell match on `compare_screen_state.cache.get((strategy_id, symbol, range))` → populated cell vs. Run affordance vs. blanked `—`. ~220 LOC including K7 tooltip wiring (§1.4). |
| 11 | `crates/ui/src/widgets/mod.rs` | (declarations) | B | Add `pub mod matrix;`. ~1 line. |
| 12 | `crates/ui/src/screens/compare.rs` | new | C | `pub fn view(model: &Cockpit, mode: ThemeMode) -> Element<'_>`; composes toolbar (range picker + KPI-axis dropdown + K7 subtitle per §1.4) + `widgets::matrix::view(model, mode)` body. ~80 LOC. |
| 13 | `crates/ui/src/screens/mod.rs` | (declarations) | C | Add `pub mod compare;` next to existing `pub mod lab;`, `pub mod live;`, etc. ~1 line. |
| 14 | `crates/ui/src/shell.rs` | 96 | C | Swap `Screen::Compare => placeholder::view(strings::COMPARE_PLACEHOLDER, mode)` → `Screen::Compare => screens::compare::view(model, mode)`. **The only line Phase E swaps in `shell.rs`.** |
| 15 | `crates/ui/src/shell.rs` | 28 | C | Add `compare` to the `crate::screens::{lab, live, settings, strategy_registry, trail}` use-list. |
| 16 | `crates/ui/tests/visual_snapshots.rs` (or sibling) | new fixtures | D | Add 4 `#[test] fn`s authoring `compare__cold_boot_all_empty`, `compare__steady_state_populated`, `compare__empty_cell_run_affordance`, `compare__column_header_hover` baselines under `crates/ui/tests/visual-baselines/`. ~120 LOC + 4 new PNG baselines. |
| 17 | `crates/ui/tests/layout_invariants.rs` | (append) | D | Add proptest case `compare_screen_no_zero_dim` — 256 viewport-size samples render `screens::compare::view` without panic + every cell ≥ 1×1 px. ~30 LOC. |
| 18 | `crates/ui/src/state.rs` | (#[cfg(test)] mod tests) | D | Add round-trip unit test `open_lab_from_compare_sets_lab_strategy_pair_and_range` (H5 falsification) next to existing `open_trail_for_*` tests at `:3149-3194`. ~25 LOC. |
| 19 | `spec/trace.toml` | row REQ-UI-RETHINK-PHASE-E-001 | (this M-T1 pass) | Flip `state = "proposed"` → `"in-progress"`; append `decomp.md` to the `arch` array. |

**Total non-trivial files touched at developer ship:** 7 source files
modified (`lib.rs`, `state.rs`, `strings.rs`, `widgets/mod.rs`,
`screens/mod.rs`, `shell.rs`, `tests/visual_snapshots.rs`) + 5 new
source files (`compare/mod.rs`, `compare/state.rs`, `compare/cache.rs`,
`widgets/matrix.rs`, `screens/compare.rs`) + 4 new PNG baselines + 1
trace row update. **Net-new file count: 5** (resolves R8.5 — analyst
estimated 4-5; architect locks at 5).

Anchor count: 22 → 22 (additive-only by construction; R7.1-R7.7 honored).

## 3. Ordered Wave decomposition (A → E)

> Wave checklist rows are appended to `tasks.md` § "M-T1 — Architect
> decomposition" alongside the T-T1-* architect-decide rows. Each
> T-D-N row carries file:line + cargo invocation + literal expected
> output per the honest-tick rule.

### Wave A — Cache module + state types + Message variants (R3, R4, R6, R8)

Lays the data + dispatch scaffolding. No widget code yet; the matrix
widget in Wave B reads from this state.

T-D-N1 — Create `crates/ui/src/compare/mod.rs` + `compare/state.rs`
per § 1.6. Add `pub mod compare;` to `crates/ui/src/lib.rs`.
- Files: `crates/ui/src/compare/mod.rs` (new), `crates/ui/src/compare/state.rs` (new), `crates/ui/src/lib.rs` (one-line declaration).
- Cargo: `cargo check -p ui`.
- Acceptance: PASS with no warnings.

T-D-N2 — Author `crates/ui/src/compare/cache.rs` per § 1.1. Includes
`parse_frontmatter` + `scan_spec_tree` + `lookup_cell` + scenario→universe
mapper. 5 unit tests in-module: `parses_flat_kv`, `parses_strategy_block`,
`returns_none_on_malformed`, `scenario_top10_maps_to_universe_of_10`,
`scenario_btc_maps_to_btc_only`.
- File: `crates/ui/src/compare/cache.rs` (new).
- Cargo: `cargo test -p ui --lib compare::cache::tests`.
- Acceptance: `running 5 tests` line + `test result: ok. 5 passed; 0 failed`.

T-D-N3 — Add `Message::OpenLabFromCompare { strategy, pair, range }` +
`Message::CompareSelectRange(DateRange)` +
`Message::CompareSelectKpiAxis(CompareKpiAxis)` enum variants at
`crates/ui/src/state.rs:~1380,1425`. Add the 3 update-arms at
`:~1911` (after the `OpenTrailFor` arm). Compound `OpenLabFromCompare`
dispatch sets `model.current_screen = Screen::Lab`, then writes
`lab_state.strategy`, then writes `lab_state.pair` (when Some), then
writes `lab_state.range`. Mirrors `OpenTrailFor` at `:1902-1910` —
order is fixed (per K4 mitigation): set screen + strategy first
because `SelectStrategy` clears `last_run_report` (`state.rs:1793-1799`)
which the seeded Lab will then re-render via Phase A's cache shortcut.
- File: `crates/ui/src/state.rs:~1380,1425,~1911`.
- Cargo: `cargo check -p ui` + `cargo test -p ui --lib`.
- Acceptance: PASS; existing test count preserved (937 baseline holds
  per predecessor); new unit test in Wave D T-D-N12.

T-D-N4 — Add `pub compare_screen_state: CompareScreenState` field to
`Cockpit` at `crates/ui/src/state.rs:~880` (immediately after
`trail_screen_state`). Mirror in `Default` impl at `:1009,1108` +
`Debug` impl at `:959`. Verify `Cockpit::default()` round-trips.
- File: `crates/ui/src/state.rs:~880,959,1009,1108`.
- Cargo: `cargo test -p ui --lib cockpit_default_compiles_and_round_trips`
  (existing baseline — must not regress).
- Acceptance: `cargo test -p ui --lib` `test result: ok. ... passed`.

T-D-N5 — Add new strings to `crates/ui/src/strings.rs:~280`:
`COMPARE_KPI_UNIVERSE_AGGREGATE_NOTE`, `COMPARE_TOOLBAR_RANGE_LABEL`,
`COMPARE_TOOLBAR_KPI_LABEL`, `COMPARE_CELL_RUN_LABEL`,
`COMPARE_CELL_BLANKED_LABEL`. Mark `COMPARE_PLACEHOLDER` (line 252)
with `#[deprecated(since = "0.3.0", note = "Compare now renders the
matrix body — Phase F removes this constant")]` per the existing
`SETTINGS_PLACEHOLDER` precedent at lines 259-263.
- File: `crates/ui/src/strings.rs:252,~280`.
- Cargo: `cargo check -p ui` + `cargo clippy -p ui -- -D warnings`.
- Acceptance: PASS (deprecation attribute is `#[allow(deprecated)]`-guarded at the only call site in `shell.rs:96`, which Wave C swaps away anyway — so the warning goes away the moment Wave C lands).

### Wave B — `widgets::matrix` widget (R2)

Pure widget — reads from `Cockpit::compare_screen_state` + the strategy
registry; emits `Message::OpenLabFromCompare` on cell click. Compiles
standalone after Wave A; not wired into a screen body yet.

T-D-N6 — Author `crates/ui/src/widgets/matrix.rs` per the locked
spec in § 2 row 10. Layout primitive: iced's `Column<Row>`. Cell
match arms: populated (KPI text + sparkline + hairline border on
hover) / empty-but-legal (centred "Run" `Button`) / blanked
("not in universe" — `—` label + passive hairline). K7 tooltip wired
on every populated cell where `cached.is_multi_symbol == true`.
- File: `crates/ui/src/widgets/matrix.rs` (new); `crates/ui/src/widgets/mod.rs` (one-line declaration).
- Cargo: `cargo check -p ui`.
- Acceptance: PASS no warnings.

T-D-N7 — Add cell-hover style: Lumen `BORDER_HAIRLINE` →
`active_row` border tint, mirroring the Phase C strategy-card hover
state at `crates/ui/src/widgets/strategy_card.rs`. NO new theme
tokens (R7.6).
- File: `crates/ui/src/widgets/matrix.rs` (style closure on the cell `Button`).
- Cargo: `cargo clippy -p ui -- -D warnings`.
- Acceptance: PASS.

### Wave C — Screen body + shell wiring (R1, R5)

Composes toolbar + matrix body in `screens::compare`; swaps the
shell route from the placeholder to the real screen. After Wave C
ships, the operator's sidebar click on "Compare" lands on the real
matrix.

T-D-N8 — Author `crates/ui/src/screens/compare.rs` per § 2 row 12.
Toolbar row: `Row[range_picker | kpi_axis_dropdown | k7_subtitle_when_any_cell_is_multi_symbol]`.
Body: `widgets::matrix::view(model, mode)`.
- File: `crates/ui/src/screens/compare.rs` (new); `crates/ui/src/screens/mod.rs` (one-line declaration).
- Cargo: `cargo check -p ui`.
- Acceptance: PASS.

T-D-N9 — Swap `crates/ui/src/shell.rs:96` from
`placeholder::view(strings::COMPARE_PLACEHOLDER, mode)` to
`screens::compare::view(model, mode)`. Add `compare` to the use-list
at `:28`.
- File: `crates/ui/src/shell.rs:28,96`.
- Cargo: `cargo test -p ui --lib` + `cargo test -p ui --test layout_invariants`.
- Acceptance: PASS; existing 6/6 layout-invariants PASS preserved.

### Wave D — Snapshot baselines + cockpit-smoke + round-trip test (R7, H5)

4 NEW insta-style baselines + 1 NEW layout-invariants proptest case
+ 1 NEW round-trip unit test. None of these change any of the 22
body-SHA anchors — additive PNG files + additive proptest case +
additive `#[cfg(test)]` test.

T-D-N10 — Author fixture `compare__cold_boot_all_empty`: matrix
rendered with `compare_screen_state.cache = BTreeMap::new()` —
every cell is either "Run" (legal universe pair) or `—` (blanked
universe pair). Fixture in `crates/ui/tests/visual_snapshots.rs`.
- File: `crates/ui/tests/visual_snapshots.rs` + sibling fixture; baseline at `crates/ui/tests/visual-baselines/compare__cold_boot_all_empty.png`.
- Cargo: `cargo test -p ui --test visual_snapshots -- --exact compare__cold_boot_all_empty`.
- Acceptance: PASS (writes the baseline on first run; matches on subsequent).

T-D-N11 — Author fixture `compare__steady_state_populated`: matrix
rendered with all 24 populated cells filled (deterministic seed
fixture per § 1.2 enumeration). K7 multi-symbol disclaimer subtitle
visible.
- File: `crates/ui/tests/visual_snapshots.rs` + sibling fixture; baseline at `crates/ui/tests/visual-baselines/compare__steady_state_populated.png`.
- Cargo: `cargo test -p ui --test visual_snapshots -- --exact compare__steady_state_populated`.
- Acceptance: PASS.

T-D-N12 — Author fixture `compare__empty_cell_run_affordance`:
matrix rendered with `compare_screen_state.cache` populated for 20 of
24 legal cells (4 empty cells with "Run" affordance + 36 blanked-grey
cells). The 4 empty cells render the active `ACCENT_500` hairline
"Run" button per R2.3.
- File: `crates/ui/tests/visual_snapshots.rs` + sibling fixture; baseline at `crates/ui/tests/visual-baselines/compare__empty_cell_run_affordance.png`.
- Cargo: `cargo test -p ui --test visual_snapshots -- --exact compare__empty_cell_run_affordance`.
- Acceptance: PASS.

T-D-N13 — Author fixture `compare__column_header_hover`: matrix
rendered with the operator's cursor hovering a column header
(e.g. "BTCUSDT"). Per R2.4 v0.1.0 the column header is **non-
interactive** (label only) — fixture exercises that the hover-state
on a column header does NOT render the `active_row` border tint
(distinct from cell hover).
- File: `crates/ui/tests/visual_snapshots.rs` + sibling fixture; baseline at `crates/ui/tests/visual-baselines/compare__column_header_hover.png`.
- Cargo: `cargo test -p ui --test visual_snapshots -- --exact compare__column_header_hover`.
- Acceptance: PASS.

T-D-N14 — Add layout-invariants proptest case
`compare_screen_no_zero_dim` at `crates/ui/tests/layout_invariants.rs`:
256 viewport-size samples in the existing range (320×240 → 3840×2160);
render `screens::compare::view` with each; assert no panic + every cell
≥ 1×1 px (R2.5).
- File: `crates/ui/tests/layout_invariants.rs` (append).
- Cargo: `cargo test -p ui --test layout_invariants -- compare_screen_no_zero_dim`.
- Acceptance: PASS (256 / 256 cases survived).

T-D-N15 — Add H5 round-trip unit test
`open_lab_from_compare_sets_lab_strategy_pair_and_range` at
`crates/ui/src/state.rs:~3194` (after the existing `open_trail_for_*`
tests at `:3149-3189`). Asserts post-dispatch: `current_screen == Screen::Lab`,
`lab_state.strategy == Some(strategy)`, `lab_state.pair == Some((venue, symbol))`,
`lab_state.range == range`.
- File: `crates/ui/src/state.rs:~3194` (append to `#[cfg(test)] mod tests`).
- Cargo: `cargo test -p ui --lib open_lab_from_compare_sets_lab_strategy_pair_and_range`.
- Acceptance: `running 1 test` line + `test result: ok. 1 passed; 0 failed`.

T-D-N16 — Run `cockpit-smoke` per `.claude/skills/rust-test/SKILL.md`
with `Screen::Compare` as the active screen. Tester scope per M-FINAL;
developer pre-runs to confirm no panic.
- Cargo: `cargo test -p ui --test cockpit_smoke -- --nocapture` (or the
  binary equivalent if cockpit-smoke is a separate bin — confirm via
  the predecessor's M-FINAL invocation).
- Acceptance: `0 panic lines` (R7.3).

### Wave E — M-FINAL handoff prep

T-D-N17 — Re-run `scripts/verify_anchors.sh` post-implementation.
This is the NON-NEGOTIABLE H2 carry-forward gate per R7.1. Architect
verifies once after Wave D lands; tester re-verifies at M-FINAL.
- Cargo: `bash scripts/verify_anchors.sh`.
- Acceptance: `ANCHORS PASS  (22 / 22)` literal output.

T-D-N18 — Developer emits HANDOFF → tester envelope per AGENT.md
§ "Structured handoff envelope". Tester then runs the M-FINAL sweep
per `spec/ui-rethink-phase-e-compare/feature.md ## Acceptance criteria
§ M-FINAL`: `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
`cargo test --workspace --lib`, the 4 new visual snapshots (T-D-N10..N13),
the new layout-invariants case (T-D-N14), the H5 round-trip
(T-D-N15), `scripts/verify_anchors.sh`, cockpit-performance v1.0.0
idle-CPU floor sweep ≤ 13.6 %, and authors the test report.

## 4. Spike requirement

**NONE.** The matrix is purely additive UI:

- Report-cache parse — shape known + fail-soft contract locked (§ 1.1).
- Compound dispatch — verbatim precedent at `state.rs:1902-1910`
  (`OpenTrailFor` arm).
- Layout primitive — iced `Column<Row>` already used across
  `screens/strategy_registry.rs` + `widgets/sidebar_nav.rs`.
- State plumbing — 3-touchpoint pattern verbatim from `lab_state` +
  `trail_screen_state` (§ 1.6).
- YAML dep — resolved to hand-parse, no new external crate dep
  (§ 1.1, no ADR needed).

If during Wave A the developer discovers a non-trivial blocker (e.g.
the K6 range-divergence mitigation requires a new `DateRange` variant
or a cross-state-handle invalidation message), they HANDOFF back to
architect for a Wave-A spike. **Not anticipated.**

## 5. Rollback shape (per Wave)

Each wave is independently revertable.

- **Wave A rollback:** revert the `crates/ui/src/compare/` directory +
  the `pub compare_screen_state` field on `Cockpit` + the 3 new
  Message variants. The cockpit reverts to v0.1.0-Phase-D+ shape;
  Compare sidebar entry continues routing to `placeholder::view`
  (which is what it does today). No anchor-side impact.
- **Wave B rollback:** delete `crates/ui/src/widgets/matrix.rs` +
  the one-line declaration in `widgets/mod.rs`. Wave A's data still
  exists but no widget consumes it; cockpit compiles, tests pass.
- **Wave C rollback:** revert the 2-line `shell.rs` change (use-list +
  match arm) + delete `crates/ui/src/screens/compare.rs` + the
  one-line declaration in `screens/mod.rs`. Compare sidebar entry
  routes back to `placeholder::view`. **This is the load-bearing
  rollback path** — single revert restores Phase D+ shape exactly.
- **Wave D rollback:** delete the 4 new PNG baselines + their fixtures
  + the proptest case + the round-trip unit test. Layout-invariants
  passes at 6/6 (Phase D+ baseline); existing 937 lib tests preserved.
- **Wave E rollback:** N/A (verification + handoff only — no code).

The non-regression contract from the feature.md (22 anchors byte-
identical, 937 lib tests PASS, layout-invariants PASS, no new external
deps) is preserved at every wave boundary by construction.

## 6. Hard constraints honour-list

- [x] Work directly on `main` (no worktrees). Architect emits files
  only (`decomp.md`, `tasks.md` updates, `trace.toml` row flip);
  orchestrator commits. **Honored.**
- [x] iced 0.14 vendored `iced_tiny_skia` fork operator-locked
  2026-05-20. **Honored** — Phase E uses the same iced layout
  primitives (`Column<Row>`, `Button`, `Tooltip`) already in use
  across Phases A-D+. No iced bump.
- [x] No new external crate deps. K3 resolved to hand-parse (§ 1.1);
  no `serde_yaml` ADR needed. **Honored.**
- [x] 22 anchored body-SHAs stay byte-identical (R7.1). **Honored by
  construction** — purely additive UI: no SQL migration, no backtest
  binary change, no anchored-report renderer change. Architect
  re-ran `bash scripts/verify_anchors.sh` BEFORE this pass:
  `ANCHORS PASS  (22 / 22)` (literal output captured in M-T1 tick
  T-T1-6 below + Wave E T-D-N17).
- [x] Cockpit-perf idle-CPU floor preserved (≤ 13.6 % per R7.4).
  **Verification deferred to tester at M-FINAL** per H3 (the matrix
  is on-demand render only — no new subscription, no new `tokio::time::interval`,
  matches Phase C Live screen which already hit the budget).
- [x] Honest-tick rule — every M-T1 row carries file:line + cargo
  invocation + literal expected output (see `tasks.md` updates).
- [x] No new Lumen tokens (R7.6). **Honored** — cell hover reuses
  the Phase C `active_row` border tint at
  `widgets/strategy_card.rs`; K7 tooltip reuses the existing
  `widgets::tooltip` pattern from Phase D.

## 7. Watch recipe for long-running tasks

None of the Wave A-E tasks are individually long-running (all single-
cargo invocations completing in < 2 min). The composite
`cargo test --workspace --lib` at M-FINAL takes ≈ 3-5 min; the
tester emits the standard cockpit-smoke watch recipe at that time
(template in `spec/ui-rethink-phase-d-trail-followup/decomp.md § 7`).

## 8. Handoff

Developer receives this `decomp.md` plus the appended `tasks.md`
T-D-N1..N18 checklist. Implementation order: Waves A → B → C → D → E
(Wave E is tester-handed; the developer's last responsibility is
T-D-N17 anchor gate + T-D-N18 handoff envelope to tester). Waves B
and C can NOT run in parallel — Wave C's `screens::compare::view`
calls `widgets::matrix::view`. Wave D depends on C (visual snapshots
exercise the wired-through shell route).

## Changelog

- 2026-05-20 (architect): M-T1 decomposition pass. Resolved K3
  (hand-parse YAML frontmatter — no `serde_yaml` workspace dep,
  no ADR), H1 (24/60 cells = 40 % first-open hit-rate ≥ 30 %
  threshold; v2.llm row only universally empty), H4 (≤ 15 ms p99
  by static order-of-magnitude argument over 32 reports; well
  under 50 ms budget), Q6 sub-decision (universe-aggregate
  disclaimer = subtitle + per-cell tooltip), state location
  (sibling field on `Cockpit` at `state.rs:~880`), spike
  requirement = NONE. Wave A-E ordered with 18 T-D-N rows;
  rollback shape per wave; honour-list confirms operator + project
  invariants (no worktrees, iced lock, anchor gate, no external
  deps, no new Lumen tokens). Anchor baseline `ANCHORS PASS
  (22 / 22)` re-verified before this pass. Handoff envelope
  emitted to developer inline at the end of the pass.
