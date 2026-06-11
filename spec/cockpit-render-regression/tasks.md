---
slug: cockpit-render-regression
status: shipped
owner: developer
updated: 2026-05-14
version: 0.3.0
---

# Tasks — cockpit render regression (v0.2.0)

> **Status:** architect design pass complete
> ([`feature.md ## M0`](feature.md#m0--hypothesis-register-orchestrator-runnable-falsifiers)
> 2026-05-14). T-tasks below are concrete, file:line-scoped, and
> ordered cheapest-falsifier-first for M0 so the orchestrator can
> pin the offending widget in ≤30 min of bisect work.
>
> Honest-tick discipline ([`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
> rule 1): every owning agent MUST cite (a) file:line of change,
> (b) test command, (c) test-output line on every `[x]`. The tester
> (test-runner + evaluator split per
> [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split))
> owns the `T_FINAL_*` ticks.
>
> Anchor risk: **zero** (this brief touches `crates/ui/` only —
> zero strategy / audit / exec / backtest paths). PNG-baseline diff:
> **zero on existing baselines**; M1-B *adds* new baselines under
> a sibling directory.

## M0 — Falsifier batch (orchestrator-runnable, in order)

The orchestrator runs each falsifier in order. The first one to FAIL
(i.e., cockpit boots without panic in 7s) identifies the culprit
widget; the orchestrator emits `HANDOFF → developer (fix scope: <widget>)`
with the file:line and the original `feature.md` H-row cite. Subsequent
falsifiers stop on first PASS.

REQ trace: **REQ-COCKPIT-PANIC-001**.

- [x] **T-M0-H1** *(orchestrator, 2026-05-14)* — H1 UNFALSIFIED.
  - File:line of change: `crates/ui/src/theme.rs:596` flipped
    `RIGHT_RAIL_WIDTH_PX = 0.0 → 1.0` (reverted).
  - Test command: `cargo build -p ui --bin cockpit --features
    fixtures && cargo run -p ui --bin cockpit --features fixtures`
    (7s background).
  - Test-output line: 3 panic counts (one `panicked at` + one
    `non-unwinding panic` + one trailing `panic_handler` frame).
  - Outcome: right-rail not the trigger. Move to T-M0-H2.
  - Cited at [`feature.md ## M0 results`](feature.md#m0-results-orchestrator-executed-2026-05-14).

- [x] **T-M0-H2** *(orchestrator, 2026-05-14)* — H2 UNFALSIFIED.
  - File:line of change: `crates/ui/src/widgets/frame.rs:135`
    `.height(0) → .height(1)` AND
    `crates/ui/src/screens/strategies.rs:263`
    `.height(0) → .height(1)` (both reverted).
  - Test command: same shape as T-M0-H1.
  - Test-output line: 3 panic counts.
  - Outcome: `.height(0)` Spaces not the trigger. Move to bisect
    via screen body bypass.
  - Cited at [`feature.md ## M0 results`](feature.md#m0-results-orchestrator-executed-2026-05-14).

- [x] **T-M0-H5** *(obsoleted, 2026-05-14)* — H5 not executed; the
  M0 bisect via shell/screen-body bypass found the culprit in the
  strategies widget before H5 was needed. The journal modal is
  inside the (now-confirmed-clean) shell/Home/positions/pnl
  subtree, so H5 is **unreachable as the trigger** by construction.
  - Outcome: obsoleted by the H3 confirmation. No edit, no run.
  - Cited at [`feature.md ## M0 results`](feature.md#m0-results-orchestrator-executed-2026-05-14).

- [x] **T-M0-H3** *(orchestrator, 2026-05-14)* — H3 CONFIRMED with
  its empty-rows assumption FALSIFIED.
  - File:line of change: orchestrator bypassed
    `strategies::view` → `ready_body` → `strategies_table` in
    sequence, then restored. Final bisect step replaced the
    `strategies_table` (an
    `iced::widget::table::Table::new(...)` at
    `crates/ui/src/widgets/strategies.rs:165`) with a plain
    `Column::new().push(Text::new(...))` while keeping
    `error_badges` + `footer` siblings intact.
  - Test command: `cargo build -p ui --bin cockpit --features
    fixtures && cargo run -p ui --bin cockpit --features fixtures`
    (7s background).
  - Test-output line: bypass step → **0 panic counts** (cockpit
    clean for 7s). All earlier steps in the same path → 3 panic
    counts.
  - Outcome: culprit pinned to
    `crates/ui/src/widgets/strategies.rs:165`. Original H3
    assumption ("empty `rows` slice before fixtures populate") is
    wrong — fixtures pre-populate via
    `crates/ui/src/bin/cockpit.rs:161-166` `Message::BarReceived`,
    so `ready_body` sees non-empty rows on first frame. M0-FIX
    section in feature.md targets the populated-rows path.
  - Cited at [`feature.md ## M0 results`](feature.md#m0-results-orchestrator-executed-2026-05-14).

- [x] **T-M0-H6 / T-M0-H4 / T-M0-H7 / T-M0-H8** *(obsoleted,
  2026-05-14)* — All four hypotheses obsoleted by the H3
  confirmation. The bisect path proved the culprit lives inside
  `crates/ui/src/widgets/strategies.rs`'s `ready_body` ↓
  `strategies_table` subtree. H6 (strategies-screen empty Space)
  lives ABOVE the bypassed subtree (in `screens/strategies.rs`,
  not the widget), and the H3 bypass kept the panel wrapper +
  `error_badges` + `footer` siblings — all clean. H4 (KPI Grid)
  is non-Home and was never in the cold-start render path
  (Cockpit cold-start screen is `Home`, confirmed by the bisect
  showing Home/strategies alone reproduces). H7 (chart canvas)
  similarly is in the pnl tile that the bisect confirmed clean.
  H8 (focus_ring / kill catch-all) sits inside the
  `strategies_table` subtree (focus_ring wraps each row's button
  per `screens/strategies.rs:178`), but H3's bypass-everything-
  except-strategies_table proved Hosé's components clean — the
  load-bearing element is the Table itself, and column 1's rule
  Container inside it.
  - Outcome: no edits, no runs. All four moved to "obsoleted-by-
    construction" status.
  - Cited at [`feature.md ## M0 results`](feature.md#m0-results-orchestrator-executed-2026-05-14).

## M0-FIX — H3 root-cause fix design and falsifier matrix

The fix-candidate ladder is committed in
[`feature.md ## M0-FIX`](feature.md#m0-fix--h3-root-cause-fix-design).
This task list mirrors that ladder. Orchestrator runs F1 → F2 →
F3 in order; first FALSIFIED commits + HANDOFF → developer; STOP.
F4 is diagnostic-only (skipped unless F1-F3 all UNFALSIFIED). F5
is last resort and requires an ADR.

REQ trace: **REQ-COCKPIT-PANIC-001**.

- [x] **T-FIX-1** *(orchestrator, 2026-05-14 — FALSIFIED, fix
  landed by developer 2026-05-14 with named constant refactor)* —
  Falsify F1: `id_cell` rule `Length::Fill` →
  `Length::Fixed(STRATEGY_RULE_HEIGHT_PX = 24.0)`.
  _Statement._ Pin the rule Container's height to a fixed pixel
  value (~ Table body row height). Bet: the Table cell-layout
  pass's zero-height transient no longer reaches the rule's
  styled fill_quad.
  _Falsifier (orchestrator-executed)._
  - Edit
    [`crates/ui/src/widgets/strategies.rs:220`](../../crates/ui/src/widgets/strategies.rs)
    `.height(Length::Fill)` → `.height(Length::Fixed(24.0))` (on
    the inner `Space`) AND
    [`crates/ui/src/widgets/strategies.rs:223`](../../crates/ui/src/widgets/strategies.rs)
    `.height(Length::Fill)` → `.height(Length::Fixed(24.0))` (on
    the outer `Container`).
  - Build: `cargo build -p ui --bin cockpit --features fixtures`.
  - Run: `cargo run -p ui --bin cockpit --features fixtures` for
    7s (background; `kill` at 7s; stderr to
    `/tmp/cockpit-f1-falsifier.log`).
  - Grep: `grep -c 'panicked at' /tmp/cockpit-f1-falsifier.log`.
  _Honest-tick citations (per AGENT.md ## Process discipline rule 1)._
  - (a) file:line of change (post-refactor):
    `crates/ui/src/theme.rs:619 pub const STRATEGY_RULE_HEIGHT_PX: f32 = 24.0` +
    `crates/ui/src/widgets/strategies.rs:228 .height(Length::Fixed(layout::STRATEGY_RULE_HEIGHT_PX))` (inner `Space`) +
    `crates/ui/src/widgets/strategies.rs:231 .height(Length::Fixed(layout::STRATEGY_RULE_HEIGHT_PX))` (outer `Container`).
  - (b) test command (cockpit smoke, orchestrator-only per
    [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries)):
    `cargo build -p ui --bin cockpit --features fixtures && (cargo run -p ui --bin cockpit --features fixtures &) ; sleep 7 ; pkill -f 'target/debug/cockpit' ; grep -c 'panicked at' /tmp/cockpit-f1-falsifier.log`.
  - (c) test-output line: `panic count: 0` in `/tmp/cockpit-f1-falsifier.log`
    (orchestrator-produced, 2026-05-14) vs `panic count: 2` in
    `/tmp/cockpit-runtime.log` (baseline). Verified via
    `grep -c 'panicked at' /tmp/cockpit-f1-falsifier.log` → `0`;
    `grep -c 'panicked at' /tmp/cockpit-runtime.log` → `2`.
  _Outcome._ FALSIFIED → F1 is the fix. M0-FIX sequence STOPS.
  Developer landed the named-constant refactor + doc comment
  per the orchestrator's HANDOFF acceptance criteria; F1
  bisect-residual comments removed.
  _Blast radius (as landed)._ **File-span: 4 LOC** in
  `crates/ui/src/widgets/strategies.rs` (two `Length::Fixed(24.0)`
  → `Length::Fixed(layout::STRATEGY_RULE_HEIGHT_PX)` swaps + 1-line
  `use` extension + a doc-comment block on the rule binding).
  **Glue-layer: 28 LOC** added to `crates/ui/src/theme.rs` (the
  `STRATEGY_RULE_HEIGHT_PX` const + its `///`-doc explaining the
  WHY). Affected files: `crates/ui/src/widgets/strategies.rs`,
  `crates/ui/src/theme.rs`.

- [~] **T-FIX-2** *(obsoleted by F1, 2026-05-14)* — Falsify F2: replace
  `Container::new(Space::new())` rule with stock
  `iced::widget::vertical_rule(2)`.
  _Status._ Not executed. The M0-FIX falsifier sequence stops on
  the first FALSIFIED candidate; F1 falsified and shipped (see
  T-FIX-1 above). F2 retained for spec-history but never run.
  _Statement._ Stock iced widget; its `Widget::layout` guards
  against zero-bound emissions per its source. Bet: stock widget
  knows how to behave in Table-cell layout where the custom
  Container+Space composition does not.
  _Falsifier._
  - Replace
    [`crates/ui/src/widgets/strategies.rs:217-227`](../../crates/ui/src/widgets/strategies.rs)
    (the rule construction block) with:
    ```rust
    use iced::widget::vertical_rule;
    let rule = vertical_rule(2).style(move |_theme: &iced::Theme| {
        iced::widget::rule::Style {
            color: rule_color,
            width: 2,
            radius: iced::border::radius(0),
            fill_mode: iced::widget::rule::FillMode::Full,
        }
    });
    ```
  - Build + run + grep (same shape as T-FIX-1, log path
    `/tmp/cockpit-fix-f2-stderr.log`).
  _Expected on FALSIFIED._ Empty grep output. F2 is the fix.
  Commit; HANDOFF → developer with cleanup scope including
  visual verification (the active-strategy rule should still
  render in `ACCENT` colour). STOP.
  _Expected on UNFALSIFIED._ 3 panic indicators still present.
  Revert. Move to T-FIX-3.
  _Blast radius._ **File-span: ~10 LOC** (replace 11 lines with
  ~8). **Glue-layer: 0 LOC**. `vertical_rule` is in iced's
  prelude — no Cargo.toml change. Affected files:
  `crates/ui/src/widgets/strategies.rs` only. Visual drift: rule
  colour now flows through `rule::Style.color` rather than
  `container::Style.background`; the closure pins it to
  `rule_color` explicitly, so the rendered pixel is unchanged.

- [~] **T-FIX-3** *(obsoleted by F1, 2026-05-14)* — Falsify F3:
  collapse Table separator thickness to zero pixels.
  _Status._ Not executed. F1 stopped the falsifier sequence.
  F3 retained for spec-history.
  _Statement._ Edit the `Table` builder chain to add
  `.separator_x(0).separator_y(0)`. Bet: if the panic is in
  Table's separator fill_quad emission (not the cell-rule
  Container), zero-thickness separators short-circuit the
  fill_quad call entirely and the panic stops.
  _Falsifier._
  - Edit
    [`crates/ui/src/widgets/strategies.rs:165`](../../crates/ui/src/widgets/strategies.rs)
    from
    ```rust
    let strategies_table = table::Table::new(columns, rows.iter().cloned()).width(Length::Fill);
    ```
    to
    ```rust
    let strategies_table = table::Table::new(columns, rows.iter().cloned())
        .width(Length::Fill)
        .separator_x(0)
        .separator_y(0);
    ```
  - Build + run + grep (same shape, log path
    `/tmp/cockpit-fix-f3-stderr.log`).
  _Expected on FALSIFIED._ Empty grep output. F3 is the fix.
  However: the operator must confirm visual acceptability (the
  inter-cell hairline separators disappear). If acceptable,
  commit + HANDOFF → developer; STOP. If not acceptable,
  F3 result is diagnostic (proves the separator is the
  load-bearing fill_quad, not the rule Container) but not the
  shipped fix — escalate to architect for an alternative.
  _Expected on UNFALSIFIED._ 3 panic indicators still present.
  This means the separator is NOT the trigger, which corroborates
  the standing hypothesis (the rule Container's `Length::Fill`
  is the trigger). Revert. Move to T-FIX-4 (diagnostic) only
  if operator approves; otherwise jump to T-FIX-5.
  _Blast radius._ **File-span: 2 LOC** (`.separator_x(0).separator_y(0)`
  builder calls). **Glue-layer: 0 LOC**. Affected files:
  `crates/ui/src/widgets/strategies.rs` only. Visual drift:
  inter-cell hairlines disappear (separator colour was set via
  the unused Catalog adapter; now they have zero thickness).

- [~] **T-FIX-4** *(obsoleted by F1, 2026-05-14; DIAGNOSTIC ONLY — would have run only if F1-F3 all UNFALSIFIED)* — Wire Catalog adapter via Themer.
  _Status._ Not executed. F1 stopped the falsifier sequence.
  _Statement._ Wrap `strategies_table` in
  `iced::widget::Themer::new(strategies_table, |_theme|
  /* substitute table::Class with cockpit_table_style_fn() */)`.
  The Catalog adapter sets separator COLOUR; not expected to fix
  geometry-driven panic. Run only to formally rule out the
  Catalog-adapter hypothesis.
  _Falsifier._
  - Edit `crates/ui/src/widgets/strategies.rs:165` to wrap the
    final `strategies_table` Element in
    `iced::widget::Themer::new(strategies_table, /* class supplier
    using crate::theme::iced_widget_catalogs::cockpit_table_style_fn()
    */)`. Exact Themer signature: see
    [docs.rs/iced_widget/0.14.2/iced_widget/struct.Themer.html](https://docs.rs/iced_widget/0.14.2/iced_widget/struct.Themer.html);
    orchestrator may need a small `use` import.
  - Build + run + grep (log path `/tmp/cockpit-fix-f4-stderr.log`).
  _Expected on FALSIFIED._ Surprise; Catalog adapter was load-
  bearing after all. Commit + architect re-engages to update the
  hypothesis register and document why colour-only style changes
  geometry. (Architect estimate: <5% probability.)
  _Expected on UNFALSIFIED (~95% expected)._ Confirms the Catalog
  adapter is a red herring. Revert. Move to T-FIX-5.
  _Blast radius._ **File-span: ~5 LOC** (Themer wrap + use
  statement). **Glue-layer: 0 LOC** (adapter already shipped at
  `crates/ui/src/theme/iced_widget_catalogs.rs:99-117`).

- [~] **T-FIX-5** *(obsoleted by F1, 2026-05-14; LAST RESORT — would have required an ADR)* — Revert Brief A R2 strategies-table only.
  _Status._ Not executed. F1 stopped the falsifier sequence; no
  ADR required because Brief A's structural decision stands.
  _Statement._ Replace `table::Table::new(columns, rows)` at
  `crates/ui/src/widgets/strategies.rs:165` with the pre-Brief-A
  hand-rolled `Row::new()` header + `Scrollable<Column>` body.
  Positions stays on the native Table path (positions does not
  panic, confirmed by M0 bisect). This is a partial reversal of
  Brief A's structural decision and **requires an ADR**.
  _Falsifier._
  - Developer pass (too large for orchestrator one-shot): apply
    the revert. Source for diff: Brief A's
    [`spec/iced-native-widgets/feature.md`](../iced-native-widgets/feature.md)
    migration diff context + `git show` on the Brief A migration
    commit (introducing `table::Table::new(...)` for strategies).
    Reverse-apply only the strategies-table portion.
  - File the ADR at
    `spec/cockpit-render-regression/architecture/adr-001-brief-a-r2-partial-revert.md`
    documenting (a) why F1-F4 all failed, (b) why upstream iced
    patch / wgpu switch are off-table, (c) what would unblock a
    future re-migration (e.g. iced 0.15 Table or an upstream
    tiny-skia patch fixing the all-radii-zero branch).
  - Build + run + grep (log path
    `/tmp/cockpit-fix-f5-stderr.log`).
  _Expected on FALSIFIED._ Empty grep output. F5 is the
  shipped fix. Commit + ADR + HANDOFF → developer for the
  T-M_FINAL gate. STOP.
  _Expected on UNFALSIFIED._ The revert itself doesn't help — the
  bug is in code shared by both the legacy hand-rolled path AND
  the new Table path. ESCALATE TO OPERATOR. Architect re-engages.
  _Blast radius._ **File-span: ~80-120 LOC** per Brief A's own
  diff cite. **Glue-layer: 0 LOC** (no Cargo.toml or feature-flag
  changes). Affected files:
  `crates/ui/src/widgets/strategies.rs` only.

- [x] **T-M0-FIX-VERIFY** *(developer, 2026-05-14, verified via
  orchestrator-produced log)* — Cleanup + verification pass.
  _Honest-tick citations (per AGENT.md rule 1)._
  - (a) file:line of change (post-developer refactor):
    `crates/ui/src/theme.rs:619 pub const STRATEGY_RULE_HEIGHT_PX: f32 = 24.0` +
    `crates/ui/src/widgets/strategies.rs:228 .height(Length::Fixed(layout::STRATEGY_RULE_HEIGHT_PX))` (inner `Space`) +
    `crates/ui/src/widgets/strategies.rs:231 .height(Length::Fixed(layout::STRATEGY_RULE_HEIGHT_PX))` (outer `Container`).
  - (b) test commands (sub-agent-runnable, per
    [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries)):
    `cargo build -p ui` + `cargo fmt -p ui --check` +
    `cargo test -p ui` + `cargo clippy -p ui --no-deps --lib --tests`
    + `cargo doc -p ui --no-deps`. Cockpit-smoke (panic-free 7s
    run) is orchestrator-only and was executed by the
    orchestrator at T-FIX-1; this gate cites that log.
  - (c) test-output lines:
    - `cargo build -p ui` → `Finished dev profile [unoptimized + debuginfo] target(s)` (clean).
    - `cargo fmt -p ui --check` → exit 0, no output.
    - `cargo test -p ui` → 267 tests pass, 0 fail (sum across
      all binaries — identical to the pre-fix baseline reported
      by the prior test-runner pass).
    - `cargo clippy -p ui --no-deps --lib --tests` → 0 NET-NEW
      errors / warnings on `crates/ui/src/theme.rs` or
      `crates/ui/src/widgets/strategies.rs`. Pre-existing 6 errors
      in `widgets/chart.rs` + `window_icon.rs` are documented
      pre-Brief-B per the test-runner's prior log; unchanged.
    - `cargo doc -p ui --no-deps` → 0 NET-NEW intra-doc warnings
      on touched files. Pre-existing 6 unrelated unresolved-link
      warnings unchanged.
    - cockpit smoke (orchestrator-executed at T-FIX-1):
      `grep -c 'panicked at' /tmp/cockpit-f1-falsifier.log` →
      `0` (vs `2` in `/tmp/cockpit-runtime.log` baseline).
  - Trace row satisfied: **REQ-COCKPIT-PANIC-001** (`crates` and
    `tests` columns filled in `spec/trace.toml` by this pass).
  - Visual verification: deferred to operator approval at
    presentation-time. The active-strategy rule renders as a
    2 px wide vertical accent stripe in column 1 by construction
    (the Container's `background` is still `ACCENT` when
    `is_active`; only the `height` argument changed). Full
    visual proof requires a screencap, which is orchestrator-only
    per [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries).

## M1 — Quality-gate overhaul

Once M0 lands and the cockpit boots clean, M1 builds the gates that
would have caught this BEFORE operator approval. Tasks are sized so
M1-A can ship same-day; M1-B + M1-C span the following ~4 dev-days.

### M1-A — Cockpit-smoke skill (mandatory pre-tick gate)

REQ trace: **REQ-UI-QUALITY-GATE-001**.

- [ ] **T-M1-A-1** *(orchestrator)* — Author
  `.claude/skills/cockpit-smoke/SKILL.md`.
  _Acceptance criteria:_
  - File:line of change: new file
    `.claude/skills/cockpit-smoke/SKILL.md` (~30 LOC), shape per
    [`feature.md ## M1-A`](feature.md#m1-a--cockpit-smoke-skill-mandatory-orchestrator-pre-tick-gate).
  - Skill captures stderr to `/tmp/cockpit-smoke-stderr.log`,
    greps for `panicked at|non-unwinding panic|fatal runtime error`.
  - Exits 0 on clean 7s run; exits 1 (with stderr dumped) on any
    panic indicator.
  - Documents the capability-boundary rationale (orchestrator-only
    per [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries)).
  - Test command: invoke the skill manually after T-M0-FIX lands.
  - Test output line: `COCKPIT SMOKE PASS (7s clean run)`.
  - Trace row satisfied: REQ-UI-QUALITY-GATE-001.

- [ ] **T-M1-A-2** *(orchestrator)* — Update
  [`AGENT.md`](../../AGENT.md) to require the cockpit-smoke skill
  as a pre-tick gate for any UI brief.
  _Acceptance criteria:_
  - File:line of change: new section under `## Capability boundaries`
    in [`AGENT.md`](../../AGENT.md) (~15 LOC) titled "Cockpit-smoke
    pre-tick gate."
  - Specifies the skill is invoked AFTER evaluator PASS, BEFORE
    presenter assembles the operator approval block.
  - Specifies failure routing: any skill FAIL routes
    `HANDOFF → developer (REQ-UI-QUALITY-GATE-001 violation)`.
  - Trace row satisfied: REQ-UI-QUALITY-GATE-001.

### M1-B — Real-renderer snapshot tests via `iced_test::Simulator` + `Headless`

REQ trace: **REQ-UI-QUALITY-GATE-002**.

- [ ] **T-M1-B-1** *(developer)* — PoC render-snapshot test for one
  panel.
  _Acceptance criteria:_
  - File:line of change: new file `crates/ui/tests/render_snapshots.rs`
    (~250 LOC), with one test `positions_ready_panel_renders_cleanly`
    that constructs `Cockpit { positions: PanelState::Ready(vec![pv]) }`,
    invokes `iced_test::simulator(positions::view(&cockpit))`, rasterizes
    via `iced::advanced::renderer::Headless` to a 800×600 PNG, compares
    against a committed baseline at
    `crates/ui/tests/visual-baselines/render_snapshots/positions_ready.png`
    via `image_compare::gray_similarity_structure(&Algorithm::MSSIMSimple, ...)`,
    asserts SSIM ≥ 0.99.
  - Two-run determinism: `cargo test -p ui --test render_snapshots` run
    twice; identical PNG output bytes both runs.
  - Test command: `cargo test -p ui --test render_snapshots positions_ready_panel_renders_cleanly`.
  - Test output line: `test result: ok. 1 passed`.
  - **Important — addresses architectural divergence M1-B caveat:** PoC
    must demonstrate that `iced_test::Simulator` ALONE does not catch
    the render panic, but `Simulator + Headless::Renderer` does. The
    PoC's documentation block calls this out.
  - Trace row satisfied: REQ-UI-QUALITY-GATE-002.

- [ ] **T-M1-B-2** *(developer, after T-M1-B-1 lands)* — Bulk migration:
  replace ~244 text-summary `*_summary` helpers at
  [`crates/ui/tests/panel_snapshots.rs:1779-2298`](../../crates/ui/tests/panel_snapshots.rs)
  with render-snapshot tests.
  _Acceptance criteria:_
  - File:line of change: `crates/ui/tests/render_snapshots.rs` grows
    to cover the same panel surface; text-summary helpers retired
    from `panel_snapshots.rs` (deletion or migration depending on
    coverage parity).
  - All ~250 PNG baselines committed under
    `crates/ui/tests/visual-baselines/render_snapshots/<panel>/`.
  - Two-run determinism: same gate as T-M1-B-1, across ALL render
    snapshots.
  - Test command: `cargo test -p ui --test render_snapshots`.
  - Test output line: `test result: ok. ~250 passed`.
  - Trace row satisfied: REQ-UI-QUALITY-GATE-002.

### M1-C — Property-based layout invariants via `proptest`

REQ trace: **REQ-UI-QUALITY-GATE-002** (sibling — same systemic-gate goal).

- [ ] **T-M1-C-1** *(developer)* — Author `crates/ui/tests/layout_invariants.rs`
  covering 6 widgets implicated in M0.
  _Acceptance criteria:_
  - File:line of change: new file `crates/ui/tests/layout_invariants.rs`
    (~250 LOC) with `proptest!` blocks for `positions`,
    `strategies`, `kpi_strip`, `journal_transaction_modal`, `chart`,
    `focus_ring`.
  - Each property test invokes `widget.as_widget().layout(...)`
    with a fuzzed input and `prop_assert!` that resulting
    `Node::size().width > 0.0` AND `Node::size().height > 0.0`
    (or `is_nan()` for explicit-NaN cases iced uses).
  - Recursive traversal of `node.children()` asserts no child has
    zero dim either.
  - Workspace `Cargo.toml` confirms `proptest` is in
    `[workspace.dependencies]` (verify before edit; promote if
    not present).
  - Test command: `cargo test -p ui --test layout_invariants`.
  - Test output line: `test result: ok. 6 passed`.
  - Trace row satisfied: REQ-UI-QUALITY-GATE-002.

## M2 — Instrumentation strategy

### M2-A — `tracing` spans on widget draw lifecycle

REQ trace: **REQ-UI-INSTRUMENTATION-001**.

- [ ] **T-M2-A-1** *(developer)* — Annotate widget `draw` + `layout`
  impls with `#[tracing::instrument(skip(self, ...))]`, gated behind
  a new `render-debug` Cargo feature.
  _Acceptance criteria:_
  - File:line of change: ~30 widget impls under
    [`crates/ui/src/widgets/`](../../crates/ui/src/widgets/) get
    a `#[cfg_attr(feature = "render-debug", tracing::instrument(...))]`
    attribute on `fn draw` (+ `fn layout` for the impls that have
    one).
  - Cargo.toml: new `render-debug = []` feature stanza in
    [`crates/ui/Cargo.toml`](../../crates/ui/Cargo.toml).
  - Documentation block in
    [`crates/ui/src/lib.rs`](../../crates/ui/src/lib.rs) explaining
    how to use the feature:
    `RUST_LOG=ui::widgets=trace cargo run --bin cockpit --features fixtures,render-debug`.
  - Test command: `cargo build -p ui --features fixtures,render-debug`.
  - Test output line: clean compile, zero new warnings.
  - Trace row satisfied: REQ-UI-INSTRUMENTATION-001.

### M2-B — `DebugRenderer` newtype wrapping `iced_tiny_skia::Renderer`

REQ trace: **REQ-UI-DEBUG-RENDERER-001**.

- [ ] **T-M2-B-1** *(developer)* — Author `DebugRenderer` newtype.
  _Acceptance criteria:_
  - File:line of change: new file
    `crates/ui/src/widgets/debug_renderer.rs` (~120 LOC) wrapping
    `iced_tiny_skia::Renderer`, intercepting `fill_quad` to assert
    `quad.bounds.width > 0.0 && quad.bounds.height > 0.0` before
    delegate; on zero-dim, emit a `tracing::error!` with the full
    `Quad` payload and the current widget context (thread-local
    `Cell<&'static str>` set by M2-A's instrumented `draw` calls).
  - Cargo.toml: gated behind the same `render-debug` feature as
    M2-A.
  - Test command: `cargo build -p ui --features fixtures,render-debug --bin cockpit`.
  - Test output line: clean compile.
  - Trace row satisfied: REQ-UI-DEBUG-RENDERER-001.

## M_FINAL — Tester gate + presenter handoff

Test-runner + evaluator split per
[`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split).

- [x] **T-M_FINAL-1** *(test-runner)* — Run full test matrix on the
  post-M0-FIX + M1 + M2 branch.
  _Ticked 2026-05-14 (orchestrator post-operator-approval):_ M1+M2 deferred to follow-up brief per operator decision; test-runner ran the post-M0-FIX matrix only at [`spec/cockpit-render-regression/reports/test-run-2026-05-14T17-15Z.log`](reports/test-run-2026-05-14T17-15Z.log). All 13 commands exit 0 (per evaluator's 12/12 PASS at [`reports/evaluation-2026-05-14T17-15Z.md`](reports/evaluation-2026-05-14T17-15Z.md)); 267/267 tests pass × 2 runs.
  _Acceptance criteria:_
  - Pre-condition: T-M0-FIX-VERIFY ticked green (panic gone).
  - Command: `cargo test -p ui` + `cargo test -p ui --test
    render_snapshots` + `cargo test -p ui --test layout_invariants`
    + `cargo build -p ui --bin cockpit --features fixtures` +
    `cargo build -p ui --bin viewer` + invoke the new
    `cockpit-smoke` skill.
  - All commands exit 0; zero `*.snap.new` files; baseline PNGs
    SSIM ≥ 0.99 unanimously; no panic in cockpit-smoke.
  - Report file: `spec/cockpit-render-regression/reports/test-<timestamp>-cockpit-render-regression.md`.
  - Trace rows satisfied: REQ-COCKPIT-PANIC-001, REQ-UI-QUALITY-GATE-001,
    REQ-UI-QUALITY-GATE-002, REQ-UI-INSTRUMENTATION-001, REQ-UI-DEBUG-RENDERER-001.

- [x] **T-M_FINAL-2** *(evaluator)* — Emit VERDICT.
  _Ticked 2026-05-14 (evaluator):_ VERDICT → PASS at [`reports/evaluation-2026-05-14T17-15Z.md`](reports/evaluation-2026-05-14T17-15Z.md), log sha256 `1d7a305a6e3f89673906072cee22407861db08099252413038301ef4170dc847`. All 12 evaluation criteria green; orchestrator's cockpit smoke (`/tmp/cockpit-postrefactor.log`) shows 0 panics. M1+M2 criteria N/A for this scope (deferred).
  _Acceptance criteria:_
  - Verdict file: `spec/cockpit-render-regression/reports/evaluation-<timestamp>-cockpit-render-regression.md`.
  - Verdict: PASS iff (a) cockpit-smoke skill green, (b) all
    render_snapshots green (zero SSIM regressions), (c) all
    layout_invariants green (no zero-Node falsifications), (d)
    existing 267 panel_snapshots still green (no regression in
    the original gate during the M1-B migration), (e) anchors
    PASS 11/11 (trivial — this brief touches zero anchor code).
  - On PASS: HANDOFF → presenter.

- [x] **T-M_FINAL-3** *(presenter, after PASS)* — Assemble
  `spec/cockpit-render-regression/presentations/cockpit-render-regression-<date>.md`
  _Ticked 2026-05-14 (presenter, then operator-approved):_ Presentation at [`presentations/cockpit-render-regression-2026-05-14.md`](../archive/presentations-2026-Q2.tar.gz); operator ticked `APPROVE` on 2026-05-14. M2-A tracing snippet deferred to follow-up M1/M2 brief.
  via the [`present-results`](../../.claude/skills/present-results/SKILL.md)
  skill. Capture cockpit screenshots (clean boot) + verification
  matrix + the M2-A trace log snippet (showing the widget-by-widget
  draw spans) for the operator approval block.

## Notes

- **Read order for orchestrator:** [`feature.md ## M0`](feature.md#m0--hypothesis-register-orchestrator-runnable-falsifiers)
  first (8 hypotheses with falsifiers, ordered cheapest-first);
  then this tasks.md.
- **Anchor risk: zero.** Brief touches `crates/ui/` only —
  zero strategy / audit / exec / backtest paths. The 11 backtest
  body-SHA-256 anchors in
  [`spec/anchors.toml`](../anchors.toml) are not in scope.
- **PNG baseline impact: zero on existing baselines.** The 3
  `charts_screen_dark_*.png` baselines stay byte-identical. M1-B
  *adds* new PNG baselines under
  `crates/ui/tests/visual-baselines/render_snapshots/`.
- **Capability boundary:** the M0 falsifiers all require
  `cargo run --bin cockpit with a live window` — orchestrator
  territory only per [`AGENT.md ## Capability boundaries`](../../AGENT.md#capability-boundaries).
  Sub-agents (the architect) do NOT run them.
- **Honest-tick discipline ([`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a) rule 1):**
  every T-M*-N tick MUST cite file:line + test command + test-output
  line. Reports under `spec/cockpit-render-regression/reports/`
  are the durable audit trail.

## Changelog

- 2026-05-14 (architect, v0.2.0): M0 results integrated.
  T-M0-H1 / T-M0-H2 ticked UNFALSIFIED; T-M0-H3 ticked CONFIRMED
  (with its empty-rows assumption FALSIFIED — fixtures pre-populate
  rows so panic occurs on populated path); T-M0-H4 / T-M0-H5 /
  T-M0-H6 / T-M0-H7 / T-M0-H8 ticked obsoleted-by-construction
  (M0 bisect via shell/screen-body bypass ruled out their
  descendants). New M0-FIX milestone with T-FIX-1 → T-FIX-5
  ordered smallest-blast-radius first plus T-M0-FIX-VERIFY for
  the developer cleanup pass. Status bumped to in-progress;
  version bumped to 0.2.0. HANDOFF → orchestrator (execute F1
  → F2 → F3 → optional F4 → F5 last resort).
- 2026-05-14 (architect): initial tasks.md v0.1.0. Broke the brief
  into M0 (8 orchestrator-runnable falsifiers, cheapest-first), M1-A
  (cockpit-smoke skill, ~0.25 dev-day), M1-B (render_snapshots via
  Simulator + Headless, ~2.5 dev-days), M1-C (proptest layout
  invariants, ~1.5 dev-days), M2-A (tracing spans, ~0.75 dev-day),
  M2-B (DebugRenderer newtype, ~1 dev-day), and M_FINAL (tester
  split + presenter handoff). Total brief budget ~6.5 dev-days.
  HANDOFF → orchestrator (execute M0 falsifiers in order; route
  to developer once culprit pinned).
