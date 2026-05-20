---
slug: audit-tick-consumer-envelope
status: proposed
owner: pending-analyst
updated: 2026-05-20
---

# Tasks — audit-tick-consumer-envelope

> Analyst-stub. Replace with full M0/M-T1/M-FINAL decomposition once
> analyst pass lands. Canonical design source:
> [ADR-0031](../architecture/adr/0031-audit-tick-consumer-envelope.md).

## M0 — Analyst synthesis

- [ ] Read ADR-0031 + the `barter-rs` reference (linked in the ADR).
- [ ] Close Q1-Q5 (see `feature.md`) with analyst-recommended defaults.
- [ ] Lock R1-Rn requirements; identify K-risks; identify falsifiable
  hypotheses for the broadcast-tee behaviour.

## M-T1 — Architect decomposition

_pending operator decision on Q1-Q5._

## M-FINAL — Tester sweep

- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D warnings` exit 0.
- [ ] `cargo test --workspace` 100% PASS.
- [ ] `scripts/verify_anchors.sh` → 22/22 PASS (anchor preservation
  contract per R10.1).
- [ ] `cockpit-smoke` PASS 0 panics.
- [ ] New `crates/audit/src/tick.rs` unit tests cover envelope
  construction + serde roundtrip.
- [ ] New broadcast-tee integration test exercises producer →
  consumer end-to-end.
- [ ] Author `spec/audit-tick-consumer-envelope/reports/test-final-<YYYY-MM-DD>.md`.

## Notes

- Process-tooling feature; additive over the existing audit journal.
- Pairs naturally with future Lab Trail (Phase D) and v2.6 bake-off
  briefs as consumers.
