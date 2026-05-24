---
slug: lab-end-to-end-v2
status: proposed
owner: analyst
updated: 2026-05-24
---

# Tasks — Lab end-to-end v2

Ordered task list. Owner per row; tester ticks `T_FINAL_*` per
[AGENT.md § Process discipline](../../AGENT.md#process-discipline-lessons-from-v0--v15a).

## Wave A — Analyst (this brief)

- [x] **T-A1** Walk Lab runtime vs Phase A/B spec rows; produce
      gap table (see `feature.md` § Why).
- [x] **T-A2** Read `crates/ui/src/state.rs::update` Lab arms;
      surface F1 (missing binary wrapper) + F2 (`LabSelectPair`
      does not update `selected_symbol`).
- [x] **T-A3** Walk `crates/backtest/src/engine.rs::run_scenario`
      dispatch table; surface F3 (cross-sectional-only).
- [x] **T-A4** Walk `crates/ui/src/bin/cockpit_live.rs::update`
      `LabRunRequested` arm; confirm `RunCancelHandle` dropped
      (F4).
- [x] **T-A5** Grep scenario modules for progress channels; F5
      (no channel exists).
- [x] **T-A6** Trace `chart_markers` data path; F6 (audit-ledger
      driven, not RunReport.fills).
- [x] **T-A7** Author `spec/lab-end-to-end-v2/feature.md` v0.1.0.
- [x] **T-A8** Author `spec/lab-end-to-end-v2/tasks.md` (this file).
- [x] **T-A9** Append REQ row to `spec/trace.toml`.
- [x] **T-A10** Append Active block to `spec/backlog.md`.

## Wave B — Operator decide (M-OD)

Q's surfaced in `feature.md § Operator decision questions`. Each
row is `[ ]` until the operator picks. Architect M-T1 cannot
start on Q1 / Q2 without the operator's call (HIGH-stakes); the
remaining Q's auto-default if the operator hasn't responded by
M-T1 kickoff (Q3-Q8 are LOW-risk analyst-recommended defaults).

- [ ] **T-OD1 — Q1**: strategy dispatch shape. (a) single-symbol
      arms / (b) `pair_filter` / (c) scope selector / (d) defer.
      Analyst default: (a)+(d). **HIGH-stakes — operator must pick.**
- [ ] **T-OD2 — Q2**: Stop button in scope. (a) in / (b) sibling
      feature. Analyst default: (a). **HIGH-stakes — operator
      should pick.**
- [ ] **T-OD3 — Q3**: `last_run_report` data path. (a) extend
      `RunSummary` / (b) on-disk re-read. Analyst default: (a).
- [ ] **T-OD4 — Q4**: progress channel shape. (a) explicit /
      (b) reuse cancel poll / (c) UI-side fake / (d) bar-count
      estimate + snap. Analyst default: (b).
- [ ] **T-OD5 — Q5**: progress UX shape. (a) bar + spinner /
      (b) bar replaces spinner / (c) bar + spinner stacked.
      Analyst default: (a).
- [ ] **T-OD6 — Q6**: fixtures cockpit pair pre-loading.
      (a) pre-load all / (b) lazy / (c) symbol-rewrite. Analyst
      default: (a).
- [ ] **T-OD7 — Q7**: cancellation receiver threading.
      (a) `ScenarioConfig` field / (b) separate arg. Analyst
      default: (b).
- [ ] **T-OD8 — Q8**: Run-button label on Stop. (a) Idle /
      (b) Cancelled. Analyst default: (b).

## Wave C — Architect M-T1

- [ ] **T-AR1** Read brief + read operator's Q answers (M-OD).
- [ ] **T-AR2** Author Design section in `feature.md` (or sibling
      ADR if surface changes warrant): `ScenarioConfig` field
      additions (Q7-driven), `RunSummary` shape (Q3-driven),
      progress Recipe pattern (mirrors `ServerTimeRecipe`).
- [ ] **T-AR3** Anchor pre-stage: run `scripts/pre_stage_anchors.sh`
      to record current 34 SHAs; confirm baseline before any
      developer extraction.
- [ ] **T-AR4** Wave / task decomposition: split into wave-1
      (R1+R2 binary wrapper + LabSelectPair fix — small, no anchor
      risk), wave-2 (R3 single-symbol dispatch arms — anchor-
      gated), wave-3 (R6 Stop button — needs `ScenarioConfig`
      change), wave-4 (R7+R8+R9 progress channel + widget). Mark
      wave-3 and wave-4 as parallelizable (different crates).
- [ ] **T-AR5** Document the assertion budget for `LabSelectPair`
      cascade (existing markers/signals fetch fires for free) —
      surface to developer brief.
- [ ] **T-AR6** Update REQ row in `spec/trace.toml`: add `arch`
      paths (the architect-authored Design + ADR if any).

## Wave D — Developer (parallel where independent)

### Wave D-1 — wiring fixes (no anchor risk)

- [ ] **T-D1.1 — R1.1**: fix `state.rs::update` `LabSelectPair`
      arm to also update `model.selected_symbol`.
- [ ] **T-D1.2 — R1.4**: extend fixtures cockpit (`bin/cockpit.rs`)
      to pre-load bars for all 10 universe pairs.
- [ ] **T-D1.3 — R2.1, R2.2, R2.3**: author binary-side wrapper
      in `cockpit_live.rs::update` that intercepts
      `Message::LabRunCompleted(Ok(summary))` BEFORE the forward
      to `ui::state::update`; rotates `last_run_report` ←
      mirror; rotates `prev_run_report` ← old last.
- [ ] **T-D1.4 — R2.4 (pending Q3)**: extend `RunSummary` to
      carry `equity_series` + `kpis` from `RunReport` (Q3-A) OR
      add `EquityCache::load_from_path` (Q3-B).
- [ ] **T-D1.5 — R2.5 (pending R5.2 / K6)**: wrapper dispatches
      `Message::ChartMarkersLoaded(Ok(fills))` after rotation
      when `RunReport.fills` non-empty.
- [ ] **T-D1.6 — R4.1**: author integration test
      `lab_run_e2e_completion` at
      `crates/ui/tests/lab_run_integration.rs`.
- [ ] **T-D1.7**: run `cargo test --workspace --lib` — confirm
      692 baseline still green.
- [ ] **T-D1.8**: run `scripts/verify_anchors.sh` — confirm
      34/34 SHAs unchanged (no anchor touch in wave D-1).

### Wave D-2 — single-symbol dispatch arms (anchor-gated; Q1=(a))

- [ ] **T-D2.1 — R3.1**: extract `run_sma_backtest` from
      `crates/backtest/src/main.rs` into
      `crates/backtest/src/scenarios/sma.rs::run`. Behaviour-
      preserving move.
- [ ] **T-D2.2 — R3.1**: same for MACD / RSI / BBands.
- [ ] **T-D2.3 — R3.1**: add 4 dispatch arms to
      `engine::run_scenario` (`"v0.sma"`, `"v0.5.macd"`,
      `"v0.5.rsi"`, `"v0.5.bbands"`).
- [ ] **T-D2.4 — R3.2**: each arm builds a single-symbol scenario
      input keyed on `cfg.pair.1`.
- [ ] **T-D2.5 — R3.3**: run `scripts/verify_anchors.sh` after
      each extraction; expect 4 legacy single-symbol anchors
      stay byte-identical.
- [ ] **T-D2.6 — R3.4**: confirm Lab strategy chip row enumerates
      the 4 new strategies (no UI change if registry already
      includes them).

### Wave D-3 — Stop button (Q2=(a))

- [ ] **T-D3.1 — R6.1**: store `RunCancelHandle` in
      `Cockpit.lab_state.run_cancel: Option<RunCancelHandle>`
      (new field).
- [ ] **T-D3.2 — R6.2 (pending Q7)**: extend `ScenarioConfig`
      with `cancel_rx` field (Q7-A) OR pass as separate arg
      (Q7-B). Thread into each scenario module's bar loop.
- [ ] **T-D3.3 — R6.2**: bar loop polls
      `cancel_rx.is_cancelled()` at `bar_idx & 0x7F == 0`. K4
      mitigation: 32-bar boundary for first 128 bars.
- [ ] **T-D3.4 — R6.3**: render Stop button in Lab top-bar when
      `lab_run_inflight == true`.
- [ ] **T-D3.5 — R6.4**: `LabRunCompleted(Err(Cancelled))` →
      `RunState::Idle` (Q8=(a)) or `Cancelled` (Q8=(b)).
- [ ] **T-D3.6**: integration test: click Run → wait 2s → click
      Stop → assert `Err(Cancelled)` arrives within 1 s.
- [ ] **T-D3.7**: anchor gate after the `run_scenario` signature
      change.

### Wave D-4 — progress channel + widget (parallelizable with D-3)

- [ ] **T-D4.1 — R7.1 (pending Q4)**: add `progress_tx` to
      `ScenarioConfig` (Q4=(b)) and thread into each scenario
      module's bar loop.
- [ ] **T-D4.2 — R7.2**: each bar loop emits `Progress { current,
      total, elapsed_ms }` at the cancellation poll boundary.
- [ ] **T-D4.3 — R7.3, R7.4**: author `LabProgressRecipe` in
      `crates/ui/src/live.rs` (or new `lab/progress.rs`); mirrors
      `ServerTimeRecipe`. Channel capacity 8, lossy.
- [ ] **T-D4.4 — R8**: author
      `crates/ui/src/widgets/progress_bar.rs`. Determinate +
      indeterminate (shimmer-stripe) variants. Lumen tokens.
- [ ] **T-D4.5 — R8.5**: add `progress_bar__*` panels to
      ui_gallery_bin.
- [ ] **T-D4.6 — R9**: extend `LabState` with `run_progress:
      Option<Progress>`; arms in `state::update` for
      `LabRunProgress` + clear on `LabRunRequested` /
      `LabRunCompleted`.
- [ ] **T-D4.7 — R9.4**: Lab view renders progress bar next to
      Run button when `lab_run_inflight`.
- [ ] **T-D4.8**: anchor gate (the scenario module changes touch
      the bar loops — confirm SHAs unchanged on a no-cancel
      no-progress-channel call path).

## Wave E — UI-Designer (parallel with D-1 / D-3 / D-4 UI bits)

- [ ] **T-UI1**: Lumen token review for progress-bar widget
      (R8.2). Confirm `ACCENT_2` is appropriate; pick fallback
      tokens for indeterminate state.
- [ ] **T-UI2**: Visual snapshot review of the running-state
      Lab top-bar with progress bar; confirm Stop button shape
      reads as paired with Run, not as a separate widget.
- [ ] **T-UI3**: Strings inventory: "Stop", "Running", "Run",
      "Cancelled" — add to `crates/ui/src/strings.rs` per
      project string-inventory convention.

## Wave F — Tester (M-FINAL)

- [ ] **T-T1**: run `rust-build` + `rust-validate` (fmt, clippy,
      docs, deny).
- [ ] **T-T2**: run `cargo test --workspace --lib` — 692 + new
      tests green.
- [ ] **T-T3**: run `scripts/verify_anchors.sh` — 34/34 PASS.
- [ ] **T-T4**: run `cockpit-smoke` skill — exit 0.
- [ ] **T-T5**: run `uv run scripts/spec_lint.py` — exit 0.
- [ ] **T-T6**: integration test `lab_run_e2e_completion` — PASS
      within 30 s budget.
- [ ] **T-T7**: H1-H6 evaluation: numerical checks against
      hypothesis register; record `test-final-2026-05-XX.md`.
- [ ] **T-T8**: cockpit-performance idle-CPU regression check —
      ≤ 13.1 %.
- [ ] **T-T9**: Phase F default-disabled byte-identity check
      (panel snapshot diff = 0).
- [ ] **T_FINAL_VERDICT**: evaluator emits VERDICT in
      `evaluation-2026-05-XX.md` — PASS / FAIL / REGRESSION.
      **Tester only ticks this after evaluator PASS + verify-
      anchors PASS.**

## Wave G — Presenter (M-P1)

- [ ] **T-P1**: assemble
      `spec/lab-end-to-end-v2/presentations/lab-end-to-end-v2-2026-05-XX.md`
      — runs the live cockpit binary, captures Run → success
      screenshots, embeds H1-H6 numerical verdict, lists Q1-Q8
      operator picks. Hand path back to operator for approval.

## Notes

- All sub-agents work on `main`; no worktrees, no branches.
- Sub-agents do NOT commit; the orchestrator owns
  `git add` + `git commit` + `git push origin main` at end of
  each wave.
- Long-running tasks (cargo test on the full workspace, anchor
  verify across 34 SHAs) must emit a `watch -n 5 '<probe>'` block
  per the watch-recipe memo (`feedback_watch_recipe_for_long_running.md`
  in the user's project-memory store).
- Honest-tick rule applies: developer rows tick `[x]` only after
  citing (a) file:line, (b) test command, (c) test-output proving
  pass. Tester owns `T_FINAL_*` ticks.

## Changelog

- 2026-05-24 (analyst): initial task list — wave A done; B-G
  ungated. Wave D split into D-1..D-4 with explicit parallel/
  serial markers.
