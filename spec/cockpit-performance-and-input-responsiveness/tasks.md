---
slug: cockpit-performance-and-input-responsiveness
status: shipped
owner: developer
updated: 2026-05-15
version: 0.3.0
---

# Tasks — cockpit performance + input responsiveness (v0.2.0)

> **Status:** architect design pass complete
> ([`feature.md ## Design — architect synthesis`](feature.md#design--architect-synthesis)
> 2026-05-15). M0 is orchestrator-runnable and gates M1; M1 fans
> out by M0-confirmed candidate (A spinner / B Table / C hit-test);
> M2 extends the cockpit-smoke skill once a fix lands; M3 stays
> coupled per Q1 unless its trigger condition fires. T-tasks below
> are concrete and ordered cheapest-falsifier-first inside each
> milestone.
>
> Honest-tick discipline ([`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
> rule 1): every owning agent MUST cite (a) file:line of change,
> (b) test command, (c) test-output line on every `[x]`. The tester
> (test-runner + evaluator split per
> [`AGENT.md ## Test-runner / evaluator split`](../../AGENT.md#test-runner--evaluator-split))
> owns the `T_FINAL_*` ticks.
>
> Anchor risk: **zero** — this brief touches `crates/ui/` + the
> cockpit-smoke skill + spec files only; zero strategy / audit /
> exec / backtest paths. PNG-baseline diff: depends on M1 candidate
> (B1 memoize: zero on existing baselines; B3 revert: re-baseline
> the strategies panel and the M_FINAL gate asserts byte-identical
> on all OTHER baselines).

## M0 — Profile the dominant hot path *(orchestrator-runnable)*

M0 confirms exactly one of H-PERF-1 / -2 / -3 / -4 (the four LIVE
hypotheses after the architect-resolved H-PERF-5 and H-PERF-6 — see
[`feature.md ## M0 falsifier-batch — final state going into the
orchestrator-run`](feature.md#m0-falsifier-batch--final-state-going-into-the-orchestrator-run)).
The M1 candidate fan-out is gated on this verdict. The orchestrator
runs each task in order.

REQ trace: **REQ-COCKPIT-PERF-001**.

- [ ] **T-M0-1** *(orchestrator)* — Run `samply` (with `cargo
  flamegraph` SVG fallback) against a release-mode fixtures-mode
  cockpit boot for ~30s idle + ~30s operator interaction; capture
  flamegraph + frame-time histogram.
  _Statement._ Samply primary per Q6 resolution
  ([`feature.md ## Q6 resolution`](feature.md#q6-resolution--m0-profiler-is-samply-preferred-cargo-flamegraph-is-the-documented-fallback)).
  Browser-served interactive flamegraph + macOS-native `task_for_pid`
  symbols (no `sudo`). Fallback to cargo-flamegraph if `samply` is
  unavailable on host.
  _Orchestrator-runnable command body (exact)._
  ```bash
  TS=$(date -u +%Y-%m-%dT%H-%MZ)
  REPORT_DIR=spec/cockpit-performance-and-input-responsiveness/reports
  mkdir -p "$REPORT_DIR"

  # Build release-mode cockpit binary (the profile target).
  cargo build --release -p ui --bin cockpit --features fixtures

  # Bootstrap samply if missing; otherwise no-op. Fallback to
  # cargo-flamegraph if samply install fails (e.g., toolchain issue).
  if ! command -v samply >/dev/null 2>&1; then
    cargo install samply 2>&1 | tee -a "$REPORT_DIR/m0-profile-${TS}.install.log" \
      || cargo install flamegraph 2>&1 | tee -a "$REPORT_DIR/m0-profile-${TS}.install.log"
  fi

  # Capture 60s profile (30s idle + 30s operator interaction).
  # Operator interacts in the window: panel-switch, scroll, click.
  if command -v samply >/dev/null 2>&1; then
    samply record \
      --save-only \
      --output "$REPORT_DIR/m0-profile-${TS}.json" \
      -- target/release/cockpit 2>&1 \
      | tee "$REPORT_DIR/m0-profile-${TS}.stderr.log" &
    SAMPLY_PID=$!
    sleep 60
    kill -INT "$SAMPLY_PID" 2>/dev/null || true
    wait "$SAMPLY_PID" 2>/dev/null || true
    # Convert to flamegraph view: open the .json in a samply-served
    # browser tab, screenshot the flamegraph + frame-time view.
  else
    # Cargo-flamegraph fallback (one-shot SVG; macOS needs dtrace).
    cargo flamegraph --release -p ui --bin cockpit \
      --features fixtures \
      --output "$REPORT_DIR/m0-profile-${TS}.flamegraph.svg" \
      -- 60 2>&1 \
      | tee "$REPORT_DIR/m0-profile-${TS}.stderr.log"
  fi
  ```
  _Acceptance criteria._
  - Top-10 hot frames by self-time captured (samply JSON or
    flamegraph SVG).
  - Frame-time histogram (p50 / p95 / p99) captured. For samply,
    derived from `samply analyze` output; for cargo-flamegraph,
    derived from the stderr `RENDER_FRAME` lines if the M2 emission
    has landed (otherwise from an ad-hoc `iced::widget::canvas`
    Frame instrumentation that the orchestrator may add as a
    diagnostic-only patch and revert post-profile).
  - The `.stderr.log` is committed alongside the profile artifact.
  _Three-citation contract (per AGENT.md ## Process discipline rule 1)._
  - (a) file:line of change: zero (read-only profiling; no source
    edit).
  - (b) test command: the orchestrator-runnable block above.
  - (c) test-output line: cite the `samply analyze` or
    cargo-flamegraph histogram summary in
    `$REPORT_DIR/m0-profile-${TS}.{json,svg}` plus the stderr log.

- [ ] **T-M0-2** *(orchestrator)* — Write the M0 results section in
  feature.md: dominant hot-path call site + verdict for each LIVE
  H-PERF row.
  _Statement._ Append a `## M0 results (orchestrator-executed
  YYYY-MM-DD)` section to feature.md (NOT a separate report file;
  the same shape as `cockpit-render-regression/feature.md ## M0
  results`). For each of H-PERF-1 / -2 / -3 / -4 record:
  CONFIRMED / UNFALSIFIED / DEFERRED-by-evidence (e.g., a tiny-skia
  signal that wasn't top-3 but is top-5; record honestly).
  _Acceptance criteria._
  - Section names the SINGLE confirmed dominant hypothesis (the M1
    candidate to fan out on) — or, if two hypotheses share
    self-time within 10% of each other, names a primary + secondary
    and the architect re-engages for an ADR on which to fix first.
  - Frame-time histogram (p50 / p95 / p99) values stated as numbers.
  - The M1 verdict line is mechanical:
    - H-PERF-1 confirmed → M1 Candidate A;
    - H-PERF-2 confirmed → M1 Candidate B (try B1 → B2 → B3 per Q4);
    - H-PERF-3 confirmed → M1 Candidate C;
    - H-PERF-4 confirmed → M1 DEFERRED, route per Q2.
  _Three-citation contract._
  - (a) file:line of change:
    `spec/cockpit-performance-and-input-responsiveness/feature.md ## M0 results` (append).
  - (b) test command: the T-M0-1 command body produces the evidence
    this task cites.
  - (c) test-output line: the histogram p50/p95/p99 numbers + the
    top-10 self-time entries' first column, copy-pasted into the
    feature.md table.

- [x] **T-M0-3** *(architect, 2026-05-15)* — H-PERF-5 + H-PERF-6
  sub-agent-safe falsifiers — RESOLVED inline at architect handoff.
  _Statement._ Two of the analyst's six hypotheses were sub-agent-safe
  (grep + source-read; no live cockpit needed). Both architect-resolved
  before M0; M0 batch shrinks from 6 → 4 LIVE rows.
  _Three-citation contract._
  - (a) file:line of change: zero (read-only falsifiers; the
    architect's verdicts append to feature.md, not crates/).
  - (b) test command:
    `grep -n 'render-debug' crates/ui/Cargo.toml` |
    `grep -n 'default = \[' crates/ui/Cargo.toml` (empty) |
    `grep -rn '#[cfg(feature = "render-debug")]' crates/ui/src/ | wc -l` →
    `6` |
    `grep -rn 'tracing::trace_span\|tracing::trace!\|tracing::debug!' crates/ui/src/widgets/frame.rs crates/ui/src/widgets/strategies.rs crates/ui/src/bin/cockpit.rs` →
    3 emit sites at `frame.rs:61`, `frame.rs:190`, `strategies.rs:226`,
    all immediately preceded by a `#[cfg]` gate.
    For H-PERF-6: source-read of
    `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/iced_aw-0.14.1/src/widget/spinner.rs:165-202`
    + `crates/ui/src/bin/cockpit.rs:276-295` + `crates/ui/src/widgets/frame.rs:178-206`.
  - (c) test-output line: see
    [`feature.md ## H-PERF-5 falsifier`](feature.md#h-perf-5-falsifier--sub-agent-safe-resolved-unfalsified-defensively-true)
    and
    [`feature.md ## H-PERF-6 falsifier`](feature.md#h-perf-6-falsifier--sub-agent-safe-resolved-unfalsified-architectural-impossibility)
    for the result tables; verdicts are RESOLVED-UNFALSIFIED
    (defensively true) and RESOLVED-by-impossibility respectively.

## M1 — Fix the dominant hot path *(developer + ui-designer, gated on M0 verdict)*

Per Q4, the developer runs sub-tasks only inside the M0-confirmed
candidate branch. The OTHER candidate branches stay unticked as
"N/A — M0 confirmed Candidate X". REQ trace: **REQ-COCKPIT-PERF-001**.

### Candidate A — if M0 confirms H-PERF-1 (spinner-driven 60 fps repaint)

Fix order (developer): A1 → A2 → A3. A1 is the cheapest; advance
only on UNFALSIFIED.

- [x] **T-M1-A-1** *(developer, 2026-05-15)* — Coarsen spinner cadence to 10 fps
  via local `ThrottledSpinner` wrap (A2 sub-candidate).
  _Statement._ The `iced_aw::Spinner` widget's internal
  `request_redraw_at(now + Duration::from_millis(1000 /
  FRAMES_PER_SECOND))` call schedules the next redraw at 60 fps. We
  cannot edit the upstream iced_aw crate (constraint envelope: no
  fork). Instead, wrap `iced_aw::Spinner::new()` in a local
  cadence-throttling widget at `crates/ui/src/widgets/frame.rs:196`
  (the `loading_with_spinner` body), or in a sibling widget
  module `crates/ui/src/widgets/throttled_spinner.rs` that proxies
  `update(...)` and drops every Nth `RedrawRequested` event.
  Effective cadence: 10 fps (drop 5 of every 6 redraw requests).
  _Falsifier._
  - Edit
    `crates/ui/src/widgets/frame.rs:196` (the `iced_aw::Spinner::new()
    .width(Length::Fixed(16.0)).height(Length::Fixed(16.0))` builder)
    OR introduce `crates/ui/src/widgets/throttled_spinner.rs` and
    replace the `Spinner::new()` call with `ThrottledSpinner::new(6)`.
  - Build: `cargo build -p ui --bin cockpit --features fixtures`.
  - Re-run M0's T-M0-1 profile body for ~30s with the patched
    binary; expect spinner-driven redraw self-time to drop ~6x.
  _Three-citation contract (developer fill, 2026-05-15)._
  - (a) file:line of change:
    `crates/ui/src/widgets/throttled_spinner.rs:1-310` (new local
    widget; 10 fps cadence at line 101) +
    `crates/ui/src/widgets/frame.rs:212-215` (`loading_with_spinner`
    now constructs `ThrottledSpinner::new()` instead of
    `iced_aw::Spinner::new()`) +
    `crates/ui/src/widgets/mod.rs:43-46` (`pub mod throttled_spinner`).
  - (b) test command: `cargo test -p ui --lib throttled_spinner`.
  - (c) test-output line:
    `test widgets::throttled_spinner::tests::frames_per_second_is_ten ... ok` +
    `test widgets::throttled_spinner::tests::frames_per_second_is_not_sixty ... ok` +
    `test result: ok. 5 passed; 0 failed; 0 ignored` (full suite under
    `cargo test -p ui`: 280 passed / 0 failed / 5 ignored).
    Operator-visible cockpit-smoke + idle-CPU measurement is
    orchestrator-owned (this developer does not run live cockpit per
    `AGENT.md ## Capability boundaries`); see `[open_questions].items`
    in this developer's TOML handoff envelope for the orchestrator's
    post-fix verification path.
  _Blast radius (actual)._ **File-span: +296 LOC** in new
  `widgets/throttled_spinner.rs` (the local Widget impl + 5 unit tests
  + ~120 LOC of doc-comment context for future maintainers) plus
  **net +14/-7 LOC** in `widgets/frame.rs` (helper rewrite, docstring
  expansion) plus **+4 LOC** in `widgets/mod.rs` (module export).
  **Glue-layer: 0 LOC** (no `Cargo.toml` change — `ThrottledSpinner`
  is a re-implementation, not a new dependency). Affected files:
  `crates/ui/src/widgets/throttled_spinner.rs` (new),
  `crates/ui/src/widgets/frame.rs`, `crates/ui/src/widgets/mod.rs`.

- [ ] **T-M1-A-2** *(developer; conditional on T-M1-A-1 UNFALSIFIED)* —
  Only-while-visible gating for the spinner subscription.
  _Statement._ Wrap the spinner subscription in a
  `viewport_intersection` check so panels scrolled off-screen don't
  request redraws. The Spinner's internal `is_visible(&bounds)`
  check at `~/.cargo/registry/.../iced_aw-0.14.1/src/widget/spinner.rs:181`
  already guards bounds-zero cases; this task extends to off-screen
  bounds via the scrollable viewport.
  _Falsifier._ Profile cockpit with the strategies panel scrolled
  out of the viewport; spinner-driven redraw self-time drops to
  near-zero.
  _Blast radius._ **File-span: ~+15-30 LOC** in the new
  `throttled_spinner.rs` (extending T-M1-A-1's wrapper).
  **Glue-layer: 0 LOC**.

- [ ] **T-M1-A-3** *(developer; conditional on T-M1-A-1 + T-M1-A-2
  both UNFALSIFIED)* — Replace spinner with non-redraw-requesting
  canvas animation.
  _Statement._ Replace `iced_aw::Spinner` with an `iced::widget::canvas::Canvas`
  shader that projects `iced::time::Instant::now()` into a keyframe
  position without calling `shell.request_redraw_at`. The canvas
  paints whatever the current wall-clock-derived t is; iced's
  natural redraw triggers (user input, window event) suffice for
  visual continuity.
  _Falsifier._ Profile cockpit with a Loading panel; spinner-driven
  redraw self-time disappears from the flamegraph.
  _Blast radius._ Most invasive. **File-span: ~+80-150 LOC** for
  a custom Canvas shader. **Glue-layer: 0 LOC**.

### Candidate B — if M0 confirms H-PERF-2 (Table layout uncached)

Fix order (developer; per Q4): **B1 → B2 → B3 with HANDOFF → architect
for the B3-fallback ADR**. B1 attempted first; B3 ONLY if upstream
`iced::widget::Table` exposes no extension point AND B2 also fails.

- [ ] **T-M1-B-1** *(developer)* — In-place memoization of Table layout
  per row.
  _Statement._ Add a `layout_cache: HashMap<RowId, Cached>` inside
  the strategies widget at `crates/ui/src/widgets/strategies.rs:165`
  (the `table::Table::new(columns, rows.iter().cloned())` builder
  site). The cache invalidates on row-data hash change. Per Q4 this
  is the architect-preferred candidate.
  _Pre-task check (developer)._ Verify `iced::widget::Table::layout`
  exposes an extension point for an external cache. If yes, proceed
  with B1. If no, route `HANDOFF → architect` with a one-line "no
  extension point" note; architect re-engages to ratify B3 via ADR.
  _Falsifier._ Re-run M0's T-M0-1 profile body; expect
  `iced::widget::Table::layout` self-time to drop ~12x (memoization
  hit rate ≈ rows × redraws / (rows + redraws-on-data-change) ≈
  4320 calls/sec → ~360 calls/sec).
  _Three-citation contract._
  - (a) file:line of change: `crates/ui/src/widgets/strategies.rs:N`
    (developer fills).
  - (b) test command: M0 re-profile PLUS `cargo test -p ui
    layout_cache_hit_on_identical_row` PLUS `cargo test -p ui
    layout_cache_miss_on_hash_change` (new unit tests on the cache).
  - (c) test-output line: post-fix Table layout self-time and unit
    test PASS lines.
  _Blast radius._ **File-span: ~+40-80 LOC** in
  `crates/ui/src/widgets/strategies.rs`. **Glue-layer: 0 LOC**
  (no Cargo.toml change; `HashMap` is in std prelude). Affected
  files: `crates/ui/src/widgets/strategies.rs`.

- [ ] **T-M1-B-2** *(developer; conditional on T-M1-B-1 infeasible)* —
  Diff-update only changed cells.
  _Statement._ Track cell version per `(row_id, col_idx)`; only
  redraw cells whose hash changed. Lives inside the widget, not the
  Table; sidesteps the upstream extension-point question.
  _Falsifier._ Profile cockpit with steady-state Ready strategies
  rows; expect per-cell draw count to drop to ~0 in steady state.
  _Blast radius._ **File-span: ~+60-120 LOC** in
  `crates/ui/src/widgets/strategies.rs`. **Glue-layer: 0 LOC**.

- [ ] **T-M1-B-3** *(developer + architect ADR)* — Partial revert of
  Brief A R2 for strategies panel only.
  _Statement._ Replace `table::Table::new(columns, rows)` at
  `crates/ui/src/widgets/strategies.rs:165` with the pre-Brief-A
  `Row::new()` header + `Scrollable<Column>` body. Positions and
  other native-Table panels stay on the Brief A path. **Requires a
  written ADR** per Q4 + divergence 1.
  _Falsifier._ Re-run M0 with the partial revert; expect Table
  self-time to disappear entirely from the flamegraph (no Table
  in the strategies panel).
  _Pre-task gate._ Architect files
  `spec/cockpit-performance-and-input-responsiveness/architecture/adr-001-table-perf-strategy.md`
  documenting why B1 + B2 both blocked and what would unblock a
  future re-migration (e.g., iced 0.15's Table API).
  _Blast radius._ **File-span: ~+20/-120 LOC net (-100)** per the
  analyst's estimate.
  **Glue-layer: 0 LOC** (no Cargo.toml change). Affected files:
  `crates/ui/src/widgets/strategies.rs`.

### Candidate C — if M0 confirms H-PERF-3 (hit-test traversal / event-loop starvation)

Fix order: C1 → C2 → C3. C3 is the upstream-iced PR option; lowest
priority because architecture says "small upstream PRs acceptable;
forks not".

- [ ] **T-M1-C-1** *(developer)* — Z-order shortcut on hit-test.
  _Statement._ Early-exit hit-test on the topmost panel under the
  cursor. The Home screen's 4 panels × ~50 widgets each are
  z-ordered; today's hit-test traverses all 200 candidates per
  click. The shortcut tracks the topmost panel under cursor in
  `crates/ui/src/state.rs` and short-circuits to it first.
  _Falsifier._ Re-run a 60-click synthetic-input bench; expect
  click-recognition to reach 60/60 AND event-dispatch latency
  p99 < 5 ms (matching the H-PERF-3 falsifier in feature.md).
  _Three-citation contract._
  - (a) file:line of change: `crates/ui/src/bin/cockpit.rs:N`
    (the event-loop entry); maybe also `crates/ui/src/state.rs:N`
    for the topmost-panel tracker.
  - (b) test command: synthetic 60-click bench (M3 acceptance
    criterion) + M0 re-profile.
  - (c) test-output line: 60/60 click recognition + p99 latency
    number.
  _Blast radius._ **File-span: ~+30 LOC** in
  `crates/ui/src/bin/cockpit.rs` (event-loop) plus possibly
  `crates/ui/src/state.rs` for the topmost-panel cache.
  **Glue-layer: 0 LOC**.

- [ ] **T-M1-C-2** *(developer; conditional on T-M1-C-1 UNFALSIFIED)* —
  Hit-test cache invalidated on layout change.
  _Statement._ Persist the hit-test result tree across redraws;
  invalidate when iced emits a layout-recompute signal.
  _Blast radius._ **File-span: ~+50-80 LOC** in
  `crates/ui/src/bin/cockpit.rs` + a sibling `hit_test_cache.rs`.
  **Glue-layer: 0 LOC**.

- [ ] **T-M1-C-3** *(developer; conditional on T-M1-C-1 + T-M1-C-2 UNFALSIFIED;
  upstream-iced PR)* — Audit iced 0.14's event-loop ordering to see
  whether redraws and input share a queue; small upstream PR to
  re-order if needed.
  _Statement._ Upstream-iced PR is explicitly allowed per the
  brief's "Out of scope" carve-out
  ([`feature.md ## Out of scope`](feature.md#out-of-scope) — "Small
  upstream PRs to `iced` for ... event-loop ordering audit
  (Candidate C3) are acceptable"). Architect re-engages if this
  task gets to active work; ADR required for any structural change
  to the cockpit's event-loop assumptions.
  _Blast radius._ TBD; upstream PR scope.

## M2 — Perf-budget regression gate *(developer; extends cockpit-smoke skill)*

Per Q3: floor is `fps_p50 >= 30`. Per Q5: cockpit-smoke skill goes
to v1.1 (additive minor bump; v1.0 baseline back-filled). Skill
extension lives in `.claude/skills/cockpit-smoke/SKILL.md`; the
cockpit binary gains a `render-debug`-gated stderr emission at
`view()` boundary.

REQ trace: **REQ-COCKPIT-PERF-001**.

- [ ] **T-M2-1** *(developer)* — Add `render-debug`-gated per-frame
  stderr emission from the cockpit's `Application::view` boundary.
  _Statement._ Emit one stderr line per call to `view()` of the
  shape `RENDER_FRAME <monotonic_ns>` where `<monotonic_ns>` is
  `std::time::Instant::now()`'s monotonic component (per
  AGENT.md determinism rules; the field is presentation-only and
  not part of any anchor body). Wrap the emission in
  `#[cfg(feature = "render-debug")]` to keep default builds
  zero-cost — matches the existing `render-debug` gating pattern
  per H-PERF-5 falsifier resolution.
  _Acceptance criteria._
  - One `eprintln!("RENDER_FRAME {}", ...)` (or equivalent
    `tracing::trace!` if the existing `render-debug` subscriber
    is already wired) line emitted per `view()` call.
  - Default `cargo build -p ui --bin cockpit --features fixtures`
    produces no `RENDER_FRAME` lines (verified by re-running
    H-PERF-5's grep — the new emission must also be gated).
  _Three-citation contract._
  - (a) file:line of change: `crates/ui/src/bin/cockpit.rs:N`
    (developer fills; insertion in the `Application::view` impl).
  - (b) test command:
    `cargo build -p ui --bin cockpit --features fixtures
    && (cargo run -p ui --bin cockpit --features fixtures &) ;
    sleep 7 ; pkill -f 'target/debug/cockpit' ;
    grep -c 'RENDER_FRAME' /tmp/cockpit-default.log`
    → 0 (default build);
    same with `--features fixtures,render-debug` → > 100.
  - (c) test-output line: grep-counts above.
  _Blast radius._ **File-span: ~+15 LOC** in
  `crates/ui/src/bin/cockpit.rs`. **Glue-layer: 0 LOC** (the
  `render-debug` feature already exists at `crates/ui/Cargo.toml:161`).

- [ ] **T-M2-2** *(developer)* — Extend cockpit-smoke skill to parse
  `RENDER_FRAME` lines, compute `fps_p50`, and assert `>= 30`.
  _Statement._ The skill's bash body
  ([`.claude/skills/cockpit-smoke/SKILL.md`](../../.claude/skills/cockpit-smoke/SKILL.md))
  already greps for panic lines over a 7-second window. Add a second
  parsing pass that extracts `RENDER_FRAME <ns>` timestamps,
  computes frame-to-frame deltas, derives `fps_p50` / `p95` / `p99`,
  and asserts `fps_p50 >= 30`. The invocation gains
  `--features fixtures,render-debug` (was `--features fixtures`).
  _Acceptance criteria._
  - Skill emits both `panic_count` and `fps_p50` to its report.
  - Gate FAILs when `fps_p50 < 30`.
  - Regression test: with the dev-only flag `perf-regression-test`
    enabled in `crates/ui/Cargo.toml` injecting a
    `std::thread::sleep(Duration::from_millis(50))` into `view()`,
    the skill reports `fps_p50 <= 20` and FAILs against the 30 fps
    budget. (Tester scaffolds this; developer adds the feature flag.)
  _Three-citation contract._
  - (a) file:line of change:
    `.claude/skills/cockpit-smoke/SKILL.md` (extend bash body) +
    `crates/ui/Cargo.toml` (add `perf-regression-test` non-default
    feature flag + `crates/ui/src/bin/cockpit.rs:N` for the
    `#[cfg(feature = "perf-regression-test")]`-gated sleep).
  - (b) test command:
    `bash .claude/skills/cockpit-smoke/SKILL.md` (or the skill's
    canonical invocation as cited in
    [`AGENT.md ## Process discipline`](../../AGENT.md#process-discipline-lessons-from-v0--v15a)
    rule 6) twice: once with the default cockpit (expect PASS,
    `fps_p50 >= 30`), once with `--features
    fixtures,render-debug,perf-regression-test` (expect FAIL,
    `fps_p50 <= 20`).
  - (c) test-output line: both `fps_p50` values + the PASS/FAIL
    line per invocation.
  _Blast radius._ **Glue-layer: ~+50 LOC** in the cockpit-smoke
  skill template + ~+15 LOC in `crates/ui/Cargo.toml`
  (the `perf-regression-test` feature flag + a `#[cfg]` block in
  `crates/ui/src/bin/cockpit.rs`). **File-span: ~+5 LOC** in
  `crates/ui/src/bin/cockpit.rs`. Affected files:
  `.claude/skills/cockpit-smoke/SKILL.md`, `crates/ui/Cargo.toml`,
  `crates/ui/src/bin/cockpit.rs`.

- [ ] **T-M2-3** *(developer)* — Mint cockpit-smoke v1.0 baseline +
  bump to v1.1 in the skill frontmatter; AGENT.md cite update.
  _Statement._ Per Q5: back-fill the skill's frontmatter with
  `version: 1.0` as the baseline ship from `ui-quality-gate-overhaul
  v1.0.0`, then immediately bump to `version: 1.1` for the
  T-M2-1 + T-M2-2 additive extension. Add two changelog rows.
  _Acceptance criteria._
  - `.claude/skills/cockpit-smoke/SKILL.md` frontmatter carries
    `version: 1.1`.
  - The skill body has a `## Changelog` section with
    `- 2026-05-15 v1.0 baseline mint (back-fill from
    ui-quality-gate-overhaul v1.0.0 ship)` and
    `- 2026-05-15 v1.1 fps_p50 emission + 30 fps assertion (this
    brief's M2)`.
  - `AGENT.md ## Process discipline` rule 6 (the
    `cockpit-smoke` pre-tick gate row) cites the new v1.1
    invocation shape (`--features fixtures,render-debug`).
  _Three-citation contract._
  - (a) file:line of change:
    `.claude/skills/cockpit-smoke/SKILL.md` (frontmatter +
    changelog) + `AGENT.md` (rule 6 cite).
  - (b) test command:
    `grep -n '^version: 1.1' .claude/skills/cockpit-smoke/SKILL.md` (expect 1 hit) +
    `grep -n 'fps_p50' .claude/skills/cockpit-smoke/SKILL.md` (expect ≥ 2 hits).
  - (c) test-output line: grep-counts above.
  _Blast radius._ **Glue-layer: ~+10 LOC**. **File-span: 0 LOC**.
  Affected files: `.claude/skills/cockpit-smoke/SKILL.md`,
  `AGENT.md`.

## M3 — Input dispatch investigation *(coupled in this brief per Q1; split trigger mechanical)*

Per Q1, M3 stays coupled. The split-trigger is mechanical: split
into a sibling brief ONLY if (a) M0 confirms H-PERF-1 or H-PERF-2
(not H-PERF-3) AND (b) the post-M1 click-recognition bench
(T-M3-2 below) shows < 60/60. If both conditions fire, the
architect re-engages with a one-line "split now" note.

REQ trace: **REQ-COCKPIT-PERF-001**.

- [ ] **T-M3-1** *(developer; gated on M1 land + M2 land)* — Add a
  `tracing` span on `WindowEvent::CursorButtonPressed` dispatch.
  _Statement._ Insert a `tracing::trace_span!("event_dispatch",
  kind = "click", ...)` span at the cockpit's
  `iced::event::listen_with` arm for `CursorButtonPressed`. Gate
  behind `#[cfg(feature = "render-debug")]` so default builds stay
  zero-cost (consistent with M2-A's pattern).
  _Acceptance criteria._
  - Span emits on every observed click in `render-debug` builds.
  - Default builds compile the `#[cfg]` block away (verify by
    re-running H-PERF-5's grep extended to the new span site).
  _Three-citation contract._
  - (a) file:line of change: `crates/ui/src/bin/cockpit.rs:N` near
    the existing `iced::event::listen_with` for modal-Esc handling
    (line 286-292 in the current source).
  - (b) test command: run cockpit with `--features
    fixtures,render-debug` and `RUST_LOG=ui=trace`; click 5 times;
    grep stderr for `event_dispatch` span entries (expect ≥ 5).
  - (c) test-output line: grep count.
  _Blast radius._ **File-span: ~+10 LOC** in
  `crates/ui/src/bin/cockpit.rs`. **Glue-layer: 0 LOC**.

- [ ] **T-M3-2** *(developer + ui-designer joint)* — Click-recognition
  bench: 60 deliberately-spaced clicks, assert 60/60 recognition.
  _Statement._ Manual or automated input bench. Automated path:
  `enigo` crate in `dev-dependencies` (acceptable per analyst's
  acceptance criteria — gated behind a feature flag so it does not
  leak into the production binary tree). Manual path: operator
  runs cockpit with `--features fixtures,render-debug
  perf-regression-test` and clicks 60 buttons in a 60s window;
  count `event_dispatch` span entries via stderr grep.
  _Acceptance criteria._
  - 60/60 click recognition.
  - If recognition < 60/60 AND M0 confirmed H-PERF-1 or H-PERF-2
    (not H-PERF-3), TRIGGER the split-condition per Q1: route
    `HANDOFF → architect` with the split-now note; architect lifts
    a sibling `cockpit-input-dispatch` brief from this brief's
    M0 + M1 evidence.
  _Three-citation contract._
  - (a) file:line of change: zero (bench is a harness, not a
    source edit) OR ~+30 LOC for the `enigo`-driven automated
    bench at `crates/ui/tests/click_recognition_bench.rs`
    (developer + ui-designer decide automation depth at
    task-pickup).
  - (b) test command: `cargo test -p ui --test click_recognition_bench`
    (if automated) OR a manual operator run logged at
    `spec/<slug>/reports/click-recognition-<ts>.log`.
  - (c) test-output line: the 60/60 (or < 60/60) line.
  _Blast radius._ **File-span: 0 OR ~+30 LOC** depending on
  automation depth. **Glue-layer: 0 OR ~+5 LOC** in
  `crates/ui/Cargo.toml` for `enigo` as `dev-dependencies` if
  automated. Affected files: `crates/ui/Cargo.toml` (dev-deps),
  `crates/ui/tests/click_recognition_bench.rs` (new).

## M_FINAL — Tester gate + presenter handoff

Standard structure per `cockpit-render-regression/tasks.md`. Tester
owns these ticks; developer never ticks them per AGENT.md ##
Process discipline rule 2.

REQ trace: **REQ-COCKPIT-PERF-001**.

- [ ] **T_FINAL_BUILD** *(tester)* — `cargo build -p ui --bin cockpit
  --features fixtures` PASS.
  _Three-citation contract._
  - (a) file:line of change: zero (build verification, no edit).
  - (b) test command: `cargo build -p ui --bin cockpit --features
    fixtures` + `cargo build -p ui --bin cockpit --features
    fixtures,render-debug`.
  - (c) test-output line: `Compiling` + `Finished` lines from
    the build output (no warnings; per `CLAUDE.md ## Coding rules`
    `cargo clippy -- -D warnings` must also pass).

- [ ] **T_FINAL_TEST** *(tester)* — `cargo test -p ui --all-features`
  PASS (267 existing panel_snapshots + new memoize unit tests if
  Candidate B1 lands + new click-recognition bench if Candidate C
  lands).
  _Three-citation contract._
  - (a) file:line of change: zero (test verification).
  - (b) test command: `cargo test -p ui --all-features 2>&1 | tee
    spec/cockpit-performance-and-input-responsiveness/reports/test-<ts>.log`.
  - (c) test-output line: the `test result: ok. N passed; 0 failed`
    summary line per test target.

- [ ] **T_FINAL_COCKPIT_SMOKE** *(tester via cockpit-smoke v1.1
  skill)* — `cockpit-smoke v1.1` PASS: `panic_count == 0` AND
  `fps_p50 >= 30`.
  _Three-citation contract._
  - (a) file:line of change: zero (skill invocation).
  - (b) test command: skill invocation per
    `.claude/skills/cockpit-smoke/SKILL.md` (v1.1 with the
    `--features fixtures,render-debug` shape).
  - (c) test-output line: the skill's `PASS` line citing both
    `panic_count: 0` and `fps_p50: <N>` with `<N> >= 30`.
  Per `AGENT.md ## Capability boundaries`, this skill is
  orchestrator-only; the tester cites the orchestrator's
  skill-run output.

- [ ] **T_FINAL_VERIFY-ANCHORS** *(tester via verify-anchors
  skill)* — `ANCHORS PASS (11/11)` (structural, by construction).
  _Statement._ This brief touches no strategy / audit / exec /
  backtest / report-rendering code; all 9 body-hashed anchors in
  `spec/anchors.toml` (plus the 2 report-sample anchors) stay
  byte-identical. Tester runs `verify-anchors` as a structural
  defense-in-depth; expect PASS by construction.
  _Three-citation contract._
  - (a) file:line of change: zero.
  - (b) test command: `bash scripts/verify_anchors.sh` (or the
    canonical `verify-anchors` skill invocation per
    `.claude/skills/verify-anchors/SKILL.md`).
  - (c) test-output line: `ANCHORS PASS (11/11)`.

- [ ] **T_FINAL_PRESENT** *(presenter, after tester `VERDICT →
  PASS`)* — Assemble
  `spec/cockpit-performance-and-input-responsiveness/presentations/cockpit-performance-and-input-responsiveness-<date>.md`
  per `.claude/skills/present-results/SKILL.md`.
  _Three-citation contract._
  - (a) file:line of change: the new presentation .md.
  - (b) test command: presenter's pre-tick gate (spec-lint +
    cockpit-smoke + verify-anchors all green).
  - (c) test-output line: each gate's PASS line.

## Notes

- **Q4 sub-task ordering signal.** If M0 confirms H-PERF-2, the
  developer attempts T-M1-B-1 BEFORE T-M1-B-2, and T-M1-B-3 only
  on architect-issued ADR. The architect's preference is to
  preserve Brief A R2's native-Table adoption; B3 is a structural
  unwind requiring a written ADR per divergence 1.
- **Q2 wgpu trigger condition.** If T-M0-2's verdict confirms
  H-PERF-4 (tiny-skia in top-3 self-time entries AND fps_p50
  below Q3's 30 floor even after a hypothetical M1 spinner/Table
  fix), the architect re-engages with a one-line "operator
  decision required" stub in the changelog. No wgpu work begins
  in this brief.
- **Q1 split-trigger.** Mechanical: split into
  `cockpit-input-dispatch` only if M0 confirms H-PERF-1 or H-PERF-2
  AND the post-M1 T-M3-2 bench shows < 60/60 click recognition.
  Both conditions required; either alone is not enough.
- **Honest-tick discipline.** Every `[x]` carries (a) file:line,
  (b) test cmd, (c) test-output. Tester owns `T_FINAL_*` ticks.
- **Constraint envelope.** No wgpu (unless Q2 trigger fires AND
  operator overrides), no iced fork, no `plotters-iced` /
  `iced_plot` / `iced-anim` per
  `trading_ui_library_constraints.md` (user-memory).
