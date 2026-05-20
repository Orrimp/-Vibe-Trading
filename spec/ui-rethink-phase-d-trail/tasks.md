---
slug: ui-rethink-phase-d-trail
status: proposed
owner: pending-analyst
updated: 2026-05-20
---

# Tasks — UI rethink Phase D (Trail view)

> Analyst-pass stub. Replace this skeleton with R-anchored milestone
> breakdown once analyst pass lands.

## M0 — Analyst synthesis

- [ ] Read dev-note §6 Phase D (scope source-of-truth) + §J4 (Trail
  contract) + §1 (current audit-screen audit).
- [ ] **Q1 schema gap analysis** — read `crates/audit/src/journal.rs`
  + `crates/audit/migrations/` and confirm which per-stage
  correlation IDs (forecast_id, signal_id, fill_id) are already
  carried. Cite file:line for each. Identify the migration shape if
  missing.
- [ ] Read predecessor `audit-tick-consumer-envelope v0.1.0` shipped
  2026-05-20 — confirm `AuditTickStream` / `AuditEvent` surface
  Phase D consumer can rely on.
- [ ] Read Phase C's `screens::live::view` recent-activity panel +
  the existing `audit::view` row list — identify the chevron
  insertion points.
- [ ] Surface Q1-Q5 from feature.md to operator before architect
  spawn.

## M-T1 — TBD (architect-decomposed)

## M-FINAL — Tester sweep

- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D warnings`
  exit 0.
- [ ] `cargo test --workspace --lib` 100% PASS.
- [ ] New snapshot baselines for `trail__steady_state`,
  `trail__side_drawer_open`, `live__recent_activity_with_chevron`.
- [ ] `scripts/verify_anchors.sh` → 22/22 PASS — non-negotiable.
- [ ] `cockpit-smoke` → 0 panic lines.
- [ ] Cockpit-performance v1.0.0 idle-CPU floor (≤13.1%) preserved
  under the new broadcast subscriber.
- [ ] Author
  `spec/ui-rethink-phase-d-trail/reports/test-final-<YYYY-MM-DD>.md`.

## Notes

- Predecessor: `ui-rethink-phase-c-sidebar-ia v0.1.0`. Phase A/B/C
  surfaces stay byte-identical.
- Estimated cost (per dev-note §6 Phase D): ~3-4 weeks (depends on
  Q1 schema gap).
- **Closes deferred T-D-14 from audit-tick-consumer-envelope** —
  TcnForecaster::with_ledger runtime wiring becomes load-bearing.
