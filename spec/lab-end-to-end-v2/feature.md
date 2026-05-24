---
slug: lab-end-to-end-v2
version: 0.1.0
status: proposed
owner: analyst
updated: 2026-05-24
parent: ui-rethink-phase-b-lab-run
---

# Lab end-to-end v2 — close the Phase A + B gaps, add progress bar

> **Predecessor chain**: this brief is a v2 closure pass on the Lab
> vertical shipped across
> [`ui-rethink-phase-a-lab v0.2.0`](../ui-rethink-phase-a-lab/feature.md)
> (2026-05-18) and
> [`ui-rethink-phase-b-lab-run v0.2.0`](../ui-rethink-phase-b-lab-run/feature.md)
> (2026-05-19). Both phases shipped operator-approved, but a 2026-05-24
> operator verification walk-through of the Lab screen confirmed
> **multiple post-ship gaps** between the locked R-items in those briefs
> and the runtime reality. v2 systematically closes the gaps and adds a
> NEW progress-bar widget the operator requested during the verification
> walk. The progress bar is **one R-row** in v2's set, not the headline —
> the headline is "Lab actually works end-to-end as Phase A+B promised."

## Why

### Operator complaint (verbatim from the 2026-05-24 verification walk)

> "The Lab screen has multiple gaps vs spec intent, plus I want a
> progress-bar widget visible during backtest computation. The Run
> button shows 'Running' indefinitely; clicking a pair chip doesn't
> swap the chart; the Stop button doesn't actually stop anything; and
> the chart's equity overlay after a fresh Run doesn't redraw with
> the new run's data."

### Spec promise vs reality (operator-validated 2026-05-24)

| Spec promise (Phase A / B locked R-row)                  | Reality at 2026-05-24                                                    | Root cause (analyst, this brief)                                                                                                  |
| -------------------------------------------------------- | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------- |
| Phase A R2.1 — Layer 1 price line for selected pair      | Broken                                                                   | Chart reads `chart_buffer` populated by live `Message::BarReceived` for `model.selected_symbol` — not for `lab_state.pair`. Engine's cross-sectional strategies (`v1.momentum` / `top10_momentum_h1`) don't expose per-pair bars even if the buffer were wired. |
| Phase A R3.3 — pair chip swaps the chart                 | Decorative                                                               | `Message::LabSelectPair` writes `lab_state.pair` (`crates/ui/src/state.rs:1892-1897`) but does NOT update `model.selected_symbol`, which is the field the chart actually reads (`crates/ui/src/screens/lab.rs:149-152`). `LabRunConfig.symbol` is captured but `backtest::engine::run_scenario` ignores it — the universe is hardcoded `top10_symbols_with_prices()` for cross-sectional dispatch arms. |
| Phase B R5 — Lab chart reads equity from `last_run_report` after fresh Run | Partial (fresh-Run path broken)                                          | `Message::LabRunCompleted` arm at `state.rs:1922-1931` carries a comment "RunReportMirror rotation … is done by the binary-side `update` wrapper" — but **no such wrapper exists** in `crates/ui/src/bin/cockpit_live.rs`. The binary's `update` handler returns to iced without rotating `last_run_report ← new RunReportMirror`. The chart therefore falls through to `EquityCache` (Phase A's cached-report read path) and the fresh result is invisible. |
| Phase B R8 — compare-to-previous delta badge after fresh Run | Partial (same root cause as above)                                       | The badge widget renders (`crates/ui/src/widgets/run_delta_badge.rs`) but only when both `last_run_report` AND `prev_run_report` are `Some` AND share the same tuple. With the binary-side rotation missing, neither field is ever populated from a real run. |
| Phase A R2.1 — buy/sell triangle markers from fresh fills | Partial                                                                  | Same root cause: the markers wire to `model.chart_markers` (`PanelState<Vec<FillView>>`). Today this is populated only by `Message::ChartMarkersLoaded` after a `SelectSymbol`, which queries the **audit ledger** (`audit::query::recent_fills_filtered`). A fresh in-process Run completes, returns a `RunSummary { report_path: Some(...), .. }`, but no message arm rotates fills from the in-memory `RunReport.fills` (which is `Vec::new()` per `engine.rs:307` momentum_result_to_report, etc. — TODO at Phase C). |
| Phase B R4 — Run state machine to completion             | Regressed — button shows "Running" indefinitely                          | Symptom the operator just reported; partial fix landed in commit `fix(cockpit_live): ServerTimeRecipe panic` (2026-05-19). Either the `LabRunCompleted` message is **not delivered** to `state::update` (subscription/task wiring), or the long-running cross-sectional dispatch never returns. Phase B's H1 latency budget claimed Last30d ≤ 8 s; real measurements absent from operator reports. The dev-loop never actually verified the message arrives. |
| Phase A R9 — Stop button works                           | Broken (pre-existing)                                                    | `RunCancelHandle` immediately dropped at `crates/ui/src/bin/cockpit_live.rs:1027`: `let (_, cancel_recv) = cancellation_pair()` discards the sender. Engine ignores `_cancel` (`#[allow(dead_code)]` on `RunCancelReceiver::rx`; `run_scenario` does not thread `RunCancelReceiver` into scenario dispatch at all). Phase B's R7 explicitly scoped this; the implementation never landed. |
| Phase B R6 — Progress UX (ThrottledSpinner, no progress bar) | Spinner ships but is opaque                                              | Operator post-ship preference: "I want a progress bar, not a spinner." No progress channel exists end-to-end: scenario modules in `crates/backtest/src/scenarios/momentum.rs` etc. have no `Sender<Progress>` parameter; UI has no Recipe that bridges scenario progress to a `Message::LabRunProgress(...)`. |
| Phase A R4.2 — multi-strategy compare ≤4 (shipped)       | Works                                                                    | No gap. Compare-set list, +/− toggle, cap-toast all functional per current state.rs path. Not in v2 scope. |

### Why we're doing this now

v2 closes a credibility gap: Phase A and B both shipped on operator
approval that did not include a fresh-Run end-to-end demo. The
verification walk now exposes the gaps. Two of them — the missing
binary-side wrapper (`LabRunCompleted` → rotate `last_run_report`)
and the `LabSelectPair` → `selected_symbol` disconnect — are
**one-line wiring bugs** that should never have shipped under the
locked Phase A/B R-rows. The rest are scope-leakage debts (Stop
button never wired; engine dispatch only knows cross-sectional
strategies; progress-bar widget not in scope at Phase B). Operator
wants the slice closed before any further Phase work on Lab.

## Scope (v2 — analyst-proposed; architect ratifies at M-T1)

- **Close the wiring bugs** that prevent Phase A/B's locked R-rows
  from delivering at runtime (R1, R2, R5).
- **Decide single-symbol vs cross-sectional strategy dispatch** so
  the pair chip actually changes which data the engine reads (R3).
- **Wire the Stop button end-to-end** (R6) — `RunCancelHandle`
  threaded into `LabState` (or `Cockpit`), receiver polled inside
  scenario dispatch.
- **Add the progress-bar widget** (R7-R9) — new widget +
  `Message::LabRunProgress` + scenario-side `Sender<Progress>` or
  derived progress source.
- **Verify the Run completion path** with an integration test that
  spawns a real run from a UI fixture and asserts
  `lab_run_inflight: false` arrives within the H1 latency budget.

## Out of scope

- Phase C (sidebar IA), Phase D (Trail), Phase E (Compare matrix),
  Phase F (Memory + Assistant). Their R-rows already shipped or are
  queued.
- New strategy code or matching-engine work — v2 wires existing
  scenarios, may add a dispatch arm for an EXISTING strategy module
  (e.g. SMA cross is already in `crates/strategy/src/sma.rs` —
  wiring it into `engine::run_scenario` is a few lines), but does
  NOT add new strategy logic.
- Real backtest of any new pair: v2 ships the wiring; the operator's
  next move is to actually run `XRPUSDT` against `v1.momentum` and
  see meaningful results. v2's success criterion is "the wiring
  delivers" not "the strategy is profitable on XRP".
- Persistence of `last_run_report` across cockpit restarts (Phase B
  Q5-A: in-memory only, unchanged in v2).
- Param-sheet editor (Phase B Out-of-scope; remains so).
- Multi-strategy / multi-pair batch runs.

## Architecture findings

### F1 — Binary-side update wrapper is missing

`crates/ui/src/state.rs:1924-1931` carries the comment:

```rust
// NOTE: RunReportMirror rotation (last→prev, set last=new) is done
// by the binary-side update wrapper which has access to the full
// RunSummary + equity series. The pure update cannot build a
// RunReportMirror because it has no equity data or BacktestKpis
// (those come from the async run result stored in the binary layer).
```

`crates/ui/src/bin/cockpit_live.rs` has an `update` wrapper at
~line 794 that intercepts `Message::LabRunRequested` (to capture
the pre-mutation `LabState` and build `LabRunConfig`), then calls
`ui::state::update(&mut self.cockpit, msg)`. **There is no
intercept for `Message::LabRunCompleted(Ok(_))`** — the message is
passed straight through to `state::update` which clears
`lab_run_inflight` and clears nothing else.

Phase B's R5 ("Lab chart reads equity from `last_run_report` first,
cache second") is therefore dead code: `last_run_report` is never
set from a real run. The Phase B determinism tests pass because
they directly construct a `RunReportMirror` and inject it via the
test helper (`set_last_run_report` at `equity_loader.rs:930`).

Concrete fix surface: add an `if let Message::LabRunCompleted(Ok(summary)) = &msg`
intercept in `cockpit_live.rs::update` BEFORE the call to
`ui::state::update`, capture the `RunSummary.report_path`, load
the on-disk report via `EquityCache::load_from_path` (or similar),
build a `RunReportMirror`, and assign to `lab_state.last_run_report`
(rotating the old `last → prev`). OR: change `RunSummary` to carry
the full `RunReport` in-memory and skip the disk read.

### F2 — `LabSelectPair` does not update `selected_symbol`

`crates/ui/src/state.rs:1892-1897`:

```rust
Message::LabSelectPair(venue, symbol) => {
    model.lab_state.pair = Some((venue, symbol));
    // T-D-N10: tuple changed — clear both run report mirrors.
    model.lab_state.last_run_report = None;
    model.lab_state.prev_run_report = None;
}
```

The chart's price-line read site is `screens/lab.rs:149-152`:

```rust
let active = model
    .selected_symbol
    .clone()
    .or_else(|| model.universe.first().cloned());
```

`active` drives `chart_buffer.bars(*v, s)` (line 243) and the markers/
signals (which key on `selected_symbol` because they're populated by
`Message::SelectSymbol` → audit-ledger fetch). Clicking a Lab pair
chip leaves `selected_symbol` unchanged, so the chart shows the
same data regardless of which pair the operator picked. Fix is
one line: add `model.selected_symbol = Some((venue, symbol.clone()));`
to the `LabSelectPair` arm. The downstream `SelectSymbol` cascade
already handles markers/signals fetch + tooltip clear.

### F3 — `engine::run_scenario` dispatch is cross-sectional-only

`crates/backtest/src/engine.rs:437-516` defines five dispatch arms:

| `cfg.strategy` value                     | Scenario module                       | Universe        |
| ---------------------------------------- | ------------------------------------- | --------------- |
| `v1.momentum` / `top10_momentum_h1`      | `scenarios::momentum::run`            | top-10 hardcoded |
| `v1.5a.mr` / `v1.5a.pairs` / `pairs_mr_h1` | `scenarios::pairs::run`               | pair universe   |
| `v2.5.tcn` / `v2.5.tcn_overlay` / `tcn_overlay_momentum` | `scenarios::tcn_overlay::run` | top-10 hardcoded |
| `v2.5.tcn.weights` / `v2.5.tcn_overlay_weights` | `scenarios::tcn_overlay_weights::run` | top-10 hardcoded |
| anything else                            | `Err(RunError::UnknownStrategy)`      | —               |

The single-symbol scenarios (SMA crossover, MACD trend, RSI
reversion, BBands mean-revert) live in `crates/backtest/src/main.rs`
(the CLI binary) only, NOT in `engine::run_scenario`. Their
in-process callability was a Phase B promise — the architecture
finding in Phase B's brief said "extract the body of `main()`'s
dispatch (and the 4 `run_*_backtest` fns + their writers) into the
`engine` module behind `run_scenario(cfg)`" — but only the four
multi-symbol paths landed (momentum, pairs, tcn, tcn-weights). The
3 single-symbol paths are still binary-only.

Consequence: when the operator clicks **`XRPUSDT` + `v1.momentum`**,
the engine actually runs the full top-10 momentum universe (which
INCLUDES XRPUSDT) and returns a portfolio equity curve. The "this
pair vs that strategy" UX is a lie — the strategy is portfolio,
the pair is decorative.

Options (Q1 operator-decide):

1. **(a) Single-symbol dispatch arms.** Extend `engine::run_scenario`
   with `"v0.sma"`, `"v0.5.macd"`, `"v0.5.rsi"`, `"v0.5.bbands"`
   arms that route to per-symbol scenarios (mostly already in
   `main.rs`). Lab gains 4 strategy chips that respect the pair.
2. **(b) `pair_filter: Option<Symbol>` on cross-sectional arms.**
   Add a config field that subsets the universe to the selected
   pair. Cross-sectional strategies degrade gracefully to single-
   symbol (`v1.momentum` on XRPUSDT alone is just "long when
   12-week return > 0"). Cheaper but the strategy semantics shift.
3. **(c) Scope-flag in Lab UI.** Add a "scope: portfolio vs single-
   symbol" selector to the Lab top-bar. Cross-sectional strategies
   gray out the pair picker (with a "portfolio mode" badge);
   single-symbol strategies enable it. Most explicit, biggest UI
   change.
4. **(d) Defer.** Accept that pair chip is decorative for v1.x
   strategies and document the gap; ship v0 single-symbol dispatch
   in a follow-up.

Analyst recommendation: **(a) + (d)** — ship single-symbol arms for
`v0.sma`, `v0.5.macd`, `v0.5.rsi`, `v0.5.bbands` (already-implemented
strategies; one dispatch-table extension per arm, no new strategy
code) AND defer (b)/(c) to a future "compare cross-sectional vs
single-symbol on the same pair" feature. The 4 new arms unblock the
operator's headline workflow ("XRPUSDT vs SMA crossover").

### F4 — `RunCancelHandle` lifecycle is broken

`crates/ui/src/bin/cockpit_live.rs:1027`:

```rust
let (_, cancel_recv) = ui::lab::runner::cancellation_pair();
ui::lab::runner::spawn_lab_run(Some(&self.rt_handle), run_cfg, cancel_recv)
```

The `RunCancelHandle` (sender) is immediately discarded into `_`.
`cancellation_pair`'s contract says "dropping the handle signals
the receiver" (`runner.rs:96-109`); the receiver therefore sees
`Disconnected` on the first `try_recv`, which means
`RunCancelReceiver::is_cancelled()` returns **`true` from the
start**. The receiver is also `#[allow(dead_code)]` because no one
calls `is_cancelled()` — the engine's `run_scenario` signature
takes `ScenarioConfig` but no `cancel_rx`.

End state: cancellation is wired in name only. No code path makes
Stop work. The cockpit doesn't even have a Stop button rendered
(the Run button toggles to "Running" on press but has no Cancel
affordance — Phase B Q3 explicitly defaulted to "bool inflight
gate only, no Cancel button"; operator's 2026-05-24 walk reverses
that decision).

### F5 — No progress channel exists end-to-end

`crates/backtest/src/scenarios/momentum.rs` has no `progress` /
`Sender` / `progress_tx` symbol. The other scenario modules likewise
emit no progress signals. The audit-tick stream is per-fill-event,
not per-bar — using it as a progress proxy would require knowing
the total bar count upfront and counting events as they arrive
(possible but indirect).

Options (Q4 operator-decide):

1. **(a) Explicit progress channel.** Add `progress_tx:
   Option<tokio::sync::mpsc::Sender<Progress>>` to
   `ScenarioConfig`. Each scenario emits `Progress { current_bar,
   total_bars }` every N bars (suggest N=128, matching the
   cancellation poll cadence). UI subscribes via an `iced::Recipe`
   that maps `Progress` → `Message::LabRunProgress(f32)`. Pure;
   touches every scenario module.
2. **(b) Derived from cancellation poll boundary.** Reuse the
   `bar_idx & 0x7F == 0` cancellation poll point. Same Sender
   threading but only one new emit site per scenario (at the poll
   boundary). Same blast radius as (a) without the per-N tunable.
3. **(c) Time-based, no scenario change.** Run a UI-side timer
   that animates a fake-but-monotone progress bar based on elapsed
   time / expected duration. Operator complaint: "wrong progress
   feels worse than no progress" — likely rejection.
4. **(d) Approximate via wall-clock + bar-count estimate.** Compute
   expected bars upfront (`bar_count = date_range_to_scenario_params(range).1`)
   and a per-bar wall-clock estimate from a smoke run; UI shows
   `elapsed_s / expected_s * 100%`. Self-corrects when the run
   actually completes ("snap to 100% on `LabRunCompleted`"). No
   backend changes.

Analyst recommendation: **(b)** — reuse the cancellation poll
boundary as the natural emit point. One Sender threading change
per scenario; bar-accurate progress; minimal blast radius. Falls
back to (d) only if the operator wants progress for the no-`live`
fixture cockpit (which has no engine call at all — the runner's
`#[cfg(not(feature = "live"))]` arm returns immediately).

### F6 — Cascaded effect on chart markers

A successful in-process Run completes with `RunReport.fills:
Vec<FillView>` populated (or with `Vec::new()` per the Phase B
TODO at `engine.rs:307`). Either way, `chart_markers` doesn't
update because no `Message::ChartMarkersLoaded` is dispatched
after `LabRunCompleted`. Phase A's R2.1 said "verify the wiring
against the Lab data source (cached report fills)" — that
verification appears to have used the audit-ledger fills from a
PRIOR live cockpit session, not the fresh-Run fills. The fix
chains: extend the binary-side wrapper (F1) to also dispatch
`Message::ChartMarkersLoaded(Ok(fills_from_runreport))` after
`LabRunCompleted(Ok(_))`.

## Requirements

Numbered, testable, derived from the operator's verification-walk
findings + the architecture findings F1-F6. Each R-item preserves
the 34 locked body-SHA-256 anchors in [`spec/anchors.toml`](../anchors.toml)
and the 692 lib tests baseline.

### R1 — Pair chip swaps the chart's reference data

- **R1.1** `Message::LabSelectPair(venue, symbol)` updates BOTH
  `model.lab_state.pair` AND `model.selected_symbol = Some((venue,
  symbol.clone()))`. The single-line fix to the existing arm at
  `state.rs:1892-1897`.
- **R1.2** As a downstream side-effect of (R1.1), the binary-side
  `update` wrapper's existing `select_pair` capture path
  (`cockpit_live.rs:861-916`) fires for pair-chip clicks and
  fetches markers + signals for the new pair. (Today
  `LabSelectPair` does not trigger `Message::SelectSymbol`; v2
  changes that.)
- **R1.3** The chart's `chart_buffer.bars(venue, symbol)` call
  returns bars for the chip-selected pair as soon as the live
  subscription is delivering bars for that pair. (Live cockpit
  only; the fixtures cockpit needs R1.4.)
- **R1.4** Fixtures cockpit (`crates/ui/src/bin/cockpit.rs`)
  pre-loads bars for ALL pairs in `XRP_FIRST_UNIVERSE` so the
  chart is non-empty regardless of pair selection. Today only
  the default pair has bars loaded.
- **Acceptance:** click XRPUSDT chip → chart's price-line redraws
  with XRP data; click ETHUSDT → ETH data. insta snapshot
  `lab__pair_chip_swaps_chart` records the post-click state for
  three pairs.

### R2 — Fresh-Run rotates `last_run_report` (binary-side wrapper)

- **R2.1** `crates/ui/src/bin/cockpit_live.rs::update` intercepts
  `Message::LabRunCompleted(Ok(summary))` BEFORE forwarding to
  `ui::state::update`. The intercept captures `summary.report_path`
  (if `Some`) or the in-memory result (R2.4), builds a
  `RunReportMirror`, and rotates `prev_run_report ←
  last_run_report` then `last_run_report ← Some(new_mirror)`.
- **R2.2** The intercept uses `Cockpit::lab_state.{strategy, pair,
  range}` (pre-update snapshot) to build the `LabTuple` so the
  mirror matches what the chart will compare against in `route_equity_overlay`.
- **R2.3** On `Message::LabRunCompleted(Err(_))`, the wrapper does
  NOT rotate (no `last_run_report` mutation on failure). The Run
  button transitions to `RunState::Failed`.
- **R2.4** `RunSummary` is extended to carry the in-memory
  `RunReport.equity_series` + `RunReport.kpis` (analyst proposes
  promoting from `Option<PathBuf>` to a struct that includes both
  the path and the in-memory data). Avoids the on-disk re-read.
  Operator-decide via Q3 below — Q3-A (extend `RunSummary`)
  preferred over Q3-B (on-disk re-read).
- **R2.5** After rotation, the wrapper dispatches a follow-up
  `Message::ChartMarkersLoaded(Ok(fills))` and
  `Message::ChartSignalsLoaded(Ok(...))` if the `RunReport`
  carries non-empty fills. (When `RunReport.fills.is_empty()`,
  which is current default per `engine.rs:307`, no marker update —
  the chart shows the equity curve but no triangles.)
- **Acceptance:** click Run → chart's equity overlay redraws with
  the new run's curve within 8 s on `Last30d` (H1 budget); the
  delta-badge appears on the second Run press. New integration
  test `lab_run_fresh_overlay_redraws_e2e` at
  `crates/ui/tests/lab_run_integration.rs`.

### R3 — Single-symbol strategy dispatch arms (operator-decide Q1)

- **R3.1** Pending Q1 resolution. Analyst-recommended default:
  extend `engine::run_scenario` with 4 new arms:
  - `"v0.sma"` / `"sma_cross_h1"` → `scenarios::sma::run` (extract
    from `main.rs`'s `run_sma_backtest`).
  - `"v0.5.macd"` / `"macd_trend_h1"` → `scenarios::macd::run`.
  - `"v0.5.rsi"` / `"rsi_reversion_h1"` → `scenarios::rsi::run`.
  - `"v0.5.bbands"` / `"bbands_mean_revert_h1"` →
    `scenarios::bbands::run`.
- **R3.2** Each new arm respects `cfg.pair.1` (the selected
  Symbol) — the dispatch builds a single-symbol scenario input
  keyed on that pair.
- **R3.3** Anchor preservation: the 4 extracted-to-engine paths
  must produce byte-identical reports vs. the CLI path on the 4
  legacy anchors (`btc-2023-1m-sma-cross`, `…-macd-trend`,
  `…-rsi-reversion`, `…-bbands-mean-revert`). Same constraint
  Phase B applied to momentum/pairs/tcn.
- **R3.4** Lab strategy chip row gains the 4 new strategies once
  the dispatch is in. The strategies registry already enumerates
  them (`crates/ui/src/state.rs:733-736`).
- **Acceptance:** click XRPUSDT + v0.sma + Last90d + Run → engine
  runs SMA crossover on XRPUSDT alone (no top-10 universe); chart's
  equity curve reflects single-symbol P&L; 4 legacy anchors stay
  green via `scripts/verify_anchors.sh` (34/34).

### R4 — Run-completion path verified end-to-end

- **R4.1** A new integration test `lab_run_e2e_completion` boots
  the live cockpit binary with `--fixtures`, dispatches
  `Message::LabRunRequested` programmatically, and asserts:
  - `lab_run_inflight` flips to `true` within 1 iced-update cycle.
  - `Message::LabRunCompleted(Ok(_))` arrives within 30 s (loose
    upper bound to accommodate `Last30d` + cold cargo build).
  - `lab_run_inflight` flips back to `false`.
  - `lab_state.last_run_report` is `Some(_)` with `equity_series.len() > 0`.
- **R4.2** The test runs in CI (not `#[ignore]`d) — it gates the
  Phase B claim that the round-trip works.
- **R4.3** Investigate whether the "Running indefinitely" symptom
  is (a) `LabRunCompleted` never dispatched (subscription /
  rt_handle issue), (b) `run_scenario` hanging in a scenario
  module's bar loop, or (c) the wrapper intercept (R2) silently
  consuming the message without forwarding. R4.1 plus tracing
  spans (already present in `runner.rs:303-339`) should triangulate.
- **Acceptance:** the test green; the Run button reliably returns
  to Idle/Completed/Failed within 30 s for `Last30d`.

### R5 — Buy/sell markers from fresh fills

- **R5.1** Per F6 + R2.5: after `LabRunCompleted(Ok(_))` with a
  populated `RunReport.fills`, the binary-side wrapper dispatches
  `Message::ChartMarkersLoaded(Ok(fills))`.
- **R5.2** Today `RunReport.fills` is `Vec::new()` for all
  scenarios (`engine.rs:307` momentum, `engine.rs:335` pairs,
  `engine.rs:363` tcn). v2 populates `fills` from the scenario
  modules' actual fill records. Each scenario module already
  produces fills internally for KPI computation — surface them
  through the result struct.
- **R5.3** Anchor risk: the fill list does NOT appear in the
  Markdown report body (the report contains KPI + equity-curve
  block + optional small fills table). Adding fills to the
  in-memory `RunReport` is byte-orthogonal to the body bytes.
  Architect to confirm at M-T1 by reading one report from each
  family.
- **Acceptance:** insta snapshot `lab__fresh_run_markers` shows
  the chart with triangle markers from a real engine run; no
  anchor SHA changes.

### R6 — Stop button works

- **R6.1** `crates/ui/src/bin/cockpit_live.rs:1027` is fixed to
  store the `RunCancelHandle` in `Cockpit` or `LabState` instead
  of dropping it: `let (handle, cancel_recv) = …;
  self.cockpit.lab_state.run_cancel = Some(handle);`. The handle's
  Drop is then tied to either a new `Message::LabStopPressed`
  (operator-explicit cancel) or the next `LabRunRequested` press.
- **R6.2** `engine::run_scenario` is extended to accept
  `cancel_rx: Option<RunCancelReceiver>` as a `ScenarioConfig`
  field (or a separate argument — architect picks at M-T1) and
  passes it into each scenario module's bar loop. Bar loop polls
  `cancel_rx.is_cancelled()` at the `bar_idx & 0x7F == 0`
  boundary; on `true`, returns `Err(RunError::Cancelled)`.
- **R6.3** Lab UI gains a Stop button rendered next to Run when
  `lab_run_inflight == true`. Clicking dispatches
  `Message::LabStopPressed`; the message arm drops the handle.
- **R6.4** `LabRunCompleted(Err(RunError::Cancelled))` transitions
  the Run button to `RunState::Idle` (not `Failed` — cancellation
  is operator-initiated, not strategy failure). Phase B's K6
  ("obtuse error") guidance applies.
- **Acceptance:** click Run → wait 2 s → click Stop → engine
  returns `Err(Cancelled)` within 1 s (one 128-bar poll period
  at ~10 ms/poll); cockpit Run button returns to Idle.

### R7 — Progress channel: scenario emits progress at poll boundary

- **R7.1** `ScenarioConfig` gains
  `progress_tx: Option<tokio::sync::mpsc::Sender<Progress>>` where
  `Progress { current_bar: usize, total_bars: usize,
  elapsed_ms: u64 }`. (Operator-decide Q4 — analyst recommends
  (b) reuse cancellation poll boundary.)
- **R7.2** Each scenario module's bar loop sends a `Progress` at
  the `bar_idx & 0x7F == 0` boundary (same gate as
  cancellation). `try_send` is used so a slow UI doesn't backpressure
  the engine; dropped progress events are acceptable.
- **R7.3** The channel is built in `spawn_lab_run`; the receiver
  end is passed to an `iced::Recipe` that polls and yields
  `Message::LabRunProgress(Progress)`. The Recipe runs alongside
  the existing `Subscription::batch` in `cockpit_live.rs::subscription`.
- **R7.4** Channel capacity 8 (lossy when UI lags). `Drop` of the
  sender (engine completes / aborts) closes the channel and the
  Recipe yields one final `Message::LabRunProgressDone`.
- **Acceptance:** mid-run, the UI receives ≥ 1 `LabRunProgress`
  message per 128-bar batch (≈ 5-10 messages over a Last30d
  720-bar run); messages arrive within 50 ms of emit (loose
  budget).

### R8 — Progress-bar widget

- **R8.1** New widget
  `crates/ui/src/widgets/progress_bar.rs` — Lumen-token-styled
  determinate progress bar. Inputs: `progress: f32 \in [0.0, 1.0]`,
  `label: Option<&str>` (e.g. "412 / 720 bars · 3.4s"),
  `mode: ThemeMode`. Outputs: `Element<'static, Message>`.
- **R8.2** Visual contract: fixed height 8 px; track color
  `color::BG_3`; fill color `color::ACCENT_2`; rounded corners
  matching the cockpit's button radius (`radius::SMALL`).
  Label sits above-left; percent sits above-right. No animations
  beyond the natural width transition between renders.
- **R8.3** Indeterminate state (`progress: None`) renders a
  shimmer-stripe pattern to distinguish "we know how far we've
  got" from "we don't know yet." Animation is a 2 s linear
  translate on the stripe; pauses when the cockpit is unfocused
  (existing `ThrottledSpinner` pattern).
- **R8.4** Widget renders in the Lab top-bar next to the Run
  button when `lab_run_inflight == true`. Hidden otherwise. The
  Run button keeps its "Running" label (R8.4 does NOT replace the
  button's state).
- **R8.5** ui_gallery_bin gets a `progress_bar__*` panel with
  three snapshot cases: 0%, 50%, 100%, indeterminate.
- **Acceptance:** insta snapshots
  `progress_bar__0pct`, `__50pct`, `__100pct`, `__indeterminate`;
  cockpit snapshot `lab__running_with_progress_bar` shows the
  widget at 50 % during a fake-paused run.

### R9 — Lab state machine carries progress

- **R9.1** `LabState` gains
  `run_progress: Option<Progress>` — `None` when no run is
  in-flight or the engine hasn't emitted a `Progress` yet.
- **R9.2** `Message::LabRunProgress(Progress)` arm in
  `state::update` writes `lab_state.run_progress = Some(progress)`.
- **R9.3** `Message::LabRunCompleted(_)` and
  `Message::LabRunRequested` both clear
  `lab_state.run_progress = None`.
- **R9.4** The Lab view reads `lab_state.run_progress` and passes
  it into `progress_bar::view`. When `None` and
  `lab_run_inflight == true`, renders the indeterminate variant.
- **Acceptance:** `cargo test -p ui lab_state_progress` covers
  the message arms; lab.rs view test covers the widget render
  with progress at three values.

### R10 — Verification gates (non-regression contract)

- **R10.1 — Anchors stay byte-identical (34/34).** v2 touches the
  engine dispatch table (R3 new arms), the binary-side update
  wrapper (R2), scenario modules' bar loops (R6/R7 add cancel +
  progress sends). Each must preserve the existing report body
  bytes for the 4 legacy single-symbol anchors + 8 cross-sectional
  anchors. `scripts/verify_anchors.sh` 34/34 PASS is the gate.
- **R10.2 — 692 lib tests stay green.** No regressions in the
  existing test surface. New tests are additive.
- **R10.3 — Phase F default-disabled byte-identity.** v2 does not
  touch Phase F code; the runtime-gated R9.3 path in `state.rs`
  + `shell.rs` for Phase F MUST remain byte-identical (panel
  snapshot diff = 0 with `RIGHT_RAIL_ENABLED = false`).
- **R10.4 — `spec-lint` clean.** New feature folder MUST pass
  `uv run scripts/spec_lint.py` with zero violations across all
  9 enforced categories.
- **R10.5 — `cockpit-smoke` PASS.** UI-affecting changes (R1, R5,
  R6, R8, R9) gate on a clean cockpit-smoke run per the always-on
  ui-quality-gate.
- **R10.6 — Idle-CPU floor ≤ 13.1 %.** Progress channel + Recipe
  + repaints MUST NOT push the cockpit-performance budget over its
  cap. Measured in a 30 s idle hold with progress bar idle (no
  run in flight).
- **R10.7 — Determinism preserved.** Cancellation introduces a
  non-determinism risk (where exactly is bar_idx when the cancel
  fires?). On a non-cancelled run, results MUST be byte-identical
  to a non-cancellation-instrumented run. The bar-loop polls are
  read-only (no RNG impact).

## Operator decision questions

Each Q has an **analyst-recommended default** the architect should
adopt unless the operator picks differently. Defaults are surfaced
explicitly so the architect's M-T1 can proceed without blocking on
the operator for low-risk paths.

### Q1 — Strategy dispatch shape (HIGH-stakes; operator decides)

Should v2 close the pair-chip gap via:

- **(a)** Single-symbol dispatch arms for `v0.sma`, `v0.5.macd`,
  `v0.5.rsi`, `v0.5.bbands` in `engine::run_scenario`. Lab gains
  4 strategy chips that respect the pair.
- **(b)** `pair_filter: Option<Symbol>` on the cross-sectional
  arms. Universe subsets to the selected pair.
- **(c)** "Scope: portfolio vs single-symbol" selector in Lab UI;
  cross-sectional strategies gray out the pair picker.
- **(d)** Defer — accept that pair chip is decorative for v1.x
  strategies; ship single-symbol dispatch in a follow-up.

**Analyst default: (a) + (d)** — extend dispatch with the 4
single-symbol arms now; defer (b)/(c) to a future feature. The
operator's headline workflow ("test SMA crossover on XRP") works
under (a); (b)/(c) add complexity v2 doesn't need.

### Q2 — Stop button: in scope for v2, or sibling feature?

- **(a)** In scope. R6 lands in v2; Stop button works end-to-end.
- **(b)** Sibling feature `cockpit-lab-stop-button`. v2 ships
  without Stop; operator forces a kill via cockpit restart for
  long-running tests.

**Analyst default: (a)** — the cancellation glue is half-shipped
(`RunCancelHandle` + receiver exist, just dropped). Wiring it in
v2 is 1-2 days; deferring leaves a known broken affordance.

### Q3 — `last_run_report` data path: in-memory vs disk-re-read

- **(a)** Extend `RunSummary` to carry the full `equity_series` +
  `kpis` from `RunReport`. Binary-side wrapper builds
  `RunReportMirror` from the in-memory data directly.
- **(b)** Keep `RunSummary` minimal; binary-side wrapper reads
  the on-disk report via `EquityCache::load_from_path(report_path)`.

**Analyst default: (a)** — Phase B's "in-memory return, no disk
at Phase B" decision (Q1-A) was clean. (b) re-introduces a disk
hop and a fail mode (report not found).

### Q4 — Progress channel shape

- **(a)** Explicit progress channel with operator-tunable cadence
  (every N bars).
- **(b)** Reuse cancellation poll boundary (`bar_idx & 0x7F == 0`).
- **(c)** UI-side time-based fake.
- **(d)** Time-based with bar-count estimate; snap to 100% on
  completion.

**Analyst default: (b)** — minimal blast radius; same poll site
as cancellation; bar-accurate.

### Q5 — Progress UX: bar vs spinner

- **(a)** Add progress bar; keep ThrottledSpinner for indeterminate
  ops elsewhere (training, audit-query).
- **(b)** Replace ThrottledSpinner with progress bar everywhere
  (out of scope for v2; documented for a Phase D+ pass).
- **(c)** Bar + spinner stacked (bar shows progress, spinner shows
  "engine still alive").

**Analyst default: (a)** — operator asked for a bar in Lab;
spinner stays for ops without a known-total (audit queries, etc.).

### Q6 — Fixtures cockpit pair pre-loading (R1.4)

- **(a)** Pre-load bars for all pairs in `XRP_FIRST_UNIVERSE` at
  fixtures boot. Cold-start memory ↑ ~10 × 60 bars ≈ 12 KB.
- **(b)** Lazy-load per pair on first `LabSelectPair`. First
  click shows empty chart for ~1 frame.
- **(c)** Use a single pair's bars + symbol-rewrite at render
  time (existing approach for the gallery).

**Analyst default: (a)** — 12 KB is free; the visual gap is
worse than the memory cost.

### Q7 — Cancellation receiver threading: `ScenarioConfig` field vs separate arg

- **(a)** Add `cancel_rx: Option<RunCancelReceiver>` to
  `ScenarioConfig`. Single call surface; `Clone` impossible on
  the config (already true today via mpsc::Receiver).
- **(b)** Separate `run_scenario(cfg, cancel_rx)` argument. Keeps
  `ScenarioConfig` cloneable.

**Analyst default: (b)** — `ScenarioConfig` is `Clone` today
(`#[derive(Debug, Clone)]` at `engine.rs:135`); preserving that
is cheap and useful for test fixtures.

### Q8 — Run-button label transitions on Stop press

- **(a)** Idle (cancellation is operator-initiated; no failure).
- **(b)** Cancelled (new RunState variant; flashes for 2 s then
  → Idle).

**Analyst default: (b)** — distinguish "stopped" from "never ran"
for delta-badge correctness (the delta badge compares last+prev
in same tuple; a cancelled run is not a valid `last`).

## Hypothesis register

Falsifiable predictions the tester checks at M-T-FINAL.

### H1 — Run completes within budget

Pressing Run on `(v1.momentum, XRPUSDT, Last30d)` completes within
8 s wall-clock on the operator's M3 Pro (Phase B's locked budget).
Falsified if the test in R4.1 times out or routinely takes > 10 s.

### H2 — Chart redraw within 100 ms of completion

Time from `Message::LabRunCompleted(Ok(_))` arrival to next
iced repaint that includes the new equity curve ≤ 100 ms.
Falsified if the operator perceives a stutter (logged via
`tracing::info!("lab.run.repaint_ms")`).

### H3 — Progress bar reaches 100% reliably

For each scenario, the progress bar reaches `1.0` within the same
50 ms window as `LabRunCompleted` arrival. Falsified if any run
ends with a stuck progress < 0.95.

### H4 — Stop returns within 1 s

Click Stop during a Last30d run; `LabRunCompleted(Err(Cancelled))`
arrives within 1 s. Falsified at 95% percentile over 20 trials.

### H5 — Anchors byte-identical post-refactor

All 34 anchors in `spec/anchors.toml` PASS `scripts/verify_anchors.sh`.
Falsified if any single anchor diff appears.

### H6 — Progress channel does NOT regress idle CPU

Idle cockpit (no run in flight, no progress events) measures ≤ 13.1 %
CPU on the 30 s idle hold per Phase B's R5/H5 (renamed).
Falsified at the cockpit-performance baseline.

## K — Risk register

### K1 — Refactor of `run_scenario` signature breaks Phase B determinism tests

Adding `cancel_rx` or `progress_tx` to `ScenarioConfig` (R6/R7)
changes the public type. Phase B's 6 unit tests
(`engine.rs:546-735`) construct `ScenarioConfig` literals. They
need `..Default::default()` or explicit `None` fields. Tester
ratchet: zero changes to the **inside** of those tests' assertions.

Mitigation: add `#[derive(Default)]` on `ScenarioConfig` (or
mirror via `ScenarioConfig::for_test()`) so existing tests stay
1-line changes.

### K2 — Single-symbol dispatch arms (R3) mutate anchor SHAs

R3 extracts the 4 single-symbol scenario paths from
`crates/backtest/src/main.rs` into engine modules. Phase B's
risk K1 applies: if any extraction changes the report body bytes
(e.g. timestamp rounding, equity-curve precision, fills-table
formatter), the anchor breaks.

Mitigation:
- Treat each extraction as a "behaviour-preserving move"
  governed by `verify_anchors.sh` on each commit.
- Architect M-T1 pre-stages anchor SHAs via
  `scripts/pre_stage_anchors.sh` before any extraction; tester
  M-FINAL verifies bytes match.

### K3 — `LabRunCompleted` interception order

The binary-side wrapper (R2) intercepts `LabRunCompleted` BEFORE
forwarding to `state::update`. If the order flips (forward first,
intercept after), `lab_run_inflight` is already false and the
wrapper's snapshot of `lab_state.{strategy, pair, range}` may be
stale (e.g. operator clicked away during the run).

Mitigation:
- Wrapper builds the snapshot BEFORE the forward call (mirroring
  the existing `lab_run_requested` capture pattern at
  `cockpit_live.rs:813-816`).
- Unit test: synthesize the message, assert the wrapper's
  captured snapshot is pre-mutation.

### K4 — Stop pressed during scenario startup (before first poll)

Cancellation polls every 128 bars (~10 s at 1 bar/80 ms). If the
operator clicks Stop in the first 10 s of a Last30d run, the
poll fires at bar 128; engine returns Cancelled around t=10 s
even though Stop was clicked at t=2 s. Operator perceives "Stop
took 8 s."

Mitigation:
- Poll at 32-bar boundary (~2.5 s worst case) for the first 128
  bars, then fall back to 128-bar boundary.
- Tester documents the worst-case latency; H4 budget loosens to
  3 s if 128-bar boundary stays.

### K5 — Progress bar render thrash

Progress events at 128-bar cadence on a 720-bar (Last30d) run
yield ~5 messages → 5 repaints over 8 s. Acceptable. But on a
TCN-overlay weights run (4344 bars), 34 messages → 34 repaints
over ~30 s. Each repaint touches the progress-bar canvas.

Mitigation:
- Repaint coalescing: progress-bar widget tracks last-rendered
  progress; render path skips if `(new - old) < 0.01`.
- ThrottledSpinner's existing 33 ms throttle pattern applies.

### K6 — Fills surfacing (R5.2) mutates `engine.rs:307` (etc.) byte-by-byte

Today `engine.rs:307` builds `RunReport { fills: Vec::new(), .. }`.
Populating `fills` from the scenario result does NOT mutate the
report body bytes (the disk-written report doesn't carry the fill
list verbatim — it carries a `MaxFills` summary at most). But if
the scenario modules' internal fill-recording paths are
non-deterministic (e.g. timestamp from `Instant::now()`), the
in-memory `FillView` carries non-determinism. Anchor is unaffected
(no body change); determinism tests fail.

Mitigation:
- Pre-flight: audit scenario modules' fill-recording paths for
  any `Instant::now()` / `SystemTime::now()` calls. Inject the
  scenario-driven bar timestamp instead.

### K7 — Wrapper-intercept makes `LabRunCompleted` arm unreachable in pure tests

If the wrapper consumes the message without forwarding, the pure
`state::update`'s `LabRunCompleted` arm never fires in live mode.
The pure test (`cargo test -p ui`) still exercises it because
those tests call `state::update` directly. But the binary-side
wrapper now needs its own test surface.

Mitigation:
- Wrapper is a free function in `cockpit_live.rs` taking
  `&mut Cockpit`. Unit-testable directly. Pattern follows the
  existing `select_pair`-capture handling.

### K8 — Progress channel + Recipe + tokio runtime

The progress Recipe (R7.3) is an `iced::Subscription` that polls a
tokio mpsc receiver. Wiring a tokio Receiver into iced's stream
adapter requires the same `rt_handle.enter()` pattern that the
`ServerTimeRecipe` panic fix introduced (commit `fix(cockpit_live):
ServerTimeRecipe panic`, 2026-05-19).

Mitigation:
- Reuse the `ServerTimeRecipe` shape verbatim; the recipe owns a
  `tokio::runtime::Handle` and enters it before stream construction.
- Unit test: construct the Recipe and call `.stream()`; assert no
  panic.

### K9 — Phase F panel snapshots mutate on Lab top-bar widget addition

R8 adds a progress-bar widget to the Lab top-bar row. The
`panel_snapshots__lab_*` snapshots in
`crates/ui/tests/panel_snapshots.rs` would then mutate (insta
diff). The 34 ANCHORS are unaffected, but the snapshot suite
is the UI's first regression gate.

Mitigation:
- Progress bar renders only when `lab_run_inflight == true`. Idle
  snapshots are byte-identical.
- New "lab_running" snapshot variant pinned via test fixture
  with `lab_run_inflight: true`.

### K10 — Two strategies in the new dispatch arms (R3) have config-file dependencies

`run_scenario`'s existing arms read `config/strategies/*.toml` at
runtime. The extracted SMA / MACD / RSI / BBands paths in
`main.rs` also read TOML configs. Tests that run from a non-root
CWD (`cargo test -p backtest --lib` runs from `crates/backtest`)
already hit this issue (engine.rs:674 `#[ignore]`).

Mitigation:
- Each new dispatch arm degrades gracefully: missing config →
  `Err(RunError::Internal("config not found at {path}"))`.
- Integration tests run from workspace root via the existing
  `cargo test -p backtest --test determinism` pattern.

## Non-regression contract

v2 MUST preserve:

1. **34/34 anchors byte-identical** — `scripts/verify_anchors.sh`
   EXIT 0 at M-FINAL. The 4 legacy single-symbol anchors (SMA /
   MACD / RSI / BBands), 8 cross-sectional anchors, and the 22
   newer scenario anchors all stay green.
2. **692 lib tests green** — `cargo test --workspace --lib` PASS.
3. **Phase F default-disabled byte-identity** — `RIGHT_RAIL_ENABLED
   = false` panel-snapshot diff = 0 vs current `main`.
4. **Cockpit idle-CPU ≤ 13.1 %** — 30 s idle measurement.
5. **`spec-lint` clean** — zero violations across the 9 enforced
   categories on the new feature folder.
6. **`cockpit-smoke` PASS** — boot fixtures cockpit 7 s, no panic
   grep.
7. **Phase B determinism tests green** — 6 tests at
   `engine.rs:546-735` continue to PASS (R6/R7 add fields; tests
   need `..Default::default()` updates but assertions unchanged).
8. **vendor/iced_tiny_skia patch untouched** — v2 does not touch
   `vendor/`; the operator-locked 2026-05-20 fork stays as-is.

## Routes (4-cell verdict × outcome)

Pre-drawn so the presenter can route the operator approval cleanly:

| Outcome →                                    | Operator approves                                                              | Operator rejects / approves-with-notes                                                                                                                       |
| -------------------------------------------- | ------------------------------------------------------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **L0 — All gaps closed; H1-H6 PASS**         | SHIP v0.1.0; trace row → `shipped`. Backlog Active block clears. | Approve-with-notes: log the notes against the presentation; route to relevant agent.                                                                       |
| **L1 — Chart still empty after success Run** | Route `HANDOFF → developer (binary-wrapper)` with `lab_run_e2e_completion` test failure + a `tracing::info` dump. Re-loop. | Same. The wrapper-intercept is the single most likely failure mode (F1 + K3). |
| **L2 — Run button stuck on Running**         | Route `HANDOFF → developer (subscription wiring)` with the H1 timing-out test attached. Check whether `LabRunCompleted` is dispatched at all (tracing span `lab.run.latency`). | Same. R4.1 is the diagnostic test for this cell. |
| **L3 — Progress bar misreports progress**    | Route `HANDOFF → developer (scenario emit-cadence)` with the H3 falsifying trace. Per K5, render-coalescing may be the culprit. | Same. The shimmer-stripe fallback at R8.3 should hide isolated misreports; if the operator sees a stuck bar at 70%, it's a scenario-side emit issue, not a widget issue. |

## References

- Predecessors:
  - [`spec/ui-rethink-phase-a-lab/feature.md`](../ui-rethink-phase-a-lab/feature.md)
    — Phase A R1-R11 (Lab screen + chart layers + chip widgets).
  - [`spec/ui-rethink-phase-b-lab-run/feature.md`](../ui-rethink-phase-b-lab-run/feature.md)
    — Phase B R1-R10 (Run button wiring + scenario-dispatch extraction).
- Architecture findings cited:
  - F1: `crates/ui/src/state.rs:1922-1931` (missing binary wrapper)
  - F2: `crates/ui/src/state.rs:1892-1897` (LabSelectPair gap)
  - F3: `crates/backtest/src/engine.rs:437-516` (cross-sectional-only dispatch)
  - F4: `crates/ui/src/bin/cockpit_live.rs:1027` (RunCancelHandle dropped)
  - F5: `crates/backtest/src/scenarios/momentum.rs` (no progress channel)
  - F6: `crates/ui/src/state.rs` markers wiring (chart_markers via audit, not RunReport.fills)
- Recent ServerTimeRecipe fix:
  commit `fix(cockpit_live): ServerTimeRecipe panic` (2026-05-19) —
  pattern v2 reuses for the progress Recipe (K8 mitigation).
- Vendor fork:
  [`vendor/iced_tiny_skia/`](../../vendor/iced_tiny_skia/) +
  [`spec/chart-fixture-line-clipping/feature.md`](../chart-fixture-line-clipping/feature.md)
  — untouched in v2.

## Trace

- `REQ-LAB-E2E-V2-001` — added at proposed state. `arch` / `crates`
  / `tests` / `anchors` filled by subsequent agents.

## Implementation

### Wave D-1 — wiring fixes (2026-05-24, developer)

**Root causes closed:**

- **F1 (binary wrapper missing)**: `cockpit_live::update` now intercepts
  `Message::LabRunCompleted(Ok(summary))` BEFORE forwarding to `state::update`.
  Captures the pre-forward `(strategy, pair, range)` tuple per K3.
  Post-forward: builds `RunReportMirror`, rotates `prev ← last; last ← new`.
  On `Err(...)`: no rotation (R2.3 / delta-badge correctness).
  Code at `crates/ui/src/bin/cockpit_live.rs:851-900`.

- **F2 (LabSelectPair does not set selected_symbol)**: One-line fix at
  `crates/ui/src/state.rs:1893` — `LabSelectPair` arm now assigns
  `model.selected_symbol = Some((venue, symbol.clone()))` before the pair
  move. The `select_pair` capture marker at `cockpit_live.rs:793-797` was
  also extended to match `LabSelectPair` so the markers/signals re-fetch
  cascade fires for Lab pair-chip clicks (R1.2).

- **Q6=(a) fixtures pre-loading**: `crates/ui/src/bin/cockpit.rs` now
  pre-loads synthetic bars for all 10 `XRP_FIRST_UNIVERSE` pairs (was
  3). Default selected symbol is BTCUSDT. (~12 KB cold memory; R1.4).

- **Q3=(a) RunSummary extension**: `RunSummary` gained `equity_series`,
  `fills`, and `kpis` fields. All 3 `RunSummary` construction sites
  updated. `BacktestKpis::default()` added at `engine.rs:122-132`.

**Tests added:**
- `crates/ui/src/state.rs::tests::lab_select_pair_updates_selected_symbol`
- `crates/ui/src/state.rs::tests::lab_select_pair_overwrites_selected_symbol_on_subsequent_click`
- `crates/ui/src/bin/cockpit_live.rs::tests::lab_run_completed_wrapper_rotates_mirror`
- `crates/ui/src/bin/cockpit_live.rs::tests::lab_run_completed_wrapper_rotates_prev_on_second_run`
- `crates/ui/tests/lab_run_integration.rs::lab_run_e2e_completion` (forensic-gate)
- `crates/ui/tests/lab_run_integration.rs::lab_run_completed_err_does_not_rotate_mirror`

**Gate results:**
- `cargo fmt --check`: PASS
- `cargo clippy --workspace --features candle,live -- -D warnings`: PASS
- `cargo test --workspace --lib`: 1070 passed, 0 failed
- `bash scripts/verify_anchors.sh`: ANCHORS PASS (34 / 34)

## Changelog

- 2026-05-24 (analyst): initial v0.1.0 brief; closes the
  verification-walk gap table; surfaces 8 operator-decide Qs
  with analyst-recommended defaults.
- 2026-05-24 (developer): Wave D-1 complete — F1 + F2 + Q6 + R4 integration
  test. "Running indefinitely + no chart" regression closed for
  cross-sectional strategies.
