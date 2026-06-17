---
slug: lumen-phase-4-backtest-panel
status: shipped
owner: analyst
updated: 2026-05-06
version: 2.3.0
---

# Lumen design adoption — Phase 4: Backtest panel (`viewer` bin)

> **Phase 4 of 6** in the
> [`lumen-design-adoption`](../feature.md) initiative. Master
> roadmap is the orientation; this brief is the **shippable feature**.
> Operator-locked constraints (no brand, no voice rewrite, sequential
> phases, Phase 6 reserved) are documented in the master file and apply
> here without re-litigation.
>
> **Status: active.** Was originally Phase 2 in the pre-2026-05-04
> roadmap; renumbered to Phase 4 at the 2026-05-04 master-roadmap
> revision. The 2026-05-04 stub (109 lines, queued status, scope
> outline only) is **superseded by this expansion**. The Why section
> is preserved verbatim and extended; high-level scope is replaced by
> R-cluster-pointing summary; open questions are replaced by the
> architect Q-items below.

## Why

The `viewer` binary today renders a markdown report from
`spec/*/reports/backtest-*.md`. The Lumen
[`Backtest.jsx`](../../archive/design-prototypes-2026-Q2.tar.gz)
pattern (purpose-built for this project at the design conversation —
see master roadmap "The Lumen bundle is purpose-built for this
project") is materially richer: **KPI strip + equity curve +
drawdown band**, with a "Deploy live" CTA explicitly excluded
(paper-only product non-goal). Phase 2's canvas chart primitives
([`crates/ui/src/widgets/chart.rs`](../../../crates/ui/src/widgets/chart.rs))
and Phase 3's read-only data discipline make the visual upgrade
affordable.

### Two surfaces, one primitive

Phase 4 closes **two** related surfaces in one ship:

1. **The viewer's Backtest panel** — the surface the master roadmap
   names (offline, reads from `spec/*/reports/backtest-*.md`).
2. **The cockpit Strategies-detail equity-since-deploy sparkline** —
   the surface
   [Phase 3 deferred at Q6](../phase-3-detail-screens/feature.md) because
   the cheap path didn't exist on the current state shape (Phase 3
   architect: "`Cockpit::pnl: PanelState<PnlSnapshot>` is a single
   snapshot, not a historical buffer"). Phase 3 shipped a
   `frame::muted_body(strings::STRATEGIES_SPARKLINE_DEFERRED)`
   placeholder reading **"Equity sparkline lands with Phase 4"** at
   [`crates/ui/src/strings.rs:261`](../../../crates/ui/src/strings.rs).
   Phase 4 honours that placeholder.

Both surfaces consume the **same shape of data** (a sequence of
`(Timestamp, Money<Usdt>)` points + drawdown vector + peak/trough
metadata) from **different sources** (offline report metadata vs
online audit ledger). Phase 4's analyst pass scopes a shared
`core::EquitySeries` primitive so the two consumers don't fork the
shape — and so Phase 5 (HumanControl) and Phase 6 (Assistant) can
reuse it without re-litigation.

### What's missing today, concretely

1. **The viewer binary doesn't exist on disk yet.** The architecture
   document reserves it (`viewer` row in the App-layout table at
   architecture.md:2947–2951; `spec/reports/` read-path contract at
   architecture.md:3089–3118), but `crates/ui/src/bin/` ships only
   `cockpit.rs` and `cockpit_live.rs`. Phase 4 *creates*
   `crates/ui/src/bin/viewer.rs` from scratch — no in-place refactor.
2. **No KPI-strip primitive.** The 11 anchored reports under
   `spec/reports/` carry the six metrics (Total
   return, CAGR, Sharpe, Max DD, Win rate, Trades) in a 2-column
   markdown table inside the body — anchored, must not be altered.
   The KPI strip reads from a **side-channel structured contract**:
   parsed from the existing summary table (Q3a) or from a parallel
   `report.json` sidecar (Q3b). Architect ratifies at Q3.
3. **No equity-history state in the cockpit.** Phase 2's
   `ChartBuffer` is keyed on `(Venue, Symbol)` and holds price
   ticks, not equity. Strategies-detail needs equity history keyed
   on `StrategyId` over a different time grid — sibling buffer.
4. **No equity-history audit query.** Phase 3's
   [`pnl_by_strategy`](../../../crates/audit/src/query.rs) returns a
   single aggregate; the sparkline needs a **vector** over the
   deploy-to-now window. Phase 4 adds an additive
   `audit::query::equity_curve_for_strategy` method (operator-
   ratified at master Q13 — extend `audit::query` for read
   additions).

### What Phase 4 ships

- **A new `viewer` binary** at `crates/ui/src/bin/viewer.rs`,
  CLI-arg-driven. Inherits Phase 1's `theme::*` tokens, dark
  default cold-start.
- **A KPI strip widget** — six metric cards (Total return / CAGR /
  Sharpe / Max DD / Win rate / Trades), `widgets::num` for
  formatting; up/down sentiment via `UP_500` / `DOWN_500`.
- **An equity-curve widget** — polyline in `ACCENT` + filled area
  in `UP_500` at low alpha (Lumen `Backtest.jsx:93–99`); five
  horizontal `BORDER_1` gridlines.
- **A drawdown band widget** — second canvas beneath, line +
  filled area in `DOWN_500` at low alpha (Lumen
  `Backtest.jsx:101–105`).
- **The existing markdown body preserved** verbatim below the
  structured strip; locked anchored body untouched.
- **A `core::EquitySeries` primitive** — shared shape;
  `(Timestamp, Money<Usdt>)` points + peak / trough / drawdown
  metadata.
- **Two consumers** — viewer's equity curve (offline source:
  report side-channel) + cockpit Strategies-detail sparkline
  (online source: new audit query).
- **An additive `audit::query::equity_curve_for_strategy`** —
  read-only sibling of Phase 2 / Phase 3 filtered-query
  additions.
- **Cockpit Strategies-detail sparkline ON** — replaces the
  Phase-3-deferred placeholder.

### What Phase 4 does NOT ship — load-bearing

- **No "Deploy live" CTA.** Lumen `Backtest.jsx:76` ships one;
  **out of scope** — paper-only product, deployment is
  config-driven.
- **No re-anchor.** 11 backtest body-SHA-256 anchors in
  [`spec/anchors.toml`](../../anchors.toml) stay byte-identical;
  viewer reads existing reports, cockpit-side consumer reads
  the audit ledger; no committed body re-rendered.
- **No new backtest scenarios.** Phase 4 is UI + additive read
  query; not a strategy / risk / exec change.
- **No cockpit chrome change.** Cockpit changes limited to: (a)
  Strategies-detail swaps placeholder for real sparkline; (b)
  `Cockpit` gains `pub strategy_equity: HashMap<StrategyId,
  PanelState<EquitySeries>>`. No sidebar / screen / shell
  change.
- **No file-picker UI on the viewer.** CLI-arg only (Q4 —
  matches "single operator, config-driven" non-goal).
- **No 1-min equity-curve render budget.** 90-day 1-min ~129 600
  points is impractical; Phase 4 assumes report-side
  downsampling (the existing
  [`render/equity_curve.rs`](../../../crates/reports/src/render/equity_curve.rs)
  ships 60-cell sparklines / 1m / 5m cadence). Expected
  N ∈ [60, 2000]. Q5 ratifies the cap.
- **No re-litigation of operator-locked constraints.**
  Master Q11–Q14 inherited (Q11 sidebar — irrelevant, viewer
  has no sidebar; Q12 both modes — irrelevant, viewer is
  offline-only; Q13 extend `audit::query` — applies to R12;
  Q14 split — past gate).

### Why now

Phase 3 shipped 2026-05-05 / approved 2026-05-06. The Phase 3
deferral note ratified Phase 4 as the owner of the equity-history
primitive ("Phase 4 owns the wiring (the Backtest panel already
needs the same equity-history primitive)"). Two consumers needing
the same shape at the same time is the cheapest moment to land the
shared type. Postponing the cockpit consumer to Phase 5 would force
the primitive's design to be re-litigated when HumanControl arrives.

## Scope (high-level)

The full R-item list is below. High-level grouping:

- **R1–R3 KPI strip** — the 6-metric band over the existing
  markdown body; data source from the report's structured
  side-channel.
- **R4–R6 Equity-curve widget** — line-series with filled area,
  reusing Phase 2's canvas primitives; horizontal gridlines.
- **R7–R8 Drawdown band** — beneath the equity curve;
  `DOWN_500` filled area at low alpha.
- **R9 Markdown body preservation** — locked-body content stays
  below the structured strip; no operator regression.
- **R10–R12 Equity-history primitive** — `core::EquitySeries`
  shape + offline source (report side-channel) + online source
  (additive `audit::query` method).
- **R13 Cockpit Strategies-detail sparkline consumer** — the
  Phase-3-deferred surface; replaces the placeholder.
- **R14 No-CTA exclusion** — explicit out-of-scope guardrail.
- **R15 Single-binary scope** — viewer-only changes + the
  additive cockpit sparkline consumer; no `cockpit_live` chrome
  change.
- **R16 Cross-feature invariants preservation** — every prior
  shipped feature still passes.
- **R17 Anchor regression** — 11/11 byte-identical; viewer
  doesn't write reports.

## Anchor risk

**Zero, by construction.** Phase 4 reads existing committed
reports + the audit ledger; both are read-only paths. No new
backtest scenarios run; no committed report body is re-rendered.
The 11/11 backtest body-SHA-256 anchor regression goal is
preserved by construction. Re-stated loudly: **anchor risk is
zero**.

## Snapshot ripple

The viewer binary has no shipped snapshot baselines today (no
`viewer__*.snap` files exist under `crates/ui/tests/snapshots/`).
Phase 4 introduces ~5 net-new baselines:

1. `viewer__kpi_strip__sample_report.snap` — the 6-metric strip
   over a deterministic sample report.
2. `viewer__equity_curve__sample_report.snap` — the equity-curve
   canvas with a fixed point set.
3. `viewer__drawdown_band__sample_report.snap` — the drawdown
   band canvas.
4. `viewer__full_view__sample_report.snap` — the assembled
   viewer surface (KPI strip + equity curve + drawdown band +
   markdown body header).
5. `strategies_screen__sparkline_present.snap` — the cockpit
   Strategies-detail surface with the real sparkline replacing
   the Phase 3 deferred placeholder. **The
   `strategies_screen__sparkline_deferred.snap` baseline retires
   in this phase** — the placeholder copy is no longer rendered;
   the deferred-snap file is deleted as part of T1xxx in the
   developer pass.

Phase 1 / Phase 2 / Phase 3 baselines stay byte-identical (the
viewer is a separate bin; the cockpit Strategies-detail diff is
local to the sparkline placement). Single `cargo insta accept`
pass at end of phase per Phase 1 Q2 / Phase 2 V11 / Phase 3
precedent.

## Requirements

Numbered, testable, derived from
[`spec/design/project/ui_kits/desktop/Backtest.jsx`](../../archive/design-prototypes-2026-Q2.tar.gz),
the
[Phase 2 chart-widget contract](../phase-2-shell-ia-charts/feature.md),
the
[Phase 3 sparkline deferral note](../phase-3-detail-screens/feature.md),
the existing
[`crates/ui/src/widgets/chart.rs`](../../../crates/ui/src/widgets/chart.rs),
[`crates/audit/src/query.rs`](../../../crates/audit/src/query.rs), and
[`spec/architecture.md` § Frontend](../../architecture.md). Each ends
with a one-line **acceptance** the tester can verify. Operator-
locked constraints inherited from the
[master roadmap](../feature.md) (no brand, no voice
rewrite, sequential phases, Q11–Q14) apply throughout.

### R1 — Viewer binary scaffold

- **R1.1** New file `crates/ui/src/bin/viewer.rs`.
  `iced::application` pattern (cockpit precedent). `Model`
  carries the loaded report state; `Message` carries
  `ReportLoaded(Result<…>)` + `ToggleTheme` (cold-start dark
  parity per master Q10).
- **R1.2** **CLI arg** — `clap`-parsed positional
  `<report-path>` (Q4 — CLI-only). Missing arg → exit code 2;
  non-existent path → exit code 3.
- **R1.3** Window title `"Backtest report — <scenario>"` —
  `<scenario>` from the report's front-matter at
  [sample report line 2](../../v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md).
  Master Constraint 1 — **no `"Lumen"`** in the title.
- **R1.4** Window 1200 × 800 initial; resizable. Tier 0
  `CANVAS` outside panel; Tier 1 `PANEL` inside report shell.
- **R1.5** No status bar — viewer is offline / single-shot,
  no connection / latency signals to anchor.
- **Acceptance:** `cargo run -p ui --bin viewer --
  spec/v05-composed-strategies/reports/backtest-…-rsi-reversion.md` launches with
  the expected title; missing-arg / missing-file exit codes
  match.

### R2 — KPI strip layout

- **R2.1** New widget `crates/ui/src/widgets/kpi_strip.rs`:
  ```rust
  pub fn view<'a>(metrics: &'a BacktestMetrics, mode: ThemeMode)
      -> Element<'a, Message>;
  ```
- **R2.2** Six metric cards in one row, equal width, gap
  `space::M` (Lumen `Backtest.jsx:80–87`): Total return, CAGR,
  Sharpe, Max DD, Win rate, Trades.
- **R2.3** Each card = label (`text::SMALL` `FG_3` muted) above
  value (`text::H1` 24 px `FG_1`). No card border; gap-only
  separation.
- **R2.4** Sentiment colouring (Lumen `Backtest.jsx:81/84`):
  Total return `UP_500` / `DOWN_500` / `FG_1` by sign; Max DD
  always `DOWN_500` (prefix with `theme::num::MINUS_SIGN_LITERAL`
  per design-principles ASCII-minus rule); CAGR / Sharpe / Win
  rate / Trades neutral `FG_1`.
- **R2.5** `BacktestMetrics` lives in `core`. Fields:
  `total_return_pct`, `cagr_pct`, `sharpe`,
  `max_drawdown_pct`, `win_rate_pct: Decimal`; `trades: u64`.
  **No `f64`**.
- **R2.6** Empty / error state — when parse fails, six muted
  `—` dashes + single-line
  `frame::muted_body(strings::VIEWER_METRICS_UNAVAILABLE)`.
- **Acceptance:** `viewer__kpi_strip__sample_report.snap`
  PASSES; sentiment colours match R2.4 for the negative-return
  RSI sample.

### R3 — KPI strip data source

- **R3.1** **Source = report side-channel** (Q3 ratification —
  architect picks markdown-parse Q3a vs `report.json` sidecar
  Q3b). Both options require **zero change to committed bodies**.
- **R3.2** `core::BacktestMetrics::parse_from_report(path)` —
  public entry. Implementation in `crates/reports/src/parse.rs`
  (new module).
- **R3.3** Determinism — same bytes → same metrics. No clock,
  no `f64`. `Decimal::from_str` only.
- **R3.4** Existing-report compatibility — all 11 anchored
  reports parse without modification (sample summary table at
  [RSI report lines 22–41](../../v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md)).
- **R3.5** Missing-field tolerance — older reports without
  CAGR → `Ok(BacktestMetrics { cagr_pct: Decimal::ZERO,
  cagr_present: false, … })` so the strip renders `—` per
  R2.6.
- **Acceptance:** unit tests
  `parses_rsi_reversion_sample_report`,
  `parses_negative_return_sample_correctly`,
  `parses_zero_trades_sample_returns_ok`,
  `missing_field_returns_marked_absent` PASS. **No anchor body
  diff** — `verify-anchors` 11/11 PASS.

### R4 — Equity-curve widget

- **R4.1** New widget at `crates/ui/src/widgets/equity_curve.rs`
  (or shared via Q2 — see R5):
  ```rust
  pub fn view<'a>(series: &'a EquitySeries, mode: ThemeMode)
      -> Element<'a, Message>;
  ```
- **R4.2** Canvas-based; X = time oldest-left → newest-right, Y
  = equity USDT low-bottom → high-top; Y range
  `(min_equity, max_equity)` + 5 % padding (mirrors
  [`chart.rs:45`](../../../crates/ui/src/widgets/chart.rs)).
- **R4.3** **Polyline in `ACCENT`** (1.5 px). Aligns with the
  design-principles' default line-series colour
  (ui-design-principles.md:419) and Phase 2 precedent; leaves
  `UP_500` for the fill below.
- **R4.4** **Filled area** beneath in **solid `UP_500` @ 0.18**
  (Q6 — Lumen `Backtest.jsx:94` uses gradient; Phase 4 goes
  solid for consistency with Lumen's drawdown band at line 103
  + Phase 2's line-fill style).
- **R4.5** **Five horizontal gridlines** `BORDER_1 @ 0.4`
  (mirror Phase 2
  [`draw_gridlines`](../../../crates/ui/src/widgets/chart.rs)
  at line 187). No vertical grid.
- **R4.6** Read-only — no hover / click / pan / zoom. Future
  phase may add hover.
- **R4.7** Empty state — `points.is_empty()` → gridlines +
  centred `frame::muted_body(strings::VIEWER_NO_EQUITY_DATA)`.
- **Acceptance:** `viewer__equity_curve__sample_report.snap`
  PASS; polyline = `ACCENT`, fill = `UP_500 @ 0.18`,
  gridlines = 5.

### R5 — Equity-curve canvas reuse

- **R5.1** Q2 ratification — architect picks **share
  `widgets::chart`** (recommended) vs copy. Phase 2 widget
  already has internal helpers (`draw_gridlines`, `inner_rect`,
  `with_alpha`).
- **R5.2** If shared (recommended): factor into a
  `widgets::canvas_chart` core + two thin wrappers
  (`widgets::price_chart` for Phase 2's marker-overlay surface;
  `widgets::equity_curve` for Phase 4's fill-beneath-line
  surface). Phase 2's public `view` signature stays byte-stable.
- **R5.3** If copied: ~50 LOC viewer-local module duplicates
  `draw_gridlines` + polyline path with area-fill diff.
  Two-source-of-truth risk; flagged in Q2 alternatives.
- **R5.4** Cockpit Strategies-detail sparkline (R13) reuses the
  same primitives either way.
- **Acceptance:** architect picks the path; V2 / V4 snapshots +
  Phase 2 chart-widget baseline byte-identical PASS.

### R6 — Equity-curve scaling

- **R6.1** Linear Y-scale (matches Phase 2 + Lumen
  `Backtest.jsx:98`); log-scale out of scope.
- **R6.2** X-axis is index-based (point[0] left, point[N-1]
  right). Reports already sample at uniform cadence; index-based
  and time-proportional render identically — index is cheaper.
- **R6.3** Point count cap — typically 60 ≤ N ≤ ~2000 (Q5).
  `EquitySeries` constructor downsamples at build time when N >
  2000; canvas always sees ≤ 2000 points.
- **Acceptance:** sample-report equity-curve render matches V4
  baseline; oversized vector (3000 points) downsamples to 2000;
  no render-path stutter.

### R7 — Drawdown band widget

- **R7.1** Second canvas beneath the equity curve at
  `crates/ui/src/widgets/drawdown_band.rs` (or sibling inside
  equity-curve module per Q2):
  ```rust
  pub fn view<'a>(series: &'a EquitySeries, mode: ThemeMode)
      -> Element<'a, Message>;
  ```
- **R7.2** Y-axis `(0, max_drawdown_pct)` inverted (0 top,
  max-DD bottom — drawdown grows downward; Lumen
  `Backtest.jsx:67/103`). Polyline + filled area in
  **solid `DOWN_500` @ 0.18** (Q6).
- **R7.3** Height ~100 px (vs equity curve's ~240 px;
  Lumen `Backtest.jsx:91/102` ratio).
- **R7.4** Five horizontal gridlines (same density for visual
  rhythm).
- **R7.5** Empty state — same as R4.7.
- **Acceptance:** `viewer__drawdown_band__sample_report.snap`
  PASS; polyline = `DOWN_500`, fill = `DOWN_500 @ 0.18`,
  gridlines = 5.

### R8 — Drawdown vector source

- **R8.1** `EquitySeries` carries precomputed
  `drawdown_pct: Vec<Decimal>`, same length as `points`
  (R10.2). Drawdown at `i` is `(running_peak[i] -
  equity[i]) / running_peak[i]`. Consumers never recompute.
- **R8.2** Single `O(N)` left-to-right pass; `running_peak =
  max(running_peak, equity[i])`. `Decimal` arithmetic only,
  no `f64`.
- **R8.3** Edge cases — empty input → empty vector;
  monotone-flat or monotone-up → all-zero vector.
- **Acceptance:** unit tests
  `equity_series_drawdown_monotone_zero`,
  `equity_series_drawdown_50_percent_then_recovery`,
  `equity_series_drawdown_empty_returns_empty` PASS.

### R9 — Markdown body preservation

- **R9.1** Viewer renders report body **below** the structured
  strip + curves; body bytes are the locked anchored body.
- **R9.2** Render path — minimal subset (h1–h3, paragraphs,
  tables, code spans). Architect picks at design: add
  `pulldown-cmark` (not yet a workspace dep) vs render as
  monospace `text::BODY` in a scrollable container. KPI strip
  + curves carry the visual weight.
- **R9.3** Anchor-preservation — bytes read by viewer = bytes
  hashed by `scripts/hash_report.py`. Viewer never **writes**
  to the spec tree (architecture.md:3116).
- **R9.4** Layout — KPI strip ~80 px + equity ~240 px +
  drawdown ~100 px + body `Length::Fill`-scroll; total
  above-the-fold ~420 px on the 800 px window.
- **Acceptance:** any of the 11 anchored reports renders body
  verbatim below structured strip; `verify-anchors` 11/11
  PASS.

### R10 — `EquitySeries` primitive (cross-phase contract)

- **R10.1** New type in `core` — module placement at Q12
  (recommended `crates/core/src/equity_series.rs`).
- **R10.2** Field set (Q1 — **richer**, consumers don't
  recompute):
  ```rust
  pub struct EquitySeries {
      pub points: Vec<(Timestamp, Money<Usdt>)>, // oldest-first
      pub drawdown_pct: Vec<Decimal>,            // same length
      pub start: Timestamp,
      pub end: Timestamp,
      pub peak: Money<Usdt>,
      pub trough: Money<Usdt>,
      pub max_drawdown_pct: Decimal,
  }
  ```
- **R10.3** `from_points(points)` walks once, computes
  drawdown / peak / trough / max-DD in one `O(N)` `Decimal`
  pass. Returns `Result<Self, EquitySeriesError>`; errors on
  empty input + non-monotone timestamps.
- **R10.4** `downsample(self, max_points)` — equal-stride
  bucketing, last-value-wins per bucket; `Decimal` arithmetic.
- **R10.5** No `f64`; no clock reads; pure-data,
  serde-friendly.
- **R10.6** Zero crate-graph back-edges — `core` does not
  import from `audit` / `reports` / `ui`. Each consumer
  brings its own constructor.
- **Acceptance:** `cargo test -p trading-core
  equity_series::tests` PASSES at minimum
  `from_points_computes_drawdown_correctly`,
  `downsample_to_2000_preserves_peak_and_trough`,
  `non_monotone_timestamps_returns_err`,
  `empty_input_returns_err`.

### R11 — Offline source — viewer's `EquitySeries`

- **R11.1** New helper in `crates/reports/src/parse.rs` —
  `pub fn equity_series_from_report(path: &Path) ->
  Result<EquitySeries, ParseError>`. Reads the report's
  side-channel data.
- **R11.2** **Companion CSV reuse** — equity-points source is
  the existing companion CSV at `<report-stem>__equity.csv`
  (written by
  [`csv_artifacts::write_equity_csv`](../../../crates/reports/src/csv_artifacts.rs)).
  `EquitySample` rows carry `ts + equity_total`; Phase 4 maps
  each row to `(ts, Money::<Usdt>::from(decimal))` into
  `EquitySeries::points`. **Zero schema change** — read-only
  consumer. The body's "Equity curve" sparkline section
  ([`render/equity_curve.rs`](../../../crates/reports/src/render/equity_curve.rs))
  is too coarse (60 chars) for Phase 4's curve regardless of
  Q3 ratification, so the CSV is the load-bearing source.
- **R11.3** Reports without a companion CSV (older, pre-
  `operator-success-reports`) — viewer renders the R4.7 empty
  state + the markdown body. KPI strip stays independent
  (R3 path).
- **Acceptance:** `equity_series_from_report` returns
  populated series for the RSI sample's companion CSV; empty
  for fixture without CSV; **no anchor body diff** —
  `verify-anchors` 11/11 PASS.

### R12 — Online source — `audit::query::equity_curve_for_strategy`

- **R12.1** New `pub async fn` in
  [`crates/audit/src/query.rs`](../../../crates/audit/src/query.rs).
  Sibling of `recent_fills_filtered` (line 180) and
  `recent_journal_filtered` (line 313). Master Q13 — extend
  `audit::query` for read additions.
- **R12.2** **Signature** (Q7):
  ```rust
  pub async fn equity_curve_for_strategy(
      ledger: &Ledger,
      strategy_id: StrategyId,
      since: Timestamp,
      until: Option<Timestamp>,
  ) -> Result<EquitySeries, LedgerError>;
  ```
  `until = None` means "to now". Returns `EquitySeries`
  directly (Q1 — query owns the drawdown computation).
- **R12.3** Reuses
  [`pnl_by_strategy`](../../../crates/audit/src/query.rs) (line
  933) algorithm but emits a vector of bar-close samples
  rather than a single aggregate. Typical 1-day at 1m =
  1440 samples — below the R6.3 cap.
- **R12.4** Read-only over committed audit data; additive —
  `pnl_by_strategy` unchanged.
- **R12.5** Determinism — `ORDER BY ts ASC, rowid ASC`
  (oldest-first per R10.3). `Decimal` only.
- **R12.6** Three unit tests:
  `equity_curve_for_strategy_returns_window_samples`,
  `…_empty_window_returns_empty`,
  `…_until_none_includes_to_now`.
- **Acceptance:** three unit tests PASS; cockpit consumer
  (R13) round-trips a non-empty series for the fixtures
  `sma_crossover` strategy.

### R13 — Cockpit Strategies-detail sparkline consumer

- **R13.1** Replace the Phase 3 placeholder
  (`frame::muted_body(strings::STRATEGIES_SPARKLINE_DEFERRED)`
  at top-right of the Strategies screen) with a real sparkline.
- **R13.2** Dimensions — ~120 × ~36 px per Phase 3 R6.1.
  `ACCENT` polyline, no axes / tooltip / fill (sparkline at
  this size reads cleanest line-only; viewer's full-size curve
  carries fill per R4.4).
- **R13.3** New `Cockpit` field —
  `pub strategy_equity: HashMap<StrategyId,
  PanelState<EquitySeries>>`. `Loading` during fetch;
  `Ready(series)` after; `Error(msg)` on `LedgerError`.
- **R13.4** **Wire path** — `Message::SelectStrategy` arm from
  Phase 3 Q11b triggers a `Task::perform` one-shot
  `audit::query::equity_curve_for_strategy(strategy_id,
  since: strategy_load_ts, until: None)`. Result lands as
  `Message::StrategyEquityRefreshed(StrategyId,
  Result<EquitySeries, _>)`. Pure assignment in `update`.
- **R13.5** Render budget (Q9 — cap + downsample at fetch).
  Typical 1-day audit result ~1440 samples; sparkline budget
  ~120 px wide → cap 120 points via
  `EquitySeries::downsample(120)`.
- **R13.6** **No live update.** One-shot on strategy-select;
  future phase may add a `Subscription` recipe.
- **R13.7** Loading state —
  `frame::muted_body(strings::STRATEGIES_SPARKLINE_LOADING)`
  ("Loading equity history…"); net-new constant. Phase 3's
  `STRATEGIES_SPARKLINE_DEFERRED` retires from
  [`crates/ui/src/strings.rs:261`](../../../crates/ui/src/strings.rs);
  consistency-test fixture allow-list updates same commit.
- **R13.8** Empty / error — empty series renders R4.7-style
  empty state at sparkline footprint;
  `Error(msg)` renders `frame::muted_body(format!("Equity
  history unavailable: {msg}"))`.
- **Acceptance:** `strategies_screen__sparkline_present.snap`
  PASS; `…__sparkline_deferred.snap` **deleted**;
  `strategies_screen_sparkline_replaces_placeholder` integration
  test PASSES.

### R14 — No-CTA exclusion

- **R14.1** No "Deploy live" button (Lumen `Backtest.jsx:76`
  panel `actions` slot omitted entirely). No write closures.
- **R14.2** No "Export" button either (Lumen
  `Backtest.jsx:75`); operator's existing tooling
  (`cat spec/*/reports/backtest-*.md`, editor inspection) covers
  export. Read-only surface.
- **R14.3** A future spec-only update may document the
  "Backtest viewer" section in the principles doc; Phase 4
  does not edit the principles doc.
- **Acceptance:** viewer ships zero buttons in its top chrome;
  snapshot baselines V1 / V2 / V4 / V5 confirm absence (no
  `button:` rows).

### R15 — Single-binary scope

- **R15.1** Phase 4 touches **two binary surfaces**:
  - **`viewer`** — the new bin (R1).
  - **`cockpit_live` + `cockpit`** — additive sparkline
    consumer on Strategies-detail (R13). No chrome change
    beyond placeholder swap; sidebar / status bar / 6
    screens unchanged.
- **R15.2** No new sidebar entry. Viewer is a separate
  top-level bin per architecture.md:2947–2951.
- **R15.3** No new `EventBus` channel — R12 is a query, not a
  subscription; R13.4 wires via `Task::perform`, not
  `Subscription::batch`.
- **Acceptance:** `cargo build -p ui --bins` produces three
  binaries; cockpit-shell snapshots (Phase 1 / 2 / 3)
  byte-identical except the R13 sparkline diff.

### R16 — Cross-feature invariants preservation

Every prior shipped feature passes. No new invariants beyond
the master roadmap.

- **R16.1** `operator-success-reports` — companion CSV is
  **read** by R11.2; never written. Viewer has no status bar
  so no latency-band regression surface.
- **R16.2** `live-cockpit-unified` — `cockpit_live` launches
  unchanged; halted-banner triggers preserved. Sparkline is
  additive on screen body.
- **R16.3** `real-mtm-unrealized-pnl` — P&L card unchanged;
  `strategy_equity` is sibling of `pnl`, not a replacement.
- **R16.4** `per-symbol-position-accounts` — unchanged.
- **R16.5** `tape-row-audit-modal` — modal trigger / frame
  unchanged.
- **R16.6** `journal-tx-metadata` — unchanged.
- **R16.7** `v1.5b-multi-venue` — R12 query does not filter by
  venue (equity is strategy aggregate, not per-venue); no
  venue-column ripple.
- **Acceptance:** tester's per-feature invariant table reads
  7/7 PASS.

### R17 — Anchor regression (11/11 byte-identical)

- **R17.1** `verify-anchors` PASS — 11/11 byte-identical.
- **R17.2** No re-render of any committed report. R3 reads
  bodies as-is; R11.2 reads companion CSVs as-is.
- **R17.3** No re-anchor budget. Q3b sidecar (if chosen) ships
  alongside *future* reports, not as edits to past ones.
- **R17.4** Viewer is **read-only on the spec tree**
  (architecture.md:3116). Build-time test asserts no
  `std::fs::File::create` against `spec/**`.
- **Acceptance:** `verify-anchors` 11/11 PASS at tester gate.

## Verification (V-items)

Numbered, each with a precise test command + expected output.

- **V1 — Viewer bin launches with sample report.**
  `cargo run -p ui --bin viewer --
  spec/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md`
  opens a window titled `"Backtest report —
  btc-2023-1m-rsi-reversion"`; KPI strip + equity curve +
  drawdown band + markdown body all render. (R1, R2, R4, R7, R9.)

- **V2 — KPI strip renders six metrics correctly.**
  `viewer__kpi_strip__sample_report.snap` PASSES with the
  RSI sample's expected values (Total return `−57.80 %` in
  `DOWN_500`, CAGR `<computed>`, Sharpe `−55.4257`, Max DD
  `−57.81 %` in `DOWN_500`, Win rate `<computed>`, Trades `14118`).
  (R2, R3.)

- **V3 — KPI strip parser correctness over all 11 reports.**
  `cargo test -p reports parse::tests::all_anchored_reports_parse_ok`
  PASSES — the new parser ingests every committed report under
  `spec/*/reports/backtest-*.md` (the 11 anchored + any extras)
  without error; no field aborts the parse. (R3.)

- **V4 — Equity-curve baseline.**
  `viewer__equity_curve__sample_report.snap` PASS; canvas
  carries 5 horizontal gridlines, `ACCENT` polyline,
  `UP_500 @ 0.18` filled area beneath. (R4, R5, R6.)

- **V5 — Drawdown band baseline.**
  `viewer__drawdown_band__sample_report.snap` PASS; canvas
  carries 5 horizontal gridlines, `DOWN_500` polyline,
  `DOWN_500 @ 0.18` filled area. (R7, R8.)

- **V6 — `EquitySeries` ctor + downsample correctness.**
  `cargo test -p trading-core equity_series::tests` PASSES —
  `from_points_computes_drawdown_correctly`,
  `downsample_to_2000_preserves_peak_and_trough`,
  `non_monotone_timestamps_returns_err`,
  `empty_input_returns_err`. (R10.)

- **V7 — Audit query online consumer.**
  `cargo test -p audit
  query::tests::equity_curve_for_strategy_*` PASSES — three
  unit tests covering window-subset, empty-window, and
  `until=None` to-now. (R12.)

- **V8 — Cockpit sparkline replaces placeholder.**
  `strategies_screen__sparkline_present.snap` PASS;
  `strategies_screen__sparkline_deferred.snap` is **deleted**
  in the same commit; consistency-test fixture allow-list
  updated. (R13.)

- **V9 — Markdown body byte-identical.**
  `verify-anchors` 11/11 PASS. The viewer never writes to
  `spec/`; tested via a unit assertion that the viewer bin's
  binary declares no write-path against `spec/**`. (R9, R17.)

- **V10 — No-CTA absence.**
  Snapshot baselines V1 / V2 / V4 / V5 carry zero
  `button:` rows in their summary section; the viewer surface
  is read-only by visual inspection. (R14.)

- **V11 — Cross-feature invariants.**
  Tester's 7/7 per-feature invariant table PASS. (R16.)

- **V12 — Snapshot baselines.**
  Single `cargo insta accept` pass at end of phase; ~5
  net-new + 1 deleted (the Phase 3 deferred placeholder). All
  Phase 1 / 2 / 3 baselines byte-identical. (R13, R15.)

- **V13 — `rust-validate` PASS.**
  `cargo fmt`, `cargo clippy -- -D warnings`, `cargo deny
  check`, `cargo audit` all PASS. The new `viewer` bin
  inherits the workspace lints; the new `core::EquitySeries`
  type carries `Debug + Clone + PartialEq` derives.

- **V14 — `rust-build` PASS.**
  `cargo build -p ui --bins` produces three binaries
  (`cockpit`, `cockpit_live`, `viewer`). `cargo build -p
  reports`, `cargo build -p audit`, `cargo build -p
  trading-core` all PASS.

## Acceptance criteria

Phase 4 ships when all of the following hold:

- **The `viewer` binary launches** against any of the 11
  anchored backtest reports under `spec/<slug>/reports/`; KPI strip +
  equity curve + drawdown band + markdown body render in the
  documented layout. (R1, R2, R4, R7, R9.)
- **KPI strip values match** the report's existing summary table
  for the sample RSI-reversion report (V2). (R2, R3.)
- **Equity curve + drawdown band** render the documented
  colour / fill / gridline contract; baselines V4 / V5 PASS.
  (R4, R5, R6, R7, R8.)
- **`EquitySeries` primitive** is consumed cleanly by both
  surfaces — the viewer's offline source (R11) and the cockpit's
  online source (R12 + R13). (R10.)
- **Cockpit Strategies-detail sparkline** replaces the Phase 3
  deferred placeholder; the placeholder snapshot retires; the
  Phase 3 placeholder constant is removed from `ui::strings`.
  (R13.)
- **No "Deploy live" CTA. No "Export" CTA. No file-picker UI.**
  (R14, Q4.)
- **No `cockpit_live` chrome change beyond the sparkline
  placement on the Strategies-detail screen.** (R15.)
- **Cross-feature invariants PASS** (7/7) and **11/11 anchor
  regression PASS** (byte-identical bodies). (R16, R17.)
- **`rust-validate` + `rust-build` PASS.** Single `cargo insta
  accept` pass for ~5 net-new baselines + 1 deletion. (V12, V13,
  V14.)

## Open questions for architect

Q11–Q14 from the master roadmap are **operator-locked** and not
opened here (Q11 sidebar fixed-width — irrelevant to the viewer;
Q12 chart data both modes — irrelevant, viewer is offline-only;
Q13 extend `audit::query` — applies to R12 / Q7; Q14 Phase 2/3
split — past gate). The questions below are the genuinely-open
design choices that ratify at architect kickoff. Each ends with a
one-line **analyst recommendation** and one-line alternatives
considered.

### Q1 — `EquitySeries` field set: minimal vs richer?

**The question:** R10 — `EquitySeries` carries only
`points: Vec<(Timestamp, Money<Usdt>)>` (consumers compute
drawdown / peak / trough at render time), or the **richer**
shape with precomputed `drawdown_pct`, `peak`, `trough`,
`max_drawdown_pct`?

**Recommended (analyst):** **richer**. Two consumers (viewer +
cockpit sparkline) need the same drawdown vector; computing
twice per render introduces divergence risk. Single `O(N)`
`Decimal` walk at build time is trivial.

**Alternatives considered:** minimal — rejected, forces every
consumer to re-implement the drawdown walk; precision-bug
divergence risk between consumers.

### Q2 — Chart-widget reuse: share or copy?

**The question:** R5 — share `widgets::chart` between cockpit
(Phase 2 / 3 / R13 consumer) and viewer (R4 / R7) by factoring
into a `widgets::canvas_chart` core + thin wrappers, OR copy
the primitives into a viewer-local module?

**Recommended (analyst):** **share, via `widgets::canvas_chart`
core**. Single source of truth for canvas drawing; copy
accumulates two-source-of-truth risk on already-stable
primitives. Bounded refactor — Phase 2 widget exposes
`draw_gridlines`, `inner_rect`, `with_alpha` as `pub(crate)`;
factor pulls them into a sibling module. Phase 2's public
`view` signature stays byte-stable.

**Alternatives considered:** copy — rejected, divergence risk.
Re-export Phase 2 internals without refactor — rejected,
visibility is intentional; explicit factor is honest.

### Q3 — KPI source format: report body vs sidecar JSON?

**The question:** R3 — KPI strip parses the report's existing
markdown summary table (Q3a — no new artefact), or reads a
parallel `report.json` sidecar (Q3b — new artefact)?

**Recommended (analyst):** **stable-contract via the existing
markdown body (Q3a)**. The 11 anchored reports already carry
the six metrics in the summary table; parsing adds a parser,
no new artefact, no write-path change to `crates/reports`, no
backfill question for past reports, no two-sources-of-truth
divergence.

**Alternatives considered:** **sidecar `report.json` (Q3b)** —
pro: typed contract. Con: write-path change in
`crates/reports`; backfill for past reports; reconciliation
overhead between table and JSON. Rejected on cost / benefit.
Note: the equity-points contract (R11) is independent — Q3
picks the *KPI* contract; R11 reuses the existing companion
CSV at `<stem>__equity.csv` regardless.

### Q4 — Operator-defined report file picker: CLI-only or UI?

**The question:** R1.2 — viewer accepts only a CLI-arg report
path, or grows a file-picker UI (`Open report…`)?

**Recommended (analyst):** **CLI-only**. Matches the v1
"single operator, config-driven" non-goal. Operator's
existing workflow is shell pipeline; in-app picker adds
widget surface (file dialog, recents list) without operator
value. A future phase may add a recents list if asked.

**Alternatives considered:** file-picker via `iced_aw` —
rejected, out-of-band of the "config-driven, operator-typed
paths" discipline.

### Q5 — Equity-curve large-report performance: point cap?

**The question:** R6.3 — upper bound on `series.points.len()`
before the canvas render stutters? Typical case 60 ≤ N ≤
~2000 (downsampled by the report's 1m / 5m cadence).

**Recommended (analyst):** **cap at 2000 points** at
series-build time. iced 0.14 `tiny-skia` paints 2000 polyline
segments comfortably within a 16 ms frame; the report's
sparkline ships 60 chars so 2000 is 33× higher resolution
plenty of detail; 90-day 1-min (~129 600 points) is
deliberately out of scope — downsample at metric-emit time.
`EquitySeries::downsample(2000)` enforces the cap.

**Alternatives considered:** no cap — render-budget risk;
cap at 500 — loses fidelity for 1-day high-cadence reports.

### Q6 — Drawdown band fill style: solid or gradient?

**The question:** R7.2 — drawdown band fills as solid
`DOWN_500 @ 0.18`, or as a gradient (Lumen equity uses a
gradient at `Backtest.jsx:94`; Lumen drawdown at line 103
already uses **flat** `fillOpacity="0.18"`)?

**Recommended (analyst):** **solid** at alpha 0.18.
Consistency with the Phase 2 line-fill style + with Lumen's
own drawdown band specifically (line 103). Gradients in iced
0.14 require `Brush::Gradient` paths — extra complexity below
the operator's "perceptible difference at workstation
distance" threshold. One fewer code path to test.

**Alternatives considered:** gradient — rejected, visual
delta below threshold; the equity curve also goes solid
(R4.4) for consistency.

### Q7 — `equity_curve_for_strategy` signature

**The question:** R12.2 — exact signature. Options:
`(strategy_id, since: Timestamp, until: Option<Timestamp>)`
vs `(strategy_id, since, until: Timestamp)` (caller passes
`now()`) vs `(strategy_id, window: Range<Timestamp>)`.

**Recommended (analyst):** **`(strategy_id, since: Timestamp,
until: Option<Timestamp>) -> Result<EquitySeries, _>`**.
`Option<until>` is explicit about "to now" semantics; sibling
consistency with the Phase 3 `recent_journal_filtered` style
(positional timestamps, not `Range`); the cockpit consumer
(R13.4) calls `until: None`, saving a clock read at the
call-site.

**Alternatives considered:** `Range<Timestamp>` — rejected,
inconsistent with sibling-method style; `until: Timestamp`
no-`Option` — rejected, forces caller-side clock read.

### Q8 — Strategies-detail sparkline placement

**The question:** R13 — sparkline above the signal-events list
(top-right per Phase 3 R6.1), or to the right of the params
block?

**Recommended (analyst):** **above (top-right)** — matches
Phase 3 R6.1 (which the deferred placeholder already
occupies); operator's normal top-down scan order on a detail
screen.

**Alternatives considered:** right-of-params — rejected,
narrow-width regression; bottom — rejected, low in scan order
for an "is this working" signal.

### Q9 — Cockpit sparkline render budget (live data)

**The question:** R13.5 / R13.6 — cockpit sparkline caps at N
points (downsample at fetch, no live update), or rebuilds the
full series on each fill (live but expensive)?

**Recommended (analyst):** **cap + downsample at fetch time;
no live update**. The sparkline is a quick visual, not a
real-time monitor — operator switches to Audit screen for
"what just happened". Live-rebuild couples render rate to
ledger write rate, violating the "screens are pure render
dispatches" invariant. ~120-point cap matches the sparkline's
120 px width.

**Alternatives considered:** live-update via 1-Hz
`Subscription` — rejected, bus-channel cost for marginal
value; future phase may add. No cap — render-budget risk.

### Q10 — Viewer dark-default cold-start

**The question:** viewer cold-start theme inherits the
cockpit's dark default (master Q10), or follows OS-level
dark-mode preference?

**Recommended (analyst):** **dark default**, inherits cockpit.
Consistency with both cockpit bins; no OS-preference plumbing
exists today; long-session-at-a-desk operator context fits
dark.

**Alternatives considered:** OS detection via `dark-light`
crate — rejected, new dependency for marginal value; light
default — rejected, inconsistent with cockpit.

### Q11 — Phase 4 snapshot baseline budget

**The question:** how many new baselines and is the refresh
single-pass or staged?

**Recommended (analyst):** **~5 net-new + 1 deletion; single
`cargo insta accept` pass at end of phase**. Phase 1 Q2 /
Phase 2 V11 / Phase 3 V12 precedent. Phase 1 / 2 / 3
baselines stay byte-identical (viewer is a separate bin; the
cockpit-side diff is local to the sparkline placement).

**Alternatives considered:** staged (KPI first, curve next,
band next) — rejected, three review passes for tightly-
coupled visual is overhead without value.

### Q12 — `EquitySeries` module placement in `core`

**The question:** R10.1 — `EquitySeries` lives in a new
module `crates/core/src/equity_series.rs`, or as a sibling
of `views.rs::PnlSnapshot`?

**Recommended (analyst):** **new module
`crates/core/src/equity_series.rs`**. The type carries
non-trivial constructor logic (R10.3 drawdown walk +
peak / trough); `views.rs` is a "thin DTOs" file by
convention. Test module has space for the four R10 unit
tests without crowding.

**Alternatives considered:** sibling in `views.rs` —
rejected, logical-group mismatch (`views.rs` is read-side
projections, not computed aggregates).

## Backlog updates

Effective on this brief's promotion (2026-05-06):

### Active

- **`lumen-phase-4-backtest-panel`** — this brief, expanded
  from stub status (109-line, queued, scope outline only) to
  active. Status: `active`. Owner: analyst. Pipeline next stage:
  **architect**.

### Queue (unchanged from master roadmap)

- **`lumen-phase-5-humancontrol-agentfeed`** — promotes on
  Phase 4 ship. Status: queued.
- **`lumen-phase-6-assistant-slot`** — reserved, linked to v2
  LLM. No analyst spawn until v2 LLM is approved.

### Recent (shipped)

- **`lumen-phase-3-detail-screens`** — shipped 2026-05-05 /
  approved 2026-05-06.
- **`lumen-phase-2-shell-ia-charts`** — shipped 2026-05-05.
- **`lumen-phase-1-foundation`** — shipped 2026-05-04.

### Stub supersede note

The 2026-05-04 stub of this brief (109 lines, queued status,
high-level scope only) is **superseded by this expansion**.
The Why section is preserved verbatim and extended with the
two-surfaces / one-primitive cross-link to Phase 3's Q6
deferral; Scope is replaced by the R-cluster-pointing summary;
Open questions are replaced by the architect Q-items below;
Acceptance criteria are extended to trace each bullet to its
R-cluster. Master roadmap reference unchanged: see
[`lumen-design-adoption.md` Phase 4 section](../feature.md).

## Cross-phase technical-debt — TD-1 keyboard focus ring

**TD-1 status check at Phase 4 analyst kickoff (2026-05-06).**
Re-verified
[`crates/ui/Cargo.toml:52`](../../../crates/ui/Cargo.toml) still
pins `iced = "=0.14.0"`. Neither upgrade trigger named in the
master roadmap's TD-1 row has fired (iced 0.15+ has not
shipped to crates.io with `button::Status::Focused` and
`text_input::Style.shadow`; no custom-widget escape hatch has
been authored). **Phase 4 restates the deferral** — the
Phase 1 hover-state ring + ACCENT input border-shift continue
as the shipped approximation across the cockpit. **The viewer
inherits this deferral** — the viewer's surface is read-only
(no buttons, no inputs per R14), so the focus-ring gap doesn't
even surface; the deferral is operationally invisible on the
viewer. Next re-evaluation: **Phase 5 (HumanControl) analyst
kickoff**, post Phase 4 ship. HumanControl is the first phase
introducing new operator-write controls (pause-strategy,
override-risk-veto) where the focus-ring ergonomic gap
sharpens; the architect re-evaluates at that point. Phase 4's
zero-button viewer surface and additive cockpit sparkline
(read-only, not focusable) leave the operator-impact bound
unchanged from Phase 1.

## Design

_Architect-owned. Resolves Q1–Q12 — every recommendation lands as
**ratified** unless flagged "Architect override". The analyst sections
above are immutable; this section is the design contract the developer
reads alongside the task list at
[`spec/lumen-design-adoption/phase-4-backtest-panel/tasks.md`](tasks.md)._

### Q-item resolutions

All 12 architect Q-items resolved. **12 / 12 ratified, zero deviations
from analyst recommendation.** Each row cites the R-item(s) the
resolution lands. Phase 4 inherits more upstream primitives than Phase
3 inherited from Phase 2 (Phase 1 tokens / `theme::space` / `text::*` /
`frame::muted_body`; Phase 2 `widgets::chart` canvas helpers
`draw_gridlines` / `inner_rect` / `with_alpha` / 5-gridline density;
Phase 3 `PanelState`, the chip-select compound-dispatch wiring, the
in-binary `Task::perform` shim, the strategies-detail screen body),
so the Q resolutions are short and the design body below leans on
"sibling of …" framing throughout.

| Q   | Question                                                | Resolution                                                                                                                                                                                                                                                                                                                                                                          | Ratifies        |
|-----|---------------------------------------------------------|-------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|-----------------|
| Q1  | `EquitySeries` field set: minimal vs richer             | **Richer.** `points: Vec<EquityPoint>` (where `EquityPoint = { ts, equity, drawdown_pct }`) + `peak`, `trough`, `max_drawdown_pct`, `inception_ts`, `as_of_ts`. Two consumers (viewer offline + cockpit sparkline online) need the same drawdown vector; precomputing in one `O(N)` `Decimal` walk at build time eliminates per-render divergence risk and keeps consumers branchless. The drawdown vector lives **inside** `EquityPoint` (not a parallel `Vec<Decimal>` per the analyst sketch — the parallel-vector shape couples lengths implicitly and invites off-by-one bugs; a single struct per point is the boring shape). | R10.2–R10.5     |
| Q2  | Chart-widget reuse: share or copy                       | **Share, via a `widgets::canvas_chart` core + thin per-surface wrappers.** Phase 2's internal helpers (`draw_gridlines`, `inner_rect`, `with_alpha`) and the 5-gridline `BORDER_1 @ 0.4` constant promote to `pub(crate)` in a new `crates/ui/src/widgets/canvas_chart.rs`; Phase 2's existing `widgets::chart` becomes a wrapper that consumes the core for the Charts-screen surface (signature byte-stable). Phase 4 adds two sibling wrappers — `widgets::equity_curve` and `widgets::drawdown_band` — over the same core. Single source of truth for canvas drawing; copy-paste rejected on divergence-risk grounds. | R5.1–R5.4       |
| Q3  | KPI source format: report body vs sidecar JSON          | **Stable-contract via the existing markdown body (Q3a).** All 11 anchored reports already carry the six metrics (with CAGR and Win rate marked-absent on the live samples — see R3.5 missing-field tolerance) in the `## Summary` table; parsing adds a parser, no new artefact, no write-path change to `crates/reports`, no backfill question for past reports. New module `crates/reports/src/parse.rs` houses `BacktestMetrics::parse_from_report(path)`. **Failure mode is graceful** — if a future report-format change breaks the parser, the viewer renders the R2.6 empty state (six `—` dashes + `VIEWER_METRICS_UNAVAILABLE` muted-body) and continues to render the equity curve + drawdown band + body. Sidecar `report.json` rejected on cost/benefit (write-path ripple, backfill question, two-source-of-truth divergence). The equity-points contract (R11) is independent — it reuses the existing `<stem>__equity.csv` companion file regardless. | R3.1–R3.5       |
| Q4  | Operator-defined report file picker: CLI vs UI          | **CLI-only.** Matches the v1 "single operator, config-driven" non-goal. `clap`-parsed positional `<report-path>`; missing arg → exit 2; non-existent path → exit 3. No file-dialog widget surface, no recents list. A future phase may add a recents list if asked. | R1.2            |
| Q5  | Equity-curve point cap                                  | **Cap at 2000 points** at series-build time. iced 0.14 `tiny-skia` paints 2000 polyline segments comfortably within a 16 ms frame; the report's existing 60-cell sparkline is 33× lower resolution, so 2000 is plenty. `EquitySeries::downsample(2000)` enforces the cap via equal-stride bucketing (last-value-wins per bucket), `Decimal` arithmetic only. | R6.3            |
| Q6  | Drawdown band fill: solid vs gradient                   | **Solid `DOWN_500 @ 0.18`.** Consistency with the Phase 2 line-fill style and Lumen's own `Backtest.jsx:103` flat fill. Gradients in iced 0.14 require `Brush::Gradient` paths (extra complexity below the operator's "perceptible difference at workstation distance" threshold). Equity curve's `UP_500 @ 0.18` fill takes the same solid treatment for consistency. | R4.4, R7.2      |
| Q7  | `equity_curve_for_strategy` signature                   | **`(ledger: &Ledger, strategy_id: StrategyId, since: Timestamp, until: Option<Timestamp>) -> Result<EquitySeries, LedgerError>`.** `Option<until>` is explicit about "to now" semantics (caller saves a clock read); sibling-style consistency with Phase 3 `recent_journal_filtered` (positional timestamps, not `Range`). The query returns `EquitySeries` directly (Q1 — query owns the drawdown computation). Column projection: same `journal_entries je JOIN journal_transactions jt ON je.transaction_id = jt.id WHERE je.account_id = 'income:realized_pnl' AND jt.strategy_id = ? AND je.ts >= ? AND je.ts < ?` as `pnl_by_strategy`, but emits the **vector** of `(ts, running_equity)` running-sum samples ordered by `ts ASC, je.id ASC` rather than the single aggregate. | R12.1–R12.6     |
| Q8  | Strategies-detail sparkline placement                   | **Above (top-right of the chip row)** — matches the Phase 3 deferred-placeholder slot at `crates/ui/src/screens/strategies.rs:135`. Same 160 px-wide `Container`, same scan position; the change is "the placeholder retires; the canvas widget lands". Operator's normal top-down scan order on a detail screen. | R13.1, R13.2    |
| Q9  | Cockpit sparkline render budget                         | **Cap + downsample at fetch; no live update.** ~120-point cap matches the sparkline's ~120 px width (R13.5). The fetched series goes through `EquitySeries::downsample(120)` before landing on `Cockpit::strategy_equity`. No `Subscription::batch` recipe — refresh is one-shot on `Message::SelectStrategy(id)` (Phase 3 Q11b compound-dispatch) firing a `Task::perform(audit::query::equity_curve_for_strategy(…))`. Live-rebuild via 1-Hz subscription rejected — couples render rate to ledger write rate, violates the "screens are pure render dispatches" invariant; future phase may add. | R13.3–R13.6     |
| Q10 | Viewer dark-default cold-start                          | **Dark default, inherits cockpit.** Phase 1 `theme::ThemeMode::Dark` is the cold-start; the viewer threads the same default through `Model::new()`. No OS-detection plumbing; long-session-at-a-desk operator context fits dark. | R1.1            |
| Q11 | Snapshot baseline budget                                | **5 net-new + 1 deletion; single `cargo insta accept` pass at end of phase.** `viewer__kpi_strip__sample_report.snap` + `viewer__equity_curve__sample_report.snap` + `viewer__drawdown_band__sample_report.snap` + `viewer__full_view__sample_report.snap` (the 4 viewer-bin baselines) + `strategies_screen__sparkline_present.snap` (replaces the deferred placeholder); `strategies_screen__sparkline_deferred.snap` retires (deleted in same commit). Phase 1 / 2 / 3 baselines stay byte-identical (viewer is a separate bin; the cockpit-side diff is local to the sparkline placement). | All visual R-items |
| Q12 | `EquitySeries` module placement in `core`               | **New module `crates/core/src/equity_series.rs`.** The type carries non-trivial constructor logic (drawdown walk + peak/trough); `views.rs` is a "thin DTOs" file by convention (Phase 2/3 `JournalRow` / `RiskTelemetry` go in `lib.rs` directly because they're plain structs without behaviour). The new module exposes `EquitySeries`, `EquityPoint`, `EquitySeriesError`; `crates/core/src/lib.rs` re-exports. Test module `equity_series::tests` has space for the four R10.5 unit tests + downsample edge cases without crowding. | R10.1, R10.6    |

**No principled overrides.** Analyst recommendations are
operator-aligned and consistent with the master roadmap's
operator-locked Q11–Q14, the cross-feature invariant table, and the
zero-anchor-risk discipline; the architect ratifies all twelve.

### `core::EquitySeries` primitive

**File:** `crates/core/src/equity_series.rs` (new module per Q12).
Re-exported from `crates/core/src/lib.rs` next to the existing
`EquitySample` re-exports (`EquitySample` is the reports-side
companion-CSV row type; `EquitySeries` is the cross-phase shape that
both consumers build from those rows + audit walks).

```rust
//! Cross-phase equity-history primitive — Phase 4 (R10).
//!
//! Two consumers build this shape from different sources:
//! * `viewer` (offline) — `crates/reports/src/parse.rs::equity_series_from_report`
//!   reads the report's `<stem>__equity.csv` companion file (R11.2).
//! * `cockpit` (online) — `audit::query::equity_curve_for_strategy`
//!   walks the realized-pnl journal rows (R12, Q7).
//!
//! Consumers never recompute drawdown / peak / trough / max-DD;
//! `EquitySeries::from_points` does it once in an O(N) `Decimal`
//! walk at build time (Q1).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::{Money, Timestamp, Usdt};

/// One point on the equity curve. Drawdown is precomputed against the
/// running peak so render-time consumers branchless-render straight from
/// the struct (Q1 — no parallel vectors, no off-by-one risk).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquityPoint {
    pub ts: Timestamp,
    pub equity: Money<Usdt>,
    /// `(running_peak - equity) / running_peak`, in fractional units
    /// (0.0 = at peak; 0.10 = 10 % below peak). Always non-negative;
    /// monotone-up runs leave this at `Decimal::ZERO`.
    pub drawdown_pct: Decimal,
}

/// Equity history with precomputed peak / trough / max-DD metadata.
/// Pure-data, `serde`-friendly. No clock reads, no `f64`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquitySeries {
    /// Oldest-first; `points[0].ts == inception_ts`,
    /// `points[N-1].ts == as_of_ts`.
    pub points: Vec<EquityPoint>,
    pub inception_ts: Timestamp,
    pub as_of_ts: Timestamp,
    pub peak: Money<Usdt>,
    pub trough: Money<Usdt>,
    /// Max of `points[i].drawdown_pct`; `Decimal::ZERO` for monotone-up
    /// inputs. Stored separately from a per-point lookup so KPI
    /// consumers (the viewer's strip) can render `Max DD` without
    /// re-walking the vector.
    pub max_drawdown_pct: Decimal,
}

#[derive(Debug, thiserror::Error)]
pub enum EquitySeriesError {
    #[error("equity series cannot be empty")]
    Empty,
    #[error("timestamps must be monotone non-decreasing")]
    NonMonotoneTimestamps,
}

impl EquitySeries {
    /// Single O(N) `Decimal` walk: running_peak / running_trough /
    /// drawdown vector / max-DD all computed in one left-to-right pass.
    pub fn from_points(
        points: Vec<(Timestamp, Money<Usdt>)>,
    ) -> Result<Self, EquitySeriesError> { /* ... */ }

    /// Equal-stride bucketing, last-value-wins per bucket; preserves
    /// `points[0]` and `points[N-1]` exactly so peak / trough / inception /
    /// as-of survive the downsample. Panics on `max_points == 0`
    /// (caller bug); short-circuits when `self.points.len() <= max_points`.
    pub fn downsample(self, max_points: usize) -> Self { /* ... */ }
}
```

**Constructor invariants** (enforced by `from_points`):

- Empty input → `Err(EquitySeriesError::Empty)`. Both consumers handle
  the empty case before calling (R4.7 / R7.5 viewer empty states; R13.8
  cockpit sparkline empty state).
- Non-monotone timestamps → `Err(EquitySeriesError::NonMonotoneTimestamps)`.
  The audit walk emits `ORDER BY ts ASC` (Q7); the report companion CSV
  is written in cadence-order (`render::equity_curve.rs`); both sources
  satisfy monotone by construction. The error variant exists so test
  fixtures can cover the contract explicitly.

**Mandatory unit tests** in `crates/core/src/equity_series.rs::tests`:

- `from_points_computes_drawdown_correctly` — five-point series with a
  known peak / trough; assert per-point drawdown vector matches a
  hand-computed reference.
- `from_points_monotone_up_returns_all_zero_drawdown` — five strictly
  increasing equities → every `drawdown_pct == Decimal::ZERO`,
  `max_drawdown_pct == Decimal::ZERO`, `trough == points[0].equity`.
- `from_points_50_percent_drawdown_then_recovery` — peak at index 1,
  trough at index 3, recovery to a new peak at index 5; assert
  `max_drawdown_pct ≈ Decimal::new(50, 2)`, `trough ==
  points[3].equity`.
- `from_points_empty_returns_err` — `vec![]` → `Err(Empty)`.
- `from_points_non_monotone_returns_err` — timestamps `[t0, t1, t0]`
  → `Err(NonMonotoneTimestamps)`.
- `downsample_to_2000_preserves_peak_and_trough` — 5000-point
  synthetic series with a known peak / trough; downsample to 2000;
  assert peak / trough survive byte-identically.
- `downsample_below_target_is_noop` — 100-point series + `downsample(2000)`
  returns `points.len() == 100` (no oversampling, no re-bucketing).

### Cockpit state diff

The state diff `crates/ui/src/state.rs` receives in Phase 4 is **small**
— Phase 4's only cockpit-side change is the equity-history mirror used
by the Strategies-detail sparkline (R13). The viewer is a separate bin
(no `Cockpit` extension); the KPI strip + curve / band widgets live in
the viewer's own `Model`.

```rust
// ── crates/ui/src/state.rs — Phase 4 additions ─────────────────────────────

/// Phase 4 — per-strategy equity history mirror for the
/// Strategies-detail sparkline (R13). Keyed on `StrategyId`; populated
/// one-shot on `Message::SelectStrategy(id)` via a `Task::perform`
/// dispatch in the binary (Q9 — no live update). `Loading` while the
/// query is in flight; `Ready(series)` on success; `Error(msg)` on
/// `LedgerError`.
pub struct Cockpit {
    // … all existing Phase 1 / 2 / 3 fields …

    // ── Phase 4 — Backtest-panel cross-link ─────────────────────────────
    /// Read-only mirror of `audit::query::equity_curve_for_strategy`
    /// results, keyed on `StrategyId`. Entry inserted at first
    /// `SelectStrategy(id)`; replaced on subsequent re-selects of the
    /// same id (one-shot semantics — operator switching screens triggers
    /// a fresh fetch). Cleared only on cockpit restart (session-scoped
    /// per Phase 3 Q5).
    pub strategy_equity: HashMap<StrategyId, PanelState<EquitySeries>>,
}

pub enum Message {
    // … all existing Phase 1 / 2 / 3 variants …

    // ── Phase 4 — Strategies sparkline cross-link ───────────────────────
    /// Async result of `audit::query::equity_curve_for_strategy`.
    /// `Ok(series)` → `strategy_equity.insert(id, Ready(series))`;
    /// `Err(msg)` → `strategy_equity.insert(id, Error(msg))`. Pure
    /// assignment — async work lives in the binary's `Task::perform`
    /// shim. The series has already been `downsample(SPARKLINE_POINT_CAP)`-d
    /// before landing here (Q9 — cap at fetch, not at view time).
    StrategyEquityRefreshed(StrategyId, Result<EquitySeries, SmolStr>),
}
```

**`Default` impl extension.** `strategy_equity: HashMap::new()`.

**Message-handler diff.** One new arm, pure assignment:

```rust
Message::StrategyEquityRefreshed(id, Ok(series)) => {
    model.strategy_equity.insert(id, PanelState::Ready(series));
}
Message::StrategyEquityRefreshed(id, Err(msg)) => {
    model.strategy_equity.insert(id, PanelState::Error(msg));
}
```

**Compound dispatch wired in the binary** (R13.4 / Q9). Phase 3's
`Message::SelectStrategy(id)` arm in `crates/ui/src/bin/cockpit.rs` +
`cockpit_live.rs` is **extended** to chain the equity-curve fetch:

```rust
Message::SelectStrategy(id) => {
    // 1. Pure update (assignment to selected_strategy + Loading marker).
    model.strategy_equity
        .insert(id.clone(), PanelState::Loading);
    // 2. Phase 3 compound dispatch — only if click came from Home.
    let switch_task = if cockpit.current_screen != Screen::Strategies {
        Task::done(Message::SwitchScreen(Screen::Strategies))
    } else {
        Task::none()
    };
    // 3. Phase 4 — fire the equity-curve fetch (cockpit_live only;
    //    cockpit fixtures uses a synthetic series via `fake_equity_series`).
    let fetch_task = Task::perform(
        audit::query::equity_curve_for_strategy(
            ledger.clone(), id.clone(), strategy_inception_ts(&id), None,
        )
        .map(|res| res.map(|s| s.downsample(SPARKLINE_POINT_CAP))),
        move |res| Message::StrategyEquityRefreshed(
            id.clone(),
            res.map_err(|e| SmolStr::from(e.to_string())),
        ),
    );
    Task::batch(vec![switch_task, fetch_task])
}
```

`SPARKLINE_POINT_CAP = 120` lives in `theme::layout` next to
`AUDIT_PAGE_SIZE` (the Phase 3 sibling). `strategy_inception_ts(&id)`
is a small helper on the binary side that reads the strategy's load
timestamp from `Cockpit::strategies` (Phase 1 R5 already populates the
buffer); for fixtures-mode the helper returns a deterministic
24-hour-ago anchor.

### Viewer binary contract

**File:** `crates/ui/src/bin/viewer.rs` (new — created from scratch
per analyst's R1.1 note that the viewer bin doesn't exist on disk yet).
Sibling of the existing `cockpit.rs` and `cockpit_live.rs`; uses the
same `iced::application` functional builder pattern.

**CLI args** (R1.2 / Q4 — CLI-only):

```rust
#[derive(Parser)]
#[command(name = "viewer", about = "Backtest report viewer")]
struct Args {
    /// Path to a backtest report under `spec/*/reports/backtest-*.md`.
    report_path: PathBuf,
}
```

Exit codes: missing arg → `clap` default 2; non-existent file → 3
(custom early check before iced boots).

**Model** (the viewer's own — independent of `Cockpit`):

```rust
pub struct ViewerModel {
    pub mode: ThemeMode,                    // Dark default per Q10.
    pub report_path: PathBuf,
    pub front_matter: ReportFrontMatter,    // scenario, strategy id, generated_at — for the title bar
    pub metrics: PanelState<BacktestMetrics>,  // R2 / R3 KPI source
    pub equity: PanelState<EquitySeries>,   // R4 / R6 / R11 — companion CSV
    pub body_markdown: String,              // R9 — read once; never mutated
}

#[derive(Debug, Clone)]
pub enum ViewerMessage {
    /// One-shot load result fired at boot. Carries (front_matter, metrics, equity, body)
    /// so the model lands fully-populated on success and the curve / strip /
    /// body all render together. Field-level errors degrade to PanelState::Error
    /// independently — a missing equity CSV does not invalidate the KPI strip
    /// (R3.5 / R11.3 missing-field tolerance).
    ReportLoaded(Box<ReportLoadResult>),
    ToggleTheme,
}

pub struct ReportLoadResult {
    pub front_matter: ReportFrontMatter,
    pub metrics: PanelState<BacktestMetrics>,
    pub equity: PanelState<EquitySeries>,
    pub body_markdown: String,
}
```

**Window title** (R1.3): `format!("Backtest report — {}",
front_matter.scenario)`. **Master Constraint 1 — no `"Lumen"` in the
title.**

**Window** (R1.4): 1200 × 800 initial, resizable. Tier 0 `CANVAS`
outside the panel; Tier 1 `PANEL` inside the report shell.

**Subscription**: `Subscription::none()` — viewer is offline /
single-shot, no live channels.

**Update** is pure assignment — `ReportLoaded` lands the four sub-states
on the model; `ToggleTheme` flips `mode`. **No status bar** (R1.5);
viewer has nothing to anchor.

**View composition** (top-to-bottom, R9.4):

```rust
fn view(model: &ViewerModel) -> Element<'_, ViewerMessage> {
    let strip = widgets::kpi_strip::view(&model.metrics, model.mode);  // ~80 px
    let curve = widgets::equity_curve::view(&model.equity, model.mode); // ~240 px
    let band  = widgets::drawdown_band::view(&model.equity, model.mode); // ~100 px
    let body  = body_render::view(&model.body_markdown, model.mode);    // Length::Fill, scroll
    container(column![strip, curve, band, body].spacing(space::M))
        .padding(space::L)
        .style(panel_style(model.mode))
        .into()
}
```

`body_render::view` is a thin module that renders the markdown body
verbatim as monospaced `text::BODY` inside a `Scrollable` (R9.2 —
**rejected `pulldown-cmark`** as a new workspace dep; the operator's
existing flow is `cat` to a terminal, so monospace + scroll is the
honest match for that mental model and avoids the markdown-renderer
surface). Heading-level styling (`# / ## / ###`) is handled by a small
in-module pre-pass that emits `text::H2` / `text::H3` lines while
leaving table / code / paragraph rows as `text::BODY` — ~30 LOC, no new
dep.

**Boot** (`fn main`): parse CLI → emit a synchronous load (tokio
`block_on` is fine — single-shot CLI tool, not a service) producing
`ReportLoadResult` → `iced::application(...)` with the result threaded
into `ViewerModel` via the `new` initializer.

### KPI strip widget contract

**File:** `crates/ui/src/widgets/kpi_strip.rs` (new).

**Public API** (R2.1):

```rust
pub fn view<'a>(
    metrics: &'a PanelState<BacktestMetrics>,
    mode: ThemeMode,
) -> Element<'a, ViewerMessage>;
```

**Layout** (R2.2 — Lumen `Backtest.jsx:80–87`): one `Row` with six
equal-width metric cards, gap `space::M`. Each card is a 2-line
`Column`: label (`text::SMALL` `FG_3` muted) over value (`text::H1`
24 px `FG_1`). No card border; gap-only separation. Outer container
is Tier 1 `PANEL`.

**Sentiment colouring** (R2.4):

| Card        | Source field                   | Colour rule                                                                                                  |
|-------------|--------------------------------|--------------------------------------------------------------------------------------------------------------|
| Total return| `total_return_pct`             | `> 0` → `UP_500`; `< 0` → `DOWN_500`; `0` → `FG_1`. Prefix negative with `theme::num::MINUS_SIGN_LITERAL`.  |
| CAGR        | `cagr_pct` (if `cagr_present`) | `FG_1` (neutral). `—` if absent.                                                                              |
| Sharpe      | `sharpe`                       | `FG_1`. `Decimal` rendered to 4 dp via `widgets::num::format_sharpe`.                                         |
| Max DD      | `max_drawdown_pct`             | **Always** `DOWN_500`; prefix with `MINUS_SIGN_LITERAL`. `—` if `max_drawdown_pct.is_zero()` AND series empty.|
| Win rate    | `win_rate_pct` (if present)    | `FG_1`. `—` if absent (the 11 anchored reports do not surface win rate today; tolerance per R3.5).            |
| Trades      | `trades: u64`                  | `FG_1`. Render via `widgets::num::format_count`.                                                              |

**Empty / error state** (R2.6 / Q3 graceful fallback): when
`metrics == PanelState::Error(_)` or the parser returned a
`BacktestMetrics::all_absent()` shape, render six muted `—` dashes +
`frame::muted_body(strings::VIEWER_METRICS_UNAVAILABLE)` below.
`VIEWER_METRICS_UNAVAILABLE = "Backtest metrics unavailable"` —
net-new constant in `ui::strings`; additive (operator-locked
Constraint 2 unchanged).

**`BacktestMetrics`** lands in `crates/core/src/equity_series.rs`
alongside `EquitySeries` (one Phase 4 module, two related types) —
**Architect note:** moving from the analyst's R2.5 placement of
"`BacktestMetrics` lives in `core`" to the same module as `EquitySeries`
keeps the cross-phase primitive grouping coherent; both types travel
together to consumers. No effect on the consumer-side `use core::{…}`
shape.

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BacktestMetrics {
    pub total_return_pct: Decimal,
    pub cagr_pct: Decimal,
    pub cagr_present: bool,
    pub sharpe: Decimal,
    pub max_drawdown_pct: Decimal,
    pub win_rate_pct: Decimal,
    pub win_rate_present: bool,
    pub trades: u64,
}

impl BacktestMetrics {
    /// All-absent sentinel for the R2.6 / Q3 graceful-fallback path.
    /// The strip renders six `—` dashes + the muted-body line.
    pub fn all_absent() -> Self;
}
```

### Equity curve widget contract

**File:** `crates/ui/src/widgets/equity_curve.rs` (new).

**Public API** (R4.1):

```rust
pub fn view<'a>(
    series: &'a PanelState<EquitySeries>,
    mode: ThemeMode,
) -> Element<'a, ViewerMessage>;
```

**Implementation** (R5 / Q2 — composes the new `widgets::canvas_chart`
core):

- `widgets::canvas_chart::draw_gridlines` — Phase 2 helper promoted
  to `pub(crate)`. Five horizontal `BORDER_1 @ 0.4` gridlines (R4.5).
- `widgets::canvas_chart::inner_rect` — Phase 2 helper for the
  inset drawing rectangle.
- `widgets::canvas_chart::with_alpha` — Phase 2 helper for the
  `BORDER_1 @ 0.4` and `UP_500 @ 0.18` alpha blends.
- New `widgets::canvas_chart::polyline_with_fill` —  Phase 4 addition
  to the canvas-chart core. Takes `points: &[(Decimal, Decimal)]`
  (logical X, Y), `line_color`, `fill_color`, `fill_alpha`. Walks
  points oldest-to-newest, draws the polyline in `line_color`
  (1.5 px stroke per Phase 2 `LINE_STROKE_PX`), then closes a polygon
  down to the inner-rect bottom and fills with `with_alpha(fill_color,
  fill_alpha)`. **Single primitive** so the equity curve and the
  drawdown band share the implementation.

**Equity-curve specifics** (R4.2 / R4.3 / R4.4):

- X = index-based per Q5/R6.2 — `points[0]` left, `points[N-1]` right
  (the report writes the CSV at uniform cadence, so index-proportional
  and time-proportional render identically; index is cheaper).
- Y range = `(min(equity), max(equity))` + 5 % padding (mirrors Phase
  2's `RANGE_PAD_FRACTION = 0.05`).
- Polyline = `ACCENT` 1.5 px stroke (R4.3 — leaves `UP_500` for the
  fill).
- Filled area = solid `UP_500 @ 0.18` (R4.4 / Q6).
- Five horizontal gridlines = `BORDER_1 @ 0.4` (R4.5).
- No vertical grid; no hover; no zoom (R4.6).

**Empty / error state** (R4.7): `Loading` → centred Phase 1 skeleton;
`Empty` (post-`Ready` with `points.is_empty()`) → gridlines + centred
`frame::muted_body(strings::VIEWER_NO_EQUITY_DATA)` ("No equity data");
`Error(msg)` → `frame::muted_body(format!("Equity curve unavailable: {msg}"))`.

### Drawdown band widget contract

**File:** `crates/ui/src/widgets/drawdown_band.rs` (new).

**Public API** (R7.1):

```rust
pub fn view<'a>(
    series: &'a PanelState<EquitySeries>,
    mode: ThemeMode,
) -> Element<'a, ViewerMessage>;
```

**Specifics** (R7.2–R7.5):

- Composes the same `widgets::canvas_chart::polyline_with_fill` core.
- X = index-based, same as equity curve.
- Y = `(0, max_drawdown_pct)` **inverted** — 0 at top, `max_drawdown_pct`
  at the bottom (drawdown grows downward; matches Lumen
  `Backtest.jsx:67/103`). Implementation: pass the points as
  `(idx, max_dd - drawdown_pct[i])` so the polyline-with-fill core
  draws the curve's interior the right way up.
- Polyline = `DOWN_500` 1.5 px stroke (R7.2).
- Filled area = solid `DOWN_500 @ 0.18` (R7.2 / Q6).
- Height = ~100 px (R7.3 — Lumen ratio).
- Five horizontal gridlines = `BORDER_1 @ 0.4` (R7.4 — same density
  for visual rhythm with the equity curve).
- Empty / error states identical to equity curve (R7.5 → R4.7).

**Why a separate widget module** (rather than a composed fn that
returns both canvases): keeps each widget's snapshot baseline
self-contained (V4 / V5 are independent) and lets the viewer's `view`
fn lay them out in the parent column with `space::M` between, matching
the Lumen reference's ~240 px / ~100 px ratio cleanly.

### Audit query addition: `equity_curve_for_strategy`

**File:** `crates/audit/src/query.rs`. Sibling of `pnl_by_strategy`
(line 933) + Phase 2 `recent_fills_filtered` + Phase 3
`recent_journal_filtered`. Master Q13 — extend `audit::query` for
read-only additions.

**Exact signature** (Q7 ratification):

```rust
/// Phase 4 addition (R12 / Q7). Walk the `journal_entries` rows on the
/// `income:realized_pnl` account joined to their parent
/// `journal_transactions` row's `strategy_id`, emitting an
/// `EquitySeries` whose points are the running-sum of realized P&L
/// samples in the half-open window `[since, until_or_now)`.
///
/// Read-only over committed audit data; additive sibling of
/// [`pnl_by_strategy`]. The inception-equity baseline is read from
/// the same journal: the first sample carries the running cash
/// balance at `since` (computed via the `cash_balance` query at the
/// same instant), and each subsequent sample increments by the row's
/// `(credit - debit)` delta. This matches the existing single-aggregate
/// shape `pnl_by_strategy` returns.
///
/// `until = None` ↔ "to now" (the function reads `Timestamp::now()`
/// once at the call boundary). The cockpit consumer (R13.4) uses
/// `until: None` so the call-site doesn't read the clock.
///
/// # Errors
///
/// Returns [`LedgerError::Database`] on SQL or parse error. Returns
/// `Err(LedgerError::EmptyWindow)` when the window contains zero rows
/// (so the cockpit consumer can render the R13.8 empty state without
/// inspecting an `Ok(EquitySeries)` for `points.is_empty()` — keeps
/// the `from_points` `Empty` invariant load-bearing).
pub async fn equity_curve_for_strategy(
    ledger: &Ledger,
    strategy_id: StrategyId,
    since: Timestamp,
    until: Option<Timestamp>,
) -> Result<EquitySeries, LedgerError>;
```

**SQL** (column-projection over the same rows `pnl_by_strategy`
consumes):

```sql
SELECT je.ts, je.debit_amount, je.credit_amount
FROM journal_entries je
JOIN journal_transactions jt ON je.transaction_id = jt.id
WHERE je.account_id = 'income:realized_pnl'
  AND jt.strategy_id = ?
  AND je.ts >= ?
  AND je.ts <  ?
ORDER BY je.ts ASC, je.id ASC;
```

**Walk algorithm** (sibling of `pnl_by_strategy`'s aggregate walk):

```rust
let baseline = cash_balance(ledger).await?;  // existing query helper
let mut running = baseline.amount();
let mut points: Vec<(Timestamp, Money<Usdt>)> = Vec::with_capacity(rows.len());
for (ts_str, dr_str, cr_str) in rows {
    let ts: Timestamp = parse_rfc3339(&ts_str)?;
    let dr: Decimal = dr_str.parse().map_err(|_| LedgerError::Database("equity_curve_for_strategy: parse debit".into()))?;
    let cr: Decimal = cr_str.parse().map_err(|_| LedgerError::Database("equity_curve_for_strategy: parse credit".into()))?;
    running += cr - dr;
    points.push((ts, Money::<Usdt>::from_decimal(running)));
}
if points.is_empty() {
    return Err(LedgerError::EmptyWindow);
}
EquitySeries::from_points(points)
    .map_err(|e| LedgerError::Database(format!("equity_curve_for_strategy: {e}")))
```

**Determinism** (R12.5): `ORDER BY je.ts ASC, je.id ASC` — identical
oldest-first ordering as Phase 3's `recent_journal_filtered`. `Decimal`
arithmetic only; no `f64`. Reuses the existing 6-digit fractional-second
timestamp format (architect.md determinism guardrail).

**Mandatory unit tests** (R12.6) in `crates/audit/src/query.rs::tests`:

- `equity_curve_for_strategy_returns_window_samples` — seed a fixture
  ledger with 5 known realized-pnl rows in the window, assert the
  returned `points.len() == 5` and the running-equity walk matches a
  hand-computed reference.
- `equity_curve_for_strategy_empty_window_returns_empty_window_err` —
  empty window → `Err(LedgerError::EmptyWindow)`. Cockpit consumer
  renders R13.8 empty state.
- `equity_curve_for_strategy_until_none_includes_to_now` — seed a
  row at `now() - 5s`; call with `until = None`; assert the row is
  in the result.
- `equity_curve_for_strategy_filters_by_strategy_id` — seed two
  strategies' rows in the same window; assert the returned series
  reflects only the target strategy's rows.

**Mandatory integration test** (sibling of Phase 3
`crates/audit/tests/recent_journal_filtered.rs`) at
`crates/audit/tests/equity_curve_for_strategy.rs`: seeds a multi-day
multi-strategy fixture, asserts the cockpit consumer's expected shape
(60–120 points after `downsample(120)`, `from_points` `Ok` round-trip,
peak / trough / max-DD math against a hand-computed reference).

### Cockpit Strategies-detail sparkline (Phase 3 deferral closure)

**Phase 3 deferral retired.** The `STRATEGIES_SPARKLINE_DEFERRED`
constant at `crates/ui/src/strings.rs:261` and its rendering at
`crates/ui/src/screens/strategies.rs:135–139` retire in this phase.
Replacement:

- **Constant change** — `STRATEGIES_SPARKLINE_DEFERRED` removed; new
  `STRATEGIES_SPARKLINE_LOADING = "Loading equity history…"` lands
  next to it (additive — Constraint 2 unchanged).
- **Render call-site at `screens/strategies.rs:135`** — the
  `Container::new(muted_body(STRATEGIES_SPARKLINE_DEFERRED)).width(Length::Fixed(160.0))`
  becomes a thin dispatch on
  `cockpit.strategy_equity.get(&id_or_default)`:
  ```rust
  let sparkline_slot = match cockpit.strategy_equity.get(&id) {
      Some(PanelState::Ready(series)) if !series.points.is_empty() =>
          widgets::sparkline::view(series, cockpit.theme_mode),
      Some(PanelState::Loading) | None =>
          muted_body(STRATEGIES_SPARKLINE_LOADING).into(),
      Some(PanelState::Ready(_)) =>
          muted_body(strings::VIEWER_NO_EQUITY_DATA).into(),
      Some(PanelState::Error(msg)) =>
          muted_body(format!("Equity history unavailable: {msg}").as_str()).into(),
  };
  let slot = Container::new(sparkline_slot).width(Length::Fixed(160.0));
  ```
- **Sparkline widget** — new `crates/ui/src/widgets/sparkline.rs` (NOT
  `equity_curve` — sparkline at 120 × 36 px reads cleanest line-only
  per R13.2; no fill, no gridlines, no axes). Composes the same
  `widgets::canvas_chart` core but passes `fill_alpha = 0.0` to the
  shared `polyline_with_fill` (which short-circuits to "stroke only"
  when alpha is zero). Public API:
  ```rust
  pub fn view<'a>(
      series: &'a EquitySeries,
      mode: ThemeMode,
  ) -> Element<'a, Message>;
  ```
- **Wiring path** — Phase 3's `Message::SelectStrategy(id)` arm in the
  binary chains the new `audit::query::equity_curve_for_strategy(...)`
  fetch via `Task::perform` (see "Cockpit state diff" above). Result
  lands as `Message::StrategyEquityRefreshed(id, Result<EquitySeries, _>)`,
  pre-downsampled to 120 points (Q9).
- **Dimensions** (R13.2) — 120 × 36 px per Phase 3 R6.1 placeholder
  reservation.
- **Snapshot baselines** — `strategies_screen__sparkline_present.snap`
  (NEW) replaces `strategies_screen__sparkline_deferred.snap`
  (DELETED). The deletion lands in the same commit as the constant
  removal so the consistency-test fixture allow-list is updated
  atomically.

### Snapshot baseline strategy

**5 net-new + 1 deletion** (Q11). Single `cargo insta accept` pass at
end of phase per Phase 1 Q2 / Phase 2 V11 / Phase 3 V12 precedent.

**Net-new (4 viewer-bin + 1 cockpit-side):**

1. `viewer__kpi_strip__sample_report.snap` — six metric cards over the
   RSI sample (Total return `−57.80 %` `DOWN_500`, CAGR `—`, Sharpe
   `−55.4257`, Max DD `−57.81 %` `DOWN_500`, Win rate `—`, Trades
   `14118`).
2. `viewer__equity_curve__sample_report.snap` — equity-curve canvas
   over a fixture series; asserts five gridlines, `ACCENT` polyline,
   `UP_500 @ 0.18` fill.
3. `viewer__drawdown_band__sample_report.snap` — drawdown-band canvas
   over the same fixture; asserts five gridlines, `DOWN_500` polyline,
   `DOWN_500 @ 0.18` fill.
4. `viewer__full_view__sample_report.snap` — the assembled viewer
   surface (KPI strip + equity curve + drawdown band + body header).
5. `strategies_screen__sparkline_present.snap` — cockpit
   Strategies-detail with the real 120 × 36 px sparkline replacing the
   Phase 3 deferred placeholder.

**Deletion:** `strategies_screen__sparkline_deferred.snap` retires.

**Phase 1 / 2 / 3 baselines stay byte-identical** — viewer is a
separate bin; the cockpit-side change is local to the sparkline
placement (the `frame::muted_body(STRATEGIES_SPARKLINE_DEFERRED)`
slot retires; the sparkline widget lands in the **same** 160 px
width slot, so layout-neighbour baselines don't shift).

**Test fixtures.** `crates/ui/src/fixtures.rs` gains:
- `fake_backtest_metrics()` — deterministic `BacktestMetrics`
  matching the RSI sample.
- `fake_equity_series_for_viewer()` — 60-point synthetic series with
  a known peak / trough (`peak = 100_000`, `trough = 42_195`,
  `max_drawdown_pct ≈ 0.5781`) to mirror the RSI report's actual
  shape.
- `fake_equity_series_for_sparkline()` — 120-point series for the
  cockpit-side baseline.

### TD-1 re-evaluation

**Verification on disk:** `crates/ui/Cargo.toml:52` reads
`iced = { version = "=0.14.0", default-features = false, features =
["tiny-skia", "thread-pool", "advanced", "canvas"] }`.

iced 0.15+ has not landed; neither `button::Status::Focused` nor
`text_input::Style.shadow` is available. **Phase 4 ships no focus-ring
upgrade.** Phase 1's bounded approximation (hover-state ring on
the kill / kill-confirm / modal-close buttons; ACCENT border-shift on
the kill confirm input) holds. **The viewer is a zero-button surface
(R14.1 / R14.2)** — no focus-ring gap surfaces on it operationally.
The cockpit-side Strategies-detail change is a sparkline canvas
(non-focusable, no destructive action), so the Phase 4 surface adds
zero new focus-ring exposure. Operator-impact bound is unchanged: the
kill-switch destructive flow remains typed-confirm gated; the focus
halo is a secondary signal.

**Master-roadmap follow-up flagged.** The TD-1 row should be appended
with a 2026-05-06 line under "Promotion timing":

> "Phase 4 design pass (2026-05-06): iced version on disk verified
> still pinned `=0.14.0`; deferral restated. Viewer is zero-button
> surface and cockpit-side sparkline is non-focusable, so the Phase 4
> deliverable doesn't surface the gap operationally. Next
> re-evaluation at Phase 5 (HumanControl) analyst kickoff. Phase 5
> introduces the first new operator-write controls (pause-strategy,
> override-risk-veto) where the focus-ring ergonomic gap sharpens —
> architect re-evaluates the cost / benefit on the custom-widget
> escape-hatch path at that point."

The architect does not edit the master roadmap directly; the
orchestrator routes this as a follow-up to the analyst on Phase 4
ship.

### Cross-feature invariants

Phase 4 column from the master roadmap, re-stated with the design
note:

| Feature                          | Phase 4 invariant note                                                              | How preserved                                                                                                                                                                                                                                                                                                                                                                  |
|----------------------------------|-------------------------------------------------------------------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `operator-success-reports`       | Companion CSV read by viewer; never written. No latency-band regression surface.    | The viewer reads `<stem>__equity.csv` via the existing `EquitySample` row type (`crates/reports/src/csv_artifacts.rs`); zero schema change. Viewer has no status bar so no latency-badge regression surface. R16.1.                                                                                                                                                              |
| `live-cockpit-unified`           | `cockpit_live` launches unchanged; halted-banner triggers preserved.                | The Phase 4 Strategies-detail diff (sparkline replaces placeholder) is local to the screen body. Halted-banner shell-level wrap (Phase 2 R3.3) untouched. R16.2.                                                                                                                                                                                                                |
| `real-mtm-unrealized-pnl`        | P&L card unchanged; `color_for_delta` unchanged.                                    | `Cockpit::strategy_equity` is a sibling field to `Cockpit::pnl`, not a replacement. P&L card stays on Home; helper signature unchanged. R16.3.                                                                                                                                                                                                                                  |
| `per-symbol-position-accounts`   | Positions widget unchanged.                                                         | No position contract change. The audit-query addition is read-only over `journal_entries`; positions feed reads from the same audit ledger via `position()` / `recent_fills`, separate query path. R16.4.                                                                                                                                                                       |
| `tape-row-audit-modal`           | Modal trigger / frame unchanged.                                                    | No widget code change to `journal_transaction_modal`; viewer is a separate bin with no modal trigger surface. R16.5.                                                                                                                                                                                                                                                            |
| `journal-tx-metadata`            | Modal continues to render `description` + `strategy_id`.                            | Modal widget body unchanged; metadata reader unchanged. The new `equity_curve_for_strategy` reads `jt.strategy_id` from the same column the metadata-reader walks, but as a filter predicate, not a row-body rewrite. R16.6.                                                                                                                                                    |
| `v1.5b-multi-venue`              | R12 query does not filter by venue (equity is strategy-aggregate, not per-venue).   | The `equity_curve_for_strategy` SQL has no `venue` predicate — equity is per-strategy regardless of venue. Phase 3's `008` migration's `venue` column is read-only present, not required by the query. No venue-column ripple. R16.7.                                                                                                                                          |

**Acceptance:** the tester's per-feature invariant table = 7 / 7 PASS.

### Anchor regression

**Zero anchor risk re-affirmed.** The design pass found no path
where Phase 4 touches committed report bodies:

- **The KPI strip reads from the existing markdown summary table**
  (Q3a) — additive parser in `crates/reports/src/parse.rs`, zero
  write-path change. The 11 anchored reports' bodies are read
  byte-identically; the parser never edits.
- **The equity-curve points read from the existing `<stem>__equity.csv`
  companion file** (R11.2) — `EquitySample` row type unchanged; the
  reports-side write path (`csv_artifacts::write_equity_csv`) is not
  touched.
- **The audit query addition `equity_curve_for_strategy`** is
  read-only over already-committed `journal_entries` + `journal_transactions`
  rows (sibling of `pnl_by_strategy`'s read-only walk). No writer, no
  schema change, no description-format change.
- **The cockpit Strategies-detail sparkline** swaps a `muted_body`
  placeholder for a canvas widget on the same screen body — zero
  effect on any committed report.
- **The viewer is read-only on the spec tree** (architecture.md:3116);
  build-time test asserts no `std::fs::File::create` against
  `spec/**` from the `viewer` bin.

`verify-anchors` gate at the Phase 4 tester run must report 11 / 11
PASS with byte-identical bodies. The R16.3 grep gate from Phase 1
(`grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800"
spec/reports/`) remains zero — Phase 4 adds no new rendered prose to
any committed report.

### Implementation parallelism map

```
T1801 (foundation gate — core::EquitySeries + BacktestMetrics + Cockpit::strategy_equity)
  ├─ T1802 (audit::query::equity_curve_for_strategy + unit tests — parallel; audit crate, no ui dep)
  ├─ T1803 (viewer.rs skeleton + CLI args + iced Application boot — parallel after T1801)
  ├─ T1804 (widgets::canvas_chart core extraction — parallel after T1801; ui crate)
  └─ T1808 (reports::parse::BacktestMetrics::parse_from_report — parallel; reports crate, no ui dep)
        │
        ▼
   After T1804 lands:
        ├─ T1805 (widgets::kpi_strip)
        ├─ T1806 (widgets::equity_curve — composes T1804 core)
        ├─ T1807 (widgets::drawdown_band — composes T1804 core)
        └─ T1809 (widgets::sparkline — composes T1804 core)
                        │
                        ▼
              After T1803 + T1805 + T1806 + T1807 + T1808 land:
              T1810 (viewer composition — KPI strip + curve + band + body_render)
                        │
                        ▼
              After T1802 + T1809 land:
              T1811 (cockpit Strategies-detail sparkline replacement —
                     compound dispatch in both bins; STRATEGIES_SPARKLINE_DEFERRED retires)
                        │
                        ▼
              T1812 (snapshot refresh + ui-designer attestation sub-block — narrow point)
                        │
                        ▼
              T1813 (cross-feature invariants verify)
                        │
                        ▼
              T1814 (anchor regression + R16.3 grep)
                        │
                        ▼
              T1815 (rust-validate + viewer + both cockpit bins launch)
                        │
                        ▼
              T_FINAL_LUMEN_PHASE_4 (tester gate — VERDICT → presenter on PASS)
```

T1801 is the foundation gate. After T1801, four tasks fan out — the
audit query (T1802), the viewer skeleton (T1803), the canvas-chart
core extraction (T1804), and the reports parser (T1808) all run
independently. T1805–T1807 + T1809 share T1804's canvas-chart core.
T1810 (viewer composition) is gated on the four widget modules + the
parser. T1811 (cockpit sparkline) is gated on the audit query +
the sparkline widget. T1812 (snapshot accept) is the narrow point.

## Implementation

- 2026-05-06 (developer): T1801–T1815 ticked with honest evidence in
  [`spec/lumen-design-adoption/phase-4-backtest-panel/tasks.md`](tasks.md).
  Phase 4 ships:
  - `crates/core/src/equity_series.rs` (NEW) — cross-phase
    `EquitySeries` + `EquityPoint` + `BacktestMetrics` primitives
    with O(N) drawdown walk + `downsample(max_points)` (Q1 / Q12).
  - `crates/audit/src/query.rs::equity_curve_for_strategy` (NEW) —
    read-only sibling of `pnl_by_strategy`; returns the
    running-equity vector keyed on `strategy_id`.
    `LedgerError::EmptyWindow` variant lands on the shared error
    enum (Q7).
  - `crates/reports/src/parse.rs` (NEW) — markdown summary-table
    parser over committed bodies; missing-field tolerant per Q3a.
  - `crates/ui/src/widgets/canvas_chart.rs` (NEW) — shared canvas
    core (`inner_rect`, `draw_gridlines`, `polyline_with_fill`,
    `with_alpha` + 5-gridline `RANGE_PAD_FRACTION` constants); the
    Phase 2 `widgets::chart` consumes it without changing its
    public `view` signature (Q2).
  - `crates/ui/src/widgets/{kpi_strip,equity_curve,drawdown_band,sparkline}.rs`
    (NEW × 4) — viewer + cockpit canvases composing the shared
    core; sentiment-coloured KPI strip (R2.4 / Q3 graceful
    fallback); `ACCENT` polyline + `UP_500 @ 0.18` fill on the
    curve; `DOWN_500 @ 0.18` fill on the band (Q6); line-only
    sparkline at 120 × 36 px (R13.2).
  - `crates/ui/src/bin/viewer.rs` (NEW) — `viewer` binary (CLI-
    arg-driven; offline; zero buttons; R14 / Q4). Window title
    `"Backtest report — {scenario}"` per Master Constraint 1.
  - `crates/ui/src/viewer.rs` (NEW) — `ViewerModel`,
    `ViewerMessage`, `ReportLoadResult`, `ReportFrontMatter`
    (lib-side so the widgets can return
    `Element<'_, ViewerMessage>`).
  - `crates/ui/src/screens/strategies.rs` — Phase 3 sparkline
    deferral closed; `STRATEGIES_SPARKLINE_DEFERRED` retired and
    replaced by `STRATEGIES_SPARKLINE_LOADING` + dispatch on
    `cockpit.strategy_equity` (T1811 / Q6).
  - `crates/ui/src/bin/cockpit_live.rs` — extends Phase 3's
    `Message::SelectStrategy(id)` arm with a
    `Task::perform(equity_curve_for_strategy.map(downsample(120)))`
    chain (Q9 — cap at fetch).
  - `crates/ui/src/fixtures.rs` — `fake_backtest_metrics`,
    `fake_equity_series_for_viewer`,
    `fake_equity_series_for_sparkline` deterministic fixtures.
  - Snapshot baselines: 8 net-new under
    `crates/ui/{src/widgets/snapshots,tests/snapshots}/`,
    1 deletion (`strategies_screen__sparkline_deferred.snap`).
    Phase 1 / 2 / 3 baselines stay byte-identical.
  - Anchors verified: `bash scripts/verify_anchors.sh` →
    `ANCHORS PASS  (11 / 11)` post the audit-query addition and
    again at T1814.
  - Cross-feature invariant table 7 / 7 PASS (T1813).
  - All 3 bins (`viewer`, `cockpit`, `cockpit_live`) build clean.

## Verification — links

_tester fills this — links to
`spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-<timestamp>-lumen-phase-4-backtest-panel.md`._

## UI

_ui-designer fills this — links to refreshed snapshots and
the Phase 4 presentation under `spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md` (phase-4-backtest-panel section)._

## Changelog

- 2026-05-06 (architect): appended `## Design`. Q1–Q12 ratified
  (richer `EquitySeries` + `EquityPoint` shape with precomputed
  drawdown vector inside each point — not a parallel `Vec<Decimal>`;
  shared `widgets::canvas_chart` core + four wrappers
  `widgets::chart` / `equity_curve` / `drawdown_band` / `sparkline`;
  KPI source = parse the existing markdown summary table with
  graceful fallback to `VIEWER_METRICS_UNAVAILABLE` on parse
  failure; CLI-only viewer; 2000-point cap; solid `DOWN_500 @ 0.18`
  drawdown fill; `equity_curve_for_strategy(ledger, strategy_id,
  since, until: Option<Timestamp>) -> Result<EquitySeries,
  LedgerError>` siblings `pnl_by_strategy` over the same
  `income:realized_pnl` rows with `EmptyWindow` err on zero rows;
  sparkline placement above the chip row; cap+downsample at fetch,
  no live update; dark default cold-start; 5 net-new + 1 deletion
  snapshot ripple; `EquitySeries` + `BacktestMetrics` co-located in
  new module `crates/core/src/equity_series.rs`). 12 / 12 ratified;
  zero principled overrides. Cockpit state diff specified — single
  net-new field `pub strategy_equity: HashMap<StrategyId,
  PanelState<EquitySeries>>` and one new `Message` variant
  `StrategyEquityRefreshed(StrategyId, Result<EquitySeries, SmolStr>)`;
  Phase 3's `Message::SelectStrategy(id)` arm extends in the binary
  to chain a `Task::perform` of the new audit query, downsampled to
  120 points before landing. Viewer binary contract specified —
  `crates/ui/src/bin/viewer.rs` from scratch; own `ViewerModel` /
  `ViewerMessage` independent of `Cockpit`; `clap`-parsed positional
  `<report-path>`; iced functional builder pattern matching cockpit
  precedent; window title `"Backtest report — <scenario>"` (no
  `"Lumen"`); 1200 × 800 initial; `Subscription::none()`; markdown
  body rendered as monospace + heading-pre-pass (no `pulldown-cmark`
  dep). Six-card KPI strip, equity curve, drawdown band, and
  sparkline widget contracts each carry a public `view` signature +
  layout / colour / fill / gridline contract. Audit query addition
  signature locked: `equity_curve_for_strategy` reuses the
  `journal_entries je JOIN journal_transactions jt ON
  je.transaction_id = jt.id WHERE je.account_id =
  'income:realized_pnl' AND jt.strategy_id = ? AND je.ts >= ? AND
  je.ts < ?` row set, walks left-to-right with running cash-balance
  baseline, returns `EquitySeries` directly via `from_points`
  (which owns the drawdown / peak / trough / max-DD computation in
  one O(N) `Decimal` pass). Cockpit Strategies-detail sparkline
  closure: `STRATEGIES_SPARKLINE_DEFERRED` retires from
  `crates/ui/src/strings.rs:261`; new `STRATEGIES_SPARKLINE_LOADING`
  + `VIEWER_METRICS_UNAVAILABLE` + `VIEWER_NO_EQUITY_DATA` net-new
  constants additive (Constraint 2 unchanged); placeholder snapshot
  `strategies_screen__sparkline_deferred.snap` deleted in same
  commit as the `_present` baseline lands. TD-1 deferral re-stated
  — verified `crates/ui/Cargo.toml:52` still pins
  `iced = "=0.14.0"`; viewer is zero-button surface and the
  cockpit-side sparkline is non-focusable so the deferral is
  operationally invisible on the Phase 4 deliverable. Next
  re-evaluation at Phase 5 (HumanControl) analyst kickoff. Cross-
  feature invariants table re-stated (7 rows). Zero anchor risk
  re-affirmed (read-only over committed reports + read-only audit
  query addition + UI-only screens; no strategy / exec / risk /
  cost / backtest / reports write-path touched). Implementation
  parallelism map: T1801 foundation gate → fan-out across
  T1802–T1804 + T1808 → widget modules T1805–T1807 + T1809 share
  T1804 canvas-chart core → narrow at T1810 viewer composition +
  T1811 cockpit sparkline → T1812 snapshot accept narrow point →
  T1813–T1815 → T_FINAL. Task list at
  [`spec/lumen-design-adoption/phase-4-backtest-panel/tasks.md`](tasks.md)
  with 15 T18xx tasks + tester `T_FINAL_LUMEN_PHASE_4` gate.
  HANDOFF → developer ‖ ui-designer (developer takes T1801–T1815
  implementation; ui-designer takes the visual-diff attestation
  sub-block at T1812 / T_FINAL after the developer's snapshot
  refresh pass).
- 2026-05-06 (analyst, Phase 4 kickoff expansion): expanded
  the 2026-05-04 stub into the full analyst brief — 17
  R-items in 7 clusters (R1–R3 viewer scaffold + KPI strip;
  R4–R6 equity-curve widget; R7–R8 drawdown band; R9
  markdown body preservation; R10–R12 `EquitySeries`
  cross-phase primitive + offline + online sources; R13
  cockpit Strategies-detail sparkline consumer; R14–R17
  no-CTA + single-binary scope + invariants + anchors), 14
  V-items, 9 acceptance criteria, 12 architect Q-items
  (Q1 shape minimal-vs-richer, Q2 chart-widget
  share-vs-copy, Q3 KPI source body-vs-sidecar, Q4 viewer
  file-picker CLI-vs-UI, Q5 large-report cap, Q6 drawdown
  band solid-vs-gradient, Q7 audit-query signature, Q8
  sparkline placement, Q9 cockpit render budget, Q10
  viewer dark-default, Q11 snapshot baseline budget, Q12
  module placement). Master Q11–Q14 inherited as
  not-re-opened (Q11/Q12 irrelevant to viewer; Q13 applies
  to R12; Q14 past gate). Phase 3 Q6 deferral closed —
  Phase 4 ships both consumers on a shared
  `core::EquitySeries` shape. TD-1 verified — iced still
  pins `=0.14.0`; deferral restated, operationally
  invisible on the viewer's zero-button surface. Anchor
  risk **zero by construction** (read-only over committed
  reports + read-only audit query addition). Snapshot
  ripple: ~5 net-new + 1 deletion; single `cargo insta
  accept` pass per Phase 1 Q2 / Phase 2 V11 / Phase 3 V12
  precedent. `STRATEGIES_SPARKLINE_DEFERRED` retires from
  `ui::strings`. Brief status `queued` → `active`; owner
  unchanged. HANDOFF → architect.
- 2026-05-04 (analyst, master-roadmap revision): stub
  created at the 6-phase roadmap revision. Replaces the
  Phase 2 sketch in the pre-revision master roadmap.
  Renumbered Phase 2 → Phase 4. Full brief expansion
  deferred to Phase 4 kickoff per master Q3.
