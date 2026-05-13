---
slug: iced-native-widgets
mode: release
status: shipped
audience: human-operator
updated: 2026-05-13
generated: 2026-05-13T10:38:54Z
---

# iced native widgets (v0.1.0) — Brief A — release

## TL;DR

Brief A ships **four widget migrations onto first-party iced 0.14 primitives**:
`positions.rs` + `strategies.rs` onto `iced::widget::table`, `kpi_strip.rs` onto
`iced::widget::grid`, and `journal_transaction_modal.rs` onto
`iced::widget::float`. A new shared theme submodule
[`crates/ui/src/theme/iced_widget_catalogs.rs`](../../../crates/ui/src/theme/iced_widget_catalogs.rs)
provides the `cockpit_table_style_fn` `StyleFn` factory — minted now, **not yet
consumed** (native `Table::new` v0.14 has no `.style()` setter; the factory is
the seam for Brief B `iced_aw` adoption). **Honest LOC accounting:** the
predecessor brief projected "~900-1100 LOC retired"; the actual delta on
touched files is **net +154 LOC** (the row/column glue retirement is real, but
the new Catalog adapter scaffold, module docs, and Table column lambdas pushed
file totals up). The value gain is **standardization**, not line-count
deletion: idiomatic iced widgets that future-proof AccessKit + Brief B, less
hand-rolled responsibility per widget, fewer footguns in column alignment /
selected-row chrome / overlay positioning. Evaluator emits **`VERDICT → PASS`**
on commit `1431409` with **1203+ workspace tests green** and **ANCHORS PASS
(11 / 11)** byte-identical to v1.5a + v2.0.0. Deferred to v0.2: Catalog
factory consumption (Brief B / iced_aw); full-row `frame::active_row`
composition (Table renders cells, not rows, by API design).

## What changed

Four `crates/ui/src/widgets/` view-surfaces re-wired to native iced 0.14
primitives + one new theme submodule:

- **R1 — positions Table migration** ([`crates/ui/src/widgets/positions.rs:37-50,63-125`](../../../crates/ui/src/widgets/positions.rs)).
  Seven-column `table::Table::new(columns, positions.iter().filter(...).cloned())`
  with `table::column(header, |p: PositionView| -> Element {...})` lambdas at
  [`positions.rs:77-120`](../../../crates/ui/src/widgets/positions.rs). PNL /
  PNL_PCT sentiment color routed through `color_for_delta(...)` inside the
  lambdas. SYMBOL left-aligned (column-builder default); QTY / COST / MARK /
  PNL / PNL_PCT / EXPOSURE right-aligned via
  `Column::align_x(Horizontal::Right)`. Legacy `Row`/`Column`/`Scrollable`
  glue + `frame::active_row` whole-row composition removed.
- **R2 — strategies Table migration + shared T2.0 Catalog adapter foundation**
  ([`crates/ui/src/widgets/strategies.rs:23-40,58-194,196-251,253-263`](../../../crates/ui/src/widgets/strategies.rs)).
  Six-column `Table::new(columns, rows.iter().cloned())`; column-1 Button
  wraps cell body for `Message::SelectStrategy(...)` whole-row click dispatch
  (Q5 committed shape). Per-row error badges rendered as a sibling
  `Column<error_badges>` BELOW the Table (Q6 / Option C). New theme submodule
  [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../../crates/ui/src/theme/iced_widget_catalogs.rs)
  (100 LOC) ships `cockpit_table_style` + `cockpit_table_style_fn` —
  `StyleFn<'_, Theme>` factories routing `color::BORDER_1` separator tokens.
  Wired into [`crates/ui/src/theme.rs:42-48`](../../../crates/ui/src/theme.rs)
  via `pub mod iced_widget_catalogs;`.
- **R3 — kpi_strip Grid migration**
  ([`crates/ui/src/widgets/kpi_strip.rs:25-26,143-153,171-182,200-207`](../../../crates/ui/src/widgets/kpi_strip.rs)).
  Six-card ready strip + six-card unavailable strip both wired via
  `Grid::new().columns(6).spacing(space::M).height(Length::Shrink)`. Per-card
  `Length::FillPortion(1)` hint removed — `columns(6)` equalizes implicitly.
  `Length::Shrink` is a deliberate override of Grid's default
  `Sizing::AspectRatio(1.0)` (which would force square cells).
- **R4 — journal_transaction_modal Float migration**
  ([`crates/ui/src/widgets/journal_transaction_modal.rs:62-65,98-154`](../../../crates/ui/src/widgets/journal_transaction_modal.rs)).
  `Float::new(content)` (single-arg, not two-arg as the brief originally
  suggested — see Architectural divergences) replaces the hand-rolled
  centered-card-via-Stack-plus-`center_x/center_y` composition. Three close
  paths preserved verbatim: Escape (cockpit subscription path at
  `state.rs:1036-1041` + `cockpit.rs:251-272`, unchanged), click-outside (the
  hand-rolled `MouseArea` backdrop at `journal_transaction_modal.rs:173`
  stays — `Float` is positioning-only, no dismiss callback), explicit Close
  button (header `on_press` at `:241`).

**LOC table (file totals, post-migration):**

| Widget | File | Pre-LOC | Post-LOC | Net |
|---|---|---:|---:|---:|
| positions | [`crates/ui/src/widgets/positions.rs`](../../../crates/ui/src/widgets/positions.rs) | 100 | 147 | +47 |
| strategies | [`crates/ui/src/widgets/strategies.rs`](../../../crates/ui/src/widgets/strategies.rs) | 344 | ~314 | −30 |
| kpi_strip | [`crates/ui/src/widgets/kpi_strip.rs`](../../../crates/ui/src/widgets/kpi_strip.rs) | 264 | ~272 | +8 |
| journal_modal | [`crates/ui/src/widgets/journal_transaction_modal.rs`](../../../crates/ui/src/widgets/journal_transaction_modal.rs) | 571 | ~600 | +29 |
| New: theme/iced_widget_catalogs.rs | [`crates/ui/src/theme/iced_widget_catalogs.rs`](../../../crates/ui/src/theme/iced_widget_catalogs.rs) | 0 | 100 | +100 |
| **Net actual** | — | — | — | **+154** |

The earlier "120 LOC retired" claim from
[`spec/iced-ecosystem-evaluation/feature.md`](../../iced-ecosystem-evaluation/feature.md)
specifically named the row/column glue layer; module docs + the new Catalog
adapter pushed the file totals up. The honest framing: **Brief A is a
standardization play, not a line-count deletion play.**

## What changed in process

This is the **second invocation** of the
[AGENT.md ## Test-runner / evaluator split](../../../AGENT.md#test-runner--evaluator-split)
(adopted 2026-05-12). The first was
[`ui-test-harness-bootstrap` v0.1](../../ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md)
on 2026-05-12. Empirical proof the pattern is repeatable under a multi-lane
feature:

- **test-runner** (write-allowed) ran fmt / build / test / clippy / anchors /
  visual / hover-grid against the 4-lane merged branch and dumped raw output
  to [`reports/test-run-2026-05-13T10-09Z.log`](../reports/test-run-2026-05-13T10-09Z.log)
  — no verdict, no prose. Three commands hit the test-runner sub-agent
  sandbox denial: `cargo doc -p ui --no-deps`, `shasum -a 256 crates/ui/tests/visual-baselines/*.png`,
  and `bash scripts/check_no_clocks_in_ui_tests.sh`. The test-runner logged
  each verbatim denial and escalated, rather than rationalizing
  ([test-run-2026-05-13T10-09Z.log:651-659](../reports/test-run-2026-05-13T10-09Z.log)).
- **orchestrator** supplemented the three sandbox-denied commands by re-running
  them in its own shell and appending the verbatim output to the run log via
  [`scripts/orch_supplement_log.sh`](../../../scripts/orch_supplement_log.sh)
  (the tooling helper extracted from
  [ui-test-harness-bootstrap v0.1](../../ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md)).
  Appended sections: `cargo doc` (6 pre-existing warnings, exit 0),
  `shasum -a 256` of the 3 baseline PNGs (matching bootstrap SHAs),
  `check_no_clocks_in_ui_tests.sh` (`CLOCKS PASS (8 files / 4 patterns)`,
  exit 0) — all at [test-run-2026-05-13T10-09Z.log:661-729](../reports/test-run-2026-05-13T10-09Z.log).
- **orchestrator-direct falsifier pass** (T-M0-J through T-M0-N, 2026-05-13
  refinement pass): ran 5 grep falsifiers against `~/.cargo/registry/.../iced_widget-0.14.2/src/{table,grid,float}.rs`
  for the sub-agent sandbox (which cannot Read into the cargo registry).
  Findings flipped H-arch-A2 to REFINED-CONFIRMED, locked H-arch-A4 / A5b /
  A7 / A7b to committed shapes (no fallback branches at developer time).
  Cited at [`spec/iced-native-widgets/tasks.md:72-117`](../tasks.md).
- **evaluator** (read-only, fresh context, never saw the developer diff) read
  the run log + cited artifacts and emitted **`VERDICT → PASS`** at
  [`reports/evaluation-2026-05-13T10-45Z.md`](../reports/evaluation-2026-05-13T10-45Z.md)
  with all 20 V-items (V1A through V4E) PASS and a verbatim `## Default-FAIL
  contract trace` of the 10 files Read during evaluation.

The split held cleanly under a **4-lane parallel dev fan-out** (Lanes 1/2/3/4
spawned in the same orchestrator tool-use block per
[AGENT.md ## Parallelism rules](../../../AGENT.md#parallelism-rules)) — no
verdict-skew from the test-runner, no sub-agent attempted a `cargo run --bin
cockpit` live launch, no architect "the bug is X" claim from live-app
instrumentation.

## Why

The operator's
[`iced-ecosystem-evaluation` prompt](../../iced-ecosystem-evaluation/feature.md)
of 2026-05-13 (Q-O3) ratified the adoption order **A → B → C → D unchanged**:
native iced 0.14 widgets first, then `iced_aw` cherry-pick (Brief B), then
`iced_dialog` (gated, Brief C), then `plotters-iced2` (gated, Brief D). This
brief IS Brief A. The four targets were greenlit by the predecessor brief's
architect synthesis (Q1 / Q2 / Q3 resolutions) as the largest-LOC,
lowest-risk surfaces whose target primitives (`table.rs` / `grid.rs` /
`float.rs`) already compile in the workspace lockfile under our current
`iced = "=0.14.0"` feature set `["tiny-skia", "thread-pool", "advanced",
"canvas"]` — **zero new direct or transitive crates**.

## What you can do now

| Action | Command |
|--------|---------|
| Re-verify the 11 backtest-report anchors stay byte-identical | `bash scripts/verify_anchors.sh` |
| Run the full ui-crate test suite (1203+ green) | `cargo test -p ui` |
| Run only the four migrated widgets' lib tests | `cargo test -p ui --lib widgets::positions widgets::strategies widgets::kpi_strip widgets::journal_transaction_modal` |
| Run the panel snapshot suite (68 baselines, all byte-identical post-migration) | `cargo test -p ui --test panel_snapshots` |
| Run the modal close-path integration tests | `cargo test -p ui --test tape_row_click_opens_modal` |
| Visual smoke on the migrated widgets | `cargo run --bin cockpit` and inspect positions / strategies tables, kpi_strip on the viewer screen, click a tape row to open the journal modal |
| Re-verify the no-clocks-in-snapshot-path gate | `bash scripts/check_no_clocks_in_ui_tests.sh` |
| Read the new theme submodule | `open crates/ui/src/theme/iced_widget_catalogs.rs` |

## Live demo

Verbatim from the test-runner + orchestrator-supplement run log
([`reports/test-run-2026-05-13T10-09Z.log`](../reports/test-run-2026-05-13T10-09Z.log)):

```
$ bash scripts/verify_anchors.sh
PASS  btc-2023-1m-sma-cross                 fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-sma-baseline-refresh      fc2e3b4a04055e60209fe85541173aa8883df226d2756352dfd101597168649c
PASS  btc-2023-1m-macd-trend                ef9c5e483fa079f670a7aa15671643fce3b39a5ce35df8cb6d797887053f8805
PASS  btc-2023-1m-rsi-reversion             bc56d20d608c680e534bf6764ce8e0e568f0d4ffdf847a539c53fef65170d7aa
PASS  btc-2023-1m-bbands-mean-revert        d8a08a23d3629556c5fca39d6af89d7e0f99418e642af0b86fce22ff4d2792e3
PASS  top10-2023-1h-momentum                3b60ef0743f006867b9e52f9de154869ee170987b27560e288b2d9597d3ecf97
PASS  top10-2024-h1-momentum                1f33534fc7c6af1c04330564bec77aac620ecf6f1058f11ff90dfb66adcf05c6
PASS  pairs-2023-zscore-mr                  90591a0ecc5d56c8ff93834b127a3780a31f51634f38f12c3c412391116abbd0
PASS  pairs-2024-h1-zscore-mr               14f50a598ba8343fc9be198a78716d036407d585c641c0b054eae6c062f1507f
PASS  report-sample-7d                      520b1f2968ad52d5981a1cdb3749235416c77c058364bd8c11ebd7d2468f46a3
PASS  report-sample-90d                     c656414ebf6f526372c27ae2d537301c68a0bc71d896f5a7cbc65a02edd60333
---
ANCHORS PASS  (11 / 11)
```
([test-run-2026-05-13T10-09Z.log:557-569](../reports/test-run-2026-05-13T10-09Z.log))

```
$ cargo test --workspace
... (149 unique "test result: ok." summary lines)
total ok: 1203, failed: 0, ignored: 4 (1 pre-existing doc-test + 3 pre-existing live-Binance integration tests)
```
([test-run-2026-05-13T10-09Z.log:23-167](../reports/test-run-2026-05-13T10-09Z.log)
+ aggregate per evaluator
[`evaluation-2026-05-13T10-45Z.md ## Non-regression`](../reports/evaluation-2026-05-13T10-45Z.md))

The 9 strategy backtest anchors at the top of the list are byte-identical to
v1.5a + v2.0.0 — confirming Brief A's non-regression contract: **zero touches
to `crates/strategy/`, `crates/audit/`, `crates/exec/`, `crates/backtest/`,
or any reports template**. The 2 `report-sample-*` anchors carry over the
v2.0.0 re-lock values verbatim.

## Verification matrix

Verbatim from
[`evaluation-2026-05-13T10-45Z.md ## V-items`](../reports/evaluation-2026-05-13T10-45Z.md).
All 20 rows PASS.

| V | Lane | Statement (abbrev) | Evidence | Result |
|---|---|---|---|---|
| V1A | R1 positions | `cargo build -p ui` + `cargo test -p ui` PASS; `panel_snapshots positions` PASS | log build exit 0 (line 21); test exit 0 (line 167); `widgets::positions` 2/0 (lines 169-180); panel_snapshots `positions_*` rows ok (lines 270-283) | PASS |
| V1B | R1 positions | 6 `positions_*.snap` baselines refresh shape-only + two-run byte-stable | panel_snapshots ok 68/0 (line 303); zero `.snap.new` between runs | PASS |
| V1C | R1 positions | 3 PNG baselines unaffected; `visual_snapshots.rs` green | visual_snapshots 4/0 (lines 580-587); shasum supplement matches bootstrap SHAs (lines 717-720) | PASS |
| V1D | R1 positions | `verify_anchors.sh` exit 0; 11/11 | `ANCHORS PASS (11 / 11)` (line 569) | PASS |
| V1E | R1 positions | `cargo doc -p ui --no-deps` warning-clean on `positions::view` surface | `cargo doc` orchestrator supplement exit 0 (line 711); 6 pre-existing warnings, ZERO on positions.rs (lines 665-707) | PASS |
| V2A | R2 strategies | `panel_snapshots strategies` PASS | `widgets::strategies` 2/0 (lines 188-192); panel_snapshots 9× `strategies_*` rows ok (lines 286-299) | PASS |
| V2B | R2 strategies | 8 strategies `.snap` baselines refresh shape-only + two-run byte-stable | panel_snapshots ok 68/0 (line 303); pause-button + override-modal snapshots byte-identical | PASS |
| V2C | R2 strategies | PNG baselines unaffected (Charts-only) | shasum supplement at bootstrap SHAs (lines 717-720); visual_snapshots 4/0 (line 586) | PASS |
| V2D | R2 strategies | Anchor verify PASS | `ANCHORS PASS (11 / 11)` (line 569) | PASS |
| V2E | R2 strategies | Docs warning-clean | `cargo doc` 6 pre-existing, ZERO on strategies.rs (lines 665-707) | PASS |
| V3A | R3 kpi_strip | `widgets::kpi_strip` PASS; in-file insta snapshots PASS | `widgets::kpi_strip` 2/0 (lines 200-206); `kpi_strip__sample_report` ok, `kpi_strip__metrics_unavailable` ok | PASS |
| V3B | R3 kpi_strip | `viewer__full_view__sample_report` snapshot refreshes + two-run byte-stable | panel_snapshots `viewer__full_view__sample_report ... ok` (line 300) | PASS |
| V3C | R3 kpi_strip | PNG baselines unaffected | shasum supplement at bootstrap SHAs (lines 717-720) | PASS |
| V3D | R3 kpi_strip | Anchor verify PASS | `ANCHORS PASS (11 / 11)` (line 569) | PASS |
| V3E | R3 kpi_strip | Docs warning-clean | `cargo doc` 6 pre-existing, ZERO on kpi_strip.rs | PASS |
| V4A | R4 journal | 4× `*_renders_without_panic` + `tape_row_click_opens_modal` green | `widgets::journal_transaction_modal` 5/0 (lines 216-222); `tape_row_click_opens_modal` 8/0 (lines 314-324) | PASS |
| V4B | R4 journal | Modal `.snap` baselines refresh shape-only; 3 close paths funnel to `Message::TapeAuditModalClosed` | `t1208_v5a_close_clears_modal`, `t1208_v5b_open_new_tx_replaces_modal`, `t1208_v5c_agent_halt_closes_modal` all ok (lines 317-320); `agent_feed_audit_modal_*` 4× ok (lines 234, 240, 250-252) | PASS |
| V4C | R4 journal | PNG baselines unaffected | shasum supplement at bootstrap SHAs (lines 717-720) | PASS |
| V4D | R4 journal | Anchor verify PASS | `ANCHORS PASS (11 / 11)` (line 569) | PASS |
| V4E | R4 journal | Docs warning-clean on the migrated widget surface | 2 pre-existing `doc-markdown` / `doc_lazy_continuation` warnings on `journal_transaction_modal.rs:111+116` (lines 479-503) — pre-existing per `## Sandbox-denied steps` note (line 655); ZERO net-new from Brief A | PASS |

## Hypothesis register

From
[`evaluation-2026-05-13T10-45Z.md ## Hypothesis register`](../reports/evaluation-2026-05-13T10-45Z.md):

- **H-arch-A1** — Two-run determinism gate. **RESOLVED-UNFALSIFIED.** Per-lane
  twice-run `panel_snapshots` produces zero `.snap.new` files; both runs
  emit identical `test result: ok.` summaries.
- **H-arch-A2** — `Table::new` constructor signature. **REFINED RESOLVED-CONFIRMED.**
  Actual sig is `Table::new<'b, T>(impl IntoIterator<Item = Column<...>>, impl IntoIterator<Item = T>) where T: Clone` — more permissive than the initial `Vec<T>` framing. Lane 1 calls `Table::new(columns, visible_iter)` at `positions.rs:122-124`; Lane 2 calls `Table::new(columns, rows.iter().cloned())` at `strategies.rs:163`. Both compile clean.
- **H-arch-A3** — Grid 6-cell layout fits kpi_strip. **RESOLVED-UNFALSIFIED.**
  `Grid::new().columns(6).push(...) × 6` shipped at `kpi_strip.rs:143, 171`.
- **H-arch-A4** — Closure-style theming on table/grid/float. **RESOLVED-PARTIAL-FALSIFIED-as-predicted.**
  Float closure works (`Float::style(...)`). Table native v0.14 has **no `.style()` setter** — Lane 2 shipped the `cockpit_table_style_fn` `StyleFn` factory at `iced_widget_catalogs.rs:95` (NOT `impl Catalog for iced::Theme` — orphan rule conflict with upstream; see Architectural divergences). Grid: no theme surface, defaults accepted.
- **H-arch-A5** — Lane 2 column-1 Button wrap is the only row-click path. **RESOLVED-UNFALSIFIED.**
  `strategies.rs:23` imports Button; `strategies_screen__*` snapshots green (lines 290-299).
- **H-arch-A5b** — Zero `row_decorator|after_row|on_row` hooks in `table.rs`. **RESOLVED-CONFIRM**
  (architect T-M0-L tick, orchestrator-direct grep against `iced_widget-0.14.2/src/table.rs`).
- **H-arch-A6** — kpi_strip ships 6 cards, not 4. **RESOLVED-UNFALSIFIED.**
  `kpi_strip.rs:143 / 171` wire confirms (Total Return / CAGR / Sharpe / Max DD / Win Rate / Trades).
- **H-arch-A7** — Float has `on_dismiss` + backdrop hook. **RESOLVED-FALSIFIED-as-predicted.**
  Zero matches in `float.rs` for dismiss / backdrop / focus-trap. Lane 4 wires `Float::new(content)` at `journal_transaction_modal.rs:151`; hand-rolled `MouseArea` backdrop preserved.
- **H-arch-A7b** — Float keyboard subscription participation. **RESOLVED-FALSIFIED-as-predicted.**
  Zero matches in `float.rs` for `keyboard|on_key|Escape|key_press|subscription`. Escape stays in `state.rs` subscription (T4.2 committed).

## Numbers that matter

- **Commits:** **5** (`3077425` research + Lane 2 → `970e857` Lane 3 → `9e5bd65` Lane 4 → `9027a0d` Lane 1 → `1431409` test-runner + evaluator M_FINAL).
- **Files touched:** **13** total (per `git diff --name-only d8c3a99..9027a0d` cited at
  [`evaluation-2026-05-13T10-45Z.md ## Non-regression`](../reports/evaluation-2026-05-13T10-45Z.md)).
  Six code files in `crates/ui/` (positions / strategies / kpi_strip /
  journal_modal / theme.rs / theme/iced_widget_catalogs.rs); 7 spec/doc files
  in `spec/iced-ecosystem-evaluation/`, `spec/iced-native-widgets/`,
  `spec/operator-success-reports/reports/` (v2.0.0 carry-forward),
  `spec/ui-design-principles.md`. **ZERO `Cargo.toml` / `Cargo.lock` edits.**
  **ZERO non-ui crates touched.** **ZERO `scripts/` edits.**
- **LOC delta:** **+154 net** across the 4 widget files + 1 new submodule
  (see LOC table in `## What changed`). The +154 includes +100 for the new
  Catalog adapter scaffold and module docs.
- **Tests passing:** **1203+ workspace tests green, 0 failed**, 4 pre-existing
  ignored (1 doc-test + 3 live-Binance integration). All four migrated
  widgets' lib tests (positions 2/0, strategies 2/0, kpi_strip 2/0,
  journal_transaction_modal 5/0) PASS; panel_snapshots 68/0 PASS;
  tape_row_click_opens_modal 8/0 PASS; visual_snapshots 4/0 PASS.
- **Anchors:** **11 / 11 PASS**, byte-identical to v1.5a + v2.0.0.
- **3 PNG visual baselines:** byte-identical to bootstrap v0.1
  (`73289bdf… / 85b73747… / a4a96ba0…`,
  [test-run-2026-05-13T10-09Z.log:717-720](../reports/test-run-2026-05-13T10-09Z.log)).
- **Brief A scope completion:** **4 / 4 widget migrations** ratified by
  Q1 / Q2 / Q3 (positions / strategies Tables + kpi_strip Grid + journal_modal
  Float). **T1938-style deferrals: 0** (vs. v2.0.0's 1).
- **Pre-existing warnings (out of Brief A scope):** 14 pedantic clippy
  warnings + 6 rustdoc warnings on `crates/ui` — ZERO net-new from any of the
  4 migrated files or the new theme submodule.

## Architectural divergences (honest)

Four divergences from the architect's initial brief, flagged in the lane
tasks.md changelog entries and the evaluator's
[`## Hypothesis register`](../reports/evaluation-2026-05-13T10-45Z.md):

1. **Lane 2 — Table Catalog orphan-rule pivot.** The brief's Q3-sub literal
   spec said `impl iced::widget::table::Catalog for iced::Theme` in the new
   submodule. **Uncompilable:** `iced_widget-0.14.2/src/table.rs:704-714`
   already implements `Catalog` for `iced::Theme` upstream; a second impl
   violates Rust's orphan rules AND conflicts with the upstream blanket impl.
   Architectural intent preserved by shipping a `StyleFn` factory function
   (`cockpit_table_style_fn`) at
   [`crates/ui/src/theme/iced_widget_catalogs.rs:95`](../../../crates/ui/src/theme/iced_widget_catalogs.rs)
   instead. The factory is currently unused — native `Table::new` v0.14 has
   no `.style()` setter. Brief B (`iced_aw`) is the planned consumer.
   Documented at [`iced_widget_catalogs.rs:34-38`](../../../crates/ui/src/theme/iced_widget_catalogs.rs).
2. **Lane 3 — Grid wire-shape correction.** The brief's wire was
   `Grid::new().columns(6).spacing(...).push(...).width(Length::Fill)`. **Two
   corrections:** (a) `Grid::width()` accepts only `Pixels`, not `Length` —
   the `.width(Length::Fill)` clause was dropped (Grid fills its parent by
   default); (b) Grid's default `Sizing::AspectRatio(1.0)` forces square
   cells — wrong for the ~80 px text strip. Lane 3 added an explicit
   `.height(Length::Shrink)` override at
   [`kpi_strip.rs:143, 171`](../../../crates/ui/src/widgets/kpi_strip.rs).
3. **Lane 4 — `Float::new(1 arg)`, not 2.** The brief's wire was
   `Float::new(stack, card)` (a base + overlay two-arg constructor). **Iced
   0.14's actual sig is `Float::new(content)` — single-arg.** The outer
   `Container::center_x / center_y` centering chrome is preserved (NOT
   removed). Wire at
   [`journal_transaction_modal.rs:151`](../../../crates/ui/src/widgets/journal_transaction_modal.rs).
4. **Lanes 1 + 2 — `Table::new` looser than expected.** Brief framed
   ownership as `Vec<T>`. Actual sig is
   `Table::new<'b, T>(impl IntoIterator<Item = Column<...>>, impl IntoIterator<Item = T>) where T: Clone` —
   more permissive. Lane 1 streams `positions.iter().filter(...).cloned()`
   directly with no intermediate `Vec`; Lane 2 streams
   `rows.iter().cloned()`. **No allocation penalty** vs. the assumed `Vec`
   build. (H-arch-A2 REFINED CONFIRMED.)

All four divergences ratified by the evaluator at
[`evaluation-2026-05-13T10-45Z.md ## Hypothesis register`](../reports/evaluation-2026-05-13T10-45Z.md);
each is consistent with the brief's intent, none routes to a re-spawn.

## Deferred items

| Item | Disposition |
|---|---|
| **Catalog factory consumption** — `cockpit_table_style_fn` minted, not yet wired | Deferred to **v0.2 / Brief B (`iced_aw`)**. Native `Table::new` v0.14 has no `.style()` setter; the factory is the seam for `iced_aw` table adopters or future Themer-wrap. Documented at [`iced_widget_catalogs.rs:34-38`](../../../crates/ui/src/theme/iced_widget_catalogs.rs). Not a regression — Brief A's V1B/V2B "shape-only snapshot drift" envelope explicitly absorbs the bounded visual drift from `Table`'s default style. |
| **`frame::active_row` whole-row composition** retired | By API design — native `iced::widget::table` renders cells, not rows; full-row composition is not expressible at the `Table` level. Lane 2's selected-row 2 px ACCENT rule now spans column-1 cell height only, not full-row (Q5 architect rationale; visual drift documented). |
| **14 pre-existing pedantic clippy warnings** on `crates/ui` | Out-of-scope per v2-llm-strategy precedent — pre-existing pedantic warnings are not blockers. ZERO net-new from Brief A on any of the 5 touched files. |
| **6 pre-existing rustdoc warnings** (chart_tooltip / volume_histogram / window_icon / test_support intra-doc links) | Out-of-scope per v2-llm-strategy precedent. Brief A introduces zero net-new doc warnings. |
| **Focus-trap inside Float modal** | Not provided by native `Float` v0.14 (H-arch-A7 / A7b). Tab-escape from modal accepted per current iced limitation; no regression vs. hand-rolled. Native dismiss + focus trap remain the Brief C (`iced_dialog`) trigger. |
| **T1938 cockpit "LLM budget" tile** | v2 LLM v2.1 follow-up, out of Brief A scope. |

## Open decisions

_None pending — ready to ship._

The four Q-resolutions (Q1 / Q2 / Q3 / Q-O3 brief ordering) were operator-
and architect-resolved 2026-05-13 in the predecessor
[`iced-ecosystem-evaluation` v0.2.0](../../iced-ecosystem-evaluation/feature.md)
synthesis. The four lane divergences (orphan rule, Grid sizing, Float
arg-count, Table IntoIterator) are evidenced verbatim in the code per the
Architectural divergences table above and ratified by the evaluator's
hypothesis register.

## Approval

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — _add reason below_

### Notes / feedback

_empty until operator fills_

## Changelog

- 2026-05-13 (presenter): initial draft. Evaluator `VERDICT → PASS` cited
  from [`reports/evaluation-2026-05-13T10-45Z.md`](../reports/evaluation-2026-05-13T10-45Z.md)
  on commit `1431409`; verbatim `ANCHORS PASS (11 / 11)` embedded; full V1A-V4E
  matrix (20 V-items, all PASS) lifted from the evaluator report; H-arch-A1
  through A7b status table cited; 4 architectural divergences flagged
  honestly across the 4 lanes (orphan-rule Catalog pivot, Grid
  `Length::Shrink` correction, `Float::new(1 arg)`, `Table::new` IntoIterator
  loosening); LOC delta presented as +154 net (vs. the predecessor brief's
  "~900-1100 LOC retired" expectation — framed honestly as
  standardization-not-deletion). Second invocation of the
  [AGENT.md ## Test-runner / evaluator split](../../../AGENT.md#test-runner--evaluator-split)
  flagged as meta-deliverable (first was
  [`ui-test-harness-bootstrap` v0.1](../../ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md)
  on 2026-05-12). Orchestrator role recorded: 5 grep falsifiers
  (T-M0-J/K/L/M/N) for the sub-agent sandbox + 3 sandbox-denied commands
  supplemented into the test-runner log via
  [`scripts/orch_supplement_log.sh`](../../../scripts/orch_supplement_log.sh)
  (tooling helper extracted from bootstrap v0.1). All 3 approval boxes
  UN-ticked for the operator gate. Presentations/ directory created new for
  this slug — direct `Write` used (no `spec-update` skill invocation needed
  for first-write).
- 2026-05-13 (operator): `[x] Approved — ship`. Pre-tick gate PASS;
  evaluator VERDICT → PASS at commit `1431409`; anchors PASS 11/11; 20/20
  V-items; 1203+ tests; zero non-target-crate diffs. Status flipped
  `draft → shipped`. 4 honest architectural divergences documented inline
  (orphan-rule pivot, Grid AspectRatio override, Float ghost API, Table
  IntoIterator). Catalog factory consumption deferred to Brief B (iced_aw)
  / v0.2. M5 markdown viewer remains queued.
