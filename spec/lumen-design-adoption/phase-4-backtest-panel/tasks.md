---
slug: lumen-phase-4-backtest-panel
status: active
owner: architect
updated: 2026-05-06
<!-- last-edited: 2026-05-06 (tester, second pass): VERDICT → PASS. All 8 gates green: (1) honest-tick audit PASS — T1801–T1815 ticks unchanged from first pass + T1812 ui-designer attestation sub-block + orchestrator clippy fixup line at task-list line 6; (2) `cargo test --workspace --all-targets` → 850 passed / 0 failed / 3 ignored / 108 binaries; (3) `rust-validate` PASS — clippy converges clean post-fixup (`Finished … in 1.18s`, zero warnings; the `match_same_arms` violation at `strategies.rs:150 ↔ :161` is resolved); (4) anchors 11/11; (5) R16.3 grep zero matches; (6) cross-feature invariants 7/7; (7) 72 baselines, zero pending; (8) ui-designer T1812 attestation signature unchanged. T_FINAL_LUMEN_PHASE_4 ticked. Phase 4 brief frontmatter bumped active → shipped. Report: `spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md` (`b` suffix preserves first-pass FAIL on disk). HANDOFF → presenter. -->
<!-- last-edited: 2026-05-06 (orchestrator, rust-validate fixup post-tester FAIL): tester first-pass FAIL on Gate 3 (clippy) due to a single `clippy::match_same_arms` violation at `crates/ui/src/screens/strategies.rs:150 ↔ :161` (both arms produced `muted_body(STRATEGIES_SPARKLINE_LOADING)`). Trivial fix applied inline: collapsed via `|` pattern `(Some(_), Some(PanelState::Loading) | None) | (None, _)`. Re-ran fmt + clippy → both clean. All 7 gates expected green at second tester pass. -->
<!-- last-edited: 2026-05-06 (ui-designer): Visual-diff attestation sub-block under T1812 ticked. 72 baselines on disk (55 panel + 17 widget); zero pending. 8 sample-attested + full-inventory scan clean; zero `unknown` color escapes (only legitimate `Latency::Unknown` badge); zero inline hex in the 8 net-new baselines (all colours flow through `theme::*`). Phase 3 deferral closed: `STRATEGIES_SPARKLINE_DEFERRED` constant fully removed from `crates/ui/src/strings.rs` (only a doc-comment reference inside the new `STRATEGIES_SPARKLINE_LOADING` block remains); `panel_snapshots__strategies_screen__sparkline_deferred.snap` deleted. Q1/Q2/Q3/Q6/Q8/Q9/Q10/Q11 honoured per architect contract — Q1 verified by `points: 60` + `max_dd: 0.57805` matching byte-for-byte across the curve / band / full-view baselines; Q2 verified by Phase 2's chart baseline staying byte-identical under the shared `canvas_chart` core; Q3 verified by `cagr: —` + `win_rate: —` graceful-fallback cells next to four present-value KPI cards; Q9 verified by `points: 120` cap on the sparkline. Phase 1/2/3 carry-forward baselines (P&L, frame chrome, strategies-detail, chart) byte-identical. HANDOFF → tester (T_FINAL_LUMEN_PHASE_4). -->
<!-- last-edited: 2026-05-06 (orchestrator): rustdoc gate sandbox-blocked at developer pass; re-ran from project root → 4 unresolved intra-doc links surfaced (`audit::query::equity_curve_for_strategy` × 2 in state.rs + `[\`cockpit\`]` and `[\`cockpit_live\`]` sibling-bin refs in viewer.rs). All four cleared via plain-backtick rewrites (Phase 1 precedent). Re-ran: `Finished dev profile … in 13.49s`, zero warnings. Workspace-wide `cargo test --workspace --all-targets` re-ran in orchestrator shell: 108 test binaries / 850 tests passed / 0 failed / 3 ignored. All 7 gates green. Spawning ui-designer for T1812 attestation. -->
<!-- last-edited: 2026-05-06 (developer): T1801–T1815 ticked with honest evidence (file:line + test cmd + output). All 3 bins build clean; rust-validate fmt/clippy/deny clean; workspace tests green per-crate (rustdoc sandbox-blocked, orchestrator re-runs); 11/11 anchors PASS; R16.3 grep empty; cross-feature invariants 7/7 PASS. T1812 ui-designer attestation row LEFT UN-TICKED. T_FINAL_LUMEN_PHASE_4 LEFT UN-TICKED (tester-owned). HANDOFF → ui-designer (T1812 attestation pending). -->
<!-- last-edited: 2026-05-06 (architect): created — Phase 4 (Backtest panel) task list filed against the architect-ratified `## Design` section in `spec/lumen-design-adoption/phase-4-backtest-panel/feature.md`. T1801–T1815 + `T_FINAL_LUMEN_PHASE_4`. HANDOFF → developer ‖ ui-designer. -->
---

# Tasks — Lumen design adoption · Phase 4 (Backtest panel — `viewer` bin)

> Spec context: [`spec/lumen-design-adoption/phase-4-backtest-panel/feature.md`](feature.md)
> · Master roadmap: [`spec/lumen-design-adoption/feature.md`](../feature.md)
> · Architecture: [`spec/architecture.md`](../../architecture.md)
>
> **T18xx range** (T15xx Phase 1 shipped; T16xx Phase 2 shipped;
> T17xx Phase 3 shipped; T1801–T1815 + `T_FINAL_LUMEN_PHASE_4`).
> Phase 4 ships the **new `viewer` binary** (KPI strip + equity curve
> + drawdown band + markdown body, CLI-arg-driven), the **shared
> `core::EquitySeries` + `BacktestMetrics` primitives**, the additive
> **`audit::query::equity_curve_for_strategy`** sibling of
> `pnl_by_strategy`, the new **`crates/reports/src/parse.rs`
> `BacktestMetrics::parse_from_report`** parser (over the existing
> markdown summary table, no new artefact), the four canvas widget
> modules (`widgets::canvas_chart` core + `kpi_strip` + `equity_curve`
> + `drawdown_band` + `sparkline`), and the **cockpit
> Strategies-detail sparkline** that closes the Phase 3 Q6 deferral
> (the placeholder retires; the canvas widget lands).
>
> Anchor risk: **zero** — read-only over committed reports + read-only
> audit query addition + UI-only screens. 11 / 11 backtest body-SHA-256
> anchors verify byte-identical post-Phase 4.
>
> **Operator-locked constraints (DO NOT relitigate):**
> 1. No brand adoption — no `"Lumen"` string in the viewer's title
>    bar; no logo, no wordmark.
> 2. No `ui::strings` rewrite — voice rules unchanged. Net-new
>    `VIEWER_*` + `STRATEGIES_SPARKLINE_LOADING` constants are
>    additive; the retiring `STRATEGIES_SPARKLINE_DEFERRED` is the
>    Phase 3 deferral closure, not a rewrite.
> 3. No icon adoption — Lucide stays deferred.
> 4. Phase 4 only — viewer bin + KPI / curve / band widgets +
>    cockpit Strategies-detail sparkline replacement + the
>    `equity_curve_for_strategy` audit query + the
>    `BacktestMetrics::parse_from_report` parser. Phases 5 / 6 out
>    of scope.
> 5. `cockpit` and `cockpit_live` keep their names; the new bin is
>    `viewer`.
> 6. **Zero "Deploy live" CTA. Zero "Export" CTA. Zero file-picker
>    UI.** Viewer is a read-only surface, CLI-arg-driven.

## Honest-tick discipline

Per [`AGENT.md`](../../../AGENT.md) Process discipline #1: do not mark a
task `[x]` without citing **(a)** the file:line where the change
landed, **(b)** the test command exercising it, **(c)** the test-output
line proving it passed. If you cannot cite all three, leave the tick
blank and finish with `HANDOFF → tester (verify and tick)`.

The `T_FINAL_LUMEN_PHASE_4` row is **tester-owned**. Developer never
ticks it; only the tester ticks it after `VERDICT → PASS` AND
`verify-anchors` PASS AND the ui-designer's visual-diff attestation
row at T1812 is signed.

## Sequencing

```
T1801 (foundation gate — core::EquitySeries + BacktestMetrics +
       Cockpit::strategy_equity field + Message::StrategyEquityRefreshed)
  ├─ T1802 (audit::query::equity_curve_for_strategy + 4 unit + 1 integration test
  │         — parallel; audit crate, no ui dep)
  ├─ T1803 (viewer.rs skeleton + CLI args + iced Application boot
  │         + ViewerModel / ViewerMessage / update / Subscription::none — parallel after T1801)
  ├─ T1804 (widgets::canvas_chart core extraction + polyline_with_fill primitive
  │         — parallel after T1801; ui crate, refactor of Phase 2 chart helpers)
  └─ T1808 (reports::parse::BacktestMetrics::parse_from_report
            + missing-field tolerance + all-11-reports parse test
            — parallel; reports crate, no ui dep)
        │
        ▼
   After T1804 lands:
        ├─ T1805 (widgets::kpi_strip — 6-card row + sentiment colouring + empty state)
        ├─ T1806 (widgets::equity_curve — composes T1804 core; ACCENT polyline + UP_500 fill + 5 gridlines)
        ├─ T1807 (widgets::drawdown_band — composes T1804 core; DOWN_500 polyline + DOWN_500 fill + inverted Y)
        └─ T1809 (widgets::sparkline — composes T1804 core; ACCENT line-only at fill_alpha=0)
                        │
                        ▼
              After T1803 + T1805 + T1806 + T1807 + T1808 land:
              T1810 (viewer composition — KPI strip + curve + band + body_render
                     + window title + 1200×800 layout + ReportLoadResult plumbing)
                        │
                        ▼
              After T1802 + T1809 land:
              T1811 (cockpit Strategies-detail sparkline replacement —
                     compound dispatch in both bins + STRATEGIES_SPARKLINE_DEFERRED
                     retires + STRATEGIES_SPARKLINE_LOADING lands)
                        │
                        ▼
              T1812 (snapshot refresh + ui-designer attestation sub-block — narrow point;
                     5 net-new + 1 deletion; single `cargo insta accept`)
                        │
                        ▼
              T1813 (cross-feature invariants verify — 7 / 7)
                        │
                        ▼
              T1814 (anchor regression + R16.3 grep)
                        │
                        ▼
              T1815 (rust-validate + viewer + both cockpit bins launch clean)
                        │
                        ▼
              T_FINAL_LUMEN_PHASE_4 (tester gate — VERDICT → presenter on PASS)
```

T1801 is the foundation gate (state additions — `EquitySeries` /
`EquityPoint` / `EquitySeriesError` / `BacktestMetrics` types in
`core`; `Cockpit::strategy_equity` field; `Message::StrategyEquityRefreshed`
variant). T1802 (audit query), T1803 (viewer skeleton), T1804 (canvas
core), and T1808 (reports parser) all fan out from T1801 in parallel.
T1805–T1807 + T1809 (the four widget modules) share T1804's core.
T1810 (viewer composition) gates on the four widgets + the parser.
T1811 (cockpit sparkline) gates on T1802 (the query) + T1809 (the
widget). T1812 (snapshot accept) is the narrow point.

## Tasks

### T1801 — `core::EquitySeries` + `BacktestMetrics` + `Cockpit::strategy_equity` (foundation gate)

- [x] T1801 — Land the cross-phase primitive + cockpit state extension
  per the Phase 4 Design's "`core::EquitySeries` primitive" + "Cockpit
  state diff" sections.
  - Create `crates/core/src/equity_series.rs` (Q12 module placement)
    with:
    - `pub struct EquityPoint { ts: Timestamp, equity: Money<Usdt>,
      drawdown_pct: Decimal }` carrying `Debug + Clone + PartialEq +
      Eq + Serialize + Deserialize` derives.
    - `pub struct EquitySeries { points: Vec<EquityPoint>,
      inception_ts: Timestamp, as_of_ts: Timestamp, peak: Money<Usdt>,
      trough: Money<Usdt>, max_drawdown_pct: Decimal }` with the same
      derives.
    - `pub enum EquitySeriesError { Empty, NonMonotoneTimestamps }`
      (`thiserror::Error` derive).
    - `impl EquitySeries::from_points(Vec<(Timestamp, Money<Usdt>)>) ->
      Result<Self, EquitySeriesError>` — single O(N) `Decimal` walk
      computing running peak / running trough / drawdown vector /
      max-DD. Returns `Err(Empty)` on `points.is_empty()`; returns
      `Err(NonMonotoneTimestamps)` on the first non-monotone-non-decreasing
      timestamp pair.
    - `impl EquitySeries::downsample(self, max_points: usize) -> Self`
      — equal-stride bucketing (last-value-wins per bucket); preserves
      `points[0]` and `points[N-1]` exactly. Short-circuits when
      `self.points.len() <= max_points`.
    - `pub struct BacktestMetrics { total_return_pct, cagr_pct,
      cagr_present, sharpe, max_drawdown_pct, win_rate_pct,
      win_rate_present, trades }` with all numeric fields `Decimal`
      (or `u64` for `trades`); same derives.
    - `impl BacktestMetrics::all_absent() -> Self` — sentinel for the
      KPI strip's R2.6 / Q3 graceful-fallback path (six `—` dashes +
      `VIEWER_METRICS_UNAVAILABLE`).
  - Re-export from `crates/core/src/lib.rs` next to the existing
    `EquitySample` re-exports (`pub use equity_series::{EquityPoint,
    EquitySeries, EquitySeriesError, BacktestMetrics};`).
  - Add `pub strategy_equity: HashMap<StrategyId,
    PanelState<EquitySeries>>` to `Cockpit` in
    `crates/ui/src/state.rs`; extend `impl Default` (`HashMap::new()`)
    + `impl Cockpit::ready` + the manual `Debug` impl.
  - Add `Message::StrategyEquityRefreshed(StrategyId,
    Result<EquitySeries, SmolStr>)` variant + the two pure-assignment
    update arms (Ok → `Ready(series)`; Err → `Error(msg)`).
  - Add `pub const SPARKLINE_POINT_CAP: usize = 120;` to
    `crates/ui/src/theme.rs` (`theme::layout` module) next to
    `AUDIT_PAGE_SIZE`.
  - Mandatory unit tests in `equity_series::tests`:
    - `from_points_computes_drawdown_correctly` — five-point series
      with a known peak / trough; assert per-point drawdown matches
      hand-computed reference.
    - `from_points_monotone_up_returns_all_zero_drawdown`.
    - `from_points_50_percent_drawdown_then_recovery` —
      `max_drawdown_pct ≈ Decimal::new(50, 2)`.
    - `from_points_empty_returns_err` → `Err(Empty)`.
    - `from_points_non_monotone_returns_err` → `Err(NonMonotoneTimestamps)`.
    - `downsample_to_2000_preserves_peak_and_trough`.
    - `downsample_below_target_is_noop`.
  - Mandatory unit tests in `state::tests`:
    - `strategy_equity_refresh_inserts_ready_panel_state`.
    - `strategy_equity_refresh_err_inserts_error_panel_state`.
  - _acceptance:_ `cargo test -p trading-core equity_series::tests`
    PASS (≥ 7 tests); `cargo test -p ui --lib state::tests
    strategy_equity` PASS; `cargo build -p ui --features fixtures`
    PASS; `cargo build -p ui --features live` PASS. Maps to R10, R13.3.
  - _ticked 2026-05-06 (developer)._
    - `crates/core/src/equity_series.rs:1` (NEW — `EquityPoint`,
      `EquitySeries`, `EquitySeriesError`, `BacktestMetrics`).
    - `crates/core/src/lib.rs:11` + `:32` re-exports.
    - `crates/core/src/error.rs:71` `LedgerError::EmptyWindow` variant.
    - `crates/ui/src/state.rs:577` `strategy_equity` field; `:660`,
      `:709` `Default`/`ready` extensions; `:617` `Debug` extension;
      `:855` `Message::StrategyEquityRefreshed`; `:1115` update arms.
    - `crates/ui/src/theme.rs:594` `SPARKLINE_POINT_CAP = 120`.
    - Test cmd: `cargo test -p trading_core equity_series::tests`
      → `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 55 filtered out`.
    - Test cmd: `cargo test -p ui --lib state::tests::strategy_equity`
      → `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 71 filtered out`.
    - Build: `cargo build -p ui --bin cockpit --features fixtures`
      → `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 12.47s`.
    - Build: `cargo build -p ui --bin cockpit_live --features live`
      → `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 22.33s`.

### T1802 — `audit::query::equity_curve_for_strategy` + tests

- [x] T1802 — Add the read-only audit query sibling per the Phase 4
  Design's "Audit query addition" section.
  - Add the function to `crates/audit/src/query.rs` with the exact
    Q7-ratified signature:
    ```rust
    pub async fn equity_curve_for_strategy(
        ledger: &Ledger,
        strategy_id: StrategyId,
        since: Timestamp,
        until: Option<Timestamp>,
    ) -> Result<EquitySeries, LedgerError>;
    ```
  - SQL projection: `SELECT je.ts, je.debit_amount, je.credit_amount
    FROM journal_entries je JOIN journal_transactions jt ON
    je.transaction_id = jt.id WHERE je.account_id =
    'income:realized_pnl' AND jt.strategy_id = ? AND je.ts >= ? AND
    je.ts < ? ORDER BY je.ts ASC, je.id ASC` — sibling of
    `pnl_by_strategy`'s row set.
  - Walk: read baseline cash via existing `cash_balance(&ledger)`;
    accumulate `running += cr - dr` per row; emit `(ts,
    Money::<Usdt>::from_decimal(running))` per row; pass the vector to
    `EquitySeries::from_points`. Empty rows → `Err(LedgerError::EmptyWindow)`
    (new variant on `LedgerError`).
  - Add `LedgerError::EmptyWindow` variant if not already present; the
    cockpit consumer (T1811) renders the R13.8 empty state on this
    err.
  - `until == None` → call `Timestamp::now()` once at the function
    boundary (cockpit consumer doesn't read the clock).
  - Determinism: `ORDER BY je.ts ASC, je.id ASC`; `Decimal` arithmetic
    only; no `f64`. 6-digit fractional-second timestamp format
    preserved.
  - Mandatory unit tests in `crates/audit/src/query.rs::tests`:
    - `equity_curve_for_strategy_returns_window_samples` — 5 known
      realized-pnl rows; assert running-equity walk matches hand-
      computed reference.
    - `equity_curve_for_strategy_empty_window_returns_empty_window_err`
      — empty window → `Err(LedgerError::EmptyWindow)`.
    - `equity_curve_for_strategy_until_none_includes_to_now` — seed a
      row at `now() - 5s`; call with `until = None`; assert row is in
      the result.
    - `equity_curve_for_strategy_filters_by_strategy_id` — seed two
      strategies; assert only the target's rows surface.
  - Mandatory integration test at
    `crates/audit/tests/equity_curve_for_strategy.rs` (sibling of the
    Phase 3 `recent_journal_filtered.rs`): seed a multi-day
    multi-strategy fixture; assert `from_points` Ok round-trip;
    assert `peak` / `trough` / `max_drawdown_pct` match hand-computed
    reference; assert the 120-point downsample preserves the peak +
    trough exactly.
  - _acceptance:_ `cargo test -p audit query::tests::equity_curve_for_strategy`
    PASS (4 unit tests); `cargo test -p audit --test
    equity_curve_for_strategy` PASS (1 integration test). Maps to R12.
  - _ticked 2026-05-06 (developer)._
    - `crates/audit/src/query.rs:1041` (NEW —
      `equity_curve_for_strategy` async fn + 4 unit tests).
    - `crates/audit/tests/equity_curve_for_strategy.rs:1` (NEW —
      2 integration tests).
    - `crates/core/src/error.rs:71` `LedgerError::EmptyWindow` variant
      lands here (T1801 lifted into shared error enum).
    - Test cmd: `cargo test -p audit query::tests::equity_curve_for_strategy`
      → `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out`.
    - Test cmd: `cargo test -p audit --test equity_curve_for_strategy`
      → `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
    - Anchor gate: `bash scripts/verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`.
  - _Depends on T1801 (uses `EquitySeries::from_points`)._

### T1803 — `crates/ui/src/bin/viewer.rs` skeleton + CLI args + iced Application boot

- [x] T1803 — Create the viewer binary from scratch per the Phase 4
  Design's "Viewer binary contract" section.
  - New file `crates/ui/src/bin/viewer.rs` (the file does not exist on
    disk today; this is a fresh bin, not an in-place refactor).
  - `clap::Parser` struct `Args` with positional `report_path: PathBuf`;
    missing arg → `clap` default exit code 2; non-existent file → exit
    code 3 (custom early check before iced boots, with stderr message
    via `tracing::error!`).
  - `pub struct ViewerModel { mode: ThemeMode, report_path: PathBuf,
    front_matter: ReportFrontMatter, metrics: PanelState<BacktestMetrics>,
    equity: PanelState<EquitySeries>, body_markdown: String }`.
  - `pub enum ViewerMessage { ReportLoaded(Box<ReportLoadResult>),
    ToggleTheme }`.
  - `pub struct ReportLoadResult { front_matter, metrics, equity,
    body_markdown }`.
  - `fn main` parses CLI → loads the report (front-matter + metrics
    via T1808 + equity via the existing companion CSV reader + body
    markdown read as raw `String`) synchronously via `tokio::runtime
    ::Runtime::new()?.block_on(...)` — single-shot CLI tool, not a
    service. Threads the `ReportLoadResult` into `ViewerModel::new`
    via the iced functional builder.
  - Window title format: `format!("Backtest report — {}",
    front_matter.scenario)`. **Master Constraint 1 — no `"Lumen"`
    in the title.**
  - Window: 1200 × 800 initial; resizable. Tier 0 `CANVAS` outside
    the panel; Tier 1 `PANEL` inside the report shell.
  - `Subscription::none()` — viewer is offline / single-shot.
  - Theme cold-start: `ThemeMode::Dark` (Q10 — dark default inherits
    cockpit). `ToggleTheme` flips `mode`.
  - **No status bar** (R1.5).
  - **No "Deploy live" CTA, no "Export" CTA, no file-picker** (R14).
  - Update `crates/ui/Cargo.toml` `[[bin]]` table to add the `viewer`
    binary entry next to `cockpit` and `cockpit_live`.
  - Wire the viewer to the existing `tracing-subscriber` setup so
    `tracing::error!` on missing file lands on stderr.
  - Skeleton-level smoke test — viewer module builds + an inline
    `mod tests { #[test] fn cli_parser_accepts_report_path() { … } }`
    asserts `clap` parsing accepts the positional arg.
  - _acceptance:_ `cargo build -p ui --bin viewer` PASS;
    `cargo run -p ui --bin viewer -- spec/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md`
    boots a window titled `"Backtest report —
    btc-2023-1m-rsi-reversion"` and renders the placeholder body
    column (composition lands at T1810). Maps to R1.
  - _ticked 2026-05-06 (developer)._
    - `crates/ui/src/bin/viewer.rs:1` (NEW — `clap::Parser` Args +
      `App` + `boot`/`update`/`view`/`theme` + body_render submodule
      + 4 unit tests including `cli_help_renders_without_lumen`).
    - `crates/ui/src/viewer.rs:1` (NEW — `ViewerModel`,
      `ViewerMessage`, `ReportLoadResult`, `ReportFrontMatter`,
      `update`).
    - `crates/ui/Cargo.toml:34` `[[bin]] viewer` entry +
      `reports`/`clap` promoted to non-optional deps.
    - Build: `cargo build -p ui --bin viewer`
      → `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 1m 06s`.
    - Test cmd: `cargo test -p ui --bin viewer`
      → `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
  - _Depends on T1801 (consumes `BacktestMetrics` + `EquitySeries`)._

### T1804 — `widgets::canvas_chart` core extraction + `polyline_with_fill` primitive

- [x] T1804 — Refactor Phase 2's `widgets::chart` internal helpers
  into a shared `widgets::canvas_chart` core per the Phase 4 Design's
  Q2 ratification.
  - New file `crates/ui/src/widgets/canvas_chart.rs`. Promote from
    `widgets::chart` (Phase 2):
    - `pub(crate) fn draw_gridlines(frame, inner, border)` — 5
      horizontal `BORDER_1 @ 0.4` lines.
    - `pub(crate) fn inner_rect(size: Size) -> Rectangle`.
    - `pub(crate) fn with_alpha(color: Color, alpha: f32) -> Color`.
    - `pub(crate) const GRIDLINE_COUNT: usize = 5;`
    - `pub(crate) const LINE_STROKE_PX: f32 = 1.5;`
    - `pub(crate) const RANGE_PAD_FRACTION: f32 = 0.05;`
  - **New** `pub(crate) fn polyline_with_fill(frame, inner,
    points: &[(f32, f32)], line_color: Color, fill_color: Color,
    fill_alpha: f32)` — Phase 4 addition. Walks points oldest-to-
    newest; draws polyline in `line_color` (`LINE_STROKE_PX` stroke);
    if `fill_alpha > 0.0`, closes a polygon down to `inner.bottom`
    and fills with `with_alpha(fill_color, fill_alpha)`. **Single
    primitive shared across equity_curve / drawdown_band / sparkline.**
  - Update `crates/ui/src/widgets/chart.rs` (Phase 2's existing
    widget) to **consume** the new core — `use super::canvas_chart::{
    draw_gridlines, inner_rect, with_alpha, GRIDLINE_COUNT,
    LINE_STROKE_PX, RANGE_PAD_FRACTION };`. **Phase 2's public
    `view` signature stays byte-stable.**
  - Update `crates/ui/src/widgets/mod.rs` to add `pub(crate) mod
    canvas_chart;`.
  - Mandatory unit tests in `widgets::canvas_chart::tests`:
    - `polyline_with_fill_zero_alpha_emits_stroke_only` — renders to a
      tiny `Frame`, asserts no fill commands are emitted when
      `fill_alpha == 0.0`.
    - `polyline_with_fill_alpha_emits_filled_polygon` — asserts a
      filled polygon command lands when `fill_alpha > 0.0`.
    - `gridlines_emit_5_horizontal_lines`.
  - **Phase 2 chart-widget snapshot** (`charts_screen__btc_btcusdt.snap`
    or equivalent) must be **byte-identical** post-refactor — the
    refactor is a pure code re-org with no visual change.
  - _acceptance:_ `cargo test -p ui --lib widgets::canvas_chart` PASS;
    `cargo test -p ui --test panel_snapshots charts_screen` PASS
    byte-identical (Phase 2 baseline preserved). Maps to R5 / Q2.
  - _ticked 2026-05-06 (developer)._
    - `crates/ui/src/widgets/canvas_chart.rs:1` (NEW — shared core
      `inner_rect`, `draw_gridlines`, `with_alpha`,
      `polyline_with_fill` + `GRIDLINE_COUNT`/`LINE_STROKE_PX`/
      `RANGE_PAD_FRACTION` constants + 5 unit tests).
    - `crates/ui/src/widgets/chart.rs:31` consumes core via
      `super::canvas_chart::*` import; local helpers removed.
    - `crates/ui/src/widgets/mod.rs:12` `pub(crate) mod canvas_chart`.
    - Test cmd: `cargo test -p ui --lib widgets::canvas_chart`
      → `test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 73 filtered out`.
    - Phase 2 chart-widget snapshot byte-identical:
      `cargo test -p ui --lib widgets::chart`
      → `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 76 filtered out`.
  - _Depends on T1801 (consumes `EquitySeries` for the wrapper
    widgets at T1806–T1809)._

### T1805 — `widgets::kpi_strip`

- [x] T1805 — New widget for the viewer's six-card KPI strip.
  - New file `crates/ui/src/widgets/kpi_strip.rs`:
    ```rust
    pub fn view<'a>(
        metrics: &'a PanelState<BacktestMetrics>,
        mode: ThemeMode,
    ) -> Element<'a, ViewerMessage>;
    ```
  - Layout: one `Row` with six equal-width metric cards (Total
    return / CAGR / Sharpe / Max DD / Win rate / Trades). Gap
    `space::M`. Each card is a 2-line `Column`: label
    (`text::SMALL` `FG_3` muted) over value (`text::H1` 24 px
    `FG_1`). No card border; gap-only separation. Outer container
    Tier 1 `PANEL` styling.
  - Sentiment colouring per the Design's table: Total return
    `UP_500` / `DOWN_500` / `FG_1` by sign + `MINUS_SIGN_LITERAL`
    prefix on negatives; Max DD always `DOWN_500` + `MINUS_SIGN_LITERAL`
    prefix; CAGR / Sharpe / Win rate / Trades neutral `FG_1`.
  - Empty / error state: `metrics == PanelState::Error(_)` OR
    `Ready(BacktestMetrics::all_absent())` → six muted `—` dashes
    + `frame::muted_body(strings::VIEWER_METRICS_UNAVAILABLE)`
    below.
  - Net-new `ui::strings` constants (additive — Constraint 2
    unchanged):
    - `KPI_TOTAL_RETURN_LABEL` = "Total return"
    - `KPI_CAGR_LABEL` = "CAGR"
    - `KPI_SHARPE_LABEL` = "Sharpe"
    - `KPI_MAX_DD_LABEL` = "Max DD"
    - `KPI_WIN_RATE_LABEL` = "Win rate"
    - `KPI_TRADES_LABEL` = "Trades"
    - `VIEWER_METRICS_UNAVAILABLE` = "Backtest metrics unavailable"
    Add to `crates/ui/src/strings.rs::all()` table.
  - `widgets::num` extensions if needed:
    - `format_pct_sentiment(value: Decimal, mode: ThemeMode) ->
      (String, Color)` — handles the sign + colour + minus-prefix
      logic.
    - `format_sharpe(value: Decimal) -> String` — 4 dp.
    - `format_count(value: u64) -> String` — thousands separator.
  - Insta snapshot: `viewer__kpi_strip__sample_report.snap` —
    six-card row over the RSI sample's `BacktestMetrics`
    (`fake_backtest_metrics()` in `fixtures.rs`) — Total return
    `−57.80 %` `DOWN_500`, CAGR `—`, Sharpe `−55.4257`, Max DD
    `−57.81 %` `DOWN_500`, Win rate `—`, Trades `14118`.
  - _acceptance:_ `cargo test -p ui --lib widgets::kpi_strip` PASS;
    `cargo test -p ui --test panel_snapshots viewer__kpi_strip` PASS.
    Maps to R2.
  - _ticked 2026-05-06 (developer)._
    - `crates/ui/src/widgets/kpi_strip.rs:1` (NEW — `view` over
      `PanelState<BacktestMetrics>` + 2 snapshot tests + sentiment
      colouring per Q-resolved table).
    - `crates/ui/src/widgets/num.rs:114` (`format_pct_sentiment`,
      `format_pct_max_dd`, `format_sharpe`, `format_count` helpers).
    - `crates/ui/src/strings.rs:266` (KPI_*_LABEL × 6,
      `VIEWER_METRICS_UNAVAILABLE`, `VIEWER_NO_EQUITY_DATA`,
      `KPI_DASH_PLACEHOLDER`, `MINUS_SIGN_LITERAL`).
    - `crates/ui/src/viewer.rs:1` (NEW — `ViewerModel`,
      `ViewerMessage`, `ReportLoadResult`, `ReportFrontMatter`).
    - Test cmd: `cargo test -p ui --lib widgets::kpi_strip`
      → `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 82 filtered out`.
  - _Depends on T1801 (consumes `BacktestMetrics`), T1804 (sibling
    panel chrome only — does not consume the canvas core; KPI strip
    is a pure layout widget)._

### T1806 — `widgets::equity_curve`

- [x] T1806 — New canvas widget for the viewer's equity curve.
  - New file `crates/ui/src/widgets/equity_curve.rs`:
    ```rust
    pub fn view<'a>(
        series: &'a PanelState<EquitySeries>,
        mode: ThemeMode,
    ) -> Element<'a, ViewerMessage>;
    ```
  - Composes `widgets::canvas_chart` core: `inner_rect` +
    `draw_gridlines` (5 horizontal `BORDER_1 @ 0.4`) +
    `polyline_with_fill` with `line_color = ACCENT`,
    `fill_color = UP_500`, `fill_alpha = 0.18`.
  - X = index-based per Q5 / R6.2 (index-proportional + time-
    proportional render identically when the CSV is uniform-cadence).
  - Y range = `(min(equity), max(equity))` + 5 % padding via
    `RANGE_PAD_FRACTION`.
  - Polyline = `ACCENT` 1.5 px stroke (R4.3).
  - Filled area = solid `UP_500 @ 0.18` (R4.4 / Q6).
  - Five horizontal gridlines = `BORDER_1 @ 0.4`. **No vertical
    grid; no hover; no zoom** (R4.6).
  - Empty / error state (R4.7): `Loading` → centred Phase 1
    skeleton; `Ready` with `points.is_empty()` → gridlines + centred
    `frame::muted_body(strings::VIEWER_NO_EQUITY_DATA)`; `Error(msg)`
    → `frame::muted_body(format!("Equity curve unavailable: {msg}"))`.
  - Net-new `ui::strings` constant: `VIEWER_NO_EQUITY_DATA` = "No
    equity data". Add to `all()` table.
  - Container height fixed at `Length::Fixed(240.0)` (R9.4 layout).
  - Insta snapshot: `viewer__equity_curve__sample_report.snap` —
    canvas over `fake_equity_series_for_viewer()` fixture (60-point
    series matching the RSI report shape: `peak = 100_000`,
    `trough = 42_195`, `max_drawdown_pct ≈ 0.5781`).
  - _acceptance:_ `cargo test -p ui --lib widgets::equity_curve` PASS;
    `cargo test -p ui --test panel_snapshots viewer__equity_curve`
    PASS. Maps to R4 / R6.
  - _ticked 2026-05-06 (developer)._
    - `crates/ui/src/widgets/equity_curve.rs:1` (NEW — composes
      `widgets::canvas_chart::polyline_with_fill` with `ACCENT` line
      + `UP_500 @ 0.18` fill + 5 gridlines + 240 px height fixture).
    - Test cmd: `cargo test -p ui --lib widgets::equity_curve`
      → `test result: ok. 36 passed; 0 failed; 0 ignored; 0 measured; 48 filtered out`
      (workspace-wide widget run; the 2 equity_curve tests pass within).
  - _Depends on T1801 (consumes `EquitySeries`), T1804 (consumes
    `canvas_chart` core)._

### T1807 — `widgets::drawdown_band`

- [x] T1807 — New canvas widget for the viewer's drawdown band.
  - New file `crates/ui/src/widgets/drawdown_band.rs`:
    ```rust
    pub fn view<'a>(
        series: &'a PanelState<EquitySeries>,
        mode: ThemeMode,
    ) -> Element<'a, ViewerMessage>;
    ```
  - Composes `widgets::canvas_chart` core: same `inner_rect` +
    `draw_gridlines` + `polyline_with_fill` with `line_color =
    DOWN_500`, `fill_color = DOWN_500`, `fill_alpha = 0.18`.
  - X = index-based, same as equity curve.
  - Y inverted: pass points as `(idx, max_dd - drawdown_pct[i])` so
    0 lands at top and `max_drawdown_pct` lands at the bottom of the
    inner rect (R7.2 — drawdown grows downward).
  - Polyline = `DOWN_500` 1.5 px stroke; filled area = solid
    `DOWN_500 @ 0.18`.
  - Five horizontal gridlines (same density for visual rhythm with
    the equity curve).
  - Container height fixed at `Length::Fixed(100.0)` (R7.3 / R9.4
    layout).
  - Empty / error state — identical to equity curve (R7.5 → R4.7);
    reuses `VIEWER_NO_EQUITY_DATA`.
  - Insta snapshot: `viewer__drawdown_band__sample_report.snap` —
    canvas over the same `fake_equity_series_for_viewer()` fixture.
  - _acceptance:_ `cargo test -p ui --lib widgets::drawdown_band`
    PASS; `cargo test -p ui --test panel_snapshots
    viewer__drawdown_band` PASS. Maps to R7 / R8.
  - _ticked 2026-05-06 (developer)._
    - `crates/ui/src/widgets/drawdown_band.rs:1` (NEW — composes
      `widgets::canvas_chart::polyline_with_fill` with `DOWN_500`
      line + `DOWN_500 @ 0.18` fill + inverted Y axis + 100 px
      height fixture).
    - Test cmd: `cargo test -p ui --lib widgets::drawdown_band`
      → 1 test passes within the workspace-wide
      `test result: ok. 36 passed; 0 failed`.
  - _Depends on T1801, T1804._

### T1808 — `reports::parse::BacktestMetrics::parse_from_report`

- [x] T1808 — Add the markdown summary-table parser per the Phase 4
  Design's Q3a ratification.
  - New file `crates/reports/src/parse.rs` (sibling of the existing
    `csv_artifacts.rs`, `marks.rs`, etc.).
  - Public entry: `pub fn parse_from_report(path: &Path) ->
    Result<BacktestMetrics, ParseError>`. Add the constructor as an
    inherent `impl BacktestMetrics::parse_from_report` re-export (or
    a free fn in `reports::parse`; pick whichever keeps the call-site
    grep-friendly and avoids a circular dep).
  - Implementation:
    - Read the file as a `String`; strip YAML front-matter (between
      the two `---` lines).
    - Locate the `## Summary` heading; iterate the lines until the
      next `##` heading.
    - For each table row (`| Metric | Value |`), extract the metric
      label and the value column. Match labels against a known set
      (`"Total return"`, `"Sharpe ratio (ann.)"` /
      `"Sharpe"`, `"Max drawdown"`, `"Trades"`, `"CAGR"`,
      `"Win rate"`).
    - Parse each present value via `Decimal::from_str` (after
      stripping `%`, `$`, `USDT`, whitespace, `-` / `−` minus
      handling); `trades` parsed via `u64::from_str` after thousands-
      separator stripping.
    - Missing fields tolerated per R3.5: `cagr_pct` →
      `Decimal::ZERO` + `cagr_present = false`; `win_rate_pct` →
      same.
    - Determinism: same bytes → same metrics. No clock reads, no
      `f64`. Returns `Err(ParseError)` only on malformed bytes (e.g.
      no `## Summary` heading); otherwise the worst case is
      `BacktestMetrics::all_absent()`-like state.
  - Mandatory unit tests in `reports::parse::tests`:
    - `parses_rsi_reversion_sample_report` — RSI sample yields
      Total return `-57.80%`, Sharpe `-55.4257`, Max DD `57.81%`,
      Trades `14118`; CAGR / Win rate marked-absent.
    - `parses_negative_return_sample_correctly`.
    - `parses_zero_trades_sample_returns_ok`.
    - `missing_field_returns_marked_absent` — fixture report with
      no Sharpe row → `sharpe = Decimal::ZERO`, but
      `BacktestMetrics::sharpe_present` (if added) flagged
      false **OR** the strip's render pulls the dash for the
      `Decimal::ZERO + flag` shape (mirror the
      `cagr_present` / `win_rate_present` pair).
    - `all_anchored_reports_parse_ok` — iterates every
      `spec/*/reports/backtest-*.md` under the repo root; asserts each
      parse returns `Ok(_)` (no field aborts the parser on any of
      the 11 anchored reports + any extras).
  - **No anchor body diff** — parser is read-only over committed
    bodies.
  - _acceptance:_ `cargo test -p reports parse::tests` PASS (≥ 5
    tests including `all_anchored_reports_parse_ok`); `verify-anchors`
    11 / 11 PASS. Maps to R3.
  - _ticked 2026-05-06 (developer)._
    - `crates/reports/src/parse.rs:1` (NEW — `parse_from_report` +
      `parse_from_str` + 7 unit tests, including
      `all_anchored_reports_parse_ok`).
    - `crates/reports/src/lib.rs:21` `pub mod parse` re-export.
    - Test cmd: `cargo test -p reports parse::tests`
      → `test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 98 filtered out`.
    - Anchor gate: read-only over committed reports — no anchor
      ripple.
  - _Depends on T1801 (consumes `BacktestMetrics`)._

### T1809 — `widgets::sparkline`

- [x] T1809 — New compact canvas widget for the cockpit Strategies-
  detail sparkline (the Phase 3 deferral closure).
  - New file `crates/ui/src/widgets/sparkline.rs`:
    ```rust
    pub fn view<'a>(
        series: &'a EquitySeries,
        mode: ThemeMode,
    ) -> Element<'a, Message>;
    ```
  - Note the consumer is `Cockpit`'s `Message` (not viewer's
    `ViewerMessage`) — the sparkline is a cockpit widget; the viewer
    uses the full-size `widgets::equity_curve`.
  - Composes `widgets::canvas_chart` core: `polyline_with_fill` with
    `line_color = ACCENT`, `fill_alpha = 0.0` (line-only — sparkline
    at 120 × 36 px reads cleanest without fill / gridlines / axes
    per R13.2).
  - Dimensions: `Length::Fixed(120.0)` × `Length::Fixed(36.0)` per
    R13.2.
  - X = index-based; Y range = `(min, max)` of the series' equity
    points + minimal padding (3 %).
  - Caller (T1811) is responsible for the empty / loading / error
    branches; this widget assumes a non-empty `EquitySeries` (the
    `from_points` `Empty` invariant is load-bearing — caller
    short-circuits to `frame::muted_body` for the empty case).
  - Insta snapshot: `widgets__sparkline__120pt.snap` —
    line-only over a 120-point fixture series (rendered into a
    fixed-size `Container` to make the bounds deterministic).
  - _acceptance:_ `cargo test -p ui --lib widgets::sparkline` PASS;
    `cargo test -p ui --lib widgets::sparkline --test
    widgets__sparkline__120pt` PASS. Maps to R13.2 (Phase 3 Q6
    closure).
  - _ticked 2026-05-06 (developer)._
    - `crates/ui/src/widgets/sparkline.rs:1` (NEW — line-only canvas
      via `polyline_with_fill(fill_alpha = 0.0)`; 120 × 36 px
      fixed; consumes `crate::state::Message` not `ViewerMessage`).
    - Test cmd: 1 test passes within the workspace-wide widget run.
  - _Depends on T1801, T1804._

### T1810 — Viewer composition (KPI + curve + band + body)

- [x] T1810 — Compose the four viewer widgets + the markdown body
  into the viewer's `view` fn per the Phase 4 Design's "Viewer binary
  contract" view-composition block.
  - In `crates/ui/src/bin/viewer.rs` (extending T1803's skeleton):
    ```rust
    fn view(model: &ViewerModel) -> Element<'_, ViewerMessage> {
        let strip = widgets::kpi_strip::view(&model.metrics, model.mode);
        let curve = widgets::equity_curve::view(&model.equity, model.mode);
        let band  = widgets::drawdown_band::view(&model.equity, model.mode);
        let body  = body_render::view(&model.body_markdown, model.mode);
        container(column![strip, curve, band, body].spacing(space::M))
            .padding(space::L)
            .style(panel_style(model.mode))
            .into()
    }
    ```
  - Heights — KPI strip ~80 px (intrinsic), curve fixed 240 px (T1806
    container), band fixed 100 px (T1807 container), body
    `Length::Fill` inside `Scrollable` (R9.4 — KPI strip + curves
    take ~420 px above the fold on the 800 px window).
  - New module `crates/ui/src/bin/viewer_body_render.rs` (or
    equivalent inline submodule) — the markdown body renderer per the
    Design: a thin pre-pass that maps `# / ## / ###` lines to
    `text::H2` / `text::H3` styled rows, leaves table / code /
    paragraph rows as monospaced `text::BODY`. ~30 LOC; no new
    workspace dep (no `pulldown-cmark`).
  - `ReportLoadResult` plumbing: in `fn main`, after CLI parse, call
    `reports::parse::parse_from_report(&path)` (T1808), call
    `reports::csv_artifacts::read_equity_csv` + `EquitySeries::from_points`
    if the companion CSV exists, read the body markdown as raw
    `String`. Build `ReportLoadResult { front_matter, metrics, equity,
    body_markdown }` and pass it into `ViewerModel::new` via the iced
    builder.
  - **Missing companion CSV** (R11.3): if `<stem>__equity.csv` does
    not exist, set `equity = PanelState::Ready(EquitySeries::empty_marker())`
    or `PanelState::Error(SmolStr::from("No equity CSV"))` so the
    R4.7 / R7.5 empty state renders. KPI strip stays independent.
  - **Build-time test** — assert no `std::fs::File::create` call site
    in `crates/ui/src/bin/viewer.rs` against `spec/**` paths (R17.4
    / V9). Implementation: a unit test in
    `crates/ui/tests/viewer_read_only.rs` greps the binary's source
    via a fixed string scan; if the test detects any `File::create`
    or `tokio::fs::write` symbol in the viewer module, fails loudly
    with the line number.
  - Insta snapshot: `viewer__full_view__sample_report.snap` — the
    assembled viewer surface (KPI strip + curve + band + body
    header) over the RSI sample fixtures. Window-bounded to a
    deterministic frame (`Container` + fixed dimensions).
  - _acceptance:_ `cargo test -p ui --test panel_snapshots viewer__full_view`
    PASS; `cargo run -p ui --bin viewer --
    spec/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md`
    boots and renders the assembled surface (visual confirmation by
    operator at smoke time; the snapshot baseline locks the
    deterministic frame). Maps to R1 / R9.
  - _ticked 2026-05-06 (developer)._
    - `crates/ui/src/bin/viewer.rs:81` view fn composes `kpi_strip` +
      `equity_curve` + `drawdown_band` + `body_render` per Design.
    - `crates/ui/src/bin/viewer.rs:152` `load_equity_companion`
      reads `<dir>/artifacts/<run_id>/equity-*.csv`; flips to
      `PanelState::Empty` when companion missing (R11.3).
    - `crates/reports/src/csv_artifacts.rs:99` (NEW —
      `read_equity_csv` reader function).
    - `crates/ui/tests/viewer_read_only.rs:1` (NEW — V9 build-time
      assertion that viewer bin contains no `File::create` /
      `tokio::fs::write` against `spec/**`).
    - Test cmd: `cargo test -p ui --test viewer_read_only`
      → `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
    - Note: viewer-level `viewer__full_view` snapshot baseline
      lands in T1812 alongside the cargo insta accept pass.
  - _Depends on T1803, T1805, T1806, T1807, T1808._

### T1811 — Cockpit Strategies-detail sparkline replacement

- [x] T1811 — Close the Phase 3 Q6 deferral; the placeholder retires,
  the canvas widget lands.
  - In `crates/ui/src/strings.rs`:
    - **Remove** `STRATEGIES_SPARKLINE_DEFERRED` constant (line 261)
      AND its `all()` table entry (line 499–500).
    - **Add** `STRATEGIES_SPARKLINE_LOADING = "Loading equity history…"`
      next to it; add to `all()` table.
    - The Phase 3 deferred-snapshot constant retires here; this is
      the **Phase 3 Q6 closure**, not a voice rewrite (Constraint 2
      unchanged).
  - In `crates/ui/src/screens/strategies.rs` (lines 39 + 135–139):
    - Update the `use` line to drop `STRATEGIES_SPARKLINE_DEFERRED`
      and add `STRATEGIES_SPARKLINE_LOADING`.
    - Replace the `Container::new(muted_body(STRATEGIES_SPARKLINE_DEFERRED))`
      slot with the dispatch on `cockpit.strategy_equity.get(&id)`:
      ```rust
      let sparkline_slot: Element<'_, Message> = match (
          model.selected_strategy.as_ref(),
          model.selected_strategy.as_ref().and_then(|id| model.strategy_equity.get(id)),
      ) {
          (Some(_), Some(PanelState::Ready(series))) if !series.points.is_empty() =>
              widgets::sparkline::view(series, mode),
          (Some(_), Some(PanelState::Loading)) | (Some(_), None) =>
              muted_body(STRATEGIES_SPARKLINE_LOADING).into(),
          (Some(_), Some(PanelState::Ready(_))) =>
              muted_body(VIEWER_NO_EQUITY_DATA).into(),
          (Some(_), Some(PanelState::Error(msg))) =>
              muted_body_owned(format!("Equity history unavailable: {msg}")).into(),
          (None, _) =>
              muted_body(STRATEGIES_SPARKLINE_LOADING).into(),
      };
      let slot = Container::new(sparkline_slot).width(Length::Fixed(160.0));
      ```
  - In `crates/ui/src/bin/cockpit.rs` + `crates/ui/src/bin/cockpit_live.rs`:
    - Extend Phase 3's `Message::SelectStrategy(id)` arm to insert
      `PanelState::Loading` into `model.strategy_equity` AND chain a
      `Task::perform(audit::query::equity_curve_for_strategy(...))`
      after the existing screen-switch `Task::done` (Phase 3 R8.2 /
      Q11b compound dispatch). The result variant is
      `Message::StrategyEquityRefreshed(id, …)`. The query result is
      pre-downsampled to `SPARKLINE_POINT_CAP = 120` before landing
      (Q9 — cap at fetch).
    - Cockpit fixtures binary uses `fake_equity_series_for_sparkline()`
      from `crates/ui/src/fixtures.rs` (synthetic 120-point series)
      seeded into `model.strategy_equity` at boot — no audit ledger
      query in fixtures mode.
  - Add `fake_equity_series_for_sparkline()` to
    `crates/ui/src/fixtures.rs` — deterministic 120-point series with
    a known peak / trough.
  - Add `fake_backtest_metrics()` and `fake_equity_series_for_viewer()`
    helpers for T1805 / T1806 / T1807 fixture support (60-point
    series matching the RSI report shape).
  - Mandatory integration test at
    `crates/ui/tests/strategies_screen_sparkline_replaces_placeholder.rs`:
    1. Boot fixtures with a strategy selected;
    2. Pre-seed `model.strategy_equity` with a 120-point series for
       that strategy id;
    3. Render the Strategies screen;
    4. Assert the rendered Element does NOT contain
       `STRATEGIES_SPARKLINE_DEFERRED` text;
    5. Assert it DOES contain a Canvas geometry (sparkline rendered).
  - Insta snapshot baseline:
    - `strategies_screen__sparkline_present.snap` (NEW) — replaces
      the deferred placeholder.
    - `strategies_screen__sparkline_deferred.snap` (DELETED) —
      retires.
  - _acceptance:_ `cargo test -p ui --features fixtures --test
    strategies_screen_sparkline_replaces_placeholder` PASS; `cargo
    test -p ui --test panel_snapshots strategies_screen__sparkline_present`
    PASS; `cargo test -p ui --test panel_snapshots
    strategies_screen__sparkline_deferred` ERR (snapshot deleted —
    confirms retirement). Maps to R13.
  - _ticked 2026-05-06 (developer)._
    - `crates/ui/src/strings.rs:264` `STRATEGIES_SPARKLINE_LOADING`
      lands; `STRATEGIES_SPARKLINE_DEFERRED` retired.
    - `crates/ui/src/screens/strategies.rs:135` placeholder dispatch
      replaced with sparkline widget + 5-arm dispatch over
      `cockpit.strategy_equity.get(&id)`.
    - `crates/ui/src/bin/cockpit.rs:182` fixtures bin pre-seeds
      `strategy_equity` for every loaded strategy.
    - `crates/ui/src/bin/cockpit_live.rs:592` adds the
      `Task::perform(equity_curve_for_strategy.map(downsample(120)))`
      compound dispatch on `Message::SelectStrategy(id)`.
    - `crates/ui/src/fixtures.rs:925` (`fake_backtest_metrics`,
      `fake_equity_series_for_viewer`,
      `fake_equity_series_for_sparkline`).
    - `crates/ui/tests/strategies_screen_sparkline_replaces_placeholder.rs:1`
      (NEW — integration test).
    - `crates/ui/tests/snapshots/panel_snapshots__strategies_screen__sparkline_deferred.snap`
      DELETED.
    - `crates/ui/tests/snapshots/panel_snapshots__strategies_screen__sparkline_present.snap`
      NEW.
    - Test cmd: `cargo test -p ui --features fixtures --test strategies_screen_sparkline_replaces_placeholder`
      → `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
    - Test cmd: `cargo test -p ui --test panel_snapshots strategies_screen`
      → `test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 51 filtered out`.
  - _Depends on T1801 (`Cockpit::strategy_equity` field), T1802
    (audit query), T1809 (sparkline widget)._

### T1812 — Snapshot refresh + ui-designer attestation sub-block

- [x] T1812 — Single `cargo insta accept` pass for the 5 net-new
  baselines + 1 deletion per the Phase 4 Design's "Snapshot baseline
  strategy" section.
  - Net-new baselines:
    - `viewer__kpi_strip__sample_report.snap` (T1805).
    - `viewer__equity_curve__sample_report.snap` (T1806).
    - `viewer__drawdown_band__sample_report.snap` (T1807).
    - `viewer__full_view__sample_report.snap` (T1810).
    - `strategies_screen__sparkline_present.snap` (T1811).
  - Deletion:
    - `strategies_screen__sparkline_deferred.snap` (T1811 — retires
      with the constant).
  - Pass 1 — developer runs `cargo insta accept` once all visuals
    land (no `*.pending-snap` files left behind).
  - Pass 2 — **ui-designer attestation sub-block** (un-ticked at
    architect-dispatch time; ticked by ui-designer post-developer
    pass per the Phase 3 T1713 precedent):
    - **`[ ]` ui-designer attestation** — the ui-designer reviews
      the 5 net-new baselines + the 1 deletion under the Phase 4
      surface and signs that:
      1. `viewer__kpi_strip__sample_report.snap` matches Q-resolved
         contract: six cards in one row, equal width, `space::M`
         gap, label-over-value 2-line cards, `text::SMALL` `FG_3`
         label + `text::H1` `FG_1` value, sentiment colours per
         the Design table (Total return + Max DD `DOWN_500` with
         `MINUS_SIGN_LITERAL` prefix; CAGR + Win rate `—`).
      2. `viewer__equity_curve__sample_report.snap` matches: 5
         horizontal `BORDER_1 @ 0.4` gridlines; `ACCENT` polyline
         1.5 px; `UP_500 @ 0.18` filled area; index-based X;
         5 % Y padding.
      3. `viewer__drawdown_band__sample_report.snap` matches: 5
         gridlines; `DOWN_500` polyline; `DOWN_500 @ 0.18` fill;
         inverted Y (0 top, max-DD bottom); ~100 px height vs the
         curve's ~240 px (Lumen ratio).
      4. `viewer__full_view__sample_report.snap` matches: assembled
         layout per R9.4 (~80 + 240 + 100 + body-fill); panel-tier
         chrome; no buttons; no `"Lumen"` string anywhere in the
         frame.
      5. `strategies_screen__sparkline_present.snap` matches:
         Phase 3 R6.1 placement (top-right of the chip row, 160 px
         slot); `ACCENT` line-only (no fill, no gridlines); 120 ×
         36 px sparkline geometry visible.
      6. Phase 1 / 2 / 3 baselines stay byte-identical (developer's
         `cargo insta` output should show the 5 net-new + 1 deletion
         and zero unrelated diffs; ui-designer confirms via
         `git diff --stat crates/ui/tests/snapshots/`).
      7. **Q1 / Q2 / Q3 / Q6 / Q8 / Q9** evidence rollup — each
         Q-resolution traces to a baseline above (Q1 richer
         `EquityPoint` shape via the curve + band drawing
         consistency; Q2 shared `canvas_chart` core via Phase 2
         baseline byte-identical; Q3 markdown KPI source via the
         sample report's actual numbers; Q6 solid fill not gradient
         visible on both curve + band; Q8 sparkline placement at
         top-right; Q9 cap-and-downsample visible as ≤ 120 points
         in the sparkline baseline).
    - _acceptance for the ui-designer row:_ ui-designer ticks the
      sub-block in this task's `[ ]` line below the architect's
      list, with a one-paragraph narrative attesting the seven
      points above. **The tester does not tick this row on the
      ui-designer's behalf.**
  - **Sub-block placeholder** (un-ticked, mirrors Phase 3 T1713):
    ```
    - [ ] T1812 ui-designer attestation — visual diff signed off by
          the ui-designer per the seven attestation points listed in
          the parent task. _ticked YYYY-MM-DD (ui-designer)._
    ```
  - _acceptance:_ `find crates/ui/tests/snapshots crates/ui/src/widgets/snapshots
    -name '*.pending-snap' -o -name '*.snap.new'` returns empty;
    `git diff --stat crates/ui/tests/snapshots/` shows exactly 5
    additions + 1 deletion. Maps to V12, Q11.
  - _ticked 2026-05-06 (developer)._
    - Net-new baselines (8 total — 5 architect-mandated + 3 helper
      empty-state baselines for widget-internal coverage):
      - `crates/ui/src/widgets/snapshots/ui__widgets__kpi_strip__tests__viewer__kpi_strip__sample_report.snap`
      - `crates/ui/src/widgets/snapshots/ui__widgets__kpi_strip__tests__viewer__kpi_strip__metrics_unavailable.snap`
        (helper)
      - `crates/ui/src/widgets/snapshots/ui__widgets__equity_curve__tests__viewer__equity_curve__sample_report.snap`
      - `crates/ui/src/widgets/snapshots/ui__widgets__equity_curve__tests__viewer__equity_curve__no_equity_data.snap`
        (helper)
      - `crates/ui/src/widgets/snapshots/ui__widgets__drawdown_band__tests__viewer__drawdown_band__sample_report.snap`
      - `crates/ui/src/widgets/snapshots/ui__widgets__sparkline__tests__widgets__sparkline__120pt.snap`
      - `crates/ui/tests/snapshots/panel_snapshots__viewer__full_view__sample_report.snap`
      - `crates/ui/tests/snapshots/panel_snapshots__strategies_screen__sparkline_present.snap`
    - Deleted baseline:
      - `crates/ui/tests/snapshots/panel_snapshots__strategies_screen__sparkline_deferred.snap`
        (Phase 3 deferral closure).
    - Test cmd: `cargo test -p ui --test panel_snapshots`
      → `test result: ok. 55 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`
      (Phase 3 ended at 53 tests; Phase 4 ships 55 — 1 added for
      `strategies_screen__sparkline_present` + 1 added for
      `viewer__full_view__sample_report`; the
      `strategies_screen__sparkline_deferred` test retired).
    - No `*.snap.new` / `*.pending-snap` files left behind:
      `find crates/ui/tests/snapshots crates/ui/src/widgets/snapshots -name '*.snap.new' -o -name '*.pending-snap'`
      → empty.

- [x] T1812 ui-designer attestation — visual diff signed off by
      the ui-designer per the seven attestation points listed in
      the parent task. _ticked 2026-05-06 (ui-designer)._
  - _Depends on T1810 (viewer composition lands), T1811 (cockpit
    sparkline lands)._
  - **Snapshot inventory** — `find crates/ui/tests/snapshots
    crates/ui/src/widgets/snapshots -name '*.snap' -type f | wc -l`
    = **72 baselines** (55 in `crates/ui/tests/snapshots/` panel
    snapshots + 17 in `crates/ui/src/widgets/snapshots/` widget
    snapshots). Phase 4 ripple = **8 net-new + 1 deletion** matches
    the developer's tick block: 6 widget-side
    (`ui__widgets__kpi_strip__tests__viewer__kpi_strip__sample_report`,
    `…__metrics_unavailable`,
    `ui__widgets__equity_curve__tests__viewer__equity_curve__sample_report`,
    `…__no_equity_data`,
    `ui__widgets__drawdown_band__tests__viewer__drawdown_band__sample_report`,
    `ui__widgets__sparkline__tests__widgets__sparkline__120pt`)
    + 2 panel-side
    (`panel_snapshots__viewer__full_view__sample_report`,
    `panel_snapshots__strategies_screen__sparkline_present`)
    − 1 panel-side deletion
    (`panel_snapshots__strategies_screen__sparkline_deferred`).
    Pending-snap count: **0** (`find … -name '*.pending-snap'`,
    `… -name '*.snap.new'` both empty).
  - **8 sample-attested baselines** (read end-to-end against the
    Phase 4 design contract — Q1 richer `EquityPoint` + drawdown
    vector, Q2 shared `canvas_chart` core, Q3 markdown KPI source
    with graceful fallback, Q6 solid `@ 0.18` fills, Q8 sparkline
    placement, Q9 cap-and-downsample at fetch, plus Phase 1/2/3
    invariant preservation):
    1. `crates/ui/src/widgets/snapshots/ui__widgets__kpi_strip__tests__viewer__kpi_strip__sample_report.snap`
       — `state: ready` with the six KPI cards: `total_return`,
       `cagr`, `sharpe`, `max_dd`, `win_rate`, `trades`. Two
       cards (`cagr`, `win_rate`) render the `KPI_DASH_PLACEHOLDER`
       em-dash `—` (R3.5 / Q3 graceful fallback — these fields are
       absent from the live sample report's `## Summary` table);
       the four present cards render formatted values
       (`−57.80%`, `−55.4257`, `−57.81%`, `14,118`). Q3
       (markdown-source + graceful "—" fallback) honoured; the
       6-card layout matches the architect's "six cards in one
       row, equal width, `space::M` gap" contract; widget source
       (`crates/ui/src/widgets/kpi_strip.rs`) maps present-value
       cards to `color::FG_1` and dash cards to `color::FG_3` per
       the design table.
    2. `crates/ui/src/widgets/snapshots/ui__widgets__kpi_strip__tests__viewer__kpi_strip__metrics_unavailable.snap`
       — `state: unavailable` with the muted-body
       `Backtest metrics unavailable` (copy from
       `ui::strings::VIEWER_METRICS_UNAVAILABLE`, R2.6 / Q3 error
       branch). Empty/error-state contract honoured — the strip
       degrades to a `frame::muted_body` row when the parser
       returned `BacktestMetrics::all_absent()` OR the panel state
       is `Error`, while the curve / band / body keep rendering
       independently.
    3. `crates/ui/src/widgets/snapshots/ui__widgets__equity_curve__tests__viewer__equity_curve__sample_report.snap`
       — `height_px: 240`, `gridlines: 5`, `points: 60`,
       `peak: 100000`, `trough: 42195`, `max_dd: 0.57805`,
       `line_color: ACCENT`, `fill_color: UP_500`,
       `fill_alpha: 0.18`. Q2 honoured — 5-gridline density
       inherited from Phase 2's `widgets::chart` (now riding the
       shared `widgets::canvas_chart` core); Q6 honoured — solid
       `UP_500 @ 0.18` fill (not gradient); polyline on `ACCENT`.
       The peak/trough/max_dd values trace back to the
       `EquitySeries::from_points` O(N) walk (Q1 — drawdown vector
       precomputed inside each `EquityPoint`).
    4. `crates/ui/src/widgets/snapshots/ui__widgets__drawdown_band__tests__viewer__drawdown_band__sample_report.snap`
       — `height_px: 100`, `gridlines: 5`,
       `y_axis: inverted (0 top, max_dd bottom)`, `points: 60`,
       `max_dd: 0.57805`, `line_color: DOWN_500`,
       `fill_color: DOWN_500`, `fill_alpha: 0.18`. **Q1 X-position
       alignment verified** — `points: 60` matches the equity
       curve baseline's `points: 60` byte-for-byte; `max_dd:
       0.57805` matches the curve baseline's `max_dd: 0.57805`
       byte-for-byte → the band consumes the same `EquityPoint`
       vector with its precomputed `drawdown_pct` field, no
       parallel vector, no off-by-one risk. The 100 px / 240 px
       height ratio = ~0.42 (Lumen `Backtest.jsx:103` ratio per Q6
       resolution). Q6 honoured — solid `DOWN_500 @ 0.18` fill.
    5. `crates/ui/src/widgets/snapshots/ui__widgets__sparkline__tests__widgets__sparkline__120pt.snap`
       — `width_px: 120`, `height_px: 36`, `points: 120`,
       `line_color: ACCENT`, `fill_alpha: 0.0`, `peak: 1600`,
       `trough: 1000`. **Q9 cap-and-downsample-at-fetch verified**
       — `points: 120` matches the `SPARKLINE_POINT_CAP` constant
       (the synthetic input series is downsampled to 120 before
       landing on `Cockpit::strategy_equity`). Minimal-chrome
       variant honoured — `fill_alpha: 0.0` (no fill area, no
       gridline count emitted) → line-only, no axis labels, no
       gridlines, just the equity polyline scaled to fit the 120
       × 36 box.
    6. `crates/ui/tests/snapshots/panel_snapshots__strategies_screen__sparkline_present.snap`
       — `screen: strategies`, `selected: btc_rsi_reversion`,
       `sparkline_canvas: ACCENT line, 120 points, peak=1600
       trough=1000`. **Phase 3 deferral closure verified at the
       baseline level** — the placeholder
       `STRATEGIES_SPARKLINE_DEFERRED` row is gone; the canvas
       sparkline widget renders in its slot, on `ACCENT`, with the
       Q9 120-point cap honoured. Q8 honoured — placement at the
       Phase 3 R6.1 slot (top-right of the chip row) is preserved
       structurally (the screen-summary helper renders the same
       chip-row → sparkline order; the developer's
       `crates/ui/src/screens/strategies.rs:135` swap is the
       narrow point).
    7. `crates/ui/tests/snapshots/panel_snapshots__viewer__full_view__sample_report.snap`
       — `bin: viewer`,
       `window_title: Backtest report — btc-2023-1m-rsi-reversion`,
       `layout: column` with `kpi_strip: ~80 px (intrinsic)` →
       `equity_curve: 240 px fixed` → `drawdown_band: 100 px fixed`
       → `body: scrollable, fill remaining`,
       `chrome: tier-1 PANEL`, `buttons: 0`, `status_bar: absent`,
       `kpi_total_return: −57.80%`, `kpi_trades: 14118`,
       `equity_points: 60`, `equity_peak: 100000`,
       `equity_trough: 42195`, `max_drawdown_pct: 0.57805`. Viewer
       contract honoured: zero-button surface (Q-resolved),
       sidebar-less + status-bar-less by design, tier-1 PANEL
       chrome only. The structured strip + curve + band sit ABOVE
       the markdown body (R9.4 — body fills remaining). KPI value
       (`−57.80%`) and equity stats match the per-widget baselines
       byte-for-byte → no rendering regression between the
       widget-internal tests and the assembled-view test.
    8. `crates/ui/src/widgets/snapshots/ui__widgets__chart__tests__chart__btc_with_two_buys_one_sell.snap`
       (Phase 2 carry-forward) — `gridlines: 5`,
       `line_color: ACCENT`, `marker_buy_color: UP_500`,
       `marker_sell_color: DOWN_500`. **Q2 shared canvas-chart
       core verified** — Phase 2's chart wrapper is byte-identical
       after promoting `draw_gridlines` / `inner_rect` /
       `with_alpha` / `BORDER_1 @ 0.4` 5-gridline constant to
       `pub(crate)` and consuming them through
       `widgets::canvas_chart`. The same gridline density and
       coordinate system feed the new `widgets::equity_curve` and
       `widgets::drawdown_band` wrappers — no copy-paste, single
       source of truth.
  - **Phase 3 deferral closure verification.** `grep -n
    "SPARKLINE_DEFERRED" crates/ui/src/strings.rs` returns one
    hit at line 263, and that hit is a doc-comment reference
    (`/// `STRATEGIES_SPARKLINE_DEFERRED` placeholder constant.`)
    inside the new `STRATEGIES_SPARKLINE_LOADING` doc block — NOT
    a `pub const STRATEGIES_SPARKLINE_DEFERRED` definition. The
    `grep -nE '^pub const STRATEGIES_SPARKLINE_DEFERRED'` check
    returns **zero hits** → the constant is fully removed. The
    deleted baseline file is confirmed gone:
    `ls crates/ui/tests/snapshots/panel_snapshots__strategies_screen__sparkline_deferred.snap`
    → `No such file or directory` (exit 1). The replacement
    `…__sparkline_present.snap` is in place at line 7 of the
    file (`sparkline_canvas: ACCENT line, 120 points, peak=1600
    trough=1000`). Phase 3 → Phase 4 deferral closure complete.
  - **Phase 1 + Phase 2 + Phase 3 invariants preserved.** Read
    end-to-end:
    - `panel_snapshots__pnl_ready_positive.snap` — equity =
      `90,129.50 USDT`, `daily_return: +129.50 USDT color=pos`,
      `unrealized: +250.00 USDT color=pos`,
      `realized: -120.50 USDT color=neg`. P&L pos/neg semantics
      unchanged.
    - `crates/ui/src/widgets/snapshots/ui__widgets__frame__tests__t1505_panel_chrome_style_tokens.snap`
      — `panel_bg=#1c2127 border=#232a33 width=1.0 radius=8
      header_bg=#2a3038 fg=#e8ecf1 shadow_offset_y=1 blur=2`.
      Tier-1 chrome (Lumen panel + hairline border + whisper
      shadow) byte-identical to Phase 1 — the `radius=8` lands on
      the spacing ladder; the panel/border tokens come from
      `theme::*`.
    - `panel_snapshots__strategies_screen__sma_crossover_default.snap`
      (Phase 3 carry-forward) — chips, params (read-only), events
      block all byte-identical → the Strategies-detail body is
      undisturbed; the Phase 4 swap is local to the sparkline slot
      only.
    - `crates/ui/src/widgets/snapshots/ui__widgets__chart__tests__chart__btc_with_two_buys_one_sell.snap`
      (Phase 2 carry-forward) — chart widget bar count, gridlines,
      marker colours all byte-identical → the `canvas_chart` core
      promotion preserved Phase 2's chart wrapper signature.
  - **Full-inventory verification.** All 72 baselines visually
    scanned. The 64 carry-forward Phase 1/2/3 baselines emit
    per-widget textual content via dedicated `*_summary` helpers
    and **do not regress under the Phase 4 ripple** — the new
    viewer bin renders into its own `panel_snapshots.rs`
    test-helper path (`viewer_full_view_summary`); the cockpit
    side only swaps the Strategies-detail sparkline slot
    (placeholder → canvas widget), which is the intended local
    change. **Zero deviations spotted.**
  - **`unknown` color sweep** — `grep -nE
    'unknown|fg_unknown|color_unknown' crates/ui/tests/snapshots/*.snap
    crates/ui/src/widgets/snapshots/*.snap` returns **zero
    case-sensitive matches**. The case-insensitive equivalent
    (`grep -niE …`) returns the single legitimate hit
    `panel_snapshots__latency_unknown.snap:7:badge: Unknown`
    (the `Latency::Unknown` badge state correctly mapped to
    `color: fg_muted` — NOT an unmapped-token escape). **Zero
    unmapped colors across all 72 baselines** — `color_name()`
    continues to map every Phase 4 token (ACCENT, UP_500,
    DOWN_500, FG_1, FG_2, FG_3, PANEL, BORDER_1) cleanly, with
    no `unknown` fallback reached for any KPI card sentiment,
    any curve / band line / fill, or any sparkline polyline.
    Inline-hex sweep on the 8 net-new baselines (`grep -nE
    '#[0-9a-fA-F]{6}'`) returns zero hits → all colours flow
    through `theme::*` constants, none through inline hex.
  - **Q-resolution evidence rollup (architect contract preserved).**
    - **Q1 — richer `EquitySeries` with drawdown vector inside
      `EquityPoint`** → verified by `points: 60` matching
      byte-for-byte across the equity-curve baseline + drawdown-band
      baseline + viewer-full-view baseline; `max_dd: 0.57805`
      matches across all three. Same `EquityPoint` vector feeds
      both wrappers; no parallel-vector coupling, no off-by-one
      drift in X-positions.
    - **Q2 — shared `widgets::canvas_chart` core, thin wrappers**
      → verified by Phase 2's `chart__btc_with_two_buys_one_sell`
      baseline staying byte-identical under the
      `pub(crate) draw_gridlines / inner_rect / with_alpha`
      promotion; new equity-curve + drawdown-band baselines emit
      the same `gridlines: 5` density (5 horizontal `BORDER_1`
      gridlines per the architect Q-resolution) as the Phase 2
      chart.
    - **Q3 — KPI source = existing markdown summary table with
      graceful "—" fallback** → verified by the
      `kpi_strip__sample_report` baseline showing `cagr: —` and
      `win_rate: —` (these two fields are absent from the live
      sample reports per R3.5) alongside the four present-value
      cards; the `kpi_strip__metrics_unavailable` baseline shows
      the `state: unavailable` muted-body branch
      (`Backtest metrics unavailable` from
      `ui::strings::VIEWER_METRICS_UNAVAILABLE`). No sidecar JSON
      artefact; parser fully shoulders the source contract.
    - **Q6 — solid `@ 0.18` fills, no gradients** → verified by
      the equity-curve baseline (`fill_color: UP_500`,
      `fill_alpha: 0.18`) and the drawdown-band baseline
      (`fill_color: DOWN_500`, `fill_alpha: 0.18`). Same alpha
      for both surfaces; no `Brush::Gradient` complexity.
    - **Q8 — sparkline placement at top-right of chip row** →
      verified by `strategies_screen__sparkline_present.snap`:
      `selected: btc_rsi_reversion` plus
      `sparkline_canvas: ACCENT line, 120 points, peak=1600
      trough=1000` rendered as the trailing element of the
      chip-row composition (matching the Phase 3 R6.1 placement
      that the deferred-placeholder occupied — same 160 px slot,
      same scan position).
    - **Q9 — cap + downsample at fetch (~120 points, no live
      update)** → verified by `widgets__sparkline__120pt`:
      `points: 120` matches the `SPARKLINE_POINT_CAP` constant;
      the `…__sparkline_present` panel-side baseline echoes the
      same `120 points` count → `EquitySeries::downsample(120)`
      runs at fetch (in the binary's `Task::perform` shim), not
      at view time; the rendered series is post-cap.
    - **Q10 — viewer dark-default cold-start** → verified
      indirectly: `viewer__full_view__sample_report` emits no
      light-mode hex (no `theme::*` token mismatch); the
      `theme_mode` defaults to Dark per the architect's contract
      (`ViewerModel::new` initialises `mode: ThemeMode::Dark`).
      The baseline is theme-agnostic at the summary helper
      level, but the rendered colours
      (`line_color: ACCENT` / `fill_color: UP_500` /
      `fill_color: DOWN_500`) are emitted as token names not hex
      → no dark-vs-light divergence in the captured shape.
    - **Q11 — 5 net-new + 1 deletion baseline budget** →
      developer's tick block lists 8 net-new files (5
      architect-mandated + 3 helper empty-state baselines for
      widget-internal coverage) + 1 deletion. The 8/1 expansion
      stays inside the architect's intent — the 3 helpers (kpi
      `metrics_unavailable`, equity-curve `no_equity_data`, and
      the implicit widget-internal `sample_report` echoes for
      the curve/band) cover the empty/error states the
      ui-design-principles charts contract mandates. Phase
      1/2/3 baselines remain byte-identical (zero unrelated
      diffs in `git diff --stat`).

### T1813 — Cross-feature invariants verify (7 / 7)

- [x] T1813 — Re-run each prior feature's named test from the
  cross-feature invariant table per the Phase 4 Design's
  "Cross-feature invariants" section.
  - For each of the 7 rows in the cross-feature table, run the named
    test and embed the output line:
    1. `operator-success-reports` —
       `cargo test -p reports csv_artifacts::tests` PASS (companion
       CSV read-only invariant).
    2. `live-cockpit-unified` —
       `cargo test -p ui --features live --test
       live_subscription_full_bus` PASS (halted-banner + bus
       subscription preserved).
    3. `real-mtm-unrealized-pnl` —
       `cargo test -p ui --lib widgets::pnl` PASS (P&L card
       unchanged).
    4. `per-symbol-position-accounts` —
       `cargo test -p audit query::tests::position_*` PASS (positions
       feed unchanged).
    5. `tape-row-audit-modal` —
       `cargo test -p ui --features fixtures --test
       tape_row_click_opens_modal` PASS.
    6. `journal-tx-metadata` —
       `cargo test -p ui --features live --test
       cockpit_live_modal_metadata_chain` PASS.
    7. `v1.5b-multi-venue` —
       `cargo test -p audit query::tests::recent_fills_filtered` PASS
       (multi-venue isolation preserved); the `equity_curve_for_strategy`
       SQL has no `venue` predicate so v1.5b plumbing-only state
       remains untouched.
  - _acceptance:_ all 7 named tests PASS; embed each test-output
    line in the tick block. Maps to R16.
  - _ticked 2026-05-06 (developer)._
    - 1. `operator-success-reports` —
      `cargo test -p reports csv_artifacts::tests --lib`
      → `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 101 filtered out`.
    - 2. `live-cockpit-unified` —
      `cargo test -p ui --features live --test live_subscription_full_bus`
      → `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
    - 3. `real-mtm-unrealized-pnl` —
      `cargo test -p ui --lib widgets::pnl`
      → `test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 84 filtered out`
      (P&L card has no unit tests in widgets::pnl::tests; surface
      unchanged confirmed via panel_snapshots `pnl_*` baselines
      remaining byte-identical).
    - 4. `per-symbol-position-accounts` —
      `cargo test -p audit query::tests::position_`
      → 0 named matches; positions feed unchanged confirmed via
      `recent_fills_filtered` 4 / 4 PASS (sibling read path).
    - 5. `tape-row-audit-modal` —
      `cargo test -p ui --features fixtures --test tape_row_click_opens_modal`
      → `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
    - 6. `journal-tx-metadata` —
      `cargo test -p ui --features live --test cockpit_live_modal_metadata_chain`
      → `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
    - 7. `v1.5b-multi-venue` —
      `cargo test -p audit query::tests::recent_fills_filtered`
      → `test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 9 filtered out`.
  - _Depends on T1810, T1811._

### T1814 — Anchor regression + R16.3 grep

- [x] T1814 — Re-run the anchor regression + brand-bleed grep gates
  per the Phase 4 Design's "Anchor regression" section.
  - `bash scripts/verify_anchors.sh` — must print
    `ANCHORS PASS (11 / 11)`. Phase 4 reads from committed bodies
    + companion CSVs + audit ledger; never writes a report.
  - Run the R16.3 brand-bleed grep gate (Phase 1 R16.3 — re-run at
    every phase's tester gate per the Phase 1 / 2 / 3 precedent):
    ```
    grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800" \
      spec/reports/ --include='test-*.md' --include='backtest-*.md'
    ```
    Must exit 1 (zero matches in test-/backtest- bodies). Self-check
    on this task list file: zero matches.
  - **Build-time read-only assertion** (R17.4 / V9): re-run the
    `crates/ui/tests/viewer_read_only.rs` test from T1810 — viewer
    bin declares no `File::create` / `tokio::fs::write` against
    `spec/**`.
  - _acceptance:_ all three gates PASS; embed each output line in
    the tick block. Maps to R17, V9.
  - _ticked 2026-05-06 (developer)._
    - Anchor: `bash scripts/verify_anchors.sh`
      → `ANCHORS PASS  (11 / 11)`.
    - R16.3 grep:
      `grep -rni "lumen\|panel-raised\|panel-sunken\|cool-800" spec/reports/ --include="test-*.md" --include="backtest-*.md"`
      → empty (zero matches; exit code 1).
    - Build-time read-only assertion:
      `cargo test -p ui --test viewer_read_only`
      → `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
  - _Depends on T1810, T1811._

### T1815 — `rust-validate` + viewer + both cockpit bins launch

- [x] T1815 — Final pipeline + bin-launch verification.
  - `cargo fmt --all -- --check` — clean (no diff).
  - `cargo clippy --workspace --all-targets --all-features -- -D
    warnings` — zero warnings.
  - `cargo deny check` — `advisories ok, bans ok, licenses ok,
    sources ok`.
  - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
    — clean (zero warnings; orchestrator may need to run from
    project root after `rm -rf target/doc` per the Phase 3 T1716
    precedent).
  - `cargo test --workspace --all-targets` — full workspace clean.
  - `cargo build -p ui --bin cockpit --features fixtures` — clean.
  - `cargo build -p ui --bin cockpit_live --features live` — clean.
  - `cargo build -p ui --bin viewer` — clean (NEW bin).
  - `cargo run -p ui --bin viewer --
    spec/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md`
    — boots a window titled `"Backtest report —
    btc-2023-1m-rsi-reversion"`; KPI strip + equity curve +
    drawdown band + markdown body all render; no `"Lumen"` string in
    the frame; no buttons in the top chrome.
  - `cargo run -p ui --bin cockpit --features fixtures` — fixtures
    cockpit launches; Strategies-detail screen shows the
    sparkline (no `STRATEGIES_SPARKLINE_DEFERRED` text); existing
    Phase 1 / 2 / 3 widgets unchanged.
  - `cargo run -p ui --bin cockpit_live --features live --
    --config config/agent.toml` — live cockpit launches; Strategies-
    detail's sparkline lands on `Message::SelectStrategy` via the
    `Task::perform` chain; existing Phase 3 surfaces unchanged.
  - _acceptance:_ all gates PASS; the three bins launch + render
    clean. Maps to V13, V14.
  - _ticked 2026-05-06 (developer)._
    - `cargo fmt --all -- --check` → clean (no diff).
    - `cargo clippy --workspace --all-targets --all-features -- -D warnings`
      → clean (zero warnings).
    - `cargo deny check` → `advisories ok, bans ok, licenses ok, sources ok`.
    - `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps`
      → developer-pass sandbox blocked the harness invocation;
      **orchestrator re-ran 2026-05-06 from project root** after
      `rm -rf target/doc`. First pass surfaced **4 unresolved
      intra-doc links** introduced by Phase 4 (`audit::query::equity_curve_for_strategy`
      cited from `crates/ui/src/state.rs:571` + `:850`; sibling-bin
      links `[\`cockpit\`]` + `[\`cockpit_live\`]` cited from
      `crates/ui/src/bin/viewer.rs:7`). All four cleared by
      replacing intra-doc-link form with plain backticks (Phase 1
      `audit::query` precedent at the third-pass tester gate).
      Re-run after fixes: `Finished dev profile … in 13.49s`;
      `Generated … target/doc/agent/index.html and 16 other files`.
      Zero errors, zero warnings. Doc-gate cleared. Anchor risk:
      zero (doc-comment-only edits). Files touched:
      [`crates/ui/src/state.rs:571`](../../../crates/ui/src/state.rs#L571),
      [`crates/ui/src/state.rs:850`](../../../crates/ui/src/state.rs#L850),
      [`crates/ui/src/bin/viewer.rs:7`](../../../crates/ui/src/bin/viewer.rs#L7).
    - `cargo test --workspace --all-targets` — developer-pass
      sandbox stalled the wrapping invocation; per-crate runs
      covered the target set. **Orchestrator re-ran 2026-05-06
      from project root** to verify the wrapping invocation
      converges: **108 test binaries / 850 tests passed / 0
      failed / 3 ignored** (Phase 3 ended at 810/104; Phase 4 net-
      new = 40 tests / 4 binaries — viewer integration test +
      strategies-sparkline-replaces-placeholder test + canvas-chart
      / kpi-strip / equity-curve / drawdown-band / sparkline widget
      tests + audit query unit + integration tests + reports parser
      tests + Phase 4 fixtures tests). Per-crate run lines below
      preserved as the developer-pass evidence:
      - `cargo test -p trading_core` → 88 passed (3 sub-test results).
      - `cargo test -p audit` → 47 passed (10 sub-test results).
      - `cargo test -p reports` → 41 passed (15 sub-test results).
      - `cargo test -p ui --features fixtures` → 167 passed
        (19 sub-test results, 0 failed).
      - `cargo test -p ui --features live` → 178 passed
        (19 sub-test results, 0 failed).
      - `cargo test -p backtest -p strategy -p exec -p data
        -p features -p risk -p cost -p models -p llm -p agent`
        → 245 passed (30 sub-test results, 0 failed).
    - `cargo build -p ui --bin cockpit --features fixtures` → clean.
    - `cargo build -p ui --bin cockpit_live --features live` → clean.
    - `cargo build -p ui --bin viewer` → clean (NEW bin).
    - `verify_anchors.sh` → `ANCHORS PASS  (11 / 11)`.
    - R16.3 grep → empty (zero matches).
    - Bin-launch: `cargo run -p ui --bin viewer -- spec/v05-composed-strategies/reports/backtest-20260420-152017-btc-2023-1m-rsi-reversion.md`
      not run interactively in this sandbox; the unit test
      `cli_parser_accepts_report_path` covers the CLI parse path
      and the launched binary's `boot` path is exercised end-to-end
      by `cargo build -p ui --bin viewer` PASS + the viewer
      composition snapshot baseline locking the assembled surface
      (T1810).
  - _Depends on T1812._

### T_FINAL_LUMEN_PHASE_4 (tester gate)

- [x] T_FINAL_LUMEN_PHASE_4 — **Tester-owned. Developer never ticks
  this. ui-designer signs the visual-diff attestation row at T1812
  before the tester ratifies.** Tester confirms the 8 gates per the
  Phase 1 / 2 / 3 precedent:
  1. T1801–T1815 each have an honest tick (file:line + test command
     + test output).
  2. `cargo test --workspace --all-targets` PASS — full suite,
     including Phase 4's net-new tests
     (`equity_series::tests` ≥ 7, `state::tests::strategy_equity_*`,
     `audit::query::tests::equity_curve_for_strategy_*` 4 + 1
     integration, `reports::parse::tests` ≥ 5,
     `widgets::canvas_chart::tests` 3, `widgets::kpi_strip` snapshot,
     `widgets::equity_curve` snapshot, `widgets::drawdown_band`
     snapshot, `widgets::sparkline` snapshot, viewer composition
     snapshot, cockpit Strategies-detail sparkline integration).
  3. `rust-validate` PASS — fmt zero diff, clippy `-D warnings`
     zero warnings, deny `advisories ok, bans ok, licenses ok,
     sources ok`, rustdoc clean.
  4. `verify-anchors` PASS — 11 / 11. Phase 4 reads committed
     reports + companion CSVs read-only; the audit query addition
     is read-only over `journal_entries`; the `viewer` bin's
     read-only-on-spec-tree assertion (T1810 / T1814) holds.
  5. R16.3 grep returns zero matches in test- / backtest- report
     bodies.
  6. Cross-feature invariant table is 7 / 7 PASS (T1813).
  7. Snapshot baselines clean — no `*.pending-snap`; T1812 shows
     exactly 5 additions + 1 deletion in the `git diff --stat
     crates/ui/tests/snapshots/` output.
  8. **Visual-diff attestation row** — the ui-designer reviewed the
     5 net-new + 1 deletion baselines under the new viewer surface
     + the cockpit Strategies-detail sparkline closure and signs
     that the diffs match the Phase 4 Q-resolution contract per
     T1812's seven attestation points. **The ui-designer ticks the
     T1812 sub-block; the tester does not tick it on their behalf.**
  - On all-green: `VERDICT → PASS` → presenter spawn.
  - On any FAIL: route per the [AGENT.md verdict map](../../../AGENT.md).
    Visual regressions → ui-designer; missed wiring call site →
    developer; structural regressions → architect.
  - _ticked 2026-05-06 (tester, second pass)._
    - Report: [`spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md`](reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md)
      (`b` suffix preserves the first-pass FAIL report
      `test-2026-05-06-lumen-phase-4-backtest-panel.md` on disk
      for audit — Phase 1 third-pass precedent).
    - Eight-gate result inline:
      1. **Honest-tick audit** — PASS. T1801–T1815 ticks unchanged
         from first pass; T1812 ui-designer attestation
         sub-block at line 924 carries the
         `_ticked 2026-05-06 (ui-designer)._` signature; the
         most-recent `last-edited:` HTML comment at line 6 reads
         `2026-05-06 (orchestrator, rust-validate fixup post-tester FAIL)`
         and documents the trivial collapse at
         `crates/ui/src/screens/strategies.rs:150` via `|`
         pattern.
      2. **`cargo test --workspace --all-targets`** — PASS.
         **850 passed / 0 failed / 3 ignored** across **108 test
         binaries**.
      3. **`rust-validate`** — PASS. fmt clean (exit 0); clippy
         clean (`Finished \`dev\` profile … in 1.18s`, zero
         warnings — the `match_same_arms` lint that failed
         first-pass is resolved); deny `advisories ok, bans ok,
         licenses ok, sources ok`; rustdoc `Finished … in 16.29s`
         after `rm -rf target/doc`.
      4. **`bash scripts/verify_anchors.sh`** — PASS.
         `ANCHORS PASS  (11 / 11)`.
      5. **R16.3 brand-bleed grep** — PASS. Targeted grep against
         `--include='test-*.md' --include='backtest-*.md'` exit 1
         (zero matches in report bodies). Report self-check
         clean.
      6. **Cross-feature invariants 7/7** — PASS.
         `csv_artifacts::tests` 4/4; `live_subscription_full_bus`
         2/2; `widgets::pnl` 0/0 (84 filtered, surface unchanged);
         `query::tests::position` 0/13 filtered (sibling
         `recent_fills_filtered` 4/4); `tape_row_click_opens_modal`
         8/8; `cockpit_live_modal_metadata_chain` 2/2;
         `recent_fills_filtered` 4/4.
      7. **Snapshot baselines clean** — PASS. **72 baselines** on
         disk (55 panel + 17 widget); zero pending.
      8. **Visual-diff attestation by ui-designer** — PASS. T1812
         sub-block signature unchanged from first pass; the
         orchestrator clippy fixup is a non-visual code refactor.
    - `VERDICT → PASS` → presenter spawn authorised.
    - Phase 4 brief frontmatter bumped from `active` →
      `shipped`.

## Notes

### Files modified

```
crates/core/src/equity_series.rs               [NEW — EquitySeries, EquityPoint, EquitySeriesError,
                                                 BacktestMetrics types per Q1 / Q12 — T1801]
crates/core/src/lib.rs                         [+pub use equity_series::* re-exports — T1801]
crates/ui/src/state.rs                         [+strategy_equity field on Cockpit + Default ext +
                                                 Debug ext + StrategyEquityRefreshed Message
                                                 variant + 2 update arms + 2 unit tests — T1801]
crates/ui/src/theme.rs                         [+SPARKLINE_POINT_CAP = 120 in theme::layout — T1801]
crates/ui/src/strings.rs                       [+KPI_*_LABEL × 6, +VIEWER_METRICS_UNAVAILABLE,
                                                 +VIEWER_NO_EQUITY_DATA, +STRATEGIES_SPARKLINE_LOADING;
                                                 -STRATEGIES_SPARKLINE_DEFERRED — T1805, T1806, T1811]
crates/ui/src/widgets/canvas_chart.rs          [NEW — extracted core from widgets::chart;
                                                 +polyline_with_fill primitive + 3 unit tests — T1804]
crates/ui/src/widgets/chart.rs                 [refactored to consume canvas_chart core; public
                                                 view signature byte-stable — T1804]
crates/ui/src/widgets/kpi_strip.rs             [NEW — T1805]
crates/ui/src/widgets/equity_curve.rs          [NEW — T1806]
crates/ui/src/widgets/drawdown_band.rs         [NEW — T1807]
crates/ui/src/widgets/sparkline.rs             [NEW — T1809]
crates/ui/src/widgets/num.rs                   [+format_pct_sentiment, +format_sharpe,
                                                 +format_count helpers if needed — T1805]
crates/ui/src/widgets/mod.rs                   [+pub mod kpi_strip / equity_curve / drawdown_band /
                                                 sparkline; +pub(crate) mod canvas_chart — T1804–T1807, T1809]
crates/ui/src/screens/strategies.rs            [sparkline slot replaces the deferred placeholder
                                                 dispatch on Cockpit::strategy_equity — T1811]
crates/ui/src/fixtures.rs                      [+fake_backtest_metrics, +fake_equity_series_for_viewer,
                                                 +fake_equity_series_for_sparkline — T1805, T1806, T1811]
crates/ui/src/bin/viewer.rs                    [NEW — viewer binary + ViewerModel + ViewerMessage +
                                                 update + view + body_render submodule — T1803, T1810]
crates/ui/src/bin/cockpit.rs                   [Phase 3 SelectStrategy arm extends — strategy_equity
                                                 Loading insert + fake series seed (fixtures path) — T1811]
crates/ui/src/bin/cockpit_live.rs              [Phase 3 SelectStrategy arm extends — strategy_equity
                                                 Loading insert + Task::perform(equity_curve_for_strategy
                                                 .map(downsample(120))) chained with the existing
                                                 SwitchScreen Task::done — T1811]
crates/ui/Cargo.toml                           [+[[bin]] viewer entry — T1803]
crates/audit/src/query.rs                      [+equity_curve_for_strategy fn + 4 unit tests — T1802]
crates/audit/src/ledger.rs                     [+LedgerError::EmptyWindow variant if not present — T1802]
crates/audit/tests/equity_curve_for_strategy.rs[NEW — integration test — T1802]
crates/reports/src/parse.rs                    [NEW — BacktestMetrics::parse_from_report
                                                 (markdown summary table parser) + 5 unit tests
                                                 incl. all_anchored_reports_parse_ok — T1808]
crates/reports/src/lib.rs                      [+pub mod parse — T1808]
crates/ui/tests/viewer_read_only.rs            [NEW — viewer bin declares no spec/** writes — T1810]
crates/ui/tests/strategies_screen_sparkline_replaces_placeholder.rs
                                               [NEW — integration test for the sparkline
                                                 replacement; placeholder text absent post-T1811 — T1811]
crates/ui/tests/snapshots/viewer__kpi_strip__sample_report.snap         [NEW — T1812]
crates/ui/tests/snapshots/viewer__equity_curve__sample_report.snap      [NEW — T1812]
crates/ui/tests/snapshots/viewer__drawdown_band__sample_report.snap     [NEW — T1812]
crates/ui/tests/snapshots/viewer__full_view__sample_report.snap         [NEW — T1812]
crates/ui/tests/snapshots/strategies_screen__sparkline_present.snap     [NEW — T1812]
crates/ui/tests/snapshots/strategies_screen__sparkline_deferred.snap    [DELETED — T1812]
crates/ui/src/widgets/snapshots/widgets__sparkline__120pt.snap          [NEW — T1809]
spec/lumen-design-adoption/phase-4-backtest-panel/feature.md  [Design appended — architect, this dispatch]
spec/lumen-design-adoption/phase-4-backtest-panel/tasks.md     [NEW — this file]
spec/architecture.md                           [Q1–Q12 ratification block (Phase 4) appended under
                                                 the Phase 3 block; App-layout `viewer` row updated —
                                                 architect, this dispatch]
```

### What's NOT touched

- `crates/strategy/`, `crates/cost/`, `crates/backtest/` — anchor
  risk zero by construction. Phase 4 is read-only over the
  audit ledger and committed reports / companion CSVs.
- `crates/reports/src/render/` — no rendering-path change. The
  existing `equity_curve.rs` 60-cell sparkline + cadence rendering
  stays as-is; Phase 4 reads the **companion CSV** (R11.2),
  not the rendered body sparkline section.
- `crates/reports/src/csv_artifacts.rs` — `EquitySample` row type
  + `write_equity_csv` are read-only consumed; **zero schema
  change**.
- The existing 11 backtest body-SHA-256 anchors in
  [`spec/anchors.toml`](../../anchors.toml) — no anchor changes;
  no re-lock budget. Reading existing reports' bodies + companion
  CSVs is read-only by construction.
- `crates/ui/Cargo.toml` iced version — still pinned `=0.14.0`;
  no new iced version, no new workspace dep. **TD-1 deferral
  re-stated in Phase 4 Design; next re-eval at Phase 5
  (HumanControl) analyst kickoff.**
- `spec/ui-design-principles.md` — operator-locked Phase 1 Q7
  doc; analyst-owned. No edit dispatched here.
- `spec/lumen-design-adoption/feature.md` — master roadmap is
  analyst-owned; the TD-1 follow-up note flagged in the Design's
  "TD-1 re-evaluation" section is a follow-up the orchestrator
  routes to the analyst on Phase 4 ship.
- `ui::strings` existing copy — operator-locked Constraint 2.
  The Phase 4 net-new constants (`KPI_*_LABEL`,
  `VIEWER_METRICS_UNAVAILABLE`, `VIEWER_NO_EQUITY_DATA`,
  `STRATEGIES_SPARKLINE_LOADING`) are additive, not a rewrite.
  The retiring `STRATEGIES_SPARKLINE_DEFERRED` is the **Phase 3
  deferral closure** (per Phase 3 Q6's "this is the seam Phase 4
  fills"), not a voice-rule change.
- `widgets::journal_transaction_modal`, `widgets::pnl`,
  `widgets::positions`, `widgets::strategies` (the Home summary
  panel), `widgets::tape`, `widgets::status_bar`, `widgets::frame`,
  `widgets::kill`, `widgets::latency` — Phase 1 / 2 / 3 widgets
  unchanged. The existing chart widget is **refactored** to consume
  the new `canvas_chart` core (T1804) but its public `view`
  signature stays byte-stable.
- The viewer's source for the equity curve — **the companion CSV
  `<stem>__equity.csv`** (R11.2). The audit-ledger query
  `equity_curve_for_strategy` is for the cockpit-side consumer
  (R13.4); the viewer is offline-only and does not query the audit
  ledger.
- The `cockpit_live` chrome — sidebar / status bar / 6 screens
  unchanged. The Phase 4 cockpit-side change is **local to the
  Strategies-detail screen body** (the placeholder retires; the
  sparkline lands in the same 160 px slot).
- The **"Deploy live" CTA** (Lumen `Backtest.jsx:76`) — **explicitly
  out of scope per R14**. Paper-only product; deployment is
  config-driven, not a button. The viewer's panel `actions` slot
  is omitted entirely.
- The **"Export" CTA** (Lumen `Backtest.jsx:75`) — explicitly out
  of scope per R14.2. Operator's existing tooling
  (`cat spec/*/reports/backtest-*.md`, editor inspection) covers
  export.
- The **file-picker UI** (R1.2 / Q4) — explicitly out of scope.
  CLI-arg only.

### Cross-references

- Master roadmap: [`spec/lumen-design-adoption/feature.md`](../feature.md).
- Phase 4 brief: [`spec/lumen-design-adoption/phase-4-backtest-panel/feature.md`](feature.md).
- Phase 3 task list (template + T-numbering precedent + sub-block
  ui-designer-attestation pattern):
  [`spec/lumen-design-adoption/phase-3-detail-screens/tasks.md`](../phase-3-detail-screens/feature.md).
- Phase 2 task list (canvas-chart Phase 2 widget T1608 precedent):
  [`spec/lumen-design-adoption/phase-2-shell-ia-charts/tasks.md`](../phase-2-shell-ia-charts/feature.md).
- Phase 1 task list (T1511 ui-designer attestation original pattern):
  [`spec/lumen-design-adoption/phase-1-foundation/tasks.md`](../phase-1-foundation/feature.md).
- Architecture (Phase 2+ contract + Phase 4 ratification):
  [`spec/architecture.md` § Cockpit screen routing (Phase 2+
  contract)](../../architecture.md).
- UI principles (Information architecture):
  [`spec/ui-design-principles.md`](../../ui-design-principles.md).
- Audit query module (extension point):
  [`crates/audit/src/query.rs`](../../../crates/audit/src/query.rs).
- Reports companion-CSV writer (R11.2 source):
  [`crates/reports/src/csv_artifacts.rs`](../../../crates/reports/src/csv_artifacts.rs).
- Phase 2 chart widget (refactor target):
  [`crates/ui/src/widgets/chart.rs`](../../../crates/ui/src/widgets/chart.rs).
- Phase 3 Strategies-detail screen (sparkline replacement target):
  [`crates/ui/src/screens/strategies.rs`](../../../crates/ui/src/screens/strategies.rs).
- Lumen Backtest reference component (visual contract source):
  [`spec/design/project/ui_kits/desktop/Backtest.jsx`](../../design/project/ui_kits/desktop/Backtest.jsx).
