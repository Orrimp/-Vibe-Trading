---
slug: backlog
status: living
owner: orchestrator
updated: 2026-05-17
---
<!-- updated 2026-05-17 (analyst, ui-rethink-phase-a-lab) — promoted
     `ui-rethink-phase-a-lab` v0.1.0 from dev-note → Active. First
     concrete carve-out of the broader UI rethink at
     `spec/dev-notes/ui-rethink-2026-05-17.md`. Operator-locked
     direction (chart-as-door, XRP-first pair ordering, three overlay
     layers, read-only at Phase A) ratified in the dev-note addendum
     2026-05-17. Brief authored: `Charts` → `Lab` rename + default-
     route flip, buy/sell markers (already shipped at v1.9.0) wired
     into Lab, new equity-curve overlay (cached reports), new
     ≤4-strategy comparison overlay, pair-chip / strategy-chip /
     date-range widgets, Lab tuple persistence. Eleven requirements
     (R1–R11); zero new Lumen tokens; zero touched anchors. Three
     operator-decide questions surfaced (Q-A1 palette / Q-A2
     cached-only / Q-A3 cold-start tuple) — each with a default the
     developer can ship against. Trace row `REQ-UI-RETHINK-PHASE-A-001`
     created in proposed state. HANDOFF → architect. -->
<!-- updated 2026-05-17 (analyst, v25-tcn-overlay) — phase 1 of 4 DL
     roadmap. Analyst pass landed on `v25-tcn-overlay`: Q1-Q8 closed
     with defaults (R1-R12 in feature.md). Status flip _draft_ →
     _in-progress_. Two operator-decide Qs surfaced (anchor checkpoint
     storage; two-checkpoint backtest split). HANDOFF → operator-decide
     → architect. Sources cited per Kronos-pivot lesson:
     BKK18 (https://arxiv.org/abs/1803.01271), locuslab/TCN,
     Keras-TCN, candle-transformers. -->
<!-- updated 2026-05-16 (orchestrator) — Kronos→DL pivot. v2.5
     `v25-kronos-forecast-overlay` dropped after Wave A bootstrap
     surfaced: (a) Kronos lives outside `transformers` so requires
     vendoring upstream Python code; (b) two-model architecture
     (KronosTokenizer + Kronos) requires reimplementing the
     autoregressive sampling loop in Rust; (c) crypto-fit was never
     validated. Reframed project goal locked: "real, working,
     auditable agent architecture; operator learns by building it"
     — and a pre-trained black box scores poorly on the learning
     axis. Pivoted to `v25-dl-forecast-overlay`: train a small
     custom Transformer/TCN in `candle` (the project's named
     prototyping ML framework per CLAUDE.md). See ADR-0028 (which
     supersedes ADR-0027). Wave A crates (forecast, replay-cache,
     core forecast types) are model-agnostic and preserved; only
     Kronos-specific files removed. New brief stub at
     `spec/v25-dl-forecast-overlay/feature.md` (status: draft);
     analyst pass spawned for model family / size / tokenisation /
     data / loss / horizon / success criterion / checkpoint storage
     / audit integration. -->
<!-- updated 2026-05-16 (analyst, Wave 2a spec-hygiene) — three
     stalled strategy features (v0-paper-sma, v05-composed-strategies,
     v1-cross-sectional-momentum) flipped to shipped (bookkeeping;
     code + reports on disk weeks earlier). ui-gallery-bin
     reclassified as shipped-partial-terminal (v0.1-partial); successor
     brief `ui-gallery-table-cell` opened in draft to own the V5+
     tiny-skia table-cell bounds fix. Full breakdown in the
     Changelog entry below. Source: `spec/dev-notes/feature-triage-2026-05-16.md`. -->
<!-- updated 2026-05-15 (analyst, ui-testability deep-dive) — added
     8 candidate features under Queue ## Process / tooling pointing
     into the new `spec/dev-notes/ui-testability-deep-dive-2026-05-15.md`
     dev-note (ui-contrast-asserter, ui-update-proptest, ui-gallery-bin,
     ui-a11y-shadow, ui-vlm-judge, ui-inspect-mcp, ui-session-journal,
     ui-mutants-pass, plus a tester agent-contract addition for the
     visual-fail HTML artifact). Annotated the existing
     ui-test-harness-ci candidate with a cross-platform falsifier
     recommendation to revisit operator decision D3. -->
<!-- updated 2026-05-12 (analyst, second pass) — promoted
     `ui-test-harness-bootstrap` v0.1 from Queue ## Process / tooling
     → Active. First feature under the new
     `AGENT.md ## Capability boundaries` regime (committed 2026-05-12).
     Scope locked to week-1 only of the dev-note 4-week plan:
     `iced_test` smoke + `insta` binary snapshots + canvas hit-test
     grid sweep at three viewport sizes. Operator decisions D1-D5 from
     `spec/dev-notes/ui-testing-direction-2026-05-12.md ## Section 9`
     LOCKED (analyst does not revisit). Brief at
     `spec/ui-test-harness-bootstrap/feature.md` (status: in-progress,
     version: 0.1.0, predecessor: chart-canvas-overhaul v1.10.0). 10
     open Qs for architect / operator routing. H1 (tiny-skia byte
     determinism) seeded in the new `Hypothesis register` section.
     Non-regression contract: 11 anchors stay byte-identical, 818
     tests stay green, zero changes to non-UI crates. Closes
     chart-canvas-overhaul V15 via the week-1 grid sweep at 3360×1890
     (operator decision D4). HANDOFF → architect. -->
<!-- updated 2026-05-12 (analyst) — promoted `chart-canvas-overhaul`
     v1.10.0 → Active. Opened in response to operator's
     2026-05-12 second visual-verification pass at native
     3360×1890 Retina: tooltip still invisible, chart still
     cropped, no SVG-style scaling, no legend, no axes,
     "not centered". Three items (tooltip, cropping, scaling)
     are regressions vs. v1.9.0's M6.2 hardening pass — the
     v1.9.0 PASS verdict was issued against a 1280×720 tester
     capture, missing the operator's actual hardware shape.
     Brief at `spec/chart-canvas-overhaul/feature.md`; tasks
     stubbed at `spec/chart-canvas-overhaul/tasks.md` (T3001+).
     Three operator-decide questions (Q1 "not centered" reading,
     Q4 UTC-vs-local time axis, Q7 viewer parity) need operator
     answers before architect spawn. -->
<!-- updated 2026-05-04 (analyst, post-Phase-1-ship roadmap revision) —
     `lumen-phase-1-foundation` shipped (tester third-pass PASS) → moved
     Active → Recent. Lumen master roadmap revised from 4 phases to 6
     at operator request: new Phase 2 Shell IA + Charts and Phase 3
     Detail screens insert ahead of original phases. Old Phase 2
     (viewer backtest) → Phase 4. Old Phase 3 (HumanControl + AgentFeed)
     → Phase 5. Old Phase 4 (Assistant slot) → Phase 6 (still reserved
     for v2 LLM). Five new feature brief stubs queued. UI / cockpit
     queue subsection rewritten for the 6-phase plan. -->
<!-- updated 2026-05-03 (analyst) — `v1.5b multi-venue` moved Active →
     Recent (shipped 2026-05-03). New `lumen-design-adoption` master
     roadmap + `lumen-phase-1-foundation` first-phase brief promoted to
     Active. `lumen-phase-2-viewer-backtest` and
     `lumen-phase-3-human-control-agent-feed` queued.
     `lumen-phase-4-assistant` reserved for v2 LLM. Three operator-locked
     constraints (no brand adoption, no voice rewrite, sequential
     phasing) documented in the master roadmap. -->


# Backlog

Queued ideas the operator has surfaced but hasn't promoted to a
feature brief yet. One line each + a note on cost or blockers. This
file is editable churn — nothing here is a commitment.

Promote an item to real work by spawning the **analyst**, who turns it
into a `spec/<slug>/feature.md` brief and removes the entry here.

## Active

- **v2.5 — TCN forecast overlay (`v25-tcn-overlay`).** _in-progress_ —
  phase 1 of the [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md)
  (operator-locked 2026-05-17 after reading the
  [v25-dl-reading-list](dev-notes/v25-dl-reading-list-2026-05-16.md)
  and deciding to build all three model families for empirical bake-off).
  Model family: Temporal Convolutional Network (Bai, Kolter, Koltun
  2018). Selected first because (a) simplest architecture, fastest to
  a working baseline; (b) establishes the training loop + audit +
  replay infrastructure that v2.5a (PatchTST) and v2.5b (Transformer)
  reuse; (c) deterministic inference (no autoregressive sampling) —
  easier to anchor and audit. Data prerequisite: 10 USDT pairs hourly
  2023+2024 bootstrapped via `cargo run -p data --bin fetch_binance_klines`
  (~72s wallclock, ~15-20 MB). **Analyst pass landed 2026-05-17**
  ([`feature.md`](v25-tcn-overlay/feature.md) §Requirements R1–R12)
  closing Q1-Q8 with defaults (TCN 8-block dilation `[1..128]`, k=3,
  H=96 → ~4.4M params; 256-bar context; 5 features; continuous
  log-return regression with Huber δ=0.001; OneCycle AdamW;
  two-checkpoint walk-forward; SHA-256 provenance over arch+data+seed+weights).
  Two operator-decide questions surface: anchor checkpoint storage
  (LFS vs regen-from-seed) and two-checkpoint backtest split.
  Carry-forward backtest scenarios: BS-1 (2023 full-year top-10 USDT),
  BS-2 (2024 full-year top-10 USDT). HANDOFF → operator-decide → architect.

- **UI rethink Phase A — chart-centric Lab (`ui-rethink-phase-a-lab`).**
  _draft_ — first concrete feature carved out of the broader UI rethink at
  [`spec/dev-notes/ui-rethink-2026-05-17.md`](dev-notes/ui-rethink-2026-05-17.md)
  (operator critique 2026-05-17 + addendum locking chart-as-door,
  XRP-first ordering, three overlay layers, read-only at Phase A).
  Predecessor: [`chart-canvas-overhaul`](chart-canvas-overhaul/feature.md)
  v1.10.0. Renames `Charts` → `Lab`, flips Lab to the default boot route,
  fuses three overlay layers on the single canvas (buy/sell markers from
  v1.9.0 emphasis ship + new equity curve + new ≤4-strategy comparison),
  introduces pair-chip / strategy-chip / date-range widgets, persists
  `(strategy, pair, range, params)` across cockpit restarts. Eleven
  requirements (R1–R11); zero new Lumen tokens; zero touched anchors;
  three operator-decide questions (Q-A1 comparison-line palette, Q-A2
  cached-only vs in-process backtest at Phase A, Q-A3 cold-start tuple
  default). Brief at [`ui-rethink-phase-a-lab/feature.md`](ui-rethink-phase-a-lab/feature.md);
  task skeleton at [`ui-rethink-phase-a-lab/tasks.md`](ui-rethink-phase-a-lab/tasks.md)
  (M0 rename → M1 chips/picker → M2 equity overlay → M3 comparison
  overlay → M-FINAL persistence + audits). Trace row
  `REQ-UI-RETHINK-PHASE-A-001` (proposed). HANDOFF → architect.

## Queue

### Strategy

- **v2.5a — PatchTST / iTransformer forecast overlay
  (`v25a-patchtst-overlay`).** _roadmap_ — phase 2 of the
  [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md). Activates
  after phase 1 (TCN) ships. Patch-based Transformer paradigm; reuses
  training infrastructure from phase 1. Stub at
  [`spec/v25a-patchtst-overlay/feature.md`](v25a-patchtst-overlay/feature.md).
- **v2.5b — Vanilla decoder-only Transformer overlay
  (`v25b-transformer-overlay`).** _roadmap_ — phase 3 of the
  [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md). Activates
  after phases 1+2 ship. Autoregressive Transformer over discretised
  OHLCV tokens (the operator's hand-built Kronos-shape successor —
  full provenance, no pre-trained weights). Stub at
  [`spec/v25b-transformer-overlay/feature.md`](v25b-transformer-overlay/feature.md).
- **v2.6 — Forecast bake-off + retirement (`v26-forecast-bakeoff`).**
  _roadmap_ — phase 4 of the
  [4-phase DL roadmap](v25-dl-forecast-overlay/feature.md). After all
  three phase-1/2/3 forecasters ship, run a head-to-head on BS-1+BS-2
  with identical criteria. Pick the canonical v2.5 overlay; mark the
  other two as research-mode only. Stub at
  [`spec/v26-forecast-bakeoff/feature.md`](v26-forecast-bakeoff/feature.md).
- **Pre-pivot breadcrumb:** the dropped Kronos approach is preserved at
  [`spec/dev-notes/kronos-evaluation-2026-05-10.md`](dev-notes/kronos-evaluation-2026-05-10.md)
  [SUPERSEDED] as a what-not-to-do reference.

### UI / cockpit (Lumen design-system adoption — Phase 6 reserved)

- **Lumen Phase 6 — Assistant slot.** _reserved_ — depends on the
  v2 LLM strategy queued item above. Right-rail collapsible panel
  for the v2 LLM assistant per
  [`spec/design/project/ui_kits/desktop/Assistant.jsx`](design/project/ui_kits/desktop/Assistant.jsx).
  Phase 2 reserved the right-rail column-track in the shell grid
  at `Length::Fixed(0.0)`; the actual Phase 6 brief lands when v2
  LLM is approved. Until then, no analyst spawn. Stub at
  [`features/lumen-phase-6-assistant-slot.md`](features/lumen-phase-6-assistant-slot.md).
  _(Phases 1–5 of the lumen-design-adoption initiative are shipped
  and live in the Recent section; this Queue entry is the only
  remaining initiative work, gated on v2 LLM.)_

- **v1.11 — Chart x-axis local time (`chart-x-axis-local-time`).**
  _candidate_ — surfaced by the
  [`chart-canvas-overhaul`](chart-canvas-overhaul/feature.md) M7
  architect pass 2026-05-12, deferred from v1.10.0 by operator-
  locked Q-revised-1 = path (b). Scope: flip the workspace `time`
  dep's `local-offset` feature on (one-line `Cargo.toml` edit;
  macOS-only cockpit so the multi-threaded glibc deadlock caveat
  doesn't bite); wire
  [`local_offset_or_utc()`](chart-canvas-overhaul/feature.md#q4-deferral-r10--t3028)
  at `crates/ui/src/widgets/chart.rs:125-160` to
  `time::UtcOffset::current_local_offset()` in production; keep
  `cfg(test)` override at `UtcOffset::UTC` so snapshot tests stay
  deterministic; add one unit test
  `local_offset_under_production_reads_os_offset` asserting the
  helper returns a non-UTC offset on macOS when the OS is set to a
  non-UTC zone; defence-in-depth `verify_anchors.sh` →
  `ANCHORS PASS (11/11)` (no strategy / audit / report path
  affected by the feature flag flip). v1.10.0 ships with UTC
  fallback; v1.11 closes the operator-friendly local-time landing.
  Analyst spawn when v1.10.0 ships; not before. Full details
  deferred to the v1.11 brief's analyst.

- **P1 — Cockpit performance + input responsiveness
  (`cockpit-performance-and-input-responsiveness`).**
  _proposed_ — surfaced 2026-05-15 by operator's live-cockpit
  verification during `ui-quality-gate-overhaul v1.0.0` approval.
  Two symptoms observed on the post-Brief-A + Brief-B + F1-fix
  build:
  1. **UI is slow** — sluggish redraw / frame pacing under
     `--features fixtures` on macOS Apple Silicon.
  2. **Input dispatch unreliable** — not every click is recognized.

  Both pre-date the M1/M2 quality gates (live in Brief A's native
  iced::widget::Table adoption + Brief B's `iced_aw::Spinner`
  continuous-redraw subscription + the existing event-handling
  surface), but they are NOT caught by any current gate — the new
  cockpit-smoke gate (M1-A) checks first-frame panic only, real-
  renderer snapshots (M1-B) test layout determinism not perf, and
  proptest layout invariants (M1-C) don't probe runtime event flow.

  **Hypothesis seeds for the analyst (orchestrator surface only —
  analyst owns falsification):**
  - H-PERF-1: `iced_aw::Spinner`'s 60 FPS `request_redraw_at`
    subscription forces continuous full-cockpit repaint while ANY
    panel is in `PanelState::Loading`. Even one loading panel
    pulls the whole cockpit into 60 fps software-rasterized
    (`iced_tiny_skia`) redraw. CPU cost compounds across the 8 B2
    call sites.
  - H-PERF-2: Native `iced::widget::Table` (Brief A R1/R2) has a
    layout pass that recomputes per-cell bounds every redraw, not
    cached. With ~12 strategy rows × 6 columns × 60 fps = 4320
    layout calls/second.
  - H-PERF-3: Click hit-test traverses the full widget tree (Home
    screen = 4 panels × ~50 widgets each ≈ 200 hit-test candidates
    per click). Missed clicks may correlate with a 60-fps redraw
    landing during the same event-loop tick and starving the
    `WindowEvent::CursorButtonPressed` dispatch.
  - H-PERF-4: `iced_tiny_skia` is software-rasterized on the
    operator's hardware; wgpu is off-table per
    [`trading_ui_library_constraints.md`](dev-notes/trading_ui_library_constraints.md)
    BUT the architect should revisit the constraint given the live
    perf signal.
  - H-PERF-5: M2-A `tracing` instrumentation (just shipped) is
    behind `render-debug` and should NOT fire in default builds —
    confirm via grep that no production code path emits spans.

  **Scope when promoted:**
  - **M0 (orchestrator-runnable):** profile a default-fixtures
    cockpit boot using `samply` or `cargo flamegraph` for ~30s
    and capture the top 10 hot frames. Identify whether spinner
    redraw, Table layout, or click hit-test dominates.
  - **M1:** fix the dominant hot path. Likely candidates: gate
    `iced_aw::Spinner` redraw to a coarser cadence (e.g. 10 fps,
    or only-while-visible); cache Table layout; or restructure
    event dispatch.
  - **M2:** add a `cockpit-perf-budget` regression test (CI gate
    that asserts <budget> fps on a fixtures cold-start). The
    `ui-quality-gate-overhaul` brief's M1-A cockpit-smoke skill
    is the natural place to plug this — extend it to also record
    frames-per-second over the 7-10s window.
  - **M3:** input-responsiveness investigation — likely a separate
    sub-thread. Use M2-A `tracing` spans on event dispatch to
    pinpoint dropped clicks.

  **Out of scope:**
  - Renderer backend swap to wgpu (per architecture pin; if the
    perf analysis forces it, that's a separate brief requiring
    operator override of the existing constraint).
  - Major iced version bump (still pinned at 0.14.0 per Brief B).

  **Analyst spawn:** operator-triggered. Brief is `proposed`
  state in trace.toml; analyst draft turns it into `draft`.
  Predecessor: `ui-quality-gate-overhaul v1.0.0`. Trigger cite:
  operator's 2026-05-15 live-cockpit observation during the
  v1.0.0 approval pass, recorded in
  [`spec/ui-quality-gate-overhaul/presentations/ui-quality-gate-overhaul-2026-05-15.md`](ui-quality-gate-overhaul/presentations/ui-quality-gate-overhaul-2026-05-15.md)
  approval-with-notes block.

- **TBD — Cockpit Windows / Linux support (`cockpit-cross-platform`).**
  _candidate_ — surfaced 2026-05-12 by operator decision D3 in
  [`spec/dev-notes/ui-testing-direction-2026-05-12.md`](dev-notes/ui-testing-direction-2026-05-12.md#section-9).
  Today the cockpit is macOS-only (Retina assumptions, `screencapture` +
  TCC dependencies, `iced_tiny_skia` chosen partly for CPU determinism on
  Apple Silicon). Scope when promoted: validate `iced_tiny_skia` rendering
  parity on Linux X11/Wayland + Windows; replace `screencapture`-based
  test artifact capture with cross-platform `xcap` or equivalent;
  re-evaluate the `time` `local-offset` feature's Linux multi-threaded
  caveat once v1.11 lands. Analyst spawn deferred — operator triggers
  when external demand (paper-trading on Linux server, Windows operator
  hardware) appears.

### Process / tooling

- **`audit-tick-consumer-envelope`** (architecture follow-up) —
  _candidate, sourced from
  [`spec/dev-notes/external-code-patterns-2026-05-17.md`](dev-notes/external-code-patterns-2026-05-17.md)_.
  Formalised by [ADR-0031](architecture/adr/0031-audit-tick-consumer-envelope.md)
  (status `proposed`). Borrows the barter-rs `AuditTick<Event, Context>`
  envelope so `crates/reflection`, future Lab `Trail` (Phase D), and
  future v2.6 bake-off + v3 success-reports can subscribe to a single
  audit broadcast instead of each requiring a dedicated write tap.
  Strictly additive (existing taps coexist); zero hot-path impact; 11
  anchors stay byte-identical. ~50 LOC addition in `crates/audit`
  per ADR. Analyst spawn when operator chooses to promote.

- **`v2x-trading-state-bus`** (v2 LLM evolution) —
  _candidate, sourced from
  [`spec/dev-notes/external-code-patterns-2026-05-17.md`](dev-notes/external-code-patterns-2026-05-17.md)_.
  Replace ad-hoc parameter threading in the v2 LLM agent pipeline with
  an owned `TradingState { fundamentals, sentiment, news, technical,
  debate: Vec<Argument>, count: u32, … }` struct that each agent
  destructures, mutates its slice, and passes on. Mirrors
  [TradingAgents'](https://github.com/TauricResearch/TradingAgents)
  LangGraph state-dict pattern but in Rust. Bull/Bear adversarial
  researcher pattern (see below) plugs in cleanly. Not a v2.0 ship
  enhancement (v2 already shipped 2026-05-13); a v2.1 or v2.2
  feature. Analyst spawn when operator chooses.

- **`v26-bakeoff-llm-arbiter`** (v2.6 enhancement) —
  _candidate, sourced from
  [`spec/dev-notes/external-code-patterns-2026-05-17.md`](dev-notes/external-code-patterns-2026-05-17.md)_.
  After v2.5 / v2.5a / v2.5b ship as TCN / PatchTST / Vanilla
  Transformer, an LLM arbiter reads all three forecasters' outputs
  + the operator's strategy params and produces a tie-break decision
  with a reasoning trace that lands in the audit ledger. Adapts the
  [TradingAgents](https://github.com/TauricResearch/TradingAgents)
  bull/bear adversarial researcher pattern to DL-forecast arbitration.
  Plugs into [`spec/v26-forecast-bakeoff/feature.md`](v26-forecast-bakeoff/feature.md);
  not a separate feature — a v2.6 design refinement the analyst
  considers when the bake-off feature activates.

- **v2.1 — Cockpit LLM-budget tile + tracing-Layer redactor +
  pedantic clippy cleanup (`v2-llm-strategy-v21-followups`).**
  _candidate, surfaced 2026-05-13 by v2-llm-strategy v2.0.0 ship_ —
  three deferred items consolidated:
  (a) **T1938 cockpit "LLM budget" tile** — was deferred in pass 6
      because its dependency `audit::query::llm_spend_this_month`
      isn't implemented. v2.1 ships the audit query helper +
      the right-rail tile (three-color thresholds: green < 60%,
      amber 60-80%, red ≥ 80%; auto-degrade at 80% per Q6).
  (b) **T1915 tracing-Layer redactor half** — pure-fn `redact()`
      landed in pass 3; the `tracing_subscriber::Layer` field-
      visitor side needs `tracing_subscriber = "0.3"` (new dep).
      v2.1 wires the Layer so structured logging redacts
      `Bearer ...` / `sk-...` / `anthropic-...` patterns in
      fields without requiring callers to invoke `redact()`
      explicitly.
  (c) **T1910 pedantic clippy cleanup** — 2 `cast_possible_truncation`
      warnings on `crates/audit/src/query.rs:219, 221` from the
      `cache_hit_ratio_since` query. Non-blocking per v2.0.0
      brief §Critical constraints #2. v2.1 cleans these up via
      `Decimal::try_from` or explicit clamp.
  Analyst spawn when operator promotes; not urgent.

- **Canvas-state seeding for snapshot tests
  (`ui-test-harness-canvas-state-seeding`).** _candidate, surfaced
  2026-05-12 by H2 operator review on `ui-test-harness-bootstrap`
  v0.1_ — closes the **render** half of `chart-canvas-overhaul` V15
  (V8 in the bootstrap brief). The v1.9.0 T2033 refactor decoupled
  tooltip rendering from `Cockpit.chart_tooltip` — the canvas reads
  hover state from its internal `ChartProgram::State`, not from
  `self.tooltip` — so the Q9 fixture's
  `Cockpit.chart_tooltip = Some(...)` has zero effect on the
  rendered PNG.  Scope: extend
  [`crates/ui/src/test_support.rs`](../crates/ui/src/test_support.rs)
  with `seed_canvas_hover_state(idx)` injecting state directly into
  `ChartProgram::State` before `iced_test::screenshot` runs. Two
  viable paths: (a) `iced_test::Simulator::send_event` to dispatch
  a synthetic `CursorMoved` over the marker centroid; (b) a
  `#[doc(hidden)]` test-only `ChartProgram::with_seeded_hover_state(
  idx, centroid) -> Self` constructor.  Acceptance: a new
  `charts_screen_dark_operator_hover.png` baseline shows the
  tooltip card next to a hovered marker; existing three baselines
  stay byte-identical (V15 detection + render fully closed).
  Analyst spawn after v0.1 ships.

- **Week-2 follow-up — full-widget viewport matrix
  (`ui-test-harness-viewport-matrix`).** _candidate, gated on
  `ui-test-harness-bootstrap` v0.1 ship_ — extends the v0.1 Charts-only
  three-viewport snapshot harness across ALL widget tests (panels,
  modals, status bar, agent feed, debug screen) per
  [dev-note §6 week 2](dev-notes/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan).
  Analyst spawn when v0.1 ships and H1 (tiny-skia byte determinism)
  is unfalsified.

- **Week-3 follow-up — evaluator subagent + PreToolUse hooks
  (`ui-test-harness-evaluator`).** _candidate, gated on
  `ui-test-harness-bootstrap` v0.1 ship_ — splits the tester role
  into test-runner (writeable) + evaluator (read-only, fresh
  context, default-FAIL PreToolUse hook) per
  [dev-note §4.2](dev-notes/ui-testing-direction-2026-05-12.md#42-default-fail-evaluator-subagent)
  and
  [`AGENT.md ## Test-runner / evaluator split`](../AGENT.md#test-runner--evaluator-split).
  Wires the PreToolUse hooks for `screencapture`, `osascript`, and
  `./target/release/cockpit` denying sub-agents (allowing
  orchestrator). Analyst spawn after v0.1 ships.

- **Week-4 follow-up — GitHub Actions CI + presenter integration
  (`ui-test-harness-ci`).** _candidate, gated on
  `ui-test-harness-viewport-matrix` + `ui-test-harness-evaluator`
  ship_ — macOS runner workflow uploading baseline+actual+diff PNG
  triples on visual snapshot failures; presenter deck format gets a
  fixed "screenshot artifacts" section pointing at the CI artifact
  URL per [dev-note §6 week 4](dev-notes/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan).
  **Per [`ui-testability-deep-dive-2026-05-15.md §5.3`](dev-notes/ui-testability-deep-dive-2026-05-15.md#53-keep--drop--replace-against-the-existing-weeks-2-4-plan)**
  the analyst recommends pairing this CI brief with the 1-day
  cross-platform falsifier (item O) to retire or confirm operator
  decision D3 (macOS-only CI). **CHEAPENED 2026-05-15:** down to
  ~4 dev-days (from 5) per
  [`iced-014-feature-analysis-2026-05-15.md §4`](dev-notes/iced-014-feature-analysis-2026-05-15.md#headless-mode).
  iced 0.14's `iced_test::emulator::Emulator` (PR #2698) ships
  embedded Fira Sans + a real headless runtime, so we don't need
  to author font-fallback / xvfb plumbing. **FURTHER DECOMPOSED
  2026-05-16:** the Emulator adapter portion shipped standalone as
  [`ui-headless-emulator` v0.1](ui-headless-emulator/feature.md);
  remaining scope is CI workflow + cross-platform falsifier only.

- **comet debugger revisit trigger (`ui-comet-eval`).** _candidate,
  REVISIT-GATED 2026-05-16 by operator decision_ — Q-COMET-EVAL
  LOCKED → defer indefinitely STILL APPLIES. Operator-acknowledged
  revisit trigger added: when iced 0.15.0 **stable** releases (not
  the current `0.15.0-dev` master pin), bump Q-014-PIN consideration
  + re-evaluate this candidate. Until then: no spawn trigger, no
  schedule. See
  [`iced-014-feature-analysis-2026-05-15.md §3`](dev-notes/iced-014-feature-analysis-2026-05-15.md#comet-debugger)
  for the original analysis.

  > **Attempted + aborted 2026-05-16.** Operator authorized the
  > full bump (Q-014-PIN + Q-COMET-EVAL override). Two strikes
  > surfaced before abort: (1) js-sys dep conflict (resolved with
  > iced-master's Cargo.lock copy); (2) **23 iced 0.14→0.15 API
  > churn errors in iced_aw alone**, with the same patterns
  > (`Theme::extended_palette`, `Text` gained `ellipsis`+`hint_factor`
  > fields, `Palette.text` removed, `Widget::update` arity changes,
  > `Font::with_name` removed, `Overlay::update` arity change)
  > guaranteed to recur across our ~30 widget/screen files.
  > **iced_aw + iced_fonts BOTH still on iced 0.14**, confirming
  > the ecosystem has not migrated yet. Revised cost estimate
  > climbed 5-9d → 6-11d before mandatory-stop. Aborted at the
  > operator's "Stop now" choice. Vendor work reverted (was
  > uncommitted). The revisit trigger above remains the correct
  > path — wait for iced 0.15.0 stable + iced_aw / iced_fonts
  > ecosystem migration.

- **Operator UI legibility — WCAG contrast asserter
  (`ui-contrast-asserter`).** _candidate, surfaced 2026-05-15 by
  [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md §3.8`](dev-notes/ui-testability-deep-dive-2026-05-15.md#38-stretch--pure-rust-wcag-contrast-asserter--ui-contrast-asserter)_
  — a `crates/ui/tests/contrast.rs` test that enumerates every
  `(fg, bg)` token pair in
  [`crates/ui/src/theme.rs`](../crates/ui/src/theme.rs) and asserts
  WCAG 2.1 contrast ratios per
  [`ui-design-principles.md ## Accessibility minimums`](ui-design-principles.md#accessibility-minimums)
  (4.5:1 AA body, 7:1 AAA equity). Half-day analyzed work. Closes
  an entire class of palette-refactor regression without rendering
  a single pixel. Run in WARN mode for two weeks before promoting
  to gate. Analyst spawn when operator promotes.

- **Pure-state property tests — update + proptest harness
  (`ui-update-proptest`).** _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.4`](dev-notes/ui-testability-deep-dive-2026-05-15.md#34-stretch--update--property-based-state-machine-harness--ui-update-proptest)_
  — drive `ui::state::update` with
  [`proptest-state-machine`](https://crates.io/crates/proptest-state-machine)
  over randomized `Message` sequences. Five invariants to start:
  kill monotonicity, no cross-screen state leakage, PanelState arm
  reachability, subscription-error recoverability, audit-write
  idempotency. ~5 dev-days. Closes ~40 `Message` variants currently
  not directly covered (analysis at
  [`ui-testability-deep-dive-2026-05-15.md §2.10`](dev-notes/ui-testability-deep-dive-2026-05-15.md#210-state-invariant-tests-vs-view-tests--quantifying-the-gap)).
  Analyst spawn when operator promotes; pairs naturally with
  `ui-mutants-pass` below.

- **Storybook-equivalent widget gallery bin
  (`ui-gallery-bin`).** _v0.1-partial shipped 2026-05-15_ —
  V1-V4 green (build, smoke, widget cell exhaustiveness, mod-rs
  cross-check). V5+ snapshot tests blocked on the iced Table cell-
  bounds panic — see [`ui-gallery-bin/tasks.md` ## Status](ui-gallery-bin/tasks.md)
  and the follow-up `ui-gallery-table-cell` candidate below.
  Original brief: [`ui-testability-deep-dive-2026-05-15.md §3.3`](dev-notes/ui-testability-deep-dive-2026-05-15.md#33-widget-gallery-binary--ui-gallery-bin).

- **iced 0.14 Table cell-bounds fix
  (`ui-gallery-table-cell`).** _candidate, surfaced 2026-05-15 by
  the [`ui-gallery-bin` v0.1-partial deliverable](ui-gallery-bin/design.md#changelog)_
  — `widgets::strategies::view` uses `iced::widget::table::Table`,
  which produces a degenerate quad and panics `iced_tiny_skia` at
  `engine.rs:686` ("Build quad rectangle") when rendered inside a
  fixed-height `gallery::cell::view` Container. Diagnostic at
  [`crates/ui/tests/gallery_bisect.rs`](../crates/ui/tests/gallery_bisect.rs)
  pinpoints `GALLERY_CELLS[7]`. Two paths: (a) special-case the
  strategies cell wrapper to drop the height constraint, or (b)
  swap strategies for a non-table render in the gallery only. ~1
  dev-day either way. Unblocks V5+ of `ui-gallery-bin` plus any
  future gallery cells that wrap table-based widgets. Analyst spawn
  when operator promotes.

- **AccessKit shadow tree + kittest assertions
  (`ui-a11y-shadow`).** _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.5`](dev-notes/ui-testability-deep-dive-2026-05-15.md#35-stretch--accesskit-shadow-tree--ui-a11y-shadow)_
  — author `crates/ui/src/a11y.rs` emitting an
  [`accesskit::TreeUpdate`](https://docs.rs/accesskit) for the
  cockpit's widget surface; wire to
  [`kittest`](https://docs.rs/kittest/) for tree-based assertions
  that render zero pixels. Establishes a Layer 2 "widget tree"
  oracle (§2.14 of the dev-note) that catches half the failure
  classes pixel-diff misses (contrast, reachability, focus, label
  drift). ~7 dev-days. **Approach B (in-repo shadow), not
  Approach A (PR iced upstream)** per
  [dev-note §2.7](dev-notes/ui-testability-deep-dive-2026-05-15.md#27-accessibility-as-a-testing-surface--the-load-bearing-pivot)
  + Q-ACCESSKIT default. Iced upstream
  [issue #552](https://github.com/iced-rs/iced/issues/552)
  remains unmerged as of May 2026.

- **VLM-as-second-opinion judge (`ui-vlm-judge`).** _candidate,
  surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.2`](dev-notes/ui-testability-deep-dive-2026-05-15.md#32-vlm-as-second-opinion-judge--ui-vlm-judge)_
  — bolt a Claude Sonnet 4.6 vision-as-oracle onto the existing
  `crates/ui/tests/fixtures/` test infrastructure, runs on
  `matches_image` failure only as a second-opinion forensic. Three
  locked claims: tooltip visibility, no-overlap, contrast ≥ 4.5:1
  per text label. Pinned model + prompt SHA + N=3 majority vote.
  Reuses [`crates/llm`](../crates/llm) provider trait +
  `BudgetedProvider` cap ($0.50/test run). **Mandatory 2-week
  shadow-mode period before any gating;** see dev-note §3.2 (d)
  for the flakiness mitigation. ~3 dev-days. Analyst spawn when
  operator answers Q-VLM (default: adopt for shadow only).

- **Live-cockpit inspect MCP shim (`ui-inspect-mcp`).**
  _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.1`](dev-notes/ui-testability-deep-dive-2026-05-15.md#31-live-inspect-mcp-shim--ui-inspect-mcp)_
  — feature-gated read-only MCP server inside `cockpit` /
  `cockpit_live` exposing `get_widget_tree()`, `screenshot()`,
  `find_by_label(s)`, `get_widget_bounds(id)`. Listens on
  `127.0.0.1:<port>` + env-var-supplied auth token; off in
  production. Mirrors
  [Slint's testing backend MCP server](https://docs.rs/i-slint-backend-testing/latest/i_slint_backend_testing/).
  Lets the orchestrator (capability-map owner of cockpit launches
  per [AGENT.md ## Capability boundaries](../AGENT.md#capability-boundaries))
  answer structural questions about a running cockpit without a
  manual `Cmd+Shift+4`. ~4 dev-days. Defer to cycle 4+ per dev-note
  §5.2 — close more tactical coverage first. Analyst spawn when
  operator answers Q-MCP (default: defer).

- **Recorded session journal — iced_tester adapter
  (`ui-session-journal-iced-tester`).** _**v0.1 SHIPPED 2026-05-16**
  ([feature.md](ui-session-journal-iced-tester/feature.md) ·
  [tasks.md](ui-session-journal-iced-tester/tasks.md)) — RESCOPED
  2026-05-15 by
  [`iced-014-feature-analysis-2026-05-15.md §5`](dev-notes/iced-014-feature-analysis-2026-05-15.md#recorder--emulator--iced_testsimulator)_
  — supersedes the original 4-dev-day `ui-session-journal`
  candidate. iced 0.14 already ships `iced_tester` (PR #3059) +
  `.ice` text format for record/replay. Adapter work is: enable the
  `record-tests` cargo feature
  ([Q-TESTER-FEATURE LOCKED](dev-notes/iced-014-feature-analysis-2026-05-15.md#migration-questions-for-the-operator)),
  add `--record-tests` boolean flag to `cockpit_live`, wire
  `iced_tester::attach()` around the existing `iced::application(...)`
  call, and add a `tests/journal_replay.rs` walker over committed
  `.ice` files. Export path is **operator-driven via `rfd` native
  file dialog** (spike-corrected from the original `--record-tests
  <path>` plan; see [feature.md ## Design](ui-session-journal-iced-tester/feature.md#design)).
  **~1 dev-day / 6.5 hours.** Two open architect questions remain
  (Q-ARCH-1 builder composition, Q-ARCH-2 replay signature) — both
  carry 15-min M0 spikes before code lands. Status: queued, no spawn
  trigger pulled. Developer pipeline at operator promotion.

- **Mutation testing one-shot pass (`ui-mutants-pass`).**
  _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §3.7`](dev-notes/ui-testability-deep-dive-2026-05-15.md#37-stretch--mutation-testing-pass--ui-mutants-pass)_
  — one-time `cargo mutants --package ui --file
  crates/ui/src/state.rs` run. Produces a triage report of
  surviving mutants in `ui::state::update`. Pairs with
  `ui-update-proptest` above — proptest writes invariants;
  cargo-mutants surfaces which arms still have surviving mutants.
  ~1 dev-day for the run + triage. Quarterly cadence after the
  one-shot lands. Analyst spawn after `ui-update-proptest` shadow
  bedding-in (operator Q-MUTANTS-CADENCE default: one-shot, then
  quarterly).

- **Test reporter — visual-fail HTML artifact (agent contract
  update).** _candidate, surfaced 2026-05-15 by
  [`ui-testability-deep-dive-2026-05-15.md §4.1`](dev-notes/ui-testability-deep-dive-2026-05-15.md#41-testermd--emit-a-structured-fail-artifact-not-just-prose)_
  — on any visual assertion FAIL, the test-runner additionally
  writes a self-contained `spec/<slug>/reports/visual-fail-<ts>.html`
  with baseline/actual/diff PNG triple inline, the assertion that
  fired, file:line, and the VLM judge's verbatim verdict (if
  enabled). Adds a stanza to
  [`.claude/agents/tester.md`](../.claude/agents/tester.md) and a
  ~50-LOC helper in `crates/ui/tests/fixtures/`. ~1 dev-day.
  Analyst spawn when `ui-vlm-judge` or
  `ui-test-harness-viewport-matrix` schedules; pairs with whichever
  lands first.

- **File the iced strategies-Table tiny-skia panic upstream
  (`ui-iced-table-panic-upstream`).** _candidate, surfaced
  2026-05-15 by
  [`iced-014-feature-analysis-2026-05-15.md §6`](dev-notes/iced-014-feature-analysis-2026-05-15.md#the-strategies-table-panic)_
  — minimal repro is already in-tree at
  [`crates/ui/tests/gallery_bisect.rs`](../crates/ui/tests/gallery_bisect.rs).
  Extract the minimal repro, file an issue against
  [iced-rs/iced](https://github.com/iced-rs/iced/issues) modeled
  on prior art issue
  [#2311](https://github.com/iced-rs/iced/issues/2311) (closed via
  PR [#2364](https://github.com/iced-rs/iced/pull/2364)), cite our
  `gallery_bisect.rs` line ranges. **Q-PANEL-UPSTREAM LOCKED:**
  bug report only, no fix PR. ~0.5 dev-day. Revisit by 2026-06-26
  if no upstream activity. Unblocks V5+ of
  [`ui-gallery-bin`](ui-gallery-bin/feature.md) eventually (via
  upstream); the in-tree `ui-gallery-table-cell` workaround above
  unblocks sooner. Analyst spawn at operator promotion.

- **comet debugger evaluation (`ui-comet-eval`).** _candidate,
  deferred 2026-05-15 by
  [`iced-014-feature-analysis-2026-05-15.md §3`](dev-notes/iced-014-feature-analysis-2026-05-15.md#comet-debugger)
  + Q-COMET-EVAL LOCKED_ — comet is pinned at iced
  `0.15.0-dev` (master); does NOT compile against our `=0.14.0`
  pin. **No spawn trigger today.** Revisit when (a) our iced pin
  moves to 0.15.x, OR (b) `ui-inspect-mcp` /
  `ui-session-journal-iced-tester` surface a gap comet would
  close, OR (c) by 2026-11-15 (6-month calendar revisit).

_(historical: the only previously queued item, the presenter smoke test against
operator-success-reports, ran 2026-05-08; surfaced 4 findings, two
of which became skill-plumbing fixes that shipped in commit
`8b139c2`. See Recent below.)_

## Recent (shipped)

- **Drop iced_aw + iced_fonts (`ui-drop-iced-aw` v0.1.0)** — shipped
  2026-05-16. Strategic decoupling from third-party iced ecosystem
  cadence after the 2026-05-16 aborted comet bump made the
  ecosystem-lag pattern explicit. spinner already self-replaced by
  [`widgets/throttled_spinner`](../crates/ui/src/widgets/throttled_spinner.rs);
  badge replaced with native Container+Text in
  [`widgets/strategies::status_badge_cell`](../crates/ui/src/widgets/strategies.rs)
  using the same Lumen palette pairs; date_picker (smoke-test demo
  per docstring) removed entirely with its state, messages, and
  snapshot test. `cargo tree -p ui` confirms zero iced_aw +
  iced_fonts. 1216 workspace tests pass (-8 deleted-as-expected),
  anchors 11/11 PASS. **Net effort: ~3h actual vs ~18h estimate** —
  the date_picker docstring saved 2 dev-days of mistaken
  reimplementation. See
  [`spec/ui-drop-iced-aw/feature.md`](ui-drop-iced-aw/feature.md).

- **headless emulator adapter (`ui-headless-emulator` v0.1.0)** —
  shipped 2026-05-16. Decomposed out of `ui-test-harness-ci` to
  close the unchecked "headless mode" cell from
  [`iced-014-feature-analysis-2026-05-15.md §4`](dev-notes/iced-014-feature-analysis-2026-05-15.md#headless-mode)
  without waiting on viewport-matrix + evaluator prereqs. Single
  test (`crates/ui/tests/headless_emulator_smoke.rs`) boots the
  cockpit through `iced_test::emulator::Emulator`, drains events
  until `Ready`, takes a 1280×720 screenshot — proves the FULL
  iced subscription pump runs without a window server. 1224
  workspace tests pass (+1). ~1 hour actual vs ~2.25h estimate.
  See [`spec/ui-headless-emulator/feature.md`](ui-headless-emulator/feature.md).

- **session journal — iced_tester adapter
  (`ui-session-journal-iced-tester` v0.1.0)** — shipped 2026-05-16
  (commit `218cab3`). Adapter for iced 0.14's `iced_tester::attach`
  (recorder overlay) + `iced_test::run` (replay). Built with
  `--features record-tests` auto-attaches overlay; production
  builds untouched. Empty `recorded-sessions/` ships; operator
  populates post-ship via the recorder workflow. See
  [`spec/ui-session-journal-iced-tester/feature.md`](ui-session-journal-iced-tester/feature.md).

- **iced native widgets (v0.1.0)** — shipped 2026-05-13
  (operator approval recorded as `[x] Approved — ship` in
  [`spec/iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md ## Approval`](iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md#approval);
  evaluator `VERDICT → PASS` at
  [`reports/evaluation-2026-05-13T10-45Z.md`](iced-native-widgets/reports/evaluation-2026-05-13T10-45Z.md)
  on commit `1431409`). Brief A of the iced ecosystem evaluation
  ([predecessor v0.2.0](iced-ecosystem-evaluation/feature.md)) — 4
  hand-rolled cockpit widgets migrated to iced 0.14 native widgets:
  - `crates/ui/src/widgets/positions.rs` → native `Table`
    (commit `9027a0d`, M1 / R1)
  - `crates/ui/src/widgets/strategies.rs` → native `Table` with
    Button-in-column-1 row-click + sibling `Column<error_badges>`
    (commit `3077425`, M2 / R2)
  - `crates/ui/src/widgets/kpi_strip.rs` → native `Grid::new()
    .columns(6).spacing(space::M).height(Length::Shrink)`
    (commit `970e857`, M3 / R3)
  - `crates/ui/src/widgets/journal_transaction_modal.rs` → native
    `Float` positioning wrapping a 3-layer `Stack` (commit
    `9e5bd65`, M4 / R4)
  New shared theme submodule: `crates/ui/src/theme/iced_widget_catalogs.rs`
  exposes `cockpit_table_style_fn` factory (commit `3077425`, T2.0)
  for Brief B `iced_aw` adoption to consume. Native v0.14 `Table::new`
  has no `.style()` setter, so the factory is unused in v0.1.0 —
  consumption deferred to Brief B / v0.2.
  **4-lane parallel dev fan-out worked**: Lanes 2/3/4 spawned in
  parallel (different files, zero overlap); Lane 1 sequenced after
  Lane 2's T2.0 Catalog adapter committed. Each lane = one
  per-widget commit (4 dev commits + 1 tester commit `1431409`).
  Workflow firsts proven:
  - **Second invocation of the test-runner / evaluator split** (first
    was `ui-test-harness-bootstrap` v0.1). Evaluator default-FAIL
    contract held; 20/20 V-items (V1A-V4E) PASS in fresh context.
  - **Orchestrator-direct M0 falsifier batch** (T-M0-J through
    T-M0-N) — 5 grep checks the sub-agent sandbox couldn't run,
    completed in one orchestrator shell pass before dev fan-out
    spawned. Caught 2 architect-spec corrections (`Float::new(1
    arg)` not 2; orphan-rule violation on `impl Catalog`) before
    code was written.
  - **`scripts/orch_supplement_log.sh`** (tooling extracted from
    bootstrap v0.1) supplemented 3 sandbox-denied checks
    (`cargo doc`, shasum, clocks-grep) into the test-runner's
    log — pattern repeatable.
  4 honest architectural divergences flagged inline (orphan-rule
  pivot to StyleFn factory; `Grid::height(Shrink)` AspectRatio
  override; `Float::new(1 arg)` not 2; `Table::new` accepts
  `IntoIterator<Item=T>` looser than `Vec<T>`).
  **Net LOC** +154 (+47 positions / −30 strategies / +8 kpi /
  +29 journal / +100 new Catalog adapter) — the predecessor brief's
  "−900-1100 LOC retired" framing measured file span, not glue
  layer; **actual value is standardization** (idiomatic iced
  widgets, future-proof AccessKit hooks, less hand-rolled
  responsibility, theme adapter scaffold for Brief B). Anchor
  neutrality preserved: 11/11 byte-identical; bootstrap V8 visual
  baseline check carry-forward — 3 PNG SHAs byte-identical
  (Charts screen unaffected). 1203+ workspace tests passing.

- **v2 LLM strategy (v2.0.0)** — shipped 2026-05-13
  (operator approval recorded as `[x] Approved — ship` in
  [`spec/v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md ## Approval`](v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md#approval);
  tester `VERDICT → PASS` at
  [`reports/test-2026-05-12-2219-v2-llm-strategy-final.md`](v2-llm-strategy/reports/test-2026-05-12-2219-v2-llm-strategy-final.md)
  on commit `8a41b47`). Foundation-only per **Q1=A** —
  `Strategy` trait unchanged; first consumer briefs queued
  (reflection-memory-llm-enrichment + reflection-memory-trader-wiring).
  Ships the LLM substrate as callable: real `LlmProvider`
  trait + 3 provider impls (Anthropic / OpenAI-compat /
  Ollama) + retry helper + Anthropic prompt-cache builder
  + `BudgetedProvider` decorator enforcing the $200/mo
  ceiling with auto-degrade at 80% (Q6 + Q11) + strict
  SQLite replay cache (D2 / Q8) + 9-row fixture cache + V9
  secrets grep + 3-provider × 3-role `llm-smoke` harness +
  two operator runbooks (`spec/runbooks/llm-{cost,replay}.md`).
  Q4 bonus rename **`cost::LlmProvider` enum → `ProviderKind`**
  (D1) freed the `LlmProvider` name for the trait; 5 call
  sites + serde wire shape preserved → zero on-disk ledger
  byte change. Q5d "Cache hit ratio" System Health row +
  Q11 denominator `$135 → $200` regenerated both
  `success-fixed-report-sample-{7d,90d}.md` bodies; tester
  re-locked the 2 corresponding anchors at T_FINAL
  (`520b1f29…` / `c656414e…`). The 9 strategy backtest
  anchors at `spec/anchors.toml:15-58` stay byte-identical
  (R14.2 / V8 enforced by T1937 negative-invariant — 11/11
  PASS).
  **Workflow shape**: 6 multi-pass developer cycles
  (`d0bcad2` → `c61afa5` → `441c136` → `f1dbe05` →
  `f1128e9` → `faaaec1`) over 2 days + tester gate
  (`8a41b47`). Two `[~]` partials flipped to `[x]` mid-
  cycle as their dependencies landed (T1912 audit-memo in
  pass 4; T1913 factory Research/Recording arms in pass 5).
  **44/45 dev tasks ticked** (T1938 cockpit "LLM budget"
  tile deferred to v2.1 + T1915 tracing-Layer half deferred
  + 2 pedantic clippy on `audit/src/query.rs:219,221` to
  v2.1 — all consolidated into `v2-llm-strategy-v21-followups`
  candidate). **1203 workspace tests passing, 0 failed.**
  **Unblocks**: Kronos v2.5 forecast overlay, Lumen Phase 6
  Assistant slot, reflection-memory follow-up briefs.
  Brief carries the architect-misdiagnosis-prevention
  workflow rules (Capability boundaries amendment from
  2026-05-12) only informally — the feature shipped on the
  pre-amendment single-tester model; future features apply
  the new test-runner/evaluator split.

- **UI test harness bootstrap (v0.1)** — shipped 2026-05-12
  (operator approval recorded as `[x] Approved — ship` in
  [`spec/ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md ## Approval`](ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md#approval);
  evaluator `VERDICT → PASS` in
  [`reports/evaluation-2026-05-12T13-15Z.md`](ui-test-harness-bootstrap/reports/evaluation-2026-05-12T13-15Z.md)).
  **First feature under the new `AGENT.md ## Capability boundaries`
  regime AND the first run of the test-runner / evaluator split.**
  Implemented week 1 of the 4-week dev-note adoption plan:
  `iced_test::screenshot` smoke test at three operator viewport
  slots (1280×720 / 1920×1080 / 3360×1890 @ 2.0); `image-compare`
  perceptual-diff forensics on snapshot failure; canvas hit-test
  grid sweep over every marker centroid at every viewport — closes
  detection-half of chart-canvas-overhaul V15; viewport-parametric
  helper on `dispatch_canvas_event_for_test`;
  `scripts/check_no_clocks_in_ui_tests.sh` determinism gate.
  Three baseline PNGs committed at
  [`crates/ui/tests/visual-baselines/charts_screen_dark_{floor,typical,operator}.png`](../crates/ui/tests/visual-baselines/).
  **V8 PASS-with-H2-caveat**: render-half of chart-canvas-overhaul
  V15 (visible tooltip card in baseline) deferred to
  `ui-test-harness-canvas-state-seeding` candidate (Queue) per
  operator decision "Commit — V14 covered, V15 partial-accept"
  2026-05-12. New deps: `iced_test = 0.14.0`, `image-compare = 0.4`,
  `image = 0.25.6` (all dev-deps; zero production runtime impact).
  **Workflow meta-deliverables proven**: (a) architect's M0 API audit
  caught a load-bearing `iced_test::Snapshot::png()` assumption
  (method doesn't exist) before code was written; (b) developer
  caught a second `iced_test::screenshot → iced::window::Screenshot`
  API correction during M1 implementation and adjusted cleanly;
  (c) test-runner emitted raw log with honest `[~]` partial for
  4 sandbox-denied checks; (d) orchestrator supplemented those
  checks verbatim; (e) evaluator (read-only, fresh context,
  default-FAIL contract) emitted PASS with file:line cites for
  every V-item. Zero capability-boundary violations across all 6
  agent roles. Anchors PASS 11/11 byte-identical; 818 existing
  tests stay green; 8 net-new tests added; zero non-UI-crate
  changes. Weeks 2 / 3 / 4 follow-ups queued in
  [`## Process / tooling`](#process--tooling).

- **Chart canvas overhaul (v1.10.0)** — shipped 2026-05-12
  (operator approval recorded as `[x] Approved — ship` in
  [`spec/chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md ## Approval`](chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md#approval);
  no overrides, no follow-up notes). Closes the six operator-
  reported items from the v1.9.0 retrospective: price axis on
  the LEFT gutter (USD labels), time axis on the BOTTOM gutter
  (HH:MM UTC), TradingView-style centering via
  `inner_rect_with_gutters`, top-right legend card
  (`PANEL_SUNKEN` fill + `BORDER_STRONG` outline at
  [`crates/ui/src/widgets/chart_legend.rs:156-160`](../crates/ui/src/widgets/chart_legend.rs#L156)),
  viewer parity for `equity_curve` + `drawdown_band`, default
  window bump to 1920×1080 (min stays 1280×720), tooltip card
  clamp to inner-rect bounds. V15 (live tooltip-hover screenshot
  at 3360×1890) DEFERRED to the queued
  `ui-test-harness-bootstrap` v0.1 feature per operator decision
  D4 in [`spec/dev-notes/ui-testing-direction-2026-05-12.md ## Section 9`](dev-notes/ui-testing-direction-2026-05-12.md#9-open-decisions-for-the-operator)
  — the first `iced_test::Simulator::snapshot().matches_image()`
  chart-hover test in that feature replaces the manual capture.
  Q4 local-time x-axis labels DEFERRED to v1.11
  `chart-x-axis-local-time` (Queue entry above; UTC fallback
  ships in v1.10.0). Retrospective surfaced the architect's
  "iced 0.14 canvas-scale bug" misdiagnosis (empirically
  disproved by orchestrator's red-rect + cyan-dot probe; T3002 /
  T3003 / T3007 / T3008 closed as no-op) — produced the
  `AGENT.md ## Capability boundaries` amendment (D5,
  load-bearing) + [`spec/dev-notes/ui-testing-direction-2026-05-12.md`](dev-notes/ui-testing-direction-2026-05-12.md)
  strategy document + `ui-test-harness-bootstrap` v0.1 follow-on
  feature now in Active. Anchor neutrality preserved: 11/11
  byte-identical (`bash scripts/verify_anchors.sh → ANCHORS PASS
  (11 / 11)`); zero changes to `crates/strategy/`, `crates/risk/`,
  `crates/backtest/`, `crates/reports/`, `crates/exec/`,
  `crates/audit/`, `crates/agent/`, `crates/core/`,
  `crates/reflection/`.

- **Chart buy/sell emphasis (v1.9.0)** — shipped 2026-05-11
  (operator verbal approval recorded as `[x] Approved — ship` in
  the presenter deck at
  [`spec/chart-buy-sell-emphasis/presentations/chart-buy-sell-emphasis-2026-05-11.md`](chart-buy-sell-emphasis/presentations/chart-buy-sell-emphasis-2026-05-11.md);
  no overrides, no follow-up notes). UI feature opened directly
  from operator visual feedback on the v1.8 cockpit — markers
  bigger + outlined + line-anchored + 6-field hover tooltip + click-
  through to existing journal_transaction_modal (R4.5 second
  consumer of the tape-row-audit-modal pattern); layered
  fills+signals ghost markers (R5 — signal source default-off via
  new `SignalLogConfig`); three counter views (cumulative-window
  volume tile + per-bar histogram + open-position mirror) above /
  below chart in Layout β; min window size 1280×720; Lumen window
  icon plumbing (macOS dock-icon limitation documented — needs
  `.app` bundle, candidate stub at
  [`spec/cockpit-app-bundle/feature.md`](cockpit-app-bundle/feature.md)).
  Anchor neutrality preserved: zero changes to `spec/anchors.toml`
  (11/11 byte-identical); zero modifications to `crates/strategy/`,
  `crates/risk/`, `crates/backtest/`, `crates/reports/`,
  `crates/exec/`. New additive surface: `crates/audit/migrations/009_strategy_signals.sql`
  + `audit::query::recent_signals` reader + `core::SignalView` type
  + `agent::SignalLogConfig { enabled: false }` + 2 new widgets
  (`chart_tooltip`, `volume_histogram`) + shared window-chrome helper
  (`window_icon`) + Lumen mark RGBA asset. Tester report:
  [`spec/chart-buy-sell-emphasis/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md`](chart-buy-sell-emphasis/reports/test-2026-05-11-2103-chart-buy-sell-emphasis-final.md)
  (V1–V13 all PASS; 1000 / 0 / 4 tests across 144 binaries;
  11/11 anchors PASS). Brief:
  [`spec/chart-buy-sell-emphasis/feature.md`](chart-buy-sell-emphasis/feature.md)
  (status: shipped, version: 1.9.0). Multi-cycle implementation
  arc: initial dev+ui-designer parallel ship + M6 follow-up
  (T2028–T2030) + M6.2 second follow-up (T2031–T2033) + hardening
  pass (corrected T2032 doc rationale + screenshot evidence). The
  iterative loop reflected headless-agent's inability to visually
  verify; addressed long-term by Screen Recording permission grant
  to the host IDE + documented screenshot-verification gate in
  M6.2 task bodies.
- **Reflection memory (v1.8.0)** — shipped 2026-05-10 (operator
  verbal approval recorded as `[x] Approve with notes` in the
  presenter deck at
  [`spec/reflection-memory/presentations/reflection-memory-2026-05-08.md`](reflection-memory/presentations/reflection-memory-2026-05-08.md);
  one note: flip `ReflectionConfig::enable_writer` default from
  `false` to `true` — applied in the same commit as approval).
  Replaces the R6 placeholder body in
  [`crates/reports/src/render/memory_highlights.rs`](../crates/reports/src/render/memory_highlights.rs)
  with real reflection-memory output. New leaf crate
  [`crates/reflection/`](../crates/reflection/) (lib only — types,
  regime + outcome classifiers, deterministic 32-dim embedding,
  post-mortem-analyst card generator, `ReflectionStore` trait + a
  `SqliteReflectionStore` linear-scan top-K impl, bounded mpsc
  writer task with Prometheus drop counter, retrieval API). Wired
  through `crates/agent/src/{config,main}.rs` + `crates/exec/src/paper.rs`.
  Re-locked the two `report-sample-*` anchors at
  `spec/anchors.toml:67-75`; the 9 strategy-backtest anchors at
  lines 15–58 are byte-identical (negative-invariant test t1812
  enforces). Q-resolutions: Q1 = Option A (deterministic v1, no
  LLM dependency); Q4 = report-only (Strategy trait unchanged);
  Q5 = distillation deferred to follow-up brief
  `reflection-memory-distillation`. Tester report:
  [`spec/reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md`](reflection-memory/reports/test-2026-05-08-2114-reflection-memory-final.md)
  (V1–V10 all PASS; 952 / 0 / 3 tests across 124 binaries; 11/11
  anchors PASS; cargo deny advisories/bans/licenses/sources all
  ok). Brief: [`spec/reflection-memory/feature.md`](reflection-memory/feature.md)
  (status: shipped, version: 1.8.0).
- **Presenter smoke test on `operator-success-reports`** — shipped
  2026-05-08 (operator verbal approval recorded as `[x] Approved —
  ship` in commit `587dad7`). Deck at
  [`spec/operator-success-reports/presentations/operator-success-reports-2026-05-08.md`](operator-success-reports/presentations/operator-success-reports-2026-05-08.md);
  pulled evidence from the archived final tester PASS (extracted
  from `spec/archive/pre-lumen-tester-reports-2026-04-to-05-03.tar.gz`)
  + a fresh `cargo test -p reports --test report_scenarios` re-run
  (4/4 PASS, body SHAs match anchors) + a fresh
  `scripts/verify_anchors.sh` PASS (11/11). Surfaced 4 smoke-test
  findings: (1) `present-results` skill missed the archive
  fallback for pre-Lumen tester reports — fixed in `8b139c2`;
  (2) `capture-screenshot` skill defaulted to a manual-capture
  instruction for non-UI features — fixed in `8b139c2` with a third
  "non-UI feature" branch; (3) backlog Recent section had stale
  relative paths inside link parens (cosmetic, fixed in `1a63156`);
  (4) confirmed the audit-immutability call on archived tester
  reports is correct (their internal `spec/features/...` /
  `spec/tasks/...` references describe the layout at time of
  writing). The presenter pipeline is now battle-tested before the
  next real-feature fire.

- **Lumen Phase 5 — HumanControl + AgentFeed rename** — shipped
  2026-05-07 (tester second-pass PASS, presenter approved
  2026-05-08). Tester reports at
  [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md`](lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07-lumen-phase-5-humancontrol-agentfeed.md)
  (first-pass FAIL on fmt drift) and
  [`spec/lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md`](lumen-design-adoption/phase-5-humancontrol-agentfeed/reports/test-2026-05-07b-lumen-phase-5-humancontrol-agentfeed.md)
  (second-pass PASS); brief at
  [`features/lumen-phase-5-humancontrol-agentfeed.md`](features/lumen-phase-5-humancontrol-agentfeed.md)
  (status: `shipped`); presenter deck at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  **First phase to ship net-new operator-write surfaces since v0**:
  HumanControl panel widget (execution-mode segmented control +
  daily loss limit / max position / used-today P&L mirror rows +
  kill button as bottom action) on a new "Control" sidebar entry
  (7th); single-click pause-strategy toggle on Strategies-detail;
  typed-confirm `OVERRIDE` flow for risk-veto override per surfaced
  veto event. Two new audit writers (`strategy_paused`,
  `risk_veto_overridden`) — additive `StrategyEventKind` variants
  with no SQL migration (column already TEXT). Module rename
  `tape` → `agent_feed` via `mv` + git rename detection;
  `Cockpit::tape` field name preserved (Q14) to avoid 100+ test
  ripple. **TD-1 four-phase deferral CLOSED** via Path (b) —
  `crates/ui/src/widgets/focus_ring.rs` Subscription-driven
  custom-widget escape hatch wraps all four destructive surfaces
  with a visible accent-bordered halo on focus. New TD-2 row
  added: risk-engine veto-emit upstream wiring deferred (Phase 5
  ships override surface over an empty live `Vec<VetoEvent>`;
  not a safety primary, an observability gap). Anchor risk: zero
  — verified PASS at ship (11/11 byte-identical post additive
  audit-writer additions). 896 tests passed across 110 binaries
  (46 + 2 net-new vs Phase 4); rust-validate clean (after one-line
  `cargo fmt --all` fixup between tester passes); 86 baselines
  attested by ui-designer (67 panel + 17 widget + 2 audit; 13
  net-new + 9 renamed); R16.3 brand-bleed grep clean. Architect
  ratified 15/15 Q-items with zero principled overrides.
  **Phase 5 is the last shippable phase of the lumen-design-adoption
  initiative absent v2 LLM** — Phase 6 (Assistant slot) is reserved
  until the v2 LLM strategy lands.

- **Lumen Phase 4 — Backtest panel (`viewer` bin)** — shipped
  2026-05-06 (tester second-pass PASS, presenter approved
  2026-05-06). Tester reports at
  [`spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06-lumen-phase-4-backtest-panel.md`](lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06-lumen-phase-4-backtest-panel.md)
  (first-pass FAIL on `clippy::match_same_arms`) and
  [`spec/lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md`](lumen-design-adoption/phase-4-backtest-panel/reports/test-2026-05-06b-lumen-phase-4-backtest-panel.md)
  (second-pass PASS); brief at
  [`features/lumen-phase-4-backtest-panel.md`](features/lumen-phase-4-backtest-panel.md)
  (status: `shipped`); presenter deck at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  Adds the new `viewer` binary at `crates/ui/src/bin/viewer.rs`
  (workspace now ships 3 bins), KPI strip + equity curve + drawdown
  band widgets sharing a refactored `widgets::canvas_chart` core
  with Phase 2's price chart, the cross-phase `core::EquitySeries`
  primitive (rich struct with precomputed drawdown vector inside
  `EquityPoint`), additive `audit::query::equity_curve_for_strategy(strategy_id, since, until)`
  sibling of Phase 2/3 filtered queries, markdown summary parser
  with graceful "—" fallback for missing fields (the 11 anchored
  sample reports omit CAGR + Win rate by design), and **closes the
  Phase 3 deferral** (Strategies-detail sparkline placeholder
  retires; real `widgets::sparkline` lands fed by the new audit
  query). Anchor risk: zero — verified PASS at ship (11/11 byte-
  identical, viewer reads existing committed reports). 850 tests
  passed across 108 binaries (40 + 4 net-new vs Phase 3); rust-
  validate clean (after orchestrator's one-line `match_same_arms`
  fix between tester passes); 72 baselines attested by ui-designer;
  R16.3 brand-bleed grep clean. Architect ratified 12/12 Q-items
  with zero principled overrides (Q1 shape refinement: drawdown_pct
  nested inside `EquityPoint` rather than parallel Vec — eliminates
  length-coupling). **Phase 5 inherits the TD-1 tightening point**:
  Phase 5 ships net-new operator-write controls with typed-confirm
  flows, making the focus-ring deferral (iced still pins `=0.14.0`)
  load-bearing — Phase 5 either folds the iced 0.15+ upgrade or
  commits to the custom-widget escape hatch.

- **Lumen Phase 3 — Detail screens (Strategies / Risk / Audit)** —
  shipped 2026-05-05 (tester first-pass PASS, presenter approved
  2026-05-06). Tester report at
  [`spec/lumen-design-adoption/phase-3-detail-screens/reports/test-2026-05-05-lumen-phase-3-detail-screens.md`](lumen-design-adoption/phase-3-detail-screens/reports/test-2026-05-05-lumen-phase-3-detail-screens.md);
  brief at
  [`features/lumen-phase-3-detail-screens.md`](features/lumen-phase-3-detail-screens.md)
  (status: `shipped`); presenter deck at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  Adds Strategies / Risk / Audit sidebar entries (3 → 6) + per-screen
  detail bodies, additive `008_journal_transactions_venue.sql`
  migration (default `'binance'` backfill), `post_fill` writer's
  new `venue: Venue` parameter wired across ~25 call-sites,
  `RiskTelemetry` channel on `agent::EventBus` mirroring `MarketHealth`,
  sibling audit query `recent_journal_filtered` (additive to
  `recent_fills_filtered`), kill-threshold proximity gauge with
  tri-band ramp (`UP_500` ≤70% → `WARN_500` >70% → `DOWN_500` >90%),
  Audit screen filter chip-row (venue · symbol · kind · time-range)
  + fixed 250-row pagination + reuse of T1208 modal, cross-link
  Home → Strategies-detail. Two developer passes (pass 1 cut at
  clean tick boundary after T1701 + T1703 due to context budget;
  pass 2 ticked T1702 migration + T1704–T1716). Architect ratified
  11/11 Q-items with one deferral (Q6 equity-since-deploy sparkline
  → Phase 4, since the cheap path doesn't exist on the current
  state shape and Phase 4 needs the same equity-history primitive).
  Anchor risk: zero — verified PASS at ship (11/11 byte-identical
  post-migration, verified twice during dev pass + once at tester
  gate). 810 tests passed across 104 binaries (29 + 6 net-new vs
  Phase 2); rust-validate clean (fmt + clippy `-D warnings` +
  cargo-deny + docs); 65 baselines attested by ui-designer
  (zero `unknown` token escapes); R16.3 brand-bleed grep clean.

- **Lumen Phase 2 — Shell IA + Charts** — shipped 2026-05-05.
  Tester first-pass `VERDICT → PASS`; report at
  [`spec/lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md`](lumen-design-adoption/phase-2-shell-ia-charts/reports/test-2026-05-05-lumen-phase-2-shell-ia-charts.md).
  Brief at
  [`features/lumen-phase-2-shell-ia-charts.md`](features/lumen-phase-2-shell-ia-charts.md)
  (status: `shipped`). Presenter deck approved by operator at
  [`spec/lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md).
  Adds left-sidebar shell (fixed 180 px, T1507-styled, no icons),
  Screen routing (`Cockpit::current_screen` × six variants — Home /
  Debug / Charts wired in Phase 2; Strategies / Risk / Audit
  declared for Phase 3), Home screen (Phase 1 widgets re-housed),
  Debug screen (kill / latency / market health / server time /
  version / placeholder logs), Charts screen (chip-row symbol
  selector + canvas line-series price plot + buy/sell triangle
  markers from `recent_fills_filtered`), per-`(Venue, Symbol)`
  rolling `ChartBuffer` (cap 60 1-min bars), live mode via existing
  `bars_tx`, fixtures mode via deterministic `synthetic_candles`,
  additive `audit::query::recent_fills_filtered(venue, symbol,
  since, until)`, right-rail Phase 6 Assistant slot reservation
  (`Length::Fixed(0.0)`). Anchor risk: zero — verified PASS at
  ship (11/11 anchors byte-identical). 781 tests passed across 98
  binaries (24 + 2 net-new vs Phase 1); rust-validate clean
  (fmt + clippy `-D warnings` + cargo-deny + docs); 53 baselines
  attested by ui-designer (zero `unknown` token escapes); R16.3
  brand-bleed grep clean. All 11 architect Q-resolutions ratified
  with zero deviations from analyst recommendation. **Phase 3
  prerequisite carried forward**: additive `journal_transactions.venue`
  column migration needed before non-Binance fills can populate
  the chart's marker query.

- **Lumen Phase 1 — Foundation (tokens + tiers + status bar)** —
  shipped 2026-05-04. Tester third-pass `VERDICT → PASS`; report at
  [`spec/lumen-design-adoption/phase-1-foundation/reports/test-2026-05-04c-lumen-phase-1-foundation.md`](lumen-design-adoption/phase-1-foundation/reports/test-2026-05-04c-lumen-phase-1-foundation.md).
  Brief at
  [`features/lumen-phase-1-foundation.md`](features/lumen-phase-1-foundation.md)
  (status: `shipped`). Replaced the 12-token theme with the full
  Lumen palette (warm + cool neutrals, accent ramp, sage / clay /
  warn / info semantics, both light and dark modes); added Tier
  0/1/2/3 elevation surface tokens; added whisper shadows + sunken
  inset; added focus ring (Q11 / TD-1 deviation: hover-state ring
  on buttons + ACCENT border-shift on focused inputs as bounded
  approximation, two named upgrade triggers); extended spacing /
  radii / typography ladders to the full Lumen scale; added motion
  tokens; applied Tier 1 styling to existing 6 widgets; applied
  sunken styling to the kill-confirm input; applied the active-row
  2 px left rule to tabular widgets; added a new
  `widgets::status_bar` widget rendering connection / latency /
  account / server time always-visible at the bottom of the shell;
  refreshed the existing 36 panel snapshot baselines (5 net-new for
  T1506 / T1508 = 41 total); superseded
  [`spec/ui-design-principles.md`](ui-design-principles.md) with a
  Lumen-anchored rewrite. Anchor risk: zero — verified PASS at
  ship (11/11 anchors byte-identical). 757 tests passed across 96
  binaries; rust-validate clean (fmt + clippy + cargo-deny + docs);
  R16.3 brand-bleed grep clean. Unblocked the 2026-05-04 master
  roadmap revision (4 → 6 phases).
- **v1.5b multi-venue + 1s aggregated trades** — shipped 2026-05-03.
  Brief at
  [`features/v1-5b-multi-venue.md`](features/v1-5b-multi-venue.md).
  Coinbase + Kraken adapters, USDC pair mirror set (10 symbols),
  T612 multi-symbol live `BinanceFeed`, 1 s aggregated trades,
  `Venue` enum on `Tick` / `Bar`, per-venue feed-reconnect
  provenance. Plumbing-only — expanded the data side, not the
  execution side. 15 R-items, 12 V-items, 12 open questions
  resolved. Anchor risk: zero by construction (no `venue` strings
  in any committed report body). Closes v1.5a Q5 (USDC pairs
  blocker) and v1 closeout T612. The cockpit / `cockpit_live` is
  now stable on the data side; this is the clean window the
  Lumen design adoption initiative lands into.

## Conventions

- One-line description; deeper context lives in the eventual
  `spec/<slug>/feature.md` brief.
- The orchestrator owns this file; agents may suggest additions but
  the operator approves promotions.
- Items can stay here indefinitely. Stale items get a `_decayed_` tag
  rather than silent deletion so the orchestrator can revisit.

## Changelog

- 2026-05-16 (analyst, Wave 2a spec-hygiene — bookkeeping flips
  per [`spec/dev-notes/feature-triage-2026-05-16.md`](dev-notes/feature-triage-2026-05-16.md)):
  five frontmatter / supersession updates landed, zero code touched,
  zero anchors moved, zero `trace.toml` rows added.
  - **v0-paper-sma** → `shipped` (was `in-progress`; row A1 —
    tasks 0/35, smoke checklist + 2 backtest reports + locked
    anchor on disk; bookkeeping flip).
  - **v05-composed-strategies** → `shipped` (was `in-progress`;
    row A2 — drift reconcile: `tasks.md` already carried
    `status: shipped` from 2026-04-20; `feature.md` now matches).
  - **v1-cross-sectional-momentum** → `shipped` (was
    `in-progress`; row A3 — `T_FINAL_A_v1` / `T_FINAL_B_v1` both
    ticked since 2026-04-30; the single open box **T612** —
    multi-symbol live `BinanceFeed` — stays `[ ]` under v1.5
    lineage with carrier
    [`v1-5b-multi-venue`](v1-5b-multi-venue/feature.md); note
    added to the v1 `tasks.md ## Notes`).
  - **ui-gallery-bin** → `shipped` with body header marking
    **v0.1-partial, terminal** + `version: 0.1.0 →
    0.1.0-partial-terminal` + new `successor:
    ui-gallery-table-cell` frontmatter field (row A4 — V5+
    blocked by `tiny-skia` `Build quad rectangle` panic in
    `widget::table::Table` bisected to `GALLERY_CELLS[7]`). The
    39 open task boxes are marked `[deferred to
    ui-gallery-table-cell]` at the top of
    [`ui-gallery-bin/tasks.md`](ui-gallery-bin/tasks.md) and
    preserved verbatim for trace. **Precedent question** raised
    in the feature.md Changelog: the `spec-update` status enum
    has no `shipped-partial`; this edit took the conservative
    `status: shipped` + version-modifier route and proposed
    formal enum promotion at the next skill revision.
  - **ui-gallery-table-cell** → new draft (successor brief
    opened at
    [`spec/ui-gallery-table-cell/feature.md`](ui-gallery-table-cell/feature.md)
    + [`tasks.md`](ui-gallery-table-cell/tasks.md)). Owns R1
    (restore V5+ render path: full
    `GALLERY_CELLS` set renders without panic). V5+ tasks
    re-keyed `T1..T9` + `T-FINAL-*`; M0 architect block
    re-spawned for Q-FIX-STRATEGY (special-case wrapper /
    swap render / upstream fix). HANDOFF → architect.
- 2026-05-15 (operator, iced-014 lock + re-sequence): locked all 5
  Q-* migration questions from
  [`iced-014-feature-analysis-2026-05-15.md`](dev-notes/iced-014-feature-analysis-2026-05-15.md)
  at analyst defaults (Q-014-PIN, Q-COMET-EVAL, Q-TESTER-FEATURE,
  Q-PANEL-UPSTREAM, Q-D3-RELITIGATE). Backlog impact: (a) RESCOPED
  `ui-session-journal` → `ui-session-journal-iced-tester` (~4d →
  ~1d) since iced 0.14 ships `iced_tester` + `.ice` format
  natively; (b) CHEAPENED `ui-test-harness-ci` 5d → 4d via the
  shipped `iced_test::emulator::Emulator` + embedded Fira Sans;
  (c) PROMOTED new candidate `ui-iced-table-panic-upstream`
  (0.5d) to file the strategies-Table tiny-skia panic upstream;
  (d) ADDED `ui-comet-eval` as deferred-no-trigger candidate
  (comet requires iced 0.15-dev, not compatible with our pinned
  `=0.14.0`). All other queued items unchanged.
- 2026-05-15 (analyst, ui-testability deep-dive): authored
  [`spec/dev-notes/ui-testability-deep-dive-2026-05-15.md`](dev-notes/ui-testability-deep-dive-2026-05-15.md)
  — a research dev-note critiquing the four-week plan in
  [`ui-testing-direction-2026-05-12.md`](dev-notes/ui-testing-direction-2026-05-12.md),
  surfacing 3 structural blind spots (canvas-state ownership
  via H2 caveat, pixels-only oracle, no reachability coverage)
  + 8 schedulable proposals across a re-shaped Layer 0..7
  pyramid. Promoted 8 new candidate features into ## Queue ##
  Process / tooling above (ui-contrast-asserter,
  ui-update-proptest, ui-gallery-bin, ui-a11y-shadow,
  ui-vlm-judge, ui-inspect-mcp, ui-session-journal,
  ui-mutants-pass, plus a tester visual-fail HTML artifact
  agent-contract update). Each item points to a specific §
  anchor in the dev-note for the operator's "schedule which"
  pick. Existing
  [`ui-test-harness-ci`](#queue) candidate annotated with a
  +1-day cross-platform falsifier to revisit operator decision
  D3 (macOS-only CI) per dev-note §5.3 + §2.6. No code
  changes; pure spec output. Six open operator questions
  surfaced in dev-note §6 (Q-VLM, Q-ACCESSKIT, Q-MCP,
  Q-GALLERY-SCOPE, Q-D3-REVISIT, Q-MUTANTS-CADENCE) — each
  has a documented default so the operator may sequence
  features without a separate Q&A round.
- 2026-05-12 (analyst, second pass): promoted
  `ui-test-harness-bootstrap` v0.1 from Queue ## Process / tooling
  → Active. First feature under the new
  [`AGENT.md ## Capability boundaries`](../AGENT.md#capability-boundaries)
  regime. Scope locked to **week-1 only** of the
  [dev-note §6 4-week plan](dev-notes/ui-testing-direction-2026-05-12.md#6-phased-adoption--4-week-plan)
  per operator decisions D1–D5; weeks 2 / 3 / 4 are separate
  candidate features queued under ## Queue ## Process / tooling for
  later analyst spawn. The week-1 grid-sweep test at 3360×1890
  closes chart-canvas-overhaul V15 (D4) without a manual
  screencapture. Brief at
  [`ui-test-harness-bootstrap/feature.md`](ui-test-harness-bootstrap/feature.md);
  task stubs at
  [`ui-test-harness-bootstrap/tasks.md`](ui-test-harness-bootstrap/tasks.md).
  HANDOFF → architect (Design section + task body fleshing). Q1
  (cockpit factory), Q2 (iced_test snapshot accessor), Q3-Q7
  architect-decide; Q8 (image-compare yes/no), Q9 (fixture
  parity), Q10 (baseline naming) operator-input.
- 2026-05-12 (architect, chart-canvas-overhaul M7): queued
  **v1.11 — Chart x-axis local time (`chart-x-axis-local-time`)**
  under UI / cockpit Queue as a `candidate`. Deferred from v1.10.0
  by operator-locked Q-revised-1 = path (b) (defer Q4 to v1.11
  follow-up). One-line scope: workspace `time` `local-offset`
  feature flag flip + `local_offset_or_utc()` body wire-up; full
  details for the v1.11 analyst.
- 2026-05-08 (orchestrator, post-Phase-5 cleanup): drained the
  Active section. The 5 prior Active entries (`live-cockpit-unified`,
  `real-mtm-unrealized-pnl`, `per-symbol-position-accounts`,
  `tape-row-audit-modal`, `journal-transactions-metadata`) all
  shipped in v1+ → operator-success-reports cycle and are referenced
  as shipped invariants in the Lumen master roadmap's cross-feature
  invariants table; never moved out of Active. Plus the
  `lumen-design-adoption` master-roadmap row dated 2026-05-03 still
  referenced the obsolete 4-phase plan. All six Active entries
  removed (organizational hygiene; their feature briefs remain at
  `spec/<slug>/feature.md`). UI / cockpit Queue subsection
  collapsed to just Phase 6 (the only remaining initiative work).
  Initiative status: 5-of-6 phases shipped; reaches a natural pause
  point absent v2 LLM. No new feature promoted.
- 2026-05-08 (presenter, Phase 5 sprint review APPROVED): operator
  signed the Phase 5 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 5 closed cleanly: tester second-
  pass PASS (8/8 gates after a one-line `cargo fmt --all` fixup
  between passes; first-pass FAIL preserved on disk for audit), 896
  tests across 110 binaries, 11/11 anchors byte-identical, 86
  baselines attested clean, **TD-1 four-phase deferral CLOSED** via
  Path (b) custom-widget escape hatch, new TD-2 row added for the
  deferred risk-engine veto-emit upstream wiring. **Phase 5 is the
  last shippable phase of the lumen-design-adoption initiative
  absent v2 LLM** — Phase 6 (Assistant slot) is reserved.
  Initiative status: 5-of-6 phases shipped; Phase 6 unlocks when
  v2 LLM is approved. No new active feature promoted from this
  approval; next-step decision is operator's (promote v2 LLM,
  promote a different Active backlog item, or pause the cockpit-
  side initiative as 5-phase-complete).
- 2026-05-06 (presenter, Phase 4 sprint review APPROVED): operator
  signed the Phase 4 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 4 closed cleanly: tester second-
  pass PASS (8/8 gates after a one-line `match_same_arms` fix
  between passes; first-pass FAIL preserved on disk for audit), 850
  tests across 108 binaries, 11/11 anchors byte-identical, 72
  baselines attested clean, three-bin workspace (`viewer` greenfield
  + `cockpit` + `cockpit_live`). Promoted `lumen-phase-5-humancontrol-agentfeed`
  from Queue → Active per the master-roadmap sequencing constraint
  (Constraint 3). Phase 5 brief stub frontmatter bumped from
  `queued` → `active`; the analyst's next pass expands the stub
  into a full brief. **Phase 5 is the first phase to introduce
  net-new operator-write paths** (pause-strategy, override-risk-
  veto, execution-mode toggle), making it the load-bearing decision
  point for the TD-1 focus-ring deferral. Mechanical pre-tick gate
  at sprint-review time PASSED before approval. HANDOFF → analyst
  (Phase 5 brief expansion).
- 2026-05-06 (presenter, Phase 3 sprint review APPROVED): operator
  signed the Phase 3 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 3 closed cleanly: tester first-pass
  PASS (8/8 gates), 810 tests across 104 binaries, 11/11 anchors
  byte-identical post-migration, 65 baselines attested clean, two
  developer passes (pass 1 cut at clean tick boundary; pass 2
  finished T1702 migration + remaining 14 tasks). Promoted
  `lumen-phase-4-backtest-panel` from Queue → Active per the
  master-roadmap sequencing constraint (Constraint 3). Phase 4
  brief stub frontmatter bumped from `queued` → `active`; the
  analyst's next pass expands the stub into a full brief with
  R-items / V-items / Q-items + task list contract. **Phase 4 will
  absorb the deferred Q6 sparkline from Phase 3** — both surfaces
  need the same equity-history primitive. Mechanical pre-tick gate
  at sprint-review time PASSED before approval (`PRESENTATION
  CHECK PASS`). HANDOFF → analyst (Phase 4 brief expansion).
- 2026-05-05 (presenter, Phase 2 sprint review APPROVED): operator
  signed the Phase 2 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 2 closed cleanly: tester first-pass
  PASS (8/8 gates), 781 tests across 98 binaries, 11/11 anchors,
  53 baselines attested clean. Promoted `lumen-phase-3-detail-screens`
  from Queue → Active per the master-roadmap sequencing constraint
  (Constraint 3). Phase 3 brief stub frontmatter bumped from `queued`
  → `active`; the analyst's next pass expands the stub into a full
  brief with R-items / V-items / Q-items + task list contract.
  Mechanical pre-tick gate at sprint-review time PASSED before
  approval (`PRESENTATION CHECK PASS`). HANDOFF → analyst (Phase 3
  brief expansion).
- 2026-05-04 (presenter, Phase 1 sprint review APPROVED): operator
  signed the Phase 1 sprint review deck at
  [`lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md`](lumen-design-adoption/presentations/lumen-design-adoption-2026-05-04-to-05-08.md)
  (`[x] Approved — ship`). Phase 1 closed cleanly. Promoted
  `lumen-phase-2-shell-ia-charts` from Queue → Active per the
  master-roadmap sequencing constraint (Constraint 3). Phase 2
  brief stub frontmatter bumped from `queued` → `active`; the
  analyst's next pass expands the stub into a full brief.
  Mechanical pre-tick gate at sprint-review time PASSED before
  approval (`PRESENTATION CHECK PASS`). HANDOFF → analyst (Phase 2
  brief expansion).
- 2026-05-04 (analyst, post-Phase-1-ship roadmap revision):
  `lumen-phase-1-foundation` shipped → moved Active → Recent.
  Lumen master roadmap revised from 4 to 6 phases at operator
  request (session of 2026-05-04, after Phase 1 third-pass tester
  PASS). Two new phases inserted ahead of the original phase plan:
  Phase 2 Shell IA + Charts (sidebar nav + Home / Debug / Charts
  screens + price chart with buy/sell markers + read-only audit
  query extension), Phase 3 Detail screens (Strategies / Risk /
  Audit sidebar entries over existing backend data). Original
  Phase 2 (Backtest panel) renumbered to Phase 4; original Phase 3
  (HumanControl + AgentFeed) renumbered to Phase 5; original Phase
  4 (Assistant slot) renumbered to Phase 6 (still reserved for v2
  LLM). Five new feature-brief stubs spawned. Operator decisions
  captured as Q11–Q14 in the master open-questions section
  (sidebar fixed-width; chart in both bins; extend `audit::query`
  for marker filtering; keep Phase 2 / 3 split). Anchor risk per
  phase table extended to 6 rows; cross-feature invariants table
  extended to 6 phase columns. UI / cockpit Queue subsection
  rewritten for the 6-phase plan. HANDOFF → architect at Phase 2
  promote (when the operator signs Phase 1 presentation).
- 2026-05-01 (orchestrator): initial draft. Captures the 5 followups
  surfaced at `operator-success-reports` ship; promotes
  live-cockpit-unified to Active.
- 2026-05-01 (analyst): live-cockpit-unified Active line updated to
  reference the just-written
  [`features/live-cockpit-unified.md`](features/live-cockpit-unified.md)
  brief.
- 2026-05-02 (analyst): promoted "Real mark-to-market unrealized P&L"
  from Queue → Active. Brief at
  [`features/real-mtm-unrealized-pnl.md`](features/real-mtm-unrealized-pnl.md);
  HANDOFF → architect.
- 2026-05-02 (analyst): promoted "R10 follow-up:
  per-symbol-position-accounts" from the implicit Queue (deferral
  note in `real-mtm-unrealized-pnl.md` Design § Q3 / R10 verdict) →
  Active. Brief at
  [`features/per-symbol-position-accounts.md`](features/per-symbol-position-accounts.md);
  HANDOFF → architect.
- 2026-05-03 (orchestrator): new UI / cockpit subsection. Added
  `tape-row-audit-modal` per operator decision on UI principles Q4
  (2026-05-03). Promotes when operator picks it up; analyst → architect
  → developer pipeline standard.
- 2026-05-03 (analyst): promoted `tape-row-audit-modal` from Queue
  (UI / cockpit) → Active. Brief at
  [`features/tape-row-audit-modal.md`](features/tape-row-audit-modal.md).
  First feature to land against
  [`ui-design-principles.md`](ui-design-principles.md) (the "Show
  the why" cockpit click-through-to-audit path begins here). 15
  R-items, 11 V-items, 9 open questions for the architect. Anchor
  risk: zero (pure UI + new additive audit reader). HANDOFF →
  architect. The now-empty `### UI / cockpit` Queue subsection has
  been removed; future UI additions will recreate it.
- 2026-05-03 (analyst): added `journal-transactions-metadata` to
  Active. Brief at
  [`features/journal-transactions-metadata.md`](features/journal-transactions-metadata.md).
  Closes the T1206 deviation note in the just-shipped
  [`features/tape-row-audit-modal.md`](features/tape-row-audit-modal.md)
  (Implementation § Async dispatch, lines 625–635) — live cockpit's
  modal header is empty because T1202's reader stays narrow
  (entries-only). New sibling reader + `core::JournalTransactionMetadata`
  struct + cockpit_live `Task::perform` chain. 7 R-items, 5 V-items,
  6 open questions for the architect. Anchor risk: zero (additive
  read-only). HANDOFF → architect.
- 2026-05-03 (analyst): promoted `v1.5b multi-venue + 1s aggregated
  trades` from Queue (Strategy) → Active. Brief at
  [`features/v1-5b-multi-venue.md`](features/v1-5b-multi-venue.md).
  **Largest queued backend feature.** Coinbase + Kraken adapters,
  USDC pair mirror set (10 symbols), T612 multi-symbol live
  `BinanceFeed` (the v1 closeout deferral lands here), 1s
  aggregated trades, `Venue` enum on `Tick` / `Bar`, per-venue
  feed-reconnect provenance (T805 extension). Plumbing-only —
  expands the data side, not the execution side. 15 R-items,
  12 V-items, 12 open questions for the architect. Anchor risk:
  zero by construction (architect-confirmed grep of
  `spec/reports/**/*.md` for `venue`/`coinbase`/`kraken`).
  Cost risk: zero — all three venues have free public
  market-data WS APIs. Failover risk: medium — N independent
  failure modes; per-venue tokio tasks the recommended
  isolation strategy. Closes v1.5a Q5 (USDC pairs blocker) and
  v1 closeout T612. HANDOFF → architect.
- 2026-05-03 (analyst): `v1.5b multi-venue + 1s aggregated
  trades` shipped (verdict PASS, presenter approved). Moved
  Active → Recent. Promoted the
  [`lumen-design-adoption`](features/lumen-design-adoption.md)
  master roadmap + the
  [`lumen-phase-1-foundation`](features/lumen-phase-1-foundation.md)
  first-phase brief to Active. Master roadmap covers the 4-phase
  Lumen design-system adoption: Phase 1 Foundation (Active —
  tokens + tiers + status bar + principles supersede), Phase 2
  Viewer Backtest (Queue — KPI strip + equity curve + drawdown
  band on the offline review surface), Phase 3 HumanControl +
  AgentFeed rename (Queue — richer override controls + tape →
  AgentFeed module rename), Phase 4 Assistant slot (Reserved —
  depends on v2 LLM strategy ship). Operator-locked constraints
  inherited at every phase: NO brand adoption (no "Lumen" name,
  no eye/lens logo), NO voice rules rewrite (`ui::strings`
  unchanged), sequential phasing (one approval per phase), dark
  as default. Phase 1 brief carries 17 R-items, 9 V-items, 9
  open questions for the architect. Anchor risk per phase:
  zero (Phase 1 / 2 / 3 are UI features, no backtest path
  touched); Phase 4 out of scope. The 11 / 11 backtest body-SHA-256
  anchor regression goal stays byte-identical across the entire
  initiative. Cross-feature invariants documented for the 7 prior
  shipped UI-touching features (operator-success-reports,
  live-cockpit-unified, real-mtm, per-symbol, tape-modal,
  journal-tx-metadata, v1.5b). HANDOFF → architect (Phase 1
  first; master roadmap for orientation).

- 2026-05-12 (operator): v1.10.0 SHIPPED — `chart-canvas-overhaul`.
  Recent entry added above. The architect-misdiagnosis
  retrospective produced
  [`AGENT.md ## Capability boundaries`](../AGENT.md#capability-boundaries-orchestrator-vs-sub-agent)
  amendment +
  [`spec/dev-notes/ui-testing-direction-2026-05-12.md`](dev-notes/ui-testing-direction-2026-05-12.md)
  strategy document + queued `ui-test-harness-bootstrap` v0.1
  feature now in Active per operator decisions D1-D5. Anchors
  PASS 11/11 (verbatim line in deck `## Live demo` block).
  Approval evidence:
  [`chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md ## Approval`](chart-canvas-overhaul/presentations/chart-canvas-overhaul-2026-05-12.md#approval).

- 2026-05-12 (operator): v0.1 SHIPPED — `ui-test-harness-bootstrap`.
  First feature under the new
  [`AGENT.md ## Capability boundaries`](../AGENT.md#capability-boundaries-orchestrator-vs-sub-agent)
  regime AND first run of the test-runner / evaluator split.
  Empirical proof the new workflow holds: 0 capability-boundary
  violations across 6 agent roles; 2 architect/developer-side
  API corrections surfaced cleanly without rework loops; evaluator
  PASS in fresh read-only context with file:line cites; anchors
  PASS 11/11. V8 (render-half of chart-canvas-overhaul V15)
  PASS-with-H2-caveat — queued as
  `ui-test-harness-canvas-state-seeding` candidate. Approval
  evidence:
  [`ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md ## Approval`](ui-test-harness-bootstrap/presentations/ui-test-harness-bootstrap-2026-05-12.md#approval).

- 2026-05-13 (operator): v2.0.0 SHIPPED — `v2-llm-strategy`.
  Foundation-only per Q1=A. 6 dev passes (`d0bcad2` →
  `faaaec1`) + tester (`8a41b47`). 44/45 dev tasks ticked +
  T_FINAL [x]. 11/11 anchors PASS (2 report-sample-*
  re-locked by tester to v2.0.0 SHAs; 9 strategy anchors
  byte-identical). 1203 workspace tests passing. D1/D2/D3
  operator-locked decisions honored. 3 deferred items
  consolidated into `v2-llm-strategy-v21-followups`
  candidate. Unblocks Kronos v2.5 + Lumen Phase 6 +
  reflection-memory follow-ups. Approval evidence:
  [`v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md ## Approval`](v2-llm-strategy/presentations/v2-llm-strategy-2026-05-13.md#approval).

- 2026-05-13 (operator): v0.1.0 SHIPPED — `iced-native-widgets`
  (Brief A of iced ecosystem evaluation). 4-lane parallel dev
  fan-out (positions / strategies Tables + kpi_strip Grid +
  journal_modal Float) across commits `3077425` → `970e857` →
  `9e5bd65` → `9027a0d` → tester `1431409`. Second invocation
  of the test-runner / evaluator split; evaluator default-FAIL
  contract held; 20/20 V-items PASS; 11/11 anchors byte-identical;
  bootstrap V8 visual baselines preserved (3 PNG SHAs unchanged);
  1203+ workspace tests passing. New shared
  `crates/ui/src/theme/iced_widget_catalogs.rs` Catalog adapter
  scaffold for Brief B (iced_aw cherry-pick: date_picker +
  spinner + badge). 4 honest architectural divergences from
  architect's brief flagged + corrected inline. Approval
  evidence:
  [`iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md ## Approval`](iced-native-widgets/presentations/iced-native-widgets-2026-05-13.md#approval).
