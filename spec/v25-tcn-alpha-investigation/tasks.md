---
slug: v25-tcn-alpha-investigation
status: draft
owner: analyst
updated: 2026-05-18
---

# Tasks — v2.5 TCN alpha-verdict investigation

> Milestone-only skeleton authored by analyst. Architect owns the per-task
> `T-D-N` / `T-AR-N` / `T-T-N` decomposition at T-AR-2 against the scope the
> operator selects in `feature.md § Operator-decide questions Q1`.
>
> Scope (operator-decide):
> - **Minimal (RECOMMENDED)** — M0 → M-R-HAT → M-SHARPE → M-FINAL.
> - **Diagnostic** — adds M-DIAG between M-R-HAT and M-SHARPE.
> - **Full root-cause-and-fix** — adds M-HORIZON before M-FINAL.
>
> Milestones are listed for the minimal scope by default; the optional
> milestones are flagged inline and architect activates them only if the
> operator picks diagnostic / full.

## Milestones

- [ ] **M0 — Scope-decision gate.** Operator answers Q1 in
      [`feature.md`](feature.md#operator-decide-questions-must-answer-before-architect-lock).
      Architect locks the milestone list and emits `T-AR-N` task
      decomposition into this file. _Acceptance: scope value
      (minimal/diagnostic/full) recorded in feature.md changelog; the
      remaining milestones list below is annotated with the active
      / skipped tag per the chosen scope._

- [ ] **M-R-HAT — Forecast-distribution inspector (bucket a).**
      Architect designs + developer ships the read-only forward-pass
      inspector per R3 (bin under `crates/forecast/src/bin/` OR a
      `--emit-r-hat-histogram` extension to the existing backtest
      path — architect's call). Run against BS-1 (87,590 bars, 2023)
      and BS-2 (87,840 bars, 2024). Emit two R1 reports under
      `spec/v25-tcn-alpha-investigation/reports/forecast-distribution-bs{1,2}-realdata-<YYYYMMDD>.md`.
      Bodies carry summary stats + histogram + the F1 / F2 / F3 / F4
      verdict per R4. _Acceptance: both reports on disk; both bodies
      byte-identical on a second run (K5 determinism); R4 verdict label
      present in `## Verdict` section._

- [ ] **M-DIAG — Checkpoint-internal inspection (bucket c).**
      _Active only under DIAGNOSTIC scope. Skipped under MINIMAL._
      Architect designs + developer ships a held-out-batch
      forward-pass log emitting intermediate-layer activation stats
      (block-wise output stdev, terminal-layer pre-projection range).
      Distinguishes case F1 (numerical zero collapse) from F2 (small
      but non-zero output, sigma_train mis-calibration) more directly
      than the R1 boundary histogram alone. _Acceptance: a single
      diagnostic report on disk per checkpoint with layer-stdev table;
      no checkpoint mutation (R6 contract verified)._

- [ ] **M-SHARPE — Sharpe-comparison report (bucket d).**
      Author the R5 report family — TCN-overlay vs v1-momentum baseline
      across the four `-realdata` anchor scenarios. Architect locks
      Sharpe / Sortino / Calmar formulas at analyst defaults (hourly
      annualised by √(24·365), zero risk-free). Report goes under
      `spec/v25-tcn-alpha-investigation/reports/sharpe-comparison-realdata-<YYYYMMDD>.md`.
      _Acceptance: report on disk; if body is deterministic (architect
      verifies frontmatter-vs-body discipline per K3 / ADR-0032 § D4),
      lock as anchor `sharpe-comparison-realdata` under
      `v2.6.0-alpha-investigation`. Otherwise ship un-anchored with
      a `## Note: not anchorable` body section explaining the
      determinism gap._

- [ ] **M-HORIZON — Horizon-bumped re-training pass (bucket b).**
      _Active only under FULL scope. Skipped under MINIMAL / DIAGNOSTIC._
      Train a multi-horizon-head TCN ({1h, 4h, 24h}) using the
      `train_tcn` binary with config overrides. Land new checkpoints
      under `crates/forecast/checkpoints/anchors/tcn-bs{1,2}-mh-<sha>.safetensors`.
      Re-run R3 + R5 against the multi-horizon checkpoints; compare
      against the single-horizon baseline. Wall-clock estimate
      ~30min per training run on M-series Metal × 2 checkpoints +
      inference passes. _Acceptance: two new LFS-tracked checkpoints
      with valid provenance metadata per ADR-0029; comparison report
      in `spec/v25-tcn-alpha-investigation/reports/` showing
      single-horizon vs multi-horizon Sharpe delta._

- [ ] **M-FINAL — Ship gate.**
      - Anchor neutrality (R6): `bash scripts/verify_anchors.sh` →
        `ANCHORS PASS (19/19)` PRE-lock and `21/21` (R1 only) or
        `22/22` (R1 + R5 Sharpe anchor) POST-lock. The 19 originals
        stay byte-identical.
      - Operator verdict recorded in `feature.md § Verification` per
        R-success-criterion 3 (`Success criteria`): a named
        follow-on feature is queued OR an explicit
        "no-follow-on" disposition is documented.
      - `cargo fmt --check`, `cargo clippy --workspace -- -D warnings`,
        and any test-suite invariants land green.
      - Tester writes the test report at
        `spec/v25-tcn-alpha-investigation/reports/test-<YYYYMMDD-HHMM>-v25-tcn-alpha-investigation.md`
        per the [tester template](../.claude/skills/rust-test/templates/test-report.md).
      - Trace row `REQ-V25-TCN-ALPHA-001` flips
        `proposed` → `in-progress` → `shipped` and gets its
        `crates`, `tests`, `anchors` columns filled.

## Parallelism map for the orchestrator (analyst's recommendation)

- M0 is sequential (operator + architect gate).
- M-R-HAT and M-SHARPE are **independent** under minimal scope and
  CAN run in parallel after M0: M-R-HAT is a new inspector bin;
  M-SHARPE only reads existing `-realdata` anchored reports +
  computes Sharpe/Sortino/Calmar; the two touch disjoint files.
  Architect may launch developer + a second developer (or developer
  + analyst-as-report-author) in parallel.
- M-DIAG (if active) depends on M-R-HAT (uses the same inspector
  surface).
- M-HORIZON (if active) is a heavy sequential step before M-FINAL.
- M-FINAL is sequential (tester gate).

## Out of scope for tasks.md

- Per-task `T-D-N` / `T-AR-N` / `T-T-N` decomposition (architect owns at T-AR-2).
- Crate split / module boundaries (architect owns).
- Specific cargo-feature flags (architect locks in design).

## Notes

- Anchor naming convention preferred by analyst:
  `forecast-distribution-bs1-realdata`,
  `forecast-distribution-bs2-realdata`, and (if anchorable)
  `sharpe-comparison-realdata`. Under version
  `v2.6.0-alpha-investigation`. Architect may rename in design.
- All milestones are read-only against the trained checkpoints
  (LFS-tracked under `crates/forecast/checkpoints/anchors/`). No
  milestone modifies a checkpoint, including M-HORIZON which adds
  NEW checkpoints (multi-horizon variants), it does not modify the
  existing two.
