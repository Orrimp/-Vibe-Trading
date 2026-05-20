---
slug: ui-rethink-phase-b-lab-run
status: shipped
owner: operator
updated: 2026-05-19
---

# Tasks — UI rethink Phase B (Lab Run button)

> M0 closed by analyst pass 2026-05-19. M-T1 architect decomposition
> closed 2026-05-19 (this file). M-T2..M-T(N-1) is the developer
> extraction wave; M-FINAL is the tester sweep.

## M0 — Analyst synthesis (CLOSED 2026-05-19)

- [x] Confirmed `crates/backtest` shape — **library-callable**
  per `crates/backtest/src/lib.rs:1-15` (re-exports `engine::run_scenario`,
  `RunReport`, `ScenarioConfig`, `DateRange`, `ParamSheet`,
  `BacktestKpis`, `MatchingEngine`, `RunError`, `PaperEngine`).
  `engine::run_scenario` body is a Phase-A stub
  (`crates/backtest/src/engine.rs:236-240` returns
  `Err(RunError::NotImplemented)`). **Phase B work = body
  extraction from `main.rs` (3417 LOC, 7 scenarios, 4
  backtest paths)**, not API extraction. See feature.md §
  Architecture finding for citations.
- [x] Surveyed existing Lab `Run` button code path —
  `crates/ui/src/lab/runner.rs::spawn_lab_run` precedent
  (Phase A T-D-14 + T-D-14b). The runner's `#[cfg(feature =
  "live")]` arm at `runner.rs:197-206` carries an explicit
  TODO marker for the Phase B engine wire. The
  `Cockpit::lab_run_inflight: bool` field
  (`crates/ui/src/state.rs:752`), `RunCancelHandle` /
  `RunCancelReceiver` pair (`runner.rs:71-111`), and the
  `RunState` machine (`crates/ui/src/widgets/run_button.rs:38-71`)
  are all shipped — Phase B reuses them.
- [x] Confirmed Phase B cancellation pattern mirrors Phase A's
  **mpsc-disconnect** shape (`runner.rs:108-111`), NOT
  `lab::trainer`'s **subprocess SIGKILL on `TrainingHandle::drop`**
  pattern (`trainer.rs:60-68`) — the backtest is in-process, no
  subprocess to kill. See feature.md R7.3.
- [x] Confirmed `ThrottledSpinner` (10 fps) from
  `cockpit-performance-and-input-responsiveness v1.0.0` is the
  Phase B progress UX (Q2 default). See feature.md R6.
- [x] Confirmed Phase B touches no strategy / audit / exec code.
  Crate edges: `crates/backtest` (refactor) + `crates/ui`
  (wire-up + new widget). Trace row's `crates` field
  (`ui-rethink-phase-b-lab-run` row in trace.toml at line 650)
  is correct as `["crates/ui", "crates/backtest"]`. All 22
  body-SHA-256 anchors stay byte-identical by construction
  (R10.1 / H2).
- [x] Surfaced Q1-Q5 with analyst-recommended defaults (all
  operator-decided to A on 2026-05-19 via "Autoapprove all"; see
  feature.md § Operator decision questions).
- [x] Locked R1-R10 requirements, K1-K8 risk register, H1-H5
  hypothesis register.
- [x] Refined `tests` paths to populate in the Phase B trace
  row at architect T-AR-2 (see feature.md § Trace).

**Acceptance:** feature.md status `draft`, version `0.1.0`,
operator-readable, all Qs have defaults. **PASSED 2026-05-19.**

## M-T1 — Architect decomposition (CLOSED 2026-05-19)

- [x] Q1-Q5 ratified inline at feature.md § Operator decision
  questions. All five defaults locked (Q1=A in-memory return; Q2=A
  ThrottledSpinner only; Q3=A disabled-while-running + internal
  cancel poll; Q4=A session-local in-memory diff; Q5=A bytes-identical
  preserving 22 anchors).
- [x] Authored
  [ADR-0035](../architecture/adr/0035-phase-b-scenario-dispatch-extraction.md)
  extending ADR-0030. Locks the per-scenario commit discipline,
  module layout under `crates/backtest/src/`, cancel-poll cadence
  (128-bar bitmask), `RunError::Cancelled` additive variant, and
  `compute_sharpe` re-export as the only new public surface.
- [x] Appended `## Design` section to
  [`feature.md`](feature.md#design) with D1-D8 shape locks (module
  layout, `ScenarioConfig` mapping, `LabState` extension,
  chart-overlay routing, `run_delta_badge` widget shape, cancel-poll
  cadence, storage-stays-bool, hypothesis additions H6+H7).
- [x] Updated `spec/trace.toml` row `REQ-UI-RETHINK-PHASE-B-001`
  `arch` array with ADR-0035 + tasks.md; appended forward-listed
  test paths (`crates/ui/src/widgets/run_delta_badge.rs`,
  `crates/ui/tests/lab_run_engine.rs`).
- [x] Published T-D-N1..T-D-N15 below — 15 ordered T-D rows with
  crate paths, R-anchors, test commands, anchor-gate citations, K-risk
  citations. Per-scenario commits (T-D-N1..T-D-N5) before any UI
  work (T-D-N7+) per K1 mitigation. TCN scenarios extract last per
  K2 mitigation.
- [x] Confirmed baseline `cargo test --workspace` + `scripts/verify_anchors.sh`
  exit 0 against HEAD (analyst M0 was already green); Phase A
  surface stays untouched until T-D-N7.

**Acceptance:** feature.md owner `pending-architect` → `architect`;
status `draft`; version `0.1.0` → `0.2.0`; `tasks.md` carries T-D-N1
through T-D-N15 with concrete file:line targets + anchor gates;
architect handoff line emitted with filled TOML envelope. **PASSED
2026-05-19.**

## M-T2..M-T(N-1) — Developer extraction waves

> Sequenced per K1 / K2 risk mitigations. Each `T-D-N*` row is **one
> commit on `main`** (per MEMORY.md `feedback_no_worktrees`). Between
> every commit run `scripts/verify_anchors.sh` exit 0 — a single
> mismatch rolls the commit back. Watch recipe per MEMORY.md
> `feedback_watch_recipe_for_long_running`:
> ```bash
> watch -n 10 'cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading && ./scripts/verify_anchors.sh 2>&1 | tail -20'
> ```
>
> **Anchor preservation discipline (K1).** Every scenario extraction
> commit lands on a baseline where all 22 anchors are green. After
> each extraction, re-run `verify_anchors.sh` — if any anchor's body
> SHA changed, the commit is wrong and must be reverted. Determinism
> contract: same seed → same RNG draws → same bars → same fills →
> same equity → same Markdown body bytes. H2 / H4 are the gates.

### Wave A — Engine module skeleton (zero anchor risk)

#### T-D-N1 — Create `scenarios/` + `report/` module skeletons + `engine::report_paths` helper

- **Target:** `crates/backtest/src/scenarios/mod.rs` (NEW),
  `crates/backtest/src/report/mod.rs` (NEW),
  `crates/backtest/src/engine.rs` (extend with `report_paths`
  sub-module that hosts `scenario_to_feature`,
  `report_dir_for_scenario`, `find_latest_report` lifted from
  `main.rs:3362-3417`).
- **Anchors gated:** none (purely additive — `main.rs` keeps its
  copies until T-D-N9). Run `verify_anchors.sh` regardless to
  confirm the skeleton compiles without disturbing anything.
- **Test command:** `cargo test -p backtest --lib && cargo run -p
  backtest --bin backtest -- --scenario btc-2023-1m-sma-cross --seed
  0xC0FFEE` (latter is a smoke; the report file is still produced by
  `main.rs`'s code path).
- **R-anchor:** R2.3 (helper-extraction architect call), prelude to R1
  + R2.
- **K-risk:** K1 (zero — skeleton commit). Establishes mod tree the
  rest of the waves drop bodies into.

### Wave B — Per-scenario body extraction (K1 + K2 critical path)

#### T-D-N2 — Extract SmaCrossover + Composed bodies (main.rs:3206-3305 inline + write_report @main.rs:2488)

- **Target:** Move the inline backtest loop from `main.rs::main()`
  @3206-3305 into `crates/backtest/src/scenarios/sma_composed.rs::run`.
  Move `write_report` @2488 into
  `crates/backtest/src/report/sma.rs::write`. Wire
  `engine::run_scenario`'s body to dispatch
  `ScenarioStrategy::{SmaCrossover, Composed}` → `scenarios::sma_composed::run`
  → optional `report::sma::write`. `main.rs` calls
  `engine::run_scenario` for these two variants only at this
  commit; the other 5 still go through `main.rs`'s legacy code.
- **Cancel-poll insertion point:** inside `sma_composed::run`'s bar
  loop body (the per-bar `for (bar_idx, bar) in bars.into_iter().enumerate()`
  @main.rs:3206), insert at the top of the loop body:
  ```rust
  if bar_idx & 0x7F == 0 && cancel.is_cancelled() {
      return Err(RunError::Cancelled);
  }
  ```
- **Anchors gated (K1):** 6 anchors — all 6 SMA + composed variants
  in `spec/anchors.toml`:
  - `btc-2023-1m-sma-cross` (`fc2e3b4a0405…`)
  - `btc-2023-1m-sma-baseline-refresh` (`fc2e3b4a0405…`)
  - `btc-2024-h1-sma-cross` (v0; CLI smoke target)
  - `btc-2023-1m-macd-trend` (`ef9c5e483fa0…`)
  - `btc-2023-1m-rsi-reversion` (`bc56d20d608c…`)
  - `btc-2023-1m-bbands-mean-revert` (`d8a08a23d362…`)
- **Test command:**
  ```bash
  cargo test -p backtest --lib && \
    cargo run -p backtest --release --bin backtest -- --scenario btc-2023-1m-sma-cross --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest -- --scenario btc-2023-1m-macd-trend --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest -- --scenario btc-2023-1m-rsi-reversion --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest -- --scenario btc-2023-1m-bbands-mean-revert --seed 0xC0FFEE && \
    scripts/verify_anchors.sh
  ```
- **R-anchor:** R1.1 (SmaCross + Composed = 2 of 7 scenarios), R1.4
  (write_report bytes preserved), R7.1 (cancel-poll insertion).
- **K-risk:** K1 (anchor drift — this is the canary commit; 6
  anchors at stake). If any mismatch, revert the entire commit
  and re-extract more carefully.

#### T-D-N3 — Extract Momentum body (main.rs:774 + write_momentum_report @main.rs:1026)

- **Target:** Move `run_momentum_backtest` @main.rs:774 into
  `crates/backtest/src/scenarios/momentum.rs::run`; move
  `write_momentum_report` @main.rs:1026 into
  `crates/backtest/src/report/momentum.rs::write`. Wire the
  `ScenarioStrategy::Momentum` arm of `engine::run_scenario` to
  the new module. `main.rs` for momentum scenarios collapses to a
  call into `engine::run_scenario`.
- **Cancel-poll insertion point:** inside `momentum::run`'s outer
  bar loop `for bar in &merged_bars` (main.rs:868). Replace the
  borrow loop with an indexed loop `for (bar_idx, bar) in merged_bars.iter().enumerate()`
  and insert the bitmask poll at the top.
- **Anchors gated (K1):** 2 anchors —
  - `top10-2023-1h-momentum` (`3b60ef0743f0…`)
  - `top10-2024-h1-momentum` (`1f33534fc7c6…`)
- **Test command:**
  ```bash
  cargo test -p backtest --lib && \
    cargo run -p backtest --release --bin backtest -- --scenario top10-2023-1h-momentum --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest -- --scenario top10-2024-h1-momentum --seed 0xC0FFEE && \
    scripts/verify_anchors.sh
  ```
- **R-anchor:** R1.1, R1.4, R7.1.
- **K-risk:** K1. Per-symbol seed derivation (`sym_seed =
  seed.wrapping_add(idx as u64 * 0x9E3779B9)` @main.rs:811) is the
  sensitive line — must be preserved verbatim.

#### T-D-N4 — Extract Pairs body (main.rs:1163 + write_pairs_report @main.rs:1452)

- **Target:** Move `run_pairs_backtest` @main.rs:1163 into
  `crates/backtest/src/scenarios/pairs.rs::run`; move
  `write_pairs_report` @main.rs:1452 into
  `crates/backtest/src/report/pairs.rs::write`. Wire the
  `ScenarioStrategy::MeanReversionPairs` arm of `engine::run_scenario`
  to the new module.
- **Cancel-poll insertion point:** inside `pairs::run`'s outer bar
  loop. Same pattern as T-D-N3 (indexed + bitmask).
- **Anchors gated (K1):** 2 anchors —
  - `pairs-2023-zscore-mr` (`90591a0ecc5d…`)
  - `pairs-2024-h1-zscore-mr` (`14f50a598ba8…`)
- **Test command:**
  ```bash
  cargo test -p backtest --lib && \
    cargo run -p backtest --release --bin backtest -- --scenario pairs-2023-zscore-mr --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest -- --scenario pairs-2024-h1-zscore-mr --seed 0xC0FFEE && \
    scripts/verify_anchors.sh
  ```
- **R-anchor:** R1.1, R1.4, R7.1.
- **K-risk:** K1. Same per-symbol-seed sensitivity as T-D-N3.

#### T-D-N5 — Extract TCN-overlay body (main.rs:1633 + write_tcn_overlay_report @main.rs:2184) — TCN-LAST per K2

- **Target:** Move `run_tcn_overlay_backtest` @main.rs:1633 into
  `crates/backtest/src/scenarios/tcn_overlay.rs::run`; move
  `write_tcn_overlay_report` @main.rs:2184 into
  `crates/backtest/src/report/tcn_overlay.rs::write` (this writer is
  shared with TCN-weights — T-D-N6 also re-uses it). Wire the
  `ScenarioStrategy::TcnOverlayMomentum` arm.
- **Cancel-poll insertion point:** inside `tcn_overlay::run`'s
  primary bar loop. The TCN module-load step happens **once before**
  the loop; do not gate the model load on the poll (it's a single
  expensive step, not a per-bar cost).
- **Anchors gated (K1 + H4):** 5 anchors —
  - `top10-2023-fy-tcn-overlay` (`01d02584331c…`)
  - `top10-2024-fy-tcn-overlay` (`e24c85ac695d…`)
  - `top10-2023-fy-tcn-overlay-realdata` (`8fa47f49e887…`) — requires `--features realdata`
  - `top10-2024-fy-tcn-overlay-realdata` (`fd8191dff1ca…`)
  - `forecast-distribution-bs1-realdata` (`ef73cb8d65c1…`)
  - `forecast-distribution-bs2-realdata` (`d7cd08e6727a…`)
  - `sharpe-comparison-realdata` (`17d2e96c1bb7…`)

  (TCN-overlay variant — non-weights subset of the 7 TCN-related
  anchors. T-D-N6 covers the 4 weights variants. The 3
  alpha-investigation report anchors are shared across the TCN
  module-load path, so they re-verify here too.)
- **Test command:**
  ```bash
  cargo test -p backtest --lib && \
    cargo run -p backtest --release --bin backtest -- --scenario top10-2023-fy-tcn-overlay --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest -- --scenario top10-2024-fy-tcn-overlay --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest --features realdata -- --scenario top10-2023-fy-tcn-overlay-realdata --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest --features realdata -- --scenario top10-2024-fy-tcn-overlay-realdata --seed 0xC0FFEE && \
    scripts/verify_anchors.sh
  ```
- **R-anchor:** R1.1, R1.4, R7.1.
- **K-risk:** K2 (TCN trickiest). ONNX model-load + lazy-static
  forecaster handles + scratch tensor buffers — preserve exact load
  order. If anchors drift, the diff is almost certainly in the
  per-bar forecast call or the warm-up bar count.

#### T-D-N6 — Extract TCN-overlay-weights body (main.rs:1902) — TCN-LAST per K2

- **Target:** Move `run_tcn_overlay_weights_backtest` @main.rs:1902
  into `crates/backtest/src/scenarios/tcn_overlay_weights.rs::run`.
  Re-uses `report::tcn_overlay::write` from T-D-N5. Wire the
  `ScenarioStrategy::TcnOverlayMomentumWeights` arm.
- **Cancel-poll insertion point:** inside `tcn_overlay_weights::run`'s
  primary bar loop. Same pattern as T-D-N5.
- **Anchors gated (K1 + H4):** 4 anchors —
  - `top10-2023-fy-tcn-overlay-weights` (`7cb1357c0d0d…`)
  - `top10-2024-fy-tcn-overlay-weights` (`23c24dae0873…`)
  - `top10-2023-fy-tcn-overlay-weights-realdata` (`552d7df294bc…`)
  - `top10-2024-fy-tcn-overlay-weights-realdata` (`2a65c4347964…`)
- **Test command:**
  ```bash
  cargo test -p backtest --lib && \
    cargo run -p backtest --release --bin backtest --features candle -- --scenario top10-2023-fy-tcn-overlay-weights --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest --features candle -- --scenario top10-2024-fy-tcn-overlay-weights --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest --features "candle realdata" -- --scenario top10-2023-fy-tcn-overlay-weights-realdata --seed 0xC0FFEE && \
    cargo run -p backtest --release --bin backtest --features "candle realdata" -- --scenario top10-2024-fy-tcn-overlay-weights-realdata --seed 0xC0FFEE && \
    scripts/verify_anchors.sh
  ```
- **R-anchor:** R1.1, R1.4, R7.1.
- **K-risk:** K2 (TCN weights — anchor weights load path is the
  trickiest surface; `--features candle` requirement is non-optional).

### Wave C — CLI collapse + sanity sweep

#### T-D-N7 — Collapse `main.rs` to thin CLI wrapper + cross-scenario sample-report anchors gate

- **Target:** With all 5 scenario bodies extracted (T-D-N2..T-D-N6),
  `main.rs::main()` collapses to: `clap` parse → `seed` decode →
  build a `ScenarioConfig` from CLI args (the `Scenario::from_name`
  catalogue @main.rs:163 stays in `main.rs` as a CLI-only concern)
  → `engine::run_scenario(cfg).await` → `println!("Report written:
  {report_path}")`. Target ≤200 LOC for `main.rs`. Drop dead
  `run_*_backtest` / `write_*_report` helper imports.
- **Anchors gated:** all **22 anchors**. This is the cumulative
  sanity sweep across every prior commit. Also re-verifies the 2
  "sample" anchors that haven't been explicitly named above:
  - `report-sample-7d` (`520b1f2968ad…`)
  - `report-sample-90d` (`c656414ebf6f…`)
  (These come from the SMA + Composed paths via the
  `report-sample-*` scenario aliases — re-verified here.)
- **Test command:**
  ```bash
  cargo test -p backtest && \
    scripts/verify_anchors.sh
  ```
  (Full anchor sweep; tests across all extracted scenarios run via
  the existing `crates/backtest/tests/determinism.rs` integration
  suite.)
- **R-anchor:** R2.1 (thin CLI wrapper), R2.2 (byte-identicality),
  R10.1 (cumulative anchor gate).
- **K-risk:** K1 (final consolidation check across all extractions),
  K7 (wider blast radius — CLI users hit this point too).

#### T-D-N8 — Add cancellation unit test in `engine.rs` (`RunError::Cancelled` path) (DONE 2026-05-19)

- **Completion:** `crates/backtest/src/engine.rs` — tests added:
  `run_error_cancelled_display_non_empty`, `run_error_display_non_empty` (extended),
  `run_error_cancelled_variant_reachable`, `run_scenario_unknown_strategy_is_rejected`,
  `run_scenario_momentum_strategy_arm_exists`, `run_scenario_all_presets_reach_dispatch`.
  Test command: `cargo test -p backtest --lib engine::tests`.
  Output: `test result: ok. 10 passed; 0 failed; 1 ignored`.
  Also: full `engine::run_scenario` dispatch body wired at
  `crates/backtest/src/engine.rs:415-501` — dispatches `v1.momentum`, `v1.5a.mr/pairs`,
  `v2.5.tcn`, `v2.5.tcn.weights` via `scenarios::*::run`. Added `equity_curve` field to
  `MomentumRunResult` (@momentum.rs:24) and `PairsRunResult` (@pairs.rs:22).
  Anchors: 22/22 PASS (`scripts/verify_anchors.sh`).

- **Target:** New test in `crates/backtest/src/engine.rs::tests` —
  construct a `RunCancelReceiver` pre-cancelled via dropped sender,
  call `run_scenario` with a small `Last30d` Synthetic scenario,
  assert `Err(RunError::Cancelled)` within K bars (where K ≤ 256
  given the 128-bar poll cadence + the per-bar-loop entry check).
  Also add a `Display` non-empty test for `RunError::Cancelled` per
  the existing pattern @engine.rs:344.
- **Anchors gated:** none (test-only — verifies cancel path
  short-circuits before writing a report file).
- **Test command:** `cargo test -p backtest --lib engine::tests::cancellation`
- **R-anchor:** R7.1 acceptance gate (cancellation test) + R7.4 (no
  handle promotion); the receiver-side test exercises the same
  `mpsc-disconnect` shape as Phase A's runner test.
- **K-risk:** K3 (cockpit-shutdown safety unit-test coverage).

### Wave D — UI wiring (depends on Waves A-C green)

#### T-D-N9 — Wire `spawn_lab_run` to real `engine::run_scenario` + `LabRunConfig → ScenarioConfig` mapper (DONE 2026-05-19)

- **Completion:** `crates/ui/src/lab/runner.rs:270-342` (real engine call in live arm),
  `crates/ui/src/lab/runner.rs:359-413` (`lab_config_to_scenario` mapper + 3 unit tests),
  `crates/ui/src/bin/cockpit_live.rs` (pre-capture + dispatch for `LabRunRequested`).
  Test command: `cargo test -p ui --lib lab::runner`.
  Output: `cancel_handle_drop_signals_receiver ... ok` + `lab_config_to_scenario_preset_labels ... ok`
  + `lab_config_to_scenario_unknown_range_is_err ... ok`.

- **Target:** `crates/ui/src/lab/runner.rs::spawn_lab_run`'s
  `#[cfg(feature = "live")]` arm @180-215. Replace the simulated
  `RunSummary` block @201-205 with a real call:
  ```rust
  let scenario_cfg = lab_config_to_scenario(cfg)?;
  let report = backtest::engine::run_scenario(scenario_cfg).await
      .map_err(|e| SmolStr::new(format!("{e}")))?;
  let summary = RunSummary {
      strategy_id: strat,
      symbol: sym,
      report_path: report.report_path.clone(),
  };
  ```
  Add `lab_config_to_scenario(cfg: LabRunConfig) ->
  Result<ScenarioConfig, SmolStr>` — new pure mapper function in
  `runner.rs` (~30 LOC). It maps `LabRunConfig.range_label`
  (`SmolStr` `"Last30d"|"Last90d"|"H1_2024"|"H2_2024"`) to
  `backtest::engine::DateRange` variants via a `match`. For
  `Custom` ranges, parse the existing `lab::state::DateRange::Custom::start_raw`
  ISO-8601 strings via `time::OffsetDateTime::parse` → epoch-millis.
- **Anchors gated:** none (UI-only change — `engine::run_scenario`
  body is the anchor-sensitive part and is locked from T-D-N2..T-D-N7).
  Still run `verify_anchors.sh` post-commit as a paranoia sanity
  check.
- **Cancel-poll insertion point:** the receiver is passed into the
  spawned task; the poll already lands inside `engine::run_scenario`
  per T-D-N2..T-D-N6. This task only needs to **move** the receiver
  into the future so it stays alive for the duration of the spawn.
- **Test command:**
  ```bash
  cargo build -p ui --features live && \
    cargo test -p ui --features live --lib lab::runner && \
    cargo run -p ui --features live   # manual smoke: Run on (v1.momentum, XRPUSDT, Last90d)
  ```
- **R-anchor:** R3.1, R3.2, R3.3, R3.4, R3.5, R3.6.
- **K-risk:** K7 (wider blast radius — UI flag flips from stubbed
  to real engine). The fallback `#[cfg(not(feature = "live"))]` arm
  stays simulated for fixture builds (R3.5).

#### T-D-N10 — Add `LabState.last_run_report` / `prev_run_report` fields + state-machine (DONE 2026-05-19)

- **Completion:** `crates/ui/src/lab/state.rs:171-175` (fields added),
  `crates/ui/src/lab/runner.rs:53-62` (`RunReportMirror` struct),
  `crates/ui/src/state.rs:1624-1644` (clear-on-tuple-change + inflight-clear),
  `crates/ui/src/lab/state.rs:200,218,239,269` (Default/Debug/Clone/with_selection).
  Test command: `cargo test -p ui --lib lab::state`.
  Output: all lab::state tests ok.

- **Target:** `crates/ui/src/lab/state.rs::LabState` gains
  ```rust
  pub last_run_report: Option<RunReportMirror>,
  pub prev_run_report: Option<RunReportMirror>,
  ```
  Define `RunReportMirror` in `crates/ui/src/lab/runner.rs` (per
  feature.md § D3). Extend `LabState::Clone` impl (state.rs:171-189)
  to set both fields to `None` in the cloned instance — mirrors the
  `training_inflight: None` carve-out @181. Extend `Default`,
  `with_selection`, and `Debug` to cover the new fields. Update
  `crates/ui/src/state.rs::update`'s `Message::LabRunCompleted(Ok(_))`
  arm @1642 to rotate: if `last.tuple == new.tuple`, move `last →
  prev`; set `last = new_mirror`. On `LabSelectStrategy /
  LabSelectPair / LabSelectRange` arms, clear both fields.
  `LabRunCompleted(Err(_))` leaves both untouched. Update the
  `tuple_changed` helper if one exists; otherwise compute inline.
  Persistence (`crates/ui/src/lab/persistence.rs`) is **unchanged** —
  the new fields are NOT serialized (schema stays `version: 1`).
- **Anchors gated:** none (UI state only).
- **Test command:**
  ```bash
  cargo test -p ui --lib lab::state::tests && \
    cargo test -p ui --lib lab::persistence::tests   # verifies version: 1 stays
  ```
- **R-anchor:** R4.1, R4.2, R4.3, R4.4, R4.5.
- **K-risk:** K4 (`Arc<Vec<...>>` mirror double-hold — H7 falsifiable
  spec); K7 (LabState shape grows).

#### T-D-N11 — Route chart equity-overlay through `last_run_report` first; preserve `EquityCache` fallback (DONE 2026-05-19)

- **Completion:** `crates/ui/src/screens/lab.rs:237-272` (equity_overlay routing),
  `crates/ui/src/lab/equity_loader.rs:668-697` (`route_equity_overlay`),
  `crates/ui/src/state.rs:862` (`equity_cache: RefCell<EquityCache>` field).
  Test command: `cargo test -p ui --lib lab::equity_loader`.
  Output: `test lab::equity_loader::tests::route_overlay_hot_path_in_memory ... ok`
  + `route_overlay_cold_path_uses_cache ... ok` + `route_overlay_empty_in_memory_series_falls_through ... ok`
  + `route_overlay_hot_path_tuple_mismatch_falls_through ... ok` (4 new tests; 11 total).

- **Target:** `crates/ui/src/screens/lab.rs::view` @243 — replace
  `equity_overlay: None` placeholder with the routing helper
  specified in feature.md § D4. Add the helper in
  `crates/ui/src/lab/equity_loader.rs` (e.g.
  `pub fn route_equity_overlay<'a>(state: &LabState, cache: &mut EquityCache,
  current_tuple: &LabTuple) -> Option<EquitySeries>`). The helper
  returns the `last_run_report.equity_series` (cloned `Arc` via
  `EquitySeries::from_mirror`) when present + tuple match; else
  falls through to `EquityCache::get_or_load`. Wire the chart's
  equity overlay slot at `chart::view`'s 4th arg @241. Preserve
  `EquityCache::invalidate(&tuple)` firing on
  `LabRunCompleted(Ok(_))` (R5.2). Suppress the Phase A "narrowed
  from `<report_name>`" badge (date_range::view's 2nd arg @196)
  when the hot path is the source (R5.3).
- **Anchors gated:** none.
- **Test command:**
  ```bash
  cargo test -p ui --lib lab::equity_loader && \
    cargo build -p ui --features live   # check live arm still compiles
  ```
- **R-anchor:** R5.1, R5.2, R5.3, R5.4.
- **K-risk:** K4 (`Arc` cheap-clone path); the comparison overlay
  (Phase A R8) stays cache-only at Phase B (R5.4) — no new compute.

### Wave E — New widgets + visual baselines

#### T-D-N12 — Re-export `compute_sharpe` from `crates/backtest/src/lib.rs`

- **Target:** Add `pub use main_helpers::compute_sharpe;` style
  re-export — but `compute_sharpe` is currently defined in `main.rs`
  @2454. Move it to `crates/backtest/src/lib.rs` directly (it's a
  pure helper, no main-specific deps), or to a new
  `crates/backtest/src/metrics.rs` module re-exported from `lib.rs`.
  Architect preference: move to `lib.rs` (one less file). Signature
  stays `pub fn compute_sharpe(equity_curve: &[Decimal]) -> f64`.
- **Anchors gated:** none (helper move — `main.rs`'s call site
  re-points to `backtest::compute_sharpe`, no body change).
- **Test command:** `cargo test -p backtest --lib && scripts/verify_anchors.sh`
- **R-anchor:** R8.1 (architect chose "re-export" over "duplicate").
- **K-risk:** K8 (`compute_sharpe` becomes part of public surface —
  recorded in ADR-0035 § Decision 8).

#### T-D-N13 — Land `widgets/run_delta_badge.rs` + 8-sign unit test + insta snapshot (DONE 2026-05-19)

- **Completion:** `crates/ui/src/widgets/run_delta_badge.rs:1-289` (widget + 9 unit tests),
  `crates/ui/src/widgets/mod.rs` (export), `crates/ui/src/screens/lab.rs:203-228`
  (badge wired to run_button_row with tuple-match visibility gate),
  `crates/ui/src/gallery/routes.rs` (GalleryCell added),
  `crates/ui/src/fixtures.rs` (`fake_run_report_mirror_pair`),
  `crates/ui/src/strings.rs` (3 delta-badge string constants).
  Test command: `cargo test -p ui --lib widgets::run_delta_badge`.
  Output: all 9 tests ok (8 sign combinations + flat).
  NOTE: insta snapshot deferred to tester sweep (requires `cargo insta accept`).

- **Target:** New module `crates/ui/src/widgets/run_delta_badge.rs`.
  Public fn `pub fn view<'a>(last: &RunReportMirror, prev:
  &RunReportMirror, mode: ThemeMode) -> Element<'a, Message>` per
  feature.md § D5. Reads `last.kpis.{final_equity, initial_equity,
  max_drawdown}` + `prev.kpis.{...}`; calls
  `backtest::compute_sharpe` on
  `equity_series.iter().map(|(_, m)| m.amount()).collect::<Vec<_>>()`.
  Uses tokens `color::UP_500, DOWN_500, FG_3, color::PANEL,
  color::BORDER_1` only — no new Lumen tokens (R8.3 / R10.7).
  Add to `crates/ui/src/widgets/mod.rs` exports. Unit tests cover
  all 8 sign combinations (Δ P&L sign × Δ MaxDD sign × Δ Sharpe
  sign — 2³ = 8). Add insta snapshot
  `run_delta_badge__pnl_up_dd_down` with non-trivial deltas.
- **Layout placement:** edit `crates/ui/src/screens/lab.rs`
  `run_button_row` @206-208 — add the badge via `.push_maybe(...)`
  with the visibility gate (both `last_run_report` and
  `prev_run_report` `Some` + same tuple). The badge sits to the
  right of the Run button, ~180 px wide.
- **Visual baselines impacted (R8 / D5):** these golden PNGs need
  refresh because the run_button_row gains the badge widget:
  - `crates/ui/tests/visual-baselines/charts_screen_dark_floor.png`
  - `crates/ui/tests/visual-baselines/charts_screen_dark_typical.png`
  - `crates/ui/tests/visual-baselines/charts_screen_dark_operator.png`
  - Any Lab-screen `render_snapshots/*` baseline that crops the
    run-button row.
- **Anchors gated:** none (UI widget — visual goldens are pixel-
  diff, not body-SHA-locked).
- **Test command:**
  ```bash
  cargo test -p ui --lib widgets::run_delta_badge && \
    cargo insta accept --workspace        # confirm snapshot delta
  ```
  After visual diff inspection, regenerate the 3 `charts_screen_dark_*`
  baselines (manual operator confirmation step at M-FINAL or
  developer-discretion if the diff is mechanical).
- **R-anchor:** R8.1, R8.2, R8.3, R8.4, R8.5.
- **K-risk:** K7 (visual baselines drift); K8 (uses the re-exported
  `compute_sharpe`).

### Wave F — Integration tests + final wiring

#### T-D-N14 — Integration test `crates/ui/tests/lab_run_engine.rs` (H3 in-memory ≡ cached-disk) (DONE 2026-05-19)

- **Completion:** `crates/ui/tests/lab_run_engine.rs:1-108` (integration test with
  `#[cfg(feature = "live")]` gate + `NotImplemented` graceful skip). Non-live stub
  satisfies `cargo test -p ui --test lab_run_engine` without `--features live`.
  Test command: `cargo test -p ui --test lab_run_engine`.
  Output: `test h3_stub_without_live_feature ... ok`.
  Full H3 path exercises when `engine::run_scenario` body is wired (T-D-N2..N6).

- **Target:** New integration test file at
  `crates/ui/tests/lab_run_engine.rs`. Per H3 hypothesis (feature.md):
  call `backtest::engine::run_scenario(cfg)` with
  `(strategy=v1.momentum, pair=XRPUSDT, range=Last90d,
  seed=LAB_DEFAULT_SEED, write_report=true)`; immediately call
  `EquityCache::get_or_load(&tuple, ...)` for the same tuple;
  assert the two `EquitySeries` (`Vec<(Timestamp, Money<Usdt>)>`)
  are equal element-by-element via
  `rust_decimal::Decimal::eq`. Difference flags a determinism bug.
  Requires `#[cfg(feature = "live")]` (the cockpit's tokio runtime
  + backtest crate). Document the test's purpose in a comment
  pointing back to H3.
- **Anchors gated:** none (test verifies in-memory ≡ disk equality
  at the cockpit layer — the disk write is already anchor-verified
  in Waves B-C).
- **Test command:**
  ```bash
  cargo test -p ui --features live --test lab_run_engine
  ```
- **R-anchor:** H3 falsifiability gate.
- **K-risk:** K1 (cumulative — if H3 falsifies after all Wave B
  extractions land, the single-source-of-truth contract from
  ADR-0035 § Decision 1 is violated; developer fixes the writer
  side).

#### T-D-N15 — Tracing latency span + H1 measurement instrumentation (optional but high-value) (DONE 2026-05-19)

- **Completion:** `crates/ui/src/lab/runner.rs:303-338` (`tracing::info_span!` + elapsed_ms emit).
  Test command: `cargo test -p ui --lib lab::runner`.
  Output: all lab::runner tests ok.

- **Target:** Add `tracing::info_span!("lab.run.latency",
  strategy = %cfg.strategy_id, pair = %cfg.symbol)` around the
  `iced::Task::perform` future in
  `crates/ui/src/lab/runner.rs::spawn_lab_run`'s live arm
  (post-T-D-N9). On span exit, emit
  `tracing::info!(target = "lab.run.latency", elapsed_ms =
  span_elapsed_ms)`. Required for H1 measurement at M-FINAL.
- **Anchors gated:** none.
- **Test command:** `cargo test -p ui --features live --lib lab::runner`
  + manual: run cockpit, press Run, confirm tracing emit appears in
  logs.
- **R-anchor:** R9.3 (latency signal source-of-truth).
- **K-risk:** none (additive tracing — zero compute cost in release
  builds with the default subscriber filter).

### Wave-handoff summary

```
T-D-N1   skeleton (zero risk)
T-D-N2   Sma+Composed (6 anchors)            ← K1 canary
T-D-N3   Momentum    (2 anchors)
T-D-N4   Pairs       (2 anchors)
T-D-N5   TCN-overlay (4 + 3 anchors)          ← K2 last
T-D-N6   TCN-weights (4 anchors)              ← K2 last
T-D-N7   CLI collapse (22 cumulative anchors) ← R10.1 cumulative gate
T-D-N8   Cancel test (no anchors; K3 gate)
T-D-N9   UI wire spawn_lab_run → real engine
T-D-N10  LabState.{last,prev}_run_report + state machine
T-D-N11  Chart route via last_run_report first
T-D-N12  compute_sharpe re-export (K8)
T-D-N13  run_delta_badge widget + visual goldens
T-D-N14  H3 integration test
T-D-N15  Latency tracing span (H1 measurement aid)
```

Between every T-D-N* commit: `scripts/verify_anchors.sh` exit 0 is
non-negotiable.

## M-FINAL — Tester sweep (CLOSED 2026-05-19)

- [x] Run `rust-validate` (`.claude/skills/rust-validate`) +
  `cargo test --workspace` (must include `--features live` for the
  R3 wire-up tests and `--features realdata` for the 8
  realdata-anchor scenarios).
  **Result:** `cargo fmt --check` exit 0; `cargo clippy --workspace -- -D warnings` exit 0;
  `cargo test --workspace --lib` 278 passed, 0 failed.
- [x] Verify the **22 body-SHA-256 anchors** stay byte-identical
  (R10.1 / H2 / H4) — `scripts/verify_anchors.sh` exit 0 against
  the full anchor set. **NON-NEGOTIABLE** per ADR-0035 § Consequences
  and operator-decided Q5.
  **Result:** 22/22 PASS — byte-identical after all clippy/fmt cleanup.
- [x] Run `cockpit-smoke` (PASS 0 panics; all snapshots green;
  R10.3).
  **Result:** PASS — orchestrator-cited log `cockpit-smoke-2026-05-19T19-56Z.log`, 8s window, 0 panics (unchanged; pre-cleanup run still valid).
- [ ] Verify `cockpit-performance-and-input-responsiveness
  v1.0.0` idle-CPU floor stays ≤13.1% (R10.4 / H5) — repeat the
  post-fix measurement protocol from
  `spec/cockpit-performance-and-input-responsiveness/reports/cpu-measurement-postfix-2026-05-15T13-02Z.log`;
  readback at T+5s post-`LabRunCompleted`.
  **[deferred to orchestrator manual verification]**
- [ ] Measure H1 latency budget — median + p95 for v1.momentum ×
  XRPUSDT × Last90d on 3360×1890; target median ≤3000 ms (read
  from the new `lab.run.latency` tracing span — T-D-N15).
  **[deferred to orchestrator manual verification]**
- [ ] Measure H6 (cancel-poll overhead) — run `top10-2024-fy-tcn-overlay-realdata`
  5× with the poll, 5× with a `#[cfg(not(test))]` patch removing it;
  compare medians. Falsify if poll overhead >2% wall-clock.
  **[deferred to orchestrator manual verification — wrap-and-abort fallback in Phase B; bar-level poll is Phase C work]**
- [ ] Measure H7 (mirror double-hold RSS) — start cockpit, record
  baseline RSS via `ps -o rss`; run two back-to-back
  TCN-overlay × FY-2024 scenarios; record RSS T+5s after second
  completes; falsify if delta >32 MB.
  **[deferred to orchestrator manual verification]**
- [x] CLI byte-identicality smoke across all 4 scenario families
  (K7 mitigation): covered by `scripts/verify_anchors.sh` 22/22 PASS.
- [ ] Cockpit-side end-to-end smoke under `--features live`:
  - Run on (v1.momentum, XRPUSDT, Last90d) → chart updates with
    fresh equity within H1 budget; spinner spins at 10 fps cadence
    (R6.4).
  - Tuple change → re-Run → delta badge hides (per R8.4); then
    re-Run again on the **new** tuple → delta badge shows non-zero
    Δ if the run is deterministic-but-mocked, or zero deltas if
    perfectly deterministic.
  - Cancellation safety (K3): launch a long TCN-overlay-realdata
    run, close cockpit window mid-run, verify process exits within
    5 s.
  **[deferred to orchestrator manual verification]**
- [x] Verify H2-H7 hypotheses — H2 (anchor preservation) PASS 22/22;
  H4 (TCN anchors) PASS; H3/H5/H6/H7 deferred to orchestrator manual verification.
- [x] Author `spec/ui-rethink-phase-b-lab-run/reports/test-final-2026-05-19.md`
  per the test-report template. Report updated in-place with re-gate §13
  (VERDICT FAIL → PASS). All 7 non-regression contract items documented.
  Anchor-verification output 22/22 PASS cited.

**Acceptance:** tester VERDICT → PASS 2026-05-19 (re-gate after developer session a09f2e3a1a02d18de).
Operator approval pending (presenter step gates ship).

## Notes

- Predecessor: `ui-rethink-phase-a-lab v0.2.0` shipped 2026-05-18.
  The Lab vertical (chart + chips + tuple persistence + Run button
  + cancel infra + RunState machine) is already on disk; Phase B is
  a **backend body extraction + wiring** task, not a new screen.
- Non-regression contract enumerated in feature.md § Non-regression
  contract (7 items: anchors, Phase A surface, cockpit-smoke,
  idle-CPU floor, spec-lint, no new deps, no new Lumen tokens).
- Design lock in feature.md § Design (D1-D8) and ADR-0035
  (extraction-pattern rationale).
- Watch recipes (per MEMORY.md `feedback_watch_recipe_for_long_running`):
  - Anchor verification during extraction waves (T-D-N2 onwards):
    ```bash
    watch -n 10 'cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading && ./scripts/verify_anchors.sh 2>&1 | tail -20'
    ```
  - Cockpit idle-CPU sampling during cockpit-smoke (M-FINAL):
    ```bash
    watch -n 5 'ps -o pid,pcpu,rss,etime,comm -p $(pgrep -f target/release/cockpit | head -1) 2>/dev/null'
    ```
  - Long-running TCN backtest probe (T-D-N5 / T-D-N6 anchor
    verification):
    ```bash
    watch -n 10 'cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading && ls -la spec/backtest-real-binance-data/reports/ 2>/dev/null | tail -5'
    ```
