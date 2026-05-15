---
slug: cockpit-render-regression
mode: release
status: draft
audience: human-operator
updated: 2026-05-14
generated: 2026-05-14T19:30:00Z
predecessor: iced-native-widgets v0.1.0 (shipped 2026-05-13)
sibling: iced-aw-cherry-pick v1.0.0 (in-progress; unblocked by this ship)
verdict_source: spec/cockpit-render-regression/reports/evaluation-2026-05-14T17-15Z.md
verdict_log_sha256: 1d7a305a6e3f89673906072cee22407861db08099252413038301ef4170dc847
---

# cockpit-render-regression v1.0.0 — release

## TL;DR

- **The panic.** `cargo run -p ui --bin cockpit --features fixtures` aborted on
  the first frame (exit 134) inside
  `iced_tiny_skia-0.14.0/src/engine.rs:686` —
  `tiny_skia::Rect::from_xywh(x, y, 0.0, 0.0).expect("Build quad rectangle")`.
  The renderer clamps every corner radius to `min(width/2, height/2)`, so a
  zero-height `Quad` lands on the all-radii-zero branch where `tiny_skia::Rect`
  rejects non-positive dimensions and `.expect` aborts. The panic crosses the
  Objective-C boundary at `WinitView::draw_rect`, which cannot unwind, so the
  whole process dies.
- **The bisect.** Orchestrator-run M0 ladder (8 hypotheses, cheapest-first)
  walked H1 → H2 → shell/screen/widget bypass. H1 (right-rail) and H2
  (`.height(0)` Spaces) came back UNFALSIFIED. Bypass-the-body bisect pinned
  the culprit to **Brief A's** `id_cell` rule Container in
  `crates/ui/src/widgets/strategies.rs:217-227` — a styled `Container::new(Space)`
  with `height(Length::Fill)` resolving to `0.0` inside an iced Table cell on
  the first frame. H3 confirmed; its original "empty-rows" assumption falsified
  (fixtures pre-populate rows, so the panic happens on the populated path).
- **The F1 fix.** Replace `Length::Fill` on the rule with
  `Length::Fixed(STRATEGY_RULE_HEIGHT_PX)`, where the new named constant
  `pub const STRATEGY_RULE_HEIGHT_PX: f32 = 24.0` lives in
  `crates/ui/src/theme.rs:619` with a `///`-doc explaining the all-radii-zero
  failure mode. **Net file-span: 4 LOC** in `widgets/strategies.rs` (two
  `Length::Fixed` swaps + `use` + doc-comment line); **glue-layer: 28 LOC**
  in `theme.rs` (the constant + its `///`-doc). F1 was the first candidate
  tried and falsified the panic on the first run — F2–F5 never executed.
- **Verification.** Two consecutive `cargo test -p ui` runs both report
  `267 passed; 0 failed` (deterministic, zero `*.snap.new`). Anchors diff
  empty (11 / 11 byte-identical — this brief touches zero anchored code).
  Orchestrator-produced cockpit-smoke log `/tmp/cockpit-postrefactor.log`
  shows panic count = **0** (vs **2** in the pre-fix `/tmp/cockpit-runtime.log`
  baseline). Evaluator emitted **12 / 12 PASS** in
  `evaluation-2026-05-14T17-15Z.md` (body-SHA-256
  `1d7a305a6e3f89673906072cee22407861db08099252413038301ef4170dc847`).
- **Brief B unblocked.** With F1 landed, the cockpit renders cleanly with
  both Brief A native-widgets AND Brief B (`iced-aw-cherry-pick`) widgets
  active. Brief B's prior evaluator-PASS verdict (body-SHA-256
  `30906659…f97d2`) stands. Brief B's v1.0.0 presentation at
  [`spec/iced-aw-cherry-pick/presentations/iced-aw-cherry-pick-2026-05-14.md`](../../iced-aw-cherry-pick/presentations/iced-aw-cherry-pick-2026-05-14.md)
  is now ready for operator approval in parallel with this one.

## What changed

### Glue-layer LOC

| Surface | LOC delta |
|---|---:|
| [`crates/ui/src/theme.rs:619`](../../../crates/ui/src/theme.rs) — new `pub const STRATEGY_RULE_HEIGHT_PX: f32 = 24.0` + `///`-doc explaining the all-radii-zero / Length::Fill / Table-cell interaction | **+28** |
| **Glue-layer total** | **+28** |

### File-span LOC

| File | LOC delta |
|---|---:|
| [`crates/ui/src/widgets/strategies.rs`](../../../crates/ui/src/widgets/strategies.rs) — `use` extension for `layout::STRATEGY_RULE_HEIGHT_PX`, two `.height(Length::Fixed(layout::STRATEGY_RULE_HEIGHT_PX))` swaps at `:228` (inner `Space`) and `:231` (outer `Container`), plus a tightened doc-comment WHY block at `:222` (replacing the orchestrator's bisect-residual `// F1 FALSIFIER 2026-05-14 — was Length::Fill`) | **+4** |
| **File-span total** | **+4** |

### Aggregate

**~32 LOC total** (4 file-span + 28 glue) — a surgical fix. No Cargo.toml
change, no feature-flag, no ADR required. Affected files: exactly two
(`crates/ui/src/widgets/strategies.rs` and `crates/ui/src/theme.rs`). Brief A
and Brief B source remain untouched.

## Why the headless tests missed it

**This is the load-bearing failure mode this presentation must surface.**

The 267 panel-snapshot tests at
[`crates/ui/tests/panel_snapshots.rs:1779-2298`](../../../crates/ui/tests/panel_snapshots.rs)
**do not render the iced widget tree**. They invoke text-summary helpers
(`tape_summary`, `positions_summary`, `strategies_summary`) that walk the
`Cockpit` state struct and emit `PanelState`-keyed `String` blocks. The widget
construction path — the path that contains the zero-dim `fill_quad` — is
never exercised.

Brief B's developer surfaced this same gap honestly in
[`spec/iced-aw-cherry-pick/tasks.md`](../../iced-aw-cherry-pick/tasks.md)
T-M2-3 / T-M3-3 ("zero snapshot bytes changed across the
`muted_body → loading_with_spinner` + `colored_cell → Badge` swaps; the helpers
route via `strings::*` regardless of the iced widget underneath"). At the time
that read like a determinism win. After this regression it reads like the
warning sign it was: **real-iced-renderer coverage of the cockpit's render
surface is ~0%** today.

The single gate that DID catch the bug was the orchestrator-only
`cargo run --bin cockpit --features fixtures` cockpit-smoke step — but it ran
**once, by hand, after presenter handoff for Brief B**. The right gate for
this class of bug exists; it was just not mandatory. M1-A turns it into a
mandatory pre-tick gate (see "What's next" below).

## Demo evidence

Verbatim from
[`reports/test-run-2026-05-14T17-15Z.log`](../reports/test-run-2026-05-14T17-15Z.log)
— the test-runner's raw log evaluated at body-SHA-256
`1d7a305a6e3f89673906072cee22407861db08099252413038301ef4170dc847`.

### Compile gates green (4 binaries, cmd 1-5)

```
## cargo fmt -p ui --check
## exit: 0
## cargo build -p ui --tests
## exit: 0
## cargo build -p ui --bin viewer
## exit: 0
## cargo build -p ui --bin cockpit --features fixtures
## exit: 0
## cargo build -p ui --bin cockpit_live --features live
## exit: 0
```
(log L1–L53)

### Cmd 6 — `cargo test -p ui` run 1 (267 / 267 PASS)

```
test result: ok. 154 passed; 0 failed; 0 ignored; ...   # lib (L248)
test result: ok. 69 passed;  0 failed; 0 ignored; ...   # panel_snapshots (L435)
... (21 more `test result: ok.` lines, summing to 267)
## exit: 0                                              # L495
```

### Cmd 7 — `cargo test -p ui` run 2 + zero-snap.new gate

```
... 23 `test result: ok` blocks summing to 267 passed ...
## exit: 0                                              # L937
## find crates/ui -name *.snap.new
## exit: 0                                              # L940 (empty output)
```

Two-run determinism confirmed. Zero `*.snap.new` files.

### Cmd 12 — cockpit smoke (grep on the orchestrator-produced post-refactor log)

```
## ls -la /tmp/cockpit-postrefactor.log
-rw-r--r--  1 Vitaliy.Schreibmann  wheel  108 May 14 19:14 /tmp/cockpit-postrefactor.log
## exit: 0

## grep -c panicked at|non-unwinding panic /tmp/cockpit-postrefactor.log
0
## exit: 1
```
(log L1555–L1561)

**Panic count: 0.** Exit 1 from `grep -c` is the standard "no matches" signal,
NOT a criterion failure — the criterion text anchors on the printed count
which is `0`. The pre-fix baseline at `/tmp/cockpit-runtime.log` printed `2`
(one `panicked at` + one `non-unwinding panic`); the post-refactor log is
clean. A second independent orchestrator log
(`/tmp/cockpit-f1-falsifier.log`) reports the same `0`.

### Cmd 13 — named-constant wired in both files

```
## grep -n STRATEGY_RULE_HEIGHT_PX crates/ui/src/theme.rs crates/ui/src/widgets/strategies.rs
crates/ui/src/theme.rs:619:    pub const STRATEGY_RULE_HEIGHT_PX: f32 = 24.0;
crates/ui/src/widgets/strategies.rs:222:    // `crate::theme::layout::STRATEGY_RULE_HEIGHT_PX` for the WHY, and
crates/ui/src/widgets/strategies.rs:228:            .height(Length::Fixed(layout::STRATEGY_RULE_HEIGHT_PX)),
crates/ui/src/widgets/strategies.rs:231:    .height(Length::Fixed(layout::STRATEGY_RULE_HEIGHT_PX))
## exit: 0
```
(log L1563–L1568)

1 definition (theme.rs:619) + 2 use sites (strategies.rs:228 + :231) + 1 doc
reference (strategies.rs:222). Refactor landed cleanly.

## Verification matrix

Verbatim from
[`evaluation-2026-05-14T17-15Z.md`](../reports/evaluation-2026-05-14T17-15Z.md).
Log body-SHA-256:
`1d7a305a6e3f89673906072cee22407861db08099252413038301ef4170dc847`. All 12 rows PASS.

| # | Criterion | Result | Cite |
|---|-----------|--------|------|
| 1 | Compile gates green (cmds 2–5 cargo build x4) | PASS | log L4–L53; all four `## exit: 0` |
| 2 | Test suite green; N ≥ 267 (cmd 6) | PASS | log L495 `## exit: 0`; sum of 23 `test result: ok` lines L248–L493 = **267 passed, 0 failed** |
| 3 | Two-run determinism: cmd-6 N == cmd-7 N AND zero `*.snap.new` | PASS | run-2 sum L690–L935 = **267 passed**; cmd-7 `find … -name *.snap.new` L939–L940 empty, exit 0 |
| 4 | Fmt clean (cmd 1) | PASS | log L1–L2 `cargo fmt -p ui --check` exit 0 |
| 5 | Clippy: zero NET-NEW errors in F1-touched files (theme.rs / widgets/strategies.rs) | PASS | 6 errors L1417–L1486, all in `widgets/chart.rs` (5) + `window_icon.rs` (1) — documented pre-existing; **0 errors in theme.rs or strategies.rs** |
| 6 | Rustdoc: zero NET-NEW warnings on F1-touched files (cmd 8) | PASS | 6 rustdoc warnings L943–L985 cite `chart_tooltip.rs`, `volume_histogram.rs`, `window_icon.rs`, `test_support.rs` — all unrelated; theme.rs / strategies.rs absent |
| 7 | Clocks-determinism gate green (cmd 10) | PASS | log L1549 `CLOCKS PASS  (8 files / 4 patterns)`, L1550 exit 0 |
| 8 | Anchor diff empty (cmd 11) | PASS | log L1552–L1553 `git diff --stat HEAD spec/anchors.toml` produced no output, exit 0 |
| 9 | Cockpit smoke clean — cmd 12 grep returns 0 | PASS | log L1559–L1561 grep `-c` printed **`0`** (exit 1 is grep's "no matches" signal with `-c`, not a criterion failure) |
| 10 | Named constant landed in both files (cmd 13) | PASS | log L1563–L1568: `theme.rs:619` definition + 3 hits in `strategies.rs` (L222 doc, L228 + L231 use sites) |
| 11 | trace.toml columns filled for REQ-COCKPIT-PANIC-001 | PASS | `spec/trace.toml` L350–L369: `crates` = `[widgets/strategies.rs, theme.rs]`; `tests` = 4 entries — both non-empty |
| 12 | Honest ticks — T-FIX-1 and T-M0-FIX-VERIFY each cite (a) file:line, (b) test cmd, (c) test-output line | PASS | tasks.md L148–L160 + L322–L350 — both provide (a)/(b)/(c) |

**12 / 12 PASS.**

## Architectural divergences (honest)

Per user-memory `feedback_research_brief_framing.md`: name anywhere this ship
contradicts prior thinking or AGENT.md guidance.

1. **The architect's H3 assumed an empty-state path; the bisect proved the panic
   happens with populated rows.** Original H3 statement claimed the trigger was
   the empty-`rows` slice path before fixtures populate. Orchestrator-confirmed
   evidence: cockpit fixtures pre-populate `cockpit.strategies` via
   `Message::BarReceived` at
   [`crates/ui/src/bin/cockpit.rs:161-166`](../../../crates/ui/src/bin/cockpit.rs)
   on the first frame, so `ready_body` receives a non-empty `rows`. The fix
   design pivoted accordingly: F1 abandoned the proposed `if rows.len() > 0`
   defensive gate and instead pinned the rule Container's height directly. Named
   so future architect passes see that **confirmed-hypothesis ≠
   confirmed-mechanism.**

2. **Positions / Strategies asymmetry — same widget, only strategies panics.**
   [`widgets/positions.rs:122`](../../../crates/ui/src/widgets/positions.rs)
   calls `table::Table::new(columns, visible_iter)` with the same widget shape
   and does not panic in isolation; only strategies' Table does. The difference
   is cell-content composition — strategies' column 1 (`id_cell`) wraps an inner
   `Space` inside a styled `Container` with `height(Length::Fill)`, whereas
   positions cols 1–7 use plain `cell` / `colored_cell` Text widgets with no
   `Length::Fill` inside a Container/Space. **Brief A R1 (positions) is fine;
   Brief A R2's `id_cell` pattern was the actual zero-dim source.** Brief A's
   structural decision (adopt `iced::widget::table::Table` for both) stands —
   only the `id_cell` rule binding needed pinning.

3. **F2–F5 candidates were NOT executed (F1 falsified the panic on first try).**
   The M0-FIX falsifier ladder committed five candidates (F1 named-constant,
   F2 stock `vertical_rule`, F3 zero-thickness separators, F4 Themer wrap
   diagnostic, F5 Brief A R2 partial revert + ADR). F1's first run produced a
   7-second clean cockpit boot; the orchestrator stopped on first FALSIFIED per
   the committed ladder discipline. F2–F5 are marked `[~]` obsoleted-by-F1 in
   [`tasks.md`](../tasks.md) and retained for spec-history only. No ADR
   required.

4. **Pre-existing clippy / rustdoc / unused-import noise (6 + 6 + 5 issues)
   surfaced but out of scope for THIS ship.** 6 clippy errors in
   `widgets/chart.rs` + `window_icon.rs` (all `expect_used` / `unwrap_used`),
   6 rustdoc broken-intra-doc-links across
   `chart_tooltip.rs` / `volume_histogram.rs` / `window_icon.rs` /
   `test_support.rs`, and 5 unused-import warnings in
   `tests/strategies_screen_sparkline_replaces_placeholder.rs`. All pre-date
   Brief B. The criterion 5/6 verdict text in the evaluation matrix confirms
   ZERO NET-NEW issues on F1-touched files. Disposition surfaced as an Open
   decision below.

## Brief B unblock

Brief B (`iced-aw-cherry-pick` v1.0.0) was held with `status: in-progress`
pending this fix. With F1 landed:

- The cockpit boots cleanly with both Brief A native-widgets AND Brief B
  `iced_aw` widgets (spinner, badge, date_picker) active — verified by the
  orchestrator's post-refactor cockpit-smoke log
  (`/tmp/cockpit-postrefactor.log`, panic count 0). The earlier bisect already
  confirmed that commenting out the `iced_aw::Spinner` and `iced_aw::Badge`
  call sites did NOT clear the panic (`spec/cockpit-render-regression/feature.md`
  ## What we know is NOT the trigger), so Brief B is structurally independent
  of the regression.
- Brief B's own evaluator-PASS verdict at
  [`spec/iced-aw-cherry-pick/reports/evaluation-2026-05-14T07-13Z.md`](../../iced-aw-cherry-pick/reports/evaluation-2026-05-14T07-13Z.md)
  (10 / 10 rows PASS, body-SHA-256 `30906659…f97d2`) stands; nothing in Brief B
  needs to be re-run.
- Brief B's presentation at
  [`spec/iced-aw-cherry-pick/presentations/iced-aw-cherry-pick-2026-05-14.md`](../../iced-aw-cherry-pick/presentations/iced-aw-cherry-pick-2026-05-14.md)
  is **ready for operator approval in parallel with this one**. Operator may
  tick both approval blocks in a single review pass.

This presentation does NOT re-derive or modify Brief B's content — it
references and unblocks.

## What's next — M1 / M2 quality-gate brief (queued, scope already ratified)

Out of scope for THIS ship. Operator ratified the scope and parameters earlier
in this session; queuing as a follow-up brief subject to operator green-light
in the approval block. Per
[`tasks.md ## M1 / ## M2`](../tasks.md):

| Milestone | Surface | Sizing |
|---|---|---:|
| **M1-A — `cockpit-smoke` skill** (mandatory pre-tick gate, always-on cadence) | New `.claude/skills/cockpit-smoke/SKILL.md` + AGENT.md gate clause. Standardized log path under `spec/<slug>/reports/cockpit-smoke-<ts>.log`. | ~0.25 dev-day |
| **M1-B — real-renderer snapshot tests** (≥0.99 SSIM threshold) | `iced_test::Simulator` + `iced::advanced::renderer::Headless` + `image-compare`. Replaces ~244 text-summary `*_summary` helpers with real PNG-baseline diffs. | ~2.5 dev-days |
| **M1-C — `proptest` layout invariants** | `widget.as_widget().layout(...)` with fuzzed inputs; `prop_assert!` on `Node::size().{width, height} > 0.0` (or explicit-NaN). Covers 6 widgets implicated by M0. | ~1.5 dev-days |
| **M2-A — `tracing` spans on widget `draw` / `layout`** | `#[cfg_attr(feature = "render-debug", tracing::instrument(...))]` on ~30 impls + new `render-debug` Cargo feature. | ~0.75 dev-day |
| **M2-B — optional `DebugRenderer` newtype** | Wraps `iced_tiny_skia::Renderer`, intercepts `fill_quad`, emits `tracing::error!` with widget context instead of the bare `Build quad rectangle` panic. Gated behind `render-debug` feature. | ~1 dev-day |
| **M2-C — LLM-as-judge for semantic visual diff** | **DEFERRED** to a separate brief — non-determinism risk requires its own design pass. | _deferred_ |

**Total queued (M1 + M2-A/B):** ~6 dev-days. **M2-C deferred.** Surface here so
the operator can ratify the M1/M2 brief launch in an
"approve with notes" tick if desired.

## Screenshots

The presenter sub-agent cannot run the cockpit binary with a live window or
invoke `screencapture` (per
[`AGENT.md ## Capability boundaries`](../../../AGENT.md)). Two
operator-instruction blocks below — please paste the resulting PNGs back into
this presentation during your review pass.

Output directory: `spec/cockpit-render-regression/reports/screenshots/`
(auto-created by the `mkdir -p` in the snippets).

### Screenshot 1 — Strategies panel (visible badge column + ID column with the now-non-zero rule)

```bash
# On your operator workstation, capture the Strategies panel:
mkdir -p spec/cockpit-render-regression/reports/screenshots
cargo run -p ui --bin cockpit --features fixtures &
sleep 4
# Click into the Home screen → Strategies panel; the ID column's vertical
# rule (column 1) renders as a 2 px × 24 px vertical accent stripe per active row.
screencapture -W spec/cockpit-render-regression/reports/screenshots/strategies-panel-post-f1.png   # macOS
pkill -f "target/debug/cockpit"
```

Caption when pasted back: _"Strategies panel post-F1. Column 1 = ID rule
binding (`Length::Fixed(STRATEGY_RULE_HEIGHT_PX = 24.0)`); column 2 = ID;
column 3 = Brief B's STATUS Badge chip (Ready / Loading / Error intent). The
rule renders as a 2 px × 24 px vertical accent stripe when `is_active`, and as
a transparent-tinted stripe otherwise — both panic-free."_

### Screenshot 2 — Loading spinner + text Row (Brief B B2 evidence)

```bash
# On your operator workstation, capture a loading spinner+text row:
mkdir -p spec/cockpit-render-regression/reports/screenshots
cargo run -p ui --bin cockpit --features fixtures &
sleep 4
# Wait for any panel still in PanelState::Loading (typically Positions or
# Strategies during fixture flush, or trigger via a fixture flag).
screencapture -W spec/cockpit-render-regression/reports/screenshots/loading-spinner-row-post-f1.png   # macOS
pkill -f "target/debug/cockpit"
```

Caption when pasted back: _"Brief B B2 — `loading_with_spinner(text, mode)`
Row rendering cleanly with F1 landed. 16 px `iced_aw::Spinner` paired with
the informational `Loading…` copy. Cross-brief evidence that Brief B widgets
co-exist with the F1-fixed Brief A `id_cell` rule binding without re-triggering
the panic."_

## Open decisions for the operator

1. **M1 / M2 follow-up brief — start now or wait?** Scope ratified earlier
   this session (M1-A cockpit-smoke skill, M1-B real-renderer snapshots,
   M1-C proptest invariants, M2-A tracing spans, M2-B optional debug renderer;
   M2-C LLM-as-judge deferred). ~6 dev-days total. Default if no tick: queue
   as the next analyst brief after Brief B approval; orchestrator launches
   when operator says go.

2. **Should `spec/iced-native-widgets/feature.md` get a post-ship erratum
   changelog entry pointing at this F1 fix?** Brief A is frozen per process
   (user-memory `trading_ui_iced_adoption_state.md`: "Brief A shipped … v0.1.0"
   — no further edits without operator approval). The `id_cell` rule binding
   was a Brief A R2 detail; an erratum-pointer in Brief A's changelog would
   close the knowledge loop for any future re-reader of Brief A who otherwise
   wouldn't know the rule binding was patched. **Asking explicit operator
   approval to amend Brief A's frontmatter / changelog with a one-line erratum
   pointer.** Default if no tick: do NOT amend Brief A; this presentation +
   `cockpit-render-regression/feature.md` carry the full diagnosis.

3. **`/tmp/cockpit-*.log` path discipline.** Two orchestrator-produced logs
   under this brief (`/tmp/cockpit-f1-falsifier.log` and
   `/tmp/cockpit-postrefactor.log`) both confirm panic-count = 0, but they
   live under `/tmp` rather than `spec/<slug>/reports/`. Should the M1-A
   cockpit-smoke skill **standardize** the log path under
   `spec/<slug>/reports/cockpit-smoke-<ts>.log` for future runs (preserving
   durable audit trail vs `/tmp` ephemerality)? Default if no tick: M1-A
   ships the standardized path when the M1 brief launches.

4. **Pre-existing clippy / rustdoc / unused-import noise** (6 clippy errors
   in `widgets/chart.rs` + `window_icon.rs`, 6 rustdoc warnings across
   `chart_tooltip.rs` / `volume_histogram.rs` / `window_icon.rs` /
   `test_support.rs`, 5 unused-import warnings in
   `tests/strategies_screen_sparkline_replaces_placeholder.rs`) — clean-up
   brief now, or fold into M1? Pre-dates Brief B; not blocking. Default if
   no tick: fold into M1 (cleanup happens before M1-B's bulk snapshot
   migration touches the same files).

## Numbers that matter

- **Tests:** **267 passed; 0 failed** across 23 binaries, two consecutive runs
  byte-identical (log L495 run 1, log L937 run 2). Test count delta: **0**.
- **Snapshot determinism:** **0** `*.snap.new` files after the two-run gate
  (log L939–L940 `find … -name *.snap.new` empty body exit 0).
- **Anchors:** **11 / 11 byte-identical**. Brief touches zero strategy / audit /
  exec / backtest paths. `spec/anchors.toml` diff empty (log L1552–L1553).
- **Cockpit-smoke panic count (post-refactor):** **0** (vs **2** in pre-fix
  baseline). Cmd 12 grep at log L1559–L1561.
- **LOC delta:** **+4 file-span + +28 glue = +32 total**. Two files touched.
- **Clippy NET-NEW on F1-touched files:** **0** (theme.rs + widgets/strategies.rs
  clean; pre-existing 6 errors confined to `chart.rs` + `window_icon.rs`).
- **Rustdoc NET-NEW on F1-touched files:** **0** (pre-existing 6 warnings
  unrelated).
- **REQ trace rows filled:** REQ-COCKPIT-PANIC-001 — both `crates` and
  `tests` columns populated (`spec/trace.toml:350-369`).
- **Evaluator verdict matrix:** **12 / 12 PASS**, log body-SHA-256
  `1d7a305a6e3f89673906072cee22407861db08099252413038301ef4170dc847`.

## Operator approval — please tick one

- [x] APPROVE — ship cockpit-render-regression v1.0.0 + unblock Brief B
- [ ] APPROVE WITH NOTES — feedback below; addressed in follow-up
- [ ] REJECT — route to <agent>, reason below

Notes/feedback:

_empty until operator fills_

## Changelog

- 2026-05-14 (presenter): initial release-mode presentation drafted after
  evaluator's `VERDICT → PASS` at
  [`reports/evaluation-2026-05-14T17-15Z.md`](../reports/evaluation-2026-05-14T17-15Z.md)
  (log body-SHA-256
  `1d7a305a6e3f89673906072cee22407861db08099252413038301ef4170dc847`).
  TL;DR covers the panic (file:line + tiny-skia clamp explanation), the M0
  bisect narrowing to Brief A's `id_cell` rule, the F1 named-constant fix,
  the 267/267 + smoke verification, and the Brief B unblock. What-changed
  splits file-span (+4 LOC) vs glue-layer (+28 LOC) per user-memory
  `feedback_research_brief_framing.md`. "Why headless tests missed it"
  section surfaces the load-bearing failure mode: `panel_snapshots` text-
  summary helpers route via `strings::*` and never exercise the iced
  widget tree. Demo-evidence section embeds verbatim log excerpts (cmd 12
  cockpit-smoke grep + cmd 6 / 7 test summaries + cmd 13 named-constant
  grep). Architectural-divergences section names H3 assumption-falsified,
  positions/strategies asymmetry, F2-F5-not-executed, and pre-existing
  clippy/rustdoc/unused-import noise. M1/M2 follow-up brief framed as
  "out-of-scope for THIS ship, queued" with the operator's earlier-ratified
  scope (M1-A skill + M1-B SSIM snapshots + M1-C proptest + M2-A tracing +
  M2-B debug renderer; M2-C deferred). Verification matrix lifts the
  evaluator's 12-row PASS table verbatim. 2 operator-instruction screenshot
  blocks emitted in lieu of in-sandbox capture (presenter cannot run
  `cargo run --bin cockpit` per
  [`AGENT.md ## Capability boundaries`](../../../AGENT.md)). 4 open
  decisions surfaced (M1/M2 brief launch, Brief A erratum-pointer, log
  path discipline, pre-existing noise cleanup). 3 approval boxes ship
  UN-TICKED — operator owns the gate. Frontmatter on
  [`feature.md`](../feature.md) bumped `version: 0.3.0 → 1.0.0` and
  `updated: 2026-05-14` in the sibling spec-update pass; `status` stays
  `in-progress` until operator approval flips it to `shipped` (orchestrator
  owns that flip per AGENT.md process discipline rule 2).
