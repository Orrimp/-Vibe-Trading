---
slug: point-in-time-data-discipline
status: proposed
owner: analyst
updated: 2026-06-18
---

# Tasks — point-in-time / as-of data discipline

> **Seed skeleton (analyst).** The analyst owns the `[[req]]` row + brief; the
> architect owns the M-T1 design lock and expands this list into ordered
> M-DEV-* tasks once the Open decisions in
> [`feature.md`](feature.md#open-decisions-for-the-architect) are resolved.
> Acceptance criteria live in [`feature.md`](feature.md#acceptance-criteria)
> (AC1–AC5).

## Open decisions to resolve first (architect, M-T1)

- [ ] OD1 — Type-level (`PitSeries<T>`/`AsOf<T>`, compile-time guarantee) vs
  runtime-guard (`as_of_join` + debug-assert). _analyst lean: type-level
  (durable, Recommended); runtime-guard = if-budget-tightens fallback._
- [ ] OD2 — Look-ahead lint (`scripts/`) vs typed-API-only. _Decide if the lint
  adds coverage the typed API does not (catches new hand-rolled bypass joins)._
- [ ] OD3 — Home crate: `crates/core` vs `crates/data` vs `crates/backtest`.
  _analyst lean: core (domain primitive alongside `Bar`/`Timestamp`)._
- [ ] OD4 — Migrate the `f64` `basis_diag.rs` copy onto the shared API, or keep a
  thin documented research adapter.
- [ ] OD5 — Falsifier home: shared canonical falsifier replaces the two existing
  `no_look_ahead_falsifier` tests, or they stay as per-loader regression guards.

## Provisional task spine (architect to order + refine)

- [ ] T1 — Implement the guarded as-of-join API in the chosen home crate
  (reusing the `partition_point(|&(t, _)| t <= bar_ts)` seam). _acceptance: AC1._
- [ ] T2 — Migrate `funding_as_of` (`crates/backtest/src/funding_data.rs:378`)
  and `basis_as_of` (`crates/backtest/src/basis_data.rs:397`) onto the API,
  behaviour-preserving. _acceptance: AC3 — zero anchor delta._
- [ ] T3 — Ship the self-proving look-ahead falsifier (deliberate future-data
  feed provably rejected; removing the guard makes it fail). _acceptance: AC2._
- [ ] T4 — (If OD2 = yes) implement the `scripts/`-level look-ahead lint + a
  planted-bypass fixture. _acceptance: AC5._
- [ ] T5 — Verify gates: `scripts/verify_anchors.sh` unchanged count;
  `cargo clippy -- -D warnings`; `python3 scripts/spec_lint.py spec/point-in-time-data-discipline`.
  _acceptance: AC3, AC4._

## Notes

- The CLAUDE.md day-1 baseline-equity-divergence e2e gate does NOT apply (this is
  a data-discipline feature, not a strategy overlay / sizing modifier — the
  correct outcome is equity UNCHANGED). Verification floor = AC2 (falsifier) +
  AC3 (zero anchor delta). See [`feature.md` § Verification floor](feature.md#verification-floor--and-why-the-day-1-e2e-divergence-gate-does-not-apply-here).
