---
slug: ui-rethink-phase-b-lab-run
status: draft
owner: analyst
updated: 2026-05-19
---

# Tasks — UI rethink Phase B (Lab Run button)

> M0 closed by analyst pass 2026-05-19. M-T1 through M-FINAL
> architect-decomposed at T-AR-2 once Q1-Q5 are operator-resolved.

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
- [x] Surfaced Q1-Q5 with analyst-recommended defaults
  (feature.md § Operator decision questions):
  - **Q1** library-call shape: A (in-memory return + opt-in
    disk write — matches ADR-0030 signature).
  - **Q2** progress UX: A (`ThrottledSpinner` only — no
    progress bar at Phase B).
  - **Q3** cancellation: A (button disabled while running +
    internal cancel poll for shutdown safety — no Cancel
    button surface).
  - **Q4** compare diff scope: A (session-local in-memory
    only; no on-disk history walk).
  - **Q5** anchor preservation: A (bytes-identical refactor;
    preserve all 22 anchors; reject v2 anchor refresh).
- [x] Locked R1-R10 requirements, K1-K8 risk register, H1-H5
  hypothesis register.
- [x] Refined `tests` paths to populate in the Phase B trace
  row at architect T-AR-2 (see feature.md § Trace).

**Acceptance:** feature.md status `draft`, version `0.1.0`,
operator-readable, all Qs have defaults. **PASSED 2026-05-19.**

## M-T1 — Architect decomposition (NEXT — gated on operator-decide Q1-Q5)

- [ ] **Operator resolves Q1-Q5** (or accepts defaults silently
  after 24h per analyst recommendation).
- [ ] Architect ratifies / overrides Q1-Q5; commits the
  resolutions inline in feature.md under each Q heading
  (mirroring Phase A's Q-A1/Q-A2/Q-A3 pattern).
- [ ] Architect publishes the `tasks.md` `T-D-N*` decomposition.
  Recommended sequencing per K1 / K2 risk mitigations:
  1. T-D-1..T-D-4: extract 4 simple scenarios (SmaCross, MacdTrend,
     RsiReversion, BBandsMeanRevert), one commit per scenario,
     anchor-gated.
  2. T-D-5: extract Momentum scenario; gate on
     `top10-{2023,2024-h1}-momentum` anchors.
  3. T-D-6: extract Pairs scenario; gate on
     `pairs-{2023,2024-h1}-zscore-mr` anchors.
  4. T-D-7..T-D-8: extract TCN-overlay + TCN-overlay-weights
     scenarios LAST (per K2 mitigation); gate on all 7
     TCN-related anchors including realdata + alpha-investigation
     reports.
  5. T-D-9: collapse `main.rs` to thin CLI wrapper (R2);
     re-verify all 22 anchors.
  6. T-D-10: wire `spawn_lab_run` to real `run_scenario` (R3);
     manual cockpit smoke under `--features live`.
  7. T-D-11: extend `LabState` with `last_run_report` /
     `prev_run_report` (R4); add Clone-skip + persistence-skip
     logic.
  8. T-D-12: route chart equity overlay through
     `last_run_report` first (R5); preserve `EquityCache`
     fallback.
  9. T-D-13: implement `engine::run_scenario` cancellation
     poll (R7.1) + cancellation unit test.
  10. T-D-14: land `widgets/run_delta_badge.rs` + tests +
      insta snapshot (R8).
  11. T-D-15: integration test `crates/ui/tests/lab_run_engine.rs`
      per H3.
- [ ] Architect updates `spec/trace.toml` Phase B row `arch`
  field with any new ADR / design dev-note links + `tests`
  field with the file paths in feature.md § Trace.
- [ ] Architect runs `rust-validate` + `rust-build` on a
  baseline commit to confirm the Phase A surface is green
  pre-extraction.

**Acceptance:** `tasks.md` carries 11-15 T-D-N* checkboxes with
crate paths + R-anchors; architect handoff line emitted with
filled TOML envelope per AGENT.md §Communication contract.

## M-T2..M-T(N-1) — Developer extraction waves

> Architect-spawned per T-D-N* decomposition. Each task lands as
> a separate commit (per MEMORY.md `feedback_no_worktrees`) on
> `main`. Watch recipe per MEMORY.md
> `feedback_watch_recipe_for_long_running`:
> ```bash
> watch -n 10 'cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading && ./scripts/verify_anchors.sh 2>&1 | tail -20'
> ```

(Specific tasks decomposed at M-T1.)

## M-FINAL — Tester sweep

- [ ] Run `rust-validate` (`.claude/skills/rust-validate`) +
  `cargo test --workspace`.
- [ ] Verify the 22 body-SHA-256 anchors stay byte-identical
  (R10.1 / H2) — `scripts/verify_anchors.sh` exit 0 against the
  full anchor set.
- [ ] Run `cockpit-smoke` (PASS 0 panics; all snapshots green;
  R10.3).
- [ ] Verify `cockpit-performance-and-input-responsiveness
  v1.0.0` idle-CPU floor stays ≤13.1% (R10.4 / H5) — repeat
  the post-fix measurement protocol from
  `spec/cockpit-performance-and-input-responsiveness/reports/
  cpu-measurement-postfix-2026-05-15T13-02Z.log`; readback at
  T+5s post-`LabRunCompleted`.
- [ ] Measure H1 latency budget — median + p95 for v1.momentum
  × XRPUSDT × Last90d on 3360×1890; target median ≤3000 ms.
- [ ] CLI byte-identicality smoke across all 4 scenario families
  (K7 mitigation):
  - `cargo run -p backtest --bin backtest -- --scenario
    btc-2023-1m-sma-cross …` → anchor-byte-identical.
  - `… --scenario top10-2024-h1-momentum …` → identical.
  - `… --scenario pairs-2024-h1-zscore-mr …` → identical.
  - `cargo run -p backtest --bin backtest --features realdata --
    --scenario top10-2024-fy-tcn-overlay-realdata …` →
    identical.
- [ ] Cockpit-side end-to-end smoke under `--features live`:
  - Run on (v1.momentum, XRPUSDT, Last90d) → chart updates
    with fresh equity within H1 budget; spinner spins at 10 fps
    cadence (R6.4).
  - Tuple change → re-Run → delta badge hides (per R8.4); then
    re-Run again → delta badge shows non-zero Δ if run is
    deterministic-but-mocked, or zero deltas if perfectly
    deterministic (the latter is the expected ChaCha20-seeded
    case — analyst note: this is a test design call; tester
    may need a non-trivial scenario to surface visible deltas).
  - Cancellation safety: launch a long run (TCN realdata),
    close cockpit window mid-run, verify process exits within
    5s (K3 mitigation).
- [ ] Verify H2-H5 hypotheses (anchor preservation, in-memory ==
  cached equality, TCN anchors green, idle-CPU floor preserved).
- [ ] Author
  `spec/ui-rethink-phase-b-lab-run/reports/test-final-<YYYY-MM-DD>.md`
  per the test-report template at
  `.claude/skills/rust-test/templates/test-report.md`. Include:
  - All 7 non-regression contract items (feature.md § Non-
    regression contract) with PASS/FAIL + evidence.
  - H1 latency numbers (median + p95 in ms).
  - Idle-CPU readback in % at T+5s.
  - Anchor-verification output (22/22 PASS).

**Acceptance:** tester VERDICT → PASS or REGRESSION. PASS gates
ship; REGRESSION returns to architect / developer for triage.

## Notes

- Predecessor: `ui-rethink-phase-a-lab v0.2.0` shipped 2026-05-18.
  The Lab vertical (chart + chips + tuple persistence + Run
  button + cancel infra + RunState machine) is already on disk;
  Phase B is a **backend body extraction + wiring** task, not a
  new screen.
- Non-regression contract enumerated in feature.md § Non-
  regression contract (7 items: anchors, Phase A surface,
  cockpit-smoke, idle-CPU floor, spec-lint, no new deps, no new
  Lumen tokens).
- Watch recipes (per MEMORY.md `feedback_watch_recipe_for_long_running`):
  - Anchor verification during extraction waves:
    ```bash
    watch -n 10 'cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading && ./scripts/verify_anchors.sh 2>&1 | tail -20'
    ```
  - Cockpit idle-CPU sampling during cockpit-smoke:
    ```bash
    watch -n 5 'ps -o pid,pcpu,etime,comm -p $(pgrep -f target/release/cockpit | head -1) 2>/dev/null'
    ```
