---
adr: 0035
title: Phase B scenario-dispatch extraction — backtest engine body without anchor drift
status: accepted
date: 2026-05-19
supersedes: none
superseded-by: none
extends: 0030
---

# ADR-0035: Phase B scenario-dispatch extraction — backtest engine body without anchor drift

## Context

[ADR-0030](0030-cockpit-in-process-backtest.md) locked the public shape
of `crates/backtest::engine::run_scenario(cfg) -> Result<RunReport,
RunError>` and shipped the **type surface** in Phase A. The body is a
literal `Err(RunError::NotImplemented)` at
`crates/backtest/src/engine.rs:236-240`; the actual scenario dispatch
lives in `crates/backtest/src/main.rs` (3417 LOC, 7 scenario variants,
4 distinct backtest-path fns: `run_momentum_backtest` @774,
`run_pairs_backtest` @1163, `run_tcn_overlay_backtest` @1633,
`run_tcn_overlay_weights_backtest` @1902, plus the inline
SmaCross/Composed loop in `main()` @3206-3305 and three report-writer
fns at @1026, @1452, @2184).

The Phase B feature `ui-rethink-phase-b-lab-run v0.1.0` requires
`run_scenario` to populate the in-memory `RunReport` for the cockpit's
Lab Run button while preserving **all 22 body-SHA-256 anchors** in
`spec/anchors.toml` byte-identically. The challenge is mechanical
(move 3400 LOC), but the failure mode is silent (one mis-ordered RNG
draw → anchor mismatch → invisible regression until a tester run weeks
later). Operator decision Q5 (locked 2026-05-19 via "Autoapprove all")
rejects a v2 anchor refresh — the only acceptable outcome is
**bytes-identical extraction**.

This ADR records the extraction pattern, the per-scenario wave
discipline, and the in-memory ↔ on-disk single-source-of-truth
contract so future engine-body refactors (Phase D Trail, Phase E
Compare matrix) follow the same shape.

## Decision

1. **Single source of truth.** `run_scenario`'s body **owns** the
   compute pass; the report writer's "format Markdown bytes" pass runs
   on the populated in-memory `RunReport` after the compute. When
   `cfg.write_report = false`, the bytes are not formatted (no-cost
   skip). When `cfg.write_report = true`, the writer reads
   `RunReport.{equity_series, fills, kpis}` and emits the same bytes
   the standalone binary writes today. H3 (in-memory ≡ cached-disk
   equity equality) is the determinism gate.

2. **Per-scenario commit discipline (K1 mitigation).** Extract one
   scenario family per commit, in the order:
   1. SmaCrossover + Composed (the inline loop in `main()` @3206-3305).
      Single commit (they share the loop).
   2. Momentum (`run_momentum_backtest` @774).
   3. Pairs (`run_pairs_backtest` @1163).
   4. TCN-overlay (`run_tcn_overlay_backtest` @1633).
   5. TCN-overlay-weights (`run_tcn_overlay_weights_backtest` @1902).

   Between every commit run `scripts/verify_anchors.sh` exit 0. Any
   mismatch rolls the commit back; the developer fixes drift before
   advancing.

3. **TCN scenarios extract LAST (K2 mitigation).** The TCN paths
   carry the heaviest surface area (ONNX model load, lazy-static
   handles, scratch tensors). The simpler scenarios anchor-verify
   first so the TCN extraction lands on a known-green base.

4. **Module layout under `crates/backtest/src/`.** New sibling modules
   replace the `run_*_backtest` fns in `main.rs`. Recommended shape:

   ```text
   crates/backtest/src/
   ├── lib.rs                # unchanged (re-exports)
   ├── engine.rs             # run_scenario body — dispatcher only
   ├── paper.rs              # unchanged
   ├── realdata.rs           # unchanged (feature-gated)
   ├── scenarios/
   │   ├── mod.rs            # pub(crate) mod boundary
   │   ├── sma_composed.rs   # SmaCrossover + Composed loop
   │   ├── momentum.rs       # extracted run_momentum_backtest body
   │   ├── pairs.rs          # extracted run_pairs_backtest body
   │   ├── tcn_overlay.rs    # extracted TCN overlay
   │   └── tcn_overlay_weights.rs
   ├── report/
   │   ├── mod.rs
   │   ├── sma.rs            # extracted write_report fn (line 2488)
   │   ├── momentum.rs       # extracted write_momentum_report (line 1026)
   │   ├── pairs.rs          # extracted write_pairs_report (line 1452)
   │   └── tcn_overlay.rs    # extracted write_tcn_overlay_report (line 2184)
   └── main.rs               # collapses to clap parse + ScenarioConfig
                             # build + run_scenario.await + println
   ```

   Modules are `pub(crate)` (engine-internal). `main.rs` becomes a
   ≤200 LOC CLI wrapper.

5. **Cancellation poll (K3 mitigation).** Each scenario's bar-loop
   gets one `cancel.is_cancelled()` check at the top of the loop body,
   gated by `if bar_idx & 0x7F == 0` (every 128 bars — power-of-two
   bitmask cheaper than `%`). On cancel return
   `Err(RunError::Cancelled)` (new variant). The poll touches the
   compute pass but **not** the writer; report-write side-effects are
   skipped on cancel. Anchor-safe by construction (the cancel branch
   is unreachable in normal CLI runs).

6. **`RunError::Cancelled` variant (additive).** Add a new variant
   `Cancelled` to `RunError`; existing match arms outside the engine
   are non-exhaustive (the runner's `format!("{e}")` covers any
   variant). The variant lives in `engine.rs` next to the existing
   variants; no public-API break.

7. **Cockpit-side in-memory mirror.** `RunReport.equity_series`,
   `fills`, `kpis` are populated unconditionally on the
   compute-pass side. The cockpit reads from this directly; the
   on-disk Markdown report is the durable audit trail. ADR-0030 § R5
   contract preserved.

8. **No new public-surface promises.** `compute_sharpe` (currently in
   `main.rs` @2454, signature `fn(&[Decimal]) -> f64`) is re-exported
   from `crates/backtest/src/lib.rs` so the cockpit's delta-badge
   widget can compute Δ Sharpe without duplicating the 30 LOC. The
   re-export is the **only** new public symbol in Phase B. Future
   shape changes require an ADR amendment.

## Alternatives considered

- **Parallel computation paths (writer-canonical + in-memory mirror
  computed separately).** Rejected: doubles the compute cost, and any
  drift between paths is a silent regression (H3 would fail without
  the per-cycle determinism contract). Analyst's recommendation —
  ratified.

- **v2 anchor refresh** (regenerate all 22 anchors with the
  refactored binary). Rejected by operator Q5 (locked 2026-05-19).
  The historical audit trail depends on byte-stability across the
  Phase B refactor.

- **One mega-commit for the whole extraction.** Rejected as K1
  mitigation: a 3400-LOC single commit is unreviewable and unbisectable.
  Per-scenario commits with `verify_anchors.sh` between each are the
  guardrail.

- **Extract TCN first (highest complexity).** Rejected as K2
  mitigation: TCN extraction depends on the simpler-scenario
  extraction patterns being known-good. TCN lands last so the
  developer has 4 prior anchor-verified extractions as the template.

- **Promote `Cockpit::lab_run_inflight: bool` to
  `Option<RunCancelHandle>` in Phase B.** Deferred. Phase A ships the
  bool; the internal cancel poll lands without needing handle
  ownership (the receiver is moved into the spawned task). Promotion
  to `Option<RunCancelHandle>` lands in Phase C when a Cancel
  button surfaces.

## Consequences

- **Anchor verification is non-negotiable.** Every developer commit
  during Phase B runs `scripts/verify_anchors.sh` and the dev refuses
  to advance to the next wave on a mismatch. Watch recipe per
  MEMORY.md `feedback_watch_recipe_for_long_running`:

  ```bash
  watch -n 10 'cd /Users/Vitaliy.Schreibmann/Projects/Privat/trading/trading && ./scripts/verify_anchors.sh 2>&1 | tail -20'
  ```

- **The 22 body-SHA-256 anchors in `spec/anchors.toml` stay locked.**
  This ADR does NOT mutate any anchor. Anchor mutation requires a
  separate ADR per the CLAUDE.md non-negotiable on anchor stability.

- **CLI behaviour byte-identical.** `cargo run -p backtest --bin
  backtest -- --scenario <name> --seed <n>` produces the same
  `spec/<feature>/reports/backtest-<stamp>-<scenario>.md` file at the
  same path with the same body bytes. The `generated:` YAML
  front-matter timestamp varies as today (already excluded from
  `report_body_hash`).

- **Cockpit's Lab Run button reads from in-memory `RunReport` first**
  (R5.1 in feature.md), falling back to `EquityCache` for cold-start.
  H3 hypothesis (in-memory ≡ cached-disk equity) is the determinism
  gate for this routing.

- **`scripts/verify_anchors.sh`** is the mechanically-checked
  enforcement point. Per-scenario commits + watch loop above are the
  developer's discipline rail.

- **Cancellation is internal-only at Phase B** (Q3 default). The
  operator never presses Cancel; the `RunCancelReceiver` exists in
  the runner so cockpit-shutdown does not hang on an in-flight
  backtest (K3 mitigation).

- **`compute_sharpe` becomes part of the `backtest` crate's public
  API** (K8). The shape `fn(&[Decimal]) -> f64` is locked; future
  changes require an ADR amendment.

## Changelog
- 2026-05-19 (architect): initial accept. Phase B M-T1 decomposition
  pass; ratifies analyst defaults from `ui-rethink-phase-b-lab-run
  v0.1.0` (operator-decided Q1=A, Q2=A, Q3=A, Q4=A, Q5=A on 2026-05-19
  via "Autoapprove all"). Extends ADR-0030 with the per-scenario
  extraction pattern + module layout + cancel-poll cadence.
