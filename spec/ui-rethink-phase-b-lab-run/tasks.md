---
slug: ui-rethink-phase-b-lab-run
status: proposed
owner: pending-analyst
updated: 2026-05-19
---

# Tasks — UI rethink Phase B (Lab Run button)

> Analyst-pass stub. Replace this skeleton with R-anchored milestone
> breakdown (M0/M-T1/.../M-FINAL) once analyst pass lands.

## M0 — Analyst synthesis

- [ ] Confirm `crates/backtest` shape — binary-first vs. library-first.
  Inspect `crates/backtest/src/main.rs` + `lib.rs` (if any). Cite the
  current entry-point pattern.
- [ ] Survey existing Lab `Run` button code path —
  `crates/ui/src/lab/runner.rs::spawn_lab_run` precedent from
  Phase A.
- [ ] Surface Q1-Q5 from feature.md to operator before architect
  spawn.
- [ ] Lock R1-Rn requirements; close Q1-Q5 with analyst-recommended
  defaults the developer can ship against.

## M-T1 — TBD (architect-decomposed at T-AR-2)

## M-FINAL — Tester sweep

- [ ] Run `rust-validate` + `cargo test --workspace`.
- [ ] Verify the 22 body-SHA-256 anchors stay byte-identical (R10
  contract).
- [ ] Run `cockpit-smoke` (PASS 0 panics).
- [ ] Verify `cockpit-performance-and-input-responsiveness v1.0.0`
  idle-CPU floor stays ≤13.1% after the engine integrates.
- [ ] Author
  `spec/ui-rethink-phase-b-lab-run/reports/test-final-<YYYY-MM-DD>.md`
  per the test-report template.

## Notes

- Predecessor: `ui-rethink-phase-a-lab v0.2.0`. The Lab vertical is
  already on disk; Phase B is a backend-cross-cut + wiring task, not a
  new screen.
- Non-regression contract enumerated in feature.md.
