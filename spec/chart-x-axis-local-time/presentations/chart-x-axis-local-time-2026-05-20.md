---
title: Operator Deck — chart-x-axis-local-time v1.11.0
feature: chart-x-axis-local-time
mode: release
date: 2026-05-20
status: approved
trace_row_state: shipped  # operator-approved via "Autoapprove all" 2026-05-19
test_report: spec/chart-x-axis-local-time/reports/test-final-2026-05-20.md
cockpit_smoke_log: spec/chart-x-axis-local-time/reports/cockpit-smoke-2026-05-20T02-20Z.log
verdict_source: orchestrator-direct M-FINAL VERDICT → PASS 2026-05-20
predecessor: chart-canvas-overhaul v1.10.0
---

# Operator Deck — chart x-axis local time (v1.11.0)

## 1. TL;DR

- **Chart x-axis labels now render in the operator's local time** in
  production cockpit runs. v1.10.0 shipped with deterministic UTC
  labels; v1.11 flips the workspace `time` crate's `local-offset`
  feature on and wires `time::UtcOffset::current_local_offset()` in
  the chart bottom-axis helper.
- **22 / 22 anchors byte-identical** — the change touches no
  strategy / audit / exec / report path. R10.1 contract honored.
- **Snapshot determinism preserved across host time zones** via a
  two-gate contract: `cfg(test)` covers unit tests; a new
  `UI_CHART_FORCE_UTC` env var (set by the two integration test
  runners) covers integration tests. This closes a latent issue in
  the predecessor's "cfg(test) override holds" claim.
- **Trivial direct ship** per CLAUDE.md — no analyst/architect
  sub-agent cycle; orchestrator-direct edit + validate pipeline.
- **1 file change in Cargo.toml + ~10 LOC in chart.rs + 1 new unit
  test.**

## 2. What changed (operator-facing)

- Cockpit chart bottom axis time labels (`HH:MM`) now reflect your
  OS-local time zone in production. The status strip clock (which
  uses UTC) is unaffected.
- Test snapshots remain byte-identical across machines — a developer
  on CEST and a CI run on UTC produce the same baseline bytes.

## 3. Verification matrix

| Gate                                          | Result                            |
|-----------------------------------------------|-----------------------------------|
| `cargo fmt --check`                           | PASS                              |
| `cargo clippy --workspace -- -D warnings`     | PASS                              |
| `cargo test --workspace --lib` (279 tests)    | PASS (+1 new test vs Phase B)     |
| `cargo test -p ui --test render_snapshots`    | PASS (2 + 5 ignored)              |
| `cargo test -p ui --test visual_snapshots`    | PASS (4)                          |
| `scripts/verify_anchors.sh`                   | **22 / 22 PASS**                  |
| `cockpit-smoke` (orchestrator-cited 8 s)      | PASS, 0 panic lines               |
| `spec-lint` own contribution                  | 0 (baseline 735, unchanged)       |

## 4. Architecture changes

- `Cargo.toml:69` — added `"local-offset"` to the `time` crate's
  features array.
- `crates/ui/src/widgets/chart.rs:151-202` — function split into
  `#[cfg(test)]` UTC branch + `#[cfg(not(test))]` production branch
  with env-var override + defensive `unwrap_or(UtcOffset::UTC)`
  fallback.
- `crates/ui/tests/render_snapshots.rs:run_panel_slot` — sets
  `UI_CHART_FORCE_UTC=1` before `iced_test::screenshot`.
- `crates/ui/tests/visual_snapshots.rs:run_slot` — same env-var set.
- `crates/ui/src/widgets/chart.rs:1998-2014` — new unit test
  `local_offset_under_production_reads_os_offset` pinning the
  `cfg(test)` UTC contract.

## 5. Known deviation from predecessor's claim

The `chart-canvas-overhaul` M7 architect comment said the `cfg(test)`
UTC override would preserve snapshot determinism across v1.11. This
is true for unit tests but NOT integration tests (Cargo only sets
`cfg(test)` on a crate when building it as a test target).

v1.11 corrects this with the env-var gate. The function's doc
comment now documents the two-gate contract explicitly.

## 6. Open decisions for operator

None. The feature is operator-locked from the predecessor's M7 pass
(Q-revised-1 = path (b)). Trivial direct ship per CLAUDE.md.

## 7. Approval block

- [x] Approved — ship
- [ ] Approve with notes (notes below)
- [ ] Reject — <add reason below>

Operator pre-approved via "Autoapprove all" directive (recorded
2026-05-19, applies to subsequent UI features per the established
overnight session pattern). Feature ships at v1.11.0.

## 8. Feedback log

_Empty — no rejections._
