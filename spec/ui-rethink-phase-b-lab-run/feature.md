---
slug: ui-rethink-phase-b-lab-run
status: draft
owner: pending-architect
updated: 2026-05-19
version: 0.1.0
predecessor: ui-rethink-phase-a-lab v0.2.0
---

# UI rethink Phase B — Lab Run button (`ui-rethink-phase-b-lab-run`)

> This brief is the **second concrete feature** carved out of the broader
> UI rethink at
> [`spec/dev-notes/ui-rethink-2026-05-17.md`](../dev-notes/ui-rethink-2026-05-17.md).
> The dev-note's §6 Phase B is the spec source of truth; this brief is
> the **implementation contract** for that slice. Predecessor:
> [`ui-rethink-phase-a-lab v0.2.0`](../ui-rethink-phase-a-lab/feature.md)
> shipped 2026-05-18 — the Lab vertical (chart + chip widgets + tuple
> persistence + `Run` button that **stubs** the engine call and reads
> equity from `spec/<strategy>/reports/` via `EquityCache`) lands as
> the foundation. Phase B promotes the `Run` button from "stubbed
> placeholder result + cached-report read" to **"actually run a
> backtest in-process and render the live result on the chart."**

## Why

Phase A shipped the full Lab vertical end-to-end — wiring, types,
runner glue, `RunCancelHandle/Receiver`, `Message::LabRunRequested`/
`LabRunCompleted`, `EquityCache::invalidate`, `RunState` machine — but
intentionally **stubbed the engine call**:

- `crates/backtest/src/engine.rs:220-240` — `run_scenario(cfg)`
  validates seed + range, then returns `Err(RunError::NotImplemented)`.
- `crates/ui/src/lab/runner.rs:197-206` — `spawn_lab_run` in the `live`
  arm carries an explicit `TODO-backtest-dep: wire
  backtest::engine::run_scenario here once T-D-13 lands`, returns a
  simulated `RunSummary` with `report_path: None`.
- `crates/ui/src/lab/equity_loader.rs` — the chart's equity overlay
  reads from disk via `EquityCache::get_or_load` after `invalidate` is
  called on `LabRunCompleted(Ok(...))`. Without a real engine invocation
  the cache miss falls through to whatever cached report (if any)
  matches the tuple — operator sees "stale or absent" data on a fresh
  Run press.

The two deferred questions the dev-note §6 Phase B promised to close:

1. **Library-callable backend** — Phase A's open question. **Status:
   already confirmed** by analyst pass. `crates/backtest/src/lib.rs`
   exports `engine::run_scenario`, `RunReport`, `ScenarioConfig`,
   `BacktestKpis`, `DateRange`, `ParamSheet`, `MatchingEngine`,
   `RunError` — the **type surface is library-shaped**. The
   `run_scenario` body is the work item, not the API extraction.
2. **End-to-end Lab vertical** — the operator's **J2** workflow
   ("test a strategy against this pair AND this date range, see how
   successful the selection is" — `product.md` §Differentiator + J2
   in dev-note §4) completes only when the Run button produces a
   **fresh result** without an out-of-band `cargo run --bin backtest`
   trip. The CLI-hint shortcut is friction; Phase A's empty-state
   accepts the friction; Phase B removes it.

Phase B closes both gaps in one slice.

## Scope (dev-note §6 Phase B — refined by analyst)

- **Backend cross-cut (revised — refactor scope).** The library
  **type surface** is already shipped (Phase A's T-D-12 / T-D-13);
  Phase B **populates** `run_scenario`'s body by extracting the
  scenario-dispatch logic from `crates/backtest/src/main.rs`
  (3417 LOC, 7 scenarios across 4 backtest paths — see §
  Architecture finding). The standalone bin becomes a thin
  CLI-parsing wrapper that builds a `ScenarioConfig` and calls
  `engine::run_scenario`. **CLI behaviour byte-identical** — all 22
  anchors must stay green.
- **Wire the Lab `Run` button** to call the real engine and populate
  `lab_state.last_run_report` (new field) directly. The chart's
  equity overlay reads from this in-memory mirror on the same
  iced-update cycle; the on-disk report write (when `write_report =
  true`) is a side-effect, not the chart's read path.
- **Add the "compare to previous run" affordance** — diff the
  current `lab_state.last_run_report` against `lab_state.prev_run_report`
  (the result of the prior Run press in the same session) to render
  a small KPI delta badge (Δ P&L, Δ MaxDD, Δ Sharpe) near the Run
  button. Pre-Run state preserved on each Run press.

## Out of scope

- Phase C (sidebar IA flip), Phase D (Trail), Phase E (Compare matrix),
  Phase F (Memory + Models + Assistant slot). Each is its own brief.
- New backtest engine internals — Phase B is wiring + scenario-
  dispatch extraction, not strategy code or matching-engine work.
- **Inline param-sheet editor** (Q-Arch-1 in Phase A § 9) — the
  `ScenarioConfig.params` field stays `None` at Phase B; the typed
  `ParamSheet` enum is a Phase C/D concern. Run uses strategy
  defaults only.
- Multi-strategy / multi-pair batch runs — that's Phase E (Compare).
- Live (paper-trading) mode — separate, gated on v2 LLM strategy.
- "Compare to previous run" beyond the most recent prior run in the
  current Lab session (no on-disk history walk, no cross-tuple
  diff — see Q4 default).
- Persistence of the in-memory `last_run_report` / `prev_run_report`
  across cockpit restarts — they live only in `LabState` (in-memory).
  Phase A's existing on-disk report read remains the cold-start
  source. Q5 covers this.
- `RunCancelHandle` storage upgrade — Phase A currently holds
  `Cockpit::lab_run_inflight: bool`, not the `RunCancelHandle` that
  ADR-0030's "drop sender → abort" design referenced. Phase B keeps
  the `bool` and adds a periodic `cancel.is_cancelled()` poll inside
  `run_scenario`'s scenario dispatch (see R7). Promoting the bool to
  an `Option<RunCancelHandle>` is a Phase B implementation detail,
  not a separate work item.

## Architecture finding — `crates/backtest` shape

> Phase A's open question (dev-note §6 Phase B bullet 1, stub Q3) was
> "confirm `crates/backtest` is library-callable; refactor the binary
> into a thin wrapper over a library entry point if not." Analyst
> findings:

**Library type surface — already shipped, no refactor needed:**

- `crates/backtest/src/lib.rs:1-15` declares `pub mod engine; pub mod
  paper;` and re-exports `run_scenario`, `BacktestKpis`, `DateRange`,
  `MatchingEngine`, `ParamSheet`, `RunError`, `RunReport`,
  `ScenarioConfig`, `PaperEngine`.
- `crates/backtest/src/engine.rs:220-240` defines
  `pub async fn run_scenario(cfg: ScenarioConfig) -> Result<RunReport,
  RunError>` per ADR-0030.
- `crates/ui/Cargo.toml` already gains `backtest = { path =
  "../backtest" }` (Phase A T-D-14 — see runner.rs:25-29 prose).
- 6 unit tests in `engine.rs:243-348` lock the seed-gate +
  range-validation invariants.

**Library body — the Phase B work item:**

- `crates/backtest/src/engine.rs:236-240` — the body is a literal
  `Err(RunError::NotImplemented)` after seed + range validation. The
  doc comment at line 11-17 says:
  > The standalone backtest binary (`crates/backtest/src/main.rs`) was
  > **not** refactored to call this function in Phase A because it
  > orchestrates many heterogeneous scenario types (SMA, Composed,
  > Momentum, Pairs, TCN) that each need their own config struct; a
  > safe refactor is a Phase B milestone.
- `crates/backtest/src/main.rs` (3417 LOC) contains:
  - `enum ScenarioStrategy { ... }` (line 104) — 7 scenarios:
    SmaCross, MacdTrend, RsiReversion, BBandsMeanRevert, Momentum,
    Pairs, TcnOverlay (with `TcnOverlayWeights` sub-mode).
  - `enum ScenarioDataSource { ... }` (line 92) — synthetic vs.
    real-data.
  - 4 distinct backtest path fns:
    - `run_momentum_backtest` (line 774, ~250 LOC)
    - `run_pairs_backtest` (line 1163, ~290 LOC)
    - `run_tcn_overlay_backtest` (line 1633, ~270 LOC)
    - `run_tcn_overlay_weights_backtest` (line 1902, ~280 LOC)
  - 3 report-writer fns: `write_momentum_report` (1026),
    `write_pairs_report` (1452), `write_tcn_overlay_report` (2184),
    plus generic `write_report` (2488).
  - `async fn main()` (line 2671, ~690 LOC) — `clap` arg parse +
    `Scenario` build + dispatch into one of the 4 backtest fns +
    report-write.
- The 22 body-SHA-256 anchors in `spec/anchors.toml` lock the
  **report body bytes** produced by these paths. Any refactor must
  preserve byte-for-byte output across all 22 scenarios.

**Phase B refactor shape (analyst recommendation, architect to ratify):**

Extract the body of `main()`'s dispatch (and the 4 `run_*_backtest`
fns + their writers) into the `engine` module behind
`run_scenario(cfg)`. `main.rs` becomes a `clap` arg-parser that
builds a `ScenarioConfig` and calls `engine::run_scenario`, with
**identical** report-write side-effects when `cfg.write_report =
true`. This is a behaviour-preserving extraction — `cargo test -p
backtest` + `verify_anchors.sh` is the guardrail.

Open architect question: should the in-memory `RunReport.equity_series`
+ `fills` + `kpis` populate from the **same code paths** that write
the report (single source of truth), or should the writer continue
to be the canonical bytes-producer and the in-memory mirror be a
parallel-computed twin? Analyst recommendation: **single source of
truth** — the writer's "format the bytes" pass becomes the last
step after the in-memory `RunReport` is populated. Lower drift
risk. Architect to confirm via § Q4 of `spec/dev-notes/...` Phase B
design pass (out of this brief).

## Requirements

Numbered, testable, derived from the dev-note §6 Phase B + Phase A's
shipped surface area + analyst architecture finding above. Each
R-item preserves the 22 locked body-SHA-256 anchors and the
cockpit-performance idle-CPU floor (≤13.1%).

### R1 — `crates/backtest::engine::run_scenario` ships a real body

- **R1.1** `crates/backtest/src/engine.rs::run_scenario` returns
  `Ok(RunReport)` for **all 7 supported scenarios** (SmaCross,
  MacdTrend, RsiReversion, BBandsMeanRevert, Momentum, Pairs,
  TcnOverlay [incl. Weights sub-mode]) when given a valid
  `ScenarioConfig`. The Phase A `Err(RunError::NotImplemented)` stub
  is replaced.
- **R1.2** `RunReport.equity_series`, `fills`, `kpis` are populated
  in-memory from the same compute pass that produces the on-disk
  report (single source of truth — see § Architecture finding).
- **R1.3** Seed gate preserved — `cfg.seed == [0u8; 32]` returns
  `RunError::ZeroSeed` (T-D-12 test at engine.rs:269 stays green).
  Range gate preserved (T-D-12 test at engine.rs:294 stays green).
- **R1.4** `cfg.write_report = true` writes the same Markdown bytes
  the standalone binary writes today for the equivalent scenario —
  the 22 body-SHA-256 anchors in `spec/anchors.toml` stay
  byte-identical (see R10).
- **R1.5** `cfg.write_report = false` skips the disk write — the
  Markdown bytes are not formatted at all (no wasted work for
  cockpit-driven runs that already populate the chart from
  `RunReport.equity_series`). Architect to confirm: the writer can
  cheaply skip when bytes are not needed, OR formatting is a no-cost
  by-product of the populate pass. Either is acceptable; the
  observable contract is "no `spec/<feature>/reports/` file
  produced when `write_report = false`."
- **Acceptance:** `cargo test -p backtest` PASS; `scripts/verify_anchors.sh
  exit 0` (22/22 anchors); `cargo run -p backtest --bin backtest
  -- --scenario btc-2023-1m-sma-cross …` produces a report file with
  the same body-SHA as locked at `anchors.toml#btc-2023-1m-sma-cross`.

### R2 — `crates/backtest/src/main.rs` becomes a thin CLI wrapper

- **R2.1** `main.rs`'s body collapses to: `clap` parse → build
  `ScenarioConfig` from CLI args → call
  `engine::run_scenario(cfg).await` → print `report_path`. The
  scenario-dispatch + report-writer fns move into `engine` (or new
  sibling modules under `crates/backtest/src/` per architect call).
- **R2.2** All 22 existing `cargo run -p backtest --bin backtest
  -- --scenario <name> --seed <n> …` invocations produce
  byte-identical report files (same path, same body bytes — only the
  `generated:` YAML wall-clock timestamp in the frontmatter differs,
  which is already excluded from the SHA per `lib.rs::report_body_hash`
  contract at lines 36-76).
- **R2.3** The `find_latest_report`, `scenario_to_feature`, and
  `report_dir_for_scenario` helpers (currently in `main.rs:3362-3417`)
  either stay in the bin (CLI-only concern) or move to the engine
  module — architect call. Either is acceptable; the contract is
  "report files land in the same `spec/<feature>/reports/` directory."
- **R2.4** `crates/backtest/Cargo.toml`'s `[[bin]]` target is
  unchanged; the binary's name, path, and feature gates (`realdata`,
  etc.) stay. No new build outputs.
- **Acceptance:** `cargo run -p backtest --bin backtest -- --scenario
  top10-2024-h1-momentum --seed 42` produces a file whose body-SHA-256
  matches `anchors.toml#top10-2024-h1-momentum`. `cargo run -p
  backtest --bin backtest --features realdata -- --scenario
  top10-2024-fy-tcn-overlay-realdata --seed 7` produces a file whose
  body-SHA matches `anchors.toml#top10-2024-fy-tcn-overlay-realdata`.

### R3 — `crates/ui/src/lab/runner.rs::spawn_lab_run` calls the real engine

- **R3.1** The `#[cfg(feature = "live")]` arm of `spawn_lab_run`
  (currently `runner.rs:180-215`) replaces the TODO simulated
  `RunSummary` with a real `rt.spawn(async move {
  backtest::engine::run_scenario(scenario_cfg).await })`. The
  `LabRunConfig` → `ScenarioConfig` mapping is a pure function in
  `runner.rs` (new helper, ~30 LOC).
- **R3.2** The mapping resolves `LabRunConfig.range_label` to a
  `backtest::engine::DateRange` variant. Phase A's range_label is a
  `SmolStr` ("Last90d", "Last30d", "H1_2024", "H2_2024", or a
  Custom-encoded form per `lab::state::DateRange` at state.rs:88-99).
  Phase B maps via a `match` against the same Preset+Custom shape
  defined in `lab::state::DateRange`. Custom-range start/end
  parsing reuses the existing
  `lab::state::DateRange::Custom::start_raw` ISO-8601 parser path
  (or extends it — architect call).
- **R3.3** The mapping passes `cfg.seed = LAB_DEFAULT_SEED` (Phase A's
  `crates/ui/src/lab/defaults.rs` constant). Phase B does **not**
  expose a seed editor (deferred to Phase D Trail or later).
- **R3.4** The mapping passes `cfg.write_report = true` so the
  cached-report path in `EquityCache` keeps working as the cold-start
  source on next cockpit launch.
- **R3.5** The `#[cfg(not(feature = "live"))]` arm continues to
  resolve with a placeholder `RunSummary` (Phase A behaviour
  preserved for fixture builds).
- **R3.6** The `RunOutcome::Err(SmolStr)` path is exercised when
  `run_scenario` returns `Err(_)` — the error's `Display` form
  surfaces as the message text (currently the runner uses `format!`
  on the tokio `JoinError`; Phase B adds the same `format!` on the
  `RunError` itself). The Run button transitions to `RunState::Failed`
  per `widgets/run_button.rs:60-70`.
- **Acceptance:** With `--features live` the cockpit boots, the
  operator presses Run on (v1.momentum, XRPUSDT, Last90d), and the
  Lab chart renders a fresh equity curve from the in-memory
  `RunReport.equity_series` within the operator-target latency budget
  (R9.2). Manual smoke: `cargo run -p ui --features live` →
  Run → chart updates.

### R4 — `LabState` gains `last_run_report` + `prev_run_report` fields

- **R4.1** `crates/ui/src/lab/state.rs::LabState` gains:
  - `pub last_run_report: Option<RunReportMirror>` — the most recent
    completed run's in-memory summary (cleared on tuple change).
  - `pub prev_run_report: Option<RunReportMirror>` — the run
    immediately preceding `last_run_report` for the same `(strategy,
    pair, range)` tuple. Set when a new run completes successfully
    AND the prior `last_run_report` was also for the same tuple. On
    tuple change, both fields clear.
- **R4.2** `RunReportMirror` is a UI-side trimmed mirror of
  `backtest::RunReport` (lives in `crates/ui/src/lab/runner.rs` next
  to `RunSummary`) carrying the fields the chart and the "compare
  delta" badge need: `equity_series: Arc<EquitySeries>`,
  `kpis: BacktestKpis` (re-exported from `backtest`), `tuple:
  LabTuple` (the `(strategy, pair, range)` triple). Excludes
  per-fill detail (the chart reads markers from the existing
  `model.chart_markers` path until the Trail screen lands).
- **R4.3** `LabState::Clone` skips `last_run_report` /
  `prev_run_report` in the existing manual Clone impl (state.rs:171-189)
  — they are session-local snapshots, not persisted. (Mirrors the
  existing `training_inflight: None` carve-out.)
- **R4.4** Persistence (`crates/ui/src/lab/persistence.rs`) **does
  not** serialize the new fields — schema `version: 1` stays.
- **R4.5** `Message::LabRunCompleted(Ok(summary))` populates
  `last_run_report` (after shifting the previous value into
  `prev_run_report` when the tuple matches; cleared otherwise).
  `LabRunCompleted(Err(_))` leaves both fields untouched (failed
  runs do not displace prior successful runs from the compare slot).
- **Acceptance:** Integration test in `crates/ui/src/lab/state.rs`
  verifies the (Run → Run-same-tuple → Run-different-tuple) state
  machine; insta snapshot `lab__compare_badge__delta` records the
  Δ-rendering shape with non-zero deltas.

### R5 — Lab chart reads equity from `last_run_report` first, `EquityCache` second

- **R5.1** `crates/ui/src/screens/lab.rs` (view fn) re-routes the
  equity-overlay data source: when
  `model.lab_state.last_run_report.is_some()` AND the tuple matches
  the current `(strategy, pair, range)` selection, the chart reads
  from `last_run_report.equity_series` directly (no
  `EquityCache::get_or_load` call). Otherwise falls back to the
  existing `EquityCache` path (Phase A behaviour).
- **R5.2** `EquityCache::invalidate(&tuple)` is **still** called on
  `Message::LabRunCompleted(Ok(_))` so a subsequent tuple-change-
  then-change-back triggers a fresh disk read consistent with the
  new on-disk report (when `write_report = true`).
- **R5.3** The R5.4 "narrowed from `<report_name>`" badge (Phase A)
  is suppressed when `last_run_report` is the source (the run was
  an exact-tuple match by construction).
- **R5.4** The comparison overlay (Phase A R8) for the up-to-4
  comparison strategies continues to read from `EquityCache`. Phase B
  does NOT run backtests for the comparison set on Run — only the
  primary strategy runs. Comparison strategies stay cached-only at
  Phase B (Q4 default; operator may revise to "run all 1+N" via
  Q-resolution).
- **Acceptance:** Manual smoke: Run on (v1.momentum, XRPUSDT,
  Last90d) — chart shows the fresh equity. Switch to (v1.momentum,
  ETHUSDT, Last90d) — chart shows the cached ETHUSDT equity (or the
  empty-state if no cached report). Switch back to XRPUSDT — chart
  shows the cached XRPUSDT equity (since the in-memory mirror
  cleared on tuple change per R4.1).

### R6 — Progress UX: reuse `ThrottledSpinner`, no progress bar at Phase B

- **R6.1** While `lab_run_inflight = true`, the Run button's
  `LAB_RUN_BUTTON_RUNNING` ("Running…") label is rendered next to a
  small `ThrottledSpinner` (10 fps, per
  `cockpit-performance-and-input-responsiveness v1.0.0`). The
  spinner is mounted via `widgets::frame::loading_with_spinner` or
  a sibling helper — architect call on placement (inside the
  button glyph vs. adjacent).
- **R6.2** No `(bars_processed / bars_total)` progress bar at
  Phase B (Q2 default — see § Operator decision questions). The
  engine emits no bar-level progress signal today, and instrumenting
  one per scenario family is a Phase D/E concern (and would touch
  the 22 anchors via tracing-span output if not carefully gated).
- **R6.3** The spinner renders only while `lab_run_inflight = true`.
  On `LabRunCompleted` (Ok or Err) the spinner unmounts and the
  button label transitions to `LAB_RUN_BUTTON_COMPLETED` ("Re-run")
  or `LAB_RUN_BUTTON_FAILED` ("Retry") per
  `widgets/run_button.rs::RunState::from_cockpit`.
- **R6.4** The 10 fps cadence keeps the cockpit's idle-CPU floor
  (≤13.1%) intact during a run — only one spinner instance is
  visible, the chart's redraws are event-driven (no continuous
  RequestRedraw beyond the spinner cadence), and the in-flight
  backtest computes on the side-thread runtime.
- **Acceptance:** Manual smoke under `cargo run -p ui --features
  live` confirms the spinner spins at the expected cadence while
  the run is in flight; cockpit idle-CPU readback after the run
  completes stays ≤13.1% (R10.4).

### R7 — Cancellation: poll `RunCancelReceiver` at scenario boundaries

- **R7.1** `engine::run_scenario` checks `cancel.is_cancelled()` at
  scenario-internal **bar boundaries** (the existing per-bar loops
  inside `run_momentum_backtest` / `run_pairs_backtest` /
  `run_tcn_overlay_backtest` /
  `run_tcn_overlay_weights_backtest`). On cancellation, the fn
  returns `Err(RunError::Internal("cancelled".into()))` (or a new
  `RunError::Cancelled` variant — architect call).
- **R7.2** The poll happens **at most once per N bars** where N is
  sized so the overhead is invisible (analyst recommendation: N=128;
  architect to tune). The poll does not touch a wall clock — it
  reads the existing `RunCancelReceiver::is_cancelled` non-blocking
  mpsc try-recv.
- **R7.3** Pressing Run while a run is in flight: the Run button is
  **disabled** per Phase A's `is_disabled = run_handle_present ||
  Running` gate (`run_button.rs:97`). Phase B does **not** introduce
  a Run/Cancel toggle (Q3 default — operator may revise). The
  trainer pattern in `lab::trainer` (SIGKILL via subprocess Drop)
  does NOT apply — the backtest runs in-process; there is no
  subprocess to kill.
- **R7.4** The `Cockpit::lab_run_inflight: bool` field stays a
  `bool` for Phase B. The implicit cancel mechanism (drop the
  `RunCancelHandle` on next press) was the ADR-0030 design but
  Phase A shipped just the bool. Architect may upgrade to
  `Option<RunCancelHandle>` storage in Phase B if R7.1's poll
  needs an owned handle to wire; otherwise the cancel-on-next-press
  semantics defer to Phase C (when a Cancel button or
  re-press semantic actually lands).
- **R7.5** If cancellation is not wired in Phase B (operator-decide
  Q3 option C), this requirement is replaced by "the in-process
  backtest is synchronous-cancellable only by tearing the cockpit
  down" — acceptable for a Phase B MVP per § Q3 default below.
- **Acceptance:** A unit test in `crates/backtest/src/engine.rs`
  exercises a cancellation path (constructs a `RunCancelReceiver`
  in the cancelled state, calls `run_scenario` with a short scenario,
  asserts `Err(RunError::Cancelled)` returns within K bars). Manual
  smoke confirms the cockpit does not hang when a run is launched
  and the cockpit window is closed mid-run.

### R8 — Compare-to-previous-run delta badge

- **R8.1** New widget `crates/ui/src/widgets/run_delta_badge.rs` —
  renders three rows of Δ values when both
  `lab_state.last_run_report` and `lab_state.prev_run_report` are
  `Some` AND share the same `(strategy, pair, range)` tuple:
  - **Δ P&L** — `(last.final_equity - last.initial_equity) -
    (prev.final_equity - prev.initial_equity)`. Color: green if
    positive, red if negative, fg-3 if zero (Lumen UP_500 / DOWN_500
    / FG_3 tokens — no new tokens).
  - **Δ MaxDD** — `last.max_drawdown - prev.max_drawdown`. Color
    inverted (smaller drawdown = green).
  - **Δ Sharpe** — `last_sharpe - prev_sharpe`. Sharpe is computed
    on the cockpit side from the in-memory equity series via a
    helper reused from `crates/backtest/src/main.rs::compute_sharpe`
    (line 2454); architect can either re-export it from the
    `backtest` crate or duplicate the ~30 LOC into `lab::runner`.
    Analyst recommendation: re-export.
- **R8.2** The badge renders adjacent to the Run button row in the
  Lab top-bar. Hidden when there is no `prev_run_report` OR the
  tuples differ. No tooltip, no chart-overlay diff (out of scope).
- **R8.3** Zero new Lumen tokens — UP_500, DOWN_500, FG_3, PANEL,
  BORDER_1 already exist.
- **R8.4** Reset semantics: tuple change clears both
  `last_run_report` and `prev_run_report` (per R4.1), so the badge
  hides; subsequent Run on the new tuple sets only
  `last_run_report`, badge still hidden until a second Run on the
  same new tuple lands.
- **R8.5** No persistence — the badge is session-local. Restart
  the cockpit, the badge is gone (the on-disk report for the last
  tuple is still readable via `EquityCache` but the "previous run"
  context is lost intentionally per Q4 default).
- **Acceptance:** insta snapshot `run_delta_badge__pnl_up_dd_down`
  records the three-row shape with non-trivial deltas; unit test
  in `widgets/run_delta_badge.rs` covers all 8 sign combinations
  (Δ P&L sign × Δ MaxDD sign × Δ Sharpe sign).

### R9 — Per-run KPI strip update

- **R9.1** The existing KPI strip above the chart (Phase A `widgets/
  kpi_strip.rs`) updates from `last_run_report.kpis` when present,
  falling back to the cached-report KPIs otherwise. This is a
  pure-routing change — no new widget.
- **R9.2** Target run latency budget (analyst hypothesis H1 — to
  validate): a v1.momentum × XRPUSDT × Last90d backtest completes
  in **≤3000 ms** on the operator's 3360×1890 machine (cold cache,
  no `realdata` feature). A v2.5 TCN-overlay × top10 × FY-2024
  scenario may exceed 3000 ms (the realdata variants are heavier);
  acceptable as long as the run completes and the spinner keeps
  spinning (no UI freeze). Architect to tighten the budget; tester
  to measure during M-FINAL.
- **R9.3** No new instrumentation surfaces — Phase A's
  `tracing::info!("lab_run_completed", …)` span (or equivalent) is
  the source-of-truth latency signal. If the span doesn't exist yet,
  add it as part of M-T1 (architect-level decision).
- **Acceptance:** Tester report cites measured latency for the
  v1.momentum × XRPUSDT × Last90d scenario; idle-CPU readback at
  T+5s post-run-completion is ≤13.1%.

### R10 — Verification gates (non-regression contract)

- **R10.1** All **22 locked body-SHA-256 anchors** in
  [`spec/anchors.toml`](../anchors.toml) stay byte-identical (15
  originals + 4 `-realdata` + 3 from `v25-tcn-alpha-investigation`).
  `scripts/verify_anchors.sh` exit 0 is non-negotiable. The
  refactor of `main.rs` into a wrapper over `engine::run_scenario`
  is **behaviour-preserving by construction** — same seed → same
  RNG draws → same bars → same fills → same equity series → same
  Markdown bytes. Any drift is a developer bug, not a spec
  ambiguity.
- **R10.2** **Phase A non-regression contract preserved:**
  - Lab tuple persistence (`crates/ui/src/lab/persistence.rs`)
    unchanged at the JSON schema level (`version: 1`).
  - Chart + chip widgets (`widgets/pair_chip.rs`, `strategy_chip.rs`,
    `date_range.rs`, `chart.rs` overlays) unchanged.
  - Equity-curve overlay reading via `EquityCache` falls through
    when `last_run_report` is `None` or tuple-mismatched (R5.1).
  - Multi-strategy comparison overlay (Phase A R8) unchanged.
- **R10.3** **`cockpit-smoke`** skill exit 0 required for tester
  `VERDICT → PASS` per AGENT.md §Process discipline rule 6.
- **R10.4** **`cockpit-performance-and-input-responsiveness v1.0.0`
  idle-CPU floor (≤13.1%)** stays under budget. An in-flight
  backtest may spike CPU during the run (the side-thread tokio
  runtime saturates as expected); the contract is **idle return-to-
  baseline within 5 s of `LabRunCompleted`** — no leaked spinners,
  no leaked redraw subscriptions, no leaked `RunCancelReceiver`
  pinging in a loop.
- **R10.5** `spec-lint` exit 0 required per AGENT.md §Process
  discipline rule 7 (no dead links, no orphan-feature, no
  trace-broken-path). The new feature.md + tasks.md must pass.
- **R10.6** No new external dependencies in `crates/backtest/
  Cargo.toml` or `crates/ui/Cargo.toml`. Phase B is wiring +
  extraction; the workspace dep graph stays the same shape.
- **R10.7** No new Lumen tokens. The compare-delta badge (R8)
  uses existing UP_500 / DOWN_500 / FG_3 / PANEL / BORDER_1.
- **Acceptance:** tester report cites all seven gates passing,
  cites verified anchor count (22/22 PASS), cites cockpit-smoke
  exit 0, cites idle-CPU readback ≤13.1%, cites cockpit-smoke
  PASS 0 panics.

## Hypothesis register

Falsifiable load-bearing claims that the architect / developer / tester
should empirically verify before promoting Phase B to `shipped`.

### H1 — Latency budget for the smallest scenario

> **If** the operator presses Run on (v1.momentum, XRPUSDT, Last90d)
> on the 3360×1890 machine with the cold `EquityCache` and **no**
> `realdata` feature, **then** `Message::LabRunCompleted(Ok(_))`
> arrives at the iced update loop within **3000 ms** of the
> `Message::LabRunRequested` dispatch.

**How to falsify:** add a `tracing::info!(target = "lab.run.latency",
elapsed_ms = …)` span pair around the `iced::Task::perform` future
in `lab::runner::spawn_lab_run`; measure 10 runs; report median +
p95.

**Why it matters:** if median > 3000 ms, the spinner-only UX
(R6) is insufficient and Q2 reopens (progress bar may be required).
If p95 > 10s, the Phase B in-process design loses to the Phase A
cached-report shortcut for casual use; the architect should
reconsider whether Phase B is operator-shippable without a
progress signal.

### H2 — Anchor preservation under refactor

> **If** the `main.rs` body is refactored into thin wrapper +
> `engine::run_scenario` body extracted from the 4 existing
> `run_*_backtest` fns **without** changing the seed handling,
> the RNG construction, the bar-iteration order, or the fill /
> equity / KPI compute, **then** all 22 body-SHA-256 anchors in
> `spec/anchors.toml` stay byte-identical.

**How to falsify:** run `scripts/verify_anchors.sh` against
HEAD after the refactor; any non-zero exit count is a falsification.

**Why it matters:** this is the single largest risk in Phase B.
The refactor is mechanically straightforward but error-prone
(3417 LOC moves around). H2 falsification means the developer
silently broke determinism — block ship until fixed.

### H3 — In-memory vs. cached-disk path produces equal equity

> **If** `engine::run_scenario` is called with
> `(strategy=v1.momentum, pair=XRPUSDT, range=Last90d, seed=LAB_DEFAULT_SEED,
> write_report=true)` **and then** `EquityCache::get_or_load(tuple)` is
> called immediately for the same tuple, **then** the in-memory
> `RunReport.equity_series` and the cache-loaded `EquitySeries`
> contain the same timestamps and the same equity values up to
> `rust_decimal::Decimal` exactness.

**How to falsify:** integration test in `crates/ui/tests/lab_run_engine.rs`
(new) drives a Run and asserts series equality. Difference flagged
as a determinism bug.

**Why it matters:** if H3 falsifies, the chart shows different equity
depending on whether the operator just-Ran the backtest or just
launched the cockpit. That's a confusing observable; operator
trust erodes. H3 is the determinism contract between the writer and
the in-memory mirror (R1.2).

### H4 — TCN-overlay scenarios stay shippable

> **If** the Phase B refactor preserves the
> `run_tcn_overlay_backtest` / `run_tcn_overlay_weights_backtest`
> code paths, **then** the 7 TCN-related anchors
> (`top10-2023-fy-tcn-overlay`, `top10-2024-fy-tcn-overlay`,
> `…-weights`, `…-realdata`, `…-weights-realdata`, plus the 3
> alpha-investigation reports `forecast-distribution-bs1-realdata`,
> `forecast-distribution-bs2-realdata`,
> `sharpe-comparison-realdata`) stay byte-identical.

**How to falsify:** run `scripts/verify_anchors.sh` against TCN
scenarios specifically; any mismatch falsifies.

**Why it matters:** the TCN paths are the most recently shipped
(v25-tcn-alpha-investigation, committed 2026-05-18) and the
heaviest computationally — a refactor regression here is the
highest-impact silent breakage.

### H5 — Spinner does NOT regress idle-CPU floor

> **If** the Lab Run button is pressed, the run completes
> successfully, and the cockpit returns to idle (no operator
> interaction) for ≥5 s, **then** the cockpit idle-CPU reading
> stays at ≤13.1% (the
> `cockpit-performance-and-input-responsiveness v1.0.0` floor).

**How to falsify:** repeat the post-fix measurement protocol from
`spec/cockpit-performance-and-input-responsiveness/reports/cpu-
measurement-postfix-2026-05-15T13-02Z.log`; idle-CPU readback at
T+5s post-`LabRunCompleted`. Floor is the upper bound; any sustained
reading >13.1% falsifies.

**Why it matters:** if H5 falsifies, Phase B introduced a leaked
subscription or a stale spinner mount. The fix is required before
ship — the perf budget is non-negotiable per R10.4.

## K — Risk register

### K1 — Refactor drift breaks one or more anchors

`crates/backtest/src/main.rs` is 3417 LOC with 4 distinct
backtest paths + 3 report writers. The mechanical extraction
into `engine::run_scenario` is the dominant Phase B risk.

- **Likelihood:** medium-high. The fns share helpers (synthetic
  bars, top10 symbols, Sharpe compute, write_report); ordering /
  call-site changes can cascade.
- **Impact:** critical. R10.1 is a hard gate; ship is blocked.
- **Mitigation:** **anchor verification at every commit during
  Phase B development.** Developer should run `scripts/verify_anchors.sh`
  after each scenario extraction (commit-per-scenario discipline),
  not just at M-FINAL. Architect should sequence the extraction
  scenario-by-scenario, smallest first (SmaCross → MacdTrend → …
  → Momentum → Pairs → TCN). Each extraction gate is a separate
  task in M-T1.
- **Watch recipe** (per MEMORY.md `feedback_watch_recipe_for_long_running`):
  ```bash
  watch -n 10 'cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading && ./scripts/verify_anchors.sh 2>&1 | tail -20'
  ```

### K2 — TCN-overlay weights scenario fails to extract cleanly

The TCN paths share a 1500-LOC arc with `run_tcn_overlay_backtest`
+ `run_tcn_overlay_weights_backtest`. They depend on ONNX model
loading (via `tract` per ADR-0027), which is currently constructed
inside `main.rs`'s dispatch.

- **Likelihood:** medium. The model-load path is the trickiest
  surface (file I/O + lazy-static model handles + scratch
  tensor buffers).
- **Impact:** high. 7 TCN anchors at risk (H4).
- **Mitigation:** architect identifies the model-load extraction
  shape in M-T1 design pass; developer extracts TCN scenarios
  **last** (after the 4 simpler scenarios are anchor-verified).
  A `--scenario top10-2024-fy-tcn-overlay-realdata --seed 7`
  smoke run is the gate between TCN extraction and ship.

### K3 — Cockpit hangs on backtest if cancellation not wired

If R7 is deferred (Q3 option C) and the developer wires
`run_scenario` without a cancel poll, killing the cockpit window
mid-run leaks the side-thread tokio task. Tokio's runtime cleanup
is supposed to abort spawned tasks on shutdown, but a long-running
backtest that's CPU-bound may not yield to the abort signal.

- **Likelihood:** low-medium (depends on whether ADR-0030's
  `cancel.is_cancelled()` poll lands or not).
- **Impact:** medium. Cockpit-smoke would catch a real hang (the
  smoke test launches and tears down); operator UX impact is "the
  backtest finishes its bars before the process exits" — annoying
  but not destructive.
- **Mitigation:** R7.1 wires the cancel poll. Even if Q3 says
  "no Cancel button at Phase B" (default), the **internal**
  cancellation plumbing should still be present so cockpit
  shutdown doesn't hang. Architect to ratify.

### K4 — `RunReport.equity_series` allocates a large `Vec<(Timestamp, Money)>`

A v2.5 TCN-overlay × FY-2024 backtest has ~525,600 minute-bars
(1 year × 365 days × 1440 min). Allocating that as an in-memory
`Vec<(Timestamp, Money<Usdt>)>` is 525_600 × (8 + 16) = ~12.6 MB
in best case (per-pair). For the top10 universe across the
year, this scales to ~126 MB just for equity-series mirrors of
fills. Memory pressure on the cockpit process.

- **Likelihood:** low at Phase B (only one run is in flight; the
  comparison overlay still reads from disk).
- **Impact:** medium. The cockpit's RSS budget isn't currently
  pinned, but 126 MB is a real footprint on the operator's
  machine.
- **Mitigation:** `RunReport.equity_series` is `Vec<(Timestamp,
  Money<Usdt>)>` per ADR-0030's signature (engine.rs:160). For
  Phase B, this is the architect-ratified shape. If memory becomes
  a problem, Phase D can introduce a downsampled
  `EquitySeriesCompact` (one point per N bars) for the chart-only
  path while the writer retains full fidelity. Not a Phase B
  concern.

### K5 — Operator-decide Q3 (no Cancel button) confuses a long-running run

Phase B default per Q3 is "Run button is disabled while a run
is in flight; no Cancel button until Phase C." A v2.5 TCN-overlay
× FY-2024 realdata run may take >30s. The operator sees "Running…"
forever with no way to abort except quitting the cockpit.

- **Likelihood:** medium (operator may test a heavy scenario early).
- **Impact:** low-medium. Annoying but recoverable (quit + relaunch
  costs ~5s).
- **Mitigation:** R6 spinner with `LAB_RUN_BUTTON_RUNNING` keeps
  the operator aware. R6.2 explicitly defers progress UX. The
  operator-decide Q3 alternative (B: Run/Cancel toggle) is the
  remediation if the default fails operator approval at M-FINAL.

### K6 — `LabRunCompleted(Err(...))` displays an obtuse error

`RunError::Internal(String)` / `RunError::UnknownStrategy(String)`
etc. surface as raw `format!("{e}")` text in the `RunOutcome::Err`
SmolStr. Operator-facing error copy may be unhelpful ("internal
backtest error: failed to load tract model: ...").

- **Likelihood:** medium (errors happen).
- **Impact:** low. Failed-run path is rare; the Retry copy
  (`LAB_RUN_BUTTON_FAILED = "Retry"`) covers the recovery action.
- **Mitigation:** Phase B keeps the raw `Display` form. Phase D
  Trail screen surfaces the run's `tracing` spans for operator
  debugging. Not a Phase B work item.

### K7 — Phase B touches `crates/backtest`, raising blast radius

Phase A's anchor risk was zero because the slice was UI-only.
Phase B touches `crates/backtest` directly. Any subtle bug in the
extraction can affect every backtest the operator runs from the
CLI, not just from the cockpit.

- **Likelihood:** the touch is in scope; the bug-risk is the K1
  refactor-drift concern.
- **Impact:** wider blast radius than Phase A — CLI users hit it
  too.
- **Mitigation:** the same R10.1 anchor gate covers CLI invocations
  (R2.2 asserts CLI byte-identicality). Tester should explicitly
  run **both** `cargo run -p backtest --bin backtest -- --scenario
  …` AND `cargo run -p ui --features live` (Run button) for at
  least one scenario per family (4 families: momentum, pairs,
  tcn-overlay, tcn-overlay-weights).

### K8 — `compute_sharpe` re-export creates a new public surface

R8.1 says "re-export `compute_sharpe` from `crates/backtest`." This
expands the `backtest` crate's public API for the cockpit's
benefit, which the `ui` crate then depends on. Future shape changes
to `compute_sharpe` would now require a cockpit-side migration.

- **Likelihood:** low.
- **Impact:** low. The fn signature is stable
  (`fn compute_sharpe(equity_curve: &[Decimal]) -> f64`) — the
  shape is unlikely to need to change.
- **Mitigation:** alternative is to duplicate the 30 LOC into
  `lab::runner` (R8.1 acknowledges this). Architect call. Either
  choice is sound; mark it for the architect.

## Operator decision questions

> **OPERATOR DECIDED 2026-05-19 via "Autoapprove all" directive — all
> 5 Qs resolved to the analyst-recommended defaults.** Specifically:
> Q1 = A (in-memory return + opt-in disk write per ADR-0030);
> Q2 = A (ThrottledSpinner only; no progress bar);
> Q3 = A (button disabled-while-running + internal cancel poll; no Cancel button);
> Q4 = A (session-local in-memory `last_run_report` / `prev_run_report` only);
> Q5 = A (bytes-identical refactor — preserve all 22 anchors; reject v2 anchor refresh).
> Architect proceeds against these defaults.

> **Original framing (analyst-pass preamble — retained for context).**
> Each Q ships with an analyst-recommended **default**; if the operator
> does not override within 24 h, the architect proceeds with the default.
> The defaults are chosen to maximize "Phase B ships before Phase C"
> velocity and minimize anchor risk.

### Q1 — Library-call shape: in-memory return vs. disk-and-reread

**Question.** Does `crates/backtest::engine::run_scenario` return the
in-memory `RunReport` to the cockpit AND optionally write the
Markdown report to disk (the ADR-0030 shape), OR write to disk
and have the cockpit re-read via `EquityCache`?

**Analyst recommendation (DEFAULT): A — in-memory return + opt-in
disk write.**

- ADR-0030's signature (`run_scenario(cfg) -> Result<RunReport,
  RunError>`) already locks this shape.
- The in-memory return saves the cockpit a disk round-trip on every
  Run press (~12 ms cold cache per Phase A measurement); cumulative
  win over a day of operator use is real.
- `write_report = true` is opt-in; the cockpit passes `true` so the
  `EquityCache` cold-start path still works on next launch. Disk is
  the durable audit trail; memory is the hot path.
- **Audit / determinism contract:** the disk-written bytes are the
  canonical artifact (same as today); the in-memory mirror is a
  fast-path twin (H3 guards equivalence).

**Alternatives:**

- **B: disk-only, cockpit re-reads.** Simpler in one axis (single
  read path) but pays the disk-roundtrip cost on every press. No
  operator benefit unless the in-memory mirror is unbounded-memory
  (K4); the K4 mitigation already addresses that risk.

**Architect impact if changed:** swapping to B would require
removing `RunReport.equity_series` / `fills` / `kpis` from the
return type (or making them `Option`-empty) and routing the
chart through `EquityCache::invalidate_then_get_or_load` on
`LabRunCompleted`. Probably ~4 hours of refactor.

### Q2 — Progress UX: spinner only, or progress bar?

**Question.** While the engine runs, does the cockpit render:
(A) the shipped `ThrottledSpinner` only, or
(B) a `(bars_processed / bars_total)` progress bar?

**Analyst recommendation (DEFAULT): A — `ThrottledSpinner` only.**

- `ThrottledSpinner` is shipped (`widgets/throttled_spinner.rs`),
  10 fps, preserves the cockpit-perf idle floor.
- The engine has **no bar-level progress signal** today. Adding
  one means injecting a callback / channel into the 4
  `run_*_backtest` paths, which (a) expands the scope of the
  refactor (more lines moved, higher K1 risk) and (b) potentially
  touches the report-write code (anchor risk).
- Phase B is a "make the wiring work" milestone; cosmetic
  progress UX is a Phase C/D refinement. The operator review
  checkpoint at the end of Phase B can re-open Q2 if the
  spinner-only UX is unacceptable.

**Alternatives:**

- **B: progress bar with `bars_processed / bars_total`.** Requires
  a new `mpsc::Sender<RunProgress>` plumbed through
  `run_scenario` into the 4 backtest paths; one extra `Send` on
  each bar; mid-run partial state. Anchor-safe (no report bytes
  change) but real implementation cost — estimate +2 days of
  developer time.

**Architect impact if changed:** B is additive; the channel can
be wired without touching the report writer. K1 risk increases
moderately.

### Q3 — Cancellation: bool inflight gate only, or Run/Cancel toggle?

**Question.** While a run is in flight, is the Run button:
(A) disabled until completion (Phase A's shipped behaviour), no
Cancel surface;
(B) a Run/Cancel toggle that switches label + dispatches a
cancel message;
(C) no cancel wiring at all — drop the `RunCancelReceiver` plumbing
and let the in-process backtest run to completion or process exit?

**Analyst recommendation (DEFAULT): A — disabled-while-running +
internal cancel poll for cockpit-shutdown safety.**

- Phase A already disables the button while in flight
  (`run_button.rs:97` — `is_disabled = run_handle_present ||
  Running`). Zero new widget work for Phase B.
- The **internal** cancel poll (R7.1) should land regardless of
  the UI surface — it prevents cockpit-shutdown hangs (K3).
  Operator never sees a Cancel button at Phase B; the
  `RunCancelReceiver` exists in the runner for safety only.
- B (Run/Cancel toggle) is a Phase C/D refinement once the
  operator validates that cancellation is actually wanted (vs.
  "just wait it out — runs are <3s anyway" — H1).
- C (no cancel wiring) is **rejected** by analyst — even if the
  operator never presses Cancel, cockpit-shutdown safety needs
  the poll. K3 mitigation is non-negotiable.

**Alternatives:**

- **B: Run/Cancel toggle.** Run button label flips to "Cancel" while
  in flight; click dispatches `Message::LabRunCancelRequested`
  which drops the `RunCancelHandle`. UI work: ~1 day. Adds 1
  string + 1 message variant + 1 `RunState::Running` branch arm
  in the button view.
- **C: no cancel wiring.** Skip R7.1 + R7.5. Lowest LOC. Risks K3.

**Architect impact if changed:** B requires the `Cockpit::lab_run_inflight`
field to promote from `bool` to `Option<RunCancelHandle>` so the
drop actually fires. Trainer (`lab::trainer::TrainingHandle` in
`LabState::training_inflight`) is the precedent — same pattern.

### Q4 — "Compare to previous run": in-memory only, on-disk history, or both?

**Question.** The "compare to previous run" affordance diffs against:
(A) the last in-memory result for the **same tuple** in the current
Lab session (session-local, lost on restart);
(B) the most recent on-disk report matching the tuple (durable,
walks `spec/<strategy>/reports/` for the second-most-recent file);
(C) both — A by default, with an operator toggle to swap to B?

**Analyst recommendation (DEFAULT): A — session-local in-memory only.**

- Simplest. Zero new disk I/O. Zero report-walker code.
- Matches the operator's natural workflow: "I just ran v1.momentum,
  let me tweak — oh wait, there's no tweaking at Phase B
  (params=None), let me just re-run and see if my fix
  changed anything." The session-local diff is the answer.
- The "tweak a param" workflow that B serves is **not yet
  possible** at Phase B (no inline param sheet — out of scope
  per § Out of scope). When the param sheet lands (Phase C/D),
  reopening Q4 to add disk-history is cheap.
- B requires:
  - A second-most-recent-report finder
    (`find_second_latest_report(dir, scenario)`) — ~30 LOC.
  - A second `parse_report_to_kpis` pass — re-uses the existing
    Phase A equity-loader Markdown parser (~10 LOC of glue).
  - Disk I/O at button-press time (small — <50 KB reports).
  - Mental-model complexity: "which previous run am I comparing
    to?" The operator may have multiple historical reports
    (e.g. backtest-20260512-…, backtest-20260518-…, etc.)
    sorted by mtime; B picks "the second-most-recent." If the
    operator's intent was "compare to the version I ran an hour
    ago," B mostly does the right thing — but it's not
    self-evident.
- C (toggle) is the most powerful but adds widget complexity
  (a small radio / segmented control) for a single Phase B
  feature. Cost > value at Phase B.

**Alternatives:**

- **B: on-disk history.** ~1 day developer time, durable across
  restarts.
- **C: both with toggle.** ~1.5 days, plus the toggle widget.

**Architect impact if changed:** B + the disk-history walker would
live in `lab::equity_loader` (new fn `load_kpis_for_second_latest_tuple`),
keeping the touched surface inside the existing module.

### Q5 — Anchor refresh: bytes-identical CLI + cockpit, or accept new v2 anchor set?

**Question.** When the Phase B refactor lands, does
`engine::run_scenario(cfg)` produce **byte-identical** Markdown
report bytes vs. the pre-Phase-B `main.rs` dispatch (R10.1 anchor
contract), OR does Phase B ship a **v2 anchor refresh** that
locks new bytes from the refactored path?

**Analyst recommendation (DEFAULT): A — bytes-identical, preserve
all 22 anchors.**

- The 22 anchors lock 8 month's worth of `cargo run -p backtest`
  outputs that the operator has used for spec-driven
  validation. Resetting them is a one-way trip — the historical
  reports stay in the repo but the determinism contract
  breaks.
- The refactor is mechanically a **move**, not a rewrite. Same
  seeds, same RNG draws, same fill computation, same writer
  format. No new compute. Byte-identical output is achievable
  by construction — H2 is the gate.
- If H2 falsifies during M-T1 / M-FINAL, **that's a Phase B
  bug, not a v2 anchor refresh.** The developer fixes the
  drift; tester re-runs `verify_anchors.sh`.

**Alternatives:**

- **B: v2 anchor refresh.** Acknowledge the refactor's bytes
  may differ in trivial ways (e.g. column ordering, whitespace
  in a Markdown table); regenerate all 22 anchors with the
  refactored binary; lock the new bytes. **Rejected** by
  analyst — the operator's audit trail depends on byte-stability,
  and Phase B explicitly says "behaviour-preserving extraction"
  (R10.1 commentary).

**Architect impact if changed:** B reduces M-FINAL gate friction
(anchor mismatches don't block) but breaks the determinism
contract with the operator's historical work product. Operator
must explicitly ratify B for it to land.

## Non-regression contract

This contract is the M-FINAL acceptance gate. The tester confirms each
item in the test-final report under `## Non-regression contract`.

1. **22 body-SHA-256 anchors stay byte-identical** —
   `scripts/verify_anchors.sh` exit 0 against the full anchor set in
   [`spec/anchors.toml`](../anchors.toml). 15 originals + 4
   `-realdata` + 3 from v25-tcn-alpha-investigation = 22 total.
2. **Phase A surface area unchanged** — Lab tuple persistence (R6
   from Phase A), chart + chip widgets, equity-curve overlay
   (cached-report path), comparison overlay (≤4 strategies). The
   `EquityCache` invalidation flow on `LabRunCompleted` still fires.
   `lab_state.json` schema is still `version: 1`.
3. **`cockpit-smoke`** skill exit 0 — 0 panics, all snapshots green.
4. **`cockpit-performance-and-input-responsiveness v1.0.0`
   idle-CPU floor (≤13.1%)** preserved — measured at T+5s
   post-`LabRunCompleted` on a v1.momentum × XRPUSDT × Last90d
   smoke run. A spike during the run is acceptable; idle return-to-
   baseline is mandatory.
5. **`spec-lint` exit 0** — no dead links, no orphan-feature, no
   trace-broken-path.
6. **No new external crate deps** — `crates/backtest/Cargo.toml`
   + `crates/ui/Cargo.toml` deps unchanged (modulo internal path
   dep edges that already exist).
7. **No new Lumen tokens.**

## Trace

Trace row [`REQ-UI-RETHINK-PHASE-B-001`](../trace.toml) — analyst
pass refines `tests` paths to include the new test files:

- `crates/backtest/src/engine.rs` (existing — extended with
  per-scenario dispatch tests + cancellation test).
- `crates/backtest/src/main.rs` (existing — thin-wrapper coverage).
- `crates/ui/src/lab/runner.rs` (existing — wire-up test).
- `crates/ui/src/lab/state.rs` (existing — `last_run_report` /
  `prev_run_report` state-machine test).
- `crates/ui/src/widgets/run_delta_badge.rs` (NEW — R8 widget
  tests + insta snapshot).
- `crates/ui/tests/lab_run_engine.rs` (NEW — integration test
  per H3).
- `crates/backtest/tests/determinism.rs` (existing — anchor
  verification path).

Architect updates `arch` rows + `anchors` rows at M-T1 / M-FINAL.

## Milestones

> Decomposed into TBD `T-D-N*` tasks at architect T-AR-2; this
> section enumerates the milestone acceptance gates.

### M0 — Analyst synthesis (this brief)

- [x] Confirm `crates/backtest` shape — library-callable per
  `lib.rs`; `engine::run_scenario` exists as a stub; **scope
  is body extraction + main.rs refactor, not API extraction.**
  See § Architecture finding.
- [x] Survey existing Lab `Run` button code path —
  `runner.rs::spawn_lab_run` precedent + `widgets/run_button.rs` +
  `Message::LabRunRequested/Completed` arm wiring (R3 in this
  brief).
- [x] Compare to `lab::trainer` cancellation pattern — different
  (subprocess SIGKILL vs in-process bool); Phase B mirrors the
  Phase A mpsc-disconnect shape, NOT trainer (R7.3).
- [x] Surface Q1-Q5 to operator with analyst-recommended defaults.
- [x] Lock R1-R10 requirements + K1-K8 risk register + H1-H5
  hypotheses.
- [x] Refine `tests` in the trace.toml Phase B row (see § Trace).

**Acceptance:** this feature.md status → `draft`, version `0.1.0`,
operator-readable, all Qs have defaults.

### M-T1 — Architect decomposition (next)

- [ ] Architect ratifies / overrides Q1-Q5 defaults; commits the
  resolutions inline in feature.md (under each Q heading).
- [ ] Architect publishes `tasks.md` T-D-N* decomposition: scenario-
  by-scenario extraction order (K1 mitigation), runner wiring
  task, `LabState` extension task, delta-badge widget task,
  integration-test task.
- [ ] Architect updates `spec/trace.toml` `arch` row for Phase B
  with any new ADR / design dev-note links.
- [ ] Architect runs `rust-validate` + `rust-build` on a baseline
  commit to confirm the Phase A surface is green pre-extraction.

**Acceptance:** `tasks.md` carries 8-12 T-D-N* checkboxes with
crate paths + R-anchors; architect handoff line emitted.

### M-T2..M-T(N-1) — Developer extraction waves

- [ ] Extract simple-scenario paths first (SmaCross, MacdTrend,
  RsiReversion, BBandsMeanRevert) — one commit per scenario,
  each gated by `scripts/verify_anchors.sh` exit 0 (K1 mitigation).
- [ ] Extract Momentum scenario — anchor gate for
  `top10-2023-1h-momentum`, `top10-2024-h1-momentum`.
- [ ] Extract Pairs scenario — anchor gate for
  `pairs-2023-zscore-mr`, `pairs-2024-h1-zscore-mr`.
- [ ] Extract TCN-overlay scenarios (last per K2 mitigation) —
  anchor gate for all 7 TCN-related anchors including the
  `-realdata` and alpha-investigation reports.
- [ ] Wire `spawn_lab_run` to real `run_scenario` (R3) — manual
  cockpit smoke under `--features live`.
- [ ] Add `LabState.last_run_report` / `prev_run_report` fields +
  population logic (R4).
- [ ] Wire chart equity overlay routing through `last_run_report`
  (R5).
- [ ] Land `widgets/run_delta_badge.rs` + tests + snapshot (R8).
- [ ] Implement `engine::run_scenario` cancellation poll (R7.1).

**Acceptance per wave:** `cargo test --workspace` green +
`scripts/verify_anchors.sh` exit 0 + (after R3 lands) manual
cockpit-smoke confirms Run produces a fresh chart.

### M-FINAL — Tester sweep

- [ ] Run `rust-validate` + `cargo test --workspace`.
- [ ] Verify the 22 body-SHA-256 anchors stay byte-identical
  (R10.1) — `scripts/verify_anchors.sh` exit 0.
- [ ] Run `cockpit-smoke` (PASS 0 panics).
- [ ] Verify `cockpit-performance-and-input-responsiveness v1.0.0`
  idle-CPU floor stays ≤13.1% after `LabRunCompleted` (R10.4 /
  H5).
- [ ] Measure H1 latency budget — median + p95 for v1.momentum ×
  XRPUSDT × Last90d.
- [ ] Run CLI smoke for one scenario per family (K7 mitigation):
  - `cargo run -p backtest --bin backtest -- --scenario
    btc-2023-1m-sma-cross --seed <anchor-seed>` →
    anchor-byte-identical.
  - `cargo run -p backtest --bin backtest -- --scenario
    top10-2024-h1-momentum --seed <anchor-seed>` → identical.
  - `cargo run -p backtest --bin backtest -- --scenario
    pairs-2024-h1-zscore-mr --seed <anchor-seed>` → identical.
  - `cargo run -p backtest --bin backtest --features realdata --
    --scenario top10-2024-fy-tcn-overlay-realdata --seed
    <anchor-seed>` → identical.
- [ ] Verify H2-H5 hypotheses (anchor preservation, in-memory ==
  cached, TCN anchors green, idle-CPU floor preserved).
- [ ] Author
  `spec/ui-rethink-phase-b-lab-run/reports/test-final-<YYYY-MM-DD>.md`
  per the test-report template at
  `.claude/skills/rust-test/templates/test-report.md`.

**Acceptance:** tester VERDICT → PASS; all 7 non-regression
contract items confirmed.

## Backtest Scenarios

**N/A** — this is a UI + backend refactor feature. No new
backtest scenarios are defined; Phase B is wiring + behaviour-
preserving extraction over the existing 22 anchored scenarios.

## Changelog

- 2026-05-19 (orchestrator, operator-decide pass): operator accepted
  all 5 analyst-recommended defaults via "Autoapprove all" directive
  (Q1=A in-memory return; Q2=A ThrottledSpinner only; Q3=A disabled-
  while-running + cancel poll; Q4=A session-local in-memory diff;
  Q5=A preserve all 22 anchors). Status stays `draft`; owner flipped
  to `pending-architect`. HANDOFF → architect for M-T1 decomposition.
- 2026-05-19 (analyst, this pass): refined the proposed stub into
  a full draft (status `draft`, version `0.1.0`); locked R1-R10
  requirements; surfaced Q1-Q5 with analyst-recommended defaults;
  added K1-K8 risk register + H1-H5 hypothesis register. Critical
  architecture finding: `crates/backtest` is already library-
  callable at the **type-surface** level (`lib.rs` re-exports
  everything ADR-0030 specified); the actual Phase B work is
  populating the `run_scenario` body via behaviour-preserving
  extraction from `main.rs`'s 3417 LOC of scenario dispatch. The
  refactor preserves all 22 body-SHA-256 anchors by construction
  (H2 is the gate). Reaffirmed default Q1=A (in-memory return),
  Q2=A (spinner only), Q3=A (disabled-while-running + internal
  cancel poll), Q4=A (session-local in-memory diff), Q5=A
  (preserve all 22 anchors). HANDOFF → operator-decide on
  Q1-Q5, then → architect for M-T1 decomposition.
- 2026-05-19 (orchestrator): brief stub opened on operator
  direction "1 then 2" (perf was already shipped; Phase B is
  the live next item). Predecessor verified at
  `ui-rethink-phase-a-lab v0.2.0` shipped 2026-05-18. Status
  `proposed`; awaiting analyst pass.
