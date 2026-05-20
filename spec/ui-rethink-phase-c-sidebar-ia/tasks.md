---
slug: ui-rethink-phase-c-sidebar-ia
status: proposed
owner: pending-analyst
updated: 2026-05-20
---

# Tasks — UI rethink Phase C (Sidebar IA flip)

> Analyst-pass stub. Replace this skeleton with R-anchored milestone
> breakdown once analyst pass lands.

## M0 — Analyst synthesis

- [ ] Read dev-note §6 Phase C (scope source-of-truth) + §3 (three-group
  sidebar IA) + §1 (current sidebar audit).
- [ ] Survey existing sidebar code path: `crates/ui/src/widgets/sidebar_nav.rs`,
  `crates/ui/src/shell.rs::screen_body`, the `Screen` enum at
  `crates/ui/src/state.rs`.
- [ ] Read the current `home::view` (will be retired) and `strategies::view`
  (will be re-shaped into the registry view).
- [ ] Surface Q1-Q5 from feature.md to operator before architect spawn.
- [ ] Lock R1-Rn requirements; close Q1-Q5 with analyst-recommended
  defaults the developer can ship against.

## M-T1 — TBD (architect-decomposed)

## M-FINAL — Tester sweep

- [ ] `cargo fmt --check` + `cargo clippy --workspace -- -D warnings` exit 0.
- [ ] `cargo test --workspace --lib` 100% PASS.
- [ ] `cargo test -p ui --test render_snapshots --test visual_snapshots
  --test panel_snapshots` — visual baselines for new Live / Strategy
  registry / Settings screens land as part of Phase C.
- [ ] `scripts/verify_anchors.sh` → ANCHORS PASS (22 / 22) — non-negotiable.
- [ ] `cockpit-smoke` → 0 panic lines in 8 s window.
- [ ] Cockpit-performance v1.0.0 idle-CPU floor (≤13.1%) verified post-flip.
- [ ] Author `spec/ui-rethink-phase-c-sidebar-ia/reports/test-final-<YYYY-MM-DD>.md`.

## Notes

- Predecessor: `ui-rethink-phase-b-lab-run v0.2.0`. Lab + chart + Train
  panel stay untouched.
- Estimated cost (per dev-note §6 Phase C): ~2-3 weeks. **Anchor risk:
  zero by construction.**
- Compat shim plan documented in feature.md "Out of scope" + Q1.
