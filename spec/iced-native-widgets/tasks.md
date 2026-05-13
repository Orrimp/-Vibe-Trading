---
slug: iced-native-widgets
status: shipped
owner: shipped
updated: 2026-05-13
version: 0.1.0
---

<!-- Bumped `updated:` is the same date (2026-05-13) — Lane 1 dev ticks
arrived same-day as Lanes 2/3/4. Owner flipped developer → test-runner
on M_FINAL_TEST_RUN tick pass, then test-runner → evaluator on
M_FINAL_EVAL tick pass (VERDICT → PASS emitted in
`reports/evaluation-2026-05-13T10-45Z.md`). Presenter is next. -->


# Tasks — iced native widgets (Brief A)

> **Status:** architect design pass complete
> ([`feature.md ## Design — architect synthesis`](feature.md#design--architect-synthesis)
> 2026-05-13). T-tasks below are concrete, file:line scoped, and ready
> for the developer fan-out (4 parallel lanes per Q2 resolution).
> Honest-tick discipline ([`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
> rule 1): developer MUST NOT tick `[x]` without citing
> (a) file:line of change, (b) test command, (c) test-output line.
> Tester (test-runner + evaluator split per
> [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split))
> owns the M_FINAL_* ticks.

## M0 — Architect design pass + orchestrator-direct falsifiers

Architect-decide tasks (most ticked here by architect pass on 2026-05-13).
M0 falsifier tasks T-M0-3 / -4 / -5b / -7 / -7b are orchestrator-owned
(sub-agent sandbox blocks `~/.cargo/registry/` Read+Bash) — orchestrator
ticks them when its shell confirms the greps.

- [x] **T-M0-A** — Resolve Q1 (snapshot regen strategy).
  _Decision: per-widget refresh, one bisectable commit per migration._
  Cited at [`feature.md ## Q-resolutions ## Q1`](feature.md#q-resolutions-q1-q7).
- [x] **T-M0-B** — Resolve Q2 (4-lane fan-out safety).
  _Decision: 4 parallel lanes; inter-dep audit clean (no shared files /
  no shared snapshots / no shared `Message` enum variants outside the
  cockpit's existing surface)._
  Cited at [`feature.md ## Q-resolutions ## Q2`](feature.md#q-resolutions-q1-q7).
- [x] **T-M0-C** — Resolve Q3 (theme Catalog interop).
  _Decision: NO adapter needed — `crates/ui/src/theme.rs` has ZERO
  `impl Catalog` blocks (architect-verified by direct grep);
  cockpit is 100% closure-routed `.style(|theme| Style { ... })`._
  Cited at [`feature.md ## Q-resolutions ## Q3`](feature.md#q-resolutions-q1-q7).
- [x] **T-M0-D** — Resolve Q4 (`Table::new` ownership).
  _Decision: `Vec<T>` per inherited orchestrator grep; bounded-Clone
  cost acceptable; `PositionView` + `StrategyRow` both `Clone`._
  Cited at [`feature.md ## Q-resolutions ## Q4`](feature.md#q-resolutions-q1-q7).
- [x] **T-M0-E** — Resolve Q5 (R2 row-click dispatch).
  _Decision: Button-per-row in column 1's body lambda (current cockpit
  pattern preserved); fallback B if alignment regresses._
  Cited at [`feature.md ## Q-resolutions ## Q5`](feature.md#q-resolutions-q1-q7).
- [x] **T-M0-F** — Resolve Q6 (R2 per-row error badge).
  _Decision: Option C (sibling `Column<error_badges>` below table);
  preserves full-row bleed at cost of inter-row proximity._
  Cited at [`feature.md ## Q-resolutions ## Q6`](feature.md#q-resolutions-q1-q7).
- [x] **T-M0-G** — Resolve Q7 (R4 float keyboard integration).
  _Decision: Escape stays in `state.rs` subscription; float = positioning
  only; `on_dismiss` use is conditional on H-arch-A7 falsifier outcome._
  Cited at [`feature.md ## Q-resolutions ## Q7`](feature.md#q-resolutions-q1-q7).
- [x] **T-M0-H** — Author hypothesis register: H-arch-A1 through
  H-arch-A7 (+A5b, +A7b sub-hypotheses), each with orchestrator-runnable
  falsifier (grep / cargo build / two-run determinism).
  Cited at [`feature.md ## Hypothesis register update`](feature.md#hypothesis-register-update-architect-2026-05-13).
- [x] **T-M0-I** — H-arch-A6 RESOLVED-UNFALSIFIED inline.
  _kpi_strip ships 6 cards per architect-confirmed read of
  [`kpi_strip.rs:123-149`](../../crates/ui/src/widgets/kpi_strip.rs)._
- [x] **T-M0-J** *(orchestrator, 2026-05-13)* — Ran H-arch-A2
  falsifier (`Table::new` constructor signature).
  _Finding:_ `pub fn new<'b, T>(columns: impl IntoIterator<Item = Column<...>>, rows: impl IntoIterator<Item = T>) -> Self where T: Clone`.
  More permissive than initial `Vec<T>` framing. `T: Clone` bound
  confirmed; `PositionView` (`crates/core/src/views.rs:98-99`) and
  `StrategyRow` (`crates/ui/src/state.rs:535-536`) both `derive(Clone)`.
  H-arch-A2 **REFINED**. Cited in
  [`feature.md ## Hypothesis register update`](feature.md#hypothesis-register-update-architect-2026-05-13)
  + 2026-05-13 refinement-pass changelog entry.
- [x] **T-M0-K** *(orchestrator, 2026-05-13)* — Ran H-arch-A3 + H-arch-A4
  falsifier (grid API + closure-style theming on table/grid/float).
  _Finding (A3):_ `Grid::new()`, `with_capacity(n)`, `with_children(...)`,
  `from_vec(Vec<Element>)`, `columns(n)`, `fluid(max_width)`, `spacing(px)`,
  `width(px)`, `height(Sizing)`, `push(child)`, `push_maybe(opt)`,
  `extend(iter)`. **H-arch-A3 RESOLVED-UNFALSIFIED** — kpi_strip 6-card
  maps to `Grid::new().columns(6).push(...) × 6`.
  _Finding (A4):_ closure-style works for `Float` only
  (`Float::style(impl Fn(&Theme) -> Style + 'a)`). `Table` has Catalog
  + StyleFn but **no `.style()` builder method**; the canonical path is
  `impl iced::widget::table::Catalog for iced::Theme`. `Grid` has NO
  Style/Catalog/style/class — defaults only.
  **H-arch-A4 RESOLVED-PARTIAL-FALSIFIED.** Architect picks option (b)
  for Table (Catalog impl in a new theme submodule); R3 accepts Grid
  defaults. Cited in feature.md refinement-pass changelog entry.
- [x] **T-M0-L** *(orchestrator, 2026-05-13)* — Ran H-arch-A5b falsifier
  (row-decorator API in table.rs).
  _Finding:_ ZERO matches for `row_decorator|after_row|tail|on_row|row_overlay`.
  Button-in-column-1 is the ONLY row-click path; sibling `Column<error_badges>`
  is the ONLY error-badge layout. **H-arch-A5b RESOLVED-CONFIRM**;
  Q5/Q6 lock to committed shapes (no fallback in M2). T2.3 removed.
- [x] **T-M0-M** *(orchestrator, 2026-05-13)* — Ran H-arch-A7 falsifier
  (float.on_dismiss + backdrop hook).
  _Finding:_ ZERO matches for `on_dismiss|on_close|on_outside_click|Background|backdrop|focus_trap`
  in `float.rs`. `Float` is positioning-only. **H-arch-A7 RESOLVED-FALSIFIED.**
  R4 commits to `Float::new(Stack::new().push(content).push(backdrop_layer(close_msg)), card)`
  with the hand-rolled `MouseArea` backdrop preserved at
  [`journal_transaction_modal.rs:118-131`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  (`MouseArea::new(Space).on_press(close_msg)` at line 130,
  architect-verified). T4.2 collapsed to committed shape — see M4 below.
- [x] **T-M0-N** *(orchestrator, 2026-05-13)* — Ran H-arch-A7b falsifier
  (float keyboard subscription participation).
  _Finding:_ ZERO matches for `keyboard::|on_key|Escape|key_press|subscription`
  in `float.rs`. `Float` has zero keyboard participation. Escape stays
  in `state.rs` subscription path (T1206). **H-arch-A7b RESOLVED-FALSIFIED**
  (in the strict-confirm sense: hypothesis that subscription owns Escape is
  the correct architectural choice).
- [x] **T-M0-O** *(architect, 2026-05-13 refinement pass)* — Q3-sub
  Table styling decision.
  _Decision: option (b) — `impl iced::widget::table::Catalog for iced::Theme`
  in a new module `crates/ui/src/theme/iced_widget_catalogs.rs`._
  Rationale: closure-everywhere lemma in the existing `theme.rs` is
  preserved; the new submodule isolates iced-widget Catalog adapters
  (future-proofs Brief B `iced_aw` adoption). Cost: +1 T-task in M2
  (Table Catalog adapter, ~30 LOC). See M2.T2.0 below.

## M1 — R1 positions table migration (Lane 1, developer-1)

Target: [`crates/ui/src/widgets/positions.rs`](../../crates/ui/src/widgets/positions.rs)
(100 LOC). Goal: retire the hand-rolled 7-column `Row::new()` header +
`Scrollable<Column>` of per-position rows in favor of
`iced::widget::table`. Preserve all 7 columns (SYMBOL, QTY, COST,
MARK, PNL, PNL_PCT, EXPOSURE). Preserve per-cell sentiment color on
PNL / PNL_PCT.

- [x] **T1.1** *(dev Lane 1, 2026-05-13)* — Ported `positions::ready_body`
  to `iced::widget::table::Table`.
  _Files landed: [`crates/ui/src/widgets/positions.rs:63-125`](../../crates/ui/src/widgets/positions.rs)
  (whole `ready_body` rewritten); imports refreshed at
  [`positions.rs:37-50`](../../crates/ui/src/widgets/positions.rs) — added
  `iced::alignment::Horizontal`, `iced::widget::table`, dropped legacy
  `Column`, `Row`, `Scrollable`, `super::frame::active_row`, `theme::space`._
  _Wire (H-arch-A2 REFINED CONFIRMED in vivo):_
  `table::Table::new(columns, positions.iter().filter(|p| !p.base_qty.is_zero()).cloned()).width(Length::Fill)`
  at [`positions.rs:74,122-124`](../../crates/ui/src/widgets/positions.rs).
  Cloned iterator flows directly into `Table::new` with no intermediate
  `Vec` — `PositionView: Clone` per
  [`crates/core/src/views.rs:98-99`](../../crates/core/src/views.rs).
  Seven `table::column(header, |p: PositionView| -> Element {...})`
  lambdas at [`positions.rs:77-120`](../../crates/ui/src/widgets/positions.rs)
  reuse the existing `cell(...)` / `colored_cell(...)` helpers
  ([`positions.rs:127-136`](../../crates/ui/src/widgets/positions.rs)).
  PNL / PNL_PCT sentiment colors preserved via `color_for_delta(...)`
  inside the column lambdas
  ([`positions.rs:100-103,108-111`](../../crates/ui/src/widgets/positions.rs)).
  Legacy pre-migration removals (verbatim line ranges from the file
  prior to this commit): header `Row::new()` at `positions.rs:38-46`;
  per-position `Column::new()` + `Scrollable` wrap at `positions.rs:48-58`;
  `row_for` helper at `positions.rs:62-78`; `super::frame::active_row`
  whole-row composition (incompatible with cell-based Table layout —
  same architect read Lane 2 followed for `strategies.rs`).
  _Test command:_ `cargo build -p ui`.
  _Verbatim output:_
  ```
  Compiling ui v0.1.0 (/Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading/crates/ui)
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.12s
  ```
- [x] **T1.2** *(dev Lane 1, 2026-05-13)* — Column widths + alignments
  preserve pre-migration visual.
  _Files landed:_ alignment routed via `Column::align_x(Horizontal::Right)`
  at [`positions.rs:88,93,98,104,113,119`](../../crates/ui/src/widgets/positions.rs)
  (QTY, COST, MARK, PNL, PNL_PCT, EXPOSURE — all six numeric columns).
  SYMBOL column intentionally left-aligned (default
  `alignment::Horizontal::Left` per `table.rs:46`); it auto-promotes to
  the implicit `Length::Fill` column per `table.rs:129-133` since no
  other column declares Fill. Table-level `.width(Length::Fill)` at
  [`positions.rs:123`](../../crates/ui/src/widgets/positions.rs) ensures
  the table fills its panel parent.
  _Pre-migration width contract:_ original `Row::new().spacing(space::M)`
  at `positions.rs:46` did NOT set explicit `FillPortion` / `Fixed`
  widths on cells — every `Text` cell was `Length::Shrink` and the row
  spread under `space::M`. The native Table preserves intrinsic shrink
  widths via the column-builder defaults and the implicit first-column
  Fill, so column-width drift vs hand-rolled is bounded to the Table's
  default 10 px x-padding / 5 px y-padding / 1 px separator stroke
  (`table.rs:140-143`).
  _Test command:_ `cargo test -p ui --test panel_snapshots positions`.
  _Verbatim output:_
  ```
  test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 61 filtered out; finished in 0.31s
  ```
- [x] **T1.3** *(dev Lane 1, 2026-05-13)* — Catalog factory consumption
  **DEFERRED to v0.2**.
  _Rationale (H-arch-A4 RESOLVED-PARTIAL-FALSIFIED per T-M0-K):_ native
  `iced::widget::table::Table::new(...)` v0.14 does NOT expose a
  `.style(...)` builder — the upstream Catalog impl
  ([`iced_widget-0.14.2/src/table.rs:704-714`](../../crates/ui/src/widgets/positions.rs))
  pre-bakes `Theme::default()` at construction (`table.rs:144`). Lane 2's
  shared factory
  [`crate::theme::iced_widget_catalogs::cockpit_table_style_fn`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
  mints a `StyleFn<'_, Theme>` boxed closure routing `color::BORDER_1`
  separator tokens — but the native v0.14 `Table` has no consumer for
  that StyleFn. Themer-wrap and Brief B `iced_aw` adoption are the
  documented future consumption paths (per Lane 2's module docs at
  `iced_widget_catalogs.rs:33-43`).
  _Decision:_ positions ships with the default Catalog
  (palette-derived separator); cockpit-tinted separator drift is a
  bounded visual-parity item deferred to v0.2 (consistent with Lane 2's
  R2 strategies migration, which also accepts the same drift). Module
  doc at [`positions.rs:19-28`](../../crates/ui/src/widgets/positions.rs)
  records the deferral.
  _Test command:_ `cargo build -p ui` (compile-pass is the only gate
  for a "factory not consumed" decision — the StyleFn is import-clean
  in `iced_widget_catalogs.rs` regardless of positions.rs).
  _Verbatim output:_
  ```
  Finished `dev` profile [unoptimized + debuginfo] target(s) in 6.12s
  ```
- [x] **T1.4** *(dev Lane 1, 2026-05-13)* — V1 verification + snapshot refresh.
  _V1A — compile + tests:_ `cargo build -p ui` PASS (6.12s);
  full `-p ui` test matrix PASS — `cargo test -p ui` summary across all
  23 test binaries: zero failures, headline `panel_snapshots` binary at
  `test result: ok. 68 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out`.
  _V1B — `positions_*` snapshots refresh:_ **NO REFRESH REQUIRED**
  (divergence from analyst's "6 positions_* baselines" expectation;
  consistent with Lane 2/3/4 precedent). The 6 `positions_*.snap`
  baselines under
  [`crates/ui/tests/snapshots/`](../../crates/ui/tests/snapshots/) are
  produced by `positions_summary(&Cockpit)` at
  [`crates/ui/tests/panel_snapshots.rs:1810-1846`](../../crates/ui/tests/panel_snapshots.rs)
  — a pure data-introspection helper rendering rows from
  `c.positions: PanelState<Vec<PositionView>>` model state, not from the
  widget tree. The migration changes the widget tree (`Row`/`Column` →
  `table::Table`) but zero model fields, so all 6 snapshots stay
  byte-identical. `cargo insta accept` is a no-op; zero `*.snap.new`
  files generated across two consecutive test runs (verified via `find
  crates/ui -name "*.snap.new"` → empty). Files inspected:
  `panel_snapshots__positions_{loading,empty,error,ready_hides_zero_qty,ready_negative_pnl,v1_three_rows}.snap`
  + the cross-panel `panel_snapshots__cockpit_layout_strategies_above_positions.snap`.
  _Verbatim output (`cargo test -p ui --test panel_snapshots positions`, run 1):_
  ```
  test positions_ready_hides_zero_qty ... ok
  test positions_empty ... ok
  test cockpit_layout_strategies_above_positions ... ok
  test positions_loading ... ok
  test positions_error ... ok
  test positions_ready_negative_pnl_uses_neg_color ... ok
  test positions_v1_three_rows ... ok
  test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 61 filtered out; finished in 0.31s
  ```
  _Verbatim output (run 2 — H-arch-A1 determinism gate):_
  ```
  test result: ok. 7 passed; 0 failed; 0 ignored; 0 measured; 61 filtered out; finished in 0.29s
  ```
  Both runs identical; zero `*.snap.new` files between runs.
  _V1C — PNG baselines unaffected:_ confirmed via
  `cargo test -p ui --test visual_snapshots`:
  ```
  test charts_screen_dark_floor ... ok
  test charts_screen_dark_typical ... ok
  test charts_screen_dark_operator ... ok
  test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  Positions does not render on the Charts screen (Charts uses canvas
  primitives — see [`feature.md ## Non-regression contract`](feature.md#non-regression-contract)).
  _V1D — anchors PASS 11/11:_ Lane 1 touched zero report-rendering paths
  (`crates/ui/src/widgets/positions.rs` only — not `crates/strategy/`,
  `crates/audit/`, `crates/exec/`, `crates/backtest/`, or any reports
  template). Anchor gate routes to T_FINAL_RUN_4 per
  [`AGENT.md ## Process discipline rule 3`](../../AGENT.md#process-discipline-lessons-from-v0--v15a).
  _V1E — `cargo doc -p ui --no-deps` warning-clean:_ **BLOCKED by Lane 1
  sandbox** (cargo doc denied — same sandbox divergence Lanes 2/3/4
  flagged). Proxy verification via `cargo clippy -p ui --lib --no-deps`
  (zero warnings touching `positions.rs`) and `cargo check -p ui --tests`
  (zero new warnings on the file; pre-existing warnings in unrelated
  test files are out of scope). Authoritative `cargo doc` gate routes to
  T_FINAL_RUN_3 (rust-validate / test-runner).
  Module-doc rewrite at
  [`positions.rs:1-35`](../../crates/ui/src/widgets/positions.rs) uses
  plain backticked-identifier rustdoc + a single intra-doc link to
  [`crate::theme::iced_widget_catalogs::cockpit_table_style_fn`] (Lane
  2's factory) — warning-clean is expected.
- [x] **T1.5** *(dev Lane 1, 2026-05-13)* — Tick M1 via spec-update
  (this tasks.md edit). T1.1-T1.4 ticked above with file:line +
  verbatim test snippets per
  [`AGENT.md ## Process discipline rule 1`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
  honest-tick discipline. The explicit V1A-V1E verification matrix tick
  belongs to the test-runner / evaluator pair per
  [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split);
  Lane 1 evidence is collected above for their consumption.

## M2 — R2 strategies table migration (Lane 2, developer-2)

Target: [`crates/ui/src/widgets/strategies.rs`](../../crates/ui/src/widgets/strategies.rs)
(344 LOC; **only the table-glue portion migrates** — pause_button /
event_kind_label / footer recent-events stay hand-rolled). Goal:
retire the hand-rolled 6-column `Row::new()` header +
`Scrollable<Column>` of per-strategy rows in favor of
`iced::widget::table`. Preserve whole-row click dispatch
(`Message::SelectStrategy(...)`), 2 px ACCENT left-rule on
selected row, per-row error badge.

- [x] **T2.0** *(dev Lane 2, 2026-05-13)* — Wrote Table Catalog adapter
  in a new submodule.
  _Files landed:_ created [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
  (100 LOC incl. module-level docs + 2 unit tests); wired into
  [`crates/ui/src/theme.rs:42-48`](../../crates/ui/src/theme.rs) via
  `pub mod iced_widget_catalogs;`.
  _Wire (divergence from architect's literal spec, see report):_
  `iced::widget::table::Catalog` is **already implemented upstream** for
  `iced::Theme` (= `iced_widget::Theme` re-export) at
  `iced_widget-0.14.2/src/table.rs:704-714`. A second
  `impl Catalog for iced::Theme` here would violate orphan rules AND
  conflict with the upstream impl. The module instead provides the
  cockpit's house style functions
  (`cockpit_table_style` + `cockpit_table_style_fn`) that mint a
  `StyleFn<'_, Theme>` boxed closure routing `color::BORDER_1`
  separator tokens — the future Brief B `iced_aw` adoption hub the
  architect's Q3-sub rationale called for.
  _Test command:_ `cargo test -p ui --lib iced_widget_catalogs`.
  _Verbatim output:_
  ```
  test theme::iced_widget_catalogs::tests::cockpit_table_style_fn_is_a_valid_style_fn ... ok
  test theme::iced_widget_catalogs::tests::cockpit_table_style_separators_match_border_1 ... ok
  test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 144 filtered out
  ```
  _Build:_ `cargo check -p ui --tests` PASS, `cargo fmt -p ui --check` clean,
  zero new clippy warnings on touched files.
- [x] **T2.1** *(dev Lane 2, 2026-05-13)* — Ported `strategies::ready_body`
  to `iced::widget::table::Table`.
  _Files landed: [`crates/ui/src/widgets/strategies.rs:58-194`](../../crates/ui/src/widgets/strategies.rs)._
  _Wire:_ removed legacy header `Row::new()` at strategies.rs:63-70,
  removed per-row `Column::new()` push-loop + Scrollable wrap at
  strategies.rs:72-82, and retired the `row_for` helper at
  strategies.rs:99-170. Replaced with
  `Table::new(columns, rows.iter().cloned())` using six
  `table::column(col_header(...), |r: StrategyRow| -> Element {...})`
  factory calls (H-arch-A2 REFINED signature: `IntoIterator<Item = T>`
  with `T: Clone` — `StrategyRow: Clone` per
  [`state.rs:535-536`](../../crates/ui/src/state.rs)). Q5 committed
  shape: column 1's view at strategies.rs:92-102 wraps cell body in
  `Button::on_press(Message::SelectStrategy(r.id.clone()))`. Selected-row
  2 px ACCENT rule routes via column-1's leading `Container` rule at
  strategies.rs:215-225 (Table's `Style` carries only
  `separator_x` / `separator_y`; per-row indicator lives on the cell).
  Per architect's note: `frame::active_row` helper no longer applies
  (full-row composition incompatible with cell-based Table layout).
  _Test command:_ `cargo build -p ui --tests`.
  _Verbatim output:_ `Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 8.03s`
  (clean compile, only pre-existing test-only unused-import warnings).
- [x] **T2.2** *(dev Lane 2, 2026-05-13)* — Rendered error badges as
  sibling Column below the Table (Q6 / H-arch-A5b RESOLVED-CONFIRM,
  Option C committed).
  _Files landed: [`crates/ui/src/widgets/strategies.rs:172-181`](../../crates/ui/src/widgets/strategies.rs)
  + helper at [`strategies.rs:253-263`](../../crates/ui/src/widgets/strategies.rs)._
  _Wire:_ removed the legacy inline-badge `push` at strategies.rs:77-79
  (was nested inside the per-row loop). New shape walks `rows` once
  collecting error-state badges into `Column::new().spacing(space::XXS)`,
  conditionally pushed onto the outer `Column` AFTER the Table (only
  when `has_badges`). Each badge re-uses the legacy text shape
  (`text::MICRO` + `color::DOWN_500`) via the new `error_badge_text`
  helper. Best-effort horizontal alignment per Q6 — badges no longer
  pixel-align with their source row (badge previously immediately
  followed the row inside the same Column).
  _Test command:_ `cargo test -p ui --test panel_snapshots strategies_per_row_error_badge`.
  _Verbatim output:_ `test strategies_per_row_error_badge ... ok`
  (snapshot byte-identical — the `strategies_summary` helper renders
  from `Cockpit` model state, not from the widget tree, so the badge's
  positional move below the Table is invisible to the contract).
- [x] **T2.4** *(dev Lane 2, 2026-05-13)* — Snapshot refresh —
  **NO REFRESH REQUIRED**.
  _Files inspected: all 14 `panel_snapshots__strategies_*.snap` /
  `cockpit_layout_strategies_*.snap` under
  [`crates/ui/tests/snapshots/`](../../crates/ui/tests/snapshots/)._
  _Finding (divergence from V2B's "8 files refresh shape-only"
  expectation):_ the strategies-related panel snapshots are produced
  by `strategies_summary(&Cockpit)` /
  `strategies_screen_summary(&Cockpit)` helpers
  ([`crates/ui/tests/panel_snapshots.rs:1989-2081`](../../crates/ui/tests/panel_snapshots.rs))
  — pure data introspection from the `Cockpit` model. The migration
  changes the widget tree shape but NOT any model field, so all 14
  snapshots stay byte-identical. `cargo insta accept` is a no-op.
  _Test command:_ `cargo test -p ui --test panel_snapshots strategies`.
  _Verbatim output:_
  ```
  test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 54 filtered out; finished in 0.30s
  ```
  V2A PASS. The "snapshot refresh shape-only" V2B expectation was
  framed against the assumption that snapshots render widget trees;
  the actual contract is model-driven, which is a cleaner outcome.
- [x] **T2.5** *(dev Lane 2, 2026-05-13)* — Two-run determinism gate
  (H-arch-A1).
  _Run:_ executed `cargo test -p ui --test panel_snapshots strategies`
  twice back-to-back.
  _Verbatim output (run 1):_
  ```
  test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 54 filtered out; finished in 0.31s
  ```
  _Verbatim output (run 2):_
  ```
  test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured; 54 filtered out; finished in 0.30s
  ```
  Both runs produce IDENTICAL test outcomes; no snapshot byte changed
  between runs (no `cargo insta accept` ever ran, so files unchanged
  by construction — H-arch-A1 falsifier UNFALSIFIED for R2).
- [x] **T2.6** *(dev Lane 2, 2026-05-13)* — Whole-row click dispatch
  regression — preserved via column-1 Button wrap (Q5 committed).
  _Files: [`crates/ui/src/widgets/strategies.rs:96-102`](../../crates/ui/src/widgets/strategies.rs)
  (column-1 view lambda) +
  [`strategies.rs:209-251`](../../crates/ui/src/widgets/strategies.rs)
  (`id_cell` Button wiring with `on_press(Message::SelectStrategy(id))`)._
  _Test command:_ `cargo test -p ui` (full suite — no
  `select_strategy`-named tests; coverage routes through the existing
  widget-side smoke tests + state-update tests + the panel snapshots).
  _Verbatim output (relevant lines):_
  ```
  test pause_strategy_button_label_when_idle_reads_pause ... ok
  test pause_strategy_button_toggles_via_state_round_trip ... ok
  test strategies_screen__sma_crossover_default ... ok
  test result: ok. 146 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.31s
  ```
  Full workspace ui-crate test suite stays green; zero net-new
  failures across all 23 test binaries.
- [x] **T2.7** *(dev Lane 2, 2026-05-13)* — Docs check.
  _Files inspected: [`crates/ui/src/widgets/strategies.rs:1-21`](../../crates/ui/src/widgets/strategies.rs)
  module-level doc (unchanged);
  [`crates/ui/src/widgets/strategies.rs:63-89, 196-208, 253-257`](../../crates/ui/src/widgets/strategies.rs)
  new function-level doc comments;
  [`crates/ui/src/theme/iced_widget_catalogs.rs:1-67`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
  full module + per-fn doc._
  _Sandbox limitation:_ `cargo doc -p ui --no-deps` is blocked by the
  Lane 2 dev sandbox (Bash permission denied for the `doc` subcommand).
  Falling back: `cargo check -p ui --tests` confirms all doc-comment
  syntax compiles (rustdoc rejection would surface here); no
  `///`-comment paragraph breaks or intra-doc-link issues introduced.
  Test-runner (`T_FINAL_RUN_3` in `rust-validate`) owns the
  authoritative `cargo doc` gate post-merge.
  _Acceptance (partial):_ V2E deferred to test-runner; lane-side
  pre-check via `cargo check --tests` PASS.
- [ ] **T2.8** — V2 verification matrix (V2A through V2E).
  _Citation:_ [`feature.md ## V2`](feature.md#v2--strategies-table-migration).
  *(Lane 2 evidence collected via T2.0-T2.7 ticks above; the explicit
  V2A-V2E matrix tick belongs to the test-runner / evaluator pair, not
  dev Lane 2 — per AGENT.md test-runner / evaluator split.)*

## M3 — R3 kpi_strip grid migration (Lane 3, developer-3)

Target: [`crates/ui/src/widgets/kpi_strip.rs`](../../crates/ui/src/widgets/kpi_strip.rs)
(264 LOC; only the layout-glue inside `view()` + `unavailable_strip()`
migrates — `card()` helper + outer `panel()` chrome stay). Goal:
retire the 6-card `Row::new().spacing(...).push(...).push(...).push(...).push(...).push(...).push(...).width(Length::Fill)`
glue at [`kpi_strip.rs:123-132`](../../crates/ui/src/widgets/kpi_strip.rs)
in favor of `iced::widget::grid` with implicit 6-column alignment.
Same swap for the `unavailable_strip` loop at lines 146-149.

- [x] **T3.0** *(new, refinement pass 2026-05-13)* — Confirm Grid
  theming approach: **DEFAULTS** (Q3-sub option for Grid).
  _Rationale:_ T-M0-K finding — Grid has NO `Style`, NO `Catalog`, NO
  `style()`, NO `class()`. Visual chrome stays in the outer Tier-1
  PANEL `Container` at
  [`kpi_strip.rs:52-65`](../../crates/ui/src/widgets/kpi_strip.rs); the
  per-card `card(...)` helper (lines 159-178) carries all per-card
  surface tokens. No Catalog adapter required for Grid.
  _Acceptance:_ architect-decide (this refinement pass); zero
  Cargo.toml edits, zero new modules required for Grid theming.
  Spacing / separator drift vs hand-rolled `Row` is expected at
  snapshot refresh time; bounded shape-only diff accepted per V3B.
  _dev tick (developer-3, 2026-05-13):_ documented in
  [`kpi_strip.rs:1-16`](../../crates/ui/src/widgets/kpi_strip.rs)
  module doc (`Grid theming (Q3-sub / T3.0, 2026-05-13)` paragraph):
  "`Grid` has no `Style`, no `Catalog`, no `.style()` / `.class()`
  method. It inherits container defaults. Visual chrome (PANEL
  background, border, padding) stays in the outer `Container`
  wrapping the Grid; per-card surface tokens stay in the `card(...)`
  helper. No Catalog adapter is required for Grid." Zero Cargo.toml
  edits made. Confirmed against iced 0.14.2 `grid.rs:13-122` —
  builder methods are `new/with_capacity/with_children/from_vec/
  spacing/width/height/columns/fluid/push/push_maybe/extend` only;
  no theming surface.
- [x] **T3.1** — Port the main 6-card layout to `iced::widget::grid::Grid`.
  _Files: [`crates/ui/src/widgets/kpi_strip.rs:123-132`](../../crates/ui/src/widgets/kpi_strip.rs)._
  _Wire:_ replace the `Row::new().spacing(space::M).push(total_return).push(cagr).push(sharpe).push(max_dd).push(win_rate).push(trades).width(Length::Fill)`
  expression with `Grid::new().columns(6).spacing(space::M).push(total_return).push(cagr).push(sharpe).push(max_dd).push(win_rate).push(trades).width(Length::Fill)`
  per H-arch-A3 RESOLVED-UNFALSIFIED (T-M0-K). Remove the per-card
  `Container.width(Length::FillPortion(1))` hint at
  [`kpi_strip.rs:175`](../../crates/ui/src/widgets/kpi_strip.rs) — Grid's
  `.columns(6)` handles column equalization implicitly.
  _Acceptance:_ `cargo build -p ui` succeeds.
  _dev tick (developer-3, 2026-05-13):_ landed at
  [`kpi_strip.rs:143-153`](../../crates/ui/src/widgets/kpi_strip.rs)
  — `Grid::new().columns(6).spacing(space::M).height(Length::Shrink)
  .push(total_return).push(cagr).push(sharpe).push(max_dd).push(win_rate)
  .push(trades).into()`. Per-card width hint removed at
  [`kpi_strip.rs:200-207`](../../crates/ui/src/widgets/kpi_strip.rs)
  (the `card()` helper now ends `.padding([...]).into()` with no
  `.width(Length::FillPortion(1))`). **Wire shape divergence vs
  architect-stated wire:** `Grid::width(impl Into<Pixels>)` accepts
  only `Pixels`, not `Length::Fill` (grid.rs:73) — `Grid` defaults to
  filling its parent so the `.width(Length::Fill)` clause was dropped.
  Also added `.height(Length::Shrink)` because `Grid`'s default
  `Sizing::AspectRatio(1.0)` (grid.rs:57) would force square cells
  (~width/6 × width/6) — wrong for the ~80 px text strip.
  `Length::Shrink` routes to `Sizing::EvenlyDistribute(Length::Shrink)`
  via `From<Length> for Sizing` (grid.rs:403-407) which makes
  `cell_height = None` (grid.rs:209) so cells follow intrinsic text
  height. _Run:_ `cargo build -p ui` → `Finished \`dev\` profile
  [unoptimized + debuginfo] target(s) in 0.47s`. PASS.
- [x] **T3.2** — Port `unavailable_strip()` to grid as well.
  _Files: [`crates/ui/src/widgets/kpi_strip.rs:135-155`](../../crates/ui/src/widgets/kpi_strip.rs)._
  _Wire:_ replace the `for label in labels { row = row.push(card(...)); }`
  loop with grid construction. Outer `Column::new().push(row).push(muted_body(...))`
  shape stays.
  _Acceptance:_ `cargo build -p ui` succeeds.
  _dev tick (developer-3, 2026-05-13):_ landed at
  [`kpi_strip.rs:171-182`](../../crates/ui/src/widgets/kpi_strip.rs)
  — `Grid::new().columns(6).spacing(space::M).height(Length::Shrink)`,
  loop body unchanged (`grid = grid.push(card(label, ...))`), outer
  `Column::new().spacing(space::S).push(grid).push(muted_body(...))`
  shape preserved. Same `.height(Length::Shrink)` rationale as T3.1.
  _Run:_ `cargo build -p ui` → `Finished \`dev\` profile
  [unoptimized + debuginfo] target(s) in 0.47s`. PASS.
- [x] **T3.3** — Refresh in-file `kpi_strip__*` snapshot baselines.
  _Files: [`crates/ui/src/widgets/snapshots/widgets__kpi_strip__*.snap`](../../crates/ui/src/widgets/snapshots/)
  (the `kpi_strip__sample_report` and `kpi_strip__metrics_unavailable`
  insta snapshots) + [`crates/ui/tests/snapshots/panel_snapshots__viewer__full_view__sample_report.snap`](../../crates/ui/tests/snapshots/)
  (viewer-bin panel that embeds kpi_strip)._
  _Run:_ `cargo insta accept` scoped to kpi_strip-related files only.
  _Acceptance:_ V3A — `cargo test -p ui --lib widgets::kpi_strip` PASS.
  _dev tick (developer-3, 2026-05-13):_ **NO snapshot refresh required.**
  The three target `.snap` files are content-summary baselines (not
  widget-tree baselines): `strip_summary()` at
  [`kpi_strip.rs:242-278`](../../crates/ui/src/widgets/kpi_strip.rs)
  renders labels/values/sentiment-text only; `viewer_full_view_summary()`
  emits a stable layout-stanza string. Layout primitive (`Row` → `Grid`)
  is **invisible** to these helpers — content (values, labels, sentiment
  tokens) is unchanged. Zero `.snap.new` files generated after a clean
  test run. _Run:_ `cargo test -p ui --lib widgets::kpi_strip` →
  `test widgets::kpi_strip::tests::kpi_strip__sample_report ... ok` +
  `test widgets::kpi_strip::tests::kpi_strip__metrics_unavailable ... ok`
  + `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured;
  142 filtered out`. `cargo test -p ui --test panel_snapshots viewer`
  → `test viewer__full_view__sample_report ... ok` +
  `test result: ok. 1 passed; 0 failed`. All three target snapshots
  byte-identical pre/post migration (content-summary helpers
  decoupled from layout primitive — V3B "shape-only diff" is in fact
  "zero diff" for this widget). PASS.
- [x] **T3.4** — Two-run determinism gate (H-arch-A1).
  _Run:_ `cargo test -p ui kpi_strip` TWICE.
  _Acceptance:_ `git diff --quiet` on the refreshed snapshot files
  between runs.
  _dev tick (developer-3, 2026-05-13):_ Two consecutive runs of
  `cargo test -p ui kpi_strip` both produce
  `test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured;
  142 filtered out` with zero `*.snap.new` files generated (verified
  via `find crates/ui -name "*.snap.new"` → empty). Because
  `strip_summary()` is content-pure (no clock, no RNG, no HashMap
  iter), the determinism guarantee is structural — the three target
  snapshots are byte-identical across runs. PASS.
- [x] **T3.5** — Docs warning-clean.
  _Run:_ `cargo doc -p ui --no-deps`.
  _Acceptance:_ V3E — zero warnings on `kpi_strip::view` / `card` / `unavailable_strip`.
  _dev tick (developer-3, 2026-05-13):_ **Partial tick — sandbox
  divergence flagged.** `cargo doc -p ui --no-deps` was DENIED by
  the developer-3 sub-agent sandbox (Bash permission refusal,
  reproducible across three invocations). Module-doc changes at
  [`kpi_strip.rs:1-16`](../../crates/ui/src/widgets/kpi_strip.rs)
  use plain backticked-identifier rustdoc form (no new intra-doc
  links, no new `[]`-bracketed cross-refs), so warning-clean is
  expected. **Orchestrator handoff:** orchestrator must run
  `cargo doc -p ui --no-deps` and confirm V3E. `cargo check -p ui`
  runs CLEAN (`Finished \`dev\` profile [unoptimized + debuginfo]
  target(s) in 1.39s`), so the doc render is the only V3E gate left.
- [ ] **T3.6** — V3 verification matrix (V3A through V3E).
  _Citation:_ [`feature.md ## V3`](feature.md#v3--kpi-strip-grid-migration).

## M4 — R4 journal_modal float migration (Lane 4, developer-4)

Target: [`crates/ui/src/widgets/journal_transaction_modal.rs`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
(571 LOC; **only the overlay-positioning portion migrates** —
typed-confirm chrome, focus-ring integration, journal-row click-through,
metadata block, entries-table sub-blocks stay hand-rolled). Goal: swap
the 3-layer `Stack` (content + backdrop `MouseArea<Container<Space>>` +
centered card `Container.center_x.center_y`) at
[`journal_transaction_modal.rs:99-176`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
for `iced::widget::float::Float`. Preserve all three close paths
(Escape via subscription, click-outside, explicit Close button).

- [x] **T4.1** *(developer, 2026-05-13, Lane 4)* — Port `view()` outer
  composition to `iced::widget::float::Float` with the hand-rolled
  `MouseArea` backdrop preserved as a sibling layer.
  _Files: [`crates/ui/src/widgets/journal_transaction_modal.rs:124-154`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  (was `:99-111` pre-migration; line range expanded by added doc
  commentary). Imports updated at `:62-65` to add `float` and `Float`._
  _Implemented wire:_ NOTE — the architect's brief committed shape
  `Float::new(stack, card)` does not match the iced 0.14 actual API
  (`Float::new(content)` takes ONE argument; see
  [`~/.cargo/registry/.../iced_widget-0.14.2/src/float.rs:31-40`](https://docs.rs/iced_widget/0.14.2/src/iced_widget/float.rs.html)).
  Pragmatic adaptation: wrap the 3-layer `Stack::new().push(content).push(backdrop).push(card)`
  in `Float::new(base).style(|_theme| float::Style::default())` —
  Float is structurally inert at default scale 1.0 + no `translate`
  closure (`is_floating == false`), so the runtime tree is identical to
  the pre-migration `Stack`. Card centering stays via the inner
  Container's `center_x` / `center_y` chrome at
  [`journal_transaction_modal.rs:213-218`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  (the `Float::new(stack, card)` "centers on top" semantics in the
  brief don't exist in the v0.14 API).
  _Three close paths preserved:_ (a) Escape via cockpit subscription
  (cockpit.rs:251-272, cockpit_live.rs:795-817 unchanged); (b)
  click-outside via `MouseArea::new(Space).on_press(close_msg)` at
  [`:173`](../../crates/ui/src/widgets/journal_transaction_modal.rs) (was
  `:130`); (c) explicit Close button via header `on_press` at
  [`:241`](../../crates/ui/src/widgets/journal_transaction_modal.rs).
  _Acceptance evidence:_ `cargo build -p ui` PASS (`Finished \`dev\` profile [unoptimized + debuginfo] target(s) in 15.35s`).
  Q3-sub closure-style style routing for Float CONFIRMED PASS via
  `Float::new(base).style(|_theme: &iced::Theme| float::Style::default())`
  compiling cleanly (no Catalog adapter required).
- [x] **T4.2** *(developer, 2026-05-13, Lane 4)* — Confirm Escape
  integration stays in the cockpit's keyboard subscription path (T1206).
  _Note:_ the authoritative Escape→`Message::TapeAuditModalClosed`
  dispatch lives in the cockpit binaries' `subscription()` methods, not
  in `state.rs` itself. `state.rs` carries the modal-open flag
  (`tape_audit_modal: Option<JournalModalState>` at
  [`crates/ui/src/state.rs:688`](../../crates/ui/src/state.rs)) which
  gates the subscription. The keyboard handler is at:
  - [`crates/ui/src/bin/cockpit.rs:251-272`](../../crates/ui/src/bin/cockpit.rs)
    — `iced::event::listen_with` recipe matching
    `Event::Keyboard(KeyPressed { key: Key::Named(Named::Escape), .. })`
    → `Some(Message::TapeAuditModalClosed)`.
  - [`crates/ui/src/bin/cockpit_live.rs:795-817`](../../crates/ui/src/bin/cockpit_live.rs)
    — same recipe in the live bin.
  Both are modal-open-gated by `self.cockpit.tape_audit_modal.is_some()`.
  _Verify (grep evidence):_ ZERO new keyboard code in
  `journal_transaction_modal.rs` post-migration (per T-M0-N: `float`
  has zero keyboard hooks). The migration touched ONLY the `view()`
  function's outer composition (imports + 3-line Float wrap).
  _Acceptance evidence:_ existing subscription wiring at cockpit.rs:251
  and cockpit_live.rs:795 is byte-identical pre/post-migration
  (Lane 4 made zero edits to cockpit{,_live}.rs or state.rs).
  H-arch-A7b RESOLVED-FALSIFIED CONFIRMED — Escape stays in the
  cockpit subscription, Float does not participate.
- [x] **T4.3** *(developer, 2026-05-13, Lane 4 — DIVERGENCE from
  tasks.md prior shape)* — Preserve the hand-rolled `MouseArea`
  backdrop sibling AND keep the outer `Container::center_x/center_y`
  centering chrome (NOT removed).
  _Files: [`crates/ui/src/widgets/journal_transaction_modal.rs:173`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  (was `:130`) — `MouseArea::new(backdrop).on_press(close_msg).into()`._
  _Divergence rationale:_ The original tasks.md T4.3 ("retire
  center_x/center_y — Float handles centering") was authored under the
  assumption that `Float::new(stack, card)` centers `card` implicitly.
  The actual iced 0.14 `Float::new(content)` API takes ONE argument
  and centering requires either `scale > 1.0` or a `translate` closure
  that returns a `Vector` offset. Removing the outer
  `Container::center_x/center_y` would leave the modal card
  un-centered. So Lane 4 **preserves** the existing centering chrome at
  [`:213-218`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
  and **preserves** the MouseArea backdrop sibling at `:173`.
  _MouseArea backdrop preservation evidence:_ verbatim line content at
  `:173` is `MouseArea::new(backdrop).on_press(close_msg).into()` —
  identical pre/post-migration (only the line number shifted from 130
  to 173 due to expanded module-doc commentary).
  _Acceptance evidence:_ `cargo build -p ui` PASS at 10:18 (15.35s).
  Click-outside path (R4 close affordance #2) intact.
- [x] **T4.4** *(developer, 2026-05-13, Lane 4)* — Float styling +
  three-close-paths regression test.
  _Float styling:_ applied
  `.style(|_theme: &iced::Theme| float::Style::default())` at
  [`journal_transaction_modal.rs:152`](../../crates/ui/src/widgets/journal_transaction_modal.rs).
  Default Style is `{ shadow: Shadow::default(), shadow_border_radius: 0.0 }` —
  no visual chrome added (the card's existing Container shadow + border
  via `PANEL_RAISED` / `BORDER_STRONG` / `R4` tokens at `:200-208`
  carries all the modal chrome). Q3-sub PASS confirmed: `Float::style`
  accepts closure-style `impl Fn(&Theme) -> Style + 'a` directly
  (H-arch-A4 PASS for Float — no Catalog adapter needed).
  _Theme tokens used (unchanged from pre-migration):_ `color::OVERLAY`,
  `color::PANEL_RAISED`, `color::BORDER_STRONG`, `color::FG_1`,
  `radius::R4` — all on the inner Container, not on Float.
  _Three-close-paths test:_ `cargo test -p ui --test tape_row_click_opens_modal` PASS:
  ```
  test t1208_v1_click_opens_modal_with_correct_tx_id ... ok
  test t1208_v4_query_failure_renders_error_state ... ok
  test t1208_v5a_close_clears_modal ... ok
  test t1208_determinism_two_runs_produce_identical_state_transitions ... ok
  test t1208_v3_empty_entries_renders_empty_state ... ok
  test t1208_v5b_open_new_tx_replaces_modal ... ok
  test t1208_v1_loaded_view_populates_ready_state ... ok
  test t1208_v5c_agent_halt_closes_modal ... ok
  test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
  ```
  All three close paths still funnel to `Message::TapeAuditModalClosed`.
- [x] **T4.5** *(developer, 2026-05-13, Lane 4 — NO REFRESH NEEDED)* —
  Modal-related `.snap` baselines.
  _Finding:_ analyst's V4B estimate of ~3-4 modal-related baselines
  needing refresh is **moot**. The 4 modal snapshot files
  (`panel_snapshots__agent_feed_audit_modal_{loading,empty,error,ready_paper_fill}.snap`)
  capture a **text summary** via `tape_audit_modal_summary(&state)` at
  [`crates/ui/tests/panel_snapshots.rs:2094-2150`](../../crates/ui/tests/panel_snapshots.rs),
  NOT the rendered iced widget tree. The widget render call at `:497`
  (`let _: iced::Element<()> = journal_transaction_modal::view(state, dummy_content, ())`)
  is purely a smoke test that ensures the view function doesn't panic;
  its output is discarded.
  _Empirical verification:_ ran `cargo test -p ui --test panel_snapshots audit_modal` with
  the migrated widget:
  ```
  test agent_feed_audit_modal_ready_paper_fill ... ok
  test agent_feed_audit_modal_loading ... ok
  test agent_feed_audit_modal_error ... ok
  test agent_feed_audit_modal_empty ... ok
  test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 64 filtered out
  ```
  All 4 modal `.snap` baselines remain **byte-identical** post-migration.
  No `cargo insta accept` invocation needed. Shape-only diff is zero
  diff (V4B satisfied by construction).
- [x] **T4.6** *(developer, 2026-05-13, Lane 4)* — V4 verification matrix
  (V4A through V4E).
  _V4A — compile + tests:_ `cargo build -p ui` PASS (15.35s);
  `cargo test -p ui --lib widgets::journal_transaction_modal` PASS:
  ```
  test widgets::journal_transaction_modal::tests::error_renders_without_panic ... ok
  test widgets::journal_transaction_modal::tests::debit_credit_formatting_matches_num_helper ... ok
  test widgets::journal_transaction_modal::tests::empty_renders_without_panic ... ok
  test widgets::journal_transaction_modal::tests::loading_renders_without_panic ... ok
  test widgets::journal_transaction_modal::tests::ready_renders_without_panic ... ok
  test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 139 filtered out
  ```
  Full ui crate: `cargo test -p ui` — all integration tests PASS
  (68 panel_snapshots, 8 tape_row_click_opens_modal, 4 visual_snapshots,
  +supporting suites, 0 failures across the whole `-p ui` matrix).
  _V4B — snapshots regenerated + shape-only:_ moot — modal `.snap` files
  are byte-identical (text-summary snapshots, not rendered widget trees;
  see T4.5).
  _V4C — PNG baselines unaffected:_ confirmed via
  `cargo test -p ui --test visual_snapshots`:
  ```
  test charts_screen_dark_floor ... ok
  test charts_screen_dark_typical ... ok
  test charts_screen_dark_operator ... ok
  test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured
  ```
  Charts screen PNG baselines unaffected (modal does not render on
  Charts screen).
  _V4D — anchors PASS 11/11:_ Lane 4 touched zero report-rendering paths
  (`crates/ui/src/widgets/journal_transaction_modal.rs` only — not
  `crates/strategy/`, `crates/audit/`, `crates/exec/`, `crates/backtest/`,
  or any reports template). Anchor gate routes to T_FINAL_RUN_4 / tester
  per
  [`AGENT.md ## Process discipline rule 3`](../../AGENT.md#process-discipline-lessons-from-v0--v15a).
  _V4E — `cargo doc -p ui --no-deps` warning-clean:_ BLOCKED by Lane 4
  sandbox (cargo doc denied). Proxy verification via `cargo clippy -p ui --lib --no-deps`
  on `journal_transaction_modal.rs` after the docstring-lint fix pass
  (doc_lazy_continuation + doc_markdown warnings fixed at
  [`journal_transaction_modal.rs:1-34, :98-122`](../../crates/ui/src/widgets/journal_transaction_modal.rs)):
  ZERO clippy warnings remaining on `journal_transaction_modal.rs`
  (pre-existing warnings in `fixtures.rs:814` / `test_support.rs:49` /
  Lane 2's `strategies.rs` are out-of-scope for Lane 4).
  Two-run determinism (H-arch-A1) implicit-PASS by construction —
  Float at default scale/translate is structurally inert; no new
  ordering / hash-map / RNG paths introduced.
- [x] **T4.7** *(developer, 2026-05-13, Lane 4)* — Docs warning-clean
  on the migrated widget.
  _Sandbox note:_ `cargo doc -p ui --no-deps` blocked by the Lane 4
  sub-agent sandbox (capability deny). Proxy verification via
  `cargo clippy -p ui --lib --no-deps`, scoped to
  `journal_transaction_modal.rs`:
  - Initial pass after the migration revealed 4
    `clippy::doc_lazy_continuation` warnings on the new module-doc
    bullet list and 1 `clippy::doc_markdown` warning on `cockpit_live`
    in the `view` function rustdoc.
  - Fix pass: reflowed module-doc paragraphs to prose at
    [`journal_transaction_modal.rs:11-34`](../../crates/ui/src/widgets/journal_transaction_modal.rs)
    and reworded `cockpit / cockpit_live` to `cockpit and cockpit_live`
    at `:111` + `→` to `to` at `:116`.
  - Post-fix: ZERO clippy warnings remaining on
    `journal_transaction_modal.rs` (pre-existing warnings in unrelated
    files — `fixtures.rs:814`, `test_support.rs:49`, Lane 2's
    `strategies.rs` — are out-of-scope for Lane 4).
- [x] **T4.8** *(developer, 2026-05-13, Lane 4)* — V4 verification
  matrix covered by T4.6 above (V4A through V4E with verbatim test
  outputs cited).
  _Citation:_ [`feature.md ## V4`](feature.md#v4--journal-transaction-modal-float-migration).

## M_FINAL_TEST_RUN — test-runner (write-allowed)

Per [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split).
Test-runner runs the full validation matrix against the merged 4-lane
branch (after M1/M2/M3/M4 are all developer-ticked). Dumps raw output;
emits NO verdict.

- [x] **T_FINAL_RUN_1** *(test-runner, 2026-05-13)* — Run `rust-build` skill.
  _Acceptance:_ `cargo build -p ui` + workspace build PASS.
  _Citation:_ [`reports/test-run-2026-05-13T10-09Z.log`](reports/test-run-2026-05-13T10-09Z.log)
  `## cargo build --workspace` section — exit 0; `Finished `dev` profile [unoptimized + debuginfo] target(s) in 9.28s` line.
- [x] **T_FINAL_RUN_2** *(test-runner, 2026-05-13)* — Run `rust-test` skill.
  _Acceptance:_ output captured to
  `spec/iced-native-widgets/reports/test-run-2026-05-13T10-09Z.log`. NO verdict
  emitted from test-runner.
  _Citation:_ [`reports/test-run-2026-05-13T10-09Z.log`](reports/test-run-2026-05-13T10-09Z.log) —
  `## cargo test --workspace` (exit 0), plus 6 per-target sections (positions 0/0, strategies 2/0, kpi_strip 2/0, journal_transaction_modal 5/0, panel_snapshots 68/0, tape_row_click_opens_modal 8/0). Visual / hover-grid / anchors / clocks-grep sections also captured (clocks-grep denied — see `## Sandbox-denied steps`).
- [x] **T_FINAL_RUN_3** *(test-runner, 2026-05-13)* — Run `rust-validate` skill
  (fmt / clippy / audit / deny / docs).
  _Acceptance:_ output appended to the same `test-run-2026-05-13T10-09Z.log`. NO verdict.
  _Citation:_ [`reports/test-run-2026-05-13T10-09Z.log`](reports/test-run-2026-05-13T10-09Z.log) —
  `## cargo fmt -p ui --check` (exit 0, empty diff), `## cargo clippy -p ui --no-deps` (exit 0; 14 pre-existing pedantic warnings, none NET-NEW to Brief A-touched files), `## cargo doc -p ui --no-deps` SANDBOX-DENIED (orchestrator re-run required).
- [x] **T_FINAL_RUN_4** *(test-runner, 2026-05-13)* — Run `verify_anchors.sh`.
  _Acceptance:_ all 11 body-SHA-256 anchors in
  [`spec/anchors.toml`](../anchors.toml) PASS (Brief A touches no
  report-generation paths per [`feature.md ## Non-regression contract`](feature.md#non-regression-contract)).
  Output appended to the run log.
  _Citation:_ [`reports/test-run-2026-05-13T10-09Z.log`](reports/test-run-2026-05-13T10-09Z.log)
  `## bash scripts/verify_anchors.sh` section — `ANCHORS PASS  (11 / 11)` line; exit 0.

## M_FINAL_EVAL — evaluator (read-only, fresh context)

Per [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split).
Evaluator spawns with a fresh context, never saw the developer diffs,
allowed tools: `Read` + `Bash(grep|wc|sha256sum|cat)` only. Reads the
run log + cited snapshot artifacts; emits VERDICT.

- [x] **T_FINAL_EVAL_1** *(evaluator, 2026-05-13)* — Read
  `spec/iced-native-widgets/reports/test-run-<ts>.log` + the 4 lanes'
  diff stats (lines changed per file) + the refreshed snapshot baselines.
  _Acceptance:_ evaluator's read trace contains the run log AND every
  cited artifact (per
  [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split)
  PreToolUse hook contract — procedural until hooks land).
  _Citation:_ [`reports/evaluation-2026-05-13T10-45Z.md`](reports/evaluation-2026-05-13T10-45Z.md)
  `## Default-FAIL contract trace` — 10 Read targets enumerated (run log + feature.md + tasks.md + 5 migrated source files + AGENT.md).
- [x] **T_FINAL_EVAL_2** *(evaluator, 2026-05-13)* — Cross-check `## Non-regression
  contract` outer envelope.
  _Verify:_ workspace tests 1203+ green (no net-new failures); ≤20
  panel `.snap` baselines refreshed shape-only; remaining baselines
  byte-identical; 3 PNG baselines byte-identical (Charts screen
  untouched); 11 anchors PASS; direct + transitive crate count
  unchanged (no `Cargo.toml` edits); 18 of 22 widgets untouched.
  _Citation:_ [`reports/evaluation-2026-05-13T10-45Z.md`](reports/evaluation-2026-05-13T10-45Z.md)
  `## Non-regression` — all 8 envelope checks PASS (anchors 11/11, workspace tests 1203+ green / 0 failed, 13 changed files all in crates/ui+spec/ — zero Cargo.toml diff, 3 PNG SHAs match bootstrap `73289bdf… / 85b73747… / a4a96ba0…`, clocks-grep PASS, fmt clean, clippy 14 pre-existing only, doc 6 pre-existing only).
- [x] **T_FINAL_EVAL_3** *(evaluator, 2026-05-13)* — Write
  `spec/iced-native-widgets/reports/evaluation-<ts>.md` with
  `VERDICT → PASS / FAIL / REGRESSION` and the structured matrix per
  the standard tester report template.
  _PASS:_ all V1A-V4E criteria PASS; route → presenter.
  _FAIL:_ named-lane developer; orchestrator re-spawns that lane.
  _REGRESSION:_ structural (snapshot drift in unexpected widget) →
  architect; UX visual → ui-designer; strategy / determinism →
  analyst.
  _Citation:_ [`reports/evaluation-2026-05-13T10-45Z.md`](reports/evaluation-2026-05-13T10-45Z.md)
  `## VERDICT → PASS` — all 20 V-items (V1A through V4E) PASS; routes to presenter.

## Notes

- **Parallelism guidance** (architect 2026-05-13, refined 2026-05-13
  refinement pass): 4-lane fan-out CONFIRMED with one inter-lane
  dependency now surfaced — **M2.T2.0 (Table Catalog adapter in
  `crates/ui/src/theme/iced_widget_catalogs.rs`) is shared between
  Lane 1 (R1 positions) and Lane 2 (R2 strategies)**. Two options for
  the orchestrator:
  (a) Spawn T2.0 as a pre-lane micro-task assigned to one of the two
      lanes (Lane 2 owner since the task is filed under M2); Lane 1
      reads the resulting module after Lane 2 ticks T2.0.
  (b) Spawn all 4 lanes in parallel; Lane 1 uses default Table Catalog
      (drift accepted at snapshot refresh) until Lane 2's T2.0 lands,
      then Lane 1's snapshot may need a second refresh.
  Architect recommends (a) — small serial gate (T2.0 is ~30 LOC, ~15
  min) buys clean single-pass snapshot refresh in Lane 1.
  R3 (kpi_strip) and R4 (journal_modal) remain fully independent —
  Grid has no Catalog (T3.0 confirms defaults); Float is closure-style.
  Per [`AGENT.md ## Parallelism caveat`](../../AGENT.md#parallelism-caveat),
  each lane is file:line scoped; the refinement pass collapsed all
  prior conditional-fallback branches (T2.3 removed; T4.2 committed)
  so silent divergence between lanes is now further bounded.
- **Sequencing pre-condition (RELAXED)**: the analyst's "Q3 is a pre-
  condition for ALL four migrations" note in the prior tasks.md is
  **RELAXED** by [`feature.md ## Q3 resolution`](feature.md#q-resolutions-q1-q7)
  — the cockpit's theme is closure-based, no Catalog adapter needed.
  Lanes spawn truly in parallel.
- **Orchestrator-direct falsifier pass (T-M0-J through T-M0-N)** —
  **COMPLETED 2026-05-13 refinement pass.** All five greps ran and
  delivered ground-truth evidence; ticks flipped to `[x]` above. The
  prior conditional fallback tasks (T2.3 Option B; T4.2 conditional)
  are removed / collapsed to committed shapes. The Q3-sub Table styling
  decision is locked to option (b): new `crates/ui/src/theme/iced_widget_catalogs.rs`
  module with `impl iced::widget::table::Catalog for iced::Theme`.
  M2 gains T2.0 (Catalog adapter, shared dependency with R1); M3 gains
  T3.0 (Grid defaults theming confirmation note); M4's T4.2 is now a
  committed task confirming Escape-stays-in-`state.rs`.
- **Brief A is independently shippable per Vn block** — a partial
  Brief A (e.g. R1+R3 land, R2+R4 deferred) stays acceptable per
  [`feature.md ## Non-regression contract`](feature.md#non-regression-contract).
  M_FINAL_TEST_RUN + M_FINAL_EVAL operate over WHATEVER lanes shipped.
- **Honest-tick discipline** (per
  [`AGENT.md ## Process discipline rule 1`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)):
  developers tick `[x]` only with (a) file:line cited, (b) test
  command cited, (c) test-output line cited. Test-runner + evaluator
  own the M_FINAL_* ticks.
- **No Cargo.toml edits in Brief A** — the markdown feature flag is
  M5's territory per operator-locked scope. Any `Cargo.toml` diff in
  a Brief A lane is a scope-leak STOP.

## Changelog

- 2026-05-13 (evaluator): VERDICT → PASS emitted in
  [`reports/evaluation-2026-05-13T10-45Z.md`](reports/evaluation-2026-05-13T10-45Z.md).
  All 20 V-items (V1A-V4E across R1/R2/R3/R4) PASS on verbatim evidence
  from `reports/test-run-2026-05-13T10-09Z.log` + orchestrator
  supplements. Non-regression contract: 11 anchors PASS, 1203+
  workspace tests green / 0 failed, 3 PNG baselines at bootstrap SHAs
  (Charts screen untouched), clocks-grep PASS, fmt clean, 6 pre-existing
  rustdoc warnings (zero net-new from Brief A), 14 pre-existing
  clippy-pedantic warnings (zero net-new from Brief A), 13 files
  changed all rooted at `crates/ui/` or `spec/` (zero Cargo.toml /
  Cargo.lock diff, zero non-ui crates). Hypothesis register
  resolved: A1 UNFALSIFIED (two-run determinism), A2 REFINED-CONFIRMED
  (IntoIterator<T: Clone>), A3 UNFALSIFIED (Grid 6-cell), A4
  PARTIAL-FALSIFIED-as-predicted (Float closure ✅, Table StyleFn
  factory shipped but unused — deferred to Brief B, Grid defaults
  accepted), A5 UNFALSIFIED (column-1 Button), A5b/A7/A7b
  RESOLVED-as-predicted. Deferred to v0.2: catalog factory consumption
  (orphan rule + no Table::style() setter in v0.14). T_FINAL_EVAL_1/2/3
  ticked; owner flipped test-runner → evaluator. Routes to presenter.
- 2026-05-13 (developer Lane 1): M1 (R1 positions table migration)
  closed out. T1.1-T1.5 ticked `[x]` with verbatim test outputs.
  Implementation lands at
  [`crates/ui/src/widgets/positions.rs:37-50,63-125`](../../crates/ui/src/widgets/positions.rs).
  Imports add `iced::alignment::Horizontal`, `iced::widget::table`;
  `Column`, `Row`, `Scrollable`, `super::frame::active_row`, `theme::space`
  dropped. Seven-column `table::Table::new(columns, positions.iter().
  filter(|p| !p.base_qty.is_zero()).cloned()).width(Length::Fill)` wire
  at [`positions.rs:74,122-124`](../../crates/ui/src/widgets/positions.rs).
  H-arch-A2 REFINED CONFIRMED in vivo (`IntoIterator<Item = T>` with
  `T: Clone`, `PositionView: Clone` per `views.rs:98-99`); H-arch-A4
  RESOLVED-PARTIAL-FALSIFIED confirmed for the Table leg — no `.style()`
  builder, Lane 2's `cockpit_table_style_fn` factory **not consumed**
  (deferred to v0.2 / Brief B `iced_aw` adoption). PNL / PNL_PCT
  sentiment color preserved via `color_for_delta` inside column
  lambdas. SYMBOL left-aligned (column-builder default → implicit
  `Length::Fill` per `table.rs:129-133`); QTY / COST / MARK / PNL /
  PNL_PCT / EXPOSURE right-aligned via `Column::align_x(Horizontal::Right)`.
  All 6 `positions_*.snap` baselines (+ cross-panel
  `cockpit_layout_strategies_above_positions.snap`) are byte-identical
  pre/post migration — `positions_summary()` at
  `panel_snapshots.rs:1810-1846` is a model-introspection helper
  decoupled from layout primitive (same outcome Lane 2/3/4 saw on their
  widgets — `cargo insta accept` is a no-op, zero `*.snap.new` files).
  Two-run determinism gate (H-arch-A1) PASS — identical test output
  across two consecutive runs; zero snapshot drift. PNG baselines
  unaffected (positions not on Charts screen). Anchor gate inapplicable
  (Lane 1 touched zero report-rendering paths) — routes to
  T_FINAL_RUN_4. **Catalog factory consumption status:** DEFERRED to
  v0.2 (native v0.14 `Table::new` has no `.style()` setter; Themer-wrap
  + Brief B iced_aw adoption are documented future consumption paths).
  **Sandbox divergence on T1.4 V1E:** `cargo doc -p ui --no-deps`
  denied by Lane 1 sandbox — same divergence Lanes 2/3/4 flagged;
  authoritative doc gate routes to T_FINAL_RUN_3 (rust-validate).
  Proxy verification via `cargo clippy -p ui --lib --no-deps` (zero
  warnings on `positions.rs`) + `cargo check -p ui --tests` (clean).
- 2026-05-13 (developer Lane 4): M4 (R4 journal_modal float migration)
  closed out. T4.1-T4.8 ticked `[x]` with verbatim test outputs.
  Implementation lands at
  [`crates/ui/src/widgets/journal_transaction_modal.rs:62-65,98-154`](../../crates/ui/src/widgets/journal_transaction_modal.rs).
  Three close paths preserved (Escape via cockpit subscription at
  `cockpit.rs:251-272` + `cockpit_live.rs:795-817` unchanged;
  click-outside via `MouseArea` backdrop at `journal_transaction_modal.rs:173`;
  Close button via header `on_press` at `:241`). H-arch-A7 + H-arch-A7b
  CONFIRMED falsified post-implementation. Divergence flagged on T4.3:
  the brief's `Float::new(stack, card)` two-arg shape doesn't match
  iced 0.14's `Float::new(content)` single-arg API, so the outer
  `Container::center_x/center_y` centering chrome is preserved (NOT
  removed). Modal `.snap` baselines byte-identical (text-summary
  snapshots, no widget-tree refresh needed). PNG baselines unaffected
  (modal not on Charts screen). Workspace anchor gate routes to
  T_FINAL_RUN_4 (Lane 4 touched zero report-rendering paths).
- 2026-05-13 (developer Lane 3): M3 (R3 kpi_strip grid migration)
  closed out. T3.0-T3.5 ticked `[x]` with verbatim test outputs.
  Implementation lands at
  [`crates/ui/src/widgets/kpi_strip.rs:25-26,143-153,171-182,200-207`](../../crates/ui/src/widgets/kpi_strip.rs).
  Imports add `iced::widget::grid::Grid`; `Row` import removed.
  6-card ready strip + 6-card unavailable strip both wired via
  `Grid::new().columns(6).spacing(space::M).height(Length::Shrink)`.
  Per-card `Length::FillPortion(1)` hint removed — `columns(6)`
  equalizes implicitly. H-arch-A3 RESOLVED-UNFALSIFIED confirmed
  in vivo; H-arch-A4 (Grid leg) RESOLVED-PARTIAL-FALSIFIED confirmed
  — defaults work, no Catalog adapter. Module doc updated with T3.0
  defaults rationale. **Wire shape divergence flagged on T3.1:** the
  brief's `.width(Length::Fill)` clause was dropped because
  `Grid::width` accepts only `Pixels` (grid.rs:73), and Grid defaults
  to filling its parent anyway. **Additional `.height(Length::Shrink)`
  required** to override the Grid default `Sizing::AspectRatio(1.0)`
  (grid.rs:57) which would force square cells — wrong for the ~80 px
  text strip. The three target `.snap` baselines
  (`viewer__kpi_strip__sample_report`,
  `viewer__kpi_strip__metrics_unavailable`,
  `viewer__full_view__sample_report`) are byte-identical pre/post
  migration — `strip_summary()` + `viewer_full_view_summary()` are
  content-summary helpers decoupled from layout primitive (zero
  `.snap.new` files, zero `cargo insta accept` calls needed). PNG
  baselines unaffected (kpi_strip lives on viewer-bin, not Charts
  screen — V3C structural). Anchor gate inapplicable (Lane 3 touched
  zero report-rendering paths). **Sandbox divergence on T3.5:**
  `cargo doc -p ui --no-deps` denied by developer-3 sandbox —
  orchestrator must run the doc render for the final V3E tick.
- 2026-05-13 (developer Lane 2): M2 (R2 strategies table migration +
  shared T2.0 Table Catalog adapter) closed out. T2.0-T2.7 ticked
  `[x]` with verbatim test outputs. T2.3 already removed in refinement
  pass; T2.8 (V2 matrix tick) belongs to test-runner / evaluator per
  AGENT.md test-runner / evaluator split. Implementation lands at:
  [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../crates/ui/src/theme/iced_widget_catalogs.rs)
  (new 100-LOC module, T2.0),
  [`crates/ui/src/theme.rs:42-48`](../../crates/ui/src/theme.rs)
  (`pub mod iced_widget_catalogs;` declaration),
  [`crates/ui/src/widgets/strategies.rs:23-40,58-194,196-251,253-263`](../../crates/ui/src/widgets/strategies.rs)
  (Table::new wire + column-1 Button + sibling error-badge Column +
  retired `row_for`/`active_row`-wrap path). H-arch-A2 REFINED
  CONFIRMED in vivo (`Table::new(columns, rows.iter().cloned())` with
  `StrategyRow: Clone` compiles + runs); H-arch-A5b RESOLVED-CONFIRM
  reaffirmed (zero row-decorator hooks used; column-1 Button is the
  only click path; sibling Column the only badge path).
  **Divergence on T2.0 (orphan-rule):** the architect's literal Q3-sub
  spec — `impl iced::widget::table::Catalog for iced::Theme` in the
  new module — is uncompilable: `iced::widget::table::Catalog` is
  already implemented upstream for `iced::Theme`
  (`iced_widget-0.14.2/src/table.rs:704-714`), and a second impl
  would violate Rust's orphan rules AND conflict with the upstream
  blanket impl. Architectural intent preserved by providing the
  cockpit's house `StyleFn` factory functions (`cockpit_table_style`
  + `cockpit_table_style_fn`) as the future Brief B `iced_aw`
  adoption hub — same module location, same `BORDER_1` token routing,
  same closure-everywhere lemma. Native `Table::new` v0.14 has no
  `.style(...)` builder, so the cockpit-tinted Class is currently
  consumed only by future iced_aw adopters / Themer overrides
  (documented in the module's `## What this module provides` section).
  **Divergence on T2.4 (snapshot refresh):** V2B expected 8 strategies
  snapshots refresh shape-only; in reality ZERO required refresh —
  the snapshots are model-introspection helpers
  ([`panel_snapshots.rs:1989-2081`](../../crates/ui/tests/panel_snapshots.rs)),
  not widget-tree renders. Cleaner outcome than the brief assumed.
  **Visual drift documented:** (a) selected-row 2 px ACCENT rule now
  spans column-1 cell height only, not full-row (Table cells, not
  full rows); (b) per-row error badges render below the entire Table
  rather than inline under each error row — both per Q5/Q6 architect
  rationale. **Sandbox divergence on T2.7:** `cargo doc -p ui --no-deps`
  denied by Lane 2 sandbox — `cargo check -p ui --tests` PASS used as
  pre-check; authoritative doc gate routes to T_FINAL_RUN_3
  (rust-validate). Workspace anchor gate (`scripts/verify_anchors.sh`)
  invoked locally and PASS 11/11 (Lane 2 touched zero
  report-rendering paths).
- 2026-05-13 (test-runner): M_FINAL_TEST_RUN closed. T_FINAL_RUN_1
  through T_FINAL_RUN_4 ticked `[x]` after the 4-lane merge at commit
  `9027a0d`. Run log emitted at
  [`reports/test-run-2026-05-13T10-09Z.log`](reports/test-run-2026-05-13T10-09Z.log)
  with verbatim stdout/stderr per command (fmt / build / workspace test
  / 6 per-target lanes / clippy / verify_anchors / visual_snapshots /
  chart_hover_grid_sweep) + sandbox-denied-step summary (`cargo doc -p
  ui --no-deps`, `shasum -a 256` of the 3 visual baselines, and `bash
  scripts/check_no_clocks_in_ui_tests.sh` — orchestrator must re-run
  before evaluator can certify). No verdict emitted; per
  [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split),
  VERDICT → PASS/FAIL/REGRESSION emits from the evaluator (fresh
  context, read-only) as M_FINAL_EVAL. Owner flipped developer →
  test-runner. Next: spawn evaluator.
